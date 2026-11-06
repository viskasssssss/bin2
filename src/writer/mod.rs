use std::io::{self, Write};

use crate::repr::BinRepr;

/// A buffer for serializing values into a contiguous sequence of bytes.
///
/// `Writer` accumulates binary representations of values written through
/// [`write`](Self::write) and allows the resulting bytes to be accessed or
/// written to an output stream.
pub struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    /// Creates a new, empty `Writer`.
    ///
    /// # Examples
    ///
    /// ```
    /// let writer = bin2::Writer::new();
    /// assert!(writer.bytes().is_empty());
    /// ```
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Serializes a value into the internal buffer.
    ///
    /// The value is serialized using its [`BinRepr`] implementation.
    /// Previously written bytes are preserved, and the new representation
    /// is appended to the buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn write<T: BinRepr>(&mut self, value: &T) -> io::Result<()> {
        value.write_to(&mut self.bytes)
    }

    /// Writes all buffered bytes to the specified output.
    ///
    /// This method does not modify or clear the internal buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output fails.
    pub fn write_to<W: Write>(&self, output: &mut W) -> io::Result<()> {
        output.write_all(&self.bytes)
    }

    /// Returns a shared reference to the buffered bytes.
    ///
    /// The returned slice contains the binary representations of all values
    /// written so far.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}
