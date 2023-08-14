use std::fmt::Display;

use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use serde_json::Value;

use crate::error::{JsonError, Result};
use crate::operation::{OperationComponent, Operator};
use crate::path::Path;
use crate::transformer::TransformSide;

const NUMBER_ADD_SUB_TYPE_NAME: &str = "na";
const TEXT_SUB_TYPE_NAME: &str = "text";

pub trait SubTypeFunctions {
    fn box_clone(&self) -> Box<dyn SubTypeFunctions>;

    fn invert(&self, path: &Path, sub_type_operator: &Value) -> Result<Operator>;

    fn compose(&self, base: &Operator, other: &Operator) -> Option<Operator>;

    fn transform(
        &self,
        new: OperationComponent,
        base: OperationComponent,
        side: TransformSide,
    ) -> Result<Vec<OperationComponent>>;

    fn apply(&self, val: Option<&Value>, sub_type_operand: &Value) -> Result<Value>;
}

impl Clone for Box<dyn SubTypeFunctions> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubType {
    NumberAdd,
    Text,
    Custome(String),
}

impl TryFrom<&Value> for SubType {
    type Error = JsonError;

    fn try_from(value: &Value) -> std::result::Result<Self, Self::Error> {
        match value {
            Value::String(sub) => {
                if sub.eq(NUMBER_ADD_SUB_TYPE_NAME) {
                    return Ok(SubType::NumberAdd);
                }
                if sub.eq(TEXT_SUB_TYPE_NAME) {
                    return Ok(SubType::Text);
                }
                Ok(SubType::Custome(sub.to_string()))
            }
            _ => Err(JsonError::InvalidOperation(format!(
                "invalid sub type: {}",
                value
            ))),
        }
    }
}

impl Display for SubType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = match self {
            SubType::NumberAdd => NUMBER_ADD_SUB_TYPE_NAME.into(),
            SubType::Text => TEXT_SUB_TYPE_NAME.into(),
            SubType::Custome(t) => t.to_string(),
        };
        f.write_str(&s)?;
        Ok(())
    }
}

pub struct SubTypeFunctionsHolder {
    subtype_operators: DashMap<SubType, Box<dyn SubTypeFunctions>>,
}

impl SubTypeFunctionsHolder {
    pub fn new() -> SubTypeFunctionsHolder {
        let subtype_operators: DashMap<SubType, Box<dyn SubTypeFunctions>> = DashMap::new();
        subtype_operators.insert(SubType::NumberAdd, Box::new(NumberAddSubType {}));
        subtype_operators.insert(SubType::Text, Box::new(TextSubType {}));
        SubTypeFunctionsHolder { subtype_operators }
    }

    pub fn register_subtype(
        &self,
        sub_type: String,
        o: Box<dyn SubTypeFunctions>,
    ) -> Result<Option<Box<dyn SubTypeFunctions>>> {
        if sub_type.eq(NUMBER_ADD_SUB_TYPE_NAME) || sub_type.eq(TEXT_SUB_TYPE_NAME) {
            return Err(JsonError::ConflictSubType(sub_type));
        }

        Ok(self.subtype_operators.insert(SubType::Custome(sub_type), o))
    }

    pub fn unregister_subtype(&self, sub_type: &String) -> Option<Box<dyn SubTypeFunctions>> {
        if sub_type.eq(NUMBER_ADD_SUB_TYPE_NAME) || sub_type.eq(TEXT_SUB_TYPE_NAME) {
            return None;
        }

        self.subtype_operators
            .remove(&SubType::Custome(sub_type.clone()))
            .map(|s| s.1)
    }

    pub fn get(&self, sub_type: &SubType) -> Option<Ref<SubType, Box<dyn SubTypeFunctions>>> {
        self.subtype_operators.get(sub_type)
    }

    pub fn clear(&self) {
        self.subtype_operators.clear();
    }
}

impl Default for SubTypeFunctionsHolder {
    fn default() -> Self {
        Self::new()
    }
}

struct NumberAddSubType {}

impl SubTypeFunctions for NumberAddSubType {
    fn box_clone(&self) -> Box<dyn SubTypeFunctions> {
        Box::new(NumberAddSubType {})
    }

    fn invert(&self, path: &Path, sub_type_operator: &Value) -> Result<Operator> {
        todo!()
    }

    fn compose(&self, base: &Operator, other: &Operator) -> Option<Operator> {
        todo!()
    }

    fn transform(
        &self,
        new: OperationComponent,
        base: OperationComponent,
        side: TransformSide,
    ) -> Result<Vec<OperationComponent>> {
        todo!()
    }

    fn apply(&self, val: Option<&Value>, sub_type_operand: &Value) -> Result<Value> {
        if let Some(old_v) = val {
            match old_v {
                Value::Number(n) => {
                    let new_v = n.as_u64().unwrap() + old_v.as_u64().unwrap();
                    let serde_v = serde_json::to_value(new_v)?;
                    Ok(serde_v)
                }
                _ => Err(JsonError::BadPath),
            }
        } else {
            Ok(sub_type_operand.clone())
        }
    }
}

struct TextSubType {}

impl SubTypeFunctions for TextSubType {
    fn box_clone(&self) -> Box<dyn SubTypeFunctions> {
        Box::new(TextSubType {})
    }

    fn invert(&self, path: &Path, sub_type_operator: &Value) -> Result<Operator> {
        todo!()
    }

    fn compose(&self, base: &Operator, other: &Operator) -> Option<Operator> {
        todo!()
    }

    fn transform(
        &self,
        new: OperationComponent,
        base: OperationComponent,
        side: TransformSide,
    ) -> Result<Vec<OperationComponent>> {
        todo!()
    }

    fn apply(&self, val: Option<&Value>, sub_type_operand: &Value) -> Result<Value> {
        todo!()
    }
}
