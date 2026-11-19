//! The parameters of `roads`, their defaults and their checks.
//!
//! Costs are in metres of flat, open road: a step of length `L` on flat bare ground costs `L`.
//! 0.3.0 (`features.py::_road_cost`) added dimensionless terms per cell to a cost per metre and
//! routed over cells of 62.5 m; its per-cell constants are converted here:
//! - `comfort_grade` 0.08 is 0.3.0's `comfort_slope`;
//! - `forest_cost` 0.8 multiplies length as 0.3.0's `forest_cost` did;
//! - `ford_cost` 250 m is 0.3.0's 4 per river cell over the cell's 62.5 m of path;
//! - `bridge_cost` 25 per metre of span is 0.3.0's 25 per big-river cell, per metre;
//! - `loop_detour` 1.7 and `merge_discount` 0.35 (0.3.0's `reuse`) are 0.3.0's;
//! - `max_grade` per class is spec ruling 35; `max_bridge_span_m` 400 and `clearance_m` 3 are the
//!   spec's (§5.9);
//! - `angle_penalty`, `deck_cost` and `width_m` are new: a crossing along the river costs twice a
//!   perpendicular one, a metre of deck rise costs the 12.5 m of ramp a road climbs it at the
//!   comfort grade, and the widths are a cart road's two lanes down to a footpath.

use serde::{Deserialize, Serialize};

use super::RoadClass;
use crate::stage::{Check, ParamError};

/// The steepest grade (rise over run) a road of each class may take in one step (spec ruling
/// 35). A class a recipe's table leaves out keeps its default.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MaxGrade {
    /// Main roads.
    pub main: f64,
    /// Roads.
    pub road: f64,
    /// Tracks.
    pub track: f64,
    /// Trails.
    pub trail: f64,
}

impl MaxGrade {
    /// The grade of `class`.
    #[must_use]
    pub fn of(&self, class: RoadClass) -> f64 {
        match class {
            RoadClass::Main => self.main,
            RoadClass::Road => self.road,
            RoadClass::Track => self.track,
            RoadClass::Trail => self.trail,
        }
    }
}

impl Default for MaxGrade {
    fn default() -> Self {
        Self {
            main: 0.08,
            road: 0.10,
            track: 0.15,
            trail: 0.25,
        }
    }
}

/// The width of a road of each class in metres. A class a recipe's table leaves out keeps its
/// default.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct WidthM {
    /// Main roads.
    pub main: f64,
    /// Roads.
    pub road: f64,
    /// Tracks.
    pub track: f64,
    /// Trails.
    pub trail: f64,
}

impl WidthM {
    /// The width of `class`.
    #[must_use]
    pub fn of(&self, class: RoadClass) -> f64 {
        match class {
            RoadClass::Main => self.main,
            RoadClass::Road => self.road,
            RoadClass::Track => self.track,
            RoadClass::Trail => self.trail,
        }
    }
}

impl Default for WidthM {
    fn default() -> Self {
        Self {
            main: 6.0,
            road: 4.5,
            track: 3.0,
            trail: 1.5,
        }
    }
}

/// The parameters of `roads`; see the module docs for the defaults' sources.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct RoadsParams {
    /// The grade at which the grade term of a step equals 6 times its length.
    pub comfort_grade: f64,
    /// The steepest step of each class.
    pub max_grade: MaxGrade,
    /// Cost per metre of step per unit of vegetation density on the cell stepped onto.
    pub forest_cost: f64,
    /// Cost of each stream cell a step crosses, m.
    pub ford_cost: f64,
    /// Cost per metre of bridge span.
    pub bridge_cost: f64,
    /// How much a crossing along the river's flow costs over a perpendicular one, as a share.
    pub angle_penalty: f64,
    /// Cost per metre the deck rises above the lower bank: a bridge pays
    /// `deck_cost·(deck_m − lower bank)`, which is `deck_cost·(|Δbank| + clearance_m)`. This is
    /// not the plan's `deck_cost·deck_m`: `deck_m` is a height above the base level, so that term
    /// would make a bridge dearer the higher its valley lies and change routes when the whole map
    /// is raised; the rise is the ramp a road climbs onto the deck. The `deck_m` property of a
    /// `bridge` site stays the spec's value, the higher bank plus `clearance_m`.
    pub deck_cost: f64,
    /// The longest bridge, bank cell to bank cell, m.
    pub max_bridge_span_m: f64,
    /// The deck's height above the higher bank, m.
    pub clearance_m: f64,
    /// A further link between villages or larger joins when the way through the network costs
    /// more than this many times its own cost.
    pub loop_detour: f64,
    /// The share of a step's cost on cells that already carry a road, so that tree links and
    /// trails merge onto existing roads. Loop links are routed without it.
    pub merge_discount: f64,
    /// The width of each class.
    pub width_m: WidthM,
}

impl Default for RoadsParams {
    fn default() -> Self {
        Self {
            comfort_grade: 0.08,
            max_grade: MaxGrade::default(),
            forest_cost: 0.8,
            ford_cost: 250.0,
            bridge_cost: 25.0,
            angle_penalty: 1.0,
            deck_cost: 12.5,
            max_bridge_span_m: 400.0,
            clearance_m: 3.0,
            loop_detour: 1.7,
            merge_discount: 0.35,
            width_m: WidthM::default(),
        }
    }
}

impl RoadsParams {
    /// Checks every value.
    ///
    /// # Errors
    /// [`ParamError`] of `stage` naming the first value out of range.
    pub(super) fn check(&self, stage: &str) -> Result<(), ParamError> {
        let check = Check::of(stage);
        check.positive("comfort_grade", self.comfort_grade)?;
        check.in_open_closed("merge_discount", self.merge_discount, 0.0, 1.0)?;
        for class in RoadClass::ALL {
            let grade = self.max_grade.of(class);
            check.positive(&format!("max_grade.{}", class.name()), grade)?;
        }
        for class in RoadClass::ALL {
            let width = self.width_m.of(class);
            check.positive(&format!("width_m.{}", class.name()), width)?;
        }
        let non_negatives = [
            ("forest_cost", self.forest_cost),
            ("ford_cost", self.ford_cost),
            ("bridge_cost", self.bridge_cost),
            ("angle_penalty", self.angle_penalty),
            ("deck_cost", self.deck_cost),
            ("max_bridge_span_m", self.max_bridge_span_m),
            ("clearance_m", self.clearance_m),
        ];
        for (name, value) in non_negatives {
            check.at_least(name, value, 0.0)?;
        }
        check.at_least("loop_detour", self.loop_detour, 1.0)
    }
}
