//! The codex part.

mod band;
mod belief;
mod clue;
mod equation;
mod faction;
mod floor;
mod location;
mod npc;
mod people;
mod rules;
mod thing;
mod topic;
mod tower;
mod truth;

use crate::kinds::{KindRegistry, RegistryError};

/// Registers the kinds of the codex part.
///
/// # Errors
/// [`RegistryError`] when a kind is registered already.
pub fn register(registry: &mut KindRegistry) -> Result<(), RegistryError> {
    registry.register("codex", tower::spec())?;
    registry.register("codex", band::spec())?;
    registry.register("codex", floor::spec())?;
    registry.register("codex", location::spec())?;
    registry.register("codex", people::spec())?;
    registry.register("codex", faction::spec())?;
    registry.register("codex", npc::spec())?;
    registry.register("codex", thing::spec())?;
    registry.register("codex", topic::spec())?;
    registry.register("codex", truth::spec())?;
    registry.register("codex", belief::spec())?;
    registry.register("codex", clue::spec())?;
    registry.register("codex", equation::spec())?;
    registry.register_rule("codex", rules::FLOOR_NUMBER)?;
    registry.register_rule("codex", rules::PARENT_CYCLE)?;
    registry.register_rule("codex", rules::NEVER_CONFIRM)?;
    registry.register_rule("codex", rules::NO_OVERSHOOT)?;
    registry.register_rule("codex", rules::REVEAL_ORDER)?;
    registry.register_rule("codex", rules::REQUIRES)?;
    registry.register_rule("codex", rules::THREE_CLUE)?;
    Ok(())
}
