//! The lore step (spec §5.5; step 4b of P03 §5.9): after the links are resolved, in the same
//! transaction, every dirty subject runs its lore rules through one [`Context`], and its
//! findings and reads replace those it had.
//!
//! A subject is dirty when it was re-parsed or added, or when it read a key that the
//! transaction changed. A cold build and an incremental run are the same code; only the source
//! of the facts differs (L8).

use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension};

use crate::finding::Finding;
use crate::index::IndexError;
use crate::index::lore_source::{Bulk, ById};
use crate::index::rows::RecordRows;
use crate::index::write;
use crate::kinds::KindRegistry;
use crate::lore::{Context, FactSource, LoreRule, ReadKey, Subject};

/// The step name of the findings that [`run`] makes.
const STEP: &str = "lore";

/// When more subjects than this have rules to run, the facts are loaded at once (L8).
const BULK_ABOVE: usize = 32;

/// What one transaction changed, for the lore step.
#[derive(Debug, Default)]
pub(crate) struct LoreWork {
    /// The changed keys, spelled as `deps` stores them (L7).
    pub changed: BTreeSet<String>,
    /// The subjects that run whatever they read: those re-parsed or added.
    pub subjects: BTreeSet<String>,
    /// The subjects that are gone; their reads and findings went with their rows.
    pub removed: BTreeSet<String>,
}

impl LoreWork {
    /// Notes, before the rows of `path` change, the keys they answer: `rec:` of the record's
    /// `old_id` and `in:` of every target of its links (L7).
    pub(crate) fn old_rows(
        &mut self,
        conn: &Connection,
        path: &str,
        old_id: Option<&str>,
    ) -> rusqlite::Result<()> {
        if let Some(id) = old_id {
            self.changed.insert(ReadKey::Record(id.to_owned()).as_db());
        }
        let mut stmt = conn.prepare_cached("SELECT DISTINCT target FROM links WHERE path = ?1")?;
        for target in stmt.query_map([path], |row| row.get::<_, String>(0))? {
            self.changed.insert(ReadKey::Inbound(target?).as_db());
        }
        Ok(())
    }

    /// Notes the keys that the new `rows` of a path answer: `rec:` of its id and `in:` of every
    /// target of its links (L7).
    pub(crate) fn new_rows(&mut self, rows: &RecordRows) {
        if let Some(record) = &rows.record {
            self.changed
                .insert(ReadKey::Record(record.id.clone()).as_db());
        }
        for link in &rows.links {
            self.changed
                .insert(ReadKey::Inbound(link.target.clone()).as_db());
        }
    }

    /// `path` was re-parsed or added: it runs. A removal earlier in the same call no longer
    /// counts.
    pub(crate) fn add_subject(&mut self, path: &str) {
        self.removed.remove(path);
        self.subjects.insert(path.to_owned());
    }

    /// `path` is gone: it does not run. A re-parse earlier in the same call no longer counts.
    pub(crate) fn remove_subject(&mut self, path: &str) {
        self.subjects.remove(path);
        self.removed.insert(path.to_owned());
    }
}

/// One dirty path: the record it holds, if any, and the rules that check it.
struct Planned<'r> {
    path: String,
    /// The id and kind of the record at the path; none for an input file.
    record: Option<(String, Option<String>)>,
    rules: Vec<&'r LoreRule>,
}

/// Runs the lore rules of `registry` on every dirty subject of `work` and stores what they
/// give, and returns the paths of the subjects it re-ran.
pub(crate) fn run(
    conn: &Connection,
    registry: &KindRegistry,
    work: &LoreWork,
) -> Result<BTreeSet<String>, IndexError> {
    let mut plans = Vec::new();
    for path in dirty(conn, work)? {
        clear(conn, &path)?;
        let record = record_at(conn, &path)?;
        let rules = registry
            .lore_rules()
            .filter(|rule| match &record {
                Some((_, kind)) => rule.on.takes_record(kind.as_deref()),
                None => rule.on.takes_input(&path),
            })
            .collect();
        plans.push(Planned {
            path,
            record,
            rules,
        });
    }
    let busy = plans.iter().filter(|plan| !plan.rules.is_empty()).count();
    let source: Box<dyn FactSource + '_> = if busy > BULK_ABOVE {
        Box::new(Bulk::load(conn)?)
    } else {
        Box::new(ById::new(conn))
    };
    let mut ctx = Context::new(source, registry);
    let mut reran = BTreeSet::new();
    for plan in plans {
        let subject = match &plan.record {
            // A record no rule checks needs no facts.
            Some(_) if plan.rules.is_empty() => None,
            Some((id, _)) => ctx.subject_record(id, &plan.path)?.map(Subject::Record),
            None => ctx.subject_input(&plan.path)?.map(Subject::Input),
        };
        let is_subject = plan.record.is_some() || subject.is_some();
        if let Some(subject) = subject {
            check(conn, &mut ctx, &subject, &plan.rules)?;
        }
        if is_subject {
            reran.insert(plan.path);
        }
    }
    Ok(reran)
}

/// The dirty subjects: the re-parsed and added ones and every subject that read a changed key,
/// without the removed ones.
fn dirty(conn: &Connection, work: &LoreWork) -> rusqlite::Result<BTreeSet<String>> {
    let mut dirty = work.subjects.clone();
    let mut stmt = conn.prepare_cached("SELECT DISTINCT subject FROM deps WHERE key = ?1")?;
    for key in &work.changed {
        for subject in stmt.query_map([key], |row| row.get::<_, String>(0))? {
            dirty.insert(subject?);
        }
    }
    for path in &work.removed {
        dirty.remove(path);
    }
    Ok(dirty)
}

/// Deletes the lore findings and the reads of the subject at `path`.
fn clear(conn: &Connection, path: &str) -> rusqlite::Result<()> {
    conn.prepare_cached("DELETE FROM findings WHERE path = ?1 AND step = ?2")?
        .execute([path, STEP])?;
    conn.prepare_cached("DELETE FROM deps WHERE subject = ?1")?
        .execute([path])?;
    Ok(())
}

/// The id and kind of the record at `path`.
fn record_at(conn: &Connection, path: &str) -> rusqlite::Result<Option<(String, Option<String>)>> {
    conn.prepare_cached("SELECT id, kind FROM records WHERE path = ?1")?
        .query_row([path], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()
}

/// Runs `rules` on `subject` and stores their findings and the reads they made.
fn check(
    conn: &Connection,
    ctx: &mut Context<'_>,
    subject: &Subject,
    rules: &[&LoreRule],
) -> Result<(), IndexError> {
    for rule in rules {
        for finding in (rule.run)(ctx, subject)? {
            if !sits_on(&finding, subject) {
                debug_assert!(false, "{}", off_subject(rule, &finding, subject));
                tracing::error!("{}", off_subject(rule, &finding, subject));
                continue;
            }
            write::insert_finding(conn, &finding, STEP)?;
        }
    }
    let mut stmt = conn.prepare_cached("INSERT INTO deps (subject, key) VALUES (?1, ?2)")?;
    for key in ctx.take_reads() {
        stmt.execute([subject.path(), key.as_db().as_str()])?;
    }
    Ok(())
}

/// Whether `finding` sits on `subject`: its path is the subject's and its id is the subject's
/// record id, none for an input file (L9).
fn sits_on(finding: &Finding, subject: &Subject) -> bool {
    let id = match subject {
        Subject::Record(record) => Some(record.id.as_str()),
        Subject::Input(_) => None,
    };
    finding.path == subject.path() && finding.id.as_deref() == id
}

/// The message of a rule that returned a finding on another subject.
fn off_subject(rule: &LoreRule, finding: &Finding, subject: &Subject) -> String {
    format!(
        "lore rule `{}` returned a finding on `{}` (id {:?}) while checking `{}`: a rule reports on its own subject only",
        rule.id,
        finding.path,
        finding.id,
        subject.path()
    )
}
