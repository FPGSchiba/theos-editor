//! What `landscape_evolution` declares for its process list: its reads, its writes and the
//! order of its processes (spec §5.3 "Two levels, one trait").

use std::collections::{BTreeMap, BTreeSet};

use super::processes::PlannedProcess;
use super::{Evolution, initial};
use crate::stage::{LayerSpec, ParamError, Registry, Stage, TypedStage};
use crate::world::LayerKind;

/// Each planned process with its index and its stage in `registry`, in order. A process the
/// registry lacks is skipped; `run` fails on it.
fn with_stages<'a>(
    registry: &'a Registry,
    planned: &'a [PlannedProcess],
) -> impl Iterator<Item = (usize, &'a PlannedProcess, &'a dyn Stage)> {
    planned
        .iter()
        .enumerate()
        .filter_map(|(i, p)| registry.get(p.id).map(|stage| (i, p, stage)))
}

/// The names of the layers `stage` surely writes with the parameters of `process`; optional
/// writes satisfy no later read, as between pipeline stages.
fn required_writes(stage: &dyn Stage, process: &PlannedProcess) -> Vec<LayerSpec> {
    let mut writes = stage.writes(process.params.as_ref());
    writes.retain(|w| !w.optional);
    writes
}

/// Adds `spec` to `specs` unless a spec of that name is there; a required `spec` makes an
/// optional one of that name required.
fn merge(specs: &mut Vec<LayerSpec>, spec: LayerSpec) {
    match specs.iter_mut().find(|s| s.name == spec.name) {
        Some(existing) => existing.optional &= spec.optional,
        None => specs.push(spec),
    }
}

/// The stage's reads: `fixed` for the initial surface, then, for each process in order, the
/// reads that neither the initial surface nor an earlier process writes.
pub(super) fn reads(registry: &Registry, planned: &[PlannedProcess]) -> Vec<LayerSpec> {
    let mut written: BTreeSet<&str> = initial::WRITES.iter().map(|w| w.name).collect();
    let mut reads = vec![initial::READ];
    for (_, process, stage) in with_stages(registry, planned) {
        for read in stage.reads(process.params.as_ref()) {
            if !written.contains(read.name) {
                merge(&mut reads, read);
            }
        }
        written.extend(required_writes(stage, process).iter().map(|w| w.name));
    }
    reads
}

/// The stage's writes: those of the initial surface and of every process, each layer once.
pub(super) fn writes(registry: &Registry, planned: &[PlannedProcess]) -> Vec<LayerSpec> {
    let mut writes = initial::WRITES.to_vec();
    for (_, process, stage) in with_stages(registry, planned) {
        for write in stage.writes(process.params.as_ref()) {
            merge(&mut writes, write);
        }
    }
    writes
}

/// Checks that every process's required reads are `available` before the stage, written by the
/// initial surface or written by an earlier process, and that a layer written inside the stage
/// is read as the kind it was written as.
///
/// # Errors
/// [`ParamError`] of `landscape_evolution` at `process[<i>]`, naming the process and the layer.
pub(super) fn check_order(
    registry: &Registry,
    planned: &[PlannedProcess],
    available: &BTreeSet<&str>,
) -> Result<(), ParamError> {
    let mut written: BTreeMap<&str, LayerKind> =
        initial::WRITES.iter().map(|w| (w.name, w.kind)).collect();
    for (i, process, stage) in with_stages(registry, planned) {
        let at = || format!("process[{i}]");
        for read in stage.reads(process.params.as_ref()) {
            let (id, layer) = (process.id, read.name);
            match written.get(layer) {
                Some(kind) if *kind != read.kind => {
                    let message = format!(
                        "process `{id}` reads `{layer}` as {}, but it is written as {kind} \
                         before it",
                        read.kind
                    );
                    return Err(ParamError::at(Evolution::ID, at(), message));
                }
                Some(_) => {}
                None if read.optional || available.contains(layer) => {}
                None => {
                    let message = format!(
                        "process `{id}` reads `{layer}`, which no earlier process and no \
                         earlier stage writes"
                    );
                    return Err(ParamError::at(Evolution::ID, at(), message));
                }
            }
        }
        for write in required_writes(stage, process) {
            written.insert(write.name, write.kind);
        }
    }
    Ok(())
}
