use diesel::prelude::*;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{self, Reply, Response, WithStatus};

use crate::voting;
use super::db::{establish_connection, models, schema};

pub fn new_poll(options: voting::CreatePollSettings) -> Response {
    let connection = &mut establish_connection();
    let new_poll_result = diesel::insert_into(schema::polls::table)
        .values(models::CreatePollSettings::from(options))
        .returning(models::Poll::as_returning())
        .get_result(connection);

    let new_poll = match new_poll_result {
        Err(err) => {
            return reply::with_status(
                format!("Error creating poll: {}", err),
                StatusCode::BAD_REQUEST,
            ).into_response();
        },
        Ok(p) => p,
    };

    // todo: insert options

    reply::json(&new_poll).into_response()
}

pub fn get_poll(id: Uuid) -> Response {
    let connection = &mut establish_connection();
    let results = schema::polls::table
        .filter(schema::polls::id.eq(id))
        .select(models::Poll::as_select())
        .load(connection)
        .expect("Error loading poll");

    if results.len() == 0 {
        return reply::with_status("No poll found", StatusCode::NOT_FOUND).into_response();
    }

    // todo: fetch options

    let voting_poll = results.into_iter().next().unwrap().as_voting();
    reply::json(&voting_poll).into_response()
}
