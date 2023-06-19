use std::{collections::BTreeMap, hash::Hash, hash::Hasher, mem, vec};

use crate::error::{JsonError, Result};
use serde::de::DeserializeOwned;
use serde_json::{Number, Value};

#[derive(Clone)]
enum Path {
    Index(usize),
    Key(String),
}

type Paths = Vec<Path>;

trait Routable {
    fn route_get(&self, paths: &Paths) -> Result<Option<Value>>;

    fn route_get_mut(&mut self, paths: &Paths) -> Result<Option<&mut Value>>;

    fn route_insert(&mut self, paths: &Paths, value: Value) -> Result<()>;

    fn route_delete(&mut self, paths: &Paths, value: Value) -> Result<()>;

    fn route_replace(&mut self, paths: &Paths, value: Value) -> Result<()>;
}

impl Routable for Value {
    fn route_get(&self, paths: &Paths) -> Result<Option<Value>> {
        match self {
            Value::Array(array) => array.route_get(paths),
            Value::Object(obj) => obj.route_get(paths),
            Value::Null => Ok(None),
            _ => {
                if paths.is_empty() {
                    Ok(Some(self.to_owned()))
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
    fn route_get(&self, paths: &Paths) -> Result<Option<Value>> {
        todo!()
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
    fn route_get(&self, paths: &Paths) -> Result<Option<Value>> {
        todo!()
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

enum OperationComponent {
    AddNumber(Paths, Number),
    ListInsert(Paths, Value),
    ListDelete(Paths, Value),
    ListReplace(Paths, Value, Value),
    ListMove(Paths, usize),
    ObjectInsert(Paths, Value),
    ObjectDelete(Paths, Value),
    ObjectReplace(Paths, Value, Value),
}

trait OperationComponentAppliable {
    fn apply(&mut self, operation_component: OperationComponent) -> Result<()>;
}

impl OperationComponentAppliable for Value {
    fn apply(&mut self, operation_component: OperationComponent) -> Result<()> {
        todo!()
    }
}

impl OperationComponentAppliable for serde_json::Map<String, serde_json::Value> {
    fn apply(&mut self, operation_component: OperationComponent) -> Result<()> {
        todo!()
    }
}

impl OperationComponentAppliable for Vec<serde_json::Value> {
    fn apply(&mut self, operation_component: OperationComponent) -> Result<()> {
        todo!()
    }
}

type Operation = Vec<OperationComponent>;

trait Validation {
    fn is_valid(&self) -> bool;
}

impl Validation for OperationComponent {
    fn is_valid(&self) -> bool {
        todo!()
    }
}

impl Validation for Operation {
    fn is_valid(&self) -> bool {
        self.iter().all(|o| o.is_valid())
    }
}

pub struct JSON {
    value: Value,
}

impl JSON {
    pub fn apply(&mut self, operations: Vec<Operation>) -> Result<()> {
        for operation in operations {
            for op_comp in operation {
                self.value.apply(op_comp)?;
            }
        }
        Ok(())
    }

    fn get_mut_by_paths(&mut self, paths: Paths) {
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
        vec,
    };

    use super::*;
    use test_log::test;

    #[test]
    fn test_create_file() {
        let dir = get_temporary_directory_path();
        let file_id = Some(123);
        let file_path = FileType::DataFile.get_path(&dir, file_id);
        assert!(!file_path.exists());
        create_file(&dir, FileType::DataFile, file_id).unwrap();
        assert!(file_path.exists());
    }
}
