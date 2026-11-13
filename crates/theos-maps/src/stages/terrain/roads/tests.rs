//! Tests of the `roads` stage on synthetic 64² worlds of 100 m cells.

use std::collections::BTreeMap;

use super::{RoadClass, RoadNetwork, Roads};
use crate::features::{Feature, FeatureHandle, FeatureSet, Geometry, Prop};
use crate::grid::Grid;
use crate::kernels::mst::UnionFind;
use crate::kernels::num::to_f64;
use crate::progress::{CancelToken, Progress};
use crate::stage::{Severity, Stage, StageContext, StageError, StageReport};
use crate::stages::terrain::support::{run_stage, table, world};
use crate::world::Layer;
use crate::world::World;

const N: usize = 64;
/// `flow_dir` code of a river flowing south (L19).
const SOUTH: u8 = 5;

fn grid<T: Copy>(f: impl Fn(usize, usize) -> T) -> Grid<T> {
    Grid::from_vec(N, N, (0..N * N).map(|i| f(i / N, i % N)).collect()).unwrap()
}

/// A world of `height` with no vegetation, no water and an empty feature list.
fn terrain(height: impl Fn(usize, usize) -> f64) -> World {
    let mut w = world(N, N);
    w.insert("height", Layer::F64(grid(height))).unwrap();
    w.insert("vegetation", Layer::F64(grid(|_, _| 0.0)))
        .unwrap();
    for name in ["river", "big_river", "lake"] {
        w.insert(name, Layer::Mask(grid(|_, _| false))).unwrap();
    }
    w.insert("flow_dir", Layer::U8(grid(|_, _| 0))).unwrap();
    w.insert("features", Layer::Value(Box::new(FeatureSet::default())))
        .unwrap();
    w
}

fn flat() -> World {
    terrain(|_, _| 100.0)
}

fn set_mask(w: &mut World, name: &str, on: impl Fn(usize, usize) -> bool) {
    w.insert(name, Layer::Mask(grid(on))).unwrap();
}

/// A river flowing south in the columns `cols`; `big` makes it a big river.
fn river(w: &mut World, cols: std::ops::RangeInclusive<usize>, big: bool) {
    let on = |_: usize, c: usize| cols.contains(&c);
    set_mask(w, "river", on);
    if big {
        set_mask(w, "big_river", on);
    }
    w.insert(
        "flow_dir",
        Layer::U8(grid(|r, c| if on(r, c) { SOUTH } else { 0 })),
    )
    .unwrap();
}

/// Adds a feature of `kind` and `ty` at the centre of cell `(r, c)`, with `name` when given.
fn add(w: &mut World, kind: &'static str, ty: &str, (r, c): (usize, usize)) -> FeatureHandle {
    let at = w.frame().centre_m(r, c);
    let mut props = BTreeMap::new();
    if kind == "settlement" {
        props.insert("name".to_owned(), Prop::Text(format!("{ty}-{r}-{c}")));
    }
    let mut set = w.value::<FeatureSet>("features").unwrap().clone();
    let h = set.add(Feature {
        kind,
        ty: ty.to_owned(),
        geometry: Geometry::Point(at),
        props,
    });
    w.insert("features", Layer::Value(Box::new(set))).unwrap();
    h
}

fn town(w: &mut World, at: (usize, usize)) -> FeatureHandle {
    add(w, "settlement", "town", at)
}

fn run(w: &mut World, params: &str) -> StageReport {
    run_stage(w, &Roads, params).unwrap()
}

fn network(w: &World) -> &RoadNetwork {
    w.value::<RoadNetwork>("road_network").unwrap()
}

fn sites<'a>(w: &'a World, ty: &str) -> Vec<&'a Feature> {
    w.value::<FeatureSet>("features")
        .unwrap()
        .of_kind("site")
        .filter(|(_, f)| f.ty == ty)
        .map(|(_, f)| f)
        .collect()
}

fn cell_of(world: &World, site: &Feature) -> usize {
    let Geometry::Point([x, y]) = site.geometry else {
        panic!("a site is a point")
    };
    let (row, col) = world.frame().cell_at_m(x, y);
    row * N + col
}

fn rc(i: usize) -> (usize, usize) {
    (i / N, i % N)
}

fn warnings(report: &StageReport) -> Vec<&str> {
    report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .map(|f| f.message.as_str())
        .collect()
}

/// The length in metres of the step from cell `a` to cell `b`.
fn step_m(a: usize, b: usize) -> f64 {
    let ((ra, ca), (rb, cb)) = (rc(a), rc(b));
    let (dr, dc) = (to_f64(ra.abs_diff(rb)), to_f64(ca.abs_diff(cb)));
    100.0 * libm::sqrt(dr * dr + dc * dc)
}

/// A ridge along column 20 with a pass at row 32, and a big river in columns 44 and 45.
fn ridge_and_river() -> World {
    let mut w = terrain(|r, c| {
        let x = (to_f64(c) - 20.0) / 5.0;
        let along = 0.4 + 0.6 * (to_f64(r) - 32.0).abs() / 32.0;
        100.0 + 250.0 * along * libm::exp(-(x * x))
    });
    river(&mut w, 44..=45, true);
    w
}

/// The ridge and river world with towns on both sides of each, a village, a hamlet, a gate and
/// a ruin on the ridge, after `roads` with `params`.
fn ridge_network(params: &str) -> (RoadNetwork, Grid<f64>) {
    let mut w = ridge_and_river();
    town(&mut w, (32, 6));
    town(&mut w, (30, 34));
    town(&mut w, (20, 56));
    add(&mut w, "settlement", "village", (52, 8));
    add(&mut w, "settlement", "hamlet", (8, 30));
    add(&mut w, "poi", "gate-down", (56, 56));
    add(&mut w, "poi", "ruin", (10, 20));
    run(&mut w, params);
    (network(&w).clone(), w.f64("height").unwrap().clone())
}

/// The default steepest grade of `class` (spec ruling 35).
fn limit(class: RoadClass) -> f64 {
    match class {
        RoadClass::Main => 0.08,
        RoadClass::Road => 0.10,
        RoadClass::Track => 0.15,
        RoadClass::Trail => 0.25,
    }
}

/// The largest amount by which a step that is not a bridge exceeds its class's default limit;
/// negative when every step is within it.
fn excess(net: &RoadNetwork, height: &Grid<f64>) -> f64 {
    let mut worst = f64::NEG_INFINITY;
    for road in &net.roads {
        for (a, b) in road.steps() {
            if road.is_bridge(a, b) {
                continue;
            }
            let grade = (height.as_slice()[b] - height.as_slice()[a]).abs() / step_m(a, b);
            worst = crate::kernels::num::max(worst, grade - limit(road.class));
        }
    }
    worst
}

// Physics test 12 (spec §5.14): no road step exceeds its class's maximum grade; bridges are
// the only exception.
#[test]
fn no_road_step_exceeds_its_class_grade() {
    // The default costs, and a lax comfort grade under which only the limits keep roads off
    // the ridge's flanks (steeper than 0.4 at its ends).
    for params in ["", "comfort_grade = 1.0"] {
        let (net, height) = ridge_network(params);
        assert!(net.roads.len() >= 5, "{params}: {} roads", net.roads.len());
        assert!(net.roads.iter().any(|r| !r.bridges.is_empty()), "no bridge");
        assert!(net.roads.iter().any(|r| r.class == RoadClass::Trail));
        let crosses_ridge = |road: &super::Road| {
            let cols = || road.cells.iter().map(|&i| i as usize % N);
            cols().any(|c| c < 18) && cols().any(|c| c > 22)
        };
        assert!(net.roads.iter().any(crosses_ridge));
        let worst = excess(&net, &height);
        assert!(
            worst <= 1e-12,
            "{params}: a step exceeds its limit by {worst}"
        );
        let frame = *world(N, N).frame();
        for road in &net.roads {
            assert!(road.grade_max(&height, &frame) <= limit(road.class) + 1e-12);
        }
    }
    // Without the limits the lax roads do take steps their class may not: the check bites.
    let lax =
        "comfort_grade = 1.0\nmax_grade = { main = 1.0, road = 1.0, track = 1.0, trail = 1.0 }";
    let (net, height) = ridge_network(lax);
    assert!(excess(&net, &height) > 0.0);
}

#[test]
fn a_bridge_spans_the_water_in_one_edge() {
    let mut w = flat();
    river(&mut w, 30..=32, true);
    let west = town(&mut w, (32, 10));
    let east = town(&mut w, (32, 52));
    run(&mut w, "");
    let net = network(&w);
    assert_eq!(net.roads.len(), 1);
    let road = &net.roads[0];
    assert_eq!(
        (road.class, road.from, road.to),
        (RoadClass::Main, west, east)
    );
    let water = w.mask("big_river").unwrap();
    assert!(road.cells.iter().all(|&i| !water.as_slice()[i as usize]));
    assert_eq!(road.bridges.len(), 1);
    let (bank, far) = road.bridges[0];
    let pos = road.cells.iter().position(|&i| i == bank).unwrap();
    assert_eq!(
        road.cells[pos + 1],
        far,
        "the bridge is one edge of the road"
    );
    let ((rp, cp), (rq, cq)) = (rc(bank as usize), rc(far as usize));
    assert_eq!(
        (rp, cp, rq, cq),
        (rp, 29, rp, 33),
        "a perpendicular span bank to bank"
    );
    let bridges = sites(&w, "bridge");
    assert_eq!(bridges.len(), 1);
    assert_eq!(bridges[0].kind, "site");
    assert_eq!(bridges[0].props.get("road"), Some(&Prop::Road(0)));
    assert_eq!(bridges[0].props.get("span_m"), Some(&Prop::Num(400.0)));
    assert_eq!(bridges[0].props.get("deck_m"), Some(&Prop::Num(103.0)));
    assert_eq!(sites(&w, "ford").len(), 0);
    // The raster carries the road across the water, as main road.
    let roads = w.u8("roads").unwrap();
    assert!((29..=33).all(|c| roads.get(rp, c) == 4));
}

#[test]
fn the_network_connects_every_settlement_and_gate() {
    let mut w = flat();
    let places = [
        town(&mut w, (10, 10)),
        add(&mut w, "settlement", "village", (12, 50)),
        add(&mut w, "settlement", "hamlet", (50, 12)),
        add(&mut w, "settlement", "hamlet", (52, 52)),
        add(&mut w, "settlement", "village", (32, 30)),
        add(&mut w, "poi", "gate-down", (30, 58)),
        add(&mut w, "poi", "gate-up", (58, 30)),
    ];
    add(&mut w, "poi", "chime", (40, 40));
    let report = run(&mut w, "");
    assert!(warnings(&report).is_empty(), "{:?}", warnings(&report));
    let net = network(&w);
    let index = |h: FeatureHandle| places.iter().position(|&p| p == h).unwrap();
    let mut sets = UnionFind::new(places.len());
    for road in &net.roads {
        sets.union(index(road.from), index(road.to));
    }
    let root = sets.find(0);
    assert!((0..places.len()).all(|i| sets.find(i) == root));
    // Classes follow the lesser end: a hamlet's link is a track, a gate counts as a town.
    for road in &net.roads {
        let (a, b) = (index(road.from), index(road.to));
        let hamlet = [2, 3].contains(&a) || [2, 3].contains(&b);
        if hamlet {
            assert_eq!(road.class, RoadClass::Track);
        }
    }
    let roads = w.u8("roads").unwrap();
    for road in &net.roads {
        for &i in &road.cells {
            assert!(roads.as_slice()[i as usize] >= road.class.code());
        }
    }
}

#[test]
fn a_loop_joins_when_the_detour_is_long() {
    let triangle = |ty: &str, params: &str| {
        let mut w = flat();
        add(&mut w, "settlement", ty, (12, 32));
        add(&mut w, "settlement", ty, (50, 10));
        add(&mut w, "settlement", ty, (50, 54));
        run(&mut w, params);
        network(&w).roads.len()
    };
    // Around the triangle is about twice the direct way: more than 1.7, less than 2.5.
    assert_eq!(triangle("town", ""), 3);
    assert_eq!(triangle("town", "loop_detour = 2.5"), 2);
    assert_eq!(triangle("village", ""), 3);
    // Loops join only settlements of class village or higher.
    assert_eq!(triangle("hamlet", "loop_detour = 1.0"), 2);
    // Loops are routed without the merge discount, so the road laid last (the loop) mostly runs
    // on new cells even with the default discount.
    let last_on_others = |params: &str| {
        let mut w = flat();
        for at in [(12, 32), (50, 10), (50, 54)] {
            town(&mut w, at);
        }
        run(&mut w, params);
        let roads = &network(&w).roads;
        let (last, earlier) = roads.split_last().unwrap();
        let on = last
            .cells
            .iter()
            .filter(|c| earlier.iter().any(|r| r.cells.contains(c)))
            .count();
        to_f64(on) / to_f64(last.cells.len())
    };
    let looped = last_on_others("");
    assert!(looped < 0.3, "{looped}");
}

#[test]
fn fords_mark_stream_crossings() {
    let mut w = flat();
    river(&mut w, 30..=30, false);
    town(&mut w, (32, 10));
    town(&mut w, (32, 52));
    run(&mut w, "");
    let net = network(&w);
    assert_eq!(net.roads.len(), 1);
    assert_eq!(net.roads[0].bridges.len(), 0);
    assert_eq!(sites(&w, "bridge").len(), 0);
    let fords = sites(&w, "ford");
    assert_eq!(fords.len(), 1);
    let at = cell_of(&w, fords[0]);
    assert_eq!(at % N, 30, "the ford sits on the stream");
    assert!(net.roads[0].cells.contains(&u32::try_from(at).unwrap()));
    assert_eq!(fords[0].props.get("road"), Some(&Prop::Road(0)));
}

#[test]
fn fewer_than_two_places_build_no_network() -> Result<(), StageError> {
    let mut w = flat();
    town(&mut w, (32, 32));
    add(&mut w, "poi", "ruin", (10, 10));
    let before = w.value::<FeatureSet>("features").unwrap().clone();
    let report = run_stage(&mut w, &Roads, "")?;
    assert_eq!(network(&w).roads.len(), 0);
    assert!(w.u8("roads").unwrap().as_slice().iter().all(|&v| v == 0));
    assert_eq!(w.value::<FeatureSet>("features").unwrap(), &before);
    assert_eq!(report.findings.len(), 0);
    // A world without any feature list builds nothing either and writes an empty list.
    let mut w = flat();
    w.remove("features");
    run_stage(&mut w, &Roads, "")?;
    assert_eq!(network(&w).roads.len(), 0);
    assert_eq!(w.value::<FeatureSet>("features").unwrap().len(), 0);
    Ok(())
}

#[test]
fn links_blocked_by_water_are_skipped_with_a_warning() {
    let mut w = flat();
    // A lake seven cells (700 m) wide around an island of 7 x 7 cells.
    let ring = |r: usize, c: usize| {
        let d = r.abs_diff(32).max(c.abs_diff(32));
        (4..=10).contains(&d)
    };
    set_mask(&mut w, "lake", ring);
    let mainland = [
        town(&mut w, (5, 5)),
        town(&mut w, (5, 58)),
        town(&mut w, (58, 32)),
    ];
    let island = add(&mut w, "settlement", "village", (32, 32));
    let report = run(&mut w, "");
    let net = network(&w);
    assert_ne!(net.roads.len(), 0);
    for road in &net.roads {
        assert!(road.from != island && road.to != island);
        assert!(mainland.contains(&road.from) && mainland.contains(&road.to));
        assert!(road.cells.iter().all(|&i| {
            let (r, c) = rc(i as usize);
            !ring(r, c)
        }));
    }
    let found = warnings(&report);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("village-32-32"), "{}", found[0]);
    assert_eq!(sites(&w, "bridge").len(), 0);
}

#[test]
fn trails_lead_to_ruins_from_the_network() {
    let mut w = flat();
    let a = town(&mut w, (20, 10));
    let b = town(&mut w, (20, 54));
    let ruin = add(&mut w, "poi", "ruin", (40, 32));
    // A ruin in a town's cell needs no trail.
    add(&mut w, "poi", "ruin", (20, 54));
    let report = run(&mut w, "");
    assert_eq!(report.findings.len(), 0);
    let net = network(&w);
    let trails: Vec<_> = net
        .roads
        .iter()
        .filter(|r| r.class == RoadClass::Trail)
        .collect();
    assert_eq!(trails.len(), 1);
    let trail = trails[0];
    assert_eq!(trail.to, ruin);
    assert!(trail.from == a || trail.from == b, "{:?}", trail.from);
    // The trail starts on the road between the towns and runs about straight south to the
    // ruin, 20 rows away.
    let start = trail.cells[0] as usize;
    assert!(net.roads[0].cells.contains(&trail.cells[0]));
    assert_eq!(rc(start).0, 20);
    assert!(
        trail.length_m(w.frame()) < 2200.0,
        "{}",
        trail.length_m(w.frame())
    );
    assert_eq!(*trail.cells.last().unwrap() as usize, 40 * N + 32);
    assert_eq!(net.end_near(trail.cells[0]), Some(trail.from));
    assert_eq!(net.end_near(u32::MAX), None);
}

#[test]
fn places_that_cannot_be_triangulated_fail_the_stage() {
    let mut w = flat();
    town(&mut w, (10, 10));
    let mut set = w.value::<FeatureSet>("features").unwrap().clone();
    set.add(Feature {
        kind: "settlement",
        ty: "town".to_owned(),
        geometry: Geometry::Point([f64::NAN, 50.0]),
        props: BTreeMap::new(),
    });
    w.insert("features", Layer::Value(Box::new(set))).unwrap();
    let err = run_stage(&mut w, &Roads, "").unwrap_err();
    assert!(
        matches!(&err, StageError::Failed { stage, .. } if stage == "roads"),
        "{err}"
    );
}

#[test]
fn an_unreachable_ruin_gets_no_trail_and_a_warning() {
    let mut w = flat();
    set_mask(&mut w, "lake", |r, c| {
        let d = r.abs_diff(40).max(c.abs_diff(32));
        (2..=8).contains(&d)
    });
    town(&mut w, (5, 5));
    town(&mut w, (5, 58));
    add(&mut w, "poi", "ruin", (40, 32));
    let report = run(&mut w, "");
    assert!(
        network(&w)
            .roads
            .iter()
            .all(|r| r.class != RoadClass::Trail)
    );
    let found = warnings(&report);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("ruin"), "{}", found[0]);
}

#[test]
fn roads_report_lengths_and_crossings() {
    let mut w = flat();
    river(&mut w, 30..=32, true);
    town(&mut w, (32, 10));
    town(&mut w, (32, 52));
    let params = Roads.params(&table("")).unwrap();
    let report = run(&mut w, "");
    let metrics = Stage::metrics(&Roads, &w, params.as_ref(), &report);
    let mut names = Stage::metric_names(&Roads, params.as_ref());
    names.sort_unstable();
    let keys: Vec<&str> = metrics.0.keys().map(String::as_str).collect();
    assert_eq!(names, keys);
    let road = &network(&w).roads[0];
    let km = metrics.0["main_km"];
    assert!((km - road.length_m(w.frame()) / 1000.0).abs() < 1e-12);
    assert!((4.1..4.6).contains(&km), "{km}");
    assert_eq!(metrics.0["bridges"], 1.0);
    assert_eq!(metrics.0["fords"], 0.0);
    assert_eq!(metrics.0["unreached"], 0.0);
    assert_eq!(metrics.0["trail_km"], 0.0);
}

#[test]
fn the_same_world_gives_the_same_network() {
    let build = || {
        let mut w = ridge_and_river();
        town(&mut w, (32, 6));
        town(&mut w, (30, 34));
        add(&mut w, "settlement", "village", (20, 56));
        add(&mut w, "poi", "ruin", (10, 20));
        run(&mut w, "");
        (
            network(&w).clone(),
            w.u8("roads").unwrap().clone(),
            w.value::<FeatureSet>("features").unwrap().clone(),
        )
    };
    assert_eq!(build(), build());
}

/// Runs `roads` over `w` with a cancel token and a progress sink.
fn run_watched(
    w: &mut World,
    cancel: &CancelToken,
) -> (Result<StageReport, StageError>, Vec<Progress>) {
    let params = Roads.params(&table("")).unwrap();
    let mut events = Vec::new();
    let mut sink = |p: &Progress| events.push(p.clone());
    let mut ctx = StageContext::new(w, &Roads, params.as_ref(), "roads#0", 11, cancel, &mut sink);
    let result = Roads.run(&mut ctx, params.as_ref());
    (result, events)
}

#[test]
fn routing_reports_progress_per_link_and_stops_on_cancel() {
    let mut w = flat();
    town(&mut w, (12, 32));
    town(&mut w, (50, 10));
    town(&mut w, (50, 54));
    add(&mut w, "poi", "ruin", (32, 32));
    let (result, events) = run_watched(&mut w, &CancelToken::new());
    assert!(result.is_ok());
    assert!(events.iter().all(|e| e.stage == "roads"));
    let last = events.last().unwrap();
    assert_eq!(last.done, last.total);
    // Three candidates costed, three links laid, one trail.
    assert_eq!(last.total, 3 + 3 + 1);
    assert!(events.windows(2).all(|p| p[0].done <= p[1].done));
    assert!(events.len() >= 4);
    let cancel = CancelToken::new();
    cancel.cancel();
    let mut w2 = flat();
    town(&mut w2, (12, 32));
    town(&mut w2, (50, 10));
    let (result, _) = run_watched(&mut w2, &cancel);
    assert_eq!(result.unwrap_err(), StageError::Cancelled);
}

#[test]
fn params_are_validated_and_unknown_keys_refused() {
    let params = Roads.params(&table("")).unwrap();
    let p = params
        .as_any()
        .downcast_ref::<super::RoadsParams>()
        .unwrap();
    assert_eq!(p, &super::RoadsParams::default());
    for bad in [
        "comfort_grade = 0.0",
        "max_grade = { main = 0.0, road = 0.1, track = 0.15, trail = 0.25 }",
        "max_grade = { main = 0.08, road = 0.1, track = 0.15, trail = inf }",
        "forest_cost = -1.0",
        "ford_cost = nan",
        "bridge_cost = -2.0",
        "angle_penalty = -0.5",
        "deck_cost = inf",
        "max_bridge_span_m = -1.0",
        "clearance_m = -3.0",
        "loop_detour = 0.9",
        "merge_discount = 0.0",
        "merge_discount = 1.5",
        "width_m = { main = 6.0, road = 4.5, track = 0.0, trail = 1.5 }",
    ] {
        let err = Roads.params(&table(bad)).unwrap_err();
        let key = bad.split([' ', '=']).next().unwrap();
        assert!(
            err.parameter.as_deref().is_some_and(|p| p.starts_with(key)),
            "{bad}: {err}"
        );
    }
    for unknown in [
        "comfort = 0.1",
        "max_grade = { main = 0.08, road = 0.1, track = 0.15, trail = 0.25, lane = 0.3 }",
        "width_m = { main = 6.0, road = 4.5, track = 3.0, trail = 1.5, path = 1.0 }",
    ] {
        let err = Roads.params(&table(unknown)).unwrap_err();
        let key = ["comfort", "lane", "path"]
            .into_iter()
            .find(|k| unknown.contains(k))
            .unwrap();
        assert!(err.to_string().contains(key), "{unknown}: {err}");
    }
    // A class a per-class table leaves out keeps its default.
    let params = Roads
        .params(&table(
            "max_grade = { main = 0.06 }
width_m = { trail = 1.0 }",
        ))
        .unwrap();
    let p = params
        .as_any()
        .downcast_ref::<super::RoadsParams>()
        .unwrap();
    let defaults = super::RoadsParams::default();
    assert_eq!(
        p.max_grade,
        super::MaxGrade {
            main: 0.06,
            ..defaults.max_grade
        }
    );
    assert_eq!(
        p.width_m,
        super::WidthM {
            trail: 1.0,
            ..defaults.width_m
        }
    );
}

#[test]
fn roads_is_a_registered_terrain_stage() {
    let registry = crate::stage::Registry::builtin();
    let stage = registry.get("roads").unwrap();
    let params = stage.params(&table("")).unwrap();
    let writes: Vec<&str> = stage
        .writes(params.as_ref())
        .iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(writes, ["road_network", "roads", "features"]);
    let reads: Vec<&str> = stage
        .reads(params.as_ref())
        .iter()
        .map(|s| s.name)
        .collect();
    for name in [
        "height",
        "vegetation",
        "river",
        "big_river",
        "lake",
        "flow_dir",
        "features",
    ] {
        assert!(reads.contains(&name), "{name}");
    }
}
