//! Repository paths as project paths: the part of git's path below the project folder.

use theos_vcs::Folding;

/// The path of `repo_path` below the project at `place`, compared by components under `folding`
/// and kept in git's spelling, or `None` when it lies outside the project or is not UTF-8.
pub(crate) fn project_path(folding: Folding, place: &str, repo_path: &[u8]) -> Option<String> {
    let rest = folding.below(place, repo_path)?;
    std::str::from_utf8(rest).ok().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXACT: Folding = Folding {
        ignore_case: false,
        precompose: false,
    };
    const CASE: Folding = Folding {
        ignore_case: true,
        precompose: false,
    };
    const NFC: Folding = Folding {
        ignore_case: false,
        precompose: true,
    };

    #[test]
    fn project_path_strips_the_place() {
        assert_eq!(
            project_path(EXACT, "", b"a/b.md").as_deref(),
            Some("a/b.md")
        );
        assert_eq!(
            project_path(EXACT, "content", b"content/a.md").as_deref(),
            Some("a.md")
        );
        assert_eq!(project_path(EXACT, "content", b"content2/x.md"), None);
        assert_eq!(project_path(EXACT, "content", b"other/a.md"), None);
        assert_eq!(project_path(EXACT, "", b"caf\xe9.md"), None);
    }

    #[test]
    fn project_path_compares_the_place_under_the_folding() {
        // The place differs in case: git's spelling of the rest is kept.
        assert_eq!(
            project_path(CASE, "Content", b"content/lore/A.md").as_deref(),
            Some("lore/A.md")
        );
        assert_eq!(project_path(EXACT, "Content", b"content/lore/A.md"), None);
        // The place is NFD on disk, NFC in the index.
        let decomposed = "Cafe\u{301}";
        let composed = "Caf\u{e9}/a.md";
        assert_eq!(
            project_path(NFC, decomposed, composed.as_bytes()).as_deref(),
            Some("a.md")
        );
        assert_eq!(project_path(EXACT, decomposed, composed.as_bytes()), None);
    }
}
