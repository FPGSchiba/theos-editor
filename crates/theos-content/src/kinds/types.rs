//! Field types, link targets and the lens entries.

use std::fmt;

/// A narrowing of a target kind: the target's `field` must equal `value`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Narrow {
    /// The field of the target record.
    pub field: &'static str,
    /// The value it must have.
    pub value: &'static str,
}

/// A kind a link may point at, optionally narrowed by a field value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Target {
    /// The kind name.
    pub kind: &'static str,
    /// The narrowing, if any.
    pub narrow: Option<Narrow>,
}

impl Target {
    /// Any record of `kind`.
    #[must_use]
    pub const fn kind(kind: &'static str) -> Target {
        Target { kind, narrow: None }
    }

    /// Records of `kind` whose `field` equals `value`.
    #[must_use]
    pub const fn narrowed(kind: &'static str, field: &'static str, value: &'static str) -> Target {
        Target {
            kind,
            narrow: Some(Narrow { field, value }),
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.narrow {
            None => write!(f, "{}", self.kind),
            Some(n) => write!(f, "{}({}={})", self.kind, n.field, n.value),
        }
    }
}

/// The type of a front-matter field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldType {
    /// The record id.
    Id,
    /// The kind name.
    KindName,
    /// A text.
    Text,
    /// A list of texts.
    TextList,
    /// One of the listed values.
    Enum(&'static [&'static str]),
    /// An integer in `min..=max`.
    Integer {
        /// Smallest value.
        min: i64,
        /// Largest value.
        max: i64,
    },
    /// A floor range.
    FloorRange,
    /// One link to one of the targets.
    Link(&'static [Target]),
    /// A list of links to the targets.
    Links(&'static [Target]),
    /// A URL.
    Url,
    /// The boolean `true`.
    True,
    /// A generator, `name@version`.
    Generator,
    /// Note ids.
    NoteIds,
    /// References to binary files.
    BinaryRefs,
    /// A map with open keys.
    OpenMap,
    /// A lens map.
    Lens,
}

fn write_targets(f: &mut fmt::Formatter<'_>, list: &[Target]) -> fmt::Result {
    for (i, target) in list.iter().enumerate() {
        if i > 0 {
            f.write_str("|")?;
        }
        write!(f, "{target}")?;
    }
    Ok(())
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldType::Id => f.write_str("id"),
            FieldType::KindName => f.write_str("kind"),
            FieldType::Text => f.write_str("text"),
            FieldType::TextList => f.write_str("text list"),
            FieldType::Enum(values) => write!(f, "one of {}", values.join("|")),
            FieldType::Integer { min, max } if *min == i64::MIN && *max == i64::MAX => {
                f.write_str("integer")
            }
            FieldType::Integer { min, max } => write!(f, "integer {min}..={max}"),
            FieldType::FloorRange => f.write_str("floor range"),
            FieldType::Link(list) => {
                f.write_str("link → ")?;
                write_targets(f, list)
            }
            FieldType::Links(list) => {
                f.write_str("links → ")?;
                write_targets(f, list)
            }
            FieldType::Url => f.write_str("url"),
            FieldType::True => f.write_str("true"),
            FieldType::Generator => f.write_str("name@version"),
            FieldType::NoteIds => f.write_str("note ids"),
            FieldType::BinaryRefs => f.write_str("binary refs"),
            FieldType::OpenMap => f.write_str("map"),
            FieldType::Lens => f.write_str("lens"),
        }
    }
}

/// One entry of a lens map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LensEntry {
    /// The entry key.
    pub key: &'static str,
    /// Its type.
    pub ty: FieldType,
}

/// The tiers a lens may name.
pub const TIERS: &[&str] = &["T1", "T2", "T3"];

/// The entries of a lens map, in writing order.
pub const LENS_ENTRIES: &[LensEntry] = &[
    LensEntry {
        key: "thinks_from",
        ty: FieldType::Text,
    },
    LensEntry {
        key: "stance",
        ty: FieldType::Text,
    },
    LensEntry {
        key: "tier",
        ty: FieldType::Enum(TIERS),
    },
    LensEntry {
        key: "unaware_of",
        ty: FieldType::Links(&[Target::kind("truth")]),
    },
    LensEntry {
        key: "rejects",
        ty: FieldType::Links(&[Target::kind("belief")]),
    },
    LensEntry {
        key: "speaks_to",
        ty: FieldType::Text,
    },
];

/// What a voice callout's speaker may point at.
pub const VOICE_SPEAKER: &[Target] = &[
    Target::kind("npc"),
    Target::kind("faction"),
    Target::kind("people"),
];

/// What a voice callout's belief may point at.
pub const VOICE_BELIEF: &[Target] = &[Target::kind("belief")];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_display_as_the_notation() {
        const BAND: &[Target] = &[Target::kind("band")];
        const BAND_ETC: &[Target] = &[
            Target::kind("band"),
            Target::kind("floor"),
            Target::kind("location"),
        ];
        assert_eq!(FieldType::Id.to_string(), "id");
        assert_eq!(FieldType::KindName.to_string(), "kind");
        assert_eq!(FieldType::Text.to_string(), "text");
        assert_eq!(FieldType::TextList.to_string(), "text list");
        assert_eq!(
            FieldType::Enum(&["a", "b", "c"]).to_string(),
            "one of a|b|c"
        );
        assert_eq!(
            FieldType::Integer { min: 1, max: 100 }.to_string(),
            "integer 1..=100"
        );
        assert_eq!(
            FieldType::Integer {
                min: i64::MIN,
                max: i64::MAX
            }
            .to_string(),
            "integer"
        );
        assert_eq!(FieldType::FloorRange.to_string(), "floor range");
        assert_eq!(FieldType::Link(BAND).to_string(), "link → band");
        assert_eq!(
            FieldType::Links(BAND_ETC).to_string(),
            "links → band|floor|location"
        );
        assert_eq!(FieldType::Url.to_string(), "url");
        assert_eq!(FieldType::True.to_string(), "true");
        assert_eq!(FieldType::Generator.to_string(), "name@version");
        assert_eq!(FieldType::NoteIds.to_string(), "note ids");
        assert_eq!(FieldType::BinaryRefs.to_string(), "binary refs");
        assert_eq!(FieldType::OpenMap.to_string(), "map");
        assert_eq!(FieldType::Lens.to_string(), "lens");
        assert_eq!(Target::kind("topic").to_string(), "topic");
        assert_eq!(
            Target::narrowed("topic", "track", "explaining").to_string(),
            "topic(track=explaining)"
        );
    }

    #[test]
    fn lens_entries_and_voice_targets_are_as_specified() {
        let keys: Vec<_> = LENS_ENTRIES.iter().map(|e| e.key).collect();
        assert_eq!(
            keys,
            [
                "thinks_from",
                "stance",
                "tier",
                "unaware_of",
                "rejects",
                "speaks_to"
            ]
        );
        assert_eq!(VOICE_SPEAKER.len(), 3);
        assert_eq!(VOICE_BELIEF, &[Target::kind("belief")]);
    }
}
