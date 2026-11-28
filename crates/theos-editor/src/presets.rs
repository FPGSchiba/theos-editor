//! The layout presets the build brings: one per rail entry of the content parts, and, with the
//! cargo feature `engine`, the engine part's (none yet; architecture §14).
//!
//! The presets themselves are data in the UI (P11 ruling 3); Rust names which ones exist.

/// The presets of the content build: the ids of the content parts that have a rail entry.
#[must_use]
pub fn content_presets() -> Vec<&'static str> {
    theos_studio::parts::content()
        .iter()
        .filter_map(|part| part.rail.map(|entry| entry.id))
        .collect()
}

/// The presets of the engine part. Empty until the engine part brings its own (CE-32.3).
#[cfg(feature = "engine")]
#[must_use]
pub fn engine_presets() -> Vec<&'static str> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rail_entry_has_a_preset() {
        let presets = content_presets();
        assert_eq!(presets.first(), Some(&"codex"));
        assert!(presets.contains(&"settings"), "{presets:?}");
        assert!(!presets.contains(&"mcp"), "{presets:?}");
    }
}
