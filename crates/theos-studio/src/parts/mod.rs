//! The built-in editor parts and the rail a project shows.
//!
//! One module per part, each with one `PART`; [`content()`] lists them, one line each.

mod atlas;
mod changes;
mod code;
mod codex;
mod concepts;
mod graph;
mod maps;
mod mcp;
mod review;
mod scripts;
mod settings;

/// One button of the rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RailEntry {
    /// The part this button opens.
    pub id: &'static str,
    /// The label the UI shows.
    pub label: &'static str,
    /// The glyph on the button.
    pub glyph: &'static str,
    /// Position on the rail, ascending.
    pub order: u8,
    /// The phase that builds the part's view, or `None` when it is built.
    pub phase: Option<&'static str>,
}

/// An editor part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Part {
    /// The name a project lists in `enabled`.
    pub id: &'static str,
    /// Its rail button, or `None` for a part without a view.
    pub rail: Option<RailEntry>,
    /// True when the part is on in every project (`code`, `settings`).
    pub always: bool,
}

/// Every part of the content build.
#[must_use]
pub fn content() -> Vec<Part> {
    vec![
        codex::PART,
        atlas::PART,
        graph::PART,
        scripts::PART,
        concepts::PART,
        maps::PART,
        review::PART,
        changes::PART,
        mcp::PART,
        code::PART,
        settings::PART,
    ]
}

/// The rail of a project with these parts enabled, in rail order. The `always` parts are added;
/// an unknown name is skipped ([`unknown`] lists them, for a warning when the project opens).
#[must_use]
pub fn rail(enabled: &[String]) -> Vec<RailEntry> {
    let parts = content();
    let mut rail: Vec<RailEntry> = parts
        .iter()
        .filter(|part| part.always || enabled.iter().any(|name| name == part.id))
        .filter_map(|part| part.rail)
        .collect();
    rail.sort_by_key(|entry| entry.order);
    rail
}

/// The names in `enabled` that name no part of the content build, in their order.
#[must_use]
pub fn unknown(enabled: &[String]) -> Vec<&str> {
    let parts = content();
    enabled
        .iter()
        .filter(|name| !parts.iter().any(|part| part.id == name.as_str()))
        .map(String::as_str)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    fn tower() -> Vec<String> {
        names(&[
            "codex", "changes", "scripts", "graph", "concepts", "maps", "review", "mcp",
        ])
    }

    // AC: CE-32.1
    #[test]
    fn rail_of_tower_of_theos() {
        let rail = rail(&tower());
        let ids: Vec<_> = rail.iter().map(|e| e.id).collect();
        assert_eq!(
            ids,
            [
                "codex", "graph", "scripts", "concepts", "maps", "review", "changes", "code",
                "settings"
            ]
        );
        let glyphs: Vec<_> = rail.iter().map(|e| e.glyph).collect();
        assert_eq!(glyphs, ["◈", "⋈", "¶", "◫", "△", "◴", "±", "</>", "⚙"]);
        let phases: Vec<_> = rail.iter().map(|e| e.phase).collect();
        assert_eq!(
            phases,
            [
                None,
                Some("P15"),
                Some("P15"),
                Some("P16"),
                Some("P16"),
                Some("P13"),
                Some("P13"),
                Some("P18"),
                Some("P14")
            ]
        );
    }

    // AC: CE-32.1
    #[test]
    fn atlas_follows_codex_when_enabled() {
        let mut enabled = tower();
        enabled.push("atlas".to_owned());
        let rail = rail(&enabled);
        assert_eq!(rail[0].id, "codex");
        assert_eq!(rail[1].id, "atlas");
        assert_eq!(rail[1].phase, Some("P25"));
        assert_eq!(rail[1].glyph, "⌾");
    }

    #[test]
    fn unknown_names_are_skipped() {
        let rail = rail(&names(&["codex", "nonsense"]));
        let ids: Vec<_> = rail.iter().map(|e| e.id).collect();
        assert_eq!(ids, ["codex", "code", "settings"]);
        assert_eq!(
            unknown(&names(&["codex", "nonsense", "maps", "other"])),
            ["nonsense", "other"]
        );
        assert_eq!(unknown(&tower()), Vec::<&str>::new());
    }

    #[test]
    fn empty_enabled_gives_code_and_settings() {
        let rail = rail(&[]);
        let ids: Vec<_> = rail.iter().map(|e| e.id).collect();
        assert_eq!(ids, ["code", "settings"]);
    }
}
