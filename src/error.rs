use thiserror::Error;

#[derive(Error, Debug)]
pub enum JsonError {
    #[error("The parameter: \"{0}\" is invalid for reason: {1}")]
    InvalidParameter(String, String),
    #[error("Unexpetec value reached while traversing path")]
    BadPath,
}

pub type Result<T> = std::result::Result<T, JsonError>;
