
use std::io::{self, Cursor, Read};

use crate::repr::BinRepr;

const DEFAULT_MAX_COLLECTION_LEN: usize = 1_000_000;
const DEFAULT_MAX_STRING_LEN: usize = 16 * 1024 * 1024;
const DEFAULT_MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;
const DEFAULT_MAX_DEPTH: usize = 64;
const DEFAULT_MAX_TOTAL_ALLOC: usize = 64 * 1024 * 1024;

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

    pub fn read<T: BinRepr>(&mut self) -> io::Result<T> {
        T::read_from(self)
    }

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

    pub fn leave_depth(&mut self) {
        debug_assert!(self.depth > 0);
        self.depth -= 1;
    }

    pub fn bytes(&self) -> &[u8] {
        self.cursor.get_ref()
    }

    pub fn allocated(&self) -> usize {
        self.allocated
    }

    pub fn remaining(&self) -> usize {
        self.bytes().len() - self.cursor.position() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub fn consumed(&self) -> usize {
        self.consumed
    }
}
