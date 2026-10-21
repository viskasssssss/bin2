fn main() -> std::io::Result<()> {
    // Create writer
    let mut writer = bin2::Writer::new();
    // Write string "Hello, bin2!"
    writer.write(&"Hello, bin2!".to_string())?;

    // Save the result
    let mut output = std::fs::File::create("hello.bin")?;
    writer.write_to(&mut output)?;

    // Read it
    let bytes = std::fs::read("hello.bin")?;
    let mut reader = bin2::Reader::from_bytes(&bytes);

    // Output string
    println!("{}", reader.read::<String>()?);

    Ok(())
}