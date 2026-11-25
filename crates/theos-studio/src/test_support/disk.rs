//! The disk around a test: what a folder holds, and a file made read-only for a while.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Every file and folder under `dir`: its path and whether it is a folder.
pub(crate) fn entries(dir: &Path) -> Vec<(PathBuf, bool)> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(folder) = stack.pop() {
        for entry in fs::read_dir(&folder).into_iter().flatten().flatten() {
            let path = entry.path();
            let is_folder = path.is_dir();
            if is_folder {
                stack.push(path.clone());
            }
            found.push((path, is_folder));
        }
    }
    found
}

/// Makes a file read-only until it is dropped, then gives it back its permissions.
pub(crate) struct ReadOnly {
    path: PathBuf,
    permissions: fs::Permissions,
}

impl ReadOnly {
    /// Makes the file `path` read-only.
    pub(crate) fn set(path: PathBuf) -> io::Result<ReadOnly> {
        let permissions = fs::metadata(&path)?.permissions();
        let mut read_only = permissions.clone();
        read_only.set_readonly(true);
        fs::set_permissions(&path, read_only)?;
        Ok(ReadOnly { path, permissions })
    }
}

impl Drop for ReadOnly {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.path, self.permissions.clone());
    }
}
