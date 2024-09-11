use diesel::prelude::*;
use diesel::result::Error as DbError;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{self, Reply, Response};

use super::super::voting;
use super::db::{establish_connection, models, schema};

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

enum GetBallotError {
    NotFound,
    DbError { err: DbError },
}

fn get_internal(
    connection: &mut PgConnection, poll_id: &Uuid, user_id: &Uuid
) -> Result<voting::Ballot, GetBallotError> {
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

    let (ballot, user) = match possible_ballot_result {
        Err(DbError::NotFound) => {
            return Err(GetBallotError::NotFound);
        }
        Err(err) => {
            return Err(GetBallotError::DbError { err });
        },
        Ok(r) => r,
    };

    let possible_votes_result: Result<Vec<models::Vote>, DbError> = schema::votes::table
        .filter(schema::votes::ballot_id.eq(ballot.id))
        .select(models::Vote::as_select())
        .load(connection);

    let db_votes = match possible_votes_result {
        Err(err) => {
            return Err(GetBallotError::DbError{ err });
        },
        Ok(v) => v,
    };

    let mut ballot = ballot.into_voting(db_votes);
    ballot.voter = Some(user.into_voting());

    Ok(ballot)
}
