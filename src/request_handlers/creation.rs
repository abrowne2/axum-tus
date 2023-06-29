use async_trait::async_trait;
use axum::{
    extract::{Extension, Path, FromRequest, FromRequestParts},
    http::{Response, StatusCode},
    body::Body
    response::IntoResponse,
    Router,
};
use hyper::{Body, Request, Response, Uri, HeaderMap};
use std::{io::Cursor, sync::Arc};
use crate::TusHeaderMap;
use crate::filesystem::file_store::*;

enum CreationResponder {
    Success(String),
    Failure(StatusCode, String)
}


#[derive(Debug)]
pub struct CreationRequest {
    upload_length: u64,
    metadata: Option<&'r str>,
}

async fn creation_handler(
    req: CreationRequest,
    Extension(file_store): Extension<Arc<dyn FileStore + Send + Sync>>,
    claims: Arc<dyn super::AuthClaims>,
) -> Result<Response<Body>, StatusCode> {

    let file_info = file_store.create_file(file_info).await.map_err(|e| {
        println!("Error creating file: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response = Response::builder()
        .status(StatusCode::CREATED)
        .header("Location", format!("/{}", file_info.get_id()))
        .body(Body::empty())
        .unwrap();

    Ok(response)
}

#[async_trait]
impl<S, B> FromRequest<S, B> for CreationRequest
where
     B: Send + 'static,
     S: Send + Sync,
{
    type Rejection = http::StatusCode;

    async fn from_request(req: &mut Request<B>, state: &S) -> Result<Self, Self::Rejection> {        
        let headers = req.headers();

        match TusHeaderMap::from_headers(&headers) {
            Ok(header_map) => {
                let tus_resumable_header = match header_map.resumable {
                    Some(resumable) => resumable,
                    None => {
                        return Self::Rejection((
                            StatusCode::BadRequest,
                            "Missing or invalid Tus-Resumable header",
                        ));
                    }
                };
                
                let upload_length = match header_map.upload_length {
                    Some(upload_length) => upload_length,
                    None => {
                        return Self::Rejection((
                            StatusCode::BadRequest,
                            "Missing Upload-Length header",
                        ));
                    }
                };

                    // TODO add max_size value for AXUM-TUS
                    
                let metadata = match header_map.upload_metadata {
                    None => None,
                    Some(metadata) if metadata.is_empty() => None,
                    Some(metadata) => Some(metadata.as_str())
                };

                let creation_values = CreationRequest {
                    upload_length,
                    metadata
                };

                creation_values
            },
            Err(e) => {
                println!("Error parsing headers: {:?}", e);
                Err((StatusCode::BadRequest, "Error parsing headers"))
            }
        }
    }
}

