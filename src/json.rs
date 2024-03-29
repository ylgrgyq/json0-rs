use std::mem;
use thiserror::Error;

use crate::{
    operation::Operator,
    path::{Path, PathElement},
};

use serde_json::Value;

#[derive(Error, Debug)]
#[error("{}")]
pub enum RouteError {
    #[error("Reach leaf node in json, but still has path: {0} remain")]
    ReachLeafNode(Path),
    #[error("No more path to route into {json_value}")]
    NotEnoughPath { json_value: Value },
    #[error("Expect key path type to route into {json_value}, but next path is {next_path}")]
    ExpectKeyPath {
        json_value: Value,
        next_path: PathElement,
    },
    #[error("Expect index path type to route into {json_value}, but next path is {next_path}")]
    ExpectIndexPath {
        json_value: Value,
        next_path: PathElement,
    },
}

pub type RouteResult<T> = std::result::Result<T, RouteError>;

#[derive(Error, Debug)]
#[error("{}")]
pub enum ApplyOperationError {
    #[error("{0}")]
    RouteError(#[from] RouteError),
    #[error("Can not apply operator: {operator} on value: {target_value}, reason: \"{reason}\"")]
    InvalidApplyTarget {
        operator: Operator,
        target_value: Value,
        reason: String,
    },
    #[error("Can not apply subtype operation: {{type: {subtype_name}, operand: {subtype_operand}}} on value: {target_value}, reason: \"{reason}\"")]
    InvalidApplySubtypeOperationTarget {
        subtype_name: String,
        target_value: Value,
        subtype_operand: Value,
        reason: String,
    },
    #[error("Invalid subtype operator: {{type: {subtype_name}, operand: {subtype_operand}}}, can not apply it on value: {target_value}, reason: \"{reason}\"")]
    InvalidSubtypeOperator {
        subtype_name: String,
        subtype_operand: Value,
        target_value: Value,
        reason: String,
    },
}

pub type ApplyResult<T> = std::result::Result<T, ApplyOperationError>;

pub trait Routable {
    fn route_get(&self, paths: &Path) -> RouteResult<Option<&Value>>;

    fn route_get_mut(&mut self, paths: &Path) -> RouteResult<Option<&mut Value>>;
}

pub trait Appliable {
    fn apply(&mut self, paths: Path, operator: Operator) -> ApplyResult<()>;
}

impl Routable for Value {
    fn route_get(&self, paths: &Path) -> RouteResult<Option<&Value>> {
        match self {
            Value::Array(array) => array.route_get(paths),
            Value::Object(obj) => obj.route_get(paths),
            Value::Null => Ok(None),
            _ => {
                if paths.is_empty() {
                    Ok(Some(self))
                } else {
                    Err(RouteError::ReachLeafNode(paths.clone()))
                }
            }
        }
    }

    fn route_get_mut(&mut self, paths: &Path) -> RouteResult<Option<&mut Value>> {
        match self {
            Value::Array(array) => array.route_get_mut(paths),
            Value::Object(obj) => obj.route_get_mut(paths),
            _ => {
                if paths.is_empty() {
                    Ok(Some(self))
                } else {
                    Err(RouteError::ReachLeafNode(paths.clone()))
                }
            }
        }
    }
}

impl Routable for serde_json::Map<String, serde_json::Value> {
    fn route_get(&self, paths: &Path) -> RouteResult<Option<&Value>> {
        let k = paths.first_key_path().ok_or(if paths.is_empty() {
            RouteError::NotEnoughPath {
                json_value: Value::Object(self.clone()),
            }
        } else {
            RouteError::ExpectKeyPath {
                json_value: Value::Object(self.clone()),
                next_path: paths.get(0).cloned().unwrap(),
            }
        })?;
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

    fn route_get_mut(&mut self, paths: &Path) -> RouteResult<Option<&mut Value>> {
        let k = paths.first_key_path().ok_or(if paths.is_empty() {
            RouteError::NotEnoughPath {
                json_value: Value::Object(self.clone()),
            }
        } else {
            RouteError::ExpectKeyPath {
                json_value: Value::Object(self.clone()),
                next_path: paths.get(0).cloned().unwrap(),
            }
        })?;
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
    fn route_get(&self, paths: &Path) -> RouteResult<Option<&Value>> {
        let i = paths.first_index_path().ok_or(if paths.is_empty() {
            RouteError::NotEnoughPath {
                json_value: Value::Array(self.clone()),
            }
        } else {
            RouteError::ExpectKeyPath {
                json_value: Value::Array(self.clone()),
                next_path: paths.get(0).cloned().unwrap(),
            }
        })?;
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

    fn route_get_mut(&mut self, paths: &Path) -> RouteResult<Option<&mut Value>> {
        let i = paths.first_index_path().ok_or(if paths.is_empty() {
            RouteError::NotEnoughPath {
                json_value: Value::Array(self.clone()),
            }
        } else {
            RouteError::ExpectIndexPath {
                json_value: Value::Array(self.clone()),
                next_path: paths.get(0).cloned().unwrap(),
            }
        })?;
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
    fn apply(&mut self, paths: Path, op: Operator) -> ApplyResult<()> {
        if paths.len() > 1 {
            let (left, right) = paths.split_at(paths.len() - 1);
            return self
                .route_get_mut(&left)
                .map_err(ApplyOperationError::RouteError)?
                .ok_or(ApplyOperationError::RouteError(RouteError::ReachLeafNode(
                    paths,
                )))?
                .apply(right, op);
        }
        match self {
            Value::Array(array) => array.apply(paths, op),
            Value::Object(obj) => obj.apply(paths, op),
            _ => match op {
                Operator::SubType(_, op, f) => {
                    if let Some(v) = f.apply(Some(self), &op)? {
                        _ = mem::replace(self, v);
                    }
                    Ok(())
                }
                Operator::Noop() => Ok(()),
                _ => Err(ApplyOperationError::InvalidApplyTarget {
                    operator: op,
                    target_value: self.clone(),
                    reason: "unexpected operator".to_string(),
                }),
            },
        }
    }
}

impl Appliable for serde_json::Map<String, serde_json::Value> {
    fn apply(&mut self, paths: Path, op: Operator) -> ApplyResult<()> {
        assert!(paths.len() == 1);

        let k = paths
            .first_key_path()
            .ok_or(ApplyOperationError::RouteError(RouteError::ExpectKeyPath {
                json_value: Value::Object(self.clone()),
                next_path: paths.get(0).cloned().unwrap(),
            }))?;
        let target_value = self.get(k);
        match &op {
            Operator::Noop() => Ok(()),
            Operator::SubType(_, op, f) => {
                if let Some(v) = f.apply(target_value, op)? {
                    self.insert(k.clone(), v);
                }
                Ok(())
            }
            Operator::ObjectInsert(v) => {
                self.insert(k.clone(), v.clone());
                Ok(())
            }
            Operator::ObjectDelete(_) => {
                if target_value.is_some() {
                    // we don't check the equality of the values
                    // because OT is hard to implement
                    // if target_v.eq(&delete_v) {
                    self.remove(k);
                    // }
                }
                Ok(())
            }
            Operator::ObjectReplace(new_v, _) => {
                if target_value.is_some() {
                    // we don't check the equality of the values
                    // because OT is hard to implement
                    // if target_v.eq(&old_v) {
                    self.insert(k.clone(), new_v.clone());
                    // }
                }
                Ok(())
            }
            _ => Err(ApplyOperationError::InvalidApplyTarget {
                operator: op,
                target_value: Value::Object(self.clone()),
                reason: "unexpected operator".to_string(),
            }),
        }
    }
}

impl Appliable for Vec<serde_json::Value> {
    fn apply(&mut self, paths: Path, op: Operator) -> ApplyResult<()> {
        assert!(paths.len() == 1);

        let index = paths
            .first_index_path()
            .ok_or(ApplyOperationError::RouteError(
                RouteError::ExpectIndexPath {
                    json_value: Value::Array(self.clone()),
                    next_path: paths.get(0).cloned().unwrap(),
                },
            ))?;
        let target_value = self.get(*index);
        match op {
            Operator::Noop() => Ok(()),
            Operator::SubType(_, op, f) => {
                if let Some(v) = f.apply(target_value, &op)? {
                    self[*index] = v;
                }
                Ok(())
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
                if target_value.is_some() {
                    // we don't check the equality of the values
                    // because OT is hard to implement
                    // if target_v.eq(&delete_v) {
                    self.remove(*index);
                    // }
                }
                Ok(())
            }
            Operator::ListReplace(new_v, _) => {
                if target_value.is_some() {
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
                    if *index != new_index {
                        let new_v = target_v.clone();
                        self.remove(*index);
                        self.insert(new_index, new_v);
                    }
                }
                Ok(())
            }
            _ => Err(ApplyOperationError::InvalidApplyTarget {
                operator: op,
                target_value: Value::Array(self.clone()),
                reason: "unexpected operator".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::path::Path;

    use super::*;
    use test_log::test;

    #[test]
    fn test_route_get_by_path_only_has_object() {
        let json: Value =
            serde_json::from_str(r#"{"level1":"world", "level12":{"level2":"world2"}}"#).unwrap();

        // simple path with only object
        let paths = Path::try_from(r#"["level1"]"#).unwrap();
        assert_eq!(
            json.route_get(&paths).unwrap().unwrap().to_string(),
            r#""world""#
        );
        let paths = Path::try_from(r#"["level12", "level2"]"#).unwrap();
        assert_eq!(
            json.route_get(&paths).unwrap().unwrap().to_string(),
            r#""world2""#
        );
        let paths = Path::try_from(r#"["level3"]"#).unwrap();
        assert!(json.route_get(&paths).unwrap().is_none());

        // complex path with array
        let json: Value =
            serde_json::from_str(r#"{"level1":[1,{"hello":[1,[7,8]]}], "level12":"world"}"#)
                .unwrap();
        let paths = Path::try_from(r#"["level1", 1, "hello"]"#).unwrap();

        assert_eq!(
            json.route_get(&paths).unwrap().unwrap().to_string(),
            r#"[1,[7,8]]"#
        );
    }

    #[test]
    fn test_route_get_by_path_has_array() {
        let json: Value =
            serde_json::from_str(r#"{"level1":["a","b"], "level12":[123, {"level2":["c","d"]}]}"#)
                .unwrap();
        // simple path
        let paths = Path::try_from(r#"["level1", 1]"#).unwrap();
        assert_eq!(
            json.route_get(&paths).unwrap().unwrap().to_string(),
            r#""b""#
        );
        let paths = Path::try_from(r#"["level12", 0]"#).unwrap();

        // complex path
        assert_eq!(
            json.route_get(&paths).unwrap().unwrap().to_string(),
            r#"123"#
        );
        let paths = Path::try_from(r#"["level12", 1, "level2"]"#).unwrap();
        assert_eq!(
            json.route_get(&paths).unwrap().unwrap().to_string(),
            r#"["c","d"]"#
        );
        let json: Value =
            serde_json::from_str(r#"{"level1":[1,{"hello":[1,[7,8]]}], "level12":"world"}"#)
                .unwrap();
        let paths = Path::try_from(r#"["level1", 1, "hello", 1]"#).unwrap();

        assert_eq!(
            json.route_get(&paths).unwrap().unwrap().to_string(),
            r#"[7,8]"#
        );
    }
}
