//! The newest bytes of a changed path: the working tree, else the index, else `HEAD`.

use std::path::Path;

use theos_vcs::read::{Content, Version, read};
use theos_vcs::{Git, GitError};

/// The versions a path is read at, newest first.
const NEWEST_FIRST: [Version<'static>; 3] = [Version::WorkTree, Version::Index, Version::Head];

/// The bytes of the repository path `path` at its newest version that holds it, or `None` when
/// that version holds no regular file (a link, a submodule, a folder) or no version holds it.
///
/// # Errors
///
/// The errors of [`read`].
pub(super) fn newest(git: &Git, top: &Path, path: &[u8]) -> Result<Option<Vec<u8>>, GitError> {
    for version in NEWEST_FIRST {
        match read(git, top, version, path)? {
            Content::Absent => {}
            Content::File(bytes) => return Ok(Some(bytes)),
            _ => return Ok(None),
        }
    }
    Ok(None)
}
