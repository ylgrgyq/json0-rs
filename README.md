# json0-rs

<p align=center>
    <a href="https://github.com/ylgrgyq/json0-rs/blob/master/LICENSE.md"><img src="https://img.shields.io/github/license/ylgrgyq/json0-rs?style=flat-square" alt="MIT License"></a>
</p>


## Usage

To use `json0-rs`, simply add it to your project's dependencies in `Cargo.toml`:

```toml
[dependencies]
json0-rs = "0.1.0"
```

Then, import it into your Rust code:

```rust
use json0_rs::Json;
```

You can create a new JSON object using the `Json::new()` method:

```rust
let mut json = Json::new();
```

From there, you can add, remove, and modify JSON data using the provided API like this:

```rust
let json0 = Json0::new();

let mut json_to_operate = Value::Object(Map::new());

let op = json0
    .operation_factory()
    .object_operation_builder()
    .append_key_path("key")
    .insert(Value::String("world".into()))
    .build()
    .unwrap()
    .into();

json0.apply(&mut json_to_operate, vec![op]).unwrap();
```

For more information on how to use `json0-rs`, see the [documentation](https://docs.rs/json0-rs).

## License

`json0-rs` is licensed under the MIT license. See the [LICENSE](https://github.com/ylgrgyq/json0-rs/blob/master/LICENSE.md) file for more information.
