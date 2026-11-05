//! Link resolution over the `links/` fixture project: slots, kinds, dangling links, duplicate
//! ids and placement.

use theos_content::{Change, Finding, Index, Project, parts};

use crate::common::{TempProject, expected, fixture, keys};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SPEAKER: &str = "people/synthetic-folk/synthetic-guild/synthetic-speaker.md";
const WRONG: &str = "people/synthetic-folk/synthetic-wrong.md";
const SCRIPT: &str = "story/scripts/S-930.md";
const CONCEPT: &str = "concepts/synthetic-concept.md";

fn indexed() -> Result<Index, Box<dyn std::error::Error>> {
    let project = Project::open(&fixture("links"))?;
    let mut index = Index::in_memory(&project, &parts::builtin()?)?;
    index.sync()?;
    Ok(index)
}

/// Line and column of `link` inside the first occurrence of `context` in the fixture file `path`.
fn pos(
    path: &str,
    context: &str,
    link: &str,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(fixture("links").join(path))?;
    let start = text.find(context).ok_or("context not found")?
        + context.find(link).ok_or("link not in context")?;
    let before = &text[..start];
    let row = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count())
        + 1;
    Ok((row, column))
}

fn at(
    findings: &[Finding],
    path: &str,
    rule: &str,
    (line, column): (usize, usize),
) -> Option<Finding> {
    findings
        .iter()
        .find(|f| {
            f.path == path
                && f.rule == rule
                && f.position
                    .is_some_and(|p| p.line == line && p.column == column)
        })
        .cloned()
}

// AC: CE-11.2
#[test]
fn wrong_kinds_are_errors_in_every_typed_slot() -> TestResult {
    let index = indexed()?;
    let findings = index.findings()?;
    assert_eq!(
        keys(&findings),
        expected(&fixture("links").join("expected.toml"))?
    );

    wrong_slots(&findings)?;
    right_kinds_are_clean(&findings)?;
    Ok(())
}

/// Every wrong-kind slot has a `link/kind` at the position of its link.
fn wrong_slots(findings: &[Finding]) -> TestResult {
    let slots = [
        (
            WRONG,
            "home: synthetic-speaker",
            "synthetic-speaker",
            "`home` must",
        ),
        (
            WRONG,
            "unaware_of: [B-930]",
            "B-930",
            "`lens.unaware_of` must",
        ),
        (
            SCRIPT,
            "with:: [[T-930]]",
            "[[T-930]]",
            "the `with::` link must",
        ),
        (
            SCRIPT,
            "at:: [[synthetic-speaker]]",
            "[[synthetic-speaker]]",
            "the `at::` link must",
        ),
        (
            SCRIPT,
            "says:: [[T-930]]",
            "[[T-930]]",
            "the `says::` link must",
        ),
        (
            SCRIPT,
            "reveals:: [[B-930]]",
            "[[B-930]]",
            "the `reveals::` link must",
        ),
        (
            SCRIPT,
            "[!voice] [[T-930]] · [[T-931]]",
            "[[T-930]]",
            "a voice speaker must",
        ),
        (
            SCRIPT,
            "[!voice] [[T-930]] · [[T-931]]",
            "[[T-931]]",
            "a voice belief must",
        ),
        (
            CONCEPT,
            "n2 · [[T-930]]",
            "[[T-930]]",
            "a note version must",
        ),
        (
            CONCEPT,
            "resolved by [[synthetic-speaker]]",
            "[[synthetic-speaker]]",
            "a note's resolved-by link must",
        ),
    ];
    for (path, context, link, message) in slots {
        let found = at(findings, path, "link/kind", pos(path, context, link)?)
            .ok_or_else(|| format!("no link/kind at {path} {context}"))?;
        assert!(found.message.starts_with(message), "{}", found.message);
    }
    let home = at(
        findings,
        WRONG,
        "link/kind",
        pos(WRONG, "home: synthetic-speaker", "synthetic-speaker")?,
    )
    .ok_or("no home finding")?;
    assert_eq!(
        home.message,
        "`home` must link to a location or floor; `synthetic-speaker` is an npc"
    );

    Ok(())
}

/// Links of the right kind report nothing.
fn right_kinds_are_clean(findings: &[Finding]) -> TestResult {
    for (path, context, link) in [
        (SCRIPT, "at:: [[synthetic-place]]", "[[synthetic-place]]"),
        (
            SCRIPT,
            "with:: [[synthetic-speaker]]",
            "[[synthetic-speaker]]",
        ),
        (SCRIPT, "says:: [[B-930]]", "[[B-930]]"),
        (SCRIPT, "reveals:: [[C-930]]", "[[C-930]]"),
        (
            SCRIPT,
            "[!voice] [[synthetic-speaker]] · [[B-930]]",
            "[[synthetic-speaker]]",
        ),
        (
            SCRIPT,
            "[!voice] [[synthetic-speaker]] · [[B-930]]",
            "[[B-930]]",
        ),
        (
            CONCEPT,
            "n1 · [[synthetic-concept-v1]]",
            "[[synthetic-concept-v1]]",
        ),
    ] {
        let place = pos(path, context, link)?;
        assert!(
            !findings
                .iter()
                .any(|f| f.path == path && f.position.is_some_and(|p| (p.line, p.column) == place)),
            "{path} {context}"
        );
    }
    for path in [
        "world/tower.md",
        "world/synthetic-band.md",
        "world/synthetic-band/f01-synthetic.md",
        "world/synthetic-band/f01-synthetic/synthetic-place.md",
        "people/synthetic-folk.md",
        "people/synthetic-folk/synthetic-guild.md",
        SPEAKER,
        "lore/topics/synthetic-subject-explaining.md",
        "lore/topics/synthetic-subject-in-world.md",
        "lore/truths/T-930.md",
        "canon/formalism/E930.md",
        "lore/beliefs/B-930.md",
        "concepts/synthetic-concept/synthetic-concept-v1.md",
    ] {
        let own: Vec<_> = findings
            .iter()
            .filter(|f| f.path == path && f.rule.starts_with("link/"))
            .collect();
        assert!(own.is_empty(), "{path}: {own:?}");
    }
    Ok(())
}

// AC: CE-11.2
#[test]
fn narrowed_targets_broken_targets_and_duplicates() -> TestResult {
    let index = indexed()?;
    let findings = index.findings()?;
    let narrowed = |path: &str| -> Vec<String> {
        findings
            .iter()
            .filter(|f| f.path == path && f.rule == "link/kind")
            .map(|f| f.message.clone())
            .collect()
    };
    assert_eq!(
        narrowed("lore/truths/T-931.md"),
        [
            "`requires` must link to a truth; `B-930` is a belief",
            "`topic` must link to an explaining topic; `synthetic-subject-in-world` is in-world",
        ]
    );
    assert_eq!(
        narrowed("lore/beliefs/B-931.md"),
        [
            "`held_by` must link to a people, faction or npc; `T-930` is a truth",
            "`topic` must link to an in-world topic; `synthetic-subject-explaining` is explaining",
        ]
    );
    assert_eq!(
        narrowed("lore/topics/synthetic-other-in-world.md"),
        [
            "`counterpart` must link to an explaining topic; `synthetic-subject-in-world` is in-world"
        ]
    );
    // A target with broken front matter has no known kind: no finding about the link.
    assert_eq!(keys(&index.findings_for("things/synthetic-target.md")?), []);
    assert_eq!(index.record("synthetic-broken")?[0].kind, None);
    // A link to a duplicated id resolves and passes the kind check.
    let twin = index.backlinks("synthetic-twin")?;
    assert_eq!(twin.len(), 1);
    assert_eq!(twin[0].source_path, CONCEPT);
    assert!(
        !findings
            .iter()
            .any(|f| f.path == CONCEPT && f.message.contains("synthetic-twin"))
    );
    Ok(())
}

// AC: CE-12.3
#[test]
fn duplicates_dangling_and_file_names_are_findings_with_paths() -> TestResult {
    let index = indexed()?;
    let findings = index.findings()?;
    for (path, other) in [
        ("things/synthetic-twin.md", "dialogue/synthetic-twin.md"),
        ("dialogue/synthetic-twin.md", "things/synthetic-twin.md"),
    ] {
        let found = at(
            &findings,
            path,
            "index/duplicate-id",
            pos(path, "id: synthetic-twin", "synthetic-twin")?,
        )
        .ok_or_else(|| format!("no duplicate-id on {path}"))?;
        assert!(found.message.contains(other), "{}", found.message);
        assert!(
            !found.message.contains(&format!("`{path}`")),
            "{}",
            found.message
        );
    }
    let dangling = [
        (
            "lore/clues/C-930.md",
            "after: [C-999]",
            "C-999",
            "`C-999` names no record",
        ),
        (
            WRONG,
            "[[synthetic-nowhere]]",
            "[[synthetic-nowhere]]",
            "`synthetic-nowhere` names no record",
        ),
        (
            "things/synthetic-refs.md",
            "[[S-930#9]]",
            "[[S-930#9]]",
            "`S-930` has no beat 9",
        ),
        (
            "things/synthetic-refs.md",
            "[[B-930#1]]",
            "[[B-930#1]]",
            "`B-930` is a belief; only scripts have beats",
        ),
        (
            "things/synthetic-refs.md",
            "[[todo]]",
            "[[todo]]",
            "`todo` names no record",
        ),
    ];
    for (path, context, link, message) in dangling {
        let found = at(&findings, path, "link/dangling", pos(path, context, link)?)
            .ok_or_else(|| format!("no link/dangling at {path} {context}"))?;
        assert_eq!(found.message, message);
    }
    let name = "things/synthetic-misnamed.md";
    assert!(
        at(
            &findings,
            name,
            "index/file-name",
            pos(name, "id: synthetic-renamed", "synthetic-renamed")?
        )
        .is_some()
    );
    // Right references and self links resolve.
    let refs = index.findings_for("things/synthetic-refs.md")?;
    assert_eq!(refs.len(), 3);
    Ok(())
}

/// A record of the unknown kind `synthetic-unknown`.
fn unknown_kind(id: &str) -> String {
    format!("---\n# synthetic fixture (P03), not canon\nid: {id}\nkind: synthetic-unknown\n---\n")
}

#[test]
fn a_beat_reference_to_a_record_of_no_known_kind_is_left_to_its_target() -> TestResult {
    let project = TempProject::new()?;
    let mut index = Index::in_memory(&project.project()?, &parts::builtin()?)?;
    let source = "things/synthetic-beat-refs.md";
    index.apply(&[
        Change::text(
            source,
            "---\n# synthetic fixture (P03), not canon\nid: synthetic-beat-refs\nkind: thing\n\
             title: Synthetic beat references\ntype: synthetic\n---\n\
             No known kind: [[S-931#1]]\n\nOne known kind among two: [[B-932#1]]\n",
        ),
        Change::text("story/scripts/S-931.md", unknown_kind("S-931")),
        Change::text("drafts/B-932.md", unknown_kind("B-932")),
        Change::text(
            "lore/beliefs/B-932.md",
            "---\n# synthetic fixture (P03), not canon\nid: B-932\nkind: belief\n\
             title: Synthetic belief C\n---\nSynthetic stance C.\n",
        ),
    ])?;
    let dangling: Vec<String> = index
        .findings_for(source)?
        .into_iter()
        .filter(|f| f.rule == "link/dangling")
        .map(|f| f.message)
        .collect();
    assert_eq!(dangling, ["`B-932` is a belief; only scripts have beats"]);
    let unknown = index.findings_for("story/scripts/S-931.md")?;
    assert!(
        unknown.iter().any(|f| f.rule == "schema/kind"),
        "{unknown:?}"
    );
    Ok(())
}

#[test]
fn backlinks_and_children() -> TestResult {
    let index = indexed()?;
    let paths = |list: Vec<theos_content::LinkRef>| -> Vec<String> {
        list.into_iter().map(|link| link.source_path).collect()
    };
    assert_eq!(
        paths(index.children("synthetic-folk")?),
        ["people/synthetic-folk/synthetic-guild.md", WRONG,]
    );
    assert_eq!(
        paths(index.children("synthetic-guild")?),
        [SPEAKER, "people/synthetic-lost.md"]
    );
    let story = index.backlinks("S-930")?;
    assert_eq!(
        story
            .iter()
            .map(|l| (l.source_path.as_str(), l.fragment.as_deref()))
            .collect::<Vec<_>>(),
        [
            ("things/synthetic-refs.md", Some("1")),
            ("things/synthetic-refs.md", Some("9"))
        ]
    );
    assert_eq!(story[0].source_id, "synthetic-refs");
    assert_eq!(story[0].slot, theos_content::Slot::Mention);
    assert!(story[0].span.start < story[1].span.start);
    let from = index.links_from("lore/truths/T-930.md")?;
    assert_eq!(
        from.iter().map(|l| l.target.as_str()).collect::<Vec<_>>(),
        ["synthetic-subject-explaining", "E930"]
    );
    Ok(())
}

#[test]
fn a_removal_relinks_its_backlinks_and_clears_duplicates() -> TestResult {
    let mut index = indexed()?;
    assert_eq!(index.sync()?, theos_content::Update::default());
    let update = index.apply(&[theos_content::Change::Removed {
        path: "dialogue/synthetic-twin.md".to_owned(),
    }])?;
    assert_eq!(update.removed, ["dialogue/synthetic-twin.md"]);
    assert_eq!(update.reparsed.len(), 0);
    assert_eq!(update.relinked, ["concepts/synthetic-concept.md"]);
    let left = index.findings_for("things/synthetic-twin.md")?;
    assert!(
        left.iter().all(|f| f.rule != "index/duplicate-id"),
        "{left:?}"
    );
    Ok(())
}
