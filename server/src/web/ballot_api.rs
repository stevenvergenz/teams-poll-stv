use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DbError};
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{self, Reply, Response};

use crate::error;
use crate::voting;
use super::db::{establish_connection, models, schema};
use super::poll_api::get_internal as get_poll;

pub fn new(poll_id: Uuid, user_id: Uuid, ballot: voting::UnvalidatedCreateBallot) -> Response {
    let connection = &mut establish_connection();

    // todo: get owner = session user
    let owner = models::User { id: user_id, display_name: String::from("Anonymous") };
    let owner_result = diesel::insert_into(schema::users::table)
        .values(&owner)
        .on_conflict_do_nothing()
        .execute(connection);
    if let Err(err) = owner_result {
        return reply::with_status(
            format!("Error creating user: {err}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        ).into_response();
    }
    let owner: voting::User = owner.into();

    // fetch poll from db
    let poll = match get_poll(connection, &poll_id) {
        Err(err) => {
            return reply::with_status(err.to_string(), StatusCode::BAD_REQUEST).into_response();
        },
        Ok(p) => p,
    };

    // validate ballot against poll
    let ballot = match ballot.validate(poll) {
        Err(err) => {
            return reply::with_status(err.to_string(), StatusCode::BAD_REQUEST).into_response();
        },
        Ok(b) => b,
    };

    let insert_result: Result<DateTime<Utc>, DbError> = connection.transaction(|connection| {
        // insert new ballot into the db
        let db_ballot: models::Ballot = diesel::insert_into(schema::ballots::table)
            .values(models::CreateBallot::new(poll_id, user_id))
            .get_result(connection)?;

        // insert votes into db
        let _ = diesel::insert_into(schema::votes::table)
            .values(ballot.ranked_preferences.iter().enumerate().map(|(preference, option)| {
                models::Vote {
                    ballot_id: db_ballot.id,
                    preference: preference as i32,
                    option: option.0 as i32,
                }
            }).collect::<Vec<_>>())
            .execute(connection)?;

        Ok(db_ballot.created_at.and_utc())
    });
    let created_at = match insert_result {
        Err(DbError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
            return reply::with_status(reply::reply(), StatusCode::CONFLICT).into_response();
        },
        Err(err) => {
            return reply::with_status(
                format!("Failed to create ballot: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response();
        },
        Ok(dt) => dt,
    };

    let mut ballot = voting::Ballot::new(owner, ballot);
    ballot.created_at = created_at;

    reply::with_status(reply::json(&ballot), StatusCode::CREATED).into_response()
}

pub fn get(poll_id: Uuid, user_id: Uuid) -> Response {
    let connection = &mut establish_connection();
    match get_internal(connection, &poll_id, &user_id) {
        Err(err) => err.into_response(),
        Ok(ballot) => reply::json(&ballot).into_response(),
    }
}

pub fn update(poll_id: Uuid, user_id: Uuid, new_ballot: voting::UnvalidatedCreateBallot) -> Response {
    let connection = &mut establish_connection();

    // fetch poll from db
    let poll = match get_poll(connection, &poll_id) {
        Err(err) => {
            return err.into_response();
        },
        Ok(p) => p,
    };

    // confirm success
    let new_ballot = match new_ballot.validate(poll) {
        Err(err) => {
            return reply::with_status(err.to_string(), StatusCode::BAD_REQUEST).into_response();
        },
        Ok(b) => b,
    };

    // fetch ballot id from db
    let result = schema::ballots::table
        .filter(schema::ballots::poll_id.eq(poll_id).and(schema::ballots::user_id.eq(user_id)))
        .select(schema::ballots::id)
        .first(connection);

    // confirm success
    let ballot_id: i32 = match result {
        Err(DbError::NotFound) => {
            return reply::with_status(reply::reply(), StatusCode::NOT_FOUND).into_response();
        },
        Err(err) => {
            return reply::with_status(
                format!("Error updating ballot: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response();
        },
        Ok(id) => id,
    };

    // update preferences
    let result = diesel::insert_into(schema::votes::table)
        .values(
            new_ballot.ranked_preferences.iter().enumerate()
            .map(|(idx, opt)| models::Vote { ballot_id, preference: idx as i32, option: opt.0 as i32, })
            .collect::<Vec<models::Vote>>()
        )
        .on_conflict((schema::votes::ballot_id, schema::votes::preference))
        .do_update()
        .set(schema::votes::option.eq(diesel::upsert::excluded(schema::votes::option)))
        .execute(connection);

    // validate success
    if let Err(err) = result {
        return reply::with_status(
            format!("Failed to update ballot: {err}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        ).into_response();
    }

    // delete excesses
    let result = diesel::delete(schema::votes::table)
        .filter(
            schema::votes::ballot_id.eq(ballot_id)
            .and(schema::votes::preference.ge(new_ballot.ranked_preferences.len() as i32))
        )
        .execute(connection);

    // confirm success
    if let Err(err) = result {
        return reply::with_status(
            format!("Failed to update ballot: {err}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        ).into_response();
    }

    match get_internal(connection, &poll_id, &user_id) {
        Err(err) => err.into_response(),
        Ok(ballot) => reply::with_status(reply::json(&ballot), StatusCode::OK).into_response(),
    }
}

pub fn delete(poll_id: Uuid, user_id: Uuid) -> Response {
    let connection = &mut establish_connection();
    let result = diesel::delete(schema::ballots::table.filter(
        schema::ballots::poll_id.eq(poll_id).and(schema::ballots::user_id.eq(user_id))
    )).execute(connection);

    match result {
        Err(err) => {
            reply::with_status(
                format!("Failed to delete ballot: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response()
        },
        Ok(affected) if affected == 0 => {
            reply::with_status(reply::reply(), StatusCode::NOT_FOUND).into_response()
        },
        Ok(_) => {
            reply::with_status(reply::reply(), StatusCode::NO_CONTENT).into_response()
        },
    }
}

fn get_internal(
    connection: &mut PgConnection, poll_id: &Uuid, user_id: &Uuid
) -> Result<voting::Ballot, error::HttpGetError> {
    // fetch ballot from db
    let ballot_result: Result<(models::Ballot, models::User), DbError> =
        schema::ballots::table.filter(
            schema::ballots::poll_id.eq(poll_id).and(schema::ballots::user_id.eq(user_id))
        ).inner_join(schema::users::table)
        .select((
            models::Ballot::as_select(),
            models::User::as_select(),
        ))
        .first(connection);

    let (db_ballot, db_user) = match ballot_result {
        Err(err @ DbError::NotFound) => {
            return Err(error::db_get(err, StatusCode::NOT_FOUND, "ballot/voter", None));
        }
        Err(err) => {
            return Err(error::db_get(err, StatusCode::INTERNAL_SERVER_ERROR, "ballot/voter", None));
        },
        Ok(r) => r,
    };

    let votes_result: Result<Vec<models::Vote>, DbError> = schema::votes::table
        .filter(schema::votes::ballot_id.eq(db_ballot.id))
        .select(models::Vote::as_select())
        .load(connection);

    let db_votes = match votes_result {
        Err(err) => {
            return Err(error::db_get(err, StatusCode::INTERNAL_SERVER_ERROR, "vote", Some("ballot")));
        },
        Ok(v) => v,
    };

    let poll = get_poll(connection, poll_id)?;

    let ballot = match (db_ballot, db_votes, db_user, poll).try_into() {
        Err(err) => {
            return Err(error::HttpGetError::from(err));
        },
        Ok(b) => b,
    };

    Ok(ballot)
}
