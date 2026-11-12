//! Opening and finding a workspace.

use theos_studio::{StudioError, Workspace, find_root};

use crate::common::{TempProject, TestResult};

#[test]
fn open_syncs_the_index() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let workspace = Workspace::open(project.root())?;
    assert!(!workspace.is_in_memory());
    assert_ne!(workspace.index().records()?.len(), 0);
    Ok(())
}

#[test]
fn find_walks_up_to_theos_toml() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let start = project.root().join("lore").join("clues");
    let workspace = Workspace::find(&start)?;
    assert_eq!(
        std::fs::canonicalize(workspace.project().root())?,
        std::fs::canonicalize(project.root())?
    );
    assert_eq!(
        std::fs::canonicalize(find_root(&start)?)?,
        std::fs::canonicalize(project.root())?
    );
    Ok(())
}

#[test]
fn find_without_a_project_is_not_a_project() -> TestResult {
    let dir = tempfile::tempdir()?;
    // The temp folder lies outside any theos project on the machines that run the tests.
    assert!(matches!(
        Workspace::find(dir.path()),
        Err(StudioError::NotAProject { .. })
    ));
    Ok(())
}

#[test]
fn open_of_a_folder_without_theos_toml_fails_with_the_project_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    assert!(matches!(
        Workspace::open(dir.path()),
        Err(StudioError::Project(_))
    ));
    Ok(())
}
