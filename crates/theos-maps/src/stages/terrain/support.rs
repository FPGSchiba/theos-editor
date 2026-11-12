//! Helpers for the unit tests of the terrain stages.

use crate::progress::{CancelToken, Progress};
use crate::stage::{Stage, StageContext, StageError, StageReport};
use crate::world::{MapFrame, World};

/// The run seed of [`run_stage`].
const SEED: u64 = 11;

/// An empty world of `rows` x `cols` cells of 100 m.
pub(crate) fn world(rows: usize, cols: usize) -> World {
    World::new(MapFrame {
        rows,
        cols,
        cell_m: 100.0,
    })
}

/// Parses `toml` into a table.
pub(crate) fn table(toml: &str) -> toml::Table {
    toml::from_str(toml).unwrap()
}

/// Runs `stage` once over `world` with the parameters `toml`, keyed `<id>#0`, and checks that
/// it wrote everything it declared.
pub(crate) fn run_stage(
    world: &mut World,
    stage: &dyn Stage,
    toml: &str,
) -> Result<StageReport, StageError> {
    let params = stage.params(&table(toml)).unwrap();
    let key = format!("{}#0", stage.id());
    let cancel = CancelToken::new();
    let mut sink = |_: &Progress| {};
    let mut ctx = StageContext::new(
        world,
        stage,
        params.as_ref(),
        &key,
        SEED,
        &cancel,
        &mut sink,
    );
    let report = stage.run(&mut ctx, params.as_ref())?;
    ctx.finish()?;
    Ok(report)
}
