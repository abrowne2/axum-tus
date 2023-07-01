use async_trait::async_trait;
use axum::{
    extract::{Extension, Path, FromRequest, FromRequestParts},
    http::{Response, StatusCode},
    // body::Body
};
use hyper::{Body, Request, Uri, HeaderMap};
use std::{io::Cursor, sync::Arc};
use crate::filesystem::file_store::*;

async fn file_info_handler(
    Path(id): Path<String>,
    Extension(file_store): Extension<Arc<dyn FileStore + Send + Sync>>,
    claims: Arc<dyn super::AuthClaims>,
) -> impl axum::response::IntoResponse {
    match file_store.get_file_info(&id).await {
        Ok(file) => {
            let mut response = http::Response::builder()
                .status(StatusCode::NO_CONTENT)
                .header(crate::AxumTusHeaders::UploadLength.name(), file.length())
                .header(crate::AxumTusHeaders::UploadOffset.name(), file.metadata())
                .body(Body::empty())
                .unwrap();
            
            response
        },
        Err(_) => StatusCode::NOT_FOUND.into_response()
    }     

    Ok(response)
}

