use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::web::schema::polls)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Poll {
    pub id: Uuid,
    pub title: String,

    pub winner_count: i32,
    pub write_ins_allowed: bool,
    pub close_after_time: Option<NaiveDateTime>,
    pub close_after_votes: Option<i32>,

    pub owner_id: Uuid,
    pub created_at: NaiveDateTime,
    pub closed_at: Option<NaiveDateTime>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::web::schema::polloptions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PollOption {
    pub poll_id: Uuid,
    pub id: i32,
    pub description: String,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::web::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,
    pub display_name: String,
}
