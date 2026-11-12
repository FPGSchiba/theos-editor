//! The maps part.

mod map;
mod map_version;

use crate::kinds::{KindRegistry, RegistryError};
use crate::parts::version_rules;

/// Registers the kinds of the maps part.
///
/// # Errors
/// [`RegistryError`] when a kind is registered already.
pub fn register(registry: &mut KindRegistry) -> Result<(), RegistryError> {
    registry.register("maps", map::spec())?;
    registry.register("maps", map_version::spec())?;
    registry.register_rule("maps", version_rules::picked(&["map"]))?;
    registry.register_rule("maps", version_rules::from(&["map-version"]))?;
    registry.register_rule("maps", version_rules::addresses(&["map-version"]))?;
    Ok(())
}
