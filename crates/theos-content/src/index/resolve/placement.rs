//! `index/placement`: a record whose file is not where the placement of its kind puts it.

use super::{LinkRule, SourceRecord, Targets, at_link, finding, folder_of, with_article};
use crate::finding::{Finding, Severity};
use crate::index::slots::Slot;
use crate::kinds::{KindRegistry, Placement};

/// The rule.
pub(super) const RULE: LinkRule = LinkRule {
    id: "index/placement",
    severity: Severity::Warning,
    run,
};

fn shown(folder: &str) -> String {
    if folder.is_empty() {
        "the project root".to_owned()
    } else {
        format!("`{folder}/`")
    }
}

fn run(source: &SourceRecord, targets: &Targets, registry: &KindRegistry) -> Vec<Finding> {
    let Some(kind) = source.kind.as_deref() else {
        return Vec::new();
    };
    let Some(spec) = registry.kind(kind) else {
        return Vec::new();
    };
    let actual = folder_of(&source.path);
    match spec.placement {
        Placement::Flat(folder) if folder != actual => source
            .kind_at
            .map(|at| {
                let message = format!(
                    "{} belongs in {}; this file is in {}",
                    with_article(kind),
                    shown(folder),
                    shown(actual)
                );
                finding(&RULE, source, at, message)
            })
            .into_iter()
            .collect(),
        Placement::Flat(_) => Vec::new(),
        Placement::UnderParent(field) => {
            let Some(link) = source
                .links
                .iter()
                .find(|link| link.slot == Slot::Field(field.to_owned()))
            else {
                return Vec::new();
            };
            let Some(parents) = targets.get(&link.target).filter(|p| !p.is_empty()) else {
                return Vec::new();
            };
            let wanted: Vec<String> = parents
                .iter()
                .map(|parent| match folder_of(&parent.path) {
                    "" => link.target.clone(),
                    folder => format!("{folder}/{}", link.target),
                })
                .collect();
            if wanted.iter().any(|folder| folder == actual) {
                return Vec::new();
            }
            let message = format!(
                "{} belongs in {}, under its parent `{}`; this file is in {}",
                with_article(kind),
                shown(&wanted[0]),
                link.target,
                shown(actual)
            );
            vec![at_link(&RULE, source, link, message)]
        }
    }
}
