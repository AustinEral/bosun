use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("config error: {0}")]
    Config(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("invalid state: {0}")]
    InvalidState(String),

    #[error(transparent)]
    Storage(#[from] storage::Error),

    #[error(transparent)]
    Policy(#[from] policy::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
