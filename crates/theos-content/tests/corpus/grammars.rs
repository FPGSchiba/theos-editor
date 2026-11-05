//! The body grammars over the `records/` corpus.

use std::error::Error;
use std::fs;

use theos_content::{CalloutTitle, Note, NoteState, Record};

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
