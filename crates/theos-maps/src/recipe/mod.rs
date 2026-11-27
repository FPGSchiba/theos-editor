//! Recipes: the TOML description of a map run (spec §5.5).
//!
//! [`parse`] reads one recipe, [`catalog`] gathers built-in and folder recipes into a
//! [`Catalog`] (folder recipes win by id; a broken file is a [`RecipeProblem`], not an error),
//! and [`plan()`] validates a recipe against the stage [`Registry`](crate::stage::Registry)
//! before any stage runs.

mod builtin;
mod inputs;
mod load;
mod plan;
mod schema;
#[cfg(test)]
pub(crate) mod test_stages;

use std::fmt::Write as _;
use std::path::PathBuf;

pub use inputs::{InputDecl, InputProblem, Inputs};
pub use load::{Catalog, RecipeProblem, catalog};
pub use plan::{MAX_CELL_M, MIN_CELL_M, Plan, PlannedStage, plan};
pub use schema::{LoadedRecipe, Recipe, RecipeOrigin, parse};

/// Why a recipe could not be read, parsed or validated.
#[derive(Debug, thiserror::Error)]
pub enum RecipeError {
    /// The text is not a recipe.
    #[error("cannot parse recipe {origin}: {message}")]
    Parse {
        /// The file path or `builtin:<name>`.
        origin: String,
        /// What the TOML parser or the schema said.
        message: String,
    },
    /// The recipe is a recipe, but not a valid one; the fields name what is known.
    #[error("{}", invalid_text(recipe, stage.as_deref(), layer.as_deref(), parameter.as_deref(), message))]
    Invalid {
        /// The recipe id.
        recipe: String,
        /// The stage (its id), when the problem lies in one.
        stage: Option<String>,
        /// The layer, when the problem names one.
        layer: Option<String>,
        /// The parameter or recipe key, when the problem names one.
        parameter: Option<String>,
        /// What is wrong.
        message: String,
    },
    /// No recipe has this id.
    #[error("unknown recipe `{0}`")]
    UnknownRecipe(String),
    /// A recipe folder or file could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Io {
        /// The folder or file.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
}

fn invalid_text(
    recipe: &str,
    stage: Option<&str>,
    layer: Option<&str>,
    parameter: Option<&str>,
    message: &str,
) -> String {
    let mut text = format!("recipe `{recipe}`");
    for (label, value) in [("stage", stage), ("layer", layer), ("parameter", parameter)] {
        if let Some(value) = value {
            let _ = write!(text, ", {label} `{value}`");
        }
    }
    let _ = write!(text, ": {message}");
    text
}
