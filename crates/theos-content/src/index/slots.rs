//! Slots: the place of the front matter or the body a link sits in.

use std::fmt;

use crate::kinds::{FieldType, KindSpec, LENS_ENTRIES, Target, VOICE_BELIEF, VOICE_SPEAKER};

/// The place a link sits in (spec §5.6). The place decides which kinds the link may point at.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Slot {
    /// A `Link` or `Links` field of the front matter, by key.
    Field(String),
    /// A link entry of a lens (`unaware_of`, `rejects`), by key.
    LensEntry(String),
    /// A beat attribute of a script (`at::`, `with::`, …), by key.
    Beat(String),
    /// The speaker of a voice block.
    VoiceSpeaker,
    /// A belief of a voice block.
    VoiceBelief,
    /// The version link in a note title.
    NoteVersion,
    /// The link of `resolved by [[id]]` in a note title.
    NoteResolvedBy,
    /// Any other wikilink of a body; it may point at any kind.
    Mention,
}

impl Slot {
    /// The text the store keeps: `field:<key>`, `lens:<key>`, `beat:<key>`, `voice:speaker`,
    /// `voice:belief`, `note:version`, `note:resolved-by` or `mention`.
    #[must_use]
    pub fn as_db(&self) -> String {
        match self {
            Slot::Field(key) => format!("field:{key}"),
            Slot::LensEntry(key) => format!("lens:{key}"),
            Slot::Beat(key) => format!("beat:{key}"),
            Slot::VoiceSpeaker => "voice:speaker".to_owned(),
            Slot::VoiceBelief => "voice:belief".to_owned(),
            Slot::NoteVersion => "note:version".to_owned(),
            Slot::NoteResolvedBy => "note:resolved-by".to_owned(),
            Slot::Mention => "mention".to_owned(),
        }
    }

    /// Reads a text written by [`Slot::as_db`]; `None` for any other text.
    #[must_use]
    pub fn from_db(text: &str) -> Option<Slot> {
        match text {
            "voice:speaker" => Some(Slot::VoiceSpeaker),
            "voice:belief" => Some(Slot::VoiceBelief),
            "note:version" => Some(Slot::NoteVersion),
            "note:resolved-by" => Some(Slot::NoteResolvedBy),
            "mention" => Some(Slot::Mention),
            _ => {
                let (kind, key) = text.split_once(':')?;
                let key = key.to_owned();
                match kind {
                    "field" => Some(Slot::Field(key)),
                    "lens" => Some(Slot::LensEntry(key)),
                    "beat" => Some(Slot::Beat(key)),
                    _ => None,
                }
            }
        }
    }

    /// The kinds a link in this slot may point at, given the spec of the record it sits in.
    ///
    /// `None` means no restriction (a mention), or that the spec does not define the slot: a
    /// field the kind does not have, a beat key or notes the kind does not use, or no spec.
    #[must_use]
    pub fn targets(&self, spec: Option<&KindSpec>) -> Option<&'static [Target]> {
        match self {
            Slot::Field(key) => match spec?.field(key)?.ty {
                FieldType::Link(targets) | FieldType::Links(targets) => Some(targets),
                _ => None,
            },
            Slot::LensEntry(key) => match LENS_ENTRIES.iter().find(|entry| entry.key == key)?.ty {
                FieldType::Link(targets) | FieldType::Links(targets) => Some(targets),
                _ => None,
            },
            Slot::Beat(key) => spec?
                .body
                .beats?
                .iter()
                .find(|attribute| attribute.key == key)
                .map(|attribute| attribute.targets),
            Slot::VoiceSpeaker => Some(VOICE_SPEAKER),
            Slot::VoiceBelief => Some(VOICE_BELIEF),
            Slot::NoteVersion | Slot::NoteResolvedBy => spec?.body.notes,
            Slot::Mention => None,
        }
    }
}

impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Slot::Field(key) => write!(f, "`{key}`"),
            Slot::LensEntry(key) => write!(f, "`lens.{key}`"),
            Slot::Beat(key) => write!(f, "the `{key}::` link"),
            Slot::VoiceSpeaker => f.write_str("a voice speaker"),
            Slot::VoiceBelief => f.write_str("a voice belief"),
            Slot::NoteVersion => f.write_str("a note version"),
            Slot::NoteResolvedBy => f.write_str("a note's resolved-by link"),
            Slot::Mention => f.write_str("a mention"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parts::builtin;

    fn every_slot() -> Vec<Slot> {
        vec![
            Slot::Field("home".to_owned()),
            Slot::LensEntry("unaware_of".to_owned()),
            Slot::Beat("with".to_owned()),
            Slot::VoiceSpeaker,
            Slot::VoiceBelief,
            Slot::NoteVersion,
            Slot::NoteResolvedBy,
            Slot::Mention,
        ]
    }

    #[test]
    fn slots_round_trip_through_the_store_text() {
        let written: Vec<String> = every_slot().iter().map(Slot::as_db).collect();
        assert_eq!(
            written,
            [
                "field:home",
                "lens:unaware_of",
                "beat:with",
                "voice:speaker",
                "voice:belief",
                "note:version",
                "note:resolved-by",
                "mention"
            ]
        );
        for slot in every_slot() {
            assert_eq!(Slot::from_db(&slot.as_db()), Some(slot));
        }
        for bad in [
            "",
            "field",
            "fields:x",
            "voice:other",
            "mention:x",
            "Mention",
        ] {
            assert_eq!(Slot::from_db(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn slots_display_as_the_place_they_name() {
        let shown: Vec<String> = every_slot().iter().map(ToString::to_string).collect();
        assert_eq!(
            shown,
            [
                "`home`",
                "`lens.unaware_of`",
                "the `with::` link",
                "a voice speaker",
                "a voice belief",
                "a note version",
                "a note's resolved-by link",
                "a mention"
            ]
        );
    }

    #[test]
    fn targets_come_from_the_spec_of_the_slot() -> Result<(), Box<dyn std::error::Error>> {
        let registry = builtin()?;
        let npc = registry.kind("npc");
        let script = registry.kind("script");
        let concept = registry.kind("concept");
        let kinds = |slot: &Slot, spec| -> Option<Vec<&str>> {
            slot.targets(spec)
                .map(|list| list.iter().map(|target| target.kind).collect())
        };
        let field = |key: &str| Slot::Field(key.to_owned());
        assert_eq!(kinds(&field("home"), npc), Some(vec!["location", "floor"]));
        assert_eq!(kinds(&field("title"), npc), None);
        assert_eq!(kinds(&field("nope"), npc), None);
        assert_eq!(kinds(&field("home"), None), None);
        assert_eq!(
            kinds(&Slot::LensEntry("rejects".to_owned()), None),
            Some(vec!["belief"])
        );
        assert_eq!(kinds(&Slot::LensEntry("tier".to_owned()), None), None);
        assert_eq!(
            kinds(&Slot::Beat("reveals".to_owned()), script),
            Some(vec!["clue"])
        );
        assert_eq!(kinds(&Slot::Beat("reveals".to_owned()), npc), None);
        assert_eq!(
            Slot::VoiceSpeaker.targets(None),
            Some(VOICE_SPEAKER),
            "a voice speaker needs no spec"
        );
        assert_eq!(Slot::VoiceBelief.targets(npc), Some(VOICE_BELIEF));
        assert_eq!(
            kinds(&Slot::NoteVersion, concept),
            Some(vec!["concept-version"])
        );
        assert_eq!(
            kinds(&Slot::NoteResolvedBy, concept),
            Some(vec!["concept-version"])
        );
        assert_eq!(kinds(&Slot::NoteVersion, npc), None);
        assert_eq!(kinds(&Slot::Mention, concept), None);
        Ok(())
    }
}
