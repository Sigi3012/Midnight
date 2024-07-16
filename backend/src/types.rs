use database::core::DatabaseError;
use std::num::ParseIntError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SubscriptionError {
    #[error("Regex error: {0}")]
    RegexError(#[from] fancy_regex::Error),
    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),
    #[error("Parse error: {0}")]
    ParseError(#[from] ParseIntError),
    #[error("An invalid link was inputted")]
    InvalidLink,
    #[error("Unexpected non-capture")]
    NonCapture,
}
