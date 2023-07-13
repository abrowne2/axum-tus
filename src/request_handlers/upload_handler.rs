use async_trait::async_trait;
use axum::{
    extract::{Extension, Path, FromRequest, FromRequestParts},
    http::{Response, StatusCode},
    body::{Full, HttpBody}, response
};
use hyper::{Body, Request};
use std::{io::Cursor, sync::Arc, result};
use crate::TusHeaderMap;
use crate::filesystem::file_store::*;

pub struct UploadRequest {
    upload_offset: u64,
    upload_bytes: bytes::Bytes
}

pub async fn upload_handler(
    req: UploadRequest,
    Path(id): Path<String>,
    Extension(file_store): Extension<Arc<dyn FileStore + Send + Sync>>,
    claims: Arc<dyn super::AuthClaims>,
) -> Result<Response<Body>, StatusCode> {    
    let mut bytes_vector = req.upload_bytes.to_vec();
    let upload_slice: &mut [u8] = bytes_vector.as_mut_slice();

    // if the file doesn't exist, return 404
    if !file_store.exists(&id).await {
        return Err(StatusCode::NOT_FOUND);
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
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let response = Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header(crate::AxumTusHeaders::UploadOffset.name(), req.upload_offset.to_string())
        .body(Body::empty())
        .unwrap();

    Ok(response)
}

#[async_trait]
impl<S, B> FromRequest<S, B> for UploadRequest
where
     B: axum::body::HttpBody + Unpin + Send + Sync + 'static,
     B::Data: Send,
     S: Send + Sync,
{
    type Rejection = http::StatusCode;
    
    async fn from_request(mut req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {        
        let headers = req.headers();

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

        match hyper::body::to_bytes(req.body_mut()).await {
            Ok(bytes) => {
                let upload_values = UploadRequest {
                    upload_offset,
                    upload_bytes: bytes
                };

                Ok(upload_values)
            },
            Err(_) => Err(StatusCode::from_u16(400).unwrap())
        }
    }
}