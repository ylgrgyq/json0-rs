use std::{collections::BTreeMap, hash::Hash, hash::Hasher, vec};

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

    fn route_get_mut<T>(&mut self, paths: &Paths) -> Result<&mut Value>;

    fn route_insert<T>(&mut self, paths: &Paths, value: T) -> Result<()>;

    fn route_delete<T>(&mut self, paths: &Paths, value: Value) -> Result<()>;

    fn route_replace<T>(&mut self, paths: &Paths, value: Value) -> Result<()>;
}

impl Routable for serde_json::Value {
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

    fn route_get_mut<T>(&mut self, paths: &Paths) -> Result<&mut Value> {
        match self {
            Value::Array(array) => array.route_get_mut(paths),
            Value::Object(obj) => obj.route_get_mut(paths),
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

    fn route_insert<T>(&mut self, paths: &Paths, value: T) -> Result<()> {
        todo!()
    }

    fn route_delete<T>(&mut self, paths: &Paths, value: Value) -> Result<()> {
        todo!()
    }

    fn route_replace<T>(&mut self, paths: &Paths, value: Value) -> Result<()> {
        todo!()
    }
}

impl Routable for serde_json::Map<String, serde_json::Value> {
    fn route_get<T>(&self, paths: &Paths) -> Result<Option<T>> {
        todo!()
    }

    fn route_get_mut<T>(&mut self, paths: &Paths) -> Result<&mut Value> {
        todo!()
    }

    fn route_insert<T>(&mut self, paths: &Paths, value: T) -> Result<()> {
        todo!()
    }

    fn route_delete<T>(&mut self, paths: &Paths, value: Value) -> Result<()> {
        todo!()
    }

    fn route_replace<T>(&mut self, paths: &Paths, value: Value) -> Result<()> {
        todo!()
    }
}

impl Routable for Vec<serde_json::Value> {
    fn route_get<T>(&self, paths: &Paths) -> Result<Option<T>> {
        todo!()
    }

    fn route_get_mut<T>(&mut self, paths: &Paths) -> Result<&mut Value> {
        todo!()
    }

    fn route_insert<T>(&mut self, paths: &Paths, value: T) -> Result<()> {
        todo!()
    }

    fn route_delete<T>(&mut self, paths: &Paths, value: Value) -> Result<()> {
        todo!()
    }

    fn route_replace<T>(&mut self, paths: &Paths, value: Value) -> Result<()> {
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

impl OperationComponent {
    pub fn apply(&self, json: JSON) {}

    pub fn get_prefix_path(&self) -> Paths {
        let mut paths = self.get_paths();
        if paths.len() < 2 {
            return vec![];
        }

        paths.pop();
        paths
    }

    pub fn get_paths(&self) -> Paths {
        match self {
            Self::AddNumber(paths, _) => paths.to_vec(),
            Self::ListInsert(paths, _) => paths.to_vec(),
            Self::ListDelete(paths, _) => paths.to_vec(),
            Self::ListReplace(paths, _, _) => paths.to_vec(),
            Self::ListMove(paths, _) => paths.to_vec(),
            Self::ObjectInsert(paths, _) => paths.to_vec(),
            Self::ObjectDelete(paths, _) => paths.to_vec(),
            Self::ObjectReplace(paths, _, _) => paths.to_vec(),
        }
    }
}

struct Operation {
    operation_components: Vec<OperationComponent>,
}

struct JSON {
    value: Value,
}

trait Component {}

impl JSON {
    pub fn apply(&mut self, operations: Vec<Operation>) {
        for op in operations {

            // op.apply(self);
        }
    }

    fn get_mut_by_paths(&mut self, paths: Paths) {
        let mut v = &mut self.value;
        for p in paths {
            match p {
                Path::Index(i) => {
                    if let &mut Value::Array(array) = v {
                        if let Some(v2) = array.get_mut(i) {
                            v = v2;
                        }
                    } else {
                    }
                }
                Path::Key(k) => {}
            }
        }
    }
}
