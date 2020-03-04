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
