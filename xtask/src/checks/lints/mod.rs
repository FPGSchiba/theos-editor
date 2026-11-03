//! The `lints` check: nothing outside `gate/` can weaken the lints, the tool configuration or the
//! test run, and no source file silences a lint or skips a test without saying why
//! (`docs/gate.md` §1).
//!
//! The check [`check`] reads the files that [`repo_files`] lists and finds:
//!
//! - a `[workspace.lints]` table in the root `Cargo.toml` that is not `gate/lints.toml`, compared
//!   as parsed TOML ([`workspace_lints_match`]);
//! - a `Cargo.toml` below the root that has no `[lints]` with `workspace = true`, so that its
//!   crate would not get the workspace lints, and a root `Cargo.toml` that is a package and has
//!   none;
//! - a root `clippy.toml` or `.config/nextest.toml` that is not a copy of its original in `gate/`,
//!   comparing the text but not the line ends ([`same_text`]);
//! - a clippy or rustfmt configuration, or a `.cargo` folder, anywhere else (the root `.cargo`
//!   and `gate/clippy.toml` are the only exceptions), because clippy and cargo find these files
//!   by walking up from the crate;
//! - a file called `rust-toolchain` anywhere, which rustup reads instead of the pinned
//!   `rust-toolchain.toml`;
//! - in every `.rs` source, an attribute with `allow(…)`, an `expect(…)` without
//!   `reason = "…"` or an `ignore` without `= "…"` ([`scan_attributes`]);
//! - a way to compile a fixture, which the scan does not read (below): a `#[path = "…"]`
//!   attribute or an `include!(…)` that leads into a `tests/fixtures/` folder or whose path
//!   cannot be told ([`scan_attributes`], [`scan_includes`]); a `mod fixtures;` declaration
//!   ([`scan_fixture_modules`]); a `main.rs` or `mod.rs` directly in a `tests/fixtures/` folder;
//!   and a `path` in a `Cargo.toml` (`[lib]`, `[[bin]]`, `[[test]]`, `[[example]]`, `[[bench]]`,
//!   dependencies of every kind, `package.build`, workspace members) that leads there.
//! - code outside the `src/` of its crate, which `coverage` does not count: a `#[path]` or an
//!   `include!` in a file under `crates/<name>/src/` or `xtask/src/` that leads out of that
//!   `src/`, and a `[lib]` or `[[bin]]` `path` in the manifest of such a crate that does not
//!   lead into it (`src_bounds.rs`). `include_str!` and `include_bytes!` are data and stay
//!   allowed.
//!
//! - code that `coverage` would not measure: `coverage` or `coverage_nightly` in a `cfg(…)` or
//!   `cfg_attr(…)` attribute, or an attribute named `coverage`, anywhere in a source, whatever
//!   it is nested in (`coverage_cfg.rs`). `#[cfg(not(test))]` stays allowed.
//!
//! The `.rs` sources are all that are listed but the files under `tests/fixtures/` and under the
//! build folder `target/` of the root; a folder called `target` or `node_modules` further down
//! is source, because `mod target;` builds it.
//!
//! Files under `tests/fixtures/` are test data with deliberate violations: the check reads none
//! of them as part of the repository, and no manifest there. So that nothing in them is built
//! unseen, every way to build one is a finding: an attribute, an include or a manifest that
//! leads there (a path is read from the folder of the file, with `.` and `..` resolved, `\` as
//! `/` and without regard for case; one that is not one string literal, is absolute, climbs out
//! of the repository or has an escape in it cannot be followed, and is a finding in source), a
//! `mod fixtures;` (any case, any visibility) that reads `tests/fixtures/mod.rs`, and the files
//! that cargo or such a declaration find by themselves, `tests/fixtures/main.rs` and
//! `tests/fixtures/mod.rs`. A module of a file may be called `tests`, so a path in source that
//! starts in a folder `fixtures` counts too. `include_str!` and `include_bytes!` read data, and
//! fixtures may be read that way. Files in folders below, such as `tests/fixtures/case/main.rs`,
//! are the fixtures themselves, and stay allowed.
//!
//! Every rule that reads attributes reads them from the token stream (`attribute_reader.rs`):
//! inner and outer, anywhere in a line, over several lines, in function bodies and inside
//! `macro_rules!` arms. Comments, strings and character literals are not tokens, so what they
//! hold never counts; block comments nest as in rustc. An attribute-like token sequence in a
//! macro body that never becomes an attribute is read as one, so the rules over-report there and
//! never under-report.
//!
//! Findings name the file and the line and say what is wrong, never what is written there: the
//! repository is the pull request's, and what a check prints is read by the runner.
//!
//! The code is in seven files: this one has the rules for the lint tables, the manifests and the
//! mirrored configuration files, `attributes.rs` the attribute scan, `fixture_routes.rs` the ways
//! to compile a fixture (the name avoids `mod fixtures;`, which this check forbids), and
//! `src_bounds.rs` the rule that code stays in `src/`, `coverage_cfg.rs` the rule on coverage
//! cfgs, `attribute_reader.rs` the reader of attributes that the rules share, and `tokens.rs` the
//! tokens that these read.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::Path;

use super::ac_trace::{extension, is_fixture};
use crate::ci::CiEvent;
use crate::files::repo_files;
use crate::verdict::{Check, Finding, Verdict};
use crate::{Error, Result};

mod attribute_reader;
mod attributes;
mod coverage_cfg;
mod fixture_routes;
mod src_bounds;
mod tokens;

pub use attributes::scan_attributes;
use fixture_routes::{MANIFEST_FIXTURES, fixture_entry_points, manifest_leads_into_fixtures};
pub use fixture_routes::{scan_fixture_modules, scan_includes};
use src_bounds::manifest_findings;

/// The root manifest, which holds `[workspace.lints]`.
const ROOT_MANIFEST: &str = "Cargo.toml";

/// The lints of the gate, in the shape of `[workspace.lints]`.
const GATE_LINTS: &str = "gate/lints.toml";

/// The files that must be copies, each with its original: the copy first.
const MIRRORS: [(&str, &str); 2] = [
    ("clippy.toml", "gate/clippy.toml"),
    (".config/nextest.toml", "gate/nextest.toml"),
];

/// The only `clippy.toml` files: the copy that clippy reads and its original.
const CLIPPY_FILES: [&str; 2] = ["clippy.toml", "gate/clippy.toml"];

/// Runs the check on the repository at `root`, on the files that git lists there (a walk
/// outside a work tree).
///
/// # Errors
///
/// Fails when the files cannot be listed or one of them cannot be read.
pub fn run(root: &Path, _ci: Option<&CiEvent>, _out: &mut dyn Write) -> Result<Verdict> {
    check(root, &repo_files(root)?)
}

/// Checks the lint tables, the configuration files and the attributes of the repository at
/// `root`, whose files are `files` (repo-relative, joined with `/`).
///
/// A file the check needs and `files` lacks is a finding, and so is a file that is not TOML where
/// it should be.
///
/// # Errors
///
/// Fails when a file in `files` cannot be read.
pub fn check(root: &Path, files: &[String]) -> Result<Verdict> {
    let mut findings = Vec::new();
    lint_tables(root, files, &mut findings)?;
    crate_manifests(root, files, &mut findings)?;
    for (copy, original) in MIRRORS {
        mirror(root, files, copy, original, &mut findings)?;
    }
    findings.extend(stray_config(files));
    findings.extend(fixture_entry_points(files));
    sources(root, files, &mut findings)?;
    Ok(Verdict::with(Check::Lints, findings))
}

/// Whether `[workspace.lints]` of the root manifest `root_manifest` holds exactly the lints of
/// `gate_lints`, the text of `gate/lints.toml`. The two are compared as parsed TOML, so a table
/// that is written differently but means the same is the same.
///
/// A manifest without the table does not match.
///
/// # Errors
///
/// Fails with [`Error::Parse`] when either text is not TOML. The message gives the line and
/// nothing of the text.
pub fn workspace_lints_match(root_manifest: &str, gate_lints: &str) -> Result<bool> {
    let manifest = parse(ROOT_MANIFEST, root_manifest)?;
    let gate = parse(GATE_LINTS, gate_lints)?;
    let table = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("lints"));
    Ok(table == Some(&toml::Value::Table(gate)))
}

/// Whether the texts `a` and `b` are the same but for their line ends: CRLF and LF are alike.
#[must_use]
pub fn same_text(a: &str, b: &str) -> bool {
    a.replace("\r\n", "\n") == b.replace("\r\n", "\n")
}

/// The text of `path` when `files` lists it; else a finding says that it is missing.
fn load(
    root: &Path,
    files: &[String],
    path: &str,
    findings: &mut Vec<Finding>,
) -> Result<Option<String>> {
    if files.iter().any(|file| file == path) {
        read(root, path).map(Some)
    } else {
        findings.push(at_file(path, "is missing, and the gate needs it"));
        Ok(None)
    }
}

/// The text of the file `path` of the repository at `root`, which must be UTF-8.
fn read(root: &Path, path: &str) -> Result<String> {
    let full = root.join(path);
    fs::read_to_string(&full).map_err(|source| Error::Io { path: full, source })
}

/// A finding about the whole of `file`.
fn at_file(file: &str, message: impl Into<String>) -> Finding {
    Finding {
        file: Some(file.to_owned()),
        line: None,
        message: message.into(),
    }
}

/// `text` as a TOML table. The error does not quote it; `what` names the file.
pub(crate) fn parse(what: &str, text: &str) -> Result<toml::Table> {
    text.parse().map_err(|error: toml::de::Error| {
        let message = match error.span() {
            Some(span) => {
                let before = text.get(..span.start).unwrap_or(text);
                let line = before.matches('\n').count() + 1;
                format!("not valid TOML, near line {line}")
            }
            None => "not valid TOML".to_owned(),
        };
        Error::Parse {
            what: what.to_owned(),
            message,
        }
    })
}

/// `[workspace.lints]` against `gate/lints.toml`.
fn lint_tables(root: &Path, files: &[String], findings: &mut Vec<Finding>) -> Result<()> {
    let manifest = load(root, files, ROOT_MANIFEST, findings)?;
    let gate = load(root, files, GATE_LINTS, findings)?;
    let (Some(manifest), Some(gate)) = (manifest, gate) else {
        return Ok(());
    };
    match workspace_lints_match(&manifest, &gate) {
        Ok(true) => {}
        Ok(false) => findings.push(at_file(
            ROOT_MANIFEST,
            "`[workspace.lints]` differs from `gate/lints.toml`: it must be the same lints at the \
             same levels; change `gate/lints.toml` in a gate pull request and copy it here",
        )),
        Err(Error::Parse { what, message }) => findings.push(at_file(&what, message)),
        Err(error) => return Err(error),
    }
    Ok(())
}

/// Every manifest below the root, and the root manifest if it is a package: each must inherit
/// the workspace lints.
fn crate_manifests(root: &Path, files: &[String], findings: &mut Vec<Finding>) -> Result<()> {
    let manifests = files
        .iter()
        .filter(|path| path.as_str() == ROOT_MANIFEST || path.ends_with("/Cargo.toml"))
        .filter(|path| !is_fixture(path));
    for path in manifests {
        let manifest = match parse(path, &read(root, path)?) {
            Ok(manifest) => manifest,
            // `lint_tables` has read the root manifest too, and reports it.
            Err(Error::Parse { .. }) if path == ROOT_MANIFEST => continue,
            Err(Error::Parse { message, .. }) => {
                findings.push(at_file(path, message));
                continue;
            }
            Err(error) => return Err(error),
        };
        if !inherits_lints(path, &manifest) {
            findings.push(at_file(
                path,
                "has no `[lints]` with `workspace = true`, so the crate does not get the \
                 workspace lints",
            ));
        }
        if manifest_leads_into_fixtures(path, &manifest) {
            findings.push(at_file(path, MANIFEST_FIXTURES));
        }
        findings.extend(manifest_findings(path, &manifest));
    }
    Ok(())
}

/// Whether the manifest `manifest` of the file `path` has `[lints]` with `workspace = true`. The
/// root manifest of a workspace that is not a package has no crate to give lints to, and passes.
fn inherits_lints(path: &str, manifest: &toml::Table) -> bool {
    if path == ROOT_MANIFEST && !manifest.contains_key("package") {
        return true;
    }
    let workspace = manifest
        .get("lints")
        .and_then(|lints| lints.get("workspace"))
        .and_then(toml::Value::as_bool);
    workspace == Some(true)
}

/// The file `copy` against its original `original`.
fn mirror(
    root: &Path,
    files: &[String],
    copy: &str,
    original: &str,
    findings: &mut Vec<Finding>,
) -> Result<()> {
    let copy_text = load(root, files, copy, findings)?;
    let original_text = load(root, files, original, findings)?;
    if let (Some(copy_text), Some(original_text)) = (copy_text, original_text)
        && !same_text(&copy_text, &original_text)
    {
        findings.push(at_file(
            copy,
            format!(
                "differs from `{original}`: it must be a copy; change `{original}` in a gate \
                 pull request and copy it here"
            ),
        ));
    }
    Ok(())
}

/// The clippy and rustfmt configuration files, the legacy `rust-toolchain` files and the `.cargo`
/// folders that must not exist: the ones that are not the root copy of `clippy.toml` and its
/// original, and not the root `.cargo`. A `rust-toolchain` file anywhere wins over the protected
/// `rust-toolchain.toml` in rustup, so none may exist.
///
/// File systems that ignore case find `Clippy.toml` as `clippy.toml`, so names are compared
/// without case. A `.cargo` folder is one finding, however many files it has.
fn stray_config(files: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut cargo_folders = BTreeSet::new();
    for path in files.iter().filter(|path| !is_fixture(path)) {
        let mut folders: Vec<&str> = path.split('/').collect();
        let name = folders.pop().unwrap_or_default().to_ascii_lowercase();
        match name.as_str() {
            "clippy.toml" | ".clippy.toml" if !CLIPPY_FILES.contains(&path.as_str()) => {
                findings.push(at_file(
                    path,
                    "a clippy configuration outside the root: clippy would read it for the \
                     crates below it; only the root `clippy.toml`, a copy of \
                     `gate/clippy.toml`, may exist",
                ));
            }
            "rustfmt.toml" | ".rustfmt.toml" => findings.push(at_file(
                path,
                "a rustfmt configuration: the gate has none, and one outside `gate/` could \
                 change what `fmt` checks",
            )),
            "rust-toolchain" => findings.push(at_file(
                path,
                "a legacy toolchain file: rustup reads it instead of `rust-toolchain.toml`, \
                 which holds the pin; remove it",
            )),
            _ => {}
        }
        for (index, folder) in folders.iter().enumerate() {
            let root_cargo = index == 0 && *folder == ".cargo";
            if folder.eq_ignore_ascii_case(".cargo") && !root_cargo {
                cargo_folders.insert(folders[..=index].join("/"));
            }
        }
    }
    findings.extend(cargo_folders.iter().map(|folder| {
        at_file(
            folder,
            "a `.cargo` folder outside the root: cargo would read its configuration for the \
             crates below it",
        )
    }));
    findings
}

/// Whether `path` is a Rust source that is scanned: any `.rs` file but the fixtures and the
/// build folder `target/` of the root. A folder called `target` or `node_modules` further down
/// is source like any other: `mod target;` builds the code in it. The listing of the files
/// already leaves out the build folder, so this is for lists that came from elsewhere.
fn is_source(path: &str) -> bool {
    extension(path) == Some("rs") && !is_fixture(path) && !path.starts_with("target/")
}

/// The attribute findings of every Rust source that is scanned.
fn sources(root: &Path, files: &[String], findings: &mut Vec<Finding>) -> Result<()> {
    for path in files.iter().filter(|path| is_source(path)) {
        let full = root.join(path);
        let bytes = fs::read(&full).map_err(|source| Error::Io { path: full, source })?;
        let text = String::from_utf8_lossy(&bytes);
        let mut found = scan_attributes(path, &text);
        found.extend(scan_includes(path, &text));
        found.extend(coverage_cfg::scan_coverage_cfg(path, &text));
        found.extend(scan_fixture_modules(path, &text));
        found.sort_by_key(|finding| finding.line);
        findings.extend(found);
    }
    Ok(())
}

/// What the tests of this module and of the files below it have in common.
#[cfg(test)]
mod testing {
    use crate::verdict::Finding;

    /// The lines joined into the text of a source file.
    pub(super) fn source(lines: &[&str]) -> String {
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }

    pub(super) fn lines_of(findings: &[Finding]) -> Vec<Option<u32>> {
        findings.iter().map(|finding| finding.line).collect()
    }

    pub(super) fn paths(files: &[&str]) -> Vec<String> {
        files.iter().map(|file| (*file).to_owned()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::testing::paths;
    use super::*;

    #[test]
    fn lints_tables_compare_parsed() -> crate::Result<()> {
        let gate = "[rust]\nunsafe_code = \"forbid\"\nrust_2018_idioms = { level = \"warn\", \
                    priority = -1 }\n\n[clippy]\npedantic = { level = \"warn\", priority = -1 }\n";
        let reformatted = "[workspace]\nmembers = [\"crates/*\"]\n\n\
                           [workspace.lints.clippy.pedantic]\nlevel = \"warn\"\npriority = -1\n\n\
                           [workspace.lints.rust]\n# a comment\nrust_2018_idioms = { priority = \
                           -1, level = \"warn\" }\nunsafe_code = \"forbid\"\n";
        assert!(workspace_lints_match(reformatted, gate)?);

        let weaker = reformatted.replace("forbid", "warn");
        assert!(!workspace_lints_match(&weaker, gate)?);
        let extra = format!(
            "{reformatted}\n[workspace.lints.rustdoc]\nbroken_intra_doc_links = \"deny\"\n"
        );
        assert!(!workspace_lints_match(&extra, gate)?);
        let fewer = "[workspace]\n\n[workspace.lints.rust]\nunsafe_code = \"forbid\"\n";
        assert!(!workspace_lints_match(fewer, gate)?);
        let none = "[workspace]\nmembers = []\n";
        assert!(!workspace_lints_match(none, gate)?);
        let package_lints = "[lints.rust]\nunsafe_code = \"forbid\"\n";
        assert!(!workspace_lints_match(package_lints, gate)?);
        Ok(())
    }

    #[test]
    fn a_table_that_is_not_toml_is_an_error_that_does_not_quote_it() {
        for (root, gate) in [
            ("secret = \n", "[rust]\n"),
            ("[workspace]\n", "hidden = = 1\n"),
        ] {
            let result = workspace_lints_match(root, gate);
            let Err(error @ Error::Parse { .. }) = result else {
                panic!("expected a parse error: {result:?}");
            };
            let text = error.to_string();
            assert!(
                !text.contains("secret") && !text.contains("hidden"),
                "{text}"
            );
            assert!(text.contains("line 1"), "{text}");
        }
    }

    #[test]
    fn crlf_mirror_equals_lf() {
        assert!(same_text("a = 1\r\nb = 2\r\n", "a = 1\nb = 2\n"));
        assert!(same_text("a = 1\nb = 2\n", "a = 1\nb = 2\n"));
        assert!(!same_text("a = 1\r\nb = 2\r\n", "a = 1\nb = 3\n"));
        // Only a line end is allowed to differ: not a missing last line end, not a lone CR.
        assert!(!same_text("a = 1\n", "a = 1"));
        assert!(!same_text("a = 1\r", "a = 1\n"));
        assert!(!same_text("a = 1\n", "a = 1\n\n"));
    }

    #[test]
    fn config_files_are_found_wherever_they_are_not_allowed() {
        let allowed = paths(&[
            "Cargo.toml",
            "clippy.toml",
            "gate/clippy.toml",
            ".config/nextest.toml",
            "gate/nextest.toml",
            ".cargo/config.toml",
            "crates/a/Cargo.toml",
            "crates/a/src/lib.rs",
            // Fixtures are test data; nothing reads their configuration.
            "xtask/tests/fixtures/lints/x/crates/a/clippy.toml",
            "tests/fixtures/rustfmt.toml",
            "crates/a/tests/fixtures/x/.cargo/config.toml",
            "rust-toolchain.toml",
            "tests/fixtures/rust-toolchain",
        ]);
        assert_eq!(stray_config(&allowed), Vec::<Finding>::new());

        // Each path, and the file or folder the finding names.
        for (stray, flagged) in [
            ("crates/a/clippy.toml", "crates/a/clippy.toml"),
            ("crates/a/.clippy.toml", "crates/a/.clippy.toml"),
            (".clippy.toml", ".clippy.toml"),
            ("gate/.clippy.toml", "gate/.clippy.toml"),
            ("crates/clippy.toml", "crates/clippy.toml"),
            ("rustfmt.toml", "rustfmt.toml"),
            (".rustfmt.toml", ".rustfmt.toml"),
            ("crates/a/rustfmt.toml", "crates/a/rustfmt.toml"),
            ("gate/rustfmt.toml", "gate/rustfmt.toml"),
            ("crates/a/.cargo/config.toml", "crates/a/.cargo"),
            ("crates/a/.cargo/config", "crates/a/.cargo"),
            ("a/b/.cargo/config.toml", "a/b/.cargo"),
            // On Windows and macOS the file system finds these under the lower case name.
            ("crates/a/Clippy.toml", "crates/a/Clippy.toml"),
            ("RUSTFMT.TOML", "RUSTFMT.TOML"),
            ("crates/a/.Cargo/config.toml", "crates/a/.Cargo"),
            (".Cargo/config.toml", ".Cargo"),
            ("rust-toolchain", "rust-toolchain"),
            ("crates/a/rust-toolchain", "crates/a/rust-toolchain"),
            ("Rust-Toolchain", "Rust-Toolchain"),
        ] {
            let findings = stray_config(&paths(&[stray]));
            assert_eq!(findings.len(), 1, "{stray}: {findings:?}");
            assert_eq!(findings[0].file.as_deref(), Some(flagged), "{stray}");
        }
    }

    #[test]
    fn a_cargo_folder_is_one_finding_however_many_files_it_has() {
        let findings = stray_config(&paths(&[
            "crates/a/.cargo/config.toml",
            "crates/a/.cargo/other",
            "crates/b/.cargo/config.toml",
        ]));
        let files: Vec<Option<&str>> = findings
            .iter()
            .map(|finding| finding.file.as_deref())
            .collect();
        assert_eq!(files, [Some("crates/a/.cargo"), Some("crates/b/.cargo")]);
    }

    #[test]
    fn sources_are_rust_files_but_fixtures_and_the_root_target() {
        for path in [
            "build.rs",
            "src/lib.rs",
            "crates/a/src/target/mod.rs",
            "crates/a/src/node_modules/x.rs",
            "crates/a/target/x.rs",
            "node_modules/x.rs",
            "crates/a/tests/fixtures.rs",
            "crates/a/tests/fixturesX/x.rs",
            "research/prose/x.rs",
        ] {
            assert!(is_source(path), "{path}");
        }
        for path in [
            "README.md",
            "crates/a/src/lib.rs.bak",
            "target/debug/build/x.rs",
            "tests/fixtures/x.rs",
            "crates/a/tests/fixtures/case/src/lib.rs",
        ] {
            assert!(!is_source(path), "{path}");
        }
    }
}
