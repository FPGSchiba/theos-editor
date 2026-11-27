//! Interior maps through the public API (P34): the fixture runs through every interior stage,
//! and the recipe loader refuses what the stages cannot build before any stage runs.

use std::path::PathBuf;

use theos_maps::recipe::RecipeError;
use theos_maps::{CancelToken, Progress, RunError, run};

use crate::support::{TestResult, recipes, request, run_package};

/// The constraint result `key` of a run.
fn result(manifest: &theos_maps::RunManifest, key: &str) -> Option<f64> {
    manifest
        .results
        .get(key)
        .and_then(serde_json::Value::as_f64)
}

// AC: CE-80.2
#[test]
fn fixture_interior_runs_and_passes() -> TestResult {
    for seed in 1..=3 {
        let (package, manifest) =
            run_package("fixture-interior", seed, &recipes()).map_err(|e| e.to_string())?;
        assert_eq!(
            manifest.results.get("pass"),
            Some(&serde_json::Value::Bool(true)),
            "seed {seed}: {:?}",
            manifest.results
        );
        let rooms =
            result(&manifest, "stages_playability_rooms").ok_or("the room count is a result")?;
        assert!((8.0..=13.0).contains(&rooms), "seed {seed}: {rooms} rooms");
        // The exit distances exist only when the graph has an exit room the party reaches
        // (ruling R29: `leaf` with `role = "exit"` places the program's exit).
        let playability = &package.manifest.summary["stages"]["playability"];
        for metric in ["start_to_exit_min_m", "goal_to_exit_min_m"] {
            let m = playability[metric]
                .as_f64()
                .ok_or(format!("seed {seed}: no {metric}"))?;
            assert!(m > 0.0, "seed {seed}: {metric} = {m}");
        }
        assert_eq!(manifest.frame.size_m, [40.0, 40.0]);
    }
    Ok(())
}

/// The fixture with `edit` applied to its text, as a recipe named `broken` in a new folder.
fn broken(edit: impl Fn(String) -> String) -> Result<tempfile::TempDir, std::io::Error> {
    let text = std::fs::read_to_string(recipes()[0].join("fixture-interior.toml"))?;
    let text = edit(text).replace("id      = \"fixture-interior\"", "id      = \"broken\"");
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("broken.toml"), text)?;
    Ok(dir)
}

/// A broken recipe: what is wrong, the edit that breaks the fixture, and the stage and the
/// parameter the refusal names.
type Refusal = (
    &'static str,
    fn(String) -> String,
    Option<&'static str>,
    &'static str,
);

// Spec §5.16: the recipe loader refuses what the stages cannot build.
#[test]
fn loader_refusals() -> TestResult {
    let cases: [Refusal; 6] = [
        (
            "an underscore in a room type",
            |t| t.replace("type = \"store\"", "type = \"store_room\""),
            Some("mission"),
            "room[3].type",
        ),
        (
            "17 keys",
            |t| {
                t.replacen(
                    "  [[stage.rule]]\n  rule = \"leaf\"",
                    "  [[stage.rule]]\n  rule = \"locked_leaf\"\n  count = 17",
                    1,
                )
            },
            Some("mission"),
            "rule",
        ),
        (
            "a pitch of 15.5 m at 1 m",
            |t| t.replace("pitch_m  = 8.0", "pitch_m  = 15.5"),
            Some("envelope"),
            "pitch_m",
        ),
        (
            "a cell size of 0.1 m",
            |t| t.replace("grid    = [40, 40]", "grid    = [400, 400]"),
            None,
            "size_km",
        ),
        (
            "an unknown rule",
            |t| t.replacen("rule = \"leaf\"", "rule = \"attic\"", 1),
            Some("mission"),
            "rule[1].rule",
        ),
        (
            "walls of 3 m in a block of 8 m with a jitter of 2 m",
            |t| {
                t.replace("pitch_m  = 8.0", "pitch_m  = 8.0\njitter_m = 2.0")
                    .replace("id = \"space\"", "id = \"space\"\nwall_m = 3.0")
            },
            Some("space"),
            "wall_m",
        ),
    ];
    for (what, edit, stage, parameter) in cases {
        let dir = broken(edit)?;
        let out = tempfile::tempdir()?;
        let mut stages_run = 0_u32;
        let result = run(
            &request("broken", 1),
            &[PathBuf::from(dir.path())],
            out.path(),
            &mut |p: &Progress| {
                if p.done == 0 && p.stage != "constraints" {
                    stages_run += 1;
                }
            },
            &CancelToken::new(),
        );
        match result {
            Err(RunError::Recipe(RecipeError::Invalid {
                stage: s,
                parameter: p,
                message,
                ..
            })) => {
                assert_eq!(s.as_deref(), stage, "{what}: {message}");
                assert_eq!(p.as_deref(), Some(parameter), "{what}: {message}");
            }
            other => {
                return Err(
                    format!("{what}: expected a refusal, got {:?}", other.map(|_| ())).into(),
                );
            }
        }
        assert_eq!(stages_run, 0, "{what}: no stage ran");
    }
    Ok(())
}
