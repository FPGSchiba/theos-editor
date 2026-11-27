//! The project the server answers about, opened on the first call that needs it.

use std::path::{Path, PathBuf};

use theos_content::Index;
use theos_studio::{StudioError, Workspace, find_root, report};

/// Where the project is, fixed at start.
#[derive(Debug, Clone)]
pub enum Locator {
    /// `--project`: this folder is the root; it must hold `theos.toml`.
    Given(PathBuf),
    /// The nearest folder at or above this one that holds `theos.toml`.
    Search(PathBuf),
}

/// The locator and the workspace, once it opened.
///
/// A failed open leaves `workspace` empty, so the next call tries again: `theos new` or a
/// checkout that creates `theos.toml` needs no restart.
pub struct State {
    locator: Locator,
    workspace: Option<Workspace>,
}

impl State {
    /// A state that opens its workspace on the first call of [`State::with`].
    #[must_use]
    pub fn new(locator: Locator) -> State {
        State {
            locator,
            workspace: None,
        }
    }

    /// Opens the workspace when none is open, syncs its index with the folder and runs `f`.
    ///
    /// # Errors
    /// The `report` of the error that stopped the open, the sync or `f`.
    pub fn with<T>(
        &mut self,
        f: impl FnOnce(&mut Workspace) -> Result<T, StudioError>,
    ) -> Result<T, String> {
        self.try_with(f).map_err(|error| report(&error))
    }

    /// The project folder, opening the workspace when none is open. The index is not synced:
    /// the caller reads files only.
    ///
    /// # Errors
    /// The `report` of the error that stopped the open.
    pub fn root(&mut self) -> Result<PathBuf, String> {
        let workspace = match &mut self.workspace {
            Some(workspace) => workspace,
            slot @ None => slot.insert(open(&self.locator).map_err(|error| report(&error))?),
        };
        Ok(Path::to_path_buf(workspace.project().root()))
    }

    fn try_with<T>(
        &mut self,
        f: impl FnOnce(&mut Workspace) -> Result<T, StudioError>,
    ) -> Result<T, StudioError> {
        let workspace = match &mut self.workspace {
            Some(workspace) => workspace,
            slot @ None => slot.insert(open(&self.locator)?),
        };
        workspace.write(Index::sync)?;
        f(workspace)
    }
}

/// Opens the workspace the locator names.
fn open(locator: &Locator) -> Result<Workspace, StudioError> {
    match locator {
        Locator::Given(root) => Workspace::open(root),
        Locator::Search(start) => Workspace::open(&find_root(start)?),
    }
}
