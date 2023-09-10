use thiserror::Error;

use crate::json::{ApplyOperationError, RouteError};

#[derive(Error, Debug)]
#[error("{}")]
pub enum JsonError {
    #[error("{0}")]
    RouteError(#[from] RouteError),
    #[error("{0}")]
    ApplyOperationError(#[from] ApplyOperationError),
    #[error("Invalid operation: \"{0}\"")]
    InvalidOperation(String),
    /// Path in JSON operation must holding path elements (number or key) splited by ',' and all of the
    /// elements must be surrounded with '[' and ']', eg: ['key1', 2, 'key2'].
    /// If not, this error will be returned
    #[error("Invalid path format")]
    InvalidPathFormat,
    /// Path elements in JSON operation can only be number or key. If not, this error will be returned
    /// This error is simillar with InvalidPathFormat, but it emphasize on
    /// the validation of each path element not the whole path.
    #[error("Invalid path element: {0}")]
    InvalidPathElement(String),
    #[error("Sub type name: {0} conflict with internal sub type name")]
    ConflictSubType(String),
}

pub type Result<T> = std::result::Result<T, JsonError>;
