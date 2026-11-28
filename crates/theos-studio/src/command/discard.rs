//! The command that discards a record's unstaged changes (P47 spec §5.4): the working tree goes
//! back to the index, and what is staged stays staged. Git is read when the command is built;
//! the plan writes exactly those bytes, so the journal can undo the discard.

use std::path::Path;

use theos_vcs::read::{Content, checkout_bytes};
use theos_vcs::status::{Change, Entry};
use theos_vcs::{Folding, Git, repo};

use super::{
    Command, CommandError, FileChange, Plan, PlanContext, Reads, boundary, project_path, read_file,
};
use crate::StudioError;
use crate::changes::RecordChange;
use crate::repo_path;
use crate::top::repo_top;
use crate::workspace::Workspace;

/// How the message of a refused discard ends: the way that remains.
const IN_A_TERMINAL: &str = "discard it with `git restore` in a terminal";

/// Discards the unstaged changes of one record or one changed file. Its verdict never refuses
/// ([`Command::restores`]): the user asked for the index's version back, errors and all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discard {
    /// `discard <path>`.
    label: String,
    /// What the discard writes, with the working bytes read when it was built.
    changes: Vec<FileChange>,
}

/// One repository path a discard brings back to the index.
#[derive(Debug, PartialEq, Eq)]
struct Target<'a> {
    /// git's bytes, relative to the top.
    repo_path: &'a [u8],
    /// Whether the index has a version to restore; `false` for an untracked file, which is
    /// removed.
    tracked: bool,
}

impl Discard {
    /// The discard of `record`, an entry of [`Changes::records`](crate::Changes).
    ///
    /// # Errors
    ///
    /// [`StudioError::Conflicted`] for a record in a merge conflict;
    /// [`StudioError::NotDiscardable`] for a path outside the project folder, a path the command
    /// layer may not write (a `.gitignore`, a hidden file on Windows), or a link or submodule in
    /// the index; [`StudioError::NotInRepository`] and [`StudioError::Git`] when git fails;
    /// [`StudioError::Command`] when a working file cannot be read.
    pub fn record(
        workspace: &Workspace,
        git: &Git,
        record: &RecordChange,
    ) -> Result<Discard, StudioError> {
        if record.conflict.is_some() {
            return Err(StudioError::Conflicted {
                id: record.id.clone(),
            });
        }
        let targets = targets(
            &record.repo_path,
            record.repo_from.as_deref(),
            record.unstaged,
            record.untracked,
        );
        build(workspace, git, &record.path, &targets)
    }

    /// The discard of `entry`, an entry of [`Changes::files`](crate::Changes).
    ///
    /// # Errors
    ///
    /// As [`Discard::record`], and [`StudioError::UnsupportedEntry`] for an entry of a kind this
    /// studio does not know.
    pub fn file(workspace: &Workspace, git: &Git, entry: &Entry) -> Result<Discard, StudioError> {
        let lossy = |path: &[u8]| String::from_utf8_lossy(path).into_owned();
        let (name, targets) = match entry {
            Entry::Changed { path, unstaged, .. } => {
                (lossy(path), targets(path, None, *unstaged, false))
            }
            Entry::Renamed {
                from, to, unstaged, ..
            } => (lossy(to), targets(to, Some(from), *unstaged, false)),
            Entry::Untracked { path } => {
                (lossy(path), targets(path, None, Change::Unchanged, true))
            }
            Entry::Conflicted { path, .. } => {
                return Err(StudioError::Conflicted { id: lossy(path) });
            }
            other => {
                return Err(StudioError::UnsupportedEntry {
                    entry: format!("{other:?}"),
                });
            }
        };
        build(workspace, git, &name, &targets)
    }
}

impl Command for Discard {
    fn label(&self) -> String {
        self.label.clone()
    }

    fn reads(&self) -> Reads {
        Reads::Paths(self.changes.iter().map(|c| c.path.clone()).collect())
    }

    /// The changes read when the command was built, if every working file still holds the bytes
    /// it had then.
    fn plan(&self, ctx: &PlanContext<'_>) -> Result<Plan, CommandError> {
        for change in &self.changes {
            if ctx.read(&change.path)? != change.before {
                return Err(CommandError::Stale {
                    path: change.path.clone(),
                });
            }
        }
        Ok(Plan {
            changes: self.changes.clone(),
            ..Plan::default()
        })
    }

    fn restores(&self) -> bool {
        true
    }
}

/// The repository paths that discarding the `unstaged` change of `path` brings back to the index.
/// A paired unstaged rename also creates its old path `from` again; an untracked file is removed;
/// nothing unstaged is nothing to discard.
fn targets<'a>(
    path: &'a [u8],
    from: Option<&'a [u8]>,
    unstaged: Change,
    untracked: bool,
) -> Vec<Target<'a>> {
    if untracked {
        return vec![Target {
            repo_path: path,
            tracked: false,
        }];
    }
    let tracked = |repo_path| Target {
        repo_path,
        tracked: true,
    };
    match (unstaged, from) {
        (Change::Unchanged, _) => Vec::new(),
        (Change::Renamed, Some(from)) => vec![tracked(from), tracked(path)],
        _ => vec![tracked(path)],
    }
}

/// The discard named `name` of `targets`, read from git and the disk now.
fn build(
    workspace: &Workspace,
    git: &Git,
    name: &str,
    targets: &[Target<'_>],
) -> Result<Discard, StudioError> {
    let label = format!("discard {name}");
    if targets.is_empty() {
        return Ok(Discard {
            label,
            changes: Vec::new(),
        });
    }
    let root = workspace.project().root();
    let top = repo_top(git, root)?;
    let place = repo::place(&top, root)?;
    let folding = Folding::read(git, &top)?;
    let mut changes = Vec::new();
    for target in targets {
        let path = in_project(folding, &place, target.repo_path)?;
        let after = if target.tracked {
            index_bytes(git, &top, target.repo_path, &path)?
        } else {
            None
        };
        let before = read_file(root, &path)?;
        changes.push(FileChange {
            path,
            before,
            after,
        });
    }
    let changes = within_boundary(root, changes)?;
    Ok(Discard { label, changes })
}

/// The command path of `repo_path` in the project at `place`; refused when it lies outside the
/// project or breaks the rules of a command path.
fn in_project(folding: Folding, place: &str, repo_path: &[u8]) -> Result<String, StudioError> {
    let Some(path) = repo_path::project_path(folding, place, repo_path) else {
        return Err(not_discardable(
            &String::from_utf8_lossy(repo_path),
            "it is outside the project folder",
        ));
    };
    project_path(&path).map_err(refused)
}

/// The bytes a checkout of `repo_path` writes from the index; `None` when the index has none.
fn index_bytes(
    git: &Git,
    top: &Path,
    repo_path: &[u8],
    path: &str,
) -> Result<Option<Vec<u8>>, StudioError> {
    match checkout_bytes(git, top, repo_path)? {
        Content::File(bytes) => Ok(Some(bytes)),
        Content::Absent => Ok(None),
        // A link, a submodule, or a kind of entry this studio does not know.
        _ => Err(not_discardable(
            path,
            "the index holds a link or a submodule there",
        )),
    }
}

/// `changes` if P06's boundary rules let the command layer write them, so a refusal comes before
/// the pipeline. The pipeline checks them again on the plan.
fn within_boundary(root: &Path, changes: Vec<FileChange>) -> Result<Vec<FileChange>, StudioError> {
    let plan = Plan {
        changes,
        ..Plan::default()
    };
    match boundary::check(root, &plan) {
        Ok(()) => Ok(plan.changes),
        Err(error) => Err(refused(error)),
    }
}

/// [`StudioError::NotDiscardable`] for a path the command layer refuses with `error`
/// ([`CommandError::Path`]); any other error as it is.
fn refused(error: CommandError) -> StudioError {
    match error {
        CommandError::Path { path, reason } => not_discardable(&path, &format!("it {reason}")),
        other => other.into(),
    }
}

/// [`StudioError::NotDiscardable`] for `path`, `why` followed by the way that remains.
fn not_discardable(path: &str, why: &str) -> StudioError {
    StudioError::NotDiscardable {
        path: path.to_owned(),
        why: format!("{why}; {IN_A_TERMINAL}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &[u8] = b"lore/truths/T-904.md";
    const FROM: &[u8] = b"lore/truths/T-901.md";

    fn tracked(repo_path: &[u8]) -> Target<'_> {
        Target {
            repo_path,
            tracked: true,
        }
    }

    #[test]
    fn each_unstaged_change_brings_its_path_back_to_the_index() {
        for change in [
            Change::Modified,
            Change::Deleted,
            Change::TypeChanged,
            Change::Added,
        ] {
            assert_eq!(
                targets(PATH, None, change, false),
                [tracked(PATH)],
                "{change:?}"
            );
        }
    }

    #[test]
    fn a_paired_unstaged_rename_brings_both_paths_back() {
        assert_eq!(
            targets(PATH, Some(FROM), Change::Renamed, false),
            [tracked(FROM), tracked(PATH)]
        );
    }

    /// A staged rename with an unstaged edit keeps the rename: only the new path is discarded.
    #[test]
    fn a_staged_rename_discards_only_its_new_path() {
        assert_eq!(
            targets(PATH, Some(FROM), Change::Modified, false),
            [tracked(PATH)]
        );
    }

    #[test]
    fn nothing_unstaged_is_nothing_to_discard() {
        assert_eq!(targets(PATH, Some(FROM), Change::Unchanged, false), []);
    }

    #[test]
    fn an_untracked_file_is_removed() {
        assert_eq!(
            targets(PATH, None, Change::Unchanged, true),
            [Target {
                repo_path: PATH,
                tracked: false,
            }]
        );
    }

    #[test]
    fn a_refusal_names_git_restore() {
        let error = refused(CommandError::Path {
            path: "lore/.gitignore".to_owned(),
            reason: "is hidden",
        });
        assert_eq!(
            error.to_string(),
            "cannot discard `lore/.gitignore`: it is hidden; discard it with `git restore` in a terminal"
        );
    }
}
