//! `fluvial` with deposition (G > 0): the Gauss–Seidel scheme of Yuan et al. 2019 on the hand
//! world, physics tests 2, 4, 5 and 14 (spec §5.14), and the edges of spec §8.

use std::f64::consts::SQRT_2;

use super::{
    ALL_EDGES, BOTH, CELL_AREA, K, N, UPLIFT, fluvial_only, landscape, layer, no_fields, recipe,
    run_to_steady_state, square,
};
use crate::grid::Grid;
use crate::kernels::num::{max, min, to_f64};
use crate::stage::{Finding, StageError, StageReport};
use crate::stages::evolution::flow::{FlowWeights, cell_index};
use crate::stages::evolution::fluvial::fill::Traps;
use crate::stages::evolution::fluvial::solve::{Deposition, Incision, deposit};
use crate::stages::evolution::resolve_sinks::EPSILON;
use crate::stages::evolution::resolve_sinks::{Depressions, SpillPoint};
use crate::stages::evolution::test_support::{evolve, processes};
use crate::stages::terrain::grid::GridStage;
use crate::stages::terrain::support::{run_stage, world};
use crate::world::{Layer, World};

/// The value `name` of `fluvial` in an evolution report.
pub(super) fn value(report: &StageReport, name: &str) -> f64 {
    report.values[&format!("fluvial_{name}")]
}

/// The three routing processes with `exponent`, then `fluvial` with `fluvial`: no uplift.
pub(super) fn routed(exponent: &str, fluvial: &str) -> String {
    format!(
        "[[process]]\nid = \"route_steepest\"\n[[process]]\nid = \"resolve_sinks\"\n\
         [[process]]\nid = \"partition_flow\"\nexponent = \"{exponent}\"\n\
         [[process]]\nid = \"fluvial\"\n{fluvial}"
    )
}

/// The volume of `height` over the map, m³.
pub(super) fn volume(height: &[f64]) -> f64 {
    height.iter().map(|h| h * CELL_AREA).sum()
}

/// The square world's step of 1e5 years with `g`, solved by hand, for a sink at `h0` and cell 3
/// starting at `h3`. Cell 3 drains to cells 1 and 0, cell 1 to cell 0. `Qs` leaves each cell
/// with its own net erosion, so each cell carries `c = G·area/Q` on its diagonal (Q3 = 1 cell,
/// Q1 = 1.25 cells), and cell 1 also gets a quarter of cell 3's net erosion:
///
/// ```text
/// h3·(1 + c3 + F31 + F30) = h3*·(1 + c3) + F31·h1 + F30·h0
/// h1·(1 + c1 + F10)       = 10·(1 + c1) + G·Qs_in(1)/Q1 + F10·h0,   Qs_in(1) = 0.25·(h3* − h3)·area
/// ```
///
/// so `G·Qs_in(1)/Q1 = 0.2·G·(h3* − h3)`. Every receiver here lies below its donor's new height.
/// Returns `(h1, h3)`.
fn square_by_hand(g: f64, h0: f64, h3: f64) -> (f64, f64) {
    let f10 = K * libm::pow(1.25e4, 0.5) * 1e5 / 100.0;
    let (f31, f30) = (0.25, 0.75 / SQRT_2);
    let (c1, c3) = (g / 1.25, g);
    let (e1, e3) = (1.0 + c1 + f10, 1.0 + c3 + f31 + f30);
    let lifted3 = h3 * (1.0 + c3) + f30 * h0;
    let new1 = (10.0 * (1.0 + c1) + 0.2 * g * h3 + f10 * h0 - 0.2 * g * lifted3 / e3)
        / (e1 + 0.2 * g * f31 / e3);
    let new3 = (lifted3 + f31 * new1) / e3;
    (new1, new3)
}

#[test]
fn the_gauss_seidel_step_follows_the_scheme() {
    for (g, h3) in [(1.0, 30.0), (6.0, 200.0)] {
        let mut w = square([0.0, 10.0, 20.0, h3], no_fields());
        let params = format!("g = {g}");
        let report = evolve(&mut w, &processes(&fluvial_only(&params)), 1, 1e5, 1).unwrap();
        assert!(report.findings.is_empty(), "{g}: {:?}", report.findings);
        let (new1, new3) = square_by_hand(g, 0.0, h3);
        let h = layer(&w, "height");
        // Without uplift the tolerance is 1e-6 m (spec §8), and the iteration contracts fast.
        assert!((h[1] - new1).abs() < 1e-6, "G = {g}: {} vs {new1}", h[1]);
        assert!((h[3] - new3).abs() < 1e-6, "G = {g}: {} vs {new3}", h[3]);
        assert_eq!(h[0].to_bits(), 0.0_f64.to_bits(), "the sink");
        assert_eq!(h[2].to_bits(), 20.0_f64.to_bits(), "the fixed cell");
        // Everything reaches the sink, which keeps its height: the sediment leaves there.
        let out = (10.0 - h[1] + h3 - h[3]) * CELL_AREA;
        let reported = value(&report, "sediment_out_m3");
        assert!((reported - out).abs() <= 1e-9 * out, "{reported} vs {out}");
        assert!(value(&report, "iterations") >= 2.0);
        let (erosion, deposition) = (layer(&w, "erosion"), layer(&w, "deposition"));
        if g > 5.0 {
            // Cell 1 gets more sediment from the steep cell 3 than it erodes: it rises.
            assert!(new1 > 10.0, "{new1}");
            assert!((deposition[1] - (new1 - 10.0)).abs() < 1e-6);
            assert_eq!(erosion[1].to_bits(), 0.0_f64.to_bits());
        } else {
            assert!(new1 < 10.0, "{new1}");
            assert!((erosion[1] - (10.0 - new1)).abs() < 1e-6);
            assert_eq!(deposition[1].to_bits(), 0.0_f64.to_bits());
        }
    }
}

/// A step that the iteration limit stops returns the plain sweep of its last iteration, as
/// `deposit` documents, not a relaxed iterate (a carry-over of Task 27b). With two iterations on
/// the square world the relaxation is on at the second: its residual would not reach the
/// tolerance in the iterations left. Both sweeps are solved by hand: the first from `h*`, where no
/// sediment reaches cell 1 yet, the second with a quarter of what cell 3 lost in the first.
#[test]
fn a_step_stopped_by_the_iteration_limit_returns_its_last_plain_sweep() {
    const G: f64 = 6.0;
    let mut w = square([0.0, 10.0, 20.0, 200.0], no_fields());
    let procs = processes(&fluvial_only("g = 6.0\nmax_iterations = 2"));
    let report = evolve(&mut w, &procs, 1, 1e5, 1).unwrap();
    let [Finding { message, .. }] = report.findings.as_slice() else {
        panic!("the limit stops the step: {:?}", report.findings);
    };
    assert!(message.contains("after 2 iterations"), "{message}");
    let f10 = K * libm::pow(1.25e4, 0.5) * 1e5 / 100.0;
    let (f31, f30) = (0.25, 0.75 / SQRT_2);
    let (c1, c3) = (G / 1.25, G);
    // One plain sweep with `inflow` metres of sediment reaching cell 1; cell 0 stays at 0 m.
    let sweep = |inflow: f64| {
        let h1 = (10.0 * (1.0 + c1) + inflow) / (1.0 + c1 + f10);
        let h3 = (200.0 * (1.0 + c3) + f31 * h1) / (1.0 + c3 + f31 + f30);
        (h1, h3)
    };
    let (_, first) = sweep(0.0);
    // G·Qs_in(1)/Q1 with Qs_in(1) = 0.25·(200 − h3)·area and Q1 = 1.25·area.
    let (h1, h3) = sweep(0.2 * G * (200.0 - first));
    let h = layer(&w, "height");
    assert!((h[1] - h1).abs() < 1e-9, "{} vs {h1}", h[1]);
    assert!((h[3] - h3).abs() < 1e-9, "{} vs {h3}", h[3]);
}

#[test]
fn a_receiver_below_the_deposit_takes_its_share() {
    // Cell 0 (11 m) stands above cell 1's h* (10 m). Cell 3's sediment lifts cell 1 above 11 m,
    // so it erodes toward cell 0 after all: the step ends between the receiver and the height
    // without incision, not at the latter (which comparing with h* alone would give).
    let mut w = square([11.0, 10.0, 20.0, 100.0], no_fields());
    evolve(&mut w, &processes(&fluvial_only("g = 1.0")), 1, 1e5, 1).unwrap();
    let (new1, new3) = square_by_hand(1.0, 11.0, 100.0);
    let h = layer(&w, "height");
    assert!(new1 > 11.0, "the active set of the hand solution: {new1}");
    assert!((h[1] - new1).abs() < 1e-6, "{} vs {new1}", h[1]);
    assert!((h[3] - new3).abs() < 1e-6, "{} vs {new3}", h[3]);
    // c1 = 0.8: without incision cell 1 reaches (10·1.8 + 0.2·(100 − h3)) / 1.8.
    let without_incision = (18.0 + 0.2 * (100.0 - h[3])) / 1.8;
    assert!(
        h[1] < without_incision - 0.5,
        "{} vs {without_incision}",
        h[1]
    );
}

/// A 1 x 4 strip of 100 m cells (synthetic), each draining to its western neighbour:
/// - cell 0, the fixed outlet at 0 m;
/// - cell 1 at 50 m, the higher cell of the pass, so the spill cell lies outside the lake;
/// - cell 2, a lake at 45 m (depression 1, spilling from cell 1 to cell 0 at 50 m);
/// - cell 3, a slope at 100 m draining into the lake.
pub(super) fn pass_strip() -> World {
    let mut w = world(1, 4);
    let grid = |v: Vec<f64>| Layer::F64(Grid::from_vec(1, 4, v).unwrap());
    let u32s = |v: Vec<u32>| Layer::U32(Grid::from_vec(1, 4, v).unwrap());
    let fixed = Grid::from_vec(1, 4, vec![true, false, false, false]).unwrap();
    let weights = FlowWeights {
        start: vec![0, 0, 1, 2, 3],
        to: vec![0, 1, 2],
        weight: vec![1.0; 3],
        dist: vec![100.0; 3],
    };
    let depressions = Depressions {
        label: Grid::from_vec(1, 4, vec![0, 0, 1, 0]).unwrap(),
        spill: vec![SpillPoint {
            cell: 1,
            to: 0,
            level: 50.0,
        }],
    };
    let layers = [
        ("height", grid(vec![0.0, 50.0, 45.0, 100.0])),
        (
            "routing_surface",
            grid(vec![0.0, 50.0, 50.0 + EPSILON, 100.0]),
        ),
        ("erodibility", grid(vec![K; 4])),
        ("erosion", grid(vec![0.0; 4])),
        ("deposition", grid(vec![0.0; 4])),
        ("fixed", Layer::Mask(fixed)),
        ("receivers", u32s(vec![0, 0, 1, 2])),
        ("stack", u32s(vec![0, 1, 2, 3])),
        ("weights", Layer::Value(Box::new(weights))),
        ("depressions", Layer::Value(Box::new(depressions))),
        ("climate", Layer::Value(Box::new(no_fields()))),
    ];
    for (name, layer) in layers {
        w.insert(name, layer).unwrap();
    }
    w
}

/// Spec §5.9: the excess of a depression "passes on from the spill node", whose `Qs` is the
/// flux leaving it. When the spill cell lies outside the lake (the higher cell of the pass on
/// the basin's side), what the lake lets go counts in that cell's own deposition term, as it
/// would at `SpillPoint.to` were the spill cell inside the lake. The full lake starts each sweep
/// at its place one ε above the spill level and is then solved like every cell, so it follows
/// its pass cell down instead of perching above it (physics test 5).
///
/// Solved by hand (G = 1, Δt = 1e5, K = 1e-5, m = 0.5, no uplift): `c3 = 1`, `F32 = 1`;
/// `c2 = 1/2`, `F21 = √2`; `c1 = 1/3`, `F10 = √3`; the lake starts at `s2 = 50 + ε`, full.
///
/// ```text
/// h3 = (200 + h2)/3
/// h2 = (1.5·s2 + √2·h1)/(1.5 + √2)
/// h1·(4/3 + √3) = 200/3 + (1/3)·[(100 − h3) + (45 − h2)]
/// ```
///
/// The last line's bracket is the release: what reaches the lake, `(100 − h3) + (s2 − h2)` of
/// one cell, less what fills it, `5 + ε`. Substituting gives `h1 = 24.7906`, `h2 = 37.7664` and
/// `h3 = 79.2555`. Holding the lake at its start instead left it at `50 + ε`, 26.8 m above its
/// pass cell at 23.2 m.
#[test]
fn a_full_lake_follows_its_pass_cell_and_feeds_its_deposit() {
    let mut w = pass_strip();
    let report = evolve(&mut w, &processes(&fluvial_only("g = 1.0")), 1, 1e5, 1).unwrap();
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    let (r2, r3) = (SQRT_2, 3.0_f64.sqrt());
    let s2 = 50.0 + EPSILON;
    let h1 = (200.0 / 3.0 + 235.0 / 9.0 - (4.0 / 9.0) * 1.5 * s2 / (1.5 + r2))
        / (4.0 / 3.0 + r3 + (4.0 / 9.0) * r2 / (1.5 + r2));
    let h2 = (1.5 * s2 + r2 * h1) / (1.5 + r2);
    let h3 = (200.0 + h2) / 3.0;
    assert!((h1 - 24.790_607).abs() < 1e-6, "{h1}");
    assert!((h2 - 37.766_351).abs() < 1e-6, "{h2}");
    assert!((h3 - 79.255_450).abs() < 1e-6, "{h3}");
    let h = layer(&w, "height");
    assert!((h[3] - h3).abs() < 1e-6, "{} vs {h3}", h[3]);
    assert!((h[2] - h2).abs() < 1e-6, "{} vs {h2}", h[2]);
    assert!((h[1] - h1).abs() < 1e-6, "{} vs {h1}", h[1]);
    // The lake is full: what reaches it fills it to its place one ε above the spill level.
    let reached = (100.0 - h3) + (s2 - h2);
    assert!(reached > 5.0 + EPSILON, "{reached} m of one cell");
    // Without the release cell 1 would reach only the blind height.
    let blind = 50.0 * (4.0 / 3.0) / (4.0 / 3.0 + r3);
    assert!(
        h[1] > blind + 1.0,
        "{} vs {blind} without the release",
        h[1]
    );
    assert!(h[1] < h[2] && h[2] < 45.0, "the lake follows: {h:?}");
    // What left is the strip's lost volume: the slope's, the lake's and cell 1's erosion,
    // 531,875.9 m³ by hand.
    let out = (100.0 - h[3] + 45.0 - h[2] + 50.0 - h[1]) * CELL_AREA;
    let reported = value(&report, "sediment_out_m3");
    assert!((reported - out).abs() <= 1e-9 * out, "{reported} vs {out}");
    let by_hand = (100.0 - h3 + 45.0 - h2 + 50.0 - h1) * CELL_AREA;
    assert!((reported - by_hand).abs() < 3e-6 * CELL_AREA, "{reported}");
}

/// The Chebyshev distance from `(r, c)` to the bowl's centre (32, 32).
fn ring(r: usize, c: usize) -> usize {
    r.abs_diff(32).max(c.abs_diff(32))
}

/// The bowl's rim, high enough that the uplifted catchment never tops it.
const RIM: f64 = 1000.0;

/// The bowl's notch, its spill level.
const NOTCH: f64 = 150.0;

/// The uplift rate of the bowl's catchment, m/yr.
const CATCHMENT_UPLIFT: f64 = 1e-4;

/// The bowl's initial height at `(r, c)`.
fn bowl_height(r: usize, c: usize) -> f64 {
    let lateral = 0.5 * to_f64(c.abs_diff(32));
    match ring(r, c) {
        d if d <= 18 => 306.0 - 4.0 * to_f64(r) + lateral,
        19 if (r, c) == (51, 32) => NOTCH,
        19 => RIM,
        _ => 2.0 * to_f64(63 - r) + lateral,
    }
}

/// A 64 x 64 world of 100 m cells with the south edge drained (synthetic):
/// - a square bowl (rows and columns 14..=50) whose floor falls 4 m per row to the south, from
///   250 m, and rises 0.5 m per column away from column 32;
/// - a rim at [`RIM`] around it with a notch at [`NOTCH`] on its south side (51, 32), so the
///   bowl's cells below [`NOTCH`] form one closed depression;
/// - the bowl's northern part (rows 14..=34) uplifts at [`CATCHMENT_UPLIFT`] and erodes with K;
///   the rest neither uplifts nor erodes (K = 0), so the only sediment comes from the catchment;
/// - outside, the land falls 2 m per row to the south edge.
pub(super) fn bowl() -> World {
    let mut w = world(N, N);
    run_stage(&mut w, &GridStage, r#"edges = ["south"]"#).unwrap();
    let grid = |f: &dyn Fn(usize, usize) -> f64| {
        Layer::F64(Grid::from_vec(N, N, (0..N * N).map(|i| f(i / N, i % N)).collect()).unwrap())
    };
    let inside = |r: usize, c: usize| ring(r, c) <= 18;
    let catchment = |r: usize, c: usize| inside(r, c) && r <= 34;
    let fixed = w.mask("fixed").unwrap().clone();
    w.insert(
        "height",
        grid(&|r, c| {
            if fixed.as_slice()[r * N + c] {
                0.0
            } else {
                bowl_height(r, c)
            }
        }),
    )
    .unwrap();
    w.insert(
        "uplift",
        grid(&|r, c| {
            if catchment(r, c) {
                CATCHMENT_UPLIFT
            } else {
                0.0
            }
        }),
    )
    .unwrap();
    w.insert("sag_rate", grid(&|_, _| 0.0)).unwrap();
    w.insert(
        "erodibility",
        grid(&|r, c| if inside(r, c) { K } else { 0.0 }),
    )
    .unwrap();
    w.insert("erosion", grid(&|_, _| 0.0)).unwrap();
    w.insert("deposition", grid(&|_, _| 0.0)).unwrap();
    w.insert("climate", Layer::Value(Box::new(no_fields())))
        .unwrap();
    w
}

/// The bowl's cells below its spill level at the start.
pub(super) fn bowl_cells() -> Vec<usize> {
    (0..N * N)
        .filter(|&i| ring(i / N, i % N) <= 18 && bowl_height(i / N, i % N) < NOTCH)
        .collect()
}

/// Physics test 5 (spec §5.14): sediment routed into a closed depression fills it to its spill
/// level and passes the rest on.
///
/// The bowl's catchment is the only source of sediment. While the bowl fills, nothing leaves the
/// map (to rounding). Its floor fills; cells above the water may erode first, where the
/// catchment's river crosses them toward the lake, so each cell's booked deposition less its
/// booked erosion is its net change. In the step the bowl fills, every cell below the notch
/// reaches the spill level on its ε slope, and the rest of the step's sediment leaves over the
/// notch; so it does in the steps after.
#[test]
fn sediment_fills_a_closed_depression_and_passes_the_excess() {
    const DT: f64 = 2.5e4;
    let cells = bowl_cells();
    for exponent in BOTH {
        let mut w = bowl();
        let start = layer(&w, "height");
        let procs = processes(&recipe(exponent, "g = 1.0"));
        let (mut full, mut places) = (None, 0);
        for step in 0..60 {
            let report = evolve(&mut w, &procs, 1, DT, 1).unwrap();
            let out = value(&report, "sediment_out_m3");
            // The step's depression also holds the cells the resolver's ε ramp lifts and the
            // catchment's channels that cut below the notch; each takes a place on the ε slope.
            let label = &w.value::<Depressions>("depressions").unwrap().label;
            let one = label.as_slice()[cells[0]];
            assert!(
                one > 0 && cells.iter().all(|&c| label.as_slice()[c] == one),
                "{exponent}, step {step}: the bowl is one depression"
            );
            places = label.as_slice().iter().filter(|&&n| n == one).count();
            let h = layer(&w, "height");
            if cells.iter().all(|&c| h[c] >= NOTCH) {
                full = Some((step, out));
                break;
            }
            assert!(
                out.abs() < 1e-3,
                "{exponent}, step {step}: {out} m³ left the map"
            );
        }
        let Some((step, out)) = full else {
            panic!("{exponent}: the bowl never filled");
        };
        assert!(
            step > 0,
            "{exponent}: the bowl should take several steps to fill"
        );
        assert!(
            out > 1e3,
            "{exponent}, step {step}: the excess leaves ({out} m³)"
        );
        let (h, deposition, erosion) = (
            layer(&w, "height"),
            layer(&w, "deposition"),
            layer(&w, "erosion"),
        );
        let slope = EPSILON * to_f64(places);
        for &c in &cells {
            assert!(
                h[c] >= NOTCH && h[c] <= NOTCH + slope,
                "{exponent}: bowl cell {c} at {} m",
                h[c]
            );
            let gained = h[c] - start[c];
            assert!(
                (deposition[c] - erosion[c] - gained).abs() <= 1e-9 * gained,
                "{exponent}: cell {c}"
            );
        }
        for later in 1..=5 {
            let report = evolve(&mut w, &procs, 1, DT, 1).unwrap();
            let out = value(&report, "sediment_out_m3");
            assert!(out > 1e3, "{exponent}, {later} steps after: {out} m³");
        }
    }
}

/// Spec ruling 29: G = 0 neither deposits nor fills. The bowl's sediment passes through it from
/// the first step, and no cell of the map rises above its height plus its uplift.
#[test]
fn g_zero_switches_off_deposition_and_filling() {
    const DT: f64 = 2.5e4;
    let cells = bowl_cells();
    for exponent in BOTH {
        let mut w = bowl();
        let procs = processes(&recipe(exponent, "g = 0.0"));
        for step in 0..10 {
            let before = layer(&w, "height");
            let report = evolve(&mut w, &procs, 1, DT, 1).unwrap();
            let h = layer(&w, "height");
            assert!(
                value(&report, "sediment_out_m3") > 0.0,
                "{exponent}, step {step}: the bowl keeps nothing"
            );
            for &c in &cells {
                assert!(
                    h[c] <= before[c],
                    "{exponent}, step {step}: bowl cell {c} rose"
                );
            }
        }
        assert!(
            layer(&w, "deposition").iter().all(|d| d.to_bits() == 0),
            "{exponent}"
        );
    }
}

/// Physics test 4 (spec §5.14): over a run with deposition and filling, the change in volume is
/// the uplifted volume less the sediment that left at the drained edges, taken from the routed
/// flux (`sediment_out_m3`), to 1e-9 relative.
#[test]
fn mass_balance_holds_with_deposition() {
    const DT: f64 = 2.5e4;
    const STEPS: u32 = 50;
    for edges in [ALL_EDGES, r#"["south"]"#] {
        for exponent in BOTH {
            let mut w = landscape(edges, 7);
            let fixed = w.mask("fixed").unwrap().as_slice().to_vec();
            let start = layer(&w, "height");
            let procs = processes(&recipe(exponent, "g = 1.0"));
            let report = evolve(&mut w, &procs, STEPS, DT, 1).unwrap();
            let out = value(&report, "sediment_out_m3");
            let lifted = fixed.iter().filter(|&&f| !f).count();
            let uplifted =
                to_f64(lifted) * (0..STEPS).fold(0.0, |sum, _| sum + UPLIFT * DT) * CELL_AREA;
            let change = volume(&layer(&w, "height")) - volume(&start);
            let at = format!("{edges}, {exponent}");
            assert!(
                (change - (uplifted - out)).abs() <= 1e-9 * uplifted,
                "{at}: change {change}, uplifted {uplifted}, out {out}"
            );
            assert!(out > 0.0, "{at}: sediment leaves");
            let deposition = layer(&w, "deposition");
            assert!(
                deposition.iter().any(|&d| d > 1.0),
                "{at}: something is deposited"
            );
            assert!(deposition.iter().all(|&d| d >= 0.0), "{at}");
        }
    }
}

/// Physics test 2 with G > 0 (spec §5.14): the run of the G = 0 stability test, at ten times
/// the default time step with G = 1, converges within the iteration limit every step and stays
/// finite.
#[test]
fn deposition_converges_at_ten_times_the_time_step() {
    const DT: f64 = 2.5e5;
    for edges in [ALL_EDGES, r#"["south"]"#] {
        let mut w = landscape(edges, 2);
        let procs = processes(&recipe("steepest", "g = 1.0"));
        for step in 0..60 {
            let report = evolve(&mut w, &procs, 1, DT, 1).unwrap();
            assert!(
                report.findings.is_empty(),
                "{edges}, step {step}: {:?}",
                report.findings
            );
            let h = layer(&w, "height");
            assert!(h.iter().all(|v| v.is_finite()), "{edges}, step {step}");
        }
    }
}

/// Spec §8: with G/p̂ above 1 convergence is not proven. At G = 4 the run either converges or
/// stops at the 50-iteration limit with one warning that carries the residual; either way every
/// height is finite and two runs are identical.
#[test]
fn strong_deposition_stays_finite_at_the_iteration_limit() {
    const DT: f64 = 2.5e4;
    for exponent in BOTH {
        let run = || {
            let mut w = landscape(ALL_EDGES, 3);
            let report =
                evolve(&mut w, &processes(&recipe(exponent, "g = 4.0")), 10, DT, 1).unwrap();
            (layer(&w, "height"), layer(&w, "deposition"), report)
        };
        let (h, deposition, report) = run();
        assert!(h.iter().all(|v| v.is_finite()), "{exponent}");
        assert!(deposition.iter().any(|&d| d > 0.0), "{exponent}");
        match report.findings.as_slice() {
            [] => {}
            [Finding { message, .. }] => {
                assert!(
                    message.contains("50 iterations") && message.contains("residual"),
                    "{exponent}: {message}"
                );
            }
            more => panic!("{exponent}: {more:?}"),
        }
        let (again, again_deposition, again_report) = run();
        let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&h), bits(&again), "{exponent}");
        assert_eq!(bits(&deposition), bits(&again_deposition), "{exponent}");
        assert_eq!(report, again_report, "{exponent}");
    }
}

/// Runoff that varies as the climate model's does across a range (synthetic): wet on the
/// windward west, falling to a dry lee in the east where evapotranspiration takes almost all
/// the rain. `p̂` runs from about 3 in the west to 0.005 in the east, below 0.5 on two fifths of
/// the map (the 384² budget recipe: 0.005 to 4.1, below 0.5 on three tenths).
fn rain_shadow() -> Grid<f64> {
    let data = (0..N * N)
        .map(|i| {
            let east = to_f64(i % N) / to_f64(N - 1);
            0.005 + 3.0 * (1.0 - east) * (1.0 - east)
        })
        .collect();
    Grid::from_vec(N, N, data).unwrap()
}

/// Spec §5.9 with a climate that varies: where runoff is low, the catchment-mean `p̂` of a
/// patch of cells falls below G, and Yuan et al.'s convergence condition `G/p̂ ≤ 1` fails. The
/// lagged sediment flux then drives the whole dry patch up and down together from one
/// iteration to the next: the iteration's dominant eigenvalue is negative and, with the
/// erodibility of soft rock (shale, alluvium) at the default time step, below −1. Every step
/// still converges within the iteration limit and stays finite (Task 27b: before the relaxation
/// both erodibilities stopped at 50 iterations from step 1 on, the alluvium one diverging).
#[test]
fn dry_catchments_converge_at_the_default_time_step() {
    const DT: f64 = 2.5e4;
    for erodibility in [2e-4, 4e-4] {
        for exponent in BOTH {
            let mut w = landscape(r#"["south"]"#, 4);
            w.insert(
                "climate",
                Layer::Value(Box::new(super::with_runoff(rain_shadow()))),
            )
            .unwrap();
            w.insert("erodibility", Layer::F64(Grid::filled(N, N, erodibility)))
                .unwrap();
            let procs = processes(&recipe(exponent, "g = 1.0"));
            for step in 0..40 {
                let report = evolve(&mut w, &procs, 1, DT, 1).unwrap();
                assert!(
                    report.findings.is_empty(),
                    "K {erodibility}, {exponent}, step {step}: {:?}",
                    report.findings
                );
                let h = layer(&w, "height");
                assert!(
                    h.iter().all(|v| v.is_finite()),
                    "K {erodibility}, {exponent}, step {step}"
                );
            }
        }
    }
}

/// Spec §8 and ruling 43: without uplift the Gauss–Seidel tolerance is 1e-6 m; with uplift it is
/// Yuan et al.'s 1e-3 × max(U)·Δt. Two iterations are too few for either, so the warning names
/// the tolerance in use; with the default limit the uplift-free step converges.
#[test]
fn uplift_free_runs_use_the_fallback_tolerance() {
    const DT: f64 = 2.5e4;
    let step = |uplift: bool, limit: &str| {
        let mut w = landscape(ALL_EDGES, 5);
        if !uplift {
            w.remove("uplift").unwrap();
        }
        let fluvial = format!("g = 1.0\n{limit}");
        let report = evolve(
            &mut w,
            &processes(&routed("slope_adaptive", &fluvial)),
            1,
            DT,
            1,
        );
        (report.unwrap(), layer(&w, "height"))
    };
    let (report, _) = step(false, "max_iterations = 2");
    let [Finding { message, .. }] = report.findings.as_slice() else {
        panic!("{:?}", report.findings);
    };
    assert!(message.contains("tolerance 1e-6 m"), "{message}");
    let (report, _) = step(true, "max_iterations = 2");
    let [Finding { message, .. }] = report.findings.as_slice() else {
        panic!("{:?}", report.findings);
    };
    let yuan = format!("tolerance {:e} m", 1e-3 * UPLIFT * DT);
    assert!(message.contains(&yuan), "{message} lacks {yuan}");
    let (report, free) = step(false, "");
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    let (_, loose) = step(true, "");
    let iterations = value(&report, "iterations");
    assert!(iterations > 2.0, "{iterations}");
    // The looser tolerance stops earlier, at a solution that differs by more than 1e-6 m.
    let most = free
        .iter()
        .zip(&loose)
        .fold(0.0, |m, (a, b)| max(m, (a - b).abs()));
    assert!(most > 1e-6 && most < 1.0, "{most}");
}

/// The flow-weighted slope `Σ_r w_r·(h − h_r)/d_r` of every cell of `w` over its weights (0 for
/// a cell without an edge).
fn flow_slopes(w: &World) -> Vec<f64> {
    let h = layer(w, "height");
    let weights = w.value::<FlowWeights>("weights").unwrap();
    (0..N * N)
        .map(|i| {
            weights
                .edges(i)
                .map(|e| {
                    weights.weight[e] * (h[i] - h[cell_index(weights.to[e])]) / weights.dist[e]
                })
                .sum()
        })
        .collect()
}

/// The cells of `w` that are not fixed and drain more than `cells` cells.
fn draining_more_than(w: &World, cells: f64) -> Vec<usize> {
    let area = layer(w, "drainage_area");
    let fixed = w.mask("fixed").unwrap().as_slice().to_vec();
    (0..N * N)
        .filter(|&i| !fixed[i] && area[i] > cells * CELL_AREA)
        .collect()
}

/// The channel cells of `w`: not fixed, with a drainage area above ten cells.
fn channel_cells(w: &World) -> Vec<usize> {
    draining_more_than(w, 10.0)
}

/// The highest of `heights` less the lowest.
fn spread(heights: &[f64]) -> f64 {
    let (low, high) = heights
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (min(lo, v), max(hi, v))
        });
    high - low
}

/// The G = 0 steady-state law on the weights and drainage area of `w`: the heights at which
/// every cell's flow-weighted slope is `(U/K)·Q^(−0.5)`, from the fixed cells at 0 m. Solved in
/// stack order, each cell from its receivers: `h_i = (law_i + Σ w·h_r/d) / Σ w/d`.
fn law_heights(w: &World) -> Vec<f64> {
    let weights = w.value::<FlowWeights>("weights").unwrap();
    let area = layer(w, "drainage_area");
    let mut h = vec![0.0; N * N];
    for &cell in w.u32("stack").unwrap().as_slice() {
        let i = cell_index(cell);
        if weights.edges(i).is_empty() {
            continue;
        }
        let slope = law(area[i]);
        let (mut top, mut bottom) = (slope, 0.0);
        for e in weights.edges(i) {
            top += weights.weight[e] * h[cell_index(weights.to[e])] / weights.dist[e];
            bottom += weights.weight[e] / weights.dist[e];
        }
        h[i] = top / bottom;
    }
    h
}

/// Asserts that `actual` is `want` within 2 %.
fn within_two_percent(actual: f64, want: f64, what: &str) {
    assert!(
        (actual / want - 1.0).abs() <= 0.02,
        "{what}: {actual} vs {want}"
    );
}

/// The G = 0 steady-state slope `(U/K)·Q^(−0.5)` at the drainage area `area` (physics test 1).
fn law(area: f64) -> f64 {
    UPLIFT / K * libm::pow(area, -0.5)
}

/// The deposition coefficient G of physics test 14.
const G14: f64 = 1.0;

/// The time step of physics test 14, years.
const DT14: f64 = 2.5e5;

/// The cells the `slope_adaptive` leg's stability check covers drain more than this many cells.
/// Ruling (Jann, 13 Nov 2026): the check applies to channel cells over 22 cells' area, the edge
/// of the divide oscillation's zone that Task 25 measured (cells of 1 to 22 cells' area).
/// Filling the depressions before the solve (Task 27f) picks a network with a stronger case of
/// that oscillation: 0.33 m on cells of 10 to 13 cells against the 0.25 m bound, 0.17 m over 22
/// cells. The bound, the window and every 2 % law check stay.
const STABLE_AREA_CELLS: f64 = 22.0;

/// Asserts that the steady state of `w` (`what`) is the G = 0 law times (1 + G) on its own flow
/// weights: every channel cell's flow-weighted slope is `(1 + G)·(U/K)·Q^(−m)`, and the relief is
/// (1 + G) times the relief of the heights the G = 0 law gives on those weights, within 2 %.
fn follows_the_law_times_one_plus_g(w: &World, what: &str) {
    let (cells, slopes, area) = (channel_cells(w), flow_slopes(w), layer(w, "drainage_area"));
    assert!(cells.len() > 100, "{what}: {} channel cells", cells.len());
    for &i in &cells {
        let cell = format!("{what}: cell {i}");
        within_two_percent(slopes[i], (1.0 + G14) * law(area[i]), &cell);
    }
    let predicted = (1.0 + G14) * spread(&law_heights(w));
    let relief = spread(&layer(w, "height"));
    within_two_percent(relief, predicted, &format!("{what}: relief"));
}

/// `uplift`, the routing with `exponent` and `fluvial` with G = [`G14`] and m = 0.5.
fn deposits(exponent: &str) -> String {
    recipe(exponent, &format!("g = {G14}\nm = 0.5"))
}

/// Test 14's `steepest` leg on the landscape of `seed`: the G = 1 run to its full steady state.
fn steepest_steady_state_is_one_plus_g(seed: u64) {
    let mut w = landscape(ALL_EDGES, seed);
    let steps = run_to_steady_state(&mut w, &deposits("steepest"), DT14, 2000);
    follows_the_law_times_one_plus_g(&w, &format!("steepest, seed {seed}, {steps} steps"));
}

/// Test 14's `slope_adaptive` leg on the landscape of `seed`: a fixed run whose channels over
/// [`STABLE_AREA_CELLS`] cells have stopped moving.
fn slope_adaptive_steady_state_is_one_plus_g(seed: u64) {
    // The fixed run, and the steps at its end whose channels are checked.
    const RUN: u32 = 1500;
    const TRACKED: u32 = 50;
    let mut w = landscape(ALL_EDGES, seed);
    let procs = processes(&deposits("slope_adaptive"));
    evolve(&mut w, &procs, RUN - TRACKED, DT14, 1).unwrap();
    let tolerance = 1e-3 * UPLIFT * DT14;
    let mut wobble = 0.0;
    for _ in 0..TRACKED {
        let before = layer(&w, "height");
        evolve(&mut w, &procs, 1, DT14, 1).unwrap();
        let h = layer(&w, "height");
        wobble = draining_more_than(&w, STABLE_AREA_CELLS)
            .iter()
            .fold(wobble, |most, &i| max(most, (h[i] - before[i]).abs()));
    }
    assert!(
        wobble < tolerance,
        "slope_adaptive, seed {seed}: a channel cell over {STABLE_AREA_CELLS} cells still \
         changes by {wobble} m per step (tolerance {tolerance} m)"
    );
    follows_the_law_times_one_plus_g(&w, &format!("slope_adaptive, seed {seed}, {RUN} steps"));
}

/// Physics test 14 (spec §5.14, rulings 44 and 59; Yuan et al. 2019, eq. 24 and 28): with
/// uniform U, K and runoff, n = 1, m = 0.5 and no hillslope processes, deposition with G = 1
/// steepens the steady state by (1 + G): its slopes and its relief, within 2 %.
///
/// Both flow partitions run with G = 1 from the landscape of seed 1, and each run's steady state
/// is compared with (1 + G) times the G = 0 law on its own flow weights: each channel cell's
/// flow-weighted slope with `(1 + G)·(U/K)·Q^(−m)`, and the relief with (1 + G) times the relief
/// of the heights the G = 0 law gives on those weights. A G = 1 run need not keep a G = 0 run's
/// network (ruling 59, Jann, 17 Nov 2026):
/// - **`steepest`:** steepest receivers do not change when every slope doubles, but on the way
///   ridge cells with nearly equal descents change receiver, and every flip moves the drainage
///   area of the whole trunk below it. A G = 0 run's network survived on only one landscape in
///   32, so the test compares with the law on the run's own final network. This leg requires a
///   full steady state.
/// - **`slope_adaptive`:** the partition exponent `p = 0.5 + 0.6·S` depends on the slope, so
///   doubling the slopes changes the weights, and two runs cannot share a network.
///
///   With G > 0 this partition shows a slow divide oscillation: ridge cells of small drainage
///   area swing by several metres per step, whatever the tolerance or the time step (seed 1
///   before Task 43b: up to 9.5 m per step at Δt = 2.5e5 and 0.47 m at 2.5e4, on cells of 1 to
///   about 20 cells of area). The landscape therefore never reaches a full steady state, and
///   this leg runs a fixed 1 500 steps, about twice the time its channels take to settle. It
///   still demands a steady state beyond the oscillation's reach: over the last 50 steps no
///   channel cell with a drainage area above 22 cells changes by Yuan's tolerance `10⁻³·U·Δt`
///   or more in a step. The 2 % law checks cover every channel cell (above 10 cells).
///
///   This leg rests on seed 1. Its wobble check does not hold on every landscape: on seed 7
///   the same run still changes a channel cell over 22 cells by 0.845 m in a step, against the
///   0.25 m bound (found in the review of Task 43d). The seed is natural, not picked to pass,
///   but the leg proves the law for one landscape, not for the partition on all of them.
#[test]
fn deposition_steepens_steady_state_by_one_plus_g() {
    // A natural seed, not one picked for the test.
    const SEED: u64 = 1;
    steepest_steady_state_is_one_plus_g(SEED);
    slope_adaptive_steady_state_is_one_plus_g(SEED);
}

/// The Gauss–Seidel iterations check for a cancel before every iteration: a token that turns
/// cancelled after two checks stops the solve at the third, with `Cancelled` and no solution,
/// where the same step runs to its tolerance when nothing cancels.
#[test]
fn the_iterations_stop_at_a_cancel() {
    use std::cell::Cell;

    fn dep<'a>(
        traps: &'a Traps,
        supply: &'a [f64],
        cancel: &'a dyn Fn() -> Result<(), StageError>,
    ) -> Deposition<'a> {
        Deposition {
            g: 6.0,
            traps,
            cell_area: CELL_AREA,
            supply,
            // No supply, so none passes.
            passing: supply,
            tolerance: 1e-6,
            max_iterations: 100,
            cancel,
        }
    }

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
    let start = Grid::from_vec(2, 2, vec![0.0, 10.0, 20.0, 200.0]).unwrap();
    let traps = Traps::new(&depressions, start.as_slice(), start.as_slice(), CELL_AREA);
    let step = Incision {
        weights: &weights,
        stack: &[0, 1, 2, 3],
        fixed: &[false, false, true, false],
        erodibility: &[K; 4],
        discharge: &[4e4, 1.25e4, 1e4, 1e4],
        m: 0.5,
        dt: 1e5,
    };
    let supply = [0.0; 4];
    let calls = Cell::new(0_u32);
    let after = |ok: u32| {
        let calls = &calls;
        move || {
            calls.set(calls.get() + 1);
            if calls.get() > ok {
                Err(StageError::Cancelled)
            } else {
                Ok(())
            }
        }
    };
    let cancelling = after(2);
    let cancelled = deposit(&step, &dep(&traps, &supply, &cancelling), &start);
    assert_eq!(cancelled.err(), Some(StageError::Cancelled));
    assert_eq!(
        calls.get(),
        3,
        "checked before each iteration, stopped at the third"
    );
    calls.set(0);
    let free = after(u32::MAX);
    let solution = deposit(&step, &dep(&traps, &supply, &free), &start).unwrap();
    assert!(
        solution.iterations > 2,
        "{} iterations",
        solution.iterations
    );
    assert_eq!(calls.get(), solution.iterations, "one check per iteration");
}
