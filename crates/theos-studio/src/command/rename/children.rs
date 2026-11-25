//! The `children` participant: a renamed record's child folder `<folder>/<id>/` moves to
//! `<folder>/<to>/` with every file in it, records or not, hidden and ignored files too. The
//! folder is read with `std::fs::read_dir`, not the index's walk, which skips such files. A
//! `.gitignore` among them makes the pipeline refuse the rename (ruling R21).

use std::collections::BTreeMap;
use std::path::Path;

use super::participant::{Participant, RenameSet, folder_of, join, on_disk, os_path, record};
use crate::command::{CommandError, PlanContext};

/// Moves every file under the child folder of each renamed record.
pub(crate) struct Children;

impl Participant for Children {
    fn extend(&self, ctx: &PlanContext<'_>, set: &mut RenameSet) -> Result<(), CommandError> {
        // Old folder → its new name; a folder inside another renamed one is renamed in turn.
        let mut folders = BTreeMap::new();
        for (old, new) in &set.ids {
            let folder = join(folder_of(&record(ctx, old)?.path), old);
            // A symbolic link is not followed: it is no child folder.
            if on_disk(ctx.root(), &folder)?.is_some_and(|found| found.is_dir()) {
                folders.insert(folder, new.clone());
            }
        }
        for folder in folders.keys() {
            let mut files = Vec::new();
            files_under(&os_path(ctx.root(), folder), folder, &mut files)?;
            for file in files {
                let moved = relocate(&file, &folders);
                set.moves.insert(file, moved);
            }
        }
        Ok(())
    }
}

/// Adds the project path of every file under the folder `dir`, whose project path is `folder`, to
/// `found`: in every subfolder, dot files included. A name that is not UTF-8 or that holds a `\`
/// (possible on Unix) fails with [`CommandError::Path`].
fn files_under(dir: &Path, folder: &str, found: &mut Vec<String>) -> Result<(), CommandError> {
    let io = |source| CommandError::read_io(folder.to_owned(), source);
    for entry in std::fs::read_dir(dir).map_err(io)? {
        let entry = entry.map_err(io)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(CommandError::Path {
                path: join(folder, &name.to_string_lossy()),
                reason: "is not valid UTF-8, so a rename cannot move it",
            });
        };
        // A project path reads `\` as `/`: the file would be read as another (ruling R22).
        if name.contains('\\') {
            return Err(CommandError::Path {
                path: join(folder, name),
                reason: "has a backslash in its name, so a rename cannot move it",
            });
        }
        let path = join(folder, name);
        if entry.file_type().map_err(io)?.is_dir() {
            files_under(&entry.path(), &path, found)?;
        } else {
            found.push(path);
        }
    }
    Ok(())
}

/// `path` with each folder of it that is a key of `folders` given its new name.
fn relocate(path: &str, folders: &BTreeMap<String, String>) -> String {
    let mut old = String::new();
    let mut new = String::new();
    for part in path.split('/') {
        if !old.is_empty() {
            old.push('/');
            new.push('/');
        }
        old.push_str(part);
        new.push_str(folders.get(&old).map_or(part, String::as_str));
    }
    new
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestResult;

    #[test]
    fn relocate_renames_every_renamed_folder_of_a_path() {
        let folders: BTreeMap<String, String> = [
            ("concepts/shrine", "temple"),
            ("concepts/shrine/shrine-v1", "temple-v1"),
        ]
        .into_iter()
        .map(|(old, new)| (old.to_owned(), new.to_owned()))
        .collect();
        assert_eq!(
            relocate("concepts/shrine/shrine-v1/a.txt", &folders),
            "concepts/temple/temple-v1/a.txt"
        );
        assert_eq!(
            relocate("concepts/shrine/x/a.md", &folders),
            "concepts/temple/x/a.md"
        );
        assert_eq!(
            relocate("concepts/shrine.md", &folders),
            "concepts/shrine.md"
        );
    }

    #[test]
    fn files_under_lists_every_file_with_dot_files_and_subfolders() -> TestResult {
        let dir = tempfile::tempdir()?;
        let folder = dir.path().join("world").join("f04");
        std::fs::create_dir_all(folder.join("hall").join("empty"))?;
        std::fs::write(folder.join(".hidden"), "x")?;
        std::fs::write(folder.join("notes.txt"), "x")?;
        std::fs::write(folder.join("hall").join("nook.md"), "x")?;
        let mut found = Vec::new();
        files_under(&folder, "world/f04", &mut found)?;
        found.sort();
        assert_eq!(
            found,
            [
                "world/f04/.hidden",
                "world/f04/hall/nook.md",
                "world/f04/notes.txt"
            ]
        );
        Ok(())
    }
}
