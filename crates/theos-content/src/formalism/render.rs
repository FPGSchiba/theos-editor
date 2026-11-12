//! Rendering the equation records of a source (spec §5.14).

use super::{Equation, FOLDER, SOURCE, Source, SourceError};
use crate::yaml::{Slot, encode};

/// The files of a source: each project-relative path with its text, by id.
///
/// # Errors
///
/// [`SourceError`] when a title or an anchor cannot be written as a YAML scalar.
pub fn render(source: &Source) -> Result<Vec<(String, String)>, SourceError> {
    let mut files = source
        .equations
        .iter()
        .map(|equation| {
            Ok((
                format!("{FOLDER}/{}.md", equation.id),
                render_equation(equation)?,
            ))
        })
        .collect::<Result<Vec<_>, SourceError>>()?;
    // Ids are `E` and a number without leading zeros: length, then text, is the number order.
    files.sort_by(|(a, _), (b, _)| (a.len(), a).cmp(&(b.len(), b)));
    Ok(files)
}

/// The text of the record of one equation: LF, with a final newline; a CR in the text becomes LF.
///
/// # Errors
///
/// [`SourceError`] when the title or the anchor cannot be written as a YAML scalar.
pub fn render_equation(equation: &Equation) -> Result<String, SourceError> {
    let spell = |what: &str, value: &str| {
        encode(value, None, Slot::Field).map_err(|error| {
            SourceError::format(format!("equation `{}`: the {what} {error}", equation.id))
        })
    };
    let title = spell("title", &equation.title)?;
    let anchor = spell("anchor", &equation.anchor)?;
    let mut text = equation.text.replace("\r\n", "\n").replace('\r', "\n");
    if !text.ends_with('\n') {
        text.push('\n');
    }
    let id = &equation.id;
    Ok(format!(
        "---\n# generated from {SOURCE} by theos; a hand edit fails the check\nid: {id}\nkind: equation\ntitle: {title}\nsource: {anchor}\ngenerated: true\n---\n{text}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn equation(id: &str, title: &str, text: &str) -> Equation {
        Equation {
            id: id.to_owned(),
            title: title.to_owned(),
            anchor: "https://example.invalid/p#a".to_owned(),
            text: text.to_owned(),
        }
    }

    #[test]
    fn a_record_is_written_byte_for_byte() -> Result<(), SourceError> {
        let text = render_equation(&equation("E7", "Synthetic equation 7", "$a = b$"))?;
        assert_eq!(
            text,
            "---\n# generated from canon/formalism/_source.json by theos; a hand edit fails the check\nid: E7\nkind: equation\ntitle: Synthetic equation 7\nsource: https://example.invalid/p#a\ngenerated: true\n---\n$a = b$\n"
        );
        Ok(())
    }

    #[test]
    fn a_title_that_needs_it_is_quoted() -> Result<(), SourceError> {
        let text = render_equation(&equation("E7", "Energy: mass", "x"))?;
        assert!(text.contains("\ntitle: \"Energy: mass\"\n"), "{text}");
        Ok(())
    }

    #[test]
    fn a_cr_becomes_lf_and_one_final_newline_is_kept() -> Result<(), SourceError> {
        let crlf = render_equation(&equation("E1", "T", "a\r\nb\rc"))?;
        assert!(crlf.ends_with("---\na\nb\nc\n") && !crlf.contains('\r'));
        let ended = render_equation(&equation("E1", "T", "a\n"))?;
        assert!(ended.ends_with("---\na\n") && !ended.ends_with("\n\n"));
        Ok(())
    }

    #[test]
    fn a_quoted_title_and_a_cr_give_the_whole_record() -> Result<(), SourceError> {
        let text = render_equation(&equation("E12", "Energy: mass", "$a$\r\nb\rc"))?;
        assert_eq!(
            text,
            "---\n# generated from canon/formalism/_source.json by theos; a hand edit fails the check\nid: E12\nkind: equation\ntitle: \"Energy: mass\"\nsource: https://example.invalid/p#a\ngenerated: true\n---\n$a$\nb\nc\n"
        );
        Ok(())
    }

    #[test]
    fn the_text_is_verbatim() -> Result<(), SourceError> {
        let text = render_equation(&equation("E3", "T", "$\\left[[a, b]\\right]$"))?;
        assert!(text.ends_with("---\n$\\left[[a, b]\\right]$\n"));
        Ok(())
    }

    #[test]
    fn the_files_are_in_id_order() -> Result<(), SourceError> {
        let source = Source {
            page: "https://example.invalid/p".to_owned(),
            equations: vec![equation("E10", "T", "x"), equation("E2", "T", "x")],
        };
        let paths: Vec<String> = render(&source)?.into_iter().map(|(path, _)| path).collect();
        assert_eq!(paths, ["canon/formalism/E2.md", "canon/formalism/E10.md"]);
        Ok(())
    }

    #[test]
    fn a_title_that_cannot_be_written_is_an_error() {
        assert!(render_equation(&equation("E1", "a\u{1}b", "x")).is_err());
    }
}
