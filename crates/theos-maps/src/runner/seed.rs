//! The values a run seeds into its world before the first stage (spec P29 §5.2 item 5) and
//! their golden pseudo stage `parent#0` (L12).

use std::collections::BTreeMap;

use super::hash_layer;
use crate::child::ParentContext;
use crate::error::RunError;
use crate::golden::GoldenStage;
use crate::kind;
use crate::progress::CancelToken;
use crate::recipe::Plan;
use crate::stage::StageError;
use crate::world::World;

/// The key of the pseudo stage that holds the hashes of the seeded values.
pub(super) const KEY: &str = "parent#0";

/// Puts the values the kind of `plan` seeds into `world`. Returns the golden pseudo stage with
/// the hash of every seeded value, `None` when the plan seeds nothing.
///
/// # Errors
/// What the kind's seeding says ([`RunError::Child`], [`RunError::Cancelled`]), a value the
/// world refuses or one with no canonical bytes.
pub(super) fn seed_world(
    world: &mut World,
    plan: &Plan,
    parent: Option<&ParentContext>,
    cancel: &CancelToken,
) -> Result<Option<GoldenStage>, RunError> {
    let seeded = (kind::table(plan.kind).seeded)(parent, plan, cancel)?;
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
