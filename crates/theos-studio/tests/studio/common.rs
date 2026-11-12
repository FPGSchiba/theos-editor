//! Helpers of the studio tests: a temporary content project, its files, a workspace on it, the
//! commands most tests build, and the helper process.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};

use theos_content::PatchOp;
use theos_content::yaml::RawValue;
use theos_studio::{Base, Patch, Workspace};

use crate::helper::HelperTask;

/// The error of a test helper.
type Error = Box<dyn std::error::Error>;

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

    /// A project in a temporary folder with the files of the P06 fixture project `name` of
    /// `theos-studio` (`tests/fixtures/<name>`).
    pub fn copy_of_studio(name: &str) -> Result<TempProject, Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let from = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name);
        copy_dir(&from, dir.path())?;
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

/// The dump of the index of a fresh `Workspace::open` of a copy of the files of `root`: what a
/// cache that matches the disk must dump.
pub fn cold_dump(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let copy = tempfile::tempdir()?;
    for (path, bytes) in files(root)? {
        let target = copy.path().join(&path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(target, bytes)?;
    }
    let cold = Workspace::open(copy.path())?;
    Ok(cold.index().dump()?)
}

/// A copy of the P06 fixture `name` and a workspace on it.
pub fn open(name: &str) -> Result<(TempProject, Workspace), Error> {
    let project = TempProject::copy_of_studio(name)?;
    let workspace = Workspace::open(project.root())?;
    Ok((project, workspace))
}

/// The base of the file `path` as the workspace reads it now.
pub fn base_of(workspace: &Workspace, path: &str) -> Result<Base, Error> {
    let (_, base) = workspace.read(path)?.ok_or("the file is missing")?;
    Ok(base)
}

/// A patch that sets the field `key` of the record `path` to `value`.
pub fn set(workspace: &Workspace, path: &str, key: &str, value: &str) -> Result<Patch, Error> {
    Ok(Patch {
        path: path.to_owned(),
        base: base_of(workspace, path)?,
        ops: vec![PatchOp::Set {
            key: key.to_owned(),
            value: RawValue::Text(value.to_owned()),
        }],
    })
}

/// The helper process; it is killed and waited for when this is dropped, so a failing test
/// never leaves it running with the lock or the project folder.
pub struct HelperProcess {
    child: Child,
}

impl HelperProcess {
    /// The process id of the helper.
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// Kills the helper.
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }

    /// Waits for the helper to exit.
    pub fn wait(&mut self) -> std::io::Result<ExitStatus> {
        self.child.wait()
    }

    /// Waits up to 30 s for the file `path` to exist; fails at once, naming the exit status, when
    /// the helper has exited before.
    pub fn wait_for(&mut self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        while !path.exists() {
            if let Some(status) = self.child.try_wait()? {
                return Err(format!(
                    "the helper exited with {status} before {} existed",
                    path.display()
                )
                .into());
            }
            if start.elapsed() > Duration::from_secs(30) {
                return Err(format!("timed out waiting for {}", path.display()).into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }
}

impl Drop for HelperProcess {
    fn drop(&mut self) {
        // The helper may have exited already; then there is nothing to stop.
        drop(self.child.kill());
        drop(self.child.wait());
    }
}

/// Starts the helper process: this test binary, running only `helper::helper`, with the task in
/// the environment variable `THEOS_STUDIO_HELPER`.
pub fn spawn_helper(task: &HelperTask) -> Result<HelperProcess, Box<dyn std::error::Error>> {
    let child = Command::new(std::env::current_exe()?)
        .args(["helper::helper", "--exact"])
        .env("THEOS_STUDIO_HELPER", serde_json::to_string(task)?)
        .spawn()?;
    Ok(HelperProcess { child })
}
