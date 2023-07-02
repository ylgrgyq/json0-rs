use serde_json::Value;

use crate::error::{JsonError, Result};

#[derive(Debug, Clone)]
pub enum Path {
    Index(usize),
    Key(String),
}

#[derive(Debug, Clone)]
pub struct Paths {
    paths: Vec<Path>,
}

impl Paths {
    pub fn from_str(input: &str) -> Result<Paths> {
        if let Ok(value) = serde_json::from_str(input) {
            return Paths::from_json_value(&value);
        }
        Err(JsonError::InvalidPathFormat)
    }

    pub fn from_json_value(value: &Value) -> Result<Paths> {
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

    pub fn first_key_path(&self) -> Option<&String> {
        let first_path = self.paths.first();
        if first_path.is_none() {
            return None;
        }

        match first_path.unwrap() {
            Path::Index(_) => None,
            Path::Key(k) => Some(k),
        }
    }

    pub fn first_index_path(&self) -> Option<&usize> {
        let first_path = self.paths.first();
        if first_path.is_none() {
            return None;
        }

        match first_path.unwrap() {
            Path::Index(i) => Some(i),
            Path::Key(_) => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn next_level(&self) -> Paths {
        Paths {
            paths: self.paths[1..].to_vec(),
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
}
