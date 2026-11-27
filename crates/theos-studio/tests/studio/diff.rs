//! `diff`: a record's or a file's diff between two versions, through real git, and the word diff
//! against `git diff --word-diff` on fixture pairs.

use std::path::{Path, PathBuf};
use std::process::Command;

use theos_content::Span;
use theos_content::diff::{FieldDelta, FieldsDiff, RecordDiff, WordDiff, WordOp, words};
use theos_studio::Workspace;
use theos_studio::diff::{FileDiff, Side, diff};
use theos_vcs::Cancel;
use theos_vcs::history::{Commit, Page, history};
use theos_vcs::read::Version;

use crate::common::{Error, TestResult, put};
use crate::repo::Repo;

const T900: &str = "lore/truths/T-900.md";
const T901: &str = "lore/truths/T-901.md";
const T905: &str = "lore/truths/T-905.md";

/// The diff of `path` (and `from`) in the project at the top of `repo`.
fn diff_of(repo: &Repo, path: &str, from: Option<&str>, side: Side<'_>) -> Result<FileDiff, Error> {
    let workspace = Workspace::open(repo.top())?;
    Ok(diff(
        &workspace,
        &repo.git()?,
        path.as_bytes(),
        from.map(str::as_bytes),
        side,
    )?)
}

/// The text of `path` below the top of `repo` with each `(from, to)` replaced, written back.
fn edit(repo: &Repo, path: &str, replacements: &[(&str, &str)]) -> TestResult {
    let mut text = std::fs::read_to_string(repo.top().join(path))?;
    for (from, to) in replacements {
        assert!(text.contains(from), "{path} has no {from:?}");
        text = text.replace(from, to);
    }
    put(repo.top(), path, text.as_bytes())?;
    Ok(())
}

/// The text of each `Delete` and each `Insert` run of `diff`, in order.
fn runs(old: &str, new: &str, diff: &WordDiff) -> Result<(Vec<String>, Vec<String>), Error> {
    let text = |source: &str, span: Span| -> Result<String, Error> {
        Ok(span.get(source).ok_or("span outside the text")?.to_owned())
    };
    let (mut deleted, mut inserted) = (Vec::new(), Vec::new());
    for op in &diff.ops {
        match op {
            WordOp::Delete { old: span } => deleted.push(text(old, *span)?),
            WordOp::Insert { new: span } => inserted.push(text(new, *span)?),
            WordOp::Equal { .. } => {}
        }
    }
    Ok((deleted, inserted))
}

/// What a record diff shows: each field's key and change, then the body's deleted and inserted
/// runs.
type Shown = (Vec<(String, &'static str)>, Vec<String>, Vec<String>);

/// The fields and body runs of `found`, which must be a record diff that is not layout only.
fn shown(found: &FileDiff) -> Result<Shown, Error> {
    let FileDiff::Record(RecordDiff::Record {
        old,
        new,
        fields: FieldsDiff::Fields(fields),
        body,
        layout_only: false,
    }) = found
    else {
        return Err(format!("not a record diff with fields: {found:?}").into());
    };
    let fields = fields
        .iter()
        .map(|change| {
            let delta = match change.delta {
                FieldDelta::Added { .. } => "added",
                FieldDelta::Removed { .. } => "removed",
                FieldDelta::Changed { .. } => "changed",
            };
            (change.key.clone(), delta)
        })
        .collect();
    let (deleted, inserted) = runs(old, new, body)?;
    Ok((fields, deleted, inserted))
}

/// `(key, "changed")` for each of `keys`.
fn changed(keys: &[&str]) -> Vec<(String, &'static str)> {
    keys.iter()
        .map(|key| ((*key).to_owned(), "changed"))
        .collect()
}

/// The commits of `path` in `repo`, newest first.
fn commits(repo: &Repo, path: &str) -> Result<Vec<Commit>, Error> {
    let page = Page { skip: 0, max: 10 };
    Ok(history(
        &repo.git()?,
        repo.top(),
        path.as_bytes(),
        page,
        &Cancel::new(),
    )?)
}

/// A staged edit of T-900: a new title and one changed body word.
fn stage_title_and_word(repo: &Repo) -> TestResult {
    edit(
        repo,
        T900,
        &[
            ("title: Synthetic truth A", "title: Synthetic truth Z"),
            ("statement A.", "statement B."),
        ],
    )?;
    repo.run(&["add", "--", T900])?;
    Ok(())
}

// AC: CE-21.2
#[test]
fn a_staged_change_shows_the_field_and_the_word() -> TestResult {
    let repo = Repo::committed("")?;
    stage_title_and_word(&repo)?;
    let found = diff_of(&repo, T900, None, Side::Staged)?;
    assert_eq!(
        shown(&found)?,
        (
            changed(&["title"]),
            vec!["A.".to_owned()],
            vec!["B.".to_owned()]
        )
    );
    let FileDiff::Record(RecordDiff::Record {
        old,
        new,
        fields: FieldsDiff::Fields(fields),
        ..
    }) = &found
    else {
        return Err("not a record diff".into());
    };
    let Some(FieldDelta::Changed {
        old: before,
        new: after,
        ..
    }) = fields.first().map(|change| &change.delta)
    else {
        return Err("no changed field".into());
    };
    assert_eq!(
        before.lines.get(old.as_str()),
        Some("title: Synthetic truth A\n")
    );
    assert_eq!(
        after.lines.get(new.as_str()),
        Some("title: Synthetic truth Z\n")
    );
    Ok(())
}

// AC: CE-21.2
#[test]
fn an_unstaged_change_shows_only_the_edit_since_the_index() -> TestResult {
    let repo = Repo::committed("")?;
    stage_title_and_word(&repo)?;
    edit(&repo, T900, &[("statement B.", "statement C.")])?;
    let found = diff_of(&repo, T900, None, Side::Unstaged)?;
    assert_eq!(
        shown(&found)?,
        (Vec::new(), vec!["B.".to_owned()], vec!["C.".to_owned()])
    );
    Ok(())
}

// AC: CE-21.2
#[test]
fn a_commit_shows_its_change_against_its_parent() -> TestResult {
    let repo = Repo::committed("")?;
    stage_title_and_word(&repo)?;
    repo.commit("synthetic edit")?;
    let commits = commits(&repo, T900)?;
    let [edit, root] = commits.as_slice() else {
        return Err(format!("two commits expected: {commits:?}").into());
    };
    let found = diff_of(&repo, T900, None, Side::Commit(edit))?;
    assert_eq!(
        shown(&found)?,
        (
            changed(&["title"]),
            vec!["A.".to_owned()],
            vec!["B.".to_owned()]
        )
    );
    // The first commit has no parent: every field is added, every word inserted.
    let found = diff_of(&repo, T900, None, Side::Commit(root))?;
    let added = ["id", "kind", "title", "status", "target"]
        .map(|key| (key.to_owned(), "added"))
        .to_vec();
    assert_eq!(
        shown(&found)?,
        (added, Vec::new(), vec!["Synthetic statement A.".to_owned()])
    );
    Ok(())
}

// AC: CE-21.2
#[test]
fn a_rename_commit_reads_its_parent_at_the_old_path() -> TestResult {
    let repo = Repo::committed("")?;
    repo.run(&["mv", "--", T901, T905])?;
    edit(&repo, T905, &[("id: T-901", "id: T-905")])?;
    repo.commit("synthetic rename")?;
    let commits = commits(&repo, T905)?;
    let renamed = commits.first().ok_or("no commit")?;
    assert_eq!(renamed.from.as_deref(), Some(T901.as_bytes()));
    let from = Some(T901);
    let found = diff_of(&repo, T905, from, Side::Commit(renamed))?;
    assert_eq!(shown(&found)?, (changed(&["id"]), Vec::new(), Vec::new()));
    Ok(())
}

// AC: CE-21.2
#[test]
fn a_staged_rename_with_an_unstaged_edit_diffs_the_edit_since_the_index() -> TestResult {
    let repo = Repo::committed("")?;
    repo.run(&["mv", "--", T901, T905])?;
    edit(&repo, T905, &[("statement B.", "statement D.")])?;
    // What `changes` gives for this record: the new path and the one before the rename.
    let found = diff_of(&repo, T905, Some(T901), Side::Unstaged)?;
    assert_eq!(
        shown(&found)?,
        (Vec::new(), vec!["B.".to_owned()], vec!["D.".to_owned()])
    );
    let found = diff_of(&repo, T905, Some(T901), Side::Staged)?;
    assert_eq!(shown(&found)?, (Vec::new(), Vec::new(), Vec::new()));
    Ok(())
}

// AC: CE-21.2
#[test]
fn between_two_branches_shows_the_branch_change() -> TestResult {
    let repo = Repo::committed("")?;
    repo.run(&["checkout", "-q", "-b", "agent/s-04"])?;
    edit(
        &repo,
        T900,
        &[("title: Synthetic truth A", "title: Synthetic truth Q")],
    )?;
    repo.commit("synthetic agent edit")?;
    repo.run(&["checkout", "-q", "main"])?;
    let side = Side::Between(Version::Rev("main"), Version::Rev("agent/s-04"));
    let found = diff_of(&repo, T900, None, side)?;
    assert_eq!(
        shown(&found)?,
        (changed(&["title"]), Vec::new(), Vec::new())
    );
    Ok(())
}

// AC: CE-21.2
#[test]
fn a_file_that_is_not_utf8_is_binary() -> TestResult {
    let repo = Repo::committed("")?;
    put(repo.top(), "lore/notes.bin", b"\x89PNG\xff\x00synthetic")?;
    let found = diff_of(&repo, "lore/notes.bin", None, Side::Unstaged)?;
    assert_eq!(found, FileDiff::Binary);
    Ok(())
}

// AC: CE-21.2
#[test]
fn a_settings_file_is_text_word_by_word() -> TestResult {
    let repo = Repo::committed("")?;
    edit(
        &repo,
        "theos.toml",
        &[("[project]", "# synthetic\n[project]")],
    )?;
    let found = diff_of(&repo, "theos.toml", None, Side::Unstaged)?;
    let FileDiff::Text { old, new, words } = &found else {
        return Err(format!("not a text diff: {found:?}").into());
    };
    assert_eq!(
        runs(old, new, words)?,
        (Vec::new(), vec!["# synthetic".to_owned()])
    );
    Ok(())
}

/// The folder of the word-diff fixture pairs.
fn word_diff_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("word-diff")
}

/// Splits `text` on git's `[:space:]`.
fn git_words(text: &str) -> impl Iterator<Item = &str> {
    text.split([' ', '\t', '\n', '\x0b', '\x0c', '\r'])
        .filter(|word| !word.is_empty())
}

/// The sorted words of `runs`.
fn sorted_words(runs: &[String]) -> Vec<String> {
    let mut found: Vec<String> = runs
        .iter()
        .flat_map(|run| git_words(run).map(str::to_owned))
        .collect();
    found.sort();
    found
}

/// The deleted and inserted words of `git diff --no-index --word-diff=porcelain` from `old` to
/// `new`, sorted, with git isolated from the user's and the system's configuration.
fn git_word_diff(old: &Path, new: &Path) -> Result<(Vec<String>, Vec<String>), Error> {
    let dir = tempfile::tempdir()?;
    let config = dir.path().join("gitconfig");
    std::fs::write(&config, "")?;
    let output = Command::new("git")
        .args([
            "diff",
            "--no-index",
            "--word-diff=porcelain",
            "--word-diff-regex=[^[:space:]]+",
            "--",
        ])
        .arg(old)
        .arg(new)
        .current_dir(dir.path())
        .env("GIT_CONFIG_GLOBAL", &config)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()?;
    // Exit code 1 means the files differ.
    if !matches!(output.status.code(), Some(0 | 1)) {
        return Err(format!(
            "git diff failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let text = String::from_utf8(output.stdout)?;
    let (mut deleted, mut inserted) = (Vec::new(), Vec::new());
    // The header ends at the first hunk; after it, `-` and `+` start the changed runs.
    for line in text.lines().skip_while(|line| !line.starts_with("@@")) {
        if let Some(run) = line.strip_prefix('-') {
            deleted.push(run.to_owned());
        } else if let Some(run) = line.strip_prefix('+') {
            inserted.push(run.to_owned());
        }
    }
    Ok((sorted_words(&deleted), sorted_words(&inserted)))
}

// AC: CE-21.2
#[test]
fn the_word_diff_deletes_and_inserts_the_words_git_does() -> TestResult {
    let folder = word_diff_fixtures();
    let mut pairs = 0;
    for entry in std::fs::read_dir(&folder)? {
        let old_path = entry?.path();
        let Some(name) = old_path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".old.md"))
        else {
            continue;
        };
        let new_path = folder.join(format!("{name}.new.md"));
        let old = std::fs::read_to_string(&old_path)?;
        let new = std::fs::read_to_string(&new_path)?;
        let (deleted, inserted) = runs(&old, &new, &words(&old, &new))?;
        let ours = (sorted_words(&deleted), sorted_words(&inserted));
        assert_eq!(ours, git_word_diff(&old_path, &new_path)?, "{name}");
        pairs += 1;
    }
    assert!(pairs >= 5, "only {pairs} fixture pairs");
    Ok(())
}
