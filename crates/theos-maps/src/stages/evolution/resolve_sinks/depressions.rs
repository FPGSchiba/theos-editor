//! The world value `depressions` that `resolve_sinks` writes for the processes after it.

use std::any::Any;

use serde::Serialize;

use crate::grid::Grid;
use crate::world::{ValueType, WorldValue};

/// Where a depression spills: over the pass from its cell `cell` to the neighbour `to`, which
/// lies in the basin the depression drains into.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SpillPoint {
    /// The spill node: the lake's cell on the pass it spills over.
    pub cell: u32,
    /// The cell across the pass, downstream.
    pub to: u32,
    /// The pass height in metres, `max(height[cell], height[to])`: the level the lake spills
    /// at. The routing surface at `cell` stands at this level.
    pub level: f64,
}

/// The world value `depressions`: the depressions `resolve_sinks` routed in the current step.
#[derive(Debug, Clone, PartialEq)]
pub struct Depressions {
    /// Per cell, the number `n` (from 1) of the depression holding it; 0 outside every
    /// depression. A depression is one lake: the cells its routing surface lifts above `height`
    /// to the lake's level, the cells of basins nested in it (whose passes lie below that level)
    /// included.
    pub label: Grid<u32>,
    /// The spill point of depression `n` at index `n − 1`. The cell across its pass,
    /// `label[spill[n − 1].to]`, is 0 (outside every depression) or less than `n`. That does
    /// not order the depressions by where their water goes: the flow from a spill cell can
    /// still reach a depression with a larger number, so a walk downstream takes them in
    /// reverse stack order of their spill cells.
    pub spill: Vec<SpillPoint>,
}

impl WorldValue for Depressions {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        let spill = theos_map_format::json::to_checked_value(&self.spill)?;
        Ok(serde_json::json!({"label": self.label.as_slice(), "spill": spill}))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for Depressions {
    const NAME: &'static str = "depressions";
}
