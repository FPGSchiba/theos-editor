//! The features output (spec §5.10, `docs/map-package.md` §5): the `GeoJSON` features of a run,
//! with their stable ids.
//!
//! The features come in this order: the [`FeatureSet`] in handle order, then the roads of the
//! `road_network` in network order, then the river lines, then the lake polygons of
//! `water_features`. Every feature gets the id `<type>-<n>`, numbered per final type from 1 in
//! that order, so a stage that changed a type (a promotion) renames no reference. Every
//! [`Prop::Ref`], [`Prop::Refs`] and [`Prop::Road`] becomes the id or ids of the features it
//! names; a reference to something that does not exist is an error. Coordinates are metres, and
//! `-0.0` is written as `0.0`. A world without one of the three values has no features of it.
//!
//! Roads are `road`/`<class>` `LineString`s of cell centres with `from`, `to`, `length_m`,
//! `width_m` (from the parameters of the `roads` stage) and `grade_max`; a bridge is the straight
//! segment between its banks. Rivers are `water`/`river` `LineString`s of `[x, y, bed height]`
//! with the per-vertex `width_m`, `depth_m` and `discharge_m3s` and the `order`. Lakes are
//! `water`/`lake` Polygons ([`lakes`]) with `surface_m`, `terminal`, `outlet` (when the lake has
//! one) and `depth_max_m`.

mod lakes;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use geojson::{
    Feature as Gj, FeatureCollection, Geometry as GjGeometry, GeometryValue, JsonObject,
};
use serde_json::Value;

use super::{Feature, FeatureHandle, FeatureSet, Geometry, Prop};
use crate::kernels::num::canonical;
use crate::stages::terrain::hydrology::{LakeShape, RiverLine, WaterFeatures};
use crate::stages::terrain::roads::{Road, RoadNetwork};
use crate::world::{MapFrame, ValueType, World};

/// Why the features cannot be built.
#[derive(Debug, thiserror::Error)]
pub enum FeaturesError {
    /// A reference names a feature that is not in the list.
    #[error("{by} refers to feature {handle}, which does not exist")]
    UnknownFeature {
        /// The handle.
        handle: u32,
        /// What holds the reference, as text.
        by: String,
    },
    /// A reference names a road that is not in the network.
    #[error("{by} refers to road {index}, which does not exist")]
    UnknownRoad {
        /// The road's index in the network.
        index: u32,
        /// What holds the reference, as text.
        by: String,
    },
    /// A feature has a property named `kind` or `type`, which the output writes itself.
    #[error("{feature} has a property named `{name}`, which is reserved")]
    ReservedProperty {
        /// The feature's id.
        feature: String,
        /// The property.
        name: String,
    },
    /// A number is NaN or infinite and cannot be written to JSON.
    #[error("{feature}: `{property}` is not a finite number")]
    NonFinite {
        /// The feature's id.
        feature: String,
        /// The property, or `geometry`.
        property: String,
    },
    /// A layer the output reads is missing from the world.
    #[error("the world has no layer `{name}`")]
    MissingLayer {
        /// The layer.
        name: &'static str,
    },
    /// A lake outline has a hole that lies inside no exterior ring.
    #[error("lake {lake} has a hole inside no outline")]
    OrphanHole {
        /// The lake's number.
        lake: u32,
    },
}

/// The ids of every feature, assigned before any feature is built so references can point
/// forward (a site names its road, a road is numbered after the sites).
/// A polygon: an exterior ring followed by its holes.
type Polygon = Vec<Vec<[f64; 2]>>;

struct Named {
    /// The id of each feature of the set, by handle.
    set: Vec<String>,
    /// The id of each road, by index.
    roads: Vec<String>,
    /// The id of each river line.
    rivers: Vec<String>,
    /// The polygons of each lake with their ids.
    lakes: Vec<Vec<(String, Polygon)>>,
    /// How many features each type has.
    counts: BTreeMap<String, usize>,
}

impl Named {
    fn take(counts: &mut BTreeMap<String, usize>, ty: &str) -> String {
        let n = counts.entry(ty.to_owned()).or_insert(0);
        *n += 1;
        format!("{ty}-{n}")
    }

    fn new(set: &FeatureSet, roads: &[Road], water: &WaterFeatures) -> Result<Self, FeaturesError> {
        let mut counts = BTreeMap::new();
        let set_ids = set
            .iter()
            .map(|(_, f)| Self::take(&mut counts, &f.ty))
            .collect();
        let road_ids = roads
            .iter()
            .map(|r| Self::take(&mut counts, r.class.name()))
            .collect();
        let river_ids = water
            .rivers
            .iter()
            .map(|_| Self::take(&mut counts, "river"))
            .collect();
        let mut lake_ids = Vec::new();
        for lake in &water.lakes {
            let polygons = lakes::polygons(lake.id, &lake.rings)?;
            lake_ids.push(
                polygons
                    .into_iter()
                    .map(|p| (Self::take(&mut counts, "lake"), p))
                    .collect(),
            );
        }
        Ok(Self {
            set: set_ids,
            roads: road_ids,
            rivers: river_ids,
            lakes: lake_ids,
            counts,
        })
    }

    fn feature(&self, handle: FeatureHandle, by: &str) -> Result<&str, FeaturesError> {
        usize::try_from(handle.0)
            .ok()
            .and_then(|i| self.set.get(i))
            .map(String::as_str)
            .ok_or_else(|| FeaturesError::UnknownFeature {
                handle: handle.0,
                by: by.to_owned(),
            })
    }

    fn road(&self, index: u32, by: &str) -> Result<&str, FeaturesError> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.roads.get(i))
            .map(String::as_str)
            .ok_or_else(|| FeaturesError::UnknownRoad {
                index,
                by: by.to_owned(),
            })
    }
}

/// A finite number as JSON, with `-0.0` written as `0.0`.
fn number(x: f64, feature: &str, property: &str) -> Result<Value, FeaturesError> {
    serde_json::Number::from_f64(canonical(x))
        .map(Value::Number)
        .ok_or_else(|| FeaturesError::NonFinite {
            feature: feature.to_owned(),
            property: property.to_owned(),
        })
}

fn numbers(xs: &[f64], feature: &str, property: &str) -> Result<Value, FeaturesError> {
    xs.iter()
        .map(|&x| number(x, feature, property))
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
}

/// A coordinate, `-0.0` written as `0.0`, refused when not finite.
fn coord(x: f64, feature: &str) -> Result<f64, FeaturesError> {
    if x.is_finite() {
        Ok(canonical(x))
    } else {
        Err(FeaturesError::NonFinite {
            feature: feature.to_owned(),
            property: "geometry".to_owned(),
        })
    }
}

fn position<const N: usize>(p: &[f64; N], feature: &str) -> Result<Vec<f64>, FeaturesError> {
    p.iter().map(|&x| coord(x, feature)).collect()
}

fn ring(r: &[[f64; 2]], feature: &str) -> Result<Vec<Vec<f64>>, FeaturesError> {
    r.iter().map(|p| position(p, feature)).collect()
}

/// The properties every feature has: `kind` and `type`.
fn base(kind: &str, ty: &str) -> JsonObject {
    let mut props = JsonObject::new();
    props.insert("kind".to_owned(), Value::from(kind));
    props.insert("type".to_owned(), Value::from(ty));
    props
}

fn assemble(id: &str, props: JsonObject, geometry: GeometryValue) -> Gj {
    Gj {
        bbox: None,
        geometry: Some(GjGeometry::new(geometry)),
        id: Some(geojson::feature::Id::String(id.to_owned())),
        properties: Some(props),
        foreign_members: None,
    }
}

fn prop_value(p: &Prop, id: &str, name: &str, named: &Named) -> Result<Value, FeaturesError> {
    let by = format!("{id}.{name}");
    Ok(match p {
        Prop::Num(x) => number(*x, id, name)?,
        Prop::Int(n) => Value::from(*n),
        Prop::Text(t) => Value::from(t.as_str()),
        Prop::Bool(b) => Value::from(*b),
        Prop::Nums(xs) => numbers(xs, id, name)?,
        Prop::At(at) => numbers(at, id, name)?,
        Prop::Ref(h) => Value::from(named.feature(*h, &by)?),
        Prop::Refs(hs) => Value::Array(
            hs.iter()
                .map(|h| named.feature(*h, &by).map(Value::from))
                .collect::<Result<_, _>>()?,
        ),
        Prop::Road(i) => Value::from(named.road(*i, &by)?),
    })
}

fn set_feature(f: &Feature, id: &str, named: &Named) -> Result<Gj, FeaturesError> {
    let mut props = JsonObject::new();
    for (name, p) in &f.props {
        if name == "kind" || name == "type" {
            return Err(FeaturesError::ReservedProperty {
                feature: id.to_owned(),
                name: name.clone(),
            });
        }
        props.insert(name.clone(), prop_value(p, id, name, named)?);
    }
    props.extend(base(f.kind, &f.ty));
    let geometry = match &f.geometry {
        Geometry::Point(p) => GeometryValue::new_point(position(p, id)?),
        Geometry::Line(points) => GeometryValue::new_line_string(
            points
                .iter()
                .map(|p| position(p, id))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Geometry::Polygon(rings) => GeometryValue::new_polygon(
            rings
                .iter()
                .map(|r| ring(r, id))
                .collect::<Result<Vec<_>, _>>()?,
        ),
    };
    Ok(assemble(id, props, geometry))
}

fn road_feature(
    road: &Road,
    id: &str,
    network: &RoadNetwork,
    height: &crate::grid::Grid<f64>,
    frame: &MapFrame,
    named: &Named,
) -> Result<Gj, FeaturesError> {
    let mut props = base("road", road.class.name());
    props.insert(
        "from".to_owned(),
        Value::from(named.feature(road.from, id)?),
    );
    props.insert("to".to_owned(), Value::from(named.feature(road.to, id)?));
    props.insert(
        "length_m".to_owned(),
        number(road.length_m(frame), id, "length_m")?,
    );
    props.insert(
        "width_m".to_owned(),
        number(network.params.width_m.of(road.class), id, "width_m")?,
    );
    props.insert(
        "grade_max".to_owned(),
        number(road.grade_max(height, frame), id, "grade_max")?,
    );
    let line = road
        .cells
        .iter()
        .map(|&cell| {
            let cell = usize::try_from(cell).unwrap_or(usize::MAX);
            position(&frame.centre_m(cell / frame.cols, cell % frame.cols), id)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(assemble(id, props, GeometryValue::new_line_string(line)))
}

fn river_feature(river: &RiverLine, id: &str) -> Result<Gj, FeaturesError> {
    let mut props = base("water", "river");
    props.insert(
        "width_m".to_owned(),
        numbers(&river.width_m, id, "width_m")?,
    );
    props.insert(
        "depth_m".to_owned(),
        numbers(&river.depth_m, id, "depth_m")?,
    );
    props.insert(
        "discharge_m3s".to_owned(),
        numbers(&river.discharge_m3s, id, "discharge_m3s")?,
    );
    props.insert("order".to_owned(), Value::from(river.order));
    let line = river
        .points
        .iter()
        .map(|p| position(p, id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(assemble(id, props, GeometryValue::new_line_string(line)))
}

fn lake_feature(
    lake: &LakeShape,
    id: &str,
    polygon: &[Vec<[f64; 2]>],
) -> Result<Gj, FeaturesError> {
    let mut props = base("water", "lake");
    props.insert(
        "surface_m".to_owned(),
        number(lake.surface_m, id, "surface_m")?,
    );
    props.insert("terminal".to_owned(), Value::from(lake.terminal));
    if let Some(at) = lake.outlet {
        props.insert("outlet".to_owned(), numbers(&at, id, "outlet")?);
    }
    props.insert(
        "depth_max_m".to_owned(),
        number(lake.depth_max_m, id, "depth_max_m")?,
    );
    let rings = polygon
        .iter()
        .map(|r| ring(r, id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(assemble(id, props, GeometryValue::new_polygon(rings)))
}

/// The features of `world` and how many of each type there are.
///
/// # Errors
/// [`FeaturesError`] for a reference that resolves to nothing, a reserved or non-finite
/// property, a lake hole outside every outline, or a world with roads but no `height` layer.
pub(crate) fn build(
    world: &World,
) -> Result<(FeatureCollection, BTreeMap<String, usize>), FeaturesError> {
    let frame = world.frame();
    let no_set = FeatureSet::default();
    let no_water = WaterFeatures::default();
    let set = world
        .value::<FeatureSet>(FeatureSet::NAME)
        .unwrap_or(&no_set);
    let network = world.value::<RoadNetwork>(RoadNetwork::NAME);
    let water = world
        .value::<WaterFeatures>(WaterFeatures::NAME)
        .unwrap_or(&no_water);
    let roads: &[Road] = network.map_or(&[], |n| &n.roads);
    let named = Named::new(set, roads, water)?;

    let mut features = Vec::new();
    for ((_, f), id) in set.iter().zip(&named.set) {
        features.push(set_feature(f, id, &named)?);
    }
    if let Some(network) = network.filter(|n| !n.roads.is_empty()) {
        let height = world
            .f64("height")
            .ok_or(FeaturesError::MissingLayer { name: "height" })?;
        for (road, id) in network.roads.iter().zip(&named.roads) {
            features.push(road_feature(road, id, network, height, frame, &named)?);
        }
    }
    for (river, id) in water.rivers.iter().zip(&named.rivers) {
        features.push(river_feature(river, id)?);
    }
    for (lake, polygons) in water.lakes.iter().zip(&named.lakes) {
        for (id, polygon) in polygons {
            features.push(lake_feature(lake, id, polygon)?);
        }
    }
    let collection = FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };
    Ok((collection, named.counts))
}
