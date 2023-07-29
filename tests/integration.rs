use log::{debug, info};
use my_json0::error::{JsonError, Result};
use my_json0::operation::Operation;
use my_json0::transformer::Transformer;
use serde_json::Value;
use std::fmt::Display;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use test_log::test;

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn read_json_value<P>(file_name: P) -> Result<Vec<(usize, Value)>>
where
    P: AsRef<Path>,
{
    let mut out = vec![];
    let mut line_number = 0;
    if let Ok(lines) = read_lines(file_name) {
        for line in lines {
            if let Ok(v) = line {
                line_number += 1;
                if !v.is_empty() && !v.starts_with("#") {
                    let val = serde_json::from_str(&v).map_err(|e| {
                        JsonError::UnexpectedError(format!("parse line: {} failed. {}", v, e))
                    })?;
                    out.push((line_number, val));
                }
            }
        }
    }
    Ok(out)
}

trait Test<E> {
    fn test(&self, executor: &E);
}

trait TestPattern<T: Test<E>, E> {
    fn load<I: Iterator<Item = (usize, Operation)>>(&self, input: &mut I) -> Result<Option<T>>;
    fn executor(&self) -> &E;
    fn test_input_path(&self) -> PathBuf;
}

#[derive(Debug)]
struct TransformTest {
    line: usize,
    input_left: Operation,
    input_right: Operation,
    result_left: Operation,
    result_right: Operation,
}

impl Test<Transformer> for TransformTest {
    fn test(&self, executor: &Transformer) {
        info!(
            "execute test transform at line: {} left: {} right: {}",
            self.line, self.input_left, self.input_right
        );

        let (l, r) = executor
            .transform(&self.input_left, &self.input_right)
            .unwrap();
        assert_eq!(self.result_left, l, "left transform failed");
        assert_eq!(self.result_right, r, "right transform failed");
    }
}

impl Display for TransformTest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "left: {}\nright: {}\nrleft: {}\nrRight: {}",
            self.input_left, self.input_right, self.result_left, self.result_right
        ))
    }
}

struct TransformTestPattern<'a> {
    path: &'a str,
    transformer: Transformer,
}

impl<'a> TransformTestPattern<'a> {
    fn new(p: &'a str) -> TransformTestPattern<'a> {
        TransformTestPattern {
            path: p,
            transformer: Transformer::new(),
        }
    }
}

impl<'a> TestPattern<TransformTest, Transformer> for TransformTestPattern<'a> {
    fn load<I: Iterator<Item = (usize, Operation)>>(
        &self,
        input: &mut I,
    ) -> Result<Option<TransformTest>> {
        if let Some((line, input_left)) = input.next() {
            if let Some((_, input_right)) = input.next() {
                if let Some((_, result_left)) = input.next() {
                    if let Some((_, result_right)) = input.next() {
                        let test = TransformTest {
                            line,
                            input_left,
                            input_right,
                            result_left,
                            result_right,
                        };
                        debug!("load test at line: {}\n{}", line, &test);
                        return Ok(Some(test));
                    }
                }
            }
            return Err(JsonError::UnexpectedError(
                "Insufficient operation to construct a transform test".into(),
            ));
        }

        Ok(None)
    }

    fn executor(&self) -> &Transformer {
        &self.transformer
    }

    fn test_input_path(&self) -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push(self.path);
        p
    }
}

fn run_test<T: Test<E>, E, P: Sized + TestPattern<T, E>>(pattern: &P) -> Result<()> {
    let input_data_path = pattern.test_input_path();
    let json_values = read_json_value(&input_data_path)?;
    let transformer = pattern.executor();
    let mut iter = json_values
        .into_iter()
        .map(|v| {
            let v_str = v.1.to_string();
            v.1.try_into()
                .map_err(|e| {
                    JsonError::InvalidOperation(format!(
                        "{} can not be parsed to Operation due to error {}",
                        v_str, e
                    ))
                })
                .map(|op| (v.0, op))
        })
        .collect::<Result<Vec<(usize, Operation)>>>()?
        .into_iter();
    loop {
        if let Some(test) = pattern.load(&mut iter)? {
            test.test(&transformer);
        } else {
            break;
        }
    }

    Ok(())
}

#[test]
fn test_transform_list() {
    let pattern = TransformTestPattern::new("tests/resources/transform_list_case.json");
    run_test(&pattern).unwrap();
}

#[test]
fn test_transform_object() {
    let pattern = TransformTestPattern::new("tests/resources/transform_object_case.json");
    run_test(&pattern).unwrap();
}

#[test]
fn test_other_transform_case() {
    let pattern = TransformTestPattern::new("tests/resources/other_transform_case.json");
    run_test(&pattern).unwrap();
}
