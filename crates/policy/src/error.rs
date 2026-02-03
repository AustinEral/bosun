use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("capability denied: {0}")]
    Denied(String),

    #[error("invalid policy: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
