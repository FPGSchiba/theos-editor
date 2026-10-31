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
//! - in every `.rs` source, an attribute with `allow(…)`, an `expect(…)` without
//!   `reason = "…"` or an `ignore` without `= "…"` ([`scan_attributes`]);
//! - a way to compile a fixture, which the scan does not read (below): a `#[path = "…"]`
//!   attribute or an `include!(…)` that leads into a `tests/fixtures/` folder or whose path
//!   cannot be told ([`scan_attributes`], [`scan_includes`]); a `mod fixtures;` declaration
//!   ([`scan_fixture_modules`]); a `main.rs` or `mod.rs` directly in a `tests/fixtures/` folder;
//!   and a `path` in a `Cargo.toml` (`[lib]`, `[[bin]]`, `[[test]]`, `[[example]]`, `[[bench]]`,
//!   dependencies of every kind, `package.build`, workspace members) that leads there.
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
//! The attribute scan is for attributes at the start of a line, because a text scan cannot know
//! more: a line whose trimmed text starts with `#[` or `#![` starts an attribute, which goes on
//! over the next lines until its brackets balance, and any attribute that follows it on its last
//! line is read too. An attribute after other code on a line, such as one in a macro body that is
//! written on one line, is not read: `fmt` moves the attributes of items and statements to lines
//! of their own, and clippy denies `allow_attributes` in the code it compiles. Comments and
//! strings inside an attribute do not count as text of it.
//!
//! Findings name the file and the line and say what is wrong, never what is written there: the
//! repository is the pull request's, and what a check prints is read by the runner.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::Path;

use super::ac_trace::rust_tags::{attribute_end, literal_end};
use super::ac_trace::{extension, is_fixture, line_number};
use crate::ci::CiEvent;
use crate::files::repo_files;
use crate::verdict::{Check, Finding, Verdict};
use crate::{Error, Result};

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

const ALLOW_FOUND: &str = "`allow(…)` in an attribute: a lint is not silenced; fix the cause, or \
                           use `#[expect(…, reason = \"…\")]` for a lint that is expected";
const EXPECT_FOUND: &str = "`expect(…)` without `reason = \"…\"`: say why the lint is expected";
const IGNORE_FOUND: &str = "`ignore` without a reason: write `#[ignore = \"…\"]`";
const PATH_FIXTURES: &str = "`#[path]` leads into a `tests/fixtures/` folder: fixtures are test \
                             data that no check reads, so they are not compiled; read one as \
                             data with `include_str!`";
const PATH_UNKNOWN: &str = "`#[path]` is not a string literal for a path inside the repository, \
                            so the check cannot tell where it leads: write the path out";
const INCLUDE_FIXTURES: &str = "`include!` leads into a `tests/fixtures/` folder: fixtures are \
                                test data that no check reads, so they are not compiled; read \
                                one as data with `include_str!`";
const INCLUDE_UNKNOWN: &str = "`include!` is not given one string literal for a path inside the \
                               repository, so the check cannot tell what it compiles: write the \
                               path out";
const MOD_FIXTURES: &str = "`mod fixtures;` reads `tests/fixtures/mod.rs` where a `tests/` folder \
                            has one: fixtures are test data that no check reads, so they are not \
                            compiled; call the module something else, or give it a body";
const FIXTURE_ENTRY_POINT: &str = "a `main.rs` or `mod.rs` directly in a `tests/fixtures/` folder: \
                                   cargo builds it as a test and `mod fixtures;` as a module, and \
                                   no check reads it; keep fixture files in a folder of their own \
                                   below";
const MANIFEST_FIXTURES: &str = "has a `path` that leads into a `tests/fixtures/` folder: fixtures \
                                 are test data that no check reads, so no crate is built from them";

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

/// The findings for the attributes of the Rust source `text` of the file `file`, in file order.
///
/// An attribute is found when it has `allow(` (also inside `cfg_attr`), when it has `expect(`
/// and no `reason = "…"` with something in it, or when it is `ignore` (also inside `cfg_attr`)
/// and has no `= "…"` with something in it. A `path = …` of an attribute (also inside `cfg_attr`)
/// is found when it leads into a `tests/fixtures/` folder or cannot be followed. Only whole words
/// count, and words in comments and strings do not.
#[must_use]
pub fn scan_attributes(file: &str, text: &str) -> Vec<Finding> {
    let lines: Vec<&str> = text.lines().collect();
    let mut findings = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if starts_attribute(lines[index].trim()) {
            index = scan_attribute_line(file, &lines, index, &mut findings);
        }
        index += 1;
    }
    findings
}

/// The findings for the `include!` macros of the Rust source `text` of the file `file`, in file
/// order, on the line where each starts.
///
/// An `include!` is found when its path leads into a `tests/fixtures/` folder, and when it is
/// not given one string literal, such as `concat!(env!("OUT_DIR"), …)`: nothing is compiled
/// that the check has not been able to look at. `include_str!` and `include_bytes!` read data
/// and are fine. The macro is looked for in the whole text, not only at the start of a line;
/// comments and strings do not count.
#[must_use]
pub fn scan_includes(file: &str, text: &str) -> Vec<Finding> {
    let tokens = located_tokens(text);
    let mut findings = Vec::new();
    for (index, &(offset, token)) in tokens.iter().enumerate() {
        if token != Token::Word("include") {
            continue;
        }
        let rest = tokens.get(index + 1..).unwrap_or_default();
        let [
            (_, Token::Symbol(b'!')),
            (_, Token::Symbol(open @ (b'(' | b'[' | b'{'))),
            arguments @ ..,
        ] = rest
        else {
            continue;
        };
        let close = match open {
            b'(' => b')',
            b'[' => b']',
            _ => b'}',
        };
        let message = match arguments {
            [(_, Token::Text(path)), (_, Token::Symbol(end)), ..]
            | [
                (_, Token::Text(path)),
                (_, Token::Symbol(b',')),
                (_, Token::Symbol(end)),
                ..,
            ] if *end == close => match target(file, path) {
                Target::Elsewhere => continue,
                Target::Fixtures => INCLUDE_FIXTURES,
                Target::Unknown => INCLUDE_UNKNOWN,
            },
            _ => INCLUDE_UNKNOWN,
        };
        findings.push(Finding::at(file, line_at(text, offset), message));
    }
    findings
}

/// The findings for the declarations `mod fixtures;` of the Rust source `text` of the file
/// `file`, in file order, on the line of the `mod`.
///
/// A module of that name that is not written out in the file is read from a file called
/// `fixtures.rs` or `fixtures/mod.rs`, which in `tests/` is a fixtures file that nobody scans. Any
/// such declaration is found, whatever its visibility and wherever the file is, and the name is
/// read without regard for case (and as `r#fixtures`) because the file system may not tell. A
/// module with a body, `mod fixtures { … }`, reads no file and is fine. Comments and strings do
/// not count.
#[must_use]
pub fn scan_fixture_modules(file: &str, text: &str) -> Vec<Finding> {
    let tokens = located_tokens(text);
    let mut findings = Vec::new();
    for (index, &(offset, token)) in tokens.iter().enumerate() {
        if token != Token::Word("mod") {
            continue;
        }
        let rest = tokens.get(index + 1..).unwrap_or_default();
        let rest = match rest {
            [(_, Token::Word("r")), (_, Token::Symbol(b'#')), name @ ..] => name,
            _ => rest,
        };
        if let [(_, Token::Word(name)), (_, Token::Symbol(b';')), ..] = rest
            && name.eq_ignore_ascii_case("fixtures")
        {
            findings.push(Finding::at(file, line_at(text, offset), MOD_FIXTURES));
        }
    }
    findings
}

/// The files that get built without a line of code that names them: a `main.rs` or `mod.rs`
/// directly in a `tests/fixtures/` folder, at any depth of the repository. Cargo finds
/// `tests/<name>/main.rs` as a test, and `mod fixtures;` in `tests/` finds `tests/fixtures/mod.rs`.
/// The files in the case folders below are not found by either.
fn fixture_entry_points(files: &[String]) -> Vec<Finding> {
    files
        .iter()
        .filter(|path| {
            let lower = path.to_ascii_lowercase();
            lower.rsplit_once('/').is_some_and(|(folder, name)| {
                matches!(name, "main.rs" | "mod.rs")
                    && (folder == "tests/fixtures" || folder.ends_with("/tests/fixtures"))
            })
        })
        .map(|path| at_file(path, FIXTURE_ENTRY_POINT))
        .collect()
}

/// Whether the manifest `manifest` of the file `file` has a path that leads into a
/// `tests/fixtures/` folder: a `path` value anywhere in it (`[lib]`, `[[bin]]`, `[[test]]`,
/// `[[example]]`, `[[bench]]`, the dependency tables of every kind, `[patch]`), the build script
/// `package.build` and the `members` of a workspace. The path is read from the folder of the
/// manifest, as for a module. A path that cannot be followed because it is absolute or leaves the
/// repository, as that of an engine crate does, is not found: a manifest may point anywhere else.
fn manifest_leads_into_fixtures(file: &str, manifest: &toml::Table) -> bool {
    let mut paths = Vec::new();
    collect_paths(manifest, &mut paths);
    let build = manifest
        .get("package")
        .and_then(|package| package.get("build"))
        .and_then(toml::Value::as_str);
    let members = ["members", "default-members"]
        .into_iter()
        .filter_map(|key| manifest.get("workspace")?.get(key)?.as_array())
        .flatten()
        .filter_map(toml::Value::as_str);
    paths.extend(build);
    paths.extend(members);
    paths.into_iter().any(|path| {
        normalise(path)
            .and_then(|own| folder_of(file, &own))
            .is_some_and(|folder| is_fixture(&folder))
    })
}

/// Adds the value of every key `path` in `table`, in its tables and in its arrays of tables.
fn collect_paths<'a>(table: &'a toml::Table, paths: &mut Vec<&'a str>) {
    for (key, value) in table {
        match value {
            toml::Value::String(path) if key == "path" => paths.push(path),
            toml::Value::Table(inner) => collect_paths(inner, paths),
            toml::Value::Array(items) => {
                for inner in items.iter().filter_map(toml::Value::as_table) {
                    collect_paths(inner, paths);
                }
            }
            _ => {}
        }
    }
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

/// The clippy and rustfmt configuration files and the `.cargo` folders that must not exist: the
/// ones that are not the root copy of `clippy.toml` and its original, and not the root `.cargo`.
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
        found.extend(scan_fixture_modules(path, &text));
        found.sort_by_key(|finding| finding.line);
        findings.extend(found);
    }
    Ok(())
}

/// Whether `text`, trimmed, starts an attribute.
fn starts_attribute(text: &str) -> bool {
    text.starts_with("#[") || text.starts_with("#![")
}

/// Scans the attributes that start with line `first` (counting from 0): the one that starts it,
/// joined over the lines that follow until its brackets balance, and the attributes after it on
/// the line it ends on. Returns the last line that was read.
fn scan_attribute_line(
    file: &str,
    lines: &[&str],
    first: usize,
    findings: &mut Vec<Finding>,
) -> usize {
    // What is left of the lines read so far, from the start of the next attribute.
    let mut text = lines[first].trim().to_owned();
    let mut start = first;
    let mut last = first;
    while starts_attribute(&text) {
        let end = loop {
            if let Some(end) = attribute_end(&text) {
                break end;
            }
            // An attribute that is never closed takes the rest of the file.
            let Some(next) = lines.get(last + 1) else {
                break text.len();
            };
            last += 1;
            text.push('\n');
            text.push_str(next);
        };
        findings.extend(judge(file, line_number(start), &text[..end]));
        text = text[end..].trim().to_owned();
        start = last;
    }
    last
}

/// A piece of attribute text that matters: a word, a string or a punctuation mark. Comments,
/// white space and character literals are left out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Token<'a> {
    /// Letters, digits and `_`.
    Word(&'a str),
    /// A string literal as written, with its quotes.
    Text(&'a str),
    /// An ASCII character that is none of the above.
    Symbol(u8),
}

/// The tokens of the attribute `text`.
fn tokens(text: &str) -> Vec<Token<'_>> {
    located_tokens(text)
        .into_iter()
        .map(|(_, token)| token)
        .collect()
}

/// The tokens of `text`, each with the byte it starts at.
fn located_tokens(text: &str) -> Vec<(usize, Token<'_>)> {
    let is_word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
        } else if bytes[i..].starts_with(b"//") {
            i = text[i..].find('\n').map_or(bytes.len(), |end| i + end);
        } else if bytes[i..].starts_with(b"/*") {
            i = text[i + 2..]
                .find("*/")
                .map_or(bytes.len(), |end| i + 2 + end + 2);
        } else if let Some(end) = literal_end(text, i) {
            let literal = &text[i..end];
            if literal.starts_with(['"', 'r']) {
                tokens.push((i, Token::Text(literal)));
            }
            i = end;
        } else if is_word(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_word(bytes[i]) {
                i += 1;
            }
            tokens.push((start, Token::Word(&text[start..i])));
        } else {
            if bytes[i].is_ascii() {
                tokens.push((i, Token::Symbol(bytes[i])));
            }
            i += 1;
        }
    }
    tokens
}

/// The findings for the attribute `attribute`, which starts on `line`.
fn judge(file: &str, line: u32, attribute: &str) -> Vec<Finding> {
    let tokens = tokens(attribute);
    // The first word is the name of the attribute: `ignore`, or `cfg_attr` that has `ignore` in it.
    let name = tokens
        .iter()
        .position(|token| matches!(token, Token::Word(_)));
    let conditional = name.is_some_and(|index| tokens[index] == Token::Word("cfg_attr"));
    let mut findings = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        let Token::Word(word) = token else {
            continue;
        };
        let after = &tokens[index + 1..];
        let opens = after.first() == Some(&Token::Symbol(b'('));
        let message = match *word {
            "allow" if opens => Some(ALLOW_FOUND),
            "expect" if opens && !has_reason(&after[1..]) => Some(EXPECT_FOUND),
            "ignore" if (name == Some(index) || conditional) && !has_value(after) => {
                Some(IGNORE_FOUND)
            }
            "path"
                if (name == Some(index) || conditional)
                    && after.first() == Some(&Token::Symbol(b'=')) =>
            {
                path_problem(file, &after[1..])
            }
            _ => None,
        };
        findings.extend(message.map(|message| Finding::at(file, line, message)));
    }
    findings
}

/// Where a path in the source leads.
enum Target {
    /// Somewhere that is not a fixtures folder.
    Elsewhere,
    /// Into a `tests/fixtures/` folder.
    Fixtures,
    /// Nowhere the check can tell.
    Unknown,
}

/// Where the string literal `literal`, as written (with quotes) in the file `file`, leads when it
/// is the path of a module or of an `include!`.
///
/// The path is read from the folder of `file`, with `.` and `..` resolved, `\` as `/` and without
/// regard for case, as Windows and macOS read it. A path that is absolute, that climbs out of the
/// repository or that has an escape other than `\\` in it is `Unknown`. A module of the file may
/// be called `tests`, so a path that starts in a folder `fixtures` (after any `..`) leads into
/// fixtures too.
fn target(file: &str, literal: &str) -> Target {
    let Some(path) = string_value(literal) else {
        return Target::Unknown;
    };
    let Some(own) = normalise(&path) else {
        return Target::Unknown;
    };
    let Some(folder) = folder_of(file, &own) else {
        return Target::Unknown;
    };
    let first = own.iter().find(|part| *part != "..");
    let starts_in_fixtures = first.is_some_and(|part| part.eq_ignore_ascii_case("fixtures"));
    if starts_in_fixtures || is_fixture(&folder) {
        Target::Fixtures
    } else {
        Target::Elsewhere
    }
}

/// The parts of `path` (`\` read as `/`) without `.` and without the parts that a `..` takes
/// away. A `..` that has nothing before it to take away stays. `None` for an absolute path.
fn normalise(path: &str) -> Option<Vec<String>> {
    let path = path.replace('\\', "/");
    if path.starts_with('/') || path.as_bytes().get(1) == Some(&b':') {
        return None;
    }
    let mut own: Vec<String> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." if own.last().is_some_and(|last| last != "..") => {
                own.pop();
            }
            other => own.push(other.to_owned()),
        }
    }
    Some(own)
}

/// Where the parts `own` of a path lead when they are read from the folder of the file `file`:
/// the path in the repository, in lower case, with a `/` at the end. `None` when the path climbs
/// out of the repository.
fn folder_of(file: &str, own: &[String]) -> Option<String> {
    let mut resolved: Vec<&str> = file.split('/').collect();
    resolved.pop();
    for part in own {
        if part == ".." {
            resolved.pop()?;
        } else {
            resolved.push(part);
        }
    }
    Some(format!("{}/", resolved.join("/")).to_ascii_lowercase())
}

/// The number of the line of `text` that the byte `offset` is on, in the unit of a finding.
fn line_at(text: &str, offset: usize) -> u32 {
    let before = text.get(..offset).unwrap_or(text);
    line_number(before.matches('\n').count())
}

/// The text of the string literal `literal` as written, with its quotes; `None` when it has an
/// escape other than `\\`, or is not closed.
fn string_value(literal: &str) -> Option<String> {
    let open = literal.find('"')?;
    let close = literal.rfind('"').filter(|close| *close > open)?;
    let body = &literal[open + 1..close];
    if literal.starts_with('r') {
        return Some(body.to_owned());
    }
    let mut value = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            value.push(c);
        } else if chars.next()? == '\\' {
            value.push('\\');
        } else {
            return None;
        }
    }
    Some(value)
}

/// What is wrong with `path = <value>` in an attribute of the file `file`, where `value` is what
/// follows the `=`.
fn path_problem(file: &str, value: &[Token<'_>]) -> Option<&'static str> {
    match value.first() {
        Some(Token::Text(literal)) => match target(file, literal) {
            Target::Elsewhere => None,
            Target::Fixtures => Some(PATH_FIXTURES),
            Target::Unknown => Some(PATH_UNKNOWN),
        },
        _ => Some(PATH_UNKNOWN),
    }
}

/// Whether `tokens` start with `= "…"`, a string that says something.
fn has_value(tokens: &[Token<'_>]) -> bool {
    matches!(tokens, [Token::Symbol(b'='), Token::Text(text), ..] if !is_blank(text))
}

/// Whether the arguments `tokens` of an `expect(`, up to the closing bracket, have a
/// `reason = "…"` that says something.
fn has_reason(tokens: &[Token<'_>]) -> bool {
    let mut depth = 1_usize;
    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Symbol(b'(') => depth += 1,
            Token::Symbol(b')') => {
                depth -= 1;
                if depth == 0 {
                    return false;
                }
            }
            Token::Word("reason") if has_value(&tokens[index + 1..]) => return true,
            _ => {}
        }
    }
    false
}

/// Whether the string literal `literal`, as written, has nothing in it but white space.
fn is_blank(literal: &str) -> bool {
    match (literal.find('"'), literal.rfind('"')) {
        (Some(open), Some(close)) if open < close => literal[open + 1..close].trim().is_empty(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lines joined into the text of a source file.
    fn source(lines: &[&str]) -> String {
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }

    /// The findings for the source file made of `lines`.
    fn scan(lines: &[&str]) -> Vec<Finding> {
        scan_attributes("src/lib.rs", &source(lines))
    }

    fn lines_of(findings: &[Finding]) -> Vec<Option<u32>> {
        findings.iter().map(|finding| finding.line).collect()
    }

    fn paths(files: &[&str]) -> Vec<String> {
        files.iter().map(|file| (*file).to_owned()).collect()
    }

    #[test]
    fn allow_attribute_found() {
        let findings = scan(&["#[allow(dead_code)]", "fn f() {}"]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].file.as_deref(), Some("src/lib.rs"));
        assert_eq!(findings[0].line, Some(1));
        assert!(findings[0].message.contains("allow"), "{findings:?}");

        let findings = scan(&[
            "mod m {",
            "    #[allow(clippy::too_many_lines, dead_code)]",
            "    fn f() {}",
            "}",
        ]);
        assert_eq!(lines_of(&findings), [Some(2)]);
    }

    #[test]
    fn inner_allow_found() {
        let findings = scan(&["//! Docs.", "", "#![allow(missing_docs)]"]);
        assert_eq!(lines_of(&findings), [Some(3)]);
        let findings = scan(&["mod m {", "    #![allow(dead_code)]", "}"]);
        assert_eq!(lines_of(&findings), [Some(2)]);
    }

    #[test]
    fn cfg_attr_allow_found() {
        let findings = scan(&["#[cfg_attr(test, allow(dead_code))]", "fn f() {}"]);
        assert_eq!(lines_of(&findings), [Some(1)]);
        let findings = scan(&[
            "#[cfg_attr(",
            "    test,",
            "    allow(dead_code)",
            ")]",
            "fn f() {}",
        ]);
        assert_eq!(lines_of(&findings), [Some(1)]);
    }

    #[test]
    fn expect_without_reason_found() {
        for attribute in [
            "#[expect(dead_code)]",
            "#![expect(clippy::too_many_lines)]",
            "#[cfg_attr(test, expect(dead_code))]",
            "#[expect(dead_code, because = \"x\")]",
        ] {
            let findings = scan(&[attribute, "fn f() {}"]);
            assert_eq!(findings.len(), 1, "{attribute}: {findings:?}");
            assert_eq!(findings[0].line, Some(1), "{attribute}");
            assert!(findings[0].message.contains("expect"), "{attribute}");
        }
    }

    #[test]
    fn expect_with_reason_passes() {
        for attribute in [
            "#[expect(dead_code, reason = \"used by the tests\")]",
            "#![expect(clippy::too_many_lines, reason = \"one table\")]",
            "#[expect(dead_code, reason=\"no spaces\")]",
            "#[cfg_attr(test, expect(dead_code, reason = \"only tests use it\"))]",
            "#[expect(dead_code, reason = r#\"a \"raw\" reason\"#)]",
        ] {
            let findings = scan(&[attribute, "fn f() {}"]);
            assert!(findings.is_empty(), "{attribute}: {findings:?}");
        }
    }

    #[test]
    fn bare_ignore_found() {
        for attribute in [
            "#[ignore]",
            "#[cfg_attr(windows, ignore)]",
            "#[cfg_attr(not(windows), ignore)]",
        ] {
            let findings = scan(&["#[test]", attribute, "fn f() {}"]);
            assert_eq!(findings.len(), 1, "{attribute}: {findings:?}");
            assert_eq!(findings[0].line, Some(2), "{attribute}");
            assert!(findings[0].message.contains("ignore"), "{attribute}");
        }
    }

    #[test]
    fn ignore_with_reason_passes() {
        for attribute in [
            "#[ignore = \"needs the network\"]",
            "#[ignore=\"no spaces\"]",
            "#[cfg_attr(windows, ignore = \"slow on windows\")]",
            "#[cfg_attr(not(feature = \"slow\"), ignore = \"slow\")]",
        ] {
            let findings = scan(&["#[test]", attribute, "fn f() {}"]);
            assert!(findings.is_empty(), "{attribute}: {findings:?}");
        }
    }

    #[test]
    fn allow_in_doc_comment_ignored() {
        let findings = scan(&[
            "/// #[allow(dead_code)]",
            "//! #![allow(dead_code)]",
            "// #[allow(dead_code)]",
            "    /// #[allow(dead_code)]",
            "    // #[expect(dead_code)]",
            "let text = \"#[allow(dead_code)]\";",
            "fn f() {}",
        ]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn multi_line_attribute_joined() {
        // The reason is on a later line than the `expect(`.
        let findings = scan(&[
            "#[expect(",
            "    dead_code,",
            "    reason = \"used by the tests\"",
            ")]",
            "fn f() {}",
        ]);
        assert!(findings.is_empty(), "{findings:?}");

        // No reason on any of the lines: the finding is at the first line.
        let findings = scan(&["fn a() {}", "#[expect(", "    dead_code", ")]", "fn f() {}"]);
        assert_eq!(lines_of(&findings), [Some(2)]);

        // A bracket in a string does not end the attribute early.
        let findings = scan(&[
            "#[doc = \"a ] bracket\"]",
            "#[cfg_attr(",
            "    test,",
            "    doc = \"another ] one\",",
            "    allow(dead_code)",
            ")]",
        ]);
        assert_eq!(lines_of(&findings), [Some(2)]);

        // The next attribute is read from its own line.
        let findings = scan(&[
            "#[cfg_attr(",
            "    test,",
            "    derive(Debug)",
            ")]",
            "#[allow(dead_code)]",
            "struct S;",
        ]);
        assert_eq!(lines_of(&findings), [Some(5)]);
    }

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
    fn words_are_not_substrings() {
        let findings = scan(&[
            "#[disallow(x)]",
            "#[allow_internal_unstable(x)]",
            "#[should_panic(expected = \"x\")]",
            "#[cfg_attr(test, expected(x))]",
            "#[cfg(feature = \"ignore\")]",
            "#[derive(Debug)]",
            "#[doc = \"allow(x) and expect(y) and ignore\"]",
            "#[expect(dead_code, reason = \"the allow(x) rule\")]",
        ]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn comments_inside_an_attribute_do_not_count() {
        // A `reason` in a comment is no reason.
        let findings = scan(&[
            "#[expect(",
            "    dead_code, // reason = \"x\"",
            ")]",
            "#[expect(dead_code, /* reason = \"y\" */)]",
        ]);
        assert_eq!(lines_of(&findings), [Some(1), Some(4)]);
        // And an `allow(` in a comment is no allow.
        let findings = scan(&[
            "#[expect(",
            "    dead_code, // allow(x) would not do",
            "    reason = \"r\"",
            ")]",
        ]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_reason_must_say_something() {
        for attribute in [
            "#[expect(dead_code, reason = \"\")]",
            "#[expect(dead_code, reason = \"   \")]",
            "#[expect(dead_code, reason)]",
            "#[expect(dead_code, reason = reason)]",
            "#[ignore = \"\"]",
            "#[ignore = \" \"]",
            "#[cfg_attr(windows, ignore = \"\")]",
        ] {
            let findings = scan(&[attribute, "fn f() {}"]);
            assert_eq!(findings.len(), 1, "{attribute}: {findings:?}");
        }
    }

    #[test]
    fn every_attribute_on_a_line_is_read() {
        let findings = scan(&[
            "#[test] #[ignore] fn a() {}",
            "#[derive(Debug)] #[allow(dead_code)] struct S;",
            "#[test] #[ignore = \"slow\"] fn b() {}",
            "#[cfg_attr(test, derive(Debug))] #[expect(dead_code)]",
        ]);
        assert_eq!(lines_of(&findings), [Some(1), Some(2), Some(4)]);
    }

    #[test]
    fn an_attribute_that_is_never_closed_is_still_read() {
        let findings = scan(&["fn a() {}", "#[allow(dead_code"]);
        assert_eq!(lines_of(&findings), [Some(2)]);
    }

    #[test]
    fn findings_are_in_file_order_with_their_lines() {
        let findings = scan(&[
            "#[allow(a)]",
            "fn a() {}",
            "",
            "#[expect(b)]",
            "fn b() {}",
            "#[test]",
            "#[ignore]",
            "fn c() {}",
        ]);
        assert_eq!(lines_of(&findings), [Some(1), Some(4), Some(7)]);
    }

    #[test]
    fn crlf_line_ends_do_not_matter() {
        let findings = scan_attributes("a.rs", "#[allow(dead_code)]\r\nfn f() {}\r\n");
        assert_eq!(lines_of(&findings), [Some(1)]);
        let text = "#[expect(\r\n    dead_code,\r\n    reason = \"x\"\r\n)]\r\nfn f() {}\r\n";
        let findings = scan_attributes("a.rs", text);
        assert!(findings.is_empty(), "{findings:?}");
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

    /// The findings of `attribute` above a module, in the file `file`.
    fn path_findings(file: &str, attribute: &str) -> Vec<Finding> {
        scan_attributes(file, &source(&[attribute, "mod a;"]))
    }

    /// The findings of the include scan for the source file made of `lines`.
    fn includes(file: &str, lines: &[&str]) -> Vec<Finding> {
        scan_includes(file, &source(lines))
    }

    #[test]
    fn path_attribute_into_fixtures_found() {
        for (file, attribute) in [
            // The path is read from the folder of the file.
            ("src/lib.rs", "#[path = \"../tests/fixtures/a.rs\"]"),
            (
                "crates/a/src/lib.rs",
                "#[path = \"../tests/./fixtures/a.rs\"]",
            ),
            ("tests/main.rs", "#[path = \"fixtures/a.rs\"]"),
            (
                "crates/a/tests/main.rs",
                "#[path = \"fixtures/deep/er/a.rs\"]",
            ),
            (
                "crates/a/tests/main.rs",
                "#[path = \"helpers/../fixtures/a.rs\"]",
            ),
            ("src/lib.rs", "#![path = \"../tests/fixtures/a.rs\"]"),
            (
                "src/lib.rs",
                "#[cfg_attr(windows, path = \"../tests/fixtures/a.rs\")]",
            ),
            // Backslashes, raw strings and case: Windows and macOS read all of these.
            (
                "src/lib.rs",
                "#[path = \"..\\\\tests\\\\fixtures\\\\a.rs\"]",
            ),
            ("src/lib.rs", "#[path = r\"..\\tests\\fixtures\\a.rs\"]"),
            ("src/lib.rs", "#[path = \"../Tests/Fixtures/a.rs\"]"),
            // A module in the file may be called `tests`, so a path that starts in `fixtures` may
            // lead into `tests/fixtures`.
            ("src/lib.rs", "#[path = \"fixtures/a.rs\"]"),
            // What cannot be told is a finding too.
            (
                "src/lib.rs",
                "#[path = concat!(\"../tests/\", \"fixtures/a.rs\")]",
            ),
            ("src/lib.rs", "#[path = \"/abs/tests/fixtures/a.rs\"]"),
            ("src/lib.rs", "#[path = \"C:/repo/tests/fixtures/a.rs\"]"),
            ("src/lib.rs", "#[path = \"../../tests/fixtures/a.rs\"]"),
            ("src/lib.rs", "#[path = \"a\\x2f../tests/fixtures/a.rs\"]"),
            ("src/lib.rs", "#[path = b\"../tests/fixtures/a.rs\"]"),
        ] {
            let findings = path_findings(file, attribute);
            assert_eq!(findings.len(), 1, "{file}: {attribute}: {findings:?}");
            assert_eq!(findings[0].line, Some(1), "{attribute}");
            assert!(findings[0].message.contains("path"), "{attribute}");
        }

        // On several lines, the finding is at the first.
        let findings = scan_attributes(
            "src/lib.rs",
            &source(&[
                "mod b;",
                "#[path =",
                "    \"../tests/fixtures/a.rs\"",
                "]",
                "mod a;",
            ]),
        );
        assert_eq!(lines_of(&findings), [Some(2)]);
    }

    #[test]
    fn path_attribute_elsewhere_allowed() {
        for (file, attribute) in [
            ("src/lib.rs", "#[path = \"helpers.rs\"]"),
            ("src/lib.rs", "#[path = \"../src/other.rs\"]"),
            ("crates/a/tests/main.rs", "#[path = \"common/mod.rs\"]"),
            (
                "crates/a/tests/main.rs",
                "#[path = \"../src/generated.rs\"]",
            ),
            // `..` leaves the folder again.
            (
                "src/lib.rs",
                "#[path = \"../tests/fixtures/../common/mod.rs\"]",
            ),
            ("src/lib.rs", "#[path = \"fixtures/../helpers.rs\"]"),
            ("src/lib.rs", "#[cfg_attr(windows, path = \"windows.rs\")]"),
            // Words that are not the attribute `path`.
            ("src/lib.rs", "#[cfg(feature = \"path\")]"),
            ("src/lib.rs", "#[doc(alias = \"path\")]"),
            (
                "src/lib.rs",
                "#[cfg_attr(test, derive(Debug), my_attr(path))]",
            ),
            ("src/lib.rs", "#[derive(Debug)]"),
        ] {
            let findings = path_findings(file, attribute);
            assert!(findings.is_empty(), "{file}: {attribute}: {findings:?}");
        }
    }

    #[test]
    fn include_macro_into_fixtures_found() {
        for line in [
            "include!(\"../tests/fixtures/a.rs\");",
            "include![\"../tests/fixtures/a.rs\"];",
            "include!{\"../tests/fixtures/a.rs\"}",
            "include ! ( \"../tests/fixtures/a.rs\" );",
            "include!(\"../tests/fixtures/a.rs\",);",
            "std::include!(\"../tests/fixtures/a.rs\");",
            "fn f() { include!(\"../tests/fixtures/a.rs\"); }",
            "include!(\"..\\\\tests\\\\fixtures\\\\a.rs\");",
            "include!(r\"..\\tests\\fixtures\\a.rs\");",
            "include!(\"../tests/./fixtures/../fixtures/a.rs\");",
            // What cannot be told is a finding too: nothing is compiled that was not looked at.
            "include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/tests/fixtures/a.rs\"));",
            "include!(env!(\"OUT_DIR\"));",
            "include!(PATH);",
            "include!();",
            "include!(\"a.rs\", \"b.rs\");",
            "include!(\"/abs/a.rs\");",
            "include!(\"../../../a.rs\");",
        ] {
            let findings = includes("src/lib.rs", &["fn a() {}", "", line]);
            assert_eq!(findings.len(), 1, "{line}: {findings:?}");
            assert_eq!(findings[0].file.as_deref(), Some("src/lib.rs"), "{line}");
            assert_eq!(findings[0].line, Some(3), "{line}");
            assert!(findings[0].message.contains("include!"), "{line}");
        }

        // The path is read from the folder of the file.
        let findings = includes("tests/main.rs", &["include!(\"fixtures/a.rs\");"]);
        assert_eq!(lines_of(&findings), [Some(1)]);
        let findings = includes("crates/a/tests/main.rs", &["include!(\"fixtures/a.rs\");"]);
        assert_eq!(lines_of(&findings), [Some(1)]);

        // Every include of a file is found, on the line it starts.
        let findings = includes(
            "src/lib.rs",
            &[
                "include!(\"../tests/fixtures/a.rs\");",
                "include!(\"b.rs\");",
                "include!(",
                "    \"../tests/fixtures/c.rs\"",
                ");",
            ],
        );
        assert_eq!(lines_of(&findings), [Some(1), Some(3)]);
    }

    #[test]
    fn include_str_of_fixture_allowed() {
        let findings = includes(
            "src/lib.rs",
            &[
                "const A: &str = include_str!(\"../tests/fixtures/a.txt\");",
                "const B: &[u8] = include_bytes!(\"../tests/fixtures/a.bin\");",
                "const C: &str = include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/tests/fixtures/a.txt\"));",
                // Other files may be included as code.
                "include!(\"helpers.rs\");",
                "include!(\"../src/generated.rs\");",
                "include!(\"../tests/fixtures/../common.rs\");",
                // Not an invocation: comments, strings, other words.
                "// include!(\"../tests/fixtures/a.rs\");",
                "/// include!(\"../tests/fixtures/a.rs\");",
                "/* include!(\"../tests/fixtures/a.rs\"); */",
                "let text = \"include!(\\\"../tests/fixtures/a.rs\\\")\";",
                "let include = 1;",
                "let same = include != 2;",
                "fn include() {}",
                "macro_rules! include { () => {} }",
            ],
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// The findings of the module scan for the source file made of `lines`.
    fn modules(lines: &[&str]) -> Vec<Finding> {
        scan_fixture_modules("tests/main.rs", &source(lines))
    }

    #[test]
    fn mod_fixtures_declaration_found() {
        for line in [
            "mod fixtures;",
            "pub mod fixtures;",
            "pub(crate) mod fixtures;",
            "pub(in crate::a) mod fixtures;",
            "#[cfg(test)] mod fixtures;",
            "mod  fixtures ;",
            "mod tests { mod fixtures; }",
            "macro_rules! m { () => { mod fixtures; }; }",
            // The file system may not tell the case, and `r#` names the same module.
            "mod Fixtures;",
            "mod FIXTURES;",
            "mod r#fixtures;",
        ] {
            let findings = modules(&["fn a() {}", line]);
            assert_eq!(findings.len(), 1, "{line}: {findings:?}");
            assert_eq!(findings[0].file.as_deref(), Some("tests/main.rs"), "{line}");
            assert_eq!(findings[0].line, Some(2), "{line}");
            assert!(findings[0].message.contains("fixtures"), "{line}");
        }

        // Across lines, and every declaration of a file on the line of its `mod`.
        let findings = modules(&[
            "mod fixtures;",
            "mod other;",
            "mod",
            "    fixtures",
            ";",
            "pub mod fixtures;",
        ]);
        let lines: Vec<Option<u32>> = findings.iter().map(|finding| finding.line).collect();
        assert_eq!(lines, [Some(1), Some(3), Some(6)]);
    }

    #[test]
    fn mod_fixtures_that_is_not_a_declaration_allowed() {
        let findings = modules(&[
            "mod fixtures { fn helper() {} }",
            "mod fixture;",
            "mod fixtures_extra;",
            "mod my_fixtures;",
            "mod tests;",
            "use crate::fixtures;",
            "use tests::fixtures as other;",
            "let fixtures = 1;",
            "// mod fixtures;",
            "/// mod fixtures;",
            "/* mod fixtures; */",
            "let text = \"mod fixtures;\";",
        ]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn fixtures_main_rs_found() {
        for path in [
            "tests/fixtures/main.rs",
            "xtask/tests/fixtures/main.rs",
            "crates/a/tests/fixtures/mod.rs",
            "a/b/tests/fixtures/main.rs",
            // The file system may not tell the case.
            "crates/a/tests/Fixtures/Main.rs",
            "crates/a/Tests/fixtures/MOD.RS",
        ] {
            let findings = fixture_entry_points(&paths(&[path]));
            assert_eq!(findings.len(), 1, "{path}: {findings:?}");
            assert_eq!(findings[0].file.as_deref(), Some(path), "{path}");
        }
    }

    #[test]
    fn deeper_fixture_lib_rs_allowed() {
        let findings = fixture_entry_points(&paths(&[
            // Below a case folder of the fixtures nothing is discovered or declared.
            "xtask/tests/fixtures/lints/clean/crates/a/src/lib.rs",
            "xtask/tests/fixtures/lints/clean/src/main.rs",
            "crates/a/tests/fixtures/case/main.rs",
            "crates/a/tests/fixtures/case/mod.rs",
            "crates/a/tests/fixtures/case/src/lib.rs",
            // Other files in the fixtures folder itself.
            "crates/a/tests/fixtures/lib.rs",
            "crates/a/tests/fixtures/data.rs",
            // And the same names elsewhere.
            "crates/a/src/main.rs",
            "crates/a/tests/main.rs",
            "crates/a/tests/mod.rs",
            "crates/a/fixtures/main.rs",
            "fixtures/mod.rs",
            "crates/a/tests/fixtures.rs",
        ]));
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn manifest_path_into_fixtures_found() -> crate::Result<()> {
        for (file, text) in [
            ("Cargo.toml", "[lib]\npath = \"tests/fixtures/x/lib.rs\"\n"),
            (
                "crates/a/Cargo.toml",
                "[[bin]]\nname = \"x\"\npath = \"tests/fixtures/x.rs\"\n",
            ),
            (
                "crates/a/Cargo.toml",
                "[[test]]\nname = \"x\"\npath = \"tests/fixtures/x.rs\"\n",
            ),
            (
                "crates/a/Cargo.toml",
                "[[example]]\nname = \"x\"\npath = \"tests/fixtures/x.rs\"\n",
            ),
            (
                "crates/a/Cargo.toml",
                "[[bench]]\nname = \"x\"\npath = \"tests/fixtures/x.rs\"\n",
            ),
            (
                "crates/a/Cargo.toml",
                "[dependencies]\nx = { path = \"tests/fixtures/x\" }\n",
            ),
            (
                "crates/a/Cargo.toml",
                "[dependencies.x]\npath = \"tests/fixtures/x\"\n",
            ),
            (
                "crates/a/Cargo.toml",
                "[dev-dependencies]\nx = { path = \"tests/fixtures/x\" }\n",
            ),
            (
                "crates/a/Cargo.toml",
                "[build-dependencies]\nx = { path = \"tests/fixtures/x\" }\n",
            ),
            (
                "crates/a/Cargo.toml",
                "[target.'cfg(windows)'.dependencies]\nx = { path = \"tests/fixtures/x\" }\n",
            ),
            (
                "Cargo.toml",
                "[workspace.dependencies]\nx = { path = \"xtask/tests/fixtures/x\" }\n",
            ),
            (
                "Cargo.toml",
                "[patch.crates-io]\nx = { path = \"xtask/tests/fixtures/x\" }\n",
            ),
            // The path is read from the folder of the manifest: `..`, `\`, case.
            (
                "crates/a/Cargo.toml",
                "[dependencies]\nx = { path = \"../../xtask/tests/fixtures/x\" }\n",
            ),
            (
                "crates/a/Cargo.toml",
                "[dependencies]\nx = { path = \"..\\\\..\\\\xtask\\\\tests\\\\fixtures\\\\x\" }\n",
            ),
            (
                "crates/a/Cargo.toml",
                "[dependencies]\nx = { path = 'Tests/Fixtures/x' }\n",
            ),
            // The build script and the members of a workspace are built from paths too.
            (
                "crates/a/Cargo.toml",
                "[package]\nbuild = \"tests/fixtures/build.rs\"\n",
            ),
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"crates/*\", \"xtask/tests/fixtures/*\"]\n",
            ),
            (
                "Cargo.toml",
                "[workspace]\ndefault-members = [\"xtask/tests/fixtures/x\"]\n",
            ),
        ] {
            let manifest = parse(file, text)?;
            assert!(
                manifest_leads_into_fixtures(file, &manifest),
                "{file}: {text}"
            );
        }

        for (file, text) in [
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"crates/*\", \"xtask\"]\n",
            ),
            ("crates/a/Cargo.toml", "[lib]\npath = \"src/lib.rs\"\n"),
            (
                "crates/a/Cargo.toml",
                "[[test]]\nname = \"x\"\npath = \"tests/common/x.rs\"\n",
            ),
            (
                "crates/a/Cargo.toml",
                "[dependencies]\nb = { path = \"../b\" }\nserde = \"1\"\n",
            ),
            // `..` leaves the folder again.
            (
                "crates/a/Cargo.toml",
                "[dependencies]\nx = { path = \"tests/fixtures/../common\" }\n",
            ),
            // A path that goes elsewhere, outside the repository too (engine crates), is not ours.
            (
                "crates/a/Cargo.toml",
                "[dependencies]\nx = { path = \"../../../Theos-Engine/crates/x\" }\n",
            ),
            (
                "crates/a/Cargo.toml",
                "[dependencies]\nx = { path = \"/abs/tests/fixtures/x\" }\n",
            ),
            ("crates/a/Cargo.toml", "[package]\nbuild = \"build.rs\"\n"),
            ("crates/a/Cargo.toml", "[package]\nbuild = false\n"),
            (
                "crates/a/Cargo.toml",
                "[package.metadata]\nnote = \"tests/fixtures/x\"\n",
            ),
        ] {
            let manifest = parse(file, text)?;
            assert!(
                !manifest_leads_into_fixtures(file, &manifest),
                "{file}: {text}"
            );
        }
        Ok(())
    }
}
