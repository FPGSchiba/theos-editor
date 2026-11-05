//! Verification: a patched record is accepted only when parsing it again shows exactly the
//! change that was asked for and nothing else.

use super::{PatchError, PatchOp};
use crate::record::Record;
use crate::yaml::{Field, FrontMatter, RawEntry, RawValue};

/// A verification failure, logged and ready to return.
fn fail(reason: String) -> PatchError {
    tracing::debug!(%reason, "patch verification failed");
    PatchError::Verify { reason }
}

/// Checks that `new` differs from `old` by `op` alone.
pub(super) fn verify(old: &Record, new: &Record, op: &PatchOp) -> Result<(), PatchError> {
    let old_front = old
        .front_matter()
        .map_err(|error| PatchError::FrontMatter(error.clone()))?;
    let new_front = new
        .front_matter()
        .map_err(|error| fail(format!("the new front matter does not parse: {error}")))?;
    if old.body_span().get(old.text()) != new.body_span().get(new.text()) {
        return Err(fail("the body text changed".to_owned()));
    }
    let key = op_key(op);
    let expected = expected(old_front, op)?;
    let others = old_front.fields.iter().filter(|field| field.key != *key);
    let mut count = 0;
    for field in others {
        count += 1;
        same_field(old, new, new_front, field)?;
    }
    let touched = new_front.get(key);
    let want = usize::from(!matches!(expected, Expect::Absent));
    if new_front.fields.len() != count + want {
        return Err(fail(
            "the set of fields changed beyond the touched one".to_owned(),
        ));
    }
    match (expected, touched) {
        (Expect::Absent, None) => Ok(()),
        (Expect::Absent, Some(_)) => {
            Err(fail(format!("`{key}` is still present after the removal")))
        }
        (_, None) => Err(fail(format!("`{key}` is missing after the write"))),
        (Expect::Value(value), Some(field)) if field.value.to_raw() == value => Ok(()),
        (Expect::Value(_), Some(_)) => Err(fail(format!("`{key}` does not read back as written"))),
        (Expect::Entry { rest, name, value }, Some(field)) => {
            entry_matches(key, field, &rest, &name, &value)
        }
    }
}

/// What the touched field must read back as.
enum Expect {
    /// The field is gone.
    Absent,
    /// The field has exactly this value.
    Value(RawValue),
    /// The field is a map of `rest` (the old entries but `name`) with `name` set to `value`; where
    /// the entry stands is the caller's order, which is not checked here.
    Entry {
        rest: Vec<(String, RawEntry)>,
        name: String,
        value: RawEntry,
    },
}

fn op_key(op: &PatchOp) -> &String {
    match op {
        PatchOp::Set { key, .. }
        | PatchOp::Remove { key }
        | PatchOp::SetEntry { key, .. }
        | PatchOp::RemoveEntry { key, .. } => key,
    }
}

/// The old entries of the map field `key` (none when the field is absent) without `entry`.
fn entries_without(
    old_front: &FrontMatter,
    key: &str,
    entry: &str,
) -> Result<Vec<(String, RawEntry)>, PatchError> {
    match old_front.get(key).map(|field| field.value.to_raw()) {
        None => Ok(Vec::new()),
        Some(RawValue::Map(entries)) => Ok(entries
            .into_iter()
            .filter(|(name, _)| name != entry)
            .collect()),
        Some(_) => Err(fail(format!("`{key}` was not a map before the edit"))),
    }
}

fn expected(old_front: &FrontMatter, op: &PatchOp) -> Result<Expect, PatchError> {
    Ok(match op {
        PatchOp::Set { value, .. } => Expect::Value(value.clone()),
        PatchOp::Remove { .. } => Expect::Absent,
        PatchOp::SetEntry { key, entry, value } => Expect::Entry {
            rest: entries_without(old_front, key, entry)?,
            name: entry.clone(),
            value: value.clone(),
        },
        PatchOp::RemoveEntry { key, entry } => {
            let rest = entries_without(old_front, key, entry)?;
            if rest.is_empty() {
                Expect::Absent
            } else {
                Expect::Value(RawValue::Map(rest))
            }
        }
    })
}

/// Checks that the map `field` is `rest` plus the entry `name` set to `value`.
fn entry_matches(
    key: &str,
    field: &Field,
    rest: &[(String, RawEntry)],
    name: &str,
    value: &RawEntry,
) -> Result<(), PatchError> {
    let RawValue::Map(entries) = field.value.to_raw() else {
        return Err(fail(format!("`{key}` is not a map after the write")));
    };
    let found = entries.iter().find(|(entry, _)| entry == name);
    if found.map(|(_, raw)| raw) != Some(value) {
        return Err(fail(format!(
            "`{key}.{name}` does not read back as written"
        )));
    }
    let others: Vec<_> = entries.iter().filter(|(entry, _)| entry != name).collect();
    if others.len() != rest.len() || !others.iter().zip(rest).all(|(a, b)| **a == *b) {
        return Err(fail(format!("the other entries of `{key}` changed")));
    }
    Ok(())
}

/// Checks that `field` of `old` is in `new` with the same value and the same bytes.
fn same_field(
    old: &Record,
    new: &Record,
    new_front: &FrontMatter,
    field: &Field,
) -> Result<(), PatchError> {
    let key = &field.key;
    let Some(found) = new_front.get(key) else {
        return Err(fail(format!("`{key}` disappeared")));
    };
    if found.value.to_raw() != field.value.to_raw() {
        return Err(fail(format!("the value of `{key}` changed")));
    }
    if found.lines.get(new.text()) != field.lines.get(old.text()) {
        return Err(fail(format!("the lines of `{key}` changed")));
    }
    Ok(())
}
