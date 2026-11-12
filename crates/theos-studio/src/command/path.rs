//! The rules for a path a command is given. The changes of a plan must also leave the boundary
//! between records and other files where the verdict sees it (`boundary`, ruling R21): no
//! `.gitignore`, no hidden file replaced on Windows, and no path spelled unlike the disk.

use super::CommandError;

/// The project-relative path with `/` for `path`: a `\` is read as `/`.
///
/// # Errors
///
/// [`CommandError::Path`] when the path is empty, absolute, has a drive or stream colon, has an
/// empty, `.` or `..` component, or lies inside `.git` or `.theos/cache`.
pub(crate) fn project_path(path: &str) -> Result<String, CommandError> {
    let unified = path.replace('\\', "/");
    match broken_rule(&unified) {
        None => Ok(unified),
        Some(reason) => Err(CommandError::Path {
            path: path.to_owned(),
            reason,
        }),
    }
}

/// The rule that `path` (with `/` only) breaks, if any.
fn broken_rule(path: &str) -> Option<&'static str> {
    if path.is_empty() {
        return Some("is empty");
    }
    if path.starts_with('/') {
        return Some("is absolute");
    }
    let parts = path.split('/');
    if parts.clone().any(|part| part == "..") {
        return Some("leaves the project");
    }
    if parts.clone().any(|part| part.contains(':')) {
        return Some("has a drive or stream colon");
    }
    if parts.clone().any(|part| part.is_empty() || part == ".") {
        return Some("has an empty or `.` component");
    }
    if parts.clone().any(|part| part.ends_with(['.', ' '])) {
        return Some("ends in a dot or a space");
    }
    // `git~1` is the short name of `.git` on Windows.
    if parts.clone().any(|part| part.eq_ignore_ascii_case("git~1"))
        || parts
            .clone()
            .next()
            .is_some_and(|first| first.eq_ignore_ascii_case(".git"))
    {
        return Some("is inside .git");
    }
    // The cache holds the repo lock and the shared index (ruling R15).
    let mut top = parts;
    if top.next().is_some_and(names_theos)
        && top
            .next()
            .is_some_and(|second| second.eq_ignore_ascii_case("cache"))
    {
        return Some("is inside .theos/cache");
    }
    None
}

/// Whether the first component `part` names the folder `.theos` on some OS: in any case, or as
/// `theos~1`, its short name on Windows (as `git~1` is that of `.git`).
fn names_theos(part: &str) -> bool {
    part.eq_ignore_ascii_case(".theos") || part.eq_ignore_ascii_case("theos~1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_outside_the_project_are_refused() {
        for bad in [
            "../outside.md",
            "C:\\x.md",
            "C:/x.md",
            "/abs.md",
            "\\\\server\\x.md",
            ".git/config",
            ".GIT/config",
            "lore/./x.md",
            "lore//x.md",
            "lore/x.md/",
            "lore/../../x.md",
            "lore/x.md:stream",
            "",
            ".theos/cache/lock",
            ".theos/cache/index.sqlite",
            ".THEOS/Cache/x",
            ".theos\\cache\\lock",
            ".theos/cache",
            "THEOS~1/cache/lock",
        ] {
            assert!(
                matches!(project_path(bad), Err(CommandError::Path { .. })),
                "{bad:?} must be refused"
            );
        }
    }

    #[test]
    fn a_backslash_reads_as_a_slash() -> Result<(), CommandError> {
        assert_eq!(
            project_path("lore\\truths\\T-960.md")?,
            "lore/truths/T-960.md"
        );
        assert_eq!(project_path(".gitignore")?, ".gitignore");
        Ok(())
    }

    #[test]
    fn the_reason_names_the_rule() {
        assert!(matches!(
            project_path("../x"),
            Err(CommandError::Path {
                reason: "leaves the project",
                ..
            })
        ));
        assert!(matches!(
            project_path(".git/config"),
            Err(CommandError::Path {
                reason: "is inside .git",
                ..
            })
        ));
        assert!(matches!(
            project_path(".theos/cache/lock"),
            Err(CommandError::Path {
                reason: "is inside .theos/cache",
                ..
            })
        ));
    }

    /// Ruling R15: the cache is refused, the rest of `.theos` stays writable, and a `cache` folder
    /// elsewhere is no cache of the project.
    #[test]
    fn the_theos_folder_outside_its_cache_is_allowed() -> Result<(), CommandError> {
        for good in [
            ".theos/settings.toml",
            ".theos/comments/x.toml",
            "lore/.theos/cache/x.md",
            "cache/x.md",
        ] {
            assert_eq!(project_path(good)?, good);
        }
        Ok(())
    }

    #[test]
    fn the_windows_spellings_of_git_are_refused() {
        for bad in [
            ".git./config",
            ".git /config",
            "GIT~1/config",
            "git~1/config",
            "lore/T-960.md.",
            "lore /T-960.md",
        ] {
            assert!(
                matches!(project_path(bad), Err(CommandError::Path { .. })),
                "{bad:?} must be refused"
            );
        }
        assert!(project_path(".gitignore").is_ok());
        assert!(project_path("lore/.gitkeep").is_ok());
    }
}
