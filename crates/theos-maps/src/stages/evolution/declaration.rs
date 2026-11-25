//! What `landscape_evolution` declares for its process list: its reads, its writes and the
//! order of its processes (spec §5.3 "Two levels, one trait").

use std::collections::{BTreeMap, BTreeSet};

use super::hillslope_order;
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
/// optional one of that name required. Kinds are not compared here: [`check_order`] refuses a
/// layer declared as two kinds before anything runs.
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

/// The first declaration of a layer inside the stage: its kind, and who declared it how, as
/// words (``process `route` (process[0]) writes``).
struct First {
    kind: LayerKind,
    by: String,
}

/// Checks the process list before the run:
/// - `hillslope` is not after `fluvial` or a routing process ([`hillslope_order::check`]);
/// - every process's required reads are `available` before the stage, written by the initial
///   surface or written by an earlier process;
/// - every layer is declared as one kind throughout: a process that reads or writes a layer as
///   another kind than the initial surface or an earlier process declared it is refused.
///
/// # Errors
/// [`ParamError`] of `landscape_evolution` at `process[<i>]`, naming the process and the layer
/// (and, for a kind conflict, both kinds and who declared the first), or, for a misplaced
/// `hillslope`, the process it follows ([`hillslope_order::check`]).
pub(super) fn check_order(
    registry: &Registry,
    planned: &[PlannedProcess],
    available: &BTreeSet<&str>,
) -> Result<(), ParamError> {
    hillslope_order::check(planned)?;
    let initial = |access: &str, spec: &LayerSpec| {
        let by = format!("the initial surface {access}");
        (
            spec.name,
            First {
                kind: spec.kind,
                by,
            },
        )
    };
    let mut first: BTreeMap<&str, First> = std::iter::once(initial("reads", &initial::READ))
        .chain(initial::WRITES.iter().map(|w| initial("writes", w)))
        .collect();
    let mut written: BTreeSet<&str> = initial::WRITES.iter().map(|w| w.name).collect();
    for (i, process, stage) in with_stages(registry, planned) {
        let at = || format!("process[{i}]");
        let (id, params) = (process.id, process.params.as_ref());
        let reads = stage.reads(params).into_iter().map(|spec| ("reads", spec));
        let writes = stage
            .writes(params)
            .into_iter()
            .map(|spec| ("writes", spec));
        for (access, spec) in reads.chain(writes) {
            let layer = spec.name;
            match first.get(layer) {
                Some(earlier) if earlier.kind != spec.kind => {
                    let message = format!(
                        "process `{id}` {access} `{layer}` as {}, but {} it as {}",
                        spec.kind, earlier.by, earlier.kind
                    );
                    return Err(ParamError::at(Evolution::ID, at(), message));
                }
                Some(_) => {}
                None => {
                    let by = format!("process `{id}` ({}) {access}", at());
                    first.insert(
                        layer,
                        First {
                            kind: spec.kind,
                            by,
                        },
                    );
                }
            }
            let missing = access == "reads"
                && !spec.optional
                && !written.contains(layer)
                && !available.contains(layer);
            if missing {
                let message = format!(
                    "process `{id}` reads `{layer}`, which no earlier process and no earlier \
                     stage writes"
                );
                return Err(ParamError::at(Evolution::ID, at(), message));
            }
        }
        written.extend(required_writes(stage, process).iter().map(|w| w.name));
    }
    Ok(())
}
