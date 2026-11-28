//! The front-matter rows of the side column: P02's front-matter model walked in file order, each
//! value typed by its kind's field spec, so links are chips and tier values draw the tier
//! selector.

use serde::Serialize;
use theos_content::kinds::{LENS_ENTRIES, TIERS};
use theos_content::yaml::{EntryValue, FrontMatter, List, Value};
use theos_content::{FieldType, KindRegistry};

use super::resolve::{Resolver, ViewLink};

/// One `key: value` row.
#[derive(Clone, Debug, Serialize)]
pub struct FieldRow {
    /// The key as written.
    pub key: String,
    /// The value.
    pub value: FieldValue,
}

/// A front-matter value as the side column draws it.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FieldValue {
    /// A text, number or enum value.
    Text(String),
    /// A link chip; a dangling one has no target.
    Link(ViewLink),
    /// The items of a list.
    List(Vec<FieldValue>),
    /// A tier (`T1` to `T3`), drawn as the read-only tier selector.
    Tier(String),
    /// The entries of a map, such as a lens.
    Map(Vec<FieldRow>),
}

/// The rows of `front` for a record of `kind`, in file order. A key the kind does not know is
/// shown as text.
#[must_use]
pub fn rows(
    front: &FrontMatter,
    kind: Option<&str>,
    registry: &KindRegistry,
    resolver: &Resolver,
) -> Vec<FieldRow> {
    let spec = kind.and_then(|kind| registry.kind(kind));
    front
        .fields
        .iter()
        .map(|field| {
            let ty = spec.and_then(|spec| {
                spec.fields
                    .iter()
                    .find(|f| f.key == field.key)
                    .map(|f| f.ty)
            });
            FieldRow {
                key: field.key.clone(),
                value: value(&field.value, ty, resolver),
            }
        })
        .collect()
}

fn value(value: &Value, ty: Option<FieldType>, r: &Resolver) -> FieldValue {
    match value {
        Value::Scalar(scalar) => scalar_value(&scalar.text, ty, r),
        Value::List(list) => list_value(list, ty, r),
        Value::Map(map) => FieldValue::Map(
            map.entries
                .iter()
                .map(|entry| {
                    let entry_ty = (ty == Some(FieldType::Lens))
                        .then(|| LENS_ENTRIES.iter().find(|e| e.key == entry.key))
                        .flatten()
                        .map(|e| e.ty);
                    FieldRow {
                        key: entry.key.clone(),
                        value: match &entry.value {
                            EntryValue::Scalar(scalar) => scalar_value(&scalar.text, entry_ty, r),
                            EntryValue::List(list) => list_value(list, entry_ty, r),
                        },
                    }
                })
                .collect(),
        ),
    }
}

fn list_value(list: &List, ty: Option<FieldType>, r: &Resolver) -> FieldValue {
    FieldValue::List(
        list.items
            .iter()
            .map(|item| scalar_value(&item.scalar.text, ty, r))
            .collect(),
    )
}

fn scalar_value(text: &str, ty: Option<FieldType>, r: &Resolver) -> FieldValue {
    match ty {
        Some(FieldType::Link(_) | FieldType::Links(_)) => {
            FieldValue::Link(r.link(text, None, None))
        }
        Some(FieldType::Enum(values)) if values == TIERS => FieldValue::Tier(text.to_owned()),
        _ => FieldValue::Text(text.to_owned()),
    }
}
