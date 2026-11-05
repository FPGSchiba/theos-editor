//! Verification: a patched record is accepted only when parsing it again shows exactly the
//! change that was asked for and nothing else.

use super::{PatchError, PatchOp};
use crate::record::Record;
use crate::yaml::{Field, FrontMatter};

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
    let (key, expected) = match op {
        PatchOp::Set { key, value } => (key, Some(value)),
        PatchOp::Remove { key } => (key, None),
    };
    let others = old_front.fields.iter().filter(|field| field.key != *key);
    let mut count = 0;
    for field in others {
        count += 1;
        same_field(old, new, new_front, field)?;
    }
    let touched = new_front.get(key);
    let want = usize::from(expected.is_some());
    if new_front.fields.len() != count + want {
        return Err(fail(
            "the set of fields changed beyond the touched one".to_owned(),
        ));
    }
    match (expected, touched) {
        (Some(value), Some(field)) if field.value.to_raw() == *value => Ok(()),
        (Some(_), Some(_)) => Err(fail(format!("`{key}` does not read back as written"))),
        (Some(_), None) => Err(fail(format!("`{key}` is missing after the write"))),
        (None, Some(_)) => Err(fail(format!("`{key}` is still present after the removal"))),
        (None, None) => Ok(()),
    }
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
