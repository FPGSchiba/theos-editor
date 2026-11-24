//! The parameters of `sites`, their defaults and their checks.

use serde::{Deserialize, Serialize};

use crate::stage::{Check, ParamError};

/// How many of each scattered site; a count a recipe leaves out keeps its default (0.3.0 merged
/// the table into its defaults).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Counts {
    /// Watchtowers over the roads.
    pub watchtower: usize,
    /// Caves in limestone cliffs.
    pub cave: usize,
    /// Hostile camps away from settlements and roads.
    pub camp: usize,
}

impl Default for Counts {
    fn default() -> Self {
        Self {
            watchtower: 3,
            cave: 2,
            camp: 3,
        }
    }
}

/// The parameters of `sites`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct SitesParams {
    /// How many watchtowers, caves and camps.
    pub counts: Counts,
    /// A mill needs this many times `river_q` of discharge (spec ruling 45: 2.7, which is
    /// 0.3.0's 0.4 m³/s at its `river_q` of 0.15).
    pub mill_q_factor: f64,
    /// The margin kept free, a share of the shorter side (spec ruling 46, 0.3.0's 0.6 km at
    /// 24 km).
    pub edge_margin: f64,
    /// The least richness of a deposit that gets a site (0.3.0's `min_richness`).
    pub min_richness: f64,
}

impl Default for SitesParams {
    fn default() -> Self {
        Self {
            counts: Counts::default(),
            mill_q_factor: 2.7,
            edge_margin: 0.025,
            min_richness: 0.45,
        }
    }
}

impl SitesParams {
    /// Checks every value.
    ///
    /// # Errors
    /// [`ParamError`] of `stage` naming the first value out of range.
    pub(super) fn check(&self, stage: &str) -> Result<(), ParamError> {
        let check = Check::of(stage);
        check.positive("mill_q_factor", self.mill_q_factor)?;
        check.in_closed_open("edge_margin", self.edge_margin, 0.0, 0.5)?;
        check.in_closed("min_richness", self.min_richness, 0.0, 1.0)
    }
}
