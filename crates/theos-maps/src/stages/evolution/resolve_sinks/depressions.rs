//! The world value `depressions` that `resolve_sinks` writes for the processes after it.

use std::any::Any;

use crate::grid::Grid;
use crate::world::{ValueType, WorldValue};

/// Where a depression spills: over the pass from its cell `cell` to the neighbour `to`, which
/// lies in the basin the depression drains into.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpillPoint {
    /// The spill node: the depression's basin's cell on the pass.
    pub cell: u32,
    /// The cell across the pass, downstream.
    pub to: u32,
    /// The pass height in metres, `max(height[cell], height[to])`. The routing surface at `cell`
    /// stands at this level, or higher when `to` lies in a lifted depression: then at the level
    /// of `to` on the routing surface.
    pub level: f64,
}

/// The world value `depressions`: the depressions `resolve_sinks` routed in the current step.
#[derive(Debug, Clone, PartialEq)]
pub struct Depressions {
    /// Per cell, the number `n` (from 1) of the depression holding it; 0 outside every
    /// depression. A depression is the set of cells its routing surface lifts above `height`.
    pub label: Grid<u32>,
    /// The spill point of depression `n` at index `n − 1`. A depression drains into the root
    /// basin or into a depression with a smaller number.
    pub spill: Vec<SpillPoint>,
}

impl WorldValue for Depressions {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> serde_json::Value {
        let spill: Vec<serde_json::Value> = self
            .spill
            .iter()
            .map(|s| serde_json::json!({"cell": s.cell, "to": s.to, "level": s.level}))
            .collect();
        serde_json::json!({"label": self.label.as_slice(), "spill": spill})
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
