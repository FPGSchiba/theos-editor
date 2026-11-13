//! The JSON view of `theos hook install`.

use serde::Serialize;

use crate::hook::Installed;

/// The result of `theos hook install` as JSON.
#[derive(Debug, Serialize)]
pub struct HookView {
    hook: String,
    replaced: bool,
    backup: Option<String>,
}

impl From<&Installed> for HookView {
    fn from(installed: &Installed) -> Self {
        HookView {
            hook: installed.hook.display().to_string(),
            replaced: installed.replaced,
            backup: installed
                .backup
                .as_ref()
                .map(|backup| backup.display().to_string()),
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
        };
        let value = serde_json::to_value(HookView::from(&installed))?;
        assert_eq!(
            value,
            serde_json::json!({"hook": "hooks/pre-commit", "replaced": false, "backup": null})
        );
        let kept = Installed {
            backup: Some(PathBuf::from("hooks/pre-commit.before-theos")),
            ..installed
        };
        let value = serde_json::to_value(HookView::from(&kept))?;
        assert_eq!(value["backup"], "hooks/pre-commit.before-theos");
        Ok(())
    }
}
