use std::io::{self, Write};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;


use crate::reader::Reader;
mod tests;

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

impl BinRepr for bool {
    fn write_to<W: Write>(
        &self,
        writer: &mut W,
    ) -> io::Result<()> {
        writer.write_all(&[u8::from(*self)])
    }

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
    fn write_to<W: Write>(
        &self,
        writer: &mut W,
    ) -> io::Result<()> {
        writer.write_all(&(*self as u32).to_le_bytes())
    }

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


impl<T: BinRepr, const N: usize> BinRepr for [T; N] {
    fn write_to<W: Write>(
        &self,
        writer: &mut W,
    ) -> io::Result<()> {
        for item in self {
            item.write_to(writer)?;
        }

        Ok(())
    }

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

// NOTE: bin2 supports a maximum of 12 tuple elements
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
