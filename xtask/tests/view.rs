//! The workspace view over fixture workspaces (`tests/fixtures/view/<case>/`).
//!
//! Every test copies a case to a temporary directory and builds the view there offline: cargo
//! checks the copy, and nothing is written into the repository.

use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use xtask::view::{Artifact, Shared, TargetKind, View, ViewOptions};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("view")
}

/// Copies the directory `from` to `to`, which is created.
fn copy_dir(from: &Path, to: &Path) -> TestResult {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// A copy of the case in a temporary directory.
fn copy_case(case: &str) -> TestResult<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    copy_dir(&fixtures().join(case), dir.path())?;
    Ok(dir)
}

fn files(artifact: &Artifact) -> BTreeSet<&str> {
    artifact.files.iter().map(String::as_str).collect()
}

fn find<'a>(
    view: &'a View,
    package: &str,
    kind: TargetKind,
    test: bool,
) -> TestResult<&'a Artifact> {
    view.artifacts
        .iter()
        .find(|a| a.package == package && a.kind == kind && a.test == test)
        .ok_or_else(|| format!("no artifact {package} {kind:?} test={test}").into())
}

#[test]
fn the_view_lists_the_artifacts_of_the_members() -> TestResult {
    let dir = copy_case("plain")?;
    let view = View::build(dir.path(), ViewOptions::FIXTURE)?;

    let lib = find(&view, "a", TargetKind::Lib, false)?;
    assert_eq!(lib.manifest_dir, "crates/a");
    assert_eq!(lib.src_path, "crates/a/src/lib.rs");
    assert_eq!(
        files(lib),
        BTreeSet::from(["crates/a/src/lib.rs", "crates/a/src/sub.rs"])
    );
    find(&view, "a", TargetKind::Lib, true)?;

    let bin = find(&view, "b", TargetKind::Bin, false)?;
    assert_eq!(bin.manifest_dir, "crates/b");
    assert_eq!(files(bin), BTreeSet::from(["crates/b/src/main.rs"]));

    let test = find(&view, "a", TargetKind::Test, true)?;
    assert_eq!(test.name, "it");
    assert_eq!(files(test), BTreeSet::from(["crates/a/tests/it.rs"]));

    let build = find(&view, "a", TargetKind::BuildScript, false)?;
    assert_eq!(files(build), BTreeSet::from(["crates/a/build.rs"]));
    assert!(view.findings.is_empty(), "{:?}", view.findings);
    Ok(())
}

// AC: CE-08.8
#[test]
fn dep_info_lists_path_modules_includes_and_data() -> TestResult {
    let dir = copy_case("path-and-include")?;
    let view = View::build(dir.path(), ViewOptions::FIXTURE)?;
    let lib = find(&view, "a", TargetKind::Lib, false)?;
    assert_eq!(
        files(lib),
        BTreeSet::from([
            "crates/a/src/lib.rs",
            "crates/a/extra/m.rs",
            "crates/a/extra/inc.rs",
            "crates/a/data.txt",
        ])
    );
    assert!(view.source("crates/a/extra/m.rs").is_some());
    assert!(view.source("crates/a/data.txt").is_none());
    Ok(())
}

#[test]
fn test_files_are_what_test_builds_compile() -> TestResult {
    let dir = copy_case("plain")?;
    let view = View::build(dir.path(), ViewOptions::FIXTURE)?;
    let test_files = view.test_files();
    assert!(test_files.contains("crates/a/tests/it.rs"));
    assert!(test_files.contains("crates/a/src/lib.rs"));
    assert!(!test_files.contains("crates/a/build.rs"));
    let sources: Vec<&str> = view.sources().map(|s| s.path.as_str()).collect();
    assert_eq!(
        sources,
        [
            "crates/a/build.rs",
            "crates/a/src/lib.rs",
            "crates/a/src/sub.rs",
            "crates/a/tests/it.rs",
            "crates/b/src/main.rs",
        ]
    );
    Ok(())
}

#[test]
fn a_byte_order_mark_changes_nothing() -> TestResult {
    let dir = copy_case("bom")?;
    let view = View::build(dir.path(), ViewOptions::FIXTURE)?;
    let source = view
        .source("crates/a/src/lib.rs")
        .ok_or("not in the view")?;
    assert!(source.file.is_some());
    assert!(source.tokens.is_some());
    assert!(view.findings.is_empty(), "{:?}", view.findings);
    Ok(())
}

#[test]
fn a_workspace_that_does_not_check_is_an_error() -> TestResult {
    let dir = copy_case("does-not-check")?;
    let result = View::build(dir.path(), ViewOptions::FIXTURE);
    match result {
        Err(error @ xtask::Error::Command { .. }) => {
            let text = error.to_string();
            assert!(!text.contains("fn {"), "{text}");
            assert!(
                text.contains("cargo check --workspace --all-targets --locked"),
                "{text}"
            );
            Ok(())
        }
        other => Err(format!("expected a command error, got {:?}", other.map(|_| ())).into()),
    }
}

#[test]
fn shared_builds_the_view_once() -> TestResult {
    let dir = copy_case("plain")?;
    let shared = Shared::with(ViewOptions::FIXTURE);
    let first: *const View = shared.view(dir.path())?;
    fs::write(dir.path().join("crates/a/src/extra.rs"), "//! Added.\n")?;
    fs::write(
        dir.path().join("crates/a/src/lib.rs"),
        "//! Crate a.\n\npub mod sub;\npub mod extra;\n",
    )?;
    let second = shared.view(dir.path())?;
    assert!(std::ptr::eq(first, second));
    assert!(second.source("crates/a/src/extra.rs").is_none());
    Ok(())
}
