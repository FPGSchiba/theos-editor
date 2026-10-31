//! How verdicts reach people: a table on the output, workflow annotations and a step summary.
//!
//! Locally only the table is written. Under GitHub Actions [`emit`] also writes one `::error`
//! annotation per finding and appends a markdown summary to `$GITHUB_STEP_SUMMARY`.

use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::verdict::{Check, Finding, Verdict};
use crate::{Error, Result};

/// Writes one line per check, `pass` or `FAIL` and the check's name, and under each failing
/// check its findings as `  path:line: message`. Notes follow as `  note: …`.
///
/// # Errors
///
/// Fails when `out` cannot be written.
pub fn write_table(verdicts: &[Verdict], out: &mut dyn Write) -> io::Result<()> {
    for verdict in verdicts {
        let status = if verdict.passed() { "pass" } else { "FAIL" };
        writeln!(out, "{status}  {}", verdict.check.name())?;
        for finding in &verdict.findings {
            match location(finding) {
                Some(place) => writeln!(out, "  {place}: {}", finding.message)?,
                None => writeln!(out, "  {}", finding.message)?,
            }
        }
        for note in &verdict.notes {
            let mut lines = note.lines();
            if let Some(first) = lines.next() {
                writeln!(out, "  note: {first}")?;
            }
            for line in lines {
                writeln!(out, "        {line}")?;
            }
        }
    }
    Ok(())
}

/// The workflow command that shows `finding` on the pull request:
/// `::error file=<file>,line=<line>,title=<check>::<message>`.
///
/// Without a file the command is `::error title=<check>::<message>`. `%`, CR and LF in the
/// message are escaped as `%25`, `%0D` and `%0A`; the file also has `:` and `,` escaped, which
/// would otherwise end its property.
#[must_use]
pub fn annotation(check: Check, finding: &Finding) -> String {
    let mut command = String::from("::error ");
    if let Some(file) = &finding.file {
        command.push_str("file=");
        command.push_str(&escape_property(file));
        command.push(',');
        if let Some(line) = finding.line {
            // Writing to a `String` cannot fail.
            let _ = write!(command, "line={line},");
        }
    }
    command.push_str("title=");
    command.push_str(check.name());
    command.push_str("::");
    command.push_str(&escape_data(&finding.message));
    command
}

/// The verdicts as markdown: a table with a row per check, then a section for every check that
/// has findings or notes. This is what goes to `$GITHUB_STEP_SUMMARY`.
#[must_use]
pub fn summary_markdown(verdicts: &[Verdict]) -> String {
    let mut text = String::from("### xtask\n\n| Check | Result |\n|---|---|\n");
    for verdict in verdicts {
        let status = if verdict.passed() { "pass" } else { "**FAIL**" };
        // Writing to a `String` cannot fail.
        let _ = writeln!(text, "| `{}` | {status} |", verdict.check.name());
    }
    text.push('\n');
    for verdict in verdicts {
        if verdict.findings.is_empty() && verdict.notes.is_empty() {
            continue;
        }
        let _ = write!(text, "#### `{}`\n\n", verdict.check.name());
        for finding in &verdict.findings {
            let message = finding.message.lines().collect::<Vec<_>>().join(" ");
            match location(finding) {
                Some(place) => {
                    let _ = writeln!(text, "- `{place}`: {message}");
                }
                None => {
                    let _ = writeln!(text, "- {message}");
                }
            }
        }
        if !verdict.findings.is_empty() {
            text.push('\n');
        }
        if !verdict.notes.is_empty() {
            text.push_str("```text\n");
            for note in &verdict.notes {
                text.push_str(note);
                text.push('\n');
            }
            text.push_str("```\n\n");
        }
    }
    text
}

/// Writes the verdict to `out`, and under GitHub Actions (`ci`) also its annotations and the
/// summary appended to the file named by `$GITHUB_STEP_SUMMARY`.
///
/// # Errors
///
/// Fails when `out` or the summary file cannot be written.
pub fn emit(verdict: &Verdict, ci: bool, out: &mut dyn Write) -> Result<()> {
    let summary = std::env::var_os("GITHUB_STEP_SUMMARY")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    emit_with(verdict, ci, summary.as_deref(), out)
}

fn emit_with(
    verdict: &Verdict,
    ci: bool,
    summary: Option<&Path>,
    out: &mut dyn Write,
) -> Result<()> {
    let verdicts = std::slice::from_ref(verdict);
    write_table(verdicts, out).map_err(output_error)?;
    if ci {
        for finding in &verdict.findings {
            writeln!(out, "{}", annotation(verdict.check, finding)).map_err(output_error)?;
        }
        if let Some(path) = summary {
            append(path, &summary_markdown(verdicts))?;
        }
    }
    Ok(())
}

fn append(path: &Path, text: &str) -> Result<()> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })
}

fn output_error(source: io::Error) -> Error {
    Error::Io {
        path: PathBuf::from("<output>"),
        source,
    }
}

/// `file:line`, or `file`, or nothing when the finding has no file.
fn location(finding: &Finding) -> Option<String> {
    let file = finding.file.as_ref()?;
    Some(match finding.line {
        Some(line) => format!("{file}:{line}"),
        None => file.clone(),
    })
}

fn escape_data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

fn escape_property(text: &str) -> String {
    escape_data(text).replace(':', "%3A").replace(',', "%2C")
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn failing() -> Verdict {
        Verdict::with(
            Check::Lints,
            vec![
                Finding::at("crates/a/src/lib.rs", 3, "allow attribute"),
                Finding::new("no clippy.toml"),
            ],
        )
    }

    fn table(verdicts: &[Verdict]) -> TestResult<String> {
        let mut out = Vec::new();
        write_table(verdicts, &mut out)?;
        Ok(String::from_utf8(out)?)
    }

    #[test]
    fn annotation_escapes_newlines_and_percent() {
        let finding = Finding::at("a.rs", 2, "a%b\nc\rd");
        assert_eq!(
            annotation(Check::Lints, &finding),
            "::error file=a.rs,line=2,title=lints::a%25b%0Ac%0Dd"
        );
    }

    #[test]
    fn annotation_without_file() {
        assert_eq!(
            annotation(Check::PrTitle, &Finding::new("missing scope")),
            "::error title=pr-title::missing scope"
        );
    }

    #[test]
    fn annotation_without_line_names_only_the_file() {
        let finding = Finding {
            file: Some("Cargo.toml".to_owned()),
            line: None,
            message: "no version".to_owned(),
        };
        assert_eq!(
            annotation(Check::Lints, &finding),
            "::error file=Cargo.toml,title=lints::no version"
        );
    }

    #[test]
    fn annotation_escapes_separators_in_the_file() {
        let finding = Finding::at("odd,name:x.rs", 1, "m");
        assert_eq!(
            annotation(Check::Lints, &finding),
            "::error file=odd%2Cname%3Ax.rs,line=1,title=lints::m"
        );
    }

    #[test]
    fn table_lists_failures_with_locations() -> TestResult {
        let text = table(&[Verdict::pass(Check::Fmt), failing()])?;
        assert_eq!(
            text,
            "pass  fmt\nFAIL  lints\n  crates/a/src/lib.rs:3: allow attribute\n  no clippy.toml\n"
        );
        Ok(())
    }

    #[test]
    fn table_shows_a_finding_with_only_a_file() -> TestResult {
        let finding = Finding {
            file: Some("Cargo.toml".to_owned()),
            line: None,
            message: "no version".to_owned(),
        };
        let text = table(&[Verdict::with(Check::Lints, vec![finding])])?;
        assert_eq!(text, "FAIL  lints\n  Cargo.toml: no version\n");
        Ok(())
    }

    #[test]
    fn table_shows_notes_under_their_check() -> TestResult {
        let mut verdict = Verdict::pass(Check::AcTrace);
        verdict
            .notes
            .push("CE-08.1\n  workspace_members".to_owned());
        let text = table(&[verdict])?;
        assert_eq!(
            text,
            "pass  ac-trace\n  note: CE-08.1\n          workspace_members\n"
        );
        Ok(())
    }

    #[test]
    fn summary_markdown_has_a_row_per_check_and_a_section_per_finding_list() {
        let text = summary_markdown(&[Verdict::pass(Check::Fmt), failing()]);
        assert_eq!(
            text,
            "### xtask\n\
             \n\
             | Check | Result |\n\
             |---|---|\n\
             | `fmt` | pass |\n\
             | `lints` | **FAIL** |\n\
             \n\
             #### `lints`\n\
             \n\
             - `crates/a/src/lib.rs:3`: allow attribute\n\
             - no clippy.toml\n\
             \n"
        );
    }

    #[test]
    fn summary_markdown_puts_notes_in_a_text_block() {
        let mut verdict = Verdict::pass(Check::AcTrace);
        verdict.notes.push("CE-08.1 -> a_test".to_owned());
        let text = summary_markdown(&[verdict]);
        assert!(
            text.contains("#### `ac-trace`\n\n```text\nCE-08.1 -> a_test\n```\n"),
            "{text}"
        );
    }

    #[test]
    fn summary_markdown_keeps_a_finding_on_one_line() {
        let text = summary_markdown(&[Verdict::with(
            Check::PrBody,
            vec![Finding::new("first\nsecond")],
        )]);
        assert!(text.contains("- first second\n"), "{text}");
    }

    #[test]
    fn emit_outside_ci_writes_the_table_only() -> TestResult {
        let summary = tempfile::NamedTempFile::new()?;
        let mut out = Vec::new();
        emit_with(&failing(), false, Some(summary.path()), &mut out)?;
        assert_eq!(
            String::from_utf8(out)?,
            "FAIL  lints\n  crates/a/src/lib.rs:3: allow attribute\n  no clippy.toml\n"
        );
        assert_eq!(std::fs::read_to_string(summary.path())?, "");
        Ok(())
    }

    #[test]
    fn emit_in_ci_adds_annotations_and_appends_the_summary() -> TestResult {
        let summary = tempfile::NamedTempFile::new()?;
        std::fs::write(summary.path(), "earlier step\n")?;
        let mut out = Vec::new();
        emit_with(&failing(), true, Some(summary.path()), &mut out)?;
        let text = String::from_utf8(out)?;
        assert!(text.starts_with("FAIL  lints\n"), "{text}");
        assert!(
            text.ends_with(
                "::error file=crates/a/src/lib.rs,line=3,title=lints::allow attribute\n\
                 ::error title=lints::no clippy.toml\n"
            ),
            "{text}"
        );
        let written = std::fs::read_to_string(summary.path())?;
        assert!(
            written.starts_with("earlier step\n### xtask\n"),
            "{written}"
        );
        assert!(written.contains("| `lints` | **FAIL** |"), "{written}");
        Ok(())
    }

    #[test]
    fn emit_in_ci_without_a_summary_file_still_annotates() -> TestResult {
        let mut out = Vec::new();
        emit_with(&failing(), true, None, &mut out)?;
        assert!(String::from_utf8(out)?.contains("::error "));
        Ok(())
    }

    #[test]
    fn emit_reports_an_unwritable_summary_as_an_io_error() -> TestResult {
        let dir = tempfile::tempdir()?;
        let mut out = Vec::new();
        let result = emit_with(&failing(), true, Some(dir.path()), &mut out);
        assert!(matches!(result, Err(crate::Error::Io { .. })), "{result:?}");
        Ok(())
    }
}
