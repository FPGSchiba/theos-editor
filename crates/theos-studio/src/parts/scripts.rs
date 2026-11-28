//! The scripts part.

use super::{Part, RailEntry};

/// The scripts part.
pub const PART: Part = Part {
    id: "scripts",
    rail: Some(RailEntry {
        id: "scripts",
        label: "Scripts",
        glyph: "¶",
        order: 40,
        phase: Some("P15"),
    }),
    always: false,
};
