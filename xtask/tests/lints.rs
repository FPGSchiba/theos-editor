//! `lints` over fixture repositories (`tests/fixtures/lints/<case>/`).
//!
//! `clean` is a small repository that passes. Every other case holds only the files that differ
//! from it: a test copies `clean` to a temporary directory, lays the case over it and checks that
//! copy, where git does not apply, so the files are listed by a walk. In this repository `lints`
//! leaves everything under `tests/fixtures/` alone, which is why the cases may hold files that
//! the check forbids elsewhere.

use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use xtask::checks::lints::{self, config_files};
use xtask::files::repo_files;
use xtask::verdict::{Check, Verdict};
use xtask::view::{View, ViewOptions};

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

/// The verdict of `lints` on the repository at `root`, over the view of the copy.
fn lint(root: &Path) -> TestResult<Verdict> {
    lint_listing(root, &[])
}

/// The verdict of the configuration-file rule alone, which needs no view.
fn config(root: &Path) -> TestResult<Verdict> {
    let findings = config_files::findings(root, &repo_files(root)?)?;
    Ok(Verdict::with(Check::Lints, findings))
}

/// The verdict of `lints` on a copy of `clean` with the case laid over it.
fn lint_case(case: &str) -> TestResult<Verdict> {
    lint(copy_fixture(Some(case))?.path())
}

/// The verdict of the configuration-file rule on a copy of `clean` with the case laid over it.
fn config_case(case: &str) -> TestResult<Verdict> {
    config(copy_fixture(Some(case))?.path())
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
    let verdict = config_case("crate-without-workspace-lints")?;
    let message = only_finding(&verdict, "crates/a/Cargo.toml");
    assert!(message.contains("workspace = true"), "{message}");
    Ok(())
}

#[test]
fn nested_clippy_toml_fails() -> TestResult {
    let verdict = config_case("nested-clippy-toml")?;
    only_finding(&verdict, "crates/a/clippy.toml");
    Ok(())
}

#[test]
fn rustfmt_toml_fails() -> TestResult {
    let verdict = config_case("rustfmt-toml")?;
    only_finding(&verdict, "rustfmt.toml");
    Ok(())
}

#[test]
fn crate_level_cargo_dir_fails() -> TestResult {
    let verdict = config_case("crate-level-cargo-dir")?;
    only_finding(&verdict, "crates/a/.cargo");
    Ok(())
}

#[test]
fn mirror_drift_fails() -> TestResult {
    let verdict = config_case("mirror-drift")?;
    let message = only_finding(&verdict, "clippy.toml");
    assert!(message.contains("gate/clippy.toml"), "{message}");

    let dir = copy_fixture(None)?;
    fs::write(
        dir.path().join(".config").join("nextest.toml"),
        "[profile.default]\nretries = 2\n",
    )?;
    let verdict = config(dir.path())?;
    let message = only_finding(&verdict, ".config/nextest.toml");
    assert!(message.contains("gate/nextest.toml"), "{message}");
    Ok(())
}

#[test]
fn lints_table_drift_fails() -> TestResult {
    let verdict = config_case("lints-table-drift")?;
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
    let verdict = config(dir.path())?;
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
        let verdict = config(dir.path())?;
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
    let verdict = config(dir.path())?;
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

/// Writes `text` to the file `relative` (joined with `/`) below `root`, and the folders it needs.
fn write(root: &Path, relative: &str, text: &str) -> TestResult {
    let path = root.join(relative);
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(path, text)?;
    Ok(())
}

/// The verdict of `lints` on the repository at `root`, with the files `extra` listed as well:
/// the walk that lists a copy leaves out every folder called `target`.
fn lint_listing(root: &Path, extra: &[&str]) -> TestResult<Verdict> {
    let mut files = repo_files(root)?;
    files.extend(extra.iter().map(|file| (*file).to_owned()));
    let view = View::build(root, ViewOptions::FIXTURE)?;
    Ok(lints::check(root, &files, &view)?)
}

#[test]
fn source_in_a_folder_named_target_or_node_modules_is_scanned() -> TestResult {
    // `mod target;` builds `src/target/mod.rs`, so it is source however it is called.
    let allow = "#[allow(dead_code)]\nfn f() {}\n";
    let dir = copy_fixture(None)?;
    write(dir.path(), "crates/a/src/target/x.rs", allow)?;
    write(dir.path(), "crates/a/src/node_modules/x.rs", allow)?;
    let verdict = lint_listing(
        dir.path(),
        &["crates/a/src/target/x.rs", "crates/a/src/node_modules/x.rs"],
    )?;
    assert_eq!(
        places(&verdict),
        [
            (Some("crates/a/src/target/x.rs"), Some(1)),
            (Some("crates/a/src/node_modules/x.rs"), Some(1)),
        ]
    );

    // The build folder of the root is the only `target` that is left alone.
    let dir = copy_fixture(None)?;
    write(dir.path(), "target/debug/build/x.rs", allow)?;
    let verdict = lint_listing(dir.path(), &["target/debug/build/x.rs"])?;
    assert!(verdict.passed(), "{:?}", verdict.findings);
    Ok(())
}

#[test]
fn manifest_path_into_fixtures_fails() -> TestResult {
    let verdict = lint_case("manifest-path-into-fixtures")?;
    let message = only_finding(&verdict, "crates/a/Cargo.toml");
    assert!(message.contains("tests/fixtures/"), "{message}");
    Ok(())
}

#[test]
fn a_main_rs_or_mod_rs_directly_in_fixtures_fails() -> TestResult {
    // Cargo builds `tests/fixtures/main.rs` as a test called `fixtures`, and `mod fixtures;` in
    // `tests/` builds `tests/fixtures/mod.rs`. The files are written here, because in this
    // repository they would be findings of their own.
    for name in ["main.rs", "mod.rs"] {
        let dir = copy_fixture(None)?;
        let file = format!("crates/a/tests/fixtures/{name}");
        write(dir.path(), &file, "fn main() {}\n")?;
        let verdict = lint(dir.path())?;
        only_finding(&verdict, &file);

        // One folder deeper, where a case of the fixtures lives, nothing builds them.
        let dir = copy_fixture(None)?;
        let deeper = format!("crates/a/tests/fixtures/case/{name}");
        write(dir.path(), &deeper, "fn main() {}\n")?;
        let verdict = lint(dir.path())?;
        assert!(verdict.passed(), "{name}: {:?}", verdict.findings);
    }
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

/// Lays `lib_rs` over the `src/lib.rs` of the crate `a` of `clean`, with code outside `src/`
/// that it reaches, and checks the copy.
fn lint_lib(lib_rs: &str) -> TestResult<Verdict> {
    let dir = copy_fixture(None)?;
    write(dir.path(), "crates/a/src/lib.rs", lib_rs)?;
    write(dir.path(), "crates/a/lib/real.rs", "pub fn real() {}\n")?;
    write(dir.path(), "crates/a/tests/helpers.rs", "pub fn h() {}\n")?;
    write(dir.path(), "crates/a/src/inner.rs", "pub fn inner() {}\n")?;
    write(dir.path(), "crates/a/data.txt", "some data\n")?;
    lint(dir.path())
}

// AC: CE-08.8
#[test]
fn code_reached_from_src_but_outside_it_fails() -> TestResult {
    // The floor of `coverage` counts `crates/<name>/src/**` only, so code that a library builds
    // from anywhere else is out of its reach: rule 1 reports the file that is compiled.
    for (lib_rs, file) in [
        (
            "#[path = \"../lib/real.rs\"]\nmod real;\n",
            "crates/a/lib/real.rs",
        ),
        (
            "#[path = \"../tests/helpers.rs\"]\nmod helpers;\n",
            "crates/a/tests/helpers.rs",
        ),
        (
            "#[path = \"../../a/lib/real.rs\"]\nmod real;\n",
            "crates/a/lib/real.rs",
        ),
        ("include!(\"../lib/real.rs\");\n", "crates/a/lib/real.rs"),
        (
            "include!(\"../tests/helpers.rs\");\n",
            "crates/a/tests/helpers.rs",
        ),
        (
            "include!(\"../src/../lib/real.rs\");\n",
            "crates/a/lib/real.rs",
        ),
    ] {
        let verdict = lint_lib(lib_rs)?;
        assert_eq!(
            found_with(&verdict, OUTSIDE_SRC),
            [(Some(file), None)],
            "{lib_rs}"
        );
    }
    Ok(())
}

// AC: CE-08.8
#[test]
fn code_reached_from_src_inside_it_passes() -> TestResult {
    // An `include!` is a finding even inside `src/` (`include_in_library_code_fails`); `#[path]`
    // inside `src/` and data macros are fine.
    for lib_rs in [
        "#[path = \"inner.rs\"]\nmod renamed;\n",
        "#[path = \"../src/inner.rs\"]\nmod renamed;\n",
        "const A: &str = include_str!(\"../data.txt\");\nconst B: &[u8] = include_bytes!(\"../data.txt\");\n",
    ] {
        let verdict = lint_lib(lib_rs)?;
        assert!(verdict.passed(), "{lib_rs}: {:?}", verdict.findings);
    }
    Ok(())
}

// AC: CE-08.8
#[test]
fn xtask_source_leaving_src_fails() -> TestResult {
    // `xtask` is a member of the copy, so the view reaches it: rule 1 reports the file that
    // `include!` compiles from outside `xtask/src/` and the `include!` itself.
    let dir = copy_fixture(None)?;
    let root_manifest = dir.path().join("Cargo.toml");
    let text = fs::read_to_string(&root_manifest)?;
    fs::write(
        &root_manifest,
        text.replace(
            "members = [\"crates/*\"]",
            "members = [\"crates/*\", \"xtask\"]",
        ),
    )?;
    write(
        dir.path(),
        "xtask/Cargo.toml",
        "[package]\nname = \"xtask\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lints]\nworkspace = true\n",
    )?;
    write(
        dir.path(),
        "xtask/src/main.rs",
        "include!(\"../real.rs\");\n",
    )?;
    write(dir.path(), "xtask/real.rs", "fn main() {}\n")?;
    let verdict = lint(dir.path())?;
    assert_eq!(
        found_with(&verdict, OUTSIDE_SRC),
        [(Some("xtask/real.rs"), None)]
    );
    assert_eq!(
        found_with(&verdict, "`include!` in"),
        [(Some("xtask/src/main.rs"), Some(1))]
    );
    Ok(())
}

/// The verdict for the crate `a` of `clean` with `extra` added to its manifest and the Rust files
/// `files` (below `crates/a/`) written, so that cargo can check the copy.
fn lint_manifest(extra: &str, files: &[&str]) -> TestResult<Verdict> {
    let dir = copy_fixture(None)?;
    let manifest = dir.path().join("crates").join("a").join("Cargo.toml");
    let text = fs::read_to_string(&manifest)?;
    fs::write(&manifest, format!("{text}\n{extra}\n"))?;
    for file in files {
        write(dir.path(), &format!("crates/a/{file}"), "fn main() {}\n")?;
    }
    lint(dir.path())
}

// AC: CE-08.8
#[test]
fn manifest_target_outside_src_fails() -> TestResult {
    // Rule 1 reports the file that the target compiles, not the manifest.
    for (extra, file) in [
        ("[lib]\npath = \"lib/lib.rs\"\n", "lib/lib.rs"),
        ("[lib]\npath = \"lib.rs\"\n", "lib.rs"),
        ("[lib]\npath = \"src/../lib/lib.rs\"\n", "lib/lib.rs"),
        ("[[bin]]\nname = \"x\"\npath = \"bin/x.rs\"\n", "bin/x.rs"),
        (
            "[[bin]]\nname = \"y\"\npath = \"src/y.rs\"\n\n[[bin]]\nname = \"x\"\npath = \"tests/x.rs\"\n",
            "tests/x.rs",
        ),
    ] {
        let verdict = lint_manifest(extra, &[file, "src/y.rs"])?;
        assert_eq!(
            found_with(&verdict, OUTSIDE_SRC),
            [(Some(format!("crates/a/{file}").as_str()), None)],
            "{extra}"
        );
    }
    Ok(())
}

// AC: CE-08.8
#[test]
fn manifest_target_inside_src_or_not_measured_passes() -> TestResult {
    for (extra, file) in [
        ("[lib]\npath = \"src/lib.rs\"\n", "src/lib.rs"),
        ("[lib]\npath = \"src/core/lib.rs\"\n", "src/core/lib.rs"),
        (
            "[[bin]]\nname = \"x\"\npath = \"src/bin/x.rs\"\n",
            "src/bin/x.rs",
        ),
        ("[lib]\nname = \"a\"\n", "src/lib.rs"),
        // Tests, benches and examples are not measured code.
        (
            "[[test]]\nname = \"t\"\npath = \"tests/t.rs\"\n",
            "tests/t.rs",
        ),
        (
            "[[bench]]\nname = \"b\"\npath = \"benches/b.rs\"\nharness = false\n",
            "benches/b.rs",
        ),
        (
            "[[example]]\nname = \"e\"\npath = \"examples/e.rs\"\n",
            "examples/e.rs",
        ),
    ] {
        let verdict = lint_manifest(extra, &[file])?;
        assert!(verdict.passed(), "{extra}: {:?}", verdict.findings);
    }
    Ok(())
}

// AC: CE-08.8
#[test]
fn a_legacy_rust_toolchain_file_fails() -> TestResult {
    // rustup reads `rust-toolchain` before the protected `rust-toolchain.toml`.
    for (relative, flagged) in [
        ("rust-toolchain", "rust-toolchain"),
        ("crates/a/rust-toolchain", "crates/a/rust-toolchain"),
        ("Rust-Toolchain", "Rust-Toolchain"),
    ] {
        let dir = copy_fixture(None)?;
        write(dir.path(), relative, "nightly\n")?;
        let verdict = config(dir.path())?;
        let message = only_finding(&verdict, flagged);
        assert!(message.contains("rust-toolchain.toml"), "{message}");
    }

    let verdict = config_case("legacy-rust-toolchain")?;
    only_finding(&verdict, "rust-toolchain");
    Ok(())
}

// AC: CE-08.8
#[test]
fn rust_toolchain_toml_and_fixture_copies_pass() -> TestResult {
    let dir = copy_fixture(None)?;
    write(
        dir.path(),
        "rust-toolchain.toml",
        "[toolchain]\nchannel = \"1.99.1\"\n",
    )?;
    write(
        dir.path(),
        "crates/a/tests/fixtures/x/rust-toolchain",
        "nightly\n",
    )?;
    write(dir.path(), "tests/fixtures/rust-toolchain", "nightly\n")?;
    let verdict = config(dir.path())?;
    assert!(verdict.passed(), "{:?}", verdict.findings);
    Ok(())
}

/// The verdict for the crate `a` of `clean` with `lib_rs` as its `src/lib.rs`.
fn lint_source(lib_rs: &str) -> TestResult<Verdict> {
    let dir = copy_fixture(None)?;
    write(dir.path(), "crates/a/src/lib.rs", lib_rs)?;
    lint(dir.path())
}

// AC: CE-08.8
#[test]
fn a_cfg_that_hides_code_from_coverage_fails() -> TestResult {
    // cargo-llvm-cov builds with `--cfg coverage`: code under `not(coverage)` would be built and
    // shipped, but never counted against the floor. Rule 2 refuses the names.
    for lib_rs in [
        "#[cfg(not(coverage))]\npub fn f() {}\n",
        "#[cfg(any(test, coverage_nightly))]\npub fn f() {}\n",
        "#[cfg_attr(coverage, derive(Debug))]\npub struct S;\n",
        "#![cfg(not(coverage))]\npub fn f() {}\n",
        "pub fn a() {} #[cfg(not(coverage))] pub fn f() {}\n",
        "#[cfg(\n    not(coverage)\n)]\npub fn f() {}\n",
    ] {
        let verdict = lint_source(lib_rs)?;
        let found = cfg_messages(&verdict);
        assert_eq!(found.len(), 1, "{lib_rs}: {:?}", verdict.findings);
        assert!(found[0].contains("`coverage"), "{lib_rs}: {}", found[0]);
    }

    let verdict = lint_case("coverage-cfg")?;
    assert_eq!(
        found_with(&verdict, "cfg-names.toml"),
        [(Some("crates/a/src/lib.rs"), Some(3))]
    );
    Ok(())
}

// AC: CE-08.8
#[test]
fn the_coverage_attribute_fails_the_check() -> TestResult {
    // `#![coverage(off)]` is unstable, so rustc refuses it on stable and the view cannot be
    // built: the check must not pass.
    assert_view_refused("#![coverage(off)]\npub fn f() {}\n")
}

/// Asserts that `lib_rs` as the library of `clean` stops the view: the error is the failed
/// `cargo check`, and no verdict says the code passes.
fn assert_view_refused(lib_rs: &str) -> TestResult {
    let error = lint_source(lib_rs)
        .err()
        .ok_or_else(|| format!("passed: {lib_rs}"))?;
    let error = error
        .downcast_ref::<xtask::Error>()
        .ok_or_else(|| format!("not an xtask error: {error}"))?;
    let xtask::Error::Command { program, args, .. } = error else {
        return Err(format!("not a failed command: {error}").into());
    };
    assert_eq!(program, "cargo", "{lib_rs}");
    assert_eq!(args.first().map(String::as_str), Some("check"), "{lib_rs}");
    Ok(())
}

// AC: CE-08.8
#[test]
fn cfgs_that_keep_code_measured_pass() -> TestResult {
    for lib_rs in [
        "#[cfg(not(test))]\npub fn f() {}\n",
        "#[cfg(test)]\nmod tests {}\n",
        "/// Do not use `#[cfg(not(coverage))]` here.\npub fn f() {}\n",
        "pub const S: &str = \"#[cfg(coverage)]\";\n",
        "pub fn f() -> bool { cfg!(coverage) }\n",
    ] {
        let verdict = lint_source(lib_rs)?;
        assert!(verdict.passed(), "{lib_rs}: {:?}", verdict.findings);
    }
    Ok(())
}

// AC: CE-08.8
#[test]
fn an_attribute_anywhere_in_a_source_is_judged() -> TestResult {
    // Each of these once escaped a scan that read attributes only at the start of a line.
    for lib_rs in [
        "#[rustfmt::skip] pub mod m { #![allow(clippy::unwrap_used)] pub fn f() {} }
",
        "pub fn a() {} #[allow(dead_code)] pub fn b() {}
",
        "pub fn a() { #[allow(unused)] let _x = 1; }
",
        "macro_rules! m { () => { #[allow(dead_code)] fn x() {} } }
",
        "#[allow(
    dead_code,
    unused
)]
pub fn f() {}
",
        "#[cfg_attr(test, allow(dead_code))]
pub fn f() {}
",
        "#[test] pub fn a() {} #[ignore] #[test] pub fn b() {}
",
        "#[test] #[expect(dead_code)] pub fn c() {}
",
        "/* /* */ \" */ #[allow(dead_code)] pub fn f() {} // \"
",
    ] {
        let verdict = lint_source(lib_rs)?;
        assert_eq!(
            verdict.findings.len(),
            1,
            "{lib_rs}: {:?}",
            verdict.findings
        );
        assert_eq!(
            verdict.findings[0].file.as_deref(),
            Some("crates/a/src/lib.rs")
        );
    }
    Ok(())
}

// AC: CE-08.8
#[test]
fn attribute_like_text_in_strings_and_comments_passes() -> TestResult {
    for lib_rs in [
        "pub const S: &str = \"#[allow(dead_code)]\";
",
        "pub const S: &str = r#\"#![allow(dead_code)]\"#;
",
        "// #[allow(dead_code)]
pub fn f() {}
",
        "/* #[allow(dead_code)] */ pub fn f() {}
",
        "/// #[allow(dead_code)]
pub fn f() {}
",
        "#[rustfmt::skip] pub mod m { pub fn f() {} }
",
    ] {
        let verdict = lint_source(lib_rs)?;
        assert!(verdict.passed(), "{lib_rs}: {:?}", verdict.findings);
    }
    Ok(())
}

// AC: CE-08.8
#[test]
fn a_path_attribute_after_code_is_judged_by_rule_1() -> TestResult {
    // The `#[path]` is in the middle of a line; the file it names exists, so cargo checks it.
    let verdict = lint_lib("pub fn a() {} #[path = \"../lib/real.rs\"] mod x;\n")?;
    assert_eq!(
        found_with(&verdict, OUTSIDE_SRC),
        [(Some("crates/a/lib/real.rs"), None)]
    );
    let verdict = lint_lib("pub fn a() {} #[path = \"inner.rs\"] mod x;\n")?;
    assert!(verdict.passed(), "{:?}", verdict.findings);
    Ok(())
}

// AC: CE-08.8
#[test]
fn a_cfg_outside_the_allowlist_fails() -> TestResult {
    // Each name is set by a tool or a profile, so code under it is not compiled by every check:
    // `clippy` is set only by clippy, the profile cfgs only by some profiles, and a `build.rs`
    // can set any name.
    for (lib_rs, name) in [
        (
            "#[cfg(not(clippy))]
pub fn f() {}
",
            "clippy",
        ),
        (
            "#[cfg(debug_assertions)]
pub fn f() {}
",
            "debug_assertions",
        ),
        (
            "#[cfg(any(test, miri))]
pub fn f() {}
",
            "miri",
        ),
        (
            "#[cfg(my_flag)]
pub fn f() {}
",
            "my_flag",
        ),
        (
            "#[cfg(panic = \"abort\")]
pub fn f() {}
",
            "panic",
        ),
        (
            "#![cfg(not(coverage))]
pub fn f() {}
",
            "coverage",
        ),
        (
            "pub fn a() {} #[cfg(not(clippy))] pub fn f() {}
",
            "clippy",
        ),
        (
            "#[cfg_attr(test, cfg(not(clippy)))]
pub fn f() {}
",
            "clippy",
        ),
        (
            "#[cfg(not(test\u{e9}))]
pub fn f() {}
",
            "test?",
        ),
        (
            "#[cfg(t\u{435}st)]
pub fn f() {}
",
            "t?st",
        ),
        (
            "#[cfg(target_os\u{e9} = \"x\")]
pub fn f() {}
",
            "target_os?",
        ),
        (
            "#[cfg(doc\u{e9})]
pub fn f() {}
",
            "doc?",
        ),
    ] {
        let verdict = lint_source(lib_rs)?;
        let found = cfg_messages(&verdict);
        assert_eq!(found.len(), 1, "{lib_rs}: {:?}", verdict.findings);
        let message = found[0];
        assert!(
            message.contains(&format!("`{name}`")),
            "{lib_rs}: {message}"
        );
        assert!(message.contains("gate change"), "{lib_rs}: {message}");
    }
    Ok(())
}

// AC: CE-08.8
#[test]
fn a_cfg_inside_the_allowlist_passes() -> TestResult {
    for lib_rs in [
        "#[cfg(test)]
mod tests {}
",
        "#[cfg(not(test))]
pub fn f() {}
",
        "#[cfg(all(unix, feature = \"x\"))]
pub fn f() {}
",
        "#[cfg(target_os = \"windows\")]
pub fn f() {}
",
        "#[cfg(doc)]
pub fn f() {}
",
        "#[cfg(any(windows, all(not(unix), target_pointer_width = \"64\")))]
pub fn f() {}
",
        "#[cfg_attr(test, derive(Debug))]
pub struct S;
",
        "#[cfg_attr(feature = \"x\", derive(Debug), derive(Clone))]
pub struct S;
",
    ] {
        let verdict = lint_source(lib_rs)?;
        assert!(verdict.passed(), "{lib_rs}: {:?}", verdict.findings);
    }
    Ok(())
}

#[test]
fn the_gate_guards_the_lints_that_guard_the_others() -> TestResult {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("gate")
        .join("lints.toml");
    let table: toml::Table = fs::read_to_string(path)?.parse()?;
    for (section, lint, expected) in [
        ("rust", "unexpected_cfgs", "forbid"),
        ("clippy", "allow_attributes", "deny"),
        ("clippy", "allow_attributes_without_reason", "deny"),
        ("clippy", "ignore_without_reason", "deny"),
    ] {
        let level = table
            .get(section)
            .and_then(|s| s.get(lint))
            .and_then(toml::Value::as_str);
        assert_eq!(level, Some(expected), "{section}.{lint}");
    }
    Ok(())
}

/// The findings of the verdict whose message contains `needle`, as file and line.
fn found_with<'a>(verdict: &'a Verdict, needle: &str) -> Vec<(Option<&'a str>, Option<u32>)> {
    verdict
        .findings
        .iter()
        .filter(|finding| finding.message.contains(needle))
        .map(|finding| (finding.file.as_deref(), finding.line))
        .collect()
}

/// What rule 1 says of a file compiled from outside `src/`.
const OUTSIDE_SRC: &str = "from outside its `src/`";

// AC: CE-08.8
#[test]
fn library_code_outside_src_fails() -> TestResult {
    let verdict = lint_case("outside-src")?;
    assert_eq!(
        found_with(&verdict, OUTSIDE_SRC),
        [
            (Some("crates/a/tests/helper.rs"), None),
            (Some("crates/a/tools/main.rs"), None),
        ]
    );
    let message = &verdict.findings[0].message;
    assert!(message.contains("library `a` of `a`"), "{message}");
    Ok(())
}

// AC: CE-08.8
#[test]
fn test_targets_may_compile_anything() -> TestResult {
    let dir = copy_fixture(Some("outside-src"))?;
    write(
        dir.path(),
        "crates/a/tests/it.rs",
        "#[path = \"fixtures/case/lib.rs\"]\nmod case;\n",
    )?;
    write(
        dir.path(),
        "crates/a/tests/fixtures/case/lib.rs",
        "pub fn c() {}\n",
    )?;
    let verdict = lint(dir.path())?;
    let outside = found_with(&verdict, OUTSIDE_SRC);
    assert!(
        outside
            .iter()
            .all(|(file, _)| *file != Some("crates/a/tests/fixtures/case/lib.rs")),
        "{outside:?}"
    );
    assert_eq!(outside.len(), 2, "{outside:?}");
    Ok(())
}

// AC: CE-08.8
#[test]
fn include_in_library_code_fails() -> TestResult {
    let verdict = lint_case("include-in-src")?;
    // The `include!` is on line 3; the `include_str!` on line 6 reads data.
    assert_eq!(
        found_with(&verdict, "`include!` in"),
        [(Some("crates/a/src/lib.rs"), Some(3))]
    );
    assert_eq!(found_with(&verdict, OUTSIDE_SRC), []);
    Ok(())
}

// AC: CE-08.8
#[test]
fn a_build_script_compiles_from_build_rs() -> TestResult {
    let dir = copy_fixture(None)?;
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("view")
            .join("plain"),
        dir.path(),
    )?;
    let verdict = lint(dir.path())?;
    assert_eq!(found_with(&verdict, OUTSIDE_SRC), []);
    assert_eq!(found_with(&verdict, "`include!` in"), []);
    Ok(())
}

// AC: CE-08.8
#[test]
fn refused_cfg_names_fail_with_their_line() -> TestResult {
    let verdict = lint_case("bad-cfg")?;
    let found: Vec<_> = verdict
        .findings
        .iter()
        .filter(|finding| finding.message.contains("cfg-names.toml"))
        .collect();
    let words = [
        "clippy",
        "debug_assertions",
        "miri",
        "coverage",
        "docsrs",
        "target_has_atomic_equal_alignment",
        "false",
        "clippy",
        "clippy",
    ];
    assert_eq!(found.len(), words.len(), "{found:?}");
    for (index, (finding, word)) in found.iter().zip(words).enumerate() {
        let line = u32::try_from(3 + 2 * index)?;
        assert_eq!(finding.file.as_deref(), Some("crates/a/src/lib.rs"));
        assert_eq!(finding.line, Some(line), "{finding:?}");
        assert!(
            finding.message.contains(&format!("`{word}`")),
            "{finding:?}"
        );
    }
    Ok(())
}

// AC: CE-08.8
#[test]
fn cfg_names_toml_is_what_the_pinned_rustc_prints() -> TestResult {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let rustc = xtask::process::rustc_program(
        std::env::var_os("CARGO").as_deref(),
        std::env::var_os("RUSTC").as_deref(),
    );
    let mut keys = BTreeSet::new();
    for target in [
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
    ] {
        let output = Command::new(&rustc)
            .args(["--print", "cfg", "--target", target])
            .output()?;
        assert!(output.status.success(), "{target}: {output:?}");
        let text = String::from_utf8(output.stdout)?;
        keys.extend(
            text.lines()
                .map(|line| line.split('=').next().unwrap_or_default().to_owned()),
        );
    }
    for own in ["debug_assertions", "panic", "unix", "windows"] {
        keys.remove(own);
    }
    keys.insert("feature".to_owned());

    let loaded = lints::CfgNames::load(&repo)?;
    assert_eq!(loaded.keys, keys);
    let names: BTreeSet<String> = ["test", "doc", "unix", "windows"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(loaded.names, names);
    Ok(())
}

/// The messages of the findings of rule 2 (cfg names).
fn cfg_messages(verdict: &Verdict) -> Vec<&str> {
    verdict
        .findings
        .iter()
        .map(|finding| finding.message.as_str())
        .filter(|message| message.contains("cfg-names.toml"))
        .collect()
}

// AC: CE-08.8
#[test]
fn cfg_names_that_stable_rustc_refuses_fail_the_check() -> TestResult {
    // `overflow_checks` and `ub_checks` are unstable cfgs: rustc refuses them on stable, so the
    // view cannot be built and the check must not pass.
    for lib_rs in [
        "#[cfg_attr(overflow_checks, inline)]\npub fn f() {}\n",
        "#[cfg(ub_checks)]\npub fn f() {}\n",
    ] {
        assert_view_refused(lib_rs)?;
    }
    Ok(())
}

// AC: CE-08.8
#[test]
fn a_target_outside_the_repository_is_reported_at_its_absolute_path() -> TestResult {
    // Ruling 4: a file outside the root keeps its absolute normal form, with `/` separators.
    let outside = tempfile::tempdir()?;
    write(outside.path(), "lib.rs", "pub fn f() {}\n")?;
    let absolute = outside.path().join("lib.rs");
    let manifest_path = absolute.to_string_lossy().into_owned();
    let verdict = lint_manifest(&format!("[lib]\npath = '{manifest_path}'\n"), &[])?;
    let found = found_with(&verdict, OUTSIDE_SRC);
    let [(Some(file), None)] = found.as_slice() else {
        return Err(format!("expected one finding at the file: {found:?}").into());
    };
    let expected = manifest_path.replace('\\', "/");
    let expected = expected.trim_start_matches("//?/");
    assert!(
        file.eq_ignore_ascii_case(expected),
        "{file} against {expected}"
    );
    Ok(())
}

// AC: CE-08.8
#[test]
fn a_backslash_path_never_passes() -> TestResult {
    // A `\` separates folders in a module path on Windows only. There rule 1 reports the file
    // outside `src/`; elsewhere the file does not exist, and the view stops.
    let lib_rs = "#[path = \"..\\\\lib\\\\real.rs\"]\nmod real;\n";
    if cfg!(windows) {
        let verdict = lint_lib(lib_rs)?;
        assert_eq!(
            found_with(&verdict, OUTSIDE_SRC),
            [(Some("crates/a/lib/real.rs"), None)]
        );
        Ok(())
    } else {
        let dir = copy_fixture(None)?;
        write(dir.path(), "crates/a/src/lib.rs", lib_rs)?;
        write(dir.path(), "crates/a/lib/real.rs", "pub fn real() {}\n")?;
        assert!(lint(dir.path()).is_err());
        Ok(())
    }
}

#[test]
fn the_fixture_copy_of_cfg_names_is_the_real_file() -> TestResult {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let real = fs::read_to_string(manifest.join("..").join("gate").join("cfg-names.toml"))?;
    let copy = fs::read_to_string(
        manifest
            .join("tests")
            .join("fixtures")
            .join("lints")
            .join("clean")
            .join("gate")
            .join("cfg-names.toml"),
    )?;
    assert_eq!(real.replace("\r\n", "\n"), copy.replace("\r\n", "\n"));
    Ok(())
}
