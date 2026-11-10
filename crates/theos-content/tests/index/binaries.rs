//! Binary reference checks on request (spec §5.13).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use theos_content::{Finding, Index, Project, parts};

use crate::common::TempProject;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const V1: &str = "concepts/synthetic-bin/synthetic-bin-v1.md";
const V2: &str = "concepts/synthetic-bin/synthetic-bin-v2.md";

/// A binaries folder in a temporary folder, with the sentinel `outside.bin` next to it.
struct Binaries {
    dir: tempfile::TempDir,
}

impl Binaries {
    fn new() -> Result<Binaries, Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");
        std::fs::create_dir_all(dir.path().join("binaries").join("boards"))?;
        for name in ["ok.bin", "changed.bin"] {
            let from = source.join("binaries").join("boards").join(name);
            std::fs::copy(from, dir.path().join("binaries").join("boards").join(name))?;
        }
        std::fs::copy(source.join("outside.bin"), dir.path().join("outside.bin"))?;
        Ok(Binaries { dir })
    }

    fn root(&self) -> PathBuf {
        self.dir.path().join("binaries")
    }
}

/// The fixture project, opened with a cache file, and the path of that file.
fn open(project: &TempProject) -> Result<(Index, PathBuf), Box<dyn std::error::Error>> {
    let (index, _) = Index::open(&Project::open(project.root())?, &parts::builtin()?)?;
    let cache = project
        .root()
        .join(".theos")
        .join("cache")
        .join("index.sqlite");
    Ok((index, cache))
}

fn memo_rows(cache: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let conn = rusqlite::Connection::open(cache)?;
    let mut stmt = conn.prepare("SELECT path FROM binary_memo ORDER BY path")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Path, rule, severity, line and column of a finding.
fn key(f: &Finding) -> (&str, &str, &str, usize, usize) {
    let (line, column) = f.position.map_or((0, 0), |p| (p.line, p.column));
    (&f.path, &f.rule, f.severity.as_str(), line, column)
}

// AC: CE-15.4
#[test]
fn a_missing_binary_or_a_hash_mismatch_is_a_warning() -> TestResult {
    let project = TempProject::copy_of("binaries")?;
    let binaries = Binaries::new()?;
    let (mut index, _) = open(&project)?;
    let report = index.binary_findings(&binaries.root())?;
    let in_v1: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.path == V1)
        .map(key)
        .collect();
    assert_eq!(
        in_v1,
        [
            (V1, "binary/hash", "warning", 11, 12),
            (V1, "binary/missing", "warning", 12, 12),
        ]
    );
    let missing = report
        .findings
        .iter()
        .find(|f| f.rule == "binary/missing" && f.path == V1)
        .ok_or("no missing finding")?;
    assert_eq!(missing.id.as_deref(), Some("synthetic-bin-v1"));
    assert!(missing.message.contains("boards/missing.bin"));
    // The findings are not stored with the others.
    assert!(
        index
            .findings()?
            .iter()
            .all(|f| !f.rule.starts_with("binary/"))
    );
    Ok(())
}

// AC: CE-15.4
#[test]
fn a_second_call_hashes_nothing() -> TestResult {
    let project = TempProject::copy_of("binaries")?;
    let binaries = Binaries::new()?;
    let (mut index, _) = open(&project)?;
    let first = index.binary_findings(&binaries.root())?;
    let mut hashed = first.hashed.clone();
    hashed.sort();
    assert_eq!(hashed, ["boards/changed.bin", "boards/ok.bin"]);
    let second = index.binary_findings(&binaries.root())?;
    assert_eq!(second.hashed, Vec::<String>::new());
    assert_eq!(second.findings, first.findings);
    Ok(())
}

// AC: CE-15.4
#[test]
fn paths_that_leave_the_folder_are_never_opened() -> TestResult {
    let project = TempProject::copy_of("binaries")?;
    let binaries = Binaries::new()?;
    let (mut index, cache) = open(&project)?;
    let report = index.binary_findings(&binaries.root())?;
    let in_v2: Vec<_> = report.findings.iter().filter(|f| f.path == V2).collect();
    let lines: Vec<usize> = in_v2
        .iter()
        .map(|f| f.position.map_or(0, |p| p.line))
        .collect();
    assert_eq!(lines, [10, 11, 12, 13, 14]);
    for finding in &in_v2 {
        assert_eq!(finding.rule, "binary/missing");
        assert_eq!(finding.severity.as_str(), "warning");
        assert!(
            finding.message.contains("leaves the binaries folder"),
            "{}",
            finding.message
        );
    }
    // The sentinel exists and `../outside.bin` names it, yet nothing was hashed or memoised.
    assert!(binaries.dir.path().join("outside.bin").is_file());
    assert_eq!(memo_rows(&cache)?, ["boards/changed.bin", "boards/ok.bin"]);
    assert!(report.hashed.iter().all(|path| path.starts_with("boards/")));
    Ok(())
}

#[test]
fn a_changed_file_is_hashed_again() -> TestResult {
    let project = TempProject::copy_of("binaries")?;
    let binaries = Binaries::new()?;
    let (mut index, _) = open(&project)?;
    index.binary_findings(&binaries.root())?;
    // The same size, another time: the hash is read again.
    let file = binaries.root().join("boards").join("ok.bin");
    let handle = std::fs::File::options().write(true).open(file)?;
    handle.set_modified(SystemTime::now() - Duration::from_secs(3600))?;
    let again = index.binary_findings(&binaries.root())?;
    assert_eq!(again.hashed, ["boards/ok.bin"]);
    // Another size: the hash is read again and now differs from the reference.
    std::fs::write(
        binaries.root().join("boards").join("ok.bin"),
        b"edited bytes\n",
    )?;
    let edited = index.binary_findings(&binaries.root())?;
    assert_eq!(edited.hashed, ["boards/ok.bin"]);
    assert!(
        edited
            .findings
            .iter()
            .any(|f| { f.rule == "binary/hash" && f.position.is_some_and(|p| p.line == 10) })
    );
    Ok(())
}

#[test]
fn a_memo_belongs_to_the_folder_it_was_made_for() -> TestResult {
    let project = TempProject::copy_of("binaries")?;
    let (first, second) = (Binaries::new()?, Binaries::new()?);
    let (mut index, _) = open(&project)?;
    index.binary_findings(&first.root())?;
    assert_eq!(index.binary_findings(&second.root())?.hashed.len(), 2);
    Ok(())
}

#[test]
fn the_memo_is_not_part_of_the_dump_and_survives_a_reopen() -> TestResult {
    let project = TempProject::copy_of("binaries")?;
    let binaries = Binaries::new()?;
    let (mut index, _) = open(&project)?;
    let before = index.dump()?;
    index.binary_findings(&binaries.root())?;
    assert_eq!(index.dump()?, before);
    drop(index);
    let (mut reopened, _) = open(&project)?;
    let again = reopened.binary_findings(&binaries.root())?;
    assert_eq!(again.hashed, Vec::<String>::new());
    Ok(())
}
