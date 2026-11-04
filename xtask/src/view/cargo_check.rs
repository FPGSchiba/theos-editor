//! `cargo check` as the source of the workspace's artifacts and their files.
//!
//! Cargo says what it compiled (`compiler-artifact` messages, read with `cargo_metadata`), and
//! rustc says what each artifact read (its dep-info file, read by [`super::dep_info`]).

use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use cargo_metadata::{Message, PackageId, TargetKind as CargoKind};

use super::dep_info::{dep_info_of, parse_dep_info, repo_relative};
use super::{Artifact, TargetKind, ViewOptions};
use crate::process::{CargoCommand, capture_cargo_showing_stderr, cargo_metadata};
use crate::{Error, Result};

/// The command whose artifacts make the view.
#[must_use]
pub fn check_command(options: ViewOptions) -> CargoCommand {
    let mut args: Vec<String> = [
        "check",
        "--workspace",
        "--all-targets",
        "--message-format=json",
    ]
    .iter()
    .map(|arg| (*arg).to_owned())
    .collect();
    if options.locked {
        args.push("--locked".to_owned());
    }
    if options.offline {
        args.push("--offline".to_owned());
    }
    CargoCommand {
        args,
        env: Vec::new(),
    }
}

/// What the check says to run to read the diagnostics; the error does not quote them, because
/// they can quote the pull request's code.
const READ_THE_DIAGNOSTICS: &str = "run `cargo check --workspace --all-targets --locked` in the \
                                    repository to read the diagnostics";

/// The artifacts of the members of the workspace at `root`, as `cargo check --workspace
/// --all-targets` builds them, each with the files its dep-info lists.
///
/// # Errors
///
/// Fails when cargo cannot give the metadata, cannot be started, or fails to check the
/// workspace (the error names the command to run and does not echo cargo's diagnostics), when
/// its messages are not what was expected, or when an artifact has no readable dep-info.
pub fn artifacts(root: &Path, options: ViewOptions) -> Result<Vec<Artifact>> {
    let metadata = cargo_metadata(root, true, options.locked, options.offline)?;
    let members: BTreeMap<&PackageId, (&str, String)> = metadata
        .packages
        .iter()
        .map(|package| {
            let manifest = package.manifest_path.as_std_path();
            let dir = manifest.parent().unwrap_or(manifest);
            let dir = repo_relative(root, &dir.to_string_lossy());
            (&package.id, (package.name.as_str(), dir))
        })
        .collect();

    let command = check_command(options);
    let failed = |message: &str| Error::Command {
        program: "cargo".to_owned(),
        args: command.args.clone(),
        status: None,
        stderr: message.to_owned(),
    };
    let stdout = capture_cargo_showing_stderr(root, &command)?
        .ok_or_else(|| failed(READ_THE_DIAGNOSTICS))?;

    let mut artifacts = Vec::new();
    for message in Message::parse_stream(Cursor::new(stdout)) {
        let message = message.map_err(|_| Error::Parse {
            what: "the output of `cargo check`".to_owned(),
            message: "a line is not a cargo message".to_owned(),
        })?;
        match message {
            Message::BuildFinished(finished) if !finished.success => {
                return Err(failed(READ_THE_DIAGNOSTICS));
            }
            Message::CompilerArtifact(artifact) => {
                let Some((package, manifest_dir)) = members.get(&artifact.package_id) else {
                    continue;
                };
                let kind = target_kind(artifact.target.kind.first())?;
                let first = artifact.filenames.first().ok_or_else(|| Error::Parse {
                    what: "a cargo artifact".to_owned(),
                    message: "it has no files".to_owned(),
                })?;
                let dep_info = find_dep_info(first.as_std_path(), kind)?;
                let text = fs::read_to_string(&dep_info).map_err(|source| Error::Io {
                    path: dep_info.clone(),
                    source,
                })?;
                artifacts.push(Artifact {
                    package: (*package).to_owned(),
                    manifest_dir: manifest_dir.clone(),
                    kind,
                    name: artifact.target.name.clone(),
                    src_path: repo_relative(root, artifact.target.src_path.as_str()),
                    test: artifact.profile.test,
                    files: parse_dep_info(&text)
                        .iter()
                        .map(|file| repo_relative(root, file))
                        .collect::<std::collections::BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                });
            }
            _ => {}
        }
    }
    Ok(artifacts)
}

fn target_kind(kind: Option<&CargoKind>) -> Result<TargetKind> {
    Ok(match kind {
        Some(CargoKind::Bin) => TargetKind::Bin,
        Some(CargoKind::Test) => TargetKind::Test,
        Some(CargoKind::Bench) => TargetKind::Bench,
        Some(CargoKind::Example) => TargetKind::Example,
        Some(CargoKind::CustomBuild) => TargetKind::BuildScript,
        Some(
            CargoKind::Lib
            | CargoKind::ProcMacro
            | CargoKind::CDyLib
            | CargoKind::DyLib
            | CargoKind::RLib
            | CargoKind::StaticLib,
        ) => TargetKind::Lib,
        _ => {
            return Err(Error::Parse {
                what: "a cargo artifact".to_owned(),
                message: "its target kind is not one this gate knows".to_owned(),
            });
        }
    })
}

/// The dep-info file of the artifact whose first file is `first`.
///
/// A build script's first file can be `build-script-build[.exe]`, without a hash (cargo links
/// it to the hashed one beside it); its dep-info is then the one `build_script_build-*.d` in
/// the same folder, which cargo gives to this script alone.
fn find_dep_info(first: &Path, kind: TargetKind) -> Result<PathBuf> {
    let named = dep_info_of(first);
    if named.is_file() || kind != TargetKind::BuildScript {
        return Ok(named);
    }
    let folder = first.parent().unwrap_or(first);
    let listing = fs::read_dir(folder).map_err(|source| Error::Io {
        path: folder.to_path_buf(),
        source,
    })?;
    let mut found = listing
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name().is_some_and(|name| {
                let name = name.to_string_lossy();
                name.starts_with("build_script_build-")
                    && Path::new(&*name)
                        .extension()
                        .is_some_and(|extension| extension == "d")
            })
        });
    match (found.next(), found.next()) {
        (Some(only), None) => Ok(only),
        _ => Ok(named),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_check_covers_the_workspace_and_every_target() {
        let args = |options| check_command(options).args;
        assert_eq!(
            args(ViewOptions::REPOSITORY),
            [
                "check",
                "--workspace",
                "--all-targets",
                "--message-format=json",
                "--locked"
            ]
        );
        assert_eq!(
            args(ViewOptions::FIXTURE),
            [
                "check",
                "--workspace",
                "--all-targets",
                "--message-format=json",
                "--offline"
            ]
        );
    }
}
