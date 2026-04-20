use thiserror::Error;

#[derive(Debug, Error)]
pub enum HubriseError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("hubrise returned {status}: {body}")]
    Api { status: u16, body: String },

    #[error("unauthorized")]
    Unauthorized,

    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("url parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("not connected")]
    NotConnected,

    #[error("other: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, HubriseError>;
