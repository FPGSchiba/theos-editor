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

/// Edits of the fixture (given a leveled frame `f1`) that break the levels rules.
type Break = fn(&mut Value, &mut Value);

fn leveled(manifest: &mut Value) {
    manifest["frame"]["levels"] = json!([{"id": "f1", "elevation_m": 0.0}]);
}

// AC: CE-76.3
#[test]
fn levels_rules_reject_missing_and_unknown_levels() -> TestResult {
    let cases: [(&str, Break); 4] = [
        ("a layer without a level", |m, _| leveled(m)),
        ("a layer with an unknown level", |m, _| {
            leveled(m);
            m["layers"][0]["level"] = json!("f9");
            m["layers"][1]["level"] = json!("f1");
        }),
        ("a feature without a level", |m, f| {
            leveled(m);
            for layer in m["layers"].as_array_mut().into_iter().flatten() {
                layer["level"] = json!("f1");
            }
            f["features"][0]["properties"]
                .as_object_mut()
                .map(|p| p.remove("level"));
        }),
        ("a feature with an unknown level", |m, f| {
            leveled(m);
            for layer in m["layers"].as_array_mut().into_iter().flatten() {
                layer["level"] = json!("f1");
            }
            f["features"][0]["properties"]["level"] = json!("f9");
        }),
    ];
    for (label, edit) in cases {
        let (mut manifest, mut features) = (fixture_manifest()?, fixture_features()?);
        edit(&mut manifest, &mut features);
        let (package, bytes) = build(&manifest, &features)?;
        let written = write(&package, Cursor::new(Vec::new()));
        assert!(
            matches!(
                written,
                Err(FormatError::LevelMissing { .. } | FormatError::LevelUnknown { .. })
            ),
            "write accepted {label}"
        );
        let result = read(Cursor::new(bytes));
        assert!(
            matches!(
                result,
                Err(FormatError::LevelMissing { .. } | FormatError::LevelUnknown { .. })
            ),
            "read accepted {label}: {result:?}"
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
