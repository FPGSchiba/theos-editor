//! Records the golden hashes of the current generator version (CE-77.2, spec §5.14).
//!
//! ```text
//! cargo run -p theos-maps --example bless
//! ```
//!
//! It runs every golden run (`theos_maps::golden::GOLDEN_RUNS`) and writes their hashes to
//! `crates/theos-maps/tests/fixtures/golden/<VERSION>.toml`. When that file exists it refuses
//! before running anything: golden hashes of a version are never overwritten, so outputs that
//! change need a new `VERSION`. Before P08 merges, the 1.0.0 file may be deleted and recorded
//! again on the branch (spec ruling 41). The hashes do not depend on the build profile.

use std::io::Write;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use theos_maps::golden::{GOLDEN_RUNS, GoldenFile, compute, golden_path, record};
use theos_maps::{GENERATOR, VERSION};

fn main() -> Result<()> {
    let path = golden_path(VERSION);
    if path
        .try_exists()
        .with_context(|| format!("cannot look for {}", path.display()))?
    {
        bail!(
            "the golden hashes of {GENERATOR} {VERSION} exist at {}; they are never \
             overwritten: raise `VERSION` in crates/theos-maps/src/api.rs to record new ones",
            path.display()
        );
    }
    let mut out = std::io::stdout().lock();
    let mut runs = Vec::with_capacity(GOLDEN_RUNS.len());
    for &(recipe, seed) in GOLDEN_RUNS {
        let started = Instant::now();
        let dir = tempfile::tempdir().context("cannot create a temporary folder")?;
        let run = compute(recipe, seed, dir.path())
            .with_context(|| format!("the golden run {recipe} seed {seed} failed"))?;
        writeln!(
            out,
            "{recipe} seed {seed}: {} stages, package {} ({:.1} s)",
            run.stages.len(),
            run.package,
            started.elapsed().as_secs_f64()
        )?;
        runs.push(run);
    }
    let file = GoldenFile {
        generator: format!("{GENERATOR}@{VERSION}"),
        run: runs,
    };
    record(&path, &file)?;
    writeln!(out, "recorded {}", path.display())?;
    Ok(())
}
