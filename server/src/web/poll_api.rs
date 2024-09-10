use diesel::prelude::*;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{self, Reply, Response};

use crate::voting;
use super::db::{establish_connection, models, schema};

pub fn new_poll(options: voting::CreatePollSettings) -> Response {
    let connection = &mut establish_connection();

    // todo: get owner = session user
    let owner = models::User { id: Uuid::nil(), display_name: String::from("Anonymous") };
    let user_upsert_result = diesel::insert_into(schema::users::table)
        .values(&owner)
        .on_conflict_do_nothing()
        .execute(connection);

    if let Err(err) = user_upsert_result {
        return reply::with_status(
            format!("Error creating user: {}", err),
            StatusCode::INTERNAL_SERVER_ERROR,
        ).into_response();
    }

    let new_poll_result = diesel::insert_into(schema::polls::table)
        .values(models::CreatePollSettings::from(&owner.id, options))
        .execute(connection);

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

    reply::with_status(reply::json(&new_poll), StatusCode::CREATED).into_response()
}

pub fn get_poll(id: Uuid) -> Response {
    let connection = &mut establish_connection();
    let polls_result = schema::polls::table
        .filter(schema::polls::id.eq(id))
        .select(models::Poll::as_select())
        .load(connection);

    let polls = match polls_result {
        Err(err) => {
            return reply::with_status(
                format!("Error fetching poll: {}", err),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response();
        },
        Ok(p) => p,
    };

    if polls.len() == 0 {
        return reply::with_status("No poll found", StatusCode::NOT_FOUND).into_response();
    }

    // todo: fetch options

    let voting_poll: voting::Poll = polls.into_iter().next().unwrap().as_voting();
    reply::json(&voting_poll).into_response()
}
