//! The body grammars over the `records/` corpus.

use std::error::Error;
use std::fs;

use theos_content::{CalloutTitle, Record};

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
