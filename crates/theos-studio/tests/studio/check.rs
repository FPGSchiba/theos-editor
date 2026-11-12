//! Check with and without a binaries folder.

use std::path::{Path, PathBuf};

use theos_studio::{Binaries, BinariesFolder, Workspace, binaries_folder, check};

use crate::common::{TempProject, TestResult, put};

fn folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("theos-content")
        .join("tests")
        .join("fixtures")
        .join("binaries")
}

/// The record of the fixture project that holds binary references.
const V1: &str = "concepts/synthetic-bin/synthetic-bin-v1.md";

/// A copy of the `binaries` project whose `V1` also has two index findings: an unknown field at
/// line 6 and a bad rating at line 14, around the binary warnings at lines 12 and 13.
fn project_with_index_findings() -> Result<TempProject, crate::common::Error> {
    let project = TempProject::copy_of("binaries")?;
    let text = std::fs::read_to_string(project.root().join(V1))?
        .replace(
            "title: synthetic-bin-v1\n",
            "title: synthetic-bin-v1\nbogus_field: 1\n",
        )
        .replace("rating: unrated", "rating: nonsense");
    put(project.root(), V1, text.as_bytes())?;
    Ok(project)
}

#[test]
fn a_binaries_folder_adds_its_warnings_in_index_order() -> TestResult {
    let project = project_with_index_findings()?;
    let mut workspace = Workspace::open(project.root())?;
    let without = check(&mut workspace, BinariesFolder::NotSet)?;
    let with = check(&mut workspace, BinariesFolder::Folder(folder()))?;
    assert_eq!(with.binaries, Binaries::Checked { folder: folder() });

    // The index's own order is not changed by `order`.
    assert_eq!(without.findings, workspace.index().findings()?);
    let mut reordered = without.findings.clone();
    theos_studio::check::order(&mut reordered);
    assert_eq!(reordered, without.findings);

    // Index findings and binary warnings share a path: they interleave by line.
    let in_v1: Vec<(&str, usize)> = with
        .findings
        .iter()
        .filter(|f| f.path == V1)
        .map(|f| (f.rule.as_str(), f.position.map_or(0, |p| p.line)))
        .collect();
    assert_eq!(
        in_v1,
        [
            ("schema/unknown-field", 6),
            ("binary/hash", 12),
            ("binary/missing", 13),
            ("schema/value", 14),
        ]
    );
    // The index findings are all still there, and the binary warnings are the only additions.
    let others: Vec<_> = with
        .findings
        .iter()
        .filter(|f| !f.rule.starts_with("binary/"))
        .cloned()
        .collect();
    assert_eq!(others, without.findings);
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
