//! The text dump of the index: a canonical form for comparing a fresh build with an incremental
//! history. It does not depend on row ids or on the order rows were written.

use rusqlite::types::ValueRef;

use crate::index::{Index, IndexError};

/// The sections of the dump, in order, with the query that reads each one. `files` and `inputs` show path
/// and hash only: sizes, times and the racy flag differ between a sync and an apply. `records` leaves
/// out `search_rowid`, the row id of its search row. `deps` is there on purpose: reads that
/// differ from a cold build's would miss a later change.
const SECTIONS: [(&str, &str); 11] = [
    (
        "records",
        "SELECT path, id, id_fold, kind, title, status, front, body_start, body_line, facts \
         FROM records",
    ),
    ("field_values", "SELECT * FROM field_values"),
    ("links", "SELECT * FROM links"),
    ("beats", "SELECT * FROM beats"),
    ("notes", "SELECT * FROM notes"),
    ("inworld", "SELECT * FROM inworld"),
    ("findings", "SELECT * FROM findings"),
    ("deps", "SELECT subject, key FROM deps"),
    ("search", "SELECT path, title, fields, body FROM search"),
    ("files", "SELECT path, hash FROM files"),
    ("inputs", "SELECT path, hash FROM inputs"),
];

impl Index {
    /// The whole index as text: per section a `[name]` line, then one row per line with tab
    /// between the columns, the rows sorted. `\\`, tab, line feed and carriage return in a text
    /// are written as `\\\\`, `\\t`, `\\n` and `\\r`; a NULL is `\\N`.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn dump(&self) -> Result<String, IndexError> {
        let mut out = String::new();
        for (name, sql) in SECTIONS {
            out.push('[');
            out.push_str(name);
            out.push_str("]\n");
            let mut stmt = self.store.conn.prepare(sql)?;
            let columns = stmt.column_count();
            let rows = stmt.query_map([], |row| {
                let mut cells = Vec::with_capacity(columns);
                for column in 0..columns {
                    cells.push(cell(row.get_ref(column)?));
                }
                Ok(cells.join("\t"))
            })?;
            let mut lines = rows.collect::<Result<Vec<String>, _>>()?;
            lines.sort();
            for line in lines {
                out.push_str(&line);
                out.push('\n');
            }
        }
        Ok(out)
    }
}

fn cell(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => "\\N".to_owned(),
        ValueRef::Integer(number) => number.to_string(),
        ValueRef::Text(bytes) => escape(&String::from_utf8_lossy(bytes)),
        other => format!("{other:?}"),
    }
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_are_escaped_and_null_is_distinct() {
        assert_eq!(escape("a\tb\nc\r\\"), "a\\tb\\nc\\r\\\\");
        assert_eq!(cell(ValueRef::Null), "\\N");
        assert_ne!(cell(ValueRef::Text(b"\\N")), cell(ValueRef::Null));
        assert_eq!(cell(ValueRef::Integer(7)), "7");
    }
}
