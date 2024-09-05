use chrono::{DateTime, Utc};
use serde::Serialize;
use super::id::{Id, RelativeId, WeakId};
use super::voter::Voter;

#[derive(Serialize)]
pub struct Poll<'a> {
    pub id: Id,
    pub title: &'a str,
    pub option_ids: Vec<WeakId>,

    pub winner_count: u8,
    pub write_ins_allowed: bool,
    pub close_scheduled_for: Option<DateTime<Utc>>,

    pub created_by_id: Id,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct PollOption {
    pub id: RelativeId,
    pub description: String,
}

impl<'a> Poll<'a> {
    pub fn new(
        id: Id,
        title: &'a str,
        options: Vec<String>,
        winner_count: u8,
        write_ins_allowed: bool,
        close_scheduled_for: Option<DateTime<Utc>>,
        created_by: &Voter
    ) -> (Poll<'a>, Vec<PollOption>) {
        let poll = Poll {
            id,
            title,
            option_ids: (0..(options.len() as u32)).map(|i| WeakId(i)).collect(),
            winner_count,
            write_ins_allowed,
            close_scheduled_for,
            created_by_id: created_by.id.clone(),
            created_at: Utc::now(),
            closed_at: None,
        };

        let mut full_options: Vec<PollOption> = vec![];
        for (opt_id, text) in poll.option_ids.iter().zip(options) {
            full_options.push(PollOption {
                id: RelativeId(poll.id.clone(), opt_id.clone()),
                description: text,
            });
        }

        (poll, full_options)
    }
}
