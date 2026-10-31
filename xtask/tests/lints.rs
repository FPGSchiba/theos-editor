//! `lints` over fixture repositories (`tests/fixtures/lints/<case>/`).
//!
//! `clean` is a small repository that passes. Every other case holds only the files that differ
//! from it: a test copies `clean` to a temporary directory, lays the case over it and checks that
//! copy, where git does not apply, so the files are listed by a walk. In this repository `lints`
//! leaves everything under `tests/fixtures/` alone, which is why the cases may hold files that
//! the check forbids elsewhere.

use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use xtask::checks::lints;
use xtask::files::repo_files;
use xtask::verdict::{Check, Verdict};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// Copies the directory `from` into the existing directory `to`, over what is there.
fn copy_dir(from: &Path, to: &Path) -> TestResult {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&target)?;
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// A temporary copy of `clean` with the files of the case `case` laid over it.
fn copy_fixture(case: Option<&str>) -> TestResult<tempfile::TempDir> {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("lints");
    let dir = tempfile::tempdir()?;
    copy_dir(&fixtures.join("clean"), dir.path())?;
    if let Some(case) = case {
        copy_dir(&fixtures.join(case), dir.path())?;
    }
    Ok(dir)
}

/// The verdict of `lints` on the repository at `root`.
fn lint(root: &Path) -> TestResult<Verdict> {
    let files = repo_files(root)?;
    Ok(lints::check(root, &files)?)
}

/// The verdict of `lints` on a copy of `clean` with the case laid over it.
fn lint_case(case: &str) -> TestResult<Verdict> {
    lint(copy_fixture(Some(case))?.path())
}

/// The file and line of every finding.
fn places(verdict: &Verdict) -> Vec<(Option<&str>, Option<u32>)> {
    verdict
        .findings
        .iter()
        .map(|finding| (finding.file.as_deref(), finding.line))
        .collect()
}

/// The one finding of a verdict, which must be about `file`.
fn only_finding<'a>(verdict: &'a Verdict, file: &str) -> &'a str {
    assert_eq!(verdict.check, Check::Lints);
    assert_eq!(verdict.findings.len(), 1, "{:?}", verdict.findings);
    assert_eq!(verdict.findings[0].file.as_deref(), Some(file));
    &verdict.findings[0].message
}

#[test]
fn clean_repo_passes() -> TestResult {
    let dir = copy_fixture(None)?;
    // The pass is not for want of something to look at.
    let files = repo_files(dir.path())?;
    for file in [
        ".cargo/config.toml",
        ".config/nextest.toml",
        "clippy.toml",
        "crates/a/src/lib.rs",
        "crates/a/tests/fixtures/not-scanned.rs",
        "gate/clippy.toml",
    ] {
        assert!(
            files.iter().any(|listed| listed == file),
            "{file}: {files:?}"
        );
    }

    let verdict = lint(dir.path())?;
    assert_eq!(verdict.check, Check::Lints);
    assert!(verdict.passed(), "{:?}", verdict.findings);
    Ok(())
}

#[test]
fn crate_without_workspace_lints_fails() -> TestResult {
    let verdict = lint_case("crate-without-workspace-lints")?;
    let message = only_finding(&verdict, "crates/a/Cargo.toml");
    assert!(message.contains("workspace = true"), "{message}");
    Ok(())
}

#[test]
fn nested_clippy_toml_fails() -> TestResult {
    let verdict = lint_case("nested-clippy-toml")?;
    only_finding(&verdict, "crates/a/clippy.toml");
    Ok(())
}

#[test]
fn rustfmt_toml_fails() -> TestResult {
    let verdict = lint_case("rustfmt-toml")?;
    only_finding(&verdict, "rustfmt.toml");
    Ok(())
}

#[test]
fn crate_level_cargo_dir_fails() -> TestResult {
    let verdict = lint_case("crate-level-cargo-dir")?;
    only_finding(&verdict, "crates/a/.cargo");
    Ok(())
}

#[test]
fn mirror_drift_fails() -> TestResult {
    let verdict = lint_case("mirror-drift")?;
    let message = only_finding(&verdict, "clippy.toml");
    assert!(message.contains("gate/clippy.toml"), "{message}");

    let dir = copy_fixture(None)?;
    fs::write(
        dir.path().join(".config").join("nextest.toml"),
        "[profile.default]\nretries = 2\n",
    )?;
    let verdict = lint(dir.path())?;
    let message = only_finding(&verdict, ".config/nextest.toml");
    assert!(message.contains("gate/nextest.toml"), "{message}");
    Ok(())
}

#[test]
fn lints_table_drift_fails() -> TestResult {
    let verdict = lint_case("lints-table-drift")?;
    let message = only_finding(&verdict, "Cargo.toml");
    assert!(message.contains("gate/lints.toml"), "{message}");
    Ok(())
}

#[test]
fn a_mirror_with_crlf_line_ends_is_the_same() -> TestResult {
    let dir = copy_fixture(None)?;
    let original = fs::read_to_string(dir.path().join("gate").join("clippy.toml"))?;
    fs::write(
        dir.path().join("clippy.toml"),
        original.replace("\r\n", "\n").replace('\n', "\r\n"),
    )?;
    let verdict = lint(dir.path())?;
    assert!(verdict.passed(), "{:?}", verdict.findings);
    Ok(())
}

#[test]
fn a_missing_file_is_a_finding() -> TestResult {
    for file in [
        "Cargo.toml",
        "gate/lints.toml",
        "clippy.toml",
        "gate/clippy.toml",
        ".config/nextest.toml",
        "gate/nextest.toml",
    ] {
        let dir = copy_fixture(None)?;
        fs::remove_file(dir.path().join(file))?;
        let verdict = lint(dir.path())?;
        assert!(!verdict.passed(), "{file}");
        assert!(
            verdict
                .findings
                .iter()
                .all(|finding| finding.file.as_deref() == Some(file)),
            "{file}: {:?}",
            verdict.findings
        );
        assert!(verdict.findings[0].message.contains("missing"), "{file}");
    }
    Ok(())
}

#[test]
fn a_manifest_that_is_not_toml_is_a_finding() -> TestResult {
    let dir = copy_fixture(None)?;
    fs::write(
        dir.path().join("crates").join("a").join("Cargo.toml"),
        "[package\nname = \"a\"\n",
    )?;
    let verdict = lint(dir.path())?;
    let message = only_finding(&verdict, "crates/a/Cargo.toml");
    assert!(message.contains("line 1"), "{message}");
    assert!(!message.contains("name"), "{message}");
    Ok(())
}

#[test]
fn bad_attributes_in_sources_fail() -> TestResult {
    let verdict = lint_case("bad-attributes")?;
    assert_eq!(
        places(&verdict),
        [
            (Some("crates/a/src/lib.rs"), Some(3)),
            (Some("crates/a/src/lib.rs"), Some(5)),
            (Some("crates/a/src/lib.rs"), Some(8)),
            (Some("crates/a/src/lib.rs"), Some(12)),
        ]
    );

    // The same file under `tests/fixtures/` is test data and is not read.
    let dir = copy_fixture(None)?;
    let folder = dir
        .path()
        .join("crates")
        .join("a")
        .join("tests")
        .join("fixtures");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("lints")
            .join("bad-attributes")
            .join("crates")
            .join("a")
            .join("src")
            .join("lib.rs"),
        folder.join("more.rs"),
    )?;
    assert!(lint(dir.path())?.passed());
    Ok(())
}

/// `cargo xtask lints` in `root`, outside GitHub Actions.
fn xtask_lints(root: &Path) -> TestResult<Output> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command
        .arg("--root")
        .arg(root)
        .arg("lints")
        // A real CI run must not make this one write annotations or a step summary.
        .env_remove("GITHUB_ACTIONS")
        .env_remove("GITHUB_STEP_SUMMARY");
    Ok(command.output()?)
}

#[test]
fn the_command_passes_a_clean_repository_and_fails_a_dirty_one() -> TestResult {
    let clean = copy_fixture(None)?;
    let pass = xtask_lints(clean.path())?;
    let table = String::from_utf8_lossy(&pass.stdout);
    assert_eq!(pass.status.code(), Some(0), "{pass:?}");
    assert!(table.contains("pass  lints"), "{table}");

    let dirty = copy_fixture(Some("bad-attributes"))?;
    let fail = xtask_lints(dirty.path())?;
    let table = String::from_utf8_lossy(&fail.stdout);
    assert_eq!(fail.status.code(), Some(1), "{fail:?}");
    assert!(table.contains("FAIL  lints"), "{table}");
    assert!(table.contains("crates/a/src/lib.rs:5: "), "{table}");
    Ok(())
}
