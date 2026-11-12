//! A project in a temporary folder with a workspace over it.

use std::path::Path;

use tempfile::TempDir;

use crate::{PlanContext, Workspace};

/// A synthetic truth record (LF).
pub(crate) const TRUTH: &str =
    "---\nid: T-960\nkind: truth\ntitle: Old\ntarget: confirmed\n---\nBody\n";

/// The path of [`TRUTH`] in a project.
pub(crate) const TRUTH_PATH: &str = "lore/truths/T-960.md";

/// A temporary project with the given files and its workspace.
pub(crate) struct Fixture {
    dir: TempDir,
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
        Ok(Fixture { dir, workspace })
    }

    /// The context a plan runs in.
    pub(crate) fn ctx(&self) -> PlanContext<'_> {
        PlanContext::new(
            self.workspace.project().root(),
            self.workspace.index(),
            self.workspace.registry(),
        )
    }

    /// The workspace.
    pub(crate) fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// The workspace, for running the pipeline.
    pub(crate) fn workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspace
    }

    /// The project folder.
    pub(crate) fn root(&self) -> &Path {
        self.dir.path()
    }
}
