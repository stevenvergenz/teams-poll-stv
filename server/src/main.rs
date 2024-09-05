#[tokio::main]
async fn main() {
    server::web::setup().await;
}
