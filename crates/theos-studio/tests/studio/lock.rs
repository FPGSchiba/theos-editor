//! The repo lock across processes.

use std::path::Path;
use std::time::Duration;

use theos_studio::lock::{self, LockError, Owner};

use crate::common::{HelperProcess, TempProject, files, spawn_helper};
use crate::helper::HelperTask;

/// Starts a helper that holds the lock of `project` and waits until it has it.
fn hold(
    project: &TempProject,
    scratch: &Path,
) -> Result<HelperProcess, Box<dyn std::error::Error>> {
    let task = HelperTask::Hold {
        root: project.root().to_path_buf(),
        purpose: "synthetic hold".to_owned(),
        ready: scratch.join("ready"),
        release: scratch.join("release"),
    };
    let mut child = spawn_helper(&task)?;
    child.wait_for(&scratch.join("ready"))?;
    Ok(child)
}

// AC: CE-22.2
#[test]
fn a_second_process_waits_and_names_the_holder() -> Result<(), Box<dyn std::error::Error>> {
    let project = TempProject::new()?;
    let scratch = tempfile::tempdir()?;
    let mut child = hold(&project, scratch.path())?;

    let Err(LockError::Busy { owner, waited }) =
        lock::acquire(project.root(), "test", Duration::from_millis(300))
    else {
        return Err("expected Busy".into());
    };
    let owner = owner.ok_or("no owner named")?;
    assert_eq!(owner.pid, child.id());
    assert_eq!(owner.purpose, "synthetic hold");
    assert!(waited >= Duration::from_millis(300), "waited {waited:?}");

    std::fs::write(scratch.path().join("release"), "")?;
    let guard = lock::acquire(project.root(), "test", Duration::from_secs(5))?;
    drop(guard);
    child.wait()?;
    Ok(())
}

// AC: CE-22.2
#[test]
fn a_killed_holder_frees_the_lock_at_once() -> Result<(), Box<dyn std::error::Error>> {
    let project = TempProject::new()?;
    let scratch = tempfile::tempdir()?;
    let mut child = hold(&project, scratch.path())?;
    child.kill()?;
    child.wait()?;
    let guard = lock::acquire(project.root(), "test", Duration::ZERO)?;
    drop(guard);
    Ok(())
}

#[test]
fn the_owner_file_goes_with_the_guard() -> Result<(), Box<dyn std::error::Error>> {
    let project = TempProject::new()?;
    let owner_path = project.root().join(".theos/cache/lock.owner");
    let guard = lock::acquire(project.root(), "unit", Duration::ZERO)?;
    let owner: Owner = serde_json::from_str(&std::fs::read_to_string(&owner_path)?)?;
    assert_eq!(owner.pid, std::process::id());
    assert_eq!(owner.purpose, "unit");
    drop(guard);
    assert!(!owner_path.exists());
    Ok(())
}

#[test]
fn busy_without_an_owner_file_says_so() -> Result<(), Box<dyn std::error::Error>> {
    let project = TempProject::new()?;
    let guard = lock::acquire(project.root(), "unit", Duration::ZERO)?;
    std::fs::remove_file(project.root().join(".theos/cache/lock.owner"))?;
    let Err(err) = lock::acquire(project.root(), "other", Duration::ZERO) else {
        return Err("expected Busy".into());
    };
    assert!(matches!(err, LockError::Busy { owner: None, .. }));
    assert_eq!(
        err.to_string(),
        "the project is busy: another theos process holds .theos/cache/lock"
    );
    drop(guard);
    Ok(())
}

#[test]
fn acquire_creates_the_cache_folder() -> Result<(), Box<dyn std::error::Error>> {
    let project = TempProject::new()?;
    assert!(!project.root().join(".theos").exists());
    let guard = lock::acquire(project.root(), "unit", Duration::ZERO)?;
    assert!(project.root().join(".theos/cache/lock").is_file());
    drop(guard);
    Ok(())
}

#[test]
fn taking_the_lock_leaves_the_project_files_alone() -> Result<(), Box<dyn std::error::Error>> {
    let project = TempProject::copy_of("body")?;
    let before = files(project.root())?;
    let guard = lock::acquire(project.root(), "unit", Duration::ZERO)?;
    drop(guard);
    assert_eq!(files(project.root())?, before);
    Ok(())
}
