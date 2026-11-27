//! The recipe schema and its parser (spec §5.5). TOML is read by the `toml` crate.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

use super::RecipeError;
use super::inputs::{InputDecl, Inputs};
use crate::stage::MapKind;

/// A recipe as the schema of spec §5.5 has it. Stage tables stay raw: each stage parses its own.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    /// The id, unique in a catalog.
    pub id: String,
    /// A title for people.
    pub title: String,
    /// The kind of map.
    pub kind: MapKind,
    /// `[rows, columns]`; terrain and interior maps need it, a settlement map has no `grid`.
    pub grid: Option<[usize; 2]>,
    /// `[width, height]` in kilometres; as `grid`.
    pub size_km: Option<[f64; 2]>,
    /// The `[child]` table of a child map; its keys are read by the stage that needs them.
    pub child: Option<toml::Table>,
    /// Extra renders beyond the standard set.
    #[serde(default)]
    pub renders: Vec<String>,
    /// The inputs the recipe declares (`[inputs.<name>]`).
    #[serde(default)]
    pub inputs: BTreeMap<String, InputDecl>,
    /// The stages, in order (`[[stage]]`).
    #[serde(default, rename = "stage")]
    pub stages: Vec<toml::Table>,
    /// Constraint ranges by metric path.
    #[serde(default)]
    pub constraints: BTreeMap<String, [f64; 2]>,
    /// Options per render (`[render.<id>]`).
    #[serde(default)]
    pub render: BTreeMap<String, toml::Table>,
}

/// Where a recipe came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecipeOrigin {
    /// Embedded in the crate.
    Builtin,
    /// A file in a recipe folder.
    File(PathBuf),
}

/// A parsed recipe with its raw table (the manifest keeps the recipe as it ran) and origin.
#[derive(Debug, Clone)]
pub struct LoadedRecipe {
    /// The typed recipe.
    pub recipe: Recipe,
    /// The recipe as TOML.
    pub raw: toml::Table,
    /// Where it came from.
    pub origin: RecipeOrigin,
}

/// Parses recipe text.
///
/// # Errors
/// [`RecipeError::Parse`] when the text is not TOML or does not match the schema (unknown
/// keys included); its `origin` is the file path, or `builtin`.
pub fn parse(text: &str, origin: RecipeOrigin) -> Result<LoadedRecipe, RecipeError> {
    let label = match &origin {
        RecipeOrigin::Builtin => "builtin".to_owned(),
        RecipeOrigin::File(path) => path.display().to_string(),
    };
    let raw = parse_table(text, &label)?;
    from_table(raw, origin, &label)
}

/// Parses text to a TOML table.
pub(super) fn parse_table(text: &str, label: &str) -> Result<toml::Table, RecipeError> {
    toml::from_str(text).map_err(|e| RecipeError::Parse {
        origin: label.to_owned(),
        message: e.to_string(),
    })
}

/// Reads the schema from a table.
pub(super) fn from_table(
    raw: toml::Table,
    origin: RecipeOrigin,
    label: &str,
) -> Result<LoadedRecipe, RecipeError> {
    refuse_unsupported_inputs(&raw, label)?;
    let recipe = toml::Value::Table(raw.clone())
        .try_into::<Recipe>()
        .map_err(|e| RecipeError::Parse {
            origin: label.to_owned(),
            message: e.to_string(),
        })?;
    Inputs(recipe.inputs.clone())
        .validate()
        .map_err(|e| RecipeError::Parse {
            origin: label.to_owned(),
            message: format!("input `{}`: {}", e.input, e.message),
        })?;
    Ok(LoadedRecipe {
        recipe,
        raw,
        origin,
    })
}

/// Refuses an input of type `cameras` (ruling L6) with a clearer message than "unknown variant".
fn refuse_unsupported_inputs(raw: &toml::Table, label: &str) -> Result<(), RecipeError> {
    let Some(toml::Value::Table(inputs)) = raw.get("inputs") else {
        return Ok(());
    };
    for (name, decl) in inputs {
        if decl.get("type").and_then(toml::Value::as_str) == Some("cameras") {
            return Err(RecipeError::Parse {
                origin: label.to_owned(),
                message: format!("input `{name}`: the input type `cameras` is unsupported"),
            });
        }
    }
    Ok(())
}

impl Recipe {
    /// The declared inputs.
    #[must_use]
    pub fn inputs(&self) -> Inputs {
        Inputs(self.inputs.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str =
        "id = \"t\"\ntitle = \"T\"\nkind = \"terrain\"\ngrid = [4, 8]\nsize_km = [2.0, 1.0]\n";

    #[test]
    fn a_minimal_recipe_gets_defaults() {
        let loaded = parse(MINIMAL, RecipeOrigin::Builtin).unwrap();
        assert_eq!(loaded.recipe.grid, Some([4, 8]));
        assert!(loaded.recipe.stages.is_empty() && loaded.recipe.renders.is_empty());
        assert_eq!(loaded.raw["id"].as_str(), Some("t"));
    }

    #[test]
    fn unknown_top_level_keys_are_refused() {
        let err = parse(&format!("{MINIMAL}bogus = 1\n"), RecipeOrigin::Builtin).unwrap_err();
        assert!(err.to_string().contains("bogus"), "{err}");
    }

    #[test]
    fn bad_toml_names_the_origin() {
        let origin = RecipeOrigin::File(PathBuf::from("x.toml"));
        let err = parse("id = ", origin).unwrap_err();
        assert!(matches!(&err, RecipeError::Parse { origin, .. } if origin == "x.toml"));
    }
}
