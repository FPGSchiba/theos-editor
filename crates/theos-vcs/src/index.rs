//! The paths git's index holds, in git's spelling (`ls-files`).

use std::path::Path;

use crate::Cancel;
use crate::git::{Git, GitError};
use crate::read_args::read_args;

/// The paths of the index of the repository at `top` (the one the runner's environment names,
/// such as a hook's `GIT_INDEX_FILE`): git's bytes, relative to `top`, with `/`, in the index's
/// order, each once (a conflicted path is listed once, not once per stage). Intent-to-add
/// entries (`git add -N`) are listed; untracked and ignored files are not.
///
/// The whole index is listed, without a pathspec: a pathspec compares bytes, so a folder spelled
/// otherwise on disk than in the index (a case-only difference under `core.ignorecase`, NFD
/// under `core.precomposeunicode`) would miss its entries. The caller picks its paths with
/// [`Folding`](crate::Folding) instead.
///
/// # Errors
///
/// The errors of [`Git::run_cancellable`], and [`GitError::Cancelled`] when `cancel` is set.
pub fn paths(git: &Git, top: &Path, cancel: &Cancel) -> Result<Vec<Vec<u8>>, GitError> {
    let args = read_args("ls-files", &["--cached", "-z"]);
    let out = git.run_cancellable(&args, top, None, &[], cancel)?;
    Ok(split(&out))
}

/// The NUL-terminated paths of `out`, with a path repeated next to itself listed once.
fn split(out: &[u8]) -> Vec<Vec<u8>> {
    let mut paths: Vec<Vec<u8>> = out
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(<[u8]>::to_vec)
        .collect();
    paths.dedup();
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_reads_nul_terminated_paths_once_each() {
        assert_eq!(
            split(b"a b.md\0c/[x].md\0c/[x].md\0c/[x].md\0d.md\0"),
            [b"a b.md".to_vec(), b"c/[x].md".to_vec(), b"d.md".to_vec()]
        );
        assert_eq!(split(b""), Vec::<Vec<u8>>::new());
    }
}
