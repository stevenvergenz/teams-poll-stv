use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::RangeInclusive;
use std::str::FromStr;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[cfg(feature = "server")]
use diesel::result::{DatabaseErrorKind, Error as DbError};
#[cfg(feature = "server")]
use warp::http::StatusCode;
#[cfg(feature = "server")]
use warp::reply::{self, Reply, Response};

#[derive(Debug, Deserialize, Serialize)]
pub enum ContextId {
    Uuid(Uuid),
    I32(i32),
    None,
}
impl Display for ContextId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ContextId::Uuid(id) => write!(f, "{}", id),
            ContextId::I32(id) => write!(f, "{}", id),
            ContextId::None => fmt::Result::Ok(()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Context(String, ContextId);
impl Context {
    pub fn new(obj_type: &'static str, id: ContextId) -> Context {
        Context(String::from(obj_type), id)
    }
}
impl Display for Context {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let ContextId::None = self.1 {
            write!(f, "{}", self.0)
        }
        else {
            write!(f, "{} {}", self.0, self.1)
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContextError {
    message: String,
    action: Option<String>,
    context: Context,
    code: u16,
}

impl Display for ContextError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self { context, message, .. } = self;
        if let Some(action) = &self.action {
            write!(f, "Error during {action} of {context}: {message}")
        }
        else {
            write!(f, "Error for {context}: {message}")
        }
    }
}

impl FromStr for ContextError {
    type Err = serde_json::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl Error for ContextError {}

impl ContextError {
    pub fn poll_title_invalid_size(limits: RangeInclusive<usize>, len: usize) -> Self {
        Self {
            message: format!("poll's title must be between {} and {}, got {len}", limits.start(), limits.end()),
            action: Some(String::from("validation")),
            context: Context::new("poll", ContextId::None),
            code: 400,
        }
    }

    pub fn poll_option_limit_exceeded(limits: RangeInclusive<usize>, count: usize) -> Self {
        Self {
            message: format!("poll must have between {} and {} options, got {count}", limits.start(), limits.end()),
            action: Some(String::from("validation")),
            context: Context::new("poll", ContextId::None),
            code: 400,
        }
    }

    pub fn poll_winners_limit_exceeded(limits: RangeInclusive<i32>, count: i32) -> Self {
        Self {
            message: format!("poll must have between {} and {} winners, got {count}", limits.start(), limits.end()),
            action: Some(String::from("validation")),
            context: Context::new("poll", ContextId::None),
            code: 400,
        }
    }

    pub fn poll_duration_invalid(min_minutes: i32, ends: &DateTime<Utc>) -> Self {
        Self {
            message: format!("poll cannot end less than {min_minutes} minutes from now, ends {ends}"),
            action: Some(String::from("validation")),
            context: Context::new("poll", ContextId::None),
            code: 400,
        }
    }

    pub fn poll_votes_limit_exceeded(limits: RangeInclusive<i64>, count: i64) -> Self {
        Self {
            message: format!("poll cannot end without between {} and {} votes, set to end after {count}",
                limits.start(), limits.end()
            ),
            action: Some(String::from("validation")),
            context: Context::new("poll", ContextId::None),
            code: 400,
        }
    }

    pub fn ballot_empty() -> Self {
        Self {
            message: format!("ballot is empty"),
            action: Some(String::from("validation")),
            context: Context::new("ballot", ContextId::None),
            code: 400,
        }
    }

    pub fn ballot_incomplete_selection(missing_index: usize, id: ContextId) -> Self {
        Self {
            message: format!("ballot preferences have gap at index {missing_index}"),
            action: Some(String::from("validation")),
            context: Context::new("ballot", id),
            code: 400,
        }
    }

    pub fn ballot_invalid_selection(preference_index: usize, option_id: u32, id: ContextId) -> Self {
        Self {
            message: format!("ballot preference {preference_index} is for invalid poll option {option_id}"),
            action: Some(String::from("validation")),
            context: Context::new("ballot", id),
            code: 400,
        }
    }

    pub fn ballot_duplicate_selection(option_id: u32, pref_indices: (usize, usize), id: ContextId) -> Self {
        Self {
            message: format!("ballot poll option {option_id} has multiple votes at indices {pref_indices:?}"),
            action: Some(String::from("validation")),
            context: Context::new("ballot", id),
            code: 400,
        }
    }

    pub fn poll_not_found(id: &Uuid) -> Self {
        Self {
            message: format!("Not found"),
            code: 404,
            action: Some(String::from("updating")),
            context: Context::new("poll", ContextId::Uuid(*id)),
        }
    }

    #[cfg(feature = "server")]
    pub fn from_db(error: DbError, action: &str, obj_type: &'static str, id: ContextId) -> Self {
        let (message, code) = match error {
            DbError::NotFound => {
                (String::from("Not found"), 404)
            },
            DbError::DatabaseError(DatabaseErrorKind::UniqueViolation, ..) => {
                (String::from("That ID already exists"), 409)
            },
            DbError::DatabaseError(DatabaseErrorKind::NotNullViolation, ..) => {
                (String::from("Missing data"), 400)
            },
            DbError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, ..) => {
                (String::from("Missing linked data"), 400)
            },
            DbError::QueryBuilderError(_) => {
                (String::from("Missing values"), 400)
            },
            _ => (error.to_string(), 500),
        };

        Self {
            message,
            code,
            action: Some(String::from(action)),
            context: Context::new(obj_type, id),
        }
    }
}

#[cfg(feature = "server")]
impl std::convert::Into<Response> for ContextError {
    fn into(self) -> Response {
        reply::with_status(
            self.to_string(),
            StatusCode::from_u16(self.code).expect("Bad error code"),
        ).into_response()
    }
}
