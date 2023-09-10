use thiserror::Error;

use crate::{
    json::{ApplyOperationError, RouteError},
    path::PathError,
};

#[derive(Error, Debug)]
#[error("{}")]
pub enum JsonError {
    #[error("{0}")]
    RouteError(#[from] RouteError),
    #[error("{0}")]
    ApplyOperationError(#[from] ApplyOperationError),
    #[error("Invalid operation: \"{0}\"")]
    InvalidOperation(String),
    #[error("{0}")]
    PathError(#[from] PathError),
    #[error("Sub type name: {0} conflict with internal sub type name")]
    ConflictSubType(String),
}

pub type Result<T> = std::result::Result<T, JsonError>;
