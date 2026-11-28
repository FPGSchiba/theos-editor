//! The changes part.

use super::{Part, RailEntry};

/// The changes part.
pub const PART: Part = Part {
    id: "changes",
    rail: Some(RailEntry {
        id: "changes",
        label: "Changes",
        glyph: "±",
        order: 80,
        phase: Some("P13"),
    }),
    always: false,
};
