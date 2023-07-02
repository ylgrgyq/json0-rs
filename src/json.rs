use std::{collections::BTreeMap, error, fmt::Display, hash::Hash, hash::Hasher, mem, vec};

use crate::{
    error::{JsonError, Result},
    path::Paths,
};
use log::info;
use serde::de::DeserializeOwned;
use serde_json::{Map, Number, Value};

trait Validation {
    fn is_valid(&self) -> bool;
}

trait Routable {
    fn route_get(&self, paths: &Paths) -> Result<Option<&Value>>;

    fn route_get_mut(&mut self, paths: &Paths) -> Result<Option<&mut Value>>;
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
}

impl Routable for serde_json::Map<String, serde_json::Value> {
    fn route_get(&self, paths: &Paths) -> Result<Option<&Value>> {
        let k = paths.first_key_path()?;
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

    fn route_get_mut(&mut self, paths: &Paths) -> Result<Option<&mut Value>> {
        let k = paths.first_key_path()?;
        if let Some(v) = self.get_mut(k) {
            v.route_get_mut(&paths.next_level())
        } else {
            Ok(None)
        }
    }
}

impl Routable for Vec<serde_json::Value> {
    fn route_get(&self, paths: &Paths) -> Result<Option<&Value>> {
        let i = paths.first_index_path()?;
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

    fn route_get_mut(&mut self, paths: &Paths) -> Result<Option<&mut Value>> {
        let i = paths.first_index_path()?;
        if let Some(v) = self.get_mut(*i) {
            v.route_get_mut(&paths.next_level())
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone)]
enum Operator {
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

#[derive(Clone, Debug)]
pub struct OperationComponent {
    paths: Paths,
    operator: Operator,
}

impl OperationComponent {
    pub fn from_str(input: &str) -> Result<OperationComponent> {
        let json_value: Value = serde_json::from_str(input)?;
        let path_value = json_value.get("p");

        if path_value.is_none() {
            return Err(JsonError::InvalidOperation("Missing path".into()));
        }

        let paths = Paths::from_json_value(path_value.unwrap())?;
        let operator = Operator::from_json_value(&json_value)?;

        Ok(OperationComponent { paths, operator })
    }

    pub fn get_paths(&self) -> &Paths {
        &self.paths
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
    fn apply(&mut self, paths: Paths, operator: Operator) -> Result<()> {
        let k = paths.first_key_path()?;
        let target_value = self.get_mut(k);
        if paths.len() > 1 {
            let next_paths = paths.next_level();
            target_value
                .map(|v| v.apply(next_paths, operator))
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
                Operator::ObjectReplace(new_v, old_v) => {
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
            let next_paths = paths.next_level();
            target_value
                .map(|v| v.apply(next_paths, operator))
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
                    if *index > self.len() {
                        self.push(v.clone())
                    } else {
                        self.insert(*index, v.clone());
                    }
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
                Operator::ListReplace(new_v, old_v) => {
                    if let Some(target_v) = target_value {
                        if target_v.eq(&old_v) {
                            self[*index] = new_v.clone();
                        }
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
                _ => Err(JsonError::InvalidOperation(
                    "Operation can only apply on array or object".into(),
                )),
            }
        }
    }
}

pub type Operation = Vec<OperationComponent>;

#[derive(Clone)]
pub struct JSON {
    value: Value,
}

impl Display for JSON {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
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
    use log::info;
    use test_log::test;

    #[test]
    fn test_route_get_by_path_only_has_object() {
        let json = JSON::from_str(r#"{"level1":"world", "level12":{"level2":"world2"}}"#).unwrap();

        // simple path with only object
        let paths = Paths::from_str(r#"["level1"]"#).unwrap();
        assert_eq!(json.get(&paths).unwrap().unwrap().to_string(), r#""world""#);
        let paths = Paths::from_str(r#"["level12", "level2"]"#).unwrap();
        assert_eq!(
            json.get(&paths).unwrap().unwrap().to_string(),
            r#""world2""#
        );
        let paths = Paths::from_str(r#"["level3"]"#).unwrap();
        assert!(json.get(&paths).unwrap().is_none());

        // complex path with array
        let json =
            JSON::from_str(r#"{"level1":[1,{"hello":[1,[7,8]]}], "level12":"world"}"#).unwrap();
        let paths = Paths::from_str(r#"["level1", 1, "hello"]"#).unwrap();

        assert_eq!(
            json.get(&paths).unwrap().unwrap().to_string(),
            r#"[1,[7,8]]"#
        );
    }

    #[test]
    fn test_route_get_by_path_has_array() {
        let json = JSON::from_str(r#"{"level1":["a","b"], "level12":[123, {"level2":["c","d"]}]}"#)
            .unwrap();
        // simple path
        let paths = Paths::from_str(r#"["level1", 1]"#).unwrap();
        assert_eq!(json.get(&paths).unwrap().unwrap().to_string(), r#""b""#);
        let paths = Paths::from_str(r#"["level12", 0]"#).unwrap();

        // complex path
        assert_eq!(json.get(&paths).unwrap().unwrap().to_string(), r#"123"#);
        let paths = Paths::from_str(r#"["level12", 1, "level2"]"#).unwrap();
        assert_eq!(
            json.get(&paths).unwrap().unwrap().to_string(),
            r#"["c","d"]"#
        );
        let json =
            JSON::from_str(r#"{"level1":[1,{"hello":[1,[7,8]]}], "level12":"world"}"#).unwrap();
        let paths = Paths::from_str(r#"["level1", 1, "hello", 1]"#).unwrap();

        assert_eq!(json.get(&paths).unwrap().unwrap().to_string(), r#"[7,8]"#);
    }

    #[test]
    fn test_apply_add_number() {
        let mut json = JSON::from_str("{\"level1\": 10}").unwrap();
        let operation_comp =
            OperationComponent::from_str("{\"p\":[\"level1\"], \"na\":100}").unwrap();
        json.apply(vec![vec![operation_comp.clone()]]).unwrap();

        assert_eq!(json.to_string(), r#"{"level1":110}"#);
    }

    #[test]
    fn test_object_insert() {
        let mut json = JSON::from_str(r#"{}"#).unwrap();
        // insert to empty object
        let operation_comp =
            OperationComponent::from_str(r#"{"p":["level1"], "oi":{"level2":{}}}"#).unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(json.to_string(), r#"{"level1":{"level2":{}}}"#);

        // insert to inner object
        let operation_comp = OperationComponent::from_str(
            r#"{"p":["level1", "level2"], "oi":{"level3":[1, {"level4":{}}]}}"#,
        )
        .unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(
            json.to_string(),
            r#"{"level1":{"level2":{"level3":[1,{"level4":{}}]}}}"#
        );

        // insert to deep inner object with number index in path
        let operation_comp = OperationComponent::from_str(
            r#"{"p":["level1", "level2", "level3", 1, "level4"], "oi":{"level5":[1, 2]}}"#,
        )
        .unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(
            json.to_string(),
            r#"{"level1":{"level2":{"level3":[1,{"level4":{"level5":[1,2]}}]}}}"#
        );

        // replace key without compare
        let operation_comp = OperationComponent::from_str(
            r#"{"p":["level1", "level2", "level3", 1, "level4"], "oi":[3,4]}"#,
        )
        .unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(
            json.to_string(),
            r#"{"level1":{"level2":{"level3":[1,{"level4":[3,4]}]}}}"#
        );
    }

    #[test]
    fn test_object_delete() {
        let origin_json = JSON::from_str(
            r#"{"level1":{"level2":{"level3":[1,{"level41":[1,2], "level42":[3,4]}]}}}"#,
        )
        .unwrap();

        // delete to deep inner object with number index in path
        let mut json = origin_json.clone();
        let operation_comp = OperationComponent::from_str(
            r#"{"p":["level1", "level2", "level3", 1, "level41"], "od":[1, 2]}"#,
        )
        .unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(
            json.to_string(),
            r#"{"level1":{"level2":{"level3":[1,{"level42":[3,4]}]}}}"#
        );

        // delete to inner object
        let mut json = origin_json.clone();
        let operation_comp = OperationComponent::from_str(
            r#"{"p":["level1", "level2", "level3"], "od":[1,{"level41":[1,2], "level42":[3,4]}]}"#,
        )
        .unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(json.to_string(), r#"{"level1":{"level2":{}}}"#);
    }

    #[test]
    fn test_object_replace() {
        let origin_json = JSON::from_str(
            r#"{"level1":{"level2":{"level3":[1,{"level41":[1,2], "level42":[3,4]}]}}}"#,
        )
        .unwrap();

        // replace deep inner object with number index in path
        let mut json = origin_json.clone();
        let operation_comp = OperationComponent::from_str(
            r#"{"p":["level1", "level2", "level3", 1, "level41"], "oi":{"5":"6"}, "od":[1, 2]}"#,
        )
        .unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(
            json.to_string(),
            r#"{"level1":{"level2":{"level3":[1,{"level41":{"5":"6"},"level42":[3,4]}]}}}"#
        );

        // replace to inner object
        let mut json = origin_json.clone();
        let operation_comp = OperationComponent::from_str(
            r#"{"p":["level1", "level2"], "oi":"hello", "od":{"level3":[1,{"level41":[1,2], "level42":[3,4]}]}}"#,
        )
        .unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(json.to_string(), r#"{"level1":{"level2":"hello"}}"#);
    }

    #[test]
    fn test_list_insert() {
        let mut json = JSON::from_str(r#"{"level1": []}"#).unwrap();

        // insert to empty array
        let operation_comp =
            OperationComponent::from_str(r#"{"p":["level1", 0], "li":{"hello":[1]}}"#).unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(json.to_string(), r#"{"level1":[{"hello":[1]}]}"#);

        // insert to array
        let operation_comp =
            OperationComponent::from_str(r#"{"p":["level1", 0], "li":1}"#).unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(json.to_string(), r#"{"level1":[1,{"hello":[1]}]}"#);

        // insert to inner array
        let operation_comp =
            OperationComponent::from_str(r#"{"p":["level1", 1, "hello",1], "li":[7,8]}"#).unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(json.to_string(), r#"{"level1":[1,{"hello":[1,[7,8]]}]}"#);

        // append
        let operation_comp =
            OperationComponent::from_str(r#"{"p":["level1", 10], "li":[2,3]}"#).unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(
            json.to_string(),
            r#"{"level1":[1,{"hello":[1,[7,8]]},[2,3]]}"#
        );
    }

    #[test]
    fn test_list_delete() {
        let origin_json = JSON::from_str(r#"{"level1":[1,{"hello":[1,[7,8]]}]}"#).unwrap();

        // delete from innser array
        let mut json = origin_json.clone();
        let operation_comp =
            OperationComponent::from_str(r#"{"p":["level1", 1, "hello", 1], "ld":[7,8]}"#).unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(json.to_string(), r#"{"level1":[1,{"hello":[1]}]}"#);

        // delete from inner object
        let mut json = origin_json.clone();
        let operation_comp =
            OperationComponent::from_str(r#"{"p":["level1", 1], "ld":{"hello":[1,[7,8]]}}"#)
                .unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(json.to_string(), r#"{"level1":[1]}"#);
    }

    #[test]
    fn test_list_replace() {
        let origin_json = JSON::from_str(r#"{"level1":[1,{"hello":[1,[7,8]]}]}"#).unwrap();

        // replace from innser array
        let mut json = origin_json.clone();
        let operation_comp = OperationComponent::from_str(
            r#"{"p":["level1", 1, "hello", 1], "li":{"hello":"world"}, "ld":[7,8]}"#,
        )
        .unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(
            json.to_string(),
            r#"{"level1":[1,{"hello":[1,{"hello":"world"}]}]}"#
        );

        // replace from inner object
        let mut json = origin_json.clone();
        let operation_comp = OperationComponent::from_str(
            r#"{"p":["level1", 1], "li": {"hello":"world"}, "ld":{"hello":[1,[7,8]]}}"#,
        )
        .unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(json.to_string(), r#"{"level1":[1,{"hello":"world"}]}"#);
    }

    #[test]
    fn test_list_move() {
        let origin_json = JSON::from_str(r#"{"level1":[1,{"hello":[1,[7,8], 9, 10]}]}"#).unwrap();

        // move left
        let mut json = origin_json.clone();
        let operation_comp =
            OperationComponent::from_str(r#"{"p":["level1", 1, "hello", 2], "lm":1}"#).unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(
            json.to_string(),
            r#"{"level1":[1,{"hello":[1,9,[7,8],10]}]}"#
        );

        // move right
        let mut json = origin_json.clone();
        let operation_comp =
            OperationComponent::from_str(r#"{"p":["level1", 1, "hello", 1], "lm":2}"#).unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(
            json.to_string(),
            r#"{"level1":[1,{"hello":[1,9,[7,8],10]}]}"#
        );

        // stay put
        let mut json = origin_json.clone();
        let operation_comp =
            OperationComponent::from_str(r#"{"p":["level1", 1, "hello", 1], "lm":1}"#).unwrap();
        json.apply(vec![vec![operation_comp]]).unwrap();
        assert_eq!(
            json.to_string(),
            r#"{"level1":[1,{"hello":[1,[7,8],9,10]}]}"#
        );
    }
}
