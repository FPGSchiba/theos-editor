//! The steps after the pipeline (spec §5.6, item 4): `constraints`, `features`, `renders` and
//! `package`. Each turns the finished world into results or into the bytes of one output;
//! [`super::run`] reports their progress and writes the bytes.
//!
//! The progress events name the steps in this order, which is not quite the order of the work:
//! `constraints` already builds the features, because the `features.<type>` counts come from
//! their ids; the `features` step then writes them.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use geojson::FeatureCollection;
use serde_json::{Map, Value};
use theos_map_format::FormatError;

use crate::constraints;
use crate::error::RunError;
use crate::features::FeatureSet;
use crate::features::output as features;
use crate::kernels::num::to_u32;
use crate::metrics;
use crate::package::{self, RunRecord};
use crate::progress::{CancelToken, Progress};
use crate::recipe::Plan;
use crate::render::{self, RenderError, RenderInput};
use crate::runner::{Execution, panic_message};
use crate::stage::TypedStage;
use crate::stages::terrain::travel::Travel;
use crate::summary::summary;
use crate::world::World;

/// The progress events, the cancel checks and the wall times of the steps after the pipeline.
pub(super) struct Steps<'a> {
    progress: &'a mut dyn FnMut(&Progress),
    cancel: &'a CancelToken,
    timing: BTreeMap<String, f64>,
}

impl<'a> Steps<'a> {
    /// Steps that report to `progress`, stop on `cancel` and add their times to `timing`.
    pub(super) fn new(
        progress: &'a mut dyn FnMut(&Progress),
        cancel: &'a CancelToken,
        timing: BTreeMap<String, f64>,
    ) -> Self {
        Self {
            progress,
            cancel,
            timing,
        }
    }

    /// The run's cancel token.
    pub(super) fn cancel(&self) -> &'a CancelToken {
        self.cancel
    }

    /// Reports `{stage: step, done, total}`, then checks the cancel token, as the runner does
    /// before every stage (spec §5.6).
    ///
    /// # Errors
    /// [`RunError::Cancelled`] once the token is set.
    pub(super) fn report(&mut self, step: &str, done: usize, total: usize) -> Result<(), RunError> {
        (self.progress)(&Progress {
            stage: step.to_owned(),
            done: to_u32(done),
            total: to_u32(total),
        });
        if self.cancel.is_cancelled() {
            return Err(RunError::Cancelled);
        }
        Ok(())
    }

    /// Runs `work` as the step `name` with panics caught, as the runner runs a stage (spec
    /// §5.6): `run` lives inside the editor's process (P09), so a panicking step must end the
    /// run, not unwind into the caller. Records the step's wall time under `name`.
    ///
    /// # Errors
    /// The errors of `work`; [`RunError::StagePanicked`] naming the step, with the panic's
    /// message, when `work` panics.
    pub(super) fn timed<T>(
        &mut self,
        name: &str,
        work: impl FnOnce(&mut Self) -> Result<T, RunError>,
    ) -> Result<T, RunError> {
        let started = Instant::now();
        let out = catch_unwind(AssertUnwindSafe(|| work(self))).map_err(|payload| {
            RunError::StagePanicked {
                stage: name.to_owned(),
                message: panic_message(&*payload),
            }
        })??;
        self.timing
            .insert(name.to_owned(), started.elapsed().as_secs_f64());
        Ok(out)
    }

    /// The wall times of the stages and of the steps, in seconds.
    pub(super) fn into_timing(self) -> BTreeMap<String, f64> {
        self.timing
    }
}

/// What the `constraints` step finds: the features with their ids, the summary, the results
/// and the title line of the renders.
pub(super) struct Checked {
    /// The features output.
    pub features: FeatureCollection,
    /// The run summary (spec §5.14).
    pub summary: Value,
    /// The constraint results with `pass` (`docs/generators.md` §3).
    pub results: Map<String, Value>,
    /// The title line of `truth`: recipe, seed, relief, lakes, Chime walk.
    pub title: String,
}

/// The `constraints` step: assigns the feature ids (their counts are the `features.<type>`
/// metrics, so this comes first), computes the metrics and the summary, and evaluates the
/// recipe's constraints into the results.
///
/// # Errors
/// [`RunError::Output`] for `features` when the features cannot be built, and for `package`
/// when a number of the summary is NaN or an infinity (the summary goes into its manifest).
pub(super) fn constraints(
    execution: &Execution,
    plan: &Plan,
    seed: u64,
) -> Result<Checked, RunError> {
    let world = &execution.world;
    let (collection, counts) =
        features::build(world).map_err(|e| output_error(super::outputs::FEATURES, &e))?;
    let travel = execution
        .stage_metrics
        .iter()
        .find(|(id, _)| id == Travel::ID)
        .map(|(_, metrics)| metrics);
    let not_finite =
        |e: FormatError| output_error(super::outputs::PACKAGE, &format!("the run summary: {e}"));
    let metrics = metrics::compute(world, travel).map_err(not_finite)?;
    let title = render::title_line(&plan.recipe_id, seed, &metrics);
    let summary = summary(
        metrics,
        &counts,
        &execution.stage_metrics,
        &execution.findings,
    )
    .map_err(not_finite)?;
    let results = constraints::evaluate(&plan.constraints, &summary);
    Ok(Checked {
        features: collection,
        summary,
        results,
        title,
    })
}

/// The `features` output: the collection as canonical JSON, written by the function the package
/// writer uses for `features.geojson`, so the two are the same bytes (map-package §5).
///
/// # Errors
/// [`RunError::Output`] for `features` when the collection cannot be turned into JSON.
pub(super) fn features_bytes(collection: &FeatureCollection) -> Result<Vec<u8>, RunError> {
    theos_map_format::features_bytes(collection)
        .map_err(|e| output_error(super::outputs::FEATURES, &e))
}

/// The render `name` of `world` as PNG bytes, with the recipe's `[render.<name>]` options
/// (plan ruling L9) and the title line.
///
/// # Errors
/// [`RunError::Cancelled`] when the render stopped on the token; [`RunError::Render`] naming
/// the render for any other failure.
pub(super) fn draw(
    name: &str,
    world: &World,
    plan: &Plan,
    title: &str,
    cancel: &CancelToken,
) -> Result<Vec<u8>, RunError> {
    let failed = |error: RenderError| match error {
        RenderError::Cancelled => RunError::Cancelled,
        other => RunError::Render {
            render: name.to_owned(),
            message: other.to_string(),
        },
    };
    let Some(draw) = render::render(name) else {
        return Err(RunError::Render {
            render: name.to_owned(),
            message: "no render has this name".to_owned(),
        });
    };
    let none = FeatureSet::default();
    let input = RenderInput {
        world,
        features: world.value::<FeatureSet>("features").unwrap_or(&none),
        options: plan.render_options.get(name),
        cancel,
        title: title.to_owned(),
    };
    let pixmap = draw(&input).map_err(failed)?;
    render::encode_png(&pixmap).map_err(failed)
}

/// The package of `world` as `.theosmap` bytes, written with the token checked between entries
/// (spec §5.6).
///
/// # Errors
/// [`RunError::Output`] for `package` when the package cannot be built; [`RunError::Cancelled`];
/// [`RunError::Format`] when the writer refuses it.
pub(super) fn package_bytes(
    world: &World,
    plan: &Plan,
    record: RunRecord<'_>,
    cancel: &CancelToken,
) -> Result<Vec<u8>, RunError> {
    let package = package::build(world, plan, record)
        .map_err(|e| output_error(super::outputs::PACKAGE, &e))?;
    match theos_map_format::write_cancellable(&package, Cursor::new(Vec::new()), &|| {
        cancel.is_cancelled()
    }) {
        Ok(written) => Ok(written.into_inner()),
        Err(FormatError::Cancelled) => Err(RunError::Cancelled),
        Err(other) => Err(RunError::Format(other)),
    }
}

/// [`RunError::Output`] for `role`.
fn output_error(role: &str, error: &dyn std::fmt::Display) -> RunError {
    RunError::Output {
        role: role.to_owned(),
        message: error.to_string(),
    }
}
