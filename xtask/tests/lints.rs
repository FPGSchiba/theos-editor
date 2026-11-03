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
    Ok(lints::check(root, &files)?)
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
fn fixture_compiled_through_path_fails() -> TestResult {
    // `#[path]` on line 3 and `include!` on line 6; `include_str!` on line 9 is data.
    let verdict = lint_case("fixture-compiled-through-path")?;
    assert_eq!(
        places(&verdict),
        [
            (Some("crates/a/src/lib.rs"), Some(3)),
            (Some("crates/a/src/lib.rs"), Some(6)),
        ]
    );
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

#[test]
fn a_mod_fixtures_declaration_fails() -> TestResult {
    let dir = copy_fixture(None)?;
    write(
        dir.path(),
        "crates/a/tests/it.rs",
        "mod helpers;\nmod fixtures;\n",
    )?;
    let verdict = lint(dir.path())?;
    assert_eq!(places(&verdict), [(Some("crates/a/tests/it.rs"), Some(2))]);
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
    lint(dir.path())
}

// AC: CE-08.8
#[test]
fn code_reached_from_src_but_outside_it_fails() -> TestResult {
    // The floor of `coverage` counts `crates/<name>/src/**` only, so code that a crate builds
    // from anywhere else would be out of its reach: `#[path]` and `include!` stay in `src/`.
    for lib_rs in [
        "#[path = \"../lib/real.rs\"]\nmod real;\n",
        "#[path = \"../tests/helpers.rs\"]\nmod helpers;\n",
        "#[path = \"..\\\\lib\\\\real.rs\"]\nmod real;\n",
        "#[path = \"../../a/lib/real.rs\"]\nmod real;\n",
        "include!(\"../lib/real.rs\");\n",
        "include!(\"../tests/helpers.rs\");\n",
        "include!(\"../src/../lib/real.rs\");\n",
    ] {
        let verdict = lint_lib(lib_rs)?;
        assert_eq!(
            places(&verdict),
            [(Some("crates/a/src/lib.rs"), Some(1))],
            "{lib_rs}"
        );
        let message = &verdict.findings[0].message;
        assert!(message.contains("src/"), "{lib_rs}: {message}");
    }
    Ok(())
}

// AC: CE-08.8
#[test]
fn code_reached_from_src_inside_it_passes() -> TestResult {
    for lib_rs in [
        "#[path = \"inner.rs\"]\nmod renamed;\n",
        "#[path = \"../src/inner.rs\"]\nmod renamed;\n",
        "include!(\"inner.rs\");\n",
        "include!(\"../../a/src/inner.rs\");\n",
        // Data, not code: it stays allowed.
        "const A: &str = include_str!(\"../lib/real.rs\");\nconst B: &[u8] = include_bytes!(\"../tests/helpers.rs\");\n",
    ] {
        let verdict = lint_lib(lib_rs)?;
        assert!(verdict.passed(), "{lib_rs}: {:?}", verdict.findings);
    }
    Ok(())
}

// AC: CE-08.8
#[test]
fn xtask_source_leaving_src_fails() -> TestResult {
    let dir = copy_fixture(None)?;
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
    assert_eq!(places(&verdict), [(Some("xtask/src/main.rs"), Some(1))]);
    Ok(())
}

/// The verdict for the crate `a` of `clean` with `extra` added to its manifest.
fn lint_manifest(extra: &str) -> TestResult<Verdict> {
    let dir = copy_fixture(None)?;
    let manifest = dir.path().join("crates").join("a").join("Cargo.toml");
    let text = fs::read_to_string(&manifest)?;
    fs::write(&manifest, format!("{text}\n{extra}\n"))?;
    lint(dir.path())
}

// AC: CE-08.8
#[test]
fn manifest_target_outside_src_fails() -> TestResult {
    for extra in [
        "[lib]\npath = \"lib/lib.rs\"\n",
        "[lib]\npath = \"lib.rs\"\n",
        "[lib]\npath = \"src/../lib/lib.rs\"\n",
        "[[bin]]\nname = \"x\"\npath = \"bin/x.rs\"\n",
        "[[bin]]\nname = \"y\"\npath = \"src/y.rs\"\n\n[[bin]]\nname = \"x\"\npath = \"tests/x.rs\"\n",
        "[lib]\npath = \"/abs/lib.rs\"\n",
    ] {
        let verdict = lint_manifest(extra)?;
        let message = only_finding(&verdict, "crates/a/Cargo.toml");
        assert!(message.contains("src/"), "{extra}: {message}");
    }
    Ok(())
}

// AC: CE-08.8
#[test]
fn manifest_target_inside_src_or_not_measured_passes() -> TestResult {
    for extra in [
        "[lib]\npath = \"src/lib.rs\"\n",
        "[lib]\npath = \"src/core/lib.rs\"\n",
        "[[bin]]\nname = \"x\"\npath = \"src/bin/x.rs\"\n",
        "[lib]\nname = \"a\"\n",
        // Tests, benches and examples are not measured code.
        "[[test]]\nname = \"t\"\npath = \"tests/t.rs\"\n",
        "[[bench]]\nname = \"b\"\npath = \"benches/b.rs\"\n",
        "[[example]]\nname = \"e\"\npath = \"examples/e.rs\"\n",
    ] {
        let verdict = lint_manifest(extra)?;
        assert!(verdict.passed(), "{extra}: {:?}", verdict.findings);
    }
    Ok(())
}

// AC: CE-08.5
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
        let verdict = lint(dir.path())?;
        let message = only_finding(&verdict, flagged);
        assert!(message.contains("rust-toolchain.toml"), "{message}");
    }

    let verdict = lint_case("legacy-rust-toolchain")?;
    only_finding(&verdict, "rust-toolchain");
    Ok(())
}

// AC: CE-08.5
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
    let verdict = lint(dir.path())?;
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
    // shipped, but never counted against the floor.
    for lib_rs in [
        "#[cfg(not(coverage))]\npub fn f() {}\n",
        "#[cfg(any(test, coverage_nightly))]\npub fn f() {}\n",
        "#[cfg_attr(coverage, derive(Debug))]\npub struct S;\n",
        "#![coverage(off)]\npub fn f() {}\n",
        "#![cfg(not(coverage))]\npub fn f() {}\n",
        "pub fn a() {} #[cfg(not(coverage))] pub fn f() {}\n",
        "#[cfg(\n    not(coverage)\n)]\npub fn f() {}\n",
    ] {
        let verdict = lint_source(lib_rs)?;
        let message = only_finding(&verdict, "crates/a/src/lib.rs");
        assert!(message.contains("coverage"), "{lib_rs}: {message}");
    }

    let verdict = lint_case("coverage-cfg")?;
    assert_eq!(places(&verdict), [(Some("crates/a/src/lib.rs"), Some(3))]);
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

// AC: CE-08.5
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

// AC: CE-08.5
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
fn a_path_attribute_after_code_is_judged_by_the_src_rule() -> TestResult {
    let verdict = lint_lib(
        "pub fn a() {} #[path = \"../outside.rs\"] mod x;
",
    )?;
    only_finding(&verdict, "crates/a/src/lib.rs");
    let verdict = lint_lib(
        "pub fn a() {} #[path = \"inner.rs\"] mod x;
",
    )?;
    assert!(verdict.passed(), "{:?}", verdict.findings);
    Ok(())
}
