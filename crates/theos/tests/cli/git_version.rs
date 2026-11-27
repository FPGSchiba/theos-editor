//! `theos check --staged` explains a missing or too-old git (CE-21.3).

use std::path::{Path, PathBuf};

use crate::common::{Sandbox, TestResult, staged_project, stderr};

/// A copy of the staged fixture project inside a repository made while the real git is on the
/// `PATH`.
fn repository(sandbox: &Sandbox) -> Result<PathBuf, crate::common::Error> {
    let project = sandbox.copy_project(&staged_project(), "repo")?;
    sandbox.commit_all(&project)?;
    Ok(project)
}

/// A folder holding one program `git` that reports version 2.37.9; returns the folder and the
/// program.
fn stand_in_git(sandbox: &Sandbox) -> Result<(PathBuf, PathBuf), crate::common::Error> {
    let folder = sandbox.work().join("old-git");
    std::fs::create_dir(&folder)?;
    let program = write_stand_in(&folder)?;
    Ok((folder, program))
}

#[cfg(unix)]
fn write_stand_in(folder: &Path) -> Result<PathBuf, crate::common::Error> {
    use std::os::unix::fs::PermissionsExt;

    let program = folder.join("git");
    std::fs::write(&program, "#!/bin/sh\necho \"git version 2.37.9\"\n")?;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))?;
    Ok(program)
}

#[cfg(windows)]
fn write_stand_in(folder: &Path) -> Result<PathBuf, crate::common::Error> {
    let program = folder.join("git.bat");
    std::fs::write(&program, "@echo git version 2.37.9\r\n")?;
    Ok(program)
}

// AC: CE-21.3
#[test]
fn no_git_on_the_path_is_explained() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = repository(&sandbox)?;
    let empty = sandbox.work().join("empty");
    std::fs::create_dir(&empty)?;
    let output = sandbox.run_with_path(&["check", "--staged"], &project, empty.as_os_str())?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let err = stderr(&output);
    assert!(err.contains("git was not found on the PATH"), "{err}");
    assert!(err.contains("git 2.38 or newer"), "{err}");
    Ok(())
}

/// Runs `check --staged` with only the stand-in git on the `PATH` and checks the message.
fn assert_too_old_is_explained() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = repository(&sandbox)?;
    let (folder, program) = stand_in_git(&sandbox)?;
    let output = sandbox.run_with_path(&["check", "--staged"], &project, folder.as_os_str())?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let err = stderr(&output);
    assert!(err.contains("git 2.37.9 ("), "{err}");
    assert!(err.contains(&*program.to_string_lossy()), "{err}");
    assert!(err.contains("is too old"), "{err}");
    assert!(err.contains("git 2.38 or newer"), "{err}");
    Ok(())
}

// AC: CE-21.3
#[cfg(unix)]
#[test]
fn a_too_old_git_is_explained_on_unix() -> TestResult {
    assert_too_old_is_explained()
}

// AC: CE-21.3
#[cfg(windows)]
#[test]
fn a_too_old_git_is_explained_on_windows() -> TestResult {
    assert_too_old_is_explained()
}
