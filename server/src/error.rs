use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::RangeInclusive;
use std::convert::From;

use chrono::{DateTime, Utc};
use diesel::result::Error as DbError;
use uuid::Uuid;
use warp::http::StatusCode;

#[derive(Debug)]
pub struct ValidationError {
    message: String,
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Validation error: {}", self.message)
    }
}

impl Error for ValidationError {}

pub fn poll_title_invalid_size(limits: RangeInclusive<usize>, len: usize) -> ValidationError {
    ValidationError {
        message: format!("poll's title must be between {} and {}, got {len}", limits.start(), limits.end()),
    }
}

pub fn poll_option_limit_exceeded(limits: RangeInclusive<usize>, count: usize) -> ValidationError {
    ValidationError {
        message: format!("poll must have between {} and {} options, got {count}", limits.start(), limits.end()),
    }
}

pub fn poll_winners_limit_exceeded(limits: RangeInclusive<i32>, count: i32) -> ValidationError {
    ValidationError {
        message: format!("poll must have between {} and {} winners, got {count}", limits.start(), limits.end()),
    }
}

pub fn poll_duration_invalid(min_minutes: i32, ends: &DateTime<Utc>) -> ValidationError {
    ValidationError {
        message: format!("poll cannot end less than {min_minutes} minutes from now, ends {ends}"),
    }
}

pub fn poll_votes_limit_exceeded(limits: RangeInclusive<i64>, count: i64) -> ValidationError {
    ValidationError {
        message: format!("poll cannot end without between {} and {} votes, set to end after {count}",
            limits.start(), limits.end()
        ),
    }
}

pub fn ballot_voter_mismatch(id: i32, expected: &Uuid, actual: &Uuid) -> ValidationError {
    ValidationError {
        message: format!("ballot {id} expected voter to be {expected}, got {actual}"),
    }
}
pub fn ballot_poll_mismatch(id: i32, expected: &Uuid, actual: &Uuid) -> ValidationError {
    ValidationError {
        message: format!("ballot {id} expected poll to be {expected}, got {actual}"),
    }
}
pub fn ballot_empty() -> ValidationError {
    ValidationError {
        message: format!("ballot is empty"),
    }
}
pub fn ballot_incomplete_selection(id: i32, missing_index: usize) -> ValidationError {
    ValidationError {
        message: format!("ballot {id} preferences have gap at index {missing_index}"),
    }
}
pub fn ballot_invalid_selection(id: i32, preference_index: usize, option_id: i32) -> ValidationError {
    ValidationError {
        message: format!("ballot {id} preference {preference_index} is for invalid poll option {option_id}"),
    }
}
pub fn ballot_duplicate_selection(id: i32, option_id: i32, pref_indices: (usize, usize)) -> ValidationError {
    ValidationError {
        message: format!("ballot {id} poll option {option_id} has multiple votes at indices {pref_indices:?}"),
    }
}


#[derive(Debug)]
pub struct HttpGetError {
    pub code: StatusCode,
    message: String,
    source: Option<DbError>,
}

impl Display for HttpGetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:?}", self.message, self.source)
    }
}

impl Error for HttpGetError { }

impl From<ValidationError> for HttpGetError {
    fn from(value: ValidationError) -> Self {
        HttpGetError {
            message: value.to_string(),
            code: StatusCode::BAD_REQUEST,
            source: None,
        }
    }
}

pub fn db_get(source: DbError, code: StatusCode, subject: &str, object: Option<&str>) -> HttpGetError {
    let message = match object {
        Some(object) => format!("Failed to retrieve {object} of {subject}"),
        None => format!("Failed to retrieve {subject}"),
    };
    HttpGetError { message, code, source: Some(source) }
}
