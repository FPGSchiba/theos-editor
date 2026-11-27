//! The tools of the `ui` check, which the `tools` check verifies only when `editor-ui/` exists:
//! Node (major version 24) and pnpm (the version of the `packageManager` pin of
//! `editor-ui/package.json`).
//!
//! The judging is pure ([`judge_node`], [`judge_pnpm`], [`package_manager_pin`]); [`check`] asks
//! the real `node` and `pnpm`, found through `which`, for their versions.

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

/// What `<program> --version` prints, or `None` when it is not installed or fails.
fn version_of(program: &str) -> Option<String> {
    let path = which::which(program).ok()?;
    let output = Command::new(path).arg("--version").output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The findings about node and pnpm in the repository at `root`: none when there is no
/// `editor-ui/package.json`.
#[must_use]
pub fn check(root: &Path) -> Vec<Finding> {
    let Ok(package_json) = std::fs::read_to_string(root.join("editor-ui").join("package.json"))
    else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    match version_of("node") {
        Some(output) => findings.extend(judge_node(&output)),
        None => findings.push(Finding::new(format!(
            "node is not installed; editor-ui needs major version {NODE_MAJOR}"
        ))),
    }
    match (version_of("pnpm"), package_manager_pin(&package_json)) {
        (_, None) => findings.push(Finding::new(
            "editor-ui/package.json has no `packageManager` field pinning pnpm@<version>",
        )),
        (None, Some(pin)) => findings.push(Finding::new(format!(
            "pnpm is not installed; editor-ui/package.json pins {pin}"
        ))),
        (Some(output), Some(pin)) => findings.extend(judge_pnpm(&output, &pin)),
    }
    findings
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

    #[test]
    fn without_editor_ui_there_is_nothing_to_check() -> std::io::Result<()> {
        let root = tempfile::tempdir()?;
        assert_eq!(check(root.path()).len(), 0);
        Ok(())
    }
}
