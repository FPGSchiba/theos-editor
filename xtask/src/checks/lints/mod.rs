//! The `lints` check: nothing outside `gate/` can weaken the lints, the tool configuration or the
//! test run, and no source file hides code from a check (`docs/gate.md` §1).
//!
//! The check is a list of rules, [`RULES`], each one module with one job. A rule reads the
//! [`Inputs`]: the repository files, the workspace view (what cargo compiles, every file parsed
//! once with `syn`) and the cfg names that code may test. A new rule is one module and one entry
//! in the list.
//!
//! - `sources` (rule 1): library, binary and build-script code is compiled from the `src/` of its
//!   crate only, and has no `include!`, so that `coverage` measures what ships.
//! - `cfg_names` (rule 2): `cfg` and `cfg_attr` attributes use only the names of
//!   `gate/cfg-names.toml`, so that every check builds and measures the same code.
//! - `config_files` (rule 4): the lint tables, the manifests and the mirrored configuration
//!   files, which are read as text and TOML and need no view.
//!
//! The retired scanners of the first build (`attributes.rs`, `fixture_routes.rs`, `src_bounds.rs`,
//! `attribute_reader.rs`, `tokens.rs`) still run over every `.rs` file until rule 3 replaces them.
//!
//! The view's own findings (a file that does not parse) come first. Findings name the file and the
//! line and say what is wrong, never what is written there, with one exception: the cfg rule
//! repeats a rejected cfg name, as ASCII letters, digits and `_` only (every other character as
//! `?`) and at most 40 characters. That cannot carry a format or bidi character, `::`, a newline
//! or a `%`, so it cannot form a workflow command or an option. The repository is the pull
//! request's, and what a check prints is read by the runner.

use std::fs;
use std::io::Write;
use std::path::Path;

use super::ac_trace::{extension, is_fixture};
use crate::ci::CiEvent;
use crate::files::repo_files;
use crate::verdict::{Check, Finding, Verdict};
use crate::view::{Shared, View};
use crate::{Error, Result};

mod attribute_reader;
mod attributes;
pub mod cfg_names;
pub mod config_files;
mod fixture_routes;
mod sources;
mod src_bounds;
mod tokens;

pub use attributes::scan_attributes;
pub use cfg_names::CfgNames;
pub(crate) use config_files::parse;
pub use config_files::{same_text, workspace_lints_match};
pub use fixture_routes::{scan_fixture_modules, scan_includes};

/// What a rule reads.
pub struct Inputs<'a> {
    /// The repository root.
    pub root: &'a Path,
    /// The files of the repository, repo-relative with `/`.
    pub files: &'a [String],
    /// What cargo compiles, and every Rust file of it parsed.
    pub view: &'a View,
    /// The cfg names that code may test.
    pub cfg_names: &'a CfgNames,
}

/// One rule of the check.
pub struct Rule {
    /// The rule's name.
    pub name: &'static str,
    /// Runs the rule; its findings are sorted by file and line.
    pub run: fn(&Inputs<'_>) -> Result<Vec<Finding>>,
}

/// Every rule, in the order their findings are reported.
pub const RULES: &[Rule] = &[sources::RULE, cfg_names::RULE, config_files::RULE];

/// Runs the check on the repository at `root`, on the files that git lists there (a walk
/// outside a work tree), over the view that `shared` holds.
///
/// # Errors
///
/// Fails when the cfg names, the files or the view cannot be read, or a rule fails.
pub fn run(
    root: &Path,
    _ci: Option<&CiEvent>,
    shared: &Shared,
    _out: &mut dyn Write,
) -> Result<Verdict> {
    let files = repo_files(root)?;
    check(root, &files, shared.view(root)?)
}

/// Checks the repository at `root`, whose files are `files` (repo-relative, joined with `/`),
/// over `view`: the findings of the view first, then those of each rule in the order of
/// [`RULES`].
///
/// # Errors
///
/// Fails when `gate/cfg-names.toml` or a file that a rule reads cannot be read.
pub fn check(root: &Path, files: &[String], view: &View) -> Result<Verdict> {
    let cfg_names = CfgNames::load(root)?;
    let inputs = Inputs {
        root,
        files,
        view,
        cfg_names: &cfg_names,
    };
    let mut findings = view.findings.clone();
    for rule in RULES {
        findings.extend((rule.run)(&inputs)?);
    }
    legacy_sources(root, files, &mut findings)?;
    Ok(Verdict::with(Check::Lints, findings))
}

/// A finding about the whole of `file`.
fn at_file(file: &str, message: impl Into<String>) -> Finding {
    Finding {
        file: Some(file.to_owned()),
        line: None,
        message: message.into(),
    }
}

/// Whether `path` is a Rust source that is scanned: any `.rs` file but the fixtures and the
/// build folder `target/` of the root. A folder called `target` or `node_modules` further down
/// is source like any other: `mod target;` builds the code in it. The listing of the files
/// already leaves out the build folder, so this is for lists that came from elsewhere.
fn is_source(path: &str) -> bool {
    extension(path) == Some("rs") && !is_fixture(path) && !path.starts_with("target/")
}

/// The attribute findings of every Rust source that is scanned (the scan that rule 3 replaces).
fn legacy_sources(root: &Path, files: &[String], findings: &mut Vec<Finding>) -> Result<()> {
    for path in files.iter().filter(|path| is_source(path)) {
        let full = root.join(path);
        let bytes = fs::read(&full).map_err(|source| Error::Io { path: full, source })?;
        let text = String::from_utf8_lossy(&bytes);
        let mut found = scan_attributes(path, &text);
        found.extend(scan_includes(path, &text));
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
    use super::*;

    #[test]
    fn every_rule_is_registered_once() {
        let names: Vec<&str> = RULES.iter().map(|rule| rule.name).collect();
        assert_eq!(names, ["sources", "cfg-names", "config-files"]);
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
