//! CE-12.1: a cold index of 500 records takes under a second.

use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

use theos_content::{Index, formalism, parts};

use crate::common::TempProject;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Records per kind; 20 kinds make 500 records.
const PER_KIND: usize = 25;

/// The truths that can be revealed: each has three clues that confirm it, on rising floors, and
/// requires the one before. The others have the target `never` (the three-clue rule leaves them
/// alone) and require the last revealed one. The 25th clue is a fourth confirming clue of the
/// last revealed truth.
const REVEALED: usize = 8;

/// The longest a cold index of the project may take.
const LIMIT: Duration = Duration::from_secs(1);

/// A digest of 64 hex digits, for the outputs of versions.
const HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// A body of about a kilobyte: neutral sentences, three wikilinks, one voice block and `extra`.
fn body(extra: &str) -> String {
    let sentence = "Synthetic sentence about a synthetic subject, written for the speed test.";
    let mut text = format!("{sentence} See [[tower-01]], [[thing-01]] and [[people-01]].\n\n");
    text.push_str("> [!voice] [[people-01]]\n> Synthetic spoken words.\n\n");
    text.push_str(extra);
    while text.len() < 1000 {
        let _ = write!(text, "{sentence} ");
    }
    text.push('\n');
    text
}

/// The output line of a version: an image of the given folder.
fn outputs(dir: &str, n: usize) -> String {
    format!(
        "generator: synthetic@1.0\nrecipe: synthetic-recipe\noutputs:\n  image: {dir}/v{n:02}.png@sha256:{HASH}\nrating: unrated\n"
    )
}

/// Writes records of the speed test into a project.
struct Writer<'a> {
    project: &'a TempProject,
}

impl Writer<'_> {
    /// One record: front matter with the synthetic marker, then `text`.
    fn put(
        &self,
        path: &str,
        kind: &str,
        id: &str,
        fields: &str,
        text: &str,
    ) -> std::io::Result<()> {
        // The title spells the id with spaces: an id in a belief title is a developer word.
        let title = id.replace('-', " ");
        let record = format!(
            "---\n# synthetic fixture (P03), not canon\nid: {id}\nkind: {kind}\ntitle: Synthetic {title}\n{fields}---\n{text}"
        );
        self.project.write(path, &record)
    }

    /// The places and the people: tower, band, floor, location, people, faction, npc, thing.
    fn world(&self, n: usize) -> std::io::Result<()> {
        let text = body("");
        let (band, floor, place) = (
            format!("band-{n:02}"),
            format!("f{n:02}-x"),
            format!("place-{n:02}"),
        );
        let (people, faction) = (format!("people-{n:02}"), format!("faction-{n:02}"));
        self.put(
            &format!("world/tower-{n:02}.md"),
            "tower",
            &format!("tower-{n:02}"),
            "",
            &text,
        )?;
        let fields = format!("parent: tower-{n:02}\nfloors: [{n}, {n}]\n");
        self.put(&format!("world/{band}.md"), "band", &band, &fields, &text)?;
        let fields = format!("status: draft\nparent: {band}\nnumber: {n}\n");
        self.put(
            &format!("world/{band}/{floor}.md"),
            "floor",
            &floor,
            &fields,
            &text,
        )?;
        let fields = format!("parent: {floor}\ntype: synthetic\n");
        self.put(
            &format!("world/{band}/{floor}/{place}.md"),
            "location",
            &place,
            &fields,
            &text,
        )?;
        let fields =
            format!("lives_in: [{band}]\nlens:\n  stance: synthetic stance A\n  tier: T2\n");
        self.put(
            &format!("people/{people}.md"),
            "people",
            &people,
            &fields,
            &text,
        )?;
        let fields = format!("parent: {people}\n");
        self.put(
            &format!("people/{people}/{faction}.md"),
            "faction",
            &faction,
            &fields,
            &text,
        )?;
        let fields = format!("parent: {faction}\nhome: {place}\nrole: synthetic\n");
        let npc = format!("npc-{n:02}");
        self.put(
            &format!("people/{people}/{faction}/{npc}.md"),
            "npc",
            &npc,
            &fields,
            &text,
        )?;
        let thing = format!("thing-{n:02}");
        self.put(
            &format!("things/{thing}.md"),
            "thing",
            &thing,
            "type: synthetic\n",
            &text,
        )
    }

    /// The lore: topic, truth, belief, clue. The equations are generated, see
    /// `write_formalism`.
    fn lore(&self, i: usize) -> std::io::Result<()> {
        let (n, num) = (i + 1, 930 + i);
        let text = body("");
        let topic = format!("topic-{n:02}-explaining");
        let fields =
            format!("status: draft\ntrack: explaining\nsource: https://example.invalid/t{n}\n");
        self.put(
            &format!("lore/topics/{topic}.md"),
            "topic",
            &topic,
            &fields,
            &text,
        )?;
        // The revealed truths require the one before; the others require the last revealed one.
        let requires = match i {
            0 => String::new(),
            i if i >= REVEALED => format!("requires: [T-{}]\n", 930 + REVEALED - 1),
            _ => format!("requires: [T-{}]\n", num - 1),
        };
        let target = if i < REVEALED { "confirmed" } else { "never" };
        let fields =
            format!("status: draft\ntarget: {target}\n{requires}topic: {topic}\ncites: [E{num}]\n");
        self.put(
            &format!("lore/truths/T-{num}.md"),
            "truth",
            &format!("T-{num}"),
            &fields,
            &text,
        )?;
        let fields = format!(
            "status: draft\nheld_by: [people-{n:02}]\ndistorts: T-{num}\naccuracy: partial\n"
        );
        self.put(
            &format!("lore/beliefs/B-{num}.md"),
            "belief",
            &format!("B-{num}"),
            &fields,
            &text,
        )?;
        let truth = 930 + (i / 3).min(REVEALED - 1);
        let fields = format!(
            "status: draft\nreveals: T-{truth}\nmoves_to: confirmed\nwhere: [band-{n:02}]\ngate: T1\nvector: synthetic\n"
        );
        self.put(
            &format!("lore/clues/C-{num}.md"),
            "clue",
            &format!("C-{num}"),
            &fields,
            &text,
        )
    }

    /// The scripts: script (with two beats), dialogue, template.
    fn stories(&self, i: usize) -> std::io::Result<()> {
        let (n, num) = (i + 1, 930 + i);
        let beats = format!(
            "## Beat 1 — Synthetic\nat:: [[place-{n:02}]]\nwith:: [[npc-{n:02}]]\n\n## Beat 2 — Synthetic\nat:: [[place-{n:02}]]\nreveals:: [[C-{num}]]\n\n"
        );
        let fields = format!("status: draft\nwhere: [band-{n:02}]\n");
        self.put(
            &format!("story/scripts/S-{num}.md"),
            "script",
            &format!("S-{num}"),
            &fields,
            &body(&beats),
        )?;
        let text = body("");
        let dialogue = format!("dialogue-{n:02}");
        let fields = format!("speaker: npc-{n:02}\n");
        self.put(
            &format!("dialogue/{dialogue}.md"),
            "dialogue",
            &dialogue,
            &fields,
            &text,
        )?;
        let template = format!("template-{n:02}");
        self.put(
            &format!("templates/{template}.md"),
            "template",
            &template,
            "purpose: dialogue\n",
            &text,
        )
    }

    /// The pictures: concept, concept version, map, map version.
    fn pictures(&self, n: usize) -> std::io::Result<()> {
        let text = body("");
        let concept = format!("concept-{n:02}");
        let fields = format!("for: [tower-{n:02}]\n");
        self.put(
            &format!("concepts/{concept}.md"),
            "concept",
            &concept,
            &fields,
            &text,
        )?;
        let version = format!("{concept}-v1");
        let fields = format!("concept: {concept}\n{}", outputs("concepts", n));
        self.put(
            &format!("concepts/{concept}/{version}.md"),
            "concept-version",
            &version,
            &fields,
            &text,
        )?;
        let map = format!("map-{n:02}");
        let fields = format!("for: [band-{n:02}]\ntype: synthetic\n");
        self.put(&format!("maps/{map}.md"), "map", &map, &fields, &text)?;
        let version = format!("{map}-v1");
        let fields = format!("map: {map}\n{}", outputs("maps", n));
        self.put(
            &format!("maps/{map}/{version}.md"),
            "map-version",
            &version,
            &fields,
            &text,
        )
    }
}

/// Writes the Shared Formalism source of the 25 equations and their generated records, so that
/// no equation record is stray, edited or missing. Each text has the size of the other bodies.
fn write_formalism(project: &TempProject) -> Result<(), Box<dyn std::error::Error>> {
    let page = "https://example.invalid/speed";
    let equations: Vec<formalism::Equation> = (0..PER_KIND)
        .map(|i| {
            let id = format!("E{}", 930 + i);
            formalism::Equation {
                title: format!("Synthetic {id}"),
                anchor: format!("{page}#{}", id.to_lowercase()),
                text: body(""),
                id,
            }
        })
        .collect();
    let listed: Vec<_> = equations
        .iter()
        .map(|e| serde_json::json!({"id": e.id, "title": e.title, "anchor": e.anchor, "text": e.text}))
        .collect();
    let source = serde_json::json!({"format": 1, "page": page, "equations": listed});
    project.write(formalism::SOURCE, &serde_json::to_string_pretty(&source)?)?;
    let source = formalism::Source {
        page: page.to_owned(),
        equations,
    };
    for (path, text) in formalism::render(&source)? {
        project.write(&path, &text)?;
    }
    Ok(())
}

/// Writes the 500 records of the speed test into `project`: 475 written here, 25 generated.
fn write_records(project: &TempProject) -> Result<(), Box<dyn std::error::Error>> {
    let writer = Writer { project };
    for i in 0..PER_KIND {
        writer.world(i + 1)?;
        writer.lore(i)?;
        writer.stories(i)?;
        writer.pictures(i + 1)?;
    }
    write_formalism(project)
}

/// Deletes the cache and times one cold `Index::open`; returns the time and the record count.
fn cold_open(
    root: &Path,
    project: &TempProject,
) -> Result<(Duration, usize), Box<dyn std::error::Error>> {
    let cache = root.join(".theos");
    if cache.exists() {
        std::fs::remove_dir_all(&cache)?;
    }
    let project = project.project()?;
    let registry = parts::builtin()?;
    let start = Instant::now();
    let (index, _) = Index::open(&project, &registry)?;
    let took = start.elapsed();
    let findings = index.findings()?;
    assert!(
        findings.is_empty(),
        "the records are valid: {:?}",
        findings.iter().take(5).collect::<Vec<_>>()
    );
    Ok((took, index.records()?.len()))
}

// AC: CE-12.1
#[test]
fn a_full_index_of_500_records_takes_under_a_second() -> TestResult {
    let project = TempProject::new()?;
    write_records(&project)?;
    let mut times = Vec::new();
    for _ in 0..3 {
        let (took, records) = cold_open(project.root(), &project)?;
        assert_eq!(records, 500, "records in the index");
        times.push(took);
    }
    println!("cold index of 500 records: {times:?}");
    let fastest = times.iter().min().copied().unwrap_or(Duration::MAX);
    assert!(
        fastest < LIMIT,
        "fastest of three cold opens took {fastest:?}"
    );
    Ok(())
}
