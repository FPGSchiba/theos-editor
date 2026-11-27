//! Diffs of texts and records, for the review of changes (P46).
//!
//! - [`words`]: a word-level diff of two texts.
//! - [`fields`]: old and new per front matter field, compared by value.
//! - [`record`]: a record's fields plus a word diff of its body.

pub mod fields;
pub mod record;
pub mod words;

pub use fields::{FieldChange, FieldDelta, ItemDelta, fields};
pub use record::{FieldsDiff, RecordDiff, record};
pub use words::{WordDiff, WordOp, words};
