//! Helpers of the index tests: a temporary content project and its files.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use theos_content::{Finding, Project};

/// The `theos.toml` of data model §13.
const THEOS_TOML: &str = "[project]\nname = \"synthetic\"\nkind = \"content\"\nschema_version = 1\n\n[parts]\nenabled = [\"codex\", \"changes\", \"scripts\", \"graph\", \"concepts\", \"maps\", \"review\", \"mcp\"]\n";

/// A content project in a temporary folder.
pub struct TempProject {
    dir: tempfile::TempDir,
}

impl TempProject {
    /// A project with a `theos.toml` and no records.
    pub fn new() -> Result<TempProject, Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("theos.toml"), THEOS_TOML)?;
        Ok(TempProject { dir })
    }

    /// A project in a temporary folder with the files of the fixture project `name`.
    pub fn copy_of(name: &str) -> Result<TempProject, Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        copy_dir(&fixture(name), dir.path())?;
        Ok(TempProject { dir })
    }

    /// The project folder.
    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    /// The opened project.
    pub fn project(&self) -> Result<Project, Box<dyn std::error::Error>> {
        Ok(Project::open(self.root())?)
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.root().join(rel)
    }

    /// Writes a file and sets its time 60 s back, so it is not racy.
    pub fn write(&self, rel: &str, text: &str) -> std::io::Result<()> {
        self.write_fresh(rel, text)?;
        self.set_time(rel, SystemTime::now() - Duration::from_secs(60))
    }

    /// Writes a file and leaves its time at now.
    pub fn write_fresh(&self, rel: &str, text: &str) -> std::io::Result<()> {
        let path = self.path(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)
    }

    /// Deletes a file.
    pub fn remove(&self, rel: &str) -> std::io::Result<()> {
        std::fs::remove_file(self.path(rel))
    }

    /// Sets the modification time of a file.
    pub fn set_time(&self, rel: &str, time: SystemTime) -> std::io::Result<()> {
        let file = std::fs::File::options().write(true).open(self.path(rel))?;
        file.set_modified(time)
    }

    /// The modification time of a file.
    pub fn time(&self, rel: &str) -> std::io::Result<SystemTime> {
        std::fs::metadata(self.path(rel))?.modified()
    }
}

/// Copies the folder `from` into the existing folder `to`, recursively.
fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Path, rule, severity, line and column of a finding.
pub type Key = (String, String, String, usize, usize);

/// The [`Key`] of each finding; 0 and 0 when it has no position.
pub fn keys(findings: &[Finding]) -> Vec<Key> {
    findings
        .iter()
        .map(|f| {
            let (line, column) = f.position.map_or((0, 0), |p| (p.line, p.column));
            (
                f.path.clone(),
                f.rule.clone(),
                f.severity.as_str().to_owned(),
                line,
                column,
            )
        })
        .collect()
}

/// The folder of the fixture project `name`.
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("projects")
        .join(name)
}

#[derive(serde::Deserialize)]
struct Expectation {
    path: String,
    rule: String,
    severity: String,
    line: usize,
    column: usize,
}

#[derive(serde::Deserialize)]
struct Expectations {
    finding: Vec<Expectation>,
}

/// The `[[finding]]` tables of the `expected.toml` at `path`, in the shape of [`keys`].
pub fn expected(path: &Path) -> Result<Vec<Key>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    let parsed: Expectations = toml::from_str(&text)?;
    Ok(parsed
        .finding
        .into_iter()
        .map(|f| (f.path, f.rule, f.severity, f.line, f.column))
        .collect())
}
