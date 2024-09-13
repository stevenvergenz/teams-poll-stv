use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::RangeInclusive;
use std::convert::From;

use chrono::{DateTime, Utc};
use diesel::result::Error as DbError;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{self, Reply};

#[derive(Debug)]
pub enum ContextId {
    Uuid(Uuid),
    I32(i32),
}
impl Display for ContextId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ContextId::Uuid(id) => write!(f, "{}", id),
            ContextId::I32(id) => write!(f, "{}", id),
        }
    }
}

#[derive(Debug)]
struct Context(&'static str, ContextId);
impl Context {
    pub fn new(obj_type: &'static str, id: ContextId) -> Context {
        Context(obj_type, id)
    }
}
impl Display for Context {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0, self.1)
    }
}

#[derive(Debug)]
pub struct ValidationError {
    message: String,
    context: Option<Context>,
}

impl ValidationError {
    pub fn with_context(mut self, obj_type: &'static str, id: ContextId) -> Self {
        self.context = Some(Context::new(obj_type, id));
        self
    }
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(context) = &self.context {
            write!(f, "Validation error for {}: {}", context, self.message)
        }
        else {
            write!(f, "Validation error: {}", self.message)
        }
    }
}

impl Error for ValidationError {}

pub fn poll_title_invalid_size(limits: RangeInclusive<usize>, len: usize) -> ValidationError {
    ValidationError {
        message: format!("poll's title must be between {} and {}, got {len}", limits.start(), limits.end()),
        context: None,
    }
}

pub fn poll_option_limit_exceeded(limits: RangeInclusive<usize>, count: usize) -> ValidationError {
    ValidationError {
        message: format!("poll must have between {} and {} options, got {count}", limits.start(), limits.end()),
        context: None,
    }
}

pub fn poll_winners_limit_exceeded(limits: RangeInclusive<i32>, count: i32) -> ValidationError {
    ValidationError {
        message: format!("poll must have between {} and {} winners, got {count}", limits.start(), limits.end()),
        context: None,
    }
}

pub fn poll_duration_invalid(min_minutes: i32, ends: &DateTime<Utc>) -> ValidationError {
    ValidationError {
        message: format!("poll cannot end less than {min_minutes} minutes from now, ends {ends}"),
        context: None,
    }
}

pub fn poll_votes_limit_exceeded(limits: RangeInclusive<i64>, count: i64) -> ValidationError {
    ValidationError {
        message: format!("poll cannot end without between {} and {} votes, set to end after {count}",
            limits.start(), limits.end()
        ),
        context: None,
    }
}

pub fn ballot_empty() -> ValidationError {
    ValidationError {
        message: format!("ballot is empty"),
        context: None,
    }
}

pub fn ballot_incomplete_selection(missing_index: usize) -> ValidationError {
    ValidationError {
        message: format!("ballot preferences have gap at index {missing_index}"),
        context: None,
    }
}

pub fn ballot_invalid_selection(preference_index: usize, option_id: u32) -> ValidationError {
    ValidationError {
        message: format!("ballot preference {preference_index} is for invalid poll option {option_id}"),
        context: None,
    }
}

pub fn ballot_duplicate_selection(option_id: u32, pref_indices: (usize, usize)) -> ValidationError {
    ValidationError {
        message: format!("ballot poll option {option_id} has multiple votes at indices {pref_indices:?}"),
        context: None,
    }
}


#[derive(Debug)]
pub struct HttpGetError {
    pub code: StatusCode,
    message: String,
    source: Option<DbError>,
}

impl HttpGetError {
    pub fn into_response(self) -> reply::Response {
        if self.code == StatusCode::NOT_FOUND {
            reply::with_status(reply::reply(), self.code).into_response()
        }
        else {
            reply::with_status(self.to_string(), self.code).into_response()
        }
    }
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
