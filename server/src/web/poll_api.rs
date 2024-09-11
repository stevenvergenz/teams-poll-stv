use std::collections::HashMap;

use diesel::prelude::*;
use diesel::result::Error as DbError;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{self, Reply, Response};

use crate::voting;
use super::db::{establish_connection, models, schema};

pub fn list() -> Response {
    let connection = &mut establish_connection();
    let possible_poll_results: Result<Vec<(models::Poll, models::User)>, DbError> = schema::polls::table
        .inner_join(schema::users::table)
        .select((models::Poll::as_select(), models::User::as_select()))
        .limit(100)
        .load(connection);

    let mut polls: HashMap<Uuid, voting::Poll> = match possible_poll_results {
        Err(err) => {
            return reply::with_status(
                format!("Failed to get polls: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response();
        },
        Ok(polls) => {
            polls.into_iter().map(|(poll, user)| {
                let mut poll = poll.into_voting();
                poll.owner = Some(user.into_voting());
                (poll.id.0, poll)
            }).collect()
        }
    };

    let possible_option_results: Result<Vec<models::PollOption>, DbError> = schema::polloptions::table
        .filter(schema::polloptions::poll_id.eq_any(polls.keys()))
        .order(schema::polloptions::id)
        .select(models::PollOption::as_select())
        .load(connection);
    let options = match possible_option_results {
        Err(err) => {
            return reply::with_status(
                format!("Failed to get options: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response();
        },
        Ok(options) => options,
    };
    for option in options.into_iter() {
        if let Some(poll) = polls.get_mut(&option.poll_id) {
            let option = option.into_voting();
            poll.option_ids.push(option.id);
            match &mut poll.options {
                None => { poll.options = Some(vec![option]); },
                Some(vec) => { vec.push(option); },
            }
        }
    }

    reply::json(&polls.values().collect::<Vec<&voting::Poll>>()).into_response()

}

pub fn new(settings: voting::CreatePollSettings) -> Response {
    let connection = &mut establish_connection();

    // todo: get owner = session user
    let owner = models::User { id: Uuid::nil(), display_name: String::from("Anonymous") };
    let user_upsert_result = diesel::insert_into(schema::users::table)
        .values(&owner)
        .on_conflict_do_nothing()
        .execute(connection);

    if let Err(err) = user_upsert_result {
        return reply::with_status(
            format!("Error creating user: {err}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        ).into_response();
    }

    let (settings, options) = models::CreatePollSettings::from(&owner.id, settings);

    let poll: models::Poll = match diesel::insert_into(schema::polls::table).values(settings).get_result(connection) {
        Err(err) => {
            return reply::with_status(
                format!("Failed to create new poll: {err}"),
                StatusCode::BAD_REQUEST,
            ).into_response();
        },
        Ok(result) => result,
    };

    let options: Vec<models::PollOption> = options.into_iter().enumerate().map(|(index, label)| {
        models::PollOption {
            id: index as i32,
            poll_id: poll.id.clone(),
            description: label,
        }
    }).collect();

    match diesel::insert_into(schema::polloptions::table).values(&options).execute(connection) {
        Err(err) => {
            return reply::with_status(
                format!("Failed to add options to poll {}: {err}", poll.id),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response();
        },
        Ok(affected) if affected != options.len() => {
            return reply::with_status(
                format!("Failed to create all {} options to poll {}", options.len(), &poll.id),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response();
        },
        Ok(_) => { },
    }

    match get_internal(connection, &poll.id) {
        Err(err) => {
            reply::with_status(
                format!("Failed to fetch poll with id {} after creating: {err:?}", &poll.id),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response()
        },
        Ok(poll) => reply::with_status(reply::json(&poll), StatusCode::CREATED).into_response(),
    }
}

pub fn get(id: Uuid) -> Response {
    let connection = &mut establish_connection();

    match get_internal(connection, &id) {
        Ok(poll) => reply::json(&poll).into_response(),
        Err(GetPollError::DbError { err }) => {
            reply::with_status(
                format!("Error fetching poll: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response()
        },
        Err(GetPollError::IdNotFound) => {
            reply::with_status(
                format!("No poll with id {id}"),
                StatusCode::NOT_FOUND,
            ).into_response()
        }
    }
}

pub fn update(id: Uuid, settings: voting::UpdatePollSettings) -> Response {
    let db_settings = models::UpdatePollSettings::from(settings);
    let connection = &mut establish_connection();
    match diesel::update(schema::polls::table.find(id))
        .set(db_settings)
        .execute(connection) {
        Err(DbError::QueryBuilderError(_)) => {
            reply::with_status(
                format!("Cannot update poll {id} without new values"),
                StatusCode::BAD_REQUEST,
            ).into_response()
        },
        Err(err) => {
            reply::with_status(
                format!("Failed to update poll with id {id}: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response()
        },
        Ok(0) => {
            reply::with_status(reply::reply(), StatusCode::NOT_FOUND).into_response()
        },
        Ok(_) => match get_internal(connection, &id) {
            Err(err) => {
                reply::with_status(
                    format!("Update successful, but failed to retrieve result: {err:?}"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ).into_response()
            },
            Ok(poll) => {
                reply::with_status(
                    reply::json(&poll),
                    StatusCode::OK,
                ).into_response()
            },
        },
    }
}

pub fn delete(id: Uuid) -> Response {
    let connection = &mut establish_connection();
    match diesel::delete(schema::polls::table.find(id)).execute(connection) {
        Err(err) => {
            reply::with_status(
                format!("Failed to delete poll with id {id}: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response()
        },
        Ok(0) => {
            reply::with_status(reply::reply(), StatusCode::NOT_FOUND).into_response()
        },
        Ok(_) => {
            reply::with_status(reply::reply(), StatusCode::NO_CONTENT).into_response()
        },
    }
}

#[derive(Debug)]
enum GetPollError {
    DbError { err: DbError },
    IdNotFound,
}

fn get_internal(connection: &mut PgConnection, id: &Uuid) -> Result<voting::Poll, GetPollError> {
    // fetch poll from db
    let possible_poll_result: Result<(models::Poll, models::User), DbError> = schema::polls::table.find(id)
        .inner_join(schema::users::table)
        .select((
            models::Poll::as_select(),
            models::User::as_select(),
        ))
        .first(connection);

    let (poll, user) = match possible_poll_result {
        Err(DbError::NotFound) => {
            return Err(GetPollError::IdNotFound);
        }
        Err(err) => {
            return Err(GetPollError::DbError { err });
        },
        Ok(r) => r,
    };

    let possible_options_result: Result<Vec<models::PollOption>, DbError> = models::PollOption::belonging_to(&poll)
        .select(models::PollOption::as_select())
        .load(connection);

    let db_options = match possible_options_result {
        Err(err) => {
            return Err(GetPollError::DbError{ err });
        },
        Ok(o) => o,
    };

    let mut poll = poll.into_voting();
    poll.owner = Some(user.into_voting());

    let mut options = vec![];
    for option in db_options.into_iter() {
        let option = option.into_voting();
        poll.option_ids.push(option.id);
        options.push(option);
    }
    poll.options = Some(options);

    Ok(poll)
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::*;
    use warp::hyper::body;

    #[tokio::test]
    async fn insert() -> Result<(), Box<dyn StdError>> {
        let res = new(voting::CreatePollSettings {
            id: None,
            title: String::from("Let's test this!"),
            options: vec![String::from("One"), String::from("Two"), String::from("Three")],
            winner_count: 2,
            write_ins_allowed: true,
            close_after_time: None,
            close_after_votes: Some(10),
        });

        let body_bytes = body::to_bytes(res.into_body()).await?;
        let res_poll: voting::Poll = serde_json::from_reader(body_bytes.as_ref())?;

        Ok(())
    }
}
