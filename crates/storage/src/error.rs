use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("storage error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;
