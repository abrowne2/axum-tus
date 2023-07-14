use async_trait::async_trait;
use axum::{
    extract::{Extension, Path, FromRequest, FromRequestParts},
    http::{Response, StatusCode}, response::IntoResponse,
};
use hyper::{ Request, Uri, HeaderMap};
use axum::body::Body;
use std::{io::Cursor, sync::Arc, convert::Infallible};
use crate::TusHeaderMap;
use crate::filesystem::file_store::*;

use super::AuthClaims;

pub struct CreationRequest<T> {
    upload_length: u64,
    metadata: Option<String>,
    file_store: Arc<T>
}

pub async fn creation_handler<T>(
    req: CreationRequest<T>,
    // claims: A,
) -> Result<impl IntoResponse, Infallible> 
where 
    T: FileStore + Send + Sync + 'static
{
    // NOTE - To allow extendability, we're fetching the file store from the request
    let file_store = req.file_store;
    
    let file_info = match file_store.build_file(req.upload_length, req.metadata.as_deref()).await {
        Ok(info) => info,
        Err(e) => {
            println!("Error building file: {:?}", e);
            return Ok(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::empty()).unwrap());
        }
    };

    let file_info = match file_store.create_file(file_info).await {
        Ok(info) => info,
        Err(e) => {
            println!("Error creating file: {:?}", e);
            return Ok(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::empty()).unwrap());
        }
    };

    let response = Response::builder()
        .status(StatusCode::CREATED)
        .header("Location", format!("/{}", file_info.id()))
        .body(Body::empty())
        .unwrap();

    Ok(response)
}

#[async_trait]
impl<S, B, T> FromRequest<S, B> for CreationRequest<T>
where
     B: Send + 'static,
     S: Send + Sync,
     T: FileStore + Send + Sync + 'static,
{
    type Rejection = http::StatusCode;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {        
        let headers = req.headers();
        
        let header_map = TusHeaderMap::from_headers(&headers);
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

        let fstore = Extension::from_request(req, state).await;

        let Extension(file_store): Extension<Arc<T>> = match fstore {
            Ok(file_store) => file_store,
            Err(_) => {
                return Err(StatusCode::from_u16(500).unwrap());
            }
        };

        let creation_values = CreationRequest::<T> {
            upload_length,
            metadata,
            file_store
        };

        Ok(creation_values)
    }
}

