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
//!   `reason = "…"` or an `ignore` without `= "…"` ([`scan_attributes`]).
//!
//! Files under `tests/fixtures/` are test data with deliberate violations: the check reads none
//! of them as part of the repository, the same files that `ac-trace` leaves alone.
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
use super::ac_trace::{extension, is_fixture, is_scanned, line_number};
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
/// and has no `= "…"` with something in it. Only whole words count, and words in comments and
/// strings do not.
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
fn parse(what: &str, text: &str) -> Result<toml::Table> {
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
        match inherits_lints(path, &read(root, path)?) {
            Ok(true) => {}
            Ok(false) => findings.push(at_file(
                path,
                "has no `[lints]` with `workspace = true`, so the crate does not get the \
                 workspace lints",
            )),
            // `lint_tables` has read the root manifest too, and reports it.
            Err(Error::Parse { .. }) if path == ROOT_MANIFEST => {}
            Err(Error::Parse { message, .. }) => findings.push(at_file(path, message)),
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// Whether the manifest `text` of the file `path` has `[lints]` with `workspace = true`. The root
/// manifest of a workspace that is not a package has no crate to give lints to, and passes.
fn inherits_lints(path: &str, text: &str) -> Result<bool> {
    let manifest = parse(path, text)?;
    if path == ROOT_MANIFEST && !manifest.contains_key("package") {
        return Ok(true);
    }
    let workspace = manifest
        .get("lints")
        .and_then(|lints| lints.get("workspace"))
        .and_then(toml::Value::as_bool);
    Ok(workspace == Some(true))
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

/// The attribute findings of every Rust source that is scanned.
fn sources(root: &Path, files: &[String], findings: &mut Vec<Finding>) -> Result<()> {
    for path in files
        .iter()
        .filter(|path| extension(path) == Some("rs") && is_scanned(path))
    {
        let full = root.join(path);
        let bytes = fs::read(&full).map_err(|source| Error::Io { path: full, source })?;
        findings.extend(scan_attributes(path, &String::from_utf8_lossy(&bytes)));
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
                tokens.push(Token::Text(literal));
            }
            i = end;
        } else if is_word(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_word(bytes[i]) {
                i += 1;
            }
            tokens.push(Token::Word(&text[start..i]));
        } else {
            if bytes[i].is_ascii() {
                tokens.push(Token::Symbol(bytes[i]));
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
            _ => None,
        };
        findings.extend(message.map(|message| Finding::at(file, line, message)));
    }
    findings
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
}
