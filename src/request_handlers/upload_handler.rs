use async_trait::async_trait;
use axum::{
    extract::{Extension, Path, FromRequest, FromRequestParts},
    http::{Response, StatusCode},
    body::{Body,Full, HttpBody}, response,
};
use hyper::{Request};
use std::{io::Cursor, sync::Arc, result, convert::Infallible};
use crate::TusHeaderMap;
use crate::filesystem::file_store::*;

pub struct UploadRequest<T> {
    upload_offset: u64,
    upload_bytes: bytes::Bytes,
    file_store: Arc<T>
}

pub async fn upload_handler<T>(
    Path(id): Path<String>,
    req: UploadRequest<T>,
    // claims: Arc<dyn super::AuthClaims>,
) -> Result<impl response::IntoResponse, Infallible> 
where
    T: FileStore + Send + Sync + 'static
{    
    let file_store = req.file_store;
    let mut bytes_vector = req.upload_bytes.to_vec();
    let upload_slice: &mut [u8] = bytes_vector.as_mut_slice();

    // if the file doesn't exist, return 404
    if !file_store.exists(&id).await {
        return Ok(Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap());
    }

    let final_offset: Result<u64, FileStoreError> = match file_store.patch_file(&id, req.upload_offset, upload_slice).await {
        Ok(result) => {
            let final_offset = match result {
                PatchOption::Patched(offset) => offset,
                PatchOption::Completed(file_info) => {
                    // check auto terminate logic here.

                    *file_info.length()
                }
            };

            Ok(final_offset)
        },
        Err(e) => {
            println!("Error patching file: {:?}", e);
            return Ok(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::empty()).unwrap());
        }
    };

    let response = Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header(crate::AxumTusHeaders::UploadOffset.name(), final_offset.unwrap().to_string())
        .body(Body::empty())
        .unwrap();

    Ok(response)
}

#[async_trait]
impl<S, B, T> FromRequest<S, B> for UploadRequest<T>
where
     B: axum::body::HttpBody + Unpin + Send + Sync + 'static,
     B::Data: Send,
     S: Send + Sync,
     T: FileStore + Send + Sync + 'static
{
    type Rejection = http::StatusCode;
    
    async fn from_request(mut req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {        
        let (parts, body) = req.into_parts();

        let headers = parts.headers;

        let header_map =  TusHeaderMap::from_headers(&headers);
        let tus_resumable_header = match header_map.resumable {
            Some(resumable) => resumable,
            None => {
                return Err(StatusCode::from_u16(400).unwrap());
            }
        };
        
        let upload_offset = match header_map.upload_offset {
            Some(upload_offset) => upload_offset,
            None => {
                return Err(StatusCode::from_u16(400).unwrap());
            }
        };

        match headers.get(http::header::CONTENT_TYPE) {
            Some(content_type) => {
                if content_type != "application/offset+octet-stream" {
                    return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
                }
            },
            None => {
                return Err(StatusCode::from_u16(400).unwrap());
            }
        }

        let bytes_body = match hyper::body::to_bytes(body).await {
            Ok(bytes) => {

                bytes
            },
            Err(_) => return Err(StatusCode::from_u16(400).unwrap())
        };

        let file_store = match parts.extensions.get::<Extension<Arc<T>>>() {
            Some(file_store) => Arc::clone(file_store),
            None => {
                return Err(StatusCode::from_u16(500).unwrap());
            }
        };

        let upload_values = UploadRequest::<T> {
            upload_offset,
            upload_bytes: bytes_body,
            file_store
        };

        Ok(upload_values)
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_uri_parse() {
        let uri = http::Uri::from_static("https://foo_api.com/:id");
        let path = uri.path().to_string();

        let path_parts: Vec<&str> = path.split("/:").collect();

        // assert_eq!(path_parts[0], "https://foo_api.com");
        assert_eq!(path_parts[1], "id");
    }
}