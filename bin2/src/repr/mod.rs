use std::io::{self, Write};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;


use crate::reader::Reader;
mod tests;

/// Defines a type that can be serialized to and deserialized from a binary
/// representation.
///
/// Implementations specify how values are written to an output stream and
/// reconstructed from a [`Reader`].
///
/// Types implementing `BinRepr` must ensure that their serialization and
/// deserialization formats are compatible.
pub trait BinRepr: Sized {
    /// Serializes the value into the provided writer.
    ///
    /// The binary representation is written to the output stream.
    ///
    /// # Errors
    ///
    /// Returns an error if writing the binary representation fails.
    fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()>;

    /// Deserializes a value from the provided reader.
    ///
    /// Reads the binary representation and reconstructs an instance of
    /// the implementing type.
    ///
    /// # Errors
    ///
    /// Returns an error if reading or decoding the value fails, including
    /// when the reader's configured decoding limits are exceeded.
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

impl BinRepr for bool {
    /// Writes the boolean value as a single byte.
    ///
    /// # Errors
    ///
    /// Returns an error if writing the byte to the output fails.
    fn write_to<W: Write>(
        &self,
        writer: &mut W,
    ) -> io::Result<()> {
        writer.write_all(&[u8::from(*self)])
    }

    /// Reads a boolean value from the reader.
    ///
    /// Expects exactly one byte, where `0` represents `false` and `1`
    /// represents `true`.
    ///
    /// # Errors
    ///
    /// Returns an error if reading fails or the byte is not `0` or `1`.
    fn read_from(reader: &mut Reader<'_>) -> io::Result<Self> {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte)?;

        match byte[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid bool value",
            )),
        }
    }
}


impl BinRepr for char {
    /// Writes the character as a 32-bit little-endian unsigned integer.
    ///
    /// # Errors
    ///
    /// Returns an error if writing the encoded character to the output fails.
    fn write_to<W: Write>(
        &self,
        writer: &mut W,
    ) -> io::Result<()> {
        writer.write_all(&(*self as u32).to_le_bytes())
    }

    /// Reads a character from the reader.
    ///
    /// Reads a 32-bit little-endian unsigned integer and interprets it as
    /// a Unicode scalar value.
    ///
    /// # Errors
    ///
    /// Returns an error if reading fails or the decoded value is not a
    /// valid Unicode scalar value.
    fn read_from(reader: &mut Reader<'_>) -> io::Result<Self> {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes)?;

        let value = u32::from_le_bytes(bytes);

        char::from_u32(value).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid Unicode scalar value",
            )
        })
    }
}


impl BinRepr for String {
    /// Writes the string's byte length followed by its UTF-8 contents.
    ///
    /// The length is encoded as a 32-bit unsigned integer in little-endian
    /// byte order.
    ///
    /// # Errors
    ///
    /// Returns an error if the string length exceeds the range of `u32`
    /// or if writing to the output fails.
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

    /// Reads a string from the reader.
    ///
    /// Reads the encoded byte length, checks it against the configured
    /// string and allocation limits, and decodes the following bytes as UTF-8.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Reading the length or string contents fails.
    /// - The string length exceeds the configured limit.
    /// - The allocation budget is exceeded.
    /// - Allocating the string buffer fails.
    /// - The contents are not valid UTF-8.
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
    /// Writes the vector length followed by the binary representation
    /// of each element.
    ///
    /// The length is encoded as a 32-bit unsigned integer in little-endian
    /// byte order. Elements are serialized in their original order.
    ///
    /// # Errors
    ///
    /// Returns an error if the vector length exceeds the range of `u32`
    /// or if writing the length or any element fails.
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

    /// Reads a vector from the reader.
    ///
    /// Reads the encoded element count, validates it against the configured
    /// collection length and allocation limits, and deserializes each element
    /// in sequence.
    ///
    /// Decoding the vector increases the reader's nesting depth for the
    /// duration of the operation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Reading the vector length or any element fails.
    /// - The element count exceeds the configured collection limit.
    /// - The required allocation size overflows.
    /// - The allocation budget is exceeded.
    /// - Allocating the vector fails.
    /// - The maximum decoding depth is exceeded.
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
    /// Writes the option's presence tag and, if present, its contained value.
    ///
    /// # Errors
    ///
    /// Returns an error if writing the tag or the contained value fails.
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

    /// Reads an optional value from the reader.
    ///
    /// Reads a one-byte tag to determine whether the value is `None` or
    /// `Some`. If the tag is `1`, the contained value is deserialized using
    /// its [`BinRepr`] implementation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Reading the tag or contained value fails.
    /// - The tag is neither `0` nor `1`.
    /// - The maximum decoding depth is exceeded.
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


impl<T: BinRepr, const N: usize> BinRepr for [T; N] {
    /// Writes the binary representation of each array element in order.
    ///
    /// # Errors
    ///
    /// Returns an error if writing any element to the output fails.
    fn write_to<W: Write>(
        &self,
        writer: &mut W,
    ) -> io::Result<()> {
        for item in self {
            item.write_to(writer)?;
        }

        Ok(())
    }

    /// Reads a fixed-size array from the reader.
    ///
    /// Deserializes exactly `N` elements in sequence. The reader's decoding
    /// depth is increased for the duration of the operation.
    ///
    /// If deserialization fails, all successfully initialized elements
    /// are dropped before the error is returned.
    ///
    /// # Errors
    ///
    /// Returns an error if reading any element fails or the maximum
    /// decoding depth is exceeded.
    fn read_from(reader: &mut Reader<'_>) -> io::Result<Self> {
        use std::mem::MaybeUninit;

        reader.enter_depth()?;

        let result = (|| {
            let mut values = MaybeUninit::<[T; N]>::uninit();
            let ptr = values.as_mut_ptr().cast::<T>();

            for i in 0..N {
                match T::read_from(reader) {
                    Ok(value) => unsafe {
                        ptr.add(i).write(value);
                    },
                    Err(error) => {
                        // Drop all successfully initialized elements.
                        for j in 0..i {
                            unsafe {
                                ptr.add(j).drop_in_place();
                            }
                        }

                        return Err(error);
                    }
                }
            }

            // All N elements have been initialized.
            Ok(unsafe { values.assume_init() })
        })();

        reader.leave_depth();
        result
    }
}


macro_rules! impl_tuple_bin_repr {
    ($(($($T:ident : $idx:tt : $value:ident),+)),+ $(,)?) => {
        $(
            impl<$($T: BinRepr),+> BinRepr for ($($T,)+) {
                fn write_to<W: Write>(
                    &self,
                    writer: &mut W,
                ) -> io::Result<()> {
                    $(
                        self.$idx.write_to(writer)?;
                    )+

                    Ok(())
                }

                fn read_from(
                    reader: &mut Reader<'_>,
                ) -> io::Result<Self> {
                    reader.enter_depth()?;

                    let result = (|| {
                        $(
                            let $value = $T::read_from(reader)?;
                        )+

                        Ok(($($value,)+))
                    })();

                    reader.leave_depth();
                    result
                }
            }
        )+
    };
}

// NOTE: bin2 only supports a maximum of 12 tuple elements!
impl_tuple_bin_repr! {
    (T1:0:v1),
    (T1:0:v1, T2:1:v2),
    (T1:0:v1, T2:1:v2, T3:2:v3),
    (T1:0:v1, T2:1:v2, T3:2:v3, T4:3:v4),
    (T1:0:v1, T2:1:v2, T3:2:v3, T4:3:v4, T5:4:v5),
    (T1:0:v1, T2:1:v2, T3:2:v3, T4:3:v4, T5:4:v5, T6:5:v6),
    (T1:0:v1, T2:1:v2, T3:2:v3, T4:3:v4, T5:4:v5, T6:5:v6, T7:6:v7),
    (T1:0:v1, T2:1:v2, T3:2:v3, T4:3:v4, T5:4:v5, T6:5:v6, T7:6:v7, T8:7:v8),
    (T1:0:v1, T2:1:v2, T3:2:v3, T4:3:v4, T5:4:v5, T6:5:v6, T7:6:v7, T8:7:v8, T9:8:v9),
    (T1:0:v1, T2:1:v2, T3:2:v3, T4:3:v4, T5:4:v5, T6:5:v6, T7:6:v7, T8:7:v8, T9:8:v9, T10:9:v10),
    (T1:0:v1, T2:1:v2, T3:2:v3, T4:3:v4, T5:4:v5, T6:5:v6, T7:6:v7, T8:7:v8, T9:8:v9, T10:9:v10, T11:10:v11),
    (T1:0:v1, T2:1:v2, T3:2:v3, T4:3:v4, T5:4:v5, T6:5:v6, T7:6:v7, T8:7:v8, T9:8:v9, T10:9:v10, T11:10:v11, T12:11:v12),
}

impl<K, V> BinRepr for HashMap<K, V>
where
    K: BinRepr + Eq + Hash,
    V: BinRepr,
{
    /// Writes the number of entries followed by each key-value pair.
    ///
    /// The entry count is encoded as a 32-bit unsigned integer in
    /// little-endian byte order.
    ///
    /// # Errors
    ///
    /// Returns an error if the number of entries exceeds the range of `u32`
    /// or if writing the entry count, any key, or any value fails.
    fn write_to<W: Write>(
        &self,
        writer: &mut W,
    ) -> io::Result<()> {
        let len = u32::try_from(self.len()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "hash map is too large",
            )
        })?;

        writer.write_all(&len.to_le_bytes())?;

        for (key, value) in self {
            key.write_to(writer)?;
            value.write_to(writer)?;
        }

        Ok(())
    }

    /// Reads a hash map from the reader.
    ///
    /// Reads the encoded entry count, validates it against the configured
    /// collection and allocation limits, and deserializes each key-value
    /// pair. The reader's decoding depth is increased for the duration
    /// of the operation.
    ///
    /// Duplicate keys are rejected rather than silently overwriting
    /// previously decoded entries.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Reading the entry count, any key, or any value fails.
    /// - The entry count exceeds the configured collection limit.
    /// - The required allocation size overflows.
    /// - The allocation budget is exceeded.
    /// - Reserving memory for the hash map fails.
    /// - A duplicate key is encountered.
    /// - The maximum decoding depth is exceeded.
    fn read_from(reader: &mut Reader<'_>) -> io::Result<Self> {
        reader.enter_depth()?;

        let result = (|| {
            let len = reader.read_len()?;

            let alloc_size = len
                .checked_mul(std::mem::size_of::<(K, V)>())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "hash map allocation size overflow",
                    )
                })?;

            reader.reserve_alloc(alloc_size)?;

            let mut map = HashMap::new();

            map.try_reserve(len).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("cannot allocate hash map: {e}"),
                )
            })?;

            for _ in 0..len {
                let key = K::read_from(reader)?;
                let value = V::read_from(reader)?;

                if map.insert(key, value).is_some() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "duplicate hash map key",
                    ));
                }
            }

            Ok(map)
        })();

        reader.leave_depth();
        result
    }
}

impl<T> BinRepr for HashSet<T>
where
    T: BinRepr + Eq + Hash,
{
    /// Writes the number of elements followed by their binary representations.
    ///
    /// The element count is encoded as a 32-bit unsigned integer in
    /// little-endian byte order.
    ///
    /// # Errors
    ///
    /// Returns an error if the number of elements exceeds the range of `u32`
    /// or if writing the element count or any element fails.
    fn write_to<W: Write>(
        &self,
        writer: &mut W,
    ) -> io::Result<()> {
        let len = u32::try_from(self.len()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "hash set is too large",
            )
        })?;

        writer.write_all(&len.to_le_bytes())?;

        for value in self {
            value.write_to(writer)?;
        }

        Ok(())
    }

    /// Reads a hash set from the reader.
    ///
    /// Reads the encoded element count, validates it against the configured
    /// collection and allocation limits, and deserializes each element.
    /// The reader's decoding depth is increased for the duration of
    /// the operation.
    ///
    /// Duplicate elements are rejected rather than silently ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Reading the element count or any element fails.
    /// - The element count exceeds the configured collection limit.
    /// - The required allocation size overflows.
    /// - The allocation budget is exceeded.
    /// - Reserving memory for the hash set fails.
    /// - A duplicate element is encountered.
    /// - The maximum decoding depth is exceeded.
    fn read_from(reader: &mut Reader<'_>) -> io::Result<Self> {
        reader.enter_depth()?;

        let result = (|| {
            let len = reader.read_len()?;

            let alloc_size = len
                .checked_mul(std::mem::size_of::<T>())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "hash set allocation size overflow",
                    )
                })?;

            reader.reserve_alloc(alloc_size)?;

            let mut set = HashSet::new();

            set.try_reserve(len).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("cannot allocate hash set: {e}"),
                )
            })?;

            for _ in 0..len {
                let value = T::read_from(reader)?;

                if !set.insert(value) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "duplicate hash set element",
                    ));
                }
            }

            Ok(set)
        })();

        reader.leave_depth();
        result
    }
}
