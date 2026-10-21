
# bin2

![GitHub License](https://img.shields.io/github/license/viskasssssss/bin2)
![Rust](https://img.shields.io/badge/language-Rust-orange?logo=rust&logoColor=white)
## About

Simple binary serialization and deserialization for Rust.

bin2 is a Rust library designed to make binary data serialization and deserialization straightforward.

It provides a simple API for writing data to a binary representation and reading it back when needed.
## Features

- **Simple API** — straightforward `Writer` and `Reader` interfaces.
- **Binary I/O** — write serialized data to a file.
- **Read from bytes** — deserialize data from an in-memory byte buffer.
- **Rust-native** — designed to be used directly from Rust code.


## Installation

Add **bin2** to the dependencies in your *Cargo.toml* file:

```toml
[dependencies]
bin2 = { git = "https://github.com/viskasssssss/bin2" }
```

## Usage

The following example serializes a string into a binary file and then deserializes it back.

```rust
fn main() -> std::io::Result<()> {
    // Serialize a string
    let mut writer = bin2::Writer::new();
    writer.write(&"Hello, bin2!".to_string())?;

    // Write binary data to a file
    let mut output = std::fs::File::create("hello.bin")?;
    writer.write_to(&mut output)?;

    // Read the binary file
    let bytes = std::fs::read("hello.bin")?;
    let mut reader = bin2::Reader::from_bytes(&bytes);

    // Deserialize the string
    println!("{}", reader.read::<String>()?);

    Ok(())
}
```

Output:

```text
Hello, bin2!
```

## Examples

More examples are available in the [`examples/`](./examples/) directory:

- [`hello-bin2.rs`](./examples/hello-bin2.rs) — basic serialization and deserialization.
- [`bin-repr.rs`](./examples/bin-repr.rs) — custom binary representations using `BinRepr`.
## License

[MIT](https://choosealicense.com/licenses/mit/)

