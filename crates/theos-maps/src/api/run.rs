//! `run`: one recipe and seed, from the catalog to the files in the output folder (spec §5.6).
//!
//! The order: find the recipe in the catalog, refuse `from`, plan with the request's inputs,
//! check that the stages write every layer the outputs read (ledger ruling P3), claim the output
//! names (nothing is written when one is taken), run the pipeline, then the
//! steps `constraints`, `features`, `renders` and `package`, each with its own progress event
//! and cancel check, and with panics caught like a stage's. The outputs get their names only at
//! the end; on any error, on cancel and while a panic unwinds, the files the run created are
//! removed and nothing else.

use std::path::{Path, PathBuf};

use theos_map_format::sha256_hex;

use super::needs;
use super::outputs::{FEATURES, Outputs, PACKAGE};
use super::steps::{self, Steps};
use super::{GENERATOR, ManifestFrame, RunManifest, RunRequest, VERSION};
use crate::error::RunError;
use crate::golden::GoldenRun;
use crate::package::RunRecord;
use crate::progress::{CancelToken, Progress};
use crate::recipe::{self, LoadedRecipe, RecipeError};
use crate::runner::execute;
use crate::stage::Registry;

/// Runs the recipe `request.recipe` (from the built-ins and `recipe_dirs`, later folders winning
/// by id) with `request.seed`, and writes its outputs into the existing folder `out_dir`: the
/// renders `base`, `truth`, `inworld`, `geology` and `resources`, the recipe's extra renders, the
/// `features` and the `package`, named `s<seed>-<role>.png`, `s<seed>-features.geojson` and
/// `s<seed>.theosmap` (plan ruling L10).
///
/// `progress` gets a `{stage, done, total}` event before every pipeline stage, the events long
/// stages report themselves, and one at the start of each step after the pipeline
/// (`constraints`, `features`, one `renders` event per render, `package`). These events name the
/// steps in order, not the order of the work inside them: the features are already built in
/// `constraints` (the `features.<type>` counts come from their ids), and the `features` event
/// starts the writing of their file. `cancel` stops the run at its next cancel point. Request
/// notes are ignored: the generator accepts none in P08.
///
/// # Errors
/// [`RunError::Recipe`] when the recipe is unknown, `from` is set, an input is given, the
/// recipe is invalid or its stages do not write a layer the package or a render reads; [`RunError::OutputExists`] when a file in `out_dir` has an output's name;
/// [`RunError::OverBudget`]; [`RunError::Cancelled`]; the stage errors of the pipeline
/// ([`RunError::StageFailed`], [`RunError::StagePanicked`], [`RunError::FindingFailed`],
/// [`RunError::NonFinite`], [`RunError::NonFiniteValue`]); [`RunError::StagePanicked`] naming the
/// step (`constraints`,
/// `features`, `renders` or `package`) when a step after the pipeline panics;
/// [`RunError::Render`], [`RunError::Output`], [`RunError::Format`] and [`RunError::Io`] from
/// the outputs. On every error the files the run created are removed.
///
/// # Panics
/// Not on its own: a panic inside a stage or inside a step after the pipeline is caught and
/// returned as [`RunError::StagePanicked`]. Only a panic of the caller's `progress` closure on
/// the event the runner reports before a pipeline stage unwinds out of `run`; no output is
/// written while the pipeline runs, so it leaves no file behind.
pub fn run(
    request: &RunRequest,
    recipe_dirs: &[PathBuf],
    out_dir: &Path,
    progress: &mut dyn FnMut(&Progress),
    cancel: &CancelToken,
) -> Result<RunManifest, RunError> {
    run_inner(
        &Registry::builtin(),
        request,
        recipe_dirs,
        out_dir,
        progress,
        cancel,
    )
    .map(|(manifest, _)| manifest)
}

/// [`run`] with the stages of `registry`, also returning the golden record: the stage hashes of
/// the pipeline, `features` = the SHA-256 of the features file's bytes and `package` = that of
/// the `.theosmap` (CE-77.1).
///
/// # Errors
/// The errors of [`run`].
pub(crate) fn run_inner(
    registry: &Registry,
    request: &RunRequest,
    recipe_dirs: &[PathBuf],
    out_dir: &Path,
    progress: &mut dyn FnMut(&Progress),
    cancel: &CancelToken,
) -> Result<(RunManifest, GoldenRun), RunError> {
    let loaded = find(request, recipe_dirs)?;
    let plan = recipe::plan(&loaded, registry, &request.inputs)?;
    needs::check(&plan)?;
    let renders = needs::render_names(&plan);
    let mut roles: Vec<String> = renders.iter().map(|r| (*r).to_owned()).collect();
    roles.extend([FEATURES.to_owned(), PACKAGE.to_owned()]);
    let mut outputs = Outputs::claim(out_dir, request.seed, &roles)?;
    let seed = request.seed;
    let execution = execute(&plan, seed, registry, &request.options, progress, cancel)?;
    let world = &execution.world;
    let mut post = Steps::new(progress, cancel, execution.timing_s.clone());

    let checked = post.timed("constraints", |s| {
        s.report("constraints", 0, 1)?;
        steps::constraints(&execution, &plan, seed)
    })?;
    let features_sha = post.timed(FEATURES, |s| {
        s.report(FEATURES, 0, 1)?;
        let bytes = steps::features_bytes(&checked.features)?;
        outputs.write(FEATURES, &bytes)?;
        Ok(sha256_hex(&bytes))
    })?;
    post.timed("renders", |s| {
        for (done, name) in renders.iter().enumerate() {
            s.report("renders", done, renders.len())?;
            let png = steps::draw(name, world, &plan, &checked.title, s.cancel())?;
            outputs.write(name, &png)?;
        }
        Ok(())
    })?;
    let record = RunRecord {
        raw_recipe: &loaded.raw,
        seed,
        inputs: &request.inputs,
        results: checked.results.clone(),
        summary: checked.summary,
        features: checked.features,
    };
    let package_sha = post.timed(PACKAGE, |s| {
        s.report(PACKAGE, 0, 1)?;
        let bytes = steps::package_bytes(world, &plan, record, s.cancel())?;
        outputs.write(PACKAGE, &bytes)?;
        Ok(sha256_hex(&bytes))
    })?;
    let files = outputs.commit()?;

    let manifest = RunManifest {
        generator: format!("{GENERATOR}@{VERSION}"),
        recipe: plan.recipe_id.clone(),
        seed,
        inputs: request.inputs.clone(),
        frame: ManifestFrame {
            size_m: plan.frame.size_m(),
        },
        outputs: files,
        results: checked.results,
        timing: post.into_timing(),
    };
    let mut golden = execution.golden;
    golden.features = features_sha;
    golden.package = package_sha;
    Ok((manifest, golden))
}

/// The recipe of `request` from the catalog of the built-ins and `recipe_dirs`, refused when the
/// request starts from another map.
fn find(request: &RunRequest, recipe_dirs: &[PathBuf]) -> Result<LoadedRecipe, RecipeError> {
    let mut catalog = recipe::catalog(recipe_dirs)?;
    let Some(loaded) = catalog.recipes.remove(&request.recipe) else {
        // A broken file that names this id hid the recipe: report its error, not "unknown".
        return Err(
            match catalog
                .problems
                .into_iter()
                .find(|p| p.id.as_deref() == Some(request.recipe.as_str()))
            {
                Some(problem) => RecipeError::Parse {
                    origin: problem.file.display().to_string(),
                    message: problem.message,
                },
                None => RecipeError::UnknownRecipe(request.recipe.clone()),
            },
        );
    };
    if request.from.is_some() {
        return Err(RecipeError::Invalid {
            recipe: loaded.recipe.id.clone(),
            stage: None,
            layer: None,
            parameter: Some("from".to_owned()),
            message: "a terrain recipe starts from no other map, so `from` must be empty"
                .to_owned(),
        });
    }
    Ok(loaded)
}

#[cfg(test)]
mod tests;
