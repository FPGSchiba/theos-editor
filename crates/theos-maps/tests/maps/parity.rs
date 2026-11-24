//! Parity with `theos_maps` 0.3.0 (spec §5.14, CE-28.5): over seeds 1–8 of each built-in recipe,
//! the median of each parity metric lies within the allowed range around 0.3.0's values, and at
//! least as many seeds pass all the recipe's constraints as passed with 0.3.0.
//!
//! - **The reference** is `tests/fixtures/parity/theos_maps-0.3.0.toml`, a copy of the spec's
//!   Appendix A. `the_parity_fixture_is_the_spec_s_reference` checks it against the spec's
//!   summary table (§5.14).
//! - **The allowed range** of a recipe and metric (spec §5.14 "The tolerance") runs from 0.3.0's
//!   lowest to its highest value over the eight seeds. When the recipe constrains the metric, it
//!   spans the recipe's wanted range too. Lake share's range always starts at 0: fewer lakes is
//!   the intended fix (spec §4.2).
//! - **Passes:** the seeds whose run passes all its constraints (the run's `pass`, the placement
//!   checks included) are at least 0.3.0's count. `fixture-small` had no constraints in 0.3.0, so
//!   its count is 0: the recipe's own constraints (`features.ruin`, `features.chime`) are
//!   reported, and parity needs none of its seeds to pass them.
//!
//! The runs go in parallel threads, one world each. The test writes a table of every value, median
//! and range to stderr, and its failure message holds the same table.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::thread;

use serde::Deserialize;
use serde_json::Value;
use theos_maps::kernels::num::{max, min};
use theos_maps::recipe;
use theos_maps::stages::terrain::travel::TravelParams;

use crate::support::{Error, median, run_package};

/// The reference file, relative to the crate.
const FIXTURE: &str = "tests/fixtures/parity/theos_maps-0.3.0.toml";

/// The built-in recipes, in the spec's order.
const RECIPES: [&str; 3] = ["hearth-valley", "hearth-valley-omega", "fixture-small"];

/// The seeds of the parity runs.
const SEEDS: [u64; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

/// What the fixture says of a seed's constraints.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
enum Passes {
    /// Whether the seed passed all the recipe's constraints.
    Checked(bool),
    /// The recipe had no constraints: `"no constraints"`.
    Unchecked(String),
}

/// One row of Appendix A.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    recipe: String,
    seed: u64,
    relief_m: f64,
    lake_pct: f64,
    rivers_pct: f64,
    vegetation_cover_pct: f64,
    precip_median_mm: f64,
    /// The number of lakes; reported beside the run's, not compared.
    lakes: u32,
    /// 0.3.0's extraction walk, which P08 calls the Chime-to-Chime walk; reported, not compared.
    chime_to_chime_min: f64,
    passes: Passes,
    /// The constraints a failing seed missed, as the appendix names them.
    #[serde(default)]
    failed: Vec<String>,
}

/// Where the reference comes from (spec §5.14).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    toolset: String,
    python: String,
    fastscapelib: String,
    numpy: String,
    scipy: String,
    command: String,
    date: String,
    reference: String,
}

/// The parity fixture.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    provenance: Provenance,
    seed: Vec<Row>,
}

/// One parity metric of CE-28.5.
struct Metric {
    /// The summary path the run reports it under, which a recipe constraint names too.
    path: &'static str,
    /// Its value in a row of the reference.
    of: fn(&Row) -> f64,
    /// Whether its allowed range starts at 0 whatever the reference (lake share).
    floor_zero: bool,
}

/// The parity metrics, in the spec's order.
const METRICS: [Metric; 5] = [
    Metric {
        path: "elevation_m.relief",
        of: |r| r.relief_m,
        floor_zero: false,
    },
    Metric {
        path: "lake_pct",
        of: |r| r.lake_pct,
        floor_zero: true,
    },
    Metric {
        path: "rivers_pct",
        of: |r| r.rivers_pct,
        floor_zero: false,
    },
    Metric {
        path: "vegetation_cover_pct",
        of: |r| r.vegetation_cover_pct,
        floor_zero: false,
    },
    Metric {
        path: "precip_mm_yr.median",
        of: |r| r.precip_median_mm,
        floor_zero: false,
    },
];

/// The summary path of the Chime-to-Chime walk, reported beside 0.3.0's extraction walk.
const WALK: &str = "travel.chime_to_chime_min";

impl Reference {
    /// Reads the fixture.
    fn load() -> Result<Self, Error> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
        Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
    }

    /// The rows of `recipe`, in the fixture's order.
    fn rows<'a>(&'a self, recipe: &'a str) -> impl Iterator<Item = &'a Row> {
        self.seed.iter().filter(move |r| r.recipe == recipe)
    }

    /// 0.3.0's lowest and highest value of `metric` over the seeds of `recipe`.
    fn extent(&self, recipe: &str, metric: &Metric) -> Result<[f64; 2], Error> {
        let values: Vec<f64> = self.rows(recipe).map(metric.of).collect();
        let lo = values.iter().copied().min_by(f64::total_cmp);
        let hi = values.iter().copied().max_by(f64::total_cmp);
        lo.zip(hi)
            .map(|(lo, hi)| [lo, hi])
            .ok_or_else(|| format!("the fixture has no seed of {recipe}").into())
    }

    /// The allowed range of `metric`'s median for `recipe` (spec §5.14): 0.3.0's extent, widened
    /// to the recipe's wanted range `wanted` if it has one; from 0 for lake share.
    fn allowed(
        &self,
        recipe: &str,
        metric: &Metric,
        wanted: Option<[f64; 2]>,
    ) -> Result<[f64; 2], Error> {
        let [mut lo, mut hi] = self.extent(recipe, metric)?;
        if let Some([want_lo, want_hi]) = wanted {
            lo = min(lo, want_lo);
            hi = max(hi, want_hi);
        }
        if metric.floor_zero {
            lo = 0.0;
        }
        Ok([lo, hi])
    }

    /// The number of seeds of `recipe` that passed all constraints with 0.3.0.
    fn passes(&self, recipe: &str) -> usize {
        self.rows(recipe)
            .filter(|r| r.passes == Passes::Checked(true))
            .count()
    }
}

/// What the parity test reads of one run.
struct Outcome {
    /// The seed.
    seed: u64,
    /// The run's summary, read back from its package.
    summary: Value,
    /// The run's `pass`: whether it passed all its constraints.
    pass: bool,
}

impl Outcome {
    /// The number at the dotted summary `path`.
    fn get(&self, path: &str) -> Result<f64, Error> {
        path.split('.')
            .try_fold(&self.summary, |value, key| value.get(key))
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("seed {}: the summary has no {path}", self.seed).into())
    }
}

/// Runs the built-in `recipe` with `seed` into a temporary folder and reads its summary and
/// `pass`.
fn outcome(recipe: &str, seed: u64) -> Result<Outcome, Error> {
    let (package, manifest) = run_package(recipe, seed, &[])?;
    let pass = manifest
        .results
        .get("pass")
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{recipe} seed {seed}: the run has no pass"))?;
    Ok(Outcome {
        seed,
        summary: package.manifest.summary,
        pass,
    })
}

/// Runs every recipe and seed, each run in its own thread. Returns the outcomes per recipe, in
/// the order of [`RECIPES`] and [`SEEDS`].
fn outcomes() -> Result<Vec<Vec<Outcome>>, Error> {
    thread::scope(|scope| {
        let handles: Vec<Vec<_>> = RECIPES
            .iter()
            .map(|&recipe| {
                SEEDS
                    .iter()
                    .map(|&seed| (seed, scope.spawn(move || outcome(recipe, seed))))
                    .collect()
            })
            .collect();
        let mut out = Vec::new();
        for (recipe, runs) in RECIPES.iter().zip(handles) {
            let mut outcomes = Vec::new();
            for (seed, handle) in runs {
                let panicked = |_| format!("the run of {recipe} seed {seed} panicked");
                outcomes.push(handle.join().map_err(panicked)??);
            }
            out.push(outcomes);
        }
        Ok(out)
    })
}

/// `v` for the table: three decimals at most, without the noise of a sum (1.265, not
/// 1.2650000000000001).
fn shown(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// The values of the seeds, as a table cell.
fn cells(values: &[f64]) -> String {
    values
        .iter()
        .map(|&v| shown(v).to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Checks the runs of `recipe` against the reference and the recipe's `constraints` (its wanted
/// ranges), writing every number to `table` and each failure to `failures`.
fn compare(
    reference: &Reference,
    (recipe, constraints): (&str, &BTreeMap<String, [f64; 2]>),
    runs: &[Outcome],
    table: &mut String,
    failures: &mut Vec<String>,
) -> Result<(), Error> {
    writeln!(table, "{recipe}:")?;
    for metric in &METRICS {
        let values = runs
            .iter()
            .map(|r| r.get(metric.path))
            .collect::<Result<Vec<_>, _>>()?;
        let wanted = constraints.get(metric.path).copied();
        let [lo, hi] = reference.allowed(recipe, metric, wanted)?;
        let [ref_lo, ref_hi] = reference.extent(recipe, metric)?;
        let ref_median = median(reference.rows(recipe).map(metric.of));
        let m = median(values.clone());
        let ok = (lo..=hi).contains(&m);
        writeln!(
            table,
            "  {:<22} median {:<8} allowed [{lo}, {hi}] {}; 0.3.0 {ref_lo} / {} / {ref_hi}, \
             wanted {wanted:?}; seeds: {}",
            metric.path,
            shown(m),
            if ok { "ok" } else { "OUTSIDE" },
            shown(ref_median),
            cells(&values)
        )?;
        if !ok {
            failures.push(format!(
                "{recipe} {}: median {m} outside [{lo}, {hi}]",
                metric.path
            ));
        }
    }
    let walks: Vec<f64> = runs.iter().filter_map(|r| r.get(WALK).ok()).collect();
    let ref_walk = median(reference.rows(recipe).map(|r| r.chime_to_chime_min));
    let ref_lakes = median(reference.rows(recipe).map(|r| f64::from(r.lakes)));
    writeln!(
        table,
        "  reported only: {WALK} median {} (0.3.0 {}); 0.3.0's median lake count {ref_lakes}",
        shown(median(walks)),
        shown(ref_walk)
    )?;
    let passed: Vec<u64> = runs.iter().filter(|r| r.pass).map(|r| r.seed).collect();
    let wanted = reference.passes(recipe);
    writeln!(
        table,
        "  seeds passing all constraints: {} {passed:?}, at least {wanted} (0.3.0)",
        passed.len()
    )?;
    if passed.len() < wanted {
        failures.push(format!(
            "{recipe}: {} seeds pass all constraints, fewer than 0.3.0's {wanted}",
            passed.len()
        ));
    }
    Ok(())
}

// AC: CE-28.5
#[test]
fn slow_built_in_recipes_stay_within_the_0_3_0_range() -> Result<(), Error> {
    let reference = Reference::load()?;
    let catalog = recipe::catalog(&[])?;
    let mut table = String::new();
    let mut failures = Vec::new();
    for (recipe, runs) in RECIPES.iter().zip(outcomes()?) {
        let loaded = catalog
            .recipes
            .get(*recipe)
            .ok_or_else(|| format!("{recipe} is not a built-in recipe"))?;
        let constraints = (*recipe, &loaded.recipe.constraints);
        compare(&reference, constraints, &runs, &mut table, &mut failures)?;
    }
    std::io::stderr().lock().write_all(table.as_bytes())?;
    assert!(failures.is_empty(), "failed: {failures:#?}\n{table}");
    Ok(())
}

/// Whether `row` meets every constraint in `constraints` under today's recipe (spec §5.14,
/// Appendix A). A metric constraint is a range its 0.3.0 value must lie in; the walk is divided
/// by the off-path factor first (ruling 64: P08 walks off roads at that fraction of 0.3.0's
/// speed); the placement checks (`features.*`) count as met, as Appendix A states.
fn row_meets(row: &Row, constraints: &BTreeMap<String, [f64; 2]>) -> Result<bool, Error> {
    let off_path = TravelParams::default().off_path_factor;
    for (path, [lo, hi]) in constraints {
        let value = if path.starts_with("features.") {
            continue;
        } else if path == WALK {
            row.chime_to_chime_min / off_path
        } else {
            let metric = METRICS
                .iter()
                .find(|m| m.path == path)
                .ok_or_else(|| format!("the parity test does not know the constraint {path}"))?;
            (metric.of)(row)
        };
        if !(*lo..=*hi).contains(&value) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The seeds of `recipe` that 0.3.0's values pass under `constraints`.
fn reference_passes(
    reference: &Reference,
    recipe: &str,
    constraints: &BTreeMap<String, [f64; 2]>,
) -> Result<Vec<u64>, Error> {
    let mut seeds = Vec::new();
    for row in reference.rows(recipe) {
        if row_meets(row, constraints)? {
            seeds.push(row.seed);
        }
    }
    Ok(seeds)
}

#[test]
fn the_built_in_constraints_reproduce_every_0_3_0_verdict() -> Result<(), Error> {
    let reference = Reference::load()?;
    let catalog = recipe::catalog(&[])?;
    for recipe in RECIPES {
        let loaded = catalog
            .recipes
            .get(recipe)
            .ok_or_else(|| format!("{recipe} is not a built-in recipe"))?;
        for row in reference.rows(recipe) {
            // A recipe without constraints in 0.3.0 passes by definition.
            let expected = match row.passes {
                Passes::Checked(passed) => passed,
                Passes::Unchecked(_) => true,
            };
            assert_eq!(
                row_meets(row, &loaded.recipe.constraints)?,
                expected,
                "{recipe} seed {}: today's constraints change 0.3.0's verdict",
                row.seed
            );
        }
        let seeds = reference_passes(&reference, recipe, &loaded.recipe.constraints)?;
        if recipe != "fixture-small" {
            assert_eq!(seeds.len(), reference.passes(recipe), "{recipe}");
        }
    }
    Ok(())
}

#[test]
fn widening_a_constraint_changes_a_verdict_and_the_test_sees_it() -> Result<(), Error> {
    let reference = Reference::load()?;
    let catalog = recipe::catalog(&[])?;
    // Some seeds fail on two constraints, so a case widens every range that seed misses.
    for (recipe, paths) in [
        ("hearth-valley", &["elevation_m.relief", "lake_pct"][..]),
        ("hearth-valley-omega", &[WALK][..]),
        ("hearth-valley-omega", &["vegetation_cover_pct"][..]),
    ] {
        let loaded = catalog
            .recipes
            .get(recipe)
            .ok_or_else(|| format!("{recipe} is not a built-in recipe"))?;
        let mut widened = loaded.recipe.constraints.clone();
        for path in paths {
            *widened
                .get_mut(*path)
                .ok_or_else(|| format!("{recipe} does not constrain {path}"))? = [0.0, 1.0e6];
        }
        let before = reference_passes(&reference, recipe, &loaded.recipe.constraints)?;
        let after = reference_passes(&reference, recipe, &widened)?;
        assert_ne!(
            before, after,
            "{recipe} {paths:?}: widening changed no verdict"
        );
    }
    Ok(())
}

/// The spec's summary table (§5.14): per recipe, 0.3.0's min / median / max of each metric in the
/// order of [`METRICS`], and the seeds passing all constraints.
const SPEC_TABLE: [(&str, [[f64; 3]; 5], usize); 3] = [
    (
        "hearth-valley",
        [
            [1281.0, 1765.5, 2809.0],
            [1.46, 13.31, 32.04],
            [0.93, 1.32, 1.87],
            [37.8, 55.55, 82.4],
            [735.0, 911.5, 1091.0],
        ],
        2,
    ),
    (
        "hearth-valley-omega",
        [
            [1243.0, 1786.0, 2782.0],
            [1.38, 17.57, 31.14],
            [0.88, 1.33, 1.78],
            [37.8, 55.35, 82.5],
            [731.0, 909.5, 1104.0],
        ],
        3,
    ),
    (
        "fixture-small",
        [
            [212.0, 349.0, 2292.0],
            [0.0, 2.64, 11.25],
            [0.0, 0.32, 2.25],
            [32.0, 38.45, 59.8],
            [623.0, 695.0, 852.0],
        ],
        0,
    ),
];

/// Whether the fixture's value `a` rounds to the spec table's `b`: the table gives two decimals
/// at most, and a median of two values may have a third (13.315 shows as 13.31).
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.005 + 1e-9
}

#[test]
fn the_parity_fixture_is_the_spec_s_reference() -> Result<(), Error> {
    let reference = Reference::load()?;
    let p = &reference.provenance;
    assert_eq!(p.toolset, "theos_maps 0.3.0");
    assert_eq!(
        [&p.python, &p.fastscapelib, &p.numpy, &p.scipy],
        ["3.12", "0.3.0", "2.5.4", "1.18.1"]
    );
    assert!(p.command.starts_with("uv run"), "{}", p.command);
    assert_eq!(p.date, "2026-11-10");
    assert_eq!(p.reference, "Jann's export, in no repository");
    assert_eq!(reference.seed.len(), RECIPES.len() * SEEDS.len());
    for (recipe, summary, passes) in SPEC_TABLE {
        let rows: Vec<&Row> = reference.rows(recipe).collect();
        let seeds: Vec<u64> = rows.iter().map(|r| r.seed).collect();
        assert_eq!(seeds, SEEDS, "{recipe}");
        for (metric, [lo, mid, hi]) in METRICS.iter().zip(summary) {
            let [min, max] = reference.extent(recipe, metric)?;
            let m = median(rows.iter().map(|r| (metric.of)(r)));
            assert!(
                close(min, lo) && close(m, mid) && close(max, hi),
                "{recipe} {}: {min} / {m} / {max} against the spec's {lo} / {mid} / {hi}",
                metric.path
            );
        }
        assert_eq!(reference.passes(recipe), passes, "{recipe}");
        for row in rows {
            match &row.passes {
                Passes::Checked(true) => assert!(row.failed.is_empty(), "{recipe} {}", row.seed),
                Passes::Checked(false) => assert!(!row.failed.is_empty(), "{recipe} {}", row.seed),
                Passes::Unchecked(text) => {
                    assert_eq!(text, "no constraints", "{recipe} {}", row.seed);
                    assert_eq!(passes, 0, "{recipe} has constraints in the spec");
                }
            }
        }
    }
    Ok(())
}
