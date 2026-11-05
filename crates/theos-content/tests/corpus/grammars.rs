//! The body grammars over the `records/` corpus.

use std::error::Error;
use std::fs;

use theos_content::{AttributeValue, CalloutTitle, Note, NoteState, Record};

use crate::common::corpus;

fn voices_of(name: &str) -> Result<Vec<Option<CalloutTitle>>, Box<dyn Error>> {
    let path = corpus().join("records/lore/topics").join(name);
    let record = Record::parse(&fs::read_to_string(path)?)?;
    Ok(record
        .body()
        .callouts()
        .iter()
        .filter(|c| c.name == "voice")
        .map(|c| c.title.clone())
        .collect())
}

// AC: CE-10.2
#[test]
fn corpus_voice_titles_parse() -> Result<(), Box<dyn Error>> {
    for name in [
        "synthetic-voices-in-world.md",
        "synthetic-callouts-in-world.md",
    ] {
        let titles = voices_of(name)?;
        assert!(!titles.is_empty(), "{name} has voice blocks");
        for title in &titles {
            assert!(
                matches!(title, Some(CalloutTitle::Voice(_))),
                "{name}: {title:?}"
            );
        }
    }
    let Some(CalloutTitle::Voice(first)) = &voices_of("synthetic-voices-in-world.md")?[0] else {
        return Err("the first voice block does not parse".into());
    };
    assert_eq!(first.speaker.target, "hearth-folk");
    assert_eq!(first.beliefs.len(), 0);
    assert_eq!(first.tier, None);
    Ok(())
}

fn notes_of(path: &str) -> Result<Vec<Note>, Box<dyn Error>> {
    let record = Record::parse(&fs::read_to_string(corpus().join(path))?)?;
    record
        .body()
        .callouts()
        .iter()
        .filter(|c| c.name == "note")
        .map(|c| match &c.title {
            Some(CalloutTitle::Note(note)) => Ok(note.clone()),
            other => Err(format!("{path}: not a note title: {other:?}").into()),
        })
        .collect()
}

// AC: CE-10.2
#[test]
fn corpus_note_states_parse() -> Result<(), Box<dyn Error>> {
    let notes = notes_of("records/maps/synthetic-notes.md")?;
    let seen: Vec<_> = notes
        .iter()
        .map(|n| {
            (
                n.id.text.as_str(),
                n.version.as_ref().map(|v| v.target.as_str()),
                n.state.spelling(),
            )
        })
        .collect();
    let want = [
        ("n1", None, "open"),
        ("n2", Some("f04-terrain-v1"), "open"),
        ("n3", None, "resolved by [[f04-terrain-v1]]"),
        ("n4", None, "dropped"),
        ("n5", Some("f04-terrain-v1"), "dropped"),
    ];
    assert_eq!(seen.len(), want.len());
    for (got, want) in seen.iter().zip(want) {
        assert_eq!((got.0, got.1, got.2.as_str()), want);
    }
    assert!(matches!(&notes[2].state, NoteState::ResolvedBy(l) if l.target == "f04-terrain-v1"));
    assert_eq!(notes[0].text, "A note that is still open.");
    assert!(!notes[0].text.contains("Jann"));

    let concept = notes_of("records/concepts/synthetic-concept-notes.md")?;
    assert_eq!(concept[3].id.text, "n4");
    assert_eq!(concept[3].state, NoteState::Unknown("closed".to_owned()));
    Ok(())
}

fn links_of(value: &AttributeValue) -> Vec<&theos_content::Link> {
    match value {
        AttributeValue::Links(links) => links.iter().collect(),
        _ => Vec::new(),
    }
}

// AC: CE-10.1
#[test]
fn s004_parses_into_its_beats() -> Result<(), Box<dyn Error>> {
    let text = fs::read_to_string(corpus().join("records/story/scripts/S-004.md"))?;
    let record = Record::parse(&text)?;
    let beats = record.body().beats();
    assert_eq!(beats.len(), 2);
    let want = [
        (
            1,
            "Arrival",
            vec![
                ("at", vec!["fates-shrine"]),
                ("with", vec!["mother-ilse"]),
                ("says", vec!["B-016"]),
                ("when", vec![]),
            ],
        ),
        (
            2,
            "The murmur",
            vec![
                ("at", vec!["fates-shrine"]),
                ("reveals", vec!["C-031"]),
                ("when", vec![]),
            ],
        ),
    ];
    for (beat, (number, name, attributes)) in beats.iter().zip(want) {
        assert_eq!((beat.number, beat.name.as_str()), (number, name));
        assert_eq!(beat.attributes.len(), attributes.len());
        for (attribute, (key, targets)) in beat.attributes.iter().zip(attributes) {
            assert_eq!(attribute.key, key);
            let links = links_of(&attribute.value);
            assert_eq!(links.len(), targets.len(), "{key}");
            for (link, target) in links.iter().zip(targets) {
                assert_eq!(link.target_span.get(&text), Some(target));
                assert_eq!(link.target, target);
            }
        }
        let when = beat.attributes.last().map(|a| &a.value);
        assert!(matches!(when, Some(AttributeValue::Token(t)) if t.text == "T1"));
    }
    Ok(())
}
