use warp::Filter;

#[tokio::main]
async fn main() {
    // Define the route
    let hello = warp::path::end()
        .map(|| warp::reply::html("Hello, World!"));

    // Define the static files route
    let static_files = warp::path("static")
        .and(warp::fs::dir("/path/to/static/files"));

    // Start the server
    warp::serve(hello)
        .run(([127, 0, 0, 1], 3000))
        .await;
}
