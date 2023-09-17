use std::rc::Rc;

use error::JsonError;
use json::{Appliable, Routable};
use operation::{Operation, OperationFactory};
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

pub type Result<T> = std::result::Result<T, JsonError>;

pub struct Json0 {
    functions: Rc<SubTypeFunctionsHolder>,
    transformer: Transformer,
    operation_faction: OperationFactory,
}

impl Json0 {
    pub fn new() -> Json0 {
        let functions = Rc::new(SubTypeFunctionsHolder::new());
        let transformer = Transformer::new();
        let operation_faction = OperationFactory::new(functions.clone());

        Json0 {
            functions,
            transformer,
            operation_faction,
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

    pub fn operation_factory(&self) -> &OperationFactory {
        &self.operation_faction
    }

    pub fn apply(&self, value: &mut Value, operations: Vec<Operation>) -> Result<()> {
        for operation in operations {
            for op in operation.into_iter() {
                value
                    .apply(op.path.clone(), op.operator)
                    .map_err(JsonError::ApplyOperationError)?;
            }
        }
        Ok(())
    }

    pub fn get_by_path<'a>(&self, value: &'a mut Value, paths: &Path) -> Result<Option<&'a Value>> {
        value.route_get(paths).map_err(JsonError::RouteError)
    }

    pub fn transform(
        &self,
        operation: &Operation,
        base_operation: &Operation,
    ) -> Result<(Operation, Operation)> {
        self.transformer.transform(operation, base_operation)
    }
}

impl Default for Json0 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::path::{AppendPath, PathBuilder};

    use super::*;
    use serde_json::Map;
    use test_log::test;

    #[test]
    fn test_apply_object_operation() {
        let json0 = Json0::new();

        let mut json_to_operate = Value::Object(Map::new());

        let op = json0
            .operation_factory()
            .object_operation_builder()
            .append_key_path("key")
            .insert(Value::String("world".into()))
            .build()
            .unwrap()
            .into();

        json0.apply(&mut json_to_operate, vec![op]).unwrap();

        let expect_value: Value = serde_json::from_str("{\"key\":\"world\"}").unwrap();
        assert_eq!(expect_value, json_to_operate);
    }
}
