//! `install_hook` against real repositories: where the hook goes and what is never overwritten.

use std::path::{Path, PathBuf};

use theos_studio::StudioError;
use theos_studio::hook::{HookError, MARKER, install_hook};

use crate::common::{Error, TestResult};
use crate::repo::Repo;

const THEOS: &str = "C:/Tools/theos.exe";

fn install(
    repo: &Repo,
    place: &str,
    force: bool,
) -> Result<theos_studio::hook::Installed, StudioError> {
    let git = repo.git().map_err(|error| StudioError::Io {
        path: PathBuf::from("git"),
        source: std::io::Error::other(error.to_string()),
    })?;
    install_hook(&repo.top().join(place), &git, Path::new(THEOS), force)
}

fn pre_commit(repo: &Repo) -> PathBuf {
    repo.top().join(".git").join("hooks").join("pre-commit")
}

fn pre_merge_commit(repo: &Repo) -> PathBuf {
    repo.top()
        .join(".git")
        .join("hooks")
        .join("pre-merge-commit")
}

fn hook_error(
    result: Result<theos_studio::hook::Installed, StudioError>,
) -> Result<HookError, Error> {
    match result {
        Err(StudioError::Hook(error)) => Ok(error),
        other => Err(format!("expected a hook error, got {other:?}").into()),
    }
}

#[test]
fn a_reinstall_replaces_its_own_hook() -> TestResult {
    let repo = Repo::committed("")?;
    let first = install(&repo, "", false)?;
    assert!(!first.replaced);
    assert_eq!(first.backup, None);
    let text = std::fs::read_to_string(&first.hook)?;
    assert!(text.starts_with("#!/bin/sh\n"));
    assert!(text.contains(MARKER));
    assert!(!text.contains('\r'));
    let again = install(&repo, "", false)?;
    assert!(again.replaced);
    assert_eq!(again.backup, None);
    assert_eq!(std::fs::read_to_string(&again.hook)?, text);
    Ok(())
}

// git runs `pre-commit` for `git commit` and `pre-merge-commit` for the merge commit of
// `git merge`, so both get the same script.
#[test]
fn both_hooks_are_written_with_the_same_script() -> TestResult {
    let repo = Repo::committed("")?;
    let installed = install(&repo, "", false)?;
    assert_eq!(installed.hook, pre_commit(&repo));
    let merge = &installed.pre_merge_commit;
    assert_eq!(merge.hook, pre_merge_commit(&repo));
    assert!(!merge.replaced);
    assert_eq!(merge.backup, None);
    let text = std::fs::read_to_string(&installed.hook)?;
    assert!(text.contains(MARKER));
    assert_eq!(std::fs::read_to_string(&merge.hook)?, text);
    Ok(())
}

#[test]
fn a_reinstall_replaces_both_hooks() -> TestResult {
    let repo = Repo::committed("")?;
    install(&repo, "", false)?;
    let text = std::fs::read_to_string(pre_merge_commit(&repo))?;
    let again = install(&repo, "", false)?;
    assert!(again.replaced);
    assert!(again.pre_merge_commit.replaced);
    assert_eq!(again.pre_merge_commit.backup, None);
    assert_eq!(std::fs::read_to_string(pre_merge_commit(&repo))?, text);
    assert_eq!(std::fs::read_to_string(pre_commit(&repo))?, text);
    Ok(())
}

// Every check runs before anything is written: a refused `pre-merge-commit` leaves `pre-commit`
// unwritten.
#[test]
fn a_foreign_pre_merge_commit_alone_refuses_and_writes_nothing() -> TestResult {
    let repo = Repo::committed("")?;
    std::fs::create_dir_all(repo.top().join(".git").join("hooks"))?;
    std::fs::write(pre_merge_commit(&repo), "#!/bin/sh\necho mine\n")?;
    let error = hook_error(install(&repo, "", false))?;
    assert!(
        matches!(error, HookError::Foreign { ref path } if path.ends_with("pre-merge-commit")),
        "{error:?}"
    );
    assert!(
        error.to_string().contains("pre-merge-commit.before-theos"),
        "{error}"
    );
    assert!(!pre_commit(&repo).exists(), "half installed");
    assert_eq!(
        std::fs::read_to_string(pre_merge_commit(&repo))?,
        "#!/bin/sh\necho mine\n"
    );
    Ok(())
}

#[test]
fn a_foreign_pre_commit_alone_leaves_pre_merge_commit_unwritten() -> TestResult {
    let repo = Repo::committed("")?;
    std::fs::create_dir_all(repo.top().join(".git").join("hooks"))?;
    std::fs::write(pre_commit(&repo), "#!/bin/sh\necho mine\n")?;
    let error = hook_error(install(&repo, "", false))?;
    assert!(matches!(error, HookError::Foreign { ref path } if path.ends_with("pre-commit")));
    assert!(!pre_merge_commit(&repo).exists(), "half installed");
    Ok(())
}

#[test]
fn force_keeps_a_foreign_pre_merge_commit_as_a_backup() -> TestResult {
    let repo = Repo::committed("")?;
    let hooks = repo.top().join(".git").join("hooks");
    std::fs::create_dir_all(&hooks)?;
    std::fs::write(pre_merge_commit(&repo), "#!/bin/sh\necho mine\n")?;
    let installed = install(&repo, "", true)?;
    assert_eq!(installed.backup, None);
    let backup = installed.pre_merge_commit.backup.ok_or("no backup named")?;
    assert_eq!(backup, hooks.join("pre-merge-commit.before-theos"));
    assert_eq!(std::fs::read_to_string(&backup)?, "#!/bin/sh\necho mine\n");
    assert!(std::fs::read_to_string(pre_merge_commit(&repo))?.contains(MARKER));
    assert!(std::fs::read_to_string(pre_commit(&repo))?.contains(MARKER));
    Ok(())
}

// The refused backup of `pre-merge-commit` also keeps `--force` from renaming the foreign
// `pre-commit`.
#[test]
fn an_existing_pre_merge_commit_backup_stops_force() -> TestResult {
    let repo = Repo::committed("")?;
    let hooks = repo.top().join(".git").join("hooks");
    std::fs::create_dir_all(&hooks)?;
    std::fs::write(pre_commit(&repo), "foreign\n")?;
    std::fs::write(pre_merge_commit(&repo), "foreign merge\n")?;
    let kept = hooks.join("pre-merge-commit.before-theos");
    std::fs::write(&kept, "old backup\n")?;
    let error = hook_error(install(&repo, "", true))?;
    assert!(
        matches!(error, HookError::BackupExists { ref path } if *path == kept),
        "{error:?}"
    );
    assert_eq!(std::fs::read_to_string(pre_commit(&repo))?, "foreign\n");
    assert!(!hooks.join("pre-commit.before-theos").exists());
    assert_eq!(
        std::fs::read_to_string(pre_merge_commit(&repo))?,
        "foreign merge\n"
    );
    assert_eq!(std::fs::read_to_string(&kept)?, "old backup\n");
    Ok(())
}

#[test]
fn a_hook_for_another_project_folder_is_foreign() -> TestResult {
    let repo = Repo::committed("a")?;
    install(&repo, "a", false)?;
    let before = std::fs::read(pre_commit(&repo))?;
    let error = hook_error(install(&repo, "", false))?;
    assert!(matches!(error, HookError::Foreign { ref path } if path.ends_with("pre-commit")));
    assert_eq!(std::fs::read(pre_commit(&repo))?, before);
    Ok(())
}

#[test]
fn force_keeps_the_foreign_hook_as_a_backup() -> TestResult {
    let repo = Repo::committed("")?;
    let hooks = repo.top().join(".git").join("hooks");
    std::fs::create_dir_all(&hooks)?;
    std::fs::write(pre_commit(&repo), "#!/bin/sh\necho mine\n")?;
    let installed = install(&repo, "", true)?;
    let backup = installed.backup.ok_or("no backup named")?;
    assert_eq!(backup, hooks.join("pre-commit.before-theos"));
    assert_eq!(std::fs::read_to_string(&backup)?, "#!/bin/sh\necho mine\n");
    assert!(std::fs::read_to_string(&installed.hook)?.contains(MARKER));
    Ok(())
}

#[test]
fn force_never_overwrites_a_backup() -> TestResult {
    let repo = Repo::committed("")?;
    let hooks = repo.top().join(".git").join("hooks");
    std::fs::create_dir_all(&hooks)?;
    std::fs::write(pre_commit(&repo), "foreign\n")?;
    std::fs::write(hooks.join("pre-commit.before-theos"), "old backup\n")?;
    let error = hook_error(install(&repo, "", true))?;
    assert!(matches!(error, HookError::BackupExists { .. }));
    assert_eq!(std::fs::read_to_string(pre_commit(&repo))?, "foreign\n");
    assert_eq!(
        std::fs::read_to_string(hooks.join("pre-commit.before-theos"))?,
        "old backup\n"
    );
    Ok(())
}

#[test]
fn a_local_hooks_path_inside_git_is_used() -> TestResult {
    let repo = Repo::committed("")?;
    repo.run(&["config", "core.hooksPath", ".git/myhooks"])?;
    assert!(!repo.top().join(".git").join("myhooks").exists());
    let installed = install(&repo, "", false)?;
    assert!(
        repo.top()
            .join(".git")
            .join("myhooks")
            .join("pre-commit")
            .is_file()
    );
    assert!(installed.hook.ends_with("pre-commit"));
    assert!(!pre_commit(&repo).exists());
    Ok(())
}

#[test]
fn a_tracked_hooks_path_is_refused() -> TestResult {
    let repo = Repo::committed("")?;
    repo.run(&["config", "core.hooksPath", "githooks"])?;
    let error = hook_error(install(&repo, "", false))?;
    assert!(matches!(error, HookError::OutsideGitFolder { .. }));
    assert!(!repo.top().join("githooks").exists(), "nothing is created");
    Ok(())
}

#[test]
fn a_global_hooks_path_is_refused() -> TestResult {
    let repo = Repo::committed("")?;
    let outside = tempfile::tempdir()?;
    let target = outside.path().join("global-hooks");
    let target = target.to_str().ok_or("path")?;
    repo.run(&["config", "--global", "core.hooksPath", target])?;
    let error = hook_error(install(&repo, "", false))?;
    assert!(matches!(error, HookError::OutsideGitFolder { .. }));
    assert!(!outside.path().join("global-hooks").exists());
    Ok(())
}

#[test]
fn a_linked_worktree_installs_into_the_common_hooks() -> TestResult {
    let repo = Repo::committed("")?;
    repo.run(&[
        "-c",
        "user.name=Synthetic",
        "-c",
        "user.email=synthetic@example.invalid",
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        "second",
    ])?;
    let linked = tempfile::tempdir()?;
    let linked_top = linked.path().join("linked");
    repo.run(&["worktree", "add", "-q", linked_top.to_str().ok_or("path")?])?;
    let git = repo.git()?;
    let installed = install_hook(&linked_top, &git, Path::new(THEOS), false)?;
    let common = theos_vcs::repo::common_dir(&git, repo.top())?;
    assert_eq!(
        std::fs::canonicalize(&installed.hook)?,
        std::fs::canonicalize(common.join("hooks").join("pre-commit"))?
    );
    assert!(pre_commit(&repo).is_file());
    Ok(())
}

#[test]
fn outside_git_is_not_in_a_repository() -> TestResult {
    let repo = Repo::committed("")?;
    let outside = tempfile::tempdir()?;
    let result = install_hook(outside.path(), &repo.git()?, Path::new(THEOS), false);
    assert!(
        matches!(result, Err(StudioError::NotInRepository { .. })),
        "{result:?}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_dangling_backup_link_counts_as_a_backup() -> TestResult {
    let repo = Repo::committed("")?;
    let hooks = repo.top().join(".git").join("hooks");
    std::fs::create_dir_all(&hooks)?;
    std::fs::write(pre_commit(&repo), "foreign\n")?;
    let backup = hooks.join("pre-commit.before-theos");
    std::os::unix::fs::symlink(repo.top().join("nowhere"), &backup)?;
    let error = hook_error(install(&repo, "", true))?;
    assert!(matches!(error, HookError::BackupExists { .. }));
    assert_eq!(std::fs::read_to_string(pre_commit(&repo))?, "foreign\n");
    assert!(std::fs::symlink_metadata(&backup)?.file_type().is_symlink());
    assert!(!repo.top().join("nowhere").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_dangling_hook_link_is_a_foreign_hook() -> TestResult {
    let repo = Repo::committed("")?;
    let hooks = repo.top().join(".git").join("hooks");
    std::fs::create_dir_all(&hooks)?;
    std::os::unix::fs::symlink(repo.top().join("nowhere"), pre_commit(&repo))?;
    let error = hook_error(install(&repo, "", false))?;
    assert!(matches!(error, HookError::Foreign { .. }));
    assert!(!repo.top().join("nowhere").exists());
    Ok(())
}

// A link is never read or written through, also when the file it points to holds the marker and
// the exact exec line: a re-install would write this machine's `theos` into that file, which may
// be tracked. `--force` keeps the link itself as the backup.
#[cfg(unix)]
#[test]
fn a_hook_link_to_a_marked_script_is_a_foreign_hook() -> TestResult {
    let repo = Repo::committed("")?;
    let hooks = repo.top().join(".git").join("hooks");
    std::fs::create_dir_all(&hooks)?;
    let target = repo.top().join("shared-pre-commit");
    let shared = format!(
        "#!/bin/sh\n{MARKER}, shared\ntheos='/opt/theos'\n[ -x \"$theos\" ] || theos=theos\n\
         exec \"$theos\" check --staged '.'\n"
    );
    std::fs::write(&target, &shared)?;
    std::os::unix::fs::symlink(&target, pre_commit(&repo))?;
    let is_link = |path: &Path| -> std::io::Result<bool> {
        Ok(std::fs::symlink_metadata(path)?.is_symlink())
    };
    let error = hook_error(install(&repo, "", false))?;
    assert!(matches!(error, HookError::Foreign { .. }), "{error:?}");
    assert_eq!(std::fs::read_to_string(&target)?, shared);
    assert!(is_link(&pre_commit(&repo))?);

    let installed = install(&repo, "", true)?;
    let backup = installed.backup.ok_or("no backup named")?;
    assert!(is_link(&backup)?);
    assert!(!is_link(&installed.hook)?);
    assert!(std::fs::read_to_string(&installed.hook)?.contains(THEOS));
    assert_eq!(std::fs::read_to_string(&target)?, shared);
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_non_utf8_theos_path_is_refused() -> TestResult {
    use std::os::unix::ffi::OsStrExt;
    let repo = Repo::committed("")?;
    let theos = PathBuf::from(std::ffi::OsStr::from_bytes(b"/opt/theos-\xff"));
    let result = install_hook(repo.top(), &repo.git()?, &theos, false);
    assert!(
        matches!(result, Err(StudioError::Hook(HookError::TheosPath { .. }))),
        "{result:?}"
    );
    assert!(!pre_commit(&repo).exists());
    Ok(())
}
