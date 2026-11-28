//! The review part.

use super::{Part, RailEntry};

/// The review part.
pub const PART: Part = Part {
    id: "review",
    rail: Some(RailEntry {
        id: "review",
        label: "Review",
        glyph: "◴",
        order: 70,
        phase: Some("P13"),
    }),
    always: false,
};
