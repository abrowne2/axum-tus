use futures::Future;
use http::{Request, StatusCode, HeaderValue, Response};
use axum::body::Body;
use crate::FileStore;
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
            version: "1.0.0"
        }
    }
}

pub struct TusService<S, T: FileStore> {
    service: S,
    version: &'static str,
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

    fn call(&mut self, request: http::Request<Body>) -> Self::Future {
        

        let headers = request.headers().clone();
        let tus_headers = AxumTusHeaders::from_headers(&headers);
        let fut = self.service.call(request);

        Box::pin(async move {
            let mut response = fut.await?;

            // We may want to modify the response headers here so that we can adjust for the protocol

            // for header in &headers {
            //     header.apply(response.headers_mut());
            // }

            Ok(response)
        })
    }
}
