use async_trait::async_trait;
use axum::{
    extract::{Extension, Path, FromRequest, FromRequestParts},
    http::{Response, StatusCode},
    // body::Body
};
use hyper::{Body, Request, Uri, HeaderMap};
use std::{io::Cursor, sync::Arc};
use crate::filesystem::file_store::*;

pub async fn file_info_handler(
    Path(id): Path<String>,
    Extension(file_store): Extension<Arc<dyn FileStore + Send + Sync>>,
    claims: Arc<dyn super::AuthClaims>,
) -> Result<Response<Body>, StatusCode> {
    match file_store.get_file_info(&id).await {
        Ok(file) => {
            let response = http::Response::builder()
                .status(StatusCode::NO_CONTENT)
                .header(crate::AxumTusHeaders::UploadLength.name(), file.length_str())
                .header(crate::AxumTusHeaders::UploadOffset.name(), file.metadata_str())
                .header(axum::http::header::CACHE_CONTROL, "no-store")
                .body(Body::empty())
                .unwrap();
            
            Ok(response)
        },
        Err(_) => Err(StatusCode::NOT_FOUND) 
    }     
}

