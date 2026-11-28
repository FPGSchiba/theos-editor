//! The settings part.

use super::{Part, RailEntry};

/// The settings part.
pub const PART: Part = Part {
    id: "settings",
    rail: Some(RailEntry {
        id: "settings",
        label: "Settings",
        glyph: "⚙",
        order: 100,
        phase: Some("P14"),
    }),
    always: true,
};
