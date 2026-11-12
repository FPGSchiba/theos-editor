//! The configured severities in the cache (spec §5.12): the lore step stores what the settings
//! file configures, and findings are read with those severities applied. The findings table keeps
//! the default severities.

use std::collections::BTreeMap;

use rusqlite::Connection;

use crate::finding::{Finding, Severity};
use crate::rules::{self, RuleInfo};

/// Deletes every configured severity.
pub(super) fn clear(conn: &Connection) -> rusqlite::Result<()> {
    conn.prepare_cached("DELETE FROM configured")?.execute([])?;
    Ok(())
}

/// Replaces the configured severities with `severities`.
pub(super) fn store(
    conn: &Connection,
    severities: &BTreeMap<String, Severity>,
) -> rusqlite::Result<()> {
    clear(conn)?;
    let mut insert =
        conn.prepare_cached("INSERT INTO configured (rule, severity) VALUES (?1, ?2)")?;
    for (rule, severity) in severities {
        insert.execute([rule.as_str(), severity.as_str()])?;
    }
    Ok(())
}

/// The configured severities. A stored severity that cannot be read is left out.
pub(super) fn load(conn: &Connection) -> rusqlite::Result<BTreeMap<String, Severity>> {
    let mut stmt = conn.prepare_cached("SELECT rule, severity FROM configured")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (rule, severity) = row?;
        if let Some(severity) = Severity::parse(&severity) {
            out.insert(rule, severity);
        }
    }
    Ok(out)
}

/// Gives each finding the severity its rule reports with: `rules::effective` over the stored
/// default and what the settings configure. A finding of a rule outside `table` keeps its
/// severity.
pub(super) fn apply(
    conn: &Connection,
    table: &[RuleInfo],
    mut findings: Vec<Finding>,
) -> rusqlite::Result<Vec<Finding>> {
    let configured = load(conn)?;
    for finding in &mut findings {
        if let Some(info) = table.iter().find(|info| info.id == finding.rule) {
            finding.severity = rules::effective(info, configured.get(&finding.rule).copied());
        }
    }
    Ok(findings)
}
