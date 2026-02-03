use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, Error>;
