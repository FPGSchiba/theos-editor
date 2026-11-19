//! The developer example (spec §5.15): runs terrain recipes over a range of seeds.
//!
//! ```text
//! cargo run -p theos-maps --release --example run -- <recipe>… --seeds 1..8 --out <dir> [--recipes <dir>]
//! ```
//!
//! For each recipe and seed it writes the run's outputs and its `manifest.json` (canonical JSON)
//! into `<out>/<recipe>/s<seed>/`, and for each recipe a contact sheet of the seeds' `truth`
//! renders as `<out>/<recipe>/contact-sheet.png`. On stdout it writes a Markdown table of the
//! parity metrics per seed, their medians and the number of seeds that pass the recipe's
//! constraints. `--seeds` includes both ends; the runs go in `--jobs` threads, one world each.

use std::fs::File;
use std::io::{BufReader, Write};
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Instant;

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use serde_json::Value;
use theos_maps::kernels::stats;
use theos_maps::metrics::PATHS;
use theos_maps::{
    CancelToken, Progress, RunManifest, RunOptions, RunRequest, describe, render, run,
};

/// The tiles in one row of a contact sheet.
const SHEET_COLUMNS: usize = 4;

/// The command line.
#[derive(Debug, Parser)]
#[command(about = "Runs terrain recipes and writes each seed's outputs and a contact sheet")]
struct Args {
    /// The ids of the recipes to run.
    #[arg(required = true)]
    recipe: Vec<String>,
    /// The seeds, `first..last` with both ends included, or a single seed.
    #[arg(long, default_value = "1..8", value_parser = seeds)]
    seeds: RangeInclusive<u64>,
    /// The folder the outputs go to; created when missing.
    #[arg(long)]
    out: PathBuf,
    /// A folder of recipes that win over the built-in ones by id; may be given more than once.
    #[arg(long = "recipes", value_name = "DIR")]
    recipe_dirs: Vec<PathBuf>,
    /// How many runs go at once.
    #[arg(long, default_value_t = 4)]
    jobs: usize,
}

/// Parses `first..last` (both included) or a single seed.
fn seeds(text: &str) -> Result<RangeInclusive<u64>, String> {
    let number = |s: &str| {
        s.trim()
            .parse::<u64>()
            .map_err(|e| format!("`{s}` is not a seed: {e}"))
    };
    let (first, last) = match text.split_once("..") {
        Some((first, last)) => (number(first)?, number(last)?),
        None => (number(text)?, number(text)?),
    };
    if first > last {
        return Err(format!("`{text}` holds no seed: {first} is above {last}"));
    }
    Ok(first..=last)
}

/// One finished run.
struct Done {
    /// The run's place in the list of runs: recipes in the order given, seeds ascending.
    job: usize,
    /// The recipe id.
    recipe: String,
    /// The seed.
    seed: u64,
    /// The folder of the run's outputs.
    dir: PathBuf,
    /// What `run` returned.
    manifest: RunManifest,
    /// The run summary, read back from the package.
    summary: Value,
}

/// Fails unless every recipe is one `describe` offers; a broken recipe file gives its error.
fn check_recipes(recipes: &[String], recipe_dirs: &[PathBuf]) -> Result<()> {
    let description = describe(recipe_dirs)?;
    for id in recipes {
        let problem = description
            .problems
            .iter()
            .find(|p| p.id.as_deref() == Some(id));
        if let Some(p) = problem {
            bail!("recipe `{id}` ({}): {}", p.file.display(), p.message);
        }
        if !description.recipes.iter().any(|r| r.id == *id) {
            bail!("unknown recipe `{id}`");
        }
    }
    Ok(())
}

/// The summary of a run, from the manifest inside its package.
fn read_summary(dir: &Path, manifest: &RunManifest) -> Result<Value> {
    let name = manifest
        .outputs
        .get("package")
        .context("the run wrote no package")?;
    let path = dir.join(name);
    let file = File::open(&path).with_context(|| format!("cannot open {}", path.display()))?;
    let package = theos_map_format::read(BufReader::new(file))
        .with_context(|| format!("cannot read {}", path.display()))?;
    Ok(package.manifest.summary)
}

/// Runs `recipe` with `seed` into `<out>/<recipe>/s<seed>/` and writes its `manifest.json`.
fn run_seed(job: usize, recipe: &str, seed: u64, args: &Args) -> Result<Done> {
    let dir = args.out.join(recipe).join(format!("s{seed}"));
    std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let request = RunRequest {
        recipe: recipe.to_owned(),
        seed,
        inputs: serde_json::Map::new(),
        from: None,
        notes: Vec::new(),
        options: RunOptions::default(),
    };
    let started = Instant::now();
    let manifest = run(
        &request,
        &args.recipe_dirs,
        &dir,
        &mut |_: &Progress| {},
        &CancelToken::new(),
    )
    .with_context(|| format!("{recipe} seed {seed}"))?;
    let json = theos_map_format::json::to_canonical_vec(&serde_json::to_value(&manifest)?)?;
    let path = dir.join("manifest.json");
    std::fs::write(&path, json).with_context(|| format!("cannot write {}", path.display()))?;
    let summary = read_summary(&dir, &manifest)?;
    writeln!(
        std::io::stdout().lock(),
        "{recipe} s{seed}: done in {:.1} s",
        started.elapsed().as_secs_f64()
    )?;
    Ok(Done {
        job,
        recipe: recipe.to_owned(),
        seed,
        dir,
        manifest,
        summary,
    })
}

/// Runs every recipe and seed in `args.jobs` threads and returns the runs in job order. The
/// first error stops the threads from taking more runs.
fn run_all(args: &Args) -> Result<Vec<Done>> {
    let jobs: Vec<(&str, u64)> = args
        .recipe
        .iter()
        .flat_map(|r| args.seeds.clone().map(move |s| (r.as_str(), s)))
        .collect();
    let next = AtomicUsize::new(0);
    let threads = args.jobs.min(jobs.len()).max(1);
    let worker = || -> Result<Vec<Done>> {
        let mut done = Vec::new();
        loop {
            let job = next.fetch_add(1, Ordering::Relaxed);
            let Some(&(recipe, seed)) = jobs.get(job) else {
                return Ok(done);
            };
            match run_seed(job, recipe, seed, args) {
                Ok(d) => done.push(d),
                Err(e) => {
                    next.store(jobs.len(), Ordering::Relaxed);
                    return Err(e);
                }
            }
        }
    };
    let mut all = thread::scope(|scope| {
        let handles: Vec<_> = (0..threads).map(|_| scope.spawn(worker)).collect();
        let mut all = Vec::new();
        for handle in handles {
            all.extend(handle.join().map_err(|_| anyhow!("a run panicked"))??);
        }
        Ok::<_, anyhow::Error>(all)
    })?;
    all.sort_by_key(|d| d.job);
    Ok(all)
}

/// The number at the dotted `path` of `summary`.
fn metric(summary: &Value, path: &str) -> Option<f64> {
    path.split('.')
        .try_fold(summary, |value, key| value.get(key))?
        .as_f64()
}

/// A table cell: the number to two decimals at most (the summary's own precision), or `-`
/// when the summary has none.
fn cell(value: Option<f64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |v| ((v * 100.0).round() / 100.0).to_string(),
    )
}

/// Writes the Markdown table: a row per run with the parity metrics and `pass`, then per
/// recipe a row of the medians with the number of seeds that pass.
fn write_table(out: &mut impl Write, recipes: &[String], done: &[Done]) -> Result<()> {
    writeln!(out, "| recipe | seed | {} | pass |", PATHS.join(" | "))?;
    writeln!(out, "|---|---|{}---|", "---|".repeat(PATHS.len()))?;
    for recipe in recipes {
        let runs: Vec<&Done> = done.iter().filter(|d| d.recipe == *recipe).collect();
        let mut passes = 0;
        for d in &runs {
            let pass = d.manifest.results.get("pass").and_then(Value::as_bool);
            passes += usize::from(pass == Some(true));
            let cells: Vec<String> = PATHS.iter().map(|p| cell(metric(&d.summary, p))).collect();
            let pass = pass.map_or_else(|| "-".to_owned(), |p| p.to_string());
            writeln!(
                out,
                "| {recipe} | {} | {} | {pass} |",
                d.seed,
                cells.join(" | ")
            )?;
        }
        let medians: Vec<String> = PATHS
            .iter()
            .map(|p| {
                let values: Vec<f64> = runs.iter().filter_map(|d| metric(&d.summary, p)).collect();
                cell(stats::median(&values).ok())
            })
            .collect();
        writeln!(
            out,
            "| {recipe} | median | {} | {passes} of {} |",
            medians.join(" | "),
            runs.len()
        )?;
    }
    Ok(())
}

/// Writes `<out>/<recipe>/contact-sheet.png` from the `truth` render of every seed of each
/// recipe and names each sheet on `out`.
fn write_sheets(out: &mut impl Write, args: &Args, done: &[Done]) -> Result<()> {
    for recipe in &args.recipe {
        let tiles = done
            .iter()
            .filter(|d| d.recipe == *recipe)
            .map(|d| {
                let truth = d.manifest.outputs.get("truth").with_context(|| {
                    format!("{recipe} seed {}: the run wrote no truth render", d.seed)
                })?;
                Ok((format!("{recipe} s{}", d.seed), d.dir.join(truth)))
            })
            .collect::<Result<Vec<_>>>()?;
        let path = args.out.join(recipe).join("contact-sheet.png");
        render::contact_sheet(&tiles, SHEET_COLUMNS.min(tiles.len()), &path)
            .with_context(|| format!("cannot write {}", path.display()))?;
        writeln!(out, "contact sheet: {}", path.display())?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let started = Instant::now();
    check_recipes(&args.recipe, &args.recipe_dirs)?;
    let done = run_all(&args)?;
    let mut out = std::io::stdout().lock();
    writeln!(out)?;
    write_table(&mut out, &args.recipe, &done)?;
    writeln!(out)?;
    write_sheets(&mut out, &args, &done)?;
    writeln!(
        out,
        "{} runs in {:.1} s",
        done.len(),
        started.elapsed().as_secs_f64()
    )?;
    Ok(())
}
