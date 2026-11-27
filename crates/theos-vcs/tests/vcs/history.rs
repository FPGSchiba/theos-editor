//! A file's history: newest first, across renames of the file and its id, paged, with trailers
//! and merges.

use std::path::{Path, PathBuf};

use theos_vcs::history::{self, Commit, Page, Trailer};
use theos_vcs::status::Change;
use theos_vcs::{Cancel, GitError};

use crate::common::Sandbox;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const ALL: Page = Page { skip: 0, max: 100 };

/// A record of the data model's shape; synthetic: the id and the text are made up for the test.
fn record(id: &str, extra: &str) -> String {
    format!(
        "---\nid: {id}\nkind: character\ntitle: Synthetic Mother\nstatus: draft\n---\n\n\
         A synthetic record for the history tests.\nIt has a few lines of body text,\nso that \
         git finds a renamed copy similar.\nOne more line.\n{extra}"
    )
}

fn history_of(sandbox: &Sandbox, top: &Path, path: &str, page: Page) -> Result<Vec<Commit>> {
    Ok(history::history(
        &sandbox.git()?,
        top,
        path.as_bytes(),
        page,
        &Cancel::new(),
    )?)
}

/// Subject, change, path and path before of each commit.
fn summary(commits: &[Commit]) -> Vec<(String, Change, String, Option<String>)> {
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    commits
        .iter()
        .map(|commit| {
            (
                commit.subject.clone(),
                commit.change,
                text(&commit.path),
                commit.from.as_deref().map(text),
            )
        })
        .collect()
}

fn line(
    subject: &str,
    change: Change,
    path: &str,
    from: Option<&str>,
) -> (String, Change, String, Option<String>) {
    (
        subject.to_owned(),
        change,
        path.to_owned(),
        from.map(str::to_owned),
    )
}

/// Renames `from.md` to `to.md` as P06's `Rename` does: the file, its `id:` line and the link in
/// `links.md`.
fn rename(sandbox: &Sandbox, repo: &Path, from: &str, to: &str, extra: &str) -> Result {
    std::fs::remove_file(repo.join(format!("{from}.md")))?;
    Sandbox::write(&repo.join(format!("{to}.md")), &record(to, extra))?;
    Sandbox::write(&repo.join("links.md"), &format!("See [[{to}]].\n"))?;
    sandbox.commit(repo, &format!("rename {from} to {to}"))
}

/// A repository where `synthetic-ilse.md` was added, edited, then renamed to
/// `synthetic-ilsa.md` with its id rewritten and the link in `links.md` updated.
fn renamed_once(sandbox: &Sandbox) -> Result<PathBuf> {
    let repo = sandbox.init("repo")?;
    Sandbox::write(
        &repo.join("synthetic-ilse.md"),
        &record("synthetic-ilse", ""),
    )?;
    Sandbox::write(&repo.join("links.md"), "See [[synthetic-ilse]].\n")?;
    sandbox.commit(&repo, "add synthetic-ilse")?;
    let edited = "She keeps the ledger.\n";
    Sandbox::write(
        &repo.join("synthetic-ilse.md"),
        &record("synthetic-ilse", edited),
    )?;
    sandbox.commit(&repo, "edit synthetic-ilse")?;
    rename(sandbox, &repo, "synthetic-ilse", "synthetic-ilsa", edited)?;
    Ok(repo)
}

// AC: CE-21.1
#[test]
fn a_renamed_record_keeps_its_history_newest_first() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = renamed_once(&sandbox)?;
    let commits = history_of(&sandbox, &repo, "synthetic-ilsa.md", ALL)?;
    assert_eq!(
        summary(&commits),
        [
            line(
                "rename synthetic-ilse to synthetic-ilsa",
                Change::Renamed,
                "synthetic-ilsa.md",
                Some("synthetic-ilse.md"),
            ),
            line(
                "edit synthetic-ilse",
                Change::Modified,
                "synthetic-ilse.md",
                None
            ),
            line(
                "add synthetic-ilse",
                Change::Added,
                "synthetic-ilse.md",
                None
            ),
        ]
    );
    let head = sandbox.run(&repo, &["rev-parse", "HEAD"])?;
    assert_eq!(commits[0].id, head.trim());
    assert_eq!(commits[0].author.name, "Synthetic");
    assert_eq!(commits[0].author.email, "synthetic@example.invalid");
    assert!(commits[0].time > 0);
    Ok(())
}

// AC: CE-21.1
#[test]
fn a_record_renamed_twice_keeps_its_history() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = renamed_once(&sandbox)?;
    let edited = "She keeps the ledger.\n";
    rename(&sandbox, &repo, "synthetic-ilsa", "synthetic-ilka", edited)?;
    let commits = history_of(&sandbox, &repo, "synthetic-ilka.md", ALL)?;
    assert_eq!(
        summary(&commits),
        [
            line(
                "rename synthetic-ilsa to synthetic-ilka",
                Change::Renamed,
                "synthetic-ilka.md",
                Some("synthetic-ilsa.md"),
            ),
            line(
                "rename synthetic-ilse to synthetic-ilsa",
                Change::Renamed,
                "synthetic-ilsa.md",
                Some("synthetic-ilse.md"),
            ),
            line(
                "edit synthetic-ilse",
                Change::Modified,
                "synthetic-ilse.md",
                None
            ),
            line(
                "add synthetic-ilse",
                Change::Added,
                "synthetic-ilse.md",
                None
            ),
        ]
    );
    Ok(())
}

#[test]
fn a_page_gives_the_commits_after_the_skipped_ones() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = renamed_once(&sandbox)?;
    let middle = history_of(
        &sandbox,
        &repo,
        "synthetic-ilsa.md",
        Page { skip: 1, max: 1 },
    )?;
    assert_eq!(
        summary(&middle),
        [line(
            "edit synthetic-ilse",
            Change::Modified,
            "synthetic-ilse.md",
            None
        )]
    );
    let after_the_rename = history_of(
        &sandbox,
        &repo,
        "synthetic-ilsa.md",
        Page { skip: 1, max: 10 },
    )?;
    let subjects: Vec<&str> = after_the_rename
        .iter()
        .map(|c| c.subject.as_str())
        .collect();
    assert_eq!(subjects, ["edit synthetic-ilse", "add synthetic-ilse"]);
    let past_the_end = history_of(
        &sandbox,
        &repo,
        "synthetic-ilsa.md",
        Page { skip: 3, max: 10 },
    )?;
    assert_eq!(past_the_end, []);
    Ok(())
}

#[test]
fn a_path_never_committed_has_no_history() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = renamed_once(&sandbox)?;
    assert_eq!(history_of(&sandbox, &repo, "never.md", ALL)?, []);
    Ok(())
}

#[test]
fn a_repository_before_its_first_commit_has_no_history() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(
        &repo.join("synthetic-ilse.md"),
        &record("synthetic-ilse", ""),
    )?;
    sandbox.run(&repo, &["add", "-A"])?;
    assert_eq!(history_of(&sandbox, &repo, "synthetic-ilse.md", ALL)?, []);
    Ok(())
}

#[test]
fn trailers_come_back_in_order() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(
        &repo.join("synthetic-ilse.md"),
        &record("synthetic-ilse", ""),
    )?;
    sandbox.commit(
        &repo,
        "add synthetic-ilse\n\nApproved-by: Jann\nCo-Authored-By: Claude <noreply@example.invalid>",
    )?;
    let commits = history_of(&sandbox, &repo, "synthetic-ilse.md", ALL)?;
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].subject, "add synthetic-ilse");
    assert_eq!(
        commits[0].trailers,
        [
            Trailer {
                key: "Approved-by".to_owned(),
                value: "Jann".to_owned(),
            },
            Trailer {
                key: "Co-Authored-By".to_owned(),
                value: "Claude <noreply@example.invalid>".to_owned(),
            },
        ]
    );
    Ok(())
}

/// Merges `branch` into the checked-out branch with a merge commit.
fn merge(sandbox: &Sandbox, repo: &Path, branch: &str, message: &str) -> Result {
    sandbox.run(
        repo,
        &[
            "-c",
            "user.name=Synthetic",
            "-c",
            "user.email=synthetic@example.invalid",
            "merge",
            "-q",
            "--no-ff",
            "-m",
            message,
            branch,
        ],
    )?;
    Ok(())
}

// AC: CE-21.1
#[test]
fn a_rename_merged_from_a_branch_keeps_the_older_commits() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(
        &repo.join("synthetic-ilse.md"),
        &record("synthetic-ilse", ""),
    )?;
    Sandbox::write(&repo.join("links.md"), "See [[synthetic-ilse]].\n")?;
    sandbox.commit(&repo, "add synthetic-ilse")?;
    sandbox.run(&repo, &["checkout", "-q", "-b", "agent/rename"])?;
    rename(&sandbox, &repo, "synthetic-ilse", "synthetic-ilsa", "")?;
    sandbox.run(&repo, &["checkout", "-q", "main"])?;
    Sandbox::write(&repo.join("other.md"), "Unrelated.\n")?;
    sandbox.commit(&repo, "unrelated")?;
    merge(&sandbox, &repo, "agent/rename", "merge agent/rename")?;
    let commits = history_of(&sandbox, &repo, "synthetic-ilsa.md", ALL)?;
    let subjects: Vec<&str> = commits.iter().map(|c| c.subject.as_str()).collect();
    assert_eq!(
        subjects,
        [
            "merge agent/rename",
            "rename synthetic-ilse to synthetic-ilsa",
            "add synthetic-ilse",
        ]
    );
    // The merge shows the rename against its first parent, `main`.
    assert_eq!(
        summary(&commits[..1]),
        [line(
            "merge agent/rename",
            Change::Renamed,
            "synthetic-ilsa.md",
            Some("synthetic-ilse.md"),
        )]
    );
    // git lists the branch's rename commit as a deletion of the old path: `--follow` already
    // switched to that path at the merge.
    assert_eq!(
        summary(&commits[1..]),
        [
            line(
                "rename synthetic-ilse to synthetic-ilsa",
                Change::Deleted,
                "synthetic-ilse.md",
                None,
            ),
            line(
                "add synthetic-ilse",
                Change::Added,
                "synthetic-ilse.md",
                None
            ),
        ]
    );
    Ok(())
}

#[test]
fn an_edit_merged_from_a_branch_lists_the_branch_commit_and_the_merge() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    let path = repo.join("synthetic-ilse.md");
    Sandbox::write(&path, &record("synthetic-ilse", ""))?;
    sandbox.commit(&repo, "add synthetic-ilse")?;
    sandbox.run(&repo, &["checkout", "-q", "-b", "agent/edit"])?;
    Sandbox::write(&path, &record("synthetic-ilse", "She keeps the ledger.\n"))?;
    sandbox.commit(&repo, "edit synthetic-ilse")?;
    sandbox.run(&repo, &["checkout", "-q", "main"])?;
    Sandbox::write(&repo.join("other.md"), "Unrelated.\n")?;
    sandbox.commit(&repo, "unrelated")?;
    merge(&sandbox, &repo, "agent/edit", "merge agent/edit")?;
    let commits = history_of(&sandbox, &repo, "synthetic-ilse.md", ALL)?;
    assert_eq!(
        summary(&commits),
        [
            line(
                "merge agent/edit",
                Change::Modified,
                "synthetic-ilse.md",
                None
            ),
            line(
                "edit synthetic-ilse",
                Change::Modified,
                "synthetic-ilse.md",
                None
            ),
            line(
                "add synthetic-ilse",
                Change::Added,
                "synthetic-ilse.md",
                None
            ),
        ]
    );
    Ok(())
}

#[test]
fn a_cherry_picked_commit_keeps_only_its_trailers() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(
        &repo.join("synthetic-ilse.md"),
        &record("synthetic-ilse", ""),
    )?;
    sandbox.commit(
        &repo,
        "add synthetic-ilse\n\nSome prose about the change.\n\nSome text line\n\
         Signed-off-by: Synthetic <synthetic@example.invalid>\n\
         (cherry picked from commit 9ba7ef97af7f1dd1a92bc20e28dfab34affc423d)",
    )?;
    let commits = history_of(&sandbox, &repo, "synthetic-ilse.md", ALL)?;
    assert_eq!(commits.len(), 1);
    assert_eq!(
        commits[0].trailers,
        [Trailer {
            key: "Signed-off-by".to_owned(),
            value: "Synthetic <synthetic@example.invalid>".to_owned(),
        }]
    );
    Ok(())
}

#[test]
fn a_folded_trailer_comes_back_unfolded() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(
        &repo.join("synthetic-ilse.md"),
        &record("synthetic-ilse", ""),
    )?;
    sandbox.commit(
        &repo,
        "add synthetic-ilse\n\nApproved-by: Jann\n  after review",
    )?;
    let commits = history_of(&sandbox, &repo, "synthetic-ilse.md", ALL)?;
    assert_eq!(
        commits[0].trailers,
        [Trailer {
            key: "Approved-by".to_owned(),
            value: "Jann after review".to_owned(),
        }]
    );
    Ok(())
}

#[test]
fn a_merge_that_resolves_a_conflict_is_listed_against_its_first_parent() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    let path = repo.join("synthetic-ilse.md");
    Sandbox::write(&path, &record("synthetic-ilse", ""))?;
    sandbox.commit(&repo, "add synthetic-ilse")?;
    sandbox.run(&repo, &["checkout", "-q", "-b", "side"])?;
    Sandbox::write(&path, &record("synthetic-ilse", "Side line.\n"))?;
    sandbox.commit(&repo, "side line")?;
    sandbox.run(&repo, &["checkout", "-q", "main"])?;
    Sandbox::write(&path, &record("synthetic-ilse", "Main line.\n"))?;
    sandbox.commit(&repo, "main line")?;
    assert!(merge(&sandbox, &repo, "side", "merge side").is_err());
    Sandbox::write(&path, &record("synthetic-ilse", "Both lines.\n"))?;
    sandbox.commit(&repo, "merge side")?;
    let commits = history_of(&sandbox, &repo, "synthetic-ilse.md", ALL)?;
    let subjects: Vec<&str> = commits.iter().map(|c| c.subject.as_str()).collect();
    assert_eq!(subjects[0], "merge side");
    assert_eq!(
        summary(&commits[..1]),
        [line(
            "merge side",
            Change::Modified,
            "synthetic-ilse.md",
            None
        )]
    );
    assert!(subjects.contains(&"side line"));
    assert!(subjects.contains(&"main line"));
    assert_eq!(subjects.last(), Some(&"add synthetic-ilse"));
    assert_eq!(commits.len(), 4);
    Ok(())
}

#[test]
fn a_cancelled_history_is_cancelled() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = renamed_once(&sandbox)?;
    let cancel = Cancel::new();
    cancel.cancel();
    let result = history::history(&sandbox.git()?, &repo, b"synthetic-ilsa.md", ALL, &cancel);
    assert!(
        matches!(result, Err(GitError::Cancelled { .. })),
        "{result:?}"
    );
    Ok(())
}
