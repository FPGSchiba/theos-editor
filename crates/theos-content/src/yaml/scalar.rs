//! Scalars of the restricted YAML: their text, style and place, and the typed readings a field
//! asks for. A scalar is typed by the field that holds it, never by its spelling (ruling D1).

use yaml_rust2::scanner::TScalarStyle;

use crate::text::{Position, Span};

/// How a scalar is written in the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScalarStyle {
    /// No quotes.
    Plain,
    /// In `'…'`, a quote doubled.
    SingleQuoted,
    /// In `"…"`, with `\\` and `\"` as the only escapes.
    DoubleQuoted,
}

impl ScalarStyle {
    /// The style of a scalar the YAML crate read; `None` for a block scalar.
    pub(crate) fn from_crate(style: TScalarStyle) -> Option<ScalarStyle> {
        match style {
            TScalarStyle::Plain => Some(ScalarStyle::Plain),
            TScalarStyle::SingleQuoted => Some(ScalarStyle::SingleQuoted),
            TScalarStyle::DoubleQuoted => Some(ScalarStyle::DoubleQuoted),
            _ => None,
        }
    }
}

/// A scalar with its decoded text and where it stands in the record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scalar {
    /// The decoded text.
    pub text: String,
    /// How the scalar is spelled.
    pub style: ScalarStyle,
    /// The source bytes of the scalar, quotes included, in record coordinates.
    pub span: Span,
    /// Line and column of the start of `span`.
    pub position: Position,
}

/// What a plain scalar looks like, by the lexicon of the data model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lexical {
    /// `-?[0-9]+`.
    Integer,
    /// `-?[0-9]+\.[0-9]+`.
    Decimal,
    /// `true` or `false`.
    Boolean,
    /// Anything else, and every quoted scalar.
    Text,
}

/// A validated decimal, kept as its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decimal(String);

impl Decimal {
    /// The decimal as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The nearest `f64` (NaN cannot happen for validated text).
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        self.0.parse().unwrap_or(f64::NAN)
    }
}

/// A scalar that is not the type its field asks for.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ScalarError {
    /// The scalar is quoted, so it is text only.
    #[error("{position}: a quoted value is text, not a number or a boolean")]
    NotPlain {
        /// Where the scalar starts.
        position: Position,
    },
    /// The scalar is not an integer.
    #[error("{position}: `{text}` is not an integer")]
    NotInteger {
        /// The scalar's text.
        text: String,
        /// Where the scalar starts.
        position: Position,
    },
    /// The integer does not fit `i64`.
    #[error("{position}: `{text}` is outside the integer range")]
    IntegerRange {
        /// The scalar's text.
        text: String,
        /// Where the scalar starts.
        position: Position,
    },
    /// The scalar is not a decimal.
    #[error("{position}: `{text}` is not a decimal")]
    NotDecimal {
        /// The scalar's text.
        text: String,
        /// Where the scalar starts.
        position: Position,
    },
    /// The scalar is not `true` or `false`.
    #[error("{position}: `{text}` is not a boolean")]
    NotBoolean {
        /// The scalar's text.
        text: String,
        /// Where the scalar starts.
        position: Position,
    },
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

fn is_integer(text: &str) -> bool {
    is_digits(text.strip_prefix('-').unwrap_or(text))
}

fn is_decimal(text: &str) -> bool {
    text.strip_prefix('-')
        .unwrap_or(text)
        .split_once('.')
        .is_some_and(|(whole, fraction)| is_digits(whole) && is_digits(fraction))
}

impl Scalar {
    /// The decoded text, whatever the field.
    #[must_use]
    pub fn as_text(&self) -> &str {
        &self.text
    }

    /// What the text looks like; `Text` for any quoted scalar.
    #[must_use]
    pub fn lexical(&self) -> Lexical {
        if self.style != ScalarStyle::Plain {
            Lexical::Text
        } else if is_integer(&self.text) {
            Lexical::Integer
        } else if is_decimal(&self.text) {
            Lexical::Decimal
        } else if self.text == "true" || self.text == "false" {
            Lexical::Boolean
        } else {
            Lexical::Text
        }
    }

    /// The scalar as an `i64`.
    ///
    /// # Errors
    ///
    /// `NotPlain` for a quoted scalar, `NotInteger` for other text, `IntegerRange` outside `i64`.
    pub fn as_int(&self) -> Result<i64, ScalarError> {
        self.require_plain()?;
        if !is_integer(&self.text) {
            return Err(ScalarError::NotInteger {
                text: self.text.clone(),
                position: self.position,
            });
        }
        self.text.parse().map_err(|_| ScalarError::IntegerRange {
            text: self.text.clone(),
            position: self.position,
        })
    }

    /// The scalar as a decimal.
    ///
    /// # Errors
    ///
    /// `NotPlain` for a quoted scalar, `NotDecimal` for other text.
    pub fn as_decimal(&self) -> Result<Decimal, ScalarError> {
        self.require_plain()?;
        if is_decimal(&self.text) {
            Ok(Decimal(self.text.clone()))
        } else {
            Err(ScalarError::NotDecimal {
                text: self.text.clone(),
                position: self.position,
            })
        }
    }

    /// The scalar as a boolean.
    ///
    /// # Errors
    ///
    /// `NotPlain` for a quoted scalar, `NotBoolean` for anything but `true` and `false`.
    pub fn as_bool(&self) -> Result<bool, ScalarError> {
        self.require_plain()?;
        match self.text.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(ScalarError::NotBoolean {
                text: self.text.clone(),
                position: self.position,
            }),
        }
    }

    fn require_plain(&self) -> Result<(), ScalarError> {
        if self.style == ScalarStyle::Plain {
            Ok(())
        } else {
            Err(ScalarError::NotPlain {
                position: self.position,
            })
        }
    }
}

/// The source spelling of `text` in `style`: plain as is, single-quoted with `'` doubled,
/// double-quoted with `\` and `"` escaped.
pub(crate) fn spell(style: ScalarStyle, text: &str) -> String {
    match style {
        ScalarStyle::Plain => text.to_owned(),
        ScalarStyle::SingleQuoted => format!("'{}'", text.replace('\'', "''")),
        ScalarStyle::DoubleQuoted => {
            format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
        }
    }
}

/// The end of the scalar `text` written at `offset` of `source`: `Ok(end)` when
/// `spell(style, text)` is exactly the source there, else `Err` with the first differing byte.
pub(crate) fn spelled_end(
    source: &str,
    offset: usize,
    style: ScalarStyle,
    text: &str,
) -> Result<usize, usize> {
    let spelled = spell(style, text);
    let rest = source.as_bytes().get(offset..).unwrap_or_default();
    let common = spelled
        .bytes()
        .zip(rest.iter().copied())
        .take_while(|(want, got)| want == got)
        .count();
    if common == spelled.len() {
        Ok(offset + common)
    } else {
        Err(offset + common)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::record;
    use crate::yaml::Value;

    fn scalar(text: &str, style: ScalarStyle) -> Scalar {
        Scalar {
            text: text.to_owned(),
            style,
            span: Span::new(0, text.len()),
            position: Position { line: 2, column: 4 },
        }
    }

    fn plain(text: &str) -> Scalar {
        scalar(text, ScalarStyle::Plain)
    }

    /// The scalar of the value of the first field of `front`.
    fn value_of(front: &str) -> Option<Scalar> {
        let r = record(front, "");
        match &r.front_matter().ok()?.fields.first()?.value {
            Value::Scalar(scalar) => Some(scalar.clone()),
            _ => None,
        }
    }

    // AC: CE-09.3
    #[test]
    fn multi_line_scalars_are_rejected() {
        let want = Some(("one-line-scalar", "2:4".to_owned()));
        for front in ["m: a\n  b\n", "q: 'a\n  b'\n", "q: \"a\n  b\"\n"] {
            assert_eq!(crate::test_support::front_error(front), want, "{front}");
        }
    }

    #[test]
    fn only_two_escapes_are_allowed() {
        let want = Some(("escape", "2:4".to_owned()));
        for front in [r#"a: "x\ny""#, r#"a: "\t""#] {
            let front = format!("{front}\n");
            assert_eq!(crate::test_support::front_error(&front), want, "{front}");
        }
    }

    #[test]
    fn spelled_scalars_end_where_their_spelling_ends() {
        let cases = [
            (r#"a: "a \"b\" c""#.to_owned() + "\n", "a \"b\" c", 11),
            ("a: 'It''s'\n".to_owned(), "It's", 7),
            ("a: b#c\n".to_owned(), "b#c", 3),
            (
                "u: https://example.com/p#x\n".to_owned(),
                "https://example.com/p#x",
                23,
            ),
            (
                "r: maps/a.png@sha256:00ff\n".to_owned(),
                "maps/a.png@sha256:00ff",
                22,
            ),
            ("t: draft   \n".to_owned(), "draft", 5),
            ("e: \"\"\n".to_owned(), "", 2),
            ("x: a\ty\n".to_owned(), "a\ty", 3),
        ];
        for (front, text, len) in cases {
            let got = value_of(&front);
            let Some(got) = got else {
                panic!("{front:?} has a spelled scalar");
            };
            assert_eq!(got.text, text, "{front:?}");
            assert_eq!(got.span.len(), len, "{front:?}");
        }
        let quoted = value_of("e: \"\"\n");
        assert_eq!(quoted.map(|s| s.style), Some(ScalarStyle::DoubleQuoted));
    }

    // The marker of a quoted scalar is its opening quote.
    #[test]
    fn quoted_scalars_start_at_their_quote() {
        let got = value_of("a: 'x'\n");
        assert_eq!(
            got.map(|s| s.position),
            Some(Position { line: 2, column: 4 })
        );
    }

    #[test]
    fn spell_and_spelled_end_report_the_first_difference() {
        assert_eq!(spell(ScalarStyle::SingleQuoted, "a'b"), "'a''b'");
        assert_eq!(spell(ScalarStyle::DoubleQuoted, "a\\\"b"), "\"a\\\\\\\"b\"");
        assert_eq!(
            spelled_end("xx'ab'", 2, ScalarStyle::SingleQuoted, "ab"),
            Ok(6)
        );
        assert_eq!(
            spelled_end("xxab\n  c", 2, ScalarStyle::Plain, "ab c"),
            Err(4)
        );
        assert_eq!(spelled_end("ab", 5, ScalarStyle::Plain, "ab"), Err(5));
    }

    #[test]
    fn scalars_are_typed_by_the_field() {
        let forty_two = plain("42");
        assert_eq!(forty_two.as_text(), "42");
        assert_eq!(forty_two.as_int(), Ok(42));
        assert_eq!(forty_two.lexical(), Lexical::Integer);
        assert_eq!(plain("007").as_int(), Ok(7));
        assert_eq!(plain("-3").as_int(), Ok(-3));
        let price = plain("2.99");
        assert_eq!(price.lexical(), Lexical::Decimal);
        let decimal = price.as_decimal();
        assert_eq!(decimal.as_ref().map(Decimal::as_str), Ok("2.99"));
        assert!((decimal.map_or(0.0, |d| d.to_f64()) - 2.99).abs() < f64::EPSILON);
        assert_eq!(plain("true").as_bool(), Ok(true));
        assert_eq!(plain("true").lexical(), Lexical::Boolean);
        assert_eq!(plain("false").as_text(), "false");
        assert_eq!(plain("false").as_bool(), Ok(false));
        for text in [
            "1e5", "~", "null", "yes", "no", "0x1F", "+1", ".5", "E7", "T-012",
        ] {
            let s = plain(text);
            assert_eq!(s.lexical(), Lexical::Text, "{text}");
            assert!(
                matches!(s.as_int(), Err(ScalarError::NotInteger { .. })),
                "{text}"
            );
            assert!(
                matches!(s.as_bool(), Err(ScalarError::NotBoolean { .. })),
                "{text}"
            );
            assert!(
                matches!(s.as_decimal(), Err(ScalarError::NotDecimal { .. })),
                "{text}"
            );
        }
        let quoted = scalar("42", ScalarStyle::SingleQuoted);
        assert_eq!(quoted.lexical(), Lexical::Text);
        assert!(matches!(quoted.as_int(), Err(ScalarError::NotPlain { .. })));
        assert!(matches!(
            plain("99999999999999999999").as_int(),
            Err(ScalarError::IntegerRange { .. })
        ));
    }
}
