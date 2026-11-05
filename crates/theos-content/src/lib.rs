//! Record model, front-matter patcher, body grammars (beats, voice, notes), kind registry, index,
//! search, effective lenses, validators, lore rules and ledger. Pure filesystem; no async; no UI.
//!
//! So far the crate provides [`text`] (byte spans, line and column positions, line-ending
//! normalisation and span-based text edits) and [`record`] (reading a record and splitting it into
//! front matter and body). Everything else follows in later P02 tasks.

pub mod record;
pub mod text;
pub mod yaml;

#[cfg(test)]
mod test_support;

pub use record::{ReadError, Record, RecordEditError, RecordError, decode, read_record};
pub use text::{Position, Span, TextEdit, TextIndex, apply_edits};
pub use yaml::{YamlError, YamlRule};
