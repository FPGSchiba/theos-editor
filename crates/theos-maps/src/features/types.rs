//! The feature types of terrain maps, as `(kind, type)`.

/// Every feature type a terrain map may hold, with its kind.
pub const TYPES: &[(&str, &str)] = &[
    ("settlement", "city"),
    ("settlement", "town"),
    ("settlement", "village"),
    ("settlement", "hamlet"),
    ("poi", "ruin"),
    ("poi", "chime"),
    ("poi", "gate-up"),
    ("poi", "gate-down"),
    ("site", "mill"),
    ("site", "mine"),
    ("site", "quarry"),
    ("site", "lumber-camp"),
    ("site", "shrine"),
    ("site", "watchtower"),
    ("site", "inn"),
    ("site", "cave"),
    ("site", "camp"),
    ("site", "bridge"),
    ("site", "ford"),
    ("resource", "iron-ore"),
    ("resource", "copper-ore"),
    ("resource", "silver-ore"),
    ("resource", "stone"),
    ("resource", "clay"),
    ("resource", "timber"),
    ("resource", "peat-reeds"),
    ("resource", "salt"),
    ("resource", "resonance-crystal"),
    ("resource", "herbs"),
    ("road", "trail"),
    ("road", "track"),
    ("road", "road"),
    ("road", "main"),
    ("water", "river"),
    ("water", "lake"),
];

/// The types of `kind`, in the order of [`TYPES`].
pub fn types_of(kind: &str) -> impl Iterator<Item = &'static str> + '_ {
    TYPES
        .iter()
        .filter(move |(k, _)| *k == kind)
        .map(|(_, ty)| *ty)
}
