use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("authentication failed: {0}")]
    Authentication(String),
    #[error("request timed out: {0}")]
    Timeout(String),
    #[error("API request failed with HTTP {status}: {message}")]
    Api {
        status: u16,
        message: String,
        body: Option<Value>,
    },
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
