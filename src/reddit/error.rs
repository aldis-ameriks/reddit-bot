pub enum Error {
    NetworkError(reqwest::Error),
    MalformedResponse(serde_json::error::Error),
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::NetworkError(error)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(error: serde_json::error::Error) -> Self {
        Error::MalformedResponse(error)
    }
}
