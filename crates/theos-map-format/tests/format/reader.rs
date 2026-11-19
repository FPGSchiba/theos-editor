//! The package reader: round trips, version and capability refusals, leniency, integrity checks.

use std::io::{Cursor, Write};

use serde_json::{Value, json};
use theos_map_format::{
    FormatError, LayerData, LayerKey, Manifest, Package, read, sha256_hex, write, written_manifest,
};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, System, ZipWriter};

use crate::writer::{DIR, fixture};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Builds a package from raw entries with the `zip` crate, bypassing the writer.
pub fn assemble(entries: &[(&str, Vec<u8>)]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(6))
            .last_modified_time(DateTime::DEFAULT)
            .system(System::Unix)
            .unix_permissions(0o644);
        zip.start_file(*name, options)?;
        zip.write_all(bytes)?;
    }
    Ok(zip.finish()?.into_inner())
}

/// The manifest of the fixture as the writer stores it, as JSON.
pub fn fixture_manifest() -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::to_value(written_manifest(&fixture()?)?)?)
}

/// The fixture's features as JSON.
pub fn fixture_features() -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::to_value(fixture()?.features)?)
}

/// The data a test layer gets: the fixture's `height`, otherwise twelve ones.
fn layer_data(name: &str) -> Result<LayerData, Box<dyn std::error::Error>> {
    let key = LayerKey {
        level: None,
        name: name.to_owned(),
    };
    Ok(fixture()?
        .layers
        .remove(&key)
        .unwrap_or_else(|| LayerData::U8(vec![1; 12])))
}

/// The in-memory package and the assembled ZIP bytes of a manifest and features, both made from
/// the same JSON. Dtype, file, shape and hash of each entry are recomputed from the layer's data
/// (and the file from its name and level), so a test edits only what it wants to break.
pub fn build(
    manifest: &Value,
    features: &Value,
) -> Result<(Package, Vec<u8>), Box<dyn std::error::Error>> {
    let mut manifest = manifest.clone();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut layers = std::collections::BTreeMap::new();
    for entry in manifest["layers"].as_array_mut().ok_or("layers")? {
        let name = entry["name"].as_str().ok_or("name")?.to_owned();
        let level = entry["level"].as_str().map(str::to_owned);
        let data = layer_data(&name)?;
        let bytes = data.to_bytes()?;
        let dtype = serde_json::to_value(data.dtype())?;
        let ext = dtype.as_str().ok_or("dtype")?.to_owned();
        let file = match &level {
            Some(level) => format!("layers/{level}/{name}.{ext}"),
            None => format!("layers/{name}.{ext}"),
        };
        entry["dtype"] = dtype;
        entry["file"] = json!(file);
        entry["shape"] = json!([3, 4]);
        entry["sha256"] = json!(sha256_hex(&bytes));
        files.push((file, bytes));
        layers.insert(LayerKey { level, name }, data);
    }
    let package = Package {
        manifest: serde_json::from_value(manifest.clone())?,
        features: serde_json::from_value(features.clone())?,
        layers,
    };
    let mut entries: Vec<(&str, Vec<u8>)> = vec![
        ("manifest.json", serde_json::to_vec(&manifest)?),
        ("features.geojson", serde_json::to_vec(features)?),
    ];
    entries.extend(
        files
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.clone())),
    );
    Ok((package, assemble(&entries)?))
}

type Entries = Vec<(String, Vec<u8>)>;

/// The packages `bytes` with `change` applied to its entries (name and content, in order).
pub fn edit(
    bytes: &[u8],
    change: impl FnOnce(&mut Entries) -> TestResult,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut entries: Entries = Vec::new();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut data)?;
        entries.push((file.name()?.into_owned(), data));
    }
    change(&mut entries)?;
    let refs: Vec<(&str, Vec<u8>)> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.clone()))
        .collect();
    assemble(&refs)
}

fn read_bytes(bytes: &[u8]) -> Result<Package, FormatError> {
    read(Cursor::new(bytes))
}

// AC: CE-76.1
#[test]
fn fixture_package_round_trips() -> TestResult {
    let package = fixture()?;
    let bytes = write(&package, Cursor::new(Vec::new()))?.into_inner();
    let recorded = std::fs::read_to_string(format!("{DIR}/package.sha256"))?;
    assert_eq!(sha256_hex(&bytes), recorded.trim());
    let back = read_bytes(&bytes)?;
    assert_eq!(back.manifest, written_manifest(&package)?);
    assert_eq!(back.features, package.features);
    assert_eq!(back.layers, package.layers);
    // Writing what was read gives the same bytes again.
    let again = write(&back, Cursor::new(Vec::new()))?.into_inner();
    assert_eq!(again, bytes);
    Ok(())
}

// AC: CE-76.2
#[test]
fn an_unknown_major_is_refused_naming_both_versions() -> TestResult {
    let mut manifest = fixture_manifest()?;
    manifest["format_version"] = json!("2.0");
    let (_, bytes) = build(&manifest, &fixture_features()?)?;
    let err = read_bytes(&bytes).err().ok_or("read must fail")?;
    assert!(matches!(&err, FormatError::UnknownMajor { found, .. } if found == "2.0"));
    let text = err.to_string();
    assert!(text.contains("2.0") && text.contains("1."), "{text}");
    Ok(())
}

#[test]
fn an_unparseable_version_is_refused() -> TestResult {
    let mut manifest = fixture_manifest()?;
    manifest["format_version"] = json!("one");
    let (_, bytes) = build(&manifest, &fixture_features()?)?;
    assert!(matches!(
        read_bytes(&bytes),
        Err(FormatError::UnknownMajor { .. })
    ));
    Ok(())
}

#[test]
fn another_format_is_not_a_package() -> TestResult {
    let mut manifest = fixture_manifest()?;
    manifest["format"] = json!("other-map");
    let (_, bytes) = build(&manifest, &fixture_features()?)?;
    assert!(matches!(
        read_bytes(&bytes),
        Err(FormatError::NotAPackage { .. })
    ));
    Ok(())
}

#[test]
fn a_zip_without_manifest_is_refused() -> TestResult {
    let bytes = assemble(&[("features.geojson", b"{}".to_vec())])?;
    assert!(matches!(
        read_bytes(&bytes),
        Err(FormatError::MissingFile { name }) if name == "manifest.json"
    ));
    assert!(read(Cursor::new(b"not a zip".to_vec())).is_err());
    Ok(())
}

// AC: CE-76.2
#[test]
fn unknown_fields_entries_layers_and_kinds_are_ignored() -> TestResult {
    let mut manifest = fixture_manifest()?;
    manifest["future"] = json!({"x": 1});
    let layers = manifest["layers"].as_array_mut().ok_or("layers")?;
    for (name, dtype) in [("strata", "u8"), ("wide", "f64")] {
        layers.push(json!({
            "name": name,
            "file": format!("layers/{name}.{dtype}"),
            "dtype": dtype,
            "shape": [3, 4],
            "sha256": sha256_hex(&[1; 12]),
            "description": "unknown to the format"
        }));
    }
    let mut features = fixture_features()?;
    features["features"]
        .as_array_mut()
        .ok_or("features")?
        .push(json!({
            "type": "Feature",
            "geometry": {"type": "Point", "coordinates": [1.0, 2.0]},
            "properties": {"kind": "monolith"}
        }));
    // Build the zip by hand: the f64 layer must not go through `build`.
    let mut known = manifest.clone();
    known["layers"]
        .as_array_mut()
        .ok_or("layers")?
        .retain(|l| l["dtype"] != "f64");
    let (_, base) = build(&known, &features)?;
    let bytes = edit(&base, |entries| {
        entries[0].1 = serde_json::to_vec(&manifest)?;
        entries.push(("layers/wide.f64".to_owned(), vec![0; 96]));
        entries.push(("extras/notes.txt".to_owned(), b"hello".to_vec()));
        Ok(())
    })?;
    let package = read_bytes(&bytes)?;
    let strata = LayerKey {
        level: None,
        name: "strata".to_owned(),
    };
    let wide = LayerKey {
        level: None,
        name: "wide".to_owned(),
    };
    assert_eq!(
        package.layers.get(&strata),
        Some(&LayerData::U8(vec![1; 12]))
    );
    assert!(!package.layers.contains_key(&wide));
    assert!(package.manifest.layers.iter().all(|l| l.name != "wide"));
    assert!(package.features.features.iter().any(|f| {
        f.properties
            .as_ref()
            .and_then(|p| p.get("kind"))
            .is_some_and(|k| k == "monolith")
    }));
    Ok(())
}

// Review Focus 8 (map-package §3: a reader ignores unknown fields at every level): unknown keys
// in the frame, a layer entry, a level and `parent` read, and the known values stay.
#[test]
fn unknown_fields_inside_the_manifest_are_ignored() -> TestResult {
    let mut manifest = fixture_manifest()?;
    manifest["frame"]["future"] = json!(1);
    manifest["frame"]["levels"] = json!([{"id": "f1", "elevation_m": 2.5, "future": [1]}]);
    manifest["frame"]["parent"] =
        json!({"package_sha256": "ab", "feature": "town-2", "future": {"x": 1}});
    for layer in manifest["layers"].as_array_mut().ok_or("layers")? {
        layer["level"] = json!("f1");
        layer["future"] = json!("x");
    }
    let mut features = fixture_features()?;
    for feature in features["features"].as_array_mut().ok_or("features")? {
        feature["properties"]["level"] = json!("f1");
    }
    let (_, bytes) = build(&manifest, &features)?;
    let package = read_bytes(&bytes)?;
    let frame = &package.manifest.frame;
    let level = frame
        .levels
        .as_ref()
        .and_then(|l| l.first())
        .ok_or("no level")?;
    assert_eq!(
        (level.id.as_str(), level.elevation_m.to_bits()),
        ("f1", 2.5_f64.to_bits())
    );
    let parent = frame.parent.as_ref().ok_or("no parent")?;
    assert_eq!(
        (parent.package_sha256.as_str(), parent.feature.as_str()),
        ("ab", "town-2")
    );
    let layer = &package.manifest.layers[0];
    assert_eq!(
        (layer.name.as_str(), layer.level.as_deref()),
        ("height", Some("f1"))
    );
    assert_eq!(layer.units.as_deref(), Some("m"));
    Ok(())
}

// AC: CE-76.5
#[test]
fn a_required_capability_the_reader_lacks_is_refused() -> TestResult {
    let mut manifest = fixture_manifest()?;
    manifest["requires"] = json!(["slabs"]);
    let (_, bytes) = build(&manifest, &fixture_features()?)?;
    let err = read_bytes(&bytes).err().ok_or("read must fail")?;
    assert!(
        matches!(&err, FormatError::UnsupportedCapability { capability } if capability == "slabs")
    );
    assert!(err.to_string().contains("slabs"));
    manifest["requires"] = json!([]);
    let (_, bytes) = build(&manifest, &fixture_features()?)?;
    assert!(read_bytes(&bytes).is_ok());
    assert_eq!(
        theos_map_format::read::IMPLEMENTED_CAPABILITIES,
        [] as [&str; 0]
    );
    Ok(())
}

/// The written fixture with the bytes of one entry replaced.
fn with_replaced(name: &str, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let written = write(&fixture()?, Cursor::new(Vec::new()))?.into_inner();
    edit(&written, |entries| {
        for entry in entries.iter_mut().filter(|(n, _)| n == name) {
            entry.1 = bytes.to_vec();
        }
        Ok(())
    })
}

#[test]
fn a_wrong_layer_hash_is_refused() -> TestResult {
    let bytes = with_replaced("layers/water.u8", &[9; 12])?;
    assert!(matches!(
        read_bytes(&bytes),
        Err(FormatError::LayerHash { name }) if name == "water"
    ));
    Ok(())
}

#[test]
fn a_short_layer_is_refused() -> TestResult {
    let bytes = with_replaced("layers/water.u8", &[0; 5])?;
    assert!(matches!(
        read_bytes(&bytes),
        Err(FormatError::LayerSize {
            expected: 12,
            found: 5
        })
    ));
    let long = with_replaced("layers/water.u8", &[0; 13])?;
    assert!(matches!(
        read_bytes(&long),
        Err(FormatError::LayerSize { expected: 12, .. })
    ));
    Ok(())
}

#[test]
fn a_missing_layer_file_is_refused() -> TestResult {
    let (_, bytes) = build(&fixture_manifest()?, &fixture_features()?)?;
    let bytes = edit(&bytes, |entries| {
        entries.retain(|(name, _)| name != "layers/water.u8");
        Ok(())
    })?;
    assert!(matches!(
        read_bytes(&bytes),
        Err(FormatError::MissingFile { name }) if name == "layers/water.u8"
    ));
    Ok(())
}

#[test]
fn an_index_file_that_is_not_the_expected_path_is_refused() -> TestResult {
    for file in [
        "manifest.json",
        "layers/height.f32",
        "layers/../manifest.json",
    ] {
        let mut manifest = fixture_manifest()?;
        manifest["layers"][1]["file"] = json!(file);
        let (_, bytes) = build(&fixture_manifest()?, &fixture_features()?)?;
        // `build` fixes the files, so the crafted manifest goes into the archive by hand.
        let bytes = edit(&bytes, |entries| {
            entries[0].1 = serde_json::to_vec(&manifest)?;
            Ok(())
        })?;
        assert!(
            matches!(
                read_bytes(&bytes),
                Err(FormatError::LayerFile { name, .. }) if name == "water"
            ),
            "{file}"
        );
    }
    Ok(())
}

#[test]
fn a_manifest_that_is_not_json_is_refused() -> TestResult {
    let bytes = assemble(&[("manifest.json", b"{".to_vec())])?;
    assert!(matches!(read_bytes(&bytes), Err(FormatError::Json(_))));
    Ok(())
}

#[test]
fn a_layer_whose_shape_is_not_the_grid_is_refused() -> TestResult {
    let (_, bytes) = build(&fixture_manifest()?, &fixture_features()?)?;
    let mut manifest = fixture_manifest()?;
    manifest["layers"][1]["shape"] = json!([4, 3]);
    let bytes = edit(&bytes, |entries| {
        entries[0].1 = serde_json::to_vec(&manifest)?;
        Ok(())
    })?;
    assert!(matches!(
        read_bytes(&bytes),
        Err(FormatError::LayerShape { name }) if name == "water"
    ));
    Ok(())
}

/// A manifest that deserialises to the same value as the fixture's written one.
#[test]
fn the_manifest_type_round_trips_through_json() -> TestResult {
    let manifest: Manifest = serde_json::from_value(fixture_manifest()?)?;
    assert_eq!(manifest, written_manifest(&fixture()?)?);
    Ok(())
}
