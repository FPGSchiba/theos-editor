//! The schema: every kind and rule, with the project's configured severities, and one kind in
//! detail.

use std::path::Path;

use theos_content::settings::{self, SETTINGS};
use theos_content::{KindRegistry, KindSpec, Placement, rules};

use crate::StudioError;
use crate::views::schema::{
    BeatView, BodyView, FieldView, KindSummaryView, KindView, PlacementView, RuleView, SchemaView,
};

/// Every kind of `registry` and every rule. In a project (`project` is its folder) each rule
/// also carries the severity the project's `.theos/settings.toml` gives it; without one,
/// `configured` is `null`.
///
/// # Errors
///
/// [`StudioError::Io`] when the settings file exists and cannot be read.
pub fn overview(
    registry: &KindRegistry,
    project: Option<&Path>,
) -> Result<SchemaView, StudioError> {
    let infos = rules::all(registry);
    let severities = match project {
        Some(root) => Some(
            settings::parse(read_settings(root)?.as_deref(), &infos)
                .settings
                .severities,
        ),
        None => None,
    };
    let rules = infos
        .iter()
        .map(|info| RuleView {
            id: info.id.to_owned(),
            severity: info.severity.as_str(),
            class: info.class.as_str(),
            configured: severities
                .as_ref()
                .map(|map| rules::effective(info, map.get(info.id).copied()).as_str()),
        })
        .collect();
    let kinds = registry
        .kinds()
        .map(|spec| KindSummaryView {
            name: spec.name.to_owned(),
            part: part_of(registry, spec),
            id_rule: spec.id.says.to_owned(),
            placement: placement(spec.placement),
        })
        .collect();
    Ok(SchemaView { kinds, rules })
}

/// The detail of the kind `name`: its fields in writing order and its body grammar.
///
/// # Errors
///
/// [`StudioError::UnknownKind`] when `registry` has no such kind.
pub fn kind(registry: &KindRegistry, name: &str) -> Result<KindView, StudioError> {
    let spec = registry
        .kind(name)
        .ok_or_else(|| StudioError::unknown_kind(registry, name))?;
    let body = spec.body;
    Ok(KindView {
        name: spec.name.to_owned(),
        part: part_of(registry, spec),
        id_rule: spec.id.says.to_owned(),
        placement: placement(spec.placement),
        fields: spec
            .fields
            .iter()
            .map(|field| FieldView {
                key: field.key.to_owned(),
                ty: field.ty.to_string(),
                required: field.required,
            })
            .collect(),
        body: BodyView {
            beats: body.beats.map(|beats| {
                beats
                    .iter()
                    .map(|beat| BeatView {
                        key: beat.key.to_owned(),
                        targets: beat.targets.iter().map(ToString::to_string).collect(),
                    })
                    .collect()
            }),
            notes: body
                .notes
                .map(|notes| notes.iter().map(ToString::to_string).collect()),
            verbatim: body.verbatim,
        },
    })
}

fn part_of(registry: &KindRegistry, spec: &KindSpec) -> String {
    registry.part_of(spec.name).unwrap_or_default().to_owned()
}

fn placement(placement: Placement) -> PlacementView {
    match placement {
        Placement::Flat(folder) => PlacementView::Folder(folder.to_owned()),
        Placement::UnderParent(field) => PlacementView::Under(field.to_owned()),
    }
}

/// The text of the project's settings file: none when it is missing or not UTF-8 (`parse(None)`
/// then reports it), and an empty text for a missing file.
fn read_settings(root: &Path) -> Result<Option<String>, StudioError> {
    let path = root.join(SETTINGS);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(String::from_utf8(bytes).ok()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Some(String::new())),
        Err(source) => Err(StudioError::Io { path, source }),
    }
}

#[cfg(test)]
mod tests {
    use theos_content::parts;

    use super::*;

    #[test]
    fn an_unknown_kind_lists_the_known_ones() -> Result<(), StudioError> {
        let registry = parts::builtin()?;
        let error = kind(&registry, "nope").err().map(|e| e.to_string());
        let message = error.unwrap_or_default();
        assert!(
            message.starts_with("unknown kind `nope`; the kinds are: "),
            "{message}"
        );
        assert!(message.contains("truth"), "{message}");
        Ok(())
    }

    #[test]
    fn a_settings_file_that_is_not_utf8_configures_nothing()
    -> Result<(), Box<dyn std::error::Error>> {
        let registry = parts::builtin()?;
        let project = tempfile::tempdir()?;
        std::fs::create_dir(project.path().join(".theos"))?;
        std::fs::write(project.path().join(SETTINGS), [0xff, 0xfe])?;
        let view = overview(&registry, Some(project.path()))?;
        assert!(view.rules.iter().all(|rule| rule.configured
            == Some(if rule.class == "guardrail" {
                "error"
            } else {
                rule.severity
            })));
        Ok(())
    }

    #[test]
    fn an_unreadable_settings_path_is_an_io_error() -> Result<(), Box<dyn std::error::Error>> {
        let registry = parts::builtin()?;
        let project = tempfile::tempdir()?;
        // A folder where the file should be cannot be read as a file.
        std::fs::create_dir_all(project.path().join(SETTINGS))?;
        let error = overview(&registry, Some(project.path())).err();
        assert!(matches!(error, Some(StudioError::Io { .. })), "{error:?}");
        Ok(())
    }
}
