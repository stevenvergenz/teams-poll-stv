use diesel::prelude::*;
use diesel::result::Error as DbError;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{self, Reply, Response};

use crate::error;

use super::super::voting;
use super::db::{establish_connection, models, schema};
use super::poll_api::get_internal as get_poll;

pub fn new(poll_id: Uuid, user_id: Uuid, ballot: voting::Ballot) -> Response {
    todo!()
}

pub fn get(poll_id: Uuid, user_id: Uuid) -> Response {
    todo!()
}

pub fn update(poll_id: Uuid, user_id: Uuid, ballot: voting::Ballot) -> Response {
    todo!()
}

pub fn delete(poll_id: Uuid, user_id: Uuid) -> Response {
    todo!()
}

fn get_internal(
    connection: &mut PgConnection, poll_id: &Uuid, user_id: &Uuid
) -> Result<voting::Ballot, error::HttpGetError> {
    // fetch ballot from db
    let possible_ballot_result: Result<(models::Ballot, models::User), DbError> =
        schema::ballots::table.filter(
            schema::ballots::poll_id.eq(poll_id).and(schema::ballots::user_id.eq(user_id))
        ).inner_join(schema::users::table)
        .select((
            models::Ballot::as_select(),
            models::User::as_select(),
        ))
        .first(connection);

    let (db_ballot, db_user) = match possible_ballot_result {
        Err(err @ DbError::NotFound) => {
            return Err(error::db_get(err, StatusCode::NOT_FOUND, "ballot/voter", None));
        }
        Err(err) => {
            return Err(error::db_get(err, StatusCode::INTERNAL_SERVER_ERROR, "ballot/voter", None));
        },
        Ok(r) => r,
    };

    let possible_votes_result: Result<Vec<models::Vote>, DbError> = schema::votes::table
        .filter(schema::votes::ballot_id.eq(db_ballot.id))
        .select(models::Vote::as_select())
        .load(connection);

    let db_votes = match possible_votes_result {
        Err(err) => {
            return Err(error::db_get(err, StatusCode::INTERNAL_SERVER_ERROR, "vote", Some("ballot")));
        },
        Ok(v) => v,
    };

    let poll = get_poll(connection, &db_ballot.poll_id)?;
    let ballot = match (db_ballot, db_votes, db_user, poll).try_into() {
        Err(err) => {
            return Err(error::HttpGetError::from(err));
        },
        Ok(b) => b,
    };

    Ok(ballot)
}
