use axum::Router;
use axum_tus::setup_tus_routes;
use axum_tus::LocalFileStore;

fn main() {
    /* need to localize the root path better. */
    let store = LocalFileStore::new("/tmp/tus-store").unwrap();

    let mut app = Router::new();
    
    app = setup_tus_routes::<LocalFileStore>(app, store);

    let addr = "127.0.0.1:8001".parse().unwrap();
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

}
