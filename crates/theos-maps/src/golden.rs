//! Golden hashes (CE-77, spec §5.14): the SHA-256 of every layer each pipeline stage wrote, of
//! the `features` output and of the package bytes, kept in one file per generator version.
//!
//! [`compute`] runs a built-in recipe and returns its [`GoldenRun`]; [`compare`] names the first
//! difference between two runs in pipeline order; [`record`] writes a [`GoldenFile`] and never
//! overwrites one; [`load`] reads it back. The example `bless` records the file of the current
//! [`VERSION`](crate::VERSION) at [`golden_path`]; the golden test compares fresh runs with it.

mod compare;
mod file;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use self::compare::{GoldenMismatch, compare};
pub use self::file::{GoldenError, load, record};
use crate::api::{RunRequest, run_inner};
use crate::error::RunError;
use crate::progress::CancelToken;
use crate::runner::RunOptions;
use crate::stage::Registry;

/// The golden runs, as (built-in recipe id, seed), in the order of the golden file (spec §5.14).
pub const GOLDEN_RUNS: &[(&str, u64)] = &[
    ("fixture-small", 1),
    ("fixture-small", 2),
    ("hearth-valley", 1),
];

/// The golden hashes of one generator version (plan ruling L23): a `[[run]]` table per golden
/// run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenFile {
    /// The generator, `theos-maps@<version>`.
    pub generator: String,
    /// The golden runs, in the order of [`GOLDEN_RUNS`].
    pub run: Vec<GoldenRun>,
}

/// The hashes of one run, as stored in a golden file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenRun {
    /// The recipe id.
    pub recipe: String,
    /// The run seed.
    pub seed: u64,
    /// The SHA-256 (hex) of the bytes of the run's `features` output.
    pub features: String,
    /// The SHA-256 (hex) of the bytes of the run's `.theosmap` package.
    pub package: String,
    /// The stages in pipeline order.
    #[serde(rename = "stage")]
    pub stages: Vec<GoldenStage>,
}

/// The layers one stage wrote, by name, with their SHA-256 in hex.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenStage {
    /// The stage key, `<id>#<occurrence>`.
    pub key: String,
    /// Layer name to hash.
    pub layers: BTreeMap<String, String>,
}

/// The golden file of generator version `version`:
/// `<crate folder>/tests/fixtures/golden/<version>.toml`.
#[must_use]
pub fn golden_path(version: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("golden")
        .join(format!("{version}.toml"))
}

/// Runs the built-in recipe `recipe` with `seed`, default options and no inputs, and returns its
/// golden record. The run writes its outputs into `dir`, an existing folder that holds none of
/// their names, and leaves them there; the caller owns the folder (a temporary one, in the tests
/// and the example `bless`).
///
/// # Errors
/// The errors of [`run`](crate::run): [`RunError::Recipe`] for an id that is not built in,
/// [`RunError::OutputExists`] when `dir` already holds an output's name, [`RunError::Io`] when
/// `dir` is not a folder, and the errors of the stages and the outputs.
pub fn compute(recipe: &str, seed: u64, dir: &Path) -> Result<GoldenRun, RunError> {
    let request = RunRequest {
        recipe: recipe.to_owned(),
        seed,
        inputs: serde_json::Map::new(),
        from: None,
        notes: Vec::new(),
        options: RunOptions::default(),
    };
    let (_, golden) = run_inner(
        &Registry::builtin(),
        &request,
        &[],
        dir,
        &mut |_| {},
        &CancelToken::new(),
    )?;
    Ok(golden)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages_serialize_as_stage_tables() {
        let run = GoldenRun {
            recipe: "r".to_owned(),
            seed: 1,
            features: String::new(),
            package: String::new(),
            stages: vec![GoldenStage {
                key: "a#0".to_owned(),
                layers: BTreeMap::from([("x".to_owned(), "ab".to_owned())]),
            }],
        };
        let text = toml::to_string(&run).unwrap();
        assert!(text.contains("[[stage]]"), "{text}");
        let back: GoldenRun = toml::from_str(&text).unwrap();
        assert_eq!(back, run);
    }

    #[test]
    fn a_file_holds_run_tables_with_stage_tables() {
        let file = GoldenFile {
            generator: "theos-maps@9.9.9".to_owned(),
            run: vec![GoldenRun {
                recipe: "r".to_owned(),
                seed: 2,
                features: "f".to_owned(),
                package: "p".to_owned(),
                stages: vec![GoldenStage {
                    key: "a#0".to_owned(),
                    layers: BTreeMap::new(),
                }],
            }],
        };
        let text = toml::to_string(&file).unwrap();
        assert!(
            text.starts_with("generator = \"theos-maps@9.9.9\"\n"),
            "{text}"
        );
        assert!(text.contains("[[run]]"), "{text}");
        assert!(text.contains("[[run.stage]]"), "{text}");
        let back: GoldenFile = toml::from_str(&text).unwrap();
        assert_eq!(back, file);
    }

    #[test]
    fn the_golden_path_is_the_crate_fixture_of_the_version() {
        let path = golden_path("1.2.3");
        assert!(
            path.ends_with(Path::new("tests/fixtures/golden/1.2.3.toml")),
            "{path:?}"
        );
        assert!(path.starts_with(env!("CARGO_MANIFEST_DIR")), "{path:?}");
    }

    #[test]
    fn the_golden_runs_are_built_in_recipes() {
        let built_in = crate::recipe::catalog(&[]).unwrap();
        for (recipe, _) in GOLDEN_RUNS {
            assert!(built_in.recipes.contains_key(*recipe), "{recipe}");
        }
    }

    #[test]
    fn compute_refuses_a_recipe_that_is_not_built_in() {
        let dir = tempfile::tempdir().unwrap();
        let result = compute("api-small", 1, dir.path());
        assert!(
            matches!(
                result,
                Err(RunError::Recipe(crate::recipe::RecipeError::UnknownRecipe(ref id)))
                    if id == "api-small"
            ),
            "{result:?}"
        );
    }
}
