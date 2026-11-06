
# bin2

![GitHub License](https://img.shields.io/github/license/viskasssssss/bin2)
![Rust](https://img.shields.io/badge/language-Rust-orange?logo=rust&logoColor=white)

---

**bin2 is a Rust library designed to make binary data serialization and deserialization straightforward.**

## Usage

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