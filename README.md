# Axum: TUS protocol 

A tower that implements the TUS protocol for uploading media. Can use by adding a layer to your Axum application.

Note: This is currently a work in progress, and not fully functional.


## Using it in your axum server

- You will be able to integrate this into your axum server by optionally using the `setup_tus_routes` function on an `axum::Router`. It will setup the needed routes for your TUS upload server; you are still able to extend it:

```rust
pub fn setup_tus_routes<T>(router: axum::Router, file_store: T) -> axum::Router
    where
        T: FileStore + Send + Sync + 'static,
    {
        let tus_layer = tus_service::TusLayer::<T> {
            file_store: std::sync::Arc::new(file_store)
        };

        let new_router = router
            .route("/", post(creation_handler::<T>).options(info_handler))
            .route("/:id", head(file_info_handler::<T>).patch(upload_handler::<T>))
            .layer(tus_layer);

        new_router
}
```

- Also, there is a `LocalFileStore` which is included, but as an example for local filesystem saves. It is not tested for production use and ideally you should extend the `FileStore` trait to add support for Google Cloud Storage, and Amazon S3, etc.


