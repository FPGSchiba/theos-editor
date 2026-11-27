//! Field-level diff of two front matters, compared by value.

use crate::yaml::{Field, FrontMatter, RawValue};

/// One field that differs between two records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldChange {
    /// The field's key.
    pub key: String,
    /// What happened to it.
    pub delta: FieldDelta,
}

/// How a field differs. Each variant keeps the fields themselves, so a caller shows the old and
/// the new source text through their spans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldDelta {
    /// The field exists only in the new record.
    Added {
        /// The new field.
        new: Field,
    },
    /// The field exists only in the old record.
    Removed {
        /// The old field.
        old: Field,
    },
    /// The field is in both records with a different value.
    Changed {
        /// The old field.
        old: Field,
        /// The new field.
        new: Field,
        /// The items added and removed, when both values are lists; an empty delta means the
        /// same items in a new order.
        items: Option<ItemDelta>,
    },
}

/// The items of a list field that were added and removed, by value. Both empty: the same items
/// in a new order.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ItemDelta {
    /// Items only in the new list.
    pub added: Vec<String>,
    /// Items only in the old list.
    pub removed: Vec<String>,
}

/// The items of `new` that `old` lacks, as a multiset difference in `new`'s order.
fn missing_from(old: &[String], new: &[String]) -> Vec<String> {
    let mut pool: Vec<&String> = old.iter().collect();
    let mut out = Vec::new();
    for item in new {
        match pool.iter().position(|o| *o == item) {
            Some(i) => {
                pool.remove(i);
            }
            None => out.push(item.clone()),
        }
    }
    out
}

fn item_delta(old: &RawValue, new: &RawValue) -> Option<ItemDelta> {
    match (old, new) {
        (RawValue::List(o), RawValue::List(n)) => Some(ItemDelta {
            added: missing_from(o, n),
            removed: missing_from(n, o),
        }),
        _ => None,
    }
}

/// Compares two front matters by field value.
///
/// Changes come in the new record's field order; removed fields follow in the old order. Quoting,
/// list style and comments are not part of a value, so they never make a change.
#[must_use]
pub fn fields(old: &FrontMatter, new: &FrontMatter) -> Vec<FieldChange> {
    let mut out = Vec::new();
    for field in &new.fields {
        let delta = match old.get(&field.key) {
            None => FieldDelta::Added { new: field.clone() },
            Some(before) => {
                let (a, b) = (before.value.to_raw(), field.value.to_raw());
                if a == b {
                    continue;
                }
                FieldDelta::Changed {
                    items: item_delta(&a, &b),
                    old: before.clone(),
                    new: field.clone(),
                }
            }
        };
        out.push(FieldChange {
            key: field.key.clone(),
            delta,
        });
    }
    for field in &old.fields {
        if new.get(&field.key).is_none() {
            out.push(FieldChange {
                key: field.key.clone(),
                delta: FieldDelta::Removed { old: field.clone() },
            });
        }
    }
    out
}
