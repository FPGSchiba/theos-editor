//! The MCP part: it has tools and settings but no rail entry.

use super::Part;

/// The mcp part.
pub const PART: Part = Part {
    id: "mcp",
    rail: None,
    always: false,
};
