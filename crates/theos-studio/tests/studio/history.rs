//! CE-21.1, `history`: a record's commits by id through real git, across renames of its file and
//! its id, made with P06's `Rename`, with `git mv` and by moving the file to another folder.

use theos_studio::history::{HistoryEntry, history};
use theos_studio::{Rename, Workspace};
use theos_vcs::Cancel;
use theos_vcs::history::Page;

use crate::common::{Error, TestResult, put};
use crate::repo::Repo;

const T900: &str = "lore/truths/T-900.md";
const C900: &str = "lore/clues/C-900.md";
const HALL: &str = "world/synthetic-low/f04-synthetic/synthetic-hall.md";

/// Every page at once.
const ALL: Page = Page {
    skip: 0,
    max: usize::MAX,
};

/// The history of `id` in the project at `place` below the top of `repo`, with a fresh
/// workspace.
fn history_of(repo: &Repo, place: &str, id: &str) -> Result<Vec<HistoryEntry>, Error> {
    let workspace = Workspace::open(&repo.top().join(place))?;
    Ok(history(&workspace, &repo.git()?, id, ALL, &Cancel::new())?)
}

/// Each entry as (id, project path, subject), newest first.
fn shown(entries: &[HistoryEntry]) -> Vec<(&str, &str, &str)> {
    entries
        .iter()
        .map(|entry| {
            (
                entry.id.as_str(),
                entry.path.as_str(),
                entry.commit.subject.as_str(),
            )
        })
        .collect()
}

/// The text of `path` below `root` with `from` replaced by `to`, written back.
fn edit(root: &std::path::Path, path: &str, from: &str, to: &str) -> TestResult {
    let text = std::fs::read_to_string(root.join(path))?;
    assert!(text.contains(from), "{path} has no {from:?}");
    put(root, path, text.replace(from, to).as_bytes())?;
    Ok(())
}

/// Renames `id` to `to` with P06's `Rename` in the project at the top of `repo`, then commits.
fn rename(repo: &Repo, id: &str, to: &str) -> TestResult {
    rename_in(repo, "", id, to)
}

/// Renames `id` to `to` with P06's `Rename` in the project at `place`, then commits.
fn rename_in(repo: &Repo, place: &str, id: &str, to: &str) -> TestResult {
    let mut workspace = Workspace::open(&repo.top().join(place))?;
    let outcome = workspace.run(&Rename {
        id: id.to_owned(),
        to: to.to_owned(),
    })?;
    assert_eq!(outcome.introduced, Vec::new(), "{id} → {to}");
    repo.commit(&format!("rename {id}"))?;
    Ok(())
}

// AC: CE-21.1
#[test]
fn a_record_renamed_with_the_rename_command_keeps_its_history() -> TestResult {
    let repo = Repo::committed("")?;
    edit(
        repo.top(),
        T900,
        "Synthetic statement A.",
        "Synthetic statement A, edited.",
    )?;
    repo.commit("edit T-900")?;
    rename(&repo, "T-900", "T-903")?;
    let entries = history_of(&repo, "", "T-903")?;
    assert_eq!(
        shown(&entries),
        [
            ("T-903", "lore/truths/T-903.md", "rename T-900"),
            ("T-900", T900, "edit T-900"),
            ("T-900", T900, "fixture"),
        ]
    );
    assert_eq!(entries[0].commit.path, b"lore/truths/T-903.md");
    assert_eq!(entries[0].commit.from.as_deref(), Some(T900.as_bytes()));
    Ok(())
}

// AC: CE-21.1
#[test]
fn a_short_clue_renamed_with_the_rename_command_keeps_its_history() -> TestResult {
    let repo = Repo::committed("")?;
    rename(&repo, "C-900", "C-903")?;
    assert_eq!(
        shown(&history_of(&repo, "", "C-903")?),
        [
            ("C-903", "lore/clues/C-903.md", "rename C-900"),
            ("C-900", C900, "fixture"),
        ]
    );
    Ok(())
}

// AC: CE-21.1
#[test]
fn the_smallest_record_renamed_with_the_rename_command_keeps_its_history() -> TestResult {
    // `world/tower.md` (106 bytes) is the smallest record of the fixture, and smaller than every
    // example of the data model (§14): the one changed `id:` line weighs most here.
    let repo = Repo::committed("")?;
    let size = std::fs::metadata(repo.top().join("world/tower.md"))?.len();
    assert!(size < 150, "{size}");
    rename(&repo, "tower", "synthetic-tower")?;
    assert_eq!(
        shown(&history_of(&repo, "", "synthetic-tower")?),
        [
            (
                "synthetic-tower",
                "world/synthetic-tower.md",
                "rename tower"
            ),
            ("tower", "world/tower.md", "fixture"),
        ]
    );
    Ok(())
}

// AC: CE-21.1
#[test]
fn a_record_renamed_with_git_mv_and_a_hand_edit_keeps_its_history() -> TestResult {
    let repo = Repo::committed("")?;
    repo.run(&["mv", T900, "lore/truths/T-904.md"])?;
    edit(repo.top(), "lore/truths/T-904.md", "id: T-900", "id: T-904")?;
    repo.commit("move T-900 by hand")?;
    assert_eq!(
        shown(&history_of(&repo, "", "T-904")?),
        [
            ("T-904", "lore/truths/T-904.md", "move T-900 by hand"),
            ("T-900", T900, "fixture"),
        ]
    );
    Ok(())
}

// AC: CE-21.1
#[test]
fn a_record_moved_to_another_floor_keeps_its_history() -> TestResult {
    let repo = Repo::committed("")?;
    put(
        repo.top(),
        "world/synthetic-low/f05-synthetic.md",
        b"---\n# synthetic fixture (P46), not canon\nid: f05-synthetic\nkind: floor\n\
          title: Synthetic floor five\nstatus: draft\nparent: synthetic-low\nnumber: 5\n---\n\
          Synthetic text.\n",
    )?;
    repo.commit("add floor five")?;
    let moved = "world/synthetic-low/f05-synthetic/synthetic-hall.md";
    repo.run(&[
        "mv",
        "world/synthetic-low/f04-synthetic",
        "world/synthetic-low/f05-synthetic",
    ])?;
    edit(
        repo.top(),
        moved,
        "parent: f04-synthetic",
        "parent: f05-synthetic",
    )?;
    repo.commit("move the hall to floor five")?;
    assert_eq!(
        shown(&history_of(&repo, "", "synthetic-hall")?),
        [
            ("synthetic-hall", moved, "move the hall to floor five"),
            ("synthetic-hall", HALL, "fixture"),
        ]
    );
    Ok(())
}

// AC: CE-21.1
#[test]
fn a_record_renamed_twice_keeps_its_whole_history() -> TestResult {
    let repo = Repo::committed("")?;
    rename(&repo, "T-900", "T-903")?;
    rename(&repo, "T-903", "T-904")?;
    assert_eq!(
        shown(&history_of(&repo, "", "T-904")?),
        [
            ("T-904", "lore/truths/T-904.md", "rename T-903"),
            ("T-903", "lore/truths/T-903.md", "rename T-900"),
            ("T-900", T900, "fixture"),
        ]
    );
    Ok(())
}

// AC: CE-21.1
#[test]
fn a_project_in_a_subfolder_shows_project_paths_and_keeps_paths_outside_it() -> TestResult {
    let repo = Repo::committed("content")?;
    let draft = b"---\n# synthetic fixture (P46), not canon\nid: T-907\nkind: truth\n\
                  title: Synthetic truth drafted outside\nstatus: draft\ntarget: confirmed\n---\n\
                  Synthetic statement drafted outside the project.\n";
    put(repo.top(), "drafts/T-907.md", draft)?;
    repo.commit("draft T-907 outside")?;
    repo.run(&["mv", "drafts/T-907.md", "content/lore/truths/T-907.md"])?;
    repo.commit("bring T-907 in")?;
    rename_in(&repo, "content", "T-900", "T-903")?;
    assert_eq!(
        shown(&history_of(&repo, "content", "T-907")?),
        [
            ("T-907", "lore/truths/T-907.md", "bring T-907 in"),
            ("T-907", "drafts/T-907.md", "draft T-907 outside"),
        ]
    );
    let renamed = history_of(&repo, "content", "T-903")?;
    assert_eq!(
        shown(&renamed),
        [
            ("T-903", "lore/truths/T-903.md", "rename T-900"),
            ("T-900", T900, "fixture"),
        ]
    );
    assert_eq!(renamed[1].commit.path, b"content/lore/truths/T-900.md");
    Ok(())
}

#[test]
fn a_record_only_in_the_working_tree_has_no_history() -> TestResult {
    let repo = Repo::committed("")?;
    put(
        repo.top(),
        "lore/truths/T-906.md",
        b"---\n# synthetic fixture (P46), not canon\nid: T-906\nkind: truth\n\
          title: Synthetic truth not committed\nstatus: draft\ntarget: confirmed\n---\n\
          Synthetic statement.\n",
    )?;
    assert_eq!(shown(&history_of(&repo, "", "T-906")?), []);
    // Staged with intent to add: still nothing committed.
    repo.run(&["add", "-N", "lore/truths/T-906.md"])?;
    assert_eq!(shown(&history_of(&repo, "", "T-906")?), []);
    // Staged: still nothing committed.
    repo.run(&["add", "lore/truths/T-906.md"])?;
    assert_eq!(shown(&history_of(&repo, "", "T-906")?), []);
    Ok(())
}

#[test]
fn an_unknown_id_has_no_history() -> TestResult {
    let repo = Repo::committed("")?;
    assert_eq!(shown(&history_of(&repo, "", "T-999")?), []);
    Ok(())
}
