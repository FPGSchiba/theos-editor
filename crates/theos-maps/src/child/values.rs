//! The values a child run seeds into its world before the first stage (spec P29 §5.2 item 5):
//! the parent's layers resampled into the child frame (`parent.<layer>`), the parent features
//! that touch the window (`parent.features`), the chosen feature (`parent.feature`) and, for
//! settlements, the class table entry (`class_frame`, L10).

use std::any::Any;

use serde::Serialize;
use serde_json::{Map, Value};
use theos_map_format::{FormatError, LayerData, LayerEntry};

use super::{ChildError, ParentContext, ParentFeature};
use crate::error::RunError;
use crate::features::Geometry;
use crate::grid::Grid;
use crate::kernels::KernelError;
use crate::kernels::num::{max, min};
use crate::kernels::resample::{ChildGrid, Sampling, resample};
use crate::progress::CancelToken;
use crate::recipe::Plan;
use crate::stages::settlement::class::ClassFrame;
use crate::world::{Layer, ValueType, WorldValue};

/// The world name of [`ParentFeatureValue`].
pub const PARENT_FEATURE: &str = "parent.feature";
/// The world name of [`ParentFeatures`].
pub const PARENT_FEATURES: &str = "parent.features";
/// The world name of [`ClassFrameValue`].
pub const CLASS_FRAME: &str = "class_frame";
/// The prefix of the world names of resampled parent layers.
pub const PARENT_PREFIX: &str = "parent.";

/// A feature of the parent map, in the child's frame, with the parent's frame.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParentFeatureValue {
    /// The feature id in the parent.
    pub id: String,
    /// Its kind.
    pub kind: String,
    /// Its type.
    pub ty: String,
    /// Its geometry in the child's frame, metres.
    pub geometry: Geometry,
    /// Every property of the feature.
    pub properties: Map<String, Value>,
    /// The parent's extent, m.
    pub parent_size_m: [f64; 2],
    /// The parent's cell edge, m.
    pub parent_cell_m: f64,
    /// The parent's grid, `[rows, columns]`.
    pub parent_grid: [u32; 2],
    /// SHA-256 of the parent file.
    pub package_sha256: String,
}

/// The parent's features that touch the child's window, in the child's frame.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParentFeatures {
    /// The features in the parent file's order.
    pub features: Vec<ParentFeatureValue>,
}

/// The class table entry a settlement map follows (L10).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClassFrameValue {
    /// The class, such as `town`.
    pub class: String,
    /// Its window, cell and built-up radius after the recipe's overrides.
    pub frame: ClassFrame,
}

/// Makes `$ty` a world value named `$name` whose JSON is its serialisation.
macro_rules! world_value {
    ($ty:ty, $name:literal) => {
        impl WorldValue for $ty {
            fn type_name(&self) -> &'static str {
                Self::NAME
            }
            fn canonical_json(&self) -> Result<Value, FormatError> {
                theos_map_format::json::to_checked_value(self)
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
        impl ValueType for $ty {
            const NAME: &'static str = $name;
        }
    };
}

world_value!(ParentFeatureValue, "parent_feature");
world_value!(ParentFeatures, "parent_features");
world_value!(ClassFrameValue, "class_frame");

/// The values a child kind seeds into the world before the first stage: one `(name, layer)` per
/// entry of `plan.seeded`, in that order. `parent` is the parent package of a child run.
///
/// # Errors
/// [`RunError::Child`] when a parent value is asked for without a parent or a parent layer
/// cannot be read or resampled, [`RunError::Cancelled`] when `cancel` is set.
pub(crate) fn seed(
    parent: Option<&ParentContext>,
    plan: &Plan,
    cancel: &CancelToken,
) -> Result<Vec<(String, Layer)>, RunError> {
    let mut seeded = Vec::with_capacity(plan.seeded.len());
    for spec in &plan.seeded {
        let name = spec.name;
        let layer = if name == CLASS_FRAME {
            plan.class_frame
                .clone()
                .map(|v| Layer::Value(Box::new(v)))
                .ok_or_else(|| no_parent(name))?
        } else {
            let parent = parent.ok_or_else(|| no_parent(name))?;
            parent_value(parent, plan, name, cancel)?
        };
        seeded.push((name.to_owned(), layer));
    }
    Ok(seeded)
}

fn no_parent(name: &str) -> RunError {
    ChildError::NoParent {
        value: name.to_owned(),
    }
    .into()
}

/// The world value `name` (`parent.*`) of `parent` in the child frame of `plan`.
fn parent_value(
    parent: &ParentContext,
    plan: &Plan,
    name: &str,
    cancel: &CancelToken,
) -> Result<Layer, RunError> {
    let origin = plan.origin_m.unwrap_or([0.0, 0.0]);
    match name {
        PARENT_FEATURE => Ok(Layer::Value(Box::new(feature_value(
            parent,
            &parent.feature,
            origin,
        )))),
        PARENT_FEATURES => {
            let size = plan.frame.size_m();
            let features = parent
                .features()
                .iter()
                .filter(|f| touches(&f.geometry, origin, size))
                .map(|f| feature_value(parent, f, origin))
                .collect();
            Ok(Layer::Value(Box::new(ParentFeatures { features })))
        }
        _ => {
            let layer = name.strip_prefix(PARENT_PREFIX).unwrap_or(name);
            Ok(Layer::F64(resampled(parent, plan, layer, origin, cancel)?))
        }
    }
}

/// The parent layer `layer` as floats, resampled into the child frame: nearest cell for a class
/// layer (one with a legend, L11), Catmull-Rom for a measured one.
fn resampled(
    parent: &ParentContext,
    plan: &Plan,
    layer: &str,
    origin: [f64; 2],
    cancel: &CancelToken,
) -> Result<Grid<f64>, RunError> {
    let package = &parent.package;
    let entry = package
        .manifest
        .layers
        .iter()
        .find(|e| e.level.is_none() && e.name == layer)
        .ok_or_else(|| no_parent(layer))?;
    let data = package
        .layers
        .get(&entry.key())
        .ok_or_else(|| no_parent(layer))?;
    let shape_error = || ChildError::LayerShape {
        layer: layer.to_owned(),
        rows: entry.shape[0],
        cols: entry.shape[1],
    };
    let rows = usize::try_from(entry.shape[0]).map_err(|_| shape_error())?;
    let cols = usize::try_from(entry.shape[1]).map_err(|_| shape_error())?;
    let grid = Grid::from_vec(rows, cols, decode(entry, data)).map_err(|_| shape_error())?;
    let sampling = if entry.legend.is_some() {
        Sampling::Nearest
    } else {
        Sampling::CatmullRom
    };
    let child = ChildGrid {
        origin_m: origin,
        rows: plan.frame.rows,
        cols: plan.frame.cols,
        cell_m: plan.frame.cell_m,
    };
    resample(
        &grid,
        package.manifest.frame.cell_m,
        &child,
        sampling,
        &|| cancel.is_cancelled(),
    )
    .map_err(|source| match source {
        KernelError::Cancelled => RunError::Cancelled,
        source => ChildError::Resample {
            layer: layer.to_owned(),
            source,
        }
        .into(),
    })
}

/// The values of a parent layer as floats: every value exactly, and a quantised layer (one with
/// a `scale`) decoded as `offset + raw * scale`. A class layer keeps its codes.
fn decode(entry: &LayerEntry, data: &LayerData) -> Vec<f64> {
    let raw: Vec<f64> = match data {
        LayerData::F32(v) => v.iter().map(|&x| f64::from(x)).collect(),
        LayerData::U8(v) => v.iter().map(|&x| f64::from(x)).collect(),
        LayerData::U16(v) => v.iter().map(|&x| f64::from(x)).collect(),
    };
    match (entry.scale, &entry.legend) {
        (Some(scale), None) => {
            let offset = entry.offset.unwrap_or(0.0);
            raw.into_iter().map(|v| offset + v * scale).collect()
        }
        _ => raw,
    }
}

/// `feature` in the child frame with the parent's frame next to it.
fn feature_value(
    parent: &ParentContext,
    feature: &ParentFeature,
    origin: [f64; 2],
) -> ParentFeatureValue {
    let frame = &parent.package.manifest.frame;
    ParentFeatureValue {
        id: feature.id.clone(),
        kind: feature.kind.clone(),
        ty: feature.ty.clone(),
        geometry: translated(&feature.geometry, origin),
        properties: feature.properties.clone(),
        parent_size_m: frame.size_m,
        parent_cell_m: frame.cell_m,
        parent_grid: frame.grid,
        package_sha256: parent.sha256.clone(),
    }
}

/// `geometry` moved by `-origin`.
fn translated(geometry: &Geometry, origin: [f64; 2]) -> Geometry {
    let xy = |p: &[f64; 2]| [p[0] - origin[0], p[1] - origin[1]];
    match geometry {
        Geometry::Point(p) => Geometry::Point(xy(p)),
        Geometry::Line(points) => Geometry::Line(
            points
                .iter()
                .map(|p| [p[0] - origin[0], p[1] - origin[1], p[2]])
                .collect(),
        ),
        Geometry::Polygon(rings) => {
            Geometry::Polygon(rings.iter().map(|r| r.iter().map(xy).collect()).collect())
        }
        Geometry::None => Geometry::None,
    }
}

/// Whether the bounding box of `geometry` overlaps the window at `origin` of extent `size`
/// (edges count); a feature without geometry touches nothing.
fn touches(geometry: &Geometry, origin: [f64; 2], size: [f64; 2]) -> bool {
    let points: Vec<[f64; 2]> = match geometry {
        Geometry::Point(p) => vec![*p],
        Geometry::Line(points) => points.iter().map(|p| [p[0], p[1]]).collect(),
        Geometry::Polygon(rings) => rings.iter().flatten().copied().collect(),
        Geometry::None => return false,
    };
    let Some(first) = points.first() else {
        return false;
    };
    let (mut lo, mut hi) = (*first, *first);
    for p in &points {
        for axis in 0..2 {
            lo[axis] = min(lo[axis], p[axis]);
            hi[axis] = max(hi[axis], p[axis]);
        }
    }
    (0..2).all(|axis| hi[axis] >= origin[axis] && lo[axis] <= origin[axis] + size[axis])
}
