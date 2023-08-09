use crate::error::Result;
use crate::operation::OperationComponent;

pub trait SubTypeTransformer {
    fn compose();
    fn transform();
    fn invert(o: &OperationComponent) -> Result<OperationComponent>;
    fn apply();
}
