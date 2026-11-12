//! Helpers of the studio tests, each in one place: a temporary content project, its files, a
//! workspace on it, the commands most tests build, the checks most tests make, and the helper
//! process.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};

use theos_content::yaml::RawValue;
use theos_content::{Finding, PatchOp, Span, TextEdit};
use theos_studio::{Base, CommandError, Create, Edit, Patch, Workspace};

use crate::helper::HelperTask;

/// The error of a test or a test helper.
pub type Error = Box<dyn std::error::Error>;

/// The result of a test.
pub type TestResult = Result<(), Error>;

/// The files of a project: their bytes by path with `/` separators.
pub type Files = BTreeMap<String, Vec<u8>>;

/// The truth of the fixture `commands/` that every other kind links.
pub const TRUTH: &str = "lore/truths/T-960.md";
/// The truth of `commands/` that only links itself.
pub const OTHER_TRUTH: &str = "lore/truths/T-961.md";
/// The clue of `commands/` that reveals [`TRUTH`].
pub const CLUE: &str = "lore/clues/C-960.md";
/// The second clue of `commands/`, which reveals [`TRUTH`] in a quoted value.
pub const SECOND_CLUE: &str = "lore/clues/C-961.md";
/// The script of `commands/`.
pub const SCRIPT: &str = "story/scripts/S-960.md";
/// The script of `standing/` with a standing `link/dangling` error.
pub const STANDING_SCRIPT: &str = "story/scripts/S-980.md";

/// The `theos.toml` of data model §13.
const THEOS_TOML: &str = "[project]\nname = \"synthetic\"\nkind = \"content\"\nschema_version = 1\n\n[parts]\nenabled = [\"codex\", \"changes\", \"scripts\", \"graph\", \"concepts\", \"maps\", \"review\", \"mcp\"]\n";

/// A content project in a temporary folder.
pub struct TempProject {
    dir: tempfile::TempDir,
}

impl TempProject {
    /// A project with a `theos.toml` and no records.
    pub fn new() -> Result<TempProject, Error> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("theos.toml"), THEOS_TOML)?;
        Ok(TempProject { dir })
    }

    /// A project in a temporary folder with the files of the fixture project `name` of
    /// `theos-content`.
    pub fn copy_of(name: &str) -> Result<TempProject, Error> {
        TempProject::copied(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("theos-content")
                .join("tests")
                .join("fixtures")
                .join("projects")
                .join(name),
        )
    }

    /// A project in a temporary folder with the files of the P06 fixture project `name` of
    /// `theos-studio` (`tests/fixtures/<name>`).
    pub fn copy_of_studio(name: &str) -> Result<TempProject, Error> {
        TempProject::copied(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join(name),
        )
    }

    /// A project in a temporary folder with the files under `from`, `.theos/cache` left out.
    fn copied(from: &Path) -> Result<TempProject, Error> {
        let dir = tempfile::tempdir()?;
        for (path, bytes) in files(from)? {
            put(dir.path(), &path, &bytes)?;
        }
        Ok(TempProject { dir })
    }

    /// The project folder.
    pub fn root(&self) -> &Path {
        self.dir.path()
    }
}

/// Every file under `root` except `.theos/cache`, by path with `/` separators.
pub fn files(root: &Path) -> std::io::Result<Files> {
    let mut found = BTreeMap::new();
    collect(root, root, &mut found)?;
    Ok(found)
}

fn collect(root: &Path, dir: &Path, found: &mut Files) -> std::io::Result<()> {
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

/// Writes the file `path` (with `/`) under `root` with `bytes`, and the folders above it.
pub fn put(root: &Path, path: &str, bytes: &[u8]) -> std::io::Result<()> {
    let target = root.join(path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(target, bytes)
}

/// The dump of the index of a fresh `Workspace::open` of a copy of the files of `root`: what a
/// cache that matches the disk must dump.
pub fn cold_dump(root: &Path) -> Result<String, Error> {
    let copy = TempProject::copied(root)?;
    let cold = Workspace::open(copy.root())?;
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

/// An edit that appends `text` to the record `path`, at the end of its text with LF line ends.
pub fn append(workspace: &Workspace, path: &str, text: &str) -> Result<Edit, Error> {
    let (bytes, base) = workspace.read(path)?.ok_or("the file is missing")?;
    let end = String::from_utf8(bytes)?.replace("\r\n", "\n").len();
    Ok(Edit {
        path: path.to_owned(),
        base,
        edits: vec![TextEdit {
            span: Span { start: end, end },
            replacement: text.to_owned(),
        }],
    })
}

/// A truth without clues, as `Create` takes it.
pub fn new_truth(id: &str) -> Create {
    Create {
        path: format!("lore/truths/{id}.md"),
        text: format!(
            "---\n# synthetic fixture (P06), not canon\nid: {id}\nkind: truth\n\
             title: Synthetic truth without clues\nstatus: draft\ntarget: hinted\n---\n\
             Synthetic statement.\n"
        ),
    }
}

/// Runs `command`, which must fail, and checks that it changed nothing: every file keeps its
/// bytes, the journal keeps its entries, and the index cache holds what the files hold (a cold
/// build of them dumps the same). That is the dump from before the command, unless the files
/// changed behind the workspace and the command's first step brought the cache in line with
/// them. Returns the error.
pub fn refused(
    project: &TempProject,
    workspace: &mut Workspace,
    command: &dyn theos_studio::Command,
) -> Result<CommandError, Error> {
    let files_before = files(project.root())?;
    let journal_before = (workspace.undo_entries(), workspace.redo_entries());
    let error = workspace
        .run(command)
        .err()
        .ok_or_else(|| format!("{} applied", command.label()))?;
    assert_eq!(files(project.root())?, files_before, "{error}");
    assert_eq!(
        workspace.index().dump()?,
        cold_dump(project.root())?,
        "{error}"
    );
    assert_eq!(
        (workspace.undo_entries(), workspace.redo_entries()),
        journal_before,
        "{error}"
    );
    Ok(error)
}

/// Whether `findings` hold one of `rule` on `path`.
pub fn holds(findings: &[Finding], path: &str, rule: &str) -> bool {
    findings.iter().any(|f| f.path == path && f.rule == rule)
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

    /// Waits up to five minutes for the helper to exit; fails after that, and the helper is killed
    /// when this is dropped.
    pub fn wait(&mut self) -> Result<ExitStatus, Error> {
        let limit = Duration::from_secs(300);
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait()? {
                return Ok(status);
            }
            if start.elapsed() > limit {
                return Err(format!("the helper did not exit within {limit:?}").into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Waits up to 30 s for the file `path` to exist; fails at once, naming the exit status, when
    /// the helper has exited before.
    pub fn wait_for(&mut self, path: &Path) -> Result<(), Error> {
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
pub fn spawn_helper(task: &HelperTask) -> Result<HelperProcess, Error> {
    let child = Command::new(std::env::current_exe()?)
        .args(["helper::helper", "--exact"])
        .env("THEOS_STUDIO_HELPER", serde_json::to_string(task)?)
        .spawn()?;
    Ok(HelperProcess { child })
}
