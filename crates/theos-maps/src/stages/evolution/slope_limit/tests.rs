//! `slope_limit` on random steep terrain with mixed critical slopes, on terraced and towered
//! terrain and on a whole over-steep flank: physics test 10 (spec §5.14), the limited surface
//! against a brute-force fixed point, and the stage's declaration.

use super::SlopeLimit;
use super::sweep::limited;
use crate::grid::{D8, Grid};
use crate::kernels::num::{max, to_f64};
use crate::kernels::routing::d8_distance;
use crate::rng::StageRng;
use crate::stage::{LayerSpec, Stage, StageError, StageReport};
use crate::stages::evolution::PlannedProcess;
use crate::stages::evolution::test_support::{evolve, processes};
use crate::stages::terrain::support::run_stage;
use crate::world::{Layer, MapFrame, World};

/// The side of the test terrains.
const N: usize = 64;

/// The cell size of the test terrains, m.
const CELL_M: f64 = 100.0;

/// The area of one cell, m².
const CELL_AREA: f64 = CELL_M * CELL_M;

/// A float raster of the test terrains.
fn raster(data: Vec<f64>) -> Layer {
    Layer::F64(Grid::from_vec(N, N, data).unwrap())
}

/// Random terrain of `relief` metres with critical slopes between 0.45 and 0.84; the first
/// column is fixed at 0 m (synthetic).
fn steep_world(relief: f64, seed: u64) -> World {
    let mut rng = StageRng::for_key(seed, "steep");
    let mut w = World::new(MapFrame {
        rows: N,
        cols: N,
        cell_m: CELL_M,
    });
    let fixed: Vec<bool> = (0..N * N).map(|i| i % N == 0).collect();
    let height: Vec<f64> = fixed
        .iter()
        .map(|&f| if f { 0.0 } else { rng.range(0.0, relief) })
        .collect();
    let critical: Vec<f64> = (0..N * N).map(|_| rng.range(0.45, 0.84)).collect();
    w.insert("height", raster(height)).unwrap();
    w.insert("critical_slope", raster(critical)).unwrap();
    w.insert("fixed", Layer::Mask(Grid::from_vec(N, N, fixed).unwrap()))
        .unwrap();
    w.insert("erosion", raster(vec![0.0; N * N])).unwrap();
    w
}

/// `w` with every height replaced by `f` of it.
fn reshaped(mut w: World, f: impl Fn(usize, f64) -> f64) -> World {
    let h = w.f64("height").unwrap().as_slice().to_vec();
    let data = h.iter().enumerate().map(|(i, &v)| f(i, v)).collect();
    w.insert("height", raster(data)).unwrap();
    w
}

/// Terraced terrain, whose equal heights order the sweep by cell index.
fn terraced() -> World {
    reshaped(steep_world(400.0, 7), |_, h| 40.0 * (h / 40.0).floor())
}

/// Towers on a stable plain: a few cells far above quiet neighbours.
fn towers() -> World {
    let rises = [
        (10 * N + 10, 900.0),
        (30 * N + 40, 400.0),
        (50 * N + 20, 250.0),
        (5 * N + 60, 600.0),
    ];
    reshaped(steep_world(30.0, 8), |i, h| {
        h + rises
            .iter()
            .find(|&&(at, _)| at == i)
            .map_or(0.0, |&(_, rise)| rise)
    })
}

/// A whole flank rising 2 m per metre to the east from the fixed west column at 0 m, with the
/// critical slope 0.6 everywhere (synthetic): the half-excess passes of the old method settled
/// such a flank only after hundreds of passes (Task 27b).
fn flank() -> World {
    let w = steep_world(1.0, 4);
    let mut w = reshaped(w, |i, _| 2.0 * to_f64(i % N) * CELL_M);
    w.insert("critical_slope", raster(vec![0.6; N * N]))
        .unwrap();
    w
}

fn slope_limit() -> Vec<PlannedProcess> {
    processes("[[process]]\nid = \"slope_limit\"")
}

/// The float raster `name` of `w`, zeros when `w` does not hold it.
fn values(w: &World, name: &str) -> Vec<f64> {
    w.f64(name)
        .map_or_else(|| vec![0.0; N * N], |g| g.as_slice().to_vec())
}

/// The layers `slope_limit` changes, at one moment.
struct Snapshot {
    height: Vec<f64>,
    erosion: Vec<f64>,
    supply: Vec<f64>,
}

fn snapshot(w: &World) -> Snapshot {
    Snapshot {
        height: values(w, "height"),
        erosion: values(w, "erosion"),
        supply: values(w, "slope_supply"),
    }
}

/// Runs `slope_limit` once over `w` and returns the layers before and its report.
fn limit_once(w: &mut World) -> (Snapshot, StageReport) {
    let before = snapshot(w);
    let report = evolve(w, &slope_limit(), 1, 1e4, 1).unwrap();
    (before, report)
}

/// The bits of `v`.
fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Asserts physics test 10 (spec §5.14) for one run of `slope_limit` from `before` to `w`:
/// - no cell that is not fixed lies more than its critical slope × the D8 distance above a
///   neighbour, to 1e-9 m;
/// - no cell rises, and fixed cells keep their height bit for bit;
/// - each cell's lowering is booked as erosion and added to `slope_supply`, and the volume
///   removed (`removed_m3`) equals the lowered volume, the booked erosion and the supply handed
///   on, to 1e-9 relative.
fn assert_physics_10(w: &World, before: &Snapshot, report: &StageReport, what: &str) {
    let after = snapshot(w);
    let critical = w.f64("critical_slope").unwrap().as_slice();
    let fixed = w.mask("fixed").unwrap().as_slice();
    let height = w.f64("height").unwrap();
    let h = height.as_slice();
    for i in (0..N * N).filter(|&i| !fixed[i]) {
        for (k, &offset) in D8.iter().enumerate() {
            let Some(j) = height.neighbour(i, offset) else {
                continue;
            };
            let allowed = critical[i] * d8_distance(CELL_M, k);
            assert!(
                h[i] - h[j] <= allowed + 1e-9,
                "{what}: cell {i} lies {} m above cell {j}, allowed {allowed} m",
                h[i] - h[j]
            );
        }
    }
    let (mut lowered, mut eroded, mut handed) = (0.0, 0.0, 0.0);
    for (i, &is_fixed) in fixed.iter().enumerate() {
        let drop = before.height[i] - after.height[i];
        assert!(drop >= 0.0, "{what}: cell {i} rose by {}", -drop);
        if is_fixed {
            assert_eq!(
                after.height[i].to_bits(),
                before.height[i].to_bits(),
                "{what}: fixed cell {i}"
            );
        }
        let booked = after.erosion[i] - before.erosion[i];
        let supplied = after.supply[i] - before.supply[i];
        assert!(
            (booked - drop).abs() <= 1e-9,
            "{what}: cell {i} booked {booked} of {drop}"
        );
        assert!(
            (supplied - drop).abs() <= 1e-9,
            "{what}: cell {i} handed on {supplied} of {drop}"
        );
        lowered += drop * CELL_AREA;
        eroded += booked * CELL_AREA;
        handed += supplied * CELL_AREA;
    }
    let removed = report.values["slope_limit_removed_m3"];
    let tolerance = 1e-9 * max(removed, 1.0);
    for (name, volume) in [
        ("lowered", lowered),
        ("eroded", eroded),
        ("handed on", handed),
    ] {
        assert!(
            (volume - removed).abs() <= tolerance,
            "{what}: {name} {volume} m³, removed {removed} m³"
        );
    }
}

// Physics test 10 (spec §5.14).
#[test]
fn slope_limit_leaves_no_slope_above_critical() {
    for (what, mut w) in [
        ("150 m relief", steep_world(150.0, 5)),
        ("300 m relief", steep_world(300.0, 11)),
        ("1e7 m relief", steep_world(1.0e7, 3)),
        ("terraced", terraced()),
        ("towers", towers()),
        ("flank", flank()),
    ] {
        let (before, report) = limit_once(&mut w);
        assert!(report.findings.is_empty(), "{what}: {:?}", report.findings);
        assert!(report.values["slope_limit_removed_m3"] > 0.0, "{what}");
        assert_physics_10(&w, &before, &report, what);
        // The limited surface keeps the rule, so a second run leaves it alone, bit for bit.
        let (settled, again) = limit_once(&mut w);
        assert_physics_10(&w, &settled, &again, what);
        let now = snapshot(&w);
        assert_eq!(bits(&now.height), bits(&settled.height), "{what}");
        assert_eq!(bits(&now.erosion), bits(&settled.erosion), "{what}");
        assert_eq!(bits(&now.supply), bits(&settled.supply), "{what}");
        assert_eq!(
            again.values["slope_limit_removed_m3"].to_bits(),
            0.0_f64.to_bits(),
            "{what}"
        );
    }
}

#[test]
fn a_stable_surface_is_left_alone() {
    let mut w = steep_world(30.0, 8);
    // Supply that `fluvial` has not taken yet stays as it is.
    w.insert("slope_supply", raster(vec![0.5; N * N])).unwrap();
    let (before, report) = limit_once(&mut w);
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    let after = snapshot(&w);
    assert_eq!(bits(&after.height), bits(&before.height));
    assert_eq!(bits(&after.erosion), bits(&before.erosion));
    assert_eq!(bits(&after.supply), bits(&before.supply));
    assert_eq!(
        report.values["slope_limit_removed_m3"].to_bits(),
        0.0_f64.to_bits()
    );
    assert_physics_10(&w, &before, &report, "stable");
}

#[test]
fn an_over_steep_flank_settles_at_its_critical_slope_in_one_step() {
    // Every cell stands at 200 m per column; the highest surface that keeps 0.6 rises 60 m per
    // column from the fixed west edge, the shortest D8 path being the row itself.
    let mut w = flank();
    let (before, report) = limit_once(&mut w);
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    let h = w.f64("height").unwrap();
    for i in 0..N * N {
        let want = 0.6 * CELL_M * to_f64(i % N);
        assert!(
            (h.as_slice()[i] - want).abs() <= 1e-9,
            "cell {i}: {}",
            h.as_slice()[i]
        );
    }
    let removed: f64 = before
        .height
        .iter()
        .zip(h.as_slice())
        .map(|(b, a)| (b - a) * CELL_AREA)
        .sum();
    let reported = report.values["slope_limit_removed_m3"];
    assert!(
        (reported - removed).abs() <= 1e-9 * removed,
        "{reported} vs {removed}"
    );
}

#[test]
fn fixed_cells_keep_their_height_and_a_cliff_next_to_one_drains() {
    let mut w = reshaped(steep_world(30.0, 9), |i, h| {
        if i == 10 * N + 1 { 400.0 } else { h }
    });
    let (before, report) = limit_once(&mut w);
    assert!(report.values["slope_limit_removed_m3"] > 0.0);
    let h = w.f64("height").unwrap();
    assert!((0..N).all(|r| h.get(r, 0).to_bits() == 0.0_f64.to_bits()));
    // The cliff falls to its critical slope above the edge; what it lost is supply.
    let critical = w.f64("critical_slope").unwrap().get(10, 1);
    assert!(h.get(10, 1) <= critical * CELL_M + 1e-9, "{}", h.get(10, 1));
    let supply = w.f64("slope_supply").unwrap();
    assert!((supply.get(10, 1) - (400.0 - h.get(10, 1))).abs() <= 1e-9);
    assert!((0..N).all(|r| supply.get(r, 0).to_bits() == 0.0_f64.to_bits()));
    assert_physics_10(&w, &before, &report, "cliff");
}

/// The highest surface at or below `height` with `L_i ≤ L_j + S_c,i·d_ij` for every cell `i`
/// that is not fixed and each D8 neighbour `j`, by brute force: repeat
/// `L_i = min(h_i, min_j L_j + S_c,i·d_ij)` over all cells until nothing changes.
fn brute_force(height: &Grid<f64>, critical: &[f64], fixed: &[bool], cell_m: f64) -> Vec<f64> {
    let mut level = height.as_slice().to_vec();
    loop {
        let mut changed = false;
        for i in (0..level.len()).filter(|&i| !fixed[i]) {
            for (k, &offset) in D8.iter().enumerate() {
                let Some(j) = height.neighbour(i, offset) else {
                    continue;
                };
                let cap = level[j] + critical[i] * d8_distance(cell_m, k);
                if cap < level[i] {
                    level[i] = cap;
                    changed = true;
                }
            }
        }
        if !changed {
            return level;
        }
    }
}

#[test]
fn the_limited_surface_is_the_highest_that_keeps_the_rule() {
    // Small random grids (synthetic): heights up to `relief` (some terraced, so ties order the
    // sweep), critical slopes from 0.1 to 1.2, and fixed cells scattered or along an edge.
    for seed in 0..24_u64 {
        let mut rng = StageRng::for_key(seed, "brute");
        let n = usize::try_from(seed).unwrap();
        let (rows, cols) = (5 + n % 7, 4 + n % 5);
        let cell_m = rng.range(10.0, 200.0);
        let relief = if seed % 3 == 0 { 5.0 } else { 2000.0 };
        let mut data: Vec<f64> = (0..rows * cols).map(|_| rng.range(0.0, relief)).collect();
        if seed % 4 == 1 {
            for h in &mut data {
                *h = 100.0 * (*h / 100.0).floor();
            }
        }
        let critical: Vec<f64> = (0..rows * cols).map(|_| rng.range(0.1, 1.2)).collect();
        let fixed: Vec<bool> = (0..rows * cols)
            .map(|i| match seed % 3 {
                0 => false,
                1 => i % cols == 0,
                _ => rng.range(0.0, 1.0) < 0.1,
            })
            .collect();
        let height = Grid::from_vec(rows, cols, data).unwrap();
        let want = brute_force(&height, &critical, &fixed, cell_m);
        let got = limited(&height, &critical, &fixed, cell_m);
        assert_eq!((got.rows(), got.cols()), (rows, cols));
        for (i, (g, w)) in got.as_slice().iter().zip(&want).enumerate() {
            assert!((g - w).abs() <= 1e-9, "seed {seed}, cell {i}: {g} vs {w}");
        }
    }
}

#[test]
fn the_result_does_not_depend_on_the_run() {
    let run = || {
        let mut w = steep_world(150.0, 5);
        let report = evolve(&mut w, &slope_limit(), 1, 1e4, 1).unwrap();
        let now = snapshot(&w);
        (
            bits(&now.height),
            bits(&now.erosion),
            bits(&now.supply),
            report,
        )
    };
    assert_eq!(run(), run());
}

#[test]
fn a_critical_slope_that_cannot_limit_is_a_failure() {
    let with_slope = |r: usize, c: usize, slope: f64| {
        let mut w = steep_world(10.0, 1);
        let mut critical = w.f64("critical_slope").unwrap().clone();
        critical.set(r, c, slope);
        w.insert("critical_slope", Layer::F64(critical)).unwrap();
        w
    };
    for bad in [-0.1, f64::NAN, f64::INFINITY] {
        let mut w = with_slope(5, 7, bad);
        let err = evolve(&mut w, &slope_limit(), 1, 1e4, 1).unwrap_err();
        let want = format!("cell {} has the critical slope {bad}", 5 * N + 7);
        assert!(
            matches!(&err, StageError::Failed { stage, message }
                if stage == "landscape_evolution/slope_limit" && message.contains(&want)),
            "{bad}: {err:?}"
        );
    }
    // A fixed cell keeps its height, so its critical slope is never used.
    let mut w = with_slope(5, 0, f64::NAN);
    evolve(&mut w, &slope_limit(), 1, 1e4, 1).unwrap();
}

#[test]
fn slope_limit_declares_its_layers_and_runs_only_in_the_evolution() {
    let stage: &dyn Stage = &SlopeLimit;
    let params = stage.params(&toml::Table::new()).unwrap();
    assert_eq!(
        stage.reads(params.as_ref()),
        [
            LayerSpec::f64("height"),
            LayerSpec::f64("critical_slope"),
            LayerSpec::mask("fixed"),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("slope_supply").optional(),
        ]
    );
    assert_eq!(
        stage.writes(params.as_ref()),
        [
            LayerSpec::f64("height"),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("slope_supply"),
        ]
    );
    assert!(stage.params(&"limit = 1.0".parse().unwrap()).is_err());
    let mut w = steep_world(10.0, 1);
    let err = run_stage(&mut w, &SlopeLimit, "").unwrap_err();
    assert!(matches!(&err, StageError::Failed { stage, .. } if stage == "slope_limit"));
}
