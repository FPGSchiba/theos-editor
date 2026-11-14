//! The library API types: the run request, the manifest and the description (`docs/generators.md`
//! section 3, spec 5.12).

mod describe;
mod outputs;
mod run;
mod steps;

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use self::describe::describe;
pub use self::run::run;
use crate::recipe::RecipeProblem;
use crate::runner::RunOptions;

/// The generator's name in the manifest and the description.
pub const GENERATOR: &str = "theos-maps";
/// The generator's version.
pub const VERSION: &str = "1.0.0";

/// One run's request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRequest {
    /// The recipe id.
    pub recipe: String,
    /// The seed.
    pub seed: u64,
    /// The recipe's inputs.
    pub inputs: serde_json::Map<String, serde_json::Value>,
    /// A map to start from, when the run is a re-run.
    pub from: Option<PathBuf>,
    /// Notes that steer the run.
    pub notes: Vec<Note>,
    /// How the run is carried out.
    pub options: RunOptions,
}

/// A note attached to a feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// The note's id.
    pub id: String,
    /// What the note is about.
    pub about: String,
    /// The note's text.
    pub text: String,
}

/// What a run produced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    /// The generator's name.
    pub generator: String,
    /// The recipe id.
    pub recipe: String,
    /// The seed.
    pub seed: u64,
    /// The inputs the run used.
    pub inputs: serde_json::Map<String, serde_json::Value>,
    /// The map's frame.
    pub frame: ManifestFrame,
    /// The output files by name.
    pub outputs: BTreeMap<String, String>,
    /// The run's results.
    pub results: serde_json::Map<String, serde_json::Value>,
    /// Seconds per stage.
    pub timing: BTreeMap<String, f64>,
}

/// The map's frame in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFrame {
    /// Width and height in metres.
    pub size_m: [f64; 2],
}

/// What the generator can do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Description {
    /// The generator's name.
    pub name: String,
    /// The generator's version.
    pub version: String,
    /// The kinds of records it produces.
    pub produces: Vec<String>,
    /// Whether it accepts notes.
    pub accepts_notes: bool,
    /// The recipes it offers.
    pub recipes: Vec<RecipeInfo>,
    /// The recipe files that could not be loaded.
    pub problems: Vec<RecipeProblem>,
}

/// One recipe in the description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeInfo {
    /// The recipe id.
    pub id: String,
    /// The recipe's title.
    pub title: String,
    /// The inputs it declares.
    pub inputs: serde_json::Map<String, serde_json::Value>,
    /// Whether the seed matters.
    pub seeded: bool,
    /// Its stages in order.
    pub stages: Vec<StageInfo>,
}

/// One stage in the description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageInfo {
    /// The stage id.
    pub id: String,
    /// What the stage does.
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_description_serialises_with_the_protocol_keys() -> Result<(), serde_json::Error> {
        let d = Description {
            name: GENERATOR.to_owned(),
            version: VERSION.to_owned(),
            produces: vec!["map".to_owned()],
            accepts_notes: true,
            recipes: vec![RecipeInfo {
                id: "r".to_owned(),
                title: "R".to_owned(),
                inputs: serde_json::Map::new(),
                seeded: true,
                stages: vec![StageInfo {
                    id: "s".to_owned(),
                    summary: "x".to_owned(),
                }],
            }],
            problems: vec![RecipeProblem {
                file: PathBuf::from("a.toml"),
                id: None,
                message: "bad".to_owned(),
            }],
        };
        let v = serde_json::to_value(&d)?;
        for key in ["name", "version", "produces", "accepts_notes", "recipes"] {
            assert!(v.get(key).is_some(), "missing {key}");
        }
        for key in ["id", "title", "inputs", "seeded"] {
            assert!(v["recipes"][0].get(key).is_some(), "missing {key}");
        }
        let back: Description = serde_json::from_value(v)?;
        assert_eq!(back.problems[0].message, "bad");
        Ok(())
    }

    #[test]
    fn a_run_manifest_serialises_with_the_protocol_keys() -> Result<(), serde_json::Error> {
        let m = RunManifest {
            generator: GENERATOR.to_owned(),
            recipe: "r".to_owned(),
            seed: 7,
            inputs: serde_json::Map::new(),
            frame: ManifestFrame {
                size_m: [1000.0, 500.0],
            },
            outputs: BTreeMap::from([("map".to_owned(), "m.theosmap".to_owned())]),
            results: serde_json::Map::new(),
            timing: BTreeMap::from([("s".to_owned(), 0.5)]),
        };
        let v = serde_json::to_value(&m)?;
        for key in [
            "generator",
            "recipe",
            "seed",
            "inputs",
            "frame",
            "outputs",
            "results",
        ] {
            assert!(v.get(key).is_some(), "missing {key}");
        }
        assert_eq!(v["frame"]["size_m"], json!([1000.0, 500.0]));
        let back: RunManifest = serde_json::from_value(v)?;
        assert_eq!(back.seed, 7);
        Ok(())
    }

    #[test]
    fn a_request_deserialises_from_json() -> Result<(), serde_json::Error> {
        let r: RunRequest = serde_json::from_value(json!({
            "recipe": "r", "seed": 1, "inputs": {}, "from": null,
            "notes": [{"id": "n", "about": "a", "text": "t"}], "options": {}
        }))?;
        assert_eq!(r.notes[0].id, "n");
        assert_eq!(r.options.memory_budget, None);
        Ok(())
    }
}
