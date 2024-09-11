mod db;
mod poll_api;
mod ballot_api;

use std::env;
use uuid::Uuid;
use warp::Filter;

use crate::voting::{Ballot, CreatePollSettings, UpdatePollSettings};

pub async fn setup() {
    // define the poll API

    let list_polls = warp::get()
        .and(warp::path!("api" / "poll"))
        .and(warp::path::end())
        .map(poll_api::list);

    let new_poll = warp::post()
        .and(warp::path!("api" / "poll"))
        .and(warp::path::end())
        .and(warp::header::<Uuid>("user-id"))
        .and(warp::body::json::<CreatePollSettings>())
        .map(poll_api::new);

    let get_poll = warp::get()
        .and(warp::path!("api" / "poll" / Uuid))
        .and(warp::path::end())
        .map(poll_api::get);

    let update_poll = warp::patch()
        .and(warp::path!("api" / "poll" / Uuid))
        .and(warp::path::end())
        .and(warp::header::<Uuid>("user-id"))
        .and(warp::body::json::<UpdatePollSettings>())
        .map(poll_api::update);

    let delete_poll = warp::delete()
        .and(warp::path!("api" / "poll" / Uuid))
        .and(warp::path::end())
        .and(warp::header::<Uuid>("user-id"))
        .map(poll_api::delete);

    // define the ballot API

    let new_ballot = warp::post()
        .and(warp::path!("api" / "poll" / Uuid / "my_ballot"))
        .and(warp::path::end())
        .and(warp::header::<Uuid>("user-id"))
        .and(warp::body::json::<Ballot>())
        .map(ballot_api::new);

    let get_ballot = warp::get()
        .and(warp::path!("api" / "poll" / Uuid / "my_ballot"))
        .and(warp::path::end())
        .and(warp::header::<Uuid>("user-id"))
        .map(ballot_api::get);

    let update_ballot = warp::patch()
        .and(warp::path!("api" / "poll" / Uuid / "my_ballot"))
        .and(warp::path::end())
        .and(warp::header::<Uuid>("user-id"))
        .and(warp::body::json::<Ballot>())
        .map(ballot_api::update);

    let delete_ballot = warp::delete()
        .and(warp::path!("api" / "poll" / Uuid / "my_ballot"))
        .and(warp::path::end())
        .and(warp::header::<Uuid>("user-id"))
        .map(ballot_api::delete);

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
    let routes =
        list_polls.or(new_poll).or(get_poll).or(update_poll).or(delete_poll)
        .or(new_ballot).or(get_ballot).or(update_ballot).or(delete_ballot)
        .or(static_files);
    warp::serve(routes).run(([0, 0, 0, 0], 3000)).await;
}
