//! The values a run seeds into its world before the first stage (spec P29 §5.2 item 5) and
//! their golden pseudo stage `parent#0` (L12).

use std::collections::BTreeMap;

use super::hash_layer;
use crate::child::values::PARENT_PREFIX;
use crate::child::{ChildError, ParentContext, parent_value};
use crate::error::RunError;
use crate::golden::GoldenStage;
use crate::kind;
use crate::progress::CancelToken;
use crate::recipe::Plan;
use crate::stage::StageError;
use crate::world::World;

/// The key of the pseudo stage that holds the hashes of the seeded values.
pub(super) const KEY: &str = "parent#0";

/// Puts the values `plan` seeds into `world`: `parent.*` from the parent package, the others
/// from the entry of the kind's table that offers them. Returns the golden pseudo stage with
/// the hash of every seeded value, `None` when the plan seeds nothing.
///
/// # Errors
/// [`ChildError::NoParent`] or [`ChildError::ParentMismatch`] when the plan names a parent and
/// the run has none or another one; what building a value says ([`RunError::Child`],
/// [`RunError::Recipe`], [`RunError::Cancelled`]); a value the world refuses or one with no
/// canonical bytes.
pub(super) fn seed_world(
    world: &mut World,
    plan: &Plan,
    parent: Option<&ParentContext>,
    cancel: &CancelToken,
) -> Result<Option<GoldenStage>, RunError> {
    check_parent(plan, parent)?;
    let mut seeded = Vec::with_capacity(plan.seeded.len());
    for spec in &plan.seeded {
        let layer = if spec.name.starts_with(PARENT_PREFIX) {
            let parent = parent.ok_or_else(|| no_parent(spec.name))?;
            parent_value(parent, plan, spec.name, cancel)?
        } else {
            let own = kind::table(plan.kind)
                .own_seeds
                .iter()
                .find(|own| own.spec.name == spec.name)
                .ok_or_else(|| no_parent(spec.name))?;
            (own.build)(plan, parent)?
        };
        seeded.push((spec.name.to_owned(), layer));
    }
    if seeded.is_empty() {
        return Ok(None);
    }
    let mut layers = BTreeMap::new();
    for (name, layer) in seeded {
        layers.insert(name.clone(), hash_layer("parent", &name, &layer)?);
        world
            .insert(&name, layer)
            .map_err(|e| RunError::StageFailed {
                stage: "parent".to_owned(),
                source: StageError::Failed {
                    stage: "parent".to_owned(),
                    message: e.to_string(),
                },
            })?;
    }
    Ok(Some(GoldenStage {
        key: KEY.to_owned(),
        layers,
    }))
}

fn no_parent(value: &str) -> RunError {
    ChildError::NoParent {
        value: value.to_owned(),
    }
    .into()
}

/// A plan made from a parent runs only with that parent.
fn check_parent(plan: &Plan, parent: Option<&ParentContext>) -> Result<(), RunError> {
    let Some(planned) = &plan.parent else {
        return Ok(());
    };
    let given = parent.ok_or_else(|| no_parent("the parent package"))?;
    if given.sha256 != planned.package_sha256 {
        return Err(ChildError::ParentMismatch {
            planned: planned.package_sha256.clone(),
            given: given.sha256.clone(),
        }
        .into());
    }
    Ok(())
}
