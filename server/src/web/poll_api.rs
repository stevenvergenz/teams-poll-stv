use diesel::prelude::*;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{self, Reply, Response, WithStatus};

use crate::voting::{CreatePollSettings, Id, Poll, User};
use super::db::establish_connection;

pub fn new_poll(options: CreatePollSettings) -> WithStatus<&'static str> {
    reply::with_status("Created", StatusCode::CREATED)
}

pub fn get_poll(id: Uuid) -> Response {
    // let owner = User::new(Id::new(), String::from("Steven"));
    // match Poll::new(owner, CreatePollSettings {
    //     id: Some(id),
    //     title: String::from("Test Poll"),
    //     options: vec![
    //         String::from("Option 1"),
    //         String::from("Option 2")
    //     ],
    //     winner_count: 1,
    //     write_ins_allowed: false,
    //     close_after_time: None,
    //     close_after_votes: None,
    // }) {
    //     Ok(poll) => {
    //         reply::json(&poll).into_response()
    //     },
    //     Err(err) => {
    //         reply::with_status(err.to_string(), StatusCode::INTERNAL_SERVER_ERROR).into_response()
    //     },
    // }

    // use self::schema::posts::dsl::*;

    // let connection = &mut establish_connection();
    // let results = posts
    //     .filter(published.eq(true))
    //     .limit(5)
    //     .select(Post::as_select())
    //     .load(connection)
    //     .expect("Error loading posts");

    // println!("Displaying {} posts", results.len());
    // for post in results {
    //     println!("{}", post.title);
    //     println!("-----------\n");
    //     println!("{}", post.body);
    // }

    // use super::schema::polls::dsl::*;
    use super::schema::polls as Polls;
    use super::models::Poll as PollModel;

    let connection = &mut establish_connection();
    let results = Polls::table
        .filter(Polls::id.eq(id))
        .select(PollModel::as_select())
        .load(connection)
        .expect("Error loading poll");

    if results.len() == 0 {
        return reply::with_status("No poll found", StatusCode::NOT_FOUND).into_response();
    }

    // todo: fetch options

    // todo: convert to poll object instead of db model

    reply::json(results.iter().next().unwrap()).into_response()
}
