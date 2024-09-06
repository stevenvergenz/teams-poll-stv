use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{self, Reply, Response, WithStatus};

use crate::voting::{CreatePollSettings, Id, Poll, User};

pub fn new_poll(options: CreatePollSettings) -> WithStatus<&'static str> {
    reply::with_status("Created", StatusCode::CREATED)
}

pub fn get_poll(id: Uuid) -> Response {
    let owner = User::new(Id::new(), String::from("Steven"));
    match Poll::new(owner, CreatePollSettings {
        id: Some(id),
        title: String::from("Test Poll"),
        options: vec![
            String::from("Option 1"),
            String::from("Option 2")
        ],
        winner_count: 1,
        write_ins_allowed: false,
        close_after_time: None,
        close_after_votes: None,
    }) {
        Ok(poll) => {
            reply::json(&poll).into_response()
        },
        Err(err) => {
            reply::with_status(err.to_string(), StatusCode::INTERNAL_SERVER_ERROR).into_response()
        },
    }
}
