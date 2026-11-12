//! The concepts part.

mod concept;
mod concept_version;

use crate::kinds::{KindRegistry, RegistryError};
use crate::parts::version_rules;

/// Registers the kinds of the concepts part.
///
/// # Errors
/// [`RegistryError`] when a kind is registered already.
pub fn register(registry: &mut KindRegistry) -> Result<(), RegistryError> {
    registry.register("concepts", concept::spec())?;
    registry.register("concepts", concept_version::spec())?;
    registry.register_rule("concepts", version_rules::picked(&["concept"]))?;
    registry.register_rule("concepts", version_rules::from(&["concept-version"]))?;
    registry.register_rule("concepts", version_rules::addresses(&["concept-version"]))?;
    Ok(())
}
