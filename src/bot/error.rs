use std::error::Error;
use std::fmt;
use std::fmt::Formatter;

use diesel::result::Error as DatabaseError;

use crate::reddit::error::RedditError;
use crate::telegram::error::TelegramError;

#[derive(Debug)]
pub enum BotError {
    TelegramError(TelegramError),
    DatabaseError(DatabaseError),
    RedditError(RedditError),
}

impl From<TelegramError> for BotError {
    fn from(error: TelegramError) -> Self {
        BotError::TelegramError(error)
    }
}

impl From<DatabaseError> for BotError {
    fn from(error: DatabaseError) -> Self {
        BotError::DatabaseError(error)
    }
}

impl From<RedditError> for BotError {
    fn from(error: RedditError) -> Self {
        BotError::RedditError(error)
    }
}

impl Error for BotError {}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BotError::TelegramError(err) => err.fmt(f),
            BotError::DatabaseError(err) => err.fmt(f),
            BotError::RedditError(err) => err.fmt(f),
        }
    }
}
