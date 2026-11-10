//! The lore step (spec §5.5): checks across records read through a context that stores every
//! read, and a change re-runs exactly the subjects that read something it touched (CE-14.3).

use std::collections::BTreeSet;

use theos_content::lore::{Context, FactError, LoreRule, On, Subject};
use theos_content::rules::RuleClass;
use theos_content::{
    Change, Finding, Index, KindRegistry, Position, Severity, Span, Update, parts,
};

use crate::common::TempProject;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MARK: &str = "# synthetic fixture (P04), not canon\n";

const A: &str = "things/synthetic-a.md";
const B: &str = "things/synthetic-b.md";
const B_RENAMED: &str = "things/synthetic-b2.md";
const C: &str = "things/synthetic-c.md";
const D: &str = "things/synthetic-d.md";

/// `test/probe`: a thing reads the records with the id its `type` names, and is reported when
/// no record has it.
const PROBE: LoreRule = LoreRule {
    id: "test/probe",
    severity: Severity::Warning,
    class: RuleClass::Fixed,
    on: On::Kinds(&["thing"]),
    run: probe,
};

fn probe(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(record) = subject else {
        return Ok(Vec::new());
    };
    let Some(value) = record.text("type") else {
        return Ok(Vec::new());
    };
    let id = value.text.clone().unwrap_or_default();
    if !ctx.records(&id)?.is_empty() {
        return Ok(Vec::new());
    }
    Ok(vec![subject.finding(
        &PROBE,
        value.span,
        value.position,
        format!("no record has the id `{id}`"),
    )])
}

/// The builtin kinds and [`PROBE`].
fn registry() -> Result<KindRegistry, Box<dyn std::error::Error>> {
    let mut registry = parts::builtin()?;
    registry.register_rule("test", PROBE)?;
    Ok(registry)
}

/// `test/echo`: every record reads the records its links name and the links that name it, and
/// reports all it read, so that a read the lore step failed to redo shows in the dump.
const ECHO: LoreRule = LoreRule {
    id: "test/echo",
    severity: Severity::Warning,
    class: RuleClass::Fixed,
    on: On::AnyRecord,
    run: echo,
};

fn echo(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    let Subject::Record(record) = subject else {
        return Ok(Vec::new());
    };
    let mut said = Vec::new();
    let targets: BTreeSet<&str> = record.links.iter().map(|l| l.target.as_str()).collect();
    for target in targets {
        let found: Vec<String> = ctx
            .records(target)?
            .iter()
            .map(|r| format!("{} {:?} {} {}", r.path, r.kind, r.hash, r.links.len()))
            .collect();
        said.push(format!("{target}=[{}]", found.join(", ")));
    }
    let inbound: Vec<String> = ctx
        .linking_to(&record.id)?
        .iter()
        .map(|l| {
            let at = l.position;
            format!(
                "{} {} {}:{}",
                l.source_path,
                l.slot.as_db(),
                at.line,
                at.column
            )
        })
        .collect();
    said.push(format!("in=[{}]", inbound.join(", ")));
    Ok(vec![subject.finding(
        &ECHO,
        Span::default(),
        Position { line: 1, column: 1 },
        said.join("; "),
    )])
}

/// The builtin kinds and [`ECHO`], for P03's history test.
pub fn echo_registry() -> Result<KindRegistry, Box<dyn std::error::Error>> {
    let mut registry = parts::builtin()?;
    registry.register_rule("test", ECHO)?;
    Ok(registry)
}

/// A thing whose `type` names the id it reads.
fn thing(id: &str, title: &str, reads: &str) -> String {
    format!(
        "---\n{MARK}id: {id}\nkind: thing\ntitle: {title}\ntype: {reads}\n---\nSynthetic text.\n"
    )
}

fn a() -> String {
    thing("synthetic-a", "Synthetic A", "synthetic-b")
}

fn b() -> String {
    thing("synthetic-b", "Synthetic B", "synthetic-a")
}

/// A project where A reads B, B reads A and C reads itself, indexed.
fn opened() -> Result<(TempProject, Index), Box<dyn std::error::Error>> {
    let project = TempProject::new()?;
    project.write(A, &a())?;
    project.write(B, &b())?;
    project.write(C, &thing("synthetic-c", "Synthetic C", "synthetic-c"))?;
    let mut index = Index::in_memory(&project.project()?, &registry()?)?;
    index.sync()?;
    Ok((project, index))
}

fn lists(
    read: &[&str],
    reparsed: &[&str],
    removed: &[&str],
    relinked: &[&str],
    revalidated: &[&str],
) -> Update {
    let own = |list: &[&str]| list.iter().map(|path| (*path).to_owned()).collect();
    Update {
        read: own(read),
        reparsed: own(reparsed),
        removed: own(removed),
        relinked: own(relinked),
        revalidated: own(revalidated),
    }
}

/// The paths with a `test/probe` finding, sorted.
fn probed(index: &Index) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(index
        .findings()?
        .into_iter()
        .filter(|f| f.rule == "test/probe")
        .map(|f| f.path)
        .collect())
}

/// The dump of a fresh index over the same folder.
fn fresh(project: &TempProject) -> Result<String, Box<dyn std::error::Error>> {
    fresh_with(project, &registry()?)
}

/// The dump of a fresh index with `registry` over the same folder.
fn fresh_with(
    project: &TempProject,
    registry: &KindRegistry,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut index = Index::in_memory(&project.project()?, registry)?;
    index.sync()?;
    Ok(index.dump()?)
}

/// The rows of the section `name` of a dump.
fn section(dump: &str, name: &str) -> Vec<String> {
    dump.lines()
        .skip_while(|line| *line != format!("[{name}]"))
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .map(str::to_owned)
        .collect()
}

/// The rows of `section` whose first column is `path`.
fn rows_of(dump: &str, name: &str, path: &str) -> Vec<String> {
    let prefix = format!("{path}\t");
    section(dump, name)
        .into_iter()
        .filter(|row| row.starts_with(&prefix))
        .collect()
}

// AC: CE-14.3
#[test]
fn a_change_revalidates_the_records_that_read_it() -> TestResult {
    let (project, mut index) = opened()?;
    assert_eq!(probed(&index)?, Vec::<String>::new());

    // Editing B's title re-runs A, which reads B: through preview, and then on disk.
    let retitled = thing("synthetic-b", "Synthetic B, retitled", "synthetic-a");
    project.write(B, &retitled)?;
    let before = index.dump()?;
    let preview = index.preview(&[Change::text(B, retitled)])?;
    assert_eq!(index.dump()?, before, "a preview changes no row");
    assert_eq!(preview.update, lists(&[], &[B], &[], &[], &[A]));
    assert_eq!(index.sync()?, lists(&[B], &[B], &[], &[], &[A]));
    let mut stored = index.findings_for(A)?;
    stored.extend(index.findings_for(B)?);
    assert_eq!(preview.findings, stored);
    assert_eq!(index.dump()?, fresh(&project)?);

    // A preview that removes B returns the finding A would get, and changes no row.
    let before = index.dump()?;
    let removal = index.preview(&[Change::Removed { path: B.to_owned() }])?;
    assert_eq!(removal.update, lists(&[], &[], &[B], &[], &[A]));
    let probes: Vec<&str> = removal
        .findings
        .iter()
        .filter(|f| f.rule == "test/probe")
        .map(|f| f.path.as_str())
        .collect();
    assert_eq!(probes, [A]);
    assert_eq!(index.dump()?, before);
    Ok(())
}

#[test]
fn a_renamed_or_removed_dependency_reruns_its_readers() -> TestResult {
    let (project, mut index) = opened()?;
    let b_renamed = thing("synthetic-b2", "Synthetic B", "synthetic-a");

    project.remove(B)?;
    project.write(B_RENAMED, &b_renamed)?;
    let update = index.sync()?;
    assert_eq!(update, lists(&[B_RENAMED], &[B_RENAMED], &[B], &[], &[A]));
    assert_eq!(probed(&index)?, [A]);
    assert_eq!(index.dump()?, fresh(&project)?);

    project.remove(B_RENAMED)?;
    project.write(B, &b())?;
    let update = index.sync()?;
    assert_eq!(update, lists(&[B], &[B], &[B_RENAMED], &[], &[A]));
    assert_eq!(probed(&index)?, Vec::<String>::new());
    assert_eq!(index.dump()?, fresh(&project)?);

    project.remove(B)?;
    assert_eq!(index.sync()?, lists(&[], &[], &[B], &[], &[A]));
    assert_eq!(probed(&index)?, [A]);
    assert_eq!(index.dump()?, fresh(&project)?);

    project.write(B, &b())?;
    assert_eq!(index.sync()?, lists(&[B], &[B], &[], &[], &[A]));
    assert_eq!(probed(&index)?, Vec::<String>::new());
    assert_eq!(index.dump()?, fresh(&project)?);
    Ok(())
}

#[test]
fn a_record_reading_a_missing_id_reruns_when_it_appears() -> TestResult {
    let (project, mut index) = opened()?;
    // A now reads D, which does not exist; B, which reads A, runs again too.
    project.write(A, &thing("synthetic-a", "Synthetic A", "synthetic-d"))?;
    assert_eq!(index.sync()?, lists(&[A], &[A], &[], &[], &[B]));
    assert_eq!(probed(&index)?, [A]);

    project.write(D, &thing("synthetic-d", "Synthetic D", "synthetic-c"))?;
    assert_eq!(index.sync()?, lists(&[D], &[D], &[], &[], &[A]));
    assert_eq!(probed(&index)?, Vec::<String>::new());
    assert_eq!(index.dump()?, fresh(&project)?);
    Ok(())
}

#[test]
fn a_removed_subject_leaves_no_reads_or_findings() -> TestResult {
    let (project, mut index) = opened()?;
    project.write(A, &thing("synthetic-a", "Synthetic A", "synthetic-d"))?;
    index.sync()?;
    assert_eq!(
        rows_of(&index.dump()?, "deps", A),
        [format!("{A}\trec:synthetic-d")]
    );
    assert_eq!(probed(&index)?, [A]);

    project.remove(A)?;
    assert_eq!(index.sync()?, lists(&[], &[], &[A], &[], &[B]));
    let dump = index.dump()?;
    assert_eq!(rows_of(&dump, "deps", A), Vec::<String>::new());
    assert_eq!(rows_of(&dump, "findings", A), Vec::<String>::new());
    assert_eq!(probed(&index)?, [B]);
    assert_eq!(dump, fresh(&project)?);
    Ok(())
}

#[test]
fn untouched_records_are_not_rerun() -> TestResult {
    let (project, mut index) = opened()?;
    project.write(
        C,
        &thing("synthetic-c", "Synthetic C, retitled", "synthetic-c"),
    )?;
    assert_eq!(index.sync()?, lists(&[C], &[C], &[], &[], &[]));
    assert_eq!(index.dump()?, fresh(&project)?);
    Ok(())
}

#[test]
fn deps_of_a_cold_build_equal_those_of_a_history() -> TestResult {
    let (project, mut index) = opened()?;
    assert_eq!(
        section(&index.dump()?, "deps"),
        [
            format!("{A}\trec:synthetic-b"),
            format!("{B}\trec:synthetic-a"),
            format!("{C}\trec:synthetic-c"),
        ]
    );

    // Rename B, point C at D, add D, and delete A and give it again: through sync, update and
    // apply.
    project.remove(B)?;
    project.write(
        B_RENAMED,
        &thing("synthetic-b2", "Synthetic B", "synthetic-a"),
    )?;
    index.sync()?;
    project.write(C, &thing("synthetic-c", "Synthetic C", "synthetic-d"))?;
    index.update(&[C])?;
    let d = thing("synthetic-d", "Synthetic D", "synthetic-b2");
    project.write(D, &d)?;
    index.apply(&[Change::text(D, d)])?;
    project.remove(A)?;
    index.apply(&[Change::Removed { path: A.to_owned() }])?;
    project.write(A, &a())?;
    index.apply(&[Change::text(A, a())])?;

    let deps = section(&index.dump()?, "deps");
    assert_eq!(deps, section(&fresh(&project)?, "deps"));
    assert_eq!(
        deps,
        [
            format!("{A}\trec:synthetic-b"),
            format!("{B_RENAMED}\trec:synthetic-a"),
            format!("{C}\trec:synthetic-d"),
            format!("{D}\trec:synthetic-b2"),
        ]
    );
    assert_eq!(index.dump()?, fresh(&project)?);
    Ok(())
}

#[test]
fn a_record_reading_its_backlinks_reruns_when_one_comes_or_goes() -> TestResult {
    let registry = echo_registry()?;
    let project = TempProject::new()?;
    let named = thing("synthetic-x", "Synthetic X", "synthetic");
    let x = "things/synthetic-x.md";
    let y = "things/synthetic-y.md";
    let z = "things/synthetic-z.md";
    let linking = |id: &str, body: &str| {
        format!(
            "---\n{MARK}id: {id}\nkind: thing\ntitle: Synthetic {id}\ntype: synthetic\n---\n{body}"
        )
    };
    project.write(x, &named)?;
    project.write(y, &linking("synthetic-y", "See [[synthetic-x]].\n"))?;
    let mut index = Index::in_memory(&project.project()?, &registry)?;
    index.sync()?;
    assert!(rows_of(&index.dump()?, "deps", x).contains(&format!("{x}\tin:synthetic-x")));

    // The link goes: X reads the links that name it, so it runs again.
    project.write(y, &linking("synthetic-y", "No link.\n"))?;
    assert_eq!(index.sync()?, lists(&[y], &[y], &[], &[], &[x]));
    assert_eq!(index.dump()?, fresh_with(&project, &registry)?);

    // A new record links to X.
    project.write(z, &linking("synthetic-z", "See [[synthetic-x]].\n"))?;
    assert_eq!(index.sync()?, lists(&[z], &[z], &[], &[], &[x]));
    assert_eq!(index.dump()?, fresh_with(&project, &registry)?);
    Ok(())
}

/// The path of the `n`th thing of [`a_chain`].
fn link_path(n: usize) -> String {
    format!("things/synthetic-n{n:02}.md")
}

/// The `n`th thing of a chain, which reads the next one.
fn link(n: usize) -> String {
    thing(
        &format!("synthetic-n{n:02}"),
        "Synthetic link",
        &format!("synthetic-n{:02}", n + 1),
    )
}

/// A chain of 40 things, each reading the next: more subjects than an incremental run fetches
/// one by one, so a cold build loads its facts in bulk.
fn a_chain(project: &TempProject) -> std::io::Result<()> {
    for n in 0..40 {
        project.write(&link_path(n), &link(n))?;
    }
    Ok(())
}

#[test]
fn many_dirty_subjects_read_the_same_facts_in_bulk() -> TestResult {
    let project = TempProject::new()?;
    a_chain(&project)?;
    let mut index = Index::in_memory(&project.project()?, &registry()?)?;
    index.sync()?;
    assert_eq!(probed(&index)?, [link_path(39)]);
    assert_eq!(section(&index.dump()?, "deps").len(), 40);

    // Single changes fetch by id; the fresh builds load in bulk.
    project.write(&link_path(40), &link(40))?;
    let update = index.sync()?;
    assert_eq!(update.revalidated, [link_path(39)]);
    assert_eq!(probed(&index)?, [link_path(40)]);
    assert_eq!(index.dump()?, fresh(&project)?);

    project.remove(&link_path(20))?;
    let update = index.sync()?;
    assert_eq!(update.revalidated, [link_path(19)]);
    assert_eq!(probed(&index)?, [link_path(19), link_path(40)]);
    assert_eq!(index.dump()?, fresh(&project)?);
    Ok(())
}

/// `test/stray`: reports a finding on another path than its subject's.
const STRAY: LoreRule = LoreRule {
    id: "test/stray",
    severity: Severity::Warning,
    class: RuleClass::Fixed,
    on: On::Kinds(&["thing"]),
    run: stray,
};

fn stray(ctx: &mut Context<'_>, subject: &Subject) -> Result<Vec<Finding>, FactError> {
    ctx.records("synthetic-nowhere")?;
    let mut finding = subject.finding(
        &STRAY,
        Span::default(),
        Position { line: 1, column: 1 },
        "on another subject".to_owned(),
    );
    "things/synthetic-other.md".clone_into(&mut finding.path);
    Ok(vec![finding])
}

/// A rule's finding on another subject is a bug in the rule: a debug build panics.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "a rule reports on its own subject only")]
fn a_finding_on_another_subject_panics_in_a_debug_build() {
    let run = || -> TestResult {
        let project = TempProject::new()?;
        project.write(A, &a())?;
        let mut registry = parts::builtin()?;
        registry.register_rule("test", STRAY)?;
        let mut index = Index::in_memory(&project.project()?, &registry)?;
        index.sync()?;
        Ok(())
    };
    let _ = run();
}
