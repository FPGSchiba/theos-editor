//! The graph part.

use super::{Part, RailEntry};

/// The graph part.
pub const PART: Part = Part {
    id: "graph",
    rail: Some(RailEntry {
        id: "graph",
        label: "Graph",
        glyph: "⋈",
        order: 30,
        phase: Some("P15"),
    }),
    always: false,
};
