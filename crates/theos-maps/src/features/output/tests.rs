//! The features output: ids, references, water and roads.

use std::collections::BTreeMap;

use geojson::{Feature as Gj, FeatureCollection, GeometryValue};
use serde_json::Value;

use super::*;
use crate::features::{Feature, FeatureHandle, FeatureSet, Geometry, Prop};
use crate::grid::Grid;
use crate::progress::CancelToken;
use crate::recipe::{RecipeOrigin, parse, plan};
use crate::runner::{RunOptions, execute};
use crate::stage::Registry;
use crate::stages::terrain::hydrology::{LakeShape, RiverLine, WaterFeatures};
use crate::stages::terrain::roads::{Road, RoadClass, RoadNetwork, RoadsParams};
use crate::world::{Layer, MapFrame, ValueType, World};

/// Terrain's build, numbered per type, as the runner calls it.
fn build(world: &World) -> Result<(FeatureCollection, BTreeMap<String, usize>), FeaturesError> {
    super::build(world, IdScheme::ByType)
}

type TestResult = Result<(), Box<dyn std::error::Error>>;

const WATER_SMALL: &str = include_str!("../../../tests/fixtures/recipes/water-small.toml");

/// Plans the recipe `text`, executes it with `seed` and builds the features.
fn run_and_build(text: &str, seed: u64) -> Result<FeatureCollection, Box<dyn std::error::Error>> {
    let loaded = parse(text, RecipeOrigin::Builtin)?;
    let registry = Registry::builtin();
    let planned = plan(&loaded, &registry, &serde_json::Map::new())?;
    let run = execute(
        &planned,
        seed,
        &registry,
        &RunOptions::default(),
        &mut |_| {},
        &CancelToken::new(),
    )?;
    Ok(build(&run.world)?.0)
}

fn of<'a>(fc: &'a FeatureCollection, kind: &str, ty: &str) -> Vec<&'a Gj> {
    fc.features
        .iter()
        .filter(|f| {
            prop(f, "kind") == Some(&Value::from(kind)) && prop(f, "type") == Some(&Value::from(ty))
        })
        .collect()
}

fn prop<'a>(f: &'a Gj, key: &str) -> Option<&'a Value> {
    f.properties.as_ref()?.get(key)
}

fn array_len(f: &Gj, key: &str) -> usize {
    prop(f, key).and_then(Value::as_array).map_or(0, Vec::len)
}

fn coords(f: &Gj) -> Vec<Vec<f64>> {
    match f.geometry.as_ref().map(|g| &g.value) {
        Some(GeometryValue::LineString { coordinates }) => {
            coordinates.iter().map(|p| p.as_slice().to_vec()).collect()
        }
        _ => Vec::new(),
    }
}

fn id(f: &Gj) -> String {
    match &f.id {
        Some(geojson::feature::Id::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn ids_of(fc: &FeatureCollection) -> Vec<String> {
    fc.features.iter().map(id).collect()
}

fn num(f: &Gj, key: &str) -> Option<f64> {
    prop(f, key).and_then(Value::as_f64)
}

// AC: CE-28.6
#[test]
fn features_carry_ids_kinds_types_and_water_properties() -> TestResult {
    let fc = run_and_build(WATER_SMALL, 1)?;
    let lakes = of(&fc, "water", "lake");
    let rivers = of(&fc, "water", "river");
    assert!(!lakes.is_empty() && !rivers.is_empty()); // the test's precondition
    let mut ids = std::collections::BTreeSet::new();
    for f in &fc.features {
        let Some(geojson::feature::Id::String(id)) = &f.id else {
            return Err(format!("{:?} is no string id", f.id).into());
        };
        assert_ne!(id.as_str(), "");
        // Ids are unique (map-package §5; final review round 4, A2).
        assert!(ids.insert(id.clone()), "{id} twice");
        assert!(prop(f, "kind").is_some() && prop(f, "type").is_some());
    }
    for l in lakes {
        assert!(num(l, "surface_m").is_some_and(f64::is_finite), "{l:?}");
    }
    for r in rivers {
        // A river is a LineString of at least two vertices; `coords` is empty for any other
        // geometry, which would make the per-vertex checks vacuous.
        let n = coords(r).len();
        assert!(n >= 2, "{:?}", r.geometry);
        for key in ["width_m", "discharge_m3s"] {
            assert_eq!(array_len(r, key), n);
            let numbers = prop(r, key)
                .and_then(Value::as_array)
                .is_some_and(|a| a.iter().all(|v| v.as_f64().is_some_and(f64::is_finite)));
            assert!(numbers, "{key} of {r:?}");
        }
        assert!(coords(r).iter().all(|c| c.len() == 3)); // bed height
    }
    Ok(())
}

fn point(kind: &'static str, ty: &str, at: [f64; 2]) -> Feature {
    Feature {
        kind,
        ty: ty.to_owned(),
        geometry: Geometry::Point(at),
        props: BTreeMap::new(),
    }
}

fn frame() -> MapFrame {
    MapFrame {
        rows: 10,
        cols: 10,
        cell_m: 100.0,
    }
}

fn square(x: f64, y: f64, s: f64) -> Vec<[f64; 2]> {
    vec![[x, y], [x + s, y], [x + s, y + s], [x, y + s], [x, y]]
}

/// A world with `set`, one road between its first and third feature, one river and one lake.
fn world_with(set: FeatureSet) -> Result<World, Box<dyn std::error::Error>> {
    let mut world = World::new(frame());
    world.insert("height", Layer::F64(Grid::filled(10, 10, 5.0)))?;
    world.insert("features", Layer::Value(Box::new(set)))?;
    let road = Road {
        class: RoadClass::Road,
        cells: vec![0, 1, 12],
        from: FeatureHandle(0),
        to: FeatureHandle(2),
        bridges: Vec::new(),
    };
    let network = RoadNetwork {
        roads: vec![road],
        params: RoadsParams::default(),
    };
    world.insert("road_network", Layer::Value(Box::new(network)))?;
    let water = WaterFeatures {
        rivers: vec![RiverLine {
            points: vec![[0.0, 0.0, 3.0], [10.0, 0.0, 2.5]],
            width_m: vec![1.0, 2.0],
            depth_m: vec![0.1, 0.2],
            discharge_m3s: vec![0.5, 0.7],
            order: 2,
        }],
        lakes: vec![LakeShape {
            id: 1,
            rings: vec![square(100.0, 100.0, 300.0)],
            surface_m: 4.0,
            terminal: false,
            outlet: Some([150.0, 150.0]),
            depth_max_m: 1.5,
        }],
    };
    world.insert("water_features", Layer::Value(Box::new(water)))?;
    Ok(world)
}

fn three_places() -> FeatureSet {
    let mut set = FeatureSet::default();
    set.add(point("settlement", "hamlet", [50.0, 50.0]));
    set.add(point("poi", "ruin", [350.0, 50.0]));
    set.add(point("settlement", "hamlet", [250.0, 150.0]));
    set
}

/// The water of `world`, changed by `edit` and put back.
fn edit_water(world: &mut World, edit: impl FnOnce(&mut WaterFeatures)) -> TestResult {
    let mut water = world
        .value::<WaterFeatures>("water_features")
        .cloned()
        .unwrap_or_default();
    edit(&mut water);
    world.insert("water_features", Layer::Value(Box::new(water)))?;
    Ok(())
}

#[test]
fn ids_are_numbered_per_type_in_order() -> TestResult {
    let world = world_with(three_places())?;
    let (fc, counts) = build(&world)?;
    assert_eq!(
        ids_of(&fc),
        [
            "hamlet-1", "ruin-1", "hamlet-2", "road-1", "river-1", "lake-1"
        ]
    );
    assert_eq!(counts.get("hamlet"), Some(&2));
    assert_eq!(counts.get("ruin"), Some(&1));
    assert_eq!(counts.get("road"), Some(&1));
    assert_eq!(counts.len(), 5);
    assert_eq!(prop(&fc.features[3], "kind"), Some(&Value::from("road")));
    assert_eq!(prop(&fc.features[3], "type"), Some(&Value::from("road")));
    Ok(())
}

#[test]
fn references_resolve_to_final_ids() -> TestResult {
    let mut set = three_places();
    let mut site = point("site", "mill", [100.0, 100.0]);
    let refs = vec![FeatureHandle(0), FeatureHandle(2)];
    site.props.insert("serves".into(), Prop::Refs(refs));
    site.props
        .insert("deposit".into(), Prop::Ref(FeatureHandle(1)));
    site.props.insert("road".into(), Prop::Road(0));
    site.props.insert("at".into(), Prop::At([1.0, 2.0]));
    site.props.insert("note".into(), Prop::Text("t".into()));
    site.props.insert("flag".into(), Prop::Bool(true));
    site.props.insert("n".into(), Prop::Int(3));
    site.props.insert("x".into(), Prop::Num(-0.0));
    site.props.insert("xs".into(), Prop::Nums(vec![1.5, 2.5]));
    set.add(site);
    let (fc, _) = build(&world_with(set)?)?;
    let mill = of(&fc, "site", "mill")[0];
    assert_eq!(
        prop(mill, "serves"),
        Some(&serde_json::json!(["hamlet-1", "hamlet-2"]))
    );
    assert_eq!(prop(mill, "deposit"), Some(&Value::from("ruin-1")));
    assert_eq!(prop(mill, "road"), Some(&Value::from("road-1")));
    assert_eq!(prop(mill, "at"), Some(&serde_json::json!([1.0, 2.0])));
    assert_eq!(prop(mill, "note"), Some(&Value::from("t")));
    assert_eq!(prop(mill, "flag"), Some(&Value::from(true)));
    assert_eq!(prop(mill, "n"), Some(&Value::from(3)));
    // Negative zero is written as zero.
    assert_eq!(num(mill, "x").map(f64::to_bits), Some(0.0_f64.to_bits()));
    assert_eq!(prop(mill, "xs"), Some(&serde_json::json!([1.5, 2.5])));
    // The road's ends are the ids of the hamlets.
    let road = of(&fc, "road", "road")[0];
    assert_eq!(prop(road, "from"), Some(&Value::from("hamlet-1")));
    assert_eq!(prop(road, "to"), Some(&Value::from("hamlet-2")));
    Ok(())
}

#[test]
fn a_type_change_before_ids_renames_no_reference() -> TestResult {
    let mut set = three_places();
    let mut site = point("site", "inn", [100.0, 100.0]);
    site.props
        .insert("serves".into(), Prop::Ref(FeatureHandle(2)));
    set.add(site);
    // The first hamlet becomes a village: the second keeps its handle and is now the first hamlet.
    assert!(set.set_type(FeatureHandle(0), "village"));
    let (fc, counts) = build(&world_with(set)?)?;
    assert_eq!(
        ids_of(&fc)[..4],
        ["village-1", "ruin-1", "hamlet-1", "inn-1"]
    );
    let inn = of(&fc, "site", "inn")[0];
    assert_eq!(prop(inn, "serves"), Some(&Value::from("hamlet-1")));
    let road = of(&fc, "road", "road")[0];
    assert_eq!(prop(road, "from"), Some(&Value::from("village-1")));
    assert_eq!(prop(road, "to"), Some(&Value::from("hamlet-1")));
    assert_eq!(counts.get("village"), Some(&1));
    assert_eq!(counts.get("hamlet"), Some(&1));
    Ok(())
}

#[test]
fn a_road_carries_geometry_and_measures() -> TestResult {
    let (fc, _) = build(&world_with(three_places())?)?;
    let road = of(&fc, "road", "road")[0];
    assert_eq!(coords(road), [[50.0, 50.0], [150.0, 50.0], [250.0, 150.0]]);
    let length = num(road, "length_m").unwrap_or(0.0);
    let expected = 100.0 + 100.0 * 2.0_f64.sqrt();
    assert!((length - expected).abs() < 1e-9, "{length}");
    assert_eq!(
        num(road, "width_m"),
        Some(RoadsParams::default().width_m.road)
    );
    assert_eq!(num(road, "grade_max"), Some(0.0));
    Ok(())
}

#[test]
fn a_river_and_a_lake_carry_their_properties() -> TestResult {
    let (fc, _) = build(&world_with(three_places())?)?;
    let river = of(&fc, "water", "river")[0];
    assert_eq!(coords(river), [[0.0, 0.0, 3.0], [10.0, 0.0, 2.5]]);
    assert_eq!(prop(river, "depth_m"), Some(&serde_json::json!([0.1, 0.2])));
    assert_eq!(prop(river, "order"), Some(&Value::from(2)));
    let lake = of(&fc, "water", "lake")[0];
    assert_eq!(num(lake, "surface_m"), Some(4.0));
    assert_eq!(prop(lake, "terminal"), Some(&Value::from(false)));
    assert_eq!(
        prop(lake, "outlet"),
        Some(&serde_json::json!([150.0, 150.0]))
    );
    assert_eq!(num(lake, "depth_max_m"), Some(1.5));
    assert!(matches!(
        lake.geometry.as_ref().map(|g| &g.value),
        Some(GeometryValue::Polygon { coordinates }) if coordinates.len() == 1
    ));
    Ok(())
}

#[test]
fn a_terminal_lake_has_no_outlet() -> TestResult {
    let mut world = world_with(three_places())?;
    edit_water(&mut world, |w| {
        w.lakes[0].terminal = true;
        w.lakes[0].outlet = None;
    })?;
    let (fc, _) = build(&world)?;
    let lake = of(&fc, "water", "lake")[0];
    assert_eq!(prop(lake, "terminal"), Some(&Value::from(true)));
    assert!(prop(lake, "outlet").is_none());
    Ok(())
}

#[test]
fn a_world_without_features_roads_or_water_gives_an_empty_collection() -> TestResult {
    let (fc, counts) = build(&World::new(frame()))?;
    assert!(fc.features.is_empty() && counts.is_empty());
    Ok(())
}

#[test]
fn a_reference_to_no_feature_is_an_error() -> TestResult {
    let mut set = three_places();
    let mut site = point("site", "mill", [0.0, 0.0]);
    site.props
        .insert("serves".into(), Prop::Ref(FeatureHandle(9)));
    set.add(site);
    let result = build(&world_with(set)?);
    assert!(
        matches!(result, Err(FeaturesError::UnknownFeature { handle: 9, .. })),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn a_reference_to_no_road_is_an_error() -> TestResult {
    let mut set = three_places();
    let mut site = point("site", "ford", [0.0, 0.0]);
    site.props.insert("road".into(), Prop::Road(4));
    set.add(site);
    let result = build(&world_with(set)?);
    assert!(
        matches!(result, Err(FeaturesError::UnknownRoad { index: 4, .. })),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn a_road_that_starts_at_no_feature_is_an_error() -> TestResult {
    let world = world_with(FeatureSet::default())?;
    let result = build(&world);
    assert!(
        matches!(result, Err(FeaturesError::UnknownFeature { .. })),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn a_property_may_not_take_the_name_of_kind_or_type() -> TestResult {
    let mut set = three_places();
    let mut site = point("site", "mill", [0.0, 0.0]);
    site.props.insert("type".into(), Prop::Int(1));
    set.add(site);
    let result = build(&world_with(set)?);
    assert!(
        matches!(result, Err(FeaturesError::ReservedProperty { .. })),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn a_number_that_is_not_finite_is_an_error() -> TestResult {
    let mut set = three_places();
    let mut site = point("resource", "iron-ore", [0.0, 0.0]);
    site.props.insert("richness".into(), Prop::Num(f64::NAN));
    set.add(site);
    let result = build(&world_with(set)?);
    assert!(
        matches!(result, Err(FeaturesError::NonFinite { .. })),
        "{result:?}"
    );
    let mut world = world_with(three_places())?;
    edit_water(&mut world, |w| w.lakes[0].surface_m = f64::INFINITY)?;
    assert!(matches!(
        build(&world),
        Err(FeaturesError::NonFinite { .. })
    ));
    let mut world = world_with(three_places())?;
    edit_water(&mut world, |w| w.rivers[0].points[0][2] = f64::NAN)?;
    assert!(matches!(
        build(&world),
        Err(FeaturesError::NonFinite { .. })
    ));
    Ok(())
}

#[test]
fn roads_need_the_height_layer() -> TestResult {
    let mut world = world_with(three_places())?;
    world.remove("height");
    let result = build(&world);
    assert!(
        matches!(result, Err(FeaturesError::MissingLayer { .. })),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn the_value_names_are_the_world_values_names() {
    assert_eq!(FeatureSet::NAME, "features");
    assert_eq!(RoadNetwork::NAME, "road_network");
    assert_eq!(WaterFeatures::NAME, "water_features");
}

/// The polygons of a lake feature: one for a `Polygon`, one per part for a `MultiPolygon`.
fn lake_polygons(f: &Gj) -> Result<Vec<Vec<Vec<geojson::Position>>>, Box<dyn std::error::Error>> {
    match f.geometry.as_ref().map(|g| &g.value) {
        Some(GeometryValue::Polygon { coordinates }) => Ok(vec![coordinates.clone()]),
        Some(GeometryValue::MultiPolygon { coordinates }) => Ok(coordinates.clone()),
        _ => Err("a lake is neither a Polygon nor a MultiPolygon".into()),
    }
}

#[test]
fn the_lakes_of_a_run_are_polygons_of_closed_rings() -> TestResult {
    let fc = run_and_build(WATER_SMALL, 1)?;
    let lakes = of(&fc, "water", "lake");
    assert!(!lakes.is_empty() && !of(&fc, "water", "river").is_empty());
    for lake in lakes {
        let polygons = lake_polygons(lake)?;
        assert_ne!(polygons.len(), 0);
        for polygon in polygons {
            assert_ne!(polygon.len(), 0);
            for ring in polygon {
                assert!(ring.len() >= 4 && ring.first() == ring.last());
            }
        }
    }
    Ok(())
}

#[test]
fn a_lake_without_an_outline_is_an_error() -> TestResult {
    let mut world = world_with(three_places())?;
    edit_water(&mut world, |w| w.lakes[0].rings.clear())?;
    let result = build(&world);
    assert!(
        matches!(result, Err(FeaturesError::NoOutline { lake: 1 })),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn a_lake_with_several_outlines_is_one_multipolygon_feature() -> TestResult {
    let mut world = world_with(three_places())?;
    edit_water(&mut world, |w| {
        let mut hole = square(150.0, 150.0, 100.0);
        hole.reverse();
        // Two basins under one lake number (the second touches the first at a corner), the first
        // with a hole; then a second lake with one outline.
        w.lakes[0].rings = vec![
            square(100.0, 100.0, 300.0),
            hole,
            square(400.0, 400.0, 100.0),
        ];
        w.lakes.push(LakeShape {
            id: 2,
            rings: vec![square(600.0, 600.0, 100.0)],
            surface_m: 9.0,
            terminal: true,
            outlet: None,
            depth_max_m: 0.5,
        });
    })?;
    let (fc, counts) = build(&world)?;
    let lakes = of(&fc, "water", "lake");
    assert_eq!(lakes.len(), 2);
    assert_eq!(counts.get("lake"), Some(&2));
    assert_eq!(
        (id(lakes[0]), id(lakes[1])),
        ("lake-1".into(), "lake-2".into())
    );
    let polygons = lake_polygons(lakes[0])?;
    assert!(matches!(
        lakes[0].geometry.as_ref().map(|g| &g.value),
        Some(GeometryValue::MultiPolygon { .. })
    ));
    assert_eq!(polygons.iter().map(Vec::len).collect::<Vec<_>>(), [2, 1]);
    // The properties appear once, on the one feature.
    assert_eq!(num(lakes[0], "surface_m"), Some(4.0));
    assert_eq!(num(lakes[1], "surface_m"), Some(9.0));
    assert_eq!(prop(lakes[1], "terminal"), Some(&Value::from(true)));
    assert!(matches!(
        lakes[1].geometry.as_ref().map(|g| &g.value),
        Some(GeometryValue::Polygon { .. })
    ));
    Ok(())
}

#[test]
fn every_reference_of_a_run_is_the_id_of_a_feature() -> TestResult {
    let fc = run_and_build(WATER_SMALL, 1)?;
    let known: std::collections::BTreeSet<String> = fc.features.iter().map(id).collect();
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for f in &fc.features {
        for key in ["road", "serves", "deposit", "from", "to"] {
            let Some(value) = prop(f, key) else { continue };
            let refs: Vec<&str> = match value {
                Value::String(s) => vec![s.as_str()],
                Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
                _ => Vec::new(),
            };
            assert_ne!(refs.len(), 0, "{} {key}", id(f));
            for r in refs {
                assert!(known.contains(r), "{} {key}: {r}", id(f));
            }
            *seen.entry(key).or_insert(0) += 1;
        }
    }
    for key in ["road", "serves", "deposit", "from", "to"] {
        assert!(
            seen.get(key).is_some_and(|&n| n > 0),
            "no `{key}` in the run"
        );
    }
    Ok(())
}

#[test]
fn a_road_carries_its_class() -> TestResult {
    let (fc, _) = build(&world_with(three_places())?)?;
    let road = of(&fc, "road", "road")[0];
    assert_eq!(prop(road, "class"), Some(&Value::from("road")));
    Ok(())
}

// The ids of a ByKind table count per feature kind, and the set-only output skips the terrain
// values.
#[test]
fn ids_by_kind_count_per_kind() -> TestResult {
    let mut set = FeatureSet::default();
    set.add(point("settlement", "town", [1.0, 1.0]));
    set.add(point("settlement", "village", [2.0, 1.0]));
    set.add(point("poi", "ruin", [3.0, 1.0]));
    let world = world_with(set)?;
    let (fc, counts) = super::build_set(&world, IdScheme::ByKind)?;
    let ids: Vec<_> = fc.features.iter().filter_map(|f| f.id.clone()).collect();
    let expected = ["settlement-1", "settlement-2", "poi-1"]
        .map(|s| geojson::feature::Id::String(s.to_owned()));
    assert_eq!(ids, expected);
    // The counts are per type in both schemes: they feed `features.<type>`.
    assert_eq!(counts.get("town"), Some(&1));
    assert_eq!(counts.get("ruin"), Some(&1));
    Ok(())
}

fn assembly(features: Vec<Feature>) -> Assembly {
    Assembly {
        features,
        lists: BTreeMap::new(),
    }
}

#[test]
fn a_feature_without_geometry_is_written_as_null() -> TestResult {
    let mut f = point("connection", "open", [0.0, 0.0]);
    f.geometry = Geometry::None;
    let (fc, _) = assemble(&assembly(vec![f]), IdScheme::ByKind)?;
    let json = serde_json::to_value(&fc.features[0])?;
    assert_eq!(json["geometry"], Value::Null);
    assert_eq!(json["id"], "connection-1");
    assert_eq!(json["properties"]["kind"], "connection");
    assert_eq!(json["properties"]["type"], "open");
    Ok(())
}

#[test]
fn ids_by_type_number_each_type_from_one() -> TestResult {
    let features = ["street", "lane", "street"]
        .map(|t| point("street", t, [0.0, 0.0]))
        .to_vec();
    let (fc, counts) = assemble(&assembly(features), IdScheme::ByType)?;
    assert_eq!(ids_of(&fc), ["street-1", "lane-1", "street-2"]);
    assert_eq!(counts.get("street"), Some(&2));
    Ok(())
}

#[test]
fn ids_by_kind_number_each_kind_from_one() -> TestResult {
    let features = vec![
        point("room", "hall", [0.0, 0.0]),
        point("door", "open", [0.0, 0.0]),
        point("room", "cell", [0.0, 0.0]),
    ];
    let (fc, counts) = assemble(&assembly(features), IdScheme::ByKind)?;
    assert_eq!(ids_of(&fc), ["room-1", "door-1", "room-2"]);
    let expected: BTreeMap<String, usize> = [("hall", 1), ("cell", 1), ("open", 1)]
        .map(|(k, n)| (k.to_owned(), n))
        .into();
    assert_eq!(counts, expected);
    Ok(())
}

#[test]
fn an_item_reference_becomes_the_items_id() -> TestResult {
    let item = |index| Prop::Item {
        list: "parcels",
        index,
    };
    let mut block = point("block", "block", [0.0, 0.0]);
    block.props.insert("parcel".into(), item(1));
    let features = vec![
        point("parcel", "plot", [1.0, 0.0]),
        point("parcel", "plot", [2.0, 0.0]),
        block,
    ];
    let lists = BTreeMap::from([("parcels", vec![FeatureHandle(0), FeatureHandle(1)])]);
    let good = Assembly { features, lists };
    let (fc, _) = assemble(&good, IdScheme::ByType)?;
    assert_eq!(
        prop(&fc.features[2], "parcel"),
        Some(&Value::from("plot-2"))
    );

    let mut bad = good.clone();
    bad.features[2].props.insert("parcel".into(), item(5));
    let err = assemble(&bad, IdScheme::ByType)
        .err()
        .map(|e| e.to_string());
    let text = err.unwrap_or_default();
    assert!(
        text.contains("parcels") && text.contains("block-1.parcel"),
        "{text}"
    );
    Ok(())
}

#[test]
fn ids_stay_unique_when_a_set_type_is_a_water_or_road_type() -> TestResult {
    let mut set = three_places();
    set.add(point("water", "lake", [10.0, 10.0]));
    set.add(point("water", "river", [20.0, 10.0]));
    set.add(point("road", "road", [30.0, 10.0]));
    let (fc, _) = build(&world_with(set)?)?;
    let ids = ids_of(&fc);
    let unique: std::collections::BTreeSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "{ids:?}");
    Ok(())
}

#[test]
fn ids_by_kind_stay_unique_across_set_roads_and_water() -> TestResult {
    let mut set = three_places();
    set.add(point("road", "ford", [30.0, 10.0]));
    set.add(point("water", "spring", [10.0, 10.0]));
    let (fc, _) = super::build(&world_with(set)?, IdScheme::ByKind)?;
    let ids = ids_of(&fc);
    let unique: std::collections::BTreeSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "{ids:?}");
    Ok(())
}
