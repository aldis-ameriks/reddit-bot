use std::error::Error;
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug)]
pub enum RedditError {
    NetworkError(reqwest::Error),
    MalformedResponse(serde_json::error::Error),
}

impl From<reqwest::Error> for RedditError {
    fn from(error: reqwest::Error) -> Self {
        RedditError::NetworkError(error)
    }
}

impl From<serde_json::error::Error> for RedditError {
    fn from(error: serde_json::error::Error) -> Self {
        RedditError::MalformedResponse(error)
    }
}

impl Error for RedditError {}

impl fmt::Display for RedditError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RedditError::NetworkError(err) => err.fmt(f),
            RedditError::MalformedResponse(err) => err.fmt(f),
        }
    }
}
