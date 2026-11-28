# theos-content

Library. Record model, front-matter patcher, body grammars (beats, voice, notes), kind registry,
index, search, effective lenses, validators, lore rules and ledger. Pure filesystem; no async; no
UI.

After phase P02 the crate provided records and grammars. After P03 it also provided the project,
the kind registry, the persistent index, search and effective lenses. After P04 it also validates:
body rules, lore rules across records, the reveal ledger, validation settings, binary checks and
the Shared Formalism mirror. After P46 it also diffs texts and records.

## What it provides

| Module | What it does |
|---|---|
| `text` | `Span` (a byte range), `TextIndex` (byte offset, line and column, UTF-16), `TextEdit` and `apply_edits`, line-ending normalisation |
| `record` | `Record`: reads a record (`read_record`, `decode`, `Record::parse`), splits it into front matter and body, `was_normalized()` |
| `yaml` | The restricted YAML of the data model: one rule per forbidden form, the front-matter model with spans (`FrontMatter`, `Field`, `Scalar`) and the patcher (`Record::patch`, `set`, `remove`, `set_entry`, `remove_entry`, `Order`) |
| `body` | One pass over the body: blocks, wikilinks, callouts (`[!voice]`, `[!design]`, `[!note]`), design notes, beats of a script, and the malformed constructs with their line and column; `body::tree` gives the body as a block tree (`tree()`, `Node`, `Inline`) for a view to draw |
| `project` | `Project::open` reads `theos.toml` (name, kind, enabled parts); `SCHEMA_VERSION` |
| `finding` | `Finding` and `Severity`: rule id, path, span, line and column, message; `Severity` serialises (`Serialize`) as its lowercase name |
| `kinds` | `KindRegistry` of `KindSpec`s: declarative `FieldSpec` tables, id rules, `Placement`, body grammar; the generic checker |
| `parts` | The 20 builtin kinds, one module per kind and one `register` per part; `parts::builtin()` |
| `check` | `check_record` runs the record, YAML and schema checks of one record, the checks of its kind included; `rules()` lists them. Link and duplicate-id checks run in the index's relink step; body checks and lore rules are P04's (below) |
| `walk` | `walk` lists the record files of a project (`ignore` crate), as the next section says; `is_record_path` |
| `rules` | The rule table: `RuleInfo` (id, default severity, `RuleClass`), `rules::all`, the guardrails, `effective` severity |
| `lore` | Lore rules and what they read: `LoreRule`, `On`, `Context`, the fact types |
| `settings` | `[validate]` of `.theos/settings.toml`: `parse` gives `ValidationSettings` and `SettingsProblem`s |
| `formalism` | The Shared Formalism mirror: `Source`, `Equation`, `render`, `regenerate` |
| `index` | `Index`: the persistent index with sync, update, apply, preview, queries, search, lens and dump |
| `diff` | Diffs for review (P46): `words` (a word-level diff of two texts), `fields` (old and new per front matter field, compared by value) and `record` (a record's fields plus a word diff of its body) |

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

## After P04: validation

**The rule model.** Every rule has an id (`group/name`), a default severity and a class
(`rules::RuleClass`): a *guardrail* is always an error and nothing lowers it; a *configurable*
rule takes its severity from the project settings; a *fixed* rule keeps its default.
`rules::all(&registry)` lists every rule, kind-registered ones included. `Index::findings` and
`findings_for` report with the configured severities.

**Body rules.** The index checks the body of each record with the grammar of its kind: P02's
malformed constructs, voice tiers, and the beats and notes of the kinds that have them (`when::`
tiers, `at::` links, repeated and unknown keys, beat numbers, note ids and states). A construct
that its kind gives no meaning, such as a beat heading in a topic, gives no finding. Body rules
run in the parse step of a record and never read another record. The parse step also stores the
record's in-world text: voice blocks, in-world topics, belief titles and dialogue bodies, never
voice titles (malformed ones included), design notes or a dialogue's level-2 headings. Developer
words in that text are a lore rule (`lore/inworld-vocab`, below).

**The lore step.** After the links are resolved, in the same transaction, each subject (a record
or an input file) runs the lore rules that match it (`LoreRule` with `On::Kinds`, `On::AnyRecord`
or `On::Input`). A rule gets its data only through a `lore::Context`, which stores every read in
the table `deps`. A subject is dirty when it was parsed again or when it read a key that the
transaction changed, so a change re-runs exactly the subjects that read something it touched. A
cold build and an incremental run are the same code; when more than 32 subjects with a matching
rule are dirty the facts are loaded at once, otherwise by id. The rules cover places, truths,
clues, their order and gates, `requires` cycles, reveal sites, beliefs and lenses, developer words
in in-world text (with the words of `.theos/settings.toml`) and the formalism mirror. Parts
register more rules with `KindRegistry::register_rule`. `dump()` shows the reads.

**The ledger.** For every truth the index stores where it is hinted, suspected and confirmed
(reveal sites with their floor, gate and order). `Index::ledger()` and `ledger_of(id)` return
`TruthLedger`s; `theos ledger` prints them.

**Validation settings.** `settings::parse` reads the `[validate]` table of
`.theos/settings.toml`: severities of configurable rules and the project's developer words with
allowlist entries. Two kinds of fault differ. A file that is not valid TOML, or a value of the
wrong TOML type (a shape problem), gives `settings/toml` and empty settings: `[validate]` counts
as empty. A bad single entry (a guardrail, a fixed or an unknown rule named, a bad severity
string, an allow entry without a reason or with an unknown key, a bad word) gives a finding with
a span and has no effect, while the rest of the settings stays. One word fault reaches further:
a word list too large to match gives `settings/word` at the list, and the lore step then checks
only the built-in words and drops the allowlist, the safe side for the guardrail. The guardrails
are `lore/never-confirm`, `lore/no-overshoot` and `lore/inworld-vocab`: they are guardrails by
name, always errors on read, and no setting names them. The settings file and
`canon/formalism/_source.json` are inputs of the index; like records, they count only when the
file on disk is spelled exactly so. `update`, `apply` and `preview` take them as changes too,
though they are no records, so a preview of a settings change gives the findings it would cause.

**Binary checks.** `Index::binary_findings(root)` checks the files that records name (images,
outputs of versions): they exist inside the folder, and their digest matches `@sha256:`. A
reference leaves the binaries folder, and is reported as `binary/missing` and never opened, when
it is empty, starts with a separator, has a `..` segment or a `:` in any segment, or when the
file it resolves to (links followed) lies outside the folder. Like records, a reference counts
only when the folder spells it exactly so: a file that Windows or macOS finds under another case
is `binary/missing` there too, with the spelling on disk in the message. Hashes are memoised by
size and modification time and are computed without the index's write lock; the result is a
`BinaryReport`.

**The formalism mirror.** `canon/formalism/_source.json` is parsed by `formalism::parse`
into a `Source` of `Equation`s; `formalism::render` makes the generated equation records and
`regenerate` writes those whose bytes differ. It never deletes, and it never writes through a
symbolic link or outside the project. Like the index, it takes the source and the records only
under exactly their names: where the file system finds one only under another spelling, as
Windows and macOS do for another case, it writes nothing and names both spellings. The rules
report an equation record that is stray, edited or missing.
`theos formalism` runs the regeneration.

## What later phases use from P04

| Phase | Uses |
|---|---|
| P05 | `findings()` with configured severities; `binary_findings(root)`; `ledger()` and `ledger_of`; `rules::all` with classes; `preview` with neighbours for the hook; `theos ledger` and `theos formalism`, which P05 extends |
| P06 | `Preview.findings` with the neighbours to reject a command; blocking means error after configuration, and guardrails are always errors |
| P07 | The same for proposals; `generated: true` and the `equation` kind to lock the mirror; truth `target` to lock `never` truths |
| P09 | The `notes` rows (id, state, version) for open notes in run requests |
| P11, P12 | Body findings and positions while typing, through `preview`; in-world segments to flag developer words inline |
| P14 | `rules::all` and classes for the settings form; `[validate]`; `preview` of a settings change |
| P15 | Never-confirm on the beat; `ledger_of` for the truth screen; reveal sites |
| P17 | `preview` of a draft runs every lore rule |
| P21 | Rules registered by parts; `BodyGrammar` with step attributes; quest steps as reveal sites; the cycle walk |
| P26 | `BinaryRefs` checks for style `refs`; the cycle walk for `extends` |

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

## Which files are records

The walk lists every file below the project root whose name ends in `.md`, except:

- hidden files and folders, the native way of each OS: on Linux and macOS a name that starts with
  `.`, on Windows the hidden attribute, whatever the name. A `.drafts/` folder is hidden on Linux
  and macOS, and on Windows only when it carries the attribute;
- the folders `.theos`, `.git`, `.obsidian`, `.vscode`, `.github`, `.worktrees` and `.trash`, by
  name, at any depth and on every OS;
- what a `.gitignore` inside the project keeps out (never one above the root, the global excludes
  or `.git/info/exclude`), and symbolic links, which are not followed;
- every `README.md`, and everything under `templates/records/`, `tools/` and `attachments/`.

`update`, `apply` and `preview` take a path by the same rules, so a path they index is one the
next sync keeps. The command boundary of `theos-studio` judges a plan by the same facts,
`walk::is_hidden_entry` and `walk::disk_spelling`.

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
`tests/index/speed.rs`): about 200 ms in a debug build on a Windows development machine, lore
step included (P03, before it: about 155 ms), and a one-file change through `update` takes about
4 ms.

## The fixture projects

`tests/fixtures/projects/` holds small content projects, each with an `expected.toml`:
`schema/` (one record per rule and field type, valid and invalid), `links/` (links, parents,
backlinks and dangling targets), `lenses/` (layers of lenses and beliefs) and `search/` (ranking
and matches) from P03; from P04 `body/`, `world/`, `lore/`, `story/`, `versions/`, `settings/`,
`binaries/` and `formalism/` (one per group of rules). Every record is marked synthetic. The
tests in `tests/index/` index them in place with `Index::in_memory`, so the repository never gets a `.theos/cache`; a test that needs the file
cache copies a project into a temporary folder first.

## Who uses what

| Phase | Uses |
|---|---|
| P05 | `Project::open`, `SCHEMA_VERSION`, `findings`, `preview` for the hook, `KindSpec`, `Placement` and `order()`, `search`, records, links, backlinks and `lens` for the MCP read tools |
| P06 | `preview` before every write and `update` after it; `KindSpec::order()`; backlinks with spans for renaming an id; `Placement`; `walk::is_hidden_entry` and `walk::disk_spelling` for the command boundary |
| P07 | One project and one `.theos/cache` per agent worktree; `preview` for proposals; `diff::record` and `diff::words` for `theos review show` |
| P11, P12 | `search`, backlinks, slot targets for `[[` suggestions, `link/dangling` findings; P11 draws the record body from `body::tree` (`tree()`, `Node`, `Inline`) and sends `Severity` through its `Serialize`; P12 also `diff` to compare an unsaved edit with the saved record |
| P15 | `lens` and its beliefs; links for the graph |
| P17 | `preview` for drafts; `unaware_of` of the effective lens; embeddings in the same store |
| P46 | `theos-studio`'s `diff` builds its `FileDiff` on `diff::record` and `diff::words` |
| P19 | Comment lists may be cached in the same store |
