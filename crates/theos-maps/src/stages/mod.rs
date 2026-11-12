//! The built-in stages. A new stage is one module here plus one line in [`register`].

use crate::stage::Registry;

/// Adds every built-in stage to `registry`, one line per stage.
pub fn register(_registry: &mut Registry) {}
