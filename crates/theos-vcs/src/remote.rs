//! The environment of a command that talks to a remote, and `fetch`.
//!
//! theos never reads, stores or passes a credential: git asks the user's credential helper. What
//! theos does is stop git from asking anywhere else: no terminal prompt and no askpass program.

use std::ffi::OsStr;
use std::path::Path;

use crate::repo;
use crate::write_args::{checked, write_args_with};
use crate::{Cancel, Git, GitError, Output};

/// The `GIT_SSH_COMMAND` that makes ssh fail instead of asking.
const BATCH_SSH: &str = "ssh -o BatchMode=yes";

/// What a command that talks to a remote adds to git's environment, per call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteEnv {
    /// Variables to set.
    pub(crate) set: Vec<(String, String)>,
    /// Variables to remove from git's environment.
    pub(crate) removed: Vec<String>,
    /// Arguments that go before the command.
    pub(crate) config: Vec<String>,
}

/// The `GIT_SSH_COMMAND` to set: [`BATCH_SSH`] only when the user has set none of
/// `GIT_SSH_COMMAND`, `GIT_SSH` and `core.sshCommand`.
fn ssh_command(
    env_ssh_command: Option<&OsStr>,
    env_ssh: Option<&OsStr>,
    config: Option<&str>,
) -> Option<&'static str> {
    match (env_ssh_command, env_ssh, config) {
        (None, None, None) => Some(BATCH_SSH),
        _ => None,
    }
}

/// Builds the [`RemoteEnv`] for the repository at `top`.
///
/// # Errors
///
/// The errors of [`repo::value`].
pub(crate) fn remote_env(git: &Git, top: &Path) -> Result<RemoteEnv, GitError> {
    let from_env = |name: &str| {
        git.env_value(name)
            .map(OsStr::to_owned)
            .or_else(|| std::env::var_os(name))
    };
    let (env_ssh_command, env_ssh) = (from_env("GIT_SSH_COMMAND"), from_env("GIT_SSH"));
    let config = repo::value(git, top, "core.sshCommand")?;
    let mut set = vec![
        ("GIT_TERMINAL_PROMPT".to_owned(), "0".to_owned()),
        ("SSH_ASKPASS_REQUIRE".to_owned(), "never".to_owned()),
    ];
    if let Some(command) = ssh_command(
        env_ssh_command.as_deref(),
        env_ssh.as_deref(),
        config.as_deref(),
    ) {
        set.push(("GIT_SSH_COMMAND".to_owned(), command.to_owned()));
    }
    Ok(RemoteEnv {
        set,
        removed: vec!["GIT_ASKPASS".to_owned(), "SSH_ASKPASS".to_owned()],
        config: vec!["-c".to_owned(), "core.askPass=".to_owned()],
    })
}

/// Runs a write with the remote environment and returns git's whole answer, whatever its exit
/// code. stdin is empty.
///
/// # Errors
///
/// [`GitError::Cancelled`] when `cancel` is set, the errors of [`remote_env`], and
/// [`GitError::Io`] when git cannot be run.
pub(crate) fn remote_write(
    git: &Git,
    top: &Path,
    command: &str,
    flags: &[&str],
    cancel: &Cancel,
) -> Result<Output, GitError> {
    let env = remote_env(git, top)?;
    // `env.config` holds `-c` pairs; `write_args_with` adds the `-c` itself.
    let settings: Vec<&str> = env
        .config
        .iter()
        .map(String::as_str)
        .filter(|part| *part != "-c")
        .collect();
    let vars: Vec<(&str, &str)> = env
        .set
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let removed: Vec<&str> = env.removed.iter().map(String::as_str).collect();
    git.output_without(
        &write_args_with(&settings, command, flags),
        top,
        None,
        &vars,
        &removed,
        cancel,
    )
}

/// Fetches `remote` into its remote-tracking refs, never touching the working tree or the index.
///
/// # Errors
///
/// [`GitError::Cancelled`], and [`GitError::Failed`] with git's own message when the fetch
/// fails, for instance when no credential helper answers.
pub fn fetch(git: &Git, top: &Path, remote: &str, cancel: &Cancel) -> Result<(), GitError> {
    let flags = ["--quiet", remote];
    let out = remote_write(git, top, "fetch", &flags, cancel)?;
    checked(&write_args_with(&[], "fetch", &flags), out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(text: &str) -> &OsStr {
        OsStr::new(text)
    }

    #[test]
    fn batch_mode_when_the_user_set_nothing() {
        assert_eq!(ssh_command(None, None, None), Some("ssh -o BatchMode=yes"));
    }

    #[test]
    fn the_users_ssh_command_is_kept() {
        assert_eq!(ssh_command(Some(os("ssh -i k")), None, None), None);
        assert_eq!(ssh_command(None, Some(os("plink")), None), None);
        assert_eq!(ssh_command(None, None, Some("ssh -F c")), None);
    }
}
