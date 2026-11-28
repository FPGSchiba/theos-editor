//! The code part.

use super::{Part, RailEntry};

/// The code part.
pub const PART: Part = Part {
    id: "code",
    rail: Some(RailEntry {
        id: "code",
        label: "Code",
        glyph: "</>",
        order: 90,
        phase: Some("P18"),
    }),
    always: true,
};
