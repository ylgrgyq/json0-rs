use std::fmt::Display;

use serde_json::Value;

use crate::error::{JsonError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum PathElement {
    Index(usize),
    Key(String),
}

impl PartialOrd for PathElement {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self {
            // only index can compare
            PathElement::Index(a) => match other {
                PathElement::Index(b) => a.partial_cmp(b),
                PathElement::Key(_) => None,
            },
            PathElement::Key(_) => None,
        }
    }
}

impl From<usize> for PathElement {
    fn from(i: usize) -> Self {
        PathElement::Index(i)
    }
}

impl From<String> for PathElement {
    fn from(k: String) -> Self {
        PathElement::Key(k)
    }
}

impl Display for PathElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathElement::Index(i) => f.write_fmt(format_args!("{}", i)),
            PathElement::Key(k) => f.write_fmt(format_args!("\"{}\"", k)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    paths: Vec<PathElement>,
}

impl Path {
    pub fn first_key_path(&self) -> Option<&String> {
        self.get_key_at(0)
    }

    pub fn first_index_path(&self) -> Option<&usize> {
        self.get_index_at(0)
    }

    pub fn get(&self, index: usize) -> Option<&PathElement> {
        self.paths.get(index)
    }

    pub fn get_elements(&self) -> &Vec<PathElement> {
        &self.paths
    }

    pub fn get_mut_elements(&mut self) -> &mut Vec<PathElement> {
        &mut self.paths
    }

    pub fn get_key_at(&self, index: usize) -> Option<&String> {
        let first_path = self.paths.get(index);
        if first_path.is_none() {
            return None;
        }

        match first_path.unwrap() {
            PathElement::Index(_) => None,
            PathElement::Key(k) => Some(k),
        }
    }

    pub fn get_index_at(&self, index: usize) -> Option<&usize> {
        let first_path = self.paths.get(index);
        if first_path.is_none() {
            return None;
        }

        match first_path.unwrap() {
            PathElement::Index(i) => Some(i),
            PathElement::Key(_) => None,
        }
    }

    pub fn last(&self) -> Option<&PathElement> {
        self.get(self.len() - 1)
    }

    pub fn replace(&mut self, index: usize, path_elem: PathElement) -> Option<PathElement> {
        if let Some(_) = self.paths.get(index) {
            let o = std::mem::replace(&mut self.paths[index], path_elem);
            return Some(o);
        }
        return None;
    }

    pub fn increase_index(&mut self, index: usize) -> bool {
        if let Some(p) = self.paths.get(index) {
            if let PathElement::Index(i) = p {
                self.replace(index, PathElement::Index(i + 1));
                return true;
            }
        }
        false
    }

    pub fn decrease_index(&mut self, index: usize) -> bool {
        if let Some(p) = self.paths.get(index) {
            if let PathElement::Index(i) = p {
                self.replace(index, PathElement::Index(i - 1));
                return true;
            }
        }
        false
    }

    pub fn split_at(&self, mid: usize) -> (Path, Path) {
        let (left, right) = self.paths.split_at(mid);
        (
            Path {
                paths: left.to_vec(),
            },
            Path {
                paths: right.to_vec(),
            },
        )
    }

    pub fn max_common_path(&self, path: &Path) -> Path {
        let mut common_p = vec![];
        for (i, pa) in path.get_elements().iter().enumerate() {
            if let Some(pb) = self.get(i) {
                if pa.eq(pb) {
                    common_p.push(pb.clone());
                    continue;
                }
            }
            break;
        }
        Path { paths: common_p }
    }

    pub fn common_path_prefix(&self, path: &Path) -> Path {
        let mut common_p = vec![];
        for (i, pa) in path.get_elements().iter().enumerate() {
            if let Some(pb) = path.get(i) {
                if pa.eq(pb) {
                    common_p.push(pb.clone());
                    continue;
                }
            }
            break;
        }
        Path { paths: common_p }
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn is_prefix_of(&self, path: &Path) -> bool {
        for (i, p) in self.paths.iter().enumerate() {
            if let Some(p2) = path.paths.get(i) {
                if p != p2 {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn next_level(&self) -> Path {
        Path {
            paths: self.paths[1..].to_vec(),
        }
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "[{}]",
            self.paths
                .iter()
                .map(|p| format!("{}", p))
                .collect::<Vec<String>>()
                .join(", ")
        ))?;
        Ok(())
    }
}

impl TryFrom<&str> for Path {
    type Error = JsonError;

    fn try_from(input: &str) -> std::result::Result<Self, Self::Error> {
        if let Ok(value) = serde_json::from_str::<Value>(input) {
            return Path::try_from(&value);
        }
        Err(JsonError::InvalidPathFormat)
    }
}

impl TryFrom<&Value> for Path {
    type Error = JsonError;

    fn try_from(value: &Value) -> std::result::Result<Self, Self::Error> {
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
                                    Ok(PathElement::Index(i as usize))
                                } else {
                                    Err(JsonError::InvalidPathElement(pe.to_string()))
                                }
                            }
                            Value::String(k) => Ok(PathElement::Key(k.to_string())),
                            _ => Err(JsonError::InvalidPathElement(pe.to_string())),
                        })
                        .collect::<Result<Vec<PathElement>>>()?;
                    Ok(Path { paths })
                }
            }
            _ => Err(JsonError::InvalidPathFormat),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_parse_invalid_path() {
        assert_matches!(
            Path::try_from("]").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Path::try_from("[").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Path::try_from("").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Path::try_from("[]").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Path::try_from("hello").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Path::try_from("[hello]").unwrap_err(),
            JsonError::InvalidPathFormat
        );
    }

    #[test]
    fn test_parse_index_path() {
        let paths = Path::try_from("[1]").unwrap();
        assert_eq!(1, paths.len());
        assert_eq!(1, *paths.first_index_path().unwrap());
        let paths = Path::try_from("[2, 3, 4]").unwrap();
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
        let paths = Path::try_from("[\"hello\"]").unwrap();
        assert_eq!(1, paths.len());
        assert_eq!("hello", paths.first_key_path().unwrap());
        let paths = Path::try_from("[\"hello\", \"word\", \"hello\"]").unwrap();
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
        let paths = Path::try_from("[ \"hello \"  ,  1,  \"  world \",  4  ]").unwrap();
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
    fn test_increase_decrease_path() {
        let mut paths = Path::try_from("[ \"hello \"  ,  1,  \"  world \",  4  ]").unwrap();
        assert!(paths.increase_index(1));
        assert_eq!(2, *paths.get_index_at(1).unwrap());
        assert!(paths.increase_index(3));
        assert_eq!(5, *paths.get_index_at(3).unwrap());
        assert!(paths.decrease_index(1));
        assert_eq!(1, *paths.get_index_at(1).unwrap());
        assert!(paths.decrease_index(3));
        assert_eq!(4, *paths.get_index_at(3).unwrap());

        assert!(!paths.decrease_index(0));
        assert!(!paths.increase_index(0));
    }
}
