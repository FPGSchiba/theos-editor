//! The maps part.

mod map;
mod map_version;

use crate::kinds::{KindRegistry, RegistryError};

/// Registers the kinds of the maps part.
///
/// # Errors
/// [`RegistryError`] when a kind is registered already.
pub fn register(registry: &mut KindRegistry) -> Result<(), RegistryError> {
    registry.register("maps", map::spec())?;
    registry.register("maps", map_version::spec())?;
    Ok(())
}
