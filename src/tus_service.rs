use futures::{Future, future::BoxFuture};
use http::{Request, StatusCode, HeaderValue, Response};
use axum::body::Body;
use crate::{FileStore, TusHeaderMap};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use crate::AxumTusHeaders;

pub type BoxBody = http_body::combinators::UnsyncBoxBody<bytes::Bytes, axum::Error>;

#[derive(Clone)]
pub struct TusLayer<T: FileStore + Send + Sync + 'static> {
    pub file_store: Arc<T>,
}

impl<S, T> Layer<S> for TusLayer<T>
where
    S: Service<http::Request<axum::body::Body>> + Clone + Send + 'static,
    T: FileStore + Send + Sync + 'static,
    S::Response: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    S::Future: Send + 'static,
{
    type Service = TusService<S, T>;

    fn layer(&self, service: S) -> Self::Service {
        TusService {
            service,
            file_store: Arc::clone(&self.file_store),
        }
    }
}

#[derive(Clone)]
pub struct TusService<S, T: FileStore> {
    service: S,
    file_store: Arc<T>,
}

impl<S, T> Service<Request<axum::body::Body>> for TusService<S, T>
where
    S: Service<Request<axum::body::Body>, Response = Response<BoxBody>> + Send + 'static,
    T: FileStore + Send + Sync + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<BoxBody>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut request: http::Request<axum::body::Body>) -> Self::Future {
        
        // make filestore usable inside request handlers.
        request.extensions_mut().insert(Arc::clone(&self.file_store));
        
        let fut = self.service.call(request);

        Box::pin(async move {
            let mut response = fut.await?;

            let tus_header_map = TusHeaderMap::with_tus_version();

            tus_header_map.apply(response.headers_mut());

            Ok(response)
        })
    }
}
