use std::mem;

use serde_json::{Map, Value};

use crate::{common::Validation, error::JsonError, error::Result, path::Path};

pub trait Appliable {
    fn apply(&mut self, paths: Path, operator: Operator) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Noop(),
    AddNumber(Value),
    ListInsert(Value),
    ListDelete(Value),
    // Replace value from last value to first value in json array.
    // First value is the new value.
    // Last value is the old value.
    ListReplace(Value, Value),
    ListMove(usize),
    ObjectInsert(Value),
    ObjectDelete(Value),
    // Replace value from last value to first value in json object.
    // First value is the new value.
    // Last value is the old value.
    ObjectReplace(Value, Value),
}

impl Operator {
    fn from_json_value(input: &Value) -> Result<Operator> {
        match input {
            Value::Object(obj) => {
                let operator = Operator::map_to_operator(obj)?;
                operator.validate_json_object_size(obj)?;
                Ok(operator)
            }
            _ => Err(JsonError::InvalidOperation(
                "Operator can only be parsed from JSON Object".into(),
            )),
        }
    }

    fn map_to_operator(obj: &Map<String, Value>) -> Result<Operator> {
        if let Some(na) = obj.get("na") {
            return Ok(Operator::AddNumber(na.clone()));
        }

        if let Some(lm) = obj.get("lm") {
            let i = Operator::value_to_index(lm)?;
            return Ok(Operator::ListMove(i));
        }

        if let Some(li) = obj.get("li") {
            if let Some(ld) = obj.get("ld") {
                return Ok(Operator::ListReplace(li.clone(), ld.clone()));
            }
            return Ok(Operator::ListInsert(li.clone()));
        }

        if let Some(ld) = obj.get("ld") {
            return Ok(Operator::ListDelete(ld.clone()));
        }

        if let Some(oi) = obj.get("oi") {
            if let Some(od) = obj.get("od") {
                return Ok(Operator::ObjectReplace(oi.clone(), od.clone()));
            }
            return Ok(Operator::ObjectInsert(oi.clone()));
        }

        if let Some(od) = obj.get("od") {
            return Ok(Operator::ObjectDelete(od.clone()));
        }

        Err(JsonError::InvalidOperation("Unknown operator".into()))
    }

    fn validate_json_object_size(&self, obj: &Map<String, Value>) -> Result<()> {
        let size = match self {
            Operator::Noop() => 1,
            Operator::AddNumber(_) => 2,
            Operator::ListInsert(_) => 2,
            Operator::ListDelete(_) => 2,
            Operator::ListReplace(_, _) => 3,
            Operator::ListMove(_) => 2,
            Operator::ObjectInsert(_) => 2,
            Operator::ObjectDelete(_) => 2,
            Operator::ObjectReplace(_, _) => 3,
        };
        if obj.len() != size {
            return Err(JsonError::InvalidOperation(
                "JSON object size bigger than operator required".into(),
            ));
        }
        Ok(())
    }

    fn value_to_index(val: &Value) -> Result<usize> {
        if let Some(i) = val.as_u64() {
            return Ok(i as usize);
        }
        return Err(JsonError::InvalidOperation(format!(
            "{} can not parsed to index",
            val.to_string()
        )));
    }
}

impl Validation for Operator {
    fn validates(&self) -> Result<()> {
        match self {
            Operator::AddNumber(v) => match v {
                Value::Number(n) => Ok(()),
                _ => Err(JsonError::InvalidOperation(
                    "Value in AddNumber operator is not a number".into(),
                )),
            },
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct OperationComponent {
    path: Path,
    operator: Operator,
}

impl OperationComponent {
    pub fn new(path: Path, operator: Operator) -> OperationComponent {
        OperationComponent { path, operator }
    }

    pub fn from_str(input: &str) -> Result<OperationComponent> {
        let json_value: Value = serde_json::from_str(input)?;
        let path_value = json_value.get("p");

        if path_value.is_none() {
            return Err(JsonError::InvalidOperation("Missing path".into()));
        }

        let paths = Path::from_json_value(path_value.unwrap())?;
        let operator = Operator::from_json_value(&json_value)?;

        Ok(OperationComponent {
            path: paths,
            operator,
        })
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    pub fn get_operator(&self) -> &Operator {
        &self.operator
    }

    pub fn merge(&mut self, op: &OperationComponent) -> bool {
        if let Some(new_operator) = match &self.operator {
            Operator::Noop() => Some(op.operator.clone()),
            Operator::AddNumber(v1) => match &op.operator {
                Operator::AddNumber(v2) => Some(Operator::AddNumber(
                    serde_json::to_value(v1.as_i64().unwrap() + v2.as_i64().unwrap()).unwrap(),
                )),
                _ => None,
            },

            Operator::ListInsert(v1) => match &op.operator {
                Operator::ListDelete(v2) => {
                    if v1.eq(v2) {
                        Some(Operator::Noop())
                    } else {
                        None
                    }
                }
                Operator::ListReplace(new_v, old_v) => {
                    if old_v.eq(v1) {
                        Some(Operator::ListInsert(new_v.clone()))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            Operator::ListReplace(new_v1, old_v1) => match &op.operator {
                Operator::ListDelete(v2) => {
                    if new_v1.eq(v2) {
                        Some(Operator::ListDelete(old_v1.clone()))
                    } else {
                        None
                    }
                }
                Operator::ListReplace(new_v2, old_v2) => {
                    if new_v1.eq(old_v2) {
                        Some(Operator::ListReplace(new_v2.clone(), old_v1.clone()))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            Operator::ObjectInsert(v1) => match &op.operator {
                Operator::ObjectDelete(v2) => {
                    if v1.eq(v2) {
                        Some(Operator::Noop())
                    } else {
                        None
                    }
                }
                Operator::ObjectReplace(new_v2, old_v2) => {
                    if v1.eq(old_v2) {
                        Some(Operator::ObjectInsert(new_v2.clone()))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            Operator::ObjectDelete(v1) => match &op.operator {
                Operator::ObjectInsert(v2) => Some(Operator::ObjectReplace(v1.clone(), v2.clone())),
                _ => None,
            },
            Operator::ObjectReplace(new_v1, old_v1) => match &op.operator {
                Operator::ObjectDelete(v2) => {
                    if new_v1.eq(v2) {
                        Some(Operator::ObjectDelete(old_v1.clone()))
                    } else {
                        None
                    }
                }
                Operator::ObjectReplace(new_v2, old_v2) => {
                    if new_v1.eq(old_v2) {
                        Some(Operator::ObjectReplace(new_v2.clone(), old_v1.clone()))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        } {
            _ = mem::replace(&mut self.operator, new_operator);
            return true;
        }

        false
    }

    pub fn consume(&mut self, common_path: &Path, op: &OperationComponent) -> Result<()> {
        if op.get_path().len() > self.get_path().len()
            || common_path.len() > self.get_path().len()
            || common_path.len() > op.get_path().len()
        {
            return Ok(());
        }

        debug_assert!(self
            .get_path()
            .split_at(common_path.len())
            .0
            .eq(common_path));

        if let Some(new_p) = match &mut self.operator {
            Operator::ListDelete(v)
            | Operator::ListReplace(_, v)
            | Operator::ObjectDelete(v)
            | Operator::ObjectReplace(_, v) => {
                let (p1, p2) = self.path.split_at(common_path.len());
                v.apply(p2, op.operator.clone())?;
                Some(p1)
            }
            _ => None,
        } {
            self.path = new_p;
        }
        Ok(())
    }
}

impl Validation for OperationComponent {
    fn validates(&self) -> Result<()> {
        if self.get_path().is_empty() {
            return Err(JsonError::InvalidOperation("Path is empty".into()));
        }

        self.operator.validates()
    }
}
