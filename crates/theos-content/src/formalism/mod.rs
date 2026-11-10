//! The Shared Formalism mirror (spec §5.14): the format of `canon/formalism/_source.json`, the
//! equation records rendered from it and the regeneration that writes them.

mod regenerate;
mod render;
mod source;

pub use regenerate::{FormalismError, Regenerated, regenerate};
pub use render::{render, render_equation};
pub use source::{Equation, Source, SourceError, parse};

/// The folder of the equation records, project-relative.
pub const FOLDER: &str = "canon/formalism";

/// The source file, project-relative.
pub const SOURCE: &str = "canon/formalism/_source.json";
