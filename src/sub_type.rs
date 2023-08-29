use std::fmt::Display;
use std::hash::Hash;
use std::vec;

use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use serde_json::{Map, Value};

use crate::error::{JsonError, Result};
use crate::operation::Operator;
use crate::path::Path;
use crate::transformer::TransformSide;

const NUMBER_ADD_SUB_TYPE_NAME: &str = "na";
const TEXT_SUB_TYPE_NAME: &str = "text";

pub trait SubTypeFunctions {
    fn box_clone(&self) -> Box<dyn SubTypeFunctions>;

    fn invert(&self, path: &Path, sub_type_operand: &Value) -> Result<Operator>;

    fn merge(&self, base_operand: &Value, other: &Operator) -> Option<Operator>;

    fn transform(&self, new: &Value, base: &Value, side: TransformSide) -> Result<Vec<Value>>;

    fn apply(&self, val: Option<&Value>, sub_type_operand: &Value) -> Result<Option<Value>>;

    fn validate_operand(&self, val: &Value) -> Result<()>;
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

    fn invert(&self, _: &Path, sub_type_operand: &Value) -> Result<Operator> {
        if let Value::Number(n) = sub_type_operand {
            if n.is_i64() {
                Ok(Operator::SubType2(
                    SubType::NumberAdd,
                    serde_json::to_value(-n.as_i64().unwrap()).unwrap(),
                    self.box_clone(),
                ))
            } else if n.is_f64() {
                Ok(Operator::SubType2(
                    SubType::NumberAdd,
                    serde_json::to_value(-n.as_f64().unwrap()).unwrap(),
                    self.box_clone(),
                ))
            } else {
                Err(JsonError::InvalidOperation(format!(
                    "invalid number value:\"{}\" in NumberAdd sub type operand",
                    sub_type_operand
                )))
            }
        } else {
            Err(JsonError::InvalidOperation(format!(
                "invalid operand:\"{}\" for NumberAdd sub type",
                sub_type_operand
            )))
        }
    }

    fn merge(&self, base_operand: &Value, other: &Operator) -> Option<Operator> {
        match &other {
            Operator::SubType2(_, other_v, _) => {
                if base_operand.is_i64() && other_v.is_i64() {
                    let new_v = base_operand.as_i64().unwrap() + other_v.as_i64().unwrap();
                    Some(Operator::SubType2(
                        SubType::NumberAdd,
                        serde_json::to_value(new_v).unwrap(),
                        self.box_clone(),
                    ))
                } else if base_operand.is_f64() || other_v.is_f64() {
                    let new_v = base_operand.as_f64().unwrap() + other_v.as_f64().unwrap();
                    Some(Operator::SubType2(
                        SubType::NumberAdd,
                        serde_json::to_value(new_v).unwrap(),
                        self.box_clone(),
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn transform(&self, new: &Value, _: &Value, _: TransformSide) -> Result<Vec<Value>> {
        Ok(vec![new.clone()])
    }

    fn apply(&self, val: Option<&Value>, sub_type_operand: &Value) -> Result<Option<Value>> {
        if let Value::Number(new_n) = sub_type_operand {
            if let Some(old_v) = val {
                match old_v {
                    Value::Number(old_n) => {
                        if old_n.is_i64() && new_n.is_i64() {
                            return Ok(Some(serde_json::to_value(
                                old_n.as_i64().unwrap() + new_n.as_i64().unwrap(),
                            )?));
                        }

                        Ok(Some(serde_json::to_value(
                            old_n.as_f64().unwrap() + new_n.as_f64().unwrap(),
                        )?))
                    }
                    _ => Err(JsonError::BadPath),
                }
            } else {
                Ok(Some(sub_type_operand.clone()))
            }
        } else {
            Err(JsonError::InvalidOperation(format!(
                "operand: \"{}\" for NumberAdd sub type is not a number",
                sub_type_operand
            )))
        }
    }

    fn validate_operand(&self, val: &Value) -> Result<()> {
        match val {
            Value::Number(_) => Ok(()),
            _ => Err(JsonError::InvalidOperation(
                "Value in AddNumber operator is not a number".into(),
            )),
        }
    }
}

struct TextSubType {}

impl TextSubType {
    fn invert_object(&self, op: &serde_json::Map<String, Value>) -> Result<Map<String, Value>> {
        let mut new_op: Map<String, Value> = serde_json::Map::new();
        if let Some(p) = op.get("p") {
            new_op.insert("p".into(), p.clone());
        }

        if let Some(i) = op.get("i") {
            new_op.insert("d".into(), i.clone());
        } else if let Some(d) = op.get("d") {
            new_op.insert("i".into(), d.clone());
        } else {
            return Err(JsonError::InvalidOperation(format!(
                "invalid sub type operand:\"{}\" for TextSubType",
                Value::Object(op.clone())
            ))
            .into());
        }
        Ok(new_op)
    }
}

impl SubTypeFunctions for TextSubType {
    fn box_clone(&self) -> Box<dyn SubTypeFunctions> {
        Box::new(TextSubType {})
    }

    fn invert(&self, _: &Path, sub_type_operand: &Value) -> Result<Operator> {
        match sub_type_operand {
            Value::Array(ops) => {
                let new_ops = ops
                    .iter()
                    .map(|op| {
                        if let Value::Object(o) = op {
                            Ok(Value::Object(self.invert_object(o)?))
                        } else {
                            Err(JsonError::BadPath)
                        }
                    })
                    .collect::<Result<Vec<Value>>>()?;
                Ok(Operator::SubType2(
                    SubType::Text,
                    Value::Array(new_ops),
                    self.box_clone(),
                ))
            }
            Value::Object(op) => Ok(Operator::SubType2(
                SubType::Text,
                Value::Object(self.invert_object(op)?),
                self.box_clone(),
            )),
            _ => Err(JsonError::InvalidOperation(format!(
                "invalid sub type operand:\"{}\" for TextSubType",
                sub_type_operand
            ))
            .into()),
        }
    }

    fn merge(&self, base: &Value, other: &Operator) -> Option<Operator> {
        if let Operator::SubType2(sub_type, sub_type_operand, f) = other {
            if SubType::Text.eq(sub_type) {}
        }

        None
    }

    fn transform(&self, new: &Value, base: &Value, side: TransformSide) -> Result<Vec<Value>> {
        todo!()
    }

    fn apply(&self, val: Option<&Value>, sub_type_operand: &Value) -> Result<Option<Value>> {
        let p = sub_type_operand.get("p").unwrap().as_u64().unwrap() as usize;
        if let Some(v) = val {
            match v {
                Value::Null => {}
                Value::String(s) => {
                    if let Some(insert) = sub_type_operand.get("i") {
                        return Ok(Some(Value::String(format!(
                            "{}{}{}",
                            &s[0..p],
                            insert.as_str().unwrap(),
                            &s[p..]
                        ))));
                    } else {
                        let to_delete = sub_type_operand.get("d").unwrap().as_str().unwrap();
                        let deleted = &s[p..to_delete.len()];
                        if !to_delete.eq(deleted) {
                            return Err(JsonError::InvalidOperation(format!(
                                "text to delete in text operation is not match target text"
                            )));
                        }

                        return Ok(Some(Value::String(format!(
                            "{}{}",
                            &s[0..p],
                            &s[p + to_delete.len()..]
                        ))));
                    }
                }
                _ => {
                    return Err(JsonError::InvalidOperation(format!(
                        "can not apply text sub operation on value: {}",
                        v
                    )))
                }
            }
        }

        if let Some(insert) = sub_type_operand.get("i") {
            return Ok(Some(insert.clone()));
        }
        return Ok(None);
    }

    fn validate_operand(&self, val: &Value) -> Result<()> {
        let p = val.get("p");
        if p.is_none() {
            return Err(JsonError::InvalidOperation(
                "text sub type operand does not contains Offset".into(),
            ));
        }

        if let Some(insert) = val.get("i") {
            if !insert.is_string() {
                return Err(JsonError::InvalidOperation(
                    format!("text insert non-string value: {}", insert).into(),
                ));
            }
        }

        if let Some(delete) = val.get("d") {
            if !delete.is_string() {
                return Err(JsonError::InvalidOperation(
                    format!("text delete non-string value: {}", delete).into(),
                ));
            }
        }
        Ok(())
    }
}
