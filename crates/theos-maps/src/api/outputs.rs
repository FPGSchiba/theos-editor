//! The files of a run in its output folder (spec §5.6, plan ruling L10).
//!
//! Output `role` of the run with seed `n` is `s<n>-<role>.png` for a render,
//! `s<n>-features.geojson` for the features and `s<n>.theosmap` for the package. Each is written
//! as `.<name>.part` and renamed to its name only when every output is written, so a crash never
//! leaves a complete-looking output. Until then [`Outputs`] removes, when it is dropped, exactly
//! the files it created: on an error, on cancel and while a panic unwinds. Files that were in the
//! folder before stay untouched.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write as _};
use std::path::{Path, PathBuf};

use crate::error::RunError;

/// The role of the package output.
pub(crate) const PACKAGE: &str = "package";
/// The role of the features output.
pub(crate) const FEATURES: &str = "features";

/// The file name of output `role` of the run with `seed`.
pub(crate) fn file_name(seed: u64, role: &str) -> String {
    match role {
        PACKAGE => format!("s{seed}.theosmap"),
        FEATURES => format!("s{seed}-features.geojson"),
        render => format!("s{seed}-{render}.png"),
    }
}

/// The temporary name an output is written under.
fn part_name(name: &str) -> String {
    format!(".{name}.part")
}

/// How far one output got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Its names are claimed; nothing is on disk.
    Claimed,
    /// Its part file exists.
    Part,
    /// It has its name.
    Named,
}

/// One output of the run.
#[derive(Debug)]
struct Output {
    role: String,
    name: String,
    state: State,
}

/// The outputs of one run in its folder.
#[derive(Debug)]
pub(crate) struct Outputs {
    dir: PathBuf,
    entries: Vec<Output>,
    committed: bool,
}

/// The error for `path`.
fn io(path: &Path) -> impl FnOnce(std::io::Error) -> RunError + '_ {
    move |source| RunError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// [`RunError::OutputExists`] when anything, a dangling link included, has the name `path`.
fn refuse_existing(path: &Path) -> Result<(), RunError> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Err(RunError::OutputExists {
            path: path.to_path_buf(),
        }),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io(path)(error)),
    }
}

impl Outputs {
    /// Claims the names of `roles` (and their part names) in the existing folder `dir`, before
    /// anything is written (plan ruling L10).
    ///
    /// # Errors
    /// [`RunError::Io`] when `dir` cannot be read or is not a folder; [`RunError::OutputExists`]
    /// naming the first output name or part name that is already taken.
    pub(crate) fn claim(dir: &Path, seed: u64, roles: &[String]) -> Result<Self, RunError> {
        let meta = std::fs::metadata(dir).map_err(io(dir))?;
        if !meta.is_dir() {
            let source = std::io::Error::new(ErrorKind::NotADirectory, "not a folder");
            return Err(io(dir)(source));
        }
        let mut entries = Vec::with_capacity(roles.len());
        for role in roles {
            let name = file_name(seed, role);
            refuse_existing(&dir.join(&name))?;
            refuse_existing(&dir.join(part_name(&name)))?;
            entries.push(Output {
                role: role.clone(),
                name,
                state: State::Claimed,
            });
        }
        Ok(Self {
            dir: dir.to_path_buf(),
            entries,
            committed: false,
        })
    }

    /// Writes `bytes` as the part file of output `role`. The file is created new, so a file
    /// that appeared since the claim is never overwritten.
    ///
    /// # Errors
    /// [`RunError::Io`] when `role` was not claimed or is written already, or the file cannot be
    /// created or written.
    pub(crate) fn write(&mut self, role: &str, bytes: &[u8]) -> Result<(), RunError> {
        let dir = &self.dir;
        let Some(output) = self
            .entries
            .iter_mut()
            .find(|o| o.role == role && o.state == State::Claimed)
        else {
            let source = std::io::Error::new(
                ErrorKind::InvalidInput,
                format!("output `{role}` was not claimed or is written already"),
            );
            return Err(io(dir)(source));
        };
        let path = dir.join(part_name(&output.name));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(io(&path))?;
        output.state = State::Part;
        file.write_all(bytes).map_err(io(&path))
    }

    /// Renames every part file to its output name and returns each role with its file name.
    ///
    /// `std::fs::rename` replaces an existing target on every platform (on Windows too, with
    /// `MOVEFILE_REPLACE_EXISTING` semantics). The names were free at the claim (plan ruling
    /// L10); each is checked again just before its rename, so a file that someone else created
    /// in the folder during the run is refused rather than replaced.
    ///
    /// # Errors
    /// [`RunError::Io`] when an output was never written or a rename fails;
    /// [`RunError::OutputExists`] when a name was taken during the run. Every file of the run is
    /// then removed.
    pub(crate) fn commit(mut self) -> Result<BTreeMap<String, String>, RunError> {
        if let Some(missing) = self.entries.iter().find(|o| o.state != State::Part) {
            let source =
                std::io::Error::other(format!("output `{}` was never written", missing.role));
            return Err(io(&self.dir.join(&missing.name))(source));
        }
        let dir = self.dir.clone();
        for output in &mut self.entries {
            let target = dir.join(&output.name);
            refuse_existing(&target)?;
            std::fs::rename(dir.join(part_name(&output.name)), &target).map_err(io(&target))?;
            output.state = State::Named;
        }
        self.committed = true;
        Ok(self
            .entries
            .iter()
            .map(|o| (o.role.clone(), o.name.clone()))
            .collect())
    }

    /// Removes every file this run created, as far as the file system lets it; the run is
    /// failing already, so a file that cannot be removed adds no second error.
    fn remove_created(&self) {
        for output in &self.entries {
            let path = match output.state {
                State::Claimed => continue,
                State::Part => self.dir.join(part_name(&output.name)),
                State::Named => self.dir.join(&output.name),
            };
            // The run returns its own error, or unwinds: a second error has nowhere to go.
            let _ = std::fs::remove_file(&path);
        }
    }
}

impl Drop for Outputs {
    fn drop(&mut self) {
        if !self.committed {
            self.remove_created();
        }
    }
}

#[cfg(test)]
mod tests;
