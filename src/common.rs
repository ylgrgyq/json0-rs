use crate::error::Result;

pub trait Validation {
    fn validates(&self) -> Result<()>;
}
