use std::mem;

use crate::{
    error::{JsonError, Result},
    operation::Operator,
    path::Path,
    sub_type::SubTypeFunctionsHolder,
};

use serde_json::Value;

pub trait Routable {
    fn route_get(&self, paths: &Path) -> Result<Option<&Value>>;

    fn route_get_mut(&mut self, paths: &Path) -> Result<Option<&mut Value>>;
}

pub trait Appliable {
    fn apply(
        &mut self,
        paths: Path,
        operator: Operator,
        sub_type_functions: &SubTypeFunctionsHolder,
    ) -> Result<()>;
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
    fn apply(
        &mut self,
        paths: Path,
        op: Operator,
        sub_type_functions: &SubTypeFunctionsHolder,
    ) -> Result<()> {
        if paths.len() > 1 {
            let (left, right) = paths.split_at(paths.len() - 1);
            return self.route_get_mut(&left)?.ok_or(JsonError::BadPath)?.apply(
                right,
                op,
                sub_type_functions,
            );
        }
        match self {
            Value::Array(array) => array.apply(paths, op, sub_type_functions),
            Value::Object(obj) => obj.apply(paths, op, sub_type_functions),
            Value::Number(n) => match op {
                Operator::AddNumber(v) => {
                    let new_v = n.as_u64().unwrap() + v.as_u64().unwrap();
                    let serde_v = serde_json::to_value(new_v)?;
                    _ = mem::replace(self, serde_v);
                    Ok(())
                }
                _ => Err(JsonError::InvalidOperation(
                    "Only AddNumber operation can apply to a Number JSON Value".into(),
                )),
            },
            _ => Err(JsonError::InvalidOperation(
                "Operation can only apply on array or object".into(),
            )),
        }
    }
}

impl Appliable for serde_json::Map<String, serde_json::Value> {
    fn apply(
        &mut self,
        paths: Path,
        op: Operator,
        sub_type_functions: &SubTypeFunctionsHolder,
    ) -> Result<()> {
        assert!(paths.len() == 1);

        let k = paths.first_key_path().ok_or(JsonError::BadPath)?;
        let target_value = self.get_mut(k);
        match &op {
            Operator::AddNumber(v) => {
                if let Some(old_v) = target_value {
                    old_v.apply(paths, op, sub_type_functions)
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
            _ => Err(JsonError::BadPath),
        }
    }
}

impl Appliable for Vec<serde_json::Value> {
    fn apply(
        &mut self,
        paths: Path,
        op: Operator,
        sub_type_functions: &SubTypeFunctionsHolder,
    ) -> Result<()> {
        assert!(paths.len() == 1);

        let index = paths.first_index_path().ok_or(JsonError::BadPath)?;
        let target_value = self.get_mut(*index);
        match op {
            Operator::AddNumber(v) => {
                if let Some(old_v) = target_value {
                    match old_v {
                        Value::Number(n) => {
                            let new_v = n.as_u64().unwrap() + v.as_u64().unwrap();
                            let serde_v = serde_json::to_value(new_v)?;
                            self[*index] = serde_v;
                            Ok(())
                        }
                        _ => Err(JsonError::BadPath),
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
            _ => Err(JsonError::BadPath),
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
