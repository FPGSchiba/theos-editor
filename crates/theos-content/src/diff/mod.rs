//! Diffs of texts and records, for the review of changes (P46).
//!
//! - [`words`]: a word-level diff of two texts.

pub mod words;

pub use words::{WordDiff, WordOp, words};
