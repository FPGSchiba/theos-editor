//! Running cargo as a child of the gate.
//!
//! The checks that are cargo commands (`fmt`, `clippy`, `test`, `docs`, `deny`) describe what to
//! run as [`CargoCommand`] values, which keeps their arguments testable without running them.
//! The child gets an argument list, never a shell. [`run_cargo`] shows the child's output;
//! [`capture_cargo`] reads it, and [`capture_cargo_showing_stderr`] reads it while cargo's stderr
//! stays visible.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use cargo_metadata::{Metadata, MetadataCommand};

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

/// Runs cargo in `root` like [`run_cargo`], but captures stdout instead of showing it, drops
/// stderr, and gives cargo no stdin. This is for asking a question, such as a tool's version.
///
/// Returns what cargo wrote to stdout when it exited with 0, and `None` when it exited with
/// anything else, which is what cargo does for a subcommand that is not installed.
///
/// # Errors
///
/// Fails when cargo cannot be started.
pub fn capture_cargo(root: &Path, cmd: &CargoCommand) -> Result<Option<String>> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    capture_program(&cargo, root, cmd)
}

/// Runs cargo in `root` like [`capture_cargo`], but lets cargo's stderr through to ours, so a
/// long command shows its progress and its diagnostics to the person who runs it.
///
/// Returns what cargo wrote to stdout when it exited with 0, and `None` otherwise.
///
/// # Errors
///
/// Fails when cargo cannot be started.
pub fn capture_cargo_showing_stderr(root: &Path, cmd: &CargoCommand) -> Result<Option<String>> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    tracing::debug!(args = ?cmd.args, root = %root.display(), "capture, showing stderr");
    let output = command(&cargo, root, cmd)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|source| Error::Command {
            program: cargo.to_string_lossy().into_owned(),
            args: cmd.args.clone(),
            status: None,
            stderr: source.to_string(),
        })?;
    Ok(output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned()))
}

/// The metadata of the workspace at `root`, without the dependencies when `no_deps`.
///
/// # Errors
///
/// Fails when cargo cannot be started, fails, or writes something else than the metadata.
pub(crate) fn cargo_metadata(
    root: &Path,
    no_deps: bool,
    locked: bool,
    offline: bool,
) -> Result<Metadata> {
    let mut args = vec!["metadata", "--format-version", "1"];
    for (wanted, flag) in [
        (no_deps, "--no-deps"),
        (locked, "--locked"),
        (offline, "--offline"),
    ] {
        if wanted {
            args.push(flag);
        }
    }
    let command = CargoCommand {
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        env: Vec::new(),
    };
    let stdout = capture_cargo(root, &command)?.ok_or_else(|| Error::Missing {
        what: format!(
            "the metadata of the workspace: `cargo {}` failed, and what it said can quote the              manifests of the pull request, so it is not shown; run it in the repository to              read it",
            args.join(" ")
        ),
    })?;
    // Cargo writes the metadata as one line of JSON.
    let json = stdout
        .lines()
        .find(|line| line.starts_with('{'))
        .ok_or_else(|| metadata_parse_error("it has no JSON"))?;
    MetadataCommand::parse(json).map_err(|_| metadata_parse_error("it is not what was expected"))
}

fn metadata_parse_error(message: &str) -> Error {
    Error::Parse {
        what: "the output of `cargo metadata`".to_owned(),
        message: message.to_owned(),
    }
}

/// Runs `rustc -vV` in `root` with the rustc that belongs to the cargo that runs the gate, and
/// returns what it wrote to stdout. `None` when it exited with anything else.
///
/// The rustc is `$RUSTC` when that is set, else the `rustc` next to `$CARGO` (the same toolchain
/// folder, which is how rustup lays them out), else the `rustc` on the path. Nothing goes through
/// a shell.
///
/// # Errors
///
/// Fails when rustc cannot be started.
pub fn capture_rustc_version(root: &Path) -> Result<Option<String>> {
    let program = rustc_program(
        std::env::var_os("CARGO").as_deref(),
        std::env::var_os("RUSTC").as_deref(),
    );
    let cmd = CargoCommand {
        args: vec!["-vV".to_owned()],
        env: Vec::new(),
    };
    capture_program(&program, root, &cmd)
}

/// The program to run for rustc: `rustc_env` when it is not empty, else `rustc` in the folder of
/// `cargo` when there is one, else plain `rustc`.
fn rustc_program(cargo: Option<&OsStr>, rustc_env: Option<&OsStr>) -> OsString {
    if let Some(rustc) = rustc_env.filter(|rustc| !rustc.is_empty()) {
        return rustc.to_owned();
    }
    let beside = cargo
        .map(Path::new)
        .and_then(Path::parent)
        .filter(|folder| !folder.as_os_str().is_empty())
        .map(|folder| folder.join(format!("rustc{}", std::env::consts::EXE_SUFFIX)))
        .filter(|path| path.is_file());
    beside.map_or_else(|| OsString::from("rustc"), PathBuf::into_os_string)
}

fn capture_program(program: &OsStr, root: &Path, cmd: &CargoCommand) -> Result<Option<String>> {
    tracing::debug!(
        program = %program.to_string_lossy(),
        args = ?cmd.args,
        root = %root.display(),
        "capture"
    );
    let output = command(program, root, cmd)
        .output()
        .map_err(|source| Error::Command {
            program: program.to_string_lossy().into_owned(),
            args: cmd.args.clone(),
            status: None,
            stderr: source.to_string(),
        })?;
    Ok(output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned()))
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
    fn the_output_of_a_successful_program_is_captured() -> crate::Result<()> {
        let root = std::env::current_dir().map_err(|source| crate::Error::Io {
            path: PathBuf::from("."),
            source,
        })?;
        let output = capture_program(OsStr::new("git"), &root, &cmd(&["--version"], &[]))?;
        assert!(
            output.is_some_and(|text| text.starts_with("git version")),
            "git --version did not say `git version`"
        );
        Ok(())
    }

    #[test]
    fn a_program_that_exits_with_failure_has_no_output() -> crate::Result<()> {
        let root = std::env::current_dir().map_err(|source| crate::Error::Io {
            path: PathBuf::from("."),
            source,
        })?;
        let output = capture_program(
            OsStr::new("git"),
            &root,
            &cmd(&["check-ref-format", "refs/heads/.."], &[]),
        )?;
        assert_eq!(output, None);
        Ok(())
    }

    #[test]
    fn rustc_is_the_one_in_the_environment_then_the_one_beside_cargo_then_the_one_on_the_path()
    -> crate::Result<()> {
        let dir = tempfile::tempdir().map_err(|source| crate::Error::Io {
            path: PathBuf::from("temporary directory"),
            source,
        })?;
        let io = |source| crate::Error::Io {
            path: dir.path().to_path_buf(),
            source,
        };
        let suffix = std::env::consts::EXE_SUFFIX;
        let cargo = dir.path().join(format!("cargo{suffix}"));
        let beside = dir.path().join(format!("rustc{suffix}"));
        std::fs::write(&cargo, "").map_err(io)?;

        // No rustc beside cargo: the one on the path.
        assert_eq!(
            rustc_program(Some(cargo.as_os_str()), None),
            OsString::from("rustc")
        );
        std::fs::write(&beside, "").map_err(io)?;
        assert_eq!(
            rustc_program(Some(cargo.as_os_str()), None),
            beside.as_os_str()
        );
        // `$RUSTC` comes first, unless it is empty.
        assert_eq!(
            rustc_program(Some(cargo.as_os_str()), Some(OsStr::new("my-rustc"))),
            OsString::from("my-rustc")
        );
        assert_eq!(
            rustc_program(Some(cargo.as_os_str()), Some(OsStr::new(""))),
            beside.as_os_str()
        );
        // A cargo without a folder (found on the path) or no cargo at all: the one on the path.
        assert_eq!(
            rustc_program(Some(OsStr::new("cargo")), None),
            OsString::from("rustc")
        );
        assert_eq!(rustc_program(None, None), OsString::from("rustc"));
        Ok(())
    }

    #[test]
    fn the_rustc_of_this_toolchain_reports_its_version() -> crate::Result<()> {
        let root = std::env::current_dir().map_err(|source| crate::Error::Io {
            path: PathBuf::from("."),
            source,
        })?;
        let output = capture_rustc_version(&root)?;
        assert!(
            output.is_some_and(|text| text.lines().any(|line| line.starts_with("release: "))),
            "rustc -vV has no release line"
        );
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
