use std::{collections::HashMap, rc::Rc, sync::Arc};

use error::Result;
use json::{Appliable, Routable};
use operation::Operation;
use path::Path;
use serde_json::Value;
use sub_type::{CustomSubTypeHolder, SubTypeTransformer};
use transformer::Transformer;

mod common;
pub mod error;
pub mod json;
pub mod operation;
pub mod path;
mod sub_type;
pub mod transformer;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

struct Json0 {
    sub_type_holder: Rc<CustomSubTypeHolder>,
    transformer: Transformer,
}

impl Json0 {
    pub fn new() -> Json0 {
        let sub_type_holder = Rc::new(CustomSubTypeHolder::new());
        let transformer = Transformer::new(sub_type_holder.clone());
        Json0 {
            sub_type_holder,
            transformer,
        }
    }

    pub fn register_subtype(
        &self,
        sub_type: String,
        o: Box<dyn SubTypeTransformer>,
    ) -> Result<Option<Box<dyn SubTypeTransformer>>> {
        self.sub_type_holder.register_subtype(sub_type, o)
    }

    pub fn unregister_subtype(&self, sub_type: String) -> Option<Box<dyn SubTypeTransformer>> {
        self.sub_type_holder.unregister_subtype(sub_type)
    }

    pub fn clear_registered_subtype(&self) {
        self.sub_type_holder.clear();
    }

    pub fn apply(&mut self, value: &mut Value, operations: Vec<Operation>) -> Result<()> {
        for operation in operations {
            for op_comp in operation.into_iter() {
                value.apply(op_comp.path.clone(), op_comp)?;
            }
        }
        Ok(())
    }

    pub fn get<'a, 'b>(&self, value: &'a mut Value, paths: &'b Path) -> Result<Option<&'a Value>> {
        value.route_get(paths)
    }
}
