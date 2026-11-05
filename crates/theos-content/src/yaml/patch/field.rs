//! The edits of one op: which bytes of the front matter change and what takes their place.

use super::list::{block_edits, flow_edits};
use super::map;
use super::{Order, PatchError, PatchOp};
use crate::record::Record;
use crate::text::{Span, TextEdit};
use crate::yaml::{Field, FrontMatter, ListStyle, RawValue, Slot, Value, encode, is_key_name};

/// The edits that carry out `op` on `record`.
pub(super) fn edits(
    record: &Record,
    op: &PatchOp,
    order: &Order,
) -> Result<Vec<TextEdit>, PatchError> {
    let front = record
        .front_matter()
        .map_err(|error| PatchError::FrontMatter(error.clone()))?;
    match op {
        PatchOp::Set { key, value } => match front.get(key) {
            Some(field) => set_existing(record, field, value),
            None => insert(record, front, key, value, order),
        },
        PatchOp::Remove { key } => {
            let field = front.get(key).ok_or_else(|| PatchError::NotFound {
                key: key.clone(),
                entry: None,
            })?;
            Ok(vec![replace(field.lines, String::new())])
        }
        PatchOp::SetEntry { key, entry, value } => {
            if let Some(field) = front.get(key) {
                return map::set_entry(record.text(), field, entry, value, order);
            }
            let map = RawValue::Map(vec![(entry.clone(), value.clone())]);
            insert(record, front, key, &map, order)
        }
        PatchOp::RemoveEntry { key, entry } => {
            let field = front.get(key).ok_or_else(|| PatchError::NotFound {
                key: key.clone(),
                entry: None,
            })?;
            map::remove_entry(field, entry)
        }
    }
}

fn replace(span: Span, replacement: String) -> TextEdit {
    TextEdit { span, replacement }
}

/// Writes `value` over the existing `field`.
///
/// A scalar over a scalar replaces only the value bytes, and a list over a list edits per item
/// (see `list.rs`), so kept items keep their bytes; a map over a map edits per entry (see
/// `map.rs`) unless it reorders the kept entries. Every other combination rewrites the field's
/// lines; that includes a block list written empty, which becomes `key: []`.
fn set_existing(
    record: &Record,
    field: &Field,
    value: &RawValue,
) -> Result<Vec<TextEdit>, PatchError> {
    match (&field.value, value) {
        (Value::Scalar(old), RawValue::Text(new)) => {
            let spelled = encode(new, Some(old.style), Slot::Field)?;
            Ok(vec![replace(old.span, spelled)])
        }
        (Value::List(old), RawValue::List(new)) => match old.style {
            ListStyle::Flow => flow_edits(old, new),
            ListStyle::Block if !new.is_empty() => block_edits(record.text(), old, new),
            ListStyle::Block => rewrite(record, field, value),
        },
        (Value::Map(old), RawValue::Map(new)) if map::edits_per_entry(old, new) => {
            map::map_edits(record.text(), old, new)
        }
        _ => rewrite(record, field, value),
    }
}

/// Replaces the whole lines of `field` with `value` written from scratch.
fn rewrite(record: &Record, field: &Field, value: &RawValue) -> Result<Vec<TextEdit>, PatchError> {
    let comment = field.comment.and_then(|span| span.get(record.text()));
    let text = field_text(&field.key, value, comment)?;
    Ok(vec![replace(field.lines, text)])
}

/// Inserts a new field after the last present field that `order` puts before it.
fn insert(
    record: &Record,
    front: &FrontMatter,
    key: &str,
    value: &RawValue,
    order: &Order,
) -> Result<Vec<TextEdit>, PatchError> {
    if !is_key_name(key) {
        return Err(PatchError::Key {
            key: key.to_owned(),
        });
    }
    let wanted = order.position(key).ok_or_else(|| PatchError::NotInOrder {
        key: key.to_owned(),
    })?;
    let before = front
        .fields
        .iter()
        .filter_map(|field| order.position(&field.key).map(|place| (place, field)))
        .filter(|(place, _)| *place < wanted)
        .max_by_key(|(place, _)| *place);
    let at = match (before, front.fields.first()) {
        (Some((_, field)), _) => field.lines.end,
        (None, Some(first)) => first.lines.start,
        (None, None) => record.front_span().start,
    };
    let text = field_text(key, value, None)?;
    Ok(vec![replace(Span::new(at, at), text)])
}

/// The text of a whole field, newline included, with `comment` after the key line when given.
fn field_text(key: &str, value: &RawValue, comment: Option<&str>) -> Result<String, PatchError> {
    let mut tail = String::new();
    let mut out = match value {
        RawValue::Text(text) => format!("{key}: {}", encode(text, None, Slot::Field)?),
        RawValue::List(items) => format!("{key}: {}", flow_list(items)?),
        RawValue::Map(entries) => {
            for (name, entry) in entries {
                tail.push_str(&map::entry_line("  ", name, entry, None)?);
            }
            format!("{key}:")
        }
    };
    if let Some(comment) = comment {
        out.push(' ');
        out.push_str(comment);
    }
    out.push('\n');
    out.push_str(&tail);
    Ok(out)
}

pub(super) fn flow_list(items: &[String]) -> Result<String, PatchError> {
    let spelled = items
        .iter()
        .map(|item| encode(item, None, Slot::FlowItem))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(format!("[{}]", spelled.join(", ")))
}
