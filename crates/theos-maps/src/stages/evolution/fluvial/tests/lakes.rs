//! `fluvial`'s routed depressions with deposition (spec §5.9 "Depression filling", physics test
//! 5): each Gauss–Seidel iteration fills them with the sediment that reaches them first, then
//! solves every cell, their cells included. A lake that is not full keeps its level and the slope
//! that feeds it ends above it; a full lake follows its spill node and never perches above it.
//!
//! Every world here is a synthetic strip of 100 m cells whose west edge drains: cell 0 is the
//! fixed outlet and every cell drains west.

use super::deposition::{routed, value, volume};
use super::{CELL_AREA, layer, no_fields, recipe};
use crate::grid::Grid;
use crate::kernels::num::{max, to_f64};
use crate::stage::{Finding, StageReport};
use crate::stages::evolution::resolve_sinks::Depressions;
use crate::stages::evolution::test_support::{evolve, processes};
use crate::stages::terrain::grid::GridStage;
use crate::stages::terrain::support::{run_stage, world};
use crate::world::{Layer, World};

/// A row of `values` over a strip as long as it.
fn row(values: Vec<f64>) -> Layer {
    let n = values.len();
    Layer::F64(Grid::from_vec(1, n, values).unwrap())
}

/// A synthetic 1 x n strip of 100 m cells at `height`, with the erodibility `erodibility`, whose
/// west edge drains (cell 0 is fixed); no erosion or deposition yet and a climate without fields.
fn strip(height: Vec<f64>, erodibility: Vec<f64>) -> World {
    let mut w = world(1, height.len());
    run_stage(&mut w, &GridStage, r#"edges = ["west"]"#).unwrap();
    let zeros = vec![0.0; height.len()];
    w.insert("height", row(height)).unwrap();
    w.insert("erodibility", row(erodibility)).unwrap();
    w.insert("erosion", row(zeros.clone())).unwrap();
    w.insert("deposition", row(zeros)).unwrap();
    w.insert("climate", Layer::Value(Box::new(no_fields())))
        .unwrap();
    w
}

/// The cells of [`incising_strip`].
const CELLS: usize = 16;

/// The cell of [`incising_strip`] that uplifts slower than the rest.
const SLOW: usize = 8;

/// The time step of [`incising_strip`], years.
const DT: f64 = 2.5e4;

/// The steps [`incising_strip`] runs.
const STEPS: usize = 120;

/// The uplift rate of [`incising_strip`]'s cell `i`, m/yr.
fn strip_uplift(i: usize) -> f64 {
    if i == SLOW { 4e-4 } else { 1e-3 }
}

/// A synthetic 1 x 16 strip rising 2 m per cell from the outlet, K = 1e-4. Every cell uplifts
/// 1 mm/yr but cell [`SLOW`], which uplifts 0.4 mm/yr: after uplift it lies below its downstream
/// neighbour in every step, so `resolve_sinks` routes it as a depression every step, and the
/// river that cuts through the strip passes it as a lake.
fn incising_strip() -> World {
    let height = (0..CELLS).map(|i| 2.0 * to_f64(i)).collect();
    let mut w = strip(height, vec![1e-4; CELLS]);
    w.insert("uplift", row((0..CELLS).map(strip_uplift).collect()))
        .unwrap();
    w.insert("sag_rate", row(vec![0.0; CELLS])).unwrap();
    w
}

/// `uplift`, the routing with steepest descent, then `fluvial` with G = 1, m = 0.5.
fn incising_recipe() -> String {
    recipe("steepest", "g = 1.0\nm = 0.5")
}

/// Physics test 5, second half (spec §5.14): a full lake follows the incision of its spill node
/// instead of perching above it (Task 27e's H6).
///
/// On [`incising_strip`] the river keeps cutting down to the outlet while the slow cell's pit
/// fills every step. Holding the lake's cells at their height through the iterations and
/// filling them afterwards left the lake perched above the cell it spills into; the river below
/// then saw a different base level every other step and the strip cycled with period 2 (13 of
/// the 16 cells alternating, heights swinging by up to 17 m between steps). Filled first and
/// then solved with the other cells, the lake follows its outlet and the strip reaches a steady
/// state: over the last 20 of 120 steps the slow cell is a routed depression in every step and
/// no height changes by 1e-9 m from one step to the next.
#[test]
fn a_lake_on_an_incising_river_does_not_perch() {
    let mut w = incising_strip();
    let procs = processes(&incising_recipe());
    let mut last = layer(&w, "height");
    for step in 0..STEPS {
        let report = evolve(&mut w, &procs, 1, DT, 1).unwrap();
        assert!(
            report.findings.is_empty(),
            "step {step}: {:?}",
            report.findings
        );
        let h = layer(&w, "height");
        if step >= STEPS - 20 {
            let label = &w.value::<Depressions>("depressions").unwrap().label;
            assert!(
                label.as_slice()[SLOW] > 0,
                "step {step}: cell {SLOW} is no routed depression"
            );
            let moved = h
                .iter()
                .zip(&last)
                .fold(0.0, |most, (a, b)| max(most, (a - b).abs()));
            assert!(
                moved < 1e-9,
                "step {step}: a height changed by {moved} m: {h:?}"
            );
        }
        last = h;
    }
}

/// The strip of physics test 5's first half (synthetic): the outlet at 0 m, a sill at 50 m that
/// does not erode (K = 0), a pit at 20 m, and a slope at 60 m that drains into it; K = 1e-4
/// elsewhere. No uplift, so the tolerance is 1e-6 m.
fn lake_strip() -> World {
    strip(vec![0.0, 50.0, 20.0, 60.0], vec![1e-4, 0.0, 1e-4, 1e-4])
}

/// One step of 1e5 years of the routing and `fluvial` with G = 1, m = 0.5 and `extra`.
fn lake_step(w: &mut World, extra: &str) -> StageReport {
    let fluvial = format!("g = 1.0\nm = 0.5\n{extra}");
    evolve(w, &processes(&routed("steepest", &fluvial)), 1, 1e5, 1).unwrap()
}

/// Physics test 5, first half (spec §5.14), solved by hand: sediment routed into a closed
/// depression fills it, and the slope that feeds a lake that is not full ends above it.
///
/// On [`lake_strip`] the pit (cell 2) is a depression that spills over the sill at 50 m and
/// holds 30 m of one cell. The slope (cell 3) drains one cell into it: `F32 = K·√area·Δt/d = 10`
/// and `c3 = G = 1`, so `h3 = (120 + 10·h2)/12`. The lake has no receiver below it (the sill
/// stands higher), so it keeps its filled height, `h2 = 20 + f`, and what fills it is what the
/// slope lost: `f = 60 − h3`. Together `22·f = 400`: `f = 18.1818 m`, the lake at 38.1818 m and
/// the slope at 41.8182 m, above it. Nothing passes the sill. Holding the lake at 20 m through
/// the iterations instead lowered the slope to 26.7 m, and the fill afterwards topped the lake
/// at the sill: a slope 23 m below the lake it drains into.
#[test]
fn a_lake_not_full_rises_to_what_fills_it_and_its_slope_stays_above() {
    let mut w = lake_strip();
    let report = lake_step(&mut w, "");
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    let label = &w.value::<Depressions>("depressions").unwrap().label;
    assert_eq!(label.as_slice(), [0, 0, 1, 0], "the pit is the depression");
    let f = 400.0 / 22.0;
    let h = layer(&w, "height");
    assert!((h[2] - (20.0 + f)).abs() < 1e-6, "lake at {}", h[2]);
    assert!((h[3] - (60.0 - f)).abs() < 1e-6, "slope at {}", h[3]);
    // The sill does not erode; its solve, `50·(1 + c)/(1 + c)`, rounds within an ulp.
    assert!((h[1] - 50.0).abs() < 1e-9, "the sill at {}", h[1]);
    assert!(h[3] > h[2], "the slope ends above the lake");
    let out = value(&report, "sediment_out_m3");
    assert!(
        out.abs() <= 1e-6 * f * CELL_AREA,
        "{out} m³ passed the sill"
    );
    let (deposition, erosion) = (layer(&w, "deposition"), layer(&w, "erosion"));
    assert!((deposition[2] - f).abs() < 1e-6, "{}", deposition[2]);
    assert!((erosion[3] - f).abs() < 1e-6, "{}", erosion[3]);
}

/// A synthetic 1 x 7 strip with two pits in one closed basin: the outlet at 0 m, a sill at 50 m,
/// an outer pit at 10 m, an inner pass at 30 m, an inner pit at 20 m, its slope at 40 m and a
/// rim at 60 m. The inner pit spills over its pass at 30 m into the outer pit, which spills over
/// the sill at 50 m, so water stands over both pits in one lake at 50 m, which holds 100 m of one
/// cell. K = 0: nothing erodes.
fn nested_strip() -> World {
    strip(vec![0.0, 50.0, 10.0, 30.0, 20.0, 40.0, 60.0], vec![0.0; 7])
}

/// Physics test 5 for a pit nested in a lake (spec §5.9 "Depression filling"): sediment that
/// reaches a closed basin stays in it while the basin has room below its spill level.
///
/// On [`nested_strip`], 80 m of one cell of slope supply on the inner slope (cell 5) reach the
/// lake, which holds 100 m. The lake keeps all of it, and its four cells fill in height order to
/// one level ℓ (each with its ε slope): (ℓ − 10) + (ℓ − 30) + (ℓ − 20) + (ℓ − 40) = 80, so
/// ℓ = 45 m. Nothing passes the sill, and the sill keeps its height. When the inner pit counted
/// as a depression of its own that fills only to its pass at 30 m, its slope at 40 m was never
/// filled; the outer pit then filled to the sill and let 10 m of one cell over it while 30 m of
/// the lake's room stayed empty, and the closed basin lost sediment (Task 43e).
#[test]
fn a_pit_nested_in_a_lake_fills_with_the_lake_and_keeps_its_sediment() {
    let mut w = nested_strip();
    let start = layer(&w, "height");
    let supply = vec![0.0, 0.0, 0.0, 0.0, 0.0, 80.0, 0.0];
    w.insert("slope_supply", row(supply)).unwrap();
    let report = lake_step(&mut w, "");
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    let supplied = 80.0 * CELL_AREA;
    let out = value(&report, "sediment_out_m3");
    assert!(out.abs() <= 1e-9 * supplied, "{out} m³ passed the sill");
    let h = layer(&w, "height");
    let kept: f64 = (2..6).map(|c| (h[c] - start[c]) * CELL_AREA).sum();
    assert!(
        (kept - supplied).abs() <= 1e-9 * supplied,
        "the lake kept {kept} of {supplied} m³"
    );
    for (c, lake) in h.iter().enumerate().take(6).skip(2) {
        assert!(
            (lake - 45.0).abs() < 1e-5,
            "cell {c} at {lake}, not at 45 m"
        );
    }
    assert!((h[1] - 50.0).abs() < 1e-9, "the sill at {}", h[1]);
    let label = &w.value::<Depressions>("depressions").unwrap().label;
    assert_eq!(
        label.as_slice(),
        [0, 0, 1, 1, 1, 1, 0],
        "one lake over both pits"
    );
}

/// Physics test 4's balance with supply (spec §5.14) while lakes fill and follow: over 120 steps
/// of [`incising_strip`], with 1 m of slope supply on cells 10 to 15 in each of the first 10
/// steps, the strip's volume changes by the uplifted volume plus the supply taken in, less the
/// sediment that left at the outlet (none of the supply is still pending), to 1e-9 relative.
#[test]
fn the_books_close_while_lakes_fill_and_follow() {
    let mut w = incising_strip();
    let start = layer(&w, "height");
    let procs = processes(&incising_recipe());
    let lifted: f64 = (1..CELLS).map(|i| strip_uplift(i) * DT * CELL_AREA).sum();
    let (mut uplifted, mut supplied, mut out) = (0.0, 0.0, 0.0);
    for step in 0..STEPS {
        if step < 10 {
            let supply = (0..CELLS)
                .map(|i| if i >= 10 { 1.0 } else { 0.0 })
                .collect();
            w.insert("slope_supply", row(supply)).unwrap();
        }
        let report = evolve(&mut w, &procs, 1, DT, 1).unwrap();
        out += value(&report, "sediment_out_m3");
        supplied += value(&report, "supply_m3");
        uplifted += lifted;
    }
    assert!(
        (supplied - 10.0 * 6.0 * CELL_AREA).abs() <= 1e-9 * supplied,
        "{supplied}"
    );
    let pending: f64 = layer(&w, "slope_supply").iter().sum();
    assert_eq!(pending.to_bits(), 0.0_f64.to_bits());
    let change = volume(&layer(&w, "height")) - volume(&start);
    let want = uplifted + supplied - out;
    assert!(
        (change - want).abs() <= 1e-9 * uplifted,
        "change {change}, uplifted {uplifted}, supplied {supplied}, out {out}"
    );
}

/// Spec §5.9 and §8 at the iteration limit: a step that the limit stops returns its last plain
/// sweep, and the final routing of the step's sediment makes the depressions consistent with
/// it. On [`lake_strip`] with 4 iterations the lake holds exactly what the final routing brings
/// it, the slope's erosion (`20 + (60 − h3)`), nothing passes the sill, the volume is kept, and
/// the slope still ends above the lake. The limit's warning comes first; the books' correction
/// of the lake is a second warning, folded into the first.
#[test]
fn a_capped_step_returns_a_consistent_state() {
    let mut w = lake_strip();
    let start = layer(&w, "height");
    let report = lake_step(&mut w, "max_iterations = 4");
    let [Finding { message, .. }] = report.findings.as_slice() else {
        panic!("the limit stops the step: {:?}", report.findings);
    };
    assert!(message.contains("after 4 iterations"), "{message}");
    assert!(message.contains("1 more warning"), "{message}");
    let h = layer(&w, "height");
    let lake = 20.0 + (60.0 - h[3]);
    assert!((h[2] - lake).abs() < 1e-9, "lake at {} vs {lake}", h[2]);
    assert!(h[3] > h[2], "the slope at {} ends above the lake", h[3]);
    let out = value(&report, "sediment_out_m3");
    assert!(out.abs() < 1e-6, "{out} m³ passed the sill");
    let change = volume(&h) - volume(&start);
    assert!(
        (change + out).abs() <= 1e-9 * (start[3] - h[3]) * CELL_AREA,
        "change {change}, out {out}"
    );
}

/// A lake that is not full sets the base level of the slope that feeds it, and the slope's
/// erosion sets the lake's level: a loop whose gain is about the slope's drainage area over the
/// lake's (here 7). On a synthetic strip with a sill at 50 m (K = 0), a pit at −300 m and a
/// slope of seven cells rising from 60 m to 120 m, K = 1e-3, one step of 1e5 years converges
/// within the iteration limit, the lake holds exactly what the slope lost, nothing passes the
/// sill, and the slope ends above the lake. Holding the lake through the iterations left the
/// slope 330 m below the filled lake.
#[test]
fn a_lake_that_sets_its_slopes_base_level_converges() {
    let height: Vec<f64> = [0.0, 50.0, -300.0]
        .into_iter()
        .chain((0..7).map(|k| 60.0 + 10.0 * to_f64(k)))
        .collect();
    let erodibility = (0..height.len())
        .map(|i| if i == 1 { 0.0 } else { 1e-3 })
        .collect();
    let mut w = strip(height.clone(), erodibility);
    let report = lake_step(&mut w, "");
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    let h = layer(&w, "height");
    let eroded: f64 = (3..h.len()).map(|i| height[i] - h[i]).sum();
    let filled = h[2] + 300.0;
    assert!(
        (filled - eroded).abs() <= 1e-9 * eroded,
        "filled {filled}, eroded {eroded}"
    );
    assert!(filled < 350.0, "the lake is not full: {}", h[2]);
    assert!(h[3] > h[2], "the slope at {} ends above the lake", h[3]);
    let out = value(&report, "sediment_out_m3");
    assert!(out.abs() <= 1e-9 * eroded * CELL_AREA, "{out} m³ left");
}

/// Spec ruling 29: G = 0 neither deposits nor fills, so the depressions change nothing. One
/// G = 0 step on [`lake_strip`] with its routed depression gives the same bits as the same step
/// with no depressions.
#[test]
fn g_zero_ignores_depressions_bit_for_bit() {
    let routing = "[[process]]\nid = \"route_steepest\"\n[[process]]\nid = \"resolve_sinks\"\n\
                   [[process]]\nid = \"partition_flow\"\nexponent = \"steepest\"\n";
    let step = |drop: bool| {
        let mut w = lake_strip();
        evolve(&mut w, &processes(routing), 1, 1e5, 1).unwrap();
        let label = &w.value::<Depressions>("depressions").unwrap().label;
        assert_eq!(label.as_slice(), [0, 0, 1, 0], "the pit is routed");
        if drop {
            let none = Depressions {
                label: Grid::filled(1, 4, 0),
                spill: Vec::new(),
            };
            w.insert("depressions", Layer::Value(Box::new(none)))
                .unwrap();
        }
        let fluvial = "[[process]]\nid = \"fluvial\"\nm = 0.5\ng = 0.0\n";
        let report = evolve(&mut w, &processes(fluvial), 1, 1e5, 1).unwrap();
        let bits =
            |name: &str| -> Vec<u64> { layer(&w, name).iter().map(|x| x.to_bits()).collect() };
        (
            bits("height"),
            bits("erosion"),
            bits("deposition"),
            value(&report, "sediment_out_m3").to_bits(),
        )
    };
    let (with, without) = (step(false), step(true));
    assert_eq!(with, without);
    assert!(with.1.iter().any(|&e| e != 0), "the slope erodes");
}
