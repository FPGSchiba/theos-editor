//! The `versions` participant: a renamed concept or map renames its versions too, since a
//! version's id is its parent's id, `-v` and a number (`schema/version-id`).

use super::participant::{Participant, RenameSet, record};
use crate::command::{CommandError, PlanContext};

/// Each kind that has versions, with the kind of its versions.
const VERSIONED: &[(&str, &str)] = &[("concept", "concept-version"), ("map", "map-version")];

/// Adds `<id>-v<number>` → `<to>-v<number>` for each version of a renamed concept or map.
pub(crate) struct Versions;

impl Participant for Versions {
    fn extend(&self, ctx: &PlanContext<'_>, set: &mut RenameSet) -> Result<(), CommandError> {
        let renamed: Vec<(String, String)> = set
            .ids
            .iter()
            .map(|(old, new)| (old.clone(), new.clone()))
            .collect();
        for (old, new) in renamed {
            let kind = record(ctx, &old)?.kind;
            let Some(&(_, version_kind)) = VERSIONED
                .iter()
                .find(|(parent, _)| kind.as_deref() == Some(*parent))
            else {
                continue;
            };
            for child in ctx.index().children(&old)? {
                let Some(number) = version_number(&child.source_id, &old) else {
                    continue;
                };
                let is_version = ctx.index().record(&child.source_id)?.iter().any(|found| {
                    found.path == child.source_path && found.kind.as_deref() == Some(version_kind)
                });
                if is_version {
                    let renamed = format!("{new}-v{number}");
                    set.ids.insert(child.source_id, renamed);
                }
            }
        }
        Ok(())
    }
}

/// The number of the version id `id` of `parent`: `<parent>-v<number>`.
fn version_number<'a>(id: &'a str, parent: &str) -> Option<&'a str> {
    let number = id.strip_prefix(parent)?.strip_prefix("-v")?;
    (!number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())).then_some(number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_number_follows_the_parent_and_v() {
        assert_eq!(version_number("shrine-v3", "shrine"), Some("3"));
        assert_eq!(version_number("shrine-v12", "shrine"), Some("12"));
        assert_eq!(version_number("shrine-v", "shrine"), None);
        assert_eq!(version_number("shrine-v1a", "shrine"), None);
        assert_eq!(version_number("shrine-x1", "shrine"), None);
        assert_eq!(version_number("temple-v1", "shrine"), None);
    }
}
