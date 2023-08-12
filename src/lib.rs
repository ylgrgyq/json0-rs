use std::rc::Rc;

use error::Result;
use json::{Appliable, Routable};
use operation::Operation;
use path::Path;
use serde_json::Value;
use sub_type::{SubTypeFunctions, SubTypeFunctionsHolder};
use transformer::Transformer;

mod common;
pub mod error;
mod json;
pub mod operation;
pub mod path;
mod sub_type;
mod transformer;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

pub struct Json0 {
    functions: Rc<SubTypeFunctionsHolder>,
    transformer: Transformer,
}

impl Json0 {
    pub fn new() -> Json0 {
        let functions = Rc::new(SubTypeFunctionsHolder::new());
        let transformer = Transformer::new(functions.clone());
        Json0 {
            functions,
            transformer,
        }
    }

    pub fn register_subtype(
        &self,
        sub_type: String,
        o: Box<dyn SubTypeFunctions>,
    ) -> Result<Option<Box<dyn SubTypeFunctions>>> {
        self.functions.register_subtype(sub_type, o)
    }

    pub fn unregister_subtype(&self, sub_type: &String) -> Option<Box<dyn SubTypeFunctions>> {
        self.functions.unregister_subtype(sub_type)
    }

    pub fn clear_registered_subtype(&self) {
        self.functions.clear();
    }

    pub fn apply(&self, value: &mut Value, operations: Vec<Operation>) -> Result<()> {
        for operation in operations {
            for op in operation.into_iter() {
                value.apply(op.path.clone(), op.operator, &self.functions)?;
            }
        }
        Ok(())
    }

    pub fn get_by_path<'a, 'b>(
        &self,
        value: &'a mut Value,
        paths: &'b Path,
    ) -> Result<Option<&'a Value>> {
        value.route_get(paths)
    }

    pub fn transform(
        &self,
        operation: &Operation,
        base_operation: &Operation,
    ) -> Result<(Operation, Operation)> {
        self.transformer.transform(operation, base_operation)
    }
}
