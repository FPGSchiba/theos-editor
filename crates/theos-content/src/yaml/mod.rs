//! The restricted YAML of the data model: one pass of `yaml-rust2` over the front matter, and
//! one module per rule.

mod error;
mod input;
mod rules;

pub use error::{YamlError, YamlRule};
pub(crate) use input::Input;
pub(crate) use rules::first_error;
