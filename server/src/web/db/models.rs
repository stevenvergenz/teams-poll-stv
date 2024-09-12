use std::convert::TryInto;

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::voting;
use crate::error;
use super::schema;

#[derive(Associations, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(table_name = schema::polls)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(User, foreign_key = owner_id))]
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

impl TryInto<voting::Poll> for Poll {
    type Error = error::ValidationError;
    fn try_into(self) -> Result<voting::Poll, Self::Error> {
        let Self {
            id,
            title,
            winner_count,
            write_ins_allowed,
            close_after_time,
            close_after_votes,
            owner_id,
            created_at,
            closed_at,
        } = self;

        // re-validate timeless settings
        let settings = voting::UnvalidatedCreatePollSettings {
            title, winner_count, write_ins_allowed, close_after_votes,
            ..voting::UnvalidatedCreatePollSettings::from(voting::CreatePollSettings::default())
        };
        let settings = voting::CreatePollSettings::try_from(settings)?;

        // straight-up copy unvalidated, generated, or timely elements
        let mut poll = voting::Poll::from(settings);
        poll.id = voting::Id(id);
        poll.close_after_time = close_after_time.map(|t| t.and_utc());
        poll.owner_id = voting::Id(owner_id);
        poll.created_at = created_at.and_utc();
        poll.closed_at = closed_at.map(|t| t.and_utc());

        Ok(poll)
    }
}

#[derive(Insertable)]
#[diesel(table_name = schema::polls)]
pub struct CreatePollSettings {
    pub id: Option<Uuid>,
    pub title: String,

    pub winner_count: i32,
    pub write_ins_allowed: bool,
    pub close_after_time: Option<NaiveDateTime>,
    pub close_after_votes: Option<i32>,

    pub owner_id: Uuid,
}

impl CreatePollSettings {
    pub fn from(owner_id: &Uuid, voting::CreatePollSettings {
        id: _,
        title,
        options,
        winner_count,
        write_ins_allowed,
        close_after_time,
        close_after_votes,
    }: voting::CreatePollSettings) -> (Self, Vec<String>) {
        let poll_settings = Self {
            id: None, // discard any ID provided as input, force random ID from DB
            title,
            winner_count: winner_count as i32,
            write_ins_allowed,
            close_after_time: close_after_time.map(|t| t.naive_utc()),
            close_after_votes: close_after_votes.map(|v| v as i32),
            owner_id: owner_id.clone(),
        };

        (poll_settings, options)
    }
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = schema::polls)]
pub struct UpdatePollSettings {
    pub title: Option<String>,
    pub winner_count: Option<i32>,
    pub write_ins_allowed: Option<bool>,
    pub close_after_time: Option<Option<NaiveDateTime>>,
    pub close_after_votes: Option<Option<i32>>,
}

impl From<voting::UpdatePollSettings> for UpdatePollSettings {
    fn from(voting::UpdatePollSettings {
        title,
        winner_count,
        write_ins_allowed,
        close_after_time,
        close_after_votes,
    }: voting::UpdatePollSettings) -> Self {
        Self {
            title,
            winner_count: winner_count.map(|x| x as i32),
            write_ins_allowed,
            close_after_time: close_after_time.map(|odt| {
                odt.map(|dt| {
                    dt.naive_utc()
                })
            }),
            close_after_votes: close_after_votes.map(|ox| {
                ox.map(|x| x as i32)
            }),
        }
    }
}

#[derive(Associations, Identifiable, Queryable, Selectable, Insertable)]
#[diesel(table_name = schema::polloptions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(Poll))]
pub struct PollOption {
    pub poll_id: Uuid,
    pub id: i32,
    pub description: String,
}

impl Into<voting::PollOption> for PollOption {
    fn into(self) -> voting::PollOption {
        voting::PollOption {
            id: voting::WeakId(self.id as u32),
            description: self.description,
        }
    }
}

#[derive(Queryable, Selectable, Identifiable, Insertable)]
#[diesel(table_name = schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,
    pub display_name: String,
}

impl Into<voting::User> for User {
    fn into(self) -> voting::User {
        voting::User {
            id: voting::Id(self.id),
            display_name: self.display_name,
        }
    }
}

#[derive(Associations, Queryable, Selectable, Identifiable, Insertable)]
#[diesel(table_name = schema::ballots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(belongs_to(Poll, foreign_key = poll_id))]
pub struct Ballot {
    pub id: i32,
    pub poll_id: Uuid,
    pub user_id: Uuid,
    pub created_at: NaiveDateTime,
}

impl TryInto<voting::Ballot> for Ballot {
    fn try_into(self, poll: voting::Poll, voter: voting::User, votes: Vec<Vote>) -> Result<voting::Ballot, error::ValidationError> {
        if self.user_id != voter.id.0 {
            return Err(BallotValidationError::new_voter_mismatch(self.id, &self.user_id, &voter.id.0));
        }
        if self.poll_id != poll.id.0 {
            return Err(BallotValidationError::new_poll_mismatch(self.id, &self.poll_id, &poll.id.0));
        }

        let mut ranked_prefs = vec![];
        for i in 0..votes.len() {
            let ov = votes.iter().find(|v| v.preference == i as i32);
            if let Some(v) = ov {
                let option_id = voting::WeakId(v.option as u32);
                if !poll.option_ids.contains(&option_id) {
                    return Err(BallotValidationError::new_invalid_selection(self.id, i, v.option));
                }
                ranked_prefs.push(option_id);
            }
            else {
                return Err(BallotValidationError::new_incomplete_selection(self.id, i));
            }
        }

        Ok(voting::Ballot {
            poll: Some(poll),
            voter: Some(voter),
            ranked_preferences: ranked_prefs,
            created_at: self.created_at.and_utc(),
        })
    }
}

#[derive(Associations, Queryable, Selectable, Insertable)]
#[diesel(table_name = schema::votes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(Ballot, foreign_key = ballot_id))]
pub struct Vote {
    pub ballot_id: i32,
    pub option: i32,
    pub preference: i32,
}
