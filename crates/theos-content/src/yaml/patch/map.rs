//! Per-entry edits of a map field: entries an edit does not name keep their bytes and the comment
//! lines around them; only the changed, new and removed entries produce edits.

use super::field::flow_list;
use super::list::flow_edits;
use super::{Order, PatchError};
use crate::text::{Span, TextEdit};
use crate::yaml::{Entry, EntryValue, Field, Map, RawEntry, Slot, Value, encode, is_key_name};

/// Whether a map written over `old` can be edited per entry: both have entries and the kept
/// entries stay in their order (a reorder is written from scratch).
pub(super) fn edits_per_entry(old: &Map, new: &[(String, RawEntry)]) -> bool {
    let kept_old = old
        .entries
        .iter()
        .map(|entry| entry.key.as_str())
        .filter(|key| new.iter().any(|(name, _)| name == key));
    let kept_new = new
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| old.entries.iter().any(|entry| entry.key == *name));
    !old.entries.is_empty() && !new.is_empty() && kept_old.eq(kept_new)
}

/// The edits that turn `old` into the entries `new`: remove the missing, set the kept, insert the
/// new ones in the given order.
pub(super) fn map_edits(
    text: &str,
    old: &Map,
    new: &[(String, RawEntry)],
) -> Result<Vec<TextEdit>, PatchError> {
    let first = old.entries.first().ok_or_else(missing)?;
    let indent = indent_of(text, first);
    let mut edits: Vec<TextEdit> = old
        .entries
        .iter()
        .filter(|entry| !new.iter().any(|(name, _)| *name == entry.key))
        .map(|entry| delete(entry.lines))
        .collect();
    let mut anchor: Option<&Entry> = None;
    let mut pending = String::new();
    for (name, value) in new {
        let Some(kept) = old.entries.iter().find(|entry| entry.key == *name) else {
            pending.push_str(&entry_line(indent, name, value, None)?);
            continue;
        };
        flush(
            &mut pending,
            &mut edits,
            anchor.unwrap_or(first),
            anchor.is_some(),
        );
        edits.extend(entry_edits(text, kept, value)?);
        anchor = Some(kept);
    }
    flush(
        &mut pending,
        &mut edits,
        anchor.unwrap_or(first),
        anchor.is_some(),
    );
    Ok(edits)
}

/// Turns the `pending` new lines into an insertion behind `anchor`, or in front of it when it is
/// not a real anchor (the new lines come before the first entry).
fn flush(pending: &mut String, edits: &mut Vec<TextEdit>, anchor: &Entry, behind: bool) {
    if pending.is_empty() {
        return;
    }
    let at = if behind {
        anchor.lines.end
    } else {
        anchor.lines.start
    };
    edits.push(TextEdit {
        span: Span::new(at, at),
        replacement: std::mem::take(pending),
    });
}

/// The edits that set the entry `name` of the map field `field` to `value`; a new entry is the line
/// after the last present entry that `order` puts before it, or last when `order` lacks it.
pub(super) fn set_entry(
    text: &str,
    field: &Field,
    name: &str,
    value: &RawEntry,
    order: &Order,
) -> Result<Vec<TextEdit>, PatchError> {
    let map = as_map(field)?;
    if let Some(entry) = map.entries.iter().find(|entry| entry.key == name) {
        return entry_edits(text, entry, value);
    }
    let first = map.entries.first().ok_or_else(missing)?;
    let wanted = order
        .entries_of(&field.key)
        .iter()
        .position(|entry| entry == name);
    let before = wanted.and_then(|wanted| {
        map.entries
            .iter()
            .filter_map(|entry| position(order, &field.key, &entry.key).map(|place| (place, entry)))
            .filter(|(place, _)| *place < wanted)
            .max_by_key(|(place, _)| *place)
            .map(|(_, entry)| entry)
    });
    let at = match (before, wanted) {
        (Some(entry), _) => entry.lines.end,
        (None, Some(_)) => first.lines.start,
        (None, None) => map
            .entries
            .last()
            .map_or(first.lines.end, |last| last.lines.end),
    };
    let line = entry_line(indent_of(text, first), name, value, None)?;
    Ok(vec![TextEdit {
        span: Span::new(at, at),
        replacement: line,
    }])
}

/// The edits that remove the entry `name` of the map field `field`; the last entry takes the whole
/// field with it.
pub(super) fn remove_entry(field: &Field, name: &str) -> Result<Vec<TextEdit>, PatchError> {
    let map = as_map(field)?;
    let entry = map
        .entries
        .iter()
        .find(|entry| entry.key == name)
        .ok_or_else(|| PatchError::NotFound {
            key: field.key.clone(),
            entry: Some(name.to_owned()),
        })?;
    let lines = if map.entries.len() == 1 {
        field.lines
    } else {
        entry.lines
    };
    Ok(vec![delete(lines)])
}

/// The text of one entry line, newline included, with `comment` after the value when given.
pub(super) fn entry_line(
    indent: &str,
    name: &str,
    value: &RawEntry,
    comment: Option<&str>,
) -> Result<String, PatchError> {
    if !is_key_name(name) {
        return Err(PatchError::Key {
            key: name.to_owned(),
        });
    }
    let spelled = match value {
        RawEntry::Text(text) => encode(text, None, Slot::Entry)?,
        RawEntry::List(items) => flow_list(items)?,
    };
    let comment = comment
        .map(|comment| format!(" {comment}"))
        .unwrap_or_default();
    Ok(format!("{indent}{name}: {spelled}{comment}\n"))
}

/// The edits that write `value` over `entry`: a scalar over a scalar replaces the value bytes, a
/// flow list over a flow list edits per item, a shape change rewrites the entry's line.
fn entry_edits(text: &str, entry: &Entry, value: &RawEntry) -> Result<Vec<TextEdit>, PatchError> {
    match (&entry.value, value) {
        (EntryValue::Scalar(old), RawEntry::Text(new)) => {
            let spelled = encode(new, Some(old.style), Slot::Entry)?;
            Ok(vec![TextEdit {
                span: old.span,
                replacement: spelled,
            }])
        }
        (EntryValue::List(old), RawEntry::List(new)) => flow_edits(old, new),
        _ => {
            let comment = entry.comment.and_then(|span| span.get(text));
            let line = entry_line(indent_of(text, entry), &entry.key, value, comment)?;
            Ok(vec![TextEdit {
                span: entry.lines,
                replacement: line,
            }])
        }
    }
}

fn position(order: &Order, key: &str, entry: &str) -> Option<usize> {
    order.entries_of(key).iter().position(|name| name == entry)
}

fn as_map(field: &Field) -> Result<&Map, PatchError> {
    match &field.value {
        Value::Map(map) => Ok(map),
        _ => Err(PatchError::NotAMap {
            key: field.key.clone(),
        }),
    }
}

fn delete(span: Span) -> TextEdit {
    TextEdit {
        span,
        replacement: String::new(),
    }
}

/// The leading blanks of the entry's line.
fn indent_of<'a>(text: &'a str, entry: &Entry) -> &'a str {
    let line = entry.lines.get(text).unwrap_or_default();
    &line[..line.len() - line.trim_start_matches([' ', '\t']).len()]
}

/// The refusal for a map model without entries; the model builds maps with at least one.
fn missing() -> PatchError {
    PatchError::Verify {
        reason: "the map model has no entries".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use crate::record::Record;
    use crate::test_support::record;
    use crate::yaml::patch::{Order, PatchError};
    use crate::yaml::{RawEntry, RawValue};

    const ILSE: &str = include_str!(
        "../../../tests/fixtures/corpus/records/people/hearth-folk/fates-cult/mother-ilse.md"
    );

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn text(value: &str) -> RawEntry {
        RawEntry::Text(value.to_owned())
    }

    fn ilse() -> Result<Record, Box<dyn std::error::Error>> {
        Ok(Record::parse(ILSE)?)
    }

    fn none() -> Order {
        Order::default()
    }

    // AC: CE-09.2
    #[test]
    fn map_entries_change_one_by_one() -> TestResult {
        let rec = ilse()?;
        let speaks = "children and newcomers; calls the player \"stair-child\"";
        let set = rec.set_entry("lens", "speaks_to", text("everyone"), &none())?;
        assert_eq!(set.text(), ILSE.replace(speaks, "everyone"));

        let ordered = Order::new(["lens"]).with_entries("lens", ["stance", "speaks_to"]);
        let before = rec.set_entry("lens", "stance", text("wary"), &ordered)?;
        assert_eq!(
            before.text(),
            ILSE.replace("  speaks_to:", "  stance: wary\n  speaks_to:")
        );

        let last = rec.set_entry("lens", "stance", text("wary"), &none())?;
        assert_eq!(
            last.text(),
            ILSE.replace("\n---\n", "\n  stance: wary\n---\n")
        );

        let two = last.remove_entry("lens", "speaks_to")?;
        assert_eq!(
            two.text(),
            ILSE.replace(&format!("  speaks_to: {speaks}\n"), "  stance: wary\n")
        );
        assert_eq!(last.remove_entry("lens", "stance")?.text(), ILSE);

        let one = rec.remove_entry("lens", "speaks_to")?;
        assert_eq!(
            one.text(),
            ILSE.replace(&format!("lens:\n  speaks_to: {speaks}\n"), "")
        );

        let flow = record("m:\n  e: [a,b]\n", "");
        let more = RawEntry::List(vec!["a".into(), "b".into(), "c".into()]);
        let set = flow.set_entry("m", "e", more, &none())?;
        assert_eq!(set.text(), "---\nm:\n  e: [a,b, c]\n---\n");
        Ok(())
    }

    #[test]
    fn map_refusals() -> TestResult {
        let rec = ilse()?;
        assert!(matches!(
            rec.set_entry("title", "x", text("y"), &none()),
            Err(PatchError::NotAMap { .. })
        ));
        assert!(matches!(
            rec.remove_entry("lens", "nope"),
            Err(PatchError::NotFound { .. })
        ));
        assert!(matches!(
            rec.set_entry("lens", "Bad", text("y"), &none()),
            Err(PatchError::Key { .. })
        ));
        let bare = record("title: x\n", "");
        let order = Order::new(["title", "lens"]);
        let set = bare.set_entry("lens", "stance", text("wary"), &order)?;
        assert_eq!(set.text(), "---\ntitle: x\nlens:\n  stance: wary\n---\n");
        assert!(matches!(
            bare.set_entry("lens", "stance", text("wary"), &none()),
            Err(PatchError::NotInOrder { .. })
        ));
        Ok(())
    }

    #[test]
    fn a_map_written_over_a_map_edits_per_entry() -> TestResult {
        let rec = record("m:\n  # c\n  a: 1   # k\n  b: 2\n  e: x  # z\n", "");
        let entries = vec![
            ("a".to_owned(), text("1")),
            ("c".to_owned(), text("3")),
            ("e".to_owned(), RawEntry::List(vec!["p".into(), "q".into()])),
        ];
        let set = rec.set("m", RawValue::Map(entries), &none())?;
        assert_eq!(
            set.text(),
            "---\nm:\n  # c\n  a: 1   # k\n  c: 3\n  e: [p, q] # z\n---\n"
        );
        let swapped = vec![("e".to_owned(), text("x")), ("a".to_owned(), text("1"))];
        let set = rec.set("m", RawValue::Map(swapped.clone()), &none())?;
        assert_eq!(
            set.front_matter()
                .map_err(ToString::to_string)?
                .get("m")
                .map(|f| f.value.to_raw()),
            Some(RawValue::Map(swapped))
        );
        Ok(())
    }
}
