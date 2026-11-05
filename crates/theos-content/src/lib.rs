//! Record model, front-matter patcher, body grammars (beats, voice, notes), kind registry, index,
//! search, effective lenses, validators, lore rules and ledger. Pure filesystem; no async; no UI.
//!
//! So far the crate provides [`text`]: byte spans, line and column positions, line-ending
//! normalisation and span-based text edits. Everything else follows in later P02 tasks.

pub mod text;
