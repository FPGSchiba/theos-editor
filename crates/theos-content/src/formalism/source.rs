//! `_source.json`: the Shared Formalism page as the private session of CE-16S writes it.

use serde::Deserialize;

use super::place::place;
use super::render_equation;
use crate::kinds::is_url;
use crate::text::{Position, Span};

/// The source file: the page and its equations in page order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    /// The URL of the Shared Formalism page.
    pub page: String,
    /// The equations in page order.
    pub equations: Vec<Equation>,
}

/// One equation of the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Equation {
    /// `E` and the number the page gives it, without leading zeros.
    pub id: String,
    /// One line.
    pub title: String,
    /// The URL of the equation on the page.
    pub anchor: String,
    /// The equation as the page writes it: Markdown with its LaTeX.
    pub text: String,
}

/// Why a source is not valid, or cannot be rendered.
///
/// Its `Display` is the message and, where `serde_json` found the problem, its place in the
/// crate's units: `{message} at line {line} column {column}`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{message}{}", place_suffix(.position.as_ref()))]
pub struct SourceError {
    /// What is wrong, naming the equation when one is at fault. `serde_json`'s words, without
    /// the " at line L column C" it appends: the place is in `position` and `span`.
    pub message: String,
    /// Where `serde_json` found the problem, in Unicode scalar values like every position of
    /// the crate; none for a rule of the format (it gives no value spans) and for a source that
    /// is not UTF-8, which the checks report at 1:1.
    pub position: Option<Position>,
    /// The empty span at that place, as byte offsets into the source text; none where
    /// `position` is none.
    pub span: Option<Span>,
}

/// " at line L column C", or nothing.
fn place_suffix(position: Option<&Position>) -> String {
    position.map_or_else(String::new, |at| {
        format!(" at line {} column {}", at.line, at.column)
    })
}

impl SourceError {
    /// An error of the format or of the file, without a place.
    pub(super) fn format(message: String) -> SourceError {
        SourceError {
            message,
            position: None,
            span: None,
        }
    }

    /// An error of `serde_json` in `text`, placed in the crate's units.
    fn json(error: &serde_json::Error, text: &str) -> SourceError {
        let (line, column) = (error.line(), error.column());
        let shown = error.to_string();
        let suffix = format!(" at line {line} column {column}");
        let message = match shown.strip_suffix(&suffix) {
            Some(words) if line > 0 => words.to_owned(),
            _ => shown,
        };
        let (span, position) = place(text, line, column);
        SourceError {
            message,
            position: Some(position),
            span: Some(span),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSource {
    format: u32,
    page: String,
    equations: Vec<RawEquation>,
    /// A remark of the writer; ignored (L16).
    #[serde(default, rename = "note")]
    _note: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEquation {
    id: String,
    title: String,
    anchor: String,
    text: String,
}

/// Reads and validates a `_source.json`.
///
/// # Errors
///
/// [`SourceError`] when the text is not JSON of the format (with `serde_json`'s words and place,
/// the column in Unicode scalar values), or breaks a rule of spec §5.14: `format` is 1, ids are `E` and a number without
/// leading zeros and are unique, a title is one non-empty line, an anchor and the page are URLs,
/// a text is not empty.
pub fn parse(text: &str) -> Result<Source, SourceError> {
    let raw: RawSource =
        serde_json::from_str(text).map_err(|error| SourceError::json(&error, text))?;
    if raw.format != 1 {
        return Err(SourceError::format(format!(
            "`format` is {}, only 1 is known",
            raw.format
        )));
    }
    if !is_url(&raw.page) {
        return Err(SourceError::format(
            "`page` is not an http or https URL".to_owned(),
        ));
    }
    let mut equations: Vec<Equation> = Vec::with_capacity(raw.equations.len());
    for equation in raw.equations {
        check(&equation)?;
        if equations.iter().any(|seen| seen.id == equation.id) {
            return Err(SourceError::format(format!(
                "the id {} appears twice",
                equation.id
            )));
        }
        let equation = Equation {
            id: equation.id,
            title: equation.title,
            anchor: equation.anchor,
            text: equation.text,
        };
        // What cannot be rendered is not valid: the checks and `regenerate` agree.
        render_equation(&equation)?;
        equations.push(equation);
    }
    Ok(Source {
        page: raw.page,
        equations,
    })
}

/// The rules of one equation.
fn check(equation: &RawEquation) -> Result<(), SourceError> {
    let id = &equation.id;
    let problem = if !is_equation_id(id) {
        Some("is not `E` and a number without leading zeros")
    } else if equation.title.trim().is_empty() || equation.title.contains(['\n', '\r']) {
        Some("has a title that is empty or not one line")
    } else if !is_url(&equation.anchor) {
        Some("has an anchor that is not an http or https URL")
    } else if equation.text.trim().is_empty() {
        Some("has no text")
    } else {
        None
    };
    match problem {
        Some(problem) => Err(SourceError::format(format!("equation `{id}` {problem}"))),
        None => Ok(()),
    }
}

/// `^E[1-9][0-9]*$`.
fn is_equation_id(id: &str) -> bool {
    id.strip_prefix('E').is_some_and(|number| {
        number.starts_with(|c: char| ('1'..='9').contains(&c))
            && number.chars().all(|c| c.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A valid source with the given equation objects.
    fn source(equations: &str) -> String {
        format!(
            r#"{{"format": 1, "page": "https://example.invalid/p", "equations": [{equations}]}}"#
        )
    }

    fn equation(id: &str, title: &str, anchor: &str, text: &str) -> String {
        format!(r#"{{"id": "{id}", "title": "{title}", "anchor": "{anchor}", "text": "{text}"}}"#)
    }

    fn valid(id: &str) -> String {
        equation(id, "T", "https://example.invalid/p#a", "x")
    }

    fn message(text: &str) -> Result<String, Box<dyn std::error::Error>> {
        match parse(text) {
            Ok(_) => Err("expected an error".into()),
            Err(error) => Ok(error.message),
        }
    }

    #[test]
    fn a_valid_source_is_read_in_page_order() -> Result<(), SourceError> {
        let found = parse(&source(&format!("{}, {}", valid("E10"), valid("E2"))))?;
        assert_eq!(found.page, "https://example.invalid/p");
        let ids: Vec<&str> = found.equations.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["E10", "E2"]);
        Ok(())
    }

    #[test]
    fn a_note_is_ignored_and_other_keys_are_refused() -> Result<(), Box<dyn std::error::Error>> {
        let with_note = source("").replace("{\"format\"", "{\"note\": \"hi\", \"format\"");
        assert!(parse(&with_note).is_ok());
        let extra = source("").replace("{\"format\"", "{\"extra\": 1, \"format\"");
        assert!(message(&extra)?.contains("unknown field `extra`"));
        let inner = source(&valid("E1").replace("\"id\"", "\"x\": 1, \"id\""));
        assert!(message(&inner)?.contains("unknown field `x`"));
        Ok(())
    }

    #[test]
    fn a_syntax_error_has_the_line_and_column_of_serde_json() {
        let error = parse("{\n  \"format\": 1,\n  oops\n}").err();
        assert_eq!(
            error.and_then(|e| e.position),
            Some(Position { line: 3, column: 3 })
        );
    }

    #[test]
    fn the_place_is_in_scalar_values_and_not_in_the_message()
    -> Result<(), Box<dyn std::error::Error>> {
        // `é😀` is 6 bytes and 2 scalar values; serde_json says column 10 after the key.
        let text = "{
  \"é😀\": 1
}
";
        let error = parse(text).err().ok_or("an error")?;
        assert!(!error.message.contains(" at line"), "{}", error.message);
        assert!(
            error.message.contains("unknown field `é😀`"),
            "{}",
            error.message
        );
        assert_eq!(error.position, Some(Position { line: 2, column: 6 }));
        let at = text.find("\": 1").ok_or("the key's end")?;
        assert_eq!(error.span, Some(Span::new(at, at)));
        assert_eq!(
            error.to_string(),
            format!("{} at line 2 column 6", error.message)
        );
        Ok(())
    }

    #[test]
    fn an_error_of_the_format_has_no_place() -> Result<(), Box<dyn std::error::Error>> {
        let error = parse(&source("").replace("\"format\": 1", "\"format\": 2"))
            .err()
            .ok_or("an error")?;
        assert_eq!((error.position, error.span), (None, None));
        assert_eq!(error.to_string(), error.message);
        Ok(())
    }

    #[test]
    fn the_format_is_1() -> Result<(), Box<dyn std::error::Error>> {
        assert!(message(&source("").replace("\"format\": 1", "\"format\": 2"))?.contains("format"));
        Ok(())
    }

    #[test]
    fn the_page_is_a_url() -> Result<(), Box<dyn std::error::Error>> {
        let text = source("").replace("https://example.invalid/p", "nowhere");
        assert!(message(&text)?.contains("`page`"));
        Ok(())
    }

    #[test]
    fn an_id_is_e_and_a_number_without_leading_zeros() -> Result<(), Box<dyn std::error::Error>> {
        for id in ["E0", "E07", "e7", "7", "E", "E1a", "E-1"] {
            let said = message(&source(&valid(id)))?;
            assert!(said.contains(&format!("`{id}`")), "{id}: {said}");
        }
        Ok(())
    }

    #[test]
    fn ids_are_unique() -> Result<(), Box<dyn std::error::Error>> {
        let said = message(&source(&format!("{}, {}", valid("E3"), valid("E3"))))?;
        assert!(said.contains("E3 appears twice"), "{said}");
        Ok(())
    }

    #[test]
    fn a_title_is_one_non_empty_line() -> Result<(), Box<dyn std::error::Error>> {
        for title in ["", "  ", "a\\nb", "a\\rb"] {
            let text = source(&equation("E1", title, "https://example.invalid/p#a", "x"));
            assert!(message(&text)?.contains("title"), "{title:?}");
        }
        Ok(())
    }

    #[test]
    fn a_title_or_anchor_that_cannot_be_written_is_refused()
    -> Result<(), Box<dyn std::error::Error>> {
        let title = source(&equation(
            "E1",
            "a\\u0001b",
            "https://example.invalid/p#a",
            "x",
        ));
        let said = message(&title)?;
        assert!(said.contains("`E1`") && said.contains("title"), "{said}");
        let anchor = source(&equation(
            "E2",
            "T",
            "https://example.invalid/a\\u0001b",
            "x",
        ));
        let said = message(&anchor)?;
        assert!(said.contains("`E2`") && said.contains("anchor"), "{said}");
        Ok(())
    }

    #[test]
    fn an_anchor_is_a_url() -> Result<(), Box<dyn std::error::Error>> {
        for anchor in ["", "ftp://x", "https://a b", "#a"] {
            let text = source(&equation("E1", "T", anchor, "x"));
            assert!(message(&text)?.contains("anchor"), "{anchor:?}");
        }
        Ok(())
    }

    #[test]
    fn a_text_is_not_empty() -> Result<(), Box<dyn std::error::Error>> {
        for text in ["", " \\n"] {
            let said = message(&source(&equation(
                "E1",
                "T",
                "https://example.invalid/p#a",
                text,
            )))?;
            assert!(said.contains("no text"), "{text:?}");
        }
        Ok(())
    }
}
