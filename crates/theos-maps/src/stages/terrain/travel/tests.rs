//! Tests of the `travel` stage on synthetic 64² worlds of 100 m cells.

use super::{Travel, TravelParams, tobler_ms};
use crate::features::{Feature, FeatureHandle, FeatureSet, Geometry};
use crate::grid::Grid;
use crate::kernels::num::to_f64;
use crate::stage::{Registry, Stage, StageReport};
use crate::stages::terrain::roads::{Road, RoadClass, RoadNetwork, RoadsParams};
use crate::stages::terrain::support::{run_stage, table, world};
use crate::world::{Layer, World};
use std::collections::BTreeMap;

const N: usize = 64;

fn grid<T: Copy>(f: impl Fn(usize, usize) -> T) -> Grid<T> {
    Grid::from_vec(N, N, (0..N * N).map(|i| f(i / N, i % N)).collect()).unwrap()
}

fn at(r: usize, c: usize) -> usize {
    r * N + c
}

fn flat() -> World {
    let mut w = world(N, N);
    w.insert("height", Layer::F64(grid(|_, _| 100.0))).unwrap();
    for name in ["river", "big_river", "lake"] {
        w.insert(name, Layer::Mask(grid(|_, _| false))).unwrap();
    }
    w.insert("features", Layer::Value(Box::new(FeatureSet::default())))
        .unwrap();
    w
}

fn chime(w: &mut World, (r, c): (usize, usize)) -> FeatureHandle {
    let mut set = w.value::<FeatureSet>("features").unwrap().clone();
    let h = set.add(Feature {
        kind: "poi",
        ty: "chime".to_owned(),
        geometry: Geometry::Point(w.frame().centre_m(r, c)),
        props: BTreeMap::new(),
    });
    w.insert("features", Layer::Value(Box::new(set))).unwrap();
    h
}

fn road_along_row(w: &mut World, r: usize, from: usize, to: usize) {
    let cells = (from..=to)
        .map(|c| u32::try_from(at(r, c)).unwrap())
        .collect();
    let net = RoadNetwork {
        roads: vec![Road {
            class: RoadClass::Road,
            cells,
            from: FeatureHandle(0),
            to: FeatureHandle(0),
            bridges: Vec::new(),
        }],
        params: RoadsParams::default(),
    };
    w.insert("road_network", Layer::Value(Box::new(net)))
        .unwrap();
}

fn run(w: &mut World, params: &str) -> StageReport {
    run_stage(w, &Travel, params).unwrap()
}

fn reach(w: &World, r: usize, c: usize) -> f64 {
    w.f64("reach_min").unwrap().as_slice()[at(r, c)]
}

#[test]
fn tobler_speed_peaks_on_a_gentle_downhill() {
    let peak = tobler_ms(-0.05);
    // 6 km/h at the peak.
    assert!((peak - 6.0 / 3.6).abs() < 1e-12);
    for s in [-0.2, -0.1, -0.06, -0.049, -0.04, 0.0, 0.05, 0.3] {
        assert!(tobler_ms(s) < peak, "{s}");
    }
    // Symmetric about the peak.
    assert!((tobler_ms(-0.02) - tobler_ms(-0.08)).abs() < 1e-12);
    // Flat ground: 6 exp(-0.175) = 5.037 km/h.
    assert!((tobler_ms(0.0) - 6.0 * libm::exp(-0.175) / 3.6).abs() < 1e-12);
}

#[test]
fn roads_are_faster_than_open_ground() {
    let walk = |with_road: bool| {
        let mut w = flat();
        chime(&mut w, (32, 2));
        if with_road {
            road_along_row(&mut w, 32, 2, 60);
        }
        run(&mut w, "");
        reach(&w, 32, 60)
    };
    let (off, on) = (walk(false), walk(true));
    // 58 cells of 100 m on the flat: off the road at 0.6 of Tobler's speed, on it at
    // max(speed, 1.2 m/s) x 1.3.
    let flat_ms = tobler_ms(0.0);
    assert!(
        (off - 5800.0 / (flat_ms * 0.6) / 60.0).abs() < 1e-6,
        "{off}"
    );
    assert!(
        (on - 5800.0 / (flat_ms.max(1.2) * 1.3) / 60.0).abs() < 1e-6,
        "{on}"
    );
    assert!(on < off);
}

#[test]
fn reach_is_minus_one_without_chimes() {
    let mut w = flat();
    let report = run(&mut w, "");
    assert!(
        w.f64("reach_min")
            .unwrap()
            .as_slice()
            .iter()
            .all(|&v| v.to_bits() == (-1.0_f64).to_bits())
    );
    assert!(!report.values.contains_key("chime_to_chime_min"));
    assert!(report.values["floor_pct_within_15_min"].abs() < 1e-12);
}

#[test]
fn the_chime_walk_is_reported_with_two_chimes() {
    let mut w = flat();
    chime(&mut w, (32, 10));
    let one = run(&mut w, "");
    assert!(!one.values.contains_key("chime_to_chime_min"));
    chime(&mut w, (32, 40));
    let report = run(&mut w, "");
    // 30 cells at the off-road speed.
    let want = 3000.0 / (tobler_ms(0.0) * 0.6) / 60.0;
    assert!((report.values["chime_to_chime_min"] - want).abs() < 1e-6);
    assert!(reach(&w, 32, 10).abs() < 1e-12 && reach(&w, 32, 40).abs() < 1e-12);
    // The midpoint is reached from the nearer chime.
    assert!(reach(&w, 32, 25) < want);
    let pct = report.values["floor_pct_within_15_min"];
    let cells = w
        .f64("reach_min")
        .unwrap()
        .as_slice()
        .iter()
        .filter(|&&v| (0.0..=15.0).contains(&v))
        .count();
    assert!((pct - 100.0 * to_f64(cells) / to_f64(N * N)).abs() < 1e-9);
    assert!(pct > 0.0 && pct < 100.0);
}

#[test]
fn fords_and_lakes_cost_time() {
    let walk = |wet: &str| {
        let mut w = flat();
        chime(&mut w, (32, 2));
        if wet == "river" {
            w.insert("river", Layer::Mask(grid(|_, c| c == 30)))
                .unwrap();
        }
        if wet == "lake" {
            w.insert("lake", Layer::Mask(grid(|_, c| c == 30))).unwrap();
        }
        run(&mut w, "");
        reach(&w, 32, 40)
    };
    let dry = walk("");
    assert!(
        walk("river") >= dry + 2.0 - 1e-9,
        "one ford cell costs 120 s"
    );
    assert!(
        walk("lake") >= dry + 15.0 - 1e-9,
        "a step over the lake costs 900 s"
    );
}

#[test]
fn params_are_checked_and_the_stage_registered() {
    let p = TravelParams::default();
    assert!((p.ford_penalty_s - 120.0).abs() < 1e-12 && (p.lake_penalty_s - 900.0).abs() < 1e-12);
    assert!((p.road_speedup - 1.3).abs() < 1e-12 && (p.off_path_factor - 0.6).abs() < 1e-12);
    assert!((p.road_min_speed_ms - 1.2).abs() < 1e-12);
    for bad in [
        "bogus = 1",
        "ford_penalty_s = -1.0",
        "lake_penalty_s = nan",
        "road_speedup = 0.0",
        "off_path_factor = 0.0",
        "road_min_speed_ms = -1.0",
    ] {
        assert!(Travel.params(&table(bad)).is_err(), "{bad}");
    }
    let registry = Registry::builtin();
    let stage = registry.get("travel").unwrap();
    let params = stage.params(&table("")).unwrap();
    let writes: Vec<&str> = stage
        .writes(params.as_ref())
        .iter()
        .map(|l| l.name)
        .collect();
    assert_eq!(writes, ["reach_min"]);
}

#[test]
fn a_bridge_is_walked_at_road_speed_without_a_ford() {
    let mut w = flat();
    chime(&mut w, (32, 2));
    w.insert("river", Layer::Mask(grid(|_, c| c == 30)))
        .unwrap();
    w.insert("big_river", Layer::Mask(grid(|_, c| c == 30)))
        .unwrap();
    let cells = (2..=29)
        .chain(31..=60)
        .map(|c| u32::try_from(at(32, c)).unwrap())
        .collect();
    let net = RoadNetwork {
        roads: vec![Road {
            class: RoadClass::Road,
            cells,
            from: FeatureHandle(0),
            to: FeatureHandle(0),
            bridges: vec![(
                u32::try_from(at(32, 29)).unwrap(),
                u32::try_from(at(32, 31)).unwrap(),
            )],
        }],
        params: RoadsParams::default(),
    };
    w.insert("road_network", Layer::Value(Box::new(net)))
        .unwrap();
    run(&mut w, "");
    let want = 3800.0 / (tobler_ms(0.0).max(1.2) * 1.3) / 60.0;
    assert!(
        (reach(&w, 32, 40) - want).abs() < 1e-6,
        "{}",
        reach(&w, 32, 40)
    );
}
