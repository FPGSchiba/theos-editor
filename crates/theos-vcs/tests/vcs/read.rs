//! A file's bytes at a version: HEAD, the index, a revision and the working tree.

use std::fmt::Write;
use std::path::Path;

use theos_vcs::read::{self, Content, Version};

use crate::common::Sandbox;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn read_at(sandbox: &Sandbox, top: &Path, version: Version<'_>, path: &str) -> Result<Content> {
    Ok(read::read(&sandbox.git()?, top, version, path.as_bytes())?)
}

fn file(text: &str) -> Content {
    Content::File(text.as_bytes().to_vec())
}

/// Commits `a.md` as `head`, stages `index` and leaves `tree` on disk.
fn three_versions(sandbox: &Sandbox) -> Result<std::path::PathBuf> {
    let repo = sandbox.init("repo")?;
    let a = repo.join("a.md");
    Sandbox::write(&a, "head\n")?;
    sandbox.commit(&repo, "first")?;
    Sandbox::write(&a, "index\n")?;
    sandbox.run(&repo, &["add", "a.md"])?;
    Sandbox::write(&a, "tree\n")?;
    Ok(repo)
}

#[test]
fn head_index_and_work_tree_of_one_file_give_three_texts() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = three_versions(&sandbox)?;
    assert_eq!(
        read_at(&sandbox, &repo, Version::Head, "a.md")?,
        file("head\n")
    );
    assert_eq!(
        read_at(&sandbox, &repo, Version::Index, "a.md")?,
        file("index\n")
    );
    assert_eq!(
        read_at(&sandbox, &repo, Version::WorkTree, "a.md")?,
        file("tree\n")
    );
    Ok(())
}

#[test]
fn a_revision_gives_that_commits_bytes() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = three_versions(&sandbox)?;
    let id = sandbox.run(&repo, &["rev-parse", "HEAD"])?;
    let id = id.trim();
    assert_eq!(
        read_at(&sandbox, &repo, Version::Rev(id), "a.md")?,
        file("head\n")
    );
    assert_eq!(
        read_at(&sandbox, &repo, Version::Rev("main"), "a.md")?,
        file("head\n")
    );
    Ok(())
}

#[test]
fn a_path_a_version_does_not_hold_is_absent() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = three_versions(&sandbox)?;
    for version in [
        Version::Head,
        Version::Index,
        Version::Rev("main"),
        Version::WorkTree,
    ] {
        assert_eq!(
            read_at(&sandbox, &repo, version, "none.md")?,
            Content::Absent
        );
    }
    Ok(())
}

#[test]
fn before_the_first_commit_head_is_absent() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("a.md"), "x\n")?;
    sandbox.run(&repo, &["add", "a.md"])?;
    assert_eq!(
        read_at(&sandbox, &repo, Version::Head, "a.md")?,
        Content::Absent
    );
    assert_eq!(
        read_at(&sandbox, &repo, Version::Index, "a.md")?,
        file("x\n")
    );
    Ok(())
}

#[test]
fn an_intent_to_add_entry_reads_absent_from_the_index() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = three_versions(&sandbox)?;
    Sandbox::write(&repo.join("new.md"), "new\n")?;
    sandbox.run(&repo, &["add", "-N", "new.md"])?;
    assert_eq!(
        read_at(&sandbox, &repo, Version::Index, "new.md")?,
        Content::Absent
    );
    assert_eq!(
        read_at(&sandbox, &repo, Version::WorkTree, "new.md")?,
        file("new\n")
    );
    Ok(())
}

#[test]
fn an_intent_to_add_entry_of_a_committed_path_reads_absent_from_the_index() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = three_versions(&sandbox)?;
    sandbox.run(&repo, &["rm", "-q", "-f", "--cached", "a.md"])?;
    sandbox.run(&repo, &["add", "-N", "a.md"])?;
    assert_eq!(
        read_at(&sandbox, &repo, Version::Index, "a.md")?,
        Content::Absent
    );
    assert_eq!(
        read_at(&sandbox, &repo, Version::Head, "a.md")?,
        file("head\n")
    );
    Ok(())
}

/// The names a commit made from objects can hold on every OS, although no Windows work tree can.
const COMMITTED: [&str; 5] = ["*.md", "?.md", "a b:c.md", "1:notes.md", "d/1:n.md"];

/// A commit holding `COMMITTED` as paths, built from objects so that no file on disk, and no index
/// entry, has to carry names that NTFS refuses.
#[test]
fn colon_and_glob_names_in_a_commit_are_literal() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    let blob = sandbox.run_input(&repo, &["hash-object", "-w", "--stdin"], b"x\n")?;
    // `mktree` takes one folder at a time: the names below `d/` go into their own tree.
    let mut listings = [String::new(), String::new()];
    for name in COMMITTED {
        match name.strip_prefix("d/") {
            Some(inner) => writeln!(listings[1], "100644 blob {}\t{inner}", blob.trim())?,
            None => writeln!(listings[0], "100644 blob {}\t{name}", blob.trim())?,
        }
    }
    let inner = sandbox.run_input(&repo, &["mktree"], listings[1].as_bytes())?;
    writeln!(listings[0], "040000 tree {}\td", inner.trim())?;
    let tree = sandbox.run_input(&repo, &["mktree"], listings[0].as_bytes())?;
    let commit = sandbox.run(
        &repo,
        &[
            "-c",
            "user.name=Synthetic",
            "-c",
            "user.email=synthetic@example.invalid",
            "commit-tree",
            "-m",
            "special",
            tree.trim(),
        ],
    )?;
    sandbox.run(&repo, &["update-ref", "refs/heads/main", commit.trim()])?;
    for name in COMMITTED {
        for version in [
            Version::Head,
            Version::Rev("main"),
            Version::Rev(commit.trim()),
        ] {
            assert_eq!(
                read_at(&sandbox, &repo, version, name)?,
                file("x\n"),
                "{name}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_revision_that_does_not_exist_is_an_error_but_a_missing_path_is_absent() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = three_versions(&sandbox)?;
    assert!(read_at(&sandbox, &repo, Version::Rev("nosuch"), "a.md").is_err());
    assert_eq!(
        read_at(&sandbox, &repo, Version::Rev("main"), "none.md")?,
        Content::Absent
    );
    Ok(())
}

/// The index and the working tree with the same names; Windows cannot hold them (`:`, `*`, `?`).
#[cfg(unix)]
#[test]
fn colon_and_glob_names_in_the_index_and_on_disk_are_literal() -> Result {
    const SPECIAL: [&str; 3] = ["1:notes.md", "*.md", "?.md"];
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    for name in SPECIAL {
        Sandbox::write(&repo.join(name), "x\n")?;
    }
    sandbox.commit(&repo, "first")?;
    for name in SPECIAL {
        for version in [Version::Head, Version::Index, Version::WorkTree] {
            assert_eq!(
                read_at(&sandbox, &repo, version, name)?,
                file("x\n"),
                "{name}"
            );
        }
    }
    Ok(())
}

#[test]
fn awkward_paths_are_literal_paths() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    let names = ["-x.md", "a b [x].md", "é.md"];
    for name in names {
        Sandbox::write(&repo.join(name), &format!("{name}\n"))?;
    }
    sandbox.commit(&repo, "first")?;
    for name in names {
        let want = file(&format!("{name}\n"));
        for version in [
            Version::Head,
            Version::Index,
            Version::Rev("main"),
            Version::WorkTree,
        ] {
            assert_eq!(read_at(&sandbox, &repo, version, name)?, want, "{name}");
        }
    }
    Ok(())
}

#[test]
fn a_folder_is_not_a_file_in_every_version() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("dir").join("a.md"), "x\n")?;
    sandbox.commit(&repo, "first")?;
    for version in [Version::Head, Version::Rev("main"), Version::WorkTree] {
        assert_eq!(read_at(&sandbox, &repo, version, "dir")?, Content::NotAFile);
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_is_not_a_file() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("a.md"), "x\n")?;
    std::os::unix::fs::symlink("a.md", repo.join("link.md"))?;
    assert_eq!(
        read_at(&sandbox, &repo, Version::WorkTree, "link.md")?,
        Content::NotAFile
    );
    sandbox.commit(&repo, "first")?;
    assert_eq!(
        read_at(&sandbox, &repo, Version::Head, "link.md")?,
        Content::NotAFile
    );
    assert_eq!(
        read_at(&sandbox, &repo, Version::Index, "link.md")?,
        Content::NotAFile
    );
    Ok(())
}

#[test]
fn a_committed_link_is_not_a_file_in_head() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("a.md"), "x\n")?;
    sandbox.commit(&repo, "first")?;
    // A link entry made without a link on disk, so it works on Windows too.
    let target = sandbox.run(&repo, &["rev-parse", "HEAD:a.md"])?;
    let cacheinfo = format!("120000,{},link.md", target.trim());
    sandbox.run(&repo, &["update-index", "--add", "--cacheinfo", &cacheinfo])?;
    sandbox.run(
        &repo,
        &[
            "-c",
            "user.name=Synthetic",
            "-c",
            "user.email=synthetic@example.invalid",
            "commit",
            "-q",
            "-m",
            "link",
        ],
    )?;
    assert_eq!(
        read_at(&sandbox, &repo, Version::Head, "link.md")?,
        Content::NotAFile
    );
    assert_eq!(
        read_at(&sandbox, &repo, Version::Index, "link.md")?,
        Content::NotAFile
    );
    Ok(())
}

#[test]
fn a_path_that_is_not_utf_8_is_an_output_error() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    let result = read::read(
        &sandbox.git()?,
        &repo,
        Version::Head,
        &[0xff, b'.', b'm', b'd'],
    );
    assert!(
        matches!(result, Err(theos_vcs::GitError::Output { .. })),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn a_folder_that_is_no_repository_is_a_git_failure_for_the_index_and_a_commit() -> Result {
    let sandbox = Sandbox::new()?;
    let folder = sandbox.path().join("plain");
    std::fs::create_dir_all(&folder)?;
    for version in [Version::Index, Version::Head, Version::Rev("main")] {
        let result = read::read(&sandbox.git()?, &folder, version, b"a.md");
        assert!(
            matches!(result, Err(theos_vcs::GitError::Failed { .. })),
            "{result:?}"
        );
    }
    Ok(())
}
