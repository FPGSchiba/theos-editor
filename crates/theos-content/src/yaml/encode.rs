//! Encoding a raw text value into the spelling a front-matter slot takes. Our own parser decides
//! whether a text can stay plain; this module never lexes YAML itself.

use super::scalar::{ScalarStyle, spell};
use super::{EntryValue, Value};
use crate::record::Record;

/// Where in the front matter an encoded value is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Slot {
    /// The value of a top-level field: `key: <value>`.
    Field,
    /// An item of a flow list: `key: [<value>]`.
    FlowItem,
    /// An item of a block list: `  - <value>`.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the block-list patcher of Task 15 writes these")
    )]
    BlockItem,
    /// The value of an entry of a map: `  e: <value>`.
    Entry,
}

/// A text that no scalar spelling of the restricted YAML can hold.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    /// The text has a line break or a control character other than a tab.
    #[error("a value cannot hold a line break or a control character other than a tab")]
    Unrepresentable {
        /// The refused text.
        text: String,
    },
}

/// The source spelling of `text` for `slot`: the style `keep` when it is a quote style, else plain
/// when our parser reads it back as the same plain text, else double-quoted.
///
/// # Errors
///
/// `Unrepresentable` when `text` has a line break or a control character other than a tab.
pub(crate) fn encode(
    text: &str,
    keep: Option<ScalarStyle>,
    slot: Slot,
) -> Result<String, EncodeError> {
    if text.chars().any(|c| c.is_control() && c != '\t') {
        return Err(EncodeError::Unrepresentable {
            text: text.to_owned(),
        });
    }
    match keep {
        Some(style @ (ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted)) => {
            Ok(spell(style, text))
        }
        _ if reads_back_plain(text, slot) => Ok(text.to_owned()),
        _ => Ok(spell(ScalarStyle::DoubleQuoted, text)),
    }
}

/// Whether our parser reads `text` written plain in `slot` as one plain scalar of that text.
fn reads_back_plain(text: &str, slot: Slot) -> bool {
    let front = match slot {
        Slot::Field => format!("k: {text}\n"),
        Slot::FlowItem => format!("k: [{text}]\n"),
        Slot::BlockItem => format!("k:\n  - {text}\n"),
        Slot::Entry => format!("k:\n  e: {text}\n"),
    };
    let Ok(record) = Record::parse(&format!("---\n{front}---\n")) else {
        return false;
    };
    let Ok(front_matter) = record.front_matter() else {
        return false;
    };
    let [field] = front_matter.fields.as_slice() else {
        return false;
    };
    let scalar = match (&field.value, slot) {
        (Value::Scalar(scalar), Slot::Field) => scalar,
        (Value::List(list), Slot::FlowItem | Slot::BlockItem) => match list.items.as_slice() {
            [item] => &item.scalar,
            _ => return false,
        },
        (Value::Map(map), Slot::Entry) => match map.entries.as_slice() {
            [entry] => match &entry.value {
                EntryValue::Scalar(scalar) => scalar,
                EntryValue::List(_) => return false,
            },
            _ => return false,
        },
        _ => return false,
    };
    scalar.style == ScalarStyle::Plain && scalar.text == text
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const SLOTS: [Slot; 4] = [Slot::Field, Slot::FlowItem, Slot::BlockItem, Slot::Entry];

    /// The text of the one scalar the oracle reads from `text` spelled as `spelled` in `slot`.
    fn read_back(spelled: &str, slot: Slot) -> Option<String> {
        let front = match slot {
            Slot::Field => format!("k: {spelled}\n"),
            Slot::FlowItem => format!("k: [{spelled}]\n"),
            Slot::BlockItem => format!("k:\n  - {spelled}\n"),
            Slot::Entry => format!("k:\n  e: {spelled}\n"),
        };
        let record = Record::parse(&format!("---\n{front}---\n")).ok()?;
        let field = record.front_matter().ok()?.fields.first()?.clone();
        match field.value {
            Value::Scalar(s) => Some(s.text),
            Value::List(list) => list.items.first().map(|i| i.scalar.text.clone()),
            Value::Map(map) => match &map.entries.first()?.value {
                EntryValue::Scalar(s) => Some(s.text.clone()),
                EntryValue::List(_) => None,
            },
        }
    }

    #[test]
    fn plain_when_it_reads_back() {
        for text in [
            "draft", "a b", "42", "true", "null", "a \"b\"", "a\\b", "x, y", "a\tb",
        ] {
            assert_eq!(encode(text, None, Slot::Field).as_deref(), Ok(text));
        }
    }

    #[test]
    fn values_that_need_quotes_are_double_quoted() {
        let texts = [
            "", " lead", "trail ", "a: b", "b #c", "#x", "- x", "[x", "]x", "{x", "}x", "&a", "*a",
            "!a", "|", ">", "'x", "\"x", "%a", "@a", "`a", "?", ":",
        ];
        for text in texts {
            let want = spell(ScalarStyle::DoubleQuoted, text);
            assert_eq!(encode(text, None, Slot::Field), Ok(want), "{text:?}");
        }
        assert_eq!(encode("\"x", None, Slot::Field).as_deref(), Ok("\"\\\"x\""));
        assert_eq!(encode("", None, Slot::Field).as_deref(), Ok("\"\""));
        for text in ["x, y", "a]", "a["] {
            let want = spell(ScalarStyle::DoubleQuoted, text);
            assert_eq!(encode(text, None, Slot::FlowItem), Ok(want), "{text:?}");
        }
        for slot in SLOTS {
            for text in texts.into_iter().chain(["x, y", "a]", "a["]) {
                let spelled = encode(text, None, slot);
                let read = spelled.ok().and_then(|s| read_back(&s, slot));
                assert_eq!(read.as_deref(), Some(text), "{text:?} in {slot:?}");
            }
        }
    }

    #[test]
    fn existing_styles_are_kept() {
        let single = encode("It's", Some(ScalarStyle::SingleQuoted), Slot::Field);
        assert_eq!(single.as_deref(), Ok("'It''s'"));
        let double = encode("a \"b\"", Some(ScalarStyle::DoubleQuoted), Slot::Field);
        assert_eq!(double.as_deref(), Ok("\"a \\\"b\\\"\""));
        let plain = encode("a: b", Some(ScalarStyle::Plain), Slot::Field);
        assert_eq!(plain.as_deref(), Ok("\"a: b\""));
    }

    #[test]
    fn line_breaks_and_controls_are_refused() {
        for text in ["a\nb", "a\rb", "\u{0}", "\u{7f}", "\u{85}"] {
            assert_eq!(
                encode(text, None, Slot::Field),
                Err(EncodeError::Unrepresentable {
                    text: text.to_owned()
                }),
                "{text:?}"
            );
        }
    }

    proptest! {
        #[test]
        fn encoded_text_reads_back(
            text in prop_oneof!["[^\\p{Cc}]{0,24}", "[a-z\\t #:,\\[\\]-]{0,12}"],
        ) {
            for slot in SLOTS {
                let spelled = encode(&text, None, slot);
                prop_assert!(spelled.is_ok());
                let read = spelled.ok().and_then(|s| read_back(&s, slot));
                prop_assert_eq!(read.as_deref(), Some(text.as_str()));
            }
        }
    }
}
