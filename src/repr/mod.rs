use std::io::{self, Write};

use crate::reader::Reader;

pub trait BinRepr: Sized {
    fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()>;

    fn read_from(reader: &mut Reader<'_>) -> io::Result<Self>;
}

macro_rules! impl_bin_repr {
    ($($t:ty),* $(,)?) => {
        $(
            impl BinRepr for $t {
                fn write_to<W: Write>(
                    &self,
                    writer: &mut W,
                ) -> io::Result<()> {
                    writer.write_all(&self.to_le_bytes())
                }

                fn read_from(reader: &mut Reader) -> io::Result<Self> {
                    let mut bytes = [0u8; std::mem::size_of::<$t>()];
                    reader.read_exact(&mut bytes)?;
                    Ok(<$t>::from_le_bytes(bytes))
                }
            }
        )*
    };
}

impl_bin_repr! {
    u8, i8,
    u16, i16,
    u32, i32, f32,
    u64, i64, f64,
    u128, i128,
}

impl BinRepr for String {
    fn write_to<W: Write>(
        &self,
        writer: &mut W,
    ) -> io::Result<()> {
        let len = u32::try_from(self.len()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "string is too long",
            )
        })?;

        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(self.as_bytes())
    }

    fn read_from(reader: &mut Reader) -> io::Result<Self> {
        let len = reader.read_string_len()?;

        reader.reserve_alloc(len)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(len).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("cannot allocate string buffer: {e}"),
            )
        })?;

        bytes.resize(len, 0);
        reader.read_exact(&mut bytes)?;

        String::from_utf8(bytes).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid UTF-8: {e}"),
            )
        })
    }
}

impl<T: BinRepr> BinRepr for Vec<T> {
    fn write_to<W: Write>(
        &self,
        writer: &mut W,
    ) -> io::Result<()> {
        let len = u32::try_from(self.len()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "vector is too long",
            )
        })?;

        writer.write_all(&len.to_le_bytes())?;

        for item in self {
            item.write_to(writer)?;
        }

        Ok(())
    }

    fn read_from(reader: &mut Reader<'_>) -> io::Result<Self> {
        reader.enter_depth()?;

        let result: io::Result<Self> = (|| {
            let len = reader.read_len()?;

            let alloc_size = len
                .checked_mul(std::mem::size_of::<T>())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "vector allocation size overflow",
                    )
                })?;

            reader.reserve_alloc(alloc_size)?;

            let mut values = Vec::new();
            values.try_reserve_exact(len).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("cannot allocate vector: {e}"),
                )
            })?;

            for _ in 0..len {
                values.push(T::read_from(reader)?);
            }

            Ok(values)
        })();

        reader.leave_depth();
        result
    }
}

impl<T: BinRepr> BinRepr for Option<T> {
    fn write_to<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        match self {
            None => writer.write_all(&[0]),
            Some(value) => {
                writer.write_all(&[1])?;
                value.write_to(writer)
            }
        }
    }
 
    fn read_from(
        reader: &mut crate::reader::Reader<'_>,
    ) -> std::io::Result<Self> {
        use std::io;

        reader.enter_depth()?;

        let result: io::Result<Self> = (|| {
            let mut tag = [0u8; 1];
            reader.read_exact(&mut tag)?;

            match tag[0] {
                0 => Ok(None),
                1 => Ok(Some(T::read_from(reader)?)),
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid Option tag",
                )),
            }
        })();

        reader.leave_depth();
        result
    }
}


#[cfg(test)]
mod security_tests {
    use crate::reader::Reader;
    use std::io::ErrorKind;

    #[test]
    fn rejects_invalid_option_tag() {
        let bytes = [2u8];
        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<Option<u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_truncated_option_value() {
        // Some(u32)
        let bytes = [1u8, 42, 0];
        let mut reader = Reader::from_bytes(&bytes);

        let result = reader.read::<Option<u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn rejects_vector_exceeding_allocation_budget() {
        // Длина вектора: 2 элемента u32.
        let bytes = [2u8, 0, 0, 0, 10, 0, 0, 0, 20, 0, 0, 0];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            4,
            64,
        );

        let result = reader.read::<Vec<u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn enforces_maximum_nesting_depth() {
        // Option<Option<u32>>
        let bytes = [1u8, 1, 42, 0, 0, 0];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            100,
            1,
        );

        let result = reader.read::<Option<Option<u32>>>();

        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[test]
    fn accepts_empty_vector_with_zero_allocation_budget() {
        let bytes = [0u8, 0, 0, 0];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            0,
            64,
        );

        let values = reader.read::<Vec<u32>>().unwrap();

        assert!(values.is_empty());
        assert_eq!(reader.allocated(), 0);
    }

    #[test]
    fn rejects_vector_exceeding_collection_limit() {
        let bytes = [
            3u8, 0, 0, 0,
            10, 0, 0, 0,
            20, 0, 0, 0,
            30, 0, 0, 0,
        ];

        let mut reader = Reader::with_decode_limits(
            &bytes,
            2,
            100,
            100,
            1024,
            64,
        );

        let result = reader.read::<Vec<u32>>();

        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );

        assert_eq!(reader.allocated(), 0);
    }

    #[test]
    fn enforces_shared_allocation_budget_for_strings() {
        let bytes = [
            2u8, 0, 0, 0,
            5, 0, 0, 0, b'h', b'e', b'l', b'l', b'o',
            5, 0, 0, 0, b'w', b'o', b'r', b'l', b'd',
        ];

        let budget = 2 * std::mem::size_of::<String>() + 9;

        let mut reader = Reader::with_decode_limits(
            &bytes,
            100,
            100,
            100,
            budget,
            64,
        );

        let result = reader.read::<Vec<String>>();

        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );

        assert_eq!(
            reader.allocated(),
            2 * std::mem::size_of::<String>() + 5
        );
    }
}
