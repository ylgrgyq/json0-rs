use std::{fmt::Display, mem};

use crate::{
    error::{JsonError, Result},
    operation::{Operation, OperationComponent, Operator},
    path::Path,
};

use serde_json::Value;

pub trait Routable {
    fn route_get(&self, paths: &Path) -> Result<Option<&Value>>;

    fn route_get_mut(&mut self, paths: &Path) -> Result<Option<&mut Value>>;
}

pub trait Appliable {
    fn apply(&mut self, paths: Path, operator: OperationComponent) -> Result<()>;
}

impl Routable for Value {
    fn route_get(&self, paths: &Path) -> Result<Option<&Value>> {
        match self {
            Value::Array(array) => array.route_get(paths),
            Value::Object(obj) => obj.route_get(paths),
            Value::Null => Ok(None),
            _ => {
                if paths.is_empty() {
                    Ok(Some(self))
                } else {
                    Err(JsonError::BadPath)
                }
            }
        }
    }

    fn route_get_mut(&mut self, paths: &Path) -> Result<Option<&mut Value>> {
        match self {
            Value::Array(array) => array.route_get_mut(paths),
            Value::Object(obj) => obj.route_get_mut(paths),
            _ => {
                if paths.is_empty() {
                    Ok(Some(self))
                } else {
                    Err(JsonError::BadPath)
                }
            }
        }
    }
}

impl Routable for serde_json::Map<String, serde_json::Value> {
    fn route_get(&self, paths: &Path) -> Result<Option<&Value>> {
        let k = paths.first_key_path().ok_or(JsonError::BadPath)?;
        if let Some(v) = self.get(k) {
            let next_level = paths.next_level();
            if next_level.is_empty() {
                Ok(Some(v))
            } else {
                v.route_get(&next_level)
            }
        } else {
            Ok(None)
        }
    }

    fn route_get_mut(&mut self, paths: &Path) -> Result<Option<&mut Value>> {
        let k = paths.first_key_path().ok_or(JsonError::BadPath)?;
        if let Some(v) = self.get_mut(k) {
            let next_level = paths.next_level();
            if next_level.is_empty() {
                Ok(Some(v))
            } else {
                v.route_get_mut(&next_level)
            }
        } else {
            Ok(None)
        }
    }
}

impl Routable for Vec<serde_json::Value> {
    fn route_get(&self, paths: &Path) -> Result<Option<&Value>> {
        let i = paths.first_index_path().ok_or(JsonError::BadPath)?;
        if let Some(v) = self.get(*i) {
            let next_level = paths.next_level();
            if next_level.is_empty() {
                Ok(Some(v))
            } else {
                v.route_get(&next_level)
            }
        } else {
            Ok(None)
        }
    }

    fn route_get_mut(&mut self, paths: &Path) -> Result<Option<&mut Value>> {
        let i = paths.first_index_path().ok_or(JsonError::BadPath)?;
        if let Some(v) = self.get_mut(*i) {
            let next_level = paths.next_level();
            if next_level.is_empty() {
                Ok(Some(v))
            } else {
                v.route_get_mut(&next_level)
            }
        } else {
            Ok(None)
        }
    }
}

impl Appliable for Value {
    fn apply(&mut self, paths: Path, op: OperationComponent) -> Result<()> {
        if paths.len() > 1 {
            let (left, right) = paths.split_at(paths.len() - 1);
            return self
                .route_get_mut(&left)?
                .ok_or(JsonError::BadPath)?
                .apply(right, op);
        }
        match self {
            Value::Array(array) => array.apply(paths, op),
            Value::Object(obj) => obj.apply(paths, op),
            Value::Number(n) => match op.operator {
                Operator::AddNumber(v) => {
                    let new_v = n.as_u64().unwrap() + v.as_u64().unwrap();
                    let serde_v = serde_json::to_value(new_v)?;
                    _ = mem::replace(self, serde_v);
                    Ok(())
                }
                _ => {
                    return Err(JsonError::InvalidOperation(
                        "Only AddNumber operation can apply to a Number JSON Value".into(),
                    ));
                }
            },
            _ => {
                return Err(JsonError::InvalidOperation(
                    "Operation can only apply on array or object".into(),
                ));
            }
        }
    }
}

impl Appliable for serde_json::Map<String, serde_json::Value> {
    fn apply(&mut self, paths: Path, op: OperationComponent) -> Result<()> {
        assert!(paths.len() == 1);

        let k = paths.first_key_path().ok_or(JsonError::BadPath)?;
        let target_value = self.get_mut(k);
        match &op.operator {
            Operator::AddNumber(v) => {
                if let Some(old_v) = target_value {
                    old_v.apply(paths, op)
                } else {
                    self.insert(k.clone(), v.clone());
                    Ok(())
                }
            }
            Operator::ObjectInsert(v) => {
                self.insert(k.clone(), v.clone());
                Ok(())
            }
            Operator::ObjectDelete(_) => {
                if let Some(_) = target_value {
                    // we don't check the equality of the values
                    // because OT is hard to implement
                    // if target_v.eq(&delete_v) {
                    self.remove(k);
                    // }
                }
                Ok(())
            }
            Operator::ObjectReplace(new_v, _) => {
                if let Some(_) = target_value {
                    // we don't check the equality of the values
                    // because OT is hard to implement
                    // if target_v.eq(&old_v) {
                    self.insert(k.clone(), new_v.clone());
                    // }
                }
                Ok(())
            }
            _ => Err(JsonError::BadPath),
        }
    }
}

impl Appliable for Vec<serde_json::Value> {
    fn apply(&mut self, paths: Path, op: OperationComponent) -> Result<()> {
        assert!(paths.len() == 1);

        let index = paths.first_index_path().ok_or(JsonError::BadPath)?;
        let target_value = self.get_mut(*index);
        match &op.operator {
            Operator::AddNumber(v) => {
                if let Some(old_v) = target_value {
                    match old_v {
                        Value::Number(n) => {
                            let new_v = n.as_u64().unwrap() + v.as_u64().unwrap();
                            let serde_v = serde_json::to_value(new_v)?;
                            self[*index] = serde_v;
                            Ok(())
                        }
                        _ => return Err(JsonError::BadPath),
                    }
                } else {
                    self[*index] = v.clone();
                    Ok(())
                }
            }
            Operator::ListInsert(v) => {
                if *index > self.len() {
                    self.push(v.clone())
                } else {
                    self.insert(*index, v.clone());
                }
                Ok(())
            }
            Operator::ListDelete(_) => {
                if let Some(_) = target_value {
                    // we don't check the equality of the values
                    // because OT is hard to implement
                    // if target_v.eq(&delete_v) {
                    self.remove(*index);
                    // }
                }
                Ok(())
            }
            Operator::ListReplace(new_v, _) => {
                if let Some(_) = target_value {
                    // we don't check the equality of the values
                    // because OT is hard to implement
                    // if target_v.eq(&old_v) {
                    self[*index] = new_v.clone();
                    // }
                }
                Ok(())
            }
            Operator::ListMove(new_index) => {
                if let Some(target_v) = target_value {
                    if *index != *new_index {
                        let new_v = target_v.clone();
                        self.remove(*index);
                        self.insert(*new_index, new_v);
                    }
                }
                Ok(())
            }
            _ => Err(JsonError::BadPath),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JSON {
    value: Value,
}

impl Display for JSON {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl JSON {
    pub fn apply(&mut self, operations: Vec<Operation>) -> Result<()> {
        for operation in operations {
            for op_comp in operation.into_iter() {
                self.value.apply(op_comp.path.clone(), op_comp)?;
            }
        }
        Ok(())
    }

    pub fn get(&self, paths: &Path) -> Result<Option<&Value>> {
        self.value.route_get(paths)
    }
}

impl From<Value> for JSON {
    fn from(value: Value) -> Self {
        JSON { value }
    }
}

impl TryFrom<&str> for JSON {
    type Error = JsonError;

    fn try_from(input: &str) -> std::result::Result<Self, Self::Error> {
        let value = serde_json::from_str(input)?;
        Ok(JSON { value })
    }
}

#[cfg(test)]
mod tests {
    use crate::path::Path;

    use super::*;
    use test_log::test;

    #[test]
    fn test_route_get_by_path_only_has_object() {
        let json: JSON = r#"{"level1":"world", "level12":{"level2":"world2"}}"#
            .try_into()
            .unwrap();

        // simple path with only object
        let paths = Path::try_from(r#"["level1"]"#).unwrap();
        assert_eq!(json.get(&paths).unwrap().unwrap().to_string(), r#""world""#);
        let paths = Path::try_from(r#"["level12", "level2"]"#).unwrap();
        assert_eq!(
            json.get(&paths).unwrap().unwrap().to_string(),
            r#""world2""#
        );
        let paths = Path::try_from(r#"["level3"]"#).unwrap();
        assert!(json.get(&paths).unwrap().is_none());

        // complex path with array
        let json: JSON = r#"{"level1":[1,{"hello":[1,[7,8]]}], "level12":"world"}"#
            .try_into()
            .unwrap();
        let paths = Path::try_from(r#"["level1", 1, "hello"]"#).unwrap();

        assert_eq!(
            json.get(&paths).unwrap().unwrap().to_string(),
            r#"[1,[7,8]]"#
        );
    }

    #[test]
    fn test_route_get_by_path_has_array() {
        let json: JSON = r#"{"level1":["a","b"], "level12":[123, {"level2":["c","d"]}]}"#
            .try_into()
            .unwrap();
        // simple path
        let paths = Path::try_from(r#"["level1", 1]"#).unwrap();
        assert_eq!(json.get(&paths).unwrap().unwrap().to_string(), r#""b""#);
        let paths = Path::try_from(r#"["level12", 0]"#).unwrap();

        // complex path
        assert_eq!(json.get(&paths).unwrap().unwrap().to_string(), r#"123"#);
        let paths = Path::try_from(r#"["level12", 1, "level2"]"#).unwrap();
        assert_eq!(
            json.get(&paths).unwrap().unwrap().to_string(),
            r#"["c","d"]"#
        );
        let json: JSON = r#"{"level1":[1,{"hello":[1,[7,8]]}], "level12":"world"}"#
            .try_into()
            .unwrap();
        let paths = Path::try_from(r#"["level1", 1, "hello", 1]"#).unwrap();

        assert_eq!(json.get(&paths).unwrap().unwrap().to_string(), r#"[7,8]"#);
    }
}
