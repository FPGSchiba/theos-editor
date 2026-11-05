//! The concepts part.

mod concept;
mod concept_version;

use crate::kinds::{KindRegistry, RegistryError};

/// Registers the kinds of the concepts part.
///
/// # Errors
/// [`RegistryError`] when a kind is registered already.
pub fn register(registry: &mut KindRegistry) -> Result<(), RegistryError> {
    registry.register("concepts", concept::spec())?;
    registry.register("concepts", concept_version::spec())?;
    Ok(())
}
