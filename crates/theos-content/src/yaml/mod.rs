//! The restricted YAML of the data model: one pass of `yaml-rust2` over the front matter, and
//! one module per rule.

mod encode;
mod error;
mod input;
mod model;
mod rules;
mod scalar;

pub use encode::{EncodeError, Slot, encode};
pub use error::{YamlError, YamlRule};
pub(crate) use input::Input;
pub use model::{
    Entry, EntryValue, Field, FrontMatter, Item, List, ListStyle, Map, RawEntry, RawValue, Value,
};
pub(crate) use rules::first_error;
pub use scalar::{Decimal, Lexical, Scalar, ScalarError, ScalarStyle};
