//! Unified error type. Every fallible path in the framework returns
//! `Result<T, Error>`, and the HTTP layer knows how to turn an `Error`
//! into a proper response.

use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    MethodNotAllowed(String),
    Conflict(String),
    Internal(String),
}

impl Error {
    pub fn status(&self) -> u16 {
        match self {
            Error::BadRequest(_) => 400,
            Error::Unauthorized(_) => 401,
            Error::Forbidden(_) => 403,
            Error::NotFound(_) => 404,
            Error::MethodNotAllowed(_) => 405,
            Error::Conflict(_) => 409,
            Error::Internal(_) => 500,
        }
    }

    /// The message as shown to the client. For 500s we deliberately
    /// return a generic string — the real detail stays in logs.
    pub fn client_message(&self) -> &str {
        match self {
            Error::BadRequest(m)
            | Error::Unauthorized(m)
            | Error::Forbidden(m)
            | Error::NotFound(m)
            | Error::MethodNotAllowed(m)
            | Error::Conflict(m) => m,
            Error::Internal(_) => "Internal Server Error",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Error::BadRequest(m) => format!("400 Bad Request: {m}"),
            Error::Unauthorized(m) => format!("401 Unauthorized: {m}"),
            Error::Forbidden(m) => format!("403 Forbidden: {m}"),
            Error::NotFound(m) => format!("404 Not Found: {m}"),
            Error::MethodNotAllowed(m) => format!("405 Method Not Allowed: {m}"),
            Error::Conflict(m) => format!("409 Conflict: {m}"),
            Error::Internal(m) => format!("500 Internal: {m}"),
        };
        f.write_str(&label)
    }
}

impl std::error::Error for Error {}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => Error::NotFound("row not found".into()),
            // Postgres constraint violations (FK / unique / NOT NULL /
            // check) are user-input bugs, not internal failures. Surface
            // them as 409 Conflict so callers can distinguish "the user
            // picked a bad value" from "the DB is on fire".
            sqlx::Error::Database(db_err) if db_err.constraint().is_some() => {
                let constraint = db_err.constraint().unwrap_or("?").to_string();
                Error::Conflict(format!("constraint violation ({constraint}): {db_err}"))
            }
            other => Error::Internal(format!("db error: {other}")),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Internal(format!("io error: {e}"))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::BadRequest(format!("json error: {e}"))
    }
}

impl From<minijinja::Error> for Error {
    fn from(e: minijinja::Error) -> Self {
        Error::Internal(format!("template error: {e}"))
    }
}
