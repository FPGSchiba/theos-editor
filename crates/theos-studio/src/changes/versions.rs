//! Which version of a changed path names it: the newest one, or for a conflict the first one that
//! is a record.

use std::path::Path;

use theos_vcs::read::{Content, Version, read};
use theos_vcs::{Git, GitError};

/// The versions a path is read at, newest first.
const NEWEST_FIRST: [Version<'static>; 3] = [Version::WorkTree, Version::Index, Version::Head];

/// The versions a conflicted path is read at: the working copy, whose conflict markers may break
/// the front matter, then `HEAD`. The index holds the conflict's stages, not a stage 0.
const CONFLICT_ORDER: [Version<'static>; 2] = [Version::WorkTree, Version::Head];

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

/// What `accept` makes of the first regular file at the conflicted repository path `path`, in the
/// order working copy, `HEAD`, that it accepts; `None` when it accepts none.
///
/// # Errors
///
/// The errors of [`read`].
pub(super) fn first_of_conflict<T>(
    git: &Git,
    top: &Path,
    path: &[u8],
    mut accept: impl FnMut(&[u8]) -> Option<T>,
) -> Result<Option<T>, GitError> {
    for version in CONFLICT_ORDER {
        if let Content::File(bytes) = read(git, top, version, path)?
            && let Some(found) = accept(&bytes)
        {
            return Ok(Some(found));
        }
    }
    Ok(None)
}
