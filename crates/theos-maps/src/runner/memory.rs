//! The memory estimate of a plan, checked before any stage runs (plan ruling L7).

use std::collections::BTreeMap;

use crate::recipe::Plan;
use crate::world::bytes_per_cell;

/// The memory a run is allowed when the options name no budget: 3 GiB.
pub const DEFAULT_MEMORY_BUDGET: u64 = 3 * 1024 * 1024 * 1024;

/// What a plan's layers are expected to take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEstimate {
    /// The estimate in bytes.
    pub bytes: u64,
    /// The three largest layers with their sizes, largest first, ties by name.
    pub largest: Vec<(String, u64)>,
}

/// Every distinct layer name of every stage's reads and writes counts once at
/// `cells x bytes_per_cell`; the sum is raised by half for working copies.
#[must_use]
pub fn estimate(plan: &Plan) -> MemoryEstimate {
    let cells = u64::try_from(plan.frame.cells()).unwrap_or(u64::MAX);
    let mut sizes: BTreeMap<&str, u64> = BTreeMap::new();
    for stage in &plan.stages {
        for spec in stage.reads.iter().chain(&stage.writes) {
            let size = cells.saturating_mul(bytes_per_cell(spec.kind));
            let entry = sizes.entry(spec.name).or_insert(0);
            *entry = (*entry).max(size);
        }
    }
    let total = sizes.values().fold(0_u64, |sum, &s| sum.saturating_add(s));
    let mut largest: Vec<(String, u64)> = sizes
        .into_iter()
        .map(|(name, size)| (name.to_owned(), size))
        .collect();
    largest.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    largest.truncate(3);
    MemoryEstimate {
        bytes: total.saturating_mul(3) / 2,
        largest,
    }
}

/// The largest layers as text for an error message.
pub(super) fn describe(largest: &[(String, u64)]) -> String {
    largest
        .iter()
        .map(|(name, size)| format!("{name} ({size} bytes)"))
        .collect::<Vec<_>>()
        .join(", ")
}
