//! Relinking only what a change touches: each change updates its record and its backlinks.

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use theos_content::{Change, Index, KindRegistry, Update, formalism, parts};

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

fn truth(title: &str) -> String {
    format!(
        "---\n{MARK}id: T-940\nkind: truth\ntitle: {title}\nstatus: draft\ntarget: hinted\ntopic: synthetic-subject-explaining\n---\nSynthetic statement A.\n"
    )
}

/// A `thing` with the id `T-940`, the twin of [`TRUTH`].
const TWIN: &str = "dialogue/T-940.md";

fn twin(title: &str) -> String {
    format!(
        "---\n{MARK}id: T-940\nkind: thing\ntitle: {title}\ntype: synthetic\n---\nSynthetic twin text.\n"
    )
}

/// The paths of the `index/duplicate-id` findings, sorted.
fn duplicates(index: &Index) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut paths: Vec<String> = index
        .findings()?
        .into_iter()
        .filter(|f| f.rule == "index/duplicate-id")
        .map(|f| f.path)
        .collect();
    paths.sort();
    Ok(paths)
}

fn in_world(track: &str) -> String {
    format!(
        "---\n{MARK}id: synthetic-subject-in-world\nkind: topic\ntitle: Synthetic subject, in-world\nstatus: draft\ntrack: {track}\ncounterpart: synthetic-subject-explaining\n---\nSynthetic in-world text.\n"
    )
}

fn clue() -> String {
    format!(
        "---\n{MARK}id: C-940\nkind: clue\ntitle: Synthetic clue A\nstatus: draft\nmoves_to: hinted\nwhere: [f01-synthetic]\ngate: T1\nvector: synthetic vector\n---\nSynthetic clue text.\n"
    )
}

/// The files of the project: a band and its floor, a people, faction and npc chain, a topic pair,
/// a truth, a belief the npc holds, a script with two beats, and a reader that links a beat, the
/// belief and the clue [`CLUE`], which does not exist yet; and the files of [`lore_files`].
fn project_files() -> Vec<(&'static str, String)> {
    let mut files = vec![
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
        (TRUTH, truth("Synthetic truth A")),
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
    files.extend(lore_files());
    files
}

const MARK_P04: &str = "# synthetic fixture (P04), not canon\n";

const SETTINGS: &str = ".theos/settings.toml";
const SOURCE: &str = "canon/formalism/_source.json";
const E1: &str = "canon/formalism/E1.md";

/// The settings of the project: one developer word, which no in-world text holds.
const SETTINGS_TEXT: &str =
    "# synthetic fixture (P04), not canon\n[validate.inworld-vocab]\nwords = [\"lamp\"]\n";

/// The Shared Formalism source of the project, with the equation E1 only.
const SOURCE_TEXT: &str = "{\n  \"note\": \"synthetic fixture (P04), not canon\",\n  \"format\": 1,\n  \"page\": \"https://example.invalid/synthetic-formalism\",\n  \"equations\": [\n    {\n      \"id\": \"E1\",\n      \"title\": \"Synthetic equation 1\",\n      \"anchor\": \"https://example.invalid/synthetic-formalism#E1\",\n      \"text\": \"$x_{1} = 1$\"\n    }\n  ]\n}\n";

/// The record E1 as [`SOURCE_TEXT`] renders it.
const E1_TEXT: &str = "---\n# generated from canon/formalism/_source.json by theos; a hand edit fails the check\nid: E1\nkind: equation\ntitle: Synthetic equation 1\nsource: https://example.invalid/synthetic-formalism#E1\ngenerated: true\n---\n$x_{1} = 1$\n";

/// A P04 record with these front-matter lines and this body.
fn lore_record(fields: &str, body: &str) -> String {
    format!("---\n{MARK_P04}{fields}---\n{body}")
}

fn lore_clue(id: &str, truth: &str, state: &str, place: &str, gate: &str, after: &str) -> String {
    lore_record(
        &format!(
            "id: {id}\nkind: clue\ntitle: Synthetic clue {id}\nstatus: draft\nreveals: {truth}\nmoves_to: {state}\nwhere: [{place}]\ngate: {gate}\n{after}vector: synthetic\n"
        ),
        "Synthetic clue text.\n",
    )
}

/// The files of the lore step (spec §5.17): a band with two floors and a location on the first;
/// two truths and three clues, one of them ordered after another; a script whose beats reveal two
/// of them; a concept with two notes and a version that addresses both; the settings with one
/// word; and the formalism source with E1 and its record. None of them reads what the CE-12.2
/// tests above change.
fn lore_files() -> Vec<(&'static str, String)> {
    vec![
        (
            "world/synthetic-upper.md",
            lore_record(
                "id: synthetic-upper\nkind: band\ntitle: Synthetic upper band\nparent: tower\nfloors: [2, 3]\n",
                "Synthetic text of the band.\n",
            ),
        ),
        (
            "world/synthetic-upper/f02-synthetic.md",
            lore_record(
                "id: f02-synthetic\nkind: floor\ntitle: Synthetic floor two\nstatus: draft\nparent: synthetic-upper\nnumber: 2\n",
                "Synthetic text of the floor.\n",
            ),
        ),
        (
            "world/synthetic-upper/f03-synthetic.md",
            lore_record(
                "id: f03-synthetic\nkind: floor\ntitle: Synthetic floor three\nstatus: draft\nparent: synthetic-upper\nnumber: 3\n",
                "Synthetic text of the floor.\n",
            ),
        ),
        (
            "world/synthetic-upper/f02-synthetic/synthetic-room.md",
            lore_record(
                "id: synthetic-room\nkind: location\ntitle: Synthetic room\nparent: f02-synthetic\n",
                "Synthetic text of the room.\n",
            ),
        ),
        (
            "lore/truths/T-950.md",
            lore_record(
                "id: T-950\nkind: truth\ntitle: Synthetic truth B\nstatus: draft\ntarget: confirmed\n",
                "Synthetic statement B.\n",
            ),
        ),
        (
            "lore/truths/T-951.md",
            lore_record(
                "id: T-951\nkind: truth\ntitle: Synthetic truth C\nstatus: draft\ntarget: confirmed\n",
                "Synthetic statement C.\n",
            ),
        ),
        (
            "lore/clues/C-950.md",
            lore_clue("C-950", "T-950", "hinted", "synthetic-room", "T1", ""),
        ),
        (
            "lore/clues/C-951.md",
            lore_clue("C-951", "T-950", "confirmed", "f03-synthetic", "T2", ""),
        ),
        (
            "lore/clues/C-952.md",
            lore_clue(
                "C-952",
                "T-951",
                "confirmed",
                "f02-synthetic",
                "T2",
                "after: [C-950]\n",
            ),
        ),
        (
            "story/scripts/S-950.md",
            lore_record(
                "id: S-950\nkind: script\ntitle: Synthetic script B\nstatus: draft\nwhere: [synthetic-upper]\n",
                "Synthetic opening text.\n\n## Beat 1 — Room\nat:: [[synthetic-room]]\nwhen:: T1\nreveals:: [[C-950]]\n\n## Beat 2 — Floor\nat:: [[f03-synthetic]]\nwhen:: T2\nreveals:: [[C-951]]\n",
            ),
        ),
        (
            "concepts/synthetic-sketch.md",
            lore_record(
                "id: synthetic-sketch\nkind: concept\ntitle: Synthetic sketch\nfor: [synthetic-upper]\n",
                "Synthetic brief.\n\n## Notes\n\n> [!note] n1 · open\n> Synthetic note A.\n\n> [!note] n2 · open\n> Synthetic note B.\n",
            ),
        ),
        (
            "concepts/synthetic-sketch/synthetic-sketch-v1.md",
            lore_record(
                "id: synthetic-sketch-v1\nkind: concept-version\ntitle: synthetic-sketch-v1\nconcept: synthetic-sketch\ngenerator: synthetic@1.0\nrecipe: synthetic-recipe\noutputs:\n  image: concepts/synthetic-sketch/synthetic-sketch-v1.png@sha256:0000000000000000000000000000000000000000000000000000000000000000\nrating: unrated\naddresses: [n1, n2]\n",
                "Synthetic version.\n",
            ),
        ),
        (SETTINGS, SETTINGS_TEXT.to_owned()),
        (SOURCE, SOURCE_TEXT.to_owned()),
        (E1, E1_TEXT.to_owned()),
    ]
}

#[test]
fn the_history_project_holds_e1_as_its_source_renders_it() -> TestResult {
    let rendered = formalism::render(&formalism::parse(SOURCE_TEXT)?)?;
    assert_eq!(rendered, [(E1.to_owned(), E1_TEXT.to_owned())]);
    Ok(())
}

fn write_project(project: &TempProject) -> std::io::Result<()> {
    for (path, text) in project_files() {
        project.write(path, &text)?;
    }
    Ok(())
}

/// The update of a change; no lore rule is registered, so only a truth, which stores a ledger,
/// is revalidated (see [`with_truth`]).
fn lists(read: &[&str], reparsed: &[&str], removed: &[&str], relinked: &[&str]) -> Update {
    let own = |list: &[&str]| list.iter().map(|path| (*path).to_owned()).collect();
    Update {
        read: own(read),
        reparsed: own(reparsed),
        removed: own(removed),
        relinked: own(relinked),
        revalidated: Vec::new(),
    }
}

/// `update` with the truth revalidated: it read the links to it, which the change touched.
fn with_truth(mut update: Update) -> Update {
    update.revalidated = vec![TRUTH.to_owned()];
    update
}

/// The dump of a fresh in-memory index over the same folder.
fn fresh_dump(project: &TempProject) -> Result<String, Box<dyn std::error::Error>> {
    fresh_dump_with(project, &parts::builtin()?)
}

/// The dump of a fresh in-memory index with `registry` over the same folder.
fn fresh_dump_with(
    project: &TempProject,
    registry: &KindRegistry,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut fresh = Index::in_memory(&project.project()?, registry)?;
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
    step(
        &mut index,
        with_truth(lists(&[READER], &[READER], &[], &[])),
    )?;

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
    project.write(CLUE, &clue())?;
    step(&mut index, lists(&[CLUE], &[CLUE], &[], &[READER]))?;
    assert_eq!(dangling(&index)?, 0);
    Ok(())
}

// AC: CE-12.2
#[test]
fn duplicates_come_and_go_with_changes() -> TestResult {
    let project = TempProject::new()?;
    let mut index = opened(&project)?;
    assert_eq!(duplicates(&index)?, Vec::<String>::new());

    project.write(TWIN, &twin("Synthetic twin"))?;
    index.sync()?;
    assert_eq!(duplicates(&index)?, [TWIN, TRUTH]);
    assert_eq!(index.dump()?, fresh_dump(&project)?);

    project.remove(TWIN)?;
    index.sync()?;
    assert_eq!(duplicates(&index)?, Vec::<String>::new());
    assert_eq!(index.dump()?, fresh_dump(&project)?);
    Ok(())
}

// AC: CE-12.3
#[test]
fn a_duplicate_stays_reported_when_a_twin_is_edited() -> TestResult {
    let project = TempProject::new()?;
    let mut index = opened(&project)?;
    project.write(TWIN, &twin("Synthetic twin"))?;
    index.sync()?;
    assert_eq!(duplicates(&index)?, [TWIN, TRUTH]);

    // An edit on disk that leaves the link facts as they were.
    project.write(TWIN, &twin("Synthetic twin, retitled"))?;
    let update = index.sync()?;
    assert_eq!(update, lists(&[TWIN], &[TWIN], &[], &[]));
    assert_eq!(duplicates(&index)?, [TWIN, TRUTH]);
    assert_eq!(index.dump()?, fresh_dump(&project)?);

    // The same through `apply`, on the other twin.
    let retitled = truth("Synthetic truth A, retitled");
    project.write(TRUTH, &retitled)?;
    let update = index.apply(&[Change::text(TRUTH, retitled)])?;
    assert_eq!(update, lists(&[], &[TRUTH], &[], &[]));
    assert_eq!(duplicates(&index)?, [TWIN, TRUTH]);
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

/// One edit of a generated history. Each number picks, modulo their count, among the files that
/// exist when the edit runs, or in one of the lists below.
#[derive(Clone, Debug)]
enum Edit {
    /// Appends ` B` to the title of a file.
    Title(usize),
    /// Adds a body link to one of [`TARGETS`], or removes it when the file has it.
    Link(usize, usize),
    /// Gives a file a new id and the file name that goes with it.
    Rename(usize),
    /// Moves a file into one of [`FOLDERS`].
    Move(usize, usize),
    /// Sets the `kind` of a file to one of [`KINDS`].
    Kind(usize, usize),
    /// Removes the second beat of a script, or puts it back.
    Beat(usize),
    /// Switches the `track` of a topic between `in-world` and `explaining`.
    Track(usize),
    /// Deletes a file.
    Delete(usize),
    /// Adds one of the files of [`added`].
    Add(usize),
    /// `[Removed a, Content a]`: a file removed and given again with a new title.
    ReAdd(usize),
    /// `[Content a, Removed a]`: a file given with a new title and removed.
    AddRemove(usize),
    /// Changes the case of the first letter of a file name, and nothing else.
    Recase(usize),
    /// Adds `dialogue/` to the root `.gitignore`, or takes it out again. It always reaches the
    /// index through a sync: `update` and `apply` take records, not ignore files.
    Ignore,
    /// Sets the `moves_to` of a clue to the next of [`STATES`].
    MovesTo(usize),
    /// Sets the `target` of a truth to the next of [`TARGET_STATES`].
    Target(usize),
    /// Sets the truth a clue `reveals` to the next of [`TRUTHS`].
    Truth(usize),
    /// Sets the `where` of a clue to the next of [`PLACES`].
    Where(usize),
    /// Gives a clue `after` one of [`CLUES`], or takes its `after` away.
    After(usize),
    /// Gives a truth `requires` one of [`TRUTHS`], or takes its `requires` away.
    Requires(usize),
    /// Adds a `reveals::` of one of [`CLUES`] to the last beat of a script, or removes it.
    Reveals(usize),
    /// Switches the `floors` of `synthetic-upper` between `[2, 3]` and `[3, 3]`.
    Floors,
    /// Switches the `number` of `f02-synthetic` between 2 and 3.
    Number,
    /// Moves `f03-synthetic` between the bands `synthetic-upper` and `synthetic-band`.
    FloorParent,
    /// Closes the parent cycle of `f02-synthetic` and `synthetic-room`, or opens it again.
    ParentCycle,
    /// Adds the developer word `text`, which the in-world topic holds, to the settings, or
    /// removes it; writes [`SETTINGS_TEXT`] when the project has no settings.
    Word,
    /// Changes the title of E1 in the formalism source, or changes it back; writes
    /// [`SOURCE_TEXT`] when the project has no source.
    Equation,
    /// Adds a space to the end of an equation record, or takes it away: a hand edit.
    ByHand,
}

/// How the edit of a step reaches the index.
#[derive(Clone, Copy, Debug)]
enum Via {
    /// The files are written and the project synced.
    Sync,
    /// The files are written and their paths given to `update`.
    Update,
    /// The files are written and the same texts given to `apply`.
    Apply,
    /// The files are written; the same texts are previewed, which must leave the cache as it
    /// was, and then applied, which must store what the preview returned.
    Preview,
}

const FOLDERS: [&str; 5] = [
    "things",
    "dialogue",
    "world/synthetic-band",
    "people/synthetic-folk",
    "lore/beliefs",
];

const KINDS: [&str; 7] = [
    "thing", "npc", "faction", "belief", "topic", "clue", "truth",
];

const TARGETS: [&str; 10] = [
    "S-940#2",
    "B-940",
    "C-940",
    "T-940",
    "synthetic-speaker",
    "synthetic-subject-in-world",
    "synthetic-nowhere",
    "Synthetic-Other",
    "C-950",
    "E1",
];

const STATES: [&str; 3] = ["hinted", "suspected", "confirmed"];

const TARGET_STATES: [&str; 4] = ["hinted", "suspected", "confirmed", "never"];

const TRUTHS: [&str; 3] = ["T-950", "T-951", "T-940"];

const CLUES: [&str; 3] = ["C-950", "C-951", "C-952"];

const PLACES: [&str; 5] = [
    "[synthetic-room]",
    "[f02-synthetic]",
    "[f03-synthetic]",
    "[synthetic-upper]",
    "[f01-synthetic]",
];

/// The files an [`Edit::Add`] adds: a twin of the truth's id, the clue the reader already links,
/// a twin of `synthetic-other` apart from case, a thing that links the npc and a beat, and a file
/// that fails to split.
fn added(n: usize) -> (&'static str, String) {
    match n % 5 {
        0 => (TWIN, twin("Synthetic twin")),
        1 => (CLUE, clue()),
        2 => (
            "dialogue/Synthetic-Other.md",
            format!(
                "---\n{MARK}id: Synthetic-Other\nkind: thing\ntitle: Synthetic other twin\ntype: synthetic\n---\nSynthetic text.\n"
            ),
        ),
        3 => (
            "things/synthetic-extra.md",
            format!(
                "---\n{MARK}id: synthetic-extra\nkind: thing\ntitle: Synthetic extra\ntype: synthetic\n---\nSynthetic text with [[synthetic-speaker]] and [[S-940#1]].\n"
            ),
        ),
        _ => (
            "things/synthetic-broken.md",
            "synthetic text without a fence\n".to_owned(),
        ),
    }
}

/// `text` with the value of its first line that starts with `prefix` replaced by `new` of it;
/// unchanged when no line does.
fn with_line(text: &str, prefix: &str, new: impl Fn(&str) -> String) -> String {
    let mut done = false;
    let mut out = String::with_capacity(text.len() + 8);
    for line in text.split_inclusive('\n') {
        match line.strip_prefix(prefix) {
            Some(rest) if !done => {
                done = true;
                out.push_str(prefix);
                out.push_str(&new(rest.trim_end_matches('\n')));
                out.push('\n');
            }
            _ => out.push_str(line),
        }
    }
    out
}

fn retitled(text: &str) -> String {
    with_line(text, "title: ", |title| format!("{title} B"))
}

fn toggle_link(text: &str, target: &str) -> String {
    let line = format!("Synthetic link [[{target}]].\n");
    if text.contains(&line) {
        text.replacen(&line, "", 1)
    } else {
        format!("{text}{line}")
    }
}

const BEAT_2: &str = "\n## Beat 2 — Second\nat:: [[f01-synthetic]]\nwith:: [[synthetic-speaker]]\n";

fn toggle_beat(text: &str) -> String {
    match text.find("\n## Beat 2") {
        Some(at) => text[..at].to_owned(),
        None => format!("{text}{BEAT_2}"),
    }
}

/// The item after `current` in `list`, round; the first when `current` is not in it.
fn next_of(list: &[&str], current: &str) -> String {
    let at = list
        .iter()
        .position(|item| *item == current)
        .map_or(0, |at| (at + 1) % list.len());
    list[at].to_owned()
}

/// `text` without its line `key: …`, or, when it has none, with `key: value` added as the last
/// line of its front matter; unchanged without a front matter.
fn toggle_field(text: &str, key: &str, value: &str) -> String {
    let prefix = format!("{key}: ");
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    if let Some(at) = lines.iter().position(|line| line.starts_with(&prefix)) {
        return lines
            .iter()
            .enumerate()
            .filter(|(number, _)| *number != at)
            .map(|(_, line)| *line)
            .collect();
    }
    let fence = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim_end() == "---");
    match fence {
        Some((at, _)) => format!(
            "{}{prefix}{value}\n{}",
            lines[..at].concat(),
            lines[at..].concat()
        ),
        None => text.to_owned(),
    }
}

/// `text` without the beat attribute `reveals:: [[clue]]`, or with it added at the end.
fn toggle_reveal(text: &str, clue: &str) -> String {
    let line = format!("reveals:: [[{clue}]]\n");
    if text.contains(&line) {
        text.replacen(&line, "", 1)
    } else if text.is_empty() || text.ends_with('\n') {
        format!("{text}{line}")
    } else {
        format!("{text}\n{line}")
    }
}

/// `text` with a space before its final line break, or without the one it has.
fn hand_edited(text: &str) -> String {
    if let Some(rest) = text.strip_suffix(" \n") {
        return format!("{rest}\n");
    }
    match text.strip_suffix('\n') {
        Some(rest) => format!("{rest} \n"),
        None => format!("{text} "),
    }
}

/// One of two values: `b` when `value` is `a`, else `a`.
fn switched(value: &str, a: &str, b: &str) -> String {
    if value == a { b } else { a }.to_owned()
}

/// The id after `stem`: `T-940` gives `T-941`, a slug gets `-r`.
fn bumped(stem: &str) -> String {
    if let Some((letter, digits)) = stem.split_once('-')
        && letter.len() == 1
        && let Ok(number) = digits.parse::<u32>()
    {
        return format!("{letter}-{}", number + 1);
    }
    format!("{stem}-r")
}

/// `name` with the case of its first letter changed.
fn recased(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {
            format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
        }
        Some(first) => format!("{}{}", first.to_ascii_lowercase(), chars.as_str()),
        None => String::new(),
    }
}

fn folder_and_name(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

/// The path and text of the file at `path` after an [`Edit::Rename`].
fn renamed(path: &str, text: &str) -> (String, String) {
    let (folder, name) = folder_and_name(path);
    let stem = name.strip_suffix(".md").unwrap_or(name);
    let new = bumped(stem);
    let text = with_line(text, "id: ", |id| {
        if id == stem {
            new.clone()
        } else {
            id.to_owned()
        }
    });
    (format!("{folder}/{new}.md"), text)
}

/// What one step does to the folder, in order: a path with its new text, or `None` for a
/// removal.
type Plan = Vec<(String, Option<String>)>;

/// The plan of `edit` over `files`.
fn plan(files: &BTreeMap<String, String>, edit: &Edit) -> Plan {
    let pick = |n: usize, has: &dyn Fn(&str) -> bool| -> Option<(String, String)> {
        let found: Vec<(&String, &String)> = files.iter().filter(|(_, t)| has(t)).collect();
        let (path, text) = found.get(n % found.len().max(1))?;
        Some(((*path).clone(), (*text).clone()))
    };
    let any = |_: &str| true;
    let write = |path: String, text: String| vec![(path, Some(text))];
    let planned = match *edit {
        Edit::Title(n) => pick(n, &any).map(|(path, text)| write(path, retitled(&text))),
        Edit::Link(n, m) => pick(n, &any)
            .map(|(path, text)| write(path, toggle_link(&text, TARGETS[m % TARGETS.len()]))),
        Edit::Rename(n) => pick(n, &any).map(|(path, text)| {
            let (new_path, new_text) = renamed(&path, &text);
            vec![(path, None), (new_path, Some(new_text))]
        }),
        Edit::Move(n, m) => pick(n, &any).map(|(path, text)| {
            let (_, name) = folder_and_name(&path);
            let new_path = format!("{}/{name}", FOLDERS[m % FOLDERS.len()]);
            vec![(path, None), (new_path, Some(text))]
        }),
        Edit::Kind(n, m) => pick(n, &any).map(|(path, text)| {
            write(
                path,
                with_line(&text, "kind: ", |_| KINDS[m % KINDS.len()].to_owned()),
            )
        }),
        Edit::Beat(n) => pick(n, &|text| text.contains("\n## Beat 1"))
            .map(|(path, text)| write(path, toggle_beat(&text))),
        Edit::Track(n) => pick(n, &|text| text.contains("\ntrack: ")).map(|(path, text)| {
            let track = |old: &str| {
                if old == "in-world" {
                    "explaining"
                } else {
                    "in-world"
                }
                .to_owned()
            };
            write(path, with_line(&text, "track: ", track))
        }),
        Edit::Delete(n) => pick(n, &any).map(|(path, _)| vec![(path, None)]),
        Edit::Add(n) => {
            let (path, text) = added(n);
            Some(write(path.to_owned(), text))
        }
        Edit::ReAdd(n) => pick(n, &any)
            .map(|(path, text)| vec![(path.clone(), None), (path, Some(retitled(&text)))]),
        Edit::AddRemove(n) => pick(n, &any)
            .map(|(path, text)| vec![(path.clone(), Some(retitled(&text))), (path, None)]),
        Edit::Recase(n) => pick(n, &any).map(|(path, text)| {
            let (folder, name) = folder_and_name(&path);
            let new_path = format!("{folder}/{}", recased(name));
            vec![(path, None), (new_path, Some(text))]
        }),
        Edit::Ignore => None,
        Edit::MovesTo(_)
        | Edit::Target(_)
        | Edit::Truth(_)
        | Edit::Where(_)
        | Edit::After(_)
        | Edit::Requires(_)
        | Edit::Reveals(_)
        | Edit::Floors
        | Edit::Number
        | Edit::FloorParent
        | Edit::ParentCycle
        | Edit::Word
        | Edit::Equation
        | Edit::ByHand => lore_edit(files, edit).map(|(path, text)| write(path, text)),
    };
    without_case_twins(files, planned.unwrap_or_default())
}

/// The path and new text of an edit of the lore step over `files` (spec §5.17); none when the
/// file it edits is gone, or for another edit.
fn lore_edit(files: &BTreeMap<String, String>, edit: &Edit) -> Option<(String, String)> {
    let pick = |n: usize, kind: &str| -> Option<(String, String)> {
        let line = format!("\nkind: {kind}\n");
        let found: Vec<(&String, &String)> = files
            .iter()
            .filter(|(_, text)| text.contains(&line))
            .collect();
        let (path, text) = found.get(n % found.len().max(1))?;
        Some(((*path).clone(), (*text).clone()))
    };
    let by_id = |id: &str| -> Option<(String, String)> {
        let line = format!("\nid: {id}\n");
        let (path, text) = files.iter().find(|(_, text)| text.contains(&line))?;
        Some((path.clone(), text.clone()))
    };
    let edited = |found: Option<(String, String)>, edit: &dyn Fn(&str) -> String| {
        found.map(|(path, text)| (path, edit(&text)))
    };
    match *edit {
        Edit::MovesTo(n) => edited(pick(n, "clue"), &|text| {
            with_line(text, "moves_to: ", |value| next_of(&STATES, value))
        }),
        Edit::Target(n) => edited(pick(n, "truth"), &|text| {
            with_line(text, "target: ", |value| next_of(&TARGET_STATES, value))
        }),
        Edit::Truth(n) => edited(pick(n, "clue"), &|text| {
            with_line(text, "reveals: ", |value| next_of(&TRUTHS, value))
        }),
        Edit::Where(n) => edited(pick(n, "clue"), &|text| {
            with_line(text, "where: ", |value| next_of(&PLACES, value))
        }),
        Edit::After(n) => edited(pick(n, "clue"), &|text| {
            toggle_field(
                text,
                "after",
                &format!("[{}]", CLUES[(n + 1) % CLUES.len()]),
            )
        }),
        Edit::Requires(n) => edited(pick(n, "truth"), &|text| {
            toggle_field(
                text,
                "requires",
                &format!("[{}]", TRUTHS[(n + 1) % TRUTHS.len()]),
            )
        }),
        Edit::Reveals(n) => edited(pick(n, "script"), &|text| {
            toggle_reveal(text, CLUES[n % CLUES.len()])
        }),
        Edit::Floors => edited(by_id("synthetic-upper"), &|text| {
            with_line(text, "floors: ", |value| {
                switched(value, "[2, 3]", "[3, 3]")
            })
        }),
        Edit::Number => edited(by_id("f02-synthetic"), &|text| {
            with_line(text, "number: ", |value| switched(value, "2", "3"))
        }),
        Edit::FloorParent => edited(by_id("f03-synthetic"), &|text| {
            with_line(text, "parent: ", |value| {
                switched(value, "synthetic-upper", "synthetic-band")
            })
        }),
        Edit::ParentCycle => edited(by_id("f02-synthetic"), &|text| {
            with_line(text, "parent: ", |value| {
                switched(value, "synthetic-upper", "synthetic-room")
            })
        }),
        Edit::Word => Some(match files.get(SETTINGS) {
            Some(text) => (
                SETTINGS.to_owned(),
                with_line(text, "words = ", |value| {
                    switched(value, "[\"lamp\"]", "[\"lamp\", \"text\"]")
                }),
            ),
            None => (SETTINGS.to_owned(), SETTINGS_TEXT.to_owned()),
        }),
        Edit::Equation => Some(match files.get(SOURCE) {
            Some(text) => {
                let (one, other) = ("\"Synthetic equation 1\"", "\"Synthetic equation one\"");
                let text = if text.contains(one) {
                    text.replacen(one, other, 1)
                } else {
                    text.replacen(other, one, 1)
                };
                (SOURCE.to_owned(), text)
            }
            None => (SOURCE.to_owned(), SOURCE_TEXT.to_owned()),
        }),
        Edit::ByHand => edited(pick(0, "equation"), &hand_edited),
        _ => None,
    }
}

/// `plan` with a removal of every file whose path differs only in case from a path the plan
/// writes, right before that write: on Windows and macOS the two are one file, so the history
/// removes the old spelling first and the folder looks the same on every OS.
fn without_case_twins(files: &BTreeMap<String, String>, plan: Plan) -> Plan {
    let mut present: BTreeSet<String> = files.keys().cloned().collect();
    let mut out = Plan::new();
    for (path, text) in plan {
        if text.is_some() {
            let lower = path.to_lowercase();
            let twins: Vec<String> = present
                .iter()
                .filter(|other| **other != path && other.to_lowercase() == lower)
                .cloned()
                .collect();
            for twin in twins {
                present.remove(&twin);
                out.push((twin, None));
            }
            present.insert(path.clone());
        } else {
            present.remove(&path);
        }
        out.push((path, text));
    }
    out
}

/// The lines of a dump, each with its section, that are in `left` and not in `right`.
fn only_in(left: &str, right: &str) -> Vec<String> {
    let lines = |dump: &str| -> BTreeSet<String> {
        let mut section = "";
        let mut lines = BTreeSet::new();
        for line in dump.lines() {
            if line.starts_with('[') {
                section = line;
            } else {
                lines.insert(format!("{section} {line}"));
            }
        }
        lines
    };
    let right = lines(right);
    lines(left)
        .into_iter()
        .filter(|line| !right.contains(line))
        .collect()
}

/// Fails with the lines that differ when the dump `got` is not `want`.
pub fn same_dump(what: &str, got: &str, want: &str) -> TestResult {
    if got == want {
        return Ok(());
    }
    Err(format!(
        "{what}\nonly in the index:\n{}\nonly in the fresh build:\n{}",
        only_in(got, want).join("\n"),
        only_in(want, got).join("\n")
    )
    .into())
}

/// Runs the steps of a history on a project with a file cache and `registry`, and compares the
/// cache with a fresh build of the folder after every step.
fn history(steps: &[(Edit, Via)], registry: &KindRegistry) -> TestResult {
    let project = TempProject::new()?;
    write_project(&project)?;
    let mut files: BTreeMap<String, String> = project_files()
        .into_iter()
        .map(|(path, text)| (path.to_owned(), text))
        .collect();
    let (mut index, _) = Index::open(&project.project()?, registry)?;
    same_dump(
        "after the open",
        &index.dump()?,
        &fresh_dump_with(&project, registry)?,
    )?;
    let mut ignored = false;
    for (number, (edit, via)) in steps.iter().enumerate() {
        let mut via = *via;
        if let Edit::Ignore = edit {
            ignored = !ignored;
            if ignored {
                project.write(".gitignore", "dialogue/\n")?;
            } else {
                project.remove(".gitignore")?;
            }
            via = Via::Sync;
        }
        let plan = plan(&files, edit);
        for (path, text) in &plan {
            if let Some(text) = text {
                project.write(path, text)?;
                files.insert(path.clone(), text.clone());
            } else {
                files.remove(path);
                project.remove(path)?;
            }
        }
        reach(&mut index, &plan, via)?;
        same_dump(
            &format!("after step {number}, {edit:?} via {via:?}"),
            &index.dump()?,
            &fresh_dump_with(&project, registry)?,
        )?;
    }
    Ok(())
}

/// Brings the changes of `plan`, already written to the folder, to the index the way `via` says.
fn reach(index: &mut Index, plan: &Plan, via: Via) -> TestResult {
    let given: Vec<Change> = plan
        .iter()
        .map(|(path, text)| match text {
            Some(text) => Change::text(path.clone(), text.clone()),
            None => Change::Removed { path: path.clone() },
        })
        .collect();
    match via {
        Via::Sync => {
            index.sync()?;
        }
        Via::Update => {
            let mut paths: Vec<&str> = Vec::new();
            for (path, _) in plan {
                if !paths.contains(&path.as_str()) {
                    paths.push(path);
                }
            }
            index.update(&paths)?;
        }
        Via::Apply => {
            index.apply(&given)?;
        }
        Via::Preview => previewed(index, plan, &given)?,
    }
    Ok(())
}

/// Previews `given`, which must leave the cache as it was, then applies it, which must make the
/// update and store the findings that the preview returned.
fn previewed(index: &mut Index, plan: &Plan, given: &[Change]) -> TestResult {
    let before = index.dump()?;
    let preview = index.preview(given)?;
    same_dump("the preview changed the cache", &index.dump()?, &before)?;
    let update = index.apply(given)?;
    if preview.update != update {
        return Err(format!("preview {:?}, apply {update:?}", preview.update).into());
    }
    let mut paths: BTreeSet<&String> = update
        .reparsed
        .iter()
        .chain(&update.relinked)
        .chain(&update.revalidated)
        .collect();
    paths.extend(
        plan.iter()
            .filter(|(_, text)| text.is_some())
            .map(|(path, _)| path),
    );
    let mut stored = Vec::new();
    for path in paths {
        stored.extend(index.findings_for(path)?);
    }
    if preview.findings != stored {
        return Err(format!(
            "the preview found\n{:#?}\nthe apply stored\n{stored:#?}",
            preview.findings
        )
        .into());
    }
    Ok(())
}

fn edit() -> impl Strategy<Value = Edit> {
    // Above the number of files, so that every file can be picked.
    let n = || 0..64usize;
    prop_oneof![
        n().prop_map(Edit::Title),
        (n(), 0..TARGETS.len()).prop_map(|(n, m)| Edit::Link(n, m)),
        n().prop_map(Edit::Rename),
        (n(), 0..FOLDERS.len()).prop_map(|(n, m)| Edit::Move(n, m)),
        (n(), 0..KINDS.len()).prop_map(|(n, m)| Edit::Kind(n, m)),
        n().prop_map(Edit::Beat),
        n().prop_map(Edit::Track),
        n().prop_map(Edit::Delete),
        (0..5usize).prop_map(Edit::Add),
        n().prop_map(Edit::ReAdd),
        n().prop_map(Edit::AddRemove),
        n().prop_map(Edit::Recase),
        Just(Edit::Ignore),
        n().prop_map(Edit::MovesTo),
        n().prop_map(Edit::Target),
        n().prop_map(Edit::Truth),
        n().prop_map(Edit::Where),
        n().prop_map(Edit::After),
        n().prop_map(Edit::Requires),
        n().prop_map(Edit::Reveals),
        Just(Edit::Floors),
        Just(Edit::Number),
        Just(Edit::FloorParent),
        Just(Edit::ParentCycle),
        Just(Edit::Word),
        Just(Edit::Equation),
        Just(Edit::ByHand),
    ]
}

fn via() -> impl Strategy<Value = Via> {
    prop_oneof![
        Just(Via::Sync),
        Just(Via::Update),
        Just(Via::Apply),
        Just(Via::Preview),
    ]
}

// Guards the class "an incremental history after which the cache differs from a fresh build of
// the same files" (spec §3, anti-goal "the cache becomes a source of truth"; final review R3-G):
// every generated history of edits, through sync, update, apply and preview, must leave the
// dump of a fresh build of the folder after each step. The second test runs the same histories
// with a lore rule that reads across records (P04 spec §5.5): a read the lore step fails to redo
// leaves findings or `deps` rows that differ from the fresh build's. The project holds the
// records and input files of the lore step, and the edits change what the lore rules read (P04
// spec §5.17), so both runs also judge every P04 rule, ledger and settings read.
proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn every_history_leaves_the_index_of_a_fresh_build(
        steps in proptest::collection::vec((edit(), via()), 1..=8),
    ) {
        let registry = parts::builtin().map_err(|error| TestCaseError::fail(error.to_string()))?;
        history(&steps, &registry).map_err(|error| TestCaseError::fail(error.to_string()))?;
    }

    #[test]
    fn every_history_with_reads_across_records_leaves_the_index_of_a_fresh_build(
        steps in proptest::collection::vec((edit(), via()), 1..=8),
    ) {
        let registry = crate::lore::echo_registry()
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        history(&steps, &registry).map_err(|error| TestCaseError::fail(error.to_string()))?;
    }
}
