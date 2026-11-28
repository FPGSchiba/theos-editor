//! The features of an interior map (spec P34 §5.10, `docs/map-package.md` §5): its rooms, doors,
//! connections and critical route, from the world values `room_graph`, `doors` and `analysis`
//! and the `rooms` raster.
//!
//! The features come in this order, which numbers them `<kind>-<n>` from 1 per kind: the rooms
//! by room number (`room-<k>` has `number` k − 1), the doors in connection order (`door-<i+1>`
//! is the door of connection i), the connections in graph order, and the one route. References
//! between them ([`Prop::Ref`]) are positions in that list, written as the ids.
//!
//! - A room is a `Polygon`: the cell-edge outline ([`outline`]) of its raster cells, holes
//!   included, in metres (`x = col · cell_m`, `y = row · cell_m`). The orientation is the
//!   outline's and the lakes': exteriors with a positive shoelace sum in the frame's (x east,
//!   y south), holes negative.
//! - A door is a `Point` at its middle cell's centre.
//! - A connection has no geometry; `from` and `to` give its direction for `oneway`.
//! - The route is a `LineString` through the analysis's route points (cell centres, z = 0) with
//!   `length_m` the analysis's `critical_path_m` (ruling R26: the line itself is shorter where
//!   legs bend). A world without `analysis`, or with no route, has no route feature.
//!
//! A world without `room_graph` has no features.

use std::collections::BTreeMap;

use geojson::FeatureCollection;

use super::output::{Assembly, FeaturesError, assemble};
use super::{Feature, FeatureHandle, Geometry, Prop};
use crate::grid::Grid;
use crate::kernels::num::{to_f64, to_f64_i64, to_u32, to_usize};
use crate::kernels::outline::outline;
use crate::kind::IdScheme;
use crate::rooms::{Passage, Role, Room, RoomGraph, RoomId, Shape};
use crate::stages::interior::playability_core::Analysis;
use crate::stages::interior::space_core::{Door, Doors};
use crate::world::{MapFrame, ValueType, World};

/// The features output of an interior `world`, numbered by `scheme`, and how many features each
/// type has.
///
/// # Errors
/// [`FeaturesError`] as [`interior_features`] and [`assemble`] give it.
pub(crate) fn build(
    world: &World,
    scheme: IdScheme,
) -> Result<(FeatureCollection, BTreeMap<String, usize>), FeaturesError> {
    assemble(&interior_features(world)?, scheme)
}

/// The rooms, doors, connections and route of an interior `world`, in output order.
///
/// # Errors
/// [`FeaturesError::MissingLayer`] when the world holds a room graph but not the `rooms` raster
/// or the doors, and [`FeaturesError::SplitRoom`] for a room whose cells make several outlines.
pub(crate) fn interior_features(world: &World) -> Result<Assembly, FeaturesError> {
    let Some(graph) = world.value::<RoomGraph>(RoomGraph::NAME) else {
        return Ok(Assembly::default());
    };
    let raster = world
        .u32("rooms")
        .ok_or(FeaturesError::MissingLayer { name: "rooms" })?;
    let doors = world
        .value::<Doors>(Doors::NAME)
        .ok_or(FeaturesError::MissingLayer { name: "doors" })?;
    let frame = *world.frame();
    let n_rooms = graph.rooms.len();
    let room = |r: RoomId| FeatureHandle(u32::from(r));
    let door = |i: usize| FeatureHandle(to_u32(n_rooms + i));

    let mut features = Vec::new();
    for (number, r) in graph.rooms.iter().enumerate() {
        features.push(room_feature(number, r, raster, frame)?);
    }
    for d in &doors.0 {
        features.push(door_feature(d, frame, room));
    }
    for (i, c) in graph.connections.iter().enumerate() {
        let (ty, from, to) = match c.kind {
            Passage::Open => ("open", c.a, c.b),
            Passage::Locked(_) => ("locked", c.a, c.b),
            Passage::Drop { from } => ("oneway", from, if from == c.a { c.b } else { c.a }),
        };
        let mut props = BTreeMap::from([
            ("from".to_owned(), Prop::Ref(room(from))),
            ("to".to_owned(), Prop::Ref(room(to))),
        ]);
        if let Passage::Locked(k) = c.kind {
            props.insert("key".to_owned(), Prop::Text(RoomGraph::key_name(k)));
        }
        if let Some(d) = doors.0.iter().position(|d| d.connection == i) {
            props.insert("door".to_owned(), Prop::Ref(door(d)));
        }
        features.push(Feature {
            kind: "connection",
            ty: ty.to_owned(),
            geometry: Geometry::None,
            props,
        });
    }
    if let Some(analysis) = world.value::<Analysis>(Analysis::NAME)
        && !analysis.route.is_empty()
    {
        features.push(route_feature(analysis, frame, room));
    }
    Ok(Assembly {
        features,
        lists: BTreeMap::new(),
    })
}

/// The name of a room's role.
fn role_name(role: Role) -> &'static str {
    match role {
        Role::Start => "start",
        Role::Goal => "goal",
        Role::Path => "path",
        Role::Leaf => "leaf",
        Role::Exit => "exit",
    }
}

/// The name of a room's shape.
fn shape_name(shape: Shape) -> &'static str {
    match shape {
        Shape::Rect => "rect",
        Shape::Round => "round",
        Shape::Ragged => "ragged",
        Shape::Passage => "passage",
    }
}

/// The room of number `number`: the outline of the raster cells holding `number + 1`.
fn room_feature(
    number: usize,
    r: &Room,
    raster: &Grid<u32>,
    frame: MapFrame,
) -> Result<Feature, FeaturesError> {
    let value = to_u32(number + 1);
    let mask = raster.map(|v| v == value);
    let cells = mask.as_slice().iter().filter(|&&on| on).count();
    let rings = outline(&mask);
    let exteriors = rings.iter().filter(|ring| shoelace2(ring) > 0).count();
    if exteriors > 1 {
        return Err(FeaturesError::SplitRoom {
            room: number,
            outlines: exteriors,
        });
    }
    let metres = |ring: &Vec<[i64; 2]>| {
        ring.iter()
            .map(|&[col, row]| {
                [
                    to_f64_i64(col) * frame.cell_m,
                    to_f64_i64(row) * frame.cell_m,
                ]
            })
            .collect()
    };
    let keys = r.keys.iter().map(|&k| RoomGraph::key_name(k)).collect();
    let props = BTreeMap::from([
        ("number".to_owned(), Prop::Int(i64::from(to_u32(number)))),
        ("role".to_owned(), Prop::Text(role_name(r.role).to_owned())),
        ("keys".to_owned(), Prop::Texts(keys)),
        ("locked".to_owned(), Prop::Bool(r.locked)),
        (
            "shape".to_owned(),
            Prop::Text(shape_name(r.shape).to_owned()),
        ),
        (
            "area_m2".to_owned(),
            Prop::Num(to_f64(cells) * frame.cell_area_m2()),
        ),
    ]);
    Ok(Feature {
        kind: "room",
        ty: r.ty.clone(),
        geometry: Geometry::Polygon(rings.iter().map(metres).collect()),
        props,
    })
}

/// Twice the signed area of a closed ring of `[col, row]` corners.
fn shoelace2(ring: &[[i64; 2]]) -> i64 {
    ring.windows(2)
        .map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1])
        .sum()
}

/// The centre in metres of the raster cell `(row, col)`.
fn centre(frame: MapFrame, (row, col): (u32, u32)) -> [f64; 2] {
    frame.centre_m(to_usize(row), to_usize(col))
}

/// The door `d` at its middle cell, with its two rooms, its key and, for a drop, its source.
fn door_feature(d: &Door, frame: MapFrame, room: impl Fn(RoomId) -> FeatureHandle) -> Feature {
    let mut props = BTreeMap::from([(
        "rooms".to_owned(),
        Prop::Refs(vec![room(d.rooms.0), room(d.rooms.1)]),
    )]);
    let ty = match d.kind {
        Passage::Open => "open",
        Passage::Locked(k) => {
            props.insert("key".to_owned(), Prop::Text(RoomGraph::key_name(k)));
            "locked"
        }
        Passage::Drop { from } => {
            props.insert("from".to_owned(), Prop::Ref(room(from)));
            "drop"
        }
    };
    Feature::point("door", ty, centre(frame, d.centre), props)
}

/// The critical route through the analysis's route points, on the floor (z = 0).
fn route_feature(
    analysis: &Analysis,
    frame: MapFrame,
    room: impl Fn(RoomId) -> FeatureHandle,
) -> Feature {
    let points = analysis
        .route_points
        .iter()
        .map(|&p| {
            let [x, y] = centre(frame, p);
            [x, y, 0.0]
        })
        .collect();
    let props = BTreeMap::from([
        (
            "rooms".to_owned(),
            Prop::Refs(analysis.route.iter().map(|&r| room(r)).collect()),
        ),
        ("length_m".to_owned(), Prop::Num(analysis.critical_path_m)),
    ]);
    Feature {
        kind: "route",
        ty: "critical".to_owned(),
        geometry: Geometry::Line(points),
        props,
    }
}
