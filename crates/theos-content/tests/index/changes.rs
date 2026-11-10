//! Relinking only what a change touches: each change updates its record and its backlinks.

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use theos_content::{Change, Index, KindRegistry, Update, parts};

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
/// belief and the clue [`CLUE`], which does not exist yet.
fn project_files() -> [(&'static str, String); 13] {
    [
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
    ]
}

fn write_project(project: &TempProject) -> std::io::Result<()> {
    for (path, text) in project_files() {
        project.write(path, &text)?;
    }
    Ok(())
}

/// The update of a change; no lore rule is registered, so nothing is revalidated.
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

const KINDS: [&str; 5] = ["thing", "npc", "faction", "belief", "topic"];

const TARGETS: [&str; 8] = [
    "S-940#2",
    "B-940",
    "C-940",
    "T-940",
    "synthetic-speaker",
    "synthetic-subject-in-world",
    "synthetic-nowhere",
    "Synthetic-Other",
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
    };
    without_case_twins(files, planned.unwrap_or_default())
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
fn same_dump(what: &str, got: &str, want: &str) -> TestResult {
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
    let n = || 0..16usize;
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
// leaves findings or `deps` rows that differ from the fresh build's.
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
