//! The ways to compile a fixture, which no check reads: a `#[path]` or an `include!` that leads
//! into a `tests/fixtures/` folder, a `mod fixtures;` declaration, a `main.rs` or `mod.rs` right in
//! such a folder, and a `path` in a manifest that leads there. Each is a finding, because what it
//! builds would be built unseen.

use super::at_file;
use super::tokens::{Token, located_tokens};
use crate::checks::ac_trace::{is_fixture, line_number};
use crate::verdict::Finding;

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
pub(super) const MANIFEST_FIXTURES: &str = "has a `path` that leads into a `tests/fixtures/` \
                                            folder: fixtures are test data that no check reads, \
                                            so no crate is built from them";

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
pub(super) fn fixture_entry_points(files: &[String]) -> Vec<Finding> {
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
pub(super) fn manifest_leads_into_fixtures(file: &str, manifest: &toml::Table) -> bool {
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
pub(super) fn path_problem(file: &str, value: &[Token<'_>]) -> Option<&'static str> {
    match value.first() {
        Some(Token::Text(literal)) => match target(file, literal) {
            Target::Elsewhere => None,
            Target::Fixtures => Some(PATH_FIXTURES),
            Target::Unknown => Some(PATH_UNKNOWN),
        },
        _ => Some(PATH_UNKNOWN),
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{lines_of, paths, source};
    use super::super::{parse, scan_attributes};
    use super::*;

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
