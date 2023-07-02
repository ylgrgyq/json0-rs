use serde_json::Value;

use crate::error::{JsonError, Result};

#[derive(Debug, Clone)]
pub enum PathElement {
    Index(usize),
    Key(String),
}

#[derive(Debug, Clone)]
pub struct Path {
    paths: Vec<PathElement>,
}

impl Path {
    pub fn from_str(input: &str) -> Result<Path> {
        if let Ok(value) = serde_json::from_str(input) {
            return Path::from_json_value(&value);
        }
        Err(JsonError::InvalidPathFormat)
    }

    pub fn from_json_value(value: &Value) -> Result<Path> {
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

    pub fn first_key_path(&self) -> Option<&String> {
        let first_path = self.paths.first();
        if first_path.is_none() {
            return None;
        }

        match first_path.unwrap() {
            PathElement::Index(_) => None,
            PathElement::Key(k) => Some(k),
        }
    }

    pub fn first_index_path(&self) -> Option<&usize> {
        let first_path = self.paths.first();
        if first_path.is_none() {
            return None;
        }

        match first_path.unwrap() {
            PathElement::Index(i) => Some(i),
            PathElement::Key(_) => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn next_level(&self) -> Path {
        Path {
            paths: self.paths[1..].to_vec(),
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_parse_invalid_path() {
        assert_matches!(
            Path::from_str("]").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Path::from_str("[").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Path::from_str("").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Path::from_str("[]").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Path::from_str("hello").unwrap_err(),
            JsonError::InvalidPathFormat
        );
        assert_matches!(
            Path::from_str("[hello]").unwrap_err(),
            JsonError::InvalidPathFormat
        );
    }

    #[test]
    fn test_parse_index_path() {
        let paths = Path::from_str("[1]").unwrap();
        assert_eq!(1, paths.len());
        assert_eq!(1, *paths.first_index_path().unwrap());
        let paths = Path::from_str("[2, 3, 4]").unwrap();
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
        let paths = Path::from_str("[\"hello\"]").unwrap();
        assert_eq!(1, paths.len());
        assert_eq!("hello", paths.first_key_path().unwrap());
        let paths = Path::from_str("[\"hello\", \"word\", \"hello\"]").unwrap();
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
        let paths = Path::from_str("[ \"hello \"  ,  1,  \"  world \",  4  ]").unwrap();
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
}
