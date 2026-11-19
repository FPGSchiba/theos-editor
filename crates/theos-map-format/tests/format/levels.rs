//! Levels (`docs/map-package.md` §4.3) and the names that entry paths are built from.

use std::collections::BTreeMap;
use std::io::Cursor;

use serde_json::{Value, json};
use theos_map_format::{FormatError, LayerData, LayerKey, Package, read, write, written_manifest};

use crate::reader::{build, fixture_features, fixture_manifest};
use crate::writer::fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn room(level: &str) -> Value {
    json!({
        "type": "Feature",
        "geometry": {"type": "Point", "coordinates": [1.0, 2.0]},
        "properties": {"kind": "room", "level": level}
    })
}

// AC: CE-76.3
#[test]
fn a_two_level_package_round_trips() -> TestResult {
    let mut package = fixture()?;
    package.manifest.frame.levels = serde_json::from_value(json!([
        {"id": "ground", "elevation_m": 0.0},
        {"id": "gallery", "elevation_m": 6.0, "name": "Gallery"}
    ]))?;
    let mut entries = Vec::new();
    let mut layers = BTreeMap::new();
    for level in ["ground", "gallery"] {
        let mut entry = package.manifest.layers[1].clone();
        entry.name = "walkable".to_owned();
        entry.level = Some(level.to_owned());
        entries.push(entry);
        layers.insert(
            LayerKey {
                level: Some(level.to_owned()),
                name: "walkable".to_owned(),
            },
            LayerData::U8(vec![1; 12]),
        );
    }
    package.manifest.layers = entries;
    package.layers = layers;
    package.features = serde_json::from_value(json!({
        "type": "FeatureCollection",
        "features": [room("ground"), room("gallery")]
    }))?;
    let bytes = write(&package, Cursor::new(Vec::new()))?.into_inner();
    let zip = zip::ZipArchive::new(Cursor::new(bytes.clone()))?;
    let names: Vec<String> = zip
        .file_names()
        .map(|name| name.map(std::borrow::Cow::into_owned))
        .collect::<Result<_, _>>()?;
    assert!(names.iter().any(|n| n == "layers/ground/walkable.u8"));
    assert!(names.iter().any(|n| n == "layers/gallery/walkable.u8"));
    let back = read(Cursor::new(bytes))?;
    assert_eq!(back.manifest, written_manifest(&package)?);
    assert_eq!(back.features, package.features);
    assert_eq!(back.layers, package.layers);
    Ok(())
}

/// An edit of the fixture, on a frame with the one level `f1` where every layer and feature is
/// on `f1`, that breaks the levels rules in one place.
type Break = fn(&mut Value, &mut Value);

/// The refusal a case expects.
type Expect = fn(&FormatError) -> bool;

/// The fixture on the level `f1`: the frame lists it, every layer and feature is on it.
fn all_on_f1(manifest: &mut Value, features: &mut Value) {
    manifest["frame"]["levels"] = json!([{"id": "f1", "elevation_m": 0.0}]);
    for layer in manifest["layers"].as_array_mut().into_iter().flatten() {
        layer["level"] = json!("f1");
    }
    for feature in features["features"].as_array_mut().into_iter().flatten() {
        feature["properties"]["level"] = json!("f1");
    }
}

// AC: CE-76.3
#[test]
fn levels_rules_reject_missing_and_unknown_levels() -> TestResult {
    let cases: [(&str, Break, Expect); 4] = [
        (
            "a layer without a level",
            |m, _| {
                m["layers"][0].as_object_mut().map(|l| l.remove("level"));
            },
            |e| matches!(e, FormatError::LevelMissing { owner } if owner == "layer height"),
        ),
        (
            "a layer with an unknown level",
            |m, _| m["layers"][0]["level"] = json!("f9"),
            |e| {
                matches!(e, FormatError::LevelUnknown { owner, level }
                    if owner == "layer f9/height" && level == "f9")
            },
        ),
        (
            "a feature without a level",
            |_, f| {
                f["features"][0]["properties"]
                    .as_object_mut()
                    .map(|p| p.remove("level"));
            },
            |e| matches!(e, FormatError::LevelMissing { owner } if owner == "feature 0"),
        ),
        (
            "a feature with an unknown level",
            |_, f| f["features"][0]["properties"]["level"] = json!("f9"),
            |e| {
                matches!(e, FormatError::LevelUnknown { owner, level }
                    if owner == "feature 0" && level == "f9")
            },
        ),
    ];
    let (mut manifest, mut features) = (fixture_manifest()?, fixture_features()?);
    all_on_f1(&mut manifest, &mut features);
    let (package, bytes) = build(&manifest, &features)?;
    write(&package, Cursor::new(Vec::new()))?;
    read(Cursor::new(bytes))?;
    for (label, edit, expect) in cases {
        let (mut manifest, mut features) = (fixture_manifest()?, fixture_features()?);
        all_on_f1(&mut manifest, &mut features);
        edit(&mut manifest, &mut features);
        let (package, bytes) = build(&manifest, &features)?;
        let written = write(&package, Cursor::new(Vec::new())).err();
        assert!(
            written.as_ref().is_some_and(expect),
            "write, {label}: {written:?}"
        );
        let result = read(Cursor::new(bytes)).err();
        assert!(
            result.as_ref().is_some_and(expect),
            "read, {label}: {result:?}"
        );
    }
    Ok(())
}

fn is_bad_name(result: &Result<impl std::fmt::Debug, FormatError>, wanted: &str) -> bool {
    matches!(result, Err(FormatError::InvalidName { name, .. }) if name == wanted)
}

#[test]
fn layer_names_and_level_ids_that_leave_layers_are_refused_on_both_sides() -> TestResult {
    for bad in ["", ".", "..", "a/b", "a\\b", "a\0b"] {
        // A layer name.
        let (mut manifest, features) = (fixture_manifest()?, fixture_features()?);
        manifest["layers"][1]["name"] = json!(bad);
        let (package, bytes) = build(&manifest, &features)?;
        assert!(
            is_bad_name(&write(&package, Cursor::new(Vec::new())), bad),
            "write, layer name {bad:?}"
        );
        assert!(
            is_bad_name(&read(Cursor::new(bytes)), bad),
            "read, layer name {bad:?}"
        );

        // A level id on the frame and on a layer.
        let (mut manifest, features) = (fixture_manifest()?, fixture_features()?);
        manifest["frame"]["levels"] = json!([{"id": bad, "elevation_m": 0.0}]);
        for layer in manifest["layers"].as_array_mut().into_iter().flatten() {
            layer["level"] = json!(bad);
        }
        let (package, bytes) = build(&manifest, &features)?;
        assert!(
            is_bad_name(&write(&package, Cursor::new(Vec::new())), bad),
            "write, level {bad:?}"
        );
        assert!(
            is_bad_name(&read(Cursor::new(bytes)), bad),
            "read, level {bad:?}"
        );
    }
    Ok(())
}

#[test]
fn ordinary_names_are_allowed() -> TestResult {
    let mut package: Package = fixture()?;
    package.manifest.layers[0].name = "height-2.v1".to_owned();
    let key = LayerKey {
        level: None,
        name: "height".to_owned(),
    };
    if let Some(data) = package.layers.remove(&key) {
        package.layers.insert(
            LayerKey {
                level: None,
                name: "height-2.v1".to_owned(),
            },
            data,
        );
    }
    assert!(package.validate().is_ok());
    Ok(())
}
