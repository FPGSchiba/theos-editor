//! What every kind shares: the fields of data model §5, the slug id rule and common targets.

use crate::kinds::{FieldSpec, FieldType, IdRule, Target};

/// The id rule of every kind without a rule of its own.
pub(crate) const SLUG: IdRule = IdRule {
    pattern: "^[a-z0-9]+(-[a-z0-9]+)*$",
    says: "a slug",
};

/// Places: a band, a floor or a location.
pub(crate) const PLACES: &[Target] = &[
    Target::kind("band"),
    Target::kind("floor"),
    Target::kind("location"),
];

/// Holders of lenses and beliefs: a people, a faction or an npc.
pub(crate) const HOLDERS: &[Target] = &[
    Target::kind("people"),
    Target::kind("faction"),
    Target::kind("npc"),
];

/// A field spec in one line.
pub(crate) const fn field(key: &'static str, ty: FieldType, required: bool) -> FieldSpec {
    FieldSpec { key, ty, required }
}

/// The fields every record has (data model §5), then `own`.
pub(crate) fn fields(status_required: bool, own: &[FieldSpec]) -> Vec<FieldSpec> {
    let mut all = vec![
        field("id", FieldType::Id, true),
        field("kind", FieldType::KindName, true),
        field("title", FieldType::Text, true),
        field(
            "status",
            FieldType::Enum(&["draft", "canon"]),
            status_required,
        ),
        field("tags", FieldType::TextList, false),
    ];
    all.extend_from_slice(own);
    all
}
