use std::{
    fmt::{Debug, Display},
    mem,
    ops::{Deref, DerefMut},
    rc::Rc,
    vec,
};

use crate::{
    common::Validation,
    error::JsonError,
    error::Result,
    path::{Path, PathElement},
    sub_type::{SubType, SubTypeFunctions, SubTypeFunctionsHolder},
};
use itertools::Itertools;
use serde_json::{Map, Value};

pub enum Operator {
    Noop(),
    SubType(SubType, Value, Box<dyn SubTypeFunctions>),
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
            Self::SubType(arg0, arg1, _) => {
                f.debug_tuple("SubType2").field(arg0).field(arg1).finish()
            }
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
            (Self::SubType(l0, l1, _), Self::SubType(r0, r1, _)) => l0 == r0 && l1 == r1,
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
            Self::SubType(arg0, arg1, arg2) => {
                Self::SubType(arg0.clone(), arg1.clone(), arg2.clone())
            }
            Self::ListInsert(arg0) => Self::ListInsert(arg0.clone()),
            Self::ListDelete(arg0) => Self::ListDelete(arg0.clone()),
            Self::ListReplace(arg0, arg1) => Self::ListReplace(arg0.clone(), arg1.clone()),
            Self::ListMove(arg0) => Self::ListMove(*arg0),
            Self::ObjectInsert(arg0) => Self::ObjectInsert(arg0.clone()),
            Self::ObjectDelete(arg0) => Self::ObjectDelete(arg0.clone()),
            Self::ObjectReplace(arg0, arg1) => Self::ObjectReplace(arg0.clone(), arg1.clone()),
        }
    }
}

impl Operator {
    fn value_to_index(val: &Value) -> Result<usize> {
        if let Some(i) = val.as_u64() {
            return Ok(i as usize);
        }
        Err(JsonError::InvalidOperation(format!(
            "{} can not parsed to index",
            val
        )))
    }
}

impl Validation for Operator {
    fn validates(&self) -> Result<()> {
        return match self {
            Operator::SubType(_, operand, f) => f.validate_operand(operand),
            _ => Ok(()),
        };
    }
}

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = match self {
            Operator::Noop() => "".into(),
            Operator::SubType(t, o, _) => format!("t: {}, o: {}", t, o),
            Operator::ListInsert(i) => format!("li: {}", i),
            Operator::ListDelete(d) => format!("ld: {}", d),
            Operator::ListReplace(i, d) => format!("li: {}, ld: {}", i, d),
            Operator::ListMove(m) => format!("lm: {}", m),
            Operator::ObjectInsert(i) => format!("oi: {}", i),
            Operator::ObjectDelete(d) => format!("od: {}", d),
            Operator::ObjectReplace(i, d) => {
                format!("oi: {}, od: {}", i, d)
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
        Ok(op)
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
            Operator::SubType(_, o, f) => f.invert(&path, o)?,
            Operator::ListInsert(v) => Operator::ListDelete(v.clone()),
            Operator::ListDelete(v) => Operator::ListInsert(v.clone()),
            Operator::ListReplace(new_v, old_v) => {
                Operator::ListReplace(old_v.clone(), new_v.clone())
            }
            Operator::ListMove(new) => {
                let old_p = path.replace(path.len() - 1, PathElement::Index(*new));
                if let Some(PathElement::Index(i)) = old_p {
                    Operator::ListMove(i)
                } else {
                    panic!(
                        "invalid lm operation: {self}, last path in operation is not index path type"
                    );
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

    /**
     *
     */
    pub fn merge(&mut self, op: OperationComponent) -> Option<OperationComponent> {
        if let Some(new_operator) = match &self.operator {
            Operator::Noop() => Some(op.operator.clone()),
            Operator::SubType(_, base_v, f) => f.merge(base_v, &op.operator),

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

    pub fn operate_path_len(&self) -> usize {
        match self.operator {
            Operator::SubType(_, _, _) => self.path.clone().len(),
            _ => {
                let mut p = self.path.clone();
                p.get_mut_elements().pop();
                p.len()
            }
        }
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
            if let Some(o) = last.merge(op) {
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

    pub fn compose(&mut self, other: Operation) -> Result<()> {
        for op in other.into_iter() {
            self.append(op)?;
        }

        Ok(())
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

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[")?;
        f.write_str(
            self.operations
                .iter()
                .map(|op| op.to_string())
                .join(",")
                .as_str(),
        )?;
        f.write_str("]")?;
        Ok(())
    }
}

pub struct ListOperationBuilder {
    path: Path,
    insert: Option<Value>,
    delete: Option<Value>,
    move_to: Option<usize>,
}

impl ListOperationBuilder {
    fn new(path: Path) -> ListOperationBuilder {
        ListOperationBuilder {
            path,
            insert: None,
            delete: None,
            move_to: None,
        }
    }

    pub fn insert(mut self, val: Value) -> Self {
        self.insert = Some(val);
        self
    }

    pub fn delete(mut self, val: Value) -> Self {
        self.delete = Some(val);
        self
    }

    pub fn replace(mut self, old: Value, new: Value) -> Self {
        self.insert = Some(new);
        self.delete = Some(old);
        self
    }

    pub fn move_to(mut self, new_index: usize) -> Self {
        self.move_to = Some(new_index);
        self
    }

    pub fn build(self) -> Result<OperationComponent> {
        if let Some(new_index) = self.move_to {
            return OperationComponent::new(self.path, Operator::ListMove(new_index));
        }

        if let Some(del_val) = self.delete {
            if let Some(ins_val) = self.insert {
                return OperationComponent::new(self.path, Operator::ListReplace(ins_val, del_val));
            }
            return OperationComponent::new(self.path, Operator::ListDelete(del_val));
        }

        if let Some(ins_val) = self.insert {
            return OperationComponent::new(self.path, Operator::ListInsert(ins_val));
        }

        OperationComponent::new(self.path, Operator::Noop())
    }
}

pub struct ObjectOperationBuilder {
    path: Path,
    insert: Option<Value>,
    delete: Option<Value>,
}

impl ObjectOperationBuilder {
    fn new(path: Path) -> ObjectOperationBuilder {
        ObjectOperationBuilder {
            path,
            insert: None,
            delete: None,
        }
    }

    pub fn insert(mut self, val: Value) -> Self {
        self.insert = Some(val);
        self
    }

    pub fn delete(mut self, val: Value) -> Self {
        self.delete = Some(val);
        self
    }

    pub fn replace(mut self, old: Value, new: Value) -> Self {
        self.insert = Some(new);
        self.delete = Some(old);
        self
    }

    pub fn build(self) -> Result<OperationComponent> {
        if let Some(del_val) = self.delete {
            if let Some(ins_val) = self.insert {
                return OperationComponent::new(
                    self.path,
                    Operator::ObjectReplace(ins_val, del_val),
                );
            }
            return OperationComponent::new(self.path, Operator::ObjectDelete(del_val));
        }

        if let Some(ins_val) = self.insert {
            return OperationComponent::new(self.path, Operator::ObjectInsert(ins_val));
        }

        OperationComponent::new(self.path, Operator::Noop())
    }
}

pub struct NumberAddOperationBuilder {
    path: Path,
    number_i64: Option<i64>,
    number_f64: Option<f64>,
    sub_type_function: Box<dyn SubTypeFunctions>,
}

impl NumberAddOperationBuilder {
    pub fn new(
        path: Path,
        sub_type_function: Box<dyn SubTypeFunctions>,
    ) -> NumberAddOperationBuilder {
        NumberAddOperationBuilder {
            path,
            number_i64: None,
            number_f64: None,
            sub_type_function,
        }
    }

    pub fn add_int(mut self, num: i64) -> Self {
        self.number_i64 = Some(num);
        self
    }

    pub fn add_float(mut self, num: f64) -> Self {
        self.number_f64 = Some(num);
        self
    }

    pub fn build(self) -> Result<OperationComponent> {
        // support insert/delete multipul numbers
        if self.number_f64.is_some() && self.number_i64.is_some() {
            return Err(JsonError::InvalidOperation(
                "only one number can be add".into(),
            ));
        }

        if let Some(v) = self.number_i64 {
            let o = serde_json::to_value(v).unwrap();
            OperationComponent::new(
                self.path,
                Operator::SubType(SubType::NumberAdd, o, self.sub_type_function),
            )
        } else if let Some(v) = self.number_f64 {
            let o = serde_json::to_value(v).unwrap();
            OperationComponent::new(
                self.path,
                Operator::SubType(SubType::NumberAdd, o, self.sub_type_function),
            )
        } else {
            return Err(JsonError::InvalidOperation("need a number to add".into()));
        }
    }
}

pub struct TextOperationBuilder {
    path: Path,
    offset: usize,
    insert_val: Option<String>,
    delete_val: Option<String>,
    sub_type_function: Box<dyn SubTypeFunctions>,
}

impl TextOperationBuilder {
    pub fn new(path: Path, sub_type_function: Box<dyn SubTypeFunctions>) -> TextOperationBuilder {
        TextOperationBuilder {
            path,
            offset: 0,
            insert_val: None,
            delete_val: None,
            sub_type_function,
        }
    }

    pub fn insert_string(mut self, offset: usize, insert: String) -> Self {
        self.insert_val = Some(insert);
        self.offset = offset;
        self
    }

    pub fn insert_str(mut self, offset: usize, insert: &str) -> Self {
        self.insert_val = Some(insert.into());
        self.offset = offset;
        self
    }

    pub fn delete_string(mut self, offset: usize, delete: String) -> Self {
        self.delete_val = Some(delete);
        self.offset = offset;
        self
    }

    pub fn delete_str(mut self, offset: usize, delete: &str) -> Self {
        self.delete_val = Some(delete.into());
        self.offset = offset;
        self
    }

    pub fn build(self) -> Result<OperationComponent> {
        // support insert/delete multipul strings
        if self.insert_val.is_none() && self.delete_val.is_none()
            || (self.insert_val.is_some() && self.delete_val.is_some())
        {
            return Err(JsonError::InvalidOperation(
                "text operation must either insert or delete".into(),
            ));
        }

        let mut op_map = Map::new();
        op_map.insert("p".into(), serde_json::to_value(self.offset).unwrap());
        if let Some(v) = self.insert_val {
            op_map.insert("i".into(), Value::String(v));
        } else if let Some(v) = self.delete_val {
            op_map.insert("d".into(), Value::String(v));
        }

        let o = Value::Object(op_map);
        OperationComponent::new(
            self.path,
            Operator::SubType(SubType::Text, o, self.sub_type_function),
        )
    }
}

pub struct SubTypeOperationBuilder {
    path: Path,
    sub_type: SubType,
    sub_type_operator: Option<Value>,
    sub_type_function: Option<Box<dyn SubTypeFunctions>>,
}

impl SubTypeOperationBuilder {
    fn new(
        path: Path,
        sub_type: SubType,
        sub_type_function: Option<Box<dyn SubTypeFunctions>>,
    ) -> SubTypeOperationBuilder {
        SubTypeOperationBuilder {
            path,
            sub_type,
            sub_type_operator: None,
            sub_type_function,
        }
    }

    pub fn sub_type_operand(mut self, val: Value) -> Self {
        self.sub_type_operator = Some(val);
        self
    }

    pub fn sub_type_functions(mut self, val: Box<dyn SubTypeFunctions>) -> Self {
        self.sub_type_function = Some(val);
        self
    }

    pub fn build(self) -> Result<OperationComponent> {
        if let Some(o) = self.sub_type_operator {
            if let Some(f) = self.sub_type_function {
                OperationComponent::new(self.path, Operator::SubType(self.sub_type, o, f))
            } else {
                Err(JsonError::InvalidOperation(
                    "sub type functions is required".into(),
                ))
            }
        } else {
            Err(JsonError::InvalidOperation(
                "sub type operator is required".into(),
            ))
        }
    }
}

pub struct OperationFactory {
    sub_type_holder: Rc<SubTypeFunctionsHolder>,
}

impl OperationFactory {
    pub fn new(sub_type_holder: Rc<SubTypeFunctionsHolder>) -> OperationFactory {
        OperationFactory { sub_type_holder }
    }

    /// Build an Operation by JSON Value
    pub fn from_value(&self, value: Value) -> Result<Operation> {
        let mut operations = vec![];
        match value {
            Value::Array(arr) => {
                for v in arr {
                    let op: OperationComponent = self.operation_component_from_value(v)?;
                    operations.push(op);
                }
            }
            _ => {
                operations.push(self.operation_component_from_value(value)?);
            }
        }
        Operation::new(operations)
    }

    pub fn list_operation_builder(&self, path: Path) -> ListOperationBuilder {
        ListOperationBuilder::new(path)
    }

    pub fn object_operation_builder(&self, path: Path) -> ObjectOperationBuilder {
        ObjectOperationBuilder::new(path)
    }

    pub fn number_add_operation_builder(&self, path: Path) -> NumberAddOperationBuilder {
        let f = self
            .sub_type_holder
            .get(&SubType::NumberAdd)
            .map(|f| f.value().clone())
            .unwrap();
        NumberAddOperationBuilder::new(path, f)
    }

    pub fn text_operation_builder(&self, path: Path) -> TextOperationBuilder {
        let f = self
            .sub_type_holder
            .get(&SubType::Text)
            .map(|f| f.value().clone())
            .unwrap();
        TextOperationBuilder::new(path, f)
    }

    pub fn sub_type_operation_builder(
        &self,
        path: Path,
        sub_type_name: String,
    ) -> SubTypeOperationBuilder {
        let sub_type = SubType::Custome(sub_type_name);
        let f = self
            .sub_type_holder
            .get(&sub_type)
            .map(|f| f.value().clone());
        SubTypeOperationBuilder::new(path, sub_type, f)
    }

    fn operation_component_from_value(&self, value: Value) -> Result<OperationComponent> {
        let path_value = value.get("p");

        if path_value.is_none() {
            return Err(JsonError::InvalidOperation("Missing path".into()));
        }

        let paths = Path::try_from(path_value.unwrap())?;
        let operator = self.operator_from_value(value)?;

        Ok(OperationComponent {
            path: paths,
            operator,
        })
    }

    fn operator_from_value(&self, value: Value) -> Result<Operator> {
        match &value {
            Value::Object(obj) => {
                let operator = self.map_to_operator(obj)?;
                Ok(operator)
            }
            _ => Err(JsonError::InvalidOperation(
                "Operator can only be parsed from JSON Object".into(),
            )),
        }
    }

    fn map_to_operator(&self, obj: &Map<String, Value>) -> Result<Operator> {
        if let Some(na) = obj.get("na") {
            self.validate_operation_object_size(obj, 2)?;
            return Ok(Operator::SubType(
                SubType::NumberAdd,
                na.clone(),
                self.sub_type_holder
                    .get(&SubType::NumberAdd)
                    .map(|f| f.value().clone())
                    .unwrap(),
            ));
        }

        if let Some(t) = obj.get("t") {
            self.validate_operation_object_size(obj, 3)?;
            let sub_type = t.try_into()?;
            let op = obj.get("o").cloned().unwrap_or(Value::Null);
            let sub_op_func = self
                .sub_type_holder
                .get(&sub_type)
                .map(|f| f.value().clone())
                .ok_or(JsonError::InvalidOperation(format!(
                    "no sub type functions for sub type: {}",
                    sub_type
                )))?;
            return Ok(Operator::SubType(sub_type, op, sub_op_func));
        }

        if let Some(lm) = obj.get("lm") {
            self.validate_operation_object_size(obj, 2)?;
            let i = Operator::value_to_index(lm)?;
            return Ok(Operator::ListMove(i));
        }

        if let Some(li) = obj.get("li") {
            if let Some(ld) = obj.get("ld") {
                self.validate_operation_object_size(obj, 3)?;
                return Ok(Operator::ListReplace(li.clone(), ld.clone()));
            }
            self.validate_operation_object_size(obj, 2)?;
            return Ok(Operator::ListInsert(li.clone()));
        }

        if let Some(ld) = obj.get("ld") {
            self.validate_operation_object_size(obj, 2)?;
            return Ok(Operator::ListDelete(ld.clone()));
        }

        if let Some(oi) = obj.get("oi") {
            if let Some(od) = obj.get("od") {
                self.validate_operation_object_size(obj, 3)?;
                return Ok(Operator::ObjectReplace(oi.clone(), od.clone()));
            }
            self.validate_operation_object_size(obj, 2)?;
            return Ok(Operator::ObjectInsert(oi.clone()));
        }

        if let Some(od) = obj.get("od") {
            self.validate_operation_object_size(obj, 2)?;
            return Ok(Operator::ObjectDelete(od.clone()));
        }

        self.validate_operation_object_size(obj, 1)?;
        Ok(Operator::Noop())
    }

    fn validate_operation_object_size(
        &self,
        origin_operation: &Map<String, Value>,
        expect_size: usize,
    ) -> Result<()> {
        if origin_operation.len() != expect_size {
            return Err(JsonError::InvalidOperation(
                "JSON object size bigger than operator required".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_number_add_operator() {
        let path: Path = r#"["p1","p2"]"#.try_into().unwrap();
        let op_factory = OperationFactory::new(Rc::new(SubTypeFunctionsHolder::new()));
        let op = op_factory
            .number_add_operation_builder(path)
            .add_int(100)
            .build()
            .unwrap();

        let Operator::SubType(sub_type, op_value, _) = op.operator else {
            panic!()
        };
        assert_eq!(SubType::NumberAdd, sub_type);
        assert_eq!(serde_json::to_value(100).unwrap(), op_value);
    }

    #[test]
    fn test_text_operator() {
        let sub_type_operand: Value = serde_json::from_str(r#"{"p":1, "i":"hello"}"#).unwrap();
        let path: Path = r#"["p1","p2"]"#.try_into().unwrap();
        let op_factory = OperationFactory::new(Rc::new(SubTypeFunctionsHolder::new()));
        let op = op_factory
            .text_operation_builder(path)
            .insert_str(1, "hello")
            .build()
            .unwrap();

        let Operator::SubType(sub_type, op_value, _) = op.operator else {
                panic!()
            };
        assert_eq!(SubType::Text, sub_type);
        assert_eq!(sub_type_operand, op_value);
    }
}
