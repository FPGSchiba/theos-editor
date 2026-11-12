//! A map package in memory, and the checks that decide whether it can be written
//! (`docs/map-package.md` §3–§4).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{FormatError, Frame, LayerData, LayerKey, Manifest};

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
fn label(key: &LayerKey) -> String {
    match &key.level {
        Some(level) => format!("{level}/{}", key.name),
        None => key.name.clone(),
    }
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
fn check_frame_numbers(frame: &Frame) -> Result<(), FormatError> {
    let mut numbers: Vec<(String, f64)> = vec![
        ("frame.size_m[0]".to_owned(), frame.size_m[0]),
        ("frame.size_m[1]".to_owned(), frame.size_m[1]),
        ("frame.cell_m".to_owned(), frame.cell_m),
    ];
    if let Some(origin) = frame.origin_m {
        numbers.push(("frame.origin_m[0]".to_owned(), origin[0]));
        numbers.push(("frame.origin_m[1]".to_owned(), origin[1]));
    }
    for (index, level) in frame.levels.iter().flatten().enumerate() {
        numbers.push((
            format!("frame.levels[{index}].elevation_m"),
            level.elevation_m,
        ));
    }
    match numbers.into_iter().find(|(_, value)| !value.is_finite()) {
        Some((field, _)) => Err(FormatError::ManifestNotFinite { field }),
        None => Ok(()),
    }
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
    /// A [`FormatError`] that names the first problem: a non-finite frame number, a duplicate
    /// layer, an entry without data or data without an entry, a layer that is not rows ×
    /// columns long or holds NaN or infinities, or a level that is missing, unlisted or not
    /// allowed on a layer or feature.
    pub fn validate(&self) -> Result<(), FormatError> {
        let frame = &self.manifest.frame;
        check_frame_numbers(frame)?;
        let ids: Option<BTreeSet<&str>> = frame
            .levels
            .as_ref()
            .map(|levels| levels.iter().map(|level| level.id.as_str()).collect());
        let cells = u64::from(frame.grid[0]) * u64::from(frame.grid[1]);
        let cells = usize::try_from(cells).unwrap_or(usize::MAX);

        let mut seen = BTreeSet::new();
        for entry in &self.manifest.layers {
            let key = LayerKey {
                level: entry.level.clone(),
                name: entry.name.clone(),
            };
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

    type FrameBreaker = fn(&mut Frame);

    #[test]
    fn non_finite_frame_numbers_are_refused_with_their_field() {
        let cases: [(&str, FrameBreaker); 5] = [
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
        ];
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
