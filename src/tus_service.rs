use futures::Future;
use http::{Request, StatusCode, HeaderValue, Response};
use axum::body::Body;
use crate::{FileStore, TusHeaderMap};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service, ServiceBuilder, service_fn, MakeService, ServiceExt};
use crate::AxumTusHeaders;

pub struct TusLayer<T: FileStore + Send + Sync + 'static> {
    file_store: Arc<T>,
}

impl<S, T> Layer<S> for TusLayer<T>
where
    S: Service<http::Request<Body>> + Clone + Send + 'static,
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

pub struct TusService<S, T: FileStore> {
    service: S,
    file_store: Arc<T>,
}

impl<S, T> Service<Request<Body>> for TusService<S, T>
where
    S: Service<Request<Body>, Response = Response<Body>> + Send + 'static,
    T: FileStore + Send + Sync + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut request: http::Request<Body>) -> Self::Future {
        
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
