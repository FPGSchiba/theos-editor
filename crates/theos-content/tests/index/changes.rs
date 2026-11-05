//! Relinking only what a change touches: each change updates its record and its backlinks.

use theos_content::{Index, Update, parts};

use crate::common::TempProject;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const READER: &str = "things/synthetic-reader.md";
const SPEAKER: &str = "people/synthetic-folk/synthetic-guild/synthetic-speaker.md";
const SCRIPT: &str = "story/scripts/S-940.md";
const BELIEF: &str = "lore/beliefs/B-940.md";
const BELIEF_RENAMED: &str = "lore/beliefs/B-941.md";
const IN_WORLD: &str = "lore/topics/synthetic-subject-in-world.md";
const TRUTH: &str = "lore/truths/T-940.md";
const CLUE: &str = "lore/clues/C-940.md";

const MARK: &str = "# synthetic fixture (P03), not canon\n";

fn reader(extra: &str) -> String {
    format!(
        "---\n{MARK}id: synthetic-reader\nkind: thing\ntitle: Synthetic reader\ntype: synthetic\n---\nSynthetic text with [[S-940#2]], [[B-940]] and [[C-940]].\n{extra}"
    )
}

fn speaker(kind: &str) -> String {
    format!(
        "---\n{MARK}id: synthetic-speaker\nkind: {kind}\ntitle: Synthetic speaker\nparent: synthetic-guild\n---\nSynthetic text of the speaker.\n"
    )
}

fn belief(id: &str, title: &str) -> String {
    format!(
        "---\n{MARK}id: {id}\nkind: belief\ntitle: {title}\nstatus: draft\nheld_by: [synthetic-speaker]\ntopic: synthetic-subject-in-world\n---\nSynthetic stance A.\n"
    )
}

fn script(second_beat: bool) -> String {
    let mut text = format!(
        "---\n{MARK}id: S-940\nkind: script\ntitle: Synthetic script\nstatus: draft\n---\nSynthetic opening text.\n\n## Beat 1 — First\nat:: [[f01-synthetic]]\nwith:: [[synthetic-speaker]]\n"
    );
    if second_beat {
        text.push_str(
            "\n## Beat 2 — Second\nat:: [[f01-synthetic]]\nwith:: [[synthetic-speaker]]\n",
        );
    }
    text
}

fn in_world(track: &str) -> String {
    format!(
        "---\n{MARK}id: synthetic-subject-in-world\nkind: topic\ntitle: Synthetic subject, in-world\nstatus: draft\ntrack: {track}\ncounterpart: synthetic-subject-explaining\n---\nSynthetic in-world text.\n"
    )
}

fn write_project(project: &TempProject) -> std::io::Result<()> {
    let files: [(&str, String); 13] = [
        (
            "world/synthetic-band.md",
            format!(
                "---\n{MARK}id: synthetic-band\nkind: band\ntitle: Synthetic band\nparent: tower\nfloors: [1, 1]\n---\nSynthetic text of the band.\n"
            ),
        ),
        (
            "world/synthetic-band/f01-synthetic.md",
            format!(
                "---\n{MARK}id: f01-synthetic\nkind: floor\ntitle: Synthetic floor\nstatus: draft\nparent: synthetic-band\nnumber: 1\n---\nSynthetic text of the floor.\n"
            ),
        ),
        (
            "people/synthetic-folk.md",
            format!(
                "---\n{MARK}id: synthetic-folk\nkind: people\ntitle: Synthetic folk\n---\nSynthetic text of the folk.\n"
            ),
        ),
        (
            "people/synthetic-folk/synthetic-guild.md",
            format!(
                "---\n{MARK}id: synthetic-guild\nkind: faction\ntitle: Synthetic guild\nparent: synthetic-folk\n---\nSynthetic text of the guild.\n"
            ),
        ),
        (SPEAKER, speaker("npc")),
        (
            "lore/topics/synthetic-subject-explaining.md",
            format!(
                "---\n{MARK}id: synthetic-subject-explaining\nkind: topic\ntitle: Synthetic subject, explaining\nstatus: draft\ntrack: explaining\n---\nSynthetic explaining text.\n"
            ),
        ),
        (IN_WORLD, in_world("in-world")),
        (
            TRUTH,
            format!(
                "---\n{MARK}id: T-940\nkind: truth\ntitle: Synthetic truth A\nstatus: draft\ntarget: hinted\ntopic: synthetic-subject-explaining\n---\nSynthetic statement A.\n"
            ),
        ),
        (BELIEF, belief("B-940", "Synthetic belief A")),
        (SCRIPT, script(true)),
        (READER, reader("")),
        (
            "world/tower.md",
            format!(
                "---\n{MARK}id: tower\nkind: tower\ntitle: Synthetic tower\n---\nSynthetic text of the tower.\n"
            ),
        ),
        (
            "things/synthetic-other.md",
            format!(
                "---\n{MARK}id: synthetic-other\nkind: thing\ntitle: Synthetic other\ntype: synthetic\n---\nSynthetic text.\n"
            ),
        ),
    ];
    for (path, text) in files {
        project.write(path, &text)?;
    }
    Ok(())
}

fn lists(read: &[&str], reparsed: &[&str], removed: &[&str], relinked: &[&str]) -> Update {
    let own = |list: &[&str]| list.iter().map(|path| (*path).to_owned()).collect();
    Update {
        read: own(read),
        reparsed: own(reparsed),
        removed: own(removed),
        relinked: own(relinked),
    }
}

/// The dump of a fresh in-memory index over the same folder.
fn fresh_dump(project: &TempProject) -> Result<String, Box<dyn std::error::Error>> {
    let mut fresh = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    fresh.sync()?;
    Ok(fresh.dump()?)
}

fn opened(project: &TempProject) -> Result<Index, Box<dyn std::error::Error>> {
    write_project(project)?;
    let (index, _) = Index::open(&project.project()?, &parts::builtin()?)?;
    Ok(index)
}

// AC: CE-12.2
#[test]
fn each_change_updates_its_record_and_its_backlinks() -> TestResult {
    let project = TempProject::new()?;
    let mut index = opened(&project)?;
    let step = |index: &mut Index, expected: Update| -> TestResult {
        let update = index.sync()?;
        assert_eq!(update, expected);
        assert_eq!(index.dump()?, fresh_dump(&project)?);
        Ok(())
    };

    project.write(READER, &reader("Also [[T-940]].\n"))?;
    step(&mut index, lists(&[READER], &[READER], &[], &[]))?;

    project.write(BELIEF, &belief("B-940", "Synthetic belief A, retitled"))?;
    step(&mut index, lists(&[BELIEF], &[BELIEF], &[], &[]))?;

    project.remove(BELIEF)?;
    project.write(
        BELIEF_RENAMED,
        &belief("B-941", "Synthetic belief A, retitled"),
    )?;
    step(
        &mut index,
        lists(&[BELIEF_RENAMED], &[BELIEF_RENAMED], &[BELIEF], &[READER]),
    )?;

    project.write(SPEAKER, &speaker("faction"))?;
    step(
        &mut index,
        lists(&[SPEAKER], &[SPEAKER], &[], &[BELIEF_RENAMED, SCRIPT]),
    )?;

    project.write(SCRIPT, &script(false))?;
    step(&mut index, lists(&[SCRIPT], &[SCRIPT], &[], &[READER]))?;

    project.write(IN_WORLD, &in_world("explaining"))?;
    step(
        &mut index,
        lists(&[IN_WORLD], &[IN_WORLD], &[], &[BELIEF_RENAMED]),
    )?;

    project.remove(TRUTH)?;
    step(&mut index, lists(&[], &[], &[TRUTH], &[READER]))?;

    let dangling = |index: &Index| -> Result<usize, Box<dyn std::error::Error>> {
        Ok(index
            .findings_for(READER)?
            .iter()
            .filter(|f| f.rule == "link/dangling" && f.message.contains("C-940"))
            .count())
    };
    assert_eq!(dangling(&index)?, 1);
    project.write(
        CLUE,
        &format!(
            "---\n{MARK}id: C-940\nkind: clue\ntitle: Synthetic clue A\nstatus: draft\nmoves_to: hinted\nwhere: [f01-synthetic]\ngate: T1\nvector: synthetic vector\n---\nSynthetic clue text.\n"
        ),
    )?;
    step(&mut index, lists(&[CLUE], &[CLUE], &[], &[READER]))?;
    assert_eq!(dangling(&index)?, 0);
    Ok(())
}

// AC: CE-12.2
#[test]
fn duplicates_come_and_go_with_changes() -> TestResult {
    let project = TempProject::new()?;
    let mut index = opened(&project)?;
    let twin = "dialogue/T-940.md";
    let duplicates = |index: &Index| -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(index
            .findings()?
            .into_iter()
            .filter(|f| f.rule == "index/duplicate-id")
            .map(|f| f.path)
            .collect())
    };
    assert_eq!(duplicates(&index)?, Vec::<String>::new());

    project.write(
        twin,
        &format!(
            "---\n{MARK}id: T-940\nkind: thing\ntitle: Synthetic twin\ntype: synthetic\n---\nSynthetic twin text.\n"
        ),
    )?;
    index.sync()?;
    let mut both = duplicates(&index)?;
    both.sort();
    assert_eq!(both, [twin, TRUTH]);
    assert_eq!(index.dump()?, fresh_dump(&project)?);

    project.remove(twin)?;
    index.sync()?;
    assert_eq!(duplicates(&index)?, Vec::<String>::new());
    assert_eq!(index.dump()?, fresh_dump(&project)?);
    Ok(())
}

// AC: CE-12.2
#[test]
fn a_rename_with_an_edit() -> TestResult {
    let project = TempProject::new()?;
    let mut index = opened(&project)?;
    let renamed = "people/synthetic-folk/synthetic-guild/synthetic-speaker-renamed.md";
    project.remove(SPEAKER)?;
    project.write(
        renamed,
        &speaker("npc").replace("Synthetic speaker", "Synthetic speaker, edited"),
    )?;
    let update = index.sync()?;
    assert_eq!(
        update,
        lists(&[renamed], &[renamed], &[SPEAKER], &[BELIEF, SCRIPT])
    );
    assert_eq!(index.dump()?, fresh_dump(&project)?);
    Ok(())
}
