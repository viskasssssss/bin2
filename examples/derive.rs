#[derive(bin2::BinRepr, Debug)]
struct Foo {
    a: i32,
    b: bool,
    c: String,
}

fn main() -> std::io::Result<()> {
    let foo = Foo {
        a: 16,
        b: false,
        c: "foo".to_string(),
    };

    // Serialize
    let mut writer = bin2::Writer::new();
    writer.write(&foo)?;

    // Write binary data to a file
    let mut output = std::fs::File::create("hello.bin")?;
    writer.write_to(&mut output)?;

    // Read the binary file
    let bytes = std::fs::read("hello.bin")?;
    let mut reader = bin2::Reader::from_bytes(&bytes);

    // Deserialize
    println!("{:?}", reader.read::<Foo>()?);

    Ok(())
}