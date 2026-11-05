CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE files (path TEXT PRIMARY KEY, size INTEGER, mtime_ns INTEGER, hash TEXT NOT NULL,
                    racy INTEGER NOT NULL) WITHOUT ROWID;
CREATE TABLE records (path TEXT PRIMARY KEY, id TEXT NOT NULL, id_fold TEXT NOT NULL, kind TEXT,
                      title TEXT, status TEXT, front TEXT, body_start INTEGER, body_line INTEGER,
                      facts TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE field_values (path TEXT NOT NULL, field TEXT NOT NULL, entry TEXT, item INTEGER,
                           shape TEXT NOT NULL, text TEXT, style TEXT, span_start INTEGER NOT NULL,
                           span_end INTEGER NOT NULL, line INTEGER NOT NULL, col INTEGER NOT NULL,
                           valid INTEGER NOT NULL, search_line INTEGER);
CREATE TABLE links (path TEXT NOT NULL, source_id TEXT NOT NULL, slot TEXT NOT NULL, beat INTEGER,
                    target TEXT NOT NULL, fragment TEXT, span_start INTEGER NOT NULL,
                    span_end INTEGER NOT NULL, line INTEGER NOT NULL, col INTEGER NOT NULL);
CREATE TABLE beats (path TEXT NOT NULL, number INTEGER NOT NULL, name TEXT NOT NULL,
                    span_start INTEGER NOT NULL, span_end INTEGER NOT NULL, line INTEGER NOT NULL);
CREATE TABLE findings (path TEXT NOT NULL, id TEXT, rule TEXT NOT NULL, severity TEXT NOT NULL,
                       span_start INTEGER, span_end INTEGER, line INTEGER, col INTEGER,
                       message TEXT NOT NULL, step TEXT NOT NULL);
CREATE VIRTUAL TABLE search USING fts5(path UNINDEXED, title, fields, body,
                                       tokenize = 'porter unicode61 remove_diacritics 2');
CREATE INDEX records_id ON records(id);
CREATE INDEX records_id_fold ON records(id_fold);
CREATE INDEX records_kind ON records(kind);
CREATE INDEX field_values_path ON field_values(path);
CREATE INDEX links_path ON links(path);
CREATE INDEX links_target ON links(target);
CREATE INDEX beats_path ON beats(path);
CREATE INDEX findings_path ON findings(path);
