//! `theos schema`: every kind and rule, or the fields and body of one kind (spec §5.10).

use std::fmt::Write as _;
use std::io::Write;
use std::path::Path;

use anyhow::Context;
use theos_content::parts;
use theos_studio::schema;
use theos_studio::views::schema::{BodyView, KindView, PlacementView, SchemaView};
use theos_studio::{StudioError, find_root};

use crate::output;

/// Writes the schema to `out`, as text or as one JSON document. In a project (`project`, else the
/// one above the current folder) the rules show their configured severity.
///
/// # Errors
///
/// Fails when `kind` is not a kind, when the project's settings cannot be read, or when `out`
/// cannot be written.
pub(crate) fn run(
    kind: Option<&str>,
    project: Option<&Path>,
    json: bool,
    out: &mut impl Write,
) -> anyhow::Result<()> {
    let registry = parts::builtin().map_err(StudioError::from)?;
    if let Some(name) = kind {
        let view = schema::kind(&registry, name)?;
        if json {
            return output::json(out, &view);
        }
        return out
            .write_all(kind_text(&view).as_bytes())
            .context("cannot write the output");
    }
    let root = match project {
        Some(folder) => Some(folder.to_path_buf()),
        None => std::env::current_dir()
            .ok()
            .and_then(|current| find_root(&current).ok()),
    };
    let view = schema::overview(&registry, root.as_deref())?;
    if json {
        return output::json(out, &view);
    }
    out.write_all(overview_text(&view).as_bytes())
        .context("cannot write the output")
}

/// The overview as text: the kinds, an empty line, then the rules.
fn overview_text(view: &SchemaView) -> String {
    let kinds: Vec<Vec<String>> = view
        .kinds
        .iter()
        .map(|kind| {
            vec![
                kind.name.clone(),
                kind.part.clone(),
                placement_text(&kind.placement),
                kind.id_rule.clone(),
            ]
        })
        .collect();
    let rules: Vec<Vec<String>> = view
        .rules
        .iter()
        .map(|rule| {
            let mut row = vec![
                rule.id.clone(),
                rule.severity.to_owned(),
                rule.class.to_owned(),
            ];
            if let Some(configured) = rule.configured {
                row.push(format!("configured {configured}"));
            }
            row
        })
        .collect();
    format!(
        "Kinds:\n{}\nRules:\n{}",
        output::columns(&kinds),
        output::columns(&rules)
    )
}

/// One kind as text.
fn kind_text(view: &KindView) -> String {
    let mut text = format!("{} ({})\n", view.name, view.part);
    let _ = writeln!(text, "id: {}", view.id_rule);
    let _ = writeln!(text, "placement: {}", placement_text(&view.placement));
    text.push_str("fields:\n");
    for field in &view.fields {
        let required = if field.required { ", required" } else { "" };
        let _ = writeln!(text, "  {}: {}{required}", field.key, field.ty);
    }
    text.push_str("body:\n");
    text.push_str(&body_text(&view.body));
    text
}

/// The lines of a body grammar; `prose` when it has none of beats, notes and verbatim.
fn body_text(body: &BodyView) -> String {
    let mut text = String::new();
    if let Some(beats) = &body.beats {
        text.push_str("  beats:\n");
        for beat in beats {
            let _ = writeln!(text, "    {} -> {}", beat.key, beat.targets.join(", "));
        }
    }
    if let Some(notes) = &body.notes {
        let _ = writeln!(text, "  notes link to: {}", notes.join(", "));
    }
    if body.verbatim {
        text.push_str("  verbatim\n");
    }
    if text.is_empty() {
        text.push_str("  prose\n");
    }
    text
}

/// `lore/truths/` for a folder, `under its parent` for a child kind.
fn placement_text(placement: &PlacementView) -> String {
    match placement {
        PlacementView::Folder(folder) => format!("{folder}/"),
        PlacementView::Under(_) => "under its parent".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_text_names_the_folder_or_the_parent() {
        assert_eq!(
            placement_text(&PlacementView::Folder("lore/truths".to_owned())),
            "lore/truths/"
        );
        assert_eq!(
            placement_text(&PlacementView::Under("parent".to_owned())),
            "under its parent"
        );
    }

    #[test]
    fn a_plain_body_is_prose() {
        let body = BodyView {
            beats: None,
            notes: None,
            verbatim: false,
        };
        assert_eq!(body_text(&body), "  prose\n");
        let body = BodyView {
            verbatim: true,
            ..body
        };
        assert_eq!(body_text(&body), "  verbatim\n");
    }
}
