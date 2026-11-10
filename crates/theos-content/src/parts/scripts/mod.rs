//! The scripts part.

mod dialogue;
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
    Ok(())
}
