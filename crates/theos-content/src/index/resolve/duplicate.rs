//! `index/duplicate-id`: two record files whose names are equal apart from ASCII case, in one
//! folder or in several. Windows and macOS cannot hold such files in one folder tree.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, OptionalExtension};

use crate::finding::{Finding, Severity};
use crate::index::IndexError;
use crate::index::write::insert_finding;
use crate::text::{Position, Span};

/// The rule and its default severity.
pub(crate) const ID_RULE: (&str, Severity) = ("index/duplicate-id", Severity::Error);

/// The step name of the findings that [`recheck_ids`] makes.
const STEP: &str = "ids";

/// The fold of an id: its ASCII letters in lower case, every other character as it is. Two ids
/// are duplicates when their folds are equal; the `id_fold` column holds this fold.
pub(crate) fn id_fold(id: &str) -> String {
    id.to_ascii_lowercase()
}

/// The paths that share an id apart from ASCII case, from `(path, id)` pairs. A group has at
/// least two paths, sorted; the groups come in the order of their folded id.
pub(crate) fn duplicate_groups(ids: &[(String, String)]) -> Vec<Vec<String>> {
    let mut groups: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    for (path, id) in ids {
        groups.entry(id_fold(id)).or_default().insert(path);
    }
    groups
        .into_values()
        .filter(|paths| paths.len() > 1)
        .map(|paths| paths.into_iter().map(str::to_owned).collect())
        .collect()
}

/// Replaces the `ids` findings of every record whose id fold is in `folds`, each made by
/// [`id_fold`].
pub(crate) fn recheck_ids(conn: &Connection, folds: &BTreeSet<String>) -> Result<(), IndexError> {
    for fold in folds {
        conn.execute(
            "DELETE FROM findings WHERE step = ?2 \
             AND path IN (SELECT path FROM records WHERE id_fold = ?1)",
            [fold.as_str(), STEP],
        )?;
        let mut stmt = conn.prepare("SELECT path, id FROM records WHERE id_fold = ?1")?;
        let ids = stmt
            .query_map([fold], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<(String, String)>, _>>()?;
        drop(stmt);
        for group in duplicate_groups(&ids) {
            for path in &group {
                let id = ids
                    .iter()
                    .find(|(candidate, _)| candidate == path)
                    .map(|(_, id)| id.clone())
                    .unwrap_or_default();
                let others: Vec<String> = group
                    .iter()
                    .filter(|other| *other != path)
                    .map(|other| format!("`{other}`"))
                    .collect();
                let (span, position) = id_at(conn, path)?;
                insert_finding(
                    conn,
                    &Finding {
                        rule: ID_RULE.0.to_owned(),
                        severity: ID_RULE.1,
                        path: path.clone(),
                        message: format!(
                            "the id `{id}` is also the file name of {}; ids must be unique, \
                             even apart from ASCII case",
                            others.join(", ")
                        ),
                        id: Some(id),
                        span: Some(span),
                        position: Some(position),
                    },
                    STEP,
                )?;
            }
        }
    }
    Ok(())
}

/// The span and position of the `id` value, or 1:1 when the record has none.
fn id_at(conn: &Connection, path: &str) -> Result<(Span, Position), IndexError> {
    let found = conn
        .query_row(
            "SELECT span_start, span_end, line, col FROM field_values \
             WHERE path = ?1 AND field = 'id' AND entry IS NULL AND item IS NULL",
            [path],
            |row| {
                let n = |i| {
                    row.get::<_, i64>(i)
                        .map(|value| usize::try_from(value).unwrap_or_default())
                };
                Ok((
                    Span {
                        start: n(0)?,
                        end: n(1)?,
                    },
                    Position {
                        line: n(2)?,
                        column: n(3)?,
                    },
                ))
            },
        )
        .optional()?;
    Ok(found.unwrap_or((Span::default(), Position { line: 1, column: 1 })))
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC: CE-12.3
    #[test]
    fn names_equal_apart_from_ascii_case_are_duplicates() {
        let ids = [
            ("a/Foo.md", "Foo"),
            ("b/foo.md", "foo"),
            ("c/bar.md", "bar"),
        ]
        .map(|(path, id)| (path.to_owned(), id.to_owned()));
        assert_eq!(
            duplicate_groups(&ids),
            [["a/Foo.md".to_owned(), "b/foo.md".to_owned()]]
        );
    }

    #[test]
    fn the_fold_lowers_ascii_letters_only() {
        assert_eq!(id_fold("SYNTHETIC-Ärger"), "synthetic-Ärger");
        assert_ne!(id_fold("Ärger"), id_fold("ärger"));
    }
}
