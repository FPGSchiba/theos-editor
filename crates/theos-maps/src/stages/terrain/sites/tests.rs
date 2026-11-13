//! Tests of the `sites` stage on synthetic 64² worlds of 100 m cells.

use std::collections::BTreeMap;

use super::{Sites, SitesParams};
use crate::features::{Feature, FeatureHandle, FeatureSet, Geometry, Prop};
use crate::grid::Grid;
use crate::kernels::num::to_f64;
use crate::stage::{Severity, Stage, StageReport};
use crate::stages::terrain::hydrology::WaterThresholds;
use crate::stages::terrain::roads::{Road, RoadClass, RoadNetwork, RoadsParams};
use crate::stages::terrain::support::{run_stage, table, world};
use crate::world::{Layer, World};

const N: usize = 64;
const RIVER_Q: f64 = 0.15;
const LIMESTONE: u8 = 3;
const SANDSTONE: u8 = 4;
/// No counts of the scattered sites: a test asks for the ones it needs.
const NO_SCATTER: &str = "counts = { watchtower = 0, cave = 0, camp = 0 }\n";

fn grid<T: Copy>(f: impl Fn(usize, usize) -> T) -> Grid<T> {
    Grid::from_vec(N, N, (0..N * N).map(|i| f(i / N, i % N)).collect()).unwrap()
}

fn at(r: usize, c: usize) -> usize {
    r * N + c
}

/// A flat world with no water, no road and no feature, as after `roads`.
fn base() -> World {
    let mut w = world(N, N);
    w.insert("height", Layer::F64(grid(|_, _| 100.0))).unwrap();
    w.insert("slope", Layer::F64(grid(|_, _| 0.02))).unwrap();
    w.insert("rock", Layer::U8(grid(|_, _| SANDSTONE))).unwrap();
    w.insert("vegetation", Layer::F64(grid(|_, _| 0.0)))
        .unwrap();
    for name in ["river", "big_river", "lake"] {
        w.insert(name, Layer::Mask(grid(|_, _| false))).unwrap();
    }
    w.insert("flow_dir", Layer::U8(grid(|_, _| 0))).unwrap();
    w.insert("discharge", Layer::F64(grid(|_, _| 0.0))).unwrap();
    let thresholds = WaterThresholds {
        river_q: RIVER_Q,
        big_river_q: 2.0,
    };
    w.insert("water_thresholds", Layer::Value(Box::new(thresholds)))
        .unwrap();
    w.insert("features", Layer::Value(Box::new(FeatureSet::default())))
        .unwrap();
    set_network(&mut w, Vec::new());
    w
}

/// Puts `roads` into the world as the result of `roads`, and their cells on the raster.
fn set_network(w: &mut World, roads: Vec<Road>) {
    let mut raster = Grid::filled(N, N, 0_u8);
    for road in &roads {
        for &c in &road.cells {
            let cell = &mut raster.as_mut_slice()[usize::try_from(c).unwrap()];
            *cell = (*cell).max(road.class.code());
        }
    }
    let net = RoadNetwork {
        roads,
        params: RoadsParams::default(),
    };
    w.insert("road_network", Layer::Value(Box::new(net)))
        .unwrap();
    w.insert("roads", Layer::U8(raster)).unwrap();
}

fn add(w: &mut World, kind: &'static str, ty: &str, (r, c): (usize, usize)) -> FeatureHandle {
    add_with(w, kind, ty, (r, c), &[])
}

fn add_with(
    w: &mut World,
    kind: &'static str,
    ty: &str,
    (r, c): (usize, usize),
    props: &[(&str, Prop)],
) -> FeatureHandle {
    let at = w.frame().centre_m(r, c);
    let mut set = w.value::<FeatureSet>("features").unwrap().clone();
    let h = set.add(Feature {
        kind,
        ty: ty.to_owned(),
        geometry: Geometry::Point(at),
        props: props
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect::<BTreeMap<_, _>>(),
    });
    w.insert("features", Layer::Value(Box::new(set))).unwrap();
    h
}

fn deposit(w: &mut World, ty: &str, cell: (usize, usize), richness: f64) -> FeatureHandle {
    add_with(
        w,
        "resource",
        ty,
        cell,
        &[("richness", Prop::Num(richness))],
    )
}

/// A road of `class` along `cells`, from the feature `from` to the feature `to`.
fn road(class: RoadClass, cells: &[usize], from: FeatureHandle, to: FeatureHandle) -> Road {
    Road {
        class,
        cells: cells.iter().map(|&c| u32::try_from(c).unwrap()).collect(),
        from,
        to,
        bridges: Vec::new(),
    }
}

/// The cells of the row `r` from column `from` to column `to`.
fn row(r: usize, from: usize, to: usize) -> Vec<usize> {
    (from..=to).map(|c| at(r, c)).collect()
}

fn run(w: &mut World, params: &str) -> StageReport {
    run_stage(w, &Sites, params).unwrap()
}

fn features(w: &World) -> &FeatureSet {
    w.value::<FeatureSet>("features").unwrap()
}

fn network(w: &World) -> &RoadNetwork {
    w.value::<RoadNetwork>("road_network").unwrap()
}

/// The sites of type `ty` with their handles.
fn sites<'a>(w: &'a World, ty: &str) -> Vec<(FeatureHandle, &'a Feature)> {
    features(w)
        .of_kind("site")
        .filter(|(_, f)| f.ty == ty)
        .collect()
}

fn cell_of(w: &World, f: &Feature) -> usize {
    let Geometry::Point([x, y]) = f.geometry else {
        panic!("a site is a point")
    };
    let (row, col) = w.frame().cell_at_m(x, y);
    at(row, col)
}

fn metres(a: usize, b: usize) -> f64 {
    let (dr, dc) = ((a / N).abs_diff(b / N), (a % N).abs_diff(b % N));
    100.0 * libm::sqrt(to_f64(dr * dr + dc * dc))
}

/// A river in column 30 with `discharge` on it.
fn with_river(w: &mut World, discharge: f64) {
    w.insert("river", Layer::Mask(grid(|_, c| c == 30)))
        .unwrap();
    w.insert(
        "discharge",
        Layer::F64(grid(|_, c| if c == 30 { discharge } else { 0.0 })),
    )
    .unwrap();
}

fn mill_world(factor: f64) -> World {
    let mut w = base();
    with_river(&mut w, factor * RIVER_Q);
    add(&mut w, "settlement", "town", (32, 32));
    w
}

// The spec's ruling 45: a mill needs `mill_q_factor` (2.7) times `river_q`.
#[test]
fn mills_need_enough_discharge() {
    let mut weak = mill_world(2.6);
    run(&mut weak, NO_SCATTER);
    assert_eq!(sites(&weak, "mill").len(), 0);

    let mut strong = mill_world(2.8);
    let report = run(&mut strong, NO_SCATTER);
    let mills = sites(&strong, "mill");
    assert_eq!(mills.len(), 1);
    let (_, mill) = mills[0];
    // The nearest river cell to the town, two cells west of it.
    assert_eq!(cell_of(&strong, mill), at(32, 30));
    assert_eq!(mill.props.get("serves"), Some(&Prop::Ref(FeatureHandle(0))));
    assert!((report.values["mill"] - 1.0).abs() < 1e-12);

    // The factor is a parameter: 2.6 times is enough when the factor is 2.5.
    let mut custom = mill_world(2.6);
    run(&mut custom, &format!("mill_q_factor = 2.5\n{NO_SCATTER}"));
    assert_eq!(sites(&custom, "mill").len(), 1);
}

#[test]
fn mills_serve_towns_and_villages_within_reach() {
    let mut w = base();
    with_river(&mut w, 1.0);
    // A hamlet 200 m from the river, a village 2 km from it, a town 3.2 km from it.
    add(&mut w, "settlement", "hamlet", (10, 32));
    add(&mut w, "settlement", "village", (32, 50));
    add(&mut w, "settlement", "town", (50, 62));
    run(&mut w, NO_SCATTER);
    assert_eq!(sites(&w, "mill").len(), 0);

    // A village 1.4 km from the river is served, at the nearest river cell.
    let mut w = base();
    with_river(&mut w, 1.0);
    let village = add(&mut w, "settlement", "village", (32, 44));
    run(&mut w, NO_SCATTER);
    let mills = sites(&w, "mill");
    assert_eq!(mills.len(), 1);
    assert_eq!(mills[0].1.props.get("serves"), Some(&Prop::Ref(village)));
    assert_eq!(cell_of(&w, mills[0].1), at(32, 30));
}

fn long_link_world(class: RoadClass, to: usize) -> World {
    let mut w = base();
    let a = add(&mut w, "settlement", "town", (4, 4));
    let b = add(&mut w, "settlement", "town", (to, to));
    let cells: Vec<usize> = (4..=to).map(|i| at(i, i)).collect();
    set_network(&mut w, vec![road(class, &cells, a, b)]);
    w
}

#[test]
fn inns_sit_midway_on_long_links() {
    // A diagonal of 54 steps is 7.6 km.
    let mut w = long_link_world(RoadClass::Road, 58);
    let report = run(&mut w, NO_SCATTER);
    let inns = sites(&w, "inn");
    assert_eq!(inns.len(), 1);
    let (_, inn) = inns[0];
    assert_eq!(cell_of(&w, inn), at(31, 31));
    assert_eq!(inn.props.get("road"), Some(&Prop::Road(0)));
    assert!((report.values["inn"] - 1.0).abs() < 1e-12);

    // A main road counts, a track or a short link does not.
    let mut main = long_link_world(RoadClass::Main, 58);
    run(&mut main, NO_SCATTER);
    assert_eq!(sites(&main, "inn").len(), 1);
    let mut track = long_link_world(RoadClass::Track, 58);
    run(&mut track, NO_SCATTER);
    assert_eq!(sites(&track, "inn").len(), 0);
    // 40 steps are 5.7 km.
    let mut short = long_link_world(RoadClass::Road, 44);
    run(&mut short, NO_SCATTER);
    assert_eq!(sites(&short, "inn").len(), 0);
}

#[test]
fn an_inn_keeps_away_from_settlements() {
    let mut w = long_link_world(RoadClass::Road, 58);
    // A hamlet 1.4 km from the midpoint.
    add(&mut w, "settlement", "hamlet", (31, 45));
    run(&mut w, NO_SCATTER);
    assert_eq!(sites(&w, "inn").len(), 0);
}

/// Two towns on a main road along row 32, and a deposit of `ty`.
fn spur_world(ty: &str, richness: f64, at_cell: (usize, usize)) -> (World, FeatureHandle) {
    let mut w = base();
    let a = add(&mut w, "settlement", "town", (32, 8));
    let b = add(&mut w, "settlement", "town", (32, 56));
    set_network(&mut w, vec![road(RoadClass::Main, &row(32, 8, 56), a, b)]);
    let d = deposit(&mut w, ty, at_cell, richness);
    (w, d)
}

#[test]
fn spurs_join_the_network() {
    let (mut w, d) = spur_world("iron-ore", 0.9, (20, 30));
    let report = run(&mut w, NO_SCATTER);
    let mines = sites(&w, "mine");
    assert_eq!(mines.len(), 1);
    let (mine_handle, mine) = mines[0];
    assert_eq!(cell_of(&w, mine), at(20, 30));
    assert_eq!(mine.props.get("deposit"), Some(&Prop::Ref(d)));
    assert_eq!(mine.props.get("road"), Some(&Prop::Road(1)));
    let net = network(&w);
    assert_eq!(net.roads.len(), 2, "the main road and one spur");
    let spur = &net.roads[1];
    assert_eq!(spur.class, RoadClass::Track);
    assert_eq!(spur.to, mine_handle);
    // It starts on the main road, west of the middle, and ends at the mine.
    let first = usize::try_from(spur.cells[0]).unwrap();
    assert_eq!(first / N, 32);
    assert_eq!(spur.from, FeatureHandle(0));
    assert_eq!(
        spur.cells.last().copied(),
        Some(u32::try_from(at(20, 30)).unwrap())
    );
    // The raster carries the track on its cells, and the main road keeps its class.
    let raster = w.u8("roads").unwrap();
    for &c in &spur.cells[1..] {
        assert!(raster.as_slice()[usize::try_from(c).unwrap()] >= RoadClass::Track.code());
    }
    assert_eq!(raster.as_slice()[at(32, 30)], RoadClass::Main.code());
    assert!((report.values["mine"] - 1.0).abs() < 1e-12);
    assert!((report.values["spurs"] - 1.0).abs() < 1e-12);
}

#[test]
fn deposits_give_their_sites_and_poor_ones_none() {
    for (ty, site, class) in [
        ("copper-ore", "mine", RoadClass::Track),
        ("silver-ore", "mine", RoadClass::Track),
        ("stone", "quarry", RoadClass::Track),
        ("timber", "lumber-camp", RoadClass::Track),
        ("resonance-crystal", "shrine", RoadClass::Trail),
    ] {
        let (mut w, d) = spur_world(ty, 0.6, (20, 30));
        run(&mut w, NO_SCATTER);
        let found = sites(&w, site);
        assert_eq!(found.len(), 1, "{ty}");
        assert_eq!(found[0].1.props.get("deposit"), Some(&Prop::Ref(d)), "{ty}");
        assert_eq!(network(&w).roads[1].class, class, "{ty}");
    }
    // Clay is not mined; a poor deposit gets no site (0.3.0's `min_richness`).
    let (mut w, _) = spur_world("clay", 0.9, (20, 30));
    run(&mut w, NO_SCATTER);
    assert_eq!(features(&w).of_kind("site").count(), 0);
    let (mut w, _) = spur_world("iron-ore", 0.3, (20, 30));
    run(&mut w, NO_SCATTER);
    assert_eq!(features(&w).of_kind("site").count(), 0);
    let (mut w, _) = spur_world("iron-ore", 0.3, (20, 30));
    run(&mut w, &format!("min_richness = 0.25\n{NO_SCATTER}"));
    assert_eq!(sites(&w, "mine").len(), 1);
}

#[test]
fn a_site_on_a_road_needs_no_spur() {
    let (mut w, _) = spur_world("stone", 0.9, (32, 30));
    run(&mut w, NO_SCATTER);
    let found = sites(&w, "quarry");
    assert_eq!(found[0].1.props.get("road"), Some(&Prop::Road(0)));
    assert_eq!(network(&w).roads.len(), 1);
}

#[test]
fn a_site_no_road_can_reach_gets_a_warning_and_no_road() {
    let (mut w, _) = spur_world("iron-ore", 0.9, (20, 30));
    // A ring of lake, wider than a bridge may span, around the deposit.
    let ring = |r: usize, c: usize| {
        let ring = r.abs_diff(20).max(c.abs_diff(30));
        (3..=10).contains(&ring)
    };
    w.insert("lake", Layer::Mask(grid(ring))).unwrap();
    let report = run(&mut w, NO_SCATTER);
    let found = sites(&w, "mine");
    assert_eq!(found.len(), 1);
    assert!(!found[0].1.props.contains_key("road"));
    assert_eq!(network(&w).roads.len(), 1);
    let warnings: Vec<&str> = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .map(|f| f.message.as_str())
        .collect();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("mine"), "{warnings:?}");
}

#[test]
fn without_a_road_sites_get_no_spur() {
    let mut w = base();
    deposit(&mut w, "iron-ore", (20, 30), 0.9);
    let report = run(&mut w, NO_SCATTER);
    let found = sites(&w, "mine");
    assert_eq!(found.len(), 1);
    assert!(!found[0].1.props.contains_key("road"));
    assert_eq!(network(&w).roads.len(), 0);
    assert_eq!(report.findings.len(), 0);
}

#[test]
fn watchtowers_stand_on_prominent_ground_near_main_roads() {
    let (mut w, _) = spur_world("clay", 0.0, (5, 5));
    // A hill of 80 m 6 cells north of the road, and a taller one 26 cells away from it.
    let hill = |r: usize, c: usize, (hr, hc): (f64, f64), h: f64| {
        let (dr, dc) = (to_f64(r) - hr, to_f64(c) - hc);
        h * libm::exp(-(dr * dr + dc * dc) / 8.0)
    };
    let height =
        grid(|r, c| 100.0 + hill(r, c, (26.0, 30.0), 80.0) + hill(r, c, (6.0, 30.0), 150.0));
    w.insert("height", Layer::F64(height)).unwrap();
    run(&mut w, "counts = { watchtower = 1, cave = 0, camp = 0 }\n");
    let towers = sites(&w, "watchtower");
    assert_eq!(towers.len(), 1);
    let cell = cell_of(&w, towers[0].1);
    // On the near hill, 150 m to 900 m from the road (0.3.0's band), not on the far one.
    assert!((24..=28).contains(&(cell / N)), "{cell}");
    let d = metres(cell, at(32, cell % N));
    assert!((150.0..900.0).contains(&d), "{d}");
}

#[test]
fn caves_open_in_steep_limestone() {
    let mut w = base();
    let rock = |r: usize, c: usize| {
        if (20..24).contains(&r) && (40..44).contains(&c) {
            LIMESTONE
        } else {
            SANDSTONE
        }
    };
    w.insert("rock", Layer::U8(grid(rock))).unwrap();
    w.insert("slope", Layer::F64(grid(|_, _| 0.7))).unwrap();
    run(&mut w, "counts = { watchtower = 0, cave = 1, camp = 0 }\n");
    let caves = sites(&w, "cave");
    assert_eq!(caves.len(), 1);
    let cell = cell_of(&w, caves[0].1);
    assert!((20..24).contains(&(cell / N)) && (40..44).contains(&(cell % N)));
}

#[test]
fn camps_keep_away_from_roads_and_settlements() {
    let (mut w, _) = spur_world("clay", 0.0, (5, 5));
    w.insert("vegetation", Layer::F64(grid(|_, _| 0.8)))
        .unwrap();
    run(&mut w, "counts = { watchtower = 0, cave = 0, camp = 3 }\n");
    let camps = sites(&w, "camp");
    assert!(!camps.is_empty() && camps.len() <= 3);
    for (_, camp) in camps {
        let cell = cell_of(&w, camp);
        assert!(metres(cell, at(32, cell % N)) > 600.0);
        for town in [at(32, 8), at(32, 56)] {
            assert!(metres(cell, town) >= 3000.0, "{cell}");
        }
    }
}

#[test]
fn sites_keep_off_the_edge() {
    let mut w = base();
    w.insert("rock", Layer::U8(grid(|_, _| LIMESTONE))).unwrap();
    w.insert("slope", Layer::F64(grid(|_, _| 0.9))).unwrap();
    w.insert("vegetation", Layer::F64(grid(|_, _| 0.9)))
        .unwrap();
    run(
        &mut w,
        "counts = { watchtower = 0, cave = 12, camp = 12 }\n",
    );
    let margin = 2; // round(0.025 x 64 cells)
    let mut seen = 0;
    for ty in ["cave", "camp"] {
        for (_, f) in sites(&w, ty) {
            let cell = cell_of(&w, f);
            let (r, c) = (cell / N, cell % N);
            assert!(r >= margin && r < N - margin && c >= margin && c < N - margin);
            seen += 1;
        }
    }
    assert!(seen > 2);
}

#[test]
fn a_run_twice_is_the_same() {
    let make = || {
        let (mut w, _) = spur_world("iron-ore", 0.9, (20, 30));
        w.insert("vegetation", Layer::F64(grid(|_, _| 0.5)))
            .unwrap();
        run(&mut w, "");
        (
            network(&w).clone(),
            w.u8("roads").unwrap().clone(),
            features(&w).clone(),
        )
    };
    assert_eq!(make(), make());
}

#[test]
fn params_default_and_refuse_what_is_wrong() {
    let params = Sites.params(&table("")).unwrap();
    let p = params.as_any().downcast_ref::<SitesParams>().unwrap();
    assert_eq!(p, &SitesParams::default());
    assert!((p.mill_q_factor - 2.7).abs() < 1e-12);
    assert!((p.edge_margin - 0.025).abs() < 1e-12);
    assert!((p.min_richness - 0.45).abs() < 1e-12);
    assert_eq!(
        (p.counts.watchtower, p.counts.cave, p.counts.camp),
        (3, 2, 3)
    );
    // A counts table keeps the counts it leaves out.
    let params = Sites.params(&table("counts = { cave = 5 }")).unwrap();
    let p = params.as_any().downcast_ref::<SitesParams>().unwrap();
    assert_eq!(
        (p.counts.watchtower, p.counts.cave, p.counts.camp),
        (3, 5, 3)
    );
    for (toml, name) in [
        ("bogus = 1", "bogus"),
        ("counts = { dragon = 1 }", "dragon"),
        ("mill_q_factor = 0.0", "mill_q_factor"),
        ("mill_q_factor = -1.0", "mill_q_factor"),
        ("mill_q_factor = inf", "mill_q_factor"),
        ("edge_margin = 0.5", "edge_margin"),
        ("edge_margin = -0.1", "edge_margin"),
        ("min_richness = 1.5", "min_richness"),
        ("min_richness = -0.5", "min_richness"),
    ] {
        let err = Sites.params(&table(toml)).unwrap_err();
        assert_eq!(err.stage, "sites", "{toml}");
        assert!(err.to_string().contains(name), "{toml}: {err}");
    }
}

#[test]
fn the_stage_is_registered_with_its_layers() {
    let registry = crate::stage::Registry::builtin();
    let stage = registry.get("sites").unwrap();
    let params = stage.params(&table("")).unwrap();
    let reads: Vec<&str> = stage
        .reads(params.as_ref())
        .iter()
        .map(|l| l.name)
        .collect();
    for name in [
        "road_network",
        "roads",
        "discharge",
        "water_thresholds",
        "features",
    ] {
        assert!(reads.contains(&name), "reads {name}");
    }
    let writes: Vec<&str> = stage
        .writes(params.as_ref())
        .iter()
        .map(|l| l.name)
        .collect();
    assert_eq!(writes, ["features", "road_network", "roads"]);
}
