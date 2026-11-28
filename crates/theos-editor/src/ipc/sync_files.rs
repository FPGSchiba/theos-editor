//! What a sync did to the record files, file by file (spec §5.9).
//!
//! Two files may carry one id (a duplicate the index reports), so a sync names files by their
//! path, never records by their id alone: the UI keys its tabs by path and acts on these paths
//! only. The lists come from the index's files before and after the sync:
//!
//! - **removed:** a file that was there before and is gone, also when another file keeps its id;
//! - **moved:** a file that is gone while a new file with the same id appeared (moved or renamed
//!   folder, same file name). When one id loses and gains several files in one sync, they pair in
//!   path order; the files left over are removed or new;
//! - **changed:** a file that is there after the sync and was read again, relinked or checked
//!   again, which includes the new path of each move.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

/// One record file: its id and its project-relative path with `/`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub struct RecordFile {
    /// The record's id.
    pub id: String,
    /// The file, project-relative with `/`.
    pub path: String,
}

/// A record file that moved: it keeps its id and has a new path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub struct FileMove {
    /// The record's id.
    pub id: String,
    /// The path before the sync.
    pub from: String,
    /// The path after the sync.
    pub to: String,
}

/// The record files a sync changed, removed and moved, each list sorted by path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileChanges {
    /// The files read again, relinked or checked again that are there after the sync.
    pub changed: Vec<RecordFile>,
    /// The files that are gone and did not move.
    pub removed: Vec<RecordFile>,
    /// The files that moved, sorted by id, then old path.
    pub moved: Vec<FileMove>,
}

/// The index's record files: id by path.
pub type Files = BTreeMap<String, String>;

/// The changes between the files `before` and `after` a sync that touched the paths `touched`
/// (read again, relinked or checked again; paths that are not record files are left out).
pub fn changes<'a>(
    before: &Files,
    after: &Files,
    touched: impl Iterator<Item = &'a String>,
) -> FileChanges {
    let gone = by_id(before, after);
    let mut new = by_id(after, before);
    let mut removed = Vec::new();
    let mut moved = Vec::new();
    for (id, from_paths) in gone {
        let mut to_paths = new.remove(id).unwrap_or_default().into_iter();
        for from in from_paths {
            match to_paths.next() {
                Some(to) => moved.push(FileMove {
                    id: id.to_owned(),
                    from: from.to_owned(),
                    to: to.to_owned(),
                }),
                None => removed.push(file(id, from)),
            }
        }
    }
    let touched: BTreeSet<&String> = touched.collect();
    let changed = touched
        .into_iter()
        .filter_map(|path| after.get(path).map(|id| file(id, path)))
        .collect();
    removed.sort();
    moved.sort();
    FileChanges {
        changed,
        removed,
        moved,
    }
}

/// The paths of `files` that `other` does not hold, grouped by id, each group in path order.
fn by_id<'a>(files: &'a Files, other: &Files) -> BTreeMap<&'a str, Vec<&'a str>> {
    let mut groups: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (path, id) in files.iter().filter(|(path, _)| !other.contains_key(*path)) {
        groups.entry(id).or_default().push(path);
    }
    groups
}

fn file(id: &str, path: &str) -> RecordFile {
    RecordFile {
        id: id.to_owned(),
        path: path.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(entries: &[(&str, &str)]) -> Files {
        entries
            .iter()
            .map(|(path, id)| ((*path).to_owned(), (*id).to_owned()))
            .collect()
    }

    fn touched(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| (*path).to_owned()).collect()
    }

    #[test]
    fn a_deleted_twin_is_removed_by_its_path() {
        let before = files(&[("a/twin.md", "twin"), ("b/twin.md", "twin")]);
        let after = files(&[("b/twin.md", "twin")]);
        let changes = changes(&before, &after, touched(&["a/twin.md"]).iter());
        assert_eq!(changes.removed, [file("twin", "a/twin.md")]);
        assert_eq!(changes.moved, Vec::<FileMove>::new());
        assert_eq!(
            changes.changed,
            Vec::<RecordFile>::new(),
            "a gone file is never changed"
        );
    }

    #[test]
    fn a_file_with_a_new_path_and_its_id_is_a_move() {
        let before = files(&[("a/x.md", "x"), ("y.md", "y")]);
        let after = files(&[("b/x.md", "x"), ("y.md", "y")]);
        let touched = touched(&["b/x.md", "y.md", "theos.toml"]);
        let changes = changes(&before, &after, touched.iter());
        assert_eq!(changes.removed, Vec::<RecordFile>::new());
        assert_eq!(
            changes.moved,
            [FileMove {
                id: "x".to_owned(),
                from: "a/x.md".to_owned(),
                to: "b/x.md".to_owned(),
            }]
        );
        assert_eq!(changes.changed, [file("x", "b/x.md"), file("y", "y.md")]);
    }

    #[test]
    fn moves_of_one_id_pair_in_path_order_and_the_rest_is_removed() {
        let before = files(&[("a/t.md", "t"), ("b/t.md", "t"), ("c/t.md", "t")]);
        let after = files(&[("d/t.md", "t")]);
        let changes = changes(&before, &after, std::iter::empty());
        assert_eq!(
            changes.moved,
            [FileMove {
                id: "t".to_owned(),
                from: "a/t.md".to_owned(),
                to: "d/t.md".to_owned(),
            }]
        );
        assert_eq!(changes.removed, [file("t", "b/t.md"), file("t", "c/t.md")]);
    }
}
