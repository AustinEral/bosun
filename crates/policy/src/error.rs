use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("capability denied: {0}")]
    Denied(String),

    #[error("invalid policy: {0}")]
    Invalid(String),

    #[error("failed to parse policy: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
