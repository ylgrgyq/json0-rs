use std::{
    fmt::Display,
    mem,
    ops::{Deref, DerefMut},
    vec,
};

use serde_json::{Map, Value};

use crate::{
    common::Validation,
    error::JsonError,
    error::{self, Result},
    path::{Path, PathElement},
};

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

        Ok(Operator::Noop())
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
                Value::Number(_) => Ok(()),
                _ => Err(JsonError::InvalidOperation(
                    "Value in AddNumber operator is not a number".into(),
                )),
            },
            _ => Ok(()),
        }
    }
}

impl TryFrom<Value> for Operator {
    type Error = JsonError;

    fn try_from(input: Value) -> std::result::Result<Self, Self::Error> {
        match &input {
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
}

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = match self {
            Operator::Noop() => "".into(),
            Operator::AddNumber(n) => format!("na: {}", n.to_string()),
            Operator::ListInsert(i) => format!("li: {}", i.to_string()),
            Operator::ListDelete(d) => format!("ld: {}", d.to_string()),
            Operator::ListReplace(i, d) => format!("li: {}, ld: {}", i.to_string(), d.to_string()),
            Operator::ListMove(m) => format!("lm: {}", m.to_string()),
            Operator::ObjectInsert(i) => format!("oi: {}", i.to_string()),
            Operator::ObjectDelete(d) => format!("od: {}", d.to_string()),
            Operator::ObjectReplace(i, d) => {
                format!("oi: {}, od: {}", i.to_string(), d.to_string())
            }
        };
        f.write_str(&s)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperationComponent {
    pub path: Path,
    pub operator: Operator,
}

impl OperationComponent {
    pub fn new(path: Path, operator: Operator) -> OperationComponent {
        OperationComponent { path, operator }
    }

    pub fn noop(&self) -> OperationComponent {
        OperationComponent {
            path: self.path.clone(),
            operator: Operator::Noop(),
        }
    }

    pub fn clone_not_noop(&self) -> Option<OperationComponent> {
        if let Operator::Noop() = self.operator {
            None
        } else {
            Some(self.clone())
        }
    }

    pub fn not_noop(self) -> Option<OperationComponent> {
        if let Operator::Noop() = self.operator {
            None
        } else {
            Some(self)
        }
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
        match &mut self.operator {
            Operator::ListDelete(v)
            | Operator::ListReplace(_, v)
            | Operator::ObjectDelete(v)
            | Operator::ObjectReplace(_, v) => {
                let (_, p2) = op.path.split_at(common_path.len());
                v.apply(p2, op.operator.clone())?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn operate_path(&self) -> Path {
        if let Operator::AddNumber(_) = self.operator {
            self.path.clone()
        } else {
            let mut p = self.path.clone();
            p.get_mut_elements().pop();
            p
        }
    }

    pub fn increase_last_index_path(&mut self) {
        let path_elems = self.path.get_mut_elements();
        if let Some(last_p) = path_elems.pop() {
            if let PathElement::Index(i) = last_p {
                path_elems.push(PathElement::Index(i + 1))
            }
        }
    }

    pub fn decrease_last_index_path(&mut self) {
        let path_elems = self.path.get_mut_elements();
        if let Some(last_p) = path_elems.pop() {
            if let PathElement::Index(i) = last_p {
                path_elems.push(PathElement::Index(i - 1))
            }
        }
    }

    pub fn check_may_conflict_by_path(&self, common_path: &Path, op: &OperationComponent) -> bool {
        let mut self_operate_path_len = self.path.len() - 1;
        if let Operator::AddNumber(_) = self.operator {
            self_operate_path_len += 1;
        }

        let mut op_operate_path_len = op.path.len() - 1;
        if let Operator::AddNumber(_) = op.operator {
            op_operate_path_len += 1;
        }

        common_path.len() >= self_operate_path_len || common_path.len() >= op_operate_path_len
    }
}

impl Validation for OperationComponent {
    fn validates(&self) -> Result<()> {
        if self.path.is_empty() {
            return Err(JsonError::InvalidOperation("Path is empty".into()));
        }

        self.operator.validates()
    }
}

impl TryFrom<&str> for OperationComponent {
    type Error = JsonError;

    fn try_from(input: &str) -> std::result::Result<Self, Self::Error> {
        let json_value: Value = serde_json::from_str(input)?;
        json_value.try_into()
    }
}

impl TryFrom<Value> for OperationComponent {
    type Error = error::JsonError;

    fn try_from(input: Value) -> std::result::Result<Self, Self::Error> {
        let path_value = input.get("p");

        if path_value.is_none() {
            return Err(JsonError::InvalidOperation("Missing path".into()));
        }

        let paths = Path::from_json_value(path_value.unwrap())?;
        let operator = input.try_into()?;

        Ok(OperationComponent {
            path: paths,
            operator,
        })
    }
}

impl Display for OperationComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{{\"p\": {}, {}}}", self.path, self.operator))?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Operation {
    operations: Vec<OperationComponent>,
}

impl Deref for Operation {
    type Target = Vec<OperationComponent>;

    fn deref(&self) -> &Self::Target {
        &self.operations
    }
}

impl DerefMut for Operation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.operations
    }
}

impl IntoIterator for Operation {
    type Item = OperationComponent;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.operations.into_iter()
    }
}

impl Validation for Operation {
    fn validates(&self) -> Result<()> {
        for op in self.operations.iter() {
            op.validates()?;
        }
        Ok(())
    }
}

impl From<OperationComponent> for Operation {
    fn from(input: OperationComponent) -> Self {
        Operation {
            operations: vec![input],
        }
    }
}

impl From<Vec<OperationComponent>> for Operation {
    fn from(operations: Vec<OperationComponent>) -> Self {
        Operation { operations }
    }
}

impl TryFrom<Value> for Operation {
    type Error = JsonError;

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        let mut operations = vec![];
        match value {
            Value::Array(arr) => {
                for v in arr {
                    let op: OperationComponent = v.try_into()?;
                    operations.push(op);
                }
            }
            _ => {
                operations.push(value.try_into()?);
            }
        }
        Ok(Operation { operations })
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for op in self.operations.iter() {
            f.write_str(&op.to_string())?;
        }

        Ok(())
    }
}
