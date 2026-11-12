//! The guardrail `lore/inworld-vocab` (spec §5.11) over the `lore/` fixture project: its
//! `.theos/settings.toml` has the project word `ledger` and an allowed `T1`.

use std::collections::BTreeSet;

use theos_content::{Change, Finding, Index, Severity, parts};

use crate::common::{Key, TempProject, expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const RULE: &str = "lore/inworld-vocab";
const SETTINGS: &str = ".theos/settings.toml";
const DIALOGUE: &str = "dialogue/synthetic-vocab-dialogue.md";
const VOICE_TOPIC: &str = "lore/topics/synthetic-vocab-voice-in-world.md";

/// Records of the fixture that hold no developer word in in-world text.
const PASSING: [&str; 6] = [
    "people/synthetic-folk.md",
    "lore/topics/synthetic-vocab-ok-explaining.md",
    "lore/topics/synthetic-vocab-ok-in-world.md",
    "lore/topics/synthetic-vocab-voice-explaining.md",
    "lore/beliefs/B-950.md",
    "story/scripts/S-950.md",
];

fn open(project: &TempProject) -> Result<Index, Box<dyn std::error::Error>> {
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    Ok(index)
}

fn vocab(index: &Index) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    Ok(index
        .findings()?
        .into_iter()
        .filter(|finding| finding.rule == RULE)
        .collect())
}

/// The `(path, line, column)` of the findings.
fn places(findings: &[Finding]) -> Vec<(String, usize, usize)> {
    keys(findings)
        .into_iter()
        .map(|(path, _, _, line, column)| (path, line, column))
        .collect()
}

fn at(path: &str, line: usize, column: usize) -> (String, usize, usize) {
    (path.to_owned(), line, column)
}

fn wanted() -> Result<Vec<Key>, Box<dyn std::error::Error>> {
    let all = expected(&fixture("lore").join("expected.toml"))?;
    Ok(all.into_iter().filter(|key| key.1 == RULE).collect())
}

// AC: CE-15.1
#[test]
fn inworld_vocab_passes_and_fails() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let found = vocab(&open(&project)?)?;
    assert_eq!(keys(&found), wanted()?);
    for finding in &found {
        assert!(
            !PASSING.contains(&finding.path.as_str()),
            "{finding} sits on a passing record"
        );
        assert_eq!(finding.severity, Severity::Error);
    }
    // A voice line, a caption, a project word, and a rule name in a belief title.
    let said: Vec<(&str, &str)> = found
        .iter()
        .map(|finding| (finding.path.as_str(), finding.message.as_str()))
        .filter(|(path, _)| path.starts_with("lore/") || path.starts_with("dialogue/"))
        .map(|(path, message)| (path, message.split(' ').next().unwrap_or_default()))
        .collect();
    assert_eq!(
        said,
        [
            (DIALOGUE, "`Ledger`"),
            (DIALOGUE, "`T3`"),
            ("lore/beliefs/B-951.md", "`three-clue`"),
            (VOICE_TOPIC, "`reveal-order`"),
            (VOICE_TOPIC, "`T-950`"),
            (VOICE_TOPIC, "`ledger`"),
            (VOICE_TOPIC, "`T2`"),
        ]
    );
    assert_eq!(
        found[0].message,
        "`Ledger` is a developer word, which in-world text cannot use. \
         Rephrase it in the speaker's words, or add it to `allow` with a reason."
    );
    // `T1` is exempt by an entry with a reason; the heading, the design notes and the voice
    // titles are no in-world text, and the voice block of the in-world topic gives its findings
    // once: the list above is all there is.
    Ok(())
}

// AC: CE-15.2
#[test]
fn a_word_with_a_reasonless_entry_stays_a_developer_word() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    assert!(!places(&vocab(&open(&project)?)?).contains(&at(DIALOGUE, 11, 31)));

    // The same entry without a reason: the word is flagged again, and the entry is an error.
    project.write(
        SETTINGS,
        "[validate.inworld-vocab]\nwords = [\"ledger\"]\nallow = [{ word = \"T1\" }]\n",
    )?;
    let index = open(&project)?;
    assert!(places(&vocab(&index)?).contains(&at(DIALOGUE, 11, 31)));
    let reasons: Vec<String> = index
        .findings()?
        .into_iter()
        .filter(|finding| finding.rule == "settings/allow-reason")
        .map(|finding| finding.to_string())
        .collect();
    assert_eq!(reasons.len(), 1, "{reasons:?}");
    // The `T1` of a voice title is not in-world text, with or without the entry.
    assert!(
        places(&vocab(&index)?)
            .iter()
            .all(|(path, line, _)| !(path == VOICE_TOPIC && *line == 12))
    );
    Ok(())
}

#[test]
fn an_occurrence_is_reported_once() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    // `T-950` is a built-in word and a project word now: one finding at each occurrence.
    project.write(SETTINGS, "[validate.inworld-vocab]\nwords = [\"t-950\"]\n")?;
    project.write(
        "dialogue/synthetic-twice.md",
        "---\n# synthetic fixture (P04), not canon\nid: synthetic-twice\nkind: dialogue\n\
         title: Synthetic twice\nspeaker: synthetic-folk\n---\nT-950 and T-950 again.\n",
    )?;
    let found = vocab(&open(&project)?)?;
    let twice: Vec<(usize, usize)> = places(&found)
        .into_iter()
        .filter(|(path, _, _)| path == "dialogue/synthetic-twice.md")
        .map(|(_, line, column)| (line, column))
        .collect();
    assert_eq!(twice, [(8, 1), (8, 11)]);
    let voice: Vec<(usize, usize)> = places(&found)
        .into_iter()
        .filter(|(path, line, _)| path == VOICE_TOPIC && *line == 14)
        .map(|(_, line, column)| (line, column))
        .collect();
    assert_eq!(
        voice,
        [(14, 12)],
        "the project word `ledger` is not a word now"
    );
    Ok(())
}

#[test]
fn columns_count_unicode_scalar_values() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    project.write(
        "dialogue/synthetic-unicode.md",
        "---\n# synthetic fixture (P04), not canon\nid: synthetic-unicode\nkind: dialogue\n\
         title: Synthetic unicode\nspeaker: synthetic-folk\n---\n\
         Äöü 🙂 T-950 here.\nnaïve 🙂🙂 and T2.\n",
    )?;
    let found = vocab(&open(&project)?)?;
    let unicode: Vec<(usize, usize)> = places(&found)
        .into_iter()
        .filter(|(path, _, _)| path == "dialogue/synthetic-unicode.md")
        .map(|(_, line, column)| (line, column))
        .collect();
    // `Äöü 🙂 ` is six scalar values (not eleven bytes or seven UTF-16 units); `naïve 🙂🙂 and `
    // is thirteen.
    assert_eq!(unicode, [(8, 7), (9, 14)]);
    // The span is in bytes of the record.
    let spans: Vec<(usize, usize)> = found
        .iter()
        .filter(|finding| finding.path == "dialogue/synthetic-unicode.md")
        .filter_map(|finding| finding.span.map(|span| (span.start, span.end)))
        .collect();
    let text = std::fs::read_to_string(project.root().join("dialogue/synthetic-unicode.md"))?;
    let words: Vec<&str> = spans.iter().map(|(s, e)| &text[*s..*e]).collect();
    assert_eq!(words, ["T-950", "T2"]);
    Ok(())
}

#[test]
fn a_broken_settings_file_keeps_the_built_in_words() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let mut index = open(&project)?;
    assert!(
        places(&vocab(&index)?).contains(&at(DIALOGUE, 11, 10)),
        "`Ledger`"
    );

    project.write(SETTINGS, "[validate.inworld-vocab\nwords = [\"ledger\"]\n")?;
    index.sync()?;
    let found = places(&vocab(&index)?);
    // The project word and the allowlist are gone: `Ledger` is no word, `T1` is a word again,
    // and every built-in word is still found.
    assert!(!found.contains(&at(DIALOGUE, 11, 10)), "{found:?}");
    assert!(found.contains(&at(DIALOGUE, 11, 31)), "T1: {found:?}");
    assert!(found.contains(&at(DIALOGUE, 11, 55)), "T3: {found:?}");
    assert!(found.contains(&at(VOICE_TOPIC, 14, 12)), "T-950: {found:?}");
    assert!(
        !found.contains(&at(VOICE_TOPIC, 14, 41)),
        "ledger: {found:?}"
    );
    assert_eq!(open(&project)?.dump()?, index.dump()?);
    Ok(())
}

// AC: CE-14.3
#[test]
fn a_new_word_revalidates_the_records_with_in_world_text() -> TestResult {
    let project = TempProject::copy_of("lore")?;
    let mut index = open(&project)?;
    let before = vocab(&index)?.len();

    // The records with in-world segments, from the dump of the segments.
    let dump = index.dump()?;
    let with_text: BTreeSet<String> = dump
        .lines()
        .skip_while(|line| *line != "[inworld]")
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .filter_map(|line| line.split('\t').next())
        .map(str::to_owned)
        .collect();
    assert!(with_text.contains(DIALOGUE) && with_text.contains(VOICE_TOPIC));
    assert!(!with_text.contains("people/synthetic-folk.md"));

    let settings = std::fs::read_to_string(project.root().join(SETTINGS))?;
    let update = index.apply(&[Change::text(
        SETTINGS,
        settings.replace("[\"ledger\"]", "[\"ledger\", \"hum\"]"),
    )])?;
    assert!(
        update.reparsed.iter().all(|path| path == SETTINGS),
        "{update:?}"
    );
    // The settings file is checked again itself; besides it exactly the records with in-world
    // text are, and no other record.
    let mut revalidated: BTreeSet<String> = update.revalidated.iter().cloned().collect();
    assert!(revalidated.remove(SETTINGS), "{update:?}");
    assert_eq!(revalidated, with_text, "{update:?}");
    // The voice block of the in-world topic holds the new word "hum" ("hums" is no match).
    assert!(vocab(&index)?.len() > before);
    Ok(())
}
