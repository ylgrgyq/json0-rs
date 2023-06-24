use std::{collections::BTreeMap, error, hash::Hash, hash::Hasher, mem, vec};

use crate::error::{JsonError, Result};
use serde::de::DeserializeOwned;
use serde_json::{Number, Value};

trait Validation {
    fn is_valid(&self) -> bool;
}

#[derive(Debug, Clone)]
enum Path {
    Index(usize),
    Key(String),
}

#[derive(Debug, Clone)]
struct Paths {
    paths: Vec<Path>,
}

impl Paths {
    fn from_str(input: &str) -> Result<Paths> {
        if let Ok(value) = serde_json::from_str(input) {
            Paths::from_json_value(&value)
        } else {
            Err(JsonError::InvalidPathFormat)
        }
    }

    fn from_json_value(value: &Value) -> Result<Paths> {
        match value {
            Value::Array(arr) => {
                if arr.is_empty() {
                    Err(JsonError::InvalidPathFormat)
                } else {
                    let paths = arr
                        .iter()
                        .map(|pe| match pe {
                            Value::Number(n) => {
                                if let Some(i) = n.as_u64() {
                                    Ok(Path::Index(i as usize))
                                } else {
                                    Err(JsonError::InvalidPathElement(pe.to_string()))
                                }
                            }
                            Value::String(k) => Ok(Path::Key(k.to_string())),
                            _ => Err(JsonError::InvalidPathElement(pe.to_string())),
                        })
                        .collect::<Result<Vec<Path>>>()?;
                    Ok(Paths { paths })
                }
            }
            _ => Err(JsonError::InvalidPathFormat),
        }
    }

    fn first_key_path(&self) -> Result<&String> {
        let first_path = self.paths.first();
        if first_path.is_none() {
            return Err(JsonError::BadPath);
        }

        match first_path.unwrap() {
            Path::Index(_) => return Err(JsonError::BadPath),
            Path::Key(k) => Ok(k),
        }
    }

    fn first_index_path(&self) -> Result<&usize> {
        let first_path = self.paths.first();
        if first_path.is_none() {
            return Err(JsonError::InvalidOperation(
                "Operation can only apply on array or object".into(),
            ));
        }

        match first_path.unwrap() {
            Path::Index(i) => Ok(i),
            Path::Key(k) => {
                return Err(JsonError::InvalidOperation(
                    "Operation can only apply on array or object".into(),
                ))
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    fn len(&self) -> usize {
        self.paths.len()
    }

    fn next_level(&self) -> Paths {
        Paths {
            paths: self.paths[1..].to_vec(),
        }
    }
}

trait Routable {
    fn route_get(&self, paths: &Paths) -> Result<Option<&Value>>;

    fn route_get_mut(&mut self, paths: &Paths) -> Result<Option<&mut Value>>;

    fn route_insert(&mut self, paths: &Paths, value: Value) -> Result<()>;

    fn route_delete(&mut self, paths: &Paths, value: Value) -> Result<()>;

    fn route_replace(&mut self, paths: &Paths, value: Value) -> Result<()>;
}

impl Routable for Value {
    fn route_get(&self, paths: &Paths) -> Result<Option<&Value>> {
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

    fn route_get_mut(&mut self, paths: &Paths) -> Result<Option<&mut Value>> {
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

    fn route_insert(&mut self, paths: &Paths, value: Value) -> Result<()> {
        match self {
            Value::Array(array) => array.route_insert(paths, value),
            Value::Object(obj) => obj.route_insert(paths, value),
            Value::Null => {
                if paths.is_empty() {
                    let new = serde_json::to_value(value)?;
                    let old = mem::replace(self, new);
                    Ok(serde_json::from_value(old)?)
                } else {
                    Err(JsonError::BadPath)
                }
            }
            _ => Err(JsonError::BadPath),
        }
    }

    fn route_delete(&mut self, paths: &Paths, value: Value) -> Result<()> {
        match self {
            Value::Array(array) => array.route_delete(paths, value),
            Value::Object(obj) => obj.route_delete(paths, value),
            Value::Null => {
                if paths.is_empty() {
                    let old = mem::replace(self, Value::Null);
                    Ok(serde_json::from_value(old)?)
                } else {
                    Err(JsonError::BadPath)
                }
            }
            _ => Err(JsonError::BadPath),
        }
    }

    fn route_replace(&mut self, paths: &Paths, value: Value) -> Result<()> {
        match self {
            Value::Array(array) => array.route_replace(paths, value),
            Value::Object(obj) => obj.route_replace(paths, value),
            _ => {
                if paths.is_empty() {
                    let new = serde_json::to_value(value)?;
                    let old = mem::replace(self, new);
                    Ok(serde_json::from_value(old)?)
                } else {
                    Err(JsonError::BadPath)
                }
            }
        }
    }
}

impl Routable for serde_json::Map<String, serde_json::Value> {
    fn route_get(&self, paths: &Paths) -> Result<Option<&Value>> {
        let k = paths.first_key_path()?;
        if let Some(v) = self.get(k) {
            v.route_get(&paths.next_level())
        } else {
            Ok(None)
        }
    }

    fn route_get_mut(&mut self, paths: &Paths) -> Result<Option<&mut Value>> {
        todo!()
    }

    fn route_insert(&mut self, paths: &Paths, value: Value) -> Result<()> {
        todo!()
    }

    fn route_delete(&mut self, paths: &Paths, value: Value) -> Result<()> {
        todo!()
    }

    fn route_replace(&mut self, paths: &Paths, value: Value) -> Result<()> {
        todo!()
    }
}

impl Routable for Vec<serde_json::Value> {
    fn route_get(&self, paths: &Paths) -> Result<Option<&Value>> {
        let i = paths.first_index_path()?;
        if let Some(v) = self.get(*i) {
            v.route_get(&paths.next_level())
        } else {
            Ok(None)
        }
    }

    fn route_get_mut(&mut self, paths: &Paths) -> Result<Option<&mut Value>> {
        todo!()
    }

    fn route_insert(&mut self, paths: &Paths, value: Value) -> Result<()> {
        todo!()
    }

    fn route_delete(&mut self, paths: &Paths, value: Value) -> Result<()> {
        todo!()
    }

    fn route_replace(&mut self, paths: &Paths, value: Value) -> Result<()> {
        todo!()
    }
}

#[derive(Debug)]
enum Operator {
    AddNumber(Value),
    ListInsert(Value),
    ListDelete(Value),
    ListReplace(Value, Value),
    ListMove(usize),
    ObjectInsert(Value),
    ObjectDelete(Value),
    ObjectReplace(Value, Value),
}

impl Operator {
    fn from_json_value(input: &Value) -> Result<Operator> {
        todo!()
    }
}

struct OperationComponent {
    paths: Paths,
    operator: Operator,
}

impl OperationComponent {
    fn from_str(input: &str) -> Result<OperationComponent> {
        let json_value: Value = serde_json::from_str(input)?;
        let path_value = json_value.get("path");

        if path_value.is_none() {
            return Err(JsonError::InvalidOperation("Missing path".into()));
        }

        let paths = Paths::from_json_value(path_value.unwrap())?;
        let operator = Operator::from_json_value(&json_value)?;

        Ok(OperationComponent { paths, operator })
    }
}

trait Appliable {
    fn apply(&mut self, paths: Paths, operator: Operator) -> Result<()>;
}

impl Appliable for Value {
    fn apply(&mut self, paths: Paths, operator: Operator) -> Result<()> {
        match self {
            Value::Array(array) => array.apply(paths, operator),
            Value::Object(obj) => obj.apply(paths, operator),
            Value::Number(n) => match operator {
                Operator::AddNumber(v) => {
                    let new_v = n.as_u64().unwrap() + v.as_u64().unwrap();
                    let serde_v = serde_json::to_value(new_v)?;
                    _ = mem::replace(self, serde_v);
                    Ok(())
                }
                _ => {
                    return Err(JsonError::InvalidOperation(
                        "Operation can only apply on array or object".into(),
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
    fn apply(&mut self, paths: Paths, operator: Operator) -> Result<()> {
        let k = paths.first_key_path()?;
        let target_value = self.get_mut(k);
        if paths.len() > 1 {
            target_value
                .map(|v| v.apply(paths, operator))
                .unwrap_or(Err(JsonError::InvalidOperation(
                    "Operation can only apply on array or object".into(),
                )))
        } else {
            match &operator {
                Operator::AddNumber(v) => {
                    if let Some(old_v) = target_value {
                        old_v.apply(paths, operator)
                    } else {
                        self.insert(k.clone(), v.clone());
                        Ok(())
                    }
                }
                Operator::ObjectInsert(v) => {
                    self.insert(k.clone(), v.clone());
                    Ok(())
                }
                Operator::ObjectDelete(delete_v) => {
                    if let Some(target_v) = target_value {
                        if target_v.eq(&delete_v) {
                            self.remove(k);
                        }
                    }
                    Ok(())
                }
                Operator::ObjectReplace(old_v, new_v) => {
                    if let Some(target_v) = target_value {
                        if target_v.eq(&old_v) {
                            self.insert(k.clone(), new_v.clone());
                        }
                    }
                    Ok(())
                }
                _ => Err(JsonError::InvalidOperation(
                    "Operation can only apply on array or object".into(),
                )),
            }
        }
    }
}

impl Appliable for Vec<serde_json::Value> {
    fn apply(&mut self, paths: Paths, operator: Operator) -> Result<()> {
        let index = paths.first_index_path()?;
        let target_value = self.get_mut(*index);
        if paths.len() > 1 {
            target_value
                .map(|v| v.apply(paths, operator))
                .unwrap_or(Err(JsonError::InvalidOperation(
                    "Operation can only apply on array or object".into(),
                )))
        } else {
            match &operator {
                Operator::AddNumber(v) => {
                    if let Some(old_v) = target_value {
                        match old_v {
                            Value::Number(n) => {
                                let new_v = n.as_u64().unwrap() + v.as_u64().unwrap();
                                let serde_v = serde_json::to_value(new_v)?;
                                self[*index] = serde_v;
                                Ok(())
                            }
                            _ => {
                                return Err(JsonError::InvalidOperation(
                                    "Operation can only apply on array or object".into(),
                                ))
                            }
                        }
                    } else {
                        self[*index] = v.clone();
                        Ok(())
                    }
                }
                Operator::ListInsert(v) => {
                    self[*index] = v.clone();
                    Ok(())
                }
                Operator::ListDelete(delete_v) => {
                    if let Some(target_v) = target_value {
                        if target_v.eq(&delete_v) {
                            self.remove(*index);
                        }
                    }
                    Ok(())
                }
                Operator::ListReplace(old_v, new_v) => {
                    if let Some(target_v) = target_value {
                        if target_v.eq(&old_v) {
                            self[*index] = new_v.clone();
                        }
                    }
                    Ok(())
                }
                Operator::ListMove(new_index) => {
                    if let Some(target_v) = target_value {
                        self[*new_index] = target_v.clone();
                        self[*index] = Value::Null;
                    }
                    Ok(())
                }
                _ => Err(JsonError::InvalidOperation(
                    "Operation can only apply on array or object".into(),
                )),
            }
        }
    }
}

pub type Operation = Vec<OperationComponent>;

pub struct JSON {
    value: Value,
}

impl JSON {
    pub fn from_str(input: &str) -> Result<JSON> {
        let value = serde_json::from_str(input)?;
        Ok(JSON { value })
    }

    pub fn apply(&mut self, operations: Vec<Operation>) -> Result<()> {
        for operation in operations {
            for op_comp in operation {
                self.value.apply(op_comp.paths, op_comp.operator)?;
            }
        }
        Ok(())
    }

    fn get(&self, paths: &Paths) -> Result<Option<&Value>> {
        self.value.route_get(paths)
    }

    fn get_mut_by_paths(&mut self, paths: Paths) {
        // self.value.route_get(paths)
        // let mut v = &mut self.value;
        // for p in paths {
        //     match p {
        //         Path::Index(i) => {
        //             if let &mut Value::Array(array) = v {
        //                 if let Some(v2) = array.get_mut(i) {
        //                     v = v2;
        //                 }
        //             } else {
        //             }
        //         }
        //         Path::Key(k) => {}
        //     }
        // }
    }

    fn is_valid_operations(op: Operation) {}
}

#[cfg(test)]
mod tests {

    use std::{
        io::{Read, Write},
        str::FromStr,
        vec,
    };

    use super::*;
    use test_log::test;

    #[test]
    fn test_parse_invalid_path() {
        assert_matches!(
            Paths::from_str("]").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Paths::from_str("[").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Paths::from_str("").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Paths::from_str("[]").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Paths::from_str("hello").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Paths::from_str("[hello]").unwrap_err(),
            JsonError::InvalidPathFormat
        );
    }

    #[test]
    fn test_parse_index_path() {
        let paths = Paths::from_str("[1]").unwrap();
        assert_eq!(1, paths.len());
        assert_eq!(1, *paths.first_index_path().unwrap());
        let paths = Paths::from_str("[2, 3, 4]").unwrap();
        assert_eq!(3, paths.len());
        assert_eq!(2, *paths.first_index_path().unwrap());
        let paths = paths.next_level();
        assert_eq!(2, paths.len());
        assert_eq!(3, *paths.first_index_path().unwrap());
        let paths = paths.next_level();
        assert_eq!(1, paths.len());
        assert_eq!(4, *paths.first_index_path().unwrap());
        let paths = paths.next_level();
        assert!(paths.is_empty());
    }

    #[test]
    fn test_parse_key_path() {
        let paths = Paths::from_str("[\"hello\"]").unwrap();
        assert_eq!(1, paths.len());
        assert_eq!("hello", paths.first_key_path().unwrap());
        let paths = Paths::from_str("[\"hello\", \"word\", \"hello\"]").unwrap();
        assert_eq!(3, paths.len());
        assert_eq!("hello", paths.first_key_path().unwrap());
        let paths = paths.next_level();
        assert_eq!(2, paths.len());
        assert_eq!("word", paths.first_key_path().unwrap());
        let paths = paths.next_level();
        assert_eq!(1, paths.len());
        assert_eq!("hello", paths.first_key_path().unwrap());
        let paths = paths.next_level();
        assert!(paths.is_empty());
    }

    #[test]
    fn test_parse_path_with_blanks() {
        let paths = Paths::from_str("[ \"hello \"  ,  1,  \"  world \",  4  ]").unwrap();
        assert_eq!(4, paths.len());
        assert_eq!("hello ", paths.first_key_path().unwrap());
        let paths = paths.next_level();
        assert_eq!(3, paths.len());
        assert_eq!(1, *paths.first_index_path().unwrap());
        let paths = paths.next_level();
        assert_eq!(2, paths.len());
        assert_eq!("  world ", paths.first_key_path().unwrap());
        let paths = paths.next_level();
        assert_eq!(4, *paths.first_index_path().unwrap());
        let paths = paths.next_level();
        assert!(paths.is_empty());
    }

    #[test]
    fn test_apply_add_number() {
        let mut json = JSON::from_str("{\"level1\": 10}").unwrap();
        let paths = Paths::from_str("[\"level1\"]").unwrap();
        json.apply(vec![vec![OperationComponent {
            paths: paths.clone(),
            operator: Operator::AddNumber(serde_json::to_value(100).unwrap()),
        }]])
        .unwrap();

        assert_eq!(json.get(&paths).unwrap().unwrap().as_u64().unwrap(), 110);
    }
}
