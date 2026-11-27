//! The tools of the `ui` check, which it verifies before its steps, so only a job that runs `ui`
//! needs them: Node (major version 24) and pnpm (the version of the `packageManager` pin of
//! `editor-ui/package.json`). The `tools` check does not read them: it runs first in every gate
//! job, and most jobs have no Node.
//!
//! The judging is pure ([`judge_node`], [`judge_pnpm`], [`package_manager_pin`], [`judge`]);
//! [`check`] asks the real `node` and `pnpm`, found through `which`, for their versions.

use std::path::Path;
use std::process::Command;

use crate::verdict::Finding;

/// The major version of Node the UI is built with.
const NODE_MAJOR: u32 = 24;

/// The version in `packageManager` of a `package.json` when it is `pnpm@<version>` (a `+sha`
/// suffix is dropped); `None` when the field is missing, is not text, or names another manager.
#[must_use]
pub fn package_manager_pin(package_json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(package_json).ok()?;
    let pin = value.get("packageManager")?.as_str()?;
    let version = pin.strip_prefix("pnpm@")?;
    Some(version.split('+').next()?.to_owned())
}

/// A finding when `output` of `node --version` (such as `v24.1.0`) is not major 24.
#[must_use]
pub fn judge_node(output: &str) -> Option<Finding> {
    let major = output
        .trim()
        .strip_prefix('v')
        .and_then(|rest| rest.split('.').next())
        .and_then(|major| major.parse::<u32>().ok());
    match major {
        Some(NODE_MAJOR) => None,
        _ => Some(Finding::new(format!(
            "node must be major version {NODE_MAJOR} for editor-ui; `node --version` says {:?}",
            output.trim()
        ))),
    }
}

/// A finding when `output` of `pnpm --version` is not `pin`, the `packageManager` pin.
#[must_use]
pub fn judge_pnpm(output: &str, pin: &str) -> Option<Finding> {
    if output.trim() == pin {
        return None;
    }
    Some(Finding::new(format!(
        "pnpm {pin} is pinned by `packageManager` in editor-ui/package.json; `pnpm --version` \
         says {:?}",
        output.trim()
    )))
}

/// `<program> --version` run in `dir`: Corepack applies the `packageManager` pin only inside the
/// package folder, and `ui::run` runs there too.
fn version_command(program: &Path, dir: &Path) -> Command {
    let mut command = Command::new(program);
    command.arg("--version").current_dir(dir);
    command
}

/// What `<program> --version` prints in `dir`, or `None` when it is not installed or fails.
fn version_of(program: &str, dir: &Path) -> Option<String> {
    let path = which::which(program).ok()?;
    let output = version_command(&path, dir).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The findings about node and pnpm, given the `package.json` text and what `node --version` and
/// `pnpm --version` printed (`None` when the program is not installed or fails).
#[must_use]
pub fn judge(package_json: &str, node: Option<&str>, pnpm: Option<&str>) -> Vec<Finding> {
    let mut findings = Vec::new();
    match node {
        Some(output) => findings.extend(judge_node(output)),
        None => findings.push(Finding::new(format!(
            "node is not installed; editor-ui needs major version {NODE_MAJOR}"
        ))),
    }
    match (pnpm, package_manager_pin(package_json)) {
        (_, None) => findings.push(Finding::new(
            "editor-ui/package.json has no `packageManager` field pinning pnpm@<version>",
        )),
        (None, Some(pin)) => findings.push(Finding::new(format!(
            "pnpm is not installed; editor-ui/package.json pins {pin}"
        ))),
        (Some(output), Some(pin)) => findings.extend(judge_pnpm(output, &pin)),
    }
    findings
}

/// The findings about node and pnpm in the repository at `root`: none when there is no
/// `editor-ui/package.json`.
#[must_use]
pub fn check(root: &Path) -> Vec<Finding> {
    let Ok(package_json) = std::fs::read_to_string(root.join("editor-ui").join("package.json"))
    else {
        return Vec::new();
    };
    let dir = root.join("editor-ui");
    judge(
        &package_json,
        version_of("node", &dir).as_deref(),
        version_of("pnpm", &dir).as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pin_is_the_version_after_pnpm_at() {
        assert_eq!(
            package_manager_pin(r#"{"packageManager":"pnpm@10.4.1"}"#).as_deref(),
            Some("10.4.1")
        );
        assert_eq!(
            package_manager_pin(r#"{"packageManager":"pnpm@10.4.1+sha512.ab"}"#).as_deref(),
            Some("10.4.1")
        );
        assert_eq!(
            package_manager_pin(r#"{"packageManager":"yarn@4.0.0"}"#),
            None
        );
        assert_eq!(package_manager_pin("{}"), None);
        assert_eq!(package_manager_pin("not json"), None);
    }

    #[test]
    fn versions_are_asked_inside_editor_ui() {
        let dir = Path::new("repo").join("editor-ui");
        let command = version_command(Path::new("pnpm"), &dir);
        assert_eq!(command.get_current_dir(), Some(dir.as_path()));
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, ["--version"]);
    }

    #[test]
    fn node_must_be_major_24() {
        assert_eq!(judge_node("v24.11.0\n"), None);
        assert!(judge_node("v22.1.0").is_some());
        assert!(judge_node("v240.0.0").is_some());
        assert!(judge_node("garbage").is_some());
    }

    #[test]
    fn pnpm_must_equal_the_pin() {
        assert_eq!(judge_pnpm("10.4.1\n", "10.4.1"), None);
        assert!(judge_pnpm("10.4.0", "10.4.1").is_some());
    }

    // AC: CE-32.3
    #[test]
    fn a_wrong_node_major_or_pnpm_version_is_a_finding() {
        let package = r#"{"packageManager":"pnpm@10.4.1"}"#;
        assert_eq!(judge(package, Some("v24.1.0"), Some("10.4.1")), Vec::new());
        let wrong_node = judge(package, Some("v22.1.0"), Some("10.4.1"));
        assert_eq!(wrong_node.len(), 1);
        assert!(
            wrong_node[0]
                .message
                .contains("node must be major version 24")
        );
        let wrong_pnpm = judge(package, Some("v24.1.0"), Some("9.0.0"));
        assert_eq!(wrong_pnpm.len(), 1);
        assert!(wrong_pnpm[0].message.contains("pnpm 10.4.1 is pinned"));
        // Missing programs and a missing pin are findings too.
        assert_eq!(judge(package, None, None).len(), 2);
        assert_eq!(judge("{}", Some("v24.1.0"), Some("10.4.1")).len(), 1);
    }

    #[test]
    fn without_editor_ui_there_is_nothing_to_check() -> std::io::Result<()> {
        let root = tempfile::tempdir()?;
        assert_eq!(check(root.path()).len(), 0);
        Ok(())
    }
}
