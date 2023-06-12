use std::{collections::BTreeMap, hash::Hash, hash::Hasher, vec};

use serde_json::{Number, Value};

#[derive(Clone)]
enum Path {
    Index(usize),
    Key(String),
}

type Paths = Vec<Path>;

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
