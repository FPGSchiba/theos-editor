# P03 · Kinds, index, search, lenses — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (`CLAUDE.md` makes it the execution method) to implement this plan task by task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** `theos-content` knows what a record of each kind must hold. It keeps a persistent,
incremental index of a content project in `.theos/cache/index.sqlite`, and answers findings,
backlinks, full-text search and effective lenses from it.

**Architecture:** four layers over P02's record model:
- `kinds` and `parts` hold one declarative `KindSpec` per kind. The four editor parts register
  them.
- `check` judges one record against its spec, with one module per rule and one list.
- `walk` finds the record files with `ignore`.
- `index` keeps the derived rows in SQLite (`rusqlite`, bundled, FTS5):
  - `sync`, `update`, `apply` and `preview` turn changes into rows;
  - `resolve` checks links, ids and placement over the rows;
  - `search` and `lens` answer from the rows.

The cache is never a source of truth.

**Tech stack:**
- Rust 1.99.1 (edition 2024).
- New: rusqlite 0.41 (`bundled`: SQLite 3.54, FTS5), ignore 0.4, regex 1.13.
- From the workspace: sha2 0.11 (also as a build dependency), toml 1, serde 1, thiserror 2,
  tracing 0.1, and P02's yaml-rust2 and pulldown-cmark.
- Dev: tempfile 3.

**Spec:** `docs/specs/phases/P03-kinds-index-search-lenses.md` (approved and merged, #18). Read it
with this plan. Where this plan is silent, the spec decides; where they differ, the spec wins.

**Probes run while planning** (5 Nov 2026, toolchain 1.99.1, scratch crates outside the repo), in
addition to spec §4.2:
- **FTS5**, with the spec's table (`path UNINDEXED, title, fields, body`, tokenizer `porter
  unicode61 remove_diacritics 2`, `bm25(search, 0, 5, 2, 1)`):
  - A quoted term without tokens (`"-"`, `"*"`, `"😀"`, `""`) is no error. Alone it has no
    hits; next to a real term FTS5 drops it, so `"hum" "-"` finds what `"hum"` finds.
  - `"AND"` and `"NEAR"` are plain words.
  - `"choir hall"` is a phrase: `"hall choir"` finds nothing.
  - `highlight(search, 3, ?2, ?3)` bound to U+FDD0 and U+FDD1 marks the matches exactly.
  - A stored U+FDD0 comes back as a stray marker, so the text without markers differs from the
    stored text. That is spec §5.11's fallback.
- **`pulldown-cmark` 0.13.6**, tight list items under `> [!voice] [[a]]`:
  - `> - Design note: x` gives `Start(Item)` 19..36 = `- Design note: x\n`, with no
    `Paragraph`. The item's first text event starts at 21, after the list marker.
  - A lazy line `>   Design note: x` stays inside the item (19..45).
  - `>\tDesign note: x` below the title line belongs to the title paragraph (2..34).
  - The loose twin (`> - Design note: x` / `>` / `> - two`) gives the item a `Paragraph`
    21..36.

## Global Constraints

Umbrella global constraints, verbatim. CLAUDE.md asks for G1–G16; G17 and G18 were added to the
umbrella later and are copied too (as in the P02 plan).

- **G1** Repositories: `theos-editor` (this repo) holds the editor. `tower-of-theos` holds the
  content project and the map toolset. `Theos-Engine` is never a dependency, except behind the
  cargo feature `engine`, which no phase here builds.
- **G2** Platforms: Windows 11 x64 (primary), Linux x64, and macOS 13 or later on Apple silicon
  and Intel (universal builds). Every PR is checked on Linux; Windows and macOS are tested
  weekly, on demand (a label on the PR) and before every release (`docs/gate.md` §1).
- **G3** Rust: stable toolchain pinned in `rust-toolchain.toml`, edition 2024, `unsafe_code`
  forbidden, `Cargo.lock` committed, all builds `--locked`.
- **G4** Crates: `theos-content`, `theos-vcs`, `theos-studio` (libs); `theos`, `theos-mcp`,
  `theos-editor` (bins); `xtask`. Paths `crates/<name>`, `xtask/`. UI in `editor-ui/`
  (Svelte 5, TypeScript, pnpm).
- **G5** Content files follow `docs/specs/data-model.md`; the restricted YAML parser is ours;
  any edit leaves untouched bytes identical.
- **G6** Git only through the `git` command line; credentials only through the user's
  credential helper.
- **G7** MCP: stdio only, `rmcp`, tool names `^[a-z][a-z0-9_]*$`, agents only propose.
- **G8** `never-confirm`, `no-overshoot`, `inworld-vocab` are always blocking; nothing lowers them.
- **G9** Text of a truth with target `never` never appears in an LLM prompt.
- **G10** Binaries never enter git; references are `path@sha256:<hex>` relative to the binaries
  folder.
- **G11** Generators only through `docs/generators.md`; no generator-specific field in the data
  model; the UI works for any generator that produces a kind.
- **G12** Network only to ComfyUI and the configured LLM endpoint, both opt-in; no telemetry.
- **G13** UI uses `design/theos-ui/` and follows `docs/ui/`; shortcuts use Ctrl on Windows and
  Linux and Cmd on macOS, from one keymap.
- **G14** Settings in three layers (part default → user → project); secrets in the OS
  credential store.
- **G15** Every AC below has at least one test tagged with its id (`docs/gate.md` §4).
- **G16** Errors in library crates are typed (`thiserror`); `anyhow` only in binaries. Logging
  with `tracing`.
- **G17** Branches, commits, PR titles, versions and tags follow `CONTRIBUTING.md`; releases
  follow `docs/release.md`. One product version for all crates; tags only from the pipeline.
- **G18** UI references are the HTML files in `docs/ui/screens/` (built only from
  `design/theos-ui/`); the PNG files next to them are previews rendered from that HTML.

Phase ACs, verbatim:

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

**Jann's rulings that bind the build** (spec §4.1):
- the index persists in SQLite with FTS5 and opens incrementally: no full re-parse when nothing
  changed;
- schemas are declarative tables, one module per kind, with one generic checker;
- the registry and the parts live in `theos-content`, one module per part, each with a
  `register` function.

P02's ruling D1 still holds: a scalar is typed by its field, never by its spelling.

## Build rules (every task, every subagent)

Put this section into every implementer and reviewer brief.

- **Engineering directives** (`CLAUDE.md`):
  - Small modules with one job. A new kind is one module plus one line in its part; a new rule
    is one module plus one line in its list.
  - Use the crate before custom code: SQLite does storage, transactions and search, `ignore`
    walks, `toml` reads `theos.toml`, `regex` matches id patterns, `sha2` hashes.
  - Never hand-write a parser for TOML, YAML, Markdown, SQL or a search query language. Search
    only quotes user words (spec §5.11).
  - If reviews keep finding new variants of one problem, stop and name the class (directive 3).
- **SQL** lives in `src/index/schema.sql` (tables) and in the module that runs each query. No
  ORM. Every write of a sync, update, apply or preview is one `BEGIN IMMEDIATE` transaction
  (`Connection::transaction_with_behavior(TransactionBehavior::Immediate)`), with a busy
  timeout of 5 s.
- **Lints** (workspace, `-D warnings`):
  - No `#[allow]` or `#[warn]`. `#[expect]` only with `reason = "…"`.
  - Every public item is documented (`missing_docs`), and every public fn returning `Result`
    has an `# Errors` section.
  - No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, `print!` or `exit`
    outside tests. `build.rs` writes to stdout with `writeln!`.
  - Clippy pedantic is on.
  - Coverage counts `src/**` with its error paths: test them.
- **No dead code in tests either.** A helper, or a deserialised field nobody reads, fails clippy.
  Add each in the task whose test first uses it.
- **Dead code between tasks (L29).** Tasks 8 and 9 add crate-private modules that only Task 10
  wires in. Until then, their `mod` lines carry `#[expect(dead_code, reason = "wired into the
  index by Task 10")]`. Task 10 removes them; an unfulfilled expectation would warn.
  `FieldSpec::check`, `check::check_record` and `walk::walk` are public instead, since later
  phases use them.
- **Docs:** code blocks in doc comments are ```` ```text ```` unless they are doctests that
  pass. Intra-doc links must resolve.
- **Tests:**
  - Units go in `#[cfg(test)] mod tests` next to the code.
  - Behaviour goes in the integration-test crate `crates/theos-content/tests/index/` (`main.rs`
    plus one module per file; Task 5 creates it). P02's `tests/corpus/` stays as it is, except
    Task 1's one test.
  - A test proving an AC is a plain `#[test]` fn with `// AC: CE-xx.y` on the line directly
    above `#[test]` (ruling L1).
  - Tests may return `Result<(), Box<dyn std::error::Error>>`.
  - Fixture projects are indexed in place with `Index::in_memory` plus `sync()`, so the repo
    never gets a `.theos/cache`. Tests that need the file cache copy the project into a
    `tempfile` folder first.
- **Fixtures:**
  - Every new record says it is synthetic: `# synthetic fixture (P03), not canon` as the first
    front-matter line.
  - Texts are neutral ("synthetic stance A"), never lore.
  - Ids are `synthetic-…` slugs, and truth, belief, clue, script and equation numbers from 930
    up (L24). P02's corpus files do not change.
- **Files with backslashes** (Rust string escapes, fixtures) are written with the Write or Edit
  tool, never with a shell heredoc.
- **Git:**
  - One git command per Bash call in the worktree (no `&&` chains). Never `--no-verify`.
  - Commit message: a Conventional Commit with scope `content` (`build` for a dependency-only
    commit), a body saying why, then the footers below as one block.
  - No footer line over 100 characters: split long `Refs:` into several `Refs:` lines. Leave
    `Refs:` out when a task proves no AC.

  ```text
  Refs: CE-11.1, CE-11.5
  Phase: P03
  Claude-Session: 4040ef39-41d0-4af5-93ce-fb3678512a5b
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  ```
- **Per-task checks** (run from the worktree root, read the output):
  - `cargo fmt --all --check`;
  - `cargo clippy -p theos-content --all-targets --locked -- -D warnings`;
  - `cargo nextest run -p theos-content --locked`;
  - a task that adds a dependency also runs `cargo deny --locked check`.

## Review Focus

These are the five input classes the spec implies, that no AC test covers and that are most
likely to bite. Each has its test in the owning task.

1. **Change detection that the size and time alone miss.** Expected:
   - an edit that keeps the size and the time of a racy file is seen at the next sync;
   - a file whose time lies in the future stays racy;
   - a file replaced by an older copy is read again.

   Tests: Task 10 (`racy_files_are_read_again`, `a_time_in_the_future_stays_racy`,
   `an_older_copy_is_read`).
2. **Paths that are not records, or not plain.** Expected: none of these is indexed:
   - `README.md` in a subfolder;
   - files under `templates/records/`, `tools/` or `attachments/`;
   - `.MD`;
   - hidden folders;
   - a file ignored by a `.gitignore` inside the project;
   - a symbolic link.

   A non-UTF-8 file name gives one `record/path` finding.

   Tests: Task 7 (`records_and_not_records`, `symbolic_links_are_not_records`,
   `a_non_utf8_name_is_kept_lossy`), Task 10 (`a_non_utf8_file_name_is_a_finding`).
3. **A change that creates or removes a duplicate id, and a rename with an edit in one sync.**
   Expected: the duplicate finding appears or goes on both paths; the backlinks of both ids are
   relinked; the dump equals a fresh build. Tests: Task 12 (`duplicates_come_and_go_with_changes`,
   `a_rename_with_an_edit`).
4. **Lens entries that fail their type.** Inputs: `tier: T4`; `unaware_of: T-930` as a scalar;
   `lens: x`; a valid child entry over an invalid parent entry. Expected: the farther layer's
   value applies, and the ignored entry is listed with its layer and finding (spec §5.12). Test:
   Task 14 (`invalid_entries_do_not_override`).
5. **Preview and apply at the edges.** Inputs: a preview of a removal; of a rename; of a text
   that fails to split; `apply` of `README.md` or of a `templates/records/` path. Expected:
   - the removal's backlinks dangle;
   - the rename moves the findings;
   - the text that fails to split gives exactly its `record/…` finding;
   - a path that is not a record is ignored;
   - `dump()` is unchanged after every preview.

   Tests: Task 15 (`preview_of_a_removal_and_a_rename`, `preview_of_text_that_fails_to_split`),
   Task 10 (`not_records_are_ignored_by_update_and_apply`).

## Rulings made while planning

These go into the ledger's "Rulings (build)" and the PR, in this form.

- L1. Ruling: AC tests are plain `#[test]` fns in one new integration-test crate
  (`tests/index/`) plus unit tests, as P02's ruling P1 — `ac-trace` needs the fn name; one
  crate shares its helpers without dead code — cost if wrong: none.
- L2. Ruling: R5-1's `strip_quotes` strips one space *or tab* after each `>` (and spaces or tabs
  before it). Jann's "strip spaces and tabs" is read as "a tab where a space was stripped" —
  stripping all of them would flatten the indentation that P02's
  `note_text_keeps_lists_and_code_blocks` keeps — cost if wrong: note text keeps extra
  indentation after a `>` followed by more than one blank.
- L3. Ruling: a tight item's own text runs from its first inline event to the start of the
  first block nested in it, else to the item's end. Probe: the item's range starts at the list
  marker, and a nested item would otherwise be scanned twice. Task 1 adds one test for a nested
  item (found once), besides Jann's list — cost if wrong: none.
- L4. Ruling: Jann's R5-1 guards (loose twins unchanged, corpus count unchanged) pass before the
  fix as well; they pin what must not move. Every other R5-1 test is seen failing first — cost
  if wrong: none.
- L5. Ruling: new dependencies are written like the rest of `[workspace.dependencies]` (caret:
  `"0.41"`, `"0.4"`, `"1.13"`); `Cargo.lock` pins the probed versions — cost if wrong: none.
- L6. Ruling: `FieldType` has two more variants than the spec's table, `Id` and `KindName`, for
  `id` and `kind`. They keep the data model's field order in the spec; `schema/id` and
  `schema/kind` check them, `schema/value` never does — cost if wrong: none.
- L7. Ruling: `Url`, `Generator`, `NoteIds` and `BinaryRefs` are checked with string functions
  (a prefix, one split, a character test), not regexes; `regex` stays for the id patterns,
  which are data in the specs — cost if wrong: none; P01's expressions are the reference.
- L8. Ruling: `number: 007` reads as 7, as P02's `as_int` and P01's checker read it — cost if
  wrong: one more check in `Integer`.
- L9. Ruling: `FloorRange` gives one finding for the whole value — the fault is the pair, not an
  item — cost if wrong: none.
- L10. Ruling: a file whose name is not UTF-8 gets a `files` row and its `record/path` finding
  under its lossy path, and is never read — cost if wrong: two such names that become the same
  lossy text share one row.
- L11. Ruling: a file that exists but cannot be read gives `record/read` (error), a rule the
  spec's table does not list. Its `files` row is racy, so the next sync tries again — one
  unreadable file must not stop the index — cost if wrong: one rule id P05 prints.
- L12. Ruling: a walk error (an unreadable folder) fails the sync with `IndexError::Walk` and
  rolls back — a partial walk would report every unseen record as removed — cost if wrong: one
  bad folder blocks syncing until fixed.
- L13. Ruling: `Index` owns a clone of the registry, and `Index::open` returns `(Index, Update)` —
  no lifetime ties the index to the registry (P11 holds both in one state), and CE-12.2 needs
  the open's `Update` — cost if wrong: a clone of 20 small specs per index.
- L14. Ruling: `Change::Content` carries bytes, not a `String`, so a staged non-UTF-8 file gets
  `record/utf8` in a preview (CE-17.2) — cost if wrong: none.
- L15. Ruling: in `Update`:
  - `read` lists the files read from disk;
  - `relinked` lists only the records relinked because another record changed. A re-parsed
    record is always relinked and appears only in `reparsed`.

  This is CE-12.2's "that record and its backlinks" — cost if wrong: one list more or less in
  P06's log.
- L16. Ruling: the `values` table (named `field_values`; `values` is an SQL keyword) also has
  one row per list and per map (`shape`), and a `valid` flag. A node is valid when no
  `schema/value` problem overlaps its span. Reasons:
  - an empty list (`unaware_of: []`) still sets a lens field;
  - the lens must know which entries failed;
  - links are taken only from valid items.

  Cost if wrong: none.
- L17. Ruling: links are stored only from values that pass their type (`parent: [a]` stores no
  link) — one fault, one finding — cost if wrong: a wrong-shape link is not listed in
  backlinks until fixed.
- L18. Ruling: a narrowed target whose narrowing value is missing or invalid (a topic without a
  valid `track`) counts like a target without a known kind: no `link/kind` — the target's own
  finding says what is wrong — cost if wrong: a cascade-free gap until the topic is fixed.
- L19. Ruling: `lens()` of a duplicated holder id, and each parent step, uses the first fitting
  record by path — the duplicate is already an error — cost if wrong: a lens from the other
  file until the duplicate is fixed.
- L20. Ruling: search rules:
  - a hit carries at most 10 matches, in document order;
  - an unbalanced `"` makes a phrase that runs to the end of the query;
  - terms without tokens are left to FTS5, which drops them;
  - a match's line text is the record's line that holds it, cut from the front matter
    (`records.front`) or from the body.

  Cost if wrong: one constant and one rule.
- L21. Ruling: `dump()` returns text, one row per line, tab-separated, in the sections records,
  field_values, links, beats, findings, search, files (path and hash only) — test failures show
  a readable diff — cost if wrong: none.
- L22. Ruling: `build.rs` hashes every file under `src/` (relative path with `/`, NUL, bytes,
  NUL), sorted by path, and prints `cargo::rerun-if-changed=src` and
  `cargo::rustc-env=THEOS_CONTENT_SOURCE_HASH=<hex>` with `writeln!` — the workspace denies
  `print_stdout` — cost if wrong: none.
- L23. Ruling: the CRLF files are compared in pairs. Each `[[twin]]` of `crlf/twins.toml` is
  indexed alone in its own temporary project, input and LF file at the same path:
  - the twins of `records/` files sit at their `records/` path;
  - `synthetic-crlf`, `mixed` and `lone-cr` (all `thing`) sit at `things/<input>`.

  Both give the same findings, and the latter three are also listed under `[[crlf]]` in
  `expected-index.toml` — cost if wrong: none.
- L24. Ruling: P03's fixture ids are `synthetic-…` slugs and numbers from 930 up (`T-930`,
  `S-940`, `E930`) — distinct from P02's corpus, easy to grep — cost if wrong: none.
- L25. Ruling: the rule set fingerprint lists P02's YAML rules as one entry `yaml/*` — their
  names change only with P02's code, which the source fingerprint covers — cost if wrong: none.
- L26. Ruling: until Task 12 lands, step 4 relinks every record and rechecks every id — each
  task stays shippable and correct; Task 12 narrows it to CE-12.2's set — cost if wrong: none
  after Task 12.
- L27. Ruling: an SQLite busy error after the 5 s timeout is `IndexError::Busy` naming the cache
  file — spec §9's "clear error" — cost if wrong: none.
- L28. Ruling: `expected.toml` and `expected-index.toml` are written from the index's output only
  after each finding is checked by hand against the spec. A finding that shows a bug is fixed
  in the code, never listed — the lists are evidence, not snapshots — cost if wrong: none.
- L29. Ruling: a building block that a later task wires in is public when a later phase can use
  it:
  - `FieldSpec::check` and `ValueRef` (P12 checks a field as it is typed);
  - `check::check_record` and `check::rules` (P05 checks one file, and lists the rules);
  - `walk::walk` (P05 lists records).

  Otherwise it carries `#[expect(dead_code, reason = …)]` on its `mod` line until Task 10 wires
  it in: `index::store`, `index::identity` and `index::rows`. Each task then passes clippy on
  its own, and an expectation that is no longer needed warns. Cost if wrong: five more public
  items.

## File structure

```text
crates/theos-content/
  Cargo.toml                       + rusqlite, ignore, regex, sha2, toml, serde; build-dep sha2
  build.rs                         source fingerprint (Task 8)
  README.md                        what the crate provides after P03 (Task 17)
  src/
    lib.rs                         + pub mod project, finding, kinds, parts, check, walk, index;
                                     re-exports
    project.rs                     Project, ProjectError, SCHEMA_VERSION (Task 2)
    finding.rs                     Finding, Severity (Task 3)
    kinds/
      mod.rs                       KindRegistry, RegistryError, re-exports (Task 3)
      spec.rs                      KindSpec, FieldSpec, IdRule, Placement, BodyGrammar,
                                     BeatAttribute, KindCheck, CheckInput, Problem (Task 3)
      types.rs                     FieldType, Target, Narrow, LensEntry, LENS_ENTRIES, TIERS,
                                     VOICE_SPEAKER, VOICE_BELIEF, Display (Task 3)
      value.rs                     ValueRef, FieldSpec::check, FieldType::check (Task 4)
      fingerprint.rs               canonical description, SHA-256 hex (Task 3)
    parts/
      mod.rs                       builtin() (Task 5)
      common.rs                    the §5 fields, shared target lists
      version.rs                   the version fields of data model §8, version-id check
      codex/{mod,tower,band,floor,location,people,faction,npc,thing,topic,truth,belief,
             clue,equation}.rs
      scripts/{mod,script,dialogue,template}.rs
      concepts/{mod,concept,concept_version}.rs
      maps/{mod,map,map_version}.rs
    check/
      mod.rs                       check_record, Checked, Subject, SchemaRule, SCHEMA_RULES,
                                     rules (Task 6)
      decode.rs                    record/utf8, record/<split>, yaml/<rule>
      kind.rs                      schema/kind (selects the spec)
      id.rs, file_name.rs, required.rs, value.rs, unknown_field.rs, unknown_entry.rs
    walk.rs                        walk, Walked, WalkedPath, is_record_path, WalkError (Task 7)
    index/
      mod.rs                       Index, IndexError, Change, Update, Preview (Tasks 8, 10, 15)
      schema.sql                   the tables (Task 8)
      store.rs                     Store: file or memory, pragmas, damaged → rebuilt (Task 8)
      identity.rs                  Identity, meta rows, rule_set, fingerprints (Task 8)
      slots.rs                     Slot (Task 9)
      rows.rs                      RecordRows from a Checked record (Task 9)
      write.rs                     replace and delete the rows of one path (Task 10)
      sync.rs                      steps 1–5 of spec §5.9 (Tasks 10, 11, 12)
      query.rs                     RecordSummary, LinkRef, findings, records, backlinks (10, 11)
      dump.rs                      dump() (Task 10)
      resolve/mod.rs               relink, recheck_ids, LinkRule, LINK_RULES, Targets (Task 11)
      resolve/{targets,dangling,kind,placement,duplicate}.rs
      search.rs                    Hit, Match, Column, query quoting, marker mapping (Task 13)
      lens.rs                      EffectiveLens and its parts (Task 14)
  tests/
    index/main.rs                  the integration-test crate (Task 5)
    index/common.rs                TempProject, expected.toml reader, finding keys (Task 10)
    index/{kinds,sync,links,changes,search,lenses,cache,corpus,schema,speed}.rs
    fixtures/data-model-kinds.toml the data model's field tables, transcribed (Task 5)
    fixtures/projects/{links,search,lenses,schema}/   theos.toml, expected.toml, records
    fixtures/corpus/expected-index.toml               (Task 16)
```

---

### Task 1: R5-1 — a `Design note:` line in a tight list item (P02 carry-over)

Jann's instruction, from the P03 run request. This task lands on the phase branch before P03's
own tasks.

**Files:**
- Modify: `src/body/design_note.rs` (`scan`, `note_line`, tests)
- Modify: `src/body/callout/note.rs` (`strip_quotes`), `src/body/callout/tests.rs`
- Modify: `tests/corpus/grammars.rs` (corpus count)

**Interfaces:** none change. `Body::design_notes()` finds more notes; `Note::text` strips a tab
after `>`.

**The bug.** In `design_note.rs` (`scan`, `note_line`), a `Design note:` line inside a callout
is missed when it sits in a tight list item, because pulldown-cmark gives a tight `Item` no
`Paragraph` event. A tab after `>` is not stripped either.

**The fix.**
- In `scan`, treat a `Start(Tag::Item)` that is in scope and not followed by
  `Start(Tag::Paragraph)` like a paragraph, as `callout/scan.rs` (`last_paragraph`) does for
  captions.
- The item's own text runs from the start of the event after `Start(Item)` to the start of the
  first `Start` of a list, block quote, code block, heading or HTML block nested in the item.
  Without one, it runs to the item's end (L3). Skip the item when the next event is its `End`.
  Pass that span to `note_line` with the item's quote depth.
- In `note_line`, strip spaces and tabs before and after each `>`, and before the marker
  (`trim_start_matches([' ', '\t'])`).
- In `strip_quotes`, strip spaces and tabs before each `>`, and one space or tab after it (L2).

- [ ] **Step 1: Pin the corpus count.** Add `corpus_design_note_count_is_unchanged` with
  `// AC: CE-10.2` to `tests/corpus/grammars.rs`: the sum of `body().design_notes().len()` over
  `project_records()` equals the number the current code gives. Read that number by running
  the test once, then write it as a literal.
- [ ] **Step 2: Write the failing tests**, each with `// AC: CE-10.2`. In `design_note.rs`
  tests, `m` is `r.text().find("Design note:")` and `x` is `"Design note: x\n"`:
  - `a_note_line_in_a_tight_list_item_is_found`: body `"> [!voice] [[a]]\n> - Design note:
    x\n"` gives exactly one note, whose form is `Paragraph { marker }`, with
    `marker == Span::new(m, m + 12)` and `span == Span::new(m, m + x.len())`.
  - `a_lazy_note_line_of_a_tight_item_is_found`: `"> [!voice] [[a]]\n> - item\n>   Design
    note: x\n"` gives the same.
  - `a_note_line_in_a_later_tight_item_is_found`: `"> [!voice] [[a]]\n> Speech.\n>\n> -
    one\n> - Design note: x\n"` gives the same.
  - `a_tab_after_the_quote_marker_is_stripped`: `"> [!voice] [[a]]\n>\tDesign note: x\n"`
    gives the same.
  - `a_note_line_in_a_nested_tight_item_is_found_once` (L3): `"> [!voice] [[a]]\n> - a\n>   -
    Design note: x\n"` gives exactly one note, with the same marker and span.
  - `loose_list_twins_are_unchanged` (guard, L4): `"> [!voice] [[a]]\n> - Design note:
    x\n>\n> - two\n"` and `"> [!voice] [[a]]\n> - one\n>\n> - Design note: x\n"` each give
    exactly one note with that marker and span.
  - In `callout/tests.rs`, `a_tab_after_the_quote_marker_leaves_note_text`: in
    `"> [!note] n1 · open\n>\t- a\n>\t- b\n"`, the note's `text` is `"- a\n- b"`. A block's
    first line carries no quote prefix, so the second line is the one this test needs; before
    the fix the text is `"- a\n\t- b"`.
- [ ] **Step 3: Run** `cargo nextest run -p theos-content --locked`. Expect the five new
  design-note tests and the note-text test to fail. The two guards pass (L4).
- [ ] **Step 4: Implement** the fix above. `scan` iterates with indices so it can look at the
  next event.
- [ ] **Step 5: Run** the per-task checks. Everything passes, the corpus count included.
- [ ] **Step 6: Commit** with the message `fix(content): find a design note line in a tight list
  item`. Body: P02's open finding R5-1 (PR #17), its cause and the fix. Footers: `Refs:
  CE-10.2`, `Phase: P03`, and the session and co-author lines.
  The PR lists it under the rulings as a P02 carry-over.

### Task 2: The project — `theos.toml` and `SCHEMA_VERSION`

**Files:**
- Modify: `Cargo.toml` (`serde.workspace = true` and `toml.workspace = true` move from
  dev-dependencies to dependencies)
- Create: `src/project.rs`
- Modify: `src/lib.rs` (`pub mod project;`, re-export `Project, ProjectError, SCHEMA_VERSION`)

**Interfaces — produces:**

```rust
pub const SCHEMA_VERSION: i64 = 1;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project { /* root, name, kind, enabled parts */ }
impl Project {
    pub fn open(root: &Path) -> Result<Project, ProjectError>;
    pub fn root(&self) -> &Path;
    pub fn name(&self) -> Option<&str>;
    pub fn kind(&self) -> Option<&str>;
    pub fn enabled_parts(&self) -> &[String];
}
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    NotAProject { root: PathBuf },             // "`<root>` is not a content project: it has no theos.toml"
    Io { path: PathBuf, source: std::io::Error },
    Toml { message: String },                  // "theos.toml: " + the toml crate's message (line, column)
    NoSchemaVersion,                           // names theos.toml and "reads schema_version 1"
    NotAnInteger { position: Position, found: String },
    Newer { position: Position, found: i64 },  // spec §5.2's first message
    Older { position: Position, found: i64 },  // spec §5.2's second message
}
```

`open` reads `<root>/theos.toml`, normalises line ends (P02's `normalize`) and deserialises:
`[project]` with `name` and `kind` as `toml::Value` (read, not judged) and `schema_version` as
`toml::Spanned<toml::Value>`, and `[parts] enabled` as an array of strings. Other keys are
ignored (serde's default). The value's line and column come from `TextIndex::position` of the
span start. Messages use `SCHEMA_VERSION`, never a literal `1`.

`Index::open` and `Index::in_memory` (Task 10) take a `Project`, so a project that does not open
never reaches its cache (spec §5.2).

- [ ] **Step 1: Write the failing unit tests** (`project.rs`, temp dirs; the base text is data
  model §13 verbatim, so `schema_version`'s value sits at 4:18):
  - `schema_version_mismatch_is_clear` `// AC: CE-11.3`:
    - with 2, the error displays exactly "theos.toml:4:18: this project uses data model
      schema_version 2, but this build of theos reads schema_version 1. Update theos to open
      it.";
    - with 0, exactly "theos.toml:4:18: this project uses data model schema_version 0, but this
      build of theos reads only schema_version 1.".
  - `schema_version_must_be_an_integer_and_present` `// AC: CE-11.3`:
    - `"1"` gives `NotAnInteger`, and its message contains `theos.toml:4:18`, `"1"` and
      `schema_version 1`;
    - without the line, `NoSchemaVersion`, and its message contains `theos.toml` and
      `schema_version 1`.
  - `version_one_opens` `// AC: CE-11.3`: name `tower-of-theos`, kind `content`, eight enabled
    parts in file order.
  - `not_a_project_without_theos_toml`: the message equals spec §5.2's first bullet.
  - `invalid_toml_reports_where`: `"[project\n"` gives `Toml`, and the message contains
    `line 1`.
  - `crlf_theos_toml_reports_the_same_position`: the CRLF text with version 2 reports `4:18`.
- [ ] **Step 2: Run** the tests. They fail to compile.
- [ ] **Step 3: Implement** the items above.
- [ ] **Step 4: Run** the per-task checks.
- [ ] **Step 5: Commit** with the message `feat(content): read theos.toml and judge its schema
  version` (`Refs: CE-11.3`).

### Task 3: Findings and the kind registry

**Files:**
- Modify: root `Cargo.toml` (`[workspace.dependencies]` adds `regex = "1.13"`),
  `crates/theos-content/Cargo.toml` (`regex.workspace = true`, `sha2.workspace = true`)
- Create: `src/finding.rs`, `src/kinds/{mod.rs, spec.rs, types.rs, fingerprint.rs}`
- Modify: `src/lib.rs` (`pub mod finding; pub mod kinds;`, re-export `Finding, Severity,
  KindRegistry, KindSpec, FieldSpec, FieldType, Target, Placement, BodyGrammar, RegistryError`)

**Interfaces — produces:**

```rust
// finding.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity { Error, Warning }
impl Severity { pub fn as_str(self) -> &'static str; pub fn parse(text: &str) -> Option<Severity> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub rule: String,            // "<group>/<name>"
    pub severity: Severity,      // the default severity (P04 applies configured ones on read)
    pub path: String,            // project-relative, '/'
    pub id: Option<String>,      // the record's id (file stem); None for record/path
    pub span: Option<Span>,      // record coordinates; None only for record/path
    pub position: Option<Position>,
    pub message: String,
}
impl Display for Finding  // "path:line:column: error schema/value: message"; without position "path: error rule: message"

// kinds/types.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Narrow { pub field: &'static str, pub value: &'static str }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Target { pub kind: &'static str, pub narrow: Option<Narrow> }
impl Target { pub const fn kind(kind: &'static str) -> Target;
              pub const fn narrowed(kind: &'static str, field: &'static str, value: &'static str) -> Target }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldType {
    Id, KindName,                                  // L6
    Text, TextList, Enum(&'static [&'static str]), Integer { min: i64, max: i64 }, FloorRange,
    Link(&'static [Target]), Links(&'static [Target]), Url, True, Generator, NoteIds,
    BinaryRefs, OpenMap, Lens,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LensEntry { pub key: &'static str, pub ty: FieldType }
pub const TIERS: &[&str] = &["T1", "T2", "T3"];
pub const LENS_ENTRIES: &[LensEntry];   // thinks_from Text, stance Text, tier Enum(TIERS),
                                        // unaware_of Links(truth), rejects Links(belief), speaks_to Text
pub const VOICE_SPEAKER: &[Target];     // npc, faction, people
pub const VOICE_BELIEF: &[Target];      // belief

// kinds/spec.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FieldSpec { pub key: &'static str, pub ty: FieldType, pub required: bool }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IdRule { pub pattern: &'static str, pub says: &'static str }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Placement { Flat(&'static str), UnderParent(&'static str) }  // folder; parent field
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BeatAttribute { pub key: &'static str, pub targets: &'static [Target] }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyGrammar { pub beats: Option<&'static [BeatAttribute]>, pub notes: Option<&'static [Target]> }
impl BodyGrammar { pub const PROSE: BodyGrammar; }   // no beats, no notes
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem { pub span: Span, pub message: String }
pub struct CheckInput<'a> { pub id: &'a str, pub front: &'a FrontMatter, pub kind_span: Span }
#[derive(Clone, Copy)]
pub struct KindCheck { pub rule: &'static str, pub run: fn(&CheckInput<'_>) -> Vec<Problem> }
impl Debug for KindCheck   // prints the rule only: a fn pointer's address differs per run
#[derive(Clone, Debug)]
pub struct KindSpec { pub name: &'static str, pub id: IdRule, pub placement: Placement,
                      pub fields: Vec<FieldSpec>, pub body: BodyGrammar, pub checks: Vec<KindCheck> }
impl KindSpec {
    pub fn field(&self, key: &str) -> Option<&FieldSpec>;
    pub fn order(&self) -> Order;  // field keys in order; with_entries("lens", LENS_ENTRIES keys) when a field is Lens
}

// kinds/mod.rs
#[derive(Clone, Debug, Default)]
pub struct KindRegistry { /* specs in registration order, part per kind, compiled id regexes */ }
impl KindRegistry {
    pub fn new() -> KindRegistry;
    pub fn register(&mut self, part: &'static str, spec: KindSpec) -> Result<(), RegistryError>;
    pub fn kind(&self, name: &str) -> Option<&KindSpec>;
    pub fn kinds(&self) -> impl Iterator<Item = &KindSpec>;   // registration order
    pub fn part_of(&self, name: &str) -> Option<&'static str>;
    pub fn id_fits(&self, kind: &str, id: &str) -> bool;
    pub fn fingerprint(&self) -> String;                      // 64 lower-case hex digits
}
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    DuplicateKind { kind: String, first: &'static str, second: &'static str },
    IdPattern { kind: String, source: regex::Error },
}
```

`Display` for the types (also the notation of Task 5's transcription):

| Item | Display |
|---|---|
| `Id`, `KindName` | `id`, `kind` |
| `Text`, `TextList` | `text`, `text list` |
| `Enum(values)` | `one of a\|b\|c` |
| `Integer { min, max }` | `integer 1..=100`, or `integer` for `i64::MIN..=i64::MAX` |
| `FloorRange` | `floor range` |
| `Link(targets)`, `Links(targets)` | `link → band`, `links → band\|floor\|location` |
| `Url`, `True`, `Generator` | `url`, `true`, `name@version` |
| `NoteIds`, `BinaryRefs` | `note ids`, `binary refs` |
| `OpenMap`, `Lens` | `map`, `lens` |
| `Target` | `topic` or `topic(track=explaining)` |
| `FieldSpec` | `key: <type>`, and `, required` when required |

**The fingerprint** is the SHA-256 of the lines `"{part}\t{spec:?}\n"` of every kind, sorted by
kind name, as lower-case hex (the byte-fold of `xtask/src/checks/approval/fingerprint.rs`).
`register` compiles `spec.id.pattern` once and keeps the regex next to the spec.

- [ ] **Step 1: Write the failing unit tests** (`kinds/mod.rs`, `types.rs`, `finding.rs`):
  - `an_empty_registry_knows_no_kind` `// AC: CE-11.5`: `kinds().count() == 0`, and
    `kind("npc")` and `part_of("npc")` are `None`.
  - `registering_a_kind_twice_fails` `// AC: CE-11.5`: a test spec registered by `"a"` and then
    by `"b"` gives `DuplicateKind { kind, first: "a", second: "b" }`; `part_of` is still
    `"a"`.
  - `order_follows_the_spec` `// AC: CE-11.5`: a spec with the fields `id, kind, title,
    status, tags, lens` gives `Order::new([those]).with_entries("lens", ["thinks_from",
    "stance", "tier", "unaware_of", "rejects", "speaks_to"])`. A spec without `Lens` gives no
    entries.
  - `fingerprint_is_stable_and_follows_the_specs`:
    - two registries with the same specs, registered in either order, give the same 64-hex
      fingerprint;
    - flipping one `required` changes it;
    - changing a check's rule changes it.
  - `id_patterns_are_compiled_at_registration`: with the slug pattern, `id_fits(k, "a-b")` is
    true and `id_fits(k, "A")` false. A spec with pattern `"("` gives `IdPattern`.
  - `types_display_as_the_notation`: one assertion per row of the table above.
  - `findings_display_with_and_without_position`: `things/a.md:3:7: error schema/value: m`
    and `x.md: error record/path: m`.
- [ ] **Step 2: Run** them. They fail to compile.
- [ ] **Step 3: Implement** the items above.
- [ ] **Step 4: Run** the per-task checks and `cargo deny --locked check`.
- [ ] **Step 5: Commit** with the message `feat(content): add findings and the kind registry`
  (`Refs: CE-11.5`).

### Task 4: Checking a value against its field type

**Files:**
- Create: `src/kinds/value.rs`

**Interfaces — consumes:** P02's `Value`, `EntryValue`, `List`, `Map`, `Scalar` and
`Scalar::{as_text, as_int, as_bool}`. Task 3's `FieldSpec`, `FieldType` and `Problem`.

**Produces:**

```rust
#[derive(Clone, Copy, Debug)]
pub enum ValueRef<'a> { Scalar(&'a Scalar), List(&'a List), Map(&'a Map) }   // public (L29)
impl<'a> ValueRef<'a> {
    pub fn of(value: &'a Value) -> ValueRef<'a>;
    pub fn of_entry(value: &'a EntryValue) -> ValueRef<'a>;
    pub fn span(self) -> Span;   // a map: first entry's key start to last entry's lines end
}
impl FieldSpec  { pub fn check(&self, value: ValueRef<'_>) -> Vec<Problem>; }  // + "must not be empty" for a required empty list
impl FieldType  { pub fn check(self, label: &str, value: ValueRef<'_>) -> Vec<Problem>; }
```

The rules are spec §5.4's table, with these details:
- One problem per bad list item or map entry, at the item or the entry's value. A shape error
  (a list for a scalar, a scalar for a list, anything but a map for a map type) gives one
  problem at the whole value.
- `Enum` and `Text` read `as_text` (D1). `Integer` and `FloorRange` read `as_int`, so quotes
  fail. `True` needs `as_bool() == Ok(true)`.
- `Link` is a scalar with non-empty text, and `Links` a list of them. The target is never
  checked here (spec §5.4).
- `Lens` checks each entry the lens table knows with its type, labelled `lens.<key>`. Unknown
  entries are not checked here (Task 6's `schema/unknown-entry`).
- `Url`, `Generator`, `NoteIds` and `BinaryRefs` use string functions (L7):
  - `Url`: `http://` or `https://`, then at least one character, with no whitespace;
  - `Generator`: one `@`, a non-empty part on each side, and no whitespace;
  - `NoteIds`: `n` and one or more ASCII digits;
  - `BinaryRefs`: `<path>@sha256:<64 of [0-9a-f]>`, with a non-empty path and no `@` in it.
- Messages say "`<label>` must be <what>, not `<found>`". Examples: "`number` must be an
  integer from 1 to 100, not `104`" and "`floors` must be two integers, first ≤ last, from 1
  to 100".

- [ ] **Step 1: Write the failing unit tests** (`value.rs`, each `// AC: CE-11.1`, values
  built with `test_support::record(front, "")`). Each case gives the problems listed, or none;
  a problem's span slices to the named text.
  - `texts_and_text_lists`:
    - `title: ""` gives one problem at `""`;
    - `title: [a]` gives one at the list;
    - `tags: [a, ""]` gives one at `""`;
    - `tags: a` gives one ("must be a list");
    - an optional `tags: []` gives none.
  - `enums_read_the_text`:
    - `status: Draft` gives one problem, and its message lists `draft, canon`;
    - `status: "draft"` gives none;
    - `accuracy: false` gives none (D1);
    - `tier: T4` gives one.
  - `integers_and_floor_ranges`:
    - `number: "4"` gives one problem;
    - `number: 007` gives none (L8);
    - `number: 0` and `number: 104` each give one, with the exact message above;
    - `seed: -1` gives none;
    - `floors: [20, 1]` gives exactly one at the list, with the exact message above (L9);
    - `floors: [1]` and `floors: 5` each give one;
    - `floors: [1, 20]` gives none.
  - `links_are_only_shapes`:
    - `parent: [a]` gives one at the list;
    - `parent: ""` gives one;
    - `parent: a` gives none;
    - `where: a` gives one;
    - `where: [a, ""]` gives one at `""`;
    - a required `held_by: []` gives one ("must not be empty").
  - `formats` (each case gives one problem unless it says none):
    - `source: todo`;
    - `source: "https://a b"`;
    - `source: https://x.y/z` gives none;
    - `generated: "true"`;
    - `generated: false`;
    - `generator: comfyui@`;
    - `generator: a@b@c`;
    - `generator: comfyui@1.2` gives none;
    - `addresses: [n1, x]` gives one, at `x`;
    - `outputs` with upper-case hex;
    - `outputs` with 63 digits;
    - `outputs` with `a@b@sha256:…`;
    - `outputs: x`;
    - `inputs: x`;
    - a valid `outputs`, and an `inputs` map with any entries, give none.
  - `lens_entries_follow_the_lens_table`:
    - `lens:` with `tier: T4` gives one problem at `T4`, labelled `lens.tier`;
    - `unaware_of: T-930` gives one ("must be a list");
    - `lens: x` gives one ("`lens` must be a map");
    - an entry `mood: x` gives none here.
- [ ] **Step 2: Run** them. They fail.
- [ ] **Step 3: Implement** `value.rs`.
- [ ] **Step 4: Run** the per-task checks.
- [ ] **Step 5: Commit** with the message `feat(content): check field values by their type`
  (`Refs: CE-11.1`).

### Task 5: The built-in parts and their twenty kinds

**Files:**
- Create: `src/parts/mod.rs`, `common.rs`, `version.rs`, the 20 kind modules (file structure),
  and `codex/mod.rs`, `scripts/mod.rs`, `concepts/mod.rs`, `maps/mod.rs`
- Modify: `src/lib.rs` (`pub mod parts;`)
- Create: `tests/index/main.rs`, `tests/index/kinds.rs`, `tests/fixtures/data-model-kinds.toml`

**Interfaces — produces:**

```rust
pub fn builtin() -> Result<KindRegistry, RegistryError>;      // parts/mod.rs: one line per part
pub fn register(registry: &mut KindRegistry) -> Result<(), RegistryError>;  // each part's mod.rs: one line per kind
pub(crate) fn spec() -> KindSpec;                             // each kind module
// parts/common.rs
pub(crate) fn fields(status_required: bool, own: &[FieldSpec]) -> Vec<FieldSpec>;  // id, kind, title, status, tags, then own
pub(crate) const SLUG: IdRule;   pub(crate) const PLACES: &[Target];  // band, floor, location
pub(crate) const HOLDERS: &[Target];  // people, faction, npc
// parts/version.rs
pub(crate) fn fields(parent: &'static str, own_kind: &'static [Target], with_state: bool) -> Vec<FieldSpec>;
pub(crate) const VERSION_ID: KindCheck;   // schema/version-id
```

Part names are `"codex"`, `"scripts"`, `"concepts"` and `"maps"`. Each part registers the kinds
data model §6 gives it, in §6's order. These decisions are fixed (spec §5.4):
- **Fields** come in data model order: §5's five first, then the kind's §7 or §8 table.
  - `id` is `Id`, required; `kind` is `KindName`, required; `title` is `Text`, required;
    `status` is `Enum(draft|canon)`, required only for topic, truth, belief, clue, floor and
    script; `tags` is `TextList`.
  - `state` (`Enum(T1|T3)`) belongs to concept-version only. Equation does not require
    `status`.
  - Truth `target` is `Enum(hinted|suspected|confirmed|never)`, in the data model's order.
  - `number` is `Integer { min: 1, max: 100 }`; `seed` is `Integer` over `i64`.
  - Concept `for` targets tower, band, floor, location, people, faction, npc and thing.
  - `from` targets the version's own kind.
  - Truth `topic` and topic `counterpart` target `topic(track=explaining)`; belief `topic`
    targets `topic(track=in-world)`.
- **Id rules** are spec §5.4's table:
  - truth `^T-[0-9]{3,}$`, belief `^B-…`, clue `^C-…`, script `^S-…`;
  - equation `^E[0-9]+$`;
  - floor `^f[0-9]{2}[a-z0-9]*(-[a-z0-9]+)*$`;
  - topic `^[a-z0-9]+(-[a-z0-9]+)*-(explaining|in-world)$`;
  - concept-version and map-version `^[a-z0-9]+(-[a-z0-9]+)*-v[0-9]+$`;
  - every other kind `^[a-z0-9]+(-[a-z0-9]+)*$`.

  The `says` texts are the table's third column.
- **Placement:**
  - `Flat` for:
    - tower and band: `world`;
    - people: `people`;
    - thing: `things`;
    - topic, truth, belief, clue: `lore/topics`, `lore/truths`, `lore/beliefs`, `lore/clues`;
    - equation: `canon/formalism`;
    - script: `story/scripts`;
    - dialogue, template, concept and map: `dialogue`, `templates`, `concepts`, `maps`.
  - `UnderParent`: `parent` for floor, location, faction and npc; `concept` for
    concept-version; `map` for map-version.
- **Body grammar:**
  - script has the beats `at` (location, floor), `with` (npc, faction, people), `says`
    (belief) and `reveals` (clue);
  - concept has the notes `concept-version`, and map has `map-version`;
  - every other kind is `PROSE`.
- **Kind checks:**
  - topic: `schema/topic-track`. A valid `track` that differs from the id's suffix gives a
    problem at the `track` value.
  - topic: `schema/topic-counterpart`. An id ending in `-in-world` without `counterpart` gives
    a problem at the `kind` value.
  - both versions: `schema/version-id`. When the `concept` or `map` value is a scalar, the id
    must equal `<value>-v<digits>`; otherwise a problem at the `id` value, or at the `kind`
    value when there is no `id`.

`data-model-kinds.toml` transcribes data model §5, §6, §7 and §8 by hand, one `[[kind]]` per
kind, with `name`, `part`, and `fields` as `FieldSpec` display strings in order. For example:

```toml
# Transcribed from docs/specs/data-model.md §5–§8 (P03, CE-11.1). Not canon.
[[kind]]
name = "band"
part = "codex"
fields = ["id: id, required", "kind: kind, required", "title: text, required",
          "status: one of draft|canon", "tags: text list", "parent: link → tower, required",
          "floors: floor range, required", "mode: text", "mood: text", "belief_stage: text",
          "tier: one of T1|T2|T3"]
```

- [ ] **Step 1: Write the transcription and the failing tests.**
  - `tests/index/kinds.rs`:
    - `every_kind_of_the_data_model_has_its_schema` `// AC: CE-11.1`: the kinds of
      `builtin()` are exactly the 20 of the file. For each, `part_of` equals `part`, and the
      `FieldSpec` display strings equal `fields` in order.
    - `each_part_registers_its_kinds` `// AC: CE-11.5`: each part's `register` on an empty
      registry gives exactly its kinds, as sorted lists:
      - codex: band, belief, clue, equation, faction, floor, location, npc, people, thing,
        topic, tower, truth;
      - scripts: dialogue, script, template;
      - concepts: concept, concept-version;
      - maps: map, map-version.

      Registering the codex part twice fails with `DuplicateKind`.
    - `placements_and_grammars_follow_the_data_model`: the `Placement` of each kind as listed
      above; `body.beats` only on script; `body.notes` only on concept and map.
    - `orders_follow_the_data_model` `// AC: CE-11.5`: `kind("npc").order()` equals
      `Order::new(["id", "kind", "title", "status", "tags", "parent", "home", "role",
      "lens"]).with_entries("lens", [the six lens keys])`, and `kind("clue").order()` follows
      §7's clue table after the §5 fields.
  - Units in `topic.rs` and `version.rs` (each `// AC: CE-11.1`):
    - `topic_track_matches_the_id_suffix`;
    - `an_in_world_topic_needs_a_counterpart`;
    - `a_version_id_is_its_parent_and_a_number`: `fates-shrine-v5` with `concept:
      fates-shrine` is fine; with `concept: other` it gives one problem at the id.
- [ ] **Step 2: Run** them. They fail.
- [ ] **Step 3: Implement** the parts: one module per kind with `spec()`, one line per kind in
  its part's `register`, and one line per part in `builtin()`.
- [ ] **Step 4: Run** the per-task checks.
- [ ] **Step 5: Commit** with the message `feat(content): register the twenty kinds through four
  parts` (`Refs: CE-11.1, CE-11.5`).

### Task 6: Checking one record

**Files:**
- Create: `src/check/{mod.rs, decode.rs, kind.rs, id.rs, file_name.rs, required.rs, value.rs,
  unknown_field.rs, unknown_entry.rs}`
- Modify: `src/lib.rs` (`pub mod check;`, L29)

**Interfaces — consumes:** `decode`, `Record::parse`, `Record::front_matter`, `YamlError`,
`RecordError::rule()`, and Tasks 3–5.

**Produces:**

```rust
#[derive(Clone, Debug)]
pub struct Checked {
    pub id: String,                 // the file stem
    pub record: Option<Record>,     // None after record/utf8 or a split error
    pub kind: Option<&'static str>, // when schema/kind passed
    pub findings: Vec<Finding>,     // step "record", in the order the rules ran
}
pub fn check_record(registry: &KindRegistry, path: &str, bytes: &[u8]) -> Checked;
pub fn rules() -> Vec<(&'static str, Severity)>;   // record/path, record/read (L11), record/utf8,
    // record/bom, record/opening-line, record/closing-line, yaml/* (L25), schema/kind, then the
    // ids of SCHEMA_RULES; kind checks are in the registry fingerprint instead
pub(crate) struct Subject<'a> { pub id: &'a str, pub front: &'a FrontMatter, pub spec: &'a KindSpec,
                                pub kind_span: Span, pub registry: &'a KindRegistry }
pub(crate) struct SchemaRule { pub id: &'static str, pub severity: Severity,
                               pub run: fn(&Subject<'_>) -> Vec<Problem> }
pub(crate) const SCHEMA_RULES: &[SchemaRule];   // id, file_name, required, value, unknown_field, unknown_entry
```

`record/path` and `record/read` are listed here, but the sync makes them (Task 10): only it sees
file names and read errors.

The order is spec §5.5's table:
1. `decode` (`record/utf8`, at the first bad byte; span empty at its LF offset).
2. `Record::parse` (`record/<rule()>`, at 1:1).
3. `front_matter()` (`yaml/<rule.as_str()>`, P02's span and position). The record is kept
   (`record: Some`), with no kind.
4. `schema/kind`:
   - with no `kind` field, at 1:1, "a record needs `kind`";
   - when `kind` is not a scalar, or names no registered kind, at its value, naming the
     registered kinds.

   After this rule fails, no other `schema/` rule runs.
5. `SCHEMA_RULES` in order:
   - `schema/id`: `id` missing, at the `kind` value; not a scalar, at its value; or the stem
     breaks the kind's id rule (`registry.id_fits`), at the id value, using the rule's `says`;
   - `index/file-name`: the `id` text differs from the stem, at the id value;
   - `schema/required`: a required field missing, at the `kind` value, "a truth needs
     `target`" (`id` and `kind` excepted);
   - `schema/value`: `FieldSpec::check` for every field the spec knows, except `Id` and
     `KindName`;
   - `schema/unknown-field` (warning): at the key;
   - `schema/unknown-entry` (warning): a `lens` entry not in `LENS_ENTRIES`, at the entry key.
6. The spec's `checks` (error), each with its own `rule`.

Positions come from `record.index().position(span.start)`.

- [ ] **Step 1: Write the failing unit tests** (`check/mod.rs`):
  - `each_rule_reports_at_its_position` `// AC: CE-11.1`: one row per case, each a small
    record given as bytes with its path, asserting the exact `(rule, severity, line, column)`
    list:
    - invalid UTF-8;
    - a broken opening line;
    - an anchor;
    - no `kind`;
    - `kind: kindd`;
    - no `id`;
    - `id` ≠ stem;
    - a band without `floors`, with the message exactly "a band needs `floors`";
    - `number: 104`;
    - an unknown field;
    - a topic whose `track` differs from its suffix;
    - an in-world topic without `counterpart`.
  - `unknown_fields_and_entries_warn` `// AC: CE-11.4`:
    - an npc with `colour: red` and a lens entry `mood: x` gives exactly two warnings, at the
      two keys, and no error;
    - the same npc with an error added (`role: ""`) gives that error plus the same two
      warnings, and without the two keys only the error.
  - `a_broken_front_matter_stops_the_schema` `// AC: CE-11.1`: an anchor plus an unknown kind
    gives only `yaml/anchor`, with `record: Some` and `kind: None`.
  - `an_unknown_kind_stops_the_other_schema_rules`: `kind: kindd` plus an unknown field gives
    only `schema/kind`.
  - `file_names_break_id_rules`: `lore/truths/T-12.md` gives `schema/id`, saying "`T-` and at
    least three digits".
  - `rules_are_unique_and_grouped`: every id of `rules()` holds one `/` and is unique, and the
    list holds every rule this task reports.
- [ ] **Step 2: Run** them. They fail.
- [ ] **Step 3: Implement** the checker, one rule per module.
- [ ] **Step 4: Run** the per-task checks.
- [ ] **Step 5: Commit** with the message `feat(content): check one record against its kind`
  (`Refs: CE-11.1, CE-11.4`).

### Task 7: Which files are records

**Files:**
- Modify: root `Cargo.toml` (`ignore = "0.4"`), crate `Cargo.toml` (`ignore.workspace = true`)
- Create: `src/walk.rs`
- Modify: `src/lib.rs` (`pub mod walk;`, re-export `is_record_path`)

**Interfaces — produces:**

```rust
pub fn is_record_path(path: &str) -> bool;   // project-relative with '/'
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalkedPath { Utf8(String), NotUtf8(String) }   // the latter lossy (L10)
#[derive(Clone, Debug)]
pub struct Walked { pub path: WalkedPath, pub size: u64, pub mtime_ns: i64 }
pub fn walk(root: &Path) -> Result<Vec<Walked>, WalkError>;            // sorted by path; public (L29)
pub(crate) fn relative(root: &Path, path: &Path) -> Option<WalkedPath>;  // None: not a record
pub fn mtime_ns(meta: &std::fs::Metadata) -> i64;                      // ns since the epoch, negative before it
#[derive(Debug, thiserror::Error)]
#[error("cannot walk the project: {source}")]
pub struct WalkError { #[from] source: ignore::Error }
```

`is_record_path` is true when the name ends in `.md` (lower case), the file name is not
`README.md`, and the path does not start with `templates/records/`, `tools/` or `attachments/`.

`walk` uses `ignore::WalkBuilder::new(root)` with:
- `.hidden(true)`, `.parents(false)`, `.ignore(false)`;
- `.git_ignore(true)`, `.git_global(false)`, `.git_exclude(false)`, `.require_git(false)`;
- `.follow_links(false)`.

It keeps entries whose file type is a file (not a symlink) and that `relative` accepts, and
takes the size and time from `DirEntry::metadata()`. Any walk error fails the walk (L12).

- [ ] **Step 1: Write the failing unit tests** (`walk.rs`, a tempfile tree):
  - `records_and_not_records` (Review Focus 2): only `a.md`, `world/tower.md` and
    `templates/synthetic-template.md` are walked, as `Utf8` paths with `/`, sorted. Not walked:
    - `README.md` and `world/README.md`;
    - `templates/records/npc.template.md`, `tools/x.md` and `attachments/x.md`;
    - `notes.MD` and `x.txt`;
    - `.obsidian/x.md`, `.theos/cache/x.md` and `lore/.hidden.md`;
    - `drafts/a.md`, ignored by a root `.gitignore` with `drafts/`;
    - `sub/b.md`, ignored by `sub/.gitignore` with `b.md`.
  - `is_record_path_follows_p01`: a table of the paths above plus `"x.md"` (true) and
    `"tools/a/b.md"` (false).
  - `sizes_and_times_come_from_the_walk`: a file's `size` equals its length, and `mtime_ns`
    equals the time set with `File::set_modified`.
  - `#[cfg(unix)] symbolic_links_are_not_records`: a symlink `l.md` to `a.md` is not walked.
  - `#[cfg(unix)] a_non_utf8_name_is_kept_lossy` and `#[cfg(windows)] a_non_utf8_name_is_kept_lossy`
    (named `…_on_windows`): `relative` of a path built with `OsStrExt::from_bytes(b"x\xff.md")`
    (unix) or `OsStringExt::from_wide` with an unpaired surrogate (windows) gives `NotUtf8`
    ending in `.md`. No file is created, so this runs on macOS too.
- [ ] **Step 2: Run** them. They fail.
- [ ] **Step 3: Implement** `walk.rs`.
- [ ] **Step 4: Run** the per-task checks and `cargo deny --locked check`.
- [ ] **Step 5: Commit** with the message `feat(content): find the record files of a project`.

### Task 8: The store, its identity and the source fingerprint

**Files:**
- Modify: root `Cargo.toml` (`rusqlite = { version = "0.41", features = ["bundled"] }`); crate
  `Cargo.toml` (`rusqlite.workspace = true`, `[build-dependencies] sha2.workspace = true`)
- Create: `build.rs` (L22), `src/index/{mod.rs, schema.sql, store.rs, identity.rs}`
- Modify: `src/lib.rs` (`pub mod index;`, re-export `IndexError`). In `index/mod.rs`, `mod store;`
  and `mod identity;` carry `#[expect(dead_code, reason = "wired into the index by Task 10")]`
  (L29).

**Interfaces — produces:**

```rust
// index/mod.rs (this task: the error only)
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    Sqlite(#[from] rusqlite::Error),                 // "the index cache failed: …"
    Busy { path: PathBuf },                          // L27
    Cache { path: PathBuf, source: std::io::Error }, // cannot create or delete the cache
    CacheInUse { path: PathBuf },                    // damaged and held open: "close the other theos processes"
    Walk(#[from] WalkError),
}
// identity.rs
pub(crate) const FORMAT: i64 = 1;
pub(crate) const SOURCE: &str = env!("THEOS_CONTENT_SOURCE_HASH");
pub(crate) struct Identity { pub format: i64, pub crate_version: &'static str, pub schema_version: i64,
                             pub registry: String, pub rules: String, pub source: &'static str }
impl Identity { pub(crate) fn current(registry: &KindRegistry) -> Identity; }
pub(crate) fn rule_set() -> Vec<(&'static str, Severity)>;   // check::rules() (+ Task 11's link and id rules)
pub(crate) fn rules_fingerprint(rules: &[(&str, Severity)]) -> String;  // SHA-256 of sorted "rule\tseverity\n"
// store.rs
pub(crate) struct Store { pub(crate) conn: rusqlite::Connection, pub(crate) path: Option<PathBuf> }
impl Store {
    pub(crate) fn open(file: &Path, identity: &Identity) -> Result<Store, IndexError>;
    pub(crate) fn in_memory(identity: &Identity) -> Result<Store, IndexError>;
}
```

`schema.sql` (read with `include_str!`). Spans and positions are four integer columns. `hash`
is lower-case hex.

```sql
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
                       message TEXT NOT NULL, step TEXT NOT NULL);   -- step: record | links | ids
CREATE VIRTUAL TABLE search USING fts5(path UNINDEXED, title, fields, body,
                                       tokenize = 'porter unicode61 remove_diacritics 2');
-- indexes: records(id), records(id_fold), records(kind), field_values(path), links(path),
--          links(target), beats(path), findings(path)
```

**`Store::open`:**
1. Create the parent folders.
2. Connect, `busy_timeout(5 s)`, `journal_mode = WAL`.
3. In one `BEGIN IMMEDIATE`, read `meta`. When it differs from the identity, or a table of
   `schema.sql` is missing from `sqlite_master`, drop every table, run `schema.sql`, write
   `meta`, and commit.
4. On `NotADatabase` or `DatabaseCorrupt`:
   - drop the connection;
   - delete the file and its `-wal` and `-shm`;
   - `tracing::warn!`;
   - retry once.

   A failed delete is `CacheInUse`.

`SQLITE_BUSY` maps to `Busy`. `in_memory` runs the same schema on `Connection::open_in_memory`.

- [ ] **Step 1: Write the failing unit tests** (`store.rs`, `identity.rs`):
  - `a_new_cache_has_its_tables_and_identity`: every table exists, `meta` equals the identity,
    and `journal_mode` is `wal`.
  - `a_changed_identity_empties_the_cache`: for each of the six identity fields in turn, a row
    in `records` is gone after a reopen with that field changed, and `meta` holds the new value.
  - `a_damaged_cache_is_rebuilt`: a file `not a database` plus a stray `-wal` opens with every
    table.
  - `a_cache_with_missing_tables_is_rebuilt`: a database holding only a matching `meta` opens
    with every table.
  - `the_source_fingerprint_is_built_in`: `SOURCE` is 64 lower-case hex digits.
  - `rules_fingerprint_is_order_free`: the same rules in two orders give the same fingerprint,
    and another severity gives another.
- [ ] **Step 2: Run** them. They fail.
- [ ] **Step 3: Implement** `build.rs` (a `//!` doc, `main() -> Result<(), Box<dyn
  std::error::Error>>`, a recursive walk of `src` with `std::fs`), `schema.sql`, `store.rs` and
  `identity.rs`.
- [ ] **Step 4: Run** the per-task checks, `cargo deny --locked check` and `cargo xtask lints`.
  `lints` must accept the build script, which its rule 1 allows (spec §9).
- [ ] **Step 5: Commit** with the message `feat(content): keep the index cache in sqlite`.

### Task 9: The rows of one record

**Files:**
- Create: `src/index/{slots.rs, rows.rs}`
- Modify: `src/kinds/mod.rs` (`narrow_fields`), `src/index/mod.rs` (`pub mod slots;`, and `mod
  rows;` with `#[expect(dead_code, reason = "wired into the index by Task 10")]`, L29)

**Interfaces — consumes:** `Checked` (Task 6); `Body::{links, callouts, beats}`;
`CalloutTitle::{Voice, Note}`; `NoteState::ResolvedBy`; `AttributeValue::{Links, Raw}`;
`Link::{span, target, fragment, beat}`; `KindSpec`; `LENS_ENTRIES`, `VOICE_SPEAKER` and
`VOICE_BELIEF`.

**Produces:**

```rust
// slots.rs
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Slot { Field(String), LensEntry(String), Beat(String), VoiceSpeaker, VoiceBelief,
                NoteVersion, NoteResolvedBy, Mention }
impl Slot {
    pub fn as_db(&self) -> String;   // field:<key>, lens:<key>, beat:<key>, voice:speaker, voice:belief,
                                     // note:version, note:resolved-by, mention
    pub fn from_db(text: &str) -> Option<Slot>;
    pub fn targets(&self, spec: Option<&KindSpec>) -> Option<&'static [Target]>;  // None: any kind
}
impl Display for Slot   // "`home`", "`lens.unaware_of`", "a `with::` link", "a voice speaker", …
// kinds/mod.rs
impl KindRegistry { pub fn narrow_fields(&self) -> BTreeSet<&'static str>; }  // every Narrow.field in any slot
// rows.rs
pub(crate) struct RecordRows { pub record: Option<RecordRow>, pub values: Vec<ValueRow>,
    pub links: Vec<LinkRow>, pub beats: Vec<BeatRow>, pub findings: Vec<Finding>,
    pub search: Option<SearchRow> }
pub(crate) struct RecordRow { pub path: String, pub id: String, pub kind: Option<String>,
    pub title: Option<String>, pub status: Option<String>, pub front: Option<String>,
    pub body_start: Option<usize>, pub body_line: Option<usize>, pub facts: String }
pub(crate) enum Shape { Scalar, List, Map }
pub(crate) struct ValueRow { pub field: String, pub entry: Option<String>, pub item: Option<usize>,
    pub shape: Shape, pub text: Option<String>, pub style: Option<ScalarStyle>, pub span: Span,
    pub position: Position, pub valid: bool, pub search_line: Option<usize> }
pub(crate) struct LinkRow { pub slot: Slot, pub beat: Option<u32>, pub target: String,
    pub fragment: Option<String>, pub span: Span, pub position: Position }
pub(crate) struct BeatRow { pub number: u32, pub name: String, pub span: Span, pub line: usize }
pub(crate) struct SearchRow { pub title: String, pub fields: String, pub body: String }
pub(crate) fn rows(path: &str, checked: Checked, registry: &KindRegistry) -> RecordRows;
```

**Values.** There is one row per field scalar, list, list item and map, and per map entry and
entry item (L16).
- `valid` is false when a `schema/value` finding's span overlaps the node's span.
- `search_line` numbers the lines of the `fields` column.

**Links** come only from valid nodes (L17), each with its slot:
- `Link` and `Links` fields of the spec give `Field(key)`;
- valid `unaware_of` and `rejects` items give `LensEntry`.

Body links:
- when the spec has beats: the `Links` values of the listed attribute keys give `Beat(key)`,
  with the beat number;
- every voice title's speaker gives `VoiceSpeaker`, and its beliefs give `VoiceBelief`;
- when the spec has notes: a note's version gives `NoteVersion`, and its `ResolvedBy` link
  `NoteResolvedBy`;
- every other link of `Body::links()`, matched by span, is a `Mention`. That covers unknown
  attribute keys and `Raw` values, and beats and notes in kinds without them.

A front-matter link's span is its scalar; a body link's span is the whole `[[…]]`.

**Beats** are stored for kinds with beats only.

**The search row:**
- `title`: the `title` scalar's text, or empty;
- `fields`: one line per scalar of the fields typed `Id`, `Text`, `TextList`, `Link` or
  `Links`, and of lens entries typed `Text` or `Links`, in file order, `title` excluded;
- `body`: the body text exactly.

A record without a body (decode or split failed) has no search row and no links.

**`facts`** is a canonical text of id, kind, path, the sorted beat numbers, and `field=value`
for every field of `narrow_fields()` with a valid scalar. Two records' facts are equal exactly
when their link facts are (spec §5.9 step 4).

- [ ] **Step 1: Write the failing unit tests** (`rows.rs`, records built from text):
  - `slots_of_every_kind_of_link`: a script with `where: [synthetic-place]` and these body
    lines:
    - `at:: [[a]]`, `with:: [[b]], [[c]]`, `when:: T2` and `foo:: [[d]]`;
    - `> [!voice] [[e]] · [[B-930]] · T1`;
    - prose `[[f]]`;
    - `> [!note] n1 · [[g]] · open`.

    The `(slot, beat, target)` list in document order is:
    - `field:where synthetic-place`;
    - `beat:at` (beat 1) `a`, then `beat:with` `b` and `c`;
    - `mention d`;
    - `voice:speaker e`, `voice:belief B-930`;
    - `mention f`, `mention g`.

    The `a` link's span slices to `[[a]]`.
  - `notes_are_slots_only_in_concepts_and_maps`: a concept with `> [!note] n1 · [[c-v1]] ·
    resolved by [[c-v2]]` gives `note:version` and `note:resolved-by`. A topic with the same
    lines gives two mentions.
  - `links_come_only_from_valid_values`:
    - an npc with `parent: [a]` gives no link;
    - a thing with `where: [a, ""]` gives one link, `a`.
  - `values_hold_scalars_lists_and_maps`: an npc lens with `tier: T4` and `unaware_of: [T-930,
    T-931]` gives the rows, in order:
    - `lens` map (invalid);
    - `lens.tier` scalar (invalid);
    - `lens.unaware_of` list (valid);
    - its two items (valid).
  - `search_columns_and_lines`: title column, field lines in file order with matching
    `search_line`, and a body equal to the record's body text.
  - `a_broken_front_matter_keeps_the_body`: a YAML error gives a record row with no kind, no
    values, body mentions, and a search row with an empty title and fields.
  - `an_undecodable_file_has_only_its_finding`: `record/utf8` gives a record row with no body
    and no links, and no search row.
  - `facts_change_with_link_facts_only`: a title change keeps `facts`; a `track` change or a
    removed beat changes them.
- [ ] **Step 2: Run** them. They fail.
- [ ] **Step 3: Implement** `slots.rs`, `rows.rs` and `narrow_fields`.
- [ ] **Step 4: Run** the per-task checks.
- [ ] **Step 5: Commit** with the message `feat(content): turn a checked record into index rows`.

### Task 10: The index — open, sync, update, apply, queries, dump

**Files:**
- Create: `src/index/{sync.rs, write.rs, query.rs, dump.rs}`
- Modify: `src/index/mod.rs` (remove the three `#[expect(dead_code, …)]` of Tasks 8 and 9),
  `src/lib.rs` (re-export `Index, Change, Update, RecordSummary`)
- Create: `tests/index/{common.rs, sync.rs}`; modify `tests/index/main.rs`

**Interfaces — consumes:** Tasks 2, 6, 7, 8 and 9.

**Produces:**

```rust
pub struct Index { /* root: PathBuf, registry: KindRegistry (L13), store: Store */ }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change { Content { path: String, bytes: Vec<u8> }, Removed { path: String } }   // L14
impl Change { pub fn text(path: impl Into<String>, text: impl Into<String>) -> Change;
              pub fn path(&self) -> &str }
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Update { pub read: Vec<String>, pub reparsed: Vec<String>,
                    pub removed: Vec<String>, pub relinked: Vec<String> }   // sorted; L15
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordSummary { pub id: String, pub path: String, pub kind: Option<String>,
                           pub title: Option<String>, pub status: Option<String> }
impl Index {
    pub fn open(project: &Project, registry: &KindRegistry) -> Result<(Index, Update), IndexError>; // .theos/cache/index.sqlite
    pub fn in_memory(project: &Project, registry: &KindRegistry) -> Result<Index, IndexError>;     // empty
    pub fn sync(&mut self) -> Result<Update, IndexError>;
    pub fn update(&mut self, paths: &[&str]) -> Result<Update, IndexError>;
    pub fn apply(&mut self, changes: &[Change]) -> Result<Update, IndexError>;
    pub fn findings(&self) -> Result<Vec<Finding>, IndexError>;          // by path, line, column, rule
    pub fn findings_for(&self, path: &str) -> Result<Vec<Finding>, IndexError>;
    pub fn records(&self) -> Result<Vec<RecordSummary>, IndexError>;     // by path
    pub fn record(&self, id: &str) -> Result<Vec<RecordSummary>, IndexError>;  // several when duplicated
    pub fn records_of_kind(&self, kind: &str) -> Result<Vec<RecordSummary>, IndexError>;
    pub fn dump(&self) -> Result<String, IndexError>;                    // L21
}
// sync.rs
pub(crate) enum Source { Disk, Given(Vec<u8>), Gone }   // the three changes of spec §5.9
```

**`sync()`** follows spec §5.9 steps 1, 2, 3 and 5 in one `BEGIN IMMEDIATE`. Step 4 arrives in
Task 11.
- Step 1 compares each walked path with its `files` row:
  - a `NotUtf8` path gets its `files` row and the `record/path` finding, without span or
    position (L10);
  - a read that fails with `NotFound` counts as removed;
  - any other read error gives `record/read` at 1:1, with the OS message and a racy row (L11).

  Both findings are built in `sync.rs`, with the rule ids that `check::rules()` lists.
- A file is racy when its `mtime_ns` is later than 2 s before the moment its bytes were hashed.
  The hash is SHA-256 hex.

**`update(paths)`** runs step 1 on the given paths only:
- a path that `is_record_path` refuses is ignored;
- a missing file is removed.

**`apply(changes)`:**
- a `Content` whose hash equals the stored one changes nothing;
- otherwise it is re-parsed, with a `files` row whose size and time are NULL, so the next sync
  reads the file;
- `Removed` removes the path.

`write.rs` deletes every row of a path (records, field_values, links, beats, findings of every
step, search) and inserts `RecordRows`. All of `read`, `reparsed` and `removed` are filled;
`relinked` stays empty in this task.

- [ ] **Step 1: Write the test helpers and the failing tests.**
  - `tests/index/common.rs`:
    - `TempProject::new()` writes data model §13's `theos.toml`;
    - `write(rel, text)` writes the file and sets its time 60 s back with
      `File::set_modified`, so it is not racy;
    - `write_fresh(rel, text)` leaves the time at now;
    - `remove(rel)`, `set_time(rel, SystemTime)`, `root()` and `project()`;
    - `keys(&[Finding]) -> Vec<(String, String, String, usize, usize)>`, for path, rule,
      severity, line and column.
  - `tests/index/sync.rs`:
    - `a_second_open_reads_no_file` `// AC: CE-12.2`: three records. The first open gives
      `read` and `reparsed` = all three; the second open gives an empty `Update`.
    - `changing_one_file_reads_only_that_file` `// AC: CE-12.2`: after a title edit, `sync()`
      gives `read == reparsed == [that path]` and nothing removed.
    - `a_test_part_kind_indexes_and_checks` `// AC: CE-11.5`: `builtin()` plus
      `register("test", widget())` builds an `in_memory` index. `widget()` is `kind
      synthetic-widget`, `Flat("widgets")`, `colour: Text` required. Then:
      - `apply` of `widgets/w1.md` without `colour` gives exactly `schema/required` at the
        `kind` value;
      - with `colour` it gives no finding;
      - with plain `builtin()` the first file gives `schema/kind`.
    - `racy_files_are_read_again` (Review Focus 1): `write_fresh`, then sync. Rewrite with the
      same length and `set_time` back to the stored time: the next sync reads and re-parses
      it. After `set_time` 60 s back, the sync after next reads nothing.
    - `a_time_in_the_future_stays_racy`: a file set 1 h ahead is read by every sync, and
      re-parsed by none.
    - `an_older_copy_is_read`: a file replaced by other bytes with an earlier time is re-parsed.
    - `same_bytes_only_update_the_times`: a new time with the same bytes gives `read ==
      [path]` and `reparsed == []`.
    - `apply_gives_the_dump_of_the_files`: an `in_memory` sync of a folder, and an empty
      `in_memory` with `apply` of the same texts, give equal `dump()`.
    - `not_records_are_ignored_by_update_and_apply` (Review Focus 5): `update(&["README.md"])`
      and `apply` of `templates/records/x.md` give an empty `Update` and an unchanged dump.
    - `a_deleted_file_is_removed`: `removed == [path]`, and nothing of it is left in `dump()`.
    - `#[cfg(target_os = "linux")] a_non_utf8_file_name_is_a_finding` (Review Focus 2): a file
      named with the bytes `x\xff.md` gives exactly one finding, `record/path`, without
      position, under the lossy path. A second sync reads nothing. Linux only: macOS refuses
      such names, and Windows file names are UTF-16. Task 7's `relative` tests cover the other
      two.
    - `findings_are_sorted_and_print_like_p05`: the order is by path, line, column and rule,
      and one finding's `to_string()` equals its expected line.
- [ ] **Step 2: Run** them. They fail.
- [ ] **Step 3: Implement** `Index`, `sync.rs`, `write.rs`, `query.rs` and `dump.rs`.
- [ ] **Step 4: Run** the per-task checks.
- [ ] **Step 5: Commit** with the message `feat(content): open, sync and update the index`
  (`Refs: CE-11.5, CE-12.2`).

### Task 11: Resolving links, ids and placement

**Files:**
- Create: `src/index/resolve/{mod.rs, targets.rs, dangling.rs, kind.rs, placement.rs,
  duplicate.rs}`
- Modify: `src/index/{sync.rs, query.rs, identity.rs, mod.rs}`, `src/lib.rs` (re-export
  `LinkRef`, `Slot`)
- Create: `tests/fixtures/projects/links/` (records, `theos.toml`, `expected.toml`),
  `tests/index/links.rs`
- Modify: `tests/index/common.rs`: `fixture(name) -> PathBuf`, and `expected(path) ->
  Vec<(String, String, String, usize, usize)>`, which reads `[[finding]]` tables in the shape
  of `keys()`

**Interfaces — produces:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkRef { pub source_id: String, pub source_path: String, pub slot: Slot,
    pub target: String, pub fragment: Option<String>, pub span: Span, pub position: Position }
impl Index {
    pub fn backlinks(&self, id: &str) -> Result<Vec<LinkRef>, IndexError>;   // by source path, position
    pub fn children(&self, id: &str) -> Result<Vec<LinkRef>, IndexError>;    // slots field:parent|concept|map
    pub fn links_from(&self, path: &str) -> Result<Vec<LinkRef>, IndexError>;
}
// resolve/
pub(crate) struct TargetInfo { pub path: String, pub kind: Option<String>,
    pub narrow: BTreeMap<String, String>, pub beats: BTreeSet<u32> }
pub(crate) type Targets = BTreeMap<String, Vec<TargetInfo>>;          // by id
pub(crate) struct SourceRecord { pub path: String, pub id: String, pub kind: Option<String>,
    pub kind_at: Option<(Span, Position)>, pub links: Vec<LinkRow> }
pub(crate) struct LinkRule { pub id: &'static str, pub severity: Severity,
    pub run: fn(&SourceRecord, &Targets, &KindRegistry) -> Vec<Finding> }
pub(crate) const LINK_RULES: &[LinkRule];   // dangling (link/dangling), kind (link/kind), placement (index/placement)
pub(crate) const ID_RULE: (&str, Severity); // index/duplicate-id, error
pub(crate) fn relink(tx: &rusqlite::Transaction<'_>, registry: &KindRegistry,
                     sources: &BTreeSet<String>) -> Result<(), IndexError>;
pub(crate) fn recheck_ids(tx: &rusqlite::Transaction<'_>, folds: &BTreeSet<String>) -> Result<(), IndexError>;
pub(crate) fn duplicate_groups(ids: &[(String, String)]) -> Vec<Vec<String>>;  // (path, id) → path groups by ASCII fold
```

`relink` replaces the `links` findings of each source. It loads the sources' link rows, plus
`Targets` for every target id they name: kind, the valid scalars of `narrow_fields()`, and the
beat numbers. Then it runs `LINK_RULES`, all per spec §5.6:
- **dangling**:
  - no record has the id: "`<id>` names no record";
  - a fragment, but no script with that beat: "`S-004` has no beat 3", or "`B-016` is a
    belief; only scripts have beats".
- **kind**: in every slot but `Mention`, at least one target must fit its `Slot::targets` (the
  kind, and the narrowing value).
  - No finding when no target has a known kind, or when a narrowed target lacks its value
    (L18).
  - Example: "`home` must link to a location or floor; `mother-ilse` is an npc".
- **placement** (warning):
  - `Flat(folder)`: the record's folder must equal `folder`, finding at the `kind` value.
  - `UnderParent(field)`: when the parent link resolves, the folder must equal the parent
    file's folder plus `/<parent id>`, finding at the parent link.

`recheck_ids` replaces the `ids` findings of every path whose `id_fold` is in `folds`. Each path
of a group with more than one path gets `index/duplicate-id` at its `id` value (or 1:1), naming
the other paths.

`identity::rule_set` adds the link and id rules. Sync step 4 relinks every record and rechecks
every id fold whenever anything changed (L26). `relinked` lists every record that was not
re-parsed.

**The `links/` fixture project:** all synthetic. Each line gives files, then what they
exercise.
- **World chain:** `world/tower.md`, `world/synthetic-band.md`,
  `world/synthetic-band/f01-synthetic.md` and
  `world/synthetic-band/f01-synthetic/synthetic-place.md`. Right kinds everywhere.
- **People chain:** `people/synthetic-folk.md`, `people/synthetic-folk/synthetic-guild.md`,
  and `…/synthetic-guild/synthetic-speaker.md`. The speaker has `home: synthetic-place` and lens
  `rejects: [B-930]`: right kinds.
- **`people/synthetic-folk/synthetic-wrong.md`** (an npc):
  - `home: synthetic-speaker` gives `link/kind`;
  - lens `unaware_of: [B-930]` gives `link/kind`;
  - a body `[[synthetic-nowhere]]` gives `link/dangling`.
- **Topics:** `lore/topics/synthetic-subject-explaining.md` and
  `synthetic-subject-in-world.md` (its `counterpart` is right), plus
  `synthetic-other-in-world.md`, whose `counterpart: synthetic-subject-in-world` gives a
  narrowed `link/kind`.
- **Truths:** `lore/truths/T-930.md` (`topic` right, `cites: [E930]`), and `T-931.md`, whose
  `topic: synthetic-subject-in-world` gives a narrowed `link/kind` and whose `requires: [B-930]`
  gives `link/kind`.
- **`canon/formalism/E930.md`.**
- **Beliefs:**
  - `lore/beliefs/B-930.md`: `held_by: [synthetic-speaker]`, `distorts: T-930`, and an in-world
    `topic`, all right;
  - `B-931.md`: `held_by: [T-930]` gives `link/kind`, and the explaining `topic` gives a
    narrowed `link/kind`.
- **Clues:** `lore/clues/C-930.md`, with `reveals: T-930` and `where: [f01-synthetic]` right,
  and `after: [C-999]`, which dangles.
- **`story/scripts/S-930.md`:**
  - Beat 1's `at`, `with`, `says` and `reveals` are right;
  - Beat 2's `at:: [[synthetic-speaker]]`, `with:: [[T-930]]`, `says:: [[T-930]]` and
    `reveals:: [[B-930]]` each give `link/kind`;
  - `> [!voice] [[synthetic-speaker]] · [[B-930]]` is right;
  - `> [!voice] [[T-930]] · [[T-931]]` gives `link/kind` twice.
- **`things/synthetic-refs.md`:**
  - `[[S-930#1]]` is right;
  - `[[S-930#9]]` dangles ("no beat 9");
  - `[[B-930#1]]` dangles ("only scripts have beats");
  - `[[T-930]]` is a mention of any kind;
  - `[[synthetic-refs]]` is a self link;
  - `[[todo]]` dangles.
- **Concepts:** `concepts/synthetic-concept.md` (`for: [synthetic-place]`) and
  `concepts/synthetic-concept/synthetic-concept-v1.md`. The note `n1 · [[synthetic-concept-v1]]
  · open` is right; the note `n2 · [[T-930]] · resolved by [[synthetic-speaker]]` gives
  `link/kind` twice.
- **`maps/synthetic-map.md`:** `for: [synthetic-speaker]` gives `link/kind`.
- **`dialogue/synthetic-dialogue.md`:** `speaker: B-930` gives `link/kind`.
- **Duplicates:** `things/synthetic-twin.md` and `dialogue/synthetic-twin.md`, both things,
  give `index/duplicate-id` on both. The second also gives `index/placement`.
- **`things/synthetic-misnamed.md`:** `id: synthetic-renamed` gives `index/file-name`.
- **`lore/synthetic-stray.md`:** a thing, gives `index/placement` at `kind`.
- **`people/synthetic-lost.md`:** an npc with `parent: synthetic-guild`, gives
  `index/placement` at `parent`.
- **`things/synthetic-broken.md`:** an anchor gives `yaml/anchor`.
- **`things/synthetic-target.md`:** a thing with `where: [synthetic-broken]`. The target has no
  known kind, so this gives neither `link/kind` nor `link/dangling`.

- [ ] **Step 1: Write the fixture and the failing tests.** `expected.toml` lists every finding
  as `[[finding]] path, rule, severity, line, column`, each checked by hand (L28).
  - `tests/index/links.rs`:
    - `wrong_kinds_are_errors_in_every_typed_slot` `// AC: CE-11.2`:
      - `keys(findings())` equals `expected.toml`;
      - the `link/kind` findings cover each of the slots `field`, `lens`, `beat`,
        `voice:speaker`, `voice:belief`, `note:version` and `note:resolved-by` (matched by
        their link's position);
      - the right-kind records listed above have no `link/` finding.
    - `narrowed_targets_broken_targets_and_duplicates` `// AC: CE-11.2`: the two narrowed
      findings name `explaining` and `in-world`; `synthetic-target` has no finding; a link to
      `synthetic-twin` resolves.
    - `duplicates_dangling_and_file_names_are_findings_with_paths` `// AC: CE-12.3`:
      - `index/duplicate-id` is on both twin paths, and each message names the other path;
      - `link/dangling` appears in front matter (`C-999`), in prose and in both beat
        references, each with path, line and column;
      - `index/file-name` is at the `id` value.
    - `backlinks_and_children`: `children("synthetic-folk")` are the guild, `synthetic-wrong`
      and `synthetic-lost`; `backlinks("S-930")` holds `things/synthetic-refs.md` twice.
  - Unit in `duplicate.rs`, `names_equal_apart_from_ascii_case_are_duplicates` `// AC:
    CE-12.3`: `[("a/Foo.md","Foo"), ("b/foo.md","foo"), ("c/bar.md","bar")]` gives
    `[["a/Foo.md","b/foo.md"]]`.
- [ ] **Step 2: Run** them. They fail.
- [ ] **Step 3: Implement** `resolve/`, the queries and sync step 4.
- [ ] **Step 4: Run** the per-task checks.
- [ ] **Step 5: Commit** with the message `feat(content): resolve links, ids and placement`
  (`Refs: CE-11.2, CE-12.3`).

### Task 12: Relinking only what a change touches

**Files:**
- Modify: `src/index/sync.rs`
- Create: `tests/index/changes.rs`

**Interfaces:** none new. `Update.relinked` becomes exact (L15).

**Step 4 of spec §5.9, exactly:**
1. Before a path's rows are replaced or deleted, read its old `id` and `facts`.
2. When the old and new `facts` differ (the record appeared, disappeared, or changed id, kind,
   path, beats or a narrowing value), add the old and new id to `affected`.
3. `sources` = the re-parsed paths, plus the distinct `links.path` whose `target` is in
   `affected`, minus the removed paths.
4. Run `relink(sources)` and `recheck_ids` over the ASCII folds of `affected`.
5. `relinked` = `sources` minus the re-parsed paths.

- [ ] **Step 1: Write the failing tests** (`tests/index/changes.rs`). The test writes this
  project:
  - `world/synthetic-band.md` and `world/synthetic-band/f01-synthetic.md`;
  - `people/synthetic-folk.md`, `…/synthetic-guild.md` (a faction) and
    `…/synthetic-guild/synthetic-speaker.md` (an npc);
  - `lore/topics/synthetic-subject-explaining.md` and `synthetic-subject-in-world.md`;
  - `lore/truths/T-940.md`, with the explaining `topic`;
  - `lore/beliefs/B-940.md`, with `held_by: [synthetic-speaker]` and the in-world `topic`;
  - `story/scripts/S-940.md`, whose two beats have `at:: [[f01-synthetic]]` and `with::
    [[synthetic-speaker]]`;
  - `things/synthetic-reader.md`, whose body links `[[S-940#2]]`, `[[B-940]]` and
    `[[C-940]]` (dangling at first).

  It opens a file-based index, makes each change below on disk (`write`, with a time 60 s
  back), and calls `sync()`.
  - `each_change_updates_its_record_and_its_backlinks` `// AC: CE-12.2`: the changes run in
    this order. After each, `Update` equals the given lists (the others empty), and `dump()`
    equals the dump of a fresh `in_memory` + `sync()` of the same folder.
    1. `[[T-940]]` added to the reader's body: `read` and `reparsed` = the reader;
       `relinked` empty.
    2. `B-940`'s title changed: `read` and `reparsed` = `B-940`; `relinked` empty.
    3. `B-940.md` renamed to `B-941.md`, with its `id`: `removed` = the old path; `read` and
       `reparsed` = the new one; `relinked` = the reader.
    4. `synthetic-speaker`'s `kind` changed to `faction`: `read` and `reparsed` = the speaker;
       `relinked` = `B-941` and `S-940`.
    5. Beat 2 removed from `S-940`: `read` and `reparsed` = `S-940`; `relinked` = the reader.
    6. The in-world topic's `track` changed to `explaining`: `read` and `reparsed` = that
       topic; `relinked` = `B-941`.
    7. `T-940.md` deleted: `removed` = it; `relinked` = the reader.
    8. `lore/clues/C-940.md` added: `read` and `reparsed` = it; `relinked` = the reader, whose
       `link/dangling` for `C-940` is now gone.
  - `duplicates_come_and_go_with_changes` (Review Focus 3): adding `dialogue/T-940.md` with
    `id: T-940` gives `index/duplicate-id` on both paths; removing it clears both. The dump
    equals a fresh build each time.
  - `a_rename_with_an_edit` (Review Focus 3): renaming the npc file and changing its title in
    one sync gives the same dump as a fresh build, with `relinked` = its backlinks.
- [ ] **Step 2: Run** them. The `relinked` assertions fail (L26 relinks everything).
- [ ] **Step 3: Implement** the affected set.
- [ ] **Step 4: Run** the per-task checks.
- [ ] **Step 5: Commit** with the message `feat(content): relink only what a change touches`
  (`Refs: CE-12.2`).

### Task 13: Search

**Files:**
- Create: `src/index/search.rs`, `tests/fixtures/projects/search/`, `tests/index/search.rs`
- Modify: `src/index/mod.rs`, `src/lib.rs` (re-export `Hit, Match, Column`)

**Interfaces — produces:**

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Hit { pub id: String, pub path: String, pub kind: Option<String>,
                 pub title: Option<String>, pub score: f64, pub matches: Vec<Match> }  // score = -bm25: higher is better
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match { pub column: Column, pub span: Span, pub position: Position, pub line: String }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Column { Title, Field { key: String, entry: Option<String> }, Body }
impl Index { pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Hit>, IndexError>; }
pub(crate) fn fts_query(query: &str) -> Option<String>;   // None: no terms
pub(crate) const MATCHES_PER_HIT: usize = 10;             // L20
```

**`fts_query`:**
- Split at whitespace into words. A `"` ends the current word and starts a phrase, which runs to
  the next `"` or the end of the query (L20).
- A term that is empty or only whitespace is dropped.
- Each remaining term becomes `"…"`, with any `"` doubled, and the terms are joined by spaces.
- No term left (an empty query, only spaces, `""`) gives `None`, so no hits.

**The query:**

```sql
SELECT r.id, r.path, r.kind, r.title, bm25(search, 0.0, 5.0, 2.0, 1.0) AS relevance,
       highlight(search, 1, ?2, ?3), highlight(search, 2, ?2, ?3), highlight(search, 3, ?2, ?3),
       search.title, search.fields, search.body, r.body_start, r.body_line, r.front
FROM search JOIN records r ON r.path = search.path
WHERE search MATCH ?1 ORDER BY relevance, r.id LIMIT ?4
```

`?2` is U+FDD0 and `?3` is U+FDD1. The alias is not `rank`, which is a hidden column of every
FTS5 table.

**Mapping matches.** Per column, strip the markers and collect byte ranges. When the text
without markers differs from the stored column, that column gives no matches (spec §5.11).
- **Body:**
  - span: `body_start` + offset;
  - line: `body_line` + newlines before the match;
  - column: chars since the last newline + 1;
  - line text: that body line.
- **Title and fields** go through the `field_values` row (`field = 'title'`, or the
  `search_line`):
  - when `span.len() == text.len() + q`, with `q` 0 for a plain value and 2 for a quoted one,
    the match maps word by word: span = `span.start + q/2 + offset`, and column = the value's
    column + `q/2` + chars before the offset;
  - otherwise the match is the whole value.

  The line text is that line of `records.front`.

Matches are sorted by span start and cut at `MATCHES_PER_HIT`.

**The `search/` fixture project**, all things unless stated:
- `synthetic-lantern`: the title "Synthetic lantern"; its body says "glow" once.
- `synthetic-lamp`: the title "Synthetic lamp"; its body holds "the lantern glows" once.
- `synthetic-beacon`: the body names "lantern" three times, with about the length of
  `synthetic-lamp`.
- `synthetic-cafe`: the title "Synthetic Café"; the body "Mère".
- `lore/truths/T-950.md`: a body with `T-951` in prose and `[[T-951|the other]]`.
- `synthetic-red-stone`: the body "red stone".
- `synthetic-stone-red`: the body "stone red".
- `synthetic-quoted`: `title: "Synthetic \"quoted\" lantern"`.
- `synthetic-noncharacter`: a body with U+FDD0 and "lantern".
- `synthetic-broken`: an anchor in the front matter, and "lantern" in the body.

- [ ] **Step 1: Write the fixture and the failing tests** (`tests/index/search.rs`, the
  fixture indexed in memory):
  - `hits_are_ranked_with_their_matching_text` `// AC: CE-13.1`: `search("lantern", 20)`:
    - `synthetic-lantern` (a title match) ranks above `synthetic-lamp` (a body match);
    - every match's span, applied to the file's text, gives `lantern` in any case;
    - position and line text match the file's line.
  - `more_matches_rank_higher` `// AC: CE-13.1`: `synthetic-beacon` ranks above
    `synthetic-lamp`.
  - `stems_accents_ids_and_phrases` `// AC: CE-13.1`:
    - `glow` finds the lamp (`glows`) and the lantern;
    - `cafe` and `mere` find `synthetic-cafe`;
    - `T-951` finds `T-950`, with two body matches;
    - `"red stone"` finds only `synthetic-red-stone`, and `red stone` finds both.
  - `query_syntax_is_plain_text` `// AC: CE-13.1`:
    - `AND`, `OR`, `NEAR`, `lantern*`, `-lantern`, `title:lantern`, `^lantern`, `"lantern`
      and `lantern"` give no error;
    - `lantern*` finds what `lantern` finds;
    - `""`, `"   "`, `-` and `nosuchword` give no hits.
  - `a_quoted_value_with_escapes_matches_as_a_whole`: the quoted title's match is the whole
    value's span.
  - `a_noncharacter_keeps_the_rank_without_spans`: `synthetic-noncharacter` is a hit with no
    body matches.
  - `a_broken_front_matter_is_found_by_its_body`: `synthetic-broken` is a hit.
  - Units for `fts_query`:
    - `a b` gives `"a" "b"`;
    - `"a b" c` gives `"a b" "c"`;
    - `a"b` gives `"a" "b"`;
    - `x"y z` gives `"x" "y z"`;
    - `""` and `"  "` give `None`.
- [ ] **Step 2: Run** them. They fail.
- [ ] **Step 3: Implement** `search.rs`.
- [ ] **Step 4: Run** the per-task checks.
- [ ] **Step 5: Commit** with the message `feat(content): search records with ranked matches`
  (`Refs: CE-13.1`).

### Task 14: Effective lenses

**Files:**
- Create: `src/index/lens.rs`, `tests/fixtures/projects/lenses/`, `tests/index/lenses.rs`
- Modify: `src/index/mod.rs`, `src/lib.rs` (re-exports)

**Interfaces — produces:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer { pub id: String, pub kind: String, pub path: String }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin { pub layer: Layer, pub span: Span, pub position: Position }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LensData { Text(String), Links(Vec<String>) }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overridden { pub value: LensData, pub origin: Origin }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LensValue { pub field: String, pub value: LensData, pub origin: Origin,
                       pub overrides: Vec<Overridden> }      // nearest first
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IgnoredEntry { pub field: String, pub origin: Origin, pub findings: Vec<Finding> } // field "lens" for a non-map lens
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldBelief { pub belief: String, pub path: String, pub through: Layer }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveLens { pub chain: Vec<Layer>, pub fields: Vec<LensValue>,
                           pub ignored: Vec<IgnoredEntry>, pub beliefs: Vec<HeldBelief> }
impl Index { pub fn lens(&self, holder: &str) -> Result<Option<EffectiveLens>, IndexError>; }
```

- **Holders** are the records whose kind's spec has a `Lens` field. `lens` gives `None` for an
  unknown id or another kind.
- **The chain** starts at the holder (the first record by path when the id is duplicated, L19).
  Each step follows the layer's `field:parent` link to the first record by path whose kind is
  in that field's targets and is itself a holder. It ends at a dangling or wrong-kind parent,
  or at an id already in the chain.
- **Per layer**, from `field_values`:
  - a `lens` row that is a valid map: each entry of `LENS_ENTRIES` sets its field when its
    entry node is valid, as `Text` for a scalar, or `Links` for the items of a list, an empty
    list included;
  - an invalid entry goes to `ignored` with the `schema/value` findings that overlap it;
  - a `lens` that is not a map goes to `ignored` as `lens`.
- **`fields`** come in `LENS_ENTRIES` order, only those some layer sets. The value is the
  nearest layer's; the overrides are the farther layers', nearest first, even when they are
  equal.
- **`beliefs`**: belief records with a `field:held_by` link to a layer's id, ordered by layer
  (holder first), then by belief id.

**The `lenses/` fixture project:**
- `people/synthetic-folk.md` lens: `thinks_from: synthetic frame A`, `stance: synthetic stance
  A`, `tier: T1`, `unaware_of: [T-960, T-961]`, `speaks_to: synthetic audience A`.
- `people/synthetic-folk/synthetic-guild.md` (a faction) lens: `stance: synthetic stance B`,
  `tier: T2`.
- `…/synthetic-guild/synthetic-elder.md` (an npc) lens: `stance: synthetic stance C`, `tier:
  T2`, `unaware_of: [T-962]`.
- `people/synthetic-folk/synthetic-loner.md` (an npc): no lens.
- `…/synthetic-guild/synthetic-odd.md` (an npc) lens: `tier: T4`, `unaware_of: T-960`, `mood:
  synthetic`.
- `people/synthetic-folk/synthetic-flat.md` (an npc): `lens: synthetic`.
- `people/synthetic-stray.md` (an npc): `parent: synthetic-nowhere`.
- `lore/truths/T-960…T-962`.
- `lore/beliefs/B-960` (held by the folk), `B-961` (held by the guild), `B-962` (held by the
  elder).

- [ ] **Step 1: Write the fixture and the failing tests** (`tests/index/lenses.rs`; positions
  are the values' line and column in the files):
  - `an_npc_lens_names_the_layer_of_each_value` `// AC: CE-13.2`: `lens("synthetic-elder")`:
    - the chain is elder, guild, folk;
    - `thinks_from` and `speaks_to` come from the folk (kind `people`, with path and
      position);
    - `stance`, `tier` and `unaware_of` (`[T-962]`) come from the elder;
    - there is no `rejects`.
  - `an_npc_under_its_people` `// AC: CE-13.2`: `synthetic-loner`'s chain is loner, folk, and
    every value comes from the folk.
  - `beliefs_come_through_each_layer` `// AC: CE-13.2`: B-962 through the elder, B-961 through
    the guild, B-960 through the folk.
  - `a_child_value_overrides_its_parent` `// AC: CE-13.3`:
    - the elder's `stance` overrides the guild's B, then the folk's A;
    - its `unaware_of` replaces the folk's two-item list, which is listed in `overrides`;
    - its `tier: T2` overrides the guild's equal `T2` and the folk's `T1`.
  - `invalid_entries_do_not_override` (Review Focus 4):
    - `synthetic-odd`'s `tier` is the guild's `T2`, and its `unaware_of` the folk's list;
    - `ignored` holds `tier` and `unaware_of` with their findings;
    - `mood` is in neither;
    - `synthetic-flat` ignores `lens` and takes everything from the folk.
  - `bad_parents_end_the_chain_and_non_holders_have_none`: `synthetic-stray`'s chain is only
    itself; `lens("T-960")` and `lens("nobody")` are `None`.
- [ ] **Step 2: Run** them. They fail.
- [ ] **Step 3: Implement** `lens.rs`.
- [ ] **Step 4: Run** the per-task checks.
- [ ] **Step 5: Commit** with the message `feat(content): give effective lenses with their
  layers` (`Refs: CE-13.2, CE-13.3`).

### Task 15: Preview, cache rebuilds and two processes

**Files:**
- Modify: `src/index/{mod.rs, sync.rs}`, `tests/index/common.rs` (`TempProject::copy_of(name)`
  copies a fixture project into a temp folder)
- Create: `tests/index/cache.rs`

**Interfaces — produces:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview { pub update: Update, pub findings: Vec<Finding> }  // of every re-parsed or relinked record, sorted
impl Index { pub fn preview(&mut self, changes: &[Change]) -> Result<Preview, IndexError>; }
```

`preview` runs `apply`'s steps 3 and 4 in a `BEGIN IMMEDIATE` transaction. It reads the
findings of `reparsed ∪ relinked`, then rolls back.

- [ ] **Step 1: Write the failing tests** (`tests/index/cache.rs`, temp copies of the `links/`
  fixture):
  - `deleting_the_cache_rebuilds_the_same_index` `// AC: CE-12.4`: open, then take `dump()`
    and the hits of `search` for `synthetic`, `T-930`, `"synthetic stance"` and `beat`. Delete
    `.theos/cache` and open again. The dump is equal, and the hits are equal in id, path,
    matches and order, with scores within 1e-9.
  - `a_damaged_cache_and_a_new_fingerprint_rebuild_the_same_index` `// AC: CE-12.4`:
    - garbage written over `index.sqlite` (the index dropped first) reopens to the same dump;
    - a registry with one more test kind that no file uses reopens with every file in
      `reparsed`, to the dump of a fresh build with that registry.
  - `preview_returns_what_update_stores_and_changes_nothing`: a preview of an edit adding
    `[[synthetic-nowhere]]` has that `link/dangling` among its findings, and `dump()` is
    unchanged. Writing the same text and `update` stores exactly those findings for that path.
  - `preview_of_a_removal_and_a_rename` (Review Focus 5):
    - `Removed` of `T-930` gives `link/dangling` on its backlinks;
    - `Removed` of the old path plus `Content` at a new path with the same text moves its
      findings;
    - the dump is unchanged after both.
  - `preview_of_text_that_fails_to_split` (Review Focus 5): `Content` without `---` gives
    exactly `record/opening-line` for that path.
  - `two_indexes_sync_from_two_threads`: `std::thread::scope` with two `Index::open` on one
    project; both succeed, and their dumps are equal.
- [ ] **Step 2: Run** them. They fail.
- [ ] **Step 3: Implement** `preview`.
- [ ] **Step 4: Run** the per-task checks.
- [ ] **Step 5: Commit** with the message `feat(content): preview changes and rebuild the cache`
  (`Refs: CE-12.4`).

### Task 16: The corpus as a project, and the schema fixtures

**Files:**
- Create: `tests/fixtures/corpus/expected-index.toml`, `tests/fixtures/projects/schema/`,
  `tests/index/{corpus.rs, schema.rs}`
- Modify: `tests/index/common.rs` (`TempProject::corpus()`, `TempProject::with_file(rel,
  bytes)`)

**`expected-index.toml`** holds:
- `[[project]]` entries (path, rule, severity, line, column): every finding of `records/` plus
  `malformed/` as one project with data model §13's `theos.toml`;
- `[[crlf]]` entries for the three CRLF files without a twin in `records/` (L23).

Each entry is checked by hand (L28). The expected kinds of findings are:
- the dangling data model ids and P01 placeholders;
- placement warnings, where a corpus file sits outside its kind's folder;
- real schema faults of the corpus, if any. A schema fault in a P02 corpus file that is
  actually valid by the data model is a bug in P03, to be fixed.

**The `schema/` fixture project** has one record per row of spec §5.5 that a file can show, and
per field type of §5.4, with a valid and an invalid record for each type:
- the inputs are those of Task 4's tests;
- the targets the valid ones link to exist;
- `record/utf8` comes through `apply` of bytes in the test, since fixture files are text.

- [ ] **Step 1: Write the lists, the fixture and the failing tests:**
  - `corpus.rs`:
    - `the_corpus_project_reports_exactly_its_findings` `// AC: CE-11.1`: `keys(findings())`
      equals the `[[project]]` list.
    - `crlf_files_report_their_twins_findings` `// AC: CE-11.1`: for each `[[twin]]`, the
      input and the LF file, each alone at the same path (L23), give equal findings in rule,
      severity, line, column and message. The three non-record twins' findings equal
      `[[crlf]]`.
    - `rejected_files_report_only_their_rule` `// AC: CE-11.1`: each `rejected/` file alone,
      at its own name, gives exactly one finding: `record/<rule>` for `bom`, `opening-line`
      and `closing-line`, else `yaml/<rule>`, at P02's line and column from
      `rejected/expected.toml`.
  - `schema.rs`:
    - `schema_fixtures_report_exactly_their_findings` `// AC: CE-11.1`: equal to
      `schema/expected.toml`, plus `record/utf8` for the bytes applied.
    - `unknown_fields_warn` `// AC: CE-11.4`: the unknown field and the unknown lens entry are
      warnings at their keys, and their records have no error.
- [ ] **Step 2: Run** them. Fix any failure at its cause, in the code or in an expectation
  that was not checked by hand (L28), never by weakening the test.
- [ ] **Step 3: Run** the per-task checks.
- [ ] **Step 4: Commit** with the message `test(content): check the corpus and the schema
  fixtures as projects` (`Refs: CE-11.1, CE-11.4`).

### Task 17: 500 records under a second; the README and crate docs

**Files:**
- Create: `tests/index/speed.rs`
- Modify: `crates/theos-content/README.md`, `src/lib.rs` (crate docs)

- [ ] **Step 1: Write the test.** `a_full_index_of_500_records_takes_under_a_second` `// AC:
  CE-12.1`:
  - It writes 500 records into a temp project, 25 per kind, all valid. Links go to earlier
    records of the right kinds. Bodies are about 1 KB: neutral synthetic sentences with three
    wikilinks, one voice block, and two beats in scripts.
  - Three times, it deletes `.theos` and times `Index::open`.
  - It prints all three durations, asserts the fastest is under 1 s, and asserts that
    `records()` has 500 entries.
- [ ] **Step 2: Run** it. If it fails, profile before changing anything: prepared statements
  (`prepare_cached`), one transaction, no per-row `SELECT`. Never raise the limit.
- [ ] **Step 3: Write** the README (what `theos-content` provides after P03, the cache and its
  identity, the fixtures, and what each later phase uses, from spec §6) and the `lib.rs` docs.
- [ ] **Step 4: Run** the per-task checks, then `cargo xtask ac-trace --phase P03`: every AC
  from CE-11.1 to CE-13.3 shows at least one passing test.
- [ ] **Step 5: Commit** with the message `test(content): index 500 records under a second`
  (`Refs: CE-12.1`), and the docs as `docs(content): describe what the crate provides after
  P03`.

## After the tasks

theos-phase steps 4 to 6:
1. The final review loop of `CLAUDE.md`: a fresh reviewer on the most capable model each round,
   cap 5.
2. `cargo xtask gate` on Windows, and `cargo xtask ac-trace --phase P03`.
3. The PR `feat(content): kinds, index, search, lenses (P03)`. It:
   - lists every ruling of the ledger: L1–L29 above, Task 1 as the P02 carry-over R5-1, and the
     build's;
   - gets the labels `windows` and `macos` (spec §5.14), with links to their `platforms` runs;
   - sets P03 to `pr-open` in `docs/status.md` in the same push.
