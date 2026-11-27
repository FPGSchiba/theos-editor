//! The golden file on disk: written once per generator version, never overwritten (CE-77.2).

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use super::GoldenFile;

/// Why a golden file could not be written or read.
#[derive(Debug, thiserror::Error)]
pub enum GoldenError {
    /// The file of this version exists; golden hashes are never overwritten.
    #[error(
        "the golden file {} exists and is never overwritten: raise `VERSION` in \
         crates/theos-maps/src/api.rs to record the hashes of new outputs",
        path.display()
    )]
    Exists {
        /// The golden file.
        path: PathBuf,
    },
    /// The file of this version has not been recorded.
    #[error(
        "there is no golden file {}: run `cargo run -p theos-maps --example bless`",
        path.display()
    )]
    Missing {
        /// The golden file.
        path: PathBuf,
    },
    /// Reading or writing the file failed.
    #[error("{}: {source}", path.display())]
    Io {
        /// The golden file or its folder.
        path: PathBuf,
        /// What the system said.
        source: std::io::Error,
    },
    /// The file is not a golden file.
    #[error("the golden file {} is not valid: {source}", path.display())]
    Parse {
        /// The golden file.
        path: PathBuf,
        /// What the TOML parser said.
        source: toml::de::Error,
    },
    /// The hashes could not be written as TOML, which cannot occur for a [`GoldenFile`] today.
    #[error("the golden hashes cannot be written as TOML: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// Writes `file` as TOML to `path`, creating its folder, and only when no file is there.
///
/// # Errors
/// [`GoldenError::Exists`] when `path` exists, which leaves it untouched; [`GoldenError::Io`]
/// when the folder or the file cannot be written, in which case no partial file is left;
/// [`GoldenError::Serialize`], which cannot occur today.
pub fn record(path: &Path, file: &GoldenFile) -> Result<(), GoldenError> {
    let text = toml::to_string(file)?;
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).map_err(io_at(folder))?;
    }
    // `create_new` refuses an existing file in the same call that creates it: nothing can slip
    // in between a check and the write.
    let mut out = match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(out) => out,
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            return Err(GoldenError::Exists {
                path: path.to_owned(),
            });
        }
        Err(e) => return Err(io_at(path)(e)),
    };
    if let Err(e) = out.write_all(text.as_bytes()).and_then(|()| out.sync_all()) {
        drop(out);
        // Best effort: a partial file would make the next `bless` refuse.
        let _ = fs::remove_file(path);
        return Err(io_at(path)(e));
    }
    Ok(())
}

/// Turns an I/O error at `path` into [`GoldenError::Io`].
fn io_at(path: &Path) -> impl FnOnce(std::io::Error) -> GoldenError {
    move |source| GoldenError::Io {
        path: path.to_owned(),
        source,
    }
}

/// Reads the golden file at `path`.
///
/// # Errors
/// [`GoldenError::Missing`] when there is no file, naming the command that records it;
/// [`GoldenError::Io`]; [`GoldenError::Parse`] when it is not a golden file.
pub fn load(path: &Path) -> Result<GoldenFile, GoldenError> {
    let text = fs::read_to_string(path).map_err(|source| match source.kind() {
        ErrorKind::NotFound => GoldenError::Missing {
            path: path.to_owned(),
        },
        _ => io_at(path)(source),
    })?;
    toml::from_str(&text).map_err(|source| GoldenError::Parse {
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests;
