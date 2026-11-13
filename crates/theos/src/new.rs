//! `theos new`: a record from its template or a skeleton (spec §5.9).

use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;
use theos_studio::create::{NewRequest, create};
use theos_studio::views::new::NewView;

use crate::{Exit, output};

/// What `theos new` was asked for.
pub(crate) struct Options {
    /// The kind of the record.
    pub(crate) kind: String,
    /// The id; the next number for a numbered kind by default.
    pub(crate) id: Option<String>,
    /// The title.
    pub(crate) title: Option<String>,
    /// The fields to set.
    pub(crate) set: Vec<(String, String)>,
    /// The project folder; the project above the current folder by default.
    pub(crate) project: Option<PathBuf>,
    /// Print one JSON document instead of text.
    pub(crate) json: bool,
}

/// Splits `field=value` at the first `=`; for clap.
///
/// # Errors
///
/// The usage message when there is no `=` or the field is empty.
pub(crate) fn field_value(text: &str) -> Result<(String, String), String> {
    match text.split_once('=') {
        Some((field, value)) if !field.is_empty() => Ok((field.to_owned(), value.to_owned())),
        _ => Err("expected field=value with a field name".to_owned()),
    }
}

/// Makes the record and prints the result; exit 2 when the record would introduce an error.
///
/// # Errors
///
/// Fails when there is no project, the request is wrong, the index fails, or `out` cannot be
/// written.
pub(crate) fn run(
    options: &Options,
    out: &mut impl Write,
    err: &mut impl Write,
) -> anyhow::Result<Exit> {
    let mut workspace = output::workspace(options.project.as_deref())?;
    output::in_memory_note(err, &workspace).context("cannot write the output")?;
    let created = create(
        &mut workspace,
        &NewRequest {
            kind: &options.kind,
            id: options.id.as_deref(),
            title: options.title.as_deref(),
            set: &options.set,
        },
    )?;
    if options.json {
        output::json(out, &NewView::from(&created))?;
    } else {
        output::findings(out, &created.findings).context("cannot write the output")?;
        if created.written() {
            writeln!(out, "{}", created.path).context("cannot write the output")?;
        }
    }
    if created.written() {
        return Ok(Exit::Ok);
    }
    if !options.json {
        writeln!(
            err,
            "refused: the new record would introduce {}",
            output::plural(created.introduced.len(), "error")
        )
        .context("cannot write the output")?;
    }
    Ok(Exit::Refused)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_value_splits_at_the_first_equals_sign() {
        assert_eq!(
            field_value("tags=[a, b]"),
            Ok(("tags".to_owned(), "[a, b]".to_owned()))
        );
        assert_eq!(
            field_value("vector=a=b"),
            Ok(("vector".to_owned(), "a=b".to_owned()))
        );
        assert_eq!(field_value("x="), Ok(("x".to_owned(), String::new())));
        assert!(field_value("=x").is_err());
        assert!(field_value("novalue").is_err());
    }
}
