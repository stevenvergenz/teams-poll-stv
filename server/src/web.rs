mod db;
mod poll_api;

use std::env;
use uuid::Uuid;
use warp::Filter;
use poll_api::{new_poll, get_poll};

use crate::voting::CreatePollSettings;

pub async fn setup() {
    // Define the route
    let hello = warp::path::end()
        .map(|| warp::reply::html("Hello, World!"));

    let new_poll = warp::post()
        .and(warp::path!("api" / "poll"))
        .and(warp::path::end())
        .and(warp::body::json::<CreatePollSettings>())
        .map(new_poll);

    let get_poll = warp::get()
        .and(warp::path!("api" / "poll" / Uuid))
        .and(warp::path::end())
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
        .or(new_poll)
        .or(get_poll)
        .or(static_files);
    warp::serve(routes).run(([0, 0, 0, 0], 3000)).await;
}
