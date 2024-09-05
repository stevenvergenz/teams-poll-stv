use uuid::Uuid;
use warp::reply::Json;
use crate::voting::{Id, Poll, Voter};

pub fn get_poll(id: Uuid) -> Json{
    let (poll, _) = Poll::new(
        Id(id),
        "Test Poll",
        vec![
            String::from("Option 1"),
            String::from("Option 2")
        ],
        1,
        false,
        None,
        &Voter::new(Id::nil(), String::from("Steven")),
    );
    warp::reply::json(&poll)
}
