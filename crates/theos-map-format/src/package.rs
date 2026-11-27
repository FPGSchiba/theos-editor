//! A map package in memory, and the checks that decide whether it can be written
//! (`docs/map-package.md` §3–§4).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{FormatError, Frame, LayerData, LayerEntry, LayerKey, Manifest, finite};

/// A map package: manifest, features and layer data.
///
/// `manifest.layers` lists the layers in index order; `layers` holds their values under the
/// same `level` and `name`. The writer fills in the entries' file, dtype, shape, hash and range.
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    /// The manifest; the writer completes it.
    pub manifest: Manifest,
    /// The features, as a `GeoJSON` feature collection.
    pub features: geojson::FeatureCollection,
    /// The layer values.
    pub layers: BTreeMap<LayerKey, LayerData>,
}

/// `name`, or `level/name` on a map with levels.
pub(crate) fn label(key: &LayerKey) -> String {
    match &key.level {
        Some(level) => format!("{level}/{}", key.name),
        None => key.name.clone(),
    }
}

/// Refuses a layer name or level id that could leave `layers/` once it is part of an entry path:
/// empty, `.`, `..`, or containing `/`, `\` or NUL.
pub(crate) fn check_name(kind: &'static str, name: &str) -> Result<(), FormatError> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', '\0']) {
        return Err(FormatError::InvalidName {
            kind,
            name: name.to_owned(),
        });
    }
    Ok(())
}

/// Checks that `level` is listed in `ids`, or absent when the frame has no levels.
fn check_level(
    owner: &str,
    level: Option<&str>,
    ids: Option<&BTreeSet<&str>>,
) -> Result<(), FormatError> {
    match (ids, level) {
        (Some(_), None) => Err(FormatError::LevelMissing {
            owner: owner.to_owned(),
        }),
        (Some(ids), Some(level)) if !ids.contains(level) => Err(FormatError::LevelUnknown {
            owner: owner.to_owned(),
            level: level.to_owned(),
        }),
        (None, Some(level)) => Err(FormatError::LevelUnexpected {
            owner: owner.to_owned(),
            level: level.to_owned(),
        }),
        _ => Ok(()),
    }
}

/// Refuses a NaN or infinity in the frame: JSON cannot hold one, and it would become `null`.
///
/// The check reads the frame as it is serialised, so a new float field of `Frame` or `Level` is
/// covered without a list to update: every field of the frame is skipped when absent, so a
/// `null` in the serialised frame is a non-finite number (a test pins that).
fn check_frame_numbers(frame: &Frame) -> Result<(), FormatError> {
    match first_null(&serde_json::to_value(frame)?, "frame".to_owned()) {
        Some(field) => Err(FormatError::ManifestNotFinite { field }),
        None => Ok(()),
    }
}

/// The path of the first `null` in `value`, as `frame.levels[0].elevation_m`.
fn first_null(value: &Value, path: String) -> Option<String> {
    match value {
        Value::Null => Some(path),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .find_map(|(i, item)| first_null(item, format!("{path}[{i}]"))),
        Value::Object(fields) => fields
            .iter()
            .find_map(|(name, item)| first_null(item, format!("{path}.{name}"))),
        _ => None,
    }
}

/// Refuses a NaN or infinity in a layer entry's `scale` or `offset`: it would be written as
/// `null` and read back as "no scale".
fn check_entry_numbers(entries: &[LayerEntry]) -> Result<(), FormatError> {
    for (index, entry) in entries.iter().enumerate() {
        LayerEntry::check_finite(index, [("scale", entry.scale), ("offset", entry.offset)])?;
    }
    Ok(())
}

/// Refuses a NaN or infinity anywhere in the features, naming the first feature that holds one,
/// or the collection when only its own members do.
fn check_feature_numbers(features: &geojson::FeatureCollection) -> Result<(), FormatError> {
    if let Some(index) = features.features.iter().position(|f| !finite::is_finite(f)) {
        return Err(FormatError::FeatureNotFinite {
            owner: format!("feature {index}"),
        });
    }
    if !finite::is_finite(features) {
        return Err(FormatError::FeatureNotFinite {
            owner: "the feature collection".to_owned(),
        });
    }
    Ok(())
}

/// The level a feature carries in `properties.level`; a value that is not a string is kept as
/// its JSON text, so that it is reported as an unlisted level.
fn feature_level(feature: &geojson::Feature) -> Option<String> {
    match feature.properties.as_ref()?.get("level")? {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    }
}

impl Package {
    /// Checks that the package can be written.
    ///
    /// # Errors
    ///
    /// A [`FormatError`] that names the first problem: a non-finite frame number, layer `scale` or
    /// `offset` or feature number, a layer name or level id that cannot be part of an entry
    /// path, a duplicate
    /// layer, an entry without data or data without an entry, a layer that is not rows ×
    /// columns long or holds NaN or infinities, or a level that is missing, unlisted or not
    /// allowed on a layer or feature.
    pub fn validate(&self) -> Result<(), FormatError> {
        let frame = &self.manifest.frame;
        check_frame_numbers(frame)?;
        check_entry_numbers(&self.manifest.layers)?;
        check_feature_numbers(&self.features)?;
        for level in frame.levels.iter().flatten() {
            check_name("level", &level.id)?;
        }
        for entry in &self.manifest.layers {
            entry.check_names()?;
        }
        let ids: Option<BTreeSet<&str>> = frame
            .levels
            .as_ref()
            .map(|levels| levels.iter().map(|level| level.id.as_str()).collect());
        let cells = u64::from(frame.grid[0]) * u64::from(frame.grid[1]);
        let cells = usize::try_from(cells).unwrap_or(usize::MAX);

        let mut seen = BTreeSet::new();
        for entry in &self.manifest.layers {
            let key = entry.key();
            let layer = label(&key);
            check_level(
                &format!("layer {layer}"),
                entry.level.as_deref(),
                ids.as_ref(),
            )?;
            let data = self.layers.get(&key);
            if !seen.insert(key) {
                return Err(FormatError::DuplicateLayer { layer });
            }
            let data = data.ok_or_else(|| FormatError::MissingData {
                layer: layer.clone(),
            })?;
            if data.len() != cells {
                return Err(FormatError::LayerLength {
                    layer,
                    expected: cells,
                    found: data.len(),
                });
            }
            if let LayerData::F32(values) = data
                && let Some(index) = values.iter().position(|v| !v.is_finite())
            {
                return Err(FormatError::LayerNotFinite { layer, index });
            }
        }
        if let Some(key) = self.layers.keys().find(|key| !seen.contains(*key)) {
            return Err(FormatError::MissingEntry { layer: label(key) });
        }
        for (index, feature) in self.features.features.iter().enumerate() {
            let level = feature_level(feature);
            check_level(&format!("feature {index}"), level.as_deref(), ids.as_ref())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{DType, LayerEntry, Level, manifest::FORMAT, manifest::FORMAT_VERSION};

    fn entry(name: &str, level: Option<&str>) -> LayerEntry {
        LayerEntry {
            name: name.to_owned(),
            file: String::new(),
            dtype: DType::U8,
            shape: [0, 0],
            sha256: String::new(),
            description: "unit test".to_owned(),
            units: None,
            scale: None,
            offset: None,
            legend: None,
            min: None,
            max: None,
            level: level.map(str::to_owned),
        }
    }

    /// A 2 x 3 map with one u8 layer `a`.
    fn package() -> Package {
        Package {
            manifest: Manifest {
                format: FORMAT.to_owned(),
                format_version: FORMAT_VERSION.to_owned(),
                generator: "test@0".to_owned(),
                requires: Vec::new(),
                kind: "terrain".to_owned(),
                recipe_id: "test".to_owned(),
                recipe: json!({}),
                seed: 0,
                inputs: serde_json::Map::new(),
                frame: Frame {
                    size_m: [30.0, 20.0],
                    cell_m: 10.0,
                    grid: [2, 3],
                    origin_m: None,
                    parent: None,
                    levels: None,
                },
                layers: vec![entry("a", None)],
                features: String::new(),
                results: serde_json::Map::new(),
                summary: json!({}),
            },
            features: geojson::FeatureCollection {
                bbox: None,
                features: Vec::new(),
                foreign_members: None,
            },
            layers: BTreeMap::from([(
                LayerKey {
                    level: None,
                    name: "a".to_owned(),
                },
                LayerData::U8(vec![0; 6]),
            )]),
        }
    }

    fn with_level(mut package: Package) -> Package {
        package.manifest.frame.levels = Some(vec![Level {
            id: "f1".to_owned(),
            elevation_m: 0.0,
            name: None,
        }]);
        package
    }

    fn feature(level: Option<Value>) -> geojson::Feature {
        let mut feature = geojson::Feature::default();
        if let Some(level) = level {
            feature.set_property("level", level);
        }
        feature
    }

    #[test]
    fn a_consistent_package_is_valid() {
        assert!(package().validate().is_ok());
    }

    #[test]
    fn an_entry_without_data_is_refused() {
        let mut p = package();
        p.layers.clear();
        let err = p.validate();
        assert!(matches!(&err, Err(FormatError::MissingData { layer }) if layer == "a"));
        assert!(err.is_err_and(|e| e.to_string().contains('a')));
    }

    #[test]
    fn data_without_an_entry_is_refused() {
        let mut p = package();
        p.manifest.layers.clear();
        assert!(matches!(
            p.validate(),
            Err(FormatError::MissingEntry { layer }) if layer == "a"
        ));
    }

    #[test]
    fn a_length_other_than_rows_times_columns_is_refused() {
        let mut p = package();
        p.layers.insert(
            LayerKey {
                level: None,
                name: "a".to_owned(),
            },
            LayerData::U8(vec![0; 5]),
        );
        assert!(matches!(
            p.validate(),
            Err(FormatError::LayerLength {
                expected: 6,
                found: 5,
                ..
            })
        ));
    }

    #[test]
    fn a_duplicate_name_within_a_level_is_refused() {
        let mut p = package();
        p.manifest.layers.push(entry("a", None));
        assert!(matches!(
            p.validate(),
            Err(FormatError::DuplicateLayer { layer }) if layer == "a"
        ));
    }

    #[test]
    fn the_same_name_on_two_levels_is_allowed() {
        let mut p = with_level(package());
        p.manifest.frame.levels = Some(
            ["f1", "f2"]
                .map(|id| Level {
                    id: id.to_owned(),
                    elevation_m: 0.0,
                    name: None,
                })
                .to_vec(),
        );
        p.manifest.layers = vec![entry("a", Some("f1")), entry("a", Some("f2"))];
        p.layers = ["f1", "f2"]
            .map(|id| {
                let key = LayerKey {
                    level: Some(id.to_owned()),
                    name: "a".to_owned(),
                };
                (key, LayerData::U8(vec![0; 6]))
            })
            .into_iter()
            .collect();
        assert!(p.validate().is_ok());
    }

    #[test]
    fn non_finite_layer_data_is_refused() {
        let mut p = package();
        p.manifest.layers[0].dtype = DType::F32;
        p.layers.insert(
            LayerKey {
                level: None,
                name: "a".to_owned(),
            },
            LayerData::F32(vec![0.0, 0.0, f32::NAN, 0.0, 0.0, 0.0]),
        );
        assert!(matches!(
            p.validate(),
            Err(FormatError::LayerNotFinite { index: 2, .. })
        ));
    }

    /// Every path of a floating-point number in `value`, as `frame.levels[0].elevation_m`.
    fn float_paths(value: &Value, path: String, out: &mut Vec<String>) {
        match value {
            Value::Number(n) if n.is_f64() => out.push(path),
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    float_paths(item, format!("{path}[{i}]"), out);
                }
            }
            Value::Object(fields) => {
                for (name, item) in fields {
                    float_paths(item, format!("{path}.{name}"), out);
                }
            }
            _ => {}
        }
    }

    // Targeted review: the check once listed the float fields by hand. The cases below are
    // pinned to the types: a frame with every optional field set serialises exactly the floats
    // the cases break, so a new float field of `Frame` or `Level` fails here until a case
    // exists, and the cases prove the check refuses each by name.
    #[test]
    fn the_frame_cases_cover_every_float_of_a_full_frame() {
        let mut full = package().manifest.frame;
        full.origin_m = Some([1.0, 2.0]);
        full.levels = Some(vec![Level {
            id: "f0".to_owned(),
            elevation_m: 3.0,
            name: Some("ground".to_owned()),
        }]);
        full.parent = Some(crate::Parent {
            package_sha256: "0".repeat(64),
            feature: "town-2".to_owned(),
        });
        let value = serde_json::to_value(&full).expect("a frame serialises");
        let mut paths = Vec::new();
        float_paths(&value, "frame".to_owned(), &mut paths);
        let mut cased: Vec<String> = frame_cases().iter().map(|(f, _)| (*f).to_owned()).collect();
        paths.sort();
        cased.sort();
        assert_eq!(paths, cased);
    }

    type FrameBreaker = fn(&mut Frame);

    /// One way to break each float of a frame, by the field the check must name.
    fn frame_cases() -> [(&'static str, FrameBreaker); 6] {
        [
            ("frame.size_m[0]", |f| f.size_m[0] = f64::NAN),
            ("frame.size_m[1]", |f| f.size_m[1] = f64::NAN),
            ("frame.cell_m", |f| f.cell_m = f64::INFINITY),
            ("frame.origin_m[0]", |f| f.origin_m = Some([f64::NAN, 0.0])),
            ("frame.origin_m[1]", |f| f.origin_m = Some([0.0, f64::NAN])),
            ("frame.levels[0].elevation_m", |f| {
                f.levels = Some(vec![Level {
                    id: "f1".to_owned(),
                    elevation_m: f64::NEG_INFINITY,
                    name: None,
                }]);
            }),
        ]
    }

    #[test]
    fn non_finite_frame_numbers_are_refused_with_their_field() {
        let cases = frame_cases();
        for (field, break_it) in cases {
            let mut p = package();
            break_it(&mut p.manifest.frame);
            let err = p.validate();
            assert!(
                matches!(&err, Err(FormatError::ManifestNotFinite { field: f }) if f == field),
                "{field}: {err:?}"
            );
        }
    }

    type EntryBreaker = fn(&mut LayerEntry);

    // Spec §5.13: the writer validates "no NaN or infinity"; a NaN `scale` would read back as
    // "no scale".
    #[test]
    fn non_finite_scale_and_offset_are_refused_with_their_field() {
        let cases: [(&str, EntryBreaker); 3] = [
            ("layers[0].scale", |e| e.scale = Some(f64::NAN)),
            ("layers[0].offset", |e| e.offset = Some(f64::INFINITY)),
            ("layers[0].scale", |e| e.scale = Some(f64::NEG_INFINITY)),
        ];
        for (field, break_it) in cases {
            let mut p = package();
            break_it(&mut p.manifest.layers[0]);
            let err = p.validate();
            assert!(
                matches!(&err, Err(FormatError::ManifestNotFinite { field: f }) if f == field),
                "{field}: {err:?}"
            );
        }
    }

    #[test]
    fn a_non_finite_feature_number_is_refused_naming_the_feature() {
        let mut p = package();
        let point = |x: f64| geojson::Feature {
            geometry: Some(geojson::Geometry::new_point([x, 1.0])),
            ..geojson::Feature::default()
        };
        p.features.features = vec![point(1.0), point(f64::NAN)];
        let err = p.validate();
        assert!(
            matches!(&err, Err(FormatError::FeatureNotFinite { owner }) if owner == "feature 1"),
            "{err:?}"
        );
        p.features.features = vec![point(1.0)];
        p.features.bbox = Some(vec![0.0, 0.0, f64::INFINITY, 1.0]);
        let err = p.validate();
        assert!(
            matches!(&err, Err(FormatError::FeatureNotFinite { owner }) if owner == "the feature collection"),
            "{err:?}"
        );
    }

    #[test]
    fn with_levels_a_layer_needs_a_listed_level() {
        let mut p = with_level(package());
        assert!(matches!(
            p.validate(),
            Err(FormatError::LevelMissing { owner }) if owner == "layer a"
        ));
        p.manifest.layers = vec![entry("a", Some("f9"))];
        assert!(matches!(
            p.validate(),
            Err(FormatError::LevelUnknown { level, .. }) if level == "f9"
        ));
    }

    #[test]
    fn with_levels_a_feature_needs_a_listed_level() {
        let mut p = with_level(package());
        p.manifest.layers = vec![entry("a", Some("f1"))];
        p.layers = BTreeMap::from([(
            LayerKey {
                level: Some("f1".to_owned()),
                name: "a".to_owned(),
            },
            LayerData::U8(vec![0; 6]),
        )]);
        p.features.features = vec![feature(Some(json!("f1"))), feature(None)];
        assert!(matches!(
            p.validate(),
            Err(FormatError::LevelMissing { owner }) if owner == "feature 1"
        ));
        p.features.features = vec![feature(Some(json!(7)))];
        assert!(matches!(
            p.validate(),
            Err(FormatError::LevelUnknown { owner, level }) if owner == "feature 0" && level == "7"
        ));
        p.features.features = vec![feature(Some(json!("f1")))];
        assert!(p.validate().is_ok());
    }

    #[test]
    fn without_levels_a_layer_or_feature_must_not_carry_one() {
        let mut p = package();
        p.features.features = vec![feature(Some(json!("f1")))];
        assert!(matches!(
            p.validate(),
            Err(FormatError::LevelUnexpected { owner, .. }) if owner == "feature 0"
        ));
        p.features.features = vec![feature(Some(Value::Null))];
        assert!(p.validate().is_ok());
        p.manifest.layers = vec![entry("a", Some("f1"))];
        assert!(matches!(
            p.validate(),
            Err(FormatError::LevelUnexpected { owner, .. }) if owner == "layer f1/a"
        ));
    }
}
