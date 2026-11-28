//! The manifest of a map package (`docs/map-package.md` §3).
//!
//! Unknown fields are ignored on reading, so a later minor format version stays readable.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The value of the manifest's `format` field.
pub const FORMAT: &str = "theos-map";
/// The format version this crate writes.
pub const FORMAT_VERSION: &str = "1.1";

/// The sample type of a layer file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DType {
    /// 32-bit float.
    F32,
    /// Unsigned 8-bit integer.
    U8,
    /// Unsigned 16-bit integer.
    U16,
}

impl DType {
    /// Every dtype, in declaration order.
    pub const ALL: [DType; 3] = [DType::F32, DType::U8, DType::U16];

    /// The bytes of one value.
    #[must_use]
    pub const fn width(self) -> usize {
        match self {
            Self::F32 => 4,
            Self::U8 => 1,
            Self::U16 => 2,
        }
    }
}

/// The parent a child map was made from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parent {
    /// SHA-256 of the parent package.
    pub package_sha256: String,
    /// The parent feature the child details, such as `town-2`.
    pub feature: String,
}

/// One stacked level of an indoor map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Level {
    /// The level's id.
    pub id: String,
    /// The level's elevation in metres.
    pub elevation_m: f64,
    /// A display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// The map's frame: size, resolution and, for child maps, placement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// Width and height in metres.
    pub size_m: [f64; 2],
    /// Cell edge length in metres.
    pub cell_m: f64,
    /// Rows and columns, `[rows, columns]`.
    pub grid: [u32; 2],
    /// Origin of a child map in its parent's frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_m: Option<[f64; 2]>,
    /// What a child map was made from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Parent>,
    /// Stacked levels of an indoor map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub levels: Option<Vec<Level>>,
}

/// One entry of the layer index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerEntry {
    /// Layer name.
    pub name: String,
    /// Path of the layer file inside the package.
    pub file: String,
    /// Sample type.
    pub dtype: DType,
    /// Rows and columns of the layer.
    pub shape: [u32; 2],
    /// SHA-256 of the layer file.
    pub sha256: String,
    /// What the layer holds.
    pub description: String,
    /// Units of the values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units: Option<String>,
    /// Multiplier for integer layers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// Offset for integer layers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<f64>,
    /// Meaning of the values of a categorical layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend: Option<BTreeMap<String, String>>,
    /// Smallest value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Largest value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// The level an indoor layer belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
}

/// The manifest of a map package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// Always [`FORMAT`].
    pub format: String,
    /// `MAJOR.MINOR`, [`FORMAT_VERSION`] when written by this crate.
    pub format_version: String,
    /// `name@version` of the generator.
    pub generator: String,
    /// Capabilities a reader must implement, sorted.
    pub requires: Vec<String>,
    /// The map kind: `terrain`, `settlement`, `interior`, …
    pub kind: String,
    /// The recipe's id.
    pub recipe_id: String,
    /// The full recipe as it ran.
    pub recipe: Value,
    /// The run's seed.
    pub seed: u64,
    /// The run's inputs.
    pub inputs: Map<String, Value>,
    /// The map's frame.
    pub frame: Frame,
    /// The layer index.
    pub layers: Vec<LayerEntry>,
    /// Path of the feature file.
    pub features: String,
    /// The run's results.
    pub results: Map<String, Value>,
    /// Everything else the generator reports; free form.
    pub summary: Value,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn terrain() -> Manifest {
        Manifest {
            format: FORMAT.to_owned(),
            format_version: FORMAT_VERSION.to_owned(),
            generator: "theos-maps@1.0.0".to_owned(),
            requires: Vec::new(),
            kind: "terrain".to_owned(),
            recipe_id: "hearth-valley".to_owned(),
            recipe: json!({}),
            seed: 7,
            inputs: Map::new(),
            frame: Frame {
                size_m: [24000.0, 24000.0],
                cell_m: 62.5,
                grid: [384, 384],
                origin_m: None,
                parent: None,
                levels: None,
            },
            layers: vec![LayerEntry {
                name: "height".to_owned(),
                file: "layers/height.f32".to_owned(),
                dtype: DType::F32,
                shape: [384, 384],
                sha256: "00".to_owned(),
                description: "terrain height".to_owned(),
                units: Some("m".to_owned()),
                scale: None,
                offset: None,
                legend: None,
                min: Some(0.0),
                max: Some(1874.2),
                level: None,
            }],
            features: "features.geojson".to_owned(),
            results: Map::new(),
            summary: json!({}),
        }
    }

    #[test]
    fn a_manifest_without_options_writes_sorted_keys_and_requires()
    -> Result<(), Box<dyn std::error::Error>> {
        let value = crate::json::canonicalise(&serde_json::to_value(terrain())?);
        let keys: Vec<&str> = value
            .as_object()
            .ok_or("not an object")?
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            [
                "features",
                "format",
                "format_version",
                "frame",
                "generator",
                "inputs",
                "kind",
                "layers",
                "recipe",
                "recipe_id",
                "requires",
                "results",
                "seed",
                "summary"
            ]
        );
        assert_eq!(value["requires"], json!([]));
        let frame = value["frame"].as_object().ok_or("frame")?;
        assert!(!frame.contains_key("origin_m"));
        assert!(!frame.contains_key("parent"));
        assert!(!frame.contains_key("levels"));
        let frame_keys: Vec<&str> = frame.keys().map(String::as_str).collect();
        assert_eq!(frame_keys, ["cell_m", "grid", "size_m"]);
        let layer = value["layers"][0].as_object().ok_or("layer")?;
        assert!(!layer.contains_key("scale"));
        assert!(!layer.contains_key("legend"));
        assert_eq!(layer["dtype"], json!("f32"));
        Ok(())
    }

    #[test]
    fn unknown_manifest_fields_are_ignored_on_reading() -> Result<(), Box<dyn std::error::Error>> {
        let mut value = serde_json::to_value(terrain())?;
        value["future"] = json!(1);
        value["frame"]["future"] = json!(true);
        let back: Manifest = serde_json::from_value(value)?;
        assert_eq!(back, terrain());
        Ok(())
    }

    #[test]
    fn every_dtype_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        for (dtype, text) in [(DType::F32, "f32"), (DType::U8, "u8"), (DType::U16, "u16")] {
            assert_eq!(serde_json::to_value(dtype)?, json!(text));
            assert_eq!(serde_json::from_value::<DType>(json!(text))?, dtype);
        }
        Ok(())
    }

    // Final review round 5, finding 2: `ALL` holds every dtype once, in declaration order. The
    // match has no wildcard, so a new dtype does not compile until it has its place here, and
    // then `ALL` must hold it there.
    #[test]
    fn all_holds_every_dtype() {
        let place = |dtype: DType| match dtype {
            DType::F32 => 0,
            DType::U8 => 1,
            DType::U16 => 2,
        };
        assert_eq!(DType::ALL.len(), 3);
        for (i, dtype) in DType::ALL.into_iter().enumerate() {
            assert_eq!(place(dtype), i);
        }
    }
}
