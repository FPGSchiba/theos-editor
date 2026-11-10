//! `link/kind`: the record a link names is of a kind its slot does not allow.

use super::{LinkRule, SourceRecord, TargetInfo, Targets, at_link};
use crate::finding::{Finding, Severity, with_article};
use crate::index::slots::Slot;
use crate::kinds::{KindRegistry, Target};

/// The rule.
pub(super) const RULE: LinkRule = LinkRule {
    id: "link/kind",
    severity: Severity::Error,
    run,
};

/// Whether one record fits the targets of a slot.
#[derive(PartialEq, Eq)]
enum Verdict {
    Fits,
    Misfits,
    /// No known kind, or a narrowing value that is missing or invalid: nothing to say.
    Unknown,
}

fn verdict(info: &TargetInfo, allowed: &[Target]) -> Verdict {
    let Some(kind) = &info.kind else {
        return Verdict::Unknown;
    };
    let mut undecided = false;
    for target in allowed.iter().filter(|target| target.kind == kind) {
        match target.narrow {
            None => return Verdict::Fits,
            Some(narrow) => match info.narrow.get(narrow.field) {
                Some(value) if value == narrow.value => return Verdict::Fits,
                Some(_) => {}
                None => undecided = true,
            },
        }
    }
    if undecided {
        Verdict::Unknown
    } else {
        Verdict::Misfits
    }
}

/// `a location or floor`, `an explaining topic`.
fn must_link_to(allowed: &[Target]) -> String {
    let names: Vec<String> = allowed
        .iter()
        .map(|target| match target.narrow {
            None => target.kind.to_owned(),
            Some(narrow) => format!("{} {}", narrow.value, target.kind),
        })
        .collect();
    let list = match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        _ => names.join(""),
    };
    with_article(&list)
}

/// What the misfit is: its narrowing value when a target of its kind narrows, else its kind.
fn is_what(info: &TargetInfo, allowed: &[Target]) -> String {
    let kind = info.kind.as_deref().unwrap_or_default();
    allowed
        .iter()
        .filter(|target| target.kind == kind)
        .find_map(|target| {
            target
                .narrow
                .and_then(|narrow| info.narrow.get(narrow.field))
        })
        .map_or_else(|| with_article(kind), Clone::clone)
}

fn run(source: &SourceRecord, targets: &Targets, registry: &KindRegistry) -> Vec<Finding> {
    let spec = source.kind.as_deref().and_then(|kind| registry.kind(kind));
    let mut found = Vec::new();
    for link in &source.links {
        // A mention may point at any kind. `targets()` is also `None` for a slot the spec does
        // not define, which gets no finding either.
        if link.slot == Slot::Mention {
            continue;
        }
        let Some(allowed) = link.slot.targets(spec) else {
            continue;
        };
        let Some(infos) = targets.get(&link.target) else {
            continue;
        };
        let verdicts: Vec<Verdict> = infos.iter().map(|info| verdict(info, allowed)).collect();
        if verdicts.contains(&Verdict::Fits) {
            continue;
        }
        let Some(index) = verdicts.iter().position(|v| *v == Verdict::Misfits) else {
            continue;
        };
        found.push(at_link(
            &RULE,
            source,
            link,
            format!(
                "{} must link to {}; `{}` is {}",
                link.slot,
                must_link_to(allowed),
                link.target,
                is_what(&infos[index], allowed)
            ),
        ));
    }
    found
}
