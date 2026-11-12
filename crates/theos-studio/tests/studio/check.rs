//! Check with and without a binaries folder.

use std::path::{Path, PathBuf};

use theos_studio::{Binaries, BinariesFolder, Workspace, binaries_folder, check};

use crate::common::{TempProject, TestResult};

fn folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("theos-content")
        .join("tests")
        .join("fixtures")
        .join("binaries")
}

#[test]
fn a_binaries_folder_adds_its_warnings_in_index_order() -> TestResult {
    let project = TempProject::copy_of("binaries")?;
    let mut workspace = Workspace::open(project.root())?;
    let without = check(&mut workspace, BinariesFolder::NotSet)?;
    let with = check(&mut workspace, BinariesFolder::Folder(folder()))?;
    assert_eq!(with.binaries, Binaries::Checked { folder: folder() });
    let added = with
        .findings
        .iter()
        .filter(|f| f.rule.starts_with("binary/"))
        .count();
    assert!(added > 0);
    assert!(
        without
            .findings
            .iter()
            .all(|f| !f.rule.starts_with("binary/"))
    );
    assert_eq!(with.findings.len(), without.findings.len() + added);
    let mut sorted = with.findings.clone();
    theos_studio::check::order(&mut sorted);
    assert_eq!(with.findings, sorted);
    let paths: Vec<&str> = with.findings.iter().map(|f| f.path.as_str()).collect();
    let mut by_path = paths.clone();
    by_path.sort_unstable();
    assert_eq!(paths, by_path);
    Ok(())
}

#[test]
fn no_folder_says_why() -> TestResult {
    let project = TempProject::copy_of("binaries")?;
    let mut workspace = Workspace::open(project.root())?;
    let result = check(&mut workspace, BinariesFolder::NotSet)?;
    assert_eq!(
        result.binaries,
        Binaries::NotChecked {
            reason: "no binaries folder set".to_owned()
        }
    );
    Ok(())
}

#[test]
fn an_unreadable_setting_is_the_reason() -> TestResult {
    let project = TempProject::copy_of("binaries")?;
    let mut workspace = Workspace::open(project.root())?;
    let result = check(
        &mut workspace,
        BinariesFolder::Unreadable("settings.toml: broken".to_owned()),
    )?;
    assert_eq!(
        result.binaries,
        Binaries::NotChecked {
            reason: "settings.toml: broken".to_owned()
        }
    );
    Ok(())
}

#[test]
fn the_flag_wins_over_the_settings() {
    let flag = PathBuf::from("given");
    assert_eq!(
        binaries_folder(Some(flag.clone())),
        BinariesFolder::Folder(flag)
    );
}
