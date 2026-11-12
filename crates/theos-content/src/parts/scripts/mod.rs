//! The scripts part.

mod dialogue;
mod rules;
mod script;
mod template;

use crate::kinds::{KindRegistry, RegistryError};

/// Registers the kinds of the scripts part.
///
/// # Errors
/// [`RegistryError`] when a kind is registered already.
pub fn register(registry: &mut KindRegistry) -> Result<(), RegistryError> {
    registry.register("scripts", script::spec())?;
    registry.register("scripts", dialogue::spec())?;
    registry.register("scripts", template::spec())?;
    registry.register_rule("scripts", rules::BEAT_AT)?;
    registry.register_rule("scripts", rules::SAYS_REJECTED)?;
    registry.register_rule("scripts", rules::REVEAL_WHERE)?;
    registry.register_rule("scripts", rules::REVEAL_GATE)?;
    Ok(())
}
