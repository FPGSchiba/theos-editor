//! The maps part.

use super::{Part, RailEntry};

/// The maps part.
pub const PART: Part = Part {
    id: "maps",
    rail: Some(RailEntry {
        id: "maps",
        label: "Maps",
        glyph: "△",
        order: 60,
        phase: Some("P16"),
    }),
    always: false,
};
