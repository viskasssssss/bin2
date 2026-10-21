use std::io::{self, Write};

use crate::repr::BinRepr;

pub struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn write<T: BinRepr>(&mut self, value: &T) -> io::Result<()> {
        value.write_to(&mut self.bytes)
    }

    pub fn write_to<W: Write>(&self, output: &mut W) -> io::Result<()> {
        output.write_all(&self.bytes)
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}
