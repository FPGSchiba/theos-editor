//! The context a stage runs in: its declared layers, its random streams, the cancel token and
//! the progress callback.

mod layers;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use super::{Access, LayerSpec, Stage, StageError, StageParams};
use crate::progress::{CancelToken, Progress};
use crate::rng::StageRng;
use crate::world::{Layer, LayerKind, MapFrame, World};

/// Where a per-step process is in the time loop of the stage that runs it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepInfo {
    /// The step, from 0.
    pub step: u32,
    /// The number of steps.
    pub steps: u32,
    /// The length of one step in years.
    pub dt_yr: f64,
}

/// What a stage reaches the world through. It hands out only the layers the stage declared for
/// its parameters: a read needs a declared read, a write a declared write of the same kind, and
/// [`StageContext::finish`] checks that every required write happened (spec §5.3).
///
/// Reads borrow the context and writes borrow it mutably, so a stage reads, computes an owned
/// result and writes it. An in-place update takes the layer out (`take_*`, which needs the layer
/// declared as both a read and a write) and writes it back.
///
/// # Reading and taking layers
///
/// Every read ([`StageContext::f64`], [`StageContext::value`], ...) fails with
/// [`StageError::Undeclared`] when the layer is not a declared read, and with
/// [`StageError::WrongKind`] when it is declared, or held by the world, as another kind. A
/// required read fails with [`StageError::Missing`] when the world does not hold the layer; an
/// optional read (`*_opt`) returns `None` instead. A take (`take_*`) also needs a declared write
/// of the same kind. It removes the layer from the world until the stage writes it back, and a
/// refused take leaves the layer in place.
pub struct StageContext<'w> {
    world: &'w mut World,
    /// The name in errors and progress events: the stage id, `<parent>/<process id>` in a child.
    stage: String,
    /// The key of the random streams: `<id>#<occurrence>`, longer in a child.
    key: String,
    reads: Vec<LayerSpec>,
    writes: Vec<LayerSpec>,
    written: BTreeSet<String>,
    /// The written sets of the contexts this one is a child of, nearest first.
    ancestors: Vec<&'w mut BTreeSet<String>>,
    rng: StageRng,
    run_seed: u64,
    cancel: CancelToken,
    progress: &'w mut dyn FnMut(&Progress),
    step: Option<StepInfo>,
}

impl<'w> StageContext<'w> {
    /// The context of `stage` with `params` over `world`: its declaration comes from the stage,
    /// its random stream is `StageRng::for_key(run_seed, key)`. The runner builds one per
    /// pipeline stage; [`StageContext::child`] builds those of per-step processes the same way.
    pub(crate) fn new(
        world: &'w mut World,
        stage: &dyn Stage,
        params: &dyn StageParams,
        key: &str,
        run_seed: u64,
        cancel: &CancelToken,
        progress: &'w mut dyn FnMut(&Progress),
    ) -> Self {
        Self {
            world,
            stage: stage.id().to_owned(),
            key: key.to_owned(),
            reads: stage.reads(params),
            writes: stage.writes(params),
            written: BTreeSet::new(),
            ancestors: Vec::new(),
            rng: StageRng::for_key(run_seed, key),
            run_seed,
            cancel: cancel.clone(),
            progress,
            step: None,
        }
    }

    /// The context of the per-step `process` with `params` inside this stage, keyed `key`
    /// (`<this key>/<process id>#<occurrence>`), at `step`.
    ///
    /// It hands out only the process's declared layers and names itself
    /// `<this stage>/<process id>`. Its writes count as writes of this context too, so
    /// [`StageContext::finish`] and [`StageContext::written`] of this stage see them.
    pub fn child(
        &mut self,
        process: &dyn Stage,
        params: &dyn StageParams,
        key: &str,
        step: StepInfo,
    ) -> StageContext<'_> {
        let mut ancestors = Vec::with_capacity(self.ancestors.len() + 1);
        ancestors.push(&mut self.written);
        ancestors.extend(self.ancestors.iter_mut().map(|set| &mut **set));
        let mut child = StageContext::new(
            &mut *self.world,
            process,
            params,
            key,
            self.run_seed,
            &self.cancel,
            &mut *self.progress,
        );
        child.stage = format!("{}/{}", self.stage, process.id());
        child.ancestors = ancestors;
        child.step = Some(step);
        child
    }

    /// The frame (cell size, grid, extent).
    #[must_use]
    pub fn frame(&self) -> &MapFrame {
        self.world.frame()
    }

    /// The name in errors and progress events: the stage id, `<parent>/<process id>` in a child.
    #[must_use]
    pub fn stage(&self) -> &str {
        &self.stage
    }

    /// The key of the random streams, `<id>#<occurrence>` for a pipeline stage.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The time step of a per-step process; `None` for a pipeline stage.
    #[must_use]
    pub fn step(&self) -> Option<StepInfo> {
        self.step
    }

    /// The stage's own random stream.
    pub fn rng(&mut self) -> &mut StageRng {
        &mut self.rng
    }

    /// A random stream for one `purpose` of the stage, `for_key(seed, "<key>/<purpose>")`. It
    /// starts afresh at every call and does not depend on the stage's own stream.
    #[must_use]
    pub fn sub_rng(&self, purpose: &str) -> StageRng {
        StageRng::for_key(self.run_seed, &format!("{}/{purpose}", self.key))
    }

    /// Whether the run was cancelled.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// A cancel point.
    ///
    /// # Errors
    /// [`StageError::Cancelled`] when the run was cancelled.
    pub fn check_cancel(&self) -> Result<(), StageError> {
        if self.is_cancelled() {
            Err(StageError::Cancelled)
        } else {
            Ok(())
        }
    }

    /// Reports `done` of `total` steps under this context's name.
    pub fn progress(&mut self, done: u32, total: u32) {
        (self.progress)(&Progress {
            stage: self.stage.clone(),
            done,
            total,
        });
    }

    /// [`StageError::Failed`] naming this stage.
    #[must_use]
    pub fn fail(&self, message: impl Into<String>) -> StageError {
        StageError::Failed {
            stage: self.stage.clone(),
            message: message.into(),
        }
    }

    /// Stores `layer` under `name`, replacing the layer of that name.
    ///
    /// # Errors
    /// [`StageError::Undeclared`] when `name` is not a declared write,
    /// [`StageError::WrongKind`] when `layer` is not of the declared kind, and
    /// [`StageError::World`] when a raster does not have the frame's shape.
    pub fn write(&mut self, name: &str, layer: Layer) -> Result<(), StageError> {
        self.check_declared(name, Access::Write, layer.kind())?;
        self.world
            .insert(name, layer)
            .map_err(|source| StageError::World {
                stage: self.stage.clone(),
                source,
            })?;
        self.written.insert(name.to_owned());
        for written in &mut self.ancestors {
            written.insert(name.to_owned());
        }
        Ok(())
    }

    /// The layers written through this context or its children, by name.
    #[must_use]
    pub fn written(&self) -> &BTreeSet<String> {
        &self.written
    }

    /// Checks that every declared write that is not optional was written and is still held.
    ///
    /// # Errors
    /// [`StageError::NotWritten`] naming the first missing layer.
    pub fn finish(&self) -> Result<(), StageError> {
        let missing = self
            .writes
            .iter()
            .filter(|spec| !spec.optional)
            .find(|spec| !self.written.contains(spec.name) || self.world.get(spec.name).is_none());
        match missing {
            Some(spec) => Err(StageError::NotWritten {
                stage: self.stage.clone(),
                layer: spec.name.to_owned(),
            }),
            None => Ok(()),
        }
    }

    /// Checks that `name` is declared for `access` as `kind`.
    fn check_declared(
        &self,
        name: &str,
        access: Access,
        kind: LayerKind,
    ) -> Result<(), StageError> {
        let specs = match access {
            Access::Read => &self.reads,
            Access::Write => &self.writes,
        };
        match specs.iter().find(|spec| spec.name == name) {
            None => Err(StageError::Undeclared {
                stage: self.stage.clone(),
                layer: name.to_owned(),
                access,
            }),
            Some(spec) if spec.kind != kind => Err(self.wrong_kind(name, spec.kind, kind)),
            Some(_) => Ok(()),
        }
    }

    /// [`StageError::WrongKind`] for `name`.
    fn wrong_kind(&self, name: &str, expected: LayerKind, found: LayerKind) -> StageError {
        StageError::WrongKind {
            stage: self.stage.clone(),
            layer: name.to_owned(),
            expected,
            found,
        }
    }

    /// [`StageError::Missing`] for `name`.
    fn missing(&self, name: &str) -> StageError {
        StageError::Missing {
            stage: self.stage.clone(),
            layer: name.to_owned(),
        }
    }
}
