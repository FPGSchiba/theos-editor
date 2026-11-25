//! The participants of a rename (spec §5.8 step 6): each one extends what the rename covers.
//! A new participant is one module (its `mod` line in the parent) and one line in
//! [`PARTICIPANTS`], which names it by its path.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::Metadata;
use std::path::{Path, PathBuf};

use theos_content::RecordSummary;
use theos_content::rename::IdMap;

use crate::command::{CommandError, PlanContext};

/// One part of a rename.
pub(crate) trait Participant {
    /// Extends the rename: more ids, more moved files, more records to edit.
    fn extend(&self, ctx: &PlanContext<'_>, set: &mut RenameSet) -> Result<(), CommandError>;
}

/// What a rename covers, built up by the participants in order.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RenameSet {
    /// Old id → new id.
    pub ids: IdMap,
    /// Old file path → new file path, project-relative with `/`.
    pub moves: BTreeMap<String, String>,
    /// The old paths of the records to edit.
    pub sources: BTreeSet<String>,
}

/// The participants, in the order they run: each one sees what the ones before it added.
pub(crate) const PARTICIPANTS: &[&dyn Participant] = &[
    &super::versions::Versions,
    &super::children::Children,
    &super::ids::Ids,
    &super::links::Links,
];

/// The one record with the id `id`.
///
/// # Errors
///
/// [`CommandError::NotFound`] when no record has it, [`CommandError::Ambiguous`] when several do.
pub(crate) fn record(ctx: &PlanContext<'_>, id: &str) -> Result<RecordSummary, CommandError> {
    let mut found = ctx.index().record(id)?;
    match found.len() {
        0 => Err(CommandError::NotFound {
            what: id.to_owned(),
        }),
        1 => Ok(found.remove(0)),
        _ => Err(CommandError::Ambiguous {
            id: id.to_owned(),
            paths: found.into_iter().map(|record| record.path).collect(),
        }),
    }
}

/// The folder of the project path `path`; empty at the project root.
pub(crate) fn folder_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(folder, _)| folder)
}

/// The project path of `name` in `folder`.
pub(crate) fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_owned()
    } else {
        format!("{folder}/{name}")
    }
}

/// The file or folder of the project path `path` under `root`, joined part by part with the OS's
/// separator.
pub(crate) fn os_path(root: &Path, path: &str) -> PathBuf {
    path.split('/')
        .fold(root.to_path_buf(), |os, part| os.join(part))
}

/// What is at the project path `path` under `root`, `None` when nothing is: decided by the disk
/// on its own terms, as for `Create`, so hidden and ignored files count, a symbolic link is not
/// followed, and a case-insensitive disk also finds a name in another case.
pub(crate) fn on_disk(root: &Path, path: &str) -> Result<Option<Metadata>, CommandError> {
    match std::fs::symlink_metadata(os_path(root, path)) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(CommandError::read_io(path.to_owned(), source)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestResult;

    #[test]
    fn folder_and_join_work_at_the_root_and_below() {
        assert_eq!(folder_of("world/tower.md"), "world");
        assert_eq!(folder_of("tower.md"), "");
        assert_eq!(join("world", "tower.md"), "world/tower.md");
        assert_eq!(join("", "tower.md"), "tower.md");
    }

    #[test]
    fn os_path_joins_each_part() {
        let root = Path::new("root");
        assert_eq!(
            os_path(root, "world/tower.md"),
            root.join("world").join("tower.md")
        );
    }

    #[test]
    fn on_disk_finds_files_and_folders() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir_all(dir.path().join("world"))?;
        std::fs::write(dir.path().join("world").join(".hidden"), "x")?;
        assert!(on_disk(dir.path(), "world")?.is_some_and(|m| m.is_dir()));
        assert!(on_disk(dir.path(), "world/.hidden")?.is_some_and(|m| m.is_file()));
        assert!(on_disk(dir.path(), "world/gone.md")?.is_none());
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn on_disk_finds_another_case_of_the_name_on_windows() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir_all(dir.path().join("lore"))?;
        std::fs::write(dir.path().join("lore").join("T-975.md"), "x")?;
        assert!(on_disk(dir.path(), "LORE/t-975.MD")?.is_some());
        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn on_disk_tells_cases_apart_on_linux() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir_all(dir.path().join("lore"))?;
        std::fs::write(dir.path().join("lore").join("T-975.md"), "x")?;
        assert!(on_disk(dir.path(), "lore/t-975.md")?.is_none());
        Ok(())
    }
}
