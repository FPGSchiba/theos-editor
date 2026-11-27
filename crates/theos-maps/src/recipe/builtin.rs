//! The recipes embedded in the crate: `(file name, text)` pairs, in the order of the catalog.

pub(crate) const BUILTIN: &[(&str, &str)] = &[
    (
        "fixture-small.toml",
        include_str!("../../recipes/fixture-small.toml"),
    ),
    (
        "hearth-valley.toml",
        include_str!("../../recipes/hearth-valley.toml"),
    ),
    (
        "hearth-valley-omega.toml",
        include_str!("../../recipes/hearth-valley-omega.toml"),
    ),
];
