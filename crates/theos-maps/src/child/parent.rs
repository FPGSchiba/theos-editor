//! The parent package of a child run and the feature the child details (spec P29 §5.2 item 3).

use std::io::Cursor;
use std::path::{Path, PathBuf};

use geojson::GeometryValue;
use serde_json::{Map, Value};
use theos_map_format::{FormatError, Package, sha256_hex};

use crate::features::Geometry;
use crate::kernels::num;

/// Why a parent package or its feature was refused.
#[derive(Debug, thiserror::Error)]
pub enum ChildError {
    /// The package is not a readable map package (broken hash, unmet `requires`, other major).
    #[error("{}: {source}", path.display())]
    Read {
        /// The parent file.
        path: PathBuf,
        /// What the format reader said.
        source: FormatError,
    },
    /// The file could not be read.
    #[error("{}: {source}", path.display())]
    Io {
        /// The parent file.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// The parent has no feature with this id.
    #[error("the parent {} has no feature `{id}`", path.display())]
    UnknownFeature {
        /// The feature id asked for.
        id: String,
        /// The parent file.
        path: PathBuf,
    },
    /// The feature is of a type the recipe does not detail.
    #[error("feature `{id}` is a `{ty}`; the recipe accepts: {}", accepted.join(", "))]
    WrongType {
        /// The feature id.
        id: String,
        /// Its type.
        ty: String,
        /// The recipe `parent_types`.
        accepted: Vec<String>,
    },
    /// The feature has no point to centre the child on: it is a line or has no geometry.
    #[error("feature `{id}` has no point or area to centre the map on")]
    NoAnchor {
        /// The feature id.
        id: String,
    },
    /// The request value of the feature input is not a string.
    #[error("input `{input}` must be a feature id (a string)")]
    NotAFeatureId {
        /// The input name.
        input: String,
    },
}

/// The parent feature a child map details.
#[derive(Debug, Clone, PartialEq)]
pub struct ParentFeature {
    /// The feature id, such as `town-2`.
    pub id: String,
    /// Its kind (`properties.kind`), empty when the parent gives none.
    pub kind: String,
    /// Its type (`properties.type`), empty when the parent gives none.
    pub ty: String,
    /// Its geometry in the parent frame, metres.
    pub geometry: Geometry,
    /// Every property of the feature, `kind` and `type` included.
    pub properties: Map<String, Value>,
}

/// An opened parent package with the feature a request names.
#[derive(Debug, Clone)]
pub struct ParentContext {
    /// The parent file.
    pub path: PathBuf,
    /// The package as read.
    pub package: Package,
    /// SHA-256 of the file bytes.
    pub sha256: String,
    /// The feature the child details.
    pub feature: ParentFeature,
    /// The recipe feature input name.
    pub input: String,
}

impl ParentContext {
    /// Reads `from` with `theos_map_format::read`, hashes its bytes, finds the feature the
    /// request input names and checks its type against `parent_types`.
    ///
    /// # Errors
    /// [`ChildError::Io`] for an unreadable file, [`ChildError::Read`] for a file the format
    /// reader refuses, [`ChildError::UnknownFeature`] and [`ChildError::WrongType`].
    pub fn open(
        from: &Path,
        input: &str,
        feature_id: &str,
        parent_types: &[String],
    ) -> Result<Self, ChildError> {
        let bytes = std::fs::read(from).map_err(|source| ChildError::Io {
            path: from.to_path_buf(),
            source,
        })?;
        let sha256 = sha256_hex(&bytes);
        let package =
            theos_map_format::read(Cursor::new(bytes)).map_err(|source| ChildError::Read {
                path: from.to_path_buf(),
                source,
            })?;
        let feature =
            find_feature(&package, feature_id).ok_or_else(|| ChildError::UnknownFeature {
                id: feature_id.to_owned(),
                path: from.to_path_buf(),
            })?;
        if !parent_types.contains(&feature.ty) {
            return Err(ChildError::WrongType {
                id: feature.id,
                ty: feature.ty,
                accepted: parent_types.to_vec(),
            });
        }
        Ok(Self {
            path: from.to_path_buf(),
            package,
            sha256,
            feature,
            input: input.to_owned(),
        })
    }

    /// The feature anchor in the parent frame, metres: a point, or the centre of the bounding
    /// box of a polygon outline.
    ///
    /// # Errors
    /// [`ChildError::NoAnchor`] for a line or a feature without geometry.
    pub fn anchor_m(&self) -> Result<[f64; 2], ChildError> {
        let anchor = match &self.feature.geometry {
            Geometry::Point(at) => Some(*at),
            Geometry::Polygon(rings) => rings.first().and_then(|ring| bbox_centre(ring)),
            Geometry::Line(_) | Geometry::None => None,
        };
        anchor.ok_or_else(|| ChildError::NoAnchor {
            id: self.feature.id.clone(),
        })
    }
}

/// The value of the feature input in a request.
///
/// # Errors
/// [`ChildError::NotAFeatureId`] when the value is not a string, or absent.
pub fn feature_id<'a>(given: &'a Map<String, Value>, input: &str) -> Result<&'a str, ChildError> {
    given
        .get(input)
        .and_then(Value::as_str)
        .ok_or_else(|| ChildError::NotAFeatureId {
            input: input.to_owned(),
        })
}

/// The centre of the bounding box of `ring`, `None` for an empty ring.
fn bbox_centre(ring: &[[f64; 2]]) -> Option<[f64; 2]> {
    let first = ring.first()?;
    let (mut lo, mut hi) = (*first, *first);
    for p in ring {
        for axis in 0..2 {
            lo[axis] = num::min(lo[axis], p[axis]);
            hi[axis] = num::max(hi[axis], p[axis]);
        }
    }
    Some([f64::midpoint(lo[0], hi[0]), f64::midpoint(lo[1], hi[1])])
}

fn find_feature(package: &Package, id: &str) -> Option<ParentFeature> {
    let found = package
        .features
        .features
        .iter()
        .find(|f| matches!(&f.id, Some(geojson::feature::Id::String(s)) if s == id))?;
    let properties = found.properties.clone().unwrap_or_default();
    let text = |key: &str| {
        properties
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    Some(ParentFeature {
        id: id.to_owned(),
        kind: text("kind"),
        ty: text("type"),
        geometry: found
            .geometry
            .as_ref()
            .map_or(Geometry::None, |g| geometry_of(&g.value)),
        properties,
    })
}

/// The `[x, y]` of a position, `None` when it has fewer than two numbers.
fn xy(p: &[f64]) -> Option<[f64; 2]> {
    Some([*p.first()?, *p.get(1)?])
}

/// The `[x, y, z]` of a position; a missing `z` is 0.
fn xyz(p: &[f64]) -> Option<[f64; 3]> {
    Some([*p.first()?, *p.get(1)?, p.get(2).copied().unwrap_or(0.0)])
}

/// The feature geometry of a `GeoJSON` one; the kinds a generator never writes (multi-part,
/// collections) become [`Geometry::None`].
fn geometry_of(value: &GeometryValue) -> Geometry {
    let converted = match value {
        GeometryValue::Point { coordinates } => xy(coordinates.as_slice()).map(Geometry::Point),
        GeometryValue::LineString { coordinates } => coordinates
            .iter()
            .map(|p| xyz(p.as_slice()))
            .collect::<Option<Vec<_>>>()
            .map(Geometry::Line),
        GeometryValue::Polygon { coordinates } => coordinates
            .iter()
            .map(|ring| {
                ring.iter()
                    .map(|p| xy(p.as_slice()))
                    .collect::<Option<Vec<_>>>()
            })
            .collect::<Option<Vec<_>>>()
            .map(Geometry::Polygon),
        _ => None,
    };
    converted.unwrap_or(Geometry::None)
}
