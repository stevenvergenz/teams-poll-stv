use diesel::prelude::*;
use diesel::result::Error as DbError;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{self, Reply, Response};

use crate::voting;
use super::db::{establish_connection, models, schema};
use crate::error;

pub fn new(user_id: Uuid, settings: voting::CreatePollSettings) -> Response {
    let connection = &mut establish_connection();

    // todo: get owner = session user
    let owner = models::User { id: user_id, display_name: String::from("Anonymous") };
    let (settings, options) = models::CreatePollSettings::from(&owner.id, settings);
    let mut options: Vec<models::PollOption> = options.into_iter().enumerate().map(|(index, label)| {
        models::PollOption {
            id: index as i32,
            poll_id: Uuid::nil(),
            description: label,
        }
    }).collect();

    let result: Result<models::Poll, DbError> = connection.transaction(|connection| {
        diesel::insert_into(schema::users::table)
            .values(&owner)
            .on_conflict_do_nothing()
            .execute(connection)?;

        let poll: models::Poll = diesel::insert_into(schema::polls::table)
            .values(settings)
            .get_result(connection)?;
        println!("New poll: {}", poll.id);

        for option in options.iter_mut() {
            option.poll_id = poll.id;
        }

        diesel::insert_into(schema::polloptions::table).values(&options).execute(connection)?;

        Ok(poll)
    });

    let poll = match result {
        Err(err) => {
            return error::db_insert(err, "poll").into_response();
        },
        Ok(p) => p,
    };

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
        Err(err) => err.into_response(),
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

pub fn get_internal(connection: &mut PgConnection, id: &Uuid) -> Result<voting::Poll, error::HttpGetError> {
    // fetch poll from db
    let poll_result: Result<(models::Poll, models::User), DbError> = schema::polls::table.find(id)
        .inner_join(schema::users::table)
        .select((
            models::Poll::as_select(),
            models::User::as_select(),
        ))
        .first(connection);

    let (db_poll, db_user) = match poll_result {
        Err(err @ DbError::NotFound) => {
            return Err(error::db_get(err, StatusCode::NOT_FOUND, "poll/owner", None));
        }
        Err(err) => {
            return Err(error::db_get(err, StatusCode::INTERNAL_SERVER_ERROR, "poll/owner", None));
        },
        Ok(r) => r,
    };

    let options_result: Result<Vec<models::PollOption>, DbError> = models::PollOption::belonging_to(&db_poll)
        .select(models::PollOption::as_select())
        .load(connection);

    let db_options = match options_result {
        Err(err) => {
            return Err(error::db_get(err, StatusCode::INTERNAL_SERVER_ERROR, "option", Some("poll")));
        },
        Ok(o) => o,
    };

    let poll: voting::Poll = match (db_poll, db_options, db_user).try_into() {
        Err(err) => return Err(error::HttpGetError::from(err)),
        Ok(p) => p,
    };

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
