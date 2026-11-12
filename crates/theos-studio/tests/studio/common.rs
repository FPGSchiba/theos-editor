//! Helpers of the studio tests: a temporary content project, its files, and the helper process.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use crate::helper::HelperTask;

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

    /// A project in a temporary folder with the files of the fixture project `name` of
    /// `theos-content`.
    pub fn copy_of(name: &str) -> Result<TempProject, Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        copy_dir(&fixture(name), dir.path())?;
        Ok(TempProject { dir })
    }

    /// The project folder.
    pub fn root(&self) -> &Path {
        self.dir.path()
    }
}

/// The folder of the fixture project `name` of `theos-content`.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("theos-content")
        .join("tests")
        .join("fixtures")
        .join("projects")
        .join(name)
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

/// Every file under `root` except `.theos/cache`, by path with `/` separators.
pub fn files(root: &Path) -> std::io::Result<BTreeMap<String, Vec<u8>>> {
    let mut found = BTreeMap::new();
    collect(root, root, &mut found)?;
    Ok(found)
}

fn collect(root: &Path, dir: &Path, found: &mut BTreeMap<String, Vec<u8>>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel: Vec<String> = path
            .strip_prefix(root)
            .map_err(std::io::Error::other)?
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        if rel.join("/") == ".theos/cache" {
            continue;
        }
        if entry.file_type()?.is_dir() {
            collect(root, &path, found)?;
        } else {
            found.insert(rel.join("/"), std::fs::read(&path)?);
        }
    }
    Ok(())
}

/// Starts the helper process: this test binary, running only `helper::helper`, with the task in
/// the environment variable `THEOS_STUDIO_HELPER`.
pub fn spawn_helper(task: &HelperTask) -> Result<Child, Box<dyn std::error::Error>> {
    Ok(Command::new(std::env::current_exe()?)
        .args(["helper::helper", "--exact"])
        .env("THEOS_STUDIO_HELPER", serde_json::to_string(task)?)
        .spawn()?)
}

/// Waits up to 30 s for the file `path` to exist.
pub fn wait_for(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    while !path.exists() {
        if start.elapsed() > Duration::from_secs(30) {
            return Err(format!("timed out waiting for {}", path.display()).into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}
