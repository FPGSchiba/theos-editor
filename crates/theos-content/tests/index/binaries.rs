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
    assert_eq!(lines, [10, 11, 12, 13, 14, 15]);
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
fn a_binaries_folder_that_does_not_exist_gives_missing_for_every_reference() -> TestResult {
    let project = TempProject::copy_of("binaries")?;
    let dir = tempfile::tempdir()?;
    let gone = dir.path().join("binaries");
    assert!(!gone.exists());
    let (mut index, cache) = open(&project)?;
    let report = index.binary_findings(&gone)?;
    // The three references of v1 and the six of v2, each at its value.
    let found: Vec<_> = report.findings.iter().map(key).collect();
    assert_eq!(
        found,
        [
            (V1, "binary/missing", "warning", 10, 7),
            (V1, "binary/missing", "warning", 11, 12),
            (V1, "binary/missing", "warning", 12, 12),
            (V2, "binary/missing", "warning", 10, 11),
            (V2, "binary/missing", "warning", 11, 11),
            (V2, "binary/missing", "warning", 12, 10),
            (V2, "binary/missing", "warning", 13, 8),
            (V2, "binary/missing", "warning", 14, 9),
            (V2, "binary/missing", "warning", 15, 10),
        ]
    );
    for finding in report.findings.iter().filter(|f| f.path == V1) {
        assert!(
            finding.message.starts_with("there is no file `boards/"),
            "{}",
            finding.message
        );
    }
    assert_eq!(report.hashed, Vec::<String>::new());
    assert_eq!(memo_rows(&cache)?, Vec::<String>::new());
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

/// The fixture project with one more version whose only output is `reference`.
fn with_version(reference: &str) -> Result<TempProject, Box<dyn std::error::Error>> {
    let project = TempProject::copy_of("binaries")?;
    project.write(
        "concepts/synthetic-bin/synthetic-bin-v3.md",
        &format!(
            "---\n# synthetic fixture (P04), not canon\nid: synthetic-bin-v3\nkind: concept-version\n\
             title: synthetic-bin-v3\nconcept: synthetic-bin\ngenerator: synthetic@1.0\n\
             recipe: synthetic-recipe\noutputs:\n  out: {reference}@sha256:{}\n\
             rating: unrated\n---\nSynthetic version.\n",
            "0".repeat(64)
        ),
    )?;
    Ok(project)
}

fn messages_of_v3(
    project: &TempProject,
    binaries: &Binaries,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let (mut index, _) = open(project)?;
    let report = index.binary_findings(&binaries.root())?;
    Ok(report
        .findings
        .into_iter()
        .filter(|f| f.path.ends_with("v3.md"))
        .map(|f| (f.rule, f.message))
        .collect())
}

// AC: CE-15.4
#[test]
fn a_colon_in_any_segment_leaves_the_folder() -> TestResult {
    let binaries = Binaries::new()?;
    let project = with_version("boards/C:ok.bin")?;
    let found = messages_of_v3(&project, &binaries)?;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, "binary/missing");
    assert!(found[0].1.contains("leaves the binaries folder"));
    Ok(())
}

// AC: CE-15.4
#[test]
fn a_reference_spelled_unlike_its_file_is_missing_on_every_os() -> TestResult {
    // Only `boards/ok.bin` is on disk. A case-insensitive file system (Windows, macOS) opens it
    // for `Boards/OK.bin`; the reference still counts only when spelled exactly so, as on Linux.
    let binaries = Binaries::new()?;
    let folds_case = binaries.root().join("BOARDS").join("OK.BIN").exists();
    let project = with_version("Boards/OK.bin")?;
    let found = messages_of_v3(&project, &binaries)?;
    let expected = if folds_case {
        "there is no file `Boards/OK.bin` in the binaries folder; it is spelled `boards/ok.bin` \
         there"
    } else {
        "there is no file `Boards/OK.bin` in the binaries folder"
    };
    assert_eq!(
        found,
        [("binary/missing".to_owned(), expected.to_owned())],
        "folds case: {folds_case}"
    );
    // The file was never hashed for that reference.
    let (mut index, cache) = open(&project)?;
    let report = index.binary_findings(&binaries.root())?;
    assert!(!report.hashed.contains(&"Boards/OK.bin".to_owned()));
    assert!(!memo_rows(&cache)?.contains(&"Boards/OK.bin".to_owned()));
    Ok(())
}

#[test]
fn a_folder_at_the_path_is_not_a_file() -> TestResult {
    let binaries = Binaries::new()?;
    let project = with_version("boards")?;
    let found = messages_of_v3(&project, &binaries)?;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, "binary/missing");
    assert!(
        found[0].1.contains("is a folder, not a file"),
        "{}",
        found[0].1
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_link_that_points_out_of_the_folder_is_not_read() -> TestResult {
    let binaries = Binaries::new()?;
    std::os::unix::fs::symlink(
        binaries.dir.path().join("outside.bin"),
        binaries.root().join("boards").join("link.bin"),
    )?;
    let project = with_version("boards/link.bin")?;
    let found = messages_of_v3(&project, &binaries)?;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, "binary/missing");
    assert!(found[0].1.contains("leaves the binaries folder"));
    let (mut index, cache) = open(&project)?;
    let report = index.binary_findings(&binaries.root())?;
    assert!(!report.hashed.contains(&"boards/link.bin".to_owned()));
    assert!(!memo_rows(&cache)?.contains(&"boards/link.bin".to_owned()));
    Ok(())
}
