use thiserror::Error;

#[derive(Error, Debug)]
pub enum JsonError {
    #[error("The parameter: \"{0}\" is invalid for reason: {1}")]
    InvalidParameter(String, String),
}

pub type BitcaskResult<T> = Result<T, BitcaskError>;
