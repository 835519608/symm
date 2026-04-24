use thiserror::Error;

#[derive(Debug, Error)]
pub enum SymmError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}
