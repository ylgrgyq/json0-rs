use log::info;
use my_json0::error::Result;
use my_json0::json::{Operation, Transformer};
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

trait TestPattern {
    fn new() -> T;
    fn load();
    fn run();
}

trait Test {
    type Input;
    fn load<T: Iterator<Item = Value>>(&mut self, input: T);
    fn run
}

struct TransformTest {
    input_left: Operation,
    input_right: Operation,
    result_left: Operation,
    result_right: Operation,
}

impl TestPattern for TransformTest {
    fn load() {
        todo!()
    }

    fn run() {
        todo!()
    }
}

struct TransformTestDriver<T: TestPattern> {
    pattern: T,
}

impl<T: TestPattern> TransformTestDriver<T> {
    fn load(&self) -> Result<()> {
        self.pattern.new();
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("tests/resources/transform_test_case");

        let json_values = read_json_value(&d)?;
        json_values.chunks(3);

        let transformer = Transformer::new();
        // transformer.transform(operation, base_operation)
        Ok(())
    }
}

#[test]
fn test_merge_delete_no_remain() {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("tests/resources/transform_test_case");

    info!("asdfadfasdf 11 {:?}", d);
    info!("asdfsdf {:?}", read_json_value(&d).unwrap())
    // // let db_path = get_temporary_directory_path();
    // let db_path = PathBuf::from("/tmp/haha");
    // let bc = Bitcask::open(&db_path, BitcaskOptions::default()).unwrap();
    // bc.put("k1".into(), "value1".as_bytes()).unwrap();
    // bc.put("k2".into(), "value2".as_bytes()).unwrap();
    // bc.put("k3".into(), "value3".as_bytes()).unwrap();
    // bc.delete(&"k1".into()).unwrap();
    // bc.delete(&"k2".into()).unwrap();
    // bc.delete(&"k3".into()).unwrap();

    // bc.merge().unwrap();

    // let stats = bc.stats().unwrap();
    // assert_eq!(0, stats.total_data_size_in_bytes);
    // assert_eq!(1, stats.number_of_data_files);
    // assert_eq!(0, stats.number_of_keys);
}
