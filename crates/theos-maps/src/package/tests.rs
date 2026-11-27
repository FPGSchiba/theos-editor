use std::io::Cursor;

use theos_map_format::{DType, LayerData, LayerKey, Package};

use super::table::{ROADS_LEGEND, ROCK_LEGEND};
use super::{PackageError, RunRecord, TERRAIN_LAYERS, build};
use crate::features::output::build as build_features;
use crate::grid::Grid;
use crate::kind::IdScheme;
use crate::progress::CancelToken;
use crate::recipe::{LoadedRecipe, Plan, RecipeOrigin, parse, plan};
use crate::runner::{RunOptions, execute};
use crate::stage::{MapKind, Registry};
use crate::stages::terrain::roads::RoadClass;
use crate::stages::terrain::rocks::Rock;
use crate::world::{Layer, MapFrame, World};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const WATER_SMALL: &str = include_str!("../../tests/fixtures/recipes/water-small.toml");

/// Runs the recipe and returns what `build` needs.
fn run(seed: u64) -> Result<(World, Plan, LoadedRecipe), Box<dyn std::error::Error>> {
    let loaded = parse(WATER_SMALL, RecipeOrigin::Builtin)?;
    let registry = Registry::builtin();
    let planned = plan(
        &loaded,
        &registry,
        &serde_json::Map::new(),
        crate::recipe::Planning::Standalone,
    )?;
    let done = execute(
        &planned,
        seed,
        &registry,
        &RunOptions::default(),
        None,
        &mut |_| {},
        &CancelToken::new(),
    )?;
    Ok((done.world, planned, loaded))
}

fn package_of(world: &World, plan: &Plan, loaded: &LoadedRecipe) -> Result<Package, PackageError> {
    let features = build_features(world, IdScheme::ByType)
        .map(|f| f.0)
        .unwrap_or_default();
    let inputs = serde_json::json!({"size": "small"})
        .as_object()
        .cloned()
        .unwrap_or_default();
    build(
        world,
        plan,
        RunRecord {
            raw_recipe: &loaded.raw,
            seed: 7,
            inputs: &inputs,
            results: serde_json::json!({"lakes": 2})
                .as_object()
                .cloned()
                .unwrap_or_default(),
            summary: serde_json::json!({"k": 1}),
            features,
        },
    )
}

fn names(package: &Package) -> Vec<&str> {
    package
        .manifest
        .layers
        .iter()
        .map(|l| l.name.as_str())
        .collect()
}

fn key(name: &str) -> LayerKey {
    LayerKey {
        level: None,
        name: name.to_owned(),
    }
}

#[test]
fn the_package_holds_the_standard_terrain_layers_in_order() -> TestResult {
    let (world, planned, loaded) = run(1)?;
    let package = package_of(&world, &planned, &loaded)?;
    let mut expected: Vec<&str> = TERRAIN_LAYERS.iter().map(|l| l.name).collect();
    expected.push("omega");
    assert_eq!(names(&package), expected);
    assert_eq!(
        &expected[..8],
        [
            "height",
            "water_surface",
            "flow_dir",
            "drainage_area",
            "erosion",
            "deposition",
            "discharge",
            "precipitation"
        ]
    );
    assert_eq!(package.layers.len(), expected.len());
    // the legends agree with the enums they describe
    for rock in Rock::ALL {
        assert!(ROCK_LEGEND.contains(&(rock.code().to_string().as_str(), rock.name())));
    }
    for class in RoadClass::ALL {
        assert!(ROADS_LEGEND.contains(&(class.code().to_string().as_str(), class.name())));
    }
    // and hold nothing else (final review round 4, class K)
    assert_eq!(ROCK_LEGEND.len(), Rock::ALL.len());
    assert_eq!(ROADS_LEGEND.len(), RoadClass::ALL.len() + 1);
    assert_eq!(ROADS_LEGEND[0], ("0", "none"));
    Ok(())
}

#[test]
fn quantised_layers_round_trip_within_half_a_step() -> TestResult {
    let (mut world, planned, loaded) = run(1)?;
    let values: Vec<f64> = (0..64 * 64).map(|i| f64::from(i % 101) / 100.0).collect();
    world.insert("snow", Layer::F64(Grid::from_vec(64, 64, values.clone())?))?;
    let package = package_of(&world, &planned, &loaded)?;
    let mut bytes = Vec::new();
    theos_map_format::write(&package, Cursor::new(&mut bytes))?;
    let back = theos_map_format::read(Cursor::new(bytes))?;
    let LayerData::U8(raw) = &back.layers[&key("snow")] else {
        return Err("snow is not u8".into());
    };
    let scale = back
        .manifest
        .layers
        .iter()
        .find(|l| l.name == "snow")
        .and_then(|l| l.scale);
    let scale = scale.ok_or("no scale")?;
    for (&q, &v) in raw.iter().zip(&values) {
        assert!((f64::from(q) * scale - v).abs() <= 0.5 / 255.0 + 1e-12);
    }
    let height = world.f64("height").ok_or("height")?;
    let LayerData::F32(stored) = &back.layers[&key("height")] else {
        return Err("height is not f32".into());
    };
    for (&s, &h) in stored.iter().zip(height.as_slice()) {
        assert!((f64::from(s) - h).abs() <= h.abs() * 1.2e-7 + 1e-30);
    }
    Ok(())
}

#[test]
fn the_manifest_records_recipe_seed_frame_and_empty_requires() -> TestResult {
    let (world, planned, loaded) = run(1)?;
    let package = package_of(&world, &planned, &loaded)?;
    let m = &package.manifest;
    assert_eq!(m.generator, "theos-maps@1.0.0");
    assert_eq!(m.requires.len(), 0);
    assert_eq!(m.kind, "terrain");
    assert_eq!(m.recipe_id, "water-small");
    assert_eq!(m.recipe["id"], "water-small");
    assert_eq!(m.seed, 7);
    assert_eq!(m.frame.grid, [64, 64]);
    assert_eq!(m.frame.size_m, [6400.0, 6400.0]);
    assert_eq!(m.frame.cell_m, 100.0);
    assert_eq!(m.inputs["size"], "small");
    assert_eq!(m.results["lakes"], 2);
    assert_eq!(m.summary["k"], 1);
    let water = m.layers.iter().find(|l| l.name == "water").ok_or("water")?;
    assert_eq!(water.dtype, DType::U8);
    assert_eq!(water.legend.as_ref().map(|l| l["3"].as_str()), Some("lake"));
    Ok(())
}

#[test]
fn omega_is_written_only_when_detuning_ran() -> TestResult {
    let (mut world, planned, loaded) = run(1)?;
    assert!(names(&package_of(&world, &planned, &loaded)?).contains(&"omega"));
    world.remove("omega");
    let package = package_of(&world, &planned, &loaded)?;
    assert!(!names(&package).contains(&"omega"));
    assert!(!package.layers.contains_key(&key("omega")));
    Ok(())
}

#[test]
fn the_package_reads_back_with_the_format_crate() -> TestResult {
    let (world, planned, loaded) = run(1)?;
    let package = package_of(&world, &planned, &loaded)?;
    let mut bytes = Vec::new();
    theos_map_format::write(&package, Cursor::new(&mut bytes))?;
    let back = theos_map_format::read(Cursor::new(bytes))?;
    let written = theos_map_format::written_manifest(&package)?;
    assert!(
        written
            .layers
            .iter()
            .all(|l| l.min.is_some() && l.max.is_some())
    );
    assert_eq!(back.manifest, written);
    assert_eq!(back.layers, package.layers);
    assert_eq!(
        back.features.features.len(),
        package.features.features.len()
    );
    let water = &back.layers[&key("water")];
    assert!(matches!(water, LayerData::U8(v) if v.iter().all(|&c| c <= 3)));
    Ok(())
}

#[test]
fn water_classes_let_a_lake_win_over_a_big_river_over_a_stream() -> TestResult {
    let mask = |v: [bool; 4]| -> Result<Layer, Box<dyn std::error::Error>> {
        Ok(Layer::Mask(Grid::from_vec(1, 4, v.to_vec())?))
    };
    let mut small = World::new(MapFrame {
        rows: 1,
        cols: 4,
        cell_m: 1.0,
    });
    let layer = TERRAIN_LAYERS
        .iter()
        .find(|l| l.name == "water")
        .ok_or("water")?;
    let err = super::encode::encode(&small, layer)
        .err()
        .ok_or("encoded")?;
    assert!(matches!(err, PackageError::MissingLayer { layer: "lake" }));
    small.insert("lake", mask([false, false, false, true])?)?;
    small.insert("big_river", mask([false, false, true, true])?)?;
    small.insert("river", mask([false, true, true, true])?)?;
    let data = super::encode::encode(&small, layer)?;
    assert_eq!(data, LayerData::U8(vec![0, 1, 2, 3]));
    Ok(())
}

#[test]
fn a_missing_layer_is_an_error_naming_it() -> TestResult {
    let (mut world, planned, loaded) = run(1)?;
    world.remove("height");
    let err = package_of(&world, &planned, &loaded).err().ok_or("built")?;
    assert!(matches!(
        err,
        PackageError::MissingLayer { layer: "height" }
    ));
    assert!(err.to_string().contains("height"));
    Ok(())
}

// Ledger ruling P3: `needs` lists exactly what `build` reads, with the kind it reads: a world
// holding only those layers builds, and without any one of them it does not.
#[test]
fn the_package_needs_exactly_the_layers_it_reads() -> TestResult {
    let (mut world, planned, loaded) = run(1)?;
    let needs = super::needs(MapKind::Terrain);
    let names: Vec<String> = world.names().map(str::to_owned).collect();
    for name in names {
        if needs.iter().all(|spec| spec.name != name) {
            world.remove(&name);
        }
    }
    package_of(&world, &planned, &loaded)?;
    for spec in &needs {
        let layer = world.remove(spec.name).ok_or(spec.name)?;
        assert_eq!(layer.kind(), spec.kind, "{}", spec.name);
        assert!(
            package_of(&world, &planned, &loaded).is_err(),
            "the package builds without `{}`",
            spec.name
        );
        world.insert(spec.name, layer)?;
    }
    Ok(())
}

#[test]
fn a_grid_beyond_32_bits_is_refused() -> TestResult {
    let (_, planned, loaded) = run(1)?;
    let huge = World::new(MapFrame {
        rows: 1 << 33,
        cols: 1,
        cell_m: 1.0,
    });
    let err = package_of(&huge, &planned, &loaded).err().ok_or("built")?;
    assert!(matches!(err, PackageError::GridTooLarge { .. }));
    Ok(())
}

#[test]
fn roads_are_all_zero_when_the_recipe_has_no_roads_stage() -> TestResult {
    let (mut world, planned, loaded) = run(1)?;
    world.remove("roads");
    let package = package_of(&world, &planned, &loaded)?;
    match package.layers.get(&key("roads")) {
        Some(LayerData::U8(classes)) => {
            assert_eq!(classes.len(), 64 * 64);
            assert!(classes.iter().all(|&c| c == 0));
        }
        other => return Err(format!("expected a u8 roads layer, got {other:?}").into()),
    }
    Ok(())
}

// Spec §5.13: the manifest holds no NaN or infinity. A recipe number no check refuses (a render
// option, say) would otherwise reach the manifest's `recipe` as `null`.
#[test]
fn a_recipe_number_that_is_not_finite_is_refused() -> TestResult {
    let (world, planned, mut loaded) = run(1)?;
    for bad in [f64::NAN, f64::INFINITY] {
        loaded
            .raw
            .insert("probe".to_owned(), toml::Value::Float(bad));
        let err = package_of(&world, &planned, &loaded).err().ok_or("built")?;
        assert!(
            matches!(
                err,
                PackageError::Recipe(theos_map_format::FormatError::JsonNotFinite)
            ),
            "{bad}: {err:?}"
        );
        assert!(err.to_string().contains("recipe"), "{err}");
    }
    Ok(())
}
