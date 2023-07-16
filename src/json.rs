use std::{fmt::Display, mem, vec};

use crate::{
    common::Validation,
    error::{JsonError, Result},
    operation::{Appliable, OperationComponent, Operator},
    path::{Path, PathElement},
};

use serde_json::Value;

trait Routable {
    fn route_get(&self, paths: &Path) -> Result<Option<&Value>>;

    fn route_get_mut(&mut self, paths: &Path) -> Result<Option<&mut Value>>;
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
    fn apply(&mut self, paths: Path, operator: Operator) -> Result<()> {
        if paths.len() > 1 {
            let (left, right) = paths.split_at(paths.len() - 1);
            return self
                .route_get_mut(&left)?
                .ok_or(JsonError::BadPath)?
                .apply(right, operator);
        }
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
    fn apply(&mut self, paths: Path, operator: Operator) -> Result<()> {
        assert!(paths.len() == 1);

        let k = paths.first_key_path().ok_or(JsonError::BadPath)?;
        let target_value = self.get_mut(k);
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
            _ => Err(JsonError::BadPath),
        }
    }
}

impl Appliable for Vec<serde_json::Value> {
    fn apply(&mut self, paths: Path, operator: Operator) -> Result<()> {
        assert!(paths.len() == 1);

        let index = paths.first_index_path().ok_or(JsonError::BadPath)?;
        let target_value = self.get_mut(*index);
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
            _ => Err(JsonError::BadPath),
        }
    }
}

pub type Operation = Vec<OperationComponent>;

impl Validation for Vec<OperationComponent> {
    fn validates(&self) -> Result<()> {
        for op in self.iter() {
            op.validates()?;
        }
        Ok(())
    }
}

#[derive(PartialEq)]
pub enum TransformSide {
    LEFT,
    RIGHT,
}
pub struct Transformer {}

impl Transformer {
    pub fn transform(
        &self,
        operation: &Operation,
        base_operation: &Operation,
        side: TransformSide,
    ) -> Result<Operation> {
        if base_operation.is_empty() {
            return Ok(operation.clone());
        }

        operation.validates()?;
        base_operation.validates()?;

        if operation.len() == 1 && base_operation.len() == 1 {
            let o = self.transform_component(
                operation.get(0).unwrap(),
                base_operation.get(0).unwrap(),
                side,
            )?;
            return Ok(vec![o]);
        }

        if side == TransformSide::LEFT {
            Ok(self.do_transform(operation, base_operation, side)?.0)
        } else {
            Ok(self.do_transform(operation, base_operation, side)?.1)
        }
    }

    pub fn append(&self, operation: &mut Operation, op: &OperationComponent) -> Result<()> {
        op.validates()?;

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

        if operation.is_empty() {
            operation.push(op.clone());
            return Ok(());
        }

        let last = operation.last_mut().unwrap();
        if last.path.eq(&op.path) && last.merge(op) {
            if last.operator.eq(&Operator::Noop()) {
                operation.pop();
            }
            return Ok(());
        }
        operation.push(op.clone());
        Ok(())
    }

    pub fn invert(&self, operation: &OperationComponent) -> Result<OperationComponent> {
        operation.validates()?;

        let mut path = operation.path.clone();
        let operator = match &operation.operator {
            Operator::Noop() => Operator::Noop(),
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
        Ok(OperationComponent::new(path, operator))
    }

    pub fn compose(&self, a: &Operation, b: &Operation) -> Result<Operation> {
        a.validates()?;

        let mut ret: Operation = a.clone();
        for op in b.iter() {
            self.append(&mut ret, &op)?;
        }

        Ok(ret)
    }

    fn do_transform(
        &self,
        operation: &Operation,
        base_operation: &Operation,
        side: TransformSide,
    ) -> Result<(Operation, Operation)> {
        todo!()
    }

    fn transform_component(
        &self,
        new_op: &OperationComponent,
        base_op: &OperationComponent,
        side: TransformSide,
    ) -> Result<OperationComponent> {
        let mut new_op = new_op.clone();

        let max_common_path = base_op.path.max_common_path(&new_op.path);
        if max_common_path.is_empty() {
            // new_op and base_op does not have common path
            return Ok(new_op);
        }

        let new_operate_path = new_op.operate_path();
        let base_operate_path = base_op.operate_path();
        if max_common_path.len() < new_operate_path.len()
            && max_common_path.len() < base_operate_path.len()
        {
            // common path must be equal to new_op's or base_op's operate path
            // or base_op and new_op is operating on orthogonal value
            // they don't need transform
            return Ok(new_op);
        }

        // such as:
        // new_op, base_op
        // [p1,p2,p3], [p1,p2,p4,p5]
        // [p1,p2,p3], [p1,p2,p3,p5]
        if base_operate_path.len() > new_operate_path.len() {
            // if base_op's path is longger and contains new_op's path, new_op should include base_op's effect
            if new_op.path.is_prefix_of(&base_op.path) {
                new_op.consume(&max_common_path, &base_op)?;
            }
            return Ok(new_op);
        }

        // from here, base_op's path is shorter or equal to new_op, such as:
        // new_op, base_op
        // [p1,p2,p3], [p1,p2,p3]. same operand and base_op is prefix of new_op
        // [p1,p2,p4], [p1,p2,p3]. same operand
        // [p1,p2,p3,p4,..], [p1,p2,p3], base_op is prefix of new_op
        // [p1,p2,p4,p5,..], [p1,p2,p3]
        let same_operand = base_op.path.len() == new_op.path.len();
        let base_op_is_prefix = base_op.path.is_prefix_of(&new_op.path);
        match &base_op.operator {
            Operator::ListReplace(li_v, _) => {
                if base_op_is_prefix {
                    if same_operand && side == TransformSide::LEFT {
                        if let Operator::ListReplace(new_li, _) = &new_op.operator {
                            return Ok(OperationComponent::new(
                                new_op.path.clone(),
                                Operator::ListReplace(new_li.clone(), li_v.clone()),
                            ));
                        }
                    }
                    return Ok(new_op.noop());
                }
            }
            Operator::ListInsert(_) => {
                if let Operator::ListInsert(_) = &new_op.operator {
                    if same_operand && base_op_is_prefix {
                        if side == TransformSide::RIGHT {
                            new_op.increase_last_index_path();
                        }
                        return Ok(new_op);
                    }
                }

                if base_op.path.last().unwrap() <= new_op.path.last().unwrap() {
                    new_op.increase_last_index_path();
                }

                if let Operator::ListMove(i) = &mut new_op.operator {
                    if same_operand && base_op.path.last().unwrap() <= &PathElement::Index(*i) {
                        new_op.operator = Operator::ListMove(*i + 1);
                    }
                }
            }
            Operator::ListDelete(_) => {
                let base_op_operate_path = base_op.path.get(new_operate_path.len()).unwrap();
                let new_op_operate_path = new_op.path.get(new_operate_path.len()).unwrap();
                if let Operator::ListMove(lm) = new_op.operator {
                    if same_operand {
                        if base_op_is_prefix {
                            // base_op deleted the thing we're trying to move
                            return Ok(new_op.noop());
                        }
                        let to = lm.into();
                        if base_op_operate_path < &to
                            || (base_op_operate_path.eq(&to) && new_op_operate_path < &to)
                        {
                            new_op.operator = Operator::ListMove(lm - 1);
                        }
                    }
                }

                if base_op_operate_path < new_op_operate_path {
                    new_op.decrease_last_index_path();
                } else if base_op_is_prefix {
                    if !same_operand {
                        // we're below the deleted element, so -> noop
                        return Ok(new_op.noop());
                    }
                    if let Operator::ListDelete(_) = new_op.operator {
                        // we're trying to delete the same element, -> noop
                        return Ok(new_op.noop());
                    }
                    if let Operator::ListReplace(li, _) = new_op.operator {
                        // we're replacing, they're deleting. we become an insert.
                        return Ok(OperationComponent::new(
                            new_op.path.clone(),
                            Operator::ListInsert(li.clone()),
                        ));
                    }
                }
            }
            Operator::ObjectReplace(oi, _) => {
                if base_op_is_prefix {
                    if !same_operand {
                        return Ok(new_op.noop());
                    }

                    match &new_op.operator {
                        Operator::ObjectReplace(new_oi, _) | Operator::ObjectInsert(new_oi) => {
                            if side == TransformSide::RIGHT {
                                return Ok(new_op.noop());
                            }
                            return Ok(OperationComponent {
                                path: new_op.path.clone(),
                                operator: Operator::ListReplace(new_oi.clone(), oi.clone()),
                            });
                        }
                        _ => {
                            return Ok(new_op.noop());
                        }
                    }
                }
            }
            Operator::ObjectInsert(base_oi) => {
                if base_op_is_prefix {
                    if let Operator::ObjectReplace(new_oi, _) | Operator::ObjectInsert(new_oi) =
                        &new_op.operator
                    {
                        if side == TransformSide::LEFT {
                            return Ok(OperationComponent {
                                path: new_op.path.clone(),
                                operator: Operator::ObjectReplace(new_oi.clone(), base_oi.clone()),
                            });
                        } else {
                            return Ok(new_op.noop());
                        }
                    }
                }
            }
            Operator::ObjectDelete(_) => {
                if base_op_is_prefix {
                    if !same_operand {
                        return Ok(new_op.noop());
                    }
                    if let Operator::ObjectReplace(new_oi, _) | Operator::ObjectInsert(new_oi) =
                        &new_op.operator
                    {
                        return Ok(OperationComponent {
                            path: new_op.path.clone(),
                            operator: Operator::ObjectInsert(new_oi.clone()),
                        });
                    } else {
                        return Ok(new_op.noop());
                    }
                }
            }
            Operator::ListMove(lm) => {
                if same_operand {
                    match &mut new_op.operator {
                        Operator::ListMove(new_op_lm) => {
                            let from = new_op.path.get(new_operate_path.len()).unwrap().clone();
                            let to = new_op.path.get(*lm).unwrap().clone();
                            let other_from = base_op.path.get(new_operate_path.len()).unwrap();
                            let other_to = base_op.path.get(*lm).unwrap();
                            if other_from != other_to {
                                if &from == other_from {
                                    if side == TransformSide::LEFT {
                                        new_op
                                            .path
                                            .replace(new_operate_path.len(), other_to.clone());
                                    } else {
                                        return Ok(new_op.noop());
                                    }
                                } else {
                                    let n_lm = *new_op_lm;
                                    if &from > other_from {
                                        new_op.decrease_last_index_path();
                                    }
                                    if &from > other_to {
                                        new_op.increase_last_index_path();
                                    } else if &from == other_to {
                                        if other_from > other_to {
                                            new_op.increase_last_index_path();
                                        }
                                        if from == to {
                                            new_op.operator = Operator::ListMove(n_lm + 1);
                                        }
                                    }
                                    if &to > other_from {
                                        new_op.operator = Operator::ListMove(n_lm - 1);
                                    } else if &to == other_from {
                                        if to > from {
                                            new_op.operator = Operator::ListMove(n_lm - 1);
                                        }
                                    }
                                    if &to > other_to {
                                        new_op.operator = Operator::ListMove(n_lm + 1);
                                    } else if &to == other_to {
                                        if (other_to > other_from && to > from)
                                            || (other_to < other_from && to < from)
                                        {
                                            if side == TransformSide::RIGHT {
                                                new_op.operator = Operator::ListMove(n_lm + 1);
                                            }
                                        } else {
                                            if to > from {
                                                new_op.operator = Operator::ListMove(n_lm + 1);
                                            } else if &to == other_from {
                                                new_op.operator = Operator::ListMove(n_lm - 1);
                                            }
                                        }
                                    }
                                }
                            }
                            return Ok(new_op);
                        }
                        Operator::ListInsert(_) => {
                            let from = base_op.path.get(new_operate_path.len()).unwrap();
                            let to = base_op.path.get(*lm).unwrap();
                            let p = new_op.path.get(new_operate_path.len()).unwrap().clone();
                            if &p > from {
                                new_op.decrease_last_index_path();
                            }
                            if &p > to {
                                new_op.increase_last_index_path();
                            }
                            return Ok(new_op);
                        }
                        _ => {}
                    }
                }
                let from = base_op.path.get(new_operate_path.len()).unwrap();
                let to = base_op.path.get(*lm).unwrap();
                let p = new_op.path.get(new_operate_path.len()).unwrap().clone();
                if &p == from {
                    new_op.path.replace(new_operate_path.len(), to.clone());
                } else {
                    if &p > from {
                        new_op.decrease_last_index_path();
                    }
                    if &p > to {
                        new_op.increase_last_index_path();
                    } else if &p == to && from > to {
                        new_op.increase_last_index_path();
                    }
                }
            }
            _ => {}
        }

        Ok(new_op)
    }
}

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
                self.value
                    .apply(op_comp.path.clone(), op_comp.operator.clone())?;
            }
        }
        Ok(())
    }

    pub fn get(&self, paths: &Path) -> Result<Option<&Value>> {
        self.value.route_get(paths)
    }
}

#[cfg(test)]
mod tests {

    use std::{
        io::{Read, Write},
        str::FromStr,
        vec,
    };

    use crate::path::Path;

    use super::*;
    use log::info;
    use test_log::test;

    #[test]
    fn test_route_get_by_path_only_has_object() {
        let json = JSON::from_str(r#"{"level1":"world", "level12":{"level2":"world2"}}"#).unwrap();

        // simple path with only object
        let paths = Path::from_str(r#"["level1"]"#).unwrap();
        assert_eq!(json.get(&paths).unwrap().unwrap().to_string(), r#""world""#);
        let paths = Path::from_str(r#"["level12", "level2"]"#).unwrap();
        assert_eq!(
            json.get(&paths).unwrap().unwrap().to_string(),
            r#""world2""#
        );
        let paths = Path::from_str(r#"["level3"]"#).unwrap();
        assert!(json.get(&paths).unwrap().is_none());

        // complex path with array
        let json =
            JSON::from_str(r#"{"level1":[1,{"hello":[1,[7,8]]}], "level12":"world"}"#).unwrap();
        let paths = Path::from_str(r#"["level1", 1, "hello"]"#).unwrap();

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
        let paths = Path::from_str(r#"["level1", 1]"#).unwrap();
        assert_eq!(json.get(&paths).unwrap().unwrap().to_string(), r#""b""#);
        let paths = Path::from_str(r#"["level12", 0]"#).unwrap();

        // complex path
        assert_eq!(json.get(&paths).unwrap().unwrap().to_string(), r#"123"#);
        let paths = Path::from_str(r#"["level12", 1, "level2"]"#).unwrap();
        assert_eq!(
            json.get(&paths).unwrap().unwrap().to_string(),
            r#"["c","d"]"#
        );
        let json =
            JSON::from_str(r#"{"level1":[1,{"hello":[1,[7,8]]}], "level12":"world"}"#).unwrap();
        let paths = Path::from_str(r#"["level1", 1, "hello", 1]"#).unwrap();

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
