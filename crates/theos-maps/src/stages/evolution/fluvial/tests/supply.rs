//! `fluvial` with the slope supply (spec §5.9 "Slope supply", ruling 51): what `slope_limit`
//! removed enters the sediment flux at its cell like eroded sediment, so G decides how much the
//! rivers lay down and how much they carry on; `fluvial` empties the layer once it took it in.

use super::deposition::{bowl, bowl_cells, pass_strip, routed, value, volume};
use super::lakes::{row, strip};
use super::{
    ALL_EDGES, BOTH, CELL_AREA, N, UPLIFT, fluvial_only, landscape, layer, recipe, with_runoff,
};
use crate::grid::Grid;
use crate::kernels::num::to_f64;
use crate::stages::evolution::PlannedProcess;
use crate::stages::evolution::fluvial::fill::EPSILON;
use crate::stages::evolution::test_support::{evolve, processes};
use crate::world::{Layer, World};

/// The default time step, years.
const DT: f64 = 2.5e4;

fn slope_limit() -> Vec<PlannedProcess> {
    processes("[[process]]\nid = \"slope_limit\"")
}

/// The slope supply of `w`, zeros when `w` holds none.
fn supply(w: &World) -> Vec<f64> {
    w.f64("slope_supply")
        .map_or_else(|| vec![0.0; N * N], |g| g.as_slice().to_vec())
}

/// `thickness` over the map as a volume, m³.
fn total(thickness: &[f64]) -> f64 {
    thickness.iter().map(|t| t * CELL_AREA).sum()
}

/// The bits of `v`.
fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Puts `metres` of slope supply on the cells where `on` holds and returns its volume, m³.
fn put_supply(w: &mut World, metres: f64, on: impl Fn(usize, usize) -> bool) -> f64 {
    let data: Vec<f64> = (0..N * N)
        .map(|i| if on(i / N, i % N) { metres } else { 0.0 })
        .collect();
    let held = total(&data);
    w.insert(
        "slope_supply",
        Layer::F64(Grid::from_vec(N, N, data).unwrap()),
    )
    .unwrap();
    held
}

/// A patch of 3 m of supply on rows and columns 20 to 29 of the landscape; its volume, m³.
fn patch(w: &mut World) -> f64 {
    let inside = |x: usize| (20..30).contains(&x);
    put_supply(w, 3.0, |r, c| inside(r) && inside(c))
}

#[test]
fn slope_supply_accumulates_until_fluvial_takes_it() {
    let mut w = landscape(ALL_EDGES, 8);
    // A gentle critical slope: the landscape's 40 m of noise over 100 m cells is steeper.
    w.insert("critical_slope", Layer::F64(Grid::filled(N, N, 0.05)))
        .unwrap();
    let first = evolve(&mut w, &slope_limit(), 1, DT, 1).unwrap();
    let once = supply(&w);
    assert!(
        once.iter().any(|&s| s > 0.1),
        "the first run removes material"
    );
    // Towers raised between the runs fail again; the second run adds what it removes.
    let mut height = w.f64("height").unwrap().clone();
    for (r, c) in [(20, 20), (40, 33)] {
        height.set(r, c, height.get(r, c) + 300.0);
    }
    w.insert("height", Layer::F64(height)).unwrap();
    let eroded = layer(&w, "erosion");
    let second = evolve(&mut w, &slope_limit(), 1, DT, 1).unwrap();
    let (twice, now_eroded) = (supply(&w), layer(&w, "erosion"));
    for i in 0..N * N {
        let added = now_eroded[i] - eroded[i];
        assert!(
            (twice[i] - (once[i] + added)).abs() <= 1e-9,
            "cell {i}: {} after {} and {added}",
            twice[i],
            once[i]
        );
    }
    assert!(twice[20 * N + 20] > once[20 * N + 20] + 100.0, "the tower");
    let held = total(&twice);
    let removed = first.values["slope_limit_removed_m3"] + second.values["slope_limit_removed_m3"];
    assert!(
        (held - removed).abs() <= 1e-9 * removed,
        "{held} vs {removed}"
    );
    // One fluvial step takes all of it in and empties the layer; the next takes nothing.
    let procs = processes(&routed("slope_adaptive", "g = 0.0"));
    let report = evolve(&mut w, &procs, 1, DT, 1).unwrap();
    let taken = value(&report, "supply_m3");
    assert!((taken - held).abs() <= 1e-12 * held, "{taken} vs {held}");
    assert!(supply(&w).iter().all(|s| s.to_bits() == 0), "emptied");
    let report = evolve(&mut w, &procs, 1, DT, 1).unwrap();
    assert_eq!(value(&report, "supply_m3").to_bits(), 0.0_f64.to_bits());
}

/// With K = 0 the supply is the only sediment. Without deposition (G = 0) all of it leaves at
/// the map's outlets and nothing changes height; with G = 1 part of it is laid down, the rest
/// leaves, and the two add up to the supply. (With steepest routing the patch drains into a
/// routed depression that keeps all of it, so nothing need leave.)
#[test]
fn slope_supply_is_laid_down_or_passes_on() {
    for exponent in BOTH {
        for g in [0.0, 1.0] {
            let at = format!("{exponent}, G = {g}");
            let mut w = landscape(ALL_EDGES, 9);
            w.insert("erodibility", Layer::F64(Grid::filled(N, N, 0.0)))
                .unwrap();
            let held = patch(&mut w);
            let start = layer(&w, "height");
            let procs = processes(&routed(exponent, &format!("g = {g:?}")));
            let report = evolve(&mut w, &procs, 1, DT, 1).unwrap();
            assert!(report.findings.is_empty(), "{at}: {:?}", report.findings);
            let (taken, out) = (
                value(&report, "supply_m3"),
                value(&report, "sediment_out_m3"),
            );
            assert!((taken - held).abs() <= 1e-12 * held, "{at}: took {taken}");
            assert!(supply(&w).iter().all(|s| s.to_bits() == 0), "{at}");
            let h = layer(&w, "height");
            let laid = total(&layer(&w, "deposition")) - total(&layer(&w, "erosion"));
            let raised = volume(&h) - volume(&start);
            assert!(
                (raised - laid).abs() <= 1e-9 * held,
                "{at}: raised {raised}, booked {laid}"
            );
            assert!(
                (laid + out - held).abs() <= 1e-9 * held,
                "{at}: laid down {laid}, out {out}, supply {held}"
            );
            if g > 0.0 {
                assert!(laid > 0.01 * held, "{at}: laid down {laid}");
                assert!(out >= -1e-9 * held, "{at}: passed on {out}");
            } else {
                assert!(
                    layer(&w, "deposition").iter().all(|d| d.to_bits() == 0),
                    "{at}"
                );
                assert_eq!(bits(&h), bits(&start), "{at}");
            }
        }
    }
}

/// The step's sediment after the iterations fills the routed depressions (spec §5.9), the supply
/// included: 2 m of it on the bowl's northern slope, above its depression, with K = 0. The bowl
/// holds far more, so nothing leaves the map, and its depression's cells rise.
#[test]
fn slope_supply_fills_a_routed_depression() {
    let cells = bowl_cells();
    for exponent in BOTH {
        let mut w = bowl();
        w.insert("erodibility", Layer::F64(Grid::filled(N, N, 0.0)))
            .unwrap();
        let held = put_supply(&mut w, 2.0, |r, c| {
            (14..=24).contains(&r) && (14..=50).contains(&c)
        });
        let start = layer(&w, "height");
        let report = evolve(&mut w, &processes(&routed(exponent, "g = 1.0")), 1, DT, 1).unwrap();
        let out = value(&report, "sediment_out_m3");
        assert!(out.abs() < 1e-3, "{exponent}: {out} m³ left the map");
        let h = layer(&w, "height");
        let raised = volume(&h) - volume(&start);
        assert!(
            (raised - held).abs() <= 1e-9 * held,
            "{exponent}: raised {raised} of {held}"
        );
        let filled = cells.iter().filter(|&&c| h[c] > start[c] + 1e-3).count();
        assert!(filled > 0, "{exponent}: the depression took none of it");
    }
}

/// A routed depression keeps the supply on its own cells like the sediment that reaches it, up
/// to its capacity, also where they drain to a spill cell outside it (the pass's higher cell).
/// On the strip of `pass_strip` with K = 0, the lake (cell 2, at 45 m, spilling at 50 m from
/// cell 1) holds 5 m of one cell: 2 m of supply on it stay and raise it, 8 m fill it to its place
/// one ε above the spill level and pass the rest on from the spill cell.
#[test]
fn slope_supply_on_a_depressions_cells_fills_it_up_to_its_capacity() {
    for metres in [2.0, 8.0] {
        let mut w = pass_strip();
        let strip = |v: Vec<f64>| Layer::F64(Grid::from_vec(1, 4, v).unwrap());
        w.insert("erodibility", strip(vec![0.0; 4])).unwrap();
        w.insert("slope_supply", strip(vec![0.0, 0.0, metres, 0.0]))
            .unwrap();
        let start = layer(&w, "height");
        let report = evolve(&mut w, &processes(&fluvial_only("g = 1.0")), 1, 1e5, 1).unwrap();
        assert!(
            report.findings.is_empty(),
            "{metres} m: {:?}",
            report.findings
        );
        let held = metres * CELL_AREA;
        let out = value(&report, "sediment_out_m3");
        let h = layer(&w, "height");
        let raised = volume(&h) - volume(&start);
        assert!(
            (raised + out - held).abs() <= 1e-9 * held,
            "{metres} m: raised {raised}, out {out}"
        );
        let full = 50.0 + EPSILON;
        if metres < 5.0 {
            assert!(
                (h[2] - (45.0 + metres)).abs() < 1e-9,
                "{metres} m: lake at {}",
                h[2]
            );
            assert!(out.abs() < 1e-6 * held, "{metres} m: {out} m³ left");
        } else {
            assert!((h[2] - full).abs() < 1e-9, "{metres} m: lake at {}", h[2]);
            assert!(
                out > 0.0 && h[1] > start[1],
                "{metres} m: the excess passes on"
            );
        }
    }
}

/// Plan ruling L27: `slope_limit` every step, `fluvial` every second. Nothing is counted twice or
/// lost: the evolution's summed values show the supply still pending at the end as
/// `slope_limit_removed_m3 − fluvial_supply_m3`, and the run's volume changes by the uplift less
/// the sediment that left and the supply still pending.
#[test]
fn slope_limit_and_fluvial_at_different_every_lose_nothing() {
    const STEPS: u32 = 8;
    let mut w = landscape(r#"["south"]"#, 7);
    w.insert("critical_slope", Layer::F64(Grid::filled(N, N, 0.05)))
        .unwrap();
    let fixed = w.mask("fixed").unwrap().as_slice().to_vec();
    let start = layer(&w, "height");
    let recipe = "[[process]]\nid = \"uplift\"\n[[process]]\nid = \"route_steepest\"\n\
                  [[process]]\nid = \"resolve_sinks\"\n\
                  [[process]]\nid = \"partition_flow\"\nexponent = \"slope_adaptive\"\n\
                  [[process]]\nid = \"fluvial\"\ng = 1.0\nevery = 2\n\
                  [[process]]\nid = \"slope_limit\"\n";
    let report = evolve(&mut w, &processes(recipe), STEPS, DT, 1).unwrap();
    let removed = report.values["slope_limit_removed_m3"];
    let taken = value(&report, "supply_m3");
    let pending = total(&supply(&w));
    assert!(
        taken > 0.0 && pending > 0.0,
        "taken {taken}, pending {pending}"
    );
    assert!(
        (removed - (taken + pending)).abs() <= 1e-9 * removed,
        "removed {removed}, taken {taken}, pending {pending}"
    );
    let lifted = fixed.iter().filter(|&&f| !f).count();
    let uplifted = to_f64(lifted) * (0..STEPS).fold(0.0, |sum, _| sum + UPLIFT * DT) * CELL_AREA;
    let out = value(&report, "sediment_out_m3");
    let change = volume(&layer(&w, "height")) - volume(&start);
    assert!(
        (change - (uplifted - out - pending)).abs() <= 1e-9 * uplifted,
        "change {change}, uplifted {uplifted}, out {out}, pending {pending}"
    );
}

/// The cells of [`dry_strip`].
const DRY_CELLS: usize = 32;

/// The wet cells of [`dry_strip`], at its outlet.
const WET: usize = 4;

/// The slope supply on each dry cell of [`dry_strip`], metres.
const DRY_SUPPLY: f64 = 500.0;

/// The runoff weight `p̂` of [`dry_strip`]'s dry cells: their runoff over the strip's mean.
fn dry_weight() -> f64 {
    1e-3 / mean_runoff()
}

/// The runoff weight `p̂` of [`dry_strip`]'s wet cells.
fn wet_weight() -> f64 {
    1.0 / mean_runoff()
}

/// The mean runoff of [`dry_strip`]: 1 on the wet cells, 1e-3 on the dry ones.
fn mean_runoff() -> f64 {
    (to_f64(WET) + to_f64(DRY_CELLS - WET) * 1e-3) / to_f64(DRY_CELLS)
}

/// A synthetic 1 x 32 strip of 100 m cells rising 5 m per cell from its outlet (cell 0, fixed),
/// with the erodibility `k`, in a rain shadow: the four cells at the outlet are wet, the others
/// get a thousandth of their runoff. There `p̂` is 0.008 and `G/p̂` about 126 at G = 1, as on the
/// dry patches of the 384² budget recipe in its dry climate periods (Task 27g: 100 to 200), and
/// each of them holds [`DRY_SUPPLY`] of slope supply (27g: up to 1.7 km).
fn dry_strip(k: f64) -> World {
    let height = (0..DRY_CELLS).map(|i| 5.0 * to_f64(i)).collect();
    let mut w = strip(height, vec![k; DRY_CELLS]);
    let dry = |i: usize| i >= WET;
    let runoff = (0..DRY_CELLS)
        .map(|i| if dry(i) { 1e-3 } else { 1.0 })
        .collect();
    let runoff = Grid::from_vec(1, DRY_CELLS, runoff).unwrap();
    w.insert("climate", Layer::Value(Box::new(with_runoff(runoff))))
        .unwrap();
    let supply = (0..DRY_CELLS)
        .map(|i| if dry(i) { DRY_SUPPLY } else { 0.0 })
        .collect();
    w.insert("slope_supply", row(supply)).unwrap();
    w
}

/// Task 27g's cause of the iteration caps at 384²: hundreds of metres of slope supply on dry
/// cells, where the rivers lay almost all of it down (`c/(1 + c)` with `c = G·area/Q` about 126).
/// Iterations that start from `h*` route all of it undeposited down the dry slopes in their first
/// sweep, and the transient that follows dies out too slowly for the iteration limit. On
/// [`dry_strip`] with K = 1e-5 and uplift they stopped at the limit with a residual of 1.08 m
/// against a tolerance of 0.025 m. Started from the supply laid down along its way, they
/// converge in 16 iterations.
#[test]
fn dry_slopes_with_much_supply_converge_within_the_limit() {
    let mut w = dry_strip(1e-5);
    w.insert("uplift", row(vec![UPLIFT; DRY_CELLS])).unwrap();
    w.insert("sag_rate", row(vec![0.0; DRY_CELLS])).unwrap();
    let report = evolve(&mut w, &processes(&recipe("steepest", "g = 1.0")), 1, DT, 1).unwrap();
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    let iterations = value(&report, "iterations");
    assert!(iterations <= 25.0, "{iterations} iterations");
    assert!(layer(&w, "height").iter().all(|h| h.is_finite()));
}

/// With K = 0 nothing incises, and outside the depressions the step's solution is the supply
/// laid down as it passes: each cell keeps `c/(1 + c)` of what passes through it, its own supply
/// included, and passes the rest on. The iterations start from exactly that, so on
/// [`dry_strip`] without uplift (tolerance 1e-6 m) the first one finds the solution.
#[test]
fn with_no_incision_the_first_iterate_is_the_solution() {
    let mut w = dry_strip(0.0);
    let start = layer(&w, "height");
    let held = total(&supply(&w));
    let report = evolve(&mut w, &processes(&routed("steepest", "g = 1.0")), 1, DT, 1).unwrap();
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    assert_eq!(value(&report, "iterations").to_bits(), 1.0_f64.to_bits());
    let h = layer(&w, "height");
    assert!(h.iter().zip(&start).all(|(a, b)| a >= b), "nothing incises");
    // The top cell drains no other cell, so it keeps c/(1 + c) of its own supply, c = G/p̂.
    let c = 1.0 / dry_weight();
    let top = DRY_CELLS - 1;
    let want = start[top] + DRY_SUPPLY * c / (1.0 + c);
    assert!((h[top] - want).abs() < 1e-9, "{} vs {want}", h[top]);
    let (raised, out) = (
        volume(&h) - volume(&start),
        value(&report, "sediment_out_m3"),
    );
    assert!(
        (raised + out - held).abs() <= 1e-9 * held,
        "raised {raised}, out {out}, supply {held}"
    );
}

/// The recipe switch `supply_passes_dry_cells` (Task 27h): the slope supply passes the cells
/// whose catchment is dry, `Q < G·A` (its mean runoff weight `Q/A` below G, where Yuan et al.'s
/// condition `G/p̂ ≤ 1` fails), without being laid down, and joins the sediment that the law lays
/// down at the first cell whose catchment is not. On [`dry_strip`] with K = 0, no uplift and
/// G = 0.5, cells 3 to 31 are dry (cell 3's catchment has `Q/A` = 0.28) and cells 1 and 2 are not
/// (0.78 and 0.54). The dry cells keep their height; cell 2 keeps `c/(1 + c)` of all of the
/// supply and cell 1 of what cell 2 passes on (`c = G·area/Q`); the rest leaves. The first
/// iterate lays the supply down the same way, so one iteration finds the solution.
#[test]
fn with_the_switch_dry_cells_pass_the_supply_on_to_wetter_ones() {
    let mut w = dry_strip(0.0);
    let start = layer(&w, "height");
    let held = total(&supply(&w));
    let fluvial = "g = 0.5\nsupply_passes_dry_cells = true";
    let report = evolve(&mut w, &processes(&routed("steepest", fluvial)), 1, DT, 1).unwrap();
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    assert_eq!(value(&report, "iterations").to_bits(), 1.0_f64.to_bits());
    let h = layer(&w, "height");
    for cell in 3..DRY_CELLS {
        assert!(
            (h[cell] - start[cell]).abs() < 1e-9,
            "cell {cell}: {} from {}",
            h[cell],
            start[cell]
        );
    }
    // Q of cells 2 and 1 in cells: the 28 dry cells and the wet cells from 3 down to them.
    let reaching = to_f64(DRY_CELLS - WET) * DRY_SUPPLY;
    let c2 = 0.5 / (to_f64(DRY_CELLS - WET) * dry_weight() + 2.0 * wet_weight());
    let kept2 = reaching * c2 / (1.0 + c2);
    let c1 = 0.5 / (to_f64(DRY_CELLS - WET) * dry_weight() + 3.0 * wet_weight());
    let kept1 = (reaching - kept2) * c1 / (1.0 + c1);
    assert!(
        (h[2] - (start[2] + kept2)).abs() < 1e-6,
        "{} vs {kept2}",
        h[2]
    );
    assert!(
        (h[1] - (start[1] + kept1)).abs() < 1e-6,
        "{} vs {kept1}",
        h[1]
    );
    let (raised, out) = (
        volume(&h) - volume(&start),
        value(&report, "sediment_out_m3"),
    );
    assert!(
        (raised - (kept1 + kept2) * CELL_AREA).abs() <= 1e-9 * held,
        "raised {raised}"
    );
    assert!(
        (raised + out - held).abs() <= 1e-9 * held,
        "raised {raised}, out {out}, supply {held}"
    );
}

/// The supply loop on dry cells that the switch stops (Task 27g): without it, `fluvial` lays
/// almost all of a dry cell's own supply back on it (`c/(1 + c)`, `c` about 126 on
/// [`dry_strip`]), `slope_limit` cuts the deposit straight back into the supply, and the supply
/// pending on the dry cells grows from step to step by about the uplift it adds. With the switch
/// the supply passes on, and what is pending on the dry cells after each step is only that step's
/// cut: at most the uplift of 25 m on each of the 28 dry cells (less what the rivers cut first),
/// which the critical slope of 0.05, the strip's own slope, takes off again. It does not grow.
#[test]
fn with_the_switch_the_supply_loop_on_dry_cells_stops() {
    let pending = |switch: &str| -> Vec<f64> {
        let mut w = dry_strip(1e-5);
        w.insert("slope_supply", row(vec![0.0; DRY_CELLS])).unwrap();
        w.insert("uplift", row(vec![UPLIFT; DRY_CELLS])).unwrap();
        w.insert("sag_rate", row(vec![0.0; DRY_CELLS])).unwrap();
        w.insert("critical_slope", row(vec![0.05; DRY_CELLS]))
            .unwrap();
        let fluvial = format!("g = 1.0\n{switch}");
        let procs = processes(&format!(
            "{}\n[[process]]\nid = \"slope_limit\"",
            recipe("steepest", &fluvial)
        ));
        (0..10)
            .map(|step| {
                let report = evolve(&mut w, &procs, 1, DT, 1).unwrap();
                assert!(report.findings.is_empty(), "step {step}: {report:?}");
                supply(&w)[WET..].iter().sum()
            })
            .collect()
    };
    let cut = to_f64(DRY_CELLS - WET) * UPLIFT * DT;
    let on = pending("supply_passes_dry_cells = true");
    for (step, p) in on.iter().enumerate() {
        assert!(
            *p <= cut && (p - on[0]).abs() <= 1e-3 * cut,
            "step {step}: {p} m pending: {on:?}"
        );
    }
    let off = pending("");
    assert_eq!(
        off[0].to_bits(),
        on[0].to_bits(),
        "no supply before the first cut"
    );
    for pair in off.windows(2) {
        assert!(pair[1] > pair[0] + 0.5 * cut, "{off:?}");
    }
}

/// With the switch off, as by default, the step is the one without it, bit for bit.
#[test]
fn the_switch_off_leaves_the_step_as_it_is() {
    let run = |extra: &str| {
        let mut w = dry_strip(1e-5);
        w.insert("uplift", row(vec![UPLIFT; DRY_CELLS])).unwrap();
        w.insert("sag_rate", row(vec![0.0; DRY_CELLS])).unwrap();
        let fluvial = format!("g = 1.0\n{extra}");
        let report = evolve(&mut w, &processes(&recipe("steepest", &fluvial)), 1, DT, 1).unwrap();
        let layers: Vec<Vec<u64>> = ["height", "erosion", "deposition"]
            .iter()
            .map(|name| bits(&layer(&w, name)))
            .collect();
        (layers, report)
    };
    assert_eq!(run("supply_passes_dry_cells = false"), run(""));
}

#[test]
fn the_same_supply_gives_the_same_layers() {
    let run = || {
        let mut w = landscape(ALL_EDGES, 9);
        patch(&mut w);
        let procs = processes(&routed("slope_adaptive", "g = 1.0"));
        let report = evolve(&mut w, &procs, 2, DT, 1).unwrap();
        let layers: Vec<Vec<u64>> = ["height", "erosion", "deposition", "slope_supply"]
            .iter()
            .map(|name| bits(&layer(&w, name)))
            .collect();
        (layers, report)
    };
    assert_eq!(run(), run());
}
