//! Conventional Commit messages, as `CONTRIBUTING.md` §2 defines them.
//!
//! The parser is pure text in, values out, so the `commit-msg` check (every commit of a pull
//! request) and the `pr-title` check (the title, which becomes the squash commit) share it.
//! Parsing and judging are separate: [`parse_header`] and [`parse_message`] accept any type and
//! scope word and only fail on the shape; [`header_problems`] and [`message_problems`] list
//! everything the rules forbid, one sentence per problem.

use std::collections::BTreeSet;

use crate::branch::{BranchKind, Phase, phase_of};

/// The commit types of `CONTRIBUTING.md` §2.
pub const TYPES: [&str; 10] = [
    "feat", "fix", "perf", "refactor", "test", "docs", "build", "ci", "chore", "revert",
];

/// The commit scopes of `CONTRIBUTING.md` §2.
pub const SCOPES: [&str; 14] = [
    "content",
    "vcs",
    "studio",
    "cli",
    "mcp",
    "editor",
    "ui",
    "generators",
    "xtask",
    "gate",
    "spec",
    "docs",
    "design",
    "release",
];

/// The first line of a commit: `type(scope)!: summary`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Header {
    /// The type word, as written; [`header_problems`] says whether it is one of [`TYPES`].
    pub kind: String,
    /// The word in parentheses, as written, if there are any.
    pub scope: Option<String>,
    /// Whether there is a `!` before the colon.
    pub breaking: bool,
    /// What follows the colon and its one space.
    pub summary: String,
}

/// Reads `type(scope)!: summary`. The scope and the `!` are optional, exactly one space follows
/// the colon, and the type is a word of ASCII letters, digits and hyphens.
///
/// Any such type and scope are accepted here; whether they are allowed is for
/// [`header_problems`].
///
/// # Errors
///
/// Fails, with a sentence saying what is wrong, when the line does not have that shape.
pub fn parse_header(line: &str) -> std::result::Result<Header, String> {
    let not_a_header = |why: &str| {
        let hint = hint_for(line);
        format!("the header is not `type(scope)!: summary`: {why}{hint}")
    };
    let Some(kind_end) = line.find(['(', '!', ':']) else {
        return Err(not_a_header("there is no `:`"));
    };
    let kind = &line[..kind_end];
    if kind.is_empty() {
        return Err(not_a_header("the type is missing"));
    }
    if !kind.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(not_a_header(
            "the type is one word of letters, digits and hyphens",
        ));
    }
    let mut rest = &line[kind_end..];
    let mut scope = None;
    if let Some(after) = rest.strip_prefix('(') {
        let Some((word, tail)) = after.split_once(')') else {
            return Err(not_a_header("the scope is not closed with `)`"));
        };
        if word.is_empty() {
            return Err(not_a_header("the scope in parentheses is empty"));
        }
        scope = Some(word.to_owned());
        rest = tail;
    }
    let breaking = rest.starts_with('!');
    if breaking {
        rest = &rest[1..];
    }
    let Some(after_colon) = rest.strip_prefix(':') else {
        return Err(not_a_header(
            "`:` must follow the type, the scope and the `!`",
        ));
    };
    let Some(summary) = after_colon.strip_prefix(' ') else {
        return Err(not_a_header("exactly one space must follow the colon"));
    };
    if summary.starts_with(char::is_whitespace) {
        return Err(not_a_header("exactly one space must follow the colon"));
    }
    if summary.is_empty() {
        return Err(not_a_header("the summary is empty"));
    }
    Ok(Header {
        kind: kind.to_owned(),
        scope,
        breaking,
        summary: summary.to_owned(),
    })
}

/// What to add, beyond the shape, for the headers git and its tools write by themselves.
fn hint_for(line: &str) -> &'static str {
    if ["fixup! ", "squash! ", "amend! "]
        .iter()
        .any(|prefix| line.starts_with(prefix))
    {
        " (autosquash commits, `fixup!` and `squash!`, must be squashed into the commit they fix)"
    } else if line.starts_with("Revert \"") {
        " (write a revert as `revert(scope): summary`, not with git's own `Revert \"…\"` line)"
    } else {
        ""
    }
}

/// Everything wrong with a header that parsed: a type or scope that is not in [`TYPES`] or
/// [`SCOPES`], and a summary of more than 72 characters (counted as `char`s), that starts with
/// an upper-case letter or that ends with a full stop.
#[must_use]
pub fn header_problems(header: &Header) -> Vec<String> {
    let mut problems = Vec::new();
    if !TYPES.contains(&header.kind.as_str()) {
        problems.push(format!(
            "the type `{}` is not allowed; use one of {}",
            header.kind,
            TYPES.join(", ")
        ));
    }
    if let Some(scope) = &header.scope
        && !SCOPES.contains(&scope.as_str())
    {
        problems.push(format!(
            "the scope `{scope}` is not allowed; use one of {}, or leave the scope out",
            SCOPES.join(", ")
        ));
    }
    let length = header.summary.chars().count();
    if length > MAX_SUMMARY {
        problems.push(format!(
            "the summary has {length} characters; the limit is {MAX_SUMMARY}"
        ));
    }
    if header.summary.starts_with(char::is_uppercase) {
        problems.push("the summary must not start with an upper-case letter".to_owned());
    }
    if header.summary.trim_end().ends_with('.') {
        problems.push("the summary must not end with a full stop".to_owned());
    }
    problems
}

/// The longest summary, in characters.
const MAX_SUMMARY: usize = 72;

/// The longest body or footer line, in characters, unless it has no space.
const MAX_LINE: usize = 100;

/// A commit message: the header, the body lines and the footers.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Message {
    /// The first line.
    pub header: Header,
    /// The lines between the header and the footers, with a blank line between paragraphs.
    pub body: Vec<String>,
    /// The footers as `(key, value)`; a value that continues on further lines has them joined
    /// with `\n`.
    pub footers: Vec<(String, String)>,
}

/// Reads a whole commit message.
///
/// CRLF is read as LF. A blank line must follow the header when anything follows it. The footers
/// are the last paragraph, when its first line is `Key: value` with a key of letters and hyphens
/// or `BREAKING CHANGE`; a line of that paragraph that is not itself a footer continues the
/// footer before it. Everything between the header and the footers is the body.
///
/// # Errors
///
/// Fails, with a sentence saying what is wrong, when the header does not parse or the blank line
/// after it is missing.
pub fn parse_message(text: &str) -> std::result::Result<Message, String> {
    let text = text.replace("\r\n", "\n");
    let mut lines = text.trim_end_matches('\n').split('\n');
    // `split` yields at least one item; an empty message fails in the header parser.
    let header = parse_header(lines.next().unwrap_or_default())?;
    let rest: Vec<&str> = lines.collect();
    let Some((&separator, content)) = rest.split_first() else {
        return Ok(Message {
            header,
            body: Vec::new(),
            footers: Vec::new(),
        });
    };
    if !is_blank(separator) {
        return Err("a blank line must separate the header from the body".to_owned());
    }
    let content = trim_blank_lines(content);
    let last_paragraph = content
        .iter()
        .rposition(|line| is_blank(line))
        .map_or(0, |blank| blank + 1);
    let has_footers = content
        .get(last_paragraph)
        .is_some_and(|line| split_footer(line).is_some());
    let (body, footer_lines) = if has_footers {
        content.split_at(last_paragraph)
    } else {
        (content, &[][..])
    };
    Ok(Message {
        header,
        body: trim_blank_lines(body)
            .iter()
            .map(|line| (*line).to_owned())
            .collect(),
        footers: footers_of(footer_lines),
    })
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

/// `lines` without the blank lines at its start and its end.
fn trim_blank_lines<'a, 'b>(lines: &'b [&'a str]) -> &'b [&'a str] {
    let start = lines.iter().position(|line| !is_blank(line));
    let end = lines.iter().rposition(|line| !is_blank(line));
    match (start, end) {
        (Some(start), Some(end)) => &lines[start..=end],
        _ => &[],
    }
}

/// `Key: value`, with a key of letters and hyphens or `BREAKING CHANGE`.
fn split_footer(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(": ")?;
    let word = !key.is_empty() && key.chars().all(|c| c.is_ascii_alphabetic() || c == '-');
    (word || key == "BREAKING CHANGE").then_some((key, value))
}

/// The footers in `lines`, the first of which is a footer line.
fn footers_of(lines: &[&str]) -> Vec<(String, String)> {
    let mut footers: Vec<(String, String)> = Vec::new();
    for line in lines {
        if let Some((key, value)) = split_footer(line) {
            footers.push((key.to_owned(), value.to_owned()));
        } else if let Some((_, value)) = footers.last_mut() {
            value.push('\n');
            value.push_str(line);
        }
    }
    footers
}

/// Everything wrong with a parsed message beyond its header ([`header_problems`]):
///
/// - a body or footer line of more than 100 characters, unless it has no space (a URL);
/// - `!` without a `BREAKING CHANGE` or `BREAKING-CHANGE` footer, or the footer without `!`;
/// - a `Phase:` footer that is not a phase, or that is not the phase of a `phase/` branch, or
///   that is missing on a `phase/` branch;
/// - a `Refs:` id that is not in `known_acs`. With no known ids at all the list is unavailable,
///   and an id only has to look like one (`CE-08.10`, `CE-08R.1`).
#[must_use]
pub fn message_problems(
    message: &Message,
    branch: &BranchKind,
    known_acs: &BTreeSet<String>,
) -> Vec<String> {
    let mut problems = Vec::new();
    line_length_problems(message, &mut problems);
    breaking_problems(message, &mut problems);
    phase_problems(message, branch, &mut problems);
    refs_problems(message, known_acs, &mut problems);
    problems
}

fn line_length_problems(message: &Message, problems: &mut Vec<String>) {
    // The lines as they were written: a footer's first line has its key in front.
    let footer_lines = message.footers.iter().flat_map(|(key, value)| {
        let mut lines = value.split('\n').map(str::to_owned);
        let first = lines.next().unwrap_or_default();
        std::iter::once(format!("{key}: {first}")).chain(lines)
    });
    for line in message.body.iter().cloned().chain(footer_lines) {
        let length = line.chars().count();
        if length > MAX_LINE && line.contains(' ') {
            let start: String = line.chars().take(30).collect();
            problems.push(format!(
                "the line starting \"{start}\" has {length} characters; the limit for body and \
                 footer lines is {MAX_LINE} (a line without spaces, such as a URL, may be longer)"
            ));
        }
    }
}

fn breaking_problems(message: &Message, problems: &mut Vec<String>) {
    let footer = message
        .footers
        .iter()
        .any(|(key, _)| key == "BREAKING CHANGE" || key == "BREAKING-CHANGE");
    match (message.header.breaking, footer) {
        (true, false) => problems.push(
            "the header has a `!` but there is no `BREAKING CHANGE:` footer to say what breaks"
                .to_owned(),
        ),
        (false, true) => problems.push(
            "there is a `BREAKING CHANGE:` footer but the header has no `!` after the type or \
             scope"
                .to_owned(),
        ),
        _ => {}
    }
}

fn phase_problems(message: &Message, branch: &BranchKind, problems: &mut Vec<String>) {
    let wanted = phase_of(branch);
    let mut found = false;
    for (_, value) in message.footers.iter().filter(|(key, _)| key == "Phase") {
        found = true;
        let value = value.trim();
        match (value.parse::<Phase>(), wanted) {
            (Err(_), _) => problems.push(format!(
                "`Phase: {value}` is not a phase; write `P` and two digits, such as `Phase: P02`"
            )),
            (Ok(phase), Some(wanted)) if phase != wanted => problems.push(format!(
                "`Phase: {phase}` is not the phase of this branch; the footer must be \
                 `Phase: {wanted}`"
            )),
            _ => {}
        }
    }
    if let Some(wanted) = wanted
        && !found
    {
        problems.push(format!(
            "commits on a phase branch need the footer `Phase: {wanted}`"
        ));
    }
}

fn refs_problems(message: &Message, known_acs: &BTreeSet<String>, problems: &mut Vec<String>) {
    for (_, value) in message.footers.iter().filter(|(key, _)| key == "Refs") {
        for id in value.split(',').map(str::trim) {
            let known = if known_acs.is_empty() {
                is_ac_id(id)
            } else {
                known_acs.contains(id)
            };
            if id.is_empty() {
                problems.push(
                    "`Refs:` has an empty entry; separate acceptance criteria with `, `".to_owned(),
                );
            } else if !known {
                problems.push(format!(
                    "`Refs:` names `{id}`, which is not an acceptance criterion of the umbrella \
                     spec"
                ));
            }
        }
    }
}

/// `CE-`, a group number with an optional `R`, a dot and a number: `CE-08.10`, `CE-08R.1`.
fn is_ac_id(id: &str) -> bool {
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    id.strip_prefix("CE-")
        .and_then(|rest| rest.split_once('.'))
        .is_some_and(|(group, number)| {
            digits(group.strip_suffix('R').unwrap_or(group)) && digits(number)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::branch::parse_branch;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn header(kind: &str, scope: Option<&str>, breaking: bool, summary: &str) -> Header {
        Header {
            kind: kind.to_owned(),
            scope: scope.map(str::to_owned),
            breaking,
            summary: summary.to_owned(),
        }
    }

    fn known(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    /// The problems of `text` on `branch`, from both functions that judge a message.
    fn problems(text: &str, branch: &str, known_acs: &[&str]) -> Result<Vec<String>, String> {
        let message = parse_message(text)?;
        let mut all = header_problems(&message.header);
        all.extend(message_problems(
            &message,
            &parse_branch(branch),
            &known(known_acs),
        ));
        Ok(all)
    }

    fn has_problem(found: &[String], needle: &str) -> bool {
        found.iter().any(|problem| problem.contains(needle))
    }

    // AC: CE-08.10
    #[test]
    fn parses_header_with_scope_and_bang() -> TestResult {
        assert_eq!(
            parse_header("feat(xtask)!: drop the old flag")?,
            header("feat", Some("xtask"), true, "drop the old flag")
        );
        assert_eq!(
            parse_header("fix(gate): keep the colon: it is part of the summary")?,
            header(
                "fix",
                Some("gate"),
                false,
                "keep the colon: it is part of the summary"
            )
        );
        assert_eq!(
            parse_header("chore: bump")?,
            header("chore", None, false, "bump")
        );
        assert_eq!(
            parse_header("feat!: rename")?,
            header("feat", None, true, "rename")
        );
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn headers_with_the_wrong_shape_do_not_parse() {
        for bad in [
            "",
            "Added stuff",
            "feat",
            "feat:x",
            "feat:  two spaces",
            "feat: ",
            "feat:",
            "feat(): empty scope",
            "feat(xtask: unclosed",
            "feat (xtask): space before the scope",
            "feat !: space before the bang",
            "feat!(xtask): bang before the scope",
            "feat(xtask)(gate): two scopes",
            "feat(xtask)x: junk after the scope",
            ": no type",
            "(xtask): no type",
            "fe at: a space in the type",
            "fixup! feat(xtask): x",
            "squash! feat(xtask): x",
            "Revert \"feat(xtask): x\"",
            "Merge branch 'main' into phase/P00-x",
        ] {
            assert!(parse_header(bad).is_err(), "{bad:?} parsed");
        }
    }

    // AC: CE-08.10
    #[test]
    fn autosquash_and_revert_headers_say_what_to_do() {
        let fixup = parse_header("fixup! feat(xtask): x")
            .err()
            .unwrap_or_default();
        assert!(fixup.contains("fixup!"), "{fixup}");
        assert!(fixup.contains("squash"), "{fixup}");
        let revert = parse_header("Revert \"feat(xtask): x\"")
            .err()
            .unwrap_or_default();
        assert!(revert.contains("revert("), "{revert}");
    }

    // AC: CE-08.10
    #[test]
    fn rejects_unknown_type() -> TestResult {
        let problems = header_problems(&parse_header("feature(xtask): add a thing")?);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("`feature`"), "{problems:?}");
        let problems = header_problems(&parse_header("Fix: add a thing")?);
        assert!(has_problem(&problems, "`Fix`"), "{problems:?}");
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn rejects_unknown_scope() -> TestResult {
        let problems = header_problems(&parse_header("feat(nope): add a thing")?);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("`nope`"), "{problems:?}");
        let problems = header_problems(&parse_header("feat(xtask,gate): add a thing")?);
        assert!(has_problem(&problems, "`xtask,gate`"), "{problems:?}");
        Ok(())
    }

    #[test]
    fn every_listed_type_and_scope_is_allowed() -> TestResult {
        for kind in TYPES {
            assert_eq!(
                header_problems(&parse_header(&format!("{kind}: x"))?),
                Vec::<String>::new(),
                "{kind}"
            );
            for scope in SCOPES {
                let line = format!("{kind}({scope})!: x");
                assert_eq!(
                    header_problems(&parse_header(&line)?),
                    Vec::<String>::new(),
                    "{line}"
                );
            }
        }
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn summary_of_72_chars_passes_73_fails() -> TestResult {
        let line = |summary: &str| format!("feat(xtask): {summary}");
        let at_limit = parse_header(&line(&"a".repeat(72)))?;
        assert_eq!(header_problems(&at_limit), Vec::<String>::new());
        let over = header_problems(&parse_header(&line(&"a".repeat(73)))?);
        assert_eq!(over.len(), 1, "{over:?}");
        assert!(over[0].contains("73"), "{over:?}");
        // Characters, not bytes: 72 two-byte characters fit, 73 do not.
        let wide_ok = parse_header(&line(&"é".repeat(72)))?;
        assert_eq!(header_problems(&wide_ok), Vec::<String>::new());
        let wide_over = parse_header(&line(&"é".repeat(73)))?;
        assert_eq!(header_problems(&wide_over).len(), 1);
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn rejects_uppercase_start_and_full_stop() -> TestResult {
        let upper = header_problems(&parse_header("feat(xtask): Add a thing")?);
        assert_eq!(upper.len(), 1, "{upper:?}");
        assert!(upper[0].contains("upper-case"), "{upper:?}");
        let accented = header_problems(&parse_header("feat(xtask): Écrire un test")?);
        assert!(has_problem(&accented, "upper-case"), "{accented:?}");
        let stop = header_problems(&parse_header("feat(xtask): add a thing.")?);
        assert_eq!(stop.len(), 1, "{stop:?}");
        assert!(stop[0].contains("full stop"), "{stop:?}");
        let both = header_problems(&parse_header("feat(xtask): Add a thing.")?);
        assert_eq!(both.len(), 2, "{both:?}");
        for fine in [
            "feat: 3 new checks",
            "feat: `code` first",
            "feat: add a thing..x",
        ] {
            assert_eq!(
                header_problems(&parse_header(fine)?),
                Vec::<String>::new(),
                "{fine}"
            );
        }
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn crlf_message_parses_like_lf() -> TestResult {
        let lf = "feat(xtask)!: drop the flag\n\nThe flag was never used.\nSecond line.\n\n\
                  BREAKING CHANGE: the flag is gone\nRefs: CE-08.10\nPhase: P00\n";
        let crlf = lf.replace('\n', "\r\n");
        assert_eq!(parse_message(&crlf)?, parse_message(lf)?);
        let message = parse_message(&crlf)?;
        assert_eq!(message.body, ["The flag was never used.", "Second line."]);
        assert_eq!(message.footers.len(), 3);
        assert!(
            message
                .footers
                .iter()
                .all(|(key, value)| !key.contains('\r') && !value.contains('\r')),
            "{message:?}"
        );
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn a_blank_line_must_follow_the_header() -> TestResult {
        let error = parse_message("feat: x\nbody right after the header")
            .err()
            .unwrap_or_default();
        assert!(error.contains("blank line"), "{error}");
        assert!(parse_message("feat: x\nPhase: P00").is_err());
        // Nothing after the header needs no blank line.
        for alone in ["feat: x", "feat: x\n", "feat: x\n\n", "feat: x\r\n"] {
            let message = parse_message(alone)?;
            assert!(
                message.body.is_empty() && message.footers.is_empty(),
                "{alone:?}"
            );
        }
        assert!(parse_message("Added stuff\n\nbody").is_err());
        assert!(parse_message("").is_err());
        Ok(())
    }

    #[test]
    fn body_and_footers_are_told_apart() -> TestResult {
        let both =
            parse_message("feat: x\n\nFirst paragraph\nstill first.\n\nSecond.\n\nRefs: CE-08.1")?;
        assert_eq!(
            both.body,
            ["First paragraph", "still first.", "", "Second."]
        );
        assert_eq!(both.footers, [("Refs".to_owned(), "CE-08.1".to_owned())]);

        let only_body =
            parse_message("feat: x\n\nWhy: no footers here, a single sentence.\n\nMore.")?;
        assert_eq!(only_body.footers, Vec::<(String, String)>::new());
        assert_eq!(only_body.body.len(), 3);

        // The last paragraph is the footers, even when it is the only one.
        let only_footers = parse_message("feat: x\n\nPhase: P00\nRefs: CE-08.1, CE-08.2")?;
        assert_eq!(only_footers.body, Vec::<String>::new());
        assert_eq!(
            only_footers.footers,
            [
                ("Phase".to_owned(), "P00".to_owned()),
                ("Refs".to_owned(), "CE-08.1, CE-08.2".to_owned()),
            ]
        );

        // Trailing blank lines do not count.
        let padded = parse_message("feat: x\n\nbody\n\nPhase: P00\n\n\n")?;
        assert_eq!(padded.body, ["body"]);
        assert_eq!(padded.footers.len(), 1);
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn multi_line_footer_joins() -> TestResult {
        let message = parse_message(
            "feat(cli)!: rename the flag\n\nWhy.\n\nBREAKING CHANGE: `--old` is now `--new`;\n\
             scripts that pass `--old` must change.\nRefs: CE-08.1\nPhase: P00",
        )?;
        assert_eq!(
            message.footers,
            [
                (
                    "BREAKING CHANGE".to_owned(),
                    "`--old` is now `--new`;\nscripts that pass `--old` must change.".to_owned()
                ),
                ("Refs".to_owned(), "CE-08.1".to_owned()),
                ("Phase".to_owned(), "P00".to_owned()),
            ]
        );
        assert_eq!(message.body, ["Why."]);
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn bang_requires_breaking_footer_and_back() -> TestResult {
        let bang_only = problems("feat(cli)!: rename the flag\n\nPhase: P00", "main", &[])?;
        assert_eq!(bang_only.len(), 1, "{bang_only:?}");
        assert!(bang_only[0].contains("BREAKING CHANGE"), "{bang_only:?}");

        let footer_only = problems(
            "feat(cli): rename the flag\n\nBREAKING CHANGE: it is renamed",
            "main",
            &[],
        )?;
        assert_eq!(footer_only.len(), 1, "{footer_only:?}");
        assert!(footer_only[0].contains('!'), "{footer_only:?}");

        let both = problems(
            "feat(cli)!: rename the flag\n\nBREAKING CHANGE: it is renamed",
            "main",
            &[],
        )?;
        assert_eq!(both, Vec::<String>::new());
        let neither = problems("feat(cli): rename the flag", "main", &[])?;
        assert_eq!(neither, Vec::<String>::new());
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn both_breaking_spellings_count() -> TestResult {
        for key in ["BREAKING CHANGE", "BREAKING-CHANGE"] {
            let text = format!("feat(cli)!: rename the flag\n\n{key}: it is renamed");
            assert_eq!(problems(&text, "main", &[])?, Vec::<String>::new(), "{key}");
            let without_bang = format!("feat(cli): rename the flag\n\n{key}: it is renamed");
            assert_eq!(problems(&without_bang, "main", &[])?.len(), 1, "{key}");
        }
        // Other spellings are ordinary footers.
        let lower = problems(
            "feat(cli): rename the flag\n\nBreaking change: it is renamed",
            "main",
            &[],
        )?;
        assert_eq!(lower, Vec::<String>::new());
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn long_body_line_fails_but_url_line_passes() -> TestResult {
        let at_limit = "word ".repeat(19) + "word";
        assert_eq!(at_limit.chars().count(), 99);
        let exact = format!("{at_limit}x");
        assert_eq!(exact.chars().count(), 100);
        let over = format!("{exact}y");
        let url = format!("https://example.com/{}", "a".repeat(150));

        let text = |line: &str| format!("feat: x\n\n{line}\n\nPhase: P00");
        assert_eq!(problems(&text(&exact), "main", &[])?, Vec::<String>::new());
        assert_eq!(problems(&text(&url), "main", &[])?, Vec::<String>::new());
        let failing = problems(&text(&over), "main", &[])?;
        assert_eq!(failing.len(), 1, "{failing:?}");
        assert!(failing[0].contains("101"), "{failing:?}");

        // Characters, not bytes.
        let wide = "é ".repeat(50);
        assert_eq!(wide.chars().count(), 100);
        assert_eq!(problems(&text(&wide), "main", &[])?, Vec::<String>::new());
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn footer_lines_are_held_to_the_same_limit() -> TestResult {
        let long = "words ".repeat(20);
        let footer = format!("feat: x\n\nPhase: P00\nCo-Authored-By: {long}");
        let found = problems(&footer, "main", &[])?;
        assert_eq!(found.len(), 1, "{found:?}");
        let continued = format!("feat: x\n\nCo-Authored-By: fine\n{long}\nPhase: P00");
        let found = problems(&continued, "main", &[])?;
        assert_eq!(found.len(), 1, "{found:?}");
        // The first line of a footer counts with its key.
        let key_pushes_it_over = format!("feat: x\n\nNote: {}", "w ".repeat(48));
        assert_eq!(key_pushes_it_over.lines().last().map(str::len), Some(102));
        assert_eq!(problems(&key_pushes_it_over, "main", &[])?.len(), 1);
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn phase_footer_must_match_a_phase_branch() -> TestResult {
        let on = |footers: &str| problems(&format!("feat: x\n\n{footers}"), "phase/P00-x", &[]);
        assert_eq!(on("Phase: P00")?, Vec::<String>::new());
        let missing = on("Refs: CE-08.1")?;
        assert_eq!(missing.len(), 1, "{missing:?}");
        assert!(missing[0].contains("P00"), "{missing:?}");
        let no_footers = problems("feat: x", "phase/P00-x", &[])?;
        assert!(has_problem(&no_footers, "Phase: P00"), "{no_footers:?}");
        let other = on("Phase: P01")?;
        assert_eq!(other.len(), 1, "{other:?}");
        assert!(
            other[0].contains("P01") && other[0].contains("P00"),
            "{other:?}"
        );
        let bad = on("Phase: zero")?;
        assert!(has_problem(&bad, "zero"), "{bad:?}");
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn phase_footer_is_optional_but_valid_elsewhere() -> TestResult {
        for branch in [
            "main",
            "fix/x",
            "gate/x",
            "docs/x",
            "spec/P00-x",
            "feature/x",
        ] {
            assert_eq!(
                problems("feat: x", branch, &[])?,
                Vec::<String>::new(),
                "{branch}"
            );
            assert_eq!(
                problems("feat: x\n\nPhase: P07", branch, &[])?,
                Vec::<String>::new(),
                "{branch}"
            );
            let bad = problems("feat: x\n\nPhase: P7", branch, &[])?;
            assert_eq!(bad.len(), 1, "{branch}: {bad:?}");
        }
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn refs_must_all_be_known() -> TestResult {
        let acs = ["CE-08.1", "CE-08.10", "CE-08R.1"];
        let refs = |value: &str| problems(&format!("feat: x\n\nRefs: {value}"), "main", &acs);
        assert_eq!(refs("CE-08.1")?, Vec::<String>::new());
        assert_eq!(refs("CE-08.1, CE-08.10,CE-08R.1")?, Vec::<String>::new());
        let unknown = refs("CE-08.1, CE-99.1")?;
        assert_eq!(unknown.len(), 1, "{unknown:?}");
        assert!(unknown[0].contains("CE-99.1"), "{unknown:?}");
        assert_eq!(refs("CE-98.1, CE-99.1")?.len(), 2);
        assert_eq!(refs("CE-08.1,")?.len(), 1);
        assert_eq!(refs("CE-08.1,,CE-08.10")?.len(), 1);
        Ok(())
    }

    // AC: CE-08.10
    #[test]
    fn refs_without_a_list_only_have_to_look_like_ids() -> TestResult {
        let refs = |value: &str| problems(&format!("feat: x\n\nRefs: {value}"), "main", &[]);
        assert_eq!(refs("CE-08.10, CE-08R.1, CE-123.45")?, Vec::<String>::new());
        for bad in [
            "CE-08", "CE-.1", "CE-08.", "ce-08.1", "CE-08.1a", "08.1", "CE-0 8.1", "nope",
        ] {
            assert_eq!(refs(bad)?.len(), 1, "{bad}");
        }
        Ok(())
    }

    #[test]
    fn a_good_message_has_no_problems() -> TestResult {
        let text = "feat(xtask): check commit messages against contributing rules\n\n\
                    Every commit of a pull request is parsed and judged.\n\n\
                    Refs: CE-08.10\nPhase: P00\n\
                    Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>\n\
                    Claude-Session: 666a12cd-ea1d-44dd-bf09-077295de681d";
        assert_eq!(
            problems(text, "phase/P00-foundation-and-gate", &["CE-08.10"])?,
            Vec::<String>::new()
        );
        Ok(())
    }
}
