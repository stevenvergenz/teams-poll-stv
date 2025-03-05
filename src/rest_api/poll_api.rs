use dioxus::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    error::{ContextError, ContextId},
    voting,
};

#[cfg(feature = "server")]
use diesel::prelude::*;
#[cfg(feature = "server")]
use diesel::result::Error as DbError;
#[cfg(feature = "server")]
use warp::{
    http::StatusCode,
    reply::{self, Reply, Response},
};
#[cfg(feature = "server")]
use super::db::{establish_connection, models, schema};

#[cfg(feature = "server")]
fn list() -> Result<Vec<voting::Poll>, ContextError> {
    let connection = &mut establish_connection();
    let results: Result<Vec<(models::Poll, models::User)>, DbError> = schema::polls::table
        .inner_join(schema::users::table)
        .inner_join(schema::polloptions::table)
        .select((
            models::Poll::as_select(),
            models::User::as_select(),
        ))
        .load(connection);

    match results {
        Err(e) => {
            Err(ContextError::from_db(e, "listing", "polls", ContextId::None))
        },
        Ok(polls_users) => {
            Ok(polls_users.into_iter().map(|(p, u)| p.into(u, vec![])).collect())
        },
    }
}

#[cfg(feature = "server")]
fn new(user_id: Uuid, settings: voting::CreatePollSettings) -> Result<voting::Poll, ContextError> {
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

    let result = connection.transaction(|connection| {
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
            return Err(ContextError::from_db(err, "creating", "poll", ContextId::None));
        },
        Ok(p) => p,
    };

    get(connection, &poll.id)
}

#[cfg(feature = "server")]
pub fn get(connection: &mut PgConnection, id: &Uuid) -> Result<voting::Poll, ContextError> {
    // fetch poll from db
    let result: Result<(models::Poll, models::User), DbError> = schema::polls::table.find(id)
        .inner_join(schema::users::table)
        .select((
            models::Poll::as_select(),
            models::User::as_select(),
        ))
        .first(connection);

    let (poll, user) = match result {
        Err(err) => {
            return Err(ContextError::from_db(err, "fetching", "poll", ContextId::Uuid(*id)));
        },
        Ok(pu) => pu,
    };

    let result: Result<Vec<models::PollOption>, DbError> = models::PollOption::belonging_to(&poll)
        .select(models::PollOption::as_select())
        .load(connection);

    let options = match result {
        Err(e) => {
            return Err(ContextError::from_db(e, "fetching", "options", ContextId::Uuid(*id)));
        },
        Ok(o) => o,
    };

    Ok(poll.into(user, options))
}

#[cfg(feature = "server")]
fn update(poll_id: Uuid, user_id: Uuid, settings: voting::UpdatePollSettings) -> Result<voting::Poll, ContextError> {
    let settings = models::UpdatePollSettings::from(settings);

    let connection = &mut establish_connection();
    let update = diesel::update(
        schema::polls::table.filter(
            schema::polls::id.eq(poll_id)
                .and(schema::polls::owner_id.eq(user_id))))
        .set(settings)
        .execute(connection);

    match update {
        Err(e) => {
            return Err(ContextError::from_db(e, "updating", "poll", ContextId::Uuid(poll_id)));
        },
        Ok(0) => {
            return Err(ContextError::poll_not_found(&poll_id));
        },
        Ok(_) => get(&mut establish_connection(), &poll_id),
    }
}

#[cfg(feature = "server")]
fn delete(poll_id: Uuid, user_id: Uuid) -> Result<(), ContextError> {
    let connection = &mut establish_connection();
    let delete = diesel::delete(
        schema::polls::table.filter(
            schema::polls::id.eq(poll_id)
            .and(schema::polls::owner_id.eq(user_id))
        ),
    ).execute(connection);

    match delete {
        Err(err) => {
            Err(ContextError::from_db(err, "deleting", "poll", ContextId::Uuid(poll_id)))
        },
        Ok(0) => {
            Err(ContextError::poll_not_found(&poll_id))
        },
        Ok(_) => {
            Ok(())
        },
    }
}

#[server]
pub async fn list_ssr() -> Result<Vec<voting::Poll>, ServerFnError<ContextError>> {
    match list() {
        Ok(polls) => Ok(polls),
        Err(err) => Err(ServerFnError::WrappedServerError(err)),
    }
}

#[server]
pub async fn new_ssr(
    settings: voting::UnvalidatedCreatePollSettings,
) -> Result<voting::Poll, ServerFnError<ContextError>> {
    let user_id = Uuid::nil(); // todo: get from session
    match new(user_id, settings.try_into()?) {
        Ok(poll) => Ok(poll),
        Err(err) => Err(ServerFnError::WrappedServerError(err)),
    }
}

#[server]
pub async fn get_ssr(id: Uuid) -> Result<voting::Poll, ServerFnError<ContextError>> {
    let connection = &mut establish_connection();
    match get(connection, &id) {
        Ok(poll) => Ok(poll),
        Err(err) => Err(ServerFnError::WrappedServerError(err)),
    }
}

#[server]
pub async fn update_ssr(
    poll_id: Uuid,
    user_id: Uuid,
    settings: voting::UnvalidatedUpdatePollSettings,
) -> Result<voting::Poll, ServerFnError<ContextError>> {
    match update(poll_id, user_id, settings.try_into()?) {
        Ok(poll) => Ok(poll),
        Err(err) => Err(ServerFnError::WrappedServerError(err)),
    }
}

#[server]
pub async fn delete_ssr(poll_id: Uuid, user_id: Uuid) -> Result<(), ServerFnError<ContextError>> {
    match delete(poll_id, user_id) {
        Ok(_) => Ok(()),
        Err(err) => Err(ServerFnError::WrappedServerError(err)),
    }
}

#[cfg(feature = "server")]
pub fn new_api(user_id: Uuid, settings: voting::CreatePollSettings) -> Response {
    match new(user_id, settings) {
        Err(err) => err.into(),
        Ok(p) => reply::with_status(reply::json(&p), StatusCode::CREATED).into_response(),
    }
}

#[cfg(feature = "server")]
pub fn get_api(id: Uuid) -> Response {
    let connection = &mut establish_connection();
    match get(connection, &id) {
        Err(err) => err.into(),
        Ok(poll) => reply::json(&poll).into_response(),
    }
}

#[cfg(feature = "server")]
pub fn update_api(poll_id: Uuid, user_id: Uuid, settings: voting::UpdatePollSettings) -> Response {
    match update(poll_id, user_id, settings) {
        Err(err) => err.into(),
        Ok(p) => reply::json(&p).into_response(),
    }
}

#[cfg(feature = "server")]
pub fn delete_api(poll_id: Uuid, user_id: Uuid) -> Response {
    match delete(poll_id, user_id) {
        Err(err) => err.into(),
        Ok(_) => reply::with_status(reply::reply(), StatusCode::NO_CONTENT).into_response(),
    }
}


#[cfg(all(test, feature = "server"))]
mod tests {
    use std::error::Error as StdError;

    use super::*;
    use warp::hyper::body;

    async fn setup(settings: &voting::CreatePollSettings) -> Result<voting::Poll, Box<dyn StdError>> {
        let res = new_api(Uuid::nil(), voting::CreatePollSettings::from(settings.clone()));
        let res_bytes = body::to_bytes(res.into_body()).await?;
        let res_poll: voting::Poll = serde_json::from_reader(res_bytes.as_ref())?;

        Ok(res_poll)
    }

    async fn teardown(poll: voting::Poll) -> Result<(), Box<dyn StdError>> {
        let res = delete_api(poll.id.0, poll.owner_id.0);
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
