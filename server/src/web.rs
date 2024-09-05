mod db;
mod poll_api;

use std::env;
use uuid::Uuid;
use warp::Filter;
use poll_api::get_poll;

pub async fn setup() {
    // Define the route
    let hello = warp::path::end()
        .map(|| warp::reply::html("Hello, World!"));

    let get_poll = warp::get()
        .and(warp::path!("api" / "poll" / Uuid))
        .map(get_poll);

    // Define the static files route
    let cwd = env::current_exe().expect("Could not get current executable path");
    let static_path = cwd.parent().unwrap()
        .parent().unwrap()
        .parent().unwrap()
        .parent().unwrap()
        .join("client").join("static");
    let static_files = warp::path("static")
        .and(warp::fs::dir(static_path));

    // Start the server
    let routes = hello
        .or(get_poll)
        .or(static_files);
    warp::serve(routes).run(([0, 0, 0, 0], 3000)).await;
}
