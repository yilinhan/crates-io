//! [`Value`] and friends: types representing valid configuration values.
//!
mod value;
mod ser;
#[cfg(feature = "parse-value")]
mod parse;
mod de;
mod tag;

pub mod magic;

pub(crate) use {self::ser::*, self::de::*};
pub use tag::Tag;
pub use value::{Value, Map, Num, Dict, Empty};
pub use uncased::{Uncased, UncasedStr};
