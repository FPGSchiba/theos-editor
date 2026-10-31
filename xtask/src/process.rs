//! Running cargo as a child of the gate.
//!
//! The checks that are cargo commands (`fmt`, `clippy`, `test`, `docs`, `deny`) describe what to
//! run as [`CargoCommand`] values, which keeps their arguments testable without running them.
//! The child gets an argument list, never a shell.

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

use crate::{Error, Result};

/// One cargo invocation: its arguments after `cargo`, and the environment variables to set on
/// the child on top of the inherited ones.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CargoCommand {
    /// The arguments, such as `["clippy", "--workspace", "--locked"]`.
    pub args: Vec<String>,
    /// Variables to set for the child, such as `("RUSTDOCFLAGS", "-D warnings")`.
    pub env: Vec<(String, String)>,
}

/// Runs cargo in `root` with the command's arguments and environment, with stdin, stdout and
/// stderr inherited, and says whether it exited with 0.
///
/// The cargo that started `cargo xtask` (`$CARGO`) is used when there is one, so the checks run
/// with the same toolchain.
///
/// # Errors
///
/// Fails when cargo cannot be started. A cargo that runs and fails is `Ok(false)`.
pub fn run_cargo(root: &Path, cmd: &CargoCommand) -> Result<bool> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    run_program(&cargo, root, cmd)
}

fn run_program(program: &OsStr, root: &Path, cmd: &CargoCommand) -> Result<bool> {
    tracing::debug!(
        program = %program.to_string_lossy(),
        args = ?cmd.args,
        env = ?cmd.env,
        root = %root.display(),
        "run"
    );
    let status = command(program, root, cmd)
        .status()
        .map_err(|source| Error::Command {
            program: program.to_string_lossy().into_owned(),
            args: cmd.args.clone(),
            status: None,
            stderr: source.to_string(),
        })?;
    Ok(status.success())
}

fn command(program: &OsStr, root: &Path, cmd: &CargoCommand) -> Command {
    let mut command = Command::new(program);
    command
        .current_dir(root)
        .args(&cmd.args)
        .envs(cmd.env.iter().map(|(key, value)| (key, value)));
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn cmd(args: &[&str], env: &[(&str, &str)]) -> CargoCommand {
        CargoCommand {
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            env: env
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }

    #[test]
    fn the_command_has_the_arguments_environment_and_directory() {
        let root = Path::new("some").join("root");
        let command = command(
            OsStr::new("cargo"),
            &root,
            &cmd(&["doc", "--locked"], &[("RUSTDOCFLAGS", "-D warnings")]),
        );
        assert_eq!(command.get_program(), "cargo");
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [OsStr::new("doc"), OsStr::new("--locked")]
        );
        assert_eq!(
            command.get_envs().collect::<Vec<_>>(),
            [(OsStr::new("RUSTDOCFLAGS"), Some(OsStr::new("-D warnings")))]
        );
        assert_eq!(command.get_current_dir(), Some(root.as_path()));
    }

    #[test]
    fn the_exit_status_decides_the_answer() -> crate::Result<()> {
        let root = std::env::current_dir().map_err(|source| crate::Error::Io {
            path: PathBuf::from("."),
            source,
        })?;
        let git = OsStr::new("git");
        // `check-ref-format` says nothing and exits 0 or 1, so the test output stays clean.
        assert!(run_program(
            git,
            &root,
            &cmd(&["check-ref-format", "refs/heads/ok"], &[])
        )?);
        assert!(!run_program(
            git,
            &root,
            &cmd(&["check-ref-format", "refs/heads/.."], &[])
        )?);
        Ok(())
    }

    #[test]
    fn a_program_that_cannot_start_is_a_command_error() {
        let root = std::env::temp_dir();
        let result = run_program(
            OsStr::new("xtask-no-such-program"),
            &root,
            &cmd(&["--flag"], &[]),
        );
        assert!(
            matches!(
                &result,
                Err(crate::Error::Command { program, args, status: None, .. })
                    if program == "xtask-no-such-program" && args == &["--flag"]
            ),
            "{result:?}"
        );
    }
}
