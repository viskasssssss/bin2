mod writer;
mod reader;
mod repr;

pub use writer::*;
pub use reader::*;
pub use repr::BinRepr;

#[cfg(feature = "bin2-derive")]
extern crate bin2_derive;

#[cfg(feature = "bin2-derive")]
pub use bin2_derive::BinRepr;