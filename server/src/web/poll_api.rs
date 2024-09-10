use diesel::prelude::*;
use diesel::result::Error as DbError;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{self, Reply, Response};

use crate::voting;
use super::db::{establish_connection, models, schema};

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
    let mut query = diesel::update(schema::polls::table.find(id));
    if let Some(title) = settings.title {
        query.set(schema::polls::title.eq(title));
    }

    let connection = &mut establish_connection();
}

pub fn delete(_id: Uuid) -> Response {
    todo!()
}

#[derive(Debug)]
enum GetPollError {
    DbError { err: DbError },
    IdNotFound,
}

fn get_internal(connection: &mut PgConnection, id: &Uuid) -> Result<voting::Poll, GetPollError> {
    // fetch poll from db
    let possible_poll_result: Result<(models::Poll, models::User), DbError> = schema::polls::table
        .inner_join(schema::users::table)
        .filter(schema::polls::id.eq(id))
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
            return Err(GetPollError::DbError{ err: err });
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
