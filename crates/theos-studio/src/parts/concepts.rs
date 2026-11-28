//! The concepts part.

use super::{Part, RailEntry};

/// The concepts part.
pub const PART: Part = Part {
    id: "concepts",
    rail: Some(RailEntry {
        id: "concepts",
        label: "Concepts",
        glyph: "◫",
        order: 50,
        phase: Some("P16"),
    }),
    always: false,
};
