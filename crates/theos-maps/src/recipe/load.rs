//! Loading recipes into a catalog: the built-ins first, then each folder in order. A recipe of
//! an id replaces an earlier one; a broken file is a problem, never an error (spec §5.5).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::RecipeError;
use super::builtin::BUILTIN;
use super::schema::{LoadedRecipe, RecipeOrigin, from_table, parse_table};

/// The recipes a run can choose from.
#[derive(Debug, Default)]
pub struct Catalog {
    /// The recipes by id.
    pub recipes: BTreeMap<String, LoadedRecipe>,
    /// The files that could not be loaded.
    pub problems: Vec<RecipeProblem>,
}

/// A recipe file that could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeProblem {
    /// The file (`builtin:<name>` for an embedded recipe).
    pub file: PathBuf,
    /// The id the file claims, when its TOML has a string `id`.
    pub id: Option<String>,
    /// What is wrong.
    pub message: String,
}

/// Loads the built-in recipes and then those of `dirs`, in order; later ones win by id.
///
/// A file that does not load is a [`RecipeProblem`]. When its TOML still names an `id`, the
/// recipe of that id is removed, so a broken override never lets the recipe it meant to replace
/// run silently.
///
/// # Errors
/// [`RecipeError::Io`] when a folder or file cannot be read.
pub fn catalog(dirs: &[PathBuf]) -> Result<Catalog, RecipeError> {
    catalog_with(BUILTIN, dirs)
}

/// [`catalog`] with the built-ins given.
pub(crate) fn catalog_with(
    builtins: &[(&str, &str)],
    dirs: &[PathBuf],
) -> Result<Catalog, RecipeError> {
    let mut catalog = Catalog::default();
    for (name, text) in builtins {
        let file = PathBuf::from(format!("builtin:{name}"));
        catalog.add(text, RecipeOrigin::Builtin, file);
    }
    for dir in dirs {
        for path in recipe_files(dir)? {
            let bytes = std::fs::read(&path).map_err(|source| RecipeError::Io {
                path: path.clone(),
                source,
            })?;
            match String::from_utf8(bytes) {
                Ok(text) => catalog.add(&text, RecipeOrigin::File(path.clone()), path),
                Err(e) => catalog.problems.push(RecipeProblem {
                    file: path,
                    id: None,
                    message: format!("the file is not UTF-8 text: {e}"),
                }),
            }
        }
    }
    Ok(catalog)
}

impl Catalog {
    /// Adds the recipe in `text`, or records why it cannot be.
    fn add(&mut self, text: &str, origin: RecipeOrigin, file: PathBuf) {
        let label = file.display().to_string();
        let raw = match parse_table(text, &label) {
            Ok(raw) => raw,
            Err(error) => return self.refuse(file, None, &error),
        };
        let id = raw
            .get("id")
            .and_then(toml::Value::as_str)
            .map(str::to_owned);
        match from_table(raw, origin, &label) {
            Ok(loaded) => {
                self.recipes.insert(loaded.recipe.id.clone(), loaded);
            }
            Err(error) => self.refuse(file, id, &error),
        }
    }

    /// Records a problem and removes the recipe of `id`, if the file names one.
    fn refuse(&mut self, file: PathBuf, id: Option<String>, error: &RecipeError) {
        if let Some(id) = &id {
            self.recipes.remove(id);
        }
        self.problems.push(RecipeProblem {
            file,
            id,
            message: error.to_string(),
        });
    }
}
/// The `*.toml` files directly inside `dir`, sorted by file name.
fn recipe_files(dir: &Path) -> Result<Vec<PathBuf>, RecipeError> {
    let io = |source| RecipeError::Io {
        path: dir.to_owned(),
        source,
    };
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(io)? {
        let path = entry.map_err(io)?.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "toml") {
            files.push(path);
        }
    }
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(files)
}

#[cfg(test)]
mod tests;
