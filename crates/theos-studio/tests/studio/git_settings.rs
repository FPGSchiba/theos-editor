//! The whole studio write flow under hostile user settings: stage, commit, pull and push give
//! the same results as without them (spec §5.12).

use std::path::PathBuf;

use tempfile::TempDir;
use theos_studio::changes::changes;
use theos_studio::git_write::{Pulled, commit, pull, push, stage_records};
use theos_studio::{Changes, Workspace};
use theos_vcs::Cancel;
use theos_vcs::commit::Trailer;
use theos_vcs::push::Pushed;

use crate::common::{Error, TestResult, put};
use crate::repo::Repo;

const T900: &str = "lore/truths/T-900.md";
const T901: &str = "lore/truths/T-901.md";
const T902: &str = "lore/truths/T-902.md";

/// The settings of a user whose git is set up against a tool like this one.
const HOSTILE: [(&str, &str); 10] = [
    ("pull.rebase", "true"),
    ("pull.ff", "only"),
    ("merge.ff", "false"),
    ("merge.autoStash", "true"),
    ("push.default", "matching"),
    ("commit.verbose", "true"),
    ("core.commentChar", ";"),
    ("color.ui", "always"),
    ("core.quotePath", "true"),
    ("gc.auto", "1"),
];

/// What a run of the flow came to, to compare with another run.
#[derive(Debug, PartialEq, Eq)]
struct Results {
    steps: Vec<&'static str>,
    message: String,
    stash: String,
}

fn set_up(hostile: bool) -> Result<(Repo, Repo, PathBuf, TempDir), Error> {
    let seed = Repo::committed("")?;
    let dir = tempfile::tempdir()?;
    let bare: PathBuf = dir.path().join("origin.git");
    seed.run(&["clone", "-q", "--bare", ".", bare.to_str().ok_or("path")?])?;
    let a = Repo::clone_of(&bare)?;
    let b = Repo::clone_of(&bare)?;
    if hostile {
        for repo in [&a, &b] {
            for (key, value) in HOSTILE {
                repo.run(&["config", key, value])?;
            }
        }
    }
    Ok((a, b, bare, dir))
}

fn read(repo: &Repo, path: &str) -> Result<String, Error> {
    Ok(std::fs::read_to_string(repo.top().join(path))?)
}

/// `b` commits `text` to `path` and pushes it.
fn push_from(b: &Repo, path: &str, text: &str) -> TestResult {
    put(b.top(), path, text.as_bytes())?;
    b.commit("upstream change")?;
    b.run(&["push", "-q", "origin", "main"])?;
    Ok(())
}

fn all(found: &Changes) -> Vec<&theos_studio::RecordChange> {
    found.records.iter().collect()
}

/// Stages by record, commits, pulls a fast-forward and a diverged upstream, and pushes.
fn flow(hostile: bool) -> Result<Results, Error> {
    let (a, b, _bare, _dir) = set_up(hostile)?;
    let git = a.git()?;
    let cancel = Cancel::new();
    let mut steps = Vec::new();

    // A fast-forward: `b` adds a record upstream.
    let truth = read(&b, T901)?
        .replace("id: T-901", "id: T-902")
        .replace("Synthetic truth B", "Synthetic truth C");
    push_from(&b, T902, &truth)?;
    let mut workspace = Workspace::open(a.top())?;
    match pull(&mut workspace, &git, &cancel)? {
        Pulled::FastForward { .. } => steps.push("FastForward"),
        other => return Err(format!("expected FastForward, got {other:?}").into()),
    }

    // Stage by record and commit.
    let text = format!("{}\nEdited.\n", read(&a, T900)?);
    put(a.top(), T900, text.as_bytes())?;
    let found = changes(&workspace, &git, &cancel)?;
    stage_records(&mut workspace, &git, &all(&found), &[])?;
    let trailer = Trailer {
        key: "Phase".to_owned(),
        value: "P47".to_owned(),
    };
    commit(&mut workspace, &git, "Edit T-900", &[trailer])?;
    steps.push("Committed");
    let message = a.run(&["log", "-1", "--format=%B"])?;

    // A diverged upstream.
    let remote_text = format!("{}\nRemote.\n", read(&b, T901)?);
    push_from(&b, T901, &remote_text)?;
    match pull(&mut workspace, &git, &cancel)? {
        Pulled::Merged { .. } => steps.push("Merged"),
        other => return Err(format!("expected Merged, got {other:?}").into()),
    }

    match push(&mut workspace, &git, &cancel)? {
        Pushed::Updated { .. } => steps.push("Updated"),
        other => return Err(format!("expected Updated, got {other:?}").into()),
    }
    let stash = a.run(&["stash", "list"])?;
    Ok(Results {
        steps,
        message,
        stash,
    })
}

#[test]
fn hostile_user_settings_change_no_result() -> TestResult {
    let plain = flow(false)?;
    assert_eq!(
        plain.steps,
        ["FastForward", "Committed", "Merged", "Updated"]
    );
    let hostile = flow(true)?;
    assert_eq!(hostile, plain);
    assert!(hostile.stash.is_empty(), "{}", hostile.stash);
    Ok(())
}
