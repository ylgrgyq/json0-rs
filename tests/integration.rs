use itertools::Itertools;
use log::{debug, info};
use my_json0::error::{JsonError, Result};
use my_json0::operation::{Operation, OperationComponent};
use my_json0::Json0;
use serde_json::Value;
use std::fmt::Display;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::vec;
use test_log::test;

const COMMENT_PREFIX: char = '#';

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
        for line in lines.flatten() {
            line_number += 1;
            if !line.is_empty() && !line.starts_with(COMMENT_PREFIX) {
                let val = serde_json::from_str(&line).map_err(|e| {
                    JsonError::UnexpectedError(format!("parse line: {} failed. {}", line, e))
                })?;
                out.push((line_number, val));
            }
        }
    }
    Ok(out)
}

trait Test<E> {
    fn test(&self, executor: &E);
}

trait TestPattern<T: Test<E>, E> {
    fn load<I: Iterator<Item = (usize, Value)>>(&self, input: &mut I) -> Result<Option<T>>;
    fn executor(&self) -> &E;
    fn test_input_path(&self) -> &str;
}

#[derive(Debug)]
struct TransformTest {
    line: usize,
    input_left: Operation,
    input_right: Operation,
    result_left: Operation,
    result_right: Operation,
}

impl Test<Json0> for TransformTest {
    fn test(&self, executor: &Json0) {
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
            "left:   {}\nright:  {}\nrleft:  {}\nrRight: {}",
            self.input_left, self.input_right, self.result_left, self.result_right
        ))
    }
}

struct TransformTestPattern<'a> {
    test_input_file_path: &'a str,
    transformer: Json0,
}

impl<'a> TransformTestPattern<'a> {
    fn new(p: &'a str) -> TransformTestPattern<'a> {
        TransformTestPattern {
            test_input_file_path: p,
            transformer: Json0::new(),
        }
    }
}

impl<'a> TestPattern<TransformTest, Json0> for TransformTestPattern<'a> {
    fn load<I: Iterator<Item = (usize, Value)>>(
        &self,
        input: &mut I,
    ) -> Result<Option<TransformTest>> {
        if let Some((line, i_l)) = input.next() {
            let ((_, i_r), (_, r_l), (_, r_r)) = input.next_tuple().ok_or(
                JsonError::UnexpectedError("not enough input values for test".into()),
            )?;

            let test = TransformTest {
                line,
                input_left: self.transformer.operation_factory().from_value(i_l)?,
                input_right: self.transformer.operation_factory().from_value(i_r)?,
                result_left: self.transformer.operation_factory().from_value(r_l)?,
                result_right: self.transformer.operation_factory().from_value(r_r)?,
            };
            debug!("load test at line: {}\n{}", line, &test);
            return Ok(Some(test));
        }
        Ok(None)
    }

    fn executor(&self) -> &Json0 {
        &self.transformer
    }

    fn test_input_path(&self) -> &str {
        self.test_input_file_path
    }
}

#[derive(Debug)]
struct ApplyOperationTest {
    json: Value,
    operations: Vec<Operation>,
    expect_result: Value,
}

struct ApplyOperationExecutor {
    json0: Json0,
}

impl ApplyOperationExecutor {
    fn apply(&self, json: &Value, operations: &[Operation]) -> Result<Value> {
        let mut out = json.clone();
        self.json0.apply(&mut out, operations.to_owned())?;
        Ok(out)
    }
}

impl Test<ApplyOperationExecutor> for ApplyOperationTest {
    fn test(&self, executor: &ApplyOperationExecutor) {
        let r = executor.apply(&self.json, &self.operations).unwrap();
        assert_eq!(self.expect_result.clone(), r, "apply failed");
    }
}

impl Display for ApplyOperationTest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ops_str = self.operations.iter().join(",");
        f.write_fmt(format_args!(
            "json:          {}\noperations:    [{:?}]\nexpect_result: {}",
            self.json, ops_str, self.expect_result
        ))
    }
}

struct ApplyOperationTestPattern<'a> {
    test_input_file_path: &'a str,
    executor: ApplyOperationExecutor,
}

impl<'a> ApplyOperationTestPattern<'a> {
    fn new(p: &'a str) -> ApplyOperationTestPattern<'a> {
        ApplyOperationTestPattern {
            test_input_file_path: p,
            executor: ApplyOperationExecutor {
                json0: Json0::new(),
            },
        }
    }
}

impl<'a> TestPattern<ApplyOperationTest, ApplyOperationExecutor> for ApplyOperationTestPattern<'a> {
    fn load<I: Iterator<Item = (usize, Value)>>(
        &self,
        input: &mut I,
    ) -> Result<Option<ApplyOperationTest>> {
        if let Some((line, json)) = input.next() {
            let ((_, ops), (_, expect_result)) = input.next_tuple().ok_or(
                JsonError::UnexpectedError("not enough input values for test".into()),
            )?;

            let mut operations = vec![];
            if let Value::Array(op_array) = ops {
                operations = op_array
                    .into_iter()
                    .map(|o| self.executor().json0.operation_factory().from_value(o))
                    .collect::<Result<Vec<Operation>>>()?;
            }

            let test = ApplyOperationTest {
                json,
                operations,
                expect_result,
            };
            debug!("load test at line: {}\n{}", line, &test);
            return Ok(Some(test));
        }
        Ok(None)
    }

    fn executor(&self) -> &ApplyOperationExecutor {
        &self.executor
    }

    fn test_input_path(&self) -> &str {
        self.test_input_file_path
    }
}

#[derive(Debug)]
struct InvertOperationTest {
    origin_op: Operation,
    expect_invert_op: Operation,
}

struct InvertOperationExecutor {
    json0: Json0,
}

impl Test<InvertOperationExecutor> for InvertOperationTest {
    fn test(&self, executor: &InvertOperationExecutor) {
        assert_eq!(
            *self.expect_invert_op.get(0).unwrap(),
            self.origin_op.get(0).unwrap().invert().unwrap(),
            "invert failed"
        );
    }
}

impl Display for InvertOperationTest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "origin_op:        {}\nexpect_invert_op: {}",
            self.origin_op, self.expect_invert_op
        ))
    }
}

struct InvertOperationTestPattern<'a> {
    test_input_file_path: &'a str,
    executor: InvertOperationExecutor,
}

impl<'a> InvertOperationTestPattern<'a> {
    fn new(p: &'a str) -> InvertOperationTestPattern<'a> {
        InvertOperationTestPattern {
            test_input_file_path: p,
            executor: InvertOperationExecutor {
                json0: Json0::new(),
            },
        }
    }
}

impl<'a> TestPattern<InvertOperationTest, InvertOperationExecutor>
    for InvertOperationTestPattern<'a>
{
    fn load<I: Iterator<Item = (usize, Value)>>(
        &self,
        input: &mut I,
    ) -> Result<Option<InvertOperationTest>> {
        if let Some((line, origin_op)) = input.next() {
            let (_, expect_invert_op) = input.next().ok_or(JsonError::UnexpectedError(
                "not enough input values for test".into(),
            ))?;

            let test = InvertOperationTest {
                origin_op: self
                    .executor()
                    .json0
                    .operation_factory()
                    .from_value(origin_op)?,
                expect_invert_op: self
                    .executor()
                    .json0
                    .operation_factory()
                    .from_value(expect_invert_op)?,
            };
            debug!("load test at line: {}\n{}", line, &test);
            return Ok(Some(test));
        }
        Ok(None)
    }

    fn executor(&self) -> &InvertOperationExecutor {
        &self.executor
    }

    fn test_input_path(&self) -> &str {
        self.test_input_file_path
    }
}

fn run_test<T: Test<E>, E, P: Sized + TestPattern<T, E>>(pattern: &P) -> Result<()> {
    let mut input_data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    input_data_path.push(pattern.test_input_path());
    let json_values = read_json_value(&input_data_path)?;
    let executor = pattern.executor();
    let mut iter = json_values.into_iter();

    while let Some(test) = pattern.load(&mut iter)? {
        test.test(executor);
    }

    Ok(())
}

#[test]
fn test_json_apply() {
    let pattern = ApplyOperationTestPattern::new("tests/resources/apply_op_case.json");
    run_test(&pattern).unwrap();
}

#[test]
fn test_invert() {
    let pattern = InvertOperationTestPattern::new("tests/resources/invert_op_case.json");
    run_test(&pattern).unwrap();
}

#[test]
fn test_json_compose() {
    let pattern = ApplyOperationTestPattern::new("tests/resources/compose_op_case.json");
    run_test(&pattern).unwrap();
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
