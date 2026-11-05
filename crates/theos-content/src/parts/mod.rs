//! The built-in editor parts. Each part registers its kinds; none lists all of them.

pub mod codex;
mod common;
pub mod concepts;
pub mod maps;
pub mod scripts;
mod version;

use crate::kinds::{KindRegistry, RegistryError};

/// A registry with the kinds of every built-in part.
///
/// # Errors
/// [`RegistryError`] when two parts register the same kind or an id pattern does not compile.
pub fn builtin() -> Result<KindRegistry, RegistryError> {
    let mut registry = KindRegistry::new();
    codex::register(&mut registry)?;
    scripts::register(&mut registry)?;
    concepts::register(&mut registry)?;
    maps::register(&mut registry)?;
    Ok(registry)
}
