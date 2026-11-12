//! A project in a temporary folder with a workspace over it.

use tempfile::TempDir;

use crate::Workspace;

/// A temporary project with the given files and its workspace.
pub(crate) struct Fixture {
    /// Keeps the folder until the fixture drops.
    _dir: TempDir,
    workspace: Workspace,
}

impl Fixture {
    /// A project with `theos.toml` and `files` (project-relative path, bytes).
    pub(crate) fn new(files: &[(&str, &[u8])]) -> Result<Fixture, Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(
            dir.path().join("theos.toml"),
            "[project]\nname = \"synthetic\"\nkind = \"content\"\nschema_version = 1\n",
        )?;
        for (path, bytes) in files {
            let full = dir.path().join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(full, bytes)?;
        }
        let workspace = Workspace::open(dir.path())?;
        Ok(Fixture {
            _dir: dir,
            workspace,
        })
    }

    /// The workspace.
    pub(crate) fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// The workspace, for running the pipeline.
    pub(crate) fn workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspace
    }
}
