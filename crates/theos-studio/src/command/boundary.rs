//! The boundary between records and other files (ruling R21): no plan may move it where the
//! verdict cannot see it. The preview decides which files are records from the disk as it is
//! before the write (P03's walk: the spelling of a path, hidden files, `.gitignore` files, and
//! the names the folders list), so a change that alters one of these inputs would add or remove
//! records unjudged. The pipeline refuses such a change before it reads the plan's files. A
//! moved record that would not be a record at its new path is the verdict's (ruling R16).
//!
//! This module holds only the rules, which changes to refuse. The facts they judge by come from
//! P03's walk ([`walk::is_hidden_entry`], [`walk::disk_spelling`]), so that one place decides
//! which files are records.

use std::path::Path;

use theos_content::walk::{self, DiskSpelling};

use super::{CommandError, FileChange, Plan};

/// A rule: why a change would move the boundary unseen, `None` when it would not.
type Rule = fn(&Path, &FileChange) -> Option<&'static str>;

/// The rules, in the order they are checked. A new rule is one function and one line here.
const RULES: &[Rule] = &[
    gitignore,
    #[cfg(windows)]
    hidden,
    spelling,
];

/// [`CommandError::Path`] for the first real change of `plan` ([`FileChange::is_real`]) under
/// `root` that a rule refuses; a change that writes nothing moves nothing.
pub(super) fn check(root: &Path, plan: &Plan) -> Result<(), CommandError> {
    for change in plan.real_changes() {
        if let Some(reason) = RULES.iter().find_map(|rule| rule(root, change)) {
            return Err(CommandError::Path {
                path: change.path.clone(),
                reason,
            });
        }
    }
    Ok(())
}

/// A `.gitignore`, in any case of its name: it decides which files below its folder are records.
/// A write, a creation and a removal are refused alike, those of a move included.
fn gitignore(_root: &Path, change: &FileChange) -> Option<&'static str> {
    let name = change.path.rsplit('/').next().unwrap_or(&change.path);
    name.eq_ignore_ascii_case(".gitignore").then_some(
        "is a .gitignore, which decides which files are records; edit it outside the editor",
    )
}

/// On Windows, a file with the hidden attribute that the change replaces: the write swaps in a
/// new file, which is not hidden (`tempfile`'s `keep` gives it `FILE_ATTRIBUTE_NORMAL`), so the
/// file would become a record. Removing a hidden file takes no record away; a new file is not
/// hidden. On Unix a name that starts with `.` hides, and a write keeps the name.
#[cfg(windows)]
fn hidden(root: &Path, change: &FileChange) -> Option<&'static str> {
    let replaced = change.before.is_some() && change.after.is_some();
    // The file itself, as the walk judges it: a symbolic link by its own attributes.
    (replaced && walk::is_hidden_entry(&root.join(&change.path))).then_some("is hidden")
}

/// A path whose part on disk its folders list only under another spelling (another case, or a
/// short name on Windows), or that passes a symbolic link to a folder. The file system still
/// finds the file, so the write reaches a file that the walk lists under another path, or not at
/// all, while the preview judges the path as given. The part of the path not yet on disk is
/// created as spelled.
fn spelling(root: &Path, change: &FileChange) -> Option<&'static str> {
    match walk::disk_spelling(root, &change.path) {
        DiskSpelling::Exact => None,
        DiskSpelling::Other => Some("is spelled unlike the file or folder on disk"),
        DiskSpelling::ThroughLink => Some("passes a symbolic link to a folder"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test_support::{TestResult, change, plan};

    #[test]
    fn a_gitignore_is_named_by_its_last_component_in_any_case() {
        for path in [".gitignore", "lore/.gitignore", "lore/deep/.GitIgnore"] {
            let refused = gitignore(Path::new("."), &change(path, None, Some("x")));
            assert!(refused.is_some(), "{path}");
        }
        for path in ["lore/.gitignore.md", "lore/x.gitignore", ".gitignore/x.md"] {
            assert_eq!(
                gitignore(Path::new("."), &change(path, None, Some("x"))),
                None,
                "{path}"
            );
        }
    }

    /// A change that writes nothing moves nothing, so no rule judges it.
    #[test]
    fn a_change_that_writes_nothing_is_not_judged() -> TestResult {
        let dir = tempfile::tempdir()?;
        let same = change(".gitignore", Some("x\n"), Some("x\n"));
        check(dir.path(), &plan(vec![same]))?;
        let result = check(
            dir.path(),
            &plan(vec![change("lore/.gitignore", Some("x\n"), None)]),
        );
        assert!(
            matches!(&result, Err(CommandError::Path { path, .. }) if path == "lore/.gitignore"),
            "{result:?}"
        );
        Ok(())
    }

    /// The part of a path on disk is judged; the part a write creates is spelled as given.
    #[test]
    fn a_path_spelled_as_its_folders_list_it_passes() -> TestResult {
        let dir = tempfile::tempdir()?;
        fs::create_dir_all(dir.path().join("lore").join("truths"))?;
        fs::write(dir.path().join("lore").join("truths").join("T-960.md"), "x")?;
        for path in [
            "lore/truths/T-960.md",
            "lore/truths/T-961.md",
            "lore/new/T-962.md",
            "LORE-other/x.md",
        ] {
            assert_eq!(
                spelling(dir.path(), &change(path, None, Some("x"))),
                None,
                "{path}"
            );
        }
        Ok(())
    }
}
