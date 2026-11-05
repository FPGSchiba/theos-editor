//! `link/dangling`: a link whose id names no record, or whose beat does not exist. A beat
//! reference whose target has no known kind is left to the target's own finding.

use super::{LinkRule, SourceRecord, Targets, at_link};
use crate::finding::{Finding, Severity, with_article};
use crate::kinds::KindRegistry;

/// The rule.
pub(super) const RULE: LinkRule = LinkRule {
    id: "link/dangling",
    severity: Severity::Error,
    run,
};

fn run(source: &SourceRecord, targets: &Targets, registry: &KindRegistry) -> Vec<Finding> {
    let mut found = Vec::new();
    for link in &source.links {
        let id = &link.target;
        let infos = targets.get(id).map_or(&[][..], Vec::as_slice);
        let message = if infos.is_empty() {
            format!("`{id}` names no record")
        } else if let Some(fragment) = &link.fragment {
            let has_beats = |kind: &Option<String>| {
                kind.as_deref()
                    .and_then(|kind| registry.kind(kind))
                    .is_some_and(|spec| spec.body.beats.is_some())
            };
            let number = fragment.parse::<u32>().ok();
            let scripts: Vec<_> = infos.iter().filter(|info| has_beats(&info.kind)).collect();
            if scripts.is_empty() {
                // A target without a known kind has its own finding, which says what is wrong
                // (spec §5.6).
                let Some(kind) = infos.iter().find_map(|info| info.kind.as_deref()) else {
                    continue;
                };
                format!("`{id}` is {}; only scripts have beats", with_article(kind))
            } else if number.is_some_and(|n| scripts.iter().any(|info| info.beats.contains(&n))) {
                continue;
            } else {
                format!("`{id}` has no beat {fragment}")
            }
        } else {
            continue;
        };
        found.push(at_link(&RULE, source, link, message));
    }
    found
}
