//! The JSON view of `theos hook install`.

use std::path::Path;

use serde::Serialize;

use crate::hook::{HookFile, Installed};

/// The result of `theos hook install` as JSON: the keys of `pre-commit` at the top, and
/// `pre_merge_commit` with the same keys.
#[derive(Debug, Serialize)]
pub struct HookView {
    #[serde(flatten)]
    pre_commit: HookFileView,
    pre_merge_commit: HookFileView,
}

/// One hook file of [`HookView`].
#[derive(Debug, Serialize)]
pub struct HookFileView {
    hook: String,
    replaced: bool,
    backup: Option<String>,
}

impl HookFileView {
    fn new(hook: &Path, replaced: bool, backup: Option<&Path>) -> Self {
        HookFileView {
            hook: hook.display().to_string(),
            replaced,
            backup: backup.map(|backup| backup.display().to_string()),
        }
    }
}

impl From<&HookFile> for HookFileView {
    fn from(file: &HookFile) -> Self {
        HookFileView::new(&file.hook, file.replaced, file.backup.as_deref())
    }
}

impl From<&Installed> for HookView {
    fn from(installed: &Installed) -> Self {
        HookView {
            pre_commit: HookFileView::new(
                &installed.hook,
                installed.replaced,
                installed.backup.as_deref(),
            ),
            pre_merge_commit: HookFileView::from(&installed.pre_merge_commit),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn the_hook_view_has_every_key() -> Result<(), Box<dyn std::error::Error>> {
        let installed = Installed {
            hook: PathBuf::from("hooks/pre-commit"),
            replaced: false,
            backup: None,
            pre_merge_commit: HookFile {
                hook: PathBuf::from("hooks/pre-merge-commit"),
                replaced: true,
                backup: None,
            },
        };
        let value = serde_json::to_value(HookView::from(&installed))?;
        assert_eq!(
            value,
            serde_json::json!({
                "hook": "hooks/pre-commit",
                "replaced": false,
                "backup": null,
                "pre_merge_commit": {
                    "hook": "hooks/pre-merge-commit",
                    "replaced": true,
                    "backup": null
                }
            })
        );
        let kept = Installed {
            backup: Some(PathBuf::from("hooks/pre-commit.before-theos")),
            pre_merge_commit: HookFile {
                backup: Some(PathBuf::from("hooks/pre-merge-commit.before-theos")),
                ..installed.pre_merge_commit.clone()
            },
            ..installed
        };
        let value = serde_json::to_value(HookView::from(&kept))?;
        assert_eq!(value["backup"], "hooks/pre-commit.before-theos");
        assert_eq!(
            value["pre_merge_commit"]["backup"],
            "hooks/pre-merge-commit.before-theos"
        );
        Ok(())
    }
}
