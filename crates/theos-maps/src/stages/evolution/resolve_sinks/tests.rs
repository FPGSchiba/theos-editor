//! `route_steepest` then `resolve_sinks` on 64 x 64 worlds of 100 m cells.

use super::depressions::SpillPoint;
use super::{Depressions, ResolveSinks};
use crate::grid::Grid;
use crate::kernels::noise::spectral_noise;
use crate::kernels::num::to_f64;
use crate::rng::StageRng;
use crate::stage::{LayerSpec, Stage, StageError};
use crate::stages::evolution::test_support::{evolve, processes};
use crate::stages::terrain::grid::GridStage;
use crate::stages::terrain::support::{run_stage, world};
use crate::world::{Layer, ValueType, World, WorldValue};

/// The two routing processes in the recipe's order.
const ROUTING: &str = "[[process]]\nid = \"route_steepest\"\n[[process]]\nid = \"resolve_sinks\"";

/// Every edge drained.
const ALL_EDGES: &str = r#"["north", "east", "south", "west"]"#;

/// The side of the test grids.
const N: usize = 64;

/// The lift of one flood step.
const EPS: f64 = 1e-6;

fn idx(r: usize, c: usize) -> usize {
    r * N + c
}

/// The Chebyshev distance from `(r, c)` to `at`.
fn cheb(r: usize, c: usize, at: (usize, usize)) -> usize {
    r.abs_diff(at.0).max(c.abs_diff(at.1))
}

/// `level` raised by `k` flood steps, added one at a time as the flood does.
fn lifted(level: f64, k: usize) -> f64 {
    (0..k).fold(level, |v, _| v + EPS)
}

/// A 64 x 64 world whose drained `edges` (a TOML list) are fixed, holding `height`.
fn world_with(edges: &str, height: Grid<f64>) -> World {
    let mut w = world(N, N);
    run_stage(&mut w, &GridStage, &format!("edges = {edges}")).unwrap();
    w.insert("height", Layer::F64(height)).unwrap();
    w
}

/// A 64 x 64 grid of `f(row, col)`.
fn surface(mut f: impl FnMut(usize, usize) -> f64) -> Grid<f64> {
    Grid::from_vec(N, N, (0..N * N).map(|i| f(i / N, i % N)).collect()).unwrap()
}

/// Smooth noise of 40 m relief plus up to 1 m of jitter, like the evolution's initial surface.
fn noise(seed: u64) -> Grid<f64> {
    let mut rng = StageRng::for_key(seed, "smooth");
    let smooth = spectral_noise(N, N, &mut rng, 3.0, 0.0, None);
    surface(|r, c| 40.0 * smooth.get(r, c) + rng.range(0.0, 1.0))
}

/// Independent uniform heights: a pit at about every ninth cell, nested every way.
fn white(seed: u64) -> Grid<f64> {
    let mut rng = StageRng::for_key(seed, "white");
    surface(|_, _| rng.range(0.0, 100.0))
}

/// Falls 2 m per row to the south edge and 1 m per column away from column 32: no pits.
fn background(r: usize, c: usize) -> f64 {
    2.0 * to_f64(63 - r) - to_f64(c.abs_diff(32))
}

/// Runs `route_steepest` then `resolve_sinks` once over `w`.
fn route(w: &mut World) {
    evolve(w, &processes(ROUTING), 1, 2.5e4, 1).unwrap();
}

fn receivers(w: &World) -> Vec<usize> {
    w.u32("receivers")
        .unwrap()
        .as_slice()
        .iter()
        .map(|&r| r as usize)
        .collect()
}

fn routing(w: &World) -> &Grid<f64> {
    w.f64("routing_surface").unwrap()
}

fn depressions(w: &World) -> &Depressions {
    w.value::<Depressions>("depressions").unwrap()
}

/// The receiver chain from `i` to the cell that receives itself, or `None` when it cycles.
fn chain(rec: &[usize], i: usize) -> Option<Vec<usize>> {
    let mut cells = vec![i];
    let mut at = i;
    while rec[at] != at {
        at = rec[at];
        cells.push(at);
        if cells.len() > rec.len() {
            return None;
        }
    }
    Some(cells)
}

/// Whether `j` is one of the eight neighbours of `i`.
fn is_neighbour(i: usize, j: usize) -> bool {
    i != j && (i / N).abs_diff(j / N) <= 1 && (i % N).abs_diff(j % N) <= 1
}

/// The lowest cell, ties by index.
fn lowest(height: &Grid<f64>) -> usize {
    (0..height.len())
        .min_by(|&a, &b| height.as_slice()[a].total_cmp(&height.as_slice()[b]))
        .unwrap()
}

fn same_bits(a: &Grid<f64>, b: &Grid<f64>) -> bool {
    a.as_slice()
        .iter()
        .zip(b.as_slice())
        .all(|(x, y)| x.to_bits() == y.to_bits())
}

// Review Focus 1.
#[test]
fn closed_and_fully_open_edges_route_every_cell() {
    for seed in 1..=3 {
        for height in [noise(seed), white(seed)] {
            let sink = lowest(&height);
            for edges in ["[]", ALL_EDGES] {
                let mut w = world_with(edges, height.clone());
                route(&mut w);
                let rec = receivers(&w);
                let fixed = w.mask("fixed").unwrap().as_slice();
                let at = format!("seed {seed}, edges {edges}");
                assert!(routing(&w).as_slice().iter().all(|v| v.is_finite()), "{at}");
                for i in 0..N * N {
                    assert!(rec[i] == i || is_neighbour(i, rec[i]), "{at}: {i}");
                    let Some(cells) = chain(&rec, i) else {
                        panic!("{at}: the chain from cell {i} cycles");
                    };
                    let end = *cells.last().unwrap();
                    if edges == "[]" {
                        assert_eq!(end, sink, "{at}: the chain from {i} misses the sink");
                    } else {
                        assert!(fixed[end], "{at}: the chain from {i} ends at {end}");
                    }
                }
            }
        }
    }
}

/// A bowl centred at (20, 32): floor 100 m + 5 m per ring, a rim at 200 m on ring 8 with a notch
/// at 150 m at (28, 32), in the pit-free background.
fn notched_bowl(r: usize, c: usize) -> f64 {
    match cheb(r, c, (20, 32)) {
        d if d < 8 => 100.0 + 5.0 * to_f64(d),
        8 if (r, c) == (28, 32) => 150.0,
        8 => 200.0,
        _ => background(r, c),
    }
}

#[test]
fn a_bowl_drains_through_its_notch() {
    let mut w = world_with(r#"["south"]"#, surface(notched_bowl));
    route(&mut w);
    let (notch, spill) = (idx(28, 32), idx(27, 31));
    let dep = depressions(&w);
    assert_eq!(dep.spill.len(), 1, "{:?}", dep.spill);
    let SpillPoint { cell, to, level } = dep.spill[0];
    assert_eq!((cell as usize, to as usize), (spill, notch));
    assert_eq!(level.to_bits(), 150.0_f64.to_bits());
    let (rec, fixed) = (receivers(&w), w.mask("fixed").unwrap().as_slice());
    let (lift, height) = (routing(&w), w.f64("height").unwrap());
    for i in 0..N * N {
        let (r, c) = (i / N, i % N);
        let inside = cheb(r, c, (20, 32)) < 8;
        assert_eq!(dep.label.as_slice()[i] == 1, inside, "label of {i}");
        if inside {
            let want = lifted(150.0, cheb(r, c, (27, 31)));
            assert_eq!(lift.as_slice()[i].to_bits(), want.to_bits(), "lift of {i}");
            let cells = chain(&rec, i).unwrap();
            assert!(
                cells.contains(&notch),
                "the chain from {i} misses the notch"
            );
            assert!(fixed[*cells.last().unwrap()]);
        } else {
            assert_eq!(lift.as_slice()[i].to_bits(), height.as_slice()[i].to_bits());
        }
    }
    assert_eq!(rec[spill], notch, "the spill cell drains over the pass");
}

/// The bowl of [`notched_bowl`] without a notch, its rim two rings thick (8 and 9) and flat at
/// 150 m: the inner ring drains into the bowl, the outer one out of it.
fn flat_rimmed_bowl(r: usize, c: usize) -> f64 {
    match cheb(r, c, (20, 32)) {
        d if d < 8 => 100.0 + 5.0 * to_f64(d),
        8 | 9 => 150.0,
        _ => background(r, c),
    }
}

#[test]
fn a_flat_rim_at_the_spill_level_keeps_its_way_down() {
    let mut w = world_with(r#"["south"]"#, surface(flat_rimmed_bowl));
    route(&mut w);
    let dep = depressions(&w);
    let (spill, out) = (idx(12, 24), idx(11, 23));
    let first = dep.spill.first().map(|s| (s.cell as usize, s.to as usize));
    assert_eq!(first, Some((spill, out)), "the first pass of the bowl");
    let (rec, fixed, lift) = (receivers(&w), w.mask("fixed").unwrap(), routing(&w));
    for i in 0..N * N {
        let (r, c) = (i / N, i % N);
        if cheb(r, c, (20, 32)) == 8 && i != spill {
            // Lifted above the bowl next to it, the inner rim would be left without a lower
            // neighbour unless the flood lifts it too.
            assert_eq!(
                dep.label.as_slice()[i],
                1,
                "inner rim cell {i} is not in the lake"
            );
            assert!(lift.as_slice()[i] > 150.0);
        }
        let end = *chain(&rec, i).unwrap().last().unwrap();
        assert!(fixed.as_slice()[end], "the chain from {i} ends at {end}");
    }
}

/// A bowl centred at (25, 32) with a rim at 400 m on ring 12 and a notch at 300 m at (37, 38).
/// A ridge at 250 m on column 32 with a saddle at 200 m at (25, 32) splits it into a shallow
/// pit at (25, 26) (150 m + 4 m per ring) and a deep one at (25, 38) (100 m + 4 m per ring).
fn nested_bowl(r: usize, c: usize) -> f64 {
    match cheb(r, c, (25, 32)) {
        d if d > 12 => background(r, c),
        12 if (r, c) == (37, 38) => 300.0,
        12 => 400.0,
        _ if c < 32 => 150.0 + 4.0 * to_f64(cheb(r, c, (25, 26))),
        _ if c > 32 => 100.0 + 4.0 * to_f64(cheb(r, c, (25, 38))),
        _ if r == 25 => 200.0,
        _ => 250.0,
    }
}

/// The shallow pit of [`nested_bowl`] spills over the saddle at 200 m into the deep one, which
/// spills over the notch at 300 m: water stands over both pits in one lake at 300 m. The shallow
/// pit is part of that lake, not a depression of its own that ends at its saddle: one depression,
/// which spills over the notch at 300 m, holds both pits, and the shallow pit's cells stand at
/// the lake's level on the routing surface. Every chain from the shallow pit stays in the lake
/// until it leaves over the notch. (Counted as a depression of its own, the shallow pit took
/// sediment only up to its saddle, and the cells between the saddle and the lake's level were
/// never filled, so the lake passed sediment on while it still had room; Task 43e.)
#[test]
fn a_pit_nested_in_a_lake_is_part_of_the_lake() {
    let mut w = world_with(r#"["south"]"#, surface(nested_bowl));
    route(&mut w);
    let dep = depressions(&w);
    let (notch, saddle) = (idx(37, 38), idx(25, 32));
    let spill: Vec<(usize, usize, u64)> = dep
        .spill
        .iter()
        .map(|s| (s.cell as usize, s.to as usize, s.level.to_bits()))
        .collect();
    assert_eq!(
        spill,
        [(idx(36, 37), notch, 300.0_f64.to_bits())],
        "one lake, which spills over the notch"
    );
    let label = |r, c| dep.label.as_slice()[idx(r, c)];
    assert_eq!((label(25, 38), label(25, 26)), (1, 1));
    assert_eq!((label(24, 31), label(25, 32), label(37, 38)), (1, 1, 0));
    let (rec, fixed, lift) = (receivers(&w), w.mask("fixed").unwrap(), routing(&w));
    // The shallow pit stands at the level of the lake, not at its saddle.
    let lake = lifted(300.0, cheb(25, 32, (36, 37)));
    assert_eq!(lift.as_slice()[saddle].to_bits(), lake.to_bits());
    assert_eq!(lift.as_slice()[idx(24, 31)].to_bits(), lake.to_bits());
    let shallow = (0..N * N).filter(|&i| {
        let (r, c) = (i / N, i % N);
        dep.label.as_slice()[i] > 0 && c < 32 && cheb(r, c, (25, 32)) < 12
    });
    for i in shallow {
        let cells = chain(&rec, i).unwrap();
        let Some(leaves) = cells.iter().position(|&c| c == notch) else {
            panic!("the chain from {i} misses the notch: {cells:?}");
        };
        assert!(
            cells[..leaves]
                .iter()
                .all(|&c| dep.label.as_slice()[c] == 1),
            "the chain from {i} leaves the lake before the notch: {cells:?}"
        );
        assert!(fixed.as_slice()[*cells.last().unwrap()]);
    }
}

#[test]
fn routing_surface_lifts_only_depressions_and_height_is_unchanged() {
    for seed in 1..=3 {
        for height in [noise(seed), white(seed)] {
            let mut w = world_with(r#"["south"]"#, height.clone());
            route(&mut w);
            assert!(
                same_bits(w.f64("height").unwrap(), &height),
                "height changed"
            );
            let (dep, lift) = (depressions(&w), routing(&w));
            assert!(!dep.spill.is_empty(), "seed {seed}: no depression to test");
            let (h, r) = (height.as_slice(), lift.as_slice());
            for i in 0..N * N {
                match dep.label.as_slice()[i] as usize {
                    0 => assert_eq!(r[i].to_bits(), h[i].to_bits(), "cell {i} lifted"),
                    n => {
                        assert!(r[i] > h[i], "depression cell {i} not lifted");
                        assert!(r[i] >= dep.spill[n - 1].level, "cell {i} below its spill");
                    }
                }
            }
            for (k, s) in dep.spill.iter().enumerate() {
                let (cell, to) = (s.cell as usize, s.to as usize);
                assert!(
                    (dep.label.as_slice()[to] as usize) <= k,
                    "spills downstream"
                );
                assert!(r[to] <= r[cell], "the pass from {cell} climbs");
            }
        }
    }
}

/// Every receiver lies lower on the routing surface than its cell, except over a pass: a lake's
/// spill cell receives the cell across its pass, and a pit nested in a lake drains over its pass
/// at the lake's level, so its pass cell stands exactly at its receiver's level, both in the
/// lake. Steepest descent never picks a neighbour at the cell's own level, so no other cell
/// stands level with its receiver.
#[test]
fn receivers_descend_the_routing_surface_except_over_a_pass() {
    let (mut nested, mut lakes) = (0, 0);
    for seed in 1..=3 {
        for edges in ["[]", r#"["south"]"#] {
            let height = white(seed);
            let sink = lowest(&height);
            let mut w = world_with(edges, height);
            route(&mut w);
            let (rec, dep) = (receivers(&w), depressions(&w));
            let (r, fixed) = (routing(&w).as_slice(), w.mask("fixed").unwrap().as_slice());
            let label = dep.label.as_slice();
            lakes += dep.spill.len();
            for i in 0..N * N {
                if let Some(s) = dep.spill.iter().find(|s| s.cell as usize == i) {
                    assert_eq!(rec[i], s.to as usize, "spill cell {i}");
                } else if fixed[i] || (edges == "[]" && i == sink) {
                    assert_eq!(rec[i], i, "outlet {i}");
                } else if r[rec[i]].to_bits() == r[i].to_bits() {
                    assert!(
                        label[i] > 0 && label[rec[i]] == label[i],
                        "cell {i} is level with its receiver outside a lake"
                    );
                    nested += 1;
                } else {
                    assert!(r[rec[i]] < r[i], "cell {i} does not fall to its receiver");
                }
            }
        }
    }
    assert!(
        nested > 0 && lakes > 0,
        "no nested pit ({nested}) or lake ({lakes}) to test"
    );
}

#[test]
fn flat_terrain_routes_deterministically() {
    for edges in [r#"["south"]"#, "[]"] {
        let run = || {
            let mut w = world_with(edges, Grid::filled(N, N, 0.0));
            route(&mut w);
            w
        };
        let (a, b) = (run(), run());
        assert_eq!(receivers(&a), receivers(&b));
        assert!(same_bits(routing(&a), routing(&b)));
        assert_eq!(depressions(&a), depressions(&b));
        assert!(depressions(&a).spill.is_empty(), "a flat holds no water");
        let (rec, fixed) = (receivers(&a), a.mask("fixed").unwrap().as_slice());
        for i in 0..N * N {
            let end = *chain(&rec, i).unwrap().last().unwrap();
            assert!(
                fixed[end] || (edges == "[]" && end == 0),
                "{edges}: {i} ends at {end}"
            );
        }
    }
}

#[test]
fn receivers_that_are_not_lower_neighbours_fail_naming_the_cell() {
    // Cell 5 is row 1, column 2 of a 3 x 3 grid whose height is the cell index.
    for (bad, what) in [(99, "off the grid"), (8, "uphill"), (3, "not a neighbour")] {
        let mut w = world(3, 3);
        let height = Grid::from_vec(3, 3, (0..9).map(to_f64).collect()).unwrap();
        let mut rec: Vec<u32> = (0..9).collect();
        rec[5] = bad;
        w.insert("height", Layer::F64(height)).unwrap();
        w.insert("receivers", Layer::U32(Grid::from_vec(3, 3, rec).unwrap()))
            .unwrap();
        w.insert("fixed", Layer::Mask(Grid::filled(3, 3, false)))
            .unwrap();
        let err = run_stage(&mut w, &ResolveSinks, "").unwrap_err();
        assert!(
            matches!(&err, StageError::Failed { stage, message }
                if stage == "resolve_sinks" && message.contains("cell 5")),
            "{what}: {err:?}"
        );
    }
}

#[test]
fn resolve_sinks_declares_its_layers() {
    let stage: &dyn Stage = &ResolveSinks;
    let params = stage.params(&toml::Table::new()).unwrap();
    assert_eq!(
        stage.reads(params.as_ref()),
        [
            LayerSpec::f64("height"),
            LayerSpec::u32("receivers"),
            LayerSpec::mask("fixed"),
        ]
    );
    assert_eq!(
        stage.writes(params.as_ref()),
        [
            LayerSpec::u32("receivers"),
            LayerSpec::f64("routing_surface"),
            LayerSpec::value("depressions", Depressions::NAME),
        ]
    );
    assert!(stage.params(&"epsilon = 1.0".parse().unwrap()).is_err());
}

#[test]
fn depressions_have_a_canonical_json() {
    let mut dep = Depressions {
        label: Grid::from_vec(2, 2, vec![0, 1, 1, 0]).unwrap(),
        spill: vec![SpillPoint {
            cell: 1,
            to: 0,
            level: 2.5,
        }],
    };
    assert_eq!(dep.type_name(), "depressions");
    assert_eq!(
        dep.canonical_json(),
        serde_json::json!({
            "label": [0, 1, 1, 0],
            "spill": [{"cell": 1, "to": 0, "level": 2.5}],
        })
    );
    let any = dep.as_any_mut().downcast_mut::<Depressions>().unwrap();
    any.spill.clear();
    assert_eq!(dep.spill.len(), 0, "the downcast reached the value itself");
}
