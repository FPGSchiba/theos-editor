//! `hydrology` on 64 x 64 worlds of 100 m cells, with physics tests 7 and 8 (spec §5.14).

use super::lakes::Limits;
use super::{Hydrology, Water, WaterThresholds, water};
use crate::grid::{D8, Grid};
use crate::kernels::flood::priority_flood;
use crate::kernels::num::{OrdF64, key, max, min, to_f64};
use crate::recipe::{RecipeError, RecipeOrigin, parse, plan};
use crate::stage::{LayerSpec, Registry, Stage};
use crate::stages::terrain::climate::{self, Climate, ClimateFields, ClimateParams};
use crate::stages::terrain::support::{run_stage, table, world};
use crate::world::{Layer, World};

const N: usize = 64;
const CELL_M: f64 = 100.0;
const CELL_AREA: f64 = CELL_M * CELL_M;
const SECONDS_PER_YEAR: f64 = 3.156e7;
const THRESHOLDS: &str = "river_q = 0.01\nbig_river_q = 0.2\n";
const DEFAULT_LIMITS: Limits = Limits {
    min_depth_m: 2.0,
    min_area_m2: 4.0 * CELL_AREA,
};

/// Yearly water fields equal on every cell, mm/yr.
#[derive(Debug, Clone, Copy)]
struct Weather {
    precipitation: f64,
    pet: f64,
    runoff: f64,
}

/// Lake evaporation exceeds precipitation by 1 m/yr.
const ARID: Weather = Weather {
    precipitation: 250.0,
    pet: 1250.0,
    runoff: 20.0,
};

/// Lake evaporation exceeds precipitation by 0.6 m/yr, with more runoff than `ARID`.
const SEMI_ARID: Weather = Weather {
    precipitation: 400.0,
    pet: 1000.0,
    runoff: 50.0,
};

/// Precipitation exceeds lake evaporation.
const WET: Weather = Weather {
    precipitation: 1200.0,
    pet: 600.0,
    runoff: 600.0,
};

fn fields(w: Weather) -> ClimateFields {
    let all = |v: f64| Grid::filled(N, N, v);
    ClimateFields {
        precipitation: all(w.precipitation),
        pet: all(w.pet),
        aet: all(w.precipitation - w.runoff),
        runoff: all(w.runoff),
    }
}

/// The runoff of one cell, m³/yr.
fn cell_runoff_m3_yr(w: Weather) -> f64 {
    w.runoff / 1000.0 * CELL_AREA
}

/// The runoff of one cell, m³/s.
fn cell_runoff_m3_s(w: Weather) -> f64 {
    cell_runoff_m3_yr(w) / SECONDS_PER_YEAR
}

fn grid(f: impl Fn(usize, usize) -> f64) -> Grid<f64> {
    let data = (0..N * N).map(|i| f(i / N, i % N)).collect();
    Grid::from_vec(N, N, data).unwrap()
}

fn mask(f: impl Fn(usize, usize) -> bool) -> Grid<bool> {
    let data = (0..N * N).map(|i| f(i / N, i % N)).collect();
    Grid::from_vec(N, N, data).unwrap()
}

fn all_edges() -> Grid<bool> {
    mask(|r, c| r == 0 || c == 0 || r == N - 1 || c == N - 1)
}

fn south_edge() -> Grid<bool> {
    mask(|r, _| r == N - 1)
}

fn closed() -> Grid<bool> {
    mask(|_, _| false)
}

fn at(r: usize, c: usize) -> usize {
    r * N + c
}

/// A cone around cell (32, 32), 100 m at the centre and 5 m higher per cell out to a rim 24 cells
/// away (220 m), falling 5 m per cell beyond the rim to the edges. A channel runs from a pass at
/// (40, 32), 140 m, steeply down to the south edge, so the cone below 140 m (closer than 8 cells
/// to the centre) is the one depression.
fn bowl() -> Grid<f64> {
    grid(|r, c| {
        if c == 32 && r >= 40 {
            return if r == 40 {
                140.0
            } else {
                130.0 - 2.0 * (to_f64(r) - 41.0)
            };
        }
        let (dr, dc) = (to_f64(r) - 32.0, to_f64(c) - 32.0);
        let radius = (dr * dr + dc * dc).sqrt();
        if radius <= 24.0 {
            100.0 + 5.0 * radius
        } else {
            220.0 - 5.0 * (radius - 24.0)
        }
    })
}

/// Rolling hills with several closed depressions.
fn hills() -> Grid<f64> {
    grid(|r, c| {
        let (y, x) = (to_f64(r), to_f64(c));
        200.0
            + 40.0 * libm::sin(x / 5.0) * libm::cos(y / 6.0)
            + 25.0 * libm::sin((x + 2.0 * y) / 9.0)
            + 0.8 * y
    })
}

/// A plane falling 1 m per row to the south edge, tilted 0.01 m per column away from column 32.
fn plane() -> Grid<f64> {
    grid(|r, c| 63.0 - to_f64(r) + 0.01 * (to_f64(c) - 32.0).abs())
}

/// `height` with the cells of rows `rows` and columns `cols` lowered by `by` metres.
fn lowered(
    mut height: Grid<f64>,
    rows: std::ops::Range<usize>,
    cols: std::ops::Range<usize>,
    by: f64,
) -> Grid<f64> {
    for r in rows {
        for c in cols.clone() {
            height.set(r, c, height.get(r, c) - by);
        }
    }
    height
}

/// The pure computation behind the stage, with the default limits.
fn compute(height: &Grid<f64>, fixed: &Grid<bool>, weather: Weather) -> Water {
    water(height, fixed, &fields(weather), CELL_M, &DEFAULT_LIMITS)
}

/// A world holding `height`, `fixed` and a climate with `weather`'s fields.
fn setup(height: &Grid<f64>, fixed: &Grid<bool>, weather: Weather) -> World {
    let mut w = world(N, N);
    w.insert("height", Layer::F64(height.clone())).unwrap();
    w.insert("fixed", Layer::Mask(fixed.clone())).unwrap();
    let climate = Climate {
        params: ClimateParams::default(),
        fields: Some(fields(weather)),
    };
    w.insert("climate", Layer::Value(Box::new(climate)))
        .unwrap();
    w
}

/// Runs the stage with the test thresholds and `extra` parameters.
fn run(w: &mut World, extra: &str) {
    run_stage(w, &Hydrology, &format!("{THRESHOLDS}{extra}")).unwrap();
}

/// The stage's layers on `height`, `fixed` and `weather`.
fn staged(height: &Grid<f64>, fixed: &Grid<bool>, weather: Weather, extra: &str) -> World {
    let mut w = setup(height, fixed, weather);
    run(&mut w, extra);
    w
}

/// The cell where the water of `from` ends when it follows `flow_dir`, and the cells on its way.
fn follow(flow_dir: &Grid<u8>, from: usize) -> Vec<usize> {
    let mut path = vec![from];
    let mut cell = from;
    while flow_dir.as_slice()[cell] != 0 {
        let code = usize::from(flow_dir.as_slice()[cell]);
        cell = flow_dir.neighbour(cell, D8[code - 1]).unwrap();
        path.push(cell);
        assert!(path.len() <= N * N, "flow_dir has a cycle through {from}");
    }
    path
}

fn sum_where(values: &Grid<f64>, cells: &Grid<bool>) -> f64 {
    values
        .as_slice()
        .iter()
        .zip(cells.as_slice())
        .filter(|(_, c)| **c)
        .map(|(v, _)| v)
        .sum()
}

fn is_close(a: f64, b: f64, relative: f64) -> bool {
    (a - b).abs() <= relative * max(a.abs(), b.abs())
}

/// The lowest cell of `height`, ties by index.
fn lowest(height: &Grid<f64>) -> usize {
    let h = height.as_slice();
    (0..h.len())
        .min_by_key(|&i| (OrdF64(key(h[i])), i))
        .unwrap()
}

/// Every float layer the stage writes holds only finite values.
fn assert_finite(w: &World) {
    for name in ["height", "lake_level", "discharge", "water_surface"] {
        let layer = w.f64(name).unwrap();
        assert!(layer.as_slice().iter().all(|v| v.is_finite()), "{name}");
    }
}

// Review Focus 1 (spec §8): a closed map keeps all of its water.
#[test]
fn a_map_without_outlets_keeps_its_water() {
    let height = hills();
    let water = compute(&height, &closed(), SEMI_ARID);
    assert!(
        water.basins.iter().any(|b| b.terminal && b.lake.is_some()),
        "the hills hold at least one terminal lake"
    );
    let sink = lowest(&height);
    let receivers = &water.routing.drainage.receivers;
    let seeds: Vec<usize> = (0..N * N).filter(|&i| receivers[i] == i).collect();
    let mut expected: Vec<usize> = water
        .basins
        .iter()
        .filter(|b| b.terminal)
        .map(|b| b.depression.bottom)
        .chain([sink])
        .collect();
    expected.sort_unstable();
    assert_eq!(
        seeds, expected,
        "the seeds are the sink and the terminal bottoms"
    );

    let w = staged(&height, &closed(), SEMI_ARID, "");
    let flow_dir = w.u8("flow_dir").unwrap();
    let discharge = w.f64("discharge").unwrap();
    for i in 0..N * N {
        assert_eq!(flow_dir.as_slice()[i] == 0, seeds.contains(&i), "cell {i}");
        let end = *follow(flow_dir, i).last().unwrap();
        assert!(seeds.contains(&end), "cell {i} drains to {end}");
    }
    // The lowest cell is the sink: its depression holds no lake, it absorbs what reaches it.
    assert_eq!(flow_dir.as_slice()[sink], 0);
    assert!(!w.mask("lake").unwrap().as_slice()[sink]);
    assert!(discharge.as_slice()[sink] > 2.0 * cell_runoff_m3_s(SEMI_ARID));
    // No discharge leaves the map: the seeds hold all the runoff.
    let seed_mask = mask(|r, c| seeds.contains(&at(r, c)));
    let total = to_f64(N * N) * cell_runoff_m3_s(SEMI_ARID);
    assert!(is_close(sum_where(discharge, &seed_mask), total, 1e-9));
    assert_finite(&w);
}

// Physics test 7 (spec §5.14).
#[test]
fn the_routing_surface_drains_to_an_outlet_over_non_increasing_heights() {
    let height = hills();
    let fixed = all_edges();
    let water = compute(&height, &fixed, WET);
    assert!(water.basins.len() >= 2, "the hills have depressions");
    let drainage = &water.routing.drainage;
    let surface = drainage.surface.as_slice();
    for i in 0..N * N {
        assert!(surface[i] >= water.height.as_slice()[i]);
        let to = drainage.receivers[i];
        if fixed.as_slice()[i] {
            assert_eq!(to, i, "an outlet keeps its water");
            continue;
        }
        assert_ne!(to, i, "cell {i} is a pit of the routing surface");
        assert!(surface[to] < surface[i], "cell {i} drains uphill");
    }
    let w = staged(&height, &fixed, WET, "");
    let flow_dir = w.u8("flow_dir").unwrap();
    let water_surface = w.f64("water_surface").unwrap().as_slice();
    for i in 0..N * N {
        let path = follow(flow_dir, i);
        let end = *path.last().unwrap();
        assert!(fixed.as_slice()[end], "cell {i} ends at {end}, not an edge");
        // Cells raised by at most 1 mm belong to no lake; their water surface is the ground.
        for step in path.windows(2) {
            let rise = water_surface[step[1]] - water_surface[step[0]];
            assert!(rise <= 1.1e-3, "{step:?} rises {rise} m");
        }
    }
}

// Physics test 8 (spec §5.14).
#[test]
fn a_terminal_lake_balances_evaporation_and_inflow() {
    let height = bowl();
    let water = compute(&height, &all_edges(), ARID);
    assert_eq!(water.basins.len(), 1, "the bowl is the one depression");
    let basin = &water.basins[0];
    assert!(basin.terminal);
    let balance = basin.balance;
    let net = balance.evaporation - balance.precipitation;
    assert!((net - 1.0).abs() < 1e-12, "E - P = {net}");
    let lake = basin.lake.as_ref().unwrap();
    let evaporated = to_f64(lake.cells.len()) * CELL_AREA * net;
    let inflow = balance.q_land;
    assert!(lake.cells.len() > 10, "{} lake cells", lake.cells.len());
    assert!(lake.cells.len() < basin.depression.cells.len());
    assert!(
        evaporated <= inflow && inflow - evaporated < CELL_AREA * net,
        "evaporation {evaporated} m³/yr against inflow {inflow} m³/yr"
    );
    // The lake fills the lowest cells, at the level of the highest of them.
    let h = height.as_slice();
    let top = lake
        .cells
        .iter()
        .fold(f64::NEG_INFINITY, |m, &c| max(m, h[c]));
    assert_eq!(lake.level.to_bits(), top.to_bits());
    for &c in &basin.depression.cells {
        if lake.cells.contains(&c) {
            assert!(h[c] <= lake.level);
        } else {
            assert!(h[c] >= lake.level);
        }
    }
    // The inflow agrees with the routing: the bottom absorbs it and the depression's own runoff.
    let bottom = basin.depression.bottom;
    assert_eq!(bottom, at(32, 32));
    let absorbed = water.routing.discharge.as_slice()[bottom] * SECONDS_PER_YEAR;
    let own = to_f64(basin.depression.cells.len()) * cell_runoff_m3_yr(ARID);
    assert!(
        is_close(absorbed, inflow + own, 1e-9),
        "{absorbed} vs {inflow} + {own}"
    );
    assert_eq!(water.routing.drainage.receivers[bottom], bottom);
}

#[test]
fn wet_basins_fill_to_spill_and_route_over_their_outlet() {
    let height = bowl();
    let fixed = all_edges();
    let water = compute(&height, &fixed, WET);
    assert_eq!(water.basins.len(), 1);
    let basin = &water.basins[0];
    assert!(!basin.terminal);
    let lake = basin.lake.as_ref().unwrap();
    assert_eq!(lake.cells, basin.depression.cells);
    assert_eq!(lake.level.to_bits(), basin.depression.spill_level.to_bits());
    assert!(lake.level > 140.0 && lake.level < 140.0 + 1e-3);
    // From the bottom, the water crosses the lake, leaves over the pass and runs down the channel.
    let w = staged(&height, &fixed, WET, "");
    let flow_dir = w.u8("flow_dir").unwrap();
    let path = follow(flow_dir, basin.depression.bottom);
    let pass = at(40, 32);
    let leaves = path.iter().position(|&c| c == pass).unwrap();
    assert!(path[..leaves].iter().all(|c| lake.cells.contains(c)));
    assert_eq!(path.last(), Some(&at(63, 32)));
    let discharge = w.f64("discharge").unwrap();
    let lake_total = basin.balance.q_land + to_f64(lake.cells.len()) * cell_runoff_m3_yr(WET);
    assert!(discharge.as_slice()[pass] * SECONDS_PER_YEAR >= lake_total * (1.0 - 1e-12));
    // Nothing stays on the map.
    let total = to_f64(N * N) * cell_runoff_m3_s(WET);
    assert!(is_close(sum_where(discharge, &fixed), total, 1e-9));
    // The rasters show the lake at its level.
    let lake_mask = w.mask("lake").unwrap().as_slice();
    let level = w.f64("lake_level").unwrap().as_slice();
    let surface = w.f64("water_surface").unwrap().as_slice();
    let id = w.u32("lake_id").unwrap().as_slice();
    for i in 0..N * N {
        let wet = lake.cells.contains(&i);
        assert_eq!(lake_mask[i], wet, "cell {i}");
        assert_eq!(id[i], u32::from(wet), "cell {i}");
        let want_level = if wet { lake.level } else { 0.0 };
        assert_eq!(level[i].to_bits(), want_level.to_bits(), "cell {i}");
        let want_surface = if wet {
            lake.level
        } else {
            height.as_slice()[i]
        };
        assert_eq!(surface[i].to_bits(), want_surface.to_bits(), "cell {i}");
    }
}

#[test]
fn shallow_pits_are_filled_in_height() {
    // A: 8 cells, about 1.5 m deep; B: one cell 5 m deep; C: 4 cells 5 m deep (kept, the area is
    // exactly the limit); D: 3 cells 5 m deep.
    let mut height = lowered(plane(), 20..22, 10..14, 2.5);
    height = lowered(height, 30..31, 45..46, 5.0);
    height = lowered(height, 40..42, 20..22, 5.0);
    height = lowered(height, 50..51, 40..43, 5.0);
    let fixed = south_edge();
    let outlets: Vec<usize> = (0..N).map(|c| at(N - 1, c)).collect();
    let filled = priority_flood(&height, &outlets, 1e-6).surface;
    let w = staged(&height, &fixed, WET, "");
    let out = w.f64("height").unwrap().as_slice();
    let lake = w.mask("lake").unwrap().as_slice();
    let kept: Vec<usize> = vec![at(40, 20), at(40, 21), at(41, 20), at(41, 21)];
    let mut raised = 0;
    for i in 0..N * N {
        let (before, flood) = (height.as_slice()[i], filled.as_slice()[i]);
        if kept.contains(&i) {
            assert!(lake[i], "pit C keeps its lake at {i}");
            assert_eq!(out[i].to_bits(), before.to_bits());
        } else {
            assert!(!lake[i], "no lake at {i}");
            assert_eq!(out[i].to_bits(), flood.to_bits(), "cell {i} is filled");
            raised += usize::from(flood - before > 1e-3);
        }
    }
    assert_eq!(raised, 8 + 1 + 3, "pits A, B and D are filled");
    assert_eq!(lake.iter().filter(|l| **l).count(), 4);
}

#[test]
fn channel_discharge_follows_single_receivers() {
    // A V-valley: 1 m per row down to the mouth at (63, 32), 10 m per column up the sides, so
    // every side cell drains sideways to the axis and the axis drains south.
    let height = grid(|r, c| 63.0 - to_f64(r) + 10.0 * (to_f64(c) - 32.0).abs());
    let fixed = mask(|r, c| r == 63 && c == 32);
    let w = staged(&height, &fixed, WET, "");
    let discharge = w.f64("discharge").unwrap();
    let flow_dir = w.u8("flow_dir").unwrap();
    let q = cell_runoff_m3_s(WET);
    for r in 0..N {
        for c in 0..N {
            let (want, code) = match c {
                0..32 => (to_f64(c + 1), 3),
                32 => (to_f64(64 * (r + 1)), if r == 63 { 0 } else { 5 }),
                _ => (to_f64(64 - c), 7),
            };
            let got = discharge.get(r, c);
            assert!(
                is_close(got, want * q, 1e-12),
                "({r}, {c}): {got} vs {}",
                want * q
            );
            assert_eq!(flow_dir.get(r, c), code, "({r}, {c})");
        }
    }
    let total = to_f64(N * N) * q;
    assert!(
        is_close(discharge.get(63, 32), total, 1e-12),
        "the mouth carries all"
    );
    assert!(
        is_close(discharge.get(10, 0), q, 1e-12),
        "a side cell carries its own"
    );
}

#[test]
fn rivers_are_above_river_q_outside_lakes() {
    let w = staged(&bowl(), &all_edges(), WET, "");
    let discharge = w.f64("discharge").unwrap().as_slice();
    let lake = w.mask("lake").unwrap().as_slice();
    let river = w.mask("river").unwrap().as_slice();
    let big = w.mask("big_river").unwrap().as_slice();
    for i in 0..N * N {
        assert_eq!(river[i], discharge[i] > 0.01 && !lake[i], "cell {i}");
        assert_eq!(big[i], discharge[i] > 0.2 && !lake[i], "cell {i}");
    }
    let count = |cells: &[bool]| cells.iter().filter(|c| **c).count();
    assert!(count(river) > count(big) && count(big) > 0);
    assert!(
        (0..N * N).any(|i| lake[i] && discharge[i] > 0.01),
        "the lake carries river discharge, yet its cells are no river"
    );
    let thresholds = w.value::<WaterThresholds>("water_thresholds").unwrap();
    assert_eq!(
        *thresholds,
        WaterThresholds {
            river_q: 0.01,
            big_river_q: 0.2
        }
    );
}

#[test]
fn missing_thresholds_are_parameter_errors() {
    for (toml, parameter, text) in [
        ("big_river_q = 2.0", "river_q", "required"),
        ("river_q = 0.15", "big_river_q", "required"),
        ("river_q = 0.0\nbig_river_q = 2.0", "river_q", "positive"),
        ("river_q = nan\nbig_river_q = 2.0", "river_q", "positive"),
        (
            "river_q = 0.15\nbig_river_q = inf",
            "big_river_q",
            "positive",
        ),
        ("river_q = 3.0\nbig_river_q = 2.0", "big_river_q", "below"),
        (
            "river_q = 0.1\nbig_river_q = 2.0\nmin_lake_depth_m = -1.0",
            "min_lake_depth_m",
            "not negative",
        ),
        (
            "river_q = 0.1\nbig_river_q = 2.0\nmin_lake_area_m2 = nan",
            "min_lake_area_m2",
            "not negative",
        ),
        (
            "river_q = 0.1\nbig_river_q = 2.0\nmin_lake_cells = 12",
            "min_lake_cells",
            "unknown",
        ),
    ] {
        let err = Hydrology.params(&table(toml)).unwrap_err();
        assert_eq!(err.stage, "hydrology");
        assert_eq!(err.parameter.as_deref(), Some(parameter), "{toml}: {err}");
        assert!(err.to_string().contains(text), "{toml}: {err}");
    }
    // A recipe without the thresholds fails before any stage runs, naming the parameter.
    let recipe = "id = \"t\"\ntitle = \"T\"\nkind = \"terrain\"\ngrid = [8, 8]\n\
                  size_km = [1.0, 1.0]\n[[stage]]\nid = \"grid\"\n[[stage]]\n\
                  id = \"climate_model\"\n[[stage]]\nid = \"landscape_evolution\"\nsteps = 1\n\
                  [[stage]]\nid = \"hydrology\"\nbig_river_q = 2.0\n";
    let loaded = parse(recipe, RecipeOrigin::Builtin).unwrap();
    match plan(&loaded, &Registry::builtin(), &serde_json::Map::new()) {
        Err(RecipeError::Invalid {
            stage, parameter, ..
        }) => {
            assert_eq!(stage.as_deref(), Some("hydrology"));
            assert_eq!(parameter.as_deref(), Some("river_q"));
        }
        other => panic!("{:?}", other.map(|_| ())),
    }
}

#[test]
fn a_lake_touching_a_drained_edge_drains_away() {
    // A basin of 3 x 9 cells at 6 m against the drained south edge, whose cells stand at 10 m and
    // more: the basin fills to the edge's level and its water leaves over the edge.
    let plane = grid(|r, c| 10.0 + 0.5 * (63.0 - to_f64(r)) + 0.02 * (to_f64(c) - 32.0).abs());
    let height = grid(|r, c| {
        if (60..63).contains(&r) && (28..37).contains(&c) {
            6.0
        } else {
            plane.get(r, c)
        }
    });
    let fixed = south_edge();
    let water = compute(&height, &fixed, WET);
    assert_eq!(water.basins.len(), 1);
    let basin = &water.basins[0];
    assert!(!basin.terminal);
    let lake = basin.lake.as_ref().unwrap();
    assert_eq!(lake.cells.len(), 27);
    assert!(lake.level > 10.0 && lake.level < 10.0 + 1e-3);
    let w = staged(&height, &fixed, WET, "");
    let flow_dir = w.u8("flow_dir").unwrap();
    for &cell in &lake.cells {
        let end = *follow(flow_dir, cell).last().unwrap();
        assert!(fixed.as_slice()[end], "lake cell {cell} ends at {end}");
    }
    for i in 0..N * N {
        assert_eq!(flow_dir.as_slice()[i] == 0, fixed.as_slice()[i], "cell {i}");
    }
    let discharge = w.f64("discharge").unwrap();
    let total = to_f64(N * N) * cell_runoff_m3_s(WET);
    assert!(is_close(sum_where(discharge, &fixed), total, 1e-9));
}

#[test]
fn a_pit_exactly_at_the_depth_limit_is_kept() {
    let height = lowered(plane(), 30..32, 30..32, 4.0);
    let fixed = south_edge();
    let outlets: Vec<usize> = (0..N).map(|c| at(N - 1, c)).collect();
    let filled = priority_flood(&height, &outlets, 1e-6).surface;
    let pit = [at(30, 30), at(30, 31), at(31, 30), at(31, 31)];
    let spill = pit
        .iter()
        .fold(f64::NEG_INFINITY, |m, &c| max(m, filled.as_slice()[c]));
    let floor = pit
        .iter()
        .fold(f64::INFINITY, |m, &c| min(m, height.as_slice()[c]));
    let depth = spill - floor;
    assert!(depth > 2.0, "the pit is {depth} m deep");
    let lake_cells = |limit: f64| {
        let w = staged(
            &height,
            &fixed,
            WET,
            &format!("min_lake_depth_m = {limit:?}\n"),
        );
        let lake = w.mask("lake").unwrap().as_slice();
        let h = w.f64("height").unwrap().as_slice();
        let filled_in = pit
            .iter()
            .all(|&c| h[c].to_bits() == filled.as_slice()[c].to_bits());
        (lake.iter().filter(|l| **l).count(), filled_in)
    };
    assert_eq!(
        lake_cells(depth),
        (4, false),
        "depth == limit keeps the lake"
    );
    assert_eq!(
        lake_cells(depth.next_up()),
        (0, true),
        "shallower is filled"
    );
}

#[test]
fn a_terminal_lake_below_one_cell_is_no_lake() {
    let height = bowl();
    let dry = Weather {
        runoff: 0.001,
        ..ARID
    };
    let water = compute(&height, &all_edges(), dry);
    let basin = &water.basins[0];
    assert!(basin.terminal && basin.lake.is_none());
    let area = basin.balance.q_land / (basin.balance.evaporation - basin.balance.precipitation);
    assert!(area > 0.0 && area < CELL_AREA, "equilibrium area {area} m²");
    let w = staged(&height, &all_edges(), dry, "");
    assert!(w.mask("lake").unwrap().as_slice().iter().all(|l| !l));
    assert!(
        w.u32("lake_id")
            .unwrap()
            .as_slice()
            .iter()
            .all(|&id| id == 0)
    );
    assert!(
        w.f64("lake_level")
            .unwrap()
            .as_slice()
            .iter()
            .all(|&v| v == 0.0)
    );
    assert_eq!(
        w.f64("height").unwrap(),
        &height,
        "a deep basin is not filled"
    );
    assert_eq!(w.f64("water_surface").unwrap(), &height);
    let bottom = basin.depression.bottom;
    assert_eq!(
        w.u8("flow_dir").unwrap().as_slice()[bottom],
        0,
        "it absorbs"
    );
}

#[test]
fn a_lake_covering_most_of_the_map_stays_finite() {
    // Edges at 100 m around a floor near 10 m: one depression of 62 x 62 cells.
    let height = grid(|r, c| {
        if r == 0 || c == 0 || r == N - 1 || c == N - 1 {
            100.0
        } else {
            10.0 + 0.1 * to_f64((7 * r + 13 * c) % 5)
        }
    });
    let fixed = all_edges();
    for weather in [WET, ARID] {
        let w = staged(&height, &fixed, weather, "");
        assert_finite(&w);
        let discharge = w.f64("discharge").unwrap();
        let total = to_f64(N * N) * cell_runoff_m3_s(weather);
        let absorbed: f64 = (0..N * N)
            .filter(|&i| w.u8("flow_dir").unwrap().as_slice()[i] == 0)
            .map(|i| discharge.as_slice()[i])
            .sum();
        assert!(is_close(absorbed, total, 1e-9), "{weather:?}");
    }
    let w = staged(&height, &fixed, WET, "");
    let lake = w.mask("lake").unwrap().as_slice();
    assert_eq!(lake.iter().filter(|l| **l).count(), 62 * 62);
    let level = w.f64("lake_level").unwrap().get(32, 32);
    assert!(level > 100.0 && level < 100.0 + 1e-3, "level {level}");
}

#[test]
fn missing_climate_fields_are_computed_without_writing_climate() {
    let height = hills();
    let fixed = all_edges();
    let mut bare = world(N, N);
    bare.insert("height", Layer::F64(height.clone())).unwrap();
    bare.insert("fixed", Layer::Mask(fixed.clone())).unwrap();
    let params = ClimateParams::default();
    let unfilled = Climate {
        params: params.clone(),
        fields: None,
    };
    bare.insert("climate", Layer::Value(Box::new(unfilled)))
        .unwrap();
    run(&mut bare, "");
    assert!(bare.value::<Climate>("climate").unwrap().fields.is_none());

    let mut given = world(N, N);
    given.insert("height", Layer::F64(height.clone())).unwrap();
    given.insert("fixed", Layer::Mask(fixed)).unwrap();
    let full = Climate {
        fields: Some(climate::fields(&height, given.frame(), &params)),
        params,
    };
    given
        .insert("climate", Layer::Value(Box::new(full)))
        .unwrap();
    run(&mut given, "");
    assert_eq!(bare.f64("discharge"), given.f64("discharge"));
    assert_eq!(bare.mask("lake"), given.mask("lake"));
}

#[test]
fn the_stage_is_repeatable() {
    let a = staged(&hills(), &closed(), SEMI_ARID, "");
    let b = staged(&hills(), &closed(), SEMI_ARID, "");
    for name in ["height", "discharge", "lake_level", "water_surface"] {
        assert_eq!(a.f64(name), b.f64(name), "{name}");
    }
    assert_eq!(a.u8("flow_dir"), b.u8("flow_dir"));
    assert_eq!(a.u32("lake_id"), b.u32("lake_id"));
}

#[test]
fn the_report_counts_lakes_and_the_terminal_balance() {
    let report = |weather: Weather| {
        let mut w = setup(&bowl(), &all_edges(), weather);
        run_stage(&mut w, &Hydrology, THRESHOLDS).unwrap().values
    };
    let arid = report(ARID);
    let wet = report(WET);
    let expect = |values: &std::collections::BTreeMap<String, f64>, name: &str, want: f64| {
        let got = values[name];
        assert!((got - want).abs() < 1e-9, "{name}: {got} vs {want}");
    };
    for (name, want) in [
        ("lakes", 1.0),
        ("terminal_lakes", 1.0),
        ("filled_pits", 0.0),
        ("terminal_inflow_m3_yr", 309_800.0),
        ("terminal_evaporation_m3_yr", 30.0 * CELL_AREA),
    ] {
        expect(&arid, name, want);
    }
    expect(&wet, "lakes", 1.0);
    expect(&wet, "terminal_lakes", 0.0);
    for name in ["terminal_inflow_m3_yr", "terminal_evaporation_m3_yr"] {
        assert_eq!(wet[name].to_bits(), 0, "{name} is +0.0");
    }
}

#[test]
fn hydrology_declares_its_layers() {
    let params = Hydrology.params(&table(THRESHOLDS)).unwrap();
    let names = |specs: Vec<LayerSpec>| specs.iter().map(|s| s.name).collect::<Vec<_>>();
    assert_eq!(
        names(Stage::reads(&Hydrology, params.as_ref())),
        ["height", "climate", "fixed"]
    );
    assert_eq!(
        names(Stage::writes(&Hydrology, params.as_ref())),
        [
            "height",
            "lake",
            "lake_id",
            "lake_level",
            "river",
            "big_river",
            "discharge",
            "flow_dir",
            "water_surface",
            "water_thresholds"
        ]
    );
}

#[test]
fn the_stage_needs_its_inputs() {
    let mut w = world(8, 8);
    assert!(run_stage(&mut w, &Hydrology, THRESHOLDS).is_err());
}
