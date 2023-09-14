use std::fmt::Display;
use std::hash::Hash;
use std::vec;

use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use serde_json::{Map, Value};

use crate::error::{JsonError, Result};
use crate::json::{ApplyOperationError, ApplyResult};
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

    fn apply(&self, val: Option<&Value>, sub_type_operand: &Value) -> ApplyResult<Option<Value>>;

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
                Ok(Operator::SubType(
                    SubType::NumberAdd,
                    serde_json::to_value(-n.as_i64().unwrap()).unwrap(),
                    self.box_clone(),
                ))
            } else if n.is_f64() {
                Ok(Operator::SubType(
                    SubType::NumberAdd,
                    serde_json::to_value(-n.as_f64().unwrap()).unwrap(),
                    self.box_clone(),
                ))
            } else {
                Err(JsonError::InvalidOperation(format!(
                    "invalid number value:\"{sub_type_operand}\" in NumberAdd sub type operand",
                )))
            }
        } else {
            Err(JsonError::InvalidOperation(format!(
                "invalid operand:\"{sub_type_operand}\" for NumberAdd sub type",
            )))
        }
    }

    fn merge(&self, base_operand: &Value, other: &Operator) -> Option<Operator> {
        match &other {
            Operator::SubType(_, other_v, _) => {
                if base_operand.is_i64() && other_v.is_i64() {
                    let new_v = base_operand.as_i64().unwrap() + other_v.as_i64().unwrap();
                    Some(Operator::SubType(
                        SubType::NumberAdd,
                        serde_json::to_value(new_v).unwrap(),
                        self.box_clone(),
                    ))
                } else if base_operand.is_f64() || other_v.is_f64() {
                    let new_v = base_operand.as_f64().unwrap() + other_v.as_f64().unwrap();
                    Some(Operator::SubType(
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

    fn apply(&self, val: Option<&Value>, sub_type_operand: &Value) -> ApplyResult<Option<Value>> {
        if let Value::Number(new_n) = sub_type_operand {
            if let Some(old_v) = val {
                match old_v {
                    Value::Number(old_n) => {
                        if old_n.is_i64() && new_n.is_i64() {
                            return Ok(Some(
                                serde_json::to_value(
                                    old_n.as_i64().unwrap() + new_n.as_i64().unwrap(),
                                )
                                .unwrap(),
                            ));
                        }

                        Ok(Some(
                            serde_json::to_value(old_n.as_f64().unwrap() + new_n.as_f64().unwrap())
                                .unwrap(),
                        ))
                    }
                    _ => Err(ApplyOperationError::InvalidApplySubtypeOperationTarget {
                        subtype_name: SubType::NumberAdd.to_string(),
                        target_value: old_v.clone(),
                        subtype_operand: sub_type_operand.clone(),
                        reason: "NumberAdd operation must apply to a number value".to_string(),
                    }),
                }
            } else {
                Ok(Some(sub_type_operand.clone()))
            }
        } else {
            panic!("operand: {sub_type_operand} in NumberAdd subtype operation is not a number");
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

#[derive(Debug, PartialEq)]
struct TextOperand {
    offset: usize,
    insert: Option<String>,
    delete: Option<String>,
}

impl TextOperand {
    fn new_insert(offset: usize, insert: String) -> TextOperand {
        TextOperand {
            offset,
            insert: Some(insert),
            delete: None,
        }
    }
    fn new_delete(offset: usize, delete: String) -> TextOperand {
        TextOperand {
            offset,
            insert: None,
            delete: Some(delete),
        }
    }
    fn is_insert(&self) -> bool {
        self.insert.is_some()
    }
    fn is_delete(&self) -> bool {
        self.delete.is_some()
    }
    fn get_insert(&self) -> &Option<String> {
        &self.insert
    }
    fn get_delete(&self) -> &Option<String> {
        &self.delete
    }
    fn uncheck_get_insert(&self) -> String {
        self.get_insert().as_ref().unwrap().clone()
    }
    fn uncheck_get_delete(&self) -> String {
        self.get_delete().as_ref().unwrap().clone()
    }
    fn to_value(&self) -> Value {
        let mut op = Map::new();
        op.insert("p".into(), serde_json::to_value(self.offset).unwrap());

        if let Some(i) = &self.insert {
            op.insert("i".into(), Value::String(i.clone()));
        }

        if let Some(d) = &self.delete {
            op.insert("d".into(), Value::String(d.clone()));
        }
        Value::Object(op)
    }
}

impl PartialOrd for TextOperand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.offset.partial_cmp(&other.offset)
    }
}

impl TryFrom<&Value> for TextOperand {
    type Error = JsonError;

    fn try_from(val: &Value) -> std::result::Result<Self, Self::Error> {
        let p = val.get("p");
        if p.is_none() {
            return Err(JsonError::InvalidOperation(
                "text sub type operand does not contains Offset".into(),
            ));
        }
        if !p.unwrap().is_i64() {
            return Err(JsonError::InvalidOperation(format!(
                "offset: {} in text sub type operand is not value number",
                p.unwrap()
            )));
        }

        let offset = p.unwrap().as_i64().unwrap() as usize;

        if let Some(insert) = val.get("i") {
            if val.get("d").is_some() {
                return Err(JsonError::InvalidOperation(format!(
                    "invalid text operand: {}, insert and delete at the same time",
                    val
                )));
            }
            if !insert.is_string() {
                return Err(JsonError::InvalidOperation(format!(
                    "text insert non-string value: {}",
                    insert
                )));
            }
            return Ok(TextOperand {
                offset,
                insert: Some(insert.as_str().unwrap().into()),
                delete: None,
            });
        }

        if let Some(delete) = val.get("d") {
            if !delete.is_string() {
                return Err(JsonError::InvalidOperation(format!(
                    "text delete non-string value: {}",
                    delete
                )));
            }
            return Ok(TextOperand {
                offset,
                insert: None,
                delete: Some(delete.as_str().unwrap().into()),
            });
        }
        Err(JsonError::InvalidOperation(format!(
            "invalid text operand: {}",
            val
        )))
    }
}

struct TextSubType {}

impl TextSubType {
    fn invert_object(&self, op: &TextOperand) -> Result<TextOperand> {
        if let Some(i) = op.get_insert() {
            Ok(TextOperand::new_delete(op.offset, i.clone()))
        } else if let Some(d) = op.get_delete() {
            Ok(TextOperand::new_insert(op.offset, d.clone()))
        } else {
            Err(JsonError::InvalidOperation(format!(
                "invalid sub type operand:\"{}\" for TextSubType",
                op.to_value()
            )))
        }
    }

    fn transform_position(&self, pos: usize, op: &TextOperand, insert_after: bool) -> usize {
        let p = op.offset;
        if let Some(i) = &op.insert {
            if p < pos || (p == pos && insert_after) {
                pos + i.len()
            } else {
                pos
            }
        } else if pos <= p {
            pos
        } else if pos <= p + op.delete.as_ref().unwrap().len() {
            p
        } else {
            pos - op.delete.as_ref().unwrap().len()
        }
    }
}

impl SubTypeFunctions for TextSubType {
    fn box_clone(&self) -> Box<dyn SubTypeFunctions> {
        Box::new(TextSubType {})
    }

    fn invert(&self, _: &Path, sub_type_operand: &Value) -> Result<Operator> {
        let s: TextOperand = sub_type_operand.try_into()?;
        Ok(Operator::SubType(
            SubType::Text,
            self.invert_object(&s)?.to_value(),
            self.box_clone(),
        ))
    }

    fn merge(&self, base: &Value, other: &Operator) -> Option<Operator> {
        if let Operator::SubType(sub_type, sub_type_operand, _) = other {
            if SubType::Text.eq(sub_type) {
                let base_op: TextOperand = base.try_into().ok()?;
                let other_op: TextOperand = sub_type_operand.try_into().ok()?;

                if base_op.is_insert()
                    && other_op.is_insert()
                    && base_op <= other_op
                    && other_op.offset <= base_op.offset + base_op.uncheck_get_insert().len()
                {
                    let s = format!(
                        "{}{}{}",
                        &base_op.uncheck_get_insert()[0..other_op.offset - base_op.offset],
                        &other_op.uncheck_get_insert(),
                        &base_op.uncheck_get_insert()[other_op.offset - base_op.offset..],
                    );

                    return Some(Operator::SubType(
                        SubType::Text,
                        TextOperand::new_insert(base_op.offset, s).to_value(),
                        self.box_clone(),
                    ));
                }
                if base_op.is_delete()
                    && other_op.is_delete()
                    && other_op <= base_op
                    && base_op.offset <= other_op.offset + other_op.uncheck_get_delete().len()
                {
                    let s = format!(
                        "{}{}{}",
                        &other_op.uncheck_get_delete()[0..base_op.offset - other_op.offset],
                        &base_op.uncheck_get_delete(),
                        &other_op.uncheck_get_delete()[base_op.offset - other_op.offset..],
                    );

                    return Some(Operator::SubType(
                        SubType::Text,
                        TextOperand::new_delete(other_op.offset, s).to_value(),
                        self.box_clone(),
                    ));
                }
            }
        }

        None
    }

    fn transform(&self, new: &Value, base: &Value, side: TransformSide) -> Result<Vec<Value>> {
        let new_operand: TextOperand = new.try_into()?;
        let base_operand: TextOperand = base.try_into()?;
        let mut ops = vec![];
        if new_operand.is_insert() {
            let p = self.transform_position(
                new_operand.offset,
                &base_operand,
                side == TransformSide::Right,
            );
            ops.push(TextOperand::new_insert(p, new_operand.insert.unwrap()).to_value())
        } else {
            let mut d_str = new_operand.uncheck_get_delete();
            if let Some(base_i) = base_operand.get_insert() {
                let base_p = base_operand.offset;
                let new_p = new_operand.offset;
                if new_operand < base_operand {
                    ops.push(
                        TextOperand::new_delete(
                            new_operand.offset,
                            d_str[0..(base_p - new_p)].into(),
                        )
                        .to_value(),
                    );
                    d_str = d_str[base_p - new_p..].into();
                }
                if !d_str.is_empty() {
                    ops.push(
                        TextOperand::new_delete(new_operand.offset + base_i.len(), d_str)
                            .to_value(),
                    );
                }
            } else {
                // Delete vs Delete
                let base_d_str = base_operand.uncheck_get_delete();
                if new_operand.offset >= base_operand.offset + base_d_str.len() {
                    ops.push(
                        TextOperand::new_delete(new_operand.offset - base_d_str.len(), d_str)
                            .to_value(),
                    )
                } else if new_operand.offset + d_str.len() <= base_operand.offset {
                    ops.push(new.clone())
                } else {
                    let mut new_d = "";
                    if new_operand.offset < base_operand.offset {
                        new_d = &d_str[0..base_operand.offset - new_operand.offset]
                    }
                    if new_operand.offset + d_str.len() > base_operand.offset + base_d_str.len() {
                        new_d =
                            &d_str[base_operand.offset + base_d_str.len() - new_operand.offset..]
                    }

                    if !new_d.is_empty() {
                        let p = self.transform_position(new_operand.offset, &base_operand, false);
                        ops.push(TextOperand::new_delete(p, new_d.into()).to_value());
                    }
                }
            }
        }
        Ok(ops)
    }

    fn apply(&self, val: Option<&Value>, sub_type_operand: &Value) -> ApplyResult<Option<Value>> {
        let sub_operand: TextOperand = sub_type_operand.try_into().unwrap();
        let p = sub_operand.offset;
        if let Some(v) = val {
            match v {
                Value::Null => {}
                Value::String(s) => {
                    if let Some(insert) = sub_operand.get_insert() {
                        if p <= s.len() {
                            return Ok(Some(Value::String(format!(
                                "{}{}{}",
                                &s[0..p],
                                insert,
                                &s[p..]
                            ))));
                        } else {
                            return Ok(Some(Value::String(format!("{}{}", s, insert))));
                        }
                    } else {
                        let to_delete = sub_operand.uncheck_get_delete();
                        let deleted = &s[p..to_delete.len()];
                        if !to_delete.eq(deleted) {
                            return Err(ApplyOperationError::InvalidSubtypeOperator {
                                subtype_name: SubType::Text.to_string(),
                                subtype_operand: sub_type_operand.clone(),
                                target_value: v.clone(),
                                reason: "text to delete in text operation is not match target text"
                                    .into(),
                            });
                        }

                        if p <= s.len() {
                            return Ok(Some(Value::String(format!(
                                "{}{}",
                                &s[0..p],
                                &s[p + to_delete.len()..]
                            ))));
                        } else {
                            return Ok(Some(v.clone()));
                        }
                    }
                }
                _ => {
                    return Err(ApplyOperationError::InvalidApplySubtypeOperationTarget {
                        subtype_name: SubType::Text.to_string(),
                        target_value: v.clone(),
                        subtype_operand: sub_type_operand.clone(),
                        reason: "Text operation must apply to a string value".to_string(),
                    });
                }
            }
        }

        if let Some(insert) = sub_type_operand.get("i") {
            return Ok(Some(insert.clone()));
        }
        Ok(None)
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
                return Err(JsonError::InvalidOperation(format!(
                    "text insert non-string value: {}",
                    insert
                )));
            }
        }

        if let Some(delete) = val.get("d") {
            if !delete.is_string() {
                return Err(JsonError::InvalidOperation(format!(
                    "text delete non-string value: {}",
                    delete
                )));
            }
        }
        Ok(())
    }
}
