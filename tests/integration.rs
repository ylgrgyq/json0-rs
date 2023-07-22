use log::info;
use my_json0::error::Result;
use my_json0::json::Transformer;
use my_json0::operation::{Operation, OperationComponent};
use serde_json::Value;
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

fn read_json_value<P>(file_name: P) -> Result<Vec<Value>>
where
    P: AsRef<Path>,
{
    let mut out = vec![];
    if let Ok(lines) = read_lines(file_name) {
        for line in lines {
            if let Ok(v) = line {
                out.push(serde_json::from_str(&v)?);
            }
        }
    }
    Ok(out)
}

trait Test<E> {
    fn test(&self, executor: &E);
}

trait TestPattern<T: Test<E>, E> {
    fn load<I: Iterator<Item = Value>>(&self, input: &mut I) -> Option<T>;
    fn executor(&self) -> &E;
    fn test_input_path(&self) -> PathBuf;
}

struct TransformTest {
    input_left: Operation,
    input_right: Operation,
    result_left: Operation,
    result_right: Operation,
}

impl Test<Transformer> for TransformTest {
    fn test(&self, executor: &Transformer) {
        let (l, r) = executor
            .transform(&self.input_left, &self.input_right)
            .unwrap();
        assert_eq!(self.result_left, l);
        assert_eq!(self.result_right, r);
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
    fn load<I: Iterator<Item = Value>>(&self, input: &mut I) -> Option<TransformTest> {
        if let Some(input_left) = input.next() {
            let input_left: Operation = OperationComponent::try_from(input_left).unwrap().into();
            let input_right: Operation = OperationComponent::try_from(input.next().unwrap())
                .unwrap()
                .into();
            let result_left: Operation = OperationComponent::try_from(input.next().unwrap())
                .unwrap()
                .into();
            let result_right: Operation = OperationComponent::try_from(input.next().unwrap())
                .unwrap()
                .into();
            return Some(TransformTest {
                input_left,
                input_right,
                result_left,
                result_right,
            });
        }
        None
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
    let mut iter = json_values.into_iter();
    loop {
        if let Some(test) = pattern.load(&mut iter) {
            test.test(&transformer);
        } else {
            break;
        }
    }

    Ok(())
}

#[test]
fn test_transform() {
    let pattern = TransformTestPattern::new("tests/resources/transform_test_case");
    run_test(&pattern).unwrap();
}
