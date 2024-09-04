use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug)]
pub struct Poll<'a> {
    pub id: u32,
    pub title: &'a str,
    pub option_ids: Vec<u32>,

    pub winner_count: u8,
    pub write_ins_allowed: bool,
    pub close_scheduled_for: Option<DateTime<Utc>>,

    pub created_by_id: u32,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct PollOption {
    pub id: u32,
    pub poll_id: u32,
    pub description: String,
}

impl<'a> Poll<'a> {
    pub fn new(
        id: u32,
        title: &'a str,
        options: Vec<String>,
        winner_count: u8,
        write_ins_allowed: bool,
        close_scheduled_for: Option<DateTime<Utc>>,
        created_by: &User
    ) -> (Poll<'a>, Vec<PollOption>) {
        let poll = Poll {
            id,
            title,
            option_ids: (0..(options.len() as u32)).collect(),
            winner_count,
            write_ins_allowed,
            close_scheduled_for,
            created_by_id: created_by.id,
            created_at: Utc::now(),
            closed_at: None,
        };

        let mut full_options: Vec<PollOption> = vec![];
        for (id, text) in poll.option_ids.iter().zip(options) {
            full_options.push(PollOption {
                id: *id,
                poll_id: poll.id,
                description: text,
            });
        }

        (poll, full_options)
    }
}

#[derive(Debug)]
pub struct Ballot {
    pub poll_id: u32,
    pub voter_id: u32,
    pub selection_ids: Vec<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Ballot {
    pub fn new(poll: &Poll, voter: &User, selections: Vec<u32>) -> Ballot {
        Ballot {
            poll_id: poll.id,
            voter_id: voter.id,
            selection_ids: selections,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug)]
pub struct User<'a> {
    pub id: u32,
    pub display_name: &'a str,
}

impl<'a> User<'a> {
    pub const fn new(id: u32, display_name: &'a str) -> User<'a> {
        User {
            id,
            display_name,
        }
    }
}
