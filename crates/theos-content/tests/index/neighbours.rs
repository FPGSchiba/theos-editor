//! The neighbours of a change (spec §5.5, §5.18 CE-14.3): the six changes of the spec, each
//! through `update` and through `preview` and `apply`, re-run exactly the subjects that read
//! what the change touched.
//!
//! Reads are coarse (spec §5.5, L7): a re-parse changes `rec:` of the record's id and `in:` of
//! every target of its old and new links, so the expected lists below name every subject whose
//! stored reads hold one of those keys, also when its findings stay the same.

use std::collections::BTreeSet;
use std::path::Path;

use theos_content::{Change, Finding, Index, Update, parts};

use crate::common::{TempProject, fixture, same_dump};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// What one change writes, in order: a path with its new text, or `None` for a removal.
type Plan = Vec<(String, Option<String>)>;

/// One change of spec §5.18 and what it must do.
struct Case {
    /// The change, for the messages.
    what: &'static str,
    /// The fixture project it starts from.
    fixture: &'static str,
    /// The files the change writes or removes, from the fixture's texts.
    plan: fn(&Path) -> Result<Plan, Box<dyn std::error::Error>>,
    reparsed: Vec<String>,
    removed: Vec<String>,
    relinked: Vec<String>,
    /// The neighbours: the subjects re-run without being re-parsed or relinked.
    revalidated: Vec<String>,
    /// A finding the change gives (`true`) or takes (`false`): its path, its rule and a part of
    /// its message.
    shows: (&'static str, &'static str, &'static str, bool),
}

fn owned(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

/// The paths of the clues `C-<n>` of the `lore/` fixture.
fn clues(numbers: &[u32]) -> Vec<String> {
    numbers
        .iter()
        .map(|n| format!("lore/clues/C-{n}.md"))
        .collect()
}

/// The paths of the truths `T-<n>` of the `lore/` fixture, not their copies.
fn truths(numbers: &[u32]) -> Vec<String> {
    numbers
        .iter()
        .map(|n| format!("lore/truths/T-{n}.md"))
        .collect()
}

/// `rel` of the fixture folder `root` with `from` replaced by `to`, as a one-file plan.
fn replaced(
    root: &Path,
    rel: &str,
    from: &str,
    to: &str,
) -> Result<Plan, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(root.join(rel))?;
    if !text.contains(from) {
        return Err(format!("{rel} has no `{from}`").into());
    }
    Ok(vec![(rel.to_owned(), Some(text.replace(from, to)))])
}

const LOW_F04: &str = "world/synthetic-low/f04-synthetic.md";
const HIGH_F04: &str = "world/synthetic-high/f04-synthetic.md";
const HALL: &str = "world/synthetic-low/f04-synthetic/synthetic-hall.md";
const NOOK: &str = "world/synthetic-low/f04-synthetic/synthetic-hall/synthetic-nook.md";
const SETTINGS: &str = ".theos/settings.toml";

fn band_floors(root: &Path) -> Result<Plan, Box<dyn std::error::Error>> {
    replaced(
        root,
        "world/synthetic-low.md",
        "floors: [1, 20]",
        "floors: [10, 20]",
    )
}

fn clue_moves_to(root: &Path) -> Result<Plan, Box<dyn std::error::Error>> {
    replaced(
        root,
        "lore/clues/C-956.md",
        "moves_to: suspected",
        "moves_to: hinted",
    )
}

fn truth_target(root: &Path) -> Result<Plan, Box<dyn std::error::Error>> {
    replaced(
        root,
        "lore/truths/T-950.md",
        "target: confirmed",
        "target: suspected",
    )
}

fn note_removed(root: &Path) -> Result<Plan, Box<dyn std::error::Error>> {
    let rel = "concepts/synthetic-alpha.md";
    let text = std::fs::read_to_string(root.join(rel))?;
    let at = text.find("> [!note] n2").ok_or("n2 not in the fixture")?;
    Ok(vec![(
        rel.to_owned(),
        Some(format!("{}\n", text[..at].trim_end())),
    )])
}

fn floor_moved(root: &Path) -> Result<Plan, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(root.join(LOW_F04))?;
    let moved = text.replace("parent: synthetic-low", "parent: synthetic-high");
    if moved == text {
        return Err("f04-synthetic has no parent synthetic-low".into());
    }
    Ok(vec![
        (LOW_F04.to_owned(), None),
        (HIGH_F04.to_owned(), Some(moved)),
    ])
}

fn word_added(root: &Path) -> Result<Plan, Box<dyn std::error::Error>> {
    replaced(
        root,
        SETTINGS,
        "words = [\"ledger\"]",
        "words = [\"ledger\", \"hum\"]",
    )
}

/// The subjects of the `lore/` fixture that read `f04-synthetic`, besides the hall on it. The
/// floor gives the floor of synthetic-hall, where most clues are: every clue and truth whose
/// ledger or order checks find a floor through the hall reads it, also the two copies of the
/// duplicated truths T-954 and T-955. The scripts read the hall only.
fn reading_f04() -> Vec<String> {
    [
        clues(&[
            950, 951, 955, 956, 957, 958, 960, 961, 962, 963, 968, 969, 970, 971, 972, 980, 981,
            992, 993, 994,
        ]),
        truths(&[950, 952, 953, 954, 955, 956, 957, 958, 963, 964]),
        owned(&[
            "lore/truths/synthetic-copy/T-954.md",
            "lore/truths/synthetic-copy/T-955.md",
        ]),
    ]
    .concat()
}

/// The six changes of spec §5.18, each with the lists worked out from the reads of the fixture's
/// subjects (`deps` of a cold build).
fn cases() -> Vec<Case> {
    vec![
        // The band is read (`rec:synthetic-low`) by its floors, through `world/floor-number`
        // and the parent-cycle walk, and by the locations whose parent chain passes it.
        Case {
            what: "a band's floors",
            fixture: "world",
            plan: band_floors,
            reparsed: owned(&["world/synthetic-low.md"]),
            removed: Vec::new(),
            relinked: Vec::new(),
            revalidated: owned(&[LOW_F04, HALL, NOOK, "world/synthetic-low/f25-synthetic.md"]),
            shows: (LOW_F04, "world/floor-number", "", true),
        },
        // C-956 is read by its truth T-953 (its ledger and three-clue) and by the script whose
        // beat 3 reveals it; `in:T-953` is read by T-953 only, `in:synthetic-hall` by nobody.
        Case {
            what: "a clue's moves_to",
            fixture: "lore",
            plan: clue_moves_to,
            reparsed: clues(&[956]),
            removed: Vec::new(),
            relinked: Vec::new(),
            revalidated: [truths(&[953]), owned(&["story/scripts/S-951.md"])].concat(),
            shows: ("story/scripts/S-951.md", "lore/no-overshoot", "", false),
        },
        // T-950 is read by its five clues and by S-950, whose beat reveals C-950. Its
        // `requires: [T-957]` makes `in:T-957` change too, which T-957 reads for its ledger and
        // its clues C-970 to C-972 read for reveal-order.
        Case {
            what: "a truth's target",
            fixture: "lore",
            plan: truth_target,
            reparsed: truths(&[950]),
            removed: Vec::new(),
            relinked: Vec::new(),
            revalidated: [
                clues(&[950, 951, 952, 953, 954, 970, 971, 972]),
                truths(&[957]),
                owned(&["story/scripts/S-950.md"]),
            ]
            .concat(),
            shows: ("lore/clues/C-952.md", "lore/no-overshoot", "", true),
        },
        // The concept is read by both of its versions, for `version/addresses`; nothing reads
        // the links that name its targets.
        Case {
            what: "a note removed from a concept",
            fixture: "versions",
            plan: note_removed,
            reparsed: owned(&["concepts/synthetic-alpha.md"]),
            removed: Vec::new(),
            relinked: Vec::new(),
            revalidated: owned(&[
                "concepts/synthetic-alpha/synthetic-alpha-v1.md",
                "concepts/synthetic-alpha/synthetic-alpha-v2.md",
            ]),
            shows: (
                "concepts/synthetic-alpha/synthetic-alpha-v2.md",
                "version/addresses",
                "",
                true,
            ),
        },
        // The hall links to the floor, whose path changed, so it is relinked, not revalidated.
        Case {
            what: "a floor moved to another band",
            fixture: "lore",
            plan: floor_moved,
            reparsed: owned(&[HIGH_F04]),
            removed: owned(&[LOW_F04]),
            relinked: owned(&[HALL]),
            revalidated: reading_f04(),
            shows: (HALL, "index/placement", "", true),
        },
        // The settings file is a subject of its own and is listed with its readers: the records
        // with in-world text.
        Case {
            what: "a word added to the settings",
            fixture: "lore",
            plan: word_added,
            reparsed: Vec::new(),
            removed: Vec::new(),
            relinked: Vec::new(),
            revalidated: owned(&[
                SETTINGS,
                "dialogue/synthetic-vocab-dialogue.md",
                "lore/beliefs/B-950.md",
                "lore/beliefs/B-951.md",
                "lore/topics/synthetic-vocab-ok-in-world.md",
                "lore/topics/synthetic-vocab-voice-in-world.md",
            ]),
            shows: (
                "lore/topics/synthetic-vocab-voice-in-world.md",
                "lore/inworld-vocab",
                "`hum`",
                true,
            ),
        },
    ]
}

/// A fresh in-memory index over `project`, synced.
fn indexed(project: &TempProject) -> Result<Index, Box<dyn std::error::Error>> {
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    index.sync()?;
    Ok(index)
}

/// Writes the plan to the folder of `project`.
fn write(project: &TempProject, plan: &Plan) -> TestResult {
    for (path, text) in plan {
        match text {
            Some(text) => project.write(path, text)?,
            None => project.remove(path)?,
        }
    }
    Ok(())
}

/// Whether `findings` hold one of `rule` on `path` whose message holds `part`.
fn shows(findings: &[Finding], path: &str, rule: &str, part: &str) -> bool {
    findings
        .iter()
        .any(|f| f.path == path && f.rule == rule && f.message.contains(part))
}

/// The lists of `update` against those of `case`; `read` differs by route and is left out.
fn same_lists(case: &Case, update: &Update, route: &str) -> TestResult {
    let lists = [
        ("reparsed", &update.reparsed, &case.reparsed),
        ("removed", &update.removed, &case.removed),
        ("relinked", &update.relinked, &case.relinked),
        ("revalidated", &update.revalidated, &case.revalidated),
    ];
    for (name, got, want) in lists {
        let mut want = want.clone();
        want.sort();
        if *got != want {
            return Err(format!(
                "{} through {route}: {name} is\n{got:#?}\nnot\n{want:#?}",
                case.what
            )
            .into());
        }
    }
    Ok(())
}

/// Fails with the lines that differ when the index does not equal a cold build of its folder.
fn like_a_cold_build(index: &Index, project: &TempProject, what: &str) -> TestResult {
    same_dump(
        &format!("{what}: after the change"),
        &index.dump()?,
        &indexed(project)?.dump()?,
    )
}

/// The change through `update`: its lists, and an index equal to a cold build.
fn through_update(case: &Case, plan: &Plan) -> TestResult {
    let project = TempProject::copy_of(case.fixture)?;
    let mut index = indexed(&project)?;
    write(&project, plan)?;
    let paths: Vec<&str> = plan.iter().map(|(path, _)| path.as_str()).collect();
    let update = index.update(&paths)?;
    same_lists(case, &update, "update")?;
    like_a_cold_build(&index, &project, case.what)
}

/// The change through `preview`, then `apply`: the preview changes no row and returns the
/// findings a cold build has on every path it covers, the neighbours among them; the apply makes
/// the same update and an index equal to a cold build.
fn through_preview(case: &Case, plan: &Plan) -> TestResult {
    let project = TempProject::copy_of(case.fixture)?;
    let mut index = indexed(&project)?;
    let (path, rule, part, given) = case.shows;
    let before = index.findings_for(path)?;
    assert_eq!(
        shows(&before, path, rule, part),
        !given,
        "{}: before the change",
        case.what
    );
    write(&project, plan)?;
    let changes: Vec<Change> = plan
        .iter()
        .map(|(path, text)| match text {
            Some(text) => Change::text(path.clone(), text.clone()),
            None => Change::Removed { path: path.clone() },
        })
        .collect();

    let dump = index.dump()?;
    let preview = index.preview(&changes)?;
    same_dump(
        &format!("{}: the preview changed the cache", case.what),
        &index.dump()?,
        &dump,
    )?;
    same_lists(case, &preview.update, "preview")?;
    let cold = indexed(&project)?;
    let covered: BTreeSet<&String> = preview
        .update
        .reparsed
        .iter()
        .chain(&preview.update.relinked)
        .chain(&preview.update.revalidated)
        .chain(
            plan.iter()
                .filter(|(_, text)| text.is_some())
                .map(|(path, _)| path),
        )
        .collect();
    let mut want = Vec::new();
    for path in covered {
        want.extend(cold.findings_for(path)?);
    }
    assert_eq!(
        preview.findings, want,
        "{}: the preview's findings",
        case.what
    );
    assert_eq!(
        shows(&preview.findings, path, rule, part),
        given,
        "{}: the preview",
        case.what
    );

    let update = index.apply(&changes)?;
    assert_eq!(update, preview.update, "{}: apply and preview", case.what);
    like_a_cold_build(&index, &project, case.what)
}

// AC: CE-14.3
#[test]
fn validating_one_record_validates_the_neighbours_it_affects() -> TestResult {
    for case in cases() {
        let plan = (case.plan)(&fixture(case.fixture))?;
        through_update(&case, &plan)?;
        through_preview(&case, &plan)?;
    }
    Ok(())
}
