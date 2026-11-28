//! The atlas part.

use super::{Part, RailEntry};

/// The atlas part.
pub const PART: Part = Part {
    id: "atlas",
    rail: Some(RailEntry {
        id: "atlas",
        label: "Atlas",
        glyph: "⌾",
        order: 20,
        phase: Some("P25"),
    }),
    always: false,
};
