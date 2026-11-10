//! The dump holds notes, in-world segments and `when::` tokens.

use theos_content::{Index, Project, parts};

use crate::common::TempProject;

#[test]
fn the_dump_holds_notes_inworld_and_when() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempProject::new()?;
    temp.write(
        "concepts/synthetic-concept.md",
        "---\n# synthetic fixture (P04), not canon\nid: synthetic-concept\nkind: concept\n\
         title: synthetic concept\n---\n> [!note] n1 · [[synthetic-concept]] · open\n> text\n",
    )?;
    temp.write(
        "story/scripts/S-950.md",
        "---\n# synthetic fixture (P04), not canon\nid: S-950\nkind: script\n\
         title: synthetic script\n---\n## Beat 1 — Start\nwhen:: T2\n\n\
         > [!voice] [[synthetic-speaker]] · T1\n> Spoken words.\n",
    )?;
    let project = Project::open(temp.root())?;
    let mut index = Index::in_memory(&project, &parts::builtin()?)?;
    index.sync()?;
    let dump = index.dump()?;
    let section = |name: &str| -> Vec<String> {
        dump.lines()
            .skip_while(|line| *line != format!("[{name}]"))
            .skip(1)
            .take_while(|line| !line.starts_with('['))
            .map(str::to_owned)
            .collect()
    };
    let notes = section("notes");
    assert_eq!(notes.len(), 1);
    assert!(
        notes[0]
            .starts_with("concepts/synthetic-concept.md\tn1\t1\topen\t\\N\tsynthetic-concept\t")
    );
    let inworld = section("inworld");
    assert_eq!(inworld.len(), 1);
    assert!(inworld[0].contains("Spoken words."));
    let beats = section("beats");
    assert_eq!(beats.len(), 1);
    assert!(beats[0].ends_with("\tT2"));
    Ok(())
}
