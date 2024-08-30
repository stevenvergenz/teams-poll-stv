use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct Poll {
    pub id: Uuid,
    pub title: String,
    pub option_ids: Vec<Uuid>,

    pub winner_count: u8,
    pub write_ins_allowed: bool,
    pub close_scheduled_for: Option<DateTime<Utc>>,

    pub created_by_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PollOption {
    pub id: Uuid,
    pub poll_id: Uuid,
    pub description: String,
}

impl Poll {
    pub fn new(
        title: &str,
        options: &[&str],
        winner_count: u8,
        write_ins_allowed: bool,
        close_scheduled_for: Option<DateTime<Utc>>,
        created_by: User
    ) -> (Poll, Vec<PollOption>) {
        let mut poll = Poll {
            id: Uuid::new_v4(),
            title: String::from(title),
            option_ids: options.iter().map(|_| Uuid::new_v4()).collect(),
            winner_count,
            write_ins_allowed,
            close_scheduled_for,
            created_by_id: created_by.id,
            created_at: Utc::now(),
            closed_at: None,
        };

        let mut full_options: Vec<PollOption> = vec![];
        for text in options.iter() {
            full_options.push(PollOption {
                id: Uuid::new_v4(),
                poll_id: poll.id,
                description: String::from(*text),
            });
        }

        (poll, full_options)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ballot {
    pub poll_id: Uuid,
    pub voter_id: Uuid,
    pub selection_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Ballot {
    pub fn new(poll: &Poll, voter: &User, selections: Vec<Uuid>) -> Ballot {
        Ballot {
            poll_id: poll.id,
            voter_id: voter.id,
            selection_ids: selections,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: Uuid,
    pub federated_id: String,
    pub display_name: String,
}

impl User {
    pub fn new(federated_id: &str, display_name: &str) -> User {
        User {
            id: Uuid::new_v4(),
            federated_id: String::from(federated_id),
            display_name: String::from(display_name),
        }
    }
}
