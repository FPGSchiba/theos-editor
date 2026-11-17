//! Physics test 13 (spec §5.14), resolution independence: each pair is one recipe on 192² cells
//! of 125 m and on 384² cells of 62.5 m over the same 24 km.
//! - `hearth-192` and `hearth-384` are `hearth-valley`.
//! - `basin-192` and `basin-384` hold a closed basin, so that lakes are present at both sizes:
//!   hearth-valley's lakes are few (most seeds have none at either size), and its lake check
//!   alone would rest on pairs of zeros. The basin is a sag basin that sinks faster than sediment
//!   can fill it, under a dry climate, so its lake is terminal and the water balance sets its
//!   area. Every run of this pair must hold a lake.
//!
//! The noise draws its coefficients per wavenumber of the map (spec §5.8), so a seed gives the
//! same landscape at both sizes and the test compares each seed with itself. For every pair and
//! metric it takes the ratio of the 192² value to the 384² value (the recipes' own size) per
//! seed, and the median of the ratios over seeds 1–4 must lie within the metric's tolerance of 1.
//! Two zeros agree (ratio 1); a zero at 384² against a value at 192² is an infinite ratio.
//!
//! The metrics count what both grids resolve:
//! - **relief** (15 %): `elevation_m.relief` of the summary;
//! - **lake share over lakes of at least 0.25 km²** (15 %): the share of the map covered by
//!   8-connected bodies of `water` = lake of at least [`LAKE_MIN_M2`]. That is 16 cells of 125 m
//!   and 64 of 62.5 m, a whole number of cells on both grids, and a 4 x 4 body is the smallest
//!   the coarse grid draws as a lake rather than as a pit of a few cells;
//! - **river length per km² through lakes** (25 %): the length of the channel network, every
//!   cell whose `discharge` exceeds the recipe's `river_q`, lake cells included, each counted with
//!   its D8 step to its receiver (`flow_dir`: one cell side, or √2 of it on a diagonal; nothing
//!   at an outlet or a terminal lake bottom), over the map's km².

use std::f64::consts::SQRT_2;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufReader, Write as _};
use std::thread;

use theos_map_format::{LayerData, LayerKey, Package};
use theos_maps::{CancelToken, Progress, run};

use crate::support::{recipes, request};

/// An error that can cross from a run's thread to the test.
type Error = Box<dyn std::error::Error + Send + Sync>;

/// The seeds of test 13.
const SEEDS: [u64; 4] = [1, 2, 3, 4];

/// One recipe at both grid sizes.
struct Pair {
    /// The recipe on 192² cells of 125 m.
    coarse: &'static str,
    /// The recipe on 384² cells of 62.5 m.
    fine: &'static str,
    /// Whether every run must hold a lake body of at least [`LAKE_MIN_M2`].
    lakes: bool,
}

/// The pairs of test 13.
const PAIRS: [Pair; 2] = [
    Pair {
        coarse: "hearth-192",
        fine: "hearth-384",
        lakes: false,
    },
    Pair {
        coarse: "basin-192",
        fine: "basin-384",
        lakes: true,
    },
];
/// The smallest lake body the lake share counts, m²: 0.25 km².
const LAKE_MIN_M2: f64 = 250_000.0;
/// The class of a lake cell in the layer `water`.
const LAKE: u8 = 3;

/// What test 13 compares of one run.
#[derive(Debug, Clone, Copy)]
struct Landscape {
    /// `elevation_m.relief` of the summary, m.
    relief_m: f64,
    /// The share of the map in lake bodies of at least [`LAKE_MIN_M2`], %.
    lake_pct: f64,
    /// The channel network's length through lakes over the map's area, km per km².
    river_km_per_km2: f64,
}

/// One metric of test 13: its name, how to read it and its tolerance.
struct Metric {
    name: &'static str,
    of: fn(&Landscape) -> f64,
    tolerance: f64,
}

const METRICS: [Metric; 3] = [
    Metric {
        name: "relief (m)",
        of: |l| l.relief_m,
        tolerance: 0.15,
    },
    Metric {
        name: "lake share over 0.25 km² (%)",
        of: |l| l.lake_pct,
        tolerance: 0.15,
    },
    Metric {
        name: "river length through lakes (km/km²)",
        of: |l| l.river_km_per_km2,
        tolerance: 0.25,
    },
];

/// The values of the top-level layer `name` of `package`.
fn layer<'a>(package: &'a Package, name: &str) -> Result<&'a LayerData, Error> {
    let key = LayerKey {
        level: None,
        name: name.to_owned(),
    };
    package
        .layers
        .get(&key)
        .ok_or_else(|| format!("the package has no layer {name}").into())
}

/// The `river_q` of the recipe's `hydrology` stage, m³/s.
fn river_q(package: &Package) -> Result<f64, Error> {
    package.manifest.recipe["stage"]
        .as_array()
        .and_then(|stages| stages.iter().find(|s| s["id"] == "hydrology"))
        .and_then(|stage| stage["river_q"].as_f64())
        .ok_or_else(|| "the recipe has no hydrology river_q".into())
}

/// The cells 8-adjacent to cell `i` of a `rows x cols` grid.
fn neighbours(i: usize, rows: usize, cols: usize) -> impl Iterator<Item = usize> {
    let (r, c) = (i / cols, i % cols);
    (r.saturating_sub(1)..=(r + 1).min(rows - 1))
        .flat_map(move |rr| (c.saturating_sub(1)..=(c + 1).min(cols - 1)).map(move |cc| (rr, cc)))
        .map(move |(rr, cc)| rr * cols + cc)
        .filter(move |&j| j != i)
}

/// The area of the 8-connected lake bodies of at least [`LAKE_MIN_M2`] in `water`, m².
fn big_lake_area_m2(water: &[u8], rows: usize, cols: usize, cell_m2: f64) -> f64 {
    let mut seen = vec![false; water.len()];
    let mut total = 0.0;
    for start in 0..water.len() {
        if water[start] != LAKE || seen[start] {
            continue;
        }
        seen[start] = true;
        let (mut stack, mut area) = (vec![start], 0.0);
        while let Some(i) = stack.pop() {
            area += cell_m2;
            for j in neighbours(i, rows, cols) {
                if water[j] == LAKE && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        if area >= LAKE_MIN_M2 {
            total += area;
        }
    }
    total
}

/// The length of the channel network through lakes, m: every cell with more than `river_q`
/// of `discharge` adds its D8 step (`flow_dir` 1–8 from north, clockwise; 0 has no step).
fn channel_length_m(discharge: &[f32], flow_dir: &[u8], river_q: f64, cell_m: f64) -> f64 {
    discharge
        .iter()
        .zip(flow_dir)
        .filter(|(q, _)| f64::from(**q) > river_q)
        .map(|(_, d)| match d {
            1 | 3 | 5 | 7 => cell_m,
            2 | 4 | 6 | 8 => cell_m * SQRT_2,
            _ => 0.0,
        })
        .sum()
}

/// Test 13's metrics of the package of one run.
fn measure(package: &Package) -> Result<Landscape, Error> {
    let summary = &package.manifest.summary;
    let frame = &package.manifest.frame;
    let [rows, cols] = frame.grid.map(|n| usize::try_from(n).unwrap_or(0));
    let [width_m, height_m] = frame.size_m;
    let map_m2 = width_m * height_m;
    let relief_m = summary["elevation_m"]["relief"]
        .as_f64()
        .ok_or("the summary has no relief")?;
    let (LayerData::U8(water), LayerData::F32(discharge), LayerData::U8(flow_dir)) = (
        layer(package, "water")?,
        layer(package, "discharge")?,
        layer(package, "flow_dir")?,
    ) else {
        return Err("water, discharge or flow_dir has an unexpected type".into());
    };
    let cell_m2 = frame.cell_m * frame.cell_m;
    let lakes_m2 = big_lake_area_m2(water, rows, cols, cell_m2);
    let river_m = channel_length_m(discharge, flow_dir, river_q(package)?, frame.cell_m);
    Ok(Landscape {
        relief_m,
        lake_pct: 100.0 * lakes_m2 / map_m2,
        river_km_per_km2: (river_m / 1000.0) / (map_m2 / 1.0e6),
    })
}

/// Runs `recipe` with `seed` into a temporary folder and measures its package.
fn landscape(recipe: &str, seed: u64) -> Result<Landscape, Error> {
    let dir = tempfile::tempdir()?;
    let manifest = run(
        &request(recipe, seed),
        &recipes(),
        dir.path(),
        &mut |_: &Progress| {},
        &CancelToken::new(),
    )?;
    let file = manifest
        .outputs
        .get("package")
        .ok_or("the run wrote no package")?;
    let package = theos_map_format::read(BufReader::new(File::open(dir.path().join(file))?))?;
    measure(&package).map_err(|e| format!("{recipe} seed {seed}: {e}").into())
}

/// numpy's median: the middle value, or the mean of the two middle values.
fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n.is_multiple_of(2) {
        f64::midpoint(values[n / 2 - 1], values[n / 2])
    } else {
        values[n / 2]
    }
}

/// The coarse value over the fine one; two zeros agree.
fn ratio(coarse: f64, fine: f64) -> f64 {
    if coarse == 0.0 && fine == 0.0 {
        1.0
    } else {
        coarse / fine
    }
}

/// The `(coarse, fine)` landscapes of one seed of a pair.
type Runs = Vec<(u64, Landscape, Landscape)>;

/// Runs every pair and seed of test 13, each run in its own thread (every run has its own
/// world). Returns, per pair, the `(coarse, fine)` landscapes of each seed.
fn landscapes() -> Result<Vec<Runs>, Error> {
    thread::scope(|scope| {
        let handles: Vec<Vec<_>> = PAIRS
            .iter()
            .map(|pair| {
                SEEDS
                    .iter()
                    .map(|&seed| {
                        let coarse = scope.spawn(move || landscape(pair.coarse, seed));
                        let fine = scope.spawn(move || landscape(pair.fine, seed));
                        (seed, coarse, fine)
                    })
                    .collect()
            })
            .collect();
        let mut out = Vec::new();
        for (pair, runs) in PAIRS.iter().zip(handles) {
            let mut landscapes = Vec::new();
            for (seed, coarse, fine) in runs {
                let panicked = |_| format!("a run of {} seed {seed} panicked", pair.coarse);
                landscapes.push((
                    seed,
                    coarse.join().map_err(panicked)??,
                    fine.join().map_err(panicked)??,
                ));
            }
            out.push(landscapes);
        }
        Ok(out)
    })
}

/// Compares the runs of `pair` metric by metric, writing the numbers to `table` and every
/// failure to `failures`.
fn compare(
    pair: &Pair,
    runs: &Runs,
    table: &mut String,
    failures: &mut Vec<String>,
) -> Result<(), Error> {
    for metric in &METRICS {
        writeln!(table, "{} / {}, {}:", pair.coarse, pair.fine, metric.name)?;
        let mut ratios = Vec::new();
        for (seed, coarse, fine) in runs {
            let (c, f) = ((metric.of)(coarse), (metric.of)(fine));
            let r = ratio(c, f);
            writeln!(table, "  s{seed}: {c:.4} / {f:.4} = {r:.3}")?;
            ratios.push(r);
        }
        let of_medians = ratio(
            median(runs.iter().map(|run| (metric.of)(&run.1)).collect()),
            median(runs.iter().map(|run| (metric.of)(&run.2)).collect()),
        );
        let paired = median(ratios);
        writeln!(
            table,
            "  median of the ratios {paired:.3} (tolerance {}); ratio of the medians {of_medians:.3}",
            metric.tolerance
        )?;
        if (paired - 1.0).abs() > metric.tolerance {
            failures.push(format!("{}: {}", pair.coarse, metric.name));
        }
    }
    if pair.lakes {
        for (seed, coarse, fine) in runs {
            if coarse.lake_pct <= 0.0 || fine.lake_pct <= 0.0 {
                failures.push(format!("{} seed {seed}: a run without a lake", pair.coarse));
            }
        }
    }
    Ok(())
}

#[test]
fn slow_resolution_does_not_change_the_landscape() -> Result<(), Error> {
    let mut table = String::new();
    let mut failures = Vec::new();
    for (pair, runs) in PAIRS.iter().zip(landscapes()?) {
        compare(pair, &runs, &mut table, &mut failures)?;
    }
    std::io::stderr().lock().write_all(table.as_bytes())?;
    assert!(failures.is_empty(), "failed: {failures:?}\n{table}");
    Ok(())
}
