//! Resolving links, ids and placement over the stored rows (spec §5.6): the rules that read a
//! record's links against the records they name, and the duplicate-id check.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;

use crate::finding::{Finding, Severity};
use crate::index::IndexError;
use crate::index::rows::LinkRow;
use crate::index::write::insert_finding;
use crate::kinds::KindRegistry;
use crate::text::{Position, Span};

mod dangling;
mod duplicate;
mod kind;
mod placement;
mod targets;

pub(crate) use duplicate::{ID_RULE, id_fold, recheck_ids};

/// What a link to an id depends on: one record that has the id.
pub(crate) struct TargetInfo {
    /// The project-relative path.
    pub path: String,
    /// The kind, when the record has a registered one.
    pub kind: Option<String>,
    /// The valid scalar values of the fields some target narrows by.
    pub narrow: BTreeMap<String, String>,
    /// The numbers of the record's beats.
    pub beats: BTreeSet<u32>,
}

/// The records an id names, by id; several when the id is duplicated.
pub(crate) type Targets = BTreeMap<String, Vec<TargetInfo>>;

/// One record seen as the source of links.
pub(crate) struct SourceRecord {
    /// The project-relative path.
    pub path: String,
    /// The id (the file stem).
    pub id: String,
    /// The kind, when the record has a registered one.
    pub kind: Option<String>,
    /// The span and position of the `kind` value.
    pub kind_at: Option<(Span, Position)>,
    /// The links of the record, in file order.
    pub links: Vec<LinkRow>,
}

/// A rule over the links of one record.
pub(crate) struct LinkRule {
    /// The rule, `<group>/<name>`.
    pub id: &'static str,
    /// Its default severity.
    pub severity: Severity,
    /// Runs the rule.
    pub run: fn(&SourceRecord, &Targets, &KindRegistry) -> Vec<Finding>,
}

/// Every link rule; a new rule is one module and one line here.
pub(crate) const LINK_RULES: &[LinkRule] = &[dangling::RULE, kind::RULE, placement::RULE];

/// The step name of the findings that [`relink`] makes.
const STEP: &str = "links";

/// Every rule this module reports, with its default severity.
pub(crate) fn rule_set() -> Vec<(&'static str, Severity)> {
    let mut rules: Vec<_> = LINK_RULES.iter().map(|r| (r.id, r.severity)).collect();
    rules.push(ID_RULE);
    rules
}

/// A finding of `rule` about `source`, at a span and position.
fn finding(
    rule: &LinkRule,
    source: &SourceRecord,
    at: (Span, Position),
    message: String,
) -> Finding {
    Finding {
        rule: rule.id.to_owned(),
        severity: rule.severity,
        path: source.path.clone(),
        id: Some(source.id.clone()),
        span: Some(at.0),
        position: Some(at.1),
        message,
    }
}

/// A finding at a link.
fn at_link(rule: &LinkRule, source: &SourceRecord, link: &LinkRow, message: String) -> Finding {
    finding(rule, source, (link.span, link.position), message)
}

/// The folder of a project-relative path: everything before the last `/`, or empty.
fn folder_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(folder, _)| folder)
}

/// Replaces the `links` findings of each of `sources` with the findings of [`LINK_RULES`], read
/// against the records the links name.
pub(crate) fn relink(
    conn: &Connection,
    registry: &KindRegistry,
    sources: &BTreeSet<String>,
) -> Result<(), IndexError> {
    let records = targets::load_sources(conn, sources)?;
    let targets = targets::load_targets(conn, registry, &records)?;
    for source in &records {
        conn.prepare_cached("DELETE FROM findings WHERE path = ?1 AND step = ?2")?
            .execute([source.path.as_str(), STEP])?;
        for rule in LINK_RULES {
            for found in (rule.run)(source, &targets, registry) {
                insert_finding(conn, &found, STEP)?;
            }
        }
    }
    Ok(())
}
