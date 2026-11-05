//! Full-text search (spec §5.11): FTS5 tokenises, stems and ranks; this module only quotes the
//! words of the user and maps the highlight markers back to spans in the record.

use rusqlite::{Connection, OptionalExtension, params};

use crate::index::{Index, IndexError};
use crate::text::{Position, Span};

/// The most matches one hit lists (L20).
pub(crate) const MATCHES_PER_HIT: usize = 10;

/// FTS5 `highlight` markers: two noncharacters that real text does not hold.
const OPEN: char = '\u{FDD0}';
const CLOSE: char = '\u{FDD1}';

const SQL: &str = "\
SELECT r.id, r.path, r.kind, r.title, bm25(search, 0.0, 5.0, 2.0, 1.0) AS relevance, \
       highlight(search, 1, ?2, ?3), highlight(search, 2, ?2, ?3), highlight(search, 3, ?2, ?3), \
       search.title, search.fields, search.body, r.body_start, r.body_line, r.front \
FROM search JOIN records r ON r.path = search.path \
WHERE search MATCH ?1 ORDER BY relevance, r.id LIMIT ?4";

/// One record that matches a query.
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    /// The id (the file stem).
    pub id: String,
    /// The project-relative path with `/`.
    pub path: String,
    /// The kind, when the front matter names a registered one.
    pub kind: Option<String>,
    /// The text of the `title` scalar.
    pub title: Option<String>,
    /// The relevance, the negated `bm25`: higher is better.
    pub score: f64,
    /// Where the words match, by position in the record, at most ten.
    pub matches: Vec<Match>,
}

/// One place where the query matches a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    /// The part of the record.
    pub column: Column,
    /// The bytes of the match in the LF text of the record; a whole value when the value is
    /// written with escapes.
    pub span: Span,
    /// Line and column of the span start.
    pub position: Position,
    /// The line of the record that holds the match.
    pub line: String,
}

/// The part of a record a match lies in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Column {
    /// The `title` field.
    Title,
    /// Another searched field.
    Field {
        /// The front-matter key.
        key: String,
        /// The key of the map entry, for a value inside a map.
        entry: Option<String>,
    },
    /// The body.
    Body,
}

impl Index {
    /// The records matching `query`, best first, at most `limit`. The query is plain words: a
    /// `"..."` phrase stays together, and no FTS5 operator or syntax has a meaning.
    ///
    /// # Errors
    ///
    /// Fails when SQLite fails.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Hit>, IndexError> {
        let Some(query) = fts_query(query) else {
            return Ok(Vec::new());
        };
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let conn = &self.store.conn;
        let mut stmt = conn.prepare_cached(SQL)?;
        let markers = (OPEN.to_string(), CLOSE.to_string());
        let rows = stmt.query_map(params![query, markers.0, markers.1, limit], Raw::read)?;
        let mut hits = Vec::new();
        for raw in rows {
            hits.push(raw?.into_hit(conn)?);
        }
        Ok(hits)
    }
}

/// Quotes the words and phrases of `query` so FTS5 reads them as plain terms; none left gives
/// `None`. A `"` ends the current word and opens a phrase, which runs to the next `"`. A control
/// character is read as a space, so it separates words and never reaches FTS5, which takes a
/// NUL for the end of the query.
pub(crate) fn fts_query(query: &str) -> Option<String> {
    let mut terms = Vec::new();
    let mut current = String::new();
    let mut phrase = false;
    for c in query.chars().map(|c| if c.is_control() { ' ' } else { c }) {
        if c == '"' {
            terms.push(std::mem::take(&mut current));
            phrase = !phrase;
        } else if c.is_whitespace() && !phrase {
            terms.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    terms.push(current);
    let quoted: Vec<String> = terms
        .iter()
        .filter(|term| !term.trim().is_empty())
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect();
    (!quoted.is_empty()).then(|| quoted.join(" "))
}

/// A byte range inside one column.
type Range = (usize, usize);

/// One row of [`SQL`].
struct Raw {
    id: String,
    path: String,
    kind: Option<String>,
    title: Option<String>,
    relevance: f64,
    marked: [String; 3],
    stored: [String; 3],
    body_start: Option<usize>,
    body_line: Option<usize>,
    front: Option<String>,
}

impl Raw {
    fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Raw> {
        let offset = |i: usize| -> rusqlite::Result<Option<usize>> {
            row.get::<_, Option<i64>>(i)?
                .map(|n| {
                    usize::try_from(n).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(i, n))
                })
                .transpose()
        };
        Ok(Raw {
            id: row.get(0)?,
            path: row.get(1)?,
            kind: row.get(2)?,
            title: row.get(3)?,
            relevance: row.get(4)?,
            marked: [row.get(5)?, row.get(6)?, row.get(7)?],
            stored: [row.get(8)?, row.get(9)?, row.get(10)?],
            body_start: offset(11)?,
            body_line: offset(12)?,
            front: row.get(13)?,
        })
    }

    fn into_hit(self, conn: &Connection) -> rusqlite::Result<Hit> {
        let front: Vec<&str> = self.front.as_deref().unwrap_or_default().lines().collect();
        let mut matches = self.title_matches(conn, &front)?;
        matches.extend(self.field_matches(conn, &front)?);
        matches.extend(self.body_matches());
        matches.sort_by_key(|m| m.span.start);
        matches.dedup();
        matches.truncate(MATCHES_PER_HIT);
        Ok(Hit {
            id: self.id,
            path: self.path,
            kind: self.kind,
            title: self.title,
            score: -self.relevance,
            matches,
        })
    }

    fn title_matches(&self, conn: &Connection, front: &[&str]) -> rusqlite::Result<Vec<Match>> {
        let Some(ranges) = ranges(&self.marked[0], &self.stored[0]) else {
            return Ok(Vec::new());
        };
        let Some(value) = Value::title(conn, &self.path)? else {
            return Ok(Vec::new());
        };
        Ok(ranges
            .iter()
            .filter_map(|range| value.map(&self.stored[0], range, Column::Title, front))
            .collect())
    }

    /// The matches in the `fields` column: each range lies on the line of one value.
    fn field_matches(&self, conn: &Connection, front: &[&str]) -> rusqlite::Result<Vec<Match>> {
        let Some(ranges) = ranges(&self.marked[1], &self.stored[1]) else {
            return Ok(Vec::new());
        };
        let fields = &self.stored[1];
        let mut found = Vec::new();
        for (start, end) in ranges {
            let line_start = fields[..start].rfind('\n').map_or(0, |i| i + 1);
            let line_end = fields[start..]
                .find('\n')
                .map_or(fields.len(), |i| start + i);
            let k = fields[..start].matches('\n').count();
            let Some(value) = Value::field(conn, &self.path, k)? else {
                continue;
            };
            let range = (start - line_start, end.min(line_end) - line_start);
            let column = Column::Field {
                key: value.field.clone(),
                entry: value.entry.clone(),
            };
            let text = &fields[line_start..line_end];
            found.extend(value.map(text, &range, column, front));
        }
        Ok(found)
    }

    fn body_matches(&self) -> Vec<Match> {
        let (Some(ranges), Some(start), Some(line)) = (
            ranges(&self.marked[2], &self.stored[2]),
            self.body_start,
            self.body_line,
        ) else {
            return Vec::new();
        };
        ranges
            .iter()
            .map(|range| body_match(&self.stored[2], *range, start, line))
            .collect()
    }
}

/// The byte ranges between the markers, when removing the markers gives back `stored`; none
/// when it does not (a stored noncharacter confuses the markers).
fn ranges(marked: &str, stored: &str) -> Option<Vec<Range>> {
    let mut clean = String::with_capacity(stored.len());
    let mut found = Vec::new();
    let mut open = None;
    for c in marked.chars() {
        match c {
            OPEN => open = Some(clean.len()),
            CLOSE => {
                if let Some(start) = open.take() {
                    found.push((start, clean.len()));
                }
            }
            _ => clean.push(c),
        }
    }
    (clean == stored).then_some(found)
}

/// A match in the body: spans are offsets in the record, the line text is the body line.
fn body_match(body: &str, (start, end): Range, body_start: usize, body_line: usize) -> Match {
    let line_start = body[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = body[start..].find('\n').map_or(body.len(), |i| start + i);
    Match {
        column: Column::Body,
        span: Span {
            start: body_start + start,
            end: body_start + end,
        },
        position: Position {
            line: body_line + body[..start].matches('\n').count(),
            column: body[line_start..start].chars().count() + 1,
        },
        line: body[line_start..line_end].to_owned(),
    }
}

/// A `field_values` row of a searched scalar.
struct Value {
    field: String,
    entry: Option<String>,
    quoted: bool,
    span: Span,
    position: Position,
}

impl Value {
    const COLUMNS: &'static str = "SELECT field, entry, style, span_start, span_end, line, col \
                                   FROM field_values";

    fn title(conn: &Connection, path: &str) -> rusqlite::Result<Option<Value>> {
        conn.prepare_cached(&format!(
            "{} WHERE path = ?1 AND field = 'title' AND entry IS NULL AND item IS NULL \
             AND shape = 'scalar'",
            Value::COLUMNS
        ))?
        .query_row([path], Value::read)
        .optional()
    }

    fn field(conn: &Connection, path: &str, search_line: usize) -> rusqlite::Result<Option<Value>> {
        let line = i64::try_from(search_line).unwrap_or(i64::MAX);
        conn.prepare_cached(&format!(
            "{} WHERE path = ?1 AND search_line = ?2",
            Value::COLUMNS
        ))?
        .query_row(params![path, line], Value::read)
        .optional()
    }

    fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
        let number = |i: usize| -> rusqlite::Result<usize> {
            let n: i64 = row.get(i)?;
            usize::try_from(n).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(i, n))
        };
        let style: Option<String> = row.get(2)?;
        Ok(Value {
            field: row.get(0)?,
            entry: row.get(1)?,
            quoted: style.as_deref().is_some_and(|s| s != "plain"),
            span: Span {
                start: number(3)?,
                end: number(4)?,
            },
            position: Position {
                line: number(5)?,
                column: number(6)?,
            },
        })
    }

    /// The match of `range` in the value `text`: the word itself when the span is the text plus
    /// its quotes, else the whole value (an escape shifts the offsets).
    fn map(
        &self,
        text: &str,
        (start, end): &Range,
        column: Column,
        front: &[&str],
    ) -> Option<Match> {
        let quotes = if self.quoted { 2 } else { 0 };
        let exact = self.span.end - self.span.start == text.len() + quotes;
        let (span, position) = if exact {
            let shift = quotes / 2;
            (
                Span {
                    start: self.span.start + shift + start,
                    end: self.span.start + shift + end,
                },
                Position {
                    line: self.position.line,
                    column: self.position.column + shift + text[..*start].chars().count(),
                },
            )
        } else {
            (self.span, self.position)
        };
        let line = front.get(self.position.line.checked_sub(2)?)?;
        Some(Match {
            column,
            span,
            position,
            line: (*line).to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::fts_query;

    #[test]
    fn words_become_quoted_terms() {
        assert_eq!(fts_query("a b").as_deref(), Some("\"a\" \"b\""));
        assert_eq!(fts_query("\"a b\" c").as_deref(), Some("\"a b\" \"c\""));
        assert_eq!(fts_query("a\"b").as_deref(), Some("\"a\" \"b\""));
        assert_eq!(fts_query("x\"y z").as_deref(), Some("\"x\" \"y z\""));
    }

    #[test]
    fn control_characters_separate_words() {
        assert_eq!(fts_query("a\0b").as_deref(), Some("\"a\" \"b\""));
        assert_eq!(
            fts_query("a\u{7}b\u{1b}c").as_deref(),
            Some("\"a\" \"b\" \"c\"")
        );
        assert_eq!(fts_query("\"a\0b\"").as_deref(), Some("\"a b\""));
        assert_eq!(fts_query("\0\u{7f}"), None);
    }

    #[test]
    fn no_term_gives_none() {
        assert_eq!(fts_query(""), None);
        assert_eq!(fts_query("   "), None);
        assert_eq!(fts_query("\"\""), None);
        assert_eq!(fts_query("\"  \""), None);
    }
}
