//! `fluvial` on a hand-built 2 x 2 world, and after `uplift` and the routing processes on 64 x 64
//! landscapes of 100 m cells. Deposition (G > 0) is in `deposition`, the routed depressions'
//! lakes in `lakes`, the slope supply in `supply`.

mod deposition;
mod lakes;
mod supply;

use std::f64::consts::SQRT_2;

use libm::pow;

use super::{Fluvial, FluvialParams, runoff_weight};
use crate::grid::Grid;
use crate::kernels::noise::spectral_noise;
use crate::kernels::num::{max, min, to_f64};
use crate::rng::StageRng;
use crate::stage::{LayerSpec, Stage, StageError};
use crate::stages::evolution::flow::{FlowWeights, cell_index};
use crate::stages::evolution::resolve_sinks::{Depressions, SpillPoint};
use crate::stages::evolution::test_support::{evolve, processes};
use crate::stages::terrain::climate::{Climate, ClimateFields, ClimateParams};
use crate::stages::terrain::grid::GridStage;
use crate::stages::terrain::support::{run_stage, world};
use crate::world::{Layer, MapFrame, ValueType, World};

/// The side of the landscapes.
const N: usize = 64;

/// Every edge drained.
const ALL_EDGES: &str = r#"["north", "east", "south", "west"]"#;

/// Both flow partitions.
const BOTH: [&str; 2] = ["steepest", "slope_adaptive"];

/// The uniform uplift rate of the landscapes, m/yr.
const UPLIFT: f64 = 1e-3;

/// The uniform erodibility, for Q in m² and m = 0.5.
const K: f64 = 1e-5;

/// The area of a 100 m cell, m².
const CELL_AREA: f64 = 1e4;

/// `fluvial` alone, with m = 0.5 and the parameter `extra`.
fn fluvial_only(extra: &str) -> String {
    format!("[[process]]\nid = \"fluvial\"\nm = 0.5\n{extra}")
}

/// `uplift`, the three routing processes with `exponent`, then `fluvial` with the parameters
/// `fluvial`.
fn recipe(exponent: &str, fluvial: &str) -> String {
    format!(
        "[[process]]\nid = \"uplift\"\n[[process]]\nid = \"route_steepest\"\n\
         [[process]]\nid = \"resolve_sinks\"\n\
         [[process]]\nid = \"partition_flow\"\nexponent = \"{exponent}\"\n\
         [[process]]\nid = \"fluvial\"\n{fluvial}"
    )
}

/// A climate without fields: `p̂` is 1 everywhere.
fn no_fields() -> Climate {
    Climate {
        params: ClimateParams::default(),
        fields: None,
    }
}

/// A climate whose fields hold `runoff` (and the same precipitation, no evaporation).
fn with_runoff(runoff: Grid<f64>) -> Climate {
    let zeros = runoff.map(|_| 0.0);
    Climate {
        params: ClimateParams::default(),
        fields: Some(ClimateFields {
            precipitation: runoff.clone(),
            pet: zeros.clone(),
            aet: zeros,
            runoff,
        }),
    }
}

/// A 64 x 64 landscape whose drained `edges` are fixed at 0 m, elsewhere smooth noise of 40 m
/// relief with up to 1 m of jitter; uniform uplift [`UPLIFT`] without sag, erodibility [`K`],
/// no erosion or deposition yet, and a climate without fields.
fn landscape(edges: &str, seed: u64) -> World {
    let mut w = world(N, N);
    run_stage(&mut w, &GridStage, &format!("edges = {edges}")).unwrap();
    let fixed = w.mask("fixed").unwrap().clone();
    let mut rng = StageRng::for_key(seed, "smooth");
    let smooth = spectral_noise(N, N, &mut rng, 3.0, 0.0, None);
    let data = (0..N * N)
        .map(|i| {
            let h = 40.0 * smooth.get(i / N, i % N) + rng.range(0.0, 1.0);
            if fixed.as_slice()[i] { 0.0 } else { h }
        })
        .collect();
    let filled = |v: f64| Layer::F64(Grid::filled(N, N, v));
    w.insert("height", Layer::F64(Grid::from_vec(N, N, data).unwrap()))
        .unwrap();
    w.insert("uplift", filled(UPLIFT)).unwrap();
    w.insert("sag_rate", filled(0.0)).unwrap();
    w.insert("erodibility", filled(K)).unwrap();
    w.insert("erosion", filled(0.0)).unwrap();
    w.insert("deposition", filled(0.0)).unwrap();
    w.insert("climate", Layer::Value(Box::new(no_fields())))
        .unwrap();
    w
}

/// A 2 x 2 world of 100 m cells holding `height` and `climate`, erodibility [`K`]:
/// - cell 0 is a sink without edges;
/// - cells 1 and 2 send everything to cell 0, 100 m away; cell 2 is fixed;
/// - cell 3 sends a quarter to cell 1 (north, 100 m) and three quarters to cell 0 (north-west,
///   100·√2 m).
///
/// The stack is `[0, 1, 2, 3]`.
fn square(height: [f64; 4], climate: Climate) -> World {
    let mut w = world(2, 2);
    let weights = FlowWeights {
        start: vec![0, 0, 1, 2, 4],
        to: vec![0, 0, 1, 0],
        weight: vec![1.0, 1.0, 0.25, 0.75],
        dist: vec![100.0, 100.0, 100.0, 100.0 * SQRT_2],
    };
    let depressions = Depressions {
        label: Grid::filled(2, 2, 0),
        spill: Vec::new(),
    };
    let grid = |v: Vec<f64>| Layer::F64(Grid::from_vec(2, 2, v).unwrap());
    let fixed = Grid::from_vec(2, 2, vec![false, false, true, false]).unwrap();
    let u32s = |v: Vec<u32>| Layer::U32(Grid::from_vec(2, 2, v).unwrap());
    w.insert("height", grid(height.to_vec())).unwrap();
    w.insert("routing_surface", grid(height.to_vec())).unwrap();
    w.insert("erodibility", grid(vec![K; 4])).unwrap();
    w.insert("erosion", grid(vec![0.0; 4])).unwrap();
    w.insert("deposition", grid(vec![0.0; 4])).unwrap();
    w.insert("fixed", Layer::Mask(fixed)).unwrap();
    w.insert("receivers", u32s(vec![0, 0, 0, 1])).unwrap();
    w.insert("stack", u32s(vec![0, 1, 2, 3])).unwrap();
    w.insert("weights", Layer::Value(Box::new(weights)))
        .unwrap();
    w.insert("depressions", Layer::Value(Box::new(depressions)))
        .unwrap();
    w.insert("climate", Layer::Value(Box::new(climate)))
        .unwrap();
    w
}

/// Runs `fluvial` alone over `w` for one step of 1e5 years with `extra` parameters.
fn fluvial_step(w: &mut World, extra: &str) -> Result<(), StageError> {
    evolve(w, &processes(&fluvial_only(extra)), 1, 1e5, 1).map(|_| ())
}

fn layer(w: &World, name: &str) -> Vec<f64> {
    w.f64(name).unwrap().as_slice().to_vec()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * max(1.0, b.abs())
}

/// Asserts that `actual` and `expected` agree cell by cell to 1e-12 (relative above 1).
fn assert_close(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
        assert!(close(*a, *e), "cell {i}: {a} vs {e}");
    }
}

/// The largest absolute difference between `a` and `b`.
fn largest_change(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .fold(0.0, |most, (x, y)| max(most, (x - y).abs()))
}

#[test]
fn the_implicit_step_follows_the_scheme() {
    // Runoff [0, 2, 1, 1] has mean 1, so p̂ is the runoff itself.
    let runoff = Grid::from_vec(2, 2, vec![0.0, 2.0, 1.0, 1.0]).unwrap();
    let mut w = square([0.0, 10.0, 20.0, 30.0], with_runoff(runoff));
    fluvial_step(&mut w, "g = 0.0").unwrap();
    // Cell 1: Q = 2·1e4 + 0.25·1e4 = 2.25e4, F = 1e-5 · 150 · 1 · 1e5 / 100 = 1.5.
    let h1 = (10.0 + 1.5 * 0.0) / (1.0 + 1.5);
    assert!(close(h1, 4.0), "the closed form of cell 1");
    // Cell 3: Q = 1e4, K·Q^m·Δt = 100; F to cell 1 = 100 · 0.25 / 100, to cell 0 = 100 · 0.75 /
    // (100·√2).
    let (f31, f30) = (0.25, 0.75 / SQRT_2);
    let h3 = (30.0 + f31 * h1 + f30 * 0.0) / (1.0 + f31 + f30);
    // Cell 0 is a sink and cell 2 is fixed: both keep their heights.
    assert_close(&layer(&w, "height"), &[0.0, h1, 20.0, h3]);
    assert_close(&layer(&w, "erosion"), &[0.0, 10.0 - h1, 0.0, 30.0 - h3]);
    assert!(layer(&w, "deposition").iter().all(|d| d.to_bits() == 0));
    // The plain drainage area: cell 0 gathers the whole map.
    assert_close(&layer(&w, "drainage_area"), &[4e4, 1.25e4, 1e4, 1e4]);
}

#[test]
fn a_receiver_not_below_the_cell_takes_no_share() {
    // Cell 0 (15 m) stands above cells 1 (10 m) and 3 (12 m): both edges to it climb, as over a
    // pass link or inside a routed depression, and stream power erodes only downhill.
    let mut w = square([15.0, 10.0, 20.0, 12.0], no_fields());
    fluvial_step(&mut w, "g = 0.0").unwrap();
    let h = layer(&w, "height");
    assert_eq!(
        h[1].to_bits(),
        10.0_f64.to_bits(),
        "cell 1 keeps its height"
    );
    // Cell 3 erodes toward cell 1 only: Q = 1e4, F = 0.25.
    let h3 = (12.0 + 0.25 * 10.0) / 1.25;
    assert!(close(h[3], h3), "{} vs {h3}", h[3]);
    assert!(close(h3, 11.6));
    assert!(layer(&w, "deposition").iter().all(|d| d.to_bits() == 0));
}

#[test]
fn a_receiver_above_the_solved_height_takes_no_share() {
    // Cell 3 (30 m) drains to cell 1 (29 m, K = 0 so it stays) and to cell 0 (0 m). Cell 1 lies
    // below cell 3's height without incision but above the height cell 3 reaches: an uphill
    // receiver, which must not pull cell 3 up. Only cell 0 takes a share: F30 = 0.75/√2.
    let mut w = square([0.0, 29.0, 20.0, 30.0], no_fields());
    let erodibility = Grid::from_vec(2, 2, vec![K, 0.0, K, K]).unwrap();
    w.insert("erodibility", Layer::F64(erodibility)).unwrap();
    fluvial_step(&mut w, "g = 0.0").unwrap();
    let h = layer(&w, "height");
    assert_eq!(
        h[1].to_bits(),
        29.0_f64.to_bits(),
        "cell 1 keeps its height"
    );
    let h3 = 30.0 / (1.0 + 0.75 / SQRT_2);
    assert!(close(h[3], h3), "{} vs {h3}", h[3]);
    assert!(h[3] < h[1], "cell 1 lies above cell 3's solved height");
}

#[test]
fn fixed_cells_never_change() {
    // Cell 2 of the square is fixed although it has an edge to the lower cell 0.
    let mut w = square([0.0, 10.0, 20.0, 30.0], no_fields());
    fluvial_step(&mut w, "g = 0.0").unwrap();
    assert_eq!(layer(&w, "height")[2].to_bits(), 20.0_f64.to_bits());
    assert_eq!(layer(&w, "erosion")[2].to_bits(), 0.0_f64.to_bits());
    for exponent in BOTH {
        let mut w = landscape(ALL_EDGES, 3);
        let procs = processes(&recipe(exponent, "g = 0.0"));
        evolve(&mut w, &procs, 10, 2.5e4, 1).unwrap();
        let fixed = w.mask("fixed").unwrap().as_slice().to_vec();
        let (h, erosion) = (layer(&w, "height"), layer(&w, "erosion"));
        for i in (0..N * N).filter(|&i| fixed[i]) {
            assert_eq!(h[i].to_bits(), 0.0_f64.to_bits(), "{exponent}: cell {i}");
            assert_eq!(
                erosion[i].to_bits(),
                0.0_f64.to_bits(),
                "{exponent}: cell {i}"
            );
        }
    }
}

#[test]
fn erosion_is_booked_and_non_negative() {
    const DT: f64 = 2.5e4;
    for exponent in BOTH {
        let mut w = landscape(ALL_EDGES, 4);
        let procs = processes(&recipe(exponent, "g = 0.0"));
        let fixed = w.mask("fixed").unwrap().as_slice().to_vec();
        let mut booked = vec![0.0; N * N];
        for _ in 0..10 {
            let before = layer(&w, "height");
            evolve(&mut w, &procs, 1, DT, 1).unwrap();
            let after = layer(&w, "height");
            for i in (0..N * N).filter(|&i| !fixed[i]) {
                booked[i] += before[i] + UPLIFT * DT - after[i];
            }
        }
        let erosion = layer(&w, "erosion");
        assert!(erosion.iter().all(|&e| e >= 0.0), "{exponent}");
        assert!(
            erosion.iter().any(|&e| e > 1.0),
            "{exponent}: something erodes"
        );
        for i in 0..N * N {
            assert!(
                (erosion[i] - booked[i]).abs() <= 1e-9 * max(1.0, booked[i]),
                "{exponent}: cell {i} booked {} of {}",
                erosion[i],
                booked[i]
            );
        }
        let deposition = layer(&w, "deposition");
        assert!(
            deposition.iter().all(|d| d.to_bits() == 0),
            "{exponent}: G = 0 deposits nothing (spec ruling 29)"
        );
    }
}

#[test]
fn zero_precipitation_erodes_nothing_and_stays_finite() {
    const DT: f64 = 2.5e4;
    let mut w = landscape(ALL_EDGES, 5);
    let dry = with_runoff(Grid::filled(N, N, 0.0));
    w.insert("climate", Layer::Value(Box::new(dry))).unwrap();
    let start = layer(&w, "height");
    let fixed = w.mask("fixed").unwrap().as_slice().to_vec();
    evolve(
        &mut w,
        &processes(&recipe("slope_adaptive", "g = 0.0")),
        5,
        DT,
        1,
    )
    .unwrap();
    let h = layer(&w, "height");
    for i in 0..N * N {
        let lifted = if fixed[i] {
            start[i]
        } else {
            (0..5).fold(start[i], |v, _| v + UPLIFT * DT)
        };
        assert_eq!(h[i].to_bits(), lifted.to_bits(), "cell {i} only uplifts");
    }
    assert!(layer(&w, "erosion").iter().all(|e| e.to_bits() == 0));
    let area = layer(&w, "drainage_area");
    assert!(area.iter().all(|a| a.is_finite() && *a >= CELL_AREA));
    let frame = *w.frame();
    let none = runoff_weight(&with_runoff(Grid::filled(N, N, 0.0)), &frame).unwrap();
    assert!(none.as_slice().iter().all(|p| p.to_bits() == 0));
}

#[test]
fn runoff_weight_is_runoff_over_its_mean() {
    let frame = MapFrame {
        rows: 2,
        cols: 2,
        cell_m: 100.0,
    };
    let ones = runoff_weight(&no_fields(), &frame).unwrap();
    assert_eq!((ones.rows(), ones.cols()), (2, 2));
    assert!(
        ones.as_slice()
            .iter()
            .all(|p| p.to_bits() == 1.0_f64.to_bits())
    );
    let runoff = Grid::from_vec(2, 2, vec![100.0, 300.0, 0.0, 400.0]).unwrap();
    let p = runoff_weight(&with_runoff(runoff), &frame).unwrap();
    assert_close(p.as_slice(), &[0.5, 1.5, 0.0, 2.0]);
    // A runoff of another shape, even with as many cells, does not fit the frame.
    for (rows, cols) in [(1, 4), (3, 3)] {
        let other = with_runoff(Grid::filled(rows, cols, 1.0));
        assert!(runoff_weight(&other, &frame).is_none(), "{rows} x {cols}");
    }
}

#[test]
fn drainage_area_is_not_weighted_by_runoff() {
    let wet_east = Grid::from_vec(N, N, (0..N * N).map(|i| to_f64(i % N)).collect()).unwrap();
    let mut a = landscape(ALL_EDGES, 6);
    let mut b = landscape(ALL_EDGES, 6);
    b.insert("climate", Layer::Value(Box::new(with_runoff(wet_east))))
        .unwrap();
    let procs = processes(&recipe("slope_adaptive", "g = 0.0"));
    evolve(&mut a, &procs, 1, 2.5e4, 1).unwrap();
    evolve(&mut b, &procs, 1, 2.5e4, 1).unwrap();
    assert_eq!(layer(&a, "drainage_area"), layer(&b, "drainage_area"));
    assert_ne!(
        layer(&a, "height"),
        layer(&b, "height"),
        "runoff steers erosion"
    );
    // Every drop leaves over the drained edges: the outlets gather the whole map.
    let fixed = a.mask("fixed").unwrap().as_slice().to_vec();
    let area = layer(&a, "drainage_area");
    let out: f64 = (0..N * N).filter(|&i| fixed[i]).map(|i| area[i]).sum();
    let total = CELL_AREA * to_f64(N * N);
    assert!((out - total).abs() <= 1e-9 * total, "{out} of {total}");
}

/// Runs `recipe` over `w` one step of `dt` years at a time until no height changes by 1e-6 m
/// or more in a step, and returns the number of steps.
fn run_to_steady_state(w: &mut World, recipe: &str, dt: f64, limit: u32) -> u32 {
    let procs = processes(recipe);
    let mut last = f64::INFINITY;
    for step in 1..=limit {
        let before = layer(w, "height");
        evolve(w, &procs, 1, dt, 1).unwrap();
        last = largest_change(&before, &layer(w, "height"));
        if last < 1e-6 {
            return step;
        }
    }
    panic!("no steady state after {limit} steps: the last step changed a height by {last} m");
}

/// Physics test 1 (spec §5.14). At the implicit scheme's steady state every cell's erosion
/// balances its uplift, `K·Q^m·Σ_r w_r·S_r = U`, so the flow-weighted slope over the weights of
/// the last step is `(U/K)·Q^(−m)`. With p̂ = 1, Q is the drainage area.
#[test]
fn steady_state_slopes_follow_the_stream_power_law() {
    for exponent in BOTH {
        let mut w = landscape(ALL_EDGES, 1);
        let steps = run_to_steady_state(&mut w, &recipe(exponent, "g = 0.0\nm = 0.5"), 2.5e5, 2000);
        let h = layer(&w, "height");
        let area = layer(&w, "drainage_area");
        let fixed = w.mask("fixed").unwrap().as_slice().to_vec();
        let weights = w.value::<FlowWeights>("weights").unwrap();
        let mut checked = 0;
        for i in (0..N * N).filter(|&i| !fixed[i] && area[i] > 10.0 * CELL_AREA) {
            let slope: f64 = weights
                .edges(i)
                .map(|e| {
                    weights.weight[e] * (h[i] - h[cell_index(weights.to[e])]) / weights.dist[e]
                })
                .sum();
            let law = UPLIFT / K * pow(area[i], -0.5);
            let error = (slope / law - 1.0).abs();
            assert!(
                error <= 0.02,
                "{exponent}, after {steps} steps: cell {i} with A = {} cells has slope {slope}, \
                 the law {law}",
                area[i] / CELL_AREA
            );
            checked += 1;
        }
        assert!(
            checked > 100,
            "{exponent}: only {checked} cells above 10 cells"
        );
    }
}

/// What the stability test keeps of one step: every cell's height change, receiver and
/// drainage area, and whether its receiver ended below its `h*` (so the edge took a share).
struct Step {
    change: Vec<f64>,
    receivers: Vec<u32>,
    area: Vec<f64>,
    downhill: Vec<bool>,
}

impl Step {
    /// Whether cell `i` kept its receiver, drainage area and share from `then` to `self`, so the
    /// solver's `F` for it stayed the same.
    fn kept(&self, then: &Step, i: usize) -> bool {
        self.receivers[i] == then.receivers[i]
            && self.area[i].to_bits() == then.area[i].to_bits()
            && self.downhill[i] == then.downhill[i]
    }
}

/// Physics test 2 with G = 0 (spec §5.14): steepest routing at ten times the default time step,
/// 60 steps from a noise surface.
///
/// **No cell falls below its receiver.** After every step, every non-fixed cell lies above its
/// receiver, except in the step's routed depressions (`depressions.label > 0`). There
/// `resolve_sinks` lifts the routing surface above the height, so a receiver may stand higher
/// than its cell: over a pass link that climbs, and at every other lifted cell, whose receiver
/// follows the lifted surface. Skipping only the pass-link cells (`SpillPoint.cell`) is not
/// enough: with the south edge drained, other lifted cells lie below their receivers. Outside
/// depressions the routing surface is the height, so the receiver lies lower in height too.
/// Inside them the check is the solver's own: a cell it lowers (below `h*`, the height after
/// uplift) never ends below its receiver.
///
/// **No oscillation.** Subtracting the implicit step of step `n − 1` from that of step `n`
/// gives `(1 + F)·Δh_i(n) = Δh_i(n − 1) + F·Δh_r(n)` whenever the cell's `F` stays the same
/// (same receiver, drainage area and share; uniform U and K). So every height change lies
/// between the cell's previous change and its receiver's current change, within 1e-9 m: the
/// solver never overshoots. An explicit step with `F > 1` fails this at once, since there
/// `Δh_i(n) = (1 − F)·Δh_i(n − 1)` beside a fixed receiver. The check runs on every cell step
/// whose `F` stayed the same, which must be most of them.
///
/// A plain sign test on each cell's height change ("no change flips sign over three steps")
/// does not separate the solver from the landscape. From a noise surface the drainage
/// reorganises, and a cell's area, so its height change, goes up and down at the default time
/// step too. Even on unchanged routing, a change travelling up a channel from downstream adds
/// to the cell's own decay and can flip the sign at the millimetre scale.
#[test]
fn stability_at_ten_times_the_time_step() {
    const DT: f64 = 2.5e5;
    const STEPS: usize = 60;
    const TOL: f64 = 1e-9;
    for edges in [ALL_EDGES, r#"["south"]"#] {
        let mut w = landscape(edges, 2);
        let procs = processes(&recipe("steepest", "g = 0.0"));
        let fixed = w.mask("fixed").unwrap().as_slice().to_vec();
        let mut steps: Vec<Step> = Vec::with_capacity(STEPS);
        for step in 0..STEPS {
            let before = layer(&w, "height");
            evolve(&mut w, &procs, 1, DT, 1).unwrap();
            let after = layer(&w, "height");
            let receivers = w.u32("receivers").unwrap().as_slice().to_vec();
            let label = w.value::<Depressions>("depressions").unwrap().label.clone();
            let mut downhill = vec![false; N * N];
            for i in 0..N * N {
                assert!(after[i].is_finite(), "{edges}, step {step}: cell {i}");
                let r = cell_index(receivers[i]);
                if fixed[i] || r == i {
                    continue;
                }
                let lifted = before[i] + UPLIFT * DT;
                downhill[i] = after[r] < lifted;
                if label.as_slice()[i] == 0 || after[i] < lifted {
                    assert!(
                        after[i] > after[r],
                        "{edges}, step {step}: cell {i} at {} lies below its receiver {r} at {}",
                        after[i],
                        after[r]
                    );
                }
            }
            steps.push(Step {
                change: after.iter().zip(&before).map(|(a, b)| a - b).collect(),
                receivers,
                area: layer(&w, "drainage_area"),
                downhill,
            });
        }
        let (mut checked, mut cell_steps) = (0, 0);
        for n in 1..STEPS {
            let (then, now) = (&steps[n - 1], &steps[n]);
            for i in (0..N * N).filter(|&i| !fixed[i]) {
                let r = cell_index(now.receivers[i]);
                cell_steps += 1;
                if r == i || !now.kept(then, i) {
                    continue;
                }
                checked += 1;
                let (previous, receiver, change) = (then.change[i], now.change[r], now.change[i]);
                let (low, high) = (min(previous, receiver), max(previous, receiver));
                assert!(
                    low - TOL <= change && change <= high + TOL,
                    "{edges}, step {n}: cell {i} changes by {change}, beyond its last change \
                     {previous} and its receiver's change {receiver}"
                );
            }
        }
        assert!(
            2 * checked > cell_steps,
            "{edges}: only {checked} of {cell_steps} checked"
        );
    }
}

/// Asserts that one step of `fluvial` with `params` over `w` fails with a message that is
/// exactly `text`.
fn fails_with(mut w: World, params: &str, text: &str) {
    let err = fluvial_step(&mut w, params).unwrap_err();
    assert!(
        matches!(&err, StageError::Failed { message, .. } if message == text),
        "{text}: {err:?}"
    );
}

#[test]
fn inputs_that_do_not_fit_the_grid_are_a_failure() {
    let weights = |start: Vec<u32>, to: Vec<u32>, weight: Vec<f64>, dist: Vec<f64>| FlowWeights {
        start,
        to,
        weight,
        dist,
    };
    let cases = [
        (
            weights(vec![0, 0, 1, 2], vec![0, 0], vec![1.0; 2], vec![100.0; 2]),
            "the flow weights cover 3 cells, but the grid has 4",
        ),
        (
            weights(
                vec![0, 0, 1, 2, 3],
                vec![0, 0, 0],
                vec![1.0; 2],
                vec![100.0; 3],
            ),
            "the flow weights hold 3 targets, 2 shares and 3 distances",
        ),
        (
            weights(
                vec![0, 0, 1, 5, 3],
                vec![0, 0, 0],
                vec![1.0; 3],
                vec![100.0; 3],
            ),
            "the flow weights start an edge list at 5, past their 3 edges",
        ),
        (
            weights(
                vec![0, 0, 1, 2, 3],
                vec![0, 0, 7],
                vec![1.0; 3],
                vec![100.0; 3],
            ),
            "a flow weight sends water to cell 7, outside the grid of 4 cells",
        ),
    ];
    for (bad, text) in cases {
        let mut w = square([0.0, 10.0, 20.0, 30.0], no_fields());
        w.insert("weights", Layer::Value(Box::new(bad))).unwrap();
        fails_with(w, "g = 0.0", text);
    }
    let mut w = square([0.0, 10.0, 20.0, 30.0], no_fields());
    let stack = Grid::from_vec(2, 2, vec![0, 1, 2, 9]).unwrap();
    w.insert("stack", Layer::U32(stack)).unwrap();
    fails_with(
        w,
        "g = 0.0",
        "the stack names cell 9, outside the grid of 4 cells",
    );
    let mut w = square([0.0, 10.0, 20.0, 30.0], no_fields());
    let wide = with_runoff(Grid::filled(3, 3, 1.0));
    w.insert("climate", Layer::Value(Box::new(wide))).unwrap();
    fails_with(
        w,
        "g = 0.0",
        "the climate's runoff covers 3 x 3 cells, but the grid has 2 x 2",
    );
}

#[test]
fn depressions_that_do_not_fit_the_grid_are_a_failure_with_deposition() {
    let spill = |cell: u32, level: f64| SpillPoint { cell, to: 0, level };
    let cases = [
        (
            Grid::filled(3, 3, 0),
            vec![],
            "the depressions label 9 cells, but the grid has 4",
        ),
        (
            Grid::from_vec(2, 2, vec![0, 2, 0, 0]).unwrap(),
            vec![spill(1, 15.0)],
            "cell 1 lies in depression 2, but only 1 depression spills",
        ),
        (
            Grid::from_vec(2, 2, vec![0, 1, 0, 0]).unwrap(),
            vec![spill(4, 15.0)],
            "depression 1 spills from cell 4, outside the grid of 4 cells",
        ),
        (
            Grid::from_vec(2, 2, vec![0, 1, 0, 2]).unwrap(),
            vec![spill(1, 15.0), spill(1, 15.0)],
            "depressions 1 and 2 both spill from cell 1",
        ),
        (
            Grid::from_vec(2, 2, vec![0, 1, 0, 0]).unwrap(),
            vec![spill(1, f64::NAN)],
            "depression 1 has the spill level NaN",
        ),
    ];
    for (label, spill, text) in cases {
        let bad = Depressions { label, spill };
        // G = 0 does not fill, so it never looks at the depressions.
        let mut plain = square([0.0, 10.0, 20.0, 30.0], no_fields());
        plain
            .insert("depressions", Layer::Value(Box::new(bad.clone())))
            .unwrap();
        fluvial_step(&mut plain, "g = 0.0").unwrap();
        let mut w = square([0.0, 10.0, 20.0, 30.0], no_fields());
        w.insert("depressions", Layer::Value(Box::new(bad)))
            .unwrap();
        fails_with(w, "g = 1.0", text);
    }
}

#[test]
fn fluvial_outside_the_evolution_fails() {
    let mut w = square([0.0, 10.0, 20.0, 30.0], no_fields());
    let err = run_stage(&mut w, &Fluvial, "g = 0.0").unwrap_err();
    assert!(
        matches!(&err, StageError::Failed { stage, message }
            if stage == "fluvial" && message.contains("landscape_evolution")),
        "{err:?}"
    );
}

#[test]
fn fluvial_declares_its_layers_and_parameters() {
    let stage: &dyn Stage = &Fluvial;
    let err = stage.params(&toml::Table::new()).unwrap_err();
    assert!(
        err.to_string().contains("`g`"),
        "the recipe states g: {err}"
    );
    for (bad, parameter) in [
        ("g = -0.5", Some("g")),
        ("g = nan", Some("g")),
        ("g = 0.0\nm = -1.0", Some("m")),
        ("g = 0.0\nm = inf", Some("m")),
        ("g = 0.0\nmax_iterations = 0", Some("max_iterations")),
        ("g = 0.0\nn = 1.0", Some("n")),
        (
            "g = 1.0\nsupply_passes_dry_cells = 1",
            Some("supply_passes_dry_cells"),
        ),
        (
            "g = 1.0\nsupply_passes_dry_cell = true",
            Some("supply_passes_dry_cell"),
        ),
    ] {
        let err = stage.params(&bad.parse().unwrap()).unwrap_err();
        assert_eq!(err.parameter.as_deref(), parameter, "{bad}: {err}");
    }
    let params = stage.params(&"g = 0.0".parse().unwrap()).unwrap();
    let typed = params.as_any().downcast_ref::<FluvialParams>().unwrap();
    assert!((typed.m - 0.45).abs() < 1e-12 && typed.max_iterations == 50);
    assert_eq!(typed.g.to_bits(), 0.0_f64.to_bits());
    assert!(
        !typed.supply_passes_dry_cells,
        "the switch is off by default"
    );
    assert_eq!(
        stage.reads(params.as_ref()),
        [
            LayerSpec::f64("height"),
            LayerSpec::f64("erodibility"),
            LayerSpec::u32("receivers"),
            LayerSpec::value("weights", FlowWeights::NAME),
            LayerSpec::u32("stack"),
            LayerSpec::value("depressions", Depressions::NAME),
            LayerSpec::mask("fixed"),
            LayerSpec::value("climate", Climate::NAME),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("deposition"),
            LayerSpec::f64("slope_supply").optional(),
        ]
    );
    assert_eq!(
        stage.writes(params.as_ref()),
        [
            LayerSpec::f64("height"),
            LayerSpec::f64("erosion"),
            LayerSpec::f64("deposition"),
            LayerSpec::f64("drainage_area"),
            LayerSpec::f64("slope_supply").optional(),
        ]
    );
    // Deposition also reads the routing surface (the depressions' flood order) and the uplift
    // (Yuan et al.'s tolerance), which may be absent.
    let deposits = stage.params(&"g = 1.0".parse().unwrap()).unwrap();
    let reads = stage.reads(deposits.as_ref());
    assert_eq!(reads[..11], stage.reads(params.as_ref())[..]);
    assert_eq!(
        reads[11..],
        [
            LayerSpec::f64("routing_surface"),
            LayerSpec::f64("uplift").optional(),
        ]
    );
    assert_eq!(
        stage.writes(deposits.as_ref()),
        stage.writes(params.as_ref())
    );
    // The switch reads and writes no layer of its own.
    let passing = "g = 1.0\nsupply_passes_dry_cells = true".parse().unwrap();
    let passing = stage.params(&passing).unwrap();
    assert_eq!(stage.reads(passing.as_ref()), reads);
    assert_eq!(
        stage.writes(passing.as_ref()),
        stage.writes(params.as_ref())
    );
}
