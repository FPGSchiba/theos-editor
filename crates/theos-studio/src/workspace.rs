//! An opened project with its registry and index.

use std::path::{Path, PathBuf};

use theos_content::{Index, IndexError, KindRegistry, Project, parts};

use crate::error::StudioError;

/// An opened project: its files, its editor parts and the index over them.
pub struct Workspace {
    project: Project,
    registry: KindRegistry,
    index: Index,
    in_memory: bool,
}

impl Workspace {
    /// Opens the project in `root` and syncs its index. When another process holds the cache, the
    /// index lives in memory.
    ///
    /// # Errors
    ///
    /// Fails when the project, the registry or the index cannot be opened.
    pub fn open(root: &Path) -> Result<Workspace, StudioError> {
        let project = Project::open(root)?;
        let registry = parts::builtin()?;
        let (index, in_memory) =
            fallback(Index::open(&project, &registry).map(|(i, _)| i), || {
                let mut index = Index::in_memory(&project, &registry)?;
                index.sync()?;
                Ok(index)
            })?;
        if in_memory {
            tracing::warn!(
                cache = %root.join(".theos").join("cache").join("index.sqlite").display(),
                "the index cache is in use; working with an index in memory"
            );
        }
        Ok(Workspace {
            project,
            registry,
            index,
            in_memory,
        })
    }

    /// Opens the project that holds `start`.
    ///
    /// # Errors
    ///
    /// [`StudioError::NotAProject`] without a `theos.toml` above `start`; else as
    /// [`Workspace::open`].
    pub fn find(start: &Path) -> Result<Workspace, StudioError> {
        Workspace::open(&find_root(start)?)
    }

    /// The project.
    #[must_use]
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The editor-part registry.
    #[must_use]
    pub fn registry(&self) -> &KindRegistry {
        &self.registry
    }

    /// The index.
    #[must_use]
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// The index, for writing.
    pub fn index_mut(&mut self) -> &mut Index {
        &mut self.index
    }

    /// Whether the index lives in memory because the cache was in use.
    #[must_use]
    pub fn is_in_memory(&self) -> bool {
        self.in_memory
    }

    /// Runs `f` on the index. On [`IndexError::Contended`] the workspace switches once to an
    /// index in memory, synced with the folder, and runs `f` again.
    ///
    /// # Errors
    ///
    /// The error of `f`, or of building the index in memory.
    pub fn write<T>(
        &mut self,
        mut f: impl FnMut(&mut Index) -> Result<T, IndexError>,
    ) -> Result<T, StudioError> {
        match f(&mut self.index) {
            Err(IndexError::Contended { path }) if !self.in_memory => {
                tracing::warn!(
                    cache = %path.display(),
                    "the index cache is contended; switching to an index in memory"
                );
                let mut index = Index::in_memory(&self.project, &self.registry)?;
                index.sync()?;
                self.index = index;
                self.in_memory = true;
                Ok(f(&mut self.index)?)
            }
            other => Ok(other?),
        }
    }
}

/// The folder of the project that holds `start`: the first of `start` and its ancestors with a
/// `theos.toml` file. `start` is made absolute first, so `.` searches from the current folder.
///
/// # Errors
///
/// [`StudioError::NotAProject`], naming the absolute start, when there is none, and
/// [`StudioError::Io`] when `start` cannot be made absolute.
pub fn find_root(start: &Path) -> Result<PathBuf, StudioError> {
    let absolute = std::path::absolute(start).map_err(|source| StudioError::Io {
        path: start.to_path_buf(),
        source,
    })?;
    let found = absolute
        .ancestors()
        .find(|dir| dir.join("theos.toml").is_file())
        .map(Path::to_path_buf);
    found.ok_or(StudioError::NotAProject { start: absolute })
}

/// The opened index, or, when its cache is contended, the index in memory (with `true`).
fn fallback(
    opened: Result<Index, IndexError>,
    in_memory: impl FnOnce() -> Result<Index, IndexError>,
) -> Result<(Index, bool), IndexError> {
    match opened {
        Ok(index) => Ok((index, false)),
        Err(IndexError::Contended { .. }) => Ok((in_memory()?, true)),
        Err(other) => Err(other),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::test_support::{Fixture, TestResult};

    #[test]
    fn a_contended_cache_falls_back_to_memory() -> TestResult {
        let fixture = Fixture::new(&[])?;
        let workspace = fixture.workspace();
        let (_index, in_memory) = fallback(
            Err(IndexError::Contended {
                path: PathBuf::from("index.sqlite"),
            }),
            || Index::in_memory(workspace.project(), workspace.registry()),
        )?;
        assert!(in_memory);
        Ok(())
    }

    #[test]
    fn other_errors_pass_through() {
        let called = Cell::new(false);
        let result = fallback(
            Err(IndexError::Busy {
                path: PathBuf::from("index.sqlite"),
            }),
            || {
                called.set(true);
                Err(IndexError::Contended {
                    path: PathBuf::new(),
                })
            },
        );
        assert!(matches!(result, Err(IndexError::Busy { .. })));
        assert!(!called.get());
    }

    #[test]
    fn an_opened_index_is_kept() -> TestResult {
        let fixture = Fixture::new(&[])?;
        let workspace = fixture.workspace();
        let index = Index::in_memory(workspace.project(), workspace.registry())?;
        let (_index, in_memory) = fallback(Ok(index), || {
            Err(IndexError::Contended {
                path: PathBuf::new(),
            })
        })?;
        assert!(!in_memory);
        Ok(())
    }

    #[test]
    fn a_contended_write_runs_again_in_memory() -> TestResult {
        let mut fixture = Fixture::new(&[])?;
        let workspace = fixture.workspace_mut();
        assert!(!workspace.is_in_memory());
        let mut calls = 0;
        let value = workspace.write(|_| {
            calls += 1;
            if calls == 1 {
                Err(IndexError::Contended {
                    path: PathBuf::from("index.sqlite"),
                })
            } else {
                Ok(7)
            }
        })?;
        assert_eq!(value, 7);
        assert!(workspace.is_in_memory());
        Ok(())
    }
}
