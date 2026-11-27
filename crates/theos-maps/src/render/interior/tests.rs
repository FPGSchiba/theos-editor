//! Tests of the interior renders, on the synthetic `fixture-interior` recipe.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use theos_map_format::{LayerData, LayerKey};

use super::skin::BASE;
use crate::render::test_support::{TestResult, decode};
use crate::{CancelToken, Progress, RunManifest, RunOptions, RunRequest, run};

/// The fixture recipe folder.
fn recipes() -> Vec<PathBuf> {
    vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recipes")]
}

/// A run of `recipe` with seed 1 into a new folder: the folder, the manifest.
fn run_recipe(
    recipe: &str,
    dirs: &[PathBuf],
) -> Result<(tempfile::TempDir, RunManifest), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let request = RunRequest {
        recipe: recipe.to_owned(),
        seed: 1,
        inputs: serde_json::Map::new(),
        from: None,
        notes: Vec::new(),
        options: RunOptions::default(),
    };
    let manifest = run(
        &request,
        dirs,
        dir.path(),
        &mut |_: &Progress| {},
        &CancelToken::new(),
    )?;
    Ok((dir, manifest))
}

/// The bytes of output `role` of a run.
fn output(
    dir: &Path,
    manifest: &RunManifest,
    role: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let file = manifest
        .outputs
        .get(role)
        .ok_or(format!("no output {role}"))?;
    Ok(std::fs::read(dir.join(file))?)
}

/// The `fixture-interior` recipe with the extra render `blueprint-ruin`, in a new folder.
fn ruin_recipe() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(recipes()[0].join("fixture-interior.toml"))?
        .replace(
            "id      = \"fixture-interior\"",
            "id      = \"fixture-ruin\"",
        )
        .replace(
            "kind    = \"interior\"",
            "kind    = \"interior\"\nrenders = [\"blueprint-ruin\"]",
        );
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("fixture-ruin.toml"), text)?;
    Ok(dir)
}

// AC: CE-80.1
#[test]
fn base_is_frame_exact() -> TestResult {
    let (dir, manifest) = run_recipe("fixture-interior", &recipes())?;
    let png = decode(&output(dir.path(), &manifest, "base")?);
    let file = manifest.outputs.get("package").ok_or("no package")?;
    let package = theos_map_format::read(BufReader::new(File::open(dir.path().join(file))?))?;
    let key = LayerKey {
        level: None,
        name: "walls".to_owned(),
    };
    let Some(LayerData::U8(walls)) = package.layers.get(&key) else {
        return Err("no u8 walls layer".into());
    };
    let [rows, cols] = package
        .manifest
        .frame
        .grid
        .map(|n| usize::try_from(n).unwrap_or(0));
    assert_eq!(
        (png.width, png.height),
        (u32::try_from(2 * cols)?, u32::try_from(2 * rows)?)
    );
    let i = walls.iter().position(|&w| w != 0).ok_or("no wall cell")?;
    let (r, c) = (i / cols, i % cols);
    assert_eq!(png.rgb(2 * c, 2 * r), BASE.wall);
    assert_eq!(png.rgb(2 * c + 1, 2 * r + 1), BASE.wall);
    Ok(())
}

#[test]
fn blueprint_and_graph_render() -> TestResult {
    let (d1, m1) = run_recipe("fixture-interior", &recipes())?;
    let (d2, m2) = run_recipe("fixture-interior", &recipes())?;
    for role in ["base", "blueprint", "graph"] {
        let a = output(d1.path(), &m1, role)?;
        let png = decode(&a);
        assert!(png.width > 0 && png.height > 0, "{role}");
        assert_eq!(a, output(d2.path(), &m2, role)?, "{role} is not repeatable");
    }
    assert!(!m1.outputs.contains_key("blueprint-ruin"));
    Ok(())
}

#[test]
fn blueprint_ruin_is_extra_only() -> TestResult {
    let plain: BTreeMap<_, _> = run_recipe("fixture-interior", &recipes())?
        .1
        .outputs
        .into_iter()
        .collect();
    assert!(!plain.contains_key("blueprint-ruin"));
    let folder = ruin_recipe()?;
    let (dir, manifest) = run_recipe("fixture-ruin", &[folder.path().to_path_buf()])?;
    let ruin = output(dir.path(), &manifest, "blueprint-ruin")?;
    let blue = output(dir.path(), &manifest, "blueprint")?;
    assert_eq!(decode(&ruin).width, decode(&blue).width);
    assert_ne!(ruin, blue);
    Ok(())
}
