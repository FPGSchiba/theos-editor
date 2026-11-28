//! The codex part.

use super::{Part, RailEntry};

/// The codex part.
pub const PART: Part = Part {
    id: "codex",
    rail: Some(RailEntry {
        id: "codex",
        label: "Codex",
        glyph: "◈",
        order: 10,
        phase: None,
    }),
    always: false,
};
