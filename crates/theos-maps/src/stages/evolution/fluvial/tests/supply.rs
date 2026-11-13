//! `fluvial` with the slope supply (spec §5.9 "Slope supply", ruling 51): what `slope_limit`
//! removed enters the sediment flux at its cell like eroded sediment, so G decides how much the
//! rivers lay down and how much they carry on; `fluvial` empties the layer once it took it in.

use super::deposition::{bowl, bowl_cells, pass_strip, routed, value, volume};
use super::{ALL_EDGES, BOTH, CELL_AREA, N, UPLIFT, fluvial_only, landscape, layer};
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
