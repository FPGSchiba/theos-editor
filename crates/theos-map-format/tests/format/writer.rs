//! The package writer: container rules, determinism and the fields it fills in.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::io::Cursor;

use serde_json::Value;
use theos_map_format::{
    DType, FormatError, LayerData, LayerKey, Level, Manifest, Package, features_bytes, json,
    sha256_hex, write, write_cancellable, written_manifest,
};

use crate::zip_bytes;

type TestResult = Result<(), Box<dyn std::error::Error>>;

pub const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/package");

/// Reads a fixture JSON file without its `note` key.
pub fn read_json(file: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(format!("{DIR}/{file}"))?;
    let mut value: Value = serde_json::from_str(&text)?;
    value.as_object_mut().ok_or("not an object")?.remove("note");
    Ok(value)
}

fn key(name: &str) -> LayerKey {
    LayerKey {
        level: None,
        name: name.to_owned(),
    }
}

/// The synthetic fixture package: grid 3 x 4, layers `height` (f32) and `water` (u8).
pub fn fixture() -> Result<Package, Box<dyn std::error::Error>> {
    let manifest: Manifest = serde_json::from_value(read_json("manifest.json")?)?;
    let features: geojson::FeatureCollection =
        serde_json::from_value(read_json("features.geojson")?)?;
    let table: toml::Table =
        toml::from_str(&std::fs::read_to_string(format!("{DIR}/layers.toml"))?)?;
    let values = |name: &str| -> Result<Vec<toml::Value>, Box<dyn std::error::Error>> {
        Ok(table[name]["values"].as_array().ok_or("values")?.clone())
    };
    let height: Vec<f32> = values("height")?
        .iter()
        .map(|v| Ok(v.as_float().ok_or("float")?.to_string().parse::<f32>()?))
        .collect::<Result<_, Box<dyn std::error::Error>>>()?;
    let water: Vec<u8> = values("water")?
        .iter()
        .map(|v| Ok(u8::try_from(v.as_integer().ok_or("int")?)?))
        .collect::<Result<_, Box<dyn std::error::Error>>>()?;
    let layers = BTreeMap::from([
        (key("height"), LayerData::F32(height)),
        (key("water"), LayerData::U8(water)),
    ]);
    Ok(Package {
        manifest,
        features,
        layers,
    })
}

// AC: CE-76.1
#[test]
fn written_bytes_follow_the_container_rules() -> TestResult {
    let package = fixture()?;
    let bytes = write(&package, Cursor::new(Vec::new()))?.into_inner();
    let zip = zip_bytes::parse(&bytes)?;
    assert_eq!(
        zip.names(),
        [
            "manifest.json",
            "features.geojson",
            "layers/height.f32",
            "layers/water.u8"
        ]
    );
    for e in zip.entries() {
        assert_eq!((e.method, e.mod_time, e.mod_date), (8, 0, 0x0021)); // deflate, 1980-01-01 00:00
        assert_eq!(
            (e.local_extra_len, e.central_extra_len, e.comment_len),
            (0, 0, 0)
        );
        assert_eq!(e.version_made_by >> 8, 3); // Unix
        assert_eq!(e.external_attrs >> 16, 0o100_644);
        assert!(!e.name.ends_with('/'));
        assert_eq!((e.disk_start, e.internal_attrs), (0, 0));
        // The local header agrees with the central directory.
        let l = &e.local;
        assert_eq!(l.name, e.name);
        assert_eq!(
            (l.method, l.mod_time, l.mod_date),
            (e.method, e.mod_time, e.mod_date)
        );
        assert_eq!((l.flags, l.version_needed), (e.flags, e.version_needed));
        assert_eq!(l.crc32, e.crc32);
        assert_eq!(l.compressed_size, e.compressed_size);
        assert_eq!(l.uncompressed_size, e.uncompressed_size);
        // Flags are 0 (as the zip crate writes them for ASCII names at level 6): bit 0 (encryption)
        // and bit 3 (data descriptor, so sizes and CRC sit in the headers) are off, bits 1-2
        // (deflate "normal" level) are 0, and bit 11 (UTF-8 names) is off because every name is
        // ASCII. Version needed is 20 (2.0, deflate).
        assert_eq!((e.flags, e.version_needed), (0, 20));
        let data = zip.entry(&e.name)?;
        let mut crc = flate2::Crc::new();
        crc.update(&data);
        assert_eq!(e.crc32, crc.sum());
        assert_eq!(usize::try_from(e.uncompressed_size)?, data.len());
    }
    let eocd = &zip.eocd;
    assert_eq!((eocd.disk, eocd.central_dir_disk), (0, 0));
    assert_eq!(usize::from(eocd.entries_on_disk), zip.entries().len());
    assert_eq!(usize::from(eocd.entries_total), zip.entries().len());
    assert_eq!(eocd.comment_len, 0);
    let written = written_manifest(&package)?;
    assert_eq!(
        zip.entry("manifest.json")?,
        json::to_canonical_vec(&serde_json::to_value(&written)?)?
    );
    // Final review round 4, A2. The entries lie in the archive in the order of the central
    // directory (§1).
    let offsets: Vec<usize> = zip.entries().iter().map(|e| e.local_offset).collect();
    assert!(offsets.windows(2).all(|w| w[0] < w[1]), "{offsets:?}");
    // Both JSON entries are canonical (§3), checked without the crate's own writer: serde_json's
    // pretty printer over a map with sorted keys (no `preserve_order`), then one LF.
    for name in ["manifest.json", "features.geojson"] {
        let text = zip.entry(name)?;
        let value: Value = serde_json::from_slice(&text)?;
        let pretty = format!("{}\n", serde_json::to_string_pretty(&value)?);
        assert_eq!(String::from_utf8(text)?, pretty, "{name}");
    }
    let features: geojson::FeatureCollection =
        serde_json::from_slice(&zip.entry("features.geojson")?)?;
    assert_eq!(features, package.features);
    // The layers follow the index, not their names: reversed, `water` comes first.
    let mut reversed = package.clone();
    reversed.manifest.layers.reverse();
    let bytes = write(&reversed, Cursor::new(Vec::new()))?.into_inner();
    assert_eq!(
        zip_bytes::parse(&bytes)?.names(),
        [
            "manifest.json",
            "features.geojson",
            "layers/water.u8",
            "layers/height.f32"
        ]
    );
    Ok(())
}

// AC: CE-76.1
#[test]
fn entries_are_filled_in_by_the_writer() -> TestResult {
    let package = fixture()?;
    let bytes = write(&package, Cursor::new(Vec::new()))?.into_inner();
    let zip = zip_bytes::parse(&bytes)?;
    let manifest: Manifest = serde_json::from_slice(&zip.entry("manifest.json")?)?;
    assert_eq!(manifest.format, "theos-map");
    assert_eq!(manifest.format_version, "1.0");
    assert_eq!(manifest.features, "features.geojson");
    let height = &manifest.layers[0];
    assert_eq!(height.file, "layers/height.f32");
    assert_eq!((height.dtype, height.shape), (DType::F32, [3, 4]));
    assert_eq!(height.sha256, sha256_hex(&zip.entry("layers/height.f32")?));
    assert_eq!((height.min, height.max), (Some(10.5), Some(14.0)));
    assert_eq!(height.units.as_deref(), Some("m"));
    // §4.2: f32 values little-endian, row-major (final review round 4, A2).
    let Some(LayerData::F32(heights)) = package.layers.get(&key("height")) else {
        return Err("the fixture has heights".into());
    };
    let encoded: Vec<u8> = heights.iter().flat_map(|v| v.to_le_bytes()).collect();
    assert_eq!(zip.entry("layers/height.f32")?, encoded);
    let water = &manifest.layers[1];
    assert_eq!(water.file, "layers/water.u8");
    assert_eq!(water.sha256, sha256_hex(&zip.entry("layers/water.u8")?));
    assert_eq!((water.min, water.max), (Some(0.0), Some(3.0)));
    assert!(water.legend.is_some());
    assert_eq!(
        zip.entry("layers/water.u8")?,
        [0, 0, 0, 0, 0, 3, 3, 0, 0, 3, 0, 0]
    );
    let features: Value = serde_json::from_slice(&zip.entry("features.geojson")?)?;
    assert_eq!(features["features"].as_array().map(Vec::len), Some(3));
    Ok(())
}

#[test]
fn levels_get_their_own_folder() -> TestResult {
    let mut package = fixture()?;
    package.manifest.frame.levels = Some(vec![Level {
        id: "f1".into(),
        elevation_m: 0.0,
        name: None,
    }]);
    for entry in &mut package.manifest.layers {
        entry.level = Some("f1".into());
    }
    package.layers = package
        .layers
        .into_iter()
        .map(|(k, v)| {
            let level = Some("f1".to_owned());
            (
                LayerKey {
                    level,
                    name: k.name,
                },
                v,
            )
        })
        .collect();
    for feature in &mut package.features.features {
        feature.set_property("level", "f1");
    }
    let bytes = write(&package, Cursor::new(Vec::new()))?.into_inner();
    let zip = zip_bytes::parse(&bytes)?;
    assert_eq!(
        zip.names()[2..],
        ["layers/f1/height.f32", "layers/f1/water.u8"]
    );
    Ok(())
}

#[test]
fn two_writes_are_byte_identical() -> TestResult {
    let package = fixture()?;
    let first = write(&package, Cursor::new(Vec::new()))?.into_inner();
    let second = write(&package, Cursor::new(Vec::new()))?.into_inner();
    assert_eq!(first, second);
    Ok(())
}

// AC: CE-76.5
#[test]
fn requires_is_written_sorted_and_deduplicated() -> TestResult {
    let mut package = fixture()?;
    package.manifest.requires = vec!["b".into(), "a".into(), "a".into()];
    let bytes = write(&package, Cursor::new(Vec::new()))?.into_inner();
    let zip = zip_bytes::parse(&bytes)?;
    let manifest: Manifest = serde_json::from_slice(&zip.entry("manifest.json")?)?;
    assert_eq!(manifest.requires, ["a", "b"]);
    Ok(())
}

#[test]
fn write_cancellable_stops_between_entries() -> TestResult {
    let package = fixture()?;
    let calls = Cell::new(0);
    let result = write_cancellable(&package, Cursor::new(Vec::new()), &|| {
        calls.set(calls.get() + 1);
        calls.get() > 1
    });
    assert!(matches!(result, Err(FormatError::Cancelled)));
    assert_eq!(calls.get(), 2);
    Ok(())
}

// Spec §5.13: the features are written as canonical JSON (-0.0 as +0.0), and `features_bytes`
// gives exactly the bytes of the package's `features.geojson`.
#[test]
fn features_are_written_canonically_and_features_bytes_match() -> TestResult {
    let mut package = fixture()?;
    package.features.features[0].geometry = Some(geojson::Geometry::new_point([-0.0, 5.0]));
    let bytes = write(&package, Cursor::new(Vec::new()))?.into_inner();
    let written = zip_bytes::parse(&bytes)?.entry("features.geojson")?;
    assert_eq!(written, features_bytes(&package.features)?);
    let text = String::from_utf8(written)?;
    assert!(!text.contains("-0.0"), "{text}");
    assert!(
        text.contains(
            "0.0,
"
        ),
        "{text}"
    );
    Ok(())
}

// Spec §5.13: the writer validates "no NaN or infinity" in the features, `scale` and `offset`.
#[test]
fn non_finite_feature_scale_and_offset_numbers_are_not_written() -> TestResult {
    let mut package = fixture()?;
    package.features.features[1].geometry = Some(geojson::Geometry::new_point([f64::NAN, 5.0]));
    let err = write(&package, Cursor::new(Vec::new())).err();
    assert!(
        matches!(&err, Some(FormatError::FeatureNotFinite { owner }) if owner == "feature 1"),
        "{err:?}"
    );
    assert!(matches!(
        features_bytes(&package.features),
        Err(FormatError::JsonNotFinite)
    ));
    for (field, scale, offset) in [
        ("layers[1].scale", Some(f64::NAN), None),
        ("layers[1].offset", None, Some(f64::NEG_INFINITY)),
    ] {
        let mut package = fixture()?;
        package.manifest.layers[1].scale = scale;
        package.manifest.layers[1].offset = offset;
        let err = write(&package, Cursor::new(Vec::new())).err();
        assert!(
            matches!(&err, Some(FormatError::ManifestNotFinite { field: f }) if f == field),
            "{field}: {err:?}"
        );
    }
    Ok(())
}

// Spec §5.13: a finite `scale` can still decode a value beyond f64; the range the writer fills
// in would then be an infinity, which JSON writes as `null`.
#[test]
fn a_decoded_range_that_is_not_finite_is_not_written() -> TestResult {
    let mut package = fixture()?;
    package.manifest.layers[1].scale = Some(f64::MAX);
    let err = write(&package, Cursor::new(Vec::new())).err();
    assert!(
        matches!(&err, Some(FormatError::ManifestNotFinite { field }) if field == "layers[1].max"),
        "{err:?}"
    );
    package.manifest.layers[1].offset = Some(-f64::MAX);
    package.manifest.layers[1].scale = Some(-f64::MAX);
    let err = written_manifest(&package).err();
    assert!(
        matches!(&err, Some(FormatError::ManifestNotFinite { field }) if field == "layers[1].min"),
        "{err:?}"
    );
    Ok(())
}

#[test]
fn an_invalid_package_is_not_written() -> TestResult {
    let mut package = fixture()?;
    package.layers.clear();
    assert!(write(&package, Cursor::new(Vec::new())).is_err());
    Ok(())
}
