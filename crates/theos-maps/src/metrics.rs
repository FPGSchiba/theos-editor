//! The summary paths that exist whatever stages a recipe holds (spec §5.8).

/// The summary paths a constraint may name besides `features.<type>` and `stages.<id>.<metric>`.
/// `travel.chime_to_chime_min` counts only when a `travel` stage is planned.
pub const PATHS: &[&str] = &[
    "elevation_m.relief",
    "lake_pct",
    "rivers_pct",
    "vegetation_cover_pct",
    "precip_mm_yr.median",
    "travel.chime_to_chime_min",
];
