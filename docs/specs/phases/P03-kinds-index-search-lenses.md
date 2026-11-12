# P03 · Kinds, index, search, lenses — phase spec

Status: waiting for Jann's approval · 5 Nov 2026 · repo of the build: `theos-editor`
Source: `docs/specs/00-umbrella.md` §3 P03. Scope and acceptance criteria come from there; this
spec decides how.

## 1. Goal

After P03, `theos-content` knows what a record of each kind must hold, and it keeps a persistent
index of a whole content project. Through that index it answers:
- what records exist and what is wrong with them (findings with path, line and column);
- what links to what (backlinks, children);
- full-text search with the matching text;
- an npc's effective lens, with the layer each value comes from.

The index lives in `.theos/cache/index.sqlite`. Opening a project does not re-read every file:
it compares the files' sizes and modification times with the cache and re-parses only what
changed. The editor, `theos` and every `theos-mcp` process share the one cache. The same steps
also check text that is not on disk, such as a command before it writes or a staged snapshot,
without changing the cache (§5.9).

Every later core phase builds on it:
- P04 adds lore rules as further checks over the same records, links and findings;
- P05 prints the findings (`theos check`) and answers the MCP read tools from the index;
- P06 calls the index after every command and renames ids through its backlinks;
- P11, P12 and P15 show search, backlinks, link suggestions, dangling links and lenses.

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-11 Kinds and schemas**
- CE-11.1 Every kind of the data model has a schema; every corpus record validates or reports
  exact errors.
- CE-11.2 A link to a kind not allowed for that field is an error.
- CE-11.3 A `schema_version` mismatch gives a clear message.
- CE-11.4 Unknown fields warn.
- CE-11.5 Kinds are registered by editor parts through the registry, not hard-coded in one list.

**CE-12 Index**
- CE-12.1 A full index of 500 records takes under one second on the CI runners.
- CE-12.2 Changing one file updates that record and its backlinks only.
- CE-12.3 Duplicate ids, dangling links and file names that differ from ids are findings with paths.
- CE-12.4 Deleting the cache rebuilds the same index.

**CE-13 Search and effective lenses**
- CE-13.1 Full-text search returns ranked records with the matching text.
- CE-13.2 An npc's effective lens shows which layer each value comes from.
- CE-13.3 A child overriding a parent value shows as an override.

## 3. Scope

**Goals**
- In `theos-content`:
  - a kind registry that editor parts fill;
  - one declarative schema per kind of data model §6, and a checker for one record;
  - link resolution with kind checks;
  - a persistent, incremental index in SQLite with full-text search (FTS5), and a preview that
    checks text not on disk without changing the cache;
  - effective lenses;
  - reading `theos.toml`.
- Every part is built on a crate where one exists (engineering directive 2): `rusqlite` with
  bundled SQLite for the store and the search, `ignore` for walking folders, `toml` for
  `theos.toml`, `sha2` for content hashes, `regex` for id patterns. Our code is the schema
  tables, the checker, link resolution, the sync between files and cache, and the lens.
- Findings with rule, severity, path, line and column, the type P04 extends.

**Non-goals**
- Lore rules, the ledger, guardrails, severity configuration and neighbour validation (P04).
  P03's checks judge one record's shape and whether its links resolve to the right kinds.
  Checks that compare values across records are P04's: a floor's number inside its band's range,
  `picked` naming versions of the same parent, `from` within the same parent, `addresses` naming
  existing notes (CE-15.5), parent cycles.
- Turning P02's malformed body constructs into findings, and judging the well-formed body data
  that P02 keeps: tier tokens, `when::` values, the number of links per attribute, beat
  numbering, duplicate and unknown attribute keys, note ids and states. P02 §5.12 gives all of
  these to P04.
- A file watcher. P03 offers `update(paths)` and `sync()`; P06 and P11 decide when to call them.
- Writing records, `theos new`, `theos schema`, JSON Schema export (P05, P06). The registry
  holds what they need.
- Using `[parts] enabled` of `theos.toml` (§5.3 ruling); semantic search and embeddings (P17).

**Carried over from P02.** The plan's Task 1 fixes P02's open finding R5-1 (a `Design note:` line
in a tight list item inside a callout is not found), as Jann asked. It is not part of this
spec's design.

**Anti-goals** (P03 fails if any of these holds, even with every test green)
- Opening a project re-reads or re-parses every record when nothing changed.
- The cache becomes a source of truth: a result that differs from what the files say once the
  cache is deleted.
- A hand-written parser for TOML, YAML, Markdown, SQL or a search query language (directive 2).
- One list in one place that names all kinds (CE-11.5).
- Lore invented in fixtures.

## 4. Decisions

### 4.1 Decided by Jann (5 Nov 2026)

- **Ruling (Jann): the index persists in SQLite, with FTS5 for search, and opens
  incrementally.** One file, `.theos/cache/index.sqlite` (`rusqlite` 0.41 with bundled SQLite
  3.54, WAL journal). Opening walks the project and compares each file's size and modification
  time with the cache; only changed files are read, and only changed contents are re-parsed.
  Jann rejected an in-memory index rebuilt at every start: "I am not really a fan of long
  startup times and reindexing all files makes not much sense to me." The cost of a full rebuild
  grows with the project and is paid by every process: the editor, each `theos check` in the
  hook, each `theos-mcp`. SQLite lets those processes share the cache safely, which tantivy's
  one-writer-per-index rule does not.
- **Ruling (Jann): schemas are declarative tables.** One small Rust module per kind declares its
  fields as data: key, type, required. One generic checker walks P02's front-matter model and
  collects every problem with its span. The same table gives the patcher's field order, the link
  targets for P12's `[[` suggestions, and what P05's `theos schema` prints.
- **Ruling (Jann): the kind registry and the parts that fill it live in `theos-content`.** One
  module per editor part (`codex`, `scripts`, `concepts`, `maps`), each with a `register`
  function that adds its kinds. `theos-studio`'s editor-part registry (from P06) calls them. The
  CLI, the MCP server and P04's rules get the registry without the studio.

### 4.2 Probes (5 Nov 2026, toolchain 1.99.1, Windows 11)

A scratch crate outside the repo used `theos-content` from `origin/main`.

**P02's parser, debug build:** 500 records (the corpus, cycled) parse in 66 ms. CE-12.1's limit
leaves room for slower CI runners and coverage instrumentation.

**`rusqlite` 0.41.1, feature `bundled` (SQLite 3.54.1):**
- builds with the MSVC C compiler that Rust on Windows already needs;
- FTS5 is part of the bundled build;
- tokenizer `porter unicode61 remove_diacritics 2`:
  - `hum` finds `hums`, and `tuner` finds `Tuners` (stemming);
  - `cafe` finds `café`, and `mere` finds `Mère`;
  - the quoted query `"T-012"` finds `T-012` in prose and inside `[[T-012|…]]`;
- an unquoted `T-012` is an FTS5 syntax error ("no such column: 012"), so user text must be
  quoted before it reaches `MATCH` (§5.11);
- `highlight()` with two marker characters: removing the markers gives back the stored text
  byte for byte, so match spans are exact;
- a cold build of 500 records (P02 parse, rows, FTS rows, one transaction, debug build) takes
  97 ms, and one search 0.3 ms;
- `rusqlite`, `ignore` and `theos-content` together make a tree of 53 crates;
- `cargo deny` with the repo's `deny.toml`: licences, bans and sources pass.

**Also probed:**
- `ignore` 0.4.34 (Unlicense OR MIT) walks and stats the corpus.
- `url` 2.5.8 pulls in 46 crates (ICU for IDNA); it is not used (§9).
- `regex` 1.13.1 adds only the `regex` crate itself: `globset` already brings `regex-automata`,
  `regex-syntax`, `aho-corasick` and `memchr` into the lockfile.
- `tantivy` 0.27.0 (default features off) builds and passes `deny.toml`, with about 118 crates
  in its tree; a RAM index of 500 documents takes 42 ms. It is not used (§4.1, §4.3).

### 4.3 Approaches considered

| Question | Chosen (Jann) | Considered |
|---|---|---|
| Index and search | SQLite + FTS5 in `.theos/cache`, incremental open | an in-memory index rebuilt at open with tantivy in RAM; a JSON snapshot with tantivy on disk (one writer per index across processes, ~110 more crates); a JSON snapshot with our own BM25 (custom code, last writer wins) |
| Schemas | declarative tables per kind | typed serde structs per kind (first error only, positions need our own Deserializer); JSON Schema files with `jsonschema` (P02's ruling D1 types scalars by field, link kinds need a custom keyword, error paths must be mapped to spans) |
| Registry | part modules in `theos-content` | a `Part` trait with the parts in `theos-studio` (`theos-content`'s own tests and P04 would need a test registry or a dependency on the studio) |

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Module (indicative) | Crate or tool | Our code, and why |
|---|---|---|---|
| `theos.toml` | `project` | `toml` (with `Spanned`), `serde` | Read three keys and judge `schema_version` |
| Kind registry and parts | `kinds`, `parts::{codex, scripts, concepts, maps}` | – | CE-11.5: one module per kind, one line per kind in its part |
| Field types | `kinds::types` | `regex` (id patterns) | Data model §3 and §7 as data |
| Checking one record | `check::record` | P02's model | One module per rule, registered in one list |
| Which files are records | `walk` | `ignore` (`WalkBuilder`) | P01's "not records" list (P01 spec §5) |
| Content hashes | `index::sync` | `sha2` | – |
| Source fingerprint | `build.rs` | `sha2` (build dependency), cargo's `rerun-if-changed` | Cache identity across developer builds (§5.8) |
| Store, queries, search | `index::store` | `rusqlite` (bundled, FTS5) | Tables and SQL of our own; no ORM |
| Link resolution | `index::links` | SQL over the store | Slot targets of §5.6 |
| Search queries | `index::search` | FTS5 `MATCH`, `bm25`, `highlight` | Quoting user words (§5.11); markers → spans |
| Effective lens | `index::lens` | SQL over the store | Data model §7 inheritance |
| Errors | each module | `thiserror` | G16 |

New dependencies of `theos-content`: `rusqlite` (feature `bundled`), `ignore` and `regex`
(added to `[workspace.dependencies]`), and `toml`, `serde` and `sha2` as normal dependencies
(already in the workspace). `[workspace.dependencies]` is not a protected path.

### 5.2 The project: `theos.toml` (CE-11.3)

- `Project::open(root)` reads `<root>/theos.toml` with the `toml` crate: `[project] name`,
  `kind`, `schema_version`, and `[parts] enabled`. Other keys are not P03's and are ignored.
- `theos_content::SCHEMA_VERSION` is `1`, the data model version this build reads. P05's
  `--version` prints it (CE-08R.5).
- `Project::open` fails, and nothing is indexed, when:
  - there is no `theos.toml`: "`<root>` is not a content project: it has no theos.toml";
  - the file is not valid TOML: the `toml` crate's message, with line and column;
  - `schema_version` is missing, or is not an integer;
  - `schema_version` is not `1`.
- **The message for a mismatch** names the file, the line and column of the value, the version
  found and the version this build reads, and what to do:
  - higher: "theos.toml:4:18: this project uses data model schema_version 2, but this build of
    theos reads schema_version 1. Update theos to open it.";
  - lower: "theos.toml:4:18: this project uses data model schema_version 0, but this build of
    theos reads only schema_version 1."
- The cache is never touched for a project that does not open.
- `kind` and `name` are read and exposed, not judged. Start (P14) decides what a `game` project
  does.

### 5.3 The kind registry and the parts (CE-11.5)

```text
KindRegistry::new()                          empty: knows no kind
registry.register(part, KindSpec)            fails with DuplicateKind for a kind registered twice
registry.kind(name) · kinds() · part_of(name)
registry.fingerprint()                       SHA-256 of a canonical description (§5.8)

parts::codex::register(&mut registry)        tower … equation   (13 kinds)
parts::scripts::register(&mut registry)      script, dialogue, template
parts::concepts::register(&mut registry)     concept, concept-version
parts::maps::register(&mut registry)         map, map-version
parts::builtin()                             a registry with the four parts, one line each
```

- The part of each kind is data model §6's "Editor part" column.
- **A new kind is one module plus one line in its part**, and a new part is one module plus one
  line in `builtin()`. No other list names kinds. Tests and later parts (the engine part)
  register kinds of their own the same way.
- Every built-in part registers its kinds, whatever `[parts] enabled` says. Records stay data
  whether a screen shows them or not. `enabled` decides the rail (P11).
- `KindSpec::order()` builds P02's patcher `Order` (fields, and the lens entries) from the
  spec. P02 §5.7 left the order to the registry.

### 5.4 Kind specs: the schema tables (CE-11.1)

```text
KindSpec  { name, id: IdRule, placement: Placement, fields: [FieldSpec], body: BodyGrammar,
            checks: [KindCheck] }
FieldSpec { key, ty: FieldType, required }
```

**Fields** are listed in the data model's order: the fields every record has (§5: `id`, `kind`,
`title`, `status`, `tags`), then the kind's table (§7, §8). That order is the patcher's order,
and P01's templates follow it.
- `status` is required for topic, truth, belief, clue, floor and script (§5 "lore kinds, floor,
  script"), as in P01's checker. An equation does not require it: the mirror generates
  equations (P04, CE-16), and they are locked whatever their status (architecture §9). P01 spec
  §4.5 left this open; §9 records it.
- `state` belongs to concept-version only: data model §8's shared version table gives it, but its
  meaning names concept versions of two-state concepts, and P01's checker has it there only.

**Field types.** Scalars are typed by the field, not by their spelling (P02's ruling D1): every
type reads P02's `Scalar` through the accessor that fits.

| Type | Accepts | Fields |
|---|---|---|
| `Text` | a scalar whose text is not empty | `title`, `mode`, `mood`, `belief_stage`, `role`, `type`, `vector`, `recipe`, lens texts |
| `TextList` | a list, either style, of non-empty texts | `tags` |
| `Enum(values)` | a scalar whose text is one of the values | `status`, `tier`, `track`, `target`, `accuracy`, `moves_to`, `gate`, `purpose`, `rating`, `state` |
| `Integer(range)` | a plain scalar that `as_int` reads, inside the range | `number` (1–100), `seed` (any) |
| `FloorRange` | a list of two integers, 1 ≤ first ≤ last ≤ 100 | band `floors` |
| `Link(targets)` | a scalar; where it points is checked by the index (§5.6) | `parent`, `home`, `topic`, `distorts`, `reveals`, `counterpart`, `speaker`, `concept`, `map`, `from` |
| `Links(targets)` | a list of such scalars | `lives_in`, `where`, `requires`, `cites`, `held_by`, `after`, `for`, `picked` |
| `Url` | text starting `http://` or `https://`, with no whitespace (P01's rule) | `source` |
| `True` | a plain `true` | equation `generated` |
| `Generator` | `name@version`: two non-empty parts without `@` or whitespace | `generator` |
| `NoteIds` | a list of `n<number>` | `addresses` |
| `BinaryRefs` | a map of at least one entry: role → `path@sha256:<64 lower-case hex digits>`, with no `@` in the path | `outputs` |
| `OpenMap` | a map with any keys | `inputs`, `results` |
| `Lens` | a map whose entries follow data model §7's lens table: `thinks_from`, `stance`, `speaks_to` (Text), `tier` (Enum T1–T3), `unaware_of` (Links → truth), `rejects` (Links → belief) | `lens` |

- A required list may not be empty (`held_by: []` is an error), as in P01.
- A link value is not checked against an id pattern. Whether it resolves, and to what kind, is
  the index's question (§5.6). One fault gives one finding.
- `kind` and `id` are special: `kind` selects the spec, and `id` is checked against the file
  name and the id rule (§5.5).

**Targets** of a link are kinds, optionally narrowed by one value of the target record:

| Field | Targets |
|---|---|
| truth `topic` | topic with `track: explaining` |
| belief `topic` | topic with `track: in-world` |
| topic `counterpart` | topic with `track: explaining` |
| everything else | the kinds of data model §7 and §8 (concept `for`: tower, band, floor, location, people, faction, npc, thing) |

**Id rules** (data model §3, the same expressions as P01's checker):

| Kinds | Pattern | Says |
|---|---|---|
| truth, belief, clue, script | `^T-[0-9]{3,}$` (and `B-`, `C-`, `S-`) | "`T-` and at least three digits" |
| equation | `^E[0-9]+$` | "`E` and a number" |
| floor | `^f[0-9]{2}[a-z0-9]*(-[a-z0-9]+)*$` | "`f`, two digits, then an optional slug part" |
| topic | `^<slug>-(explaining\|in-world)$` | "a slug ending in `-explaining` or `-in-world`" |
| concept-version, map-version | `^<slug>-v[0-9]+$` | "the parent's slug, `-v` and a number" |
| all others | `^[a-z0-9]+(-[a-z0-9]+)*$` | "a slug" |

**Placement** (data model §2, §6): where a kind's files sit.
- `Flat(folder)`: tower and band (`world/`), people (`people/`), thing, topic, truth, belief,
  clue, equation, script, dialogue, template, concept and map in their folders.
- `UnderParent(field)`: floor, location, faction and npc under `parent`, concept-version under
  `concept`, map-version under `map`. The file sits in `<folder of the parent's file>/<parent
  id>/`.

**Body grammar** of a kind: which of P02's body constructs mean something there.
- Beats: script only.
- Notes: concept and map only.
- Voice blocks, design notes and mentions: every kind.

Elsewhere a beat heading is an ordinary heading, and a `> [!note]` callout is an ordinary
Obsidian callout. Its links are mentions (§5.6). P04 uses the same table to decide which of P02's
malformed items become findings.

**Kind checks**: rules of one record that a field type cannot express. Each is a named function
in its kind's module:
- topic: `track` matches the id's suffix; an in-world topic has a `counterpart` (P01's checker);
- concept-version, map-version: the id is `<parent>-v<number>` of its `concept` or `map` field
  (P01's checker).

### 5.5 Checking one record

The checks run in this order. Each is a module with one job, registered in one list. Rule ids
are `<group>/<name>`, so P04's rules (`lore/never-confirm`, …) cannot collide with them.

| Rule | Severity | When | Position |
|---|---|---|---|
| `record/path` | error | the file name is not valid UTF-8 (it cannot hold an id) | – |
| `record/utf8` | error | the bytes are not UTF-8 (`decode`) | the first bad byte |
| `record/bom`, `record/opening-line`, `record/closing-line` | error | P02's `RecordError` | 1:1 |
| `yaml/<rule>` | error | P02's `YamlError` (`yaml/anchor`, `yaml/tab-indent`, …) | P02's |
| `schema/kind` | error | `kind` is missing or names no registered kind | the value, or 1:1 |
| `schema/id` | error | `id` is missing, or the file name breaks the kind's id rule | the value, or the `kind` value |
| `index/file-name` | error | `id` differs from the file name (CE-12.3) | the value |
| `schema/required` | error | a required field is missing | the `kind` value |
| `schema/value` | error | a value does not fit its type; one finding per bad list item or map entry | the value, item or entry |
| `schema/unknown-field` | warning | a key the kind does not have (CE-11.4) | the key |
| `schema/unknown-entry` | warning | a lens entry the lens table does not have (CE-11.4) | the entry's key |
| `schema/topic-track`, `schema/topic-counterpart`, `schema/version-id` | error | the kind checks of §5.4 | the value, or the `kind` value |

- A file that cannot be decoded or split (`record/…`) is indexed under its file name with that
  one finding: P02 gives no body for it, so it has no links and no search row.
- A record whose front matter fails (`yaml/…`) is indexed under its file name, without kind or
  fields. Its body is parsed for search and mentions. The rest of the schema is not checked; the
  YAML error is its one schema finding.
- After `schema/kind` fails, no other `schema/` rule runs: without a kind there is no spec.
- Messages name the field and what it needs: "`number` must be an integer from 1 to 100, not
  `104`"; "a truth needs `target`"; "`floors` must be two integers, first ≤ last, from 1 to
  100".

### 5.6 Links: slots, targets and resolution (CE-11.2, CE-12.3)

Every link of a record is stored with its **slot**, the place it sits in:

| Slot | Where | Targets |
|---|---|---|
| field `<key>` | front matter, `Link` and `Links` fields | the field's targets (§5.4) |
| lens entry | `lens.unaware_of`, `lens.rejects` | truth; belief |
| beat attribute | `at::`, `with::`, `says::`, `reveals::` of a script's beats | location, floor; npc, faction, people; belief; clue |
| voice speaker | the first title token of a voice block, any kind | npc, faction, people |
| voice belief | the further link tokens of a voice title | belief |
| note version | the version link in a note title (concept, map) | concept-version in a concept; map-version in a map |
| note resolved-by | the link of `resolved by [[id]]` (concept, map) | the same |
| mention | every other wikilink in a body: prose, design callouts, unknown attribute keys, beats and notes in kinds without them | any kind |

**Resolution.** All of these run in the index, after every record of a change is parsed:
- **Dangling (CE-12.3).** A link resolves when a record with exactly that id exists (ids are
  case-sensitive). Otherwise it is `link/dangling` (error), at the link's span: the scalar in the
  front matter, or the whole `[[…]]` in the body. The message names the id.
- **Beat references.** A link with a fragment, `[[S-004#3]]`, resolves only when the target is a
  script with at least one beat numbered 3. Otherwise it is dangling: "S-004 has no beat 3", or
  "B-016 is a belief; only scripts have beats". Two beats with one number are P04's finding.
- **Wrong kind (CE-11.2).** In every slot except mention, the target's kind must be one of the
  slot's targets, and a narrowed target must have the value: `link/kind` (error), at the link.
  For example: "`home` must link to a location or floor; `mother-ilse` is an npc", or "`topic`
  must link to an explaining topic; `runes-in-world` is in-world".
- **A target without a known kind** (its front matter failed, or its `kind` is unknown) gets no
  `link/kind` finding. The target's own finding already says what is wrong.
- **A duplicated id** resolves, and its kind check passes when any record with that id fits.
- **Placement** (warning, `index/placement`): a record whose file is not where its kind's
  placement puts it, at the parent link for `UnderParent` and at the `kind` value for `Flat`. For
  `UnderParent`, this is checked only when the parent resolves.
- **Duplicate ids (CE-12.3)** (error, `index/duplicate-id`): two record files with the same file
  name in different folders, or with names equal apart from ASCII case. Windows and macOS
  cannot hold the second kind in one folder tree. The finding is on each path, at its `id`
  value or else at 1:1, and names the other paths.

**Backlinks** are the links whose target is a given id, with source id, path, slot, span, line
and column. **Children** are the backlinks in the slot field `parent`, `concept` or `map`.

### 5.7 Which files are records

- The index walks the project root with `ignore`'s `WalkBuilder`:
  - hidden entries are skipped the native way of the OS (Jann's ruling of 10 Nov 2026): on
    Windows, a file or folder with the hidden attribute; on Linux and macOS, a name that starts
    with a dot. On every OS, `.theos/`, `.git/`, `.obsidian/`, `.vscode/`, `.github/`,
    `.worktrees/` and `.trash/` are never records, at any depth. So a file hidden only by its
    attribute on Windows is a record on Linux, and on Windows a dot folder outside that list and
    without the attribute (`.claude/`, `.idea/`) is walked unless `.gitignore` ignores it;
  - `.gitignore` files inside the project are honoured. The global git excludes,
    `.git/info/exclude` and ignore files above the root are not, so every machine sees the same
    records;
  - symbolic links are not followed and are not records.
- A record is a file whose name ends in `.md`, except what P01 spec §5 lists as not records:
  every `README.md`, and everything under `templates/records/`, `tools/` and `attachments/`.
- A record's path is stored relative to the root, with `/` between components, on every OS.
- **A record's id is its file name without `.md`** (data model §2: "the file name is the id";
  Obsidian resolves `[[id]]` the same way). An `id` field that differs is a finding (§5.5);
  links still find the record by its file name.

### 5.8 The store

**One file**, `.theos/cache/index.sqlite`, with SQLite's `-wal` and `-shm` files next to it.
Tables (indicative; the plan names the columns):

| Table | Holds |
|---|---|
| `meta` | cache format version, `theos-content` version, data model `schema_version`, registry, rule set and source fingerprints |
| `files` | path, size, modification time (nanoseconds), SHA-256 of the bytes, racy flag (§5.9) |
| `records` | path, id, kind, title, status, start line of the body |
| `values` | every front-matter scalar, one row each: path, field, entry, item index, text, style, span, line, column |
| `links` | source path and id, slot, beat number, target id, fragment, span, line, column |
| `beats` | script path, beat number, name, span, line |
| `findings` | path, id, rule, default severity, span, line, column, message, and the step that made it (record, links, ids) |
| `search` | FTS5: path, title, fields, body (§5.11) |

- **The cache is never a source of truth.** Everything in it is derived from the files, the
  registry and the rules. Deleting it changes no result (CE-12.4).
- **Identity.** When `meta` does not match the running build, the cache is emptied and rebuilt
  in full. It holds:
  - the cache format version, the `theos-content` version and `schema_version`;
  - the registry fingerprint: every kind's fields, types, targets, id rule, placement, body
    grammar and the names of its checks;
  - the rule set fingerprint: the id and default severity of every registered check, P04's
    included;
  - the source fingerprint: a SHA-256 of `crates/theos-content/src/**` that a build script
    computes at build time (`cargo:rerun-if-changed=src`) and passes in with `env!`. Every
    workspace build has version 0.0.0 until a release, so without it a rule whose code changed
    under the same id would keep serving old findings from the cache.

  A build whose `theos-content` source differs therefore rebuilds the cache once, in under a
  second for 500 records.
- **A damaged cache** (not a SQLite file, wrong tables, a corruption error) is deleted and
  rebuilt, with a `tracing` warning.
- **Several processes.** WAL lets readers run while one writer commits. Every sync or update is
  one `BEGIN IMMEDIATE` transaction with a busy timeout of 5 seconds. Two processes opening
  together: the second waits, then finds nothing to do. Readers see the last committed state.
- **No cache file:** `Index::in_memory(project, registry)` builds the same tables in memory. It
  starts empty: `sync()` fills it from the folder, `apply(changes)` from given texts (§5.9).
  Tests and read-only folders use it.
- **Queries read the store.** The `Index` keeps no copy of the rows. When another process
  commits, the next query sees it.

### 5.9 Opening, syncing and updating (CE-12.1, CE-12.2, CE-12.4)

`Index::open(&project, &registry)` connects, checks the identity, and syncs.

**Changes.** Steps 3 to 5 below work on a list of changes. A change is a path with its text read
from disk, a path with a given text, or a removed path.
- `sync()` and `update(paths)` make the changes from the folder.
- `apply(changes)` takes them as given and commits them.
- `preview(changes)` runs steps 3 and 4 on given changes inside a transaction that it rolls
  back. It returns the `Update` it would have made and the findings of every record it parsed or
  relinked, and writes nothing. P06 checks a command with it before writing (CE-20.1), P07 a
  proposal, P17 a draft (CE-46.3), and P12 the text being typed (CE-35.2). P05's hook checks the
  staged snapshot the same way (CE-17.2): the staged text of every file whose working copy
  differs from it, and the removal of files that are not staged, on the shared cache. A commit
  therefore re-parses only those files.

**`sync()`**:
1. Walk the project (§5.7). For each record path, take its size and modification time from the
   walk's metadata (free on Windows), and compare with `files`:
   - unknown path: new;
   - size or time differ, or the row is racy: read and hash the bytes. With the same hash,
     only the `files` row changes. With a different hash, the record changed;
   - otherwise: unchanged, and the file is not read.
2. Paths in `files` that the walk did not see: removed.
3. Each new or changed record is parsed (P02), checked (§5.5), and its rows are replaced:
   `records`, `values`, `links`, `beats`, its record findings, and its `search` row.
4. **Relink** (§5.6) the affected sources:
   - the links of every new or changed record;
   - when a record's link facts changed (it appeared or disappeared, or its id, kind, path,
     beat numbers or a narrowing value such as `track` changed), the links of every record that
     links to its old or new id, and the placement of its children.

   Duplicate-id findings are recomputed for the old and new ids.
5. Commit, and return `Update { read, reparsed, removed, relinked }`, lists of paths.

**`update(paths)`** does the same for the given paths only. The command layer knows what it
wrote, and a watcher knows what changed. A given path that no longer exists counts as removed;
one that is not a record (§5.7) is ignored, in `apply` and `preview` too. A record committed by
`apply` from a given text has no file times in `files`, so the next sync reads that path again.

**Racy files.** A file whose modification time is less than 2 seconds before the moment it was
hashed is stored as racy, and hashed again at the next sync. This is git's "racy git" rule: an
edit right after indexing that keeps the size and falls in the same timestamp tick cannot be
missed on file systems with coarse timestamps (FAT 2 s, HFS+ 1 s). The flag clears once the
file is older than the window.

**CE-12.2** "updates that record and its backlinks only" means exactly this: one changed file is
read and re-parsed, and the only other work is relinking the records that link to it.
**CE-12.4** "rebuilds the same index": a cold build and an incremental history give the same
rows (§5.14).

### 5.10 Findings

```text
Finding { rule, severity, path, id, span, position, message }
Severity = Error | Warning
```

- `rule` is `<group>/<name>` (§5.5, §5.6). `path` is project-relative with `/`. `span` and
  `position` are P02's coordinates in the record's LF text. They are absent only for
  `record/path`.
- `Display` gives `path:line:column: error schema/value: message`, the form P05 prints.
- `findings()` returns them sorted by path, position and rule; `findings_for(path)` returns one
  record's.
- The store keeps each finding's default severity, which is the table's. P04 adds configurable
  severities and applies them when findings are read, so a settings change needs no rebuild.

### 5.11 Search (CE-13.1)

`search(query, limit)` returns hits, best first.

- **Columns:**
  - `title`: the `title` value;
  - `fields`: `id` and the front-matter values of type Text, TextList, Link and Links, lens
    entries included, one per line, in file order;
  - `body`: the body text exactly as in the record.
- **Tokenizer:** `porter unicode61 remove_diacritics 2`, so stems, case and accents match
  (§4.2).
- **The query** is split at whitespace into words; a run in double quotes is one phrase. Each
  word or phrase reaches `MATCH` as an FTS5 string, with its quotes doubled. Every word must
  match. No other FTS5 syntax is exposed, so `AND`, `NEAR`, `*`, `-`, `:` and `^` in user text
  are plain text. An empty query, or one of only spaces, has no hits.
- **Ranking:** `bm25` with the weights title 5, fields 2, body 1. A title match outranks a body
  match. Ties go by id.
- **The matching text.** `highlight()` marks the matches with the Unicode noncharacters
  U+FDD0 and U+FDD1, which Unicode reserves for internal use. If the text without the markers
  differs from the stored text, the hit keeps its rank and loses its spans; it never gets wrong
  ones. Each match has:
  - its column (title, a field and its key, or body);
  - its span in the record and its line and column. In the body these are counted from the
    stored body start. In the title and fields, through `values`: the matched words when the
    value's source spelling is its text (plain, or quoted without escapes), else the whole
    value;
  - the text of its line.

  Matches come in document order; the plan sets how many a hit carries.
- **A hit** has id, path, kind, title, score and its matches.
- Every record is searchable. One whose front matter fails is found by its body; a file that
  cannot be decoded or split has no search row (§5.5). Truths with target `never` are found like
  any record. G9 is about prompts; P05 marks them in MCP results (CE-18.3).

### 5.12 Effective lenses (CE-13.2, CE-13.3)

`lens(holder)` gives the effective lens of a people, faction or npc record. It gives nothing for
an unknown id or another kind.

- **The chain** is the holder, then its parent, and so on, as long as the parent resolves to an
  allowed parent kind: npc → faction or people; faction → people; people has no parent. A
  dangling or wrong-kind parent ends the chain; its finding exists already.
- **Each lens field**, in the lens table's order, takes the value of the nearest layer that sets
  it. The value carries its origin: id, kind, path, span, line and column. It also carries
  **overrides**: the values of farther layers that set the same field, nearest first (CE-13.3).
- A field a layer sets replaces the inherited value as a whole, lists included (data model §7:
  "A child inherits each lens field it does not set").
- A child that sets the same value as its parent still overrides it. P15 may show "same as
  parent".
- **Beliefs.** The lens also lists the beliefs whose `held_by` names the holder or a layer of
  its chain, each with the layer it comes through. This is "what a holder believes" (data model
  §7, last line); P05 (CE-18.2) and P15 (CE-40.2) read it.
- Unknown lens entries are not part of the effective lens. They warn (§5.5).
- **An invalid entry does not count.** This covers an entry that fails its type (`tier: T4`,
  `unaware_of` written as a scalar), and a whole `lens` that is not a map. The farther layer's
  value applies, and the effective lens lists the ignored entry with its layer and finding. For
  `unaware_of` this keeps the parent's withheld truths instead of none, the safe side for P17
  (CE-46.1).

### 5.13 Fixtures

All fixtures are synthetic and say so: `# synthetic fixture (P03), not canon` in the front
matter. Their texts are neutral ("synthetic stance A"), never lore.

- **The P02 corpus as a project.** Test support copies `corpus/records/` and
  `corpus/malformed/` into one temporary project with a `theos.toml`, as P02 §5.13 prepared
  them. `corpus/expected-index.toml` lists every finding of that project: path, rule, severity,
  line and column. The P02 files do not change. Their dangling links (data model example ids,
  P01 placeholders such as `todo`) stay and are listed. No record is added to make them resolve:
  that would invent lore.
- **The rest of the P02 corpus.** Each file of `corpus/crlf/` is indexed alone in a temporary
  project. A twin reports the findings of its LF twin; the three files without a twin
  (`synthetic-crlf`, `lone-cr`, `mixed`) have their own expected findings. Each file of
  `corpus/rejected/`, indexed alone, reports its P02 rule (`yaml/…` or `record/…`) from P02's
  `expected.toml` as its only finding.
- **Fixture projects** in `crates/theos-content/tests/fixtures/projects/<name>/`, each with its
  own `theos.toml` and `expected.toml`:
  - `schema/`: one record per row of §5.5 and per field type of §5.4, valid and invalid;
  - `links/`: right and wrong kinds in every slot of §5.6, narrowed topic targets, beat
    references, dangling links, duplicate ids in two folders, an id that differs from its file
    name, a misplaced record, a target with broken front matter;
  - `lenses/`: a people → faction → npc chain with values and overrides on each layer, an npc
    directly under its people, an unknown lens entry, beliefs held by each layer;
  - `search/`: records with words in titles, fields and bodies, stems, accents, ids and phrases.
- **Built by the tests** in temporary folders: `theos.toml` variants (CE-11.3); the 500 records
  of CE-12.1 (every kind, links among them, bodies of about a kilobyte); the file changes of
  CE-12.2. Names equal apart from case are tested on paths, without files, because Windows and
  macOS cannot hold both in one folder.

### 5.14 How each AC is proven

Tests carry `// AC: <id>` directly above a `#[test]` function (P00's `ac-trace`).

| AC | Proven by |
|---|---|
| CE-11.1 | The built-in registry has a spec for each of the 20 kinds of data model §6, and each spec's fields (key, type, required, targets) equal a table that the test transcribes from data model §5, §7 and §8. The corpus project's findings equal `expected-index.toml` exactly; the `crlf/` and `rejected/` files report theirs (§5.13). Every record of `schema/` reports exactly its expected findings. |
| CE-11.2 | In `links/`, every typed slot of §5.6 with a wrong-kind target reports `link/kind` at the link's line and column, and with a right-kind target reports nothing. Narrowed topic targets; a target with broken front matter and a duplicated id behave as §5.6 says. |
| CE-11.3 | `theos.toml` with `schema_version` 2, 0, `"1"` and none: `Project::open` fails, and the message names theos.toml and the version this build reads; where the value exists, also its line, column and the version found. With 1 it opens. |
| CE-11.4 | An unknown field and an unknown lens entry give a warning at the key, and no error. The record's other findings are unchanged. |
| CE-11.5 | An empty registry knows no kind. Each built-in part registers exactly the kinds data model §6 gives it. A test part that registers a kind of its own makes records of that kind index and check, with no other change. Registering a kind twice fails. `KindSpec::order()` gives the data model's order. |
| CE-12.1 | 500 generated records; three cold opens (no cache, deleted before each); the fastest is under one second, and all three are printed. It runs in CI's `test` job on Linux and in the weekly Windows and macOS runs. |
| CE-12.2 | Changes, one at a time: a body link added; a title changed; an id changed with its file renamed; a kind changed; a beat removed; a topic's `track` changed; a file deleted; a file added. Each `Update` lists exactly the expected read, re-parsed, removed and relinked paths. After each, `dump()` equals a fresh build of the same files. A second open with no change reads no file. |
| CE-12.3 | In `links/`: duplicate ids report `index/duplicate-id` on each path, naming the other; dangling links in front matter, body and beat references report `link/dangling` with path, line and column; a file name that differs from the id reports `index/file-name` at the id. A unit test over paths finds names equal apart from case. |
| CE-12.4 | Open, take `dump()` and the results of a set of searches; delete `.theos/cache`; open again: both are equal. A damaged cache file and a changed fingerprint also rebuild to the same dump. |
| CE-13.1 | In `search/`: hits are ranked (a title match above a body match; more matches above fewer, other things equal), each with its matching text: span, line, column and the line's text. Stems, accents and ids match; quotes make a phrase; FTS5 syntax in the query is plain text; no match gives no hits. |
| CE-13.2 | In `lenses/`: an npc's effective lens gives each value with its origin layer (npc, faction or people), id and position; an npc directly under people; beliefs through each layer. |
| CE-13.3 | A value set on the faction and on the npc shows the npc's value, with the faction's in `overrides`. A list set on the npc replaces the people's list. The same value set again is still an override. |

`dump()` gives the store's rows in a canonical order: records, values, links, beats, findings and
search rows, without the `files` row's times and racy flags. It is public: tests compare it, and
it helps debugging.

Untagged tests cover the rest of §5.9: a `preview` returns the findings that `update` with the
same changes then stores, and leaves `dump()` unchanged; `apply` with given texts gives the same
dump as the files would; two indexes on one project sync from two threads.

The gate as merged applies: no `allow` or `warn` attributes, a reason on every `expect` and
`ignore`, 80 % line coverage of `crates/theos-content/src/**` with error paths included,
`missing_docs`. P03 has no code behind a `cfg`. SQLite's locking and WAL and the file timestamps
still behave differently per OS, so the build PR gets the labels `windows` and `macos` and links
their runs. `cargo xtask gate` runs on Windows before the phase is claimed done (CLAUDE.md).

## 6. Interfaces other phases use

| Phase | Uses |
|---|---|
| P04 | `Finding`, `Severity`, rule ids; the list of record checks and the relink step to add lore rules as modules, each part of the rule set fingerprint; `values`, `links` and `beats` as inputs, and its own tables in the same store (the ledger, CE-15.3); `BodyGrammar` to decide which malformed items count; `Update.relinked` and backlinks as the neighbours of CE-14.3; configured severities applied when findings are read |
| P05 | `Project::open`; `SCHEMA_VERSION` for `--version` (CE-08R.5); `findings()` and `Display` for `theos check`; `preview` with the staged texts for the hook (CE-17.2); `KindSpec` for `theos schema` and, with `Placement` and `order()`, for `theos new`; `search`, records by id and kind, links and backlinks, `lens` with its beliefs for the MCP read tools (CE-18.1, CE-18.2); truth `target` from `values` to mark `never` truths (CE-18.3) |
| P06 | `preview` before every write (CE-20.1) and `update(paths)` after it; `KindSpec::order()` for the patcher; backlinks with spans in front matter and body for renaming an id (CE-20.3); `Placement` to move a parent's child folder with it |
| P07 | Each agent worktree is a project with its own `.theos/cache`; `preview` for a proposal's findings (CE-26.3) |
| P11, P12 | `search` (CE-33.3); backlinks (CE-33.2); slot targets for `[[` suggestions (CE-35.1); `link/dangling` findings, through `preview` while typing (CE-35.2) |
| P15 | `lens` and its beliefs (CE-40.1, CE-40.2); links for the graph (CE-42.1) |
| P17 | `preview` for a draft (CE-46.3); the effective lens's `unaware_of` (CE-46.1); embeddings as a further table in the same store (CE-48.1) |
| P19 | Comment lists may be cached in the same store (decision 0002 §3) |

## 7. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| SQLite is compiled from C on each OS | A build fails where no C compiler is set up; universal macOS builds need it per architecture | `bundled` uses `cc`: MSVC on Windows (already needed by Rust), gcc on CI Linux, clang on macOS; CI builds it on every PR; the weekly runs build it on Windows and macOS |
| A stale cache answers wrongly | Findings or links that the files no longer have | Size and time first, then the content hash; the racy rule (§5.9); identity in `meta`, with the rule set and source fingerprints; a damaged cache is rebuilt; CE-12.4's test compares a rebuild with an incremental history |
| Two builds with different fingerprints share one project (a released `theos-mcp` and a dev build of the editor) | Each build rebuilds the cache the other wrote | Every write first checks the cache's identity in `meta` and rebuilds the cache for its own build when another build has written since; after three rebuilds in one write it fails with `IndexError::Contended`, which asks to close one of the theos processes; a rebuild is under a second for 500 records; noted for P05 and P11 |
| Several processes write at once | Busy errors | WAL, `BEGIN IMMEDIATE`, a busy timeout of 5 s; a test opens two indexes on one project from two threads |
| Windows: a cache file held open by another process cannot be deleted | Rebuilding a damaged cache fails | The error names the file and asks to close the other theos processes; the README already demands a local disk |
| The timing test of CE-12.1 is flaky on a busy runner | A red build without a regression | The fastest of three cold runs; the probe needs about a tenth of the limit in a debug build |
| FTS5's tokenizer splits ids and hyphenated words (`T-012` → `t`, `012`) | A loose match ranks too high | Quoted words become phrases, so `"T-012"` matches only that sequence; ranking tests use enough documents for `bm25` to mean something |
| Drift from P01's Python checker until `theos check` replaces it (P05) | A template judged differently | The same tables, id expressions, required lists and kind checks; the CE-11.1 test transcribes the data model; P01's templates, filled in P02's corpus, are checked with every other corpus record and their findings are listed |
| Directive 3: schema or link rules keep leaking in reviews | Review rounds without end | The checks sit on P02's model and on one table per kind; if reviews keep finding new value forms, the class gets named and the field types revisited, not patched case by case |

## 8. Review Focus — inputs most likely to break it

- **Paths:** Windows separators; a non-UTF-8 file name; names equal apart from case;
  `README.md` in a subfolder; files under `templates/records/`; `.MD`; a symbolic link; a record
  in the root; hidden folders; a `.gitignore` inside the project; a file deleted during the walk.
- **Change detection:** an edit that keeps the size and the time; a time in the future; a file
  replaced by an older copy; a file changed between stat and read; a racy file; a file that
  is renamed and edited in one go.
- **Cache:** deleted while another process has it open; damaged; from another format, crate
  version, schema version or registry; two processes opening at once; a read-only folder; WAL
  files left behind.
- **Identity:** `id` differs from the file name; no `id`; an id in two folders; a link to a
  duplicated id; a link to a record whose front matter fails.
- **Values (D1):** `number: "4"`, `number: 007`, `number: 0`, `seed: -1`, `floors: [20, 1]`,
  `floors: [1]`, `generated: "true"`, `accuracy: false`, `status: Draft`, `tags: []`,
  `held_by: []`, `where: a` (a scalar for a list), `parent: [a]` (a list for a scalar), lens
  `tier: T4`, an unknown lens entry, `outputs` with upper-case hex, 63 hex digits, an `@` in a
  binary path, `generator: comfyui@`, `source: todo`.
- **Links:** a beat reference to a missing beat and to a non-script; beat attributes in a topic;
  a note callout in a topic; topic narrowing both ways; a self link; P01's placeholders `todo`,
  `T-000`, `f00-todo`; a mention of a kind no slot allows.
- **Search:** an empty query; only spaces; an unbalanced quote; `AND`, `OR`, `NEAR`, `*`, `-`,
  `:`, `^`, `"` in user text; ids; non-ASCII and emoji; a very long query; a record that contains
  U+FDD0.
- **Lens:** a chain with a wrong-kind or dangling parent; an npc under people; a list replaced; a
  value set again unchanged; a holder that is not a holder kind.
- **Update:** a rename (removal and addition); a kind change; a beat change; a `track` change; a
  change that only touches the title; a change that creates or removes a duplicate.
- **Preview and apply:** a preview while another process syncs; a preview of a removal and of a
  rename; a preview whose text fails to split; `apply` of a path that is not a record (`README.md`,
  under `templates/records/`).
- **Lens values:** `tier: T4`, `unaware_of: T-001` (a scalar), `lens: x`, a valid child entry
  over an invalid parent entry.

## 9. Rulings (Claude's calls)

- Ruling: a project whose `schema_version` is not 1 does not open, and its cache is not touched —
  a build cannot read a data model it does not know, and a half-read project would give wrong
  findings — cost if wrong: a project of a newer schema can only be opened by a newer build.
- Ruling: `kind`, `name` and unknown keys of `theos.toml` are read or ignored, not judged — they
  belong to Start and Settings (P14) — cost if wrong: a malformed `theos.toml` key shows up later.
- Ruling: every built-in part registers its kinds whatever `[parts] enabled` says — records stay
  data whether a screen shows them — cost if wrong: a project that disables a part still gets its
  records checked.
- Ruling: rule ids are `<group>/<name>` (`record/`, `yaml/`, `schema/`, `link/`, `index/`) —
  P02's rule names and P04's lore rules cannot collide — cost if wrong: a rename before P05
  prints them.
- Ruling: a link value is not checked against an id pattern; the index decides whether it
  resolves and to what kind — one fault, one finding; P01's placeholders are pattern-valid and
  dangle by design — cost if wrong: a link with a malformed id is reported as dangling, not as
  malformed.
- Ruling: a missing required field is reported at the `kind` value — the kind decides what is
  required, and an editor can underline it — cost if wrong: a different anchor for one message.
- Ruling: a record's id is its file name; an `id` field that differs is `index/file-name` —
  data model §2 says the file name is the id, and Obsidian resolves links by file name — cost if
  wrong: links to a record with a mismatched `id` resolve by file name until the mismatch is
  fixed.
- Ruling: names equal apart from ASCII case are duplicate ids — Windows and macOS cannot hold
  both, so a Linux clone with both breaks every other clone — cost if wrong: a pair of ids that
  differ only in case is refused, which no id pattern needs.
- Ruling: a beat reference to a missing beat, or to a record that is not a script, is dangling —
  data model §9.4 defines `S-004#3` as a beat — cost if wrong: a different rule name for the same
  fault.
- Ruling: a duplicated id resolves, and its kind check passes when any record with that id fits —
  the duplicate is already an error; more findings on its backlinks would be cascades — cost if
  wrong: a wrong-kind link to a duplicated id goes unreported until the duplicate is fixed.
- Ruling: misplaced records are a warning (`index/placement`) — data model §2 places files, but a
  misplaced file loses no data, and ids are unique project-wide — cost if wrong: Jann may want it
  to block commits.
- Ruling: `.gitignore` files inside the project are honoured, but no global or local git
  excludes — what git does not track is not content, and every machine must see the same
  records — cost if wrong: a record hidden by a global exclude shows up in the index.
- Ruling: only files ending in `.md`, in lower case, are records — Obsidian writes `.md` — cost
  if wrong: a `.MD` file is ignored silently.
- Ruling: the `url` crate is not used; `source` is checked with P01's rule (`http://` or
  `https://`, no whitespace) — the editor never fetches it (G12), and the crate brings 46 crates
  for IDNA — cost if wrong: a malformed URL that starts right passes.
- Ruling: id patterns use the `regex` crate — the patterns are data in the kind specs, and
  `regex` costs one crate — cost if wrong: none.
- Ruling: P03 judges one record's shape and where its links point; value comparisons across
  records (floor number in its band's range, `picked`, `from`, `addresses`, parent cycles) are
  P04's — P04's validator core validates neighbours (CE-14.3) — cost if wrong: these faults wait
  for P04.
- Ruling: `values` holds one row per scalar, list items and map entries included, each with its
  span — search matches and P06's renames need the position of every item — cost if wrong: none.
- Ruling: the cache is rebuilt in full when the format, crate version, `schema_version`, registry
  fingerprint, rule set fingerprint or source fingerprint differs; there are no migrations — the
  cache is derived; a rebuild is under a second for 500 records — cost if wrong: one slow open
  after an upgrade.
- Ruling: a build script of `theos-content` hashes its `src/**` into the build, and the hash is
  part of the cache identity — every build has version 0.0.0 between releases, so a rule whose
  code changed under the same id would otherwise serve stale findings; the gate allows a crate's
  own `build.rs` (its `lints` check, rule 1) — cost if wrong: an unmeasured build script of a
  few lines, and one cache rebuild after every change to `theos-content`'s source in a
  developer's editor.
- Ruling: the store keeps default severities, and configured severities (P04) apply when
  findings are read — a settings change then needs no rebuild — cost if wrong: none.
- Ruling: the sync works on changes (text from disk, given text, removal), and `preview` runs it
  in a transaction it rolls back — P06 must check a command before it writes (CE-20.1), the hook
  checks staged texts (CE-17.2), and P07, P12 and P17 check text that is not on disk; adding this
  after P04 has hung its rules on the sync would mean reworking it — cost if wrong: one enum and
  one rollback that P06 might not use.
- Ruling: a file that cannot be decoded or split has only its one finding, without body, links or
  search row — P02 gives no body for it, and guessing one would give findings for a file whose
  structure is unknown — cost if wrong: a file with a missing `---` is not found by search until
  fixed.
- Ruling: judging P02's well-formed body data (tier tokens, `when::`, link counts, beat
  numbering, attribute keys, note ids and states) is P04's — P02 §5.12 gives it to P04, and P04
  owns the beat checks of data model §9.4 — cost if wrong: these faults wait for P04.
- Ruling: an equation does not require `status`, and `state` is allowed on concept-version only —
  the mirror generates equations and locks them whatever their status; data model §8 names
  concept versions in `state`'s meaning; P01's checker does the same — cost if wrong: Jann may
  want `status` on equations, which is one flag in one table.
- Ruling: an invalid lens entry does not override; the farther layer's value applies and the
  ignored entry is listed — for `unaware_of` this keeps the parent's withheld truths instead of
  none (G9, CE-46.1) — cost if wrong: P15 shows the parent's value under an entry the writer meant
  to set.
- Ruling: a search match in a quoted value with escapes points at the whole value — the decoded
  text does not map byte for byte to the source there — cost if wrong: a coarser highlight for
  rare values.
- Ruling: the `crlf/` and `rejected/` files of P02's corpus are checked one file at a time in
  CE-11.1 — the twins would duplicate ids in one project, and `rejected/` files are not records
  but still files the index meets — cost if wrong: none.
- Ruling: a file whose modification time is less than 2 seconds before its hash was taken is
  hashed again at the next sync — git's racy rule; FAT has a 2-second tick — cost if wrong: a
  few extra file reads right after edits.
- Ruling: a sync is one `BEGIN IMMEDIATE` transaction with a 5-second busy timeout — writers
  serialise, readers never block, a crash leaves the last commit — cost if wrong: a process waits
  up to 5 s behind another's sync before it fails with a clear error.
- Ruling: `Index::in_memory` exists beside `Index::open` — tests and read-only folders need no
  file — cost if wrong: one constructor more.
- Ruling: P05's hook checks the staged snapshot with `preview` on the shared cache (the staged
  text of each file whose working copy differs, and the removal of unstaged files), not with a
  fresh index of the whole snapshot — a fresh index would re-parse every record on every commit,
  which Jann ruled out — cost if wrong: P05 builds the change list from `git diff --cached` and
  `git status`, a few lines more than walking a checkout.
- Ruling: search quotes every user word as an FTS5 string, every word must match, and double
  quotes make a phrase; no other FTS5 syntax is exposed — the probe shows that raw user text such
  as `T-012` is a syntax error — cost if wrong: no `OR` or prefix search until a later phase adds
  it.
- Ruling: the search columns are the title, the front matter's texts, ids and links, and the
  body; their weights are 5, 2 and 1 — a title match should lead, and ids and links make
  "where is X named" searchable — cost if wrong: a ranking that Jann tunes later by three numbers.
- Ruling: match markers are U+FDD0 and U+FDD1, and a hit whose text does not map back loses its
  spans, never its rank — Unicode reserves noncharacters for internal use — cost if wrong: a
  record that holds a noncharacter shows its hit without highlights.
- Ruling: the effective lens also lists the holder's beliefs — data model §7 defines them with
  the lenses, and P05 and P15 need them — cost if wrong: a few lines that P15 would otherwise
  write.
- Ruling: a field a child sets replaces the inherited value as a whole, lists included, and the
  same value set again is still an override — data model §7 — cost if wrong: P15 shows an
  override where Jann expects a merge.
- Ruling: P02's corpus becomes one temporary project in the tests, and its findings are listed
  in `expected-index.toml`; no records are added to resolve its dangling links — P02 §5.13 built
  it as one project, and records for the example ids would invent lore — cost if wrong: a long
  expected list.
- Ruling: the build PR gets the labels `windows` and `macos` although no code is behind a `cfg` —
  SQLite locking, WAL and file timestamps differ per OS — cost if wrong: two runs of the
  `platforms` workflow.

## 10. Left to the builder

- Module, type and table names, columns and SQL, as long as §5 and §6 hold.
- The exact messages, within §5.2, §5.5 and §5.6.
- How many matches a search hit carries.
- How the 500 records of CE-12.1 are generated, within §5.13.
- Whether the record checks and the link checks share one registration list or have one each.
- Whether `dump()` returns a struct or text.
