use axum::{
    http::{StatusCode},
};
use hyper::{Body};

async fn info_handler() -> impl axum::response::IntoResponse {
    // NOTE the Tus headers are applied at the tus service level (see src/tus_service.rs)
    // for every request
    let mut response = http::Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .unwrap(); 

    response
}
