//! The cache file: rebuilds are invisible, preview changes nothing, two processes share it.

use theos_content::kinds::{BodyGrammar, IdRule, KindSpec};
use theos_content::{Change, FieldSpec, FieldType, Hit, Index, Placement, parts};

use crate::common::TempProject;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CACHE: &str = ".theos/cache";
const DANGLING: &str = "things/synthetic-refs.md";
const TRUTH: &str = "lore/truths/T-930.md";

fn test_kind() -> KindSpec {
    KindSpec {
        name: "synthetic-test",
        id: IdRule {
            pattern: "^[a-z0-9]+(-[a-z0-9]+)*$",
            says: "a slug",
        },
        placement: Placement::Flat("tests"),
        fields: vec![
            FieldSpec {
                key: "id",
                ty: FieldType::Id,
                required: true,
            },
            FieldSpec {
                key: "kind",
                ty: FieldType::KindName,
                required: true,
            },
        ],
        body: BodyGrammar::PROSE,
        checks: Vec::new(),
    }
}

fn searches(index: &Index) -> Result<Vec<Vec<Hit>>, Box<dyn std::error::Error>> {
    ["synthetic", "T-930", "\"synthetic stance\"", "beat"]
        .iter()
        .map(|query| Ok(index.search(query, 50)?))
        .collect()
}

fn assert_same_hits(left: &[Vec<Hit>], right: &[Vec<Hit>]) {
    assert_eq!(left.len(), right.len());
    for (a, b) in left.iter().zip(right) {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b) {
            assert_eq!((&x.id, &x.path, &x.matches), (&y.id, &y.path, &y.matches));
            assert!(
                (x.score - y.score).abs() < 1e-9,
                "{} vs {}",
                x.score,
                y.score
            );
        }
    }
}

// AC: CE-12.4
#[test]
fn deleting_the_cache_rebuilds_the_same_index() -> TestResult {
    let project = TempProject::copy_of("links")?;
    let registry = parts::builtin()?;
    let (first, _) = Index::open(&project.project()?, &registry)?;
    let dump = first.dump()?;
    let hits = searches(&first)?;
    assert!(hits.iter().any(|h| !h.is_empty()));
    drop(first);
    std::fs::remove_dir_all(project.root().join(CACHE))?;
    let (second, update) = Index::open(&project.project()?, &registry)?;
    assert_ne!(update.reparsed.len(), 0);
    assert_eq!(second.dump()?, dump);
    assert_same_hits(&hits, &searches(&second)?);
    Ok(())
}

// AC: CE-12.4
#[test]
fn a_damaged_cache_and_a_new_fingerprint_rebuild_the_same_index() -> TestResult {
    let project = TempProject::copy_of("links")?;
    let registry = parts::builtin()?;
    let (first, _) = Index::open(&project.project()?, &registry)?;
    let dump = first.dump()?;
    drop(first);
    std::fs::write(
        project.root().join(CACHE).join("index.sqlite"),
        "this is not a database, only synthetic garbage ".repeat(100),
    )?;
    let (second, _) = Index::open(&project.project()?, &registry)?;
    assert_eq!(second.dump()?, dump);
    drop(second);

    let mut more = parts::builtin()?;
    more.register("test", test_kind())?;
    let (third, update) = Index::open(&project.project()?, &more)?;
    let mut fresh = Index::in_memory(&project.project()?, &more)?;
    fresh.sync()?;
    assert_eq!(third.dump()?, fresh.dump()?);
    let all: Vec<String> = fresh.records()?.into_iter().map(|r| r.path).collect();
    assert_ne!(all.len(), 0);
    for path in &all {
        assert!(update.reparsed.contains(path), "{path} not reparsed");
    }
    Ok(())
}

/// Overwrites the root page of `table` in the cache `file` with garbage. Opening the cache reads
/// page 1 and `meta`; the sync reads `files` and the records.
fn damage_table(file: &std::path::Path, table: &str) -> TestResult {
    use std::io::{Seek, SeekFrom, Write};
    let conn = rusqlite::Connection::open(file)?;
    let page_size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
    let root: i64 = conn.query_row(
        "SELECT rootpage FROM sqlite_master WHERE name = ?1",
        [table],
        |r| r.get(0),
    )?;
    drop(conn);
    assert!(root > 1, "`{table}` is on page {root}");
    let offset = u64::try_from((root - 1) * page_size)?;
    let page_size = usize::try_from(page_size)?;
    let garbage = "synthetic garbage ".repeat(page_size);
    let mut cache = std::fs::OpenOptions::new().write(true).open(file)?;
    cache.seek(SeekFrom::Start(offset))?;
    cache.write_all(&garbage.as_bytes()[..page_size])?;
    Ok(())
}

// AC: CE-12.4
#[test]
fn a_cache_damaged_past_its_first_page_is_rebuilt() -> TestResult {
    let project = TempProject::copy_of("links")?;
    let registry = parts::builtin()?;
    let (first, _) = Index::open(&project.project()?, &registry)?;
    drop(first);
    let file = project.root().join(CACHE).join("index.sqlite");
    damage_table(&file, "files")?;
    let (second, update) = Index::open(&project.project()?, &registry)?;
    let mut fresh = Index::in_memory(&project.project()?, &registry)?;
    fresh.sync()?;
    assert_eq!(second.dump()?, fresh.dump()?);
    assert_ne!(update.reparsed.len(), 0);
    drop(second);
    let (_third, update) = Index::open(&project.project()?, &registry)?;
    assert_eq!(update.reparsed.len(), 0);
    Ok(())
}

#[test]
fn a_foreign_meta_table_is_rebuilt() -> TestResult {
    let project = TempProject::copy_of("links")?;
    let registry = parts::builtin()?;
    let (first, _) = Index::open(&project.project()?, &registry)?;
    let dump = first.dump()?;
    drop(first);
    let folder = project.root().join(CACHE);
    std::fs::remove_dir_all(&folder)?;
    std::fs::create_dir_all(&folder)?;
    let conn = rusqlite::Connection::open(folder.join("index.sqlite"))?;
    conn.execute_batch("CREATE TABLE meta (x INTEGER); INSERT INTO meta VALUES (1);")?;
    drop(conn);
    let (second, update) = Index::open(&project.project()?, &registry)?;
    assert_ne!(update.reparsed.len(), 0);
    assert_eq!(second.dump()?, dump);
    Ok(())
}

fn project_with_index() -> Result<(TempProject, Index), Box<dyn std::error::Error>> {
    let project = TempProject::copy_of("links")?;
    let (index, _) = Index::open(&project.project()?, &parts::builtin()?)?;
    Ok((project, index))
}

fn read(project: &TempProject, rel: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(project.root().join(rel))?.replace("\r\n", "\n"))
}

#[test]
fn preview_returns_what_update_stores_and_changes_nothing() -> TestResult {
    let (project, mut index) = project_with_index()?;
    let dump = index.dump()?;
    let text = format!(
        "{}\nSee [[synthetic-nowhere]].\n",
        read(&project, DANGLING)?
    );
    let change = Change::text(DANGLING, text);
    let preview = index.preview(std::slice::from_ref(&change))?;
    assert!(
        preview
            .findings
            .iter()
            .any(|f| f.rule == "link/dangling" && f.message.contains("synthetic-nowhere"))
    );
    assert_eq!(preview.update.reparsed, vec![DANGLING.to_owned()]);
    assert_eq!(index.dump()?, dump);

    index.apply(&[change])?;
    let stored = index.findings_for(DANGLING)?;
    let previewed: Vec<_> = preview
        .findings
        .iter()
        .filter(|f| f.path == DANGLING)
        .cloned()
        .collect();
    assert_eq!(previewed, stored);
    Ok(())
}

#[test]
fn preview_of_a_removal_and_a_rename() -> TestResult {
    let (project, mut index) = project_with_index()?;
    let dump = index.dump()?;
    let backlinks = index.backlinks("T-930")?;
    assert_ne!(backlinks.len(), 0);

    let removal = index.preview(&[Change::Removed {
        path: TRUTH.to_owned(),
    }])?;
    assert_eq!(removal.update.removed, vec![TRUTH.to_owned()]);
    for link in &backlinks {
        assert!(
            removal
                .findings
                .iter()
                .any(|f| f.rule == "link/dangling" && f.path == link.source_path),
            "no dangling finding on {}",
            link.source_path
        );
    }
    assert_eq!(index.dump()?, dump);

    let renamed = "things/synthetic-refs-moved.md";
    let dangling_at = |findings: &[theos_content::Finding], path: &str| {
        findings
            .iter()
            .filter(|f| f.rule == "link/dangling" && f.path == path)
            .map(|f| f.position)
            .collect::<Vec<_>>()
    };
    let before = dangling_at(&index.findings()?, DANGLING);
    assert_ne!(before.len(), 0);
    let moved = index.preview(&[
        Change::Removed {
            path: DANGLING.to_owned(),
        },
        Change::text(renamed, read(&project, DANGLING)?),
    ])?;
    assert!(moved.findings.iter().all(|f| f.path != DANGLING));
    let after = dangling_at(&moved.findings, renamed);
    for position in &before {
        assert!(after.contains(position), "{position:?} did not move");
    }
    assert_eq!(index.dump()?, dump);
    Ok(())
}

#[test]
fn preview_of_text_that_fails_to_split() -> TestResult {
    let (_project, mut index) = project_with_index()?;
    let path = "things/synthetic-broken.md";
    let preview = index.preview(&[Change::text(path, "synthetic text without a fence\n")])?;
    let rules: Vec<_> = preview
        .findings
        .iter()
        .filter(|f| f.path == path)
        .map(|f| f.rule.as_str())
        .collect();
    assert_eq!(rules, vec!["record/opening-line"]);
    Ok(())
}

#[test]
fn two_indexes_sync_from_two_threads() -> TestResult {
    let project = TempProject::copy_of("links")?;
    let registry = parts::builtin()?;
    let open = || -> Result<String, String> {
        let project = project.project().map_err(|e| e.to_string())?;
        let (index, _) = Index::open(&project, &registry).map_err(|e| e.to_string())?;
        index.dump().map_err(|e| e.to_string())
    };
    let (a, b) = std::thread::scope(|scope| {
        let a = scope.spawn(open);
        let b = scope.spawn(open);
        (a.join(), b.join())
    });
    let (Ok(a), Ok(b)) = (a, b) else {
        return Err("a thread panicked".into());
    };
    assert_eq!(a?, b?);
    Ok(())
}
