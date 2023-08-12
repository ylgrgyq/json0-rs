use std::{
    fmt::{Debug, Display},
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
    sub_type::{SubType, SubTypeFunctions},
};

pub enum Operator {
    Noop(),
    SubType(SubType, Value),
    SubType2(SubType, Value, Box<dyn SubTypeFunctions>),
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

impl Debug for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Noop() => f.debug_tuple("Noop").finish(),
            Self::SubType(arg0, arg1) => f.debug_tuple("SubType").field(arg0).field(arg1).finish(),
            Self::SubType2(arg0, arg1, _) => {
                f.debug_tuple("SubType2").field(arg0).field(arg1).finish()
            }
            Self::AddNumber(arg0) => f.debug_tuple("AddNumber").field(arg0).finish(),
            Self::ListInsert(arg0) => f.debug_tuple("ListInsert").field(arg0).finish(),
            Self::ListDelete(arg0) => f.debug_tuple("ListDelete").field(arg0).finish(),
            Self::ListReplace(arg0, arg1) => f
                .debug_tuple("ListReplace")
                .field(arg0)
                .field(arg1)
                .finish(),
            Self::ListMove(arg0) => f.debug_tuple("ListMove").field(arg0).finish(),
            Self::ObjectInsert(arg0) => f.debug_tuple("ObjectInsert").field(arg0).finish(),
            Self::ObjectDelete(arg0) => f.debug_tuple("ObjectDelete").field(arg0).finish(),
            Self::ObjectReplace(arg0, arg1) => f
                .debug_tuple("ObjectReplace")
                .field(arg0)
                .field(arg1)
                .finish(),
        }
    }
}

impl PartialEq for Operator {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SubType(l0, l1), Self::SubType(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::SubType2(l0, l1, _), Self::SubType2(r0, r1, r2)) => l0 == r0 && l1 == r1,
            (Self::AddNumber(l0), Self::AddNumber(r0)) => l0 == r0,
            (Self::ListInsert(l0), Self::ListInsert(r0)) => l0 == r0,
            (Self::ListDelete(l0), Self::ListDelete(r0)) => l0 == r0,
            (Self::ListReplace(l0, l1), Self::ListReplace(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::ListMove(l0), Self::ListMove(r0)) => l0 == r0,
            (Self::ObjectInsert(l0), Self::ObjectInsert(r0)) => l0 == r0,
            (Self::ObjectDelete(l0), Self::ObjectDelete(r0)) => l0 == r0,
            (Self::ObjectReplace(l0, l1), Self::ObjectReplace(r0, r1)) => l0 == r0 && l1 == r1,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Clone for Operator {
    fn clone(&self) -> Self {
        match self {
            Self::Noop() => Self::Noop(),
            Self::SubType(arg0, arg1) => Self::SubType(arg0.clone(), arg1.clone()),
            Self::SubType2(arg0, arg1, arg2) => {
                Self::SubType2(arg0.clone(), arg1.clone(), arg2.clone())
            }
            Self::AddNumber(arg0) => Self::AddNumber(arg0.clone()),
            Self::ListInsert(arg0) => Self::ListInsert(arg0.clone()),
            Self::ListDelete(arg0) => Self::ListDelete(arg0.clone()),
            Self::ListReplace(arg0, arg1) => Self::ListReplace(arg0.clone(), arg1.clone()),
            Self::ListMove(arg0) => Self::ListMove(arg0.clone()),
            Self::ObjectInsert(arg0) => Self::ObjectInsert(arg0.clone()),
            Self::ObjectDelete(arg0) => Self::ObjectDelete(arg0.clone()),
            Self::ObjectReplace(arg0, arg1) => Self::ObjectReplace(arg0.clone(), arg1.clone()),
        }
    }
}

impl Operator {
    fn map_to_operator(obj: &Map<String, Value>) -> Result<Operator> {
        if let Some(t) = obj.get("t") {
            let sub_type = t.try_into()?;
            let op = obj
                .get("o")
                .and_then(|o| Some(o.clone()))
                .unwrap_or(Value::Null);
            return Ok(Operator::SubType(sub_type, op));
        }

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
            Operator::SubType(_, _) => 3,
            Operator::SubType2(_, _, _) => 3,
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
            Operator::SubType(t, o) => format!("t: {}, o: {}", t, o),
            Operator::SubType2(t, o, _) => format!("t: {}, o: {}", t, o),
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
    pub fn new(path: Path, operator: Operator) -> Result<OperationComponent> {
        let op = OperationComponent { path, operator };
        op.validates()?;
        return Ok(op);
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

    pub fn invert(&self) -> Result<OperationComponent> {
        self.validates()?;

        let mut path = self.path.clone();
        let operator = match &self.operator {
            Operator::Noop() => Operator::Noop(),
            Operator::SubType(_, _) => todo!(),
            Operator::SubType2(_, o, f) => f.invert(&path, o)?,
            Operator::AddNumber(n) => {
                Operator::AddNumber(serde_json::to_value(-n.as_i64().unwrap()).unwrap())
            }
            Operator::ListInsert(v) => Operator::ListDelete(v.clone()),
            Operator::ListDelete(v) => Operator::ListInsert(v.clone()),
            Operator::ListReplace(new_v, old_v) => {
                Operator::ListReplace(old_v.clone(), new_v.clone())
            }
            Operator::ListMove(new) => {
                let old_p = path.replace(path.len() - 1, PathElement::Index(new.clone()));
                if let Some(PathElement::Index(i)) = old_p {
                    Operator::ListMove(i)
                } else {
                    return Err(JsonError::BadPath);
                }
            }
            Operator::ObjectInsert(v) => Operator::ObjectDelete(v.clone()),
            Operator::ObjectDelete(v) => Operator::ObjectInsert(v.clone()),
            Operator::ObjectReplace(new_v, old_v) => {
                Operator::ObjectReplace(old_v.clone(), new_v.clone())
            }
        };
        OperationComponent::new(path, operator)
    }

    pub fn compose(&mut self, op: OperationComponent) -> Option<OperationComponent> {
        if let Some(new_operator) = match &self.operator {
            Operator::Noop() => Some(op.operator.clone()),
            Operator::AddNumber(v1) => match &op.operator {
                Operator::AddNumber(v2) => Some(Operator::AddNumber(
                    serde_json::to_value(v1.as_i64().unwrap() + v2.as_i64().unwrap()).unwrap(),
                )),
                _ => None,
            },
            Operator::SubType2(_, _, f) => f.compose(&self.operator, &op.operator),

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
            return None;
        }

        Some(op)
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

impl Validation for Vec<OperationComponent> {
    fn validates(&self) -> Result<()> {
        for op in self.iter() {
            op.validates()?;
        }
        Ok(())
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

        let paths = Path::try_from(path_value.unwrap())?;
        let operator = input.try_into()?;

        Ok(OperationComponent {
            path: paths,
            operator,
        })
    }
}

impl Display for OperationComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(r#"{{"p": {}, {}}}"#, self.path, self.operator))?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Operation {
    operations: Vec<OperationComponent>,
}

impl Operation {
    pub fn empty_operation() -> Operation {
        Operation { operations: vec![] }
    }

    pub fn new(operations: Vec<OperationComponent>) -> Result<Operation> {
        operations.validates()?;
        Ok(Operation { operations })
    }

    pub fn append(&mut self, op: OperationComponent) -> Result<()> {
        if let Operator::ListMove(m) = op.operator {
            if op
                .path
                .get(op.path.len() - 1)
                .unwrap()
                .eq(&PathElement::Index(m))
            {
                return Ok(());
            }
        }

        if self.is_empty() {
            self.push(op);
            return Ok(());
        }

        let last = self.last_mut().unwrap();
        if last.path.eq(&op.path) {
            if let Some(o) = last.compose(op) {
                self.push(o);
            } else {
                if last.operator.eq(&Operator::Noop()) {
                    self.pop();
                }
                return Ok(());
            }
        } else {
            self.push(op);
        }

        Ok(())
    }

    pub fn compose(mut self, other: Operation) -> Result<Operation> {
        for op in other.into_iter() {
            self.append(op)?;
        }

        Ok(self)
    }
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
        self.operations.validates()
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
        Operation::new(operations)
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

struct OperationFactory {}

impl OperationFactory {}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_number_add_operator() {
        let op: OperationComponent = r#"{"p":["p1","p2"], "t":"na", "o":100}"#.try_into().unwrap();

        assert_eq!(
            Operator::SubType(SubType::NumberAdd, serde_json::to_value(100).unwrap()),
            op.operator
        );
    }

    #[test]
    fn test_text_operator() {
        let op: OperationComponent =
            r#"{"p":["p1","p2"], "t":"text", "o":{"p":["p3"],"si":"hello"}}"#
                .try_into()
                .unwrap();

        assert_eq!(
            Operator::SubType(
                SubType::Text,
                serde_json::from_str(r#"{"p":["p3"],"si":"hello"}"#).unwrap()
            ),
            op.operator
        );
    }
}
