//! The status of a repository: unstaged renames paired through a temporary index, the real index
//! untouched, and the shapes of status P47 and P13 rely on.

use std::path::{Path, PathBuf};

use theos_vcs::status::{self, Branch, Change, Conflict, Entry, Status, Upstream};
use theos_vcs::{Cancel, GitError};

use crate::common::Sandbox;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// A record of the data model's shape; synthetic: the id and the text are made up for the test.
fn record(id: &str) -> String {
    format!(
        "---\nid: {id}\nkind: character\nname: Synthetic Mother\nstatus: draft\n---\n\n\
         A synthetic record for the status tests.\nIt has a few lines of body text,\nso that \
         git finds a renamed copy similar.\nOne more line.\n"
    )
}

fn status_of(sandbox: &Sandbox, top: &Path) -> Result<Status> {
    Ok(status::status(&sandbox.git()?, top, &Cancel::new())?)
}

fn changed(path: &str, staged: Change, unstaged: Change) -> Entry {
    Entry::Changed {
        path: path.as_bytes().to_vec(),
        staged,
        unstaged,
    }
}

fn untracked(path: &str) -> Entry {
    Entry::Untracked {
        path: path.as_bytes().to_vec(),
    }
}

/// The renamed entry, with its score left out.
fn rename_of(entry: &Entry) -> Option<(&[u8], &[u8], Change, Change)> {
    match entry {
        Entry::Renamed {
            from,
            to,
            staged,
            unstaged,
            ..
        } => Some((from, to, *staged, *unstaged)),
        _ => None,
    }
}

/// A repository with `mother-ilse.md` committed, then renamed on disk to `mother-ilsa.md` with
/// its `id:` rewritten (the probe of P46 §4.2).
fn renamed_on_disk(sandbox: &Sandbox) -> Result<PathBuf> {
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("mother-ilse.md"), &record("mother-ilse"))?;
    sandbox.commit(&repo, "first")?;
    std::fs::remove_file(repo.join("mother-ilse.md"))?;
    Sandbox::write(&repo.join("mother-ilsa.md"), &record("mother-ilsa"))?;
    Ok(repo)
}

#[test]
fn an_unstaged_rename_with_its_id_rewritten_is_one_rename() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = renamed_on_disk(&sandbox)?;
    let status = status_of(&sandbox, &repo)?;
    assert_eq!(status.entries.len(), 1, "{:?}", status.entries);
    assert_eq!(
        rename_of(&status.entries[0]),
        Some((
            b"mother-ilse.md".as_slice(),
            b"mother-ilsa.md".as_slice(),
            Change::Unchanged,
            Change::Renamed
        ))
    );
    Ok(())
}

#[test]
fn the_real_index_is_not_written() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = renamed_on_disk(&sandbox)?;
    let index = repo.join(".git").join("index");
    let before = std::fs::read(&index)?;
    status_of(&sandbox, &repo)?;
    assert_eq!(std::fs::read(&index)?, before);
    Ok(())
}

#[test]
fn a_split_index_is_read_and_gets_no_new_shared_file() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    sandbox.run(&repo, &["config", "core.splitIndex", "true"])?;
    Sandbox::write(&repo.join("mother-ilse.md"), &record("mother-ilse"))?;
    sandbox.commit(&repo, "first")?;
    std::fs::remove_file(repo.join("mother-ilse.md"))?;
    Sandbox::write(&repo.join("mother-ilsa.md"), &record("mother-ilsa"))?;
    let git_dir = |repo: &Path| -> Result<Vec<_>> {
        Ok(std::fs::read_dir(repo.join(".git"))?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<std::io::Result<Vec<_>>>()?)
    };
    let before = git_dir(&repo)?;
    let status = status_of(&sandbox, &repo)?;
    assert_eq!(
        status.entries.iter().map(rename_of).collect::<Vec<_>>(),
        vec![Some((
            b"mother-ilse.md".as_slice(),
            b"mother-ilsa.md".as_slice(),
            Change::Unchanged,
            Change::Renamed
        ))]
    );
    assert_eq!(git_dir(&repo)?, before);
    Ok(())
}

#[test]
fn a_staged_rename_with_an_unstaged_edit() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("a.md"), &record("a"))?;
    sandbox.commit(&repo, "first")?;
    sandbox.run(&repo, &["mv", "a.md", "b.md"])?;
    Sandbox::write(&repo.join("b.md"), &format!("{}Edited.\n", record("a")))?;
    let status = status_of(&sandbox, &repo)?;
    assert_eq!(
        status.entries.iter().map(rename_of).collect::<Vec<_>>(),
        vec![Some((
            b"a.md".as_slice(),
            b"b.md".as_slice(),
            Change::Renamed,
            Change::Modified
        ))]
    );
    Ok(())
}

#[test]
fn staged_then_deleted_an_own_intent_to_add_folders_and_ignored_files() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join(".gitignore"), "*.tmp\n")?;
    sandbox.commit(&repo, "first")?;
    Sandbox::write(&repo.join("c.md"), "staged\n")?;
    sandbox.run(&repo, &["add", "c.md"])?;
    std::fs::remove_file(repo.join("c.md"))?;
    Sandbox::write(&repo.join("d.md"), "mine\n")?;
    sandbox.run(&repo, &["add", "-N", "d.md"])?;
    Sandbox::write(&repo.join("drafts").join("a.md"), "draft a\n")?;
    Sandbox::write(&repo.join("drafts").join("sub").join("b.md"), "draft b\n")?;
    Sandbox::write(&repo.join("scratch.tmp"), "ignored\n")?;
    Sandbox::write(&repo.join("drafts").join("x.tmp"), "ignored\n")?;
    let status = status_of(&sandbox, &repo)?;
    assert_eq!(
        status.entries,
        vec![
            changed("c.md", Change::Added, Change::Deleted),
            changed("d.md", Change::Unchanged, Change::Added),
            untracked("drafts/a.md"),
            untracked("drafts/sub/b.md"),
        ]
    );
    Ok(())
}

#[test]
fn no_index_file_and_no_commit() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("a.md"), "a\n")?;
    assert!(!repo.join(".git").join("index").exists());
    let status = status_of(&sandbox, &repo)?;
    assert_eq!(
        status,
        Status {
            branch: Branch {
                head: Some("main".to_owned()),
                commit: None,
                upstream: None,
            },
            entries: vec![untracked("a.md")],
        }
    );
    assert!(!repo.join(".git").join("index").exists());
    sandbox.run(&repo, &["add", "a.md"])?;
    Sandbox::write(&repo.join("b.md"), "b\n")?;
    let status = status_of(&sandbox, &repo)?;
    assert_eq!(status.branch.commit, None);
    assert_eq!(
        status.entries,
        vec![
            changed("a.md", Change::Added, Change::Unchanged),
            untracked("b.md"),
        ]
    );
    Ok(())
}

#[test]
fn a_detached_head_has_no_branch_name() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("a.md"), "a\n")?;
    sandbox.commit(&repo, "first")?;
    sandbox.run(&repo, &["checkout", "-q", "--detach"])?;
    let id = sandbox.run(&repo, &["rev-parse", "HEAD"])?;
    let branch = status_of(&sandbox, &repo)?.branch;
    assert_eq!(branch.head, None);
    assert_eq!(branch.commit.as_deref(), Some(id.trim()));
    Ok(())
}

#[test]
fn an_upstream_ahead_one_and_behind_one() -> Result {
    let sandbox = Sandbox::new()?;
    let origin = sandbox.init("origin")?;
    Sandbox::write(&origin.join("a.md"), "a\n")?;
    sandbox.commit(&origin, "first")?;
    let origin_text = origin.to_str().ok_or("origin path")?;
    sandbox.run(sandbox.path(), &["clone", "-q", origin_text, "clone"])?;
    let clone = sandbox.path().join("clone");
    Sandbox::write(&clone.join("b.md"), "b\n")?;
    sandbox.commit(&clone, "on the clone")?;
    Sandbox::write(&origin.join("c.md"), "c\n")?;
    sandbox.commit(&origin, "on the origin")?;
    sandbox.run(&clone, &["fetch", "-q"])?;
    let branch = status_of(&sandbox, &clone)?.branch;
    assert_eq!(branch.head.as_deref(), Some("main"));
    assert_eq!(
        branch.upstream,
        Some(Upstream {
            name: "origin/main".to_owned(),
            ahead: 1,
            behind: 1,
        })
    );
    Ok(())
}

#[test]
fn a_failed_merge_is_a_conflict() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("a.md"), "base\n")?;
    sandbox.commit(&repo, "base")?;
    sandbox.run(&repo, &["checkout", "-q", "-b", "other"])?;
    Sandbox::write(&repo.join("a.md"), "theirs\n")?;
    sandbox.commit(&repo, "theirs")?;
    sandbox.run(&repo, &["checkout", "-q", "main"])?;
    Sandbox::write(&repo.join("a.md"), "ours\n")?;
    sandbox.commit(&repo, "ours")?;
    let merge = sandbox.run(
        &repo,
        &[
            "-c",
            "user.name=Synthetic",
            "-c",
            "user.email=synthetic@example.invalid",
            "merge",
            "-q",
            "other",
        ],
    );
    assert!(merge.is_err(), "the merge did not stop at the conflict");
    let status = status_of(&sandbox, &repo)?;
    assert_eq!(
        status.entries,
        vec![Entry::Conflicted {
            path: b"a.md".to_vec(),
            conflict: Conflict::BothModified,
        }]
    );
    Ok(())
}

#[test]
fn more_untracked_files_than_fit_on_a_command_line() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    let folder = repo.join("lore");
    std::fs::create_dir_all(&folder)?;
    let names: Vec<String> = (0..3000)
        .map(|number| format!("lore/synthetic-untracked-record-number-{number:04}.md"))
        .collect();
    let total: usize = names.iter().map(String::len).sum();
    assert!(total > 40_000, "{total}");
    for name in &names {
        std::fs::write(repo.join(name), "synthetic\n")?;
    }
    let status = status_of(&sandbox, &repo)?;
    let mut expected: Vec<Entry> = names.iter().map(|name| untracked(name)).collect();
    expected.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    let mut entries = status.entries;
    entries.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    assert_eq!(entries, expected);
    Ok(())
}

#[test]
fn a_path_with_a_space_brackets_a_leading_dash_and_non_ascii() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    let old = "-old [x] é.md";
    let new = "-new [x] é.md";
    let other = "lore/a [b] é.md";
    Sandbox::write(&repo.join(old), &record("old"))?;
    Sandbox::write(&repo.join(other), "other\n")?;
    sandbox.commit(&repo, "first")?;
    std::fs::remove_file(repo.join(old))?;
    Sandbox::write(&repo.join(new), &record("new"))?;
    Sandbox::write(&repo.join(other), "edited\n")?;
    Sandbox::write(&repo.join("[x]"), "untracked\n")?;
    let status = status_of(&sandbox, &repo)?;
    let renames: Vec<_> = status.entries.iter().filter_map(rename_of).collect();
    assert_eq!(
        renames,
        vec![(
            old.as_bytes(),
            new.as_bytes(),
            Change::Unchanged,
            Change::Renamed
        )]
    );
    let rest: Vec<&Entry> = status
        .entries
        .iter()
        .filter(|entry| rename_of(entry).is_none())
        .collect();
    assert_eq!(
        rest,
        vec![
            &changed(other, Change::Unchanged, Change::Modified),
            &untracked("[x]"),
        ]
    );
    Ok(())
}

#[test]
fn a_cancel_before_the_call_is_cancelled() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = renamed_on_disk(&sandbox)?;
    let cancel = Cancel::new();
    cancel.cancel();
    let result = status::status(&sandbox.git()?, &repo, &cancel);
    assert!(
        matches!(result, Err(GitError::Cancelled { .. })),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn inside_a_hooks_index_the_copy_is_read() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("a.md"), "a\n")?;
    sandbox.commit(&repo, "first")?;
    let hook_index = sandbox.path().join("hook-index");
    std::fs::copy(repo.join(".git").join("index"), &hook_index)?;
    let hook_text = hook_index.to_str().ok_or("hook index path")?;
    Sandbox::write(&repo.join("b.md"), "b\n")?;
    sandbox.run_env(&repo, &["add", "b.md"], &[("GIT_INDEX_FILE", hook_text)])?;
    Sandbox::write(&repo.join("c.md"), "c\n")?;
    let before = std::fs::read(&hook_index)?;
    let git = sandbox.git()?.with_env(&[("GIT_INDEX_FILE", hook_text)]);
    let status = status::status(&git, &repo, &Cancel::new())?;
    assert_eq!(
        status.entries,
        vec![
            changed("b.md", Change::Added, Change::Unchanged),
            untracked("c.md"),
        ]
    );
    assert_eq!(std::fs::read(&hook_index)?, before);
    // Without untracked files the hook's index is read directly.
    std::fs::remove_file(repo.join("c.md"))?;
    let status = status::status(&git, &repo, &Cancel::new())?;
    assert_eq!(
        status.entries,
        vec![changed("b.md", Change::Added, Change::Unchanged)]
    );
    Ok(())
}

#[test]
fn status_runs_no_hook_of_the_repository() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    // git runs hooks through its own `sh` on every OS (from the top folder); the script needs LF
    // line endings and, on Unix, the executable bit.
    let hook = repo.join(".git").join("hooks").join("post-index-change");
    Sandbox::write(&hook, "#!/bin/sh\necho ran > .git/hook-ran\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
    }
    let marker = repo.join(".git").join("hook-ran");
    Sandbox::write(&repo.join("a.md"), "a\n")?;
    sandbox.commit(&repo, "first")?;
    assert!(marker.exists(), "the hook does not run at all");
    std::fs::remove_file(&marker)?;
    Sandbox::write(&repo.join("b.md"), "b\n")?;
    let status = status_of(&sandbox, &repo)?;
    assert_eq!(status.entries, vec![untracked("b.md")]);
    assert!(!marker.exists(), "status ran the post-index-change hook");
    Ok(())
}

#[test]
fn status_runs_no_hook_the_repository_configures() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("a.md"), "a\n")?;
    sandbox.commit(&repo, "first")?;
    // git 2.54 and later run hooks named in the configuration (`hook.<name>.event` and
    // `hook.<name>.command`, from the top folder); older gits ignore these keys, and the test
    // passes there trivially. The second name has a dot and capitals, as a subsection may.
    let names = ["probe", "Theos.probe"];
    for name in names {
        let event = format!("hook.{name}.event");
        let command = format!("hook.{name}.command");
        let script = format!("echo ran > .git/{name}-ran");
        sandbox.run(&repo, &["config", &event, "post-index-change"])?;
        sandbox.run(&repo, &["config", &command, &script])?;
    }
    Sandbox::write(&repo.join("b.md"), "b\n")?;
    let status = status_of(&sandbox, &repo)?;
    assert_eq!(status.entries, vec![untracked("b.md")]);
    let ran: Vec<&str> = names
        .into_iter()
        .filter(|name| repo.join(".git").join(format!("{name}-ran")).exists())
        .collect();
    assert_eq!(ran, Vec::<&str>::new(), "status ran these configured hooks");
    Ok(())
}
