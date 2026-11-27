//! The texts of `inworld`: the only ones the render draws besides the generated settlement
//! names. They come from the recipe's `[render.inworld]` table (plan ruling L9):
//! `title = "…"` and `labels = [{ text = "…", at = [x, y] }]`, `at` in metres.

use toml::{Table, Value};

use crate::kernels::num::to_f64_i64;
use crate::render::RenderError;

/// A label of the recipe: its text and its position in metres.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Label {
    /// The text.
    pub text: String,
    /// The point `[x, y]` where the label starts, metres east and south of the corner.
    pub at: [f64; 2],
}

/// The recipe's texts.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Texts {
    /// The title, if the recipe has a non-empty one.
    pub title: Option<String>,
    /// The labels, in recipe order.
    pub labels: Vec<Label>,
}

/// The error for a bad option `key`.
fn bad(key: &'static str) -> RenderError {
    RenderError::Option {
        render: "inworld",
        key,
    }
}

/// A TOML number as a float.
fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Float(f) => Some(*f),
        Value::Integer(i) => Some(to_f64_i64(*i)),
        _ => None,
    }
}

/// One label table.
fn label(v: &Value) -> Result<Label, RenderError> {
    let table = v.as_table().ok_or(bad("labels"))?;
    let text = table
        .get("text")
        .and_then(Value::as_str)
        .ok_or(bad("labels"))?;
    let at = table
        .get("at")
        .and_then(Value::as_array)
        .ok_or(bad("labels"))?;
    match at.as_slice() {
        [x, y] => Ok(Label {
            text: text.to_owned(),
            at: [
                number(x).ok_or(bad("labels"))?,
                number(y).ok_or(bad("labels"))?,
            ],
        }),
        _ => Err(bad("labels")),
    }
}

/// Reads the texts from the render's options.
///
/// # Errors
/// [`RenderError::Option`] when `title` is not a string, or `labels` is not an array of
/// `{ text, at = [x, y] }` tables.
pub(crate) fn read(options: Option<&Table>) -> Result<Texts, RenderError> {
    let Some(options) = options else {
        return Ok(Texts::default());
    };
    let title = match options.get("title") {
        None => None,
        Some(v) => Some(v.as_str().ok_or(bad("title"))?.to_owned()),
    }
    .filter(|t| !t.is_empty());
    let labels = match options.get("labels") {
        None => Vec::new(),
        Some(v) => v
            .as_array()
            .ok_or(bad("labels"))?
            .iter()
            .map(label)
            .collect::<Result<_, _>>()?,
    };
    Ok(Texts { title, labels })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_and_labels_are_read_and_empty_means_none() {
        let t: Table = "title = \"T\"\nlabels = [{ text = \"a\", at = [1, 2.5] }]"
            .parse()
            .unwrap();
        let texts = read(Some(&t)).unwrap();
        assert_eq!(texts.title.as_deref(), Some("T"));
        assert_eq!(
            texts.labels,
            vec![Label {
                text: "a".to_owned(),
                at: [1.0, 2.5]
            }]
        );
        let empty: Table = "title = \"\"".parse().unwrap();
        assert_eq!(read(Some(&empty)).unwrap(), Texts::default());
        assert_eq!(read(None).unwrap(), Texts::default());
    }
}
