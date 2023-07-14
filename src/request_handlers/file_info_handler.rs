use async_trait::async_trait;
use axum::{
    extract::{Extension, Path, FromRequest, FromRequestParts},
    http::{Response, StatusCode},
    body::Body
};
use hyper::{Request, Uri, HeaderMap};
use std::{io::Cursor, sync::Arc};
use crate::filesystem::file_store::*;

pub struct InfoRequest<T> {
    file_store: Arc<T>
}

pub async fn file_info_handler<T>(
    Path(id): Path<String>,
    req: InfoRequest<T>
) -> Result<Response<Body>, StatusCode> 
where 
    T: FileStore + Send + Sync + 'static
{
    let file_store = req.file_store;
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

#[async_trait]
impl<S, B, T> FromRequest<S, B> for InfoRequest<T>
where
     B: Send + 'static,
     S: Send + Sync,
     T: FileStore + Send + Sync + 'static,
{
    type Rejection = http::StatusCode;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {        
    
        let fstore = Extension::from_request(req, state).await;

        let Extension(file_store): Extension<Arc<T>> = match fstore {
            Ok(file_store) => file_store,
            Err(_) => {
                return Err(StatusCode::from_u16(500).unwrap());
            }
        };

        let info_values = InfoRequest::<T> {
            file_store
        };

        Ok(info_values)
    }
}