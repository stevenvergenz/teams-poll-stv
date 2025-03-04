// #[tokio::main]
// async fn main() {
//     server::web::setup().await;
// }

use teams_poll_stv::app::route::App;

fn main() {
    dioxus::launch(App);
}
