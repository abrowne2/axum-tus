use async_trait::async_trait;
use axum::{
    extract::{Extension, Path, FromRequest, FromRequestParts},
    http::{Response, StatusCode},
};
use hyper::{Body, Request, Uri, HeaderMap};
use std::{io::Cursor, sync::Arc};
use crate::TusHeaderMap;
use crate::filesystem::file_store::*;

pub struct CreationRequest {
    upload_length: u64,
    metadata: Option<String>,
}

async fn creation_handler(
    req: CreationRequest,
    Extension(file_store): Extension<Arc<dyn FileStore + Send + Sync>>,
    claims: Arc<dyn super::AuthClaims>,
) -> Result<Response<Body>, StatusCode> {
    
    let file_info = file_store.build_file(req.upload_length, req.metadata.as_deref()).await.map_err(|e| {
        println!("Error building file: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let file_info = file_store.create_file(file_info).await.map_err(|e| {
        println!("Error creating file: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response = Response::builder()
        .status(StatusCode::CREATED)
        .header("Location", format!("/{}", file_info.id()))
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

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {        
        let headers = req.headers();

        match TusHeaderMap::from_headers(&headers) {
            Ok(header_map) => {
                let tus_resumable_header = match header_map.resumable {
                    Some(resumable) => resumable,
                    None => {
                        return Err(StatusCode::from_u16(400).unwrap());
                    }
                };
                
                let upload_length = match header_map.upload_length {
                    Some(upload_length) => upload_length,
                    None => {
                        return Err(StatusCode::from_u16(400).unwrap());
                    }
                };

                // TODO add max_size value for AXUM-TUS
                let metadata = match header_map.upload_metadata {
                    None => None,
                    Some(metadata) if metadata.is_empty() => None,
                    Some(metadata) => Some(metadata)
                };

                let creation_values = CreationRequest {
                    upload_length,
                    metadata
                };

                Ok(creation_values)
            },
            Err(e) => {
                println!("Error parsing headers: {:?}", e);
                Err(StatusCode::from_u16(400).unwrap())
            }
        }
    }
}

