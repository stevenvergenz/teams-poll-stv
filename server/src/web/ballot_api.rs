use uuid::Uuid;
use warp::reply::{self, Response};

use super::super::voting;

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
