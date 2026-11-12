# P04 · Validation, lore rules, ledger, formalism mirror — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (`CLAUDE.md` makes it the execution method) to implement this plan task by task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** `theos-content` judges records against each other and against the canon:
- the lore rules, with guardrails no setting lowers;
- P02's body data;
- validation settings from `.theos/settings.toml`;
- the reveal ledger in the cache;
- binary references;
- the Shared Formalism mirror.

`theos` prints the ledger and regenerates the mirror.

**Architecture:** three layers over P03's index:
- **The record step** (`check`) adds body rules, judged per kind through `BodyGrammar`. The rows
  module also stores what the lore step reads: notes, `when::` tokens and in-world segments.
- **The lore step** (`lore`, `index::lore_step`) runs after P03's relink. Each subject is a
  record or an input file (`.theos/settings.toml`, `canon/formalism/_source.json`). A subject's
  rules read through a `Context` that stores every key it reads (`rec:<id>`, `in:<id>`,
  `file:<path>`) in a `deps` table. A change re-runs exactly the subjects that read a changed
  key. A cold build runs every subject, with facts loaded in bulk.
- **Rules** are modules registered by their editor part (`register_rule`). The body rules and
  the settings rules are lists of their own. Every rule has a class (guardrail, configurable,
  fixed). Configured severities apply when findings are read.

**Tech stack:**
- Rust 1.99.1 (edition 2024).
- No new crate in `Cargo.lock`.
- `theos-content` adds `serde` (derive) and `serde_json` as normal dependencies.
- `theos` adds `clap`, `anyhow` and `theos-content`.
- From P03: `rusqlite` (bundled), `toml` 1.1, `regex` 1.13, `sha2` 0.11, `thiserror`,
  `tracing`.

**Spec:** `docs/specs/phases/P04-validation-lore-rules-ledger-formalism-mirror.md` (approved and
merged, #21). Read it with this plan. Where this plan is silent, the spec decides; where they
differ, the spec wins.

**Probes run while planning** (10 Nov 2026, toolchain 1.99.1, scratch crate outside the repo), in
addition to spec §4.2:
- **`toml::de::DeTable::parse`** (toml 1.1.8) returns a `Spanned<DeTable>`, with a span for
  every key, every value and every array item. Probe: `[validate]` with `bogus = 1`,
  `[validate.severity]`, `[validate.inworld_vocab]` and `[validate.inworld-vocab]` with
  `allow = [{ word = "T1", reason = "r" }, { word = "canon" }]` gave:
  - the key spans `bogus` 37..42, `inworld_vocab` 107..120, `"lore/three-clue"` 68..85;
  - the item spans 171..200 and 202..220.

  A table header's value span covers its `[…]` line. So the settings are read from the spanned
  `DeTable`, not with serde derive: unknown keys need their positions (spec §5.12).
- **`sha2` 0.11.1 does not implement `std::io::Write`.** `std::io::copy` into a `Sha256` does not
  compile. A loop over a fixed buffer with `Read::read` and `update` gives the right digest
  (`sha256("abc") = ba7816bf…15ad`).

## Global Constraints

Umbrella global constraints, verbatim. `CLAUDE.md` asks for G1–G16; G17 to G19 were added to the
umbrella later and are copied too, as in the P02 and P03 plans.

- **G1** Repositories: `theos-editor` (this repo) holds the editor and the map generator
  (`theos-map-format`, `theos-maps`). `tower-of-theos` holds the content project, its map recipes
  and the blockout generator. `Theos-Engine` is never a dependency, except behind the cargo
  feature `engine`, which no phase here builds; the engine may depend on the map crates.
- **G2** Platforms: Windows 11 x64 (primary), Linux x64, and macOS 13 or later on Apple silicon
  and Intel (universal builds). Every PR is checked on Linux; Windows and macOS are tested
  weekly, on demand (a label on the PR) and before every release (`docs/gate.md` §1).
- **G3** Rust: stable toolchain pinned in `rust-toolchain.toml`, edition 2024, `unsafe_code`
  forbidden, `Cargo.lock` committed, all builds `--locked`.
- **G4** Crates: `theos-content`, `theos-vcs`, `theos-studio`, `theos-map-format`, `theos-maps`
  (libs); `theos`, `theos-mcp`, `theos-editor` (bins); `xtask`. Paths `crates/<name>`, `xtask/`.
  UI in `editor-ui/` (Svelte 5, TypeScript, pnpm).
- **G5** Content files follow `docs/specs/data-model.md`; the restricted YAML parser is ours;
  any edit leaves untouched bytes identical.
- **G6** Git only through the `git` command line; credentials only through the user's
  credential helper.
- **G7** MCP: stdio only, `rmcp`, tool names `^[a-z][a-z0-9_]*$`, agents only propose.
- **G8** `never-confirm`, `no-overshoot`, `inworld-vocab` are always blocking; nothing lowers them.
- **G9** Text of a truth with target `never` never appears in an LLM prompt.
- **G10** Binaries never enter git; references are `path@sha256:<hex>` relative to the binaries
  folder.
- **G11** Generators only through `docs/generators.md`, the built-in map generator included; no
  generator-specific field in the data model; the UI works for any generator that produces a kind.
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
- **G19** Positions on maps are fractions of a map version's frame (x east, y south, origin
  north-west); metres are derived from the frame. Pins and cameras live in the map record's body,
  never on the placed record (`docs/specs/data-model.md` §9.6–9.7).
- **G20** Map generation is deterministic: the same generator version, recipe, seed and inputs
  give the same package bytes and layers on every platform of G2. Canon maps are baked into
  packages (`docs/map-package.md`) and shipped, never regenerated in the game (decision 0003).

(G1, G4 and G11 were updated and G20 added on `main` while P04 was being built — umbrella changes
for P08; none of them changes what P04 builds.)

Phase ACs, verbatim:

**CE-14 Validator core**
- CE-14.1 A setting that lowers a guardrail is rejected with a message.
- CE-14.2 Findings carry rule, severity, file and line.
- CE-14.3 Validating one record also validates the neighbours it affects.

**CE-15 Lore rules and ledger**
- CE-15.1 never-confirm, no-overshoot, inworld-vocab, reveal-order, requires and three-clue each
  have a passing and a failing fixture.
- CE-15.2 An allowlist entry without a reason is an error.
- CE-15.3 The ledger is built from clue records into the cache and printed by `theos ledger`.
- CE-15.4 A missing binary or a hash mismatch is a warning.
- CE-15.5 A note title with an unknown state, or `addresses` naming a note that does not exist, is
  an error.

**CE-16 Shared Formalism mirror**
- CE-16.1 `canon/formalism/_source.json` (written once by a private session from Confluence)
  generates the records `E1`…`E18`.
- CE-16.2 A record citing `E7` resolves.
- CE-16.3 A hand edit of a mirror record fails the check.
- CE-16.4 Regenerating is idempotent.

**Jann's rulings that bind the build** (spec §4.1):
- checks across records store their reads; a change re-runs exactly the records that read what it
  touched;
- nothing reveals a `never` truth;
- developer words are built-in patterns (ids, tier tokens, rule names) plus a project list;
- validation settings come from the project's `.theos/settings.toml` only; a bad entry is an
  error finding at the entry and has no effect.

P02's ruling D1 and P03's rulings still hold. In particular, a link is stored only from a valid
value, and the cache is never a source of truth.

## Build rules (every task, every subagent)

Put this section into every implementer and reviewer brief.

- **Engineering directives** (`CLAUDE.md`):
  - Small modules with one job. A new lore rule is one module plus one line in its part's
    `register`; a new body rule is one module plus one line in `BODY_RULES`.
  - Use the crate before custom code. SQLite stores; `toml` (`DeTable`) reads the settings;
    `serde_json` reads `_source.json`; `regex` matches developer words; `sha2` hashes; P02's YAML
    encoder spells scalars; `clap` parses arguments.
  - Never hand-write a parser for TOML, JSON, YAML or Markdown, or a word matcher.
  - If reviews keep finding new variants of one problem, stop and name the class (directive 3).
- **Rules get no SQL.** A lore rule reads only through `Context` (Task 4), so every read is
  stored. Rules never touch a `Connection`, the store or the file system.
- **SQL** lives in `src/index/schema.sql` (tables) and in the module that runs each query. No
  ORM. Writes stay inside the sync's one `BEGIN IMMEDIATE` transaction; `binary_findings` opens
  its own (Task 15).
- **Lints** (workspace, `-D warnings`):
  - No `#[allow]` or `#[warn]`. `#[expect]` only with `reason = "…"`.
  - Every public item is documented (`missing_docs`). Every public fn returning `Result` has an
    `# Errors` section.
  - No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, `print!` or `exit`
    outside tests. `theos` writes with `writeln!` to a locked stdout or stderr.
  - Clippy pedantic is on.
  - Coverage counts `src/**` with its error paths: test them. Library crates need 80 %, `theos`
    60 %.
- **Dead code between tasks.** A module that a later task wires in carries
  `#[cfg_attr(not(test), expect(dead_code, reason = "wired in by Task N"))]` on its `mod` line,
  never a bare `expect`. The task that wires it in removes the attribute.
- **No dead code in tests either.** A helper, or a deserialised field nobody reads, fails clippy.
  Add each in the task whose test first uses it.
- **Docs:** code blocks in doc comments are ```` ```text ```` unless they are doctests that
  pass. Intra-doc links must resolve.
- **Tests:**
  - Units go in `#[cfg(test)] mod tests` next to the code.
  - Behaviour goes in the integration-test crate `crates/theos-content/tests/index/`
    (`main.rs` plus one module per file). CLI tests go in `crates/theos/tests/cli.rs`.
  - A test proving an AC is a plain `#[test]` fn with `// AC: CE-xx.y` on the line directly
    above `#[test]`. A tag inside `proptest!` has no owner and fails `ac-trace`.
  - Tests may return `Result<(), Box<dyn std::error::Error>>`.
  - Fixture projects are indexed in place with `Index::in_memory` plus `sync()`. A test that
    writes files (regenerate, binaries memo, history) copies the project into a `tempfile`
    folder first (`TempProject::copy_of`).
- **Fixtures:**
  - Every new record says it is synthetic: `# synthetic fixture (P04), not canon` as the first
    front-matter line. `.theos/settings.toml` and `_source.json` say it in a comment line, or
    in a `"note"` key that the reader ignores (L16).
  - Texts are neutral ("Synthetic statement A"), never lore.
  - Ids are `synthetic-…` slugs and truth, belief, clue and script numbers from 950 up (L2).
    Equation ids are `E1`…`E18`, in the `formalism/` fixture only.
  - `expected.toml` files are evidence, not snapshots (P03 L28). Write each finding only after
    checking it by hand against the spec. A finding that shows a bug is fixed in the code, never
    listed.
- **P03's expectations.** A task whose rules add findings to P03's fixtures updates P03's
  expectation files (`corpus/expected-index.toml`, `projects/{links,schema,lenses,search}/
  expected.toml`) in the same task, each new finding checked by hand. No P03 expectation is
  dropped (spec §9).
- **Files with backslashes** (Rust string escapes, JSON with LaTeX, fixtures) are written with
  the Write or Edit tool, never with a shell heredoc.
- **Git:**
  - One git command per Bash call in the worktree (no `&&` chains). Never `--no-verify`.
  - Commit message: a Conventional Commit with scope `content`, or `cli` for `crates/theos`
    (`build` for a dependency-only commit). A body says why. The footers below come as one
    block.
  - No footer line over 100 characters: split long `Refs:` into several `Refs:` lines. Leave
    `Refs:` out when a task proves no AC.

  ```text
  Refs: CE-15.1
  Phase: P04
  Claude-Session: a23138b4-01c6-4789-b5a0-17930c8eab30
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  ```
- **Per-task checks** (run from the worktree root, read the output):
  - `cargo fmt --all --check`;
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`;
  - `cargo nextest run --workspace --locked`;
  - a task that adds a dependency also runs `cargo deny --locked check`.

## Review Focus

These are the five input classes the spec implies, that no AC test covers and that are most
likely to bite. Each has its test in the owning task.

1. **A record that others read is renamed, deleted or re-created.** Expected: every subject that
   read its id re-runs. A clue of a deleted truth loses its never-confirm finding, and gets it
   back when the truth returns. `dump()` equals a cold build. Tests: Task 4
   (`a_renamed_or_removed_dependency_reruns_its_readers`), Task 17 (the history test).
2. **The settings file is broken, removed or changed in the middle of a session.** Expected:
   - broken TOML gives `settings/toml` and `[validate]` counts as empty;
   - guardrails stay errors;
   - the project's words drop to the built-in ones;
   - a fixed file restores everything without a rebuild.

   Tests: Task 12 (`a_broken_settings_file_falls_back_to_defaults`), Task 13
   (`a_broken_settings_file_keeps_the_built_in_words`).
3. **In-world text at the edges.** Inputs:
   - a developer word inside a link target, in shown text, or in a bare link;
   - a design note inside a voice block, or a caption;
   - a voice block inside an in-world topic (one finding, never two);
   - non-ASCII text before the word, so columns count Unicode scalar values.

   Tests: Task 3 (`segments_*`), Task 13 (`an_occurrence_is_reported_once`,
   `columns_count_unicode_scalar_values`).
4. **A duplicated truth or clue id.** Expected: a clue revealing a duplicated truth id is judged
   against every record with that id, and a guardrail fires if any of them breaks it. The ledger
   of each truth record lists the same clues. Test: Task 9
   (`a_duplicated_truth_id_fires_if_any_breaks_it`).
5. **Binary paths that leave the folder.** Inputs: `../x.bin`, `/x.bin`, `C:/x.bin`,
   `\\server\x.bin` and `a/../../x.bin`. Expected: each gives `binary/missing` with its message,
   on every OS, and no file outside the folder is opened (a sentinel file next to the folder
   stays unread). Test: Task 15 (`paths_that_leave_the_folder_are_never_opened`).

## Rulings made while planning

These go into the ledger's "Rulings (build)" and the PR, in this form.

- L1. Ruling: AC tests are plain `#[test]` fns in P03's integration-test crate `tests/index/`, and
  in `crates/theos/tests/cli.rs` for the CLI — `ac-trace` needs the fn name, and one crate shares
  its helpers — cost if wrong: none.
- L2. Ruling: P04's fixture ids are `synthetic-…` slugs and numbers from 950 up (`T-950`, `C-950`,
  `S-950`) — distinct from P02's corpus and P03's 930s — cost if wrong: none.
- L3. Ruling: the rule table is `theos_content::rules::all(registry)`, one `RuleInfo` per id; P02's
  and P03's rules are fixed — one place answers "what rules exist" for the fingerprint, the
  settings and P05 — cost if wrong: none.
- L4. Ruling: `rules::effective` ignores a configured value for guardrails and fixed rules, also
  when the settings never store one — the safe side for G8, whatever builds the configuration —
  cost if wrong: none.
- L5. Ruling: a lore rule is `fn(&mut Context<'_>, &Subject) -> Result<Vec<Finding>, FactError>`,
  and `FactError` wraps only SQLite errors — rules cannot fail on content, only on the store —
  cost if wrong: none.
- L6. Ruling: read keys are stored as text `rec:<id>`, `in:<id>` and `file:<path>` in
  `deps(subject, key)`, indexed by key — one lookup finds the dirty subjects — cost if wrong:
  none.
- L7. Ruling: a re-parse, an addition or a removal changes `rec:` of the old and the new id, and
  `in:` of every target of its old and new link rows — coarse, simple and complete — cost if
  wrong: more re-runs after an edit than needed.
- L8. Ruling: the lore step loads all facts in bulk when more than 32 subjects are dirty, and
  fetches by id otherwise. Task 18 may retune the number from measurements — a cold build must
  not query per read, and a one-file change must not load the project — cost if wrong: one
  constant.
- L9. Ruling: an input file's subject has its project-relative path and no id, and its findings
  have no id — spec §5.5 — cost if wrong: none.
- L10. Ruling: the lore step writes a truth's ledger rows and the settings' configured
  severities itself, after the subject's rules — they are outputs of a subject, not findings —
  cost if wrong: none.
- L11. Ruling: body rules run with `BodyGrammar::PROSE` for a record whose kind is unknown (its
  front matter failed, or `schema/kind` failed) — links, embeds and voice titles mean something
  in every body — cost if wrong: findings in a body whose kind is not known yet.
- L12. Ruling: the type of a malformed callout title is the type of the callout whose title span
  holds the item — P02's `Malformed` does not carry it, and its callout list does — cost if
  wrong: none.
- L13. Ruling: binaries are hashed in 64 KiB chunks with `Read::read` and `Sha256::update` —
  `sha2` 0.11 has no `io::Write` (probe) — cost if wrong: none.
- L14. Ruling: the settings are read from `toml::de::DeTable::parse` and walked by hand — serde
  derive would stop at the first unknown key without its position (probe) — cost if wrong: a
  few more lines than derive.
- L15. Ruling: a binary path is checked on its segments split at `/` and `\`. It leaves the
  folder when it is empty, starts with a separator, has a `:` in its first segment, or has a
  `..` segment. The test is the same on every OS — Windows prefixes then mean the same on
  Linux —
  cost if wrong: a file name with `:` in its first folder is refused.
- L16. Ruling: `_source.json` may carry a top-level `"note"` string, which the reader ignores. Any
  other unknown key is `formalism/source` — the synthetic fixture must say it is synthetic, and
  JSON has no comments — cost if wrong: one optional key in the format.
- L17. Ruling: an input file that is not UTF-8 is stored with its hash and no text. Its subject
  reports `settings/toml` or `formalism/source` at 1:1 — one fault, one finding — cost if wrong:
  none.
- L18. Ruling: `theos` returns an `ExitCode`. Output goes through `writeln!` to a locked stdout,
  and errors through `anyhow`'s `{:#}` to stderr. Exit 1 on any error — the workspace denies
  `print!` and `exit` — cost if wrong: none.
- L19. Ruling: an equation's body is verbatim: P03's rows store no body links for it (no
  mentions), the record step runs no body rule on it, and it has no in-world segments — spec
  §5.14 — cost if wrong: a link in an equation's text is not resolved.
- L20. Ruling: in-world segments store their text, and the position of a match is the segment's
  start plus the Unicode scalar values before it (line and column) — the lore step has no record
  text — cost if wrong: none.
- L21. Ruling: `Update.revalidated` is sorted and holds every subject the lore step re-ran that is
  neither in `reparsed` nor in `relinked`. A changed input file is listed there — spec §5.5 —
  cost if wrong: P06 logs an input file among records.
- L22. Ruling: the rule set fingerprint hashes lines `rule<TAB>severity<TAB>class` — a class
  change must rebuild old caches — cost if wrong: none.
- L23. Ruling: the ledger stores `placed_by` as rows of its own (`ledger_beats`), not as a joined
  text — queries and dumps stay plain — cost if wrong: none.
- L24. Ruling: `binary_findings` returns a `BinaryReport` with the findings and the paths it
  hashed — spec §5.18 asks that a second call hash nothing — cost if wrong: one public field.
- L25. Ruling: the binary hash memo is keyed by the binaries folder as given, the path, the size
  and the time, and is never part of `dump()` — spec §5.13 — cost if wrong: a moved folder
  hashes
  once more.
- L26. Ruling: `theos ledger` separates truths by one empty line, prints floors as `F` and at
  least two digits (`F04`, `F100`), `F?` without a floor, and `T?` without a valid gate — spec
  §5.15 leaves the layout to the builder — cost if wrong: a layout P05 reshapes.
- L27. Ruling: requires (b) gives one finding per `requires` item. It names the first clue by
  floor, gate and id for which no confirming clue of the required truth can be found no later —
  one fault, one finding — cost if wrong: other failing clues show only after the first is fixed.

## File structure

```text
crates/theos-content/
  Cargo.toml                       + serde (derive), serde_json (Task 14)
  README.md                        what the crate provides after P04 (Task 18)
  src/
    lib.rs                         + pub mod rules, lore, settings, formalism; re-exports
    rules/mod.rs                   RuleClass, RuleInfo, all, effective, GUARDRAILS (Task 1)
    check/body/mod.rs              BodyInput, BodyRule, BODY_RULES (Task 2)
    check/body/{malformed,tier,when,attribute_count,attribute_duplicate,attribute_unknown,
                beat_number,note_id,note_state}.rs                        (Task 2)
    inworld.rs                     in-world segments of a record (Task 3)
    kinds/spec.rs                  BodyGrammar: + beat_tokens, verbatim; VERBATIM (Task 2)
    kinds/mod.rs                   + register_rule, lore_rules, RegistryError::DuplicateRule (4)
    lore/mod.rs                    LoreRule, On, Subject (Task 4)
    lore/facts.rs                  RecordFacts, FactValue, FactLink, FactBeat, FactNote,
                                     Segment, InLink, InputFacts (Task 4)
    lore/context.rs                Context, ReadKey, FactSource, FactError (Task 4)
    lore/places.rs                 Floor, floor_of, inside (Task 6)
    lore/availability.rs           Tier, State, Availability, no_later (Task 6)
    lore/cycle.rs                  cycle_through (Task 6)
    lore/lens.rs                   rejects (Task 6)
    lore/ledger.rs                 Ledger, LedgerRow, of_truth (Task 8)
    lore/reveals.rs                truths a clue reveals, clues of a truth (Task 9)
    lore/vocab.rs                  DeveloperWords, BUILTIN (Task 13)
    settings/mod.rs                SETTINGS, ValidationSettings, Allowed, Parsed, parse (Task 12)
    settings/rules.rs              RULES: the settings/* rules (Task 12)
    formalism/{mod,source,render,regenerate}.rs                           (Task 14)
    parts/codex/rules/{mod,floor_number,parent_cycle,never_confirm,no_overshoot,reveal_order,
                       requires,three_clue,inworld_vocab,formalism_edited,formalism_stray,
                       formalism_missing,formalism_source}.rs             (Tasks 7–14)
    parts/scripts/rules/{mod,beat_at,says_rejected,reveal_where,reveal_gate}.rs (Task 11)
    parts/version_rules/{mod,picked,from,addresses}.rs                    (Task 7)
    index/schema.sql               + notes, inworld, inputs, deps, ledger, ledger_beats,
                                     configured, binary_memo; beats.when_token (Tasks 3–15)
    index/rows.rs                  + NoteRow, SegmentRow, BeatRow.when (Task 3)
    index/inputs.rs                INPUTS, reading and storing input files (Task 5)
    index/lore_source.rs           the SQL FactSource: by id and in bulk (Task 4)
    index/lore_step.rs             changed keys, dirty set, run, write (Task 4)
    index/ledger.rs                Index::ledger, ledger_of, TruthLedger, LedgerEntry (Task 8)
    index/binaries.rs              Index::binary_findings, BinaryReport (Task 15)
    index/lens.rs                  LensSource, shared chain and layer reading (Task 6)
    index/{sync,write,dump,store,query,identity,mod}.rs                   (Tasks 1–15)
  tests/
    index/{body,inworld,lore,inputs,world,versions,ledger,guardrails,order,story,
           settings,vocab,formalism,binaries,neighbours,rules}.rs
    fixtures/projects/{body,lore,world,story,versions,settings,formalism,binaries}/
    fixtures/binaries/             the binaries folder of `binaries/` (Task 15)
crates/theos/
  Cargo.toml                       + clap, anyhow, theos-content (Task 16)
  src/{main.rs,ledger.rs,formalism.rs}                                    (Task 16)
  tests/cli.rs                                                            (Task 16)
```

---

### Task 1: Rule classes, the rule table and effective severities

**Files:**
- Create: `src/rules/mod.rs`
- Modify: `src/lib.rs` (`pub mod rules`), `src/index/identity.rs` (`rule_set(registry)`, the
  fingerprint lines with the class)

**Interfaces — produces:**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuleClass { Guardrail, Configurable, Fixed }
impl RuleClass { pub fn as_str(self) -> &'static str }   // "guardrail" | "configurable" | "fixed"
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleInfo { pub id: &'static str, pub severity: Severity, pub class: RuleClass }
impl RuleInfo { pub fn group(&self) -> &'static str }    // the text before '/'
pub const GUARDRAILS: [&str; 3] = ["lore/never-confirm", "lore/no-overshoot", "lore/inworld-vocab"];
pub fn all(registry: &KindRegistry) -> Vec<RuleInfo>;    // sorted by id, one per id
pub fn effective(info: &RuleInfo, configured: Option<Severity>) -> Severity;
// identity.rs
pub(crate) fn rule_set(registry: &KindRegistry) -> Vec<RuleInfo>;   // = rules::all
```

- `all` lists `check::rules()` and `index::resolve::rule_set()` as `Fixed`. Tasks 2, 4, 12 and 15
  add their lists to it.
- `effective` returns `Error` for a guardrail, `configured.unwrap_or(info.severity)` for a
  configurable rule, and `info.severity` for a fixed rule.

- [ ] **Step 1: Write the failing tests** in `src/rules/mod.rs`:
  - `// AC: CE-14.1` `guardrails_are_errors_whatever_they_are_given`: a guardrail `RuleInfo` with
    default `Error`, and one with default `Warning`; for `None`, `Some(Error)` and
    `Some(Warning)`, `effective` is `Error`.
  - `configurable_rules_take_their_setting`: default `Error`; `Some(Warning)` gives `Warning`,
    and `None` gives `Error`.
  - `fixed_rules_keep_their_default`: `Some(Warning)` on a fixed `Error` rule gives `Error`.
  - `p02_and_p03_rules_are_fixed_and_listed_once`: `all(&parts::builtin()?)` holds
    `link/dangling` (fixed, error) and `schema/unknown-field` (fixed, warning); the ids are
    sorted and unique.
  - In `identity.rs`: `the_rules_fingerprint_changes_with_a_class`: two rule lists that differ
    only in one rule's class give different fingerprints.
- [ ] **Step 2: Run** `cargo nextest run -p theos-content --locked rules::` — FAIL (no module).
- [ ] **Step 3: Implement** `src/rules/mod.rs` and the identity change. `Identity::current`
  passes its registry to `rule_set`.
- [ ] **Step 4: Run** the per-task checks — PASS.
- [ ] **Step 5: Commit** `feat(content): rule classes and effective severities` with
  `Refs: CE-14.1`.

### Task 2: Body rules in the record step

**Files:**
- Create: `src/check/body/mod.rs` and one module per rule (see File structure)
- Modify: `src/kinds/spec.rs` (`BodyGrammar`), `src/parts/scripts/script.rs` (`beat_tokens:
  &["when"]`), `src/parts/concepts/concept.rs`, `src/parts/maps/map.rs` (the new fields),
  `src/parts/codex/equation.rs` (`BodyGrammar::VERBATIM`), `src/check/mod.rs` (run the body
  rules, list them in `rules()`), `src/index/rows.rs` (no body links for a verbatim grammar)
- Create: `tests/fixtures/projects/body/`, `tests/index/body.rs`
- Modify: P03's expectation files (Build rules)

**Interfaces — produces:**

```rust
pub struct BodyGrammar { pub beats: Option<&'static [BeatAttribute]>,
    pub beat_tokens: &'static [&'static str], pub notes: Option<&'static [Target]>,
    pub verbatim: bool }
impl BodyGrammar { pub const PROSE: BodyGrammar; pub const VERBATIM: BodyGrammar; }
// check/body/mod.rs
pub(crate) struct BodyInput<'a> { pub record: &'a Record, pub body: &'a Body,
    pub grammar: &'a BodyGrammar }
pub(crate) struct BodyRule { pub id: &'static str, pub severity: Severity,
    pub run: fn(&BodyInput<'_>) -> Vec<Problem> }
pub(crate) const BODY_RULES: &[BodyRule];   // the 15 rules of spec §5.3, in its table's order
```

- `malformed.rs` holds one `BodyRule` per `MalformedKind`. The id is `body/<kind.as_str()>`.
  Each keeps only the items its kind means in this grammar:
  - links and embeds: always;
  - a callout title: a voice title always, a note title when `grammar.notes` is set (L12);
  - beat forms: when `grammar.beats` is set.
- The other rules read P02's beats (`Beat.attributes`), callouts (`CalloutTitle::Voice`,
  `CalloutTitle::Note`) and `TIERS`. Positions are those of spec §5.3.
- `body/attribute-unknown` knows the keys `grammar.beats` (link attributes) ∪
  `grammar.beat_tokens`.
- `check_record` runs `BODY_RULES` after the kind's checks, with the kind's grammar, or `PROSE`
  when the kind is unknown (L11). It runs none when the grammar is `verbatim` (L19). Findings
  have the step `record`.

- [ ] **Step 1: Write the fixture** `tests/fixtures/projects/body/` (own `theos.toml`):
  - a script `story/scripts/S-950.md` with:
    - `when:: T4`;
    - two `at::` links;
    - `with::` twice in one beat;
    - an unknown key `synthetic::`;
    - two beats numbered 2;
    - `## Beat 3 - x` (malformed);
    - a malformed attribute line;
  - a concept `concepts/synthetic-board.md` with notes `n1` twice, a note `· closed`, and a
    malformed note title;
  - an explaining topic with a beat heading, a malformed beat heading, and a note callout with an
    unknown state (no findings: its grammar has neither), plus `[[a b]]`, `![[x]]`, a voice
    block with tier `T4`, and a malformed voice title;
  - an equation `canon/formalism/E950.md`, whose body has `[[a b]]` and `![[x]]` (verbatim: no
    body finding);
  - the records all of these links name, so that no `link/dangling` clutters the list;
  - `expected.toml`, every finding checked by hand.
- [ ] **Step 2: Write the failing tests** in `tests/index/body.rs`:
  - `// AC: CE-14.2` `body_findings_carry_rule_severity_path_and_position`: the project's
    findings equal `expected.toml`.
  - `// AC: CE-15.5` `an_unknown_note_state_is_an_error`: `body/note-state`, severity error, at
    the line and column of `closed`.
  - `constructs_without_meaning_in_a_kind_give_no_finding`: the topic has no `body/beat-*` and no
    note finding.
  - `a_verbatim_body_is_not_judged`: the equation has no `body/` finding and no `mention` link.
- [ ] **Step 3: Run** `cargo nextest run -p theos-content --locked body` — FAIL.
- [ ] **Step 4: Implement** the grammar fields, the modules, the call in `check_record`, and
  `rules()`. Then run P03's tests: list every new finding in P03's expectation files after
  checking it. The `malformed/` corpus records gain their `body/` findings.
- [ ] **Step 5: Run** the per-task checks — PASS.
- [ ] **Step 6: Commit** `feat(content): judge body data and malformed constructs per kind` with
  `Refs: CE-14.2, CE-15.5`.

### Task 3: Rows for the lore step: notes, `when::` and in-world segments

**Files:**
- Create: `src/inworld.rs`
- Modify: `src/index/rows.rs`, `src/index/write.rs`, `src/index/dump.rs`, `src/index/store.rs`
  (`TABLES`), `src/index/schema.sql`, `src/index/identity.rs` (`FORMAT = 3`)
- Create: `tests/index/inworld.rs`

**Interfaces — produces:**

```rust
pub(crate) struct NoteRow { pub id: String, pub number: u32,
    pub state: String,            // "open" | "dropped" | "resolved" | the unknown word
    pub resolved_by: Option<String>, pub version: Option<String>,
    pub span: Span, pub position: Position }          // span and position of the note id
pub(crate) struct SegmentRow { pub span: Span, pub position: Position, pub text: String }
// BeatRow gains: pub when: Option<String>   — the first `when::` token, as written
// RecordRows gains: pub notes: Vec<NoteRow>, pub inworld: Vec<SegmentRow>
pub(crate) fn segments(record: &Record, spec: Option<&KindSpec>, values: &[ValueRow])
    -> Vec<SegmentRow>;                                 // src/inworld.rs
```

```sql
CREATE TABLE notes (path TEXT NOT NULL, id TEXT NOT NULL, number INTEGER NOT NULL,
                    state TEXT NOT NULL, resolved_by TEXT, version TEXT,
                    span_start INTEGER NOT NULL, span_end INTEGER NOT NULL,
                    line INTEGER NOT NULL, col INTEGER NOT NULL);
CREATE TABLE inworld (path TEXT NOT NULL, span_start INTEGER NOT NULL, span_end INTEGER NOT NULL,
                      line INTEGER NOT NULL, col INTEGER NOT NULL, text TEXT NOT NULL);
-- beats gains: when_token TEXT   (`when` is an SQL keyword)
```

- Notes come only from kinds whose grammar has notes.
- **Segments** follow spec §5.11 exactly:
  - Included spans:
    - every voice callout's `content_span`, also nested, in every kind with a non-verbatim
      grammar;
    - the whole body of a topic with a valid `track: in-world`, and of a dialogue;
    - the `title` value of a belief.
  - Holes cut out of them:
    - design notes (both forms);
    - voice titles (callout start to `content_span.start`);
    - level-2 headings of a dialogue (ATX `## `, or setext with a `-` underline);
    - link markup. For a link with `shown_span`, everything from `span.start` to
      `shown_span.start`, and the closing `]]`. For a bare link, only `[[` and `]]`, so the id
      stays.
  - The result is sorted, merged and non-empty, with each segment's text and the position of its
    start.
- `write`, `delete` and `dump` cover the new tables. `dump` gets the sections `notes` and
  `inworld`, and `beats` shows `when_token`.

- [ ] **Step 1: Write the failing tests** in `src/inworld.rs` (units over parsed records):
  - `segments_hold_voice_content_and_caption_but_not_the_title`;
  - `segments_skip_design_notes_inside_a_voice_block`;
  - `segments_cover_an_in_world_topic_body_once`: a voice block inside it gives no second segment
    over the same bytes;
  - `segments_hold_a_belief_title`;
  - `segments_skip_dialogue_level_two_headings`;
  - `segments_keep_shown_text_and_bare_ids_but_not_link_markup`;
  - `a_verbatim_body_has_no_segments`.

  In `src/index/rows.rs`: `notes_rows_come_only_from_kinds_with_notes` and
  `a_beat_keeps_its_when_token`.

  In `tests/index/inworld.rs`: `the_dump_holds_notes_inworld_and_when` on a small temp project.
- [ ] **Step 2: Run** them — FAIL.
- [ ] **Step 3: Implement.** Bump `FORMAT` to 3.
- [ ] **Step 4: Run** the per-task checks — PASS. P03's history test must stay green.
- [ ] **Step 5: Commit** `feat(content): store notes, when tokens and in-world segments`.

### Task 4: The lore step: subjects, the context and stored reads

**Files:**
- Create: `src/lore/{mod,facts,context}.rs`, `src/index/lore_source.rs`, `src/index/lore_step.rs`
- Modify: `src/kinds/mod.rs` (`register_rule`, `lore_rules`, `DuplicateRule`, fingerprint),
  `src/rules/mod.rs` (registered rules in `all`), `src/index/sync.rs` (collect keys; run the
  step after `resolve`; `rebuild` runs it too), `src/index/mod.rs` (`Update.revalidated`,
  `Preview`), `src/index/{schema.sql,write.rs,dump.rs,store.rs}` (`deps`), `src/lib.rs`
  (`pub mod lore`)
- Create: `tests/index/lore.rs`
- Modify: `tests/index/changes.rs` and P03's other tests that build `Update` values or collect
  preview paths (`revalidated`)

**Interfaces — produces:**

```rust
// lore/mod.rs
pub struct LoreRule { pub id: &'static str, pub severity: Severity, pub class: RuleClass,
    pub on: On, pub run: fn(&mut Context<'_>, &Subject) -> Result<Vec<Finding>, FactError> }
impl fmt::Debug for LoreRule        // id, severity, class, on — never the fn address
pub enum On { Kinds(&'static [&'static str]), AnyRecord, Input(&'static str) }
pub enum Subject { Record(Rc<RecordFacts>), Input(Rc<InputFacts>) }
impl Subject {
    pub fn path(&self) -> &str;
    pub fn finding(&self, rule: &LoreRule, span: Span, position: Position, message: String)
        -> Finding;                  // rule.id, rule.severity, the subject's path and id
}
// lore/facts.rs
pub struct RecordFacts { pub path: String, pub id: String, pub kind: Option<String>,
    pub hash: String, pub values: Vec<FactValue>, pub links: Vec<FactLink>,
    pub beats: Vec<FactBeat>, pub notes: Vec<FactNote>, pub inworld: Vec<Segment> }
impl RecordFacts {
    pub fn text(&self, field: &str) -> Option<&FactValue>;   // the valid top-level scalar
    pub fn items(&self, field: &str) -> Vec<&FactValue>;     // the valid items of a list field
    pub fn value_at(&self, field: &str) -> Option<(Span, Position)>; // the top-level node, valid or not
    pub fn links_in<'a>(&'a self, slot: &'a Slot) -> impl Iterator<Item = &'a FactLink>;
}
pub struct FactValue { pub field: String, pub entry: Option<String>, pub item: Option<usize>,
    pub text: Option<String>, pub span: Span, pub position: Position, pub valid: bool }
pub struct FactLink { pub slot: Slot, pub beat: Option<u32>, pub target: String,
    pub fragment: Option<String>, pub span: Span, pub position: Position }
pub struct FactBeat { pub number: u32, pub when: Option<String> }
pub struct FactNote { pub id: String, pub state: String, pub span: Span, pub position: Position }
pub struct Segment { pub span: Span, pub position: Position, pub text: String }
pub struct InLink { pub source_id: String, pub source_path: String, pub slot: Slot,
    pub beat: Option<u32>, pub span: Span, pub position: Position }
pub struct InputFacts { pub path: String, pub hash: String, pub text: Option<String> }
// lore/context.rs
pub struct Context<'a> { /* source, registry, reads, memo */ }
impl<'a> Context<'a> {
    pub fn registry(&self) -> &KindRegistry;
    pub fn records(&mut self, id: &str) -> Result<Vec<Rc<RecordFacts>>, FactError>; // rec:<id>, by path
    pub fn linking_to(&mut self, id: &str) -> Result<Vec<InLink>, FactError>;      // in:<id>
    pub fn input(&mut self, path: &str) -> Result<Option<Rc<InputFacts>>, FactError>; // file:<path>
}
pub(crate) enum ReadKey { Record(String), Inbound(String), File(String) }
impl ReadKey { pub(crate) fn as_db(&self) -> String }   // rec:… | in:… | file:…
pub(crate) trait FactSource {
    fn records(&mut self, id: &str) -> Result<Vec<RecordFacts>, FactError>;
    fn linking_to(&mut self, id: &str) -> Result<Vec<InLink>, FactError>;
    fn input(&mut self, path: &str) -> Result<Option<InputFacts>, FactError>;
}
#[derive(Debug, thiserror::Error)] pub enum FactError { Sqlite(#[from] rusqlite::Error) }
// kinds/mod.rs
impl KindRegistry {
    pub fn register_rule(&mut self, part: &'static str, rule: LoreRule) -> Result<(), RegistryError>;
    pub fn lore_rules(&self) -> impl Iterator<Item = &LoreRule>;
}
// RegistryError::DuplicateRule { rule, first, second } when an id comes again with another
// severity or class; the same id, severity and class from another part is allowed (spec §5.2).
// index/mod.rs
pub struct Update { /* P03's lists */ pub revalidated: Vec<String> }
// index/lore_step.rs
pub(crate) struct LoreWork { pub changed: BTreeSet<String>, pub subjects: BTreeSet<String>,
    pub removed: BTreeSet<String> }
pub(crate) fn run(conn: &Connection, registry: &KindRegistry, work: &LoreWork)
    -> Result<BTreeSet<String>, IndexError>;   // the paths of every subject it re-ran
```

```sql
CREATE TABLE deps (subject TEXT NOT NULL, key TEXT NOT NULL);
CREATE INDEX deps_key ON deps(key);
CREATE INDEX deps_subject ON deps(subject);
```

- **The Syncer** gains a `LoreWork`. `settle`, `remove`, `not_utf8` and `unreadable` add the
  keys of L7. For the old link targets, they read `SELECT DISTINCT target FROM links WHERE path
  = ?1` before the rows change. Re-parsed paths go into `subjects`, removed paths into
  `removed`.
- **`run`:**
  - The dirty set is `subjects` ∪ `SELECT DISTINCT subject FROM deps WHERE key IN changed`,
    minus `removed`.
  - Removed subjects lose their `deps` and `lore` findings. `write::delete_all` covers that.
  - Each dirty subject:
    1. delete its `findings` with step `lore` and its `deps`;
    2. build a `Context` over the source (bulk when more than 32 are dirty, L8);
    3. run every rule of `registry.lore_rules()` whose `On` matches. A record matches
       `Kinds(kinds)` when its kind is in the list, and `AnyRecord` always. An input matches
       `Input(path)` with its path;
    4. insert the findings with step `lore` and the reads as `deps` rows.
- `Update.revalidated` = the re-run paths minus `reparsed` and `relinked` (L21).
  `Preview.findings` covers `revalidated` too.
- `dump` gets a `deps` section (`subject`, `key`). The registry fingerprint covers the rules
  (their `Debug`).

- [ ] **Step 1: Write the failing tests** in `tests/index/lore.rs`. They use a registry of
  `parts::builtin()` plus a test rule `test/probe` (`On::Kinds(&["thing"])`). The rule reads
  `ctx.records(<the thing's type value>)` and reports `test/probe` when no record has that id:
  - `// AC: CE-14.3` `a_change_revalidates_the_records_that_read_it`: thing A reads B. Editing
    B's title re-runs A; `Update.revalidated == [A]`; `preview` of the edit returns A's
    findings.
  - `a_renamed_or_removed_dependency_reruns_its_readers`: renaming B gives A the finding, and
    renaming B back removes it. Deleting B gives the finding, and re-adding B removes it. After
    each change, `dump()` equals a cold build.
  - `a_record_reading_a_missing_id_reruns_when_it_appears`.
  - `a_removed_subject_leaves_no_reads_or_findings`: after deleting A, `deps` and the `lore`
    findings hold nothing for A's path.
  - `untouched_records_are_not_rerun`: editing an unrelated record leaves `revalidated` empty.
  - `deps_of_a_cold_build_equal_those_of_a_history`.
  - `a_rule_id_twice_with_another_class_is_refused` (unit test in `kinds/mod.rs`).
- [ ] **Step 2: Run** them — FAIL.
- [ ] **Step 3: Implement.** Update P03's tests for `revalidated`: the `lists` helper and the
  `previewed` path set in `changes.rs`. With no lore rule registered yet, every P03 expectation
  of `revalidated` is empty.
- [ ] **Step 4: Run** the per-task checks — PASS, P03's history test included.
- [ ] **Step 5: Commit** `feat(content): run checks across records with stored reads` with
  `Refs: CE-14.3`.

### Task 5: Input files

**Files:**
- Create: `src/index/inputs.rs`, `tests/index/inputs.rs`
- Modify: `src/index/{sync.rs,schema.sql,write.rs,dump.rs,store.rs}`,
  `src/index/lore_source.rs` (`input`)

**Interfaces — produces:**

```rust
pub(crate) const INPUTS: [&str; 2] = [".theos/settings.toml", "canon/formalism/_source.json"];
pub(crate) fn is_input(path: &str) -> bool;
```

```sql
CREATE TABLE inputs (path TEXT PRIMARY KEY, size INTEGER, mtime_ns INTEGER, racy INTEGER NOT NULL,
                     hash TEXT NOT NULL, text TEXT) WITHOUT ROWID;
```

- `sync()` stats both inputs after the walk, with P03's racy rule. When the hash differs, the
  file appeared or the file went, it writes the row (`text` NULL when not UTF-8, L17), adds
  `file:<path>` to `changed` and the path to `subjects`, and lists the path in `read`.
- `update`, `apply` and `preview` take an input path like a record path: from disk, given
  bytes, or removed. `preview` reports the input subject's findings.
- `dump` gets an `inputs` section with path and hash.

- [ ] **Step 1: Write the failing tests** in `tests/index/inputs.rs`, with a test rule on
  `On::Input(".theos/settings.toml")` that reports when the text contains `synthetic-bad`:
  - `a_changed_settings_file_is_read_at_the_next_sync`;
  - `an_unchanged_input_is_not_read_again`;
  - `inputs_are_changes_for_update_apply_and_preview`;
  - `a_removed_input_loses_its_row_and_findings`;
  - `a_non_utf8_input_is_stored_without_text`;
  - `a_racy_input_is_read_again`.
- [ ] **Step 2: Run** them — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the per-task checks — PASS.
- [ ] **Step 5: Commit** `feat(content): track the settings and formalism source as inputs`.

### Task 6: Places, availability, cycles and the shared lens chain

**Files:**
- Create: `src/lore/{places,availability,cycle,lens}.rs`
- Modify: `src/index/lens.rs` (`LensSource`; P03's `lens` keeps its behaviour),
  `src/test_support.rs` (`with_context`)

**Interfaces — produces:**

```rust
// places.rs
pub struct Floor { pub number: u32, pub place: String }
pub fn floor_of(ctx: &mut Context<'_>, place: &str) -> Result<Option<Floor>, FactError>;
pub fn inside(ctx: &mut Context<'_>, inner: &str, outer: &str) -> Result<bool, FactError>;
// availability.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)] pub enum Tier { T1, T2, T3 }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum State { Hinted, Suspected, Confirmed }
impl Tier { pub fn parse(text: &str) -> Option<Tier> }
impl State { pub fn parse(text: &str) -> Option<State> }      // hinted | suspected | confirmed
pub struct Availability { pub floor: Option<Floor>, pub gate: Option<Tier> }
pub fn of_clue(ctx: &mut Context<'_>, clue: &RecordFacts) -> Result<Availability, FactError>;
pub fn no_later(a: &Availability, b: &Availability) -> Option<bool>; // None when a part is unknown
// cycle.rs
pub fn cycle_through(ctx: &mut Context<'_>, start: &RecordFacts, field: &str)
    -> Result<Option<usize>, FactError>;  // index of start's `field` link leading into a cycle back to start
// lens.rs (lore)
pub fn rejects(ctx: &mut Context<'_>, holder: &str) -> Result<Option<Vec<String>>, FactError>;
// index/lens.rs
pub(crate) trait LensSource { type Error;
    fn holder(&mut self, id: &str, kinds: &[String]) -> Result<Option<Layer>, Self::Error>;
    fn parent(&mut self, path: &str) -> Result<Option<String>, Self::Error>;
    fn lens_nodes(&mut self, path: &str) -> Result<Vec<Node>, Self::Error>; }
```

- `floor_of` follows spec §5.6. It walks `parent` links (`field:parent`) of floors and
  locations through `ctx.records` until a floor, or a band (its first `floors` item). It stops at
  a dangling or wrong-kind link, an invalid value or a repeat.
- `inside` is true when `inner == outer` or `outer` is on `inner`'s parent chain.
- `of_clue` takes the lowest floor among the clue's valid `where` items, and its `gate`.
- `cycle_through` walks valid `field` links depth-first and returns the index of the first link
  of `start` that leads back to `start`.
- P03's `chain`, and its reading of one layer's lens nodes, become generic over `LensSource`.
  `Index` implements it with SQL, `Context` with recorded reads. `rejects` is the merged
  `rejects` entry of the chain. The one implementation serves both (spec §5.5).

- [ ] **Step 1: Write the failing tests** as unit tests in the four modules. They use
  `crate::test_support::with_context(files: &[(&str, &str)], f: impl FnOnce(&mut Context<'_>)
  -> R) -> Result<R, Box<dyn Error>>`, which indexes the files in memory and runs `f` on a
  context over the store. No public test API is added. The files hold a tower, two bands,
  floors, a location under a location, a dangling parent, a cycle, and holders with `rejects`
  on several layers.
  - `a_band_counts_as_its_first_floor`;
  - `a_location_takes_the_floor_of_its_parent_chain`;
  - `a_broken_or_looping_chain_has_no_floor`;
  - `inside_follows_the_parent_chain`;
  - `availability_is_the_lowest_floor_and_the_gate`;
  - `no_later_needs_floor_and_gate`: `Some(true)` for (4, T1) against (21, T2); `Some(false)`
    for (21, T1) against (4, T2) and for (4, T3) against (21, T2); `None` when a floor is
    unknown;
  - `a_cycle_is_found_from_each_member_only`;
  - `rejects_equals_the_effective_lens`: for every holder of the project, `lore::lens::rejects`
    equals the `rejects` of `Index::lens`.
- [ ] **Step 2: Run** them — FAIL.
- [ ] **Step 3: Implement**, the `LensSource` refactor first; P03's lens tests stay green.
- [ ] **Step 4: Run** the per-task checks — PASS.
- [ ] **Step 5: Commit** `feat(content): places, availability, cycles and one lens chain`.

### Task 7: Value checks across records

**Files:**
- Create: `src/parts/codex/rules/{mod,floor_number,parent_cycle}.rs`,
  `src/parts/version_rules/{mod,picked,from,addresses}.rs`
- Modify: `src/parts/codex/mod.rs`, `src/parts/concepts/mod.rs`, `src/parts/maps/mod.rs` (one
  `register_rule` line per rule), `src/parts/mod.rs`
- Create: `tests/fixtures/projects/{world,versions}/`, `tests/index/{world,versions}.rs`

**Interfaces — produces:** the five rules of spec §5.10:
- `world/floor-number`, `On::Kinds(&["floor"])`;
- `link/parent-cycle`, `On::Kinds(&["band", "floor", "location", "faction", "npc"])`, through
  `cycle_through(…, "parent")`;
- `version/picked`, registered by Concepts with `&["concept"]` and by Maps with `&["map"]`;
- `version/from` and `version/addresses`, registered by Concepts with `&["concept-version"]` and
  by Maps with `&["map-version"]`.

`version_rules` exports `pub(crate) const fn picked(on: &'static [&'static str]) -> LoreRule`, and
the same for `from` and `addresses`. All are fixed errors. `addresses` reads the parent's notes
(`FactNote.id`).

- [ ] **Step 1: Write the fixtures:**
  - `world/`: bands `[1, 20]` and `[21, 60]`; floors 4, 21 and one with number 25 under the first
    band; locations under locations; a two-location parent cycle; a faction–npc cycle.
  - `versions/`: concepts and maps with `picked` naming their own versions and another parent's;
    `from` inside and across parents; `addresses` naming existing and missing notes, and a
    version whose parent dangles.
  - `expected.toml` for each.
- [ ] **Step 2: Write the failing tests:**
  - `// AC: CE-14.2` `world_findings_equal_the_expectation`;
  - `// AC: CE-14.2` `version_findings_equal_the_expectation`;
  - `// AC: CE-15.5` `addresses_naming_a_missing_note_is_an_error`;
  - `a_version_whose_parent_dangles_is_not_judged`;
  - `// AC: CE-14.3` `changing_a_band_revalidates_its_floors`: a copy of `world/`; narrowing
    the first band's `floors` lists its floors in `revalidated`, and floor 4 gets
    `world/floor-number`;
  - `removing_a_note_revalidates_the_versions_that_address_it`.
- [ ] **Step 3: Run** them — FAIL.
- [ ] **Step 4: Implement**, then update P03's expectations if they change.
- [ ] **Step 5: Run** the per-task checks — PASS.
- [ ] **Step 6: Commit** `feat(content): check floors, versions, notes and parent cycles` with
  `Refs: CE-14.2, CE-14.3, CE-15.5`.

### Task 8: The ledger

**Files:**
- Create: `src/lore/ledger.rs`, `src/index/ledger.rs`
- Modify: `src/index/lore_step.rs` (store a truth subject's ledger), `src/index/{schema.sql,
  write.rs,dump.rs,store.rs,mod.rs}`, `src/lib.rs` (re-exports)
- Create: `tests/fixtures/projects/lore/` (its first records), `tests/index/ledger.rs`

**Interfaces — produces:**

```rust
// lore/ledger.rs
pub struct LedgerRow { pub clue: String, pub clue_path: String, pub state: State,
    pub availability: Availability, pub placed_by: Vec<(String, u32)> }  // (script id, beat)
pub struct Ledger { pub rows: Vec<LedgerRow> }   // in spec §5.7's order
pub fn of_truth(ctx: &mut Context<'_>, truth_id: &str) -> Result<Ledger, FactError>;
// index/ledger.rs
pub struct TruthLedger { pub id: String, pub path: String, pub title: Option<String>,
    pub target: Option<String>, pub entries: Vec<LedgerEntry> }
pub struct LedgerEntry { pub state: String, pub floor: Option<u32>, pub place: Option<String>,
    pub gate: Option<String>, pub clue: String, pub clue_path: String,
    pub placed_by: Vec<BeatRef> }
pub struct BeatRef { pub script: String, pub beat: u32 }
impl Index {
    pub fn ledger(&self) -> Result<Vec<TruthLedger>, IndexError>;        // every truth, by id, path
    pub fn ledger_of(&self, id: &str) -> Result<Vec<TruthLedger>, IndexError>; // one per record with id
}
```

```sql
CREATE TABLE ledger (path TEXT NOT NULL, clue TEXT NOT NULL, clue_path TEXT NOT NULL,
                     state TEXT NOT NULL, floor INTEGER, place TEXT, gate TEXT);
CREATE TABLE ledger_beats (path TEXT NOT NULL, clue_path TEXT NOT NULL, script TEXT NOT NULL,
                           beat INTEGER NOT NULL);   -- L23
```

- `of_truth` reads `in:<truth>`. The clues are the sources in slot `field:reveals` whose record
  is a clue with a valid `moves_to`. For each, it reads the availability and the beats from
  `ctx.linking_to(<clue>)` in slot `beat:reveals`.
- The lore step writes the rows for every truth subject (L10). A truth record without clues has
  an empty ledger.

- [ ] **Step 1: Write the fixture** `lore/`, its first part:
  - a tower, bands `[1, 20]` and `[21, 60]`, floors 4, 21 and 57, and a location under floor 4;
  - truth T-950 (target confirmed) with clues hinted on F04/T1, suspected on F21/T2 and
    confirmed on F57/T2;
  - a script S-950 whose beat 2 reveals the hinted clue;
  - truth T-951 without clues;
  - `expected-ledger.toml`, the expected `ledger()` as tables (L-style evidence).
- [ ] **Step 2: Write the failing tests** in `tests/index/ledger.rs`:
  - `// AC: CE-15.3` `the_ledger_is_built_from_clue_records`: `ledger()` equals the expectation.
  - `// AC: CE-15.3` `the_ledger_comes_from_the_cache`: a temp copy, opened twice; the second
    open reads no file (`Update.read` empty) and returns the same ledger.
  - `a_clue_change_updates_its_truths_ledger`: moving the suspected clue to F57 changes the row,
    and the truth is in `revalidated`.
  - `a_clue_without_a_floor_is_listed_without_one`.
- [ ] **Step 3: Run** them — FAIL.
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** the per-task checks — PASS.
- [ ] **Step 6: Commit** `feat(content): build the reveal ledger into the cache` with
  `Refs: CE-15.3`.

### Task 9: Guardrails never-confirm and no-overshoot

**Files:**
- Create: `src/lore/reveals.rs`, `src/parts/codex/rules/{never_confirm,no_overshoot}.rs`
- Modify: `src/parts/codex/mod.rs`, `src/parts/codex/rules/mod.rs`
- Extend: `tests/fixtures/projects/lore/`, its `expected.toml`; create `tests/index/guardrails.rs`

**Interfaces — produces:**

```rust
// lore/reveals.rs
pub fn truths_of(ctx: &mut Context<'_>, clue: &RecordFacts)
    -> Result<Vec<Rc<RecordFacts>>, FactError>;  // records of kind truth with the clue's `reveals` id
pub fn clues_of(ctx: &mut Context<'_>, truth_id: &str)
    -> Result<Vec<Rc<RecordFacts>>, FactError>;  // clues whose `reveals` names the id
```

- Both rules are `Guardrail`, `Error`, `On::Kinds(&["clue", "script"])`, as spec §5.8 says:
  - on a clue, at its `reveals` value (never-confirm) or its `moves_to` value (no-overshoot);
  - on a script, at each beat's `reveals::` link to such a clue.
- Messages as the spec's example: "C-952 confirms T-952, whose target is `never`: nothing may
  reveal it. Let a belief voice it instead."
- `ledger.rs` switches to `clues_of`, so the two share one reading.

- [ ] **Step 1: Extend the fixture:**
  - passing: T-950's clues and S-950 (nothing above target; no `never` truth);
  - failing:
    - truth T-952 (target `never`) with clue C-955 (`moves_to: hinted`);
    - truth T-953 (target `hinted`) with clue C-956 (`moves_to: suspected`);
    - script S-951 whose beats reveal C-955 and C-956;
    - a duplicated truth id T-954 in two folders, one `never` and one `confirmed`, and a clue
      revealing it.
- [ ] **Step 2: Write the failing tests** in `tests/index/guardrails.rs`:
  - `// AC: CE-15.1` `never_confirm_passes_and_fails`: the `lore/never-confirm` findings equal
    the expected subset (the clue at `reveals`, the beat at its link); the passing records have
    none.
  - `// AC: CE-15.1` `no_overshoot_passes_and_fails`: the same for `lore/no-overshoot`.
  - `a_duplicated_truth_id_fires_if_any_breaks_it`.
  - `guardrail_findings_are_errors`.
- [ ] **Step 3: Run** them — FAIL.
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** the per-task checks — PASS.
- [ ] **Step 6: Commit** `feat(content): guardrails never-confirm and no-overshoot` with
  `Refs: CE-15.1`.

### Task 10: Order rules reveal-order, requires and three-clue

**Files:**
- Create: `src/parts/codex/rules/{reveal_order,requires,three_clue}.rs`
- Modify: `src/parts/codex/mod.rs`, `src/parts/codex/rules/mod.rs`
- Extend: `tests/fixtures/projects/lore/`; create `tests/index/order.rs`

**Interfaces — produces:** three `Configurable` rules, as spec §5.8 says:
- `lore/reveal-order`: `Error`, `On::Kinds(&["clue"])`, parts (a), (b) (the nearest lower state)
  and (c).
- `lore/requires`: `Error`, `On::Kinds(&["truth"])`, parts (a), (b) (one finding per item, L27)
  and (c).
- `lore/three-clue`: `Warning`, `On::Kinds(&["truth"])`. The message is spec §5.8's, with
  "No clue", "Only one clue" or "Only two clues".

All use `availability::no_later` and skip a pair where it is `None`.

- [ ] **Step 1: Extend the fixture:**
  - passing: T-950's states in order and its three confirming clues added; T-950 requires T-957,
    which is confirmed on F04/T1;
  - failing:
    - C-960 with `after: [C-961]`, where C-961 sits on a later floor;
    - C-962 and C-963, each `after` the other;
    - truth T-958, suspected on F50 and confirmed on F21 (the spec's example);
    - T-959 requiring a `hinted` truth;
    - T-960 requiring a truth confirmed only on F57 while T-960 is confirmed on F21;
    - T-961 and T-962, each requiring the other;
    - T-963 with one confirming clue.
- [ ] **Step 2: Write the failing tests** in `tests/index/order.rs`:
  - `// AC: CE-15.1` `reveal_order_passes_and_fails`;
  - `// AC: CE-15.1` `requires_passes_and_fails`;
  - `// AC: CE-15.1` `three_clue_passes_and_fails`, with its message text;
  - `states_out_of_order_give_one_finding`: T-958 has exactly one `lore/reveal-order` finding,
    on the confirming clue;
  - `order_rules_skip_unknown_floors`.
- [ ] **Step 3: Run** them — FAIL.
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** the per-task checks — PASS.
- [ ] **Step 6: Commit** `feat(content): order rules reveal-order, requires and three-clue` with
  `Refs: CE-15.1`.

### Task 11: Story rules for beats

**Files:**
- Create: `src/parts/scripts/rules/{mod,beat_at,says_rejected,reveal_where,reveal_gate}.rs`
- Modify: `src/parts/scripts/mod.rs`
- Create: `tests/fixtures/projects/story/`, `tests/index/story.rs`

**Interfaces — produces:** the four fixed `Error` rules of spec §5.9, `On::Kinds(&["script"])`.
They read the script's facts (beats with `when`, links with their beat numbers and slots
`beat:at`, `beat:with`, `beat:says`, `beat:reveals`), `places::inside`, `lore::lens::rejects`
and the clues' `where` and `gate`.

- [ ] **Step 1: Write the fixture** `story/`. It holds places, holders with `rejects` on
  several layers, clues, and scripts with beats:
  - `at::` inside and outside `where`;
  - a script without `where`;
  - a `says::` belief rejected by one of two holders, and by both;
  - a `reveals::` clue whose `where` holds and misses `at::`;
  - `when::` above, equal to and below the clue's gate;
  - beats without `at::` or `when::`.

  Plus `expected.toml`.
- [ ] **Step 2: Write the failing tests:**
  - `// AC: CE-14.2` `story_findings_equal_the_expectation`;
  - `a_belief_rejected_by_one_of_two_holders_passes`;
  - `beats_without_at_or_when_skip_those_checks`.
- [ ] **Step 3: Run** them — FAIL.
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** the per-task checks — PASS.
- [ ] **Step 6: Commit** `feat(content): check beats against places, lenses and clues` with
  `Refs: CE-14.2`.

### Task 12: Validation settings and configured severities

**Files:**
- Create: `src/settings/{mod,rules}.rs`, `tests/fixtures/projects/settings/`,
  `tests/index/settings.rs`
- Modify: `src/lore/context.rs` (`settings`), `src/index/lore_step.rs` (configured rows; the
  settings rules), `src/index/query.rs` (apply severities on read; `findings_in` takes the
  registry), `src/index/sync.rs` (`preview`), `src/index/{schema.sql,write.rs,dump.rs,store.rs}`,
  `src/rules/mod.rs` (`settings::RULES` in `all`), `src/lib.rs`

**Interfaces — produces:**

```rust
// settings/mod.rs
pub const SETTINGS: &str = ".theos/settings.toml";
pub struct ValidationSettings { pub severities: BTreeMap<String, Severity>,
    pub words: Vec<String>, pub allow: Vec<Allowed> }
pub struct Allowed { pub word: String, pub reason: String }
pub struct SettingsProblem { pub rule: &'static str, pub span: Span, pub message: String }
pub struct Parsed { pub settings: ValidationSettings, pub problems: Vec<SettingsProblem> }
pub fn parse(text: &str, rules: &[RuleInfo]) -> Parsed;   // never fails: problems are data
// settings/rules.rs
pub(crate) const RULES: &[LoreRule];   // settings/toml, unknown-key, guardrail, fixed,
                                       // unknown-rule, severity, allow-reason, word — On::Input(SETTINGS)
// lore/context.rs
impl Context<'_> { pub fn settings(&mut self) -> Result<Rc<Parsed>, FactError> } // via input(SETTINGS), memoised
```

```sql
CREATE TABLE configured (rule TEXT PRIMARY KEY, severity TEXT NOT NULL) WITHOUT ROWID;
```

- `parse` walks `DeTable::parse(text)` (L14). It reads only `[validate]`, with the keys
  `severity` and `inworld-vocab` (`words`, `allow`). Every row of spec §5.12 becomes a
  `SettingsProblem` with its rule and span:
  - a problem has no effect on `settings`;
  - a guardrail never enters `severities`;
  - a parse error, or a text of `None` (L17), gives `settings/toml` at the error's position or
    1:1, and empty settings.
- Every settings rule reports the problems of its own id. They share one parse per run through
  the memo. Without a settings file, `settings()` gives empty settings and no problem.
- The lore step writes `configured` from the settings subject (L10), deleting it first. The
  severities apply in `findings`, `findings_for` and `preview` through `rules::effective`, with
  the class from `rules::all`.

- [ ] **Step 1: Write the fixture** `settings/`. It is one project whose `.theos/settings.toml`
  has a row of spec §5.12 each:
  - a guardrail set to `warning` and one set to `error`;
  - an unknown rule, a fixed rule, a value `off`, an unknown key;
  - an `allow` entry without `reason` and one with `reason = "  "`;
  - a word with a space;
  - `"lore/three-clue" = "error"`.

  It also has a truth with fewer than three clues and a never-confirm clue, and its own
  `expected.toml` (with configured severities).
- [ ] **Step 2: Write the failing tests** in `tests/index/settings.rs`:
  - `// AC: CE-14.1` `a_setting_that_lowers_a_guardrail_is_rejected_with_a_message`: the
    `settings/guardrail` finding has path `.theos/settings.toml`, line, column and the message
    of spec §5.12, and the never-confirm finding stays an error.
  - `// AC: CE-14.1` `an_order_rule_setting_changes_severity_on_read`: three-clue reads as
    error; `dump()` holds the default `warning`; removing the entry through `apply` makes it a
    warning again, and no record is re-parsed.
  - `// AC: CE-15.2` `an_allowlist_entry_without_a_reason_is_an_error`: both entries give
    `settings/allow-reason` at the entry.
  - `settings_findings_equal_the_expectation`.
  - `a_broken_settings_file_falls_back_to_defaults` (Review Focus 2).
  - `without_a_settings_file_the_defaults_apply`: no `settings/` finding, and three-clue reads
    as a warning.
  - Unit tests in `settings/mod.rs`, one per row of the table, and
    `guardrails_never_enter_the_severities`.
- [ ] **Step 3: Run** them — FAIL.
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** the per-task checks — PASS.
- [ ] **Step 6: Commit** `feat(content): validation settings and configured severities` with
  `Refs: CE-14.1, CE-15.2`.

### Task 13: inworld-vocab

**Files:**
- Create: `src/lore/vocab.rs`, `src/parts/codex/rules/inworld_vocab.rs`, `tests/index/vocab.rs`
- Modify: `src/parts/codex/mod.rs`, `src/parts/codex/rules/mod.rs`
- Extend: `tests/fixtures/projects/lore/` (in-world records, `.theos/settings.toml` with
  `words`); `settings/expected.toml` if it changes

**Interfaces — produces:**

```rust
pub(crate) const BUILTIN: &str = r"\b(?:[TBCSQ]-[0-9]{3,}|E[0-9]+|T[1-3]|f[0-9]{2}(?:-[a-z0-9]+)*|never-confirm|no-overshoot|inworld-vocab|reveal-order|three-clue)\b";
pub(crate) struct DeveloperWords { /* builtin, project, allow_exact, allow_folded */ }
impl DeveloperWords {
    pub(crate) fn new(settings: &ValidationSettings) -> Result<DeveloperWords, regex::Error>;
    pub(crate) fn find<'t>(&self, text: &'t str) -> Vec<(Range<usize>, &'t str)>; // sorted, no overlaps
}
```

- The project regex is `(?i)\b(?:w1|w2|…)\b`, with each word passed through `regex::escape`.
- A built-in match is exempt when `allow` holds its exact text. A project match is exempt when
  `allow` holds it after lower-casing both.
- The rule is `lore/inworld-vocab`, `Guardrail`, `Error`, `On::AnyRecord`. A subject without
  segments returns at once, without reading the settings, so it stores no `file:` read. One
  finding per occurrence, at the word (L20), with spec §5.11's message.

- [ ] **Step 1: Extend the fixture:**
  - passing: an in-world topic, a belief title, a dialogue, and voice blocks without developer
    words;
  - failing: `T-950` in a voice line, `T2` in a caption, a project word in a dialogue line, and a
    rule name in a belief title;
  - exempt: `T1` with an `allow` entry that has a reason;
  - outside in-world text: a design note and a dialogue heading holding words, and an in-world
    topic whose voice block holds a word (one finding).
- [ ] **Step 2: Write the failing tests** in `tests/index/vocab.rs`:
  - `// AC: CE-15.1` `inworld_vocab_passes_and_fails`;
  - `// AC: CE-15.2` `a_word_with_a_reasonless_entry_stays_a_developer_word`: the same word with
    a reasonless entry is flagged;
  - `an_occurrence_is_reported_once`;
  - `columns_count_unicode_scalar_values`;
  - `a_broken_settings_file_keeps_the_built_in_words` (Review Focus 2);
  - `// AC: CE-14.3` `a_new_word_revalidates_the_records_with_in_world_text`: adding a word
    through `apply` of `.theos/settings.toml` lists exactly the records with segments in
    `revalidated`.
  - Unit tests in `vocab.rs`: whole words, case, `ledger’s`, the allowlist.
- [ ] **Step 3: Run** them — FAIL.
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** the per-task checks — PASS.
- [ ] **Step 6: Commit** `feat(content): guardrail inworld-vocab` with
  `Refs: CE-15.1, CE-15.2, CE-14.3`.

### Task 14: The Shared Formalism mirror

**Files:**
- Modify: `crates/theos-content/Cargo.toml` (`serde` with `derive`, `serde_json` as normal
  dependencies)
- Create: `src/formalism/{mod,source,render,regenerate}.rs`,
  `src/parts/codex/rules/{formalism_edited,formalism_stray,formalism_missing,
  formalism_source}.rs`, `tests/fixtures/projects/formalism/`, `tests/index/formalism.rs`
- Modify: `src/lore/context.rs` (`formalism`), `src/parts/codex/mod.rs`, `src/lib.rs`

**Interfaces — produces:**

```rust
// formalism/mod.rs
pub const FOLDER: &str = "canon/formalism";
pub const SOURCE: &str = "canon/formalism/_source.json";
// formalism/source.rs
pub struct Source { pub page: String, pub equations: Vec<Equation> }
pub struct Equation { pub id: String, pub title: String, pub anchor: String, pub text: String }
pub struct SourceError { pub message: String, pub position: Option<Position> }
pub fn parse(text: &str) -> Result<Source, SourceError>;
// formalism/render.rs
pub fn render(source: &Source) -> Result<Vec<(String, String)>, SourceError>; // (path, text), by id
// formalism/regenerate.rs
pub struct Regenerated { pub written: Vec<String>, pub unchanged: Vec<String> }
#[derive(Debug, thiserror::Error)] pub enum FormalismError { /* Missing, Source, Io { path, source } */ }
pub fn regenerate(root: &Path) -> Result<Regenerated, FormalismError>;
// lore/context.rs
impl Context<'_> { pub fn formalism(&mut self) -> Result<Option<Rc<Result<Source, SourceError>>>, FactError> }
```

- `parse` uses `serde_json` into private serde structs with `deny_unknown_fields`, except the
  ignored top-level `"note"` (L16). It then checks spec §5.14:
  - `format` is 1;
  - ids match `^E[1-9][0-9]*$` and are unique;
  - a title is one non-empty line;
  - `anchor` follows P03's `Url` rule (reuse its function);
  - `text` is not empty.

  A syntax error takes `serde_json`'s line and column.
- `render` writes spec §5.14's record exactly. The title and source are spelled by P02's
  `yaml::encode` (`Slot::Field`); the text has CR → LF and one final `\n`.
- `regenerate` writes a file only when its bytes differ, creates the folder, and never deletes.
- The four rules are fixed errors. edited and stray use `On::Kinds(&["equation"])`; missing and
  source use `On::Input(SOURCE)`. `formalism/edited` compares `sha256(rendered)` with
  `RecordFacts.hash`.

- [ ] **Step 1: Write the fixture** `formalism/`:
  - a `_source.json` with `"note": "synthetic fixture (P04), not canon"`, page
    `https://example.invalid/synthetic-formalism`, and 18 equations `E1`…`E18`, titled
    "Synthetic equation n", with anchors `…#En`. E3's text holds `$\left[[a, b]\right]$`, which
    is verbatim and gives no finding;
  - a truth T-950 with `cites: [E7]`;
  - no equation records: the tests generate them into a temp copy;
  - `expected.toml`, the findings after a regeneration.
- [ ] **Step 2: Write the failing tests** in `tests/index/formalism.rs`:
  - `// AC: CE-16.1` `the_source_generates_e1_to_e18`: `render` gives 18 paths, and `E7.md`'s
    text equals a literal of the expected bytes; after `regenerate`, the index holds 18 equation
    records and the findings equal `expected.toml`.
  - `// AC: CE-16.2` `a_record_citing_e7_resolves`: no link finding on T-950. Removing `E7.md`
    gives `link/dangling`.
  - `// AC: CE-16.3` `a_hand_edit_fails_the_check`: editing E7's title, its body, or adding one
    space, each gives `formalism/edited`; `regenerate` repairs it and the finding goes.
  - `// AC: CE-16.4` `regenerating_is_idempotent`: a second run has `written` empty and every
    byte unchanged, also after a repair.
  - `a_stray_equation_and_a_missing_one_are_findings`.
  - `a_broken_source_is_a_finding_with_its_position`.
  - Unit tests in `source.rs` and `render.rs`, one per rule of the format.
- [ ] **Step 3: Run** them — FAIL.
- [ ] **Step 4: Implement**, and run `cargo deny --locked check`.
- [ ] **Step 5: Run** the per-task checks — PASS.
- [ ] **Step 6: Commit** `feat(content): generate and check the Shared Formalism mirror` with
  `Refs: CE-16.1, CE-16.2, CE-16.3, CE-16.4`.

### Task 15: Binary references

**Files:**
- Create: `src/index/binaries.rs`, `tests/fixtures/projects/binaries/`,
  `tests/fixtures/binaries/` (with a `.gitattributes` of `* -text`, so the bytes and hashes stay
  exact on every checkout), `tests/index/binaries.rs`
- Modify: `src/index/{schema.sql,store.rs,mod.rs}`, `src/rules/mod.rs` (`binaries::RULES` in
  `all`), `src/lib.rs`

**Interfaces — produces:**

```rust
pub struct BinaryReport { pub findings: Vec<Finding>, pub hashed: Vec<String> }
impl Index {
    pub fn binary_findings(&mut self, binaries_root: &Path) -> Result<BinaryReport, IndexError>;
}
pub(crate) const RULES: [RuleInfo; 2];   // binary/missing, binary/hash: fixed warnings
```

```sql
CREATE TABLE binary_memo (root TEXT NOT NULL, path TEXT NOT NULL, size INTEGER NOT NULL,
                          mtime_ns INTEGER NOT NULL, hash TEXT NOT NULL,
                          PRIMARY KEY (root, path)) WITHOUT ROWID;
```

- References are the valid entries of every field whose type is `BinaryRefs`, found through the
  registry (`kind`, `field`). The path is the text before the last `@`.
- Paths are judged as L15 says, before any file system call.
- A file is hashed in 64 KiB chunks (L13) only when its size and time differ from the memo. The
  memo is written in one `BEGIN IMMEDIATE` transaction. The findings sit at the entry's value,
  with spec §5.13's messages, sorted like `findings()`.

- [ ] **Step 1: Write the fixtures:**
  - `binaries/boards/{ok.bin,changed.bin}`;
  - the project: a concept and versions whose `outputs` reference `boards/ok.bin` (its real
    hash), `boards/changed.bin` (another file's hash), `boards/missing.bin`, and each path of
    Review Focus 5;
  - a sentinel file `outside.bin` next to the binaries folder.
- [ ] **Step 2: Write the failing tests** in `tests/index/binaries.rs`:
  - `// AC: CE-15.4` `a_missing_binary_or_a_hash_mismatch_is_a_warning`: `binary/missing` and
    `binary/hash` at their references, both warnings, and nothing for `ok.bin`.
  - `// AC: CE-15.4` `a_second_call_hashes_nothing`: the first report hashes two files, the
    second none.
  - `paths_that_leave_the_folder_are_never_opened` (Review Focus 5). The reference
    `../outside.bin` names the sentinel, which exists. It still gets `binary/missing` with the
    "leaves the binaries folder" message, and the memo holds no row for it, so it was judged
    before any file system call.
  - `a_changed_file_is_hashed_again`.
- [ ] **Step 3: Run** them — FAIL.
- [ ] **Step 4: Implement.**
- [ ] **Step 5: Run** the per-task checks — PASS.
- [ ] **Step 6: Commit** `feat(content): check binary references on request` with
  `Refs: CE-15.4`.

### Task 16: `theos ledger` and `theos formalism`

**Files:**
- Modify: `crates/theos/Cargo.toml` (`anyhow`, `clap`, `theos-content = { path =
  "../theos-content" }`), `crates/theos/src/main.rs`, `crates/theos/README.md`
- Create: `crates/theos/src/{ledger,formalism}.rs`, `crates/theos/tests/cli.rs`,
  `crates/theos-content/tests/fixtures/projects/lore/expected-ledger.txt`

**Interfaces — produces:**

```rust
// main.rs
#[derive(clap::Parser)] struct Cli { #[command(subcommand)] command: Command }
#[derive(clap::Subcommand)] enum Command { Ledger { folder: Option<PathBuf> },
    Formalism { folder: Option<PathBuf> } }
fn main() -> ExitCode;                          // L18
// ledger.rs
pub(crate) fn run(folder: &Path, out: &mut impl Write) -> anyhow::Result<()>;
pub(crate) fn format(ledger: &[TruthLedger]) -> String;   // spec §5.15 with L26
// formalism.rs
pub(crate) fn run(folder: &Path, out: &mut impl Write) -> anyhow::Result<()>;
```

- `ledger::run` calls `Project::open`, `Index::open` and `ledger()`, and writes `format` to
  `out`.
- `formalism::run` calls `formalism::regenerate`. It writes each written path, or "nothing to
  write".

- [ ] **Step 1: Write `expected-ledger.txt`**, the expected output for the `lore/` project, by
  hand from its records.
- [ ] **Step 2: Write the failing tests:**
  - in `ledger.rs`: `format_lays_out_states_floors_gates_and_beats`;
  - in `tests/cli.rs`, running `env!("CARGO_BIN_EXE_theos")` on a temp copy of the fixture:
    - `// AC: CE-15.3` `theos_ledger_prints_the_ledger`: stdout equals `expected-ledger.txt`,
      exit 0;
    - `theos_formalism_writes_the_mirror_once`: the first run lists 18 paths, the second
      "nothing to write";
    - `a_folder_without_theos_toml_exits_with_1`.
- [ ] **Step 3: Run** them — FAIL.
- [ ] **Step 4: Implement.** Run `cargo deny --locked check`.
- [ ] **Step 5: Run** the per-task checks — PASS.
- [ ] **Step 6: Commit** `feat(cli): theos ledger and theos formalism` with `Refs: CE-15.3`.

### Task 17: Neighbours, the history test and the rule sweep

**Files:**
- Create: `tests/index/neighbours.rs`, `tests/index/rules.rs`
- Modify: `tests/index/changes.rs`

- [ ] **Step 1: Write `neighbours.rs`.** `// AC: CE-14.3`
  `validating_one_record_validates_the_neighbours_it_affects` makes the six changes of spec §5.18
  on temp copies of the `lore/`, `versions/` and `world/` fixtures. Each is checked with `update`
  and with `preview`:
  - its `Update.revalidated` equals the listed paths;
  - the preview returns their findings and leaves `dump()` unchanged;
  - after the change, `dump()` equals a cold build.
- [ ] **Step 2: Extend the history test.** `project_files()` gains:
  - a band, two floors and a location;
  - two truths, three clues and a script with `reveals::`;
  - a concept with notes and a version with `addresses`;
  - `.theos/settings.toml` with one word, and a `_source.json` with E1 and its record.

  `Edit` gains:
  - `MovesTo(n)`, `Target(n)`, `Floors`, `FloorParent`, `Reveals(n)`, `After(n)` and
    `Requires(n)`;
  - `Word` (adds or removes a settings word);
  - `Equation` (changes E1 in the source) and `HandEdit` (edits `E1.md`);
  - `ParentCycle` (closes or opens a cycle).

  `previewed` includes `revalidated` in its paths. After every step, `dump()` equals a cold
  build.
- [ ] **Step 3: Write `rules.rs`.** `// AC: CE-14.2`
  `every_rule_has_a_finding_with_rule_severity_path_and_line` collects the findings of every P04
  fixture project: the formalism project after regeneration, and binaries through
  `binary_findings`. It asserts two things:
  - every id of `rules::all(&builtin)` in P04's groups appears;
  - every finding has a rule, a severity, a path and a position.
- [ ] **Step 4: Run** them. A failure means a bug in the code; fix the code, never the test.
- [ ] **Step 5: Run** the per-task checks — PASS.
- [ ] **Step 6: Commit** `test(content): neighbours, histories and every rule's finding` with
  `Refs: CE-14.2, CE-14.3`.

### Task 18: Speed, the README and the crate docs

**Files:**
- Modify: `crates/theos-content/README.md`, `src/lib.rs` (crate docs),
  `src/index/lore_step.rs` (the L8 constant, if the numbers ask for it)

- [ ] **Step 1: Measure.** Run P03's CE-12.1 test alone three times and record the fastest. Then
  time a one-file change of a clue in that project, with a temporary timing test that is not
  committed, and record that too. Write both numbers, and P03's 155 ms baseline, into the
  ledger.
- [ ] **Step 2: If** the cold open lost more than 100 ms against P03's baseline, profile before
  changing anything: bulk loading, prepared statements, one insert statement per table. Never
  raise a limit.
- [ ] **Step 3: Write** the README (what `theos-content` provides after P04, the lore step and
  its reads, the settings, the mirror, and what later phases use, from spec §6) and the `lib.rs`
  docs.
- [ ] **Step 4: Run** the per-task checks, then `cargo xtask ac-trace --phase P04`: every AC from
  CE-14.1 to CE-16.4 shows at least one passing test.
- [ ] **Step 5: Commit** `docs(content): describe what the crate provides after P04`, and a
  `perf(content): …` commit for any speed change.

## After the tasks

theos-phase steps 4 to 6:
1. The final review loop of `CLAUDE.md`: a fresh reviewer on the most capable model each round,
   cap 5.
2. `cargo xtask gate` on Windows, and `cargo xtask ac-trace --phase P04`.
3. The PR `feat(content): validation, lore rules, ledger, formalism mirror (P04)`. It:
   - lists every ruling of the ledger: L1–L27 above and the build's;
   - gets the labels `windows` and `macos` (spec §5.18), with links to their `platforms` runs
     (re-dispatch `gh workflow run platforms --ref <branch>` after pushes);
   - sets P04 to `pr-open` in `docs/status.md` in the same push.
