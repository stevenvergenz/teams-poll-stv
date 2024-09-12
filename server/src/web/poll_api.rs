use std::collections::HashMap;
use std::error::Error as StdError;

use diesel::prelude::*;
use diesel::result::Error as DbError;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{self, Reply, Response};

use crate::voting;
use super::db::{establish_connection, models, schema};
use crate::error;

pub fn list() -> Response {
    let connection = &mut establish_connection();
    let possible_poll_results: Result<Vec<(models::Poll, models::User)>, DbError> = schema::polls::table
        .inner_join(schema::users::table)
        .select((models::Poll::as_select(), models::User::as_select()))
        .limit(100)
        .load(connection);

    let mut polls_lookup: HashMap<Uuid, voting::Poll> = HashMap::new();
    match possible_poll_results {
        Err(err) => {
            return reply::with_status(
                format!("Failed to get polls: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response();
        },
        Ok(polls) => {
            for (poll, user) in polls.into_iter() {
                let mut poll: voting::Poll = match poll.try_into() {
                    Err(err) => {
                        return reply::with_status(
                            format!("Failed to resolve poll owners: {err}"),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        ).into_response();
                    },
                    Ok(poll) => poll,
                };
                poll.owner = Some(user.into());
                polls_lookup.insert(poll.id.0, poll);
            }
        }
    }

    let possible_option_results: Result<Vec<models::PollOption>, DbError> = schema::polloptions::table
        .filter(schema::polloptions::poll_id.eq_any(polls_lookup.keys()))
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
        if let Some(poll) = polls_lookup.get_mut(&option.poll_id) {
            let option: voting::PollOption = option.into();
            poll.option_ids.push(option.id);
            match &mut poll.options {
                None => { poll.options = Some(vec![option]); },
                Some(vec) => { vec.push(option); },
            }
        }
    }

    reply::json(&polls_lookup.values().collect::<Vec<&voting::Poll>>()).into_response()

}

pub fn new(user_id: Uuid, settings: voting::CreatePollSettings) -> Response {
    let connection = &mut establish_connection();

    // todo: get owner = session user
    let owner = models::User { id: user_id, display_name: String::from("Anonymous") };
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
        Err(err @ error::HttpGetError { code, .. }) => {
            reply::with_status(err.to_string(), code).into_response()
        },
    }
}

pub fn update(poll_id: Uuid, user_id: Uuid, settings: voting::UpdatePollSettings) -> Response {
    let settings = models::UpdatePollSettings::from(settings);

    let connection = &mut establish_connection();
    let update = diesel::update(
        schema::polls::table.filter(
            schema::polls::id.eq(poll_id)
            .and(schema::polls::owner_id.eq(user_id))
        )
    ).set(settings).execute(connection);

    match update {
        Err(DbError::QueryBuilderError(_)) => {
            reply::with_status(
                format!("Cannot update poll {poll_id} without new values"),
                StatusCode::BAD_REQUEST,
            ).into_response()
        },
        Err(err) => {
            reply::with_status(
                format!("Failed to update poll with id {poll_id}: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response()
        },
        Ok(0) => {
            reply::with_status(reply::reply(), StatusCode::FORBIDDEN).into_response()
        },
        Ok(_) => match get_internal(connection, &poll_id) {
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

pub fn delete(poll_id: Uuid, user_id: Uuid) -> Response {
    let connection = &mut establish_connection();
    let delete = diesel::delete(
        schema::polls::table.filter(
            schema::polls::id.eq(poll_id)
            .and(schema::polls::owner_id.eq(user_id))
        ),
    ).execute(connection);

    match delete {
        Err(err) => {
            reply::with_status(
                format!("Failed to delete poll with id {poll_id}: {err}"),
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

fn get_internal(connection: &mut PgConnection, id: &Uuid) -> Result<voting::Poll, error::HttpGetError> {
    // fetch poll from db
    let possible_poll_result: Result<(models::Poll, models::User), DbError> = schema::polls::table.find(id)
        .inner_join(schema::users::table)
        .select((
            models::Poll::as_select(),
            models::User::as_select(),
        ))
        .first(connection);

    let (db_poll, db_user) = match possible_poll_result {
        Err(DbError::NotFound) => {
            return Err(error::db_get(DbError::NotFound, StatusCode::NOT_FOUND, "poll/owner", None));
        }
        Err(err) => {
            return Err(error::db_get(err, StatusCode::INTERNAL_SERVER_ERROR, "poll/owner", None));
        },
        Ok(r) => r,
    };

    let possible_options_result: Result<Vec<models::PollOption>, DbError> = models::PollOption::belonging_to(&db_poll)
        .select(models::PollOption::as_select())
        .load(connection);

    let db_options = match possible_options_result {
        Err(err) => {
            return Err(error::db_get(err, StatusCode::INTERNAL_SERVER_ERROR, "option", Some("poll")));
        },
        Ok(o) => o,
    };

    let mut poll: voting::Poll = match db_poll.try_into() {
        Err(err) => return Err(error::HttpGetError::from(err)),
        Ok(p) => p,
    };
    poll.owner = Some(db_user.into());

    let mut options = vec![];
    for db_option in db_options.into_iter() {
        let option: voting::PollOption = db_option.into();
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

    async fn setup(settings: &voting::CreatePollSettings) -> Result<voting::Poll, Box<dyn StdError>> {
        let res = new(Uuid::nil(), voting::CreatePollSettings::from(settings.clone()));
        let res_bytes = body::to_bytes(res.into_body()).await?;
        let res_poll: voting::Poll = serde_json::from_reader(res_bytes.as_ref())?;

        Ok(res_poll)
    }

    async fn teardown(poll: voting::Poll) -> Result<(), Box<dyn StdError>> {
        let res = delete(poll.id.0, poll.owner_id.0);
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
        Ok(())
    }

    #[tokio::test]
    async fn create_delete() -> Result<(), Box<dyn StdError>> {
        let req = voting::CreatePollSettings {
            title: String::from("Basic crud test"),
            ..voting::CreatePollSettings::default()
        };
        let poll = setup(&req).await?;

        assert_eq!(req.title, poll.title);
        assert_eq!(req.options.len(), poll.option_ids.len());
        assert!(poll.options.is_some());

        for (i, option) in poll.options.as_ref().unwrap().iter().enumerate() {
            assert_eq!(poll.option_ids[i].0, i as u32);
            assert_eq!(option.id.0, i as u32);
            assert_eq!(option.description, req.options[i]);
        }

        teardown(poll).await?;
        Ok(())
    }
}
