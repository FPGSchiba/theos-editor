//! Installing git's `pre-commit` hook for a content project (spec §5.8): the hooks folder, whether
//! it may be written, whether an existing hook is ours, and the backup.

use std::path::{Component, Path, PathBuf};

use theos_vcs::{Git, repo};

use crate::StudioError;
use crate::top::repo_top;

/// The marker line every hook of ours holds.
pub const MARKER: &str = "# theos pre-commit hook";

/// The name of a foreign hook that `--force` keeps.
const BACKUP: &str = "pre-commit.before-theos";

/// What an install did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    /// The hook file.
    pub hook: PathBuf,
    /// Whether an earlier hook of ours for the same project was replaced.
    pub replaced: bool,
    /// Where the foreign hook was kept, with `force`.
    pub backup: Option<PathBuf>,
}

/// Why a hook was not installed.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum HookError {
    /// The hooks folder lies outside the repository's git folder.
    #[error(
        "the hooks folder `{}` is outside the git folder `{}`: a global `core.hooksPath` would \
         install the hook into every repository of the user, and a tracked hooks folder would \
         commit a path that only fits this machine",
        hooks.display(),
        git_folder.display()
    )]
    OutsideGitFolder {
        /// The hooks folder git runs hooks from.
        hooks: PathBuf,
        /// The repository's (common) git folder.
        git_folder: PathBuf,
    },
    /// A `pre-commit` exists that is not ours for this project.
    #[error(
        "`{}` exists and is not theos's hook for this project; --force keeps it as \
         pre-commit.before-theos",
        path.display()
    )]
    Foreign {
        /// The existing hook.
        path: PathBuf,
    },
    /// The path of the running `theos` is not UTF-8, so the hook cannot name it.
    #[error("the hook cannot name `{}`: its path is not valid UTF-8", path.display())]
    TheosPath {
        /// The path of `theos`.
        path: PathBuf,
    },
    /// `--force` would overwrite an earlier backup.
    #[error(
        "`{}` exists already; theos never overwrites a backup, move it away first",
        path.display()
    )]
    BackupExists {
        /// The backup.
        path: PathBuf,
    },
}

/// Writes the `pre-commit` hook of the project at `project_root`, which checks the staged
/// snapshot with the `theos` at `theos` and falls back to `theos` on the `PATH`.
///
/// An existing hook with the marker and the same project folder is replaced. Any other hook, a
/// symbolic link included whatever it points to, is [`HookError::Foreign`], unless `force` keeps
/// it as `pre-commit.before-theos`. Nothing is written, and no folder created, when a check
/// refuses, and nothing is read or written through a link.
///
/// # Errors
///
/// [`StudioError::NotInRepository`] when the project is in no repository, [`StudioError::Hook`]
/// when the hooks folder is outside the git folder, a foreign hook is there without `force`, or
/// the backup exists, [`StudioError::Git`] when git fails and [`StudioError::Io`] when a file
/// operation fails.
pub fn install_hook(
    project_root: &Path,
    git: &Git,
    theos: &Path,
    force: bool,
) -> Result<Installed, StudioError> {
    let top = repo_top(git, project_root)?;
    let place = repo::place(&top, project_root)?;
    let hooks = repo::hooks_dir(git, &top)?;
    let common = repo::common_dir(git, &top)?;
    check_inside(&hooks, &common)?;

    let folder = if place.is_empty() { "." } else { &place };
    let theos = theos
        .to_str()
        .ok_or_else(|| HookError::TheosPath {
            path: theos.to_path_buf(),
        })?
        .replace('\\', "/");
    let text = script(&theos, folder);
    let hook = hooks.join("pre-commit");
    let mut installed = Installed {
        hook: hook.clone(),
        replaced: false,
        backup: None,
    };
    if present(&hook) {
        // A link is never ours: reading or writing through it would reach the file it points
        // to, which may be tracked. `force` moves the link itself away.
        if !is_link(&hook) && is_ours(&hook, folder) {
            installed.replaced = true;
        } else if !force {
            return Err(HookError::Foreign { path: hook }.into());
        } else {
            let backup = hooks.join(BACKUP);
            if present(&backup) {
                return Err(HookError::BackupExists { path: backup }.into());
            }
            std::fs::rename(&hook, &backup).map_err(|source| io(&hook, source))?;
            installed.backup = Some(backup);
        }
    }
    std::fs::create_dir_all(&hooks).map_err(|source| io(&hooks, source))?;
    std::fs::write(&hook, text).map_err(|source| io(&hook, source))?;
    make_executable(&hook).map_err(|source| io(&hook, source))?;
    Ok(installed)
}

/// Whether something is at `path`, a dangling symbolic link included.
fn present(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// Whether `path` is a symbolic link, whatever it points to.
fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_symlink())
}

fn io(path: &Path, source: std::io::Error) -> StudioError {
    StudioError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Checks that `hooks`, which may not exist yet, lies inside the git folder `common`: its nearest
/// existing ancestor is canonicalized and the rest of the path is appended.
fn check_inside(hooks: &Path, common: &Path) -> Result<(), StudioError> {
    let common = std::fs::canonicalize(common).map_err(|source| io(common, source))?;
    let outside = || HookError::OutsideGitFolder {
        hooks: hooks.to_path_buf(),
        git_folder: common.clone(),
    };
    let Some(existing) = hooks.ancestors().find(|path| path.exists()) else {
        return Err(outside().into());
    };
    let rest = hooks.strip_prefix(existing).map_err(|_| outside())?;
    if rest.components().any(|part| part == Component::ParentDir) {
        return Err(outside().into());
    }
    let resolved = std::fs::canonicalize(existing)
        .map_err(|source| io(existing, source))?
        .join(rest);
    if resolved.starts_with(&common) {
        Ok(())
    } else {
        Err(outside().into())
    }
}

/// Whether the hook at `path` is ours for `folder`: the marker and the exact `exec` line.
fn is_ours(path: &Path, folder: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let exec = exec_line(folder);
    text.contains(MARKER) && text.lines().any(|line| line == exec)
}

/// The line that runs the check for `folder`.
fn exec_line(folder: &str) -> String {
    format!("exec \"$theos\" check --staged {}", quoted(folder))
}

/// The text of the hook.
fn script(theos: &str, folder: &str) -> String {
    format!(
        "#!/bin/sh\n\
         {MARKER}, written by `theos hook install` (CE-17.2).\n\
         # It checks the staged snapshot and refuses the commit on any error.\n\
         theos={}\n\
         [ -x \"$theos\" ] || theos=theos\n\
         {}\n",
        quoted(theos),
        exec_line(folder)
    )
}

/// `text` in single quotes for `sh`, each `'` written as `'\''`.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// Gives the hook mode 0755.
#[cfg(unix)]
fn make_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

/// Has no mode to set: Git for Windows runs `sh` hooks without an executable bit.
#[cfg(windows)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "same signature as the unix fn, which can fail"
)]
fn make_executable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_quotes_the_path() {
        assert_eq!(
            script("C:/Program Files/it's/theos.exe", "[draft] Inhalt ä"),
            "#!/bin/sh\n\
             # theos pre-commit hook, written by `theos hook install` (CE-17.2).\n\
             # It checks the staged snapshot and refuses the commit on any error.\n\
             theos='C:/Program Files/it'\\''s/theos.exe'\n\
             [ -x \"$theos\" ] || theos=theos\n\
             exec \"$theos\" check --staged '[draft] Inhalt ä'\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_hook_is_executable() -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("pre-commit");
        std::fs::write(&file, "x")?;
        make_executable(&file)?;
        assert_eq!(
            std::fs::metadata(&file)?.permissions().mode() & 0o777,
            0o755
        );
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn the_hook_is_written_without_a_mode() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("pre-commit");
        std::fs::write(&file, "x")?;
        make_executable(&file)?;
        assert!(file.is_file());
        Ok(())
    }
}
