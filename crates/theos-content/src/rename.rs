//! The edits that rename ids in one record. Each front-matter scalar is respelled in its own
//! style for its place; a body link changes only the bytes of its target.

use std::collections::BTreeMap;

use crate::index::{LinkRef, Slot as LinkSlot};
use crate::record::Record;
use crate::text::{Span, TextEdit};
use crate::yaml::{
    EncodeError, EntryValue, FrontMatter, List, ListStyle, Scalar, Slot, Value, encode,
};

/// Old id → new id.
pub type IdMap = BTreeMap<String, String>;

/// Why the ids of a record cannot be renamed.
#[derive(Debug, thiserror::Error)]
pub enum RenameError {
    /// A new id that no scalar spelling can hold.
    #[error(transparent)]
    Encode(#[from] EncodeError),
    /// The index names a front-matter link at bytes no scalar of the record has.
    #[error("no scalar of the front matter sits at bytes {}..{}", .span.start, .span.end)]
    NoScalar {
        /// The span the index named.
        span: Span,
    },
    /// The index names a body link at bytes no link of the record has.
    #[error("no link of the body sits at bytes {}..{}", .span.start, .span.end)]
    NoLink {
        /// The span the index named.
        span: Span,
    },
}

/// The edits that rename the ids of `renames` in `record`: its `id` field when its id is a key,
/// and every link of `links` (the index's links of this record, `Index::links_from`) whose target
/// is a key. A front-matter link is the scalar alone, respelled in its own style for its place;
/// a body link changes only its `target_span`. Sorted by start, never overlapping.
///
/// # Errors
///
/// [`RenameError::Encode`] when a new id cannot be written, [`RenameError::NoScalar`] or
/// [`RenameError::NoLink`] when a link names bytes the record does not have.
pub fn edits(
    record: &Record,
    links: &[LinkRef],
    renames: &IdMap,
) -> Result<Vec<TextEdit>, RenameError> {
    let mut edits = Vec::new();
    if let Ok(front) = record.front_matter() {
        for field in front.fields.iter().filter(|field| field.key == "id") {
            if let Value::Scalar(scalar) = &field.value
                && let Some(new) = renames.get(&scalar.text)
            {
                edits.push(scalar_edit(scalar, new, Slot::Field)?);
            }
        }
    }
    for link in links {
        let Some(new) = renames.get(&link.target) else {
            continue;
        };
        edits.push(match link.slot {
            LinkSlot::Field(_) | LinkSlot::LensEntry(_) => {
                let (scalar, slot) = find_scalar(record, link.span)?;
                scalar_edit(scalar, new, slot)?
            }
            _ => body_edit(record, link, new)?,
        });
    }
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    edits.dedup();
    Ok(edits)
}

fn scalar_edit(scalar: &Scalar, new: &str, slot: Slot) -> Result<TextEdit, RenameError> {
    Ok(TextEdit {
        span: scalar.span,
        replacement: encode(new, Some(scalar.style), slot)?,
    })
}

fn body_edit(record: &Record, link: &LinkRef, new: &str) -> Result<TextEdit, RenameError> {
    let found = record
        .body()
        .links()
        .iter()
        .find(|candidate| candidate.span == link.span)
        .ok_or(RenameError::NoLink { span: link.span })?;
    Ok(TextEdit {
        span: found.target_span,
        replacement: new.to_owned(),
    })
}

/// The scalar of the front matter at exactly `span`, with the encoder slot of its place.
fn find_scalar(record: &Record, span: Span) -> Result<(&Scalar, Slot), RenameError> {
    let missing = RenameError::NoScalar { span };
    let front = record.front_matter().map_err(|_| missing)?;
    scalars(front)
        .into_iter()
        .find(|(scalar, _)| scalar.span == span)
        .ok_or(RenameError::NoScalar { span })
}

/// Every scalar of the front matter that may hold a link, with its place.
fn scalars(front: &FrontMatter) -> Vec<(&Scalar, Slot)> {
    let mut found = Vec::new();
    for field in &front.fields {
        match &field.value {
            Value::Scalar(scalar) => found.push((scalar, Slot::Field)),
            Value::List(list) => list_scalars(list, &mut found),
            Value::Map(map) => {
                for entry in &map.entries {
                    match &entry.value {
                        EntryValue::Scalar(scalar) => found.push((scalar, Slot::Entry)),
                        EntryValue::List(list) => list_scalars(list, &mut found),
                    }
                }
            }
        }
    }
    found
}

fn list_scalars<'a>(list: &'a List, found: &mut Vec<(&'a Scalar, Slot)>) {
    let slot = match list.style {
        ListStyle::Flow => Slot::FlowItem,
        ListStyle::Block => Slot::BlockItem,
    };
    found.extend(list.items.iter().map(|item| (&item.scalar, slot)));
}
