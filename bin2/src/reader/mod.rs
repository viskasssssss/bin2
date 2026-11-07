
use std::io::{self, Cursor, Read};

use crate::repr::BinRepr;

const DEFAULT_MAX_COLLECTION_LEN: usize = 1_000_000;
const DEFAULT_MAX_STRING_LEN: usize = 16 * 1024 * 1024;
const DEFAULT_MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;
const DEFAULT_MAX_DEPTH: usize = 64;
const DEFAULT_MAX_TOTAL_ALLOC: usize = 64 * 1024 * 1024;

/// A reader for deserializing binary data from a byte slice.
///
/// `Reader` provides methods for decoding values using [`BinRepr`].
pub struct Reader<'a> {
    cursor: Cursor<&'a [u8]>,
    max_collection_len: usize,
    max_string_len: usize,
    max_total_bytes: usize,
    max_depth: usize,
    max_total_alloc: usize,
    allocated: usize,
    depth: usize,
    consumed: usize,
}

impl<'a> Reader<'a> {
    /// Creates a reader with the default decoding limits.
    ///
    /// The default limits are:
    ///
    /// - Maximum collection length: 1,000,000 elements.
    /// - Maximum string length: 16 MiB.
    /// - Maximum total bytes consumed: 64 MiB.
    /// - Maximum total allocation budget: 64 MiB.
    /// - Maximum decoding depth: 64 levels.
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        Self::with_decode_limits(
            bytes,
            DEFAULT_MAX_COLLECTION_LEN,
            DEFAULT_MAX_STRING_LEN,
            DEFAULT_MAX_TOTAL_BYTES,
            DEFAULT_MAX_TOTAL_ALLOC,
            DEFAULT_MAX_DEPTH,
        )
    }

    /// Creates a reader with custom collection and string length limits.
    ///
    /// All other decoding limits retain their default values.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The input data to read from.
    /// - `max_collection_len`: Maximum permitted number of elements in a collection.
    /// - `max_string_len`: Maximum permitted string length in bytes.
    pub fn with_limits(
        bytes: &'a [u8],
        max_collection_len: usize,
        max_string_len: usize,
    ) -> Self {
        Self::with_decode_limits(
            bytes,
            max_collection_len,
            max_string_len,
            DEFAULT_MAX_TOTAL_BYTES,
            DEFAULT_MAX_TOTAL_ALLOC,
            DEFAULT_MAX_DEPTH,
        )
    }

    /// Creates a reader with fully configurable decoding limits.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The input data to read from.
    /// - `max_collection_len`: Maximum permitted collection length.
    /// - `max_string_len`: Maximum permitted string length in bytes.
    /// - `max_total_bytes`: Maximum number of bytes that may be consumed.
    /// - `max_total_alloc`: Maximum cumulative allocation budget.
    /// - `max_depth`: Maximum permitted decoding nesting depth.
    ///
    /// All resource limits are enforced independently.
    pub fn with_decode_limits(
        bytes: &'a [u8],
        max_collection_len: usize,
        max_string_len: usize,
        max_total_bytes: usize,
        max_total_alloc: usize,
        max_depth: usize,
    ) -> Self {
        Self {
            cursor: Cursor::new(bytes),
            max_collection_len,
            max_string_len,
            max_total_bytes,
            max_total_alloc,
            max_depth,
            depth: 0,
            consumed: 0,
            allocated: 0,
        }
    }

    /// Deserializes a value from the input.
    ///
    /// The value is decoded using its [`BinRepr`] implementation.
    ///
    /// # Errors
    ///
    /// Returns an error if decoding fails or a configured decoding limit
    /// is exceeded.
    pub fn read<T: BinRepr>(&mut self) -> io::Result<T> {
        T::read_from(self)
    }

    /// Reads exactly the requested number of bytes into the provided buffer.
    ///
    /// The number of successfully consumed bytes is tracked against the
    /// configured total byte limit.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested length exceeds the remaining byte
    /// budget or if insufficient input data is available.
    pub fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let remaining_budget =
            self.max_total_bytes.saturating_sub(self.consumed);

        if buf.len() > remaining_budget {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "total decoding byte limit exceeded",
            ));
        }

        let before = self.cursor.position();

        let result = self.cursor.read_exact(buf);

        let after = self.cursor.position();
        self.consumed += (after - before) as usize;

        result
    }

    /// Reads a collection length encoded as a 32-bit unsigned integer.
    ///
    /// The decoded length is checked against the configured maximum
    /// collection length before being returned.
    ///
    /// # Errors
    ///
    /// Returns an error if reading fails or the decoded length exceeds
    /// the configured collection limit.
    pub fn read_len(&mut self) -> io::Result<usize> {
        let len = u32::read_from(self)? as usize;

        if len > self.max_collection_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "collection length exceeds configured limit",
            ));
        }

        Ok(len)
    }

    /// Reads a string length encoded as a 32-bit unsigned integer.
    ///
    /// The decoded length is checked against the configured maximum
    /// string length before being returned.
    ///
    /// # Errors
    ///
    /// Returns an error if reading fails or the decoded length exceeds
    /// the configured string limit.
    pub fn read_string_len(&mut self) -> io::Result<usize> {
        let len = u32::read_from(self)? as usize;

        if len > self.max_string_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "string length exceeds configured limit",
            ));
        }

        Ok(len)
    }

    /// Reserves an amount against the cumulative allocation budget.
    ///
    /// This method updates the tracked allocation usage without performing
    /// an actual memory allocation.
    ///
    /// # Parameters
    ///
    /// - `amount`: Number of bytes to reserve against the allocation budget.
    ///
    /// # Errors
    ///
    /// Returns an error if the allocation budget would overflow or the
    /// configured maximum allocation budget would be exceeded.
    pub fn reserve_alloc(&mut self, amount: usize) -> io::Result<()> {
        let Some(new_total) = self.allocated.checked_add(amount) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "allocation budget overflow",
            ));
        };

        if new_total > self.max_total_alloc {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "total allocation budget exceeded",
            ));
        }

        self.allocated = new_total;
        Ok(())
    }

    /// Enters a deeper decoding context.
    ///
    /// Increments the current decoding depth by one, provided that the
    /// configured maximum depth has not been reached.
    ///
    /// # Errors
    ///
    /// Returns an error if entering another level would exceed the
    /// configured maximum decoding depth.
    pub fn enter_depth(&mut self) -> io::Result<()> {
        if self.depth >= self.max_depth {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "maximum decoding depth exceeded",
            ));
        }

        self.depth += 1;
        Ok(())
    }

    /// Leaves the current decoding context.
    ///
    /// Decrements the current decoding depth by one.
    ///
    /// # Panics
    ///
    /// Triggers a debug assertion if called when the current depth is zero.
    pub fn leave_depth(&mut self) {
        debug_assert!(self.depth > 0);
        self.depth -= 1;
    }

    /// Returns the complete underlying input byte slice.
    ///
    /// This includes both consumed and unread bytes.
    pub fn bytes(&self) -> &[u8] {
        self.cursor.get_ref()
    }

    /// Returns the cumulative number of bytes reserved against the
    /// allocation budget.
    pub fn allocated(&self) -> usize {
        self.allocated
    }

    /// Returns the number of unread bytes remaining in the input.
    pub fn remaining(&self) -> usize {
        self.bytes().len() - self.cursor.position() as usize
    }

    /// Returns whether all input bytes have been consumed.
    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// Returns the number of bytes successfully consumed from the input.
    ///
    /// This count is updated by [`read_exact`](Self::read_exact).
    pub fn consumed(&self) -> usize {
        self.consumed
    }
}
