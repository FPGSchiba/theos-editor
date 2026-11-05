# theos-content

Library. Record model, front-matter patcher, body grammars (beats, voice, notes), kind registry,
index, search, effective lenses, validators, lore rules and ledger. Pure filesystem; no async; no
UI.

After phase P02 the crate provided records and grammars. After P03 it also provides the project,
the kind registry, the persistent index, search and effective lenses. Validators, lore rules and
the ledger follow in P04.

## What it provides

| Module | What it does |
|---|---|
| `text` | `Span` (a byte range), `TextIndex` (byte offset, line and column, UTF-16), `TextEdit` and `apply_edits`, line-ending normalisation |
| `record` | `Record`: reads a record (`read_record`, `decode`, `Record::parse`), splits it into front matter and body, `was_normalized()` |
| `yaml` | The restricted YAML of the data model: one rule per forbidden form, the front-matter model with spans (`FrontMatter`, `Field`, `Scalar`) and the patcher (`Record::patch`, `set`, `remove`, `set_entry`, `remove_entry`, `Order`) |
| `body` | One pass over the body: blocks, wikilinks, callouts (`[!voice]`, `[!design]`, `[!note]`), design notes, beats of a script, and the malformed constructs with their line and column |
| `project` | `Project::open` reads `theos.toml` (name, kind, enabled parts); `SCHEMA_VERSION` |
| `finding` | `Finding` and `Severity`: rule id, path, span, line and column, message |
| `kinds` | `KindRegistry` of `KindSpec`s: declarative `FieldSpec` tables, id rules, `Placement`, body grammar; the generic checker |
| `parts` | The 20 builtin kinds, one module per kind and one `register` per part; `parts::builtin()` |
| `check` | `check_record` runs the record, YAML and schema checks of one record, the checks of its kind included; `rules()` lists them. Link and duplicate-id checks run in the index's relink step; body checks come with P04's validators |
| `walk` | `walk` lists the record files of a project (`ignore` crate); `is_record_path` |
| `index` | `Index`: the persistent index with sync, update, apply, preview, queries, search, lens and dump |

The patcher changes only the lines of the field it is asked to change: every edit is a span edit
on the text, the result is parsed again and checked, and nothing is returned when the check fails.
Bodies are never rewritten by the crate; callers build `TextEdit`s from the spans of the model.

## Coordinates

Every span is a byte range into the LF text of the whole record, front matter included
(`Record::text()`). For an LF file these are file offsets. For a CRLF file they are offsets into
the text a write would produce, never into the bytes on disk. The same text always gives the same
spans. Positions for people are a line and a column counted from 1 in Unicode scalar values.
`TextIndex` converts between the two and to UTF-16.

A write turns the whole file into LF; for LF files a write that changes nothing returns the same
bytes. `Record::was_normalized()` tells a caller that saving an unchanged CRLF record would change
its bytes.

## The fixture corpus

`tests/fixtures/corpus/` is one content project, shared by P03 to P05 and by the display tests of
P12:

- `records/` and `malformed/`: valid records, ids unique across both, file names equal ids;
- `crlf/`: CRLF, mixed and lone-CR inputs with their LF twins (`twins.toml`);
- `rejected/`: front matter the restricted YAML refuses, with the rule, line and column
  (`expected.toml`).

Every file says it is synthetic. The integration tests in `tests/corpus/` read the corpus: records
parse, bodies and records round-trip byte for byte through the patcher and through span edits
built from the model, and a field edit changes only that field's lines.

## The index and its cache

`Index::open(&project, &registry)` opens `.theos/cache/index.sqlite` and brings it up to date. It
returns the index and an `Update` that says what was read again. `Index::in_memory` is the same
without a file. A cache that matches the files is not parsed again: only files whose size or
modification time changed are read, and their content hash decides whether anything changed.

- `sync()` compares the folder with the cache; `update(&[paths])` re-reads the named files;
  `apply(&[Change])` takes staged texts and bytes as if they were on disk; `preview(&[Change])`
  returns the findings and changes those texts would cause and leaves the index as it was.
- Queries: `findings`, `findings_for`, `records`, `record`, `records_of_kind`, `backlinks`,
  `children`, `links_from`.
- `search(query, limit)` ranks records with SQLite FTS5. The crate quotes the words of the user;
  the query language is never exposed.
- `lens(holder)` gives the effective lens of a people, faction or npc with the layer each value
  comes from, and the beliefs the holder holds.
- `dump()` writes the whole state as text, for tests and for finding differences.

The cache is a derived copy and can be deleted at any time. It is valid for one identity: the
layout format, the crate version, the `schema_version` of the project, the fingerprint of the kind
registry, the fingerprint of the rule set and the fingerprint of the crate sources. If any one
differs, the cache is dropped and rebuilt. Every write is one `BEGIN IMMEDIATE` transaction with
a busy timeout of 5 s. A cold index of 500 records takes under a second (CE-12.1, timed by
`tests/index/speed.rs`).

## The fixture projects

`tests/fixtures/projects/` holds four small content projects, each with an `expected.toml`:
`schema/` (one record per rule and field type, valid and invalid), `links/` (links, parents,
backlinks and dangling targets), `lenses/` (layers of lenses and beliefs) and `search/` (ranking
and matches). Every record is marked synthetic. The tests in `tests/index/` index them in place
with `Index::in_memory`, so the repository never gets a `.theos/cache`; a test that needs the file
cache copies a project into a temporary folder first.

## Who uses what

| Phase | Uses |
|---|---|
| P04 | `Finding`, `Severity`, the rule list and the relink step for lore rules; `values`, `links` and `beats` as inputs; its own tables in the same store; `BodyGrammar` |
| P05 | `Project::open`, `SCHEMA_VERSION`, `findings`, `preview` for the hook, `KindSpec`, `Placement` and `order()`, `search`, records, links, backlinks and `lens` for the MCP read tools |
| P06 | `preview` before every write and `update` after it; `KindSpec::order()`; backlinks with spans for renaming an id; `Placement` |
| P07 | One project and one `.theos/cache` per agent worktree; `preview` for proposals |
| P11, P12 | `search`, backlinks, slot targets for `[[` suggestions, `link/dangling` findings |
| P15 | `lens` and its beliefs; links for the graph |
| P17 | `preview` for drafts; `unaware_of` of the effective lens; embeddings in the same store |
| P19 | Comment lists may be cached in the same store |
