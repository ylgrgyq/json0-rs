use std::{collections::BTreeMap, hash::Hash, hash::Hasher, mem, vec};

use crate::error::{JsonError, Result};
use serde::de::DeserializeOwned;
use serde_json::{Number, Value};

trait Validation {
    fn is_valid(&self) -> bool;
}

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
    AddNumber(Paths, Value),
    ListInsert(Paths, Value),
    ListDelete(Paths, Value),
    ListReplace(Paths, Value, Value),
    ListMove(Paths, usize),
    ObjectInsert(Paths, Value),
    ObjectDelete(Paths, Value),
    ObjectReplace(Paths, Value, Value),
}

impl OperationComponent {
    fn next_level(&mut self) {
        self.get_mut_paths().remove(0);
    }
}

trait HasPaths {
    fn get_paths(&self) -> &Paths;

    fn get_mut_paths(&mut self) -> &mut Paths;
}

impl HasPaths for OperationComponent {
    fn get_paths(&self) -> &Paths {
        match self {
            OperationComponent::AddNumber(paths, _) => paths,
            OperationComponent::ListInsert(paths, _) => paths,
            OperationComponent::ListDelete(paths, _) => paths,
            OperationComponent::ListReplace(paths, _, _) => paths,
            OperationComponent::ListMove(paths, _) => paths,
            OperationComponent::ObjectInsert(paths, _) => paths,
            OperationComponent::ObjectDelete(paths, _) => paths,
            OperationComponent::ObjectReplace(paths, _, _) => paths,
        }
    }

    fn get_mut_paths(&mut self) -> &mut Paths {
        match self {
            OperationComponent::AddNumber(paths, _) => paths,
            OperationComponent::ListInsert(paths, _) => paths,
            OperationComponent::ListDelete(paths, _) => paths,
            OperationComponent::ListReplace(paths, _, _) => paths,
            OperationComponent::ListMove(paths, _) => paths,
            OperationComponent::ObjectInsert(paths, _) => paths,
            OperationComponent::ObjectDelete(paths, _) => paths,
            OperationComponent::ObjectReplace(paths, _, _) => paths,
        }
    }
}

trait OperationComponentAppliable {
    fn apply(&mut self, operation_component: OperationComponent) -> Result<()>;
}

impl OperationComponentAppliable for Value {
    fn apply(&mut self, operation_component: OperationComponent) -> Result<()> {
        match self {
            Value::Array(array) => array.apply(operation_component),
            Value::Object(obj) => obj.apply(operation_component),
            _ => {
                return Err(JsonError::InvalidOperation(
                    "Operation can only apply on array or object".into(),
                ));
            }
        }
    }
}

impl OperationComponentAppliable for serde_json::Map<String, serde_json::Value> {
    fn apply(&mut self, mut operation_component: OperationComponent) -> Result<()> {
        let paths = operation_component.get_paths();
        let first_path = paths.first();
        if first_path.is_none() {
            return Err(JsonError::InvalidOperation(
                "Operation can only apply on array or object".into(),
            ));
        }

        match first_path.unwrap() {
            Path::Index(_) => {
                return Err(JsonError::InvalidOperation(
                    "Operation can only apply on array or object".into(),
                ))
            }
            Path::Key(k) => {
                let target_value = self.get_mut(k);
                if paths.len() > 1 {
                    if target_value.is_none() {
                        return Err(JsonError::InvalidOperation(
                            "Operation can only apply on array or object".into(),
                        ));
                    }
                    operation_component.next_level();
                    target_value.unwrap().apply(operation_component)
                } else {
                    match &operation_component {
                        OperationComponent::AddNumber(_, v) => {
                            if let Some(old_v) = target_value {
                                match old_v {
                                    Value::Number(n) => {
                                        let new_v = n.as_u64().unwrap() + v.as_u64().unwrap();
                                        let serde_v = serde_json::to_value(new_v)?;
                                        self.insert(k.clone(), serde_v);
                                        Ok(())
                                    }
                                    _ => {
                                        return Err(JsonError::InvalidOperation(
                                            "Operation can only apply on array or object".into(),
                                        ))
                                    }
                                }
                            } else {
                                self.insert(k.clone(), v.clone());
                                Ok(())
                            }
                        }
                        OperationComponent::ObjectInsert(_, v) => {
                            self.insert(k.clone(), v.clone());
                            Ok(())
                        }
                        OperationComponent::ObjectDelete(_, delete_v) => {
                            if target_value.is_some() && target_value.unwrap().eq(&delete_v) {
                                self.remove(k);
                            }
                            Ok(())
                        }
                        OperationComponent::ObjectReplace(_, old_v, new_v) => {
                            if target_value.is_some() && target_value.unwrap().eq(&old_v) {
                                self.insert(k.clone(), new_v.clone());
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
    }
}

impl OperationComponentAppliable for Vec<serde_json::Value> {
    fn apply(&mut self, operation_component: OperationComponent) -> Result<()> {
        todo!()
    }
}

type Operation = Vec<OperationComponent>;

pub struct JSON {
    value: Value,
}

impl JSON {
    pub fn from_str(input: &str) -> JSON {
        let value = serde_json::json!(input);
        JSON { value }
    }

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
        let mut a = JSON::from_str("{'level1': 10}");
        a.apply(vec![vec![OperationComponent::AddNumber(
            vec![Path::Key("level1".into())],
            serde_json::to_value(100).unwrap(),
        )]]);
    }
}
