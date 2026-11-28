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

/// One golden run: a built-in recipe, a seed, string inputs and, for a child recipe, the golden
/// run of its parent (spec §5.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoldenSpec {
    /// The built-in recipe id.
    pub recipe: &'static str,
    /// The run seed.
    pub seed: u64,
    /// The recipe inputs, name to string value.
    pub inputs: &'static [(&'static str, &'static str)],
    /// The parent's golden run as (recipe, seed); the parent runs first and the child starts
    /// from its package.
    pub from: Option<(&'static str, u64)>,
}

/// The golden runs, in the order of the golden file (spec §5.14).
pub const GOLDEN_RUNS: &[GoldenSpec] = &[
    GoldenSpec {
        recipe: "fixture-small",
        seed: 1,
        inputs: &[],
        from: None,
    },
    GoldenSpec {
        recipe: "fixture-small",
        seed: 2,
        inputs: &[],
        from: None,
    },
    GoldenSpec {
        recipe: "hearth-valley",
        seed: 1,
        inputs: &[],
        from: None,
    },
    GoldenSpec {
        recipe: "choir-hall-ruin",
        seed: 1,
        inputs: &[],
        from: None,
    },
    // `ruin-1` is the one ruin `fixture-small` seed 1 places.
    GoldenSpec {
        recipe: "ruin-undercroft",
        seed: 1,
        inputs: &[("feature", "ruin-1")],
        from: Some(("fixture-small", 1)),
    },
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
    /// The recipe inputs, name to string value; absent from the file when there are none.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inputs: BTreeMap<String, String>,
    /// The parent run a child run started from; absent for a standalone run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<GoldenParent>,
    /// The SHA-256 (hex) of the bytes of the run's `features` output.
    pub features: String,
    /// The SHA-256 (hex) of the bytes of the run's `.theosmap` package.
    pub package: String,
    /// The stages in pipeline order.
    #[serde(rename = "stage")]
    pub stages: Vec<GoldenStage>,
}

/// The parent run of a child golden run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenParent {
    /// The parent's recipe id.
    pub recipe: String,
    /// The parent's seed.
    pub seed: u64,
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

/// Runs the golden run `spec` with default options and returns its golden record. With
/// `spec.from`, the parent's recipe and seed run first into `dir/parent/` and the child starts
/// from that package. The runs write their outputs into `dir`, an existing folder that holds
/// none of their names, and leave them there; the caller owns the folder (a temporary one, in
/// the tests and the example `bless`).
///
/// # Errors
/// The errors of [`run`](crate::run): [`RunError::Recipe`] for an id that is not built in,
/// [`RunError::OutputExists`] when `dir` already holds an output's name, [`RunError::Io`] when
/// `dir` is not a folder, and the errors of the stages and the outputs.
pub fn compute(spec: &GoldenSpec, dir: &Path) -> Result<GoldenRun, RunError> {
    compute_in(&Registry::builtin(), &[], spec, dir)
}

/// [`compute`] with the given registry and extra recipe folders (the tests' child recipes).
pub(crate) fn compute_in(
    registry: &Registry,
    recipe_dirs: &[PathBuf],
    spec: &GoldenSpec,
    dir: &Path,
) -> Result<GoldenRun, RunError> {
    let inputs: BTreeMap<String, String> = spec
        .inputs
        .iter()
        .map(|&(name, value)| (name.to_owned(), value.to_owned()))
        .collect();
    let mut request = RunRequest {
        recipe: spec.recipe.to_owned(),
        seed: spec.seed,
        inputs: inputs
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::from(v.as_str())))
            .collect(),
        from: None,
        notes: Vec::new(),
        options: RunOptions::default(),
    };
    let mut parent = None;
    if let Some((recipe, seed)) = spec.from {
        let parent_dir = dir.join("parent");
        std::fs::create_dir(&parent_dir).map_err(|source| RunError::Io {
            path: parent_dir.clone(),
            source,
        })?;
        let parent_spec = GoldenSpec {
            recipe,
            seed,
            inputs: &[],
            from: None,
        };
        let (manifest, _) = run_golden(registry, recipe_dirs, &parent_spec, &parent_dir)?;
        let file = manifest.outputs.get("package").cloned().unwrap_or_default();
        request.from = Some(parent_dir.join(file));
        parent = Some(GoldenParent {
            recipe: recipe.to_owned(),
            seed,
        });
    }
    let (_, mut golden) = run_inner(
        registry,
        &request,
        recipe_dirs,
        dir,
        &mut |_| {},
        &CancelToken::new(),
    )?;
    golden.inputs = inputs;
    golden.from = parent;
    Ok(golden)
}

/// Runs a standalone `spec` (the parent of a child run).
fn run_golden(
    registry: &Registry,
    recipe_dirs: &[PathBuf],
    spec: &GoldenSpec,
    dir: &Path,
) -> Result<(crate::api::RunManifest, GoldenRun), RunError> {
    let request = RunRequest {
        recipe: spec.recipe.to_owned(),
        seed: spec.seed,
        inputs: serde_json::Map::new(),
        from: None,
        notes: Vec::new(),
        options: RunOptions::default(),
    };
    run_inner(
        registry,
        &request,
        recipe_dirs,
        dir,
        &mut |_| {},
        &CancelToken::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages_serialize_as_stage_tables() {
        let run = GoldenRun {
            recipe: "r".to_owned(),
            seed: 1,
            inputs: BTreeMap::new(),
            from: None,
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
                inputs: BTreeMap::new(),
                from: None,
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
        for spec in GOLDEN_RUNS {
            assert!(
                built_in.recipes.contains_key(spec.recipe),
                "{}",
                spec.recipe
            );
        }
    }

    #[test]
    fn compute_refuses_a_recipe_that_is_not_built_in() {
        let dir = tempfile::tempdir().unwrap();
        let result = compute(
            &GoldenSpec {
                recipe: "api-small",
                seed: 1,
                inputs: &[],
                from: None,
            },
            dir.path(),
        );
        assert!(
            matches!(
                result,
                Err(RunError::Recipe(crate::recipe::RecipeError::UnknownRecipe(ref id)))
                    if id == "api-small"
            ),
            "{result:?}"
        );
    }

    #[test]
    fn golden_runs_round_trip_with_inputs_and_parent() {
        let run = GoldenRun {
            recipe: "child".to_owned(),
            seed: 7,
            inputs: BTreeMap::from([("feature".to_owned(), "town-1".to_owned())]),
            from: Some(GoldenParent {
                recipe: "fixture-small".to_owned(),
                seed: 1,
            }),
            features: "f".to_owned(),
            package: "p".to_owned(),
            stages: vec![],
        };
        let text = toml::to_string(&run).unwrap();
        assert!(
            text.contains("[inputs]") && text.contains("[from]"),
            "{text}"
        );
        assert_eq!(toml::from_str::<GoldenRun>(&text).unwrap(), run);
        // A run without them (the 1.0.0 file) writes and parses neither.
        let plain = GoldenRun {
            inputs: BTreeMap::new(),
            from: None,
            ..run
        };
        let text = toml::to_string(&plain).unwrap();
        assert!(!text.contains("inputs") && !text.contains("from"), "{text}");
        assert_eq!(toml::from_str::<GoldenRun>(&text).unwrap(), plain);
    }

    #[test]
    fn slow_a_golden_spec_with_a_parent_runs_the_parent_first() {
        // The real interior stages, under the id `child` and with a feature input.
        let fixture = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/recipes/fixture-interior.toml"),
        )
        .unwrap()
        .replace("fixture-interior", "child")
        .replace(
            "size_km = [0.04, 0.04]",
            "size_km = [0.04, 0.04]
[inputs.feature]
type = \"feature\"
parent_types = [\"town\", \"village\"]",
        );
        let recipes = tempfile::tempdir().unwrap();
        std::fs::write(recipes.path().join("child.toml"), fixture).unwrap();
        let dirs = [
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recipes"),
            recipes.path().to_path_buf(),
        ];
        let spec = GoldenSpec {
            recipe: "child",
            seed: 7,
            inputs: &[("feature", "town-1")],
            from: Some(("api-small", 1)),
        };
        let out = tempfile::tempdir().unwrap();
        let run = compute_in(
            &crate::child::test_support::registry_with_builtin(),
            &dirs,
            &spec,
            out.path(),
        )
        .unwrap();
        assert_eq!(
            run.from,
            Some(GoldenParent {
                recipe: "api-small".to_owned(),
                seed: 1
            })
        );
        assert_eq!(run.inputs["feature"], "town-1");
        // An interior reads no parent layer, so it seeds nothing and has no `parent#0` stage.
        assert_eq!(
            run.stages.first().map(|s| s.key.as_str()),
            Some("envelope#0")
        );
        assert!(out.path().join("parent").is_dir());
    }
}
