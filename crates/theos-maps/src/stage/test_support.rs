//! Test stages, test values and a harness shared by the unit tests of the Stage contract.

use std::any::Any;

use serde::Deserialize;

use super::{LayerSpec, MapKind, Stage, StageContext, StageError, StageReport, TypedStage};
use crate::grid::Grid;
use crate::progress::{CancelToken, Progress};
use crate::world::{Layer, MapFrame, ValueType, World, WorldValue};

/// The run seed of every harness context.
pub(crate) const SEED: u64 = 7;

/// A value layer holding a count.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Counter(pub(crate) u32);

impl WorldValue for Counter {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        Ok(serde_json::json!({ "n": self.0 }))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for Counter {
    const NAME: &'static str = "counter";
}

/// A value type of another name than [`Counter`].
#[derive(Debug)]
pub(crate) struct Label;

impl WorldValue for Label {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        Ok(serde_json::Value::Null)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for Label {
    const NAME: &'static str = "label";
}

/// The parameters of a stage that takes none.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct NoParams {}

/// A stage declaring one layer of every kind. It reads `height`, `class`, `fixed`, `receivers`,
/// `count` and the optional `omega` and `tag`; it writes `height`, `count`, `biome` and the
/// optional `debug`.
pub(crate) struct Probe;

impl TypedStage for Probe {
    type Params = NoParams;
    const ID: &'static str = "probe";
    const SUMMARY: &'static str = "Declares one layer of every kind.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &NoParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::u8("class"),
            LayerSpec::mask("fixed"),
            LayerSpec::u32("receivers"),
            LayerSpec::value("count", Counter::NAME),
            LayerSpec::f64("omega").optional(),
            LayerSpec::value("tag", Label::NAME).optional(),
        ]
    }

    fn writes(&self, _: &NoParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::value("count", Counter::NAME),
            LayerSpec::u8("biome"),
            LayerSpec::f64("debug").optional(),
        ]
    }

    fn run(&self, _: &mut StageContext<'_>, _: &NoParams) -> Result<StageReport, StageError> {
        Ok(StageReport::default())
    }
}

/// A per-step process that reads `height` and writes `biome`.
pub(crate) struct Step;

impl TypedStage for Step {
    type Params = NoParams;
    const ID: &'static str = "step";
    const SUMMARY: &'static str = "Reads height, writes biome.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &NoParams) -> Vec<LayerSpec> {
        vec![LayerSpec::f64("height")]
    }

    fn writes(&self, _: &NoParams) -> Vec<LayerSpec> {
        vec![LayerSpec::u8("biome")]
    }

    fn run(&self, _: &mut StageContext<'_>, _: &NoParams) -> Result<StageReport, StageError> {
        Ok(StageReport::default())
    }
}

/// A stage that updates `class`, `fixed` and `receivers` in place.
pub(crate) struct Update;

impl TypedStage for Update {
    type Params = NoParams;
    const ID: &'static str = "update";
    const SUMMARY: &'static str = "Updates the integer rasters in place.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &NoParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::u8("class"),
            LayerSpec::mask("fixed"),
            LayerSpec::u32("receivers"),
        ]
    }

    fn writes(&self, params: &NoParams) -> Vec<LayerSpec> {
        TypedStage::reads(self, params)
    }

    fn run(&self, _: &mut StageContext<'_>, _: &NoParams) -> Result<StageReport, StageError> {
        Ok(StageReport::default())
    }
}

/// A 4 x 4 world of 10 m cells holding `height` (1.0), `class` (2), `fixed` (false),
/// `receivers` (0) and `count` (3).
pub(crate) fn world() -> World {
    let mut world = World::new(MapFrame {
        rows: 4,
        cols: 4,
        cell_m: 10.0,
    });
    world
        .insert("height", Layer::F64(Grid::filled(4, 4, 1.0)))
        .unwrap();
    world
        .insert("class", Layer::U8(Grid::filled(4, 4, 2)))
        .unwrap();
    world
        .insert("fixed", Layer::Mask(Grid::filled(4, 4, false)))
        .unwrap();
    world
        .insert("receivers", Layer::U32(Grid::filled(4, 4, 0)))
        .unwrap();
    world
        .insert("count", Layer::Value(Box::new(Counter(3))))
        .unwrap();
    world
}

/// A world, a cancel token and the progress events of the contexts built over them.
pub(crate) struct Harness {
    pub(crate) world: World,
    pub(crate) cancel: CancelToken,
    pub(crate) events: Vec<Progress>,
}

impl Harness {
    /// A harness over [`world`].
    pub(crate) fn new() -> Self {
        Self {
            world: world(),
            cancel: CancelToken::new(),
            events: Vec::new(),
        }
    }

    /// Runs `f` in a context of `stage` with its default parameters, keyed `<id>#0`.
    pub(crate) fn with<R>(
        &mut self,
        stage: &dyn Stage,
        f: impl FnOnce(&mut StageContext<'_>) -> R,
    ) -> R {
        let params = stage.params(&toml::Table::new()).unwrap();
        let key = format!("{}#0", stage.id());
        let events = &mut self.events;
        let mut sink = |event: &Progress| events.push(event.clone());
        let mut ctx = StageContext::new(
            &mut self.world,
            stage,
            params.as_ref(),
            &key,
            SEED,
            &self.cancel,
            &mut sink,
        );
        f(&mut ctx)
    }
}
