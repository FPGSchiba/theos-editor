# P48 · MCP read — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (`CLAUDE.md` makes it the execution method) to implement this plan task by task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** `theos-mcp` becomes an MCP server over stdio with the read tools `project_info`,
`codex_search`, `codex_read`, `codex_list`, `codex_graph` and `ledger_check`, plus the content
project's guides as prompts. Every record a result names carries the `never` and `locked` marks.

**Architecture:**
- **`theos-content`** gains one query, `Index::field_texts` (Task 1).
- **`theos-studio`** gains the logic, so P11's Codex can share it (spec §4.2):
  - `marks`: the `never` and `locked` marks;
  - `codex`: `search`, `list`, `read`, `graph` and `ledger_check`;
  - `project`: `info`;
  - `guides`;
  - `views::codex`, `views::project`: the JSON views. These derive `Serialize` and
    `schemars::JsonSchema`; P05's views get `JsonSchema` too.
- **`theos-mcp`** maps each tool to one studio call:
  - one module per tool under `src/tools/`;
  - a shared `State` that opens the workspace lazily and syncs it per call;
  - error results through `theos_studio::report`;
  - prompts from the guides.

**Tech stack:**
- Rust 1.99.1, edition 2024.
- New workspace dependencies (Task 3, Task 8):
  - `rmcp = { version = "3.12", default-features = false, features = ["server", "transport-io",
    "macros"] }`;
  - `tokio = { version = "1", features = ["rt", "io-std", "macros"] }`;
  - `schemars = "1"`.
  - Dev-only for `theos-mcp`: `rmcp` with `client` and `transport-child-process` added, and
    `tokio` with `process`.
- Already pinned: `clap`, `serde`, `serde_json`, `thiserror`, `tracing`, `tracing-subscriber`,
  `anyhow` (binary only), `pulldown-cmark`, `tempfile`.

**Spec:** `docs/specs/phases/P48-mcp-read.md` (approved and merged, #53). Read it with this plan.
Where this plan is silent, the spec decides; where they differ, the spec wins.

## Global Constraints

Umbrella G1–G16, verbatim:

- **G1** Repositories: `theos-editor` (this repo) holds the editor and the map generator
  (`theos-map-format`, `theos-maps`). `tower-of-theos` holds the content project, its map recipes
  and the blockout generator. `Theos-Engine` is never a dependency, except behind the cargo
  feature `engine`, which no phase here builds; the engine may depend on the map crates.
- **G2** Platforms: Windows 11 x64 (primary), Linux x64, and macOS 13 or later on Apple silicon
  and Intel (universal builds). Every PR is checked on Linux; Windows and macOS on Apple silicon
  are tested weekly, on demand (a label on the PR) and before every release, and macOS on Intel
  on demand and before every release (`docs/gate.md` §1).
- **G3** Rust: stable toolchain pinned in `rust-toolchain.toml`, edition 2024, `unsafe_code`
  forbidden, `Cargo.lock` committed, all builds `--locked`.
- **G4** Crates: `theos-content`, `theos-vcs`, `theos-studio`, `theos-map-format`, `theos-maps`
  (libs); `theos`, `theos-mcp`, `theos-editor` (bins); `xtask`. Paths `crates/<name>`, `xtask/`. UI in `editor-ui/`
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

Phase ACs, verbatim:

**CE-18 MCP read tools**
- CE-18.1 `project_info`, `codex_search`, `codex_read`, `codex_list`, `codex_graph`,
  `ledger_check` work over stdio from Claude Code.
- CE-18.2 Claude answers "what does Mother Ilse believe" from records alone.
- CE-18.3 Locked records are readable and flagged; truths with target `never` are marked in every
  result.

**Engineering directives for every subagent brief** (`CLAUDE.md`):
1. **Maintainability first:** small modules with one job, and shared inputs built in one place.
   A new tool is one module plus one line that registers it.
2. **Use what exists:** `rmcp`, `schemars`, `pulldown-cmark` and the P03–P05 queries. Never
   hand-write JSON-RPC or a Markdown parser.
3. **When fixes leak, change the approach.**
4. **The gate catches shortcuts:** no `#[allow]`; an `#[expect]` needs a real `reason`. No
   `unwrap`/`expect` outside tests. `print_stdout` and `print_stderr` are denied, which matters
   doubly here: stdout is the protocol.

**Conventions:**
- Tests carry `// AC: <id>`.
- Commits are Conventional Commits with a summary that starts lower case, and footers
  `Refs: CE-18.x` and `Phase: P48` plus
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Subagents never push.
- Fixtures are synthetic, marked `# synthetic fixture (P48), not canon`, with neutral texts.

## Review Focus

From spec §8. Each item is pinned by a test in the named task:

1. A `never` truth that appears only as a link target, a backlink source, a belief's `distorts`
   or a ledger clue's truth carries the mark. Tested in Task 5 and by the schema walk in Task 9.
2. A duplicated id where only one record is `never` or canon is marked by id. Tested in Task 3.
3. Invalid values (`target: Never`, `status: Canon`) give no mark. Tested in Task 3.
4. An edit, a delete and a rename on disk between two calls show in the next call. Tested in
   Task 9.
5. Anything on stdout that is not JSON-RPC, such as a log line or a panic message, fails a test.
   Tested in Task 8.

Further spec §8 inputs, each with its own test:
- Task 6: a hub with depth 3 and a cycle.
- Task 7: a guide with no heading, with CRLF, or with invalid UTF-8, and a `tools/guides` that is
  a file.
- Task 4: every parameter bound.
- Task 8: a start outside a project, followed by the creation of `theos.toml`.

## Rulings made while planning

- Ruling: the fixture project lives in `crates/theos-studio/tests/fixtures/projects/codex/`, not
  in `theos-mcp` (spec §5.13). The studio's unit tests use it first, and the `theos` CLI tests
  already read studio fixtures the same way. Cost if wrong: a folder move.
- Ruling: the studio checks the parameter bounds (`limit`, `depth`, `offset`, kind, status,
  folder) and returns `StudioError::InvalidFilter`. P11 calls the same functions without MCP's
  schema. Cost if wrong: a check in two places.
- Ruling: in `RecordRef`, `path`, `kind`, `title` and `status` are `Option`. A link target with
  no record has only its `id`. Cost if wrong: none; the keys are always present.

---

### Task 1: `Index::field_texts`

**Files:**
- Modify: `crates/theos-content/src/index/query.rs` (the query, next to `records`).
- Modify: `crates/theos-content/src/index/mod.rs` and `crates/theos-content/src/lib.rs`
  (re-export `FieldText`).

**Interfaces:**
- Produces:
  - `pub struct FieldText { pub path: String, pub text: String }` with `Clone`, `Debug`,
    `PartialEq` and `Eq`;
  - `impl Index { pub fn field_texts(&self, field: &str) -> Result<Vec<FieldText>, IndexError> }`.
    It returns the `field_values` rows with `field = ?1`, `shape = 'scalar'`, `valid = 1` and
    `entry IS NULL`, ordered by `path`.

- [ ] **Step 1: Write the failing tests.** They go in `query.rs`'s test module and use the
  existing in-memory test helpers of `theos-content`.
  - `field_texts_returns_valid_scalars`: a truth with `target: never` and one with
    `target: hinted`. `field_texts("target")` equals both, in path order. `// AC: CE-18.3`
  - `field_texts_skips_invalid_and_map_values`: `target: Never` (invalid) is absent. A lens
    entry (`lens: { stance: x }`) is not returned for `field_texts("stance")`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-content field_texts`. Expected: FAIL (no
  method).
- [ ] **Step 3: Implement** the query with `prepare_cached`, as `records()` does.
- [ ] **Step 4: Run** the same command. Expected: PASS. Then run
  `cargo clippy -p theos-content --all-targets -- -D warnings`.
- [ ] **Step 5: Commit** `feat(content): valid scalar values of a field (P48)`.

### Task 2: The `codex` fixture project

**Files:**
- Create: `crates/theos-studio/tests/fixtures/projects/codex/**`
- Create: `crates/theos-studio/src/test_support/codex.rs`, registered in
  `test_support/mod.rs`.

**Interfaces:**
- Produces: `pub(crate) fn codex_workspace() -> Result<(tempfile::TempDir, Workspace), Error>`.
  It copies the fixture to a temp folder and opens it, so the cache never lands in the repo.
  Task 8 copies the same folder from the `theos-mcp` tests (`CARGO_MANIFEST_DIR/../theos-studio/…`).

The fixture. Copy `theos.toml` from `projects/staged`, with the name `synthetic-codex`.

| Path | Content (beyond `id`, `kind`, `title`) |
|---|---|
| `people/hearth-folk.md` | `people`, the parent of the cult |
| `people/hearth-folk/fates-cult.md` | `faction`, `parent: hearth-folk`, `status: canon` |
| `people/hearth-folk/fates-cult/mother-ilse.md` | data model §14's `mother-ilse`, verbatim plus the synthetic marker |
| `world/…/fates-shrine.md` and the floor and band above it | synthetic places, so `home` and `where` resolve |
| `lore/truths/T-911.md` | `target: never`, statement "Synthetic statement A" |
| `lore/truths/T-912.md` | `target: hinted` |
| `lore/beliefs/B-911.md` | `held_by: [fates-cult]`, `distorts: T-912`, `accuracy: partial`, body "Synthetic belief A" |
| `lore/beliefs/B-912.md` | `held_by: [mother-ilse]`, `distorts: T-911`, `accuracy: false`, body "Synthetic belief B" |
| `lore/clues/C-031.md` | data model §14's clue, with `reveals: T-912` instead of `T-012` |
| `story/scripts/S-004.md` | data model §14's script, with `says:: [[B-911]]` instead of `B-016` and a mention `[[ghost-911]]` (a missing id) |
| `lore/topics/dup-911.md` and `lore/topics/sub/dup-911.md` | the same id twice, one with `status: canon` |
| `art/styles/…` | a `style` with `state: locked` |
| `canon/formalism/E91.md` | an `equation` record (the mirror) |
| `tools/guides/atlas.md`, `tools/guides/quest.md` | `# Placing on the atlas` and a guide without a heading |

Every record must be valid by P02–P04, except the findings the test below lists. The implementer
checks each kind's fields with `theos schema <kind>` from P05.

- [ ] **Step 1: Write the failing test** `codex_fixture_indexes_as_expected` in
  `test_support/codex.rs` (`// AC: CE-18.2`):
  - `index.lens("mother-ilse")` has the chain `mother-ilse`, `fates-cult`, `hearth-folk`;
  - its beliefs are `B-911` through `fates-cult` and `B-912` through `mother-ilse`;
  - `index.findings()` holds only the expected ones: the duplicated id, and the unresolved
    `ghost-911` if P03 reports it. The test lists them exactly.
- [ ] **Step 2: Run** `cargo nextest run -p theos-studio codex_fixture`. Expected: FAIL.
- [ ] **Step 3: Write the fixture** until the test passes.
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `test(studio): the codex fixture project (P48)`.

### Task 3: Marks and `JsonSchema` on the views

**Files:**
- Modify: `Cargo.toml` (`schemars = "1"`) and `crates/theos-studio/Cargo.toml`.
- Create: `crates/theos-studio/src/marks.rs`.
- Modify: `crates/theos-studio/src/views/*.rs` (add `schemars::JsonSchema` to every view's
  derive; no key changes).
- Modify: `crates/theos-studio/src/lib.rs` and `crates/theos-studio/src/error.rs`.

**Interfaces:**
- Consumes: `Index::field_texts` (Task 1) and `Index::records()`.
- Produces:
  - In `marks.rs`:
    - `#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, JsonSchema)]
      #[serde(rename_all = "lowercase")] pub enum LockReason { Canon, Never, Formalism, Style }`,
      in this order;
    - `#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, JsonSchema)] pub struct MarkView
      { pub never: bool, pub locked: Vec<LockReason> }`;
    - `pub struct Marks`, with:
      - `pub fn load(index: &Index) -> Result<Marks, StudioError>`;
      - `pub fn of_path(&self, path: &str) -> MarkView`;
      - `pub fn of_id(&self, id: &str) -> MarkView`, the union over every record with the id;
      - `pub fn is_never(&self, path: &str) -> bool`.
  - New `StudioError` variants:
    - `UnknownRecord { what: String }`, with the message "no record `{what}`";
    - `InvalidFilter { message: String }`, with the message `{message}`.

  `never` holds for a truth whose `target` is `never`. `locked` follows spec §5.5: `status` canon
  (valid values only), `never`, a path under `canon/formalism/`, and a `style` whose `state` is
  `locked`. `field_texts` rows are filtered by the record's kind from `records()`.

- [ ] **Step 1: Write the failing tests** in `marks.rs` on `codex_workspace()`, each with
  `// AC: CE-18.3`:
  - `never_truth_is_never_and_locked`: `of_path` of T-911 is `{ never: true, locked: [Never] }`.
  - `canon_style_and_mirror_are_locked`: fates-cult gives `[Canon]`, the style `[Style]` and
    E91 `[Formalism]`.
  - `duplicated_id_is_marked_by_any_record`: `of_id("dup-911")` gives `[Canon]`.
  - `invalid_values_give_no_mark`: a project with `target: Never` and `status: Canon` gives
    the default mark. Build it in a temp folder.
  - `unknown_id_has_no_mark`: `of_id("ghost-911")` is the default.
- [ ] **Step 2: Run** `cargo nextest run -p theos-studio marks`. Expected: FAIL.
- [ ] **Step 3: Implement** `Marks` (sets of paths plus an id→paths map), the error variants and
  the derives.
- [ ] **Step 4: Run** `cargo nextest run -p theos-studio` and
  `cargo clippy -p theos-studio --all-targets -- -D warnings`. Expected: PASS. P05's view tests
  still pass unchanged.
- [ ] **Step 5: Commit** `feat(studio): never and locked marks (P48)`.

### Task 4: `RecordRef`, `codex::search` and `codex::list`

**Files:**
- Create: `crates/theos-studio/src/views/codex.rs` and `crates/theos-studio/src/codex/mod.rs`,
  `search.rs` and `list.rs`.
- Modify: `views/mod.rs` and `lib.rs` (`pub mod codex;`).

**Interfaces:**
- Consumes: `Marks`, `Workspace::index()`, `Index::search`, `Index::records`,
  `theos_content::read_record`.
- Produces (spec §5.7; every view derives `Serialize`, `JsonSchema`, `Debug`, `Clone`):
  - `pub struct RecordRef { pub id: String, pub path: Option<String>, pub kind: Option<String>,
    pub title: Option<String>, pub status: Option<String>, #[serde(flatten)] pub mark: MarkView }`;
    `RecordRef::from_summary(&RecordSummary, &Marks)` and `RecordRef::missing(id)`.
  - `pub struct SearchView { pub query: String, pub hits: Vec<HitView>, pub truncated: bool }`.
    `HitView` is a flattened `RecordRef` plus `score: f64` and `matches: Vec<MatchView>`.
    `MatchView` is `{ column: String, line: u32, column_number: u32, r#match: String (serde
    "match"), text: String }`. `column` is `title`, `body`, `field:<key>` or
    `field:<key>.<entry>`.
  - `pub struct ListView { pub records: Vec<RecordRef>, pub total: usize, pub offset: usize,
    pub truncated: bool }`.
  - `pub fn search(ws: &Workspace, query: &str, limit: usize) -> Result<SearchView,
    StudioError>`. `limit` must be 1–100.
  - `pub struct ListFilter { pub kind: Option<String>, pub status: Option<String>, pub folder:
    Option<String>, pub offset: usize, pub limit: usize }`.
  - `pub fn list(ws: &Workspace, filter: &ListFilter) -> Result<ListView, StudioError>`.
    - `limit` must be 1–500.
    - `kind` must be registered; the error names the kinds.
    - `status` must be `draft` or `canon`.
    - `folder` must be relative, use `/`, and contain no `..` segment; a trailing `/` is
      trimmed.

  `match` is the record text under the `span`. Each hit's file is read once with `read_record`;
  a file that cannot be read gives `match` = `""`.

- [ ] **Step 1: Write the failing tests** in `codex/search.rs` and `codex/list.rs`, each with
  `// AC: CE-18.1`, and with `// AC: CE-18.3` where marks are checked:
  - `search_finds_mother_ilse`: `search(ws, "Mother Ilse", 20)` has its first hit
    `mother-ilse`, and a match with column `title`, line ≥ 1, and `match` `"Mother Ilse"` (or the
    matched word).
  - `search_marks_never_truth`: `"Synthetic statement A"` hits T-911 with `never: true` and
    `locked: ["never"]`. The JSON is checked through `serde_json::to_value`.
  - `search_limit_bounds`: limit 0 and 101 give `InvalidFilter`; limit 1 gives `truncated: true`
    when there are more hits.
  - `list_filters_combine`: kind `belief` gives B-911 and B-912; status `canon` gives the two
    canon records; folder `lore/truths` gives T-911 and T-912; folder `lore/truth` gives none;
    offset and limit page with the right `total`.
  - `list_rejects_bad_filters`: an unknown kind (the message names `npc`), status `Canon`, folder
    `../x`, folder `a\b`, limit 0 and limit 501.
  - `record_ref_json_keys`: every key of a `RecordRef` is present, `null` when absent, and
    `never` and `locked` are at the top level.
- [ ] **Step 2: Run** `cargo nextest run -p theos-studio codex::`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(studio): search and list records with marks (P48)`.

### Task 5: `codex::read`

**Files:**
- Create: `crates/theos-studio/src/codex/read.rs`.
- Modify: `crates/theos-studio/src/views/codex.rs`.

**Interfaces:**
- Consumes:
  - from Tasks 3–4: `Marks`, `RecordRef`, `views::check::FindingView`;
  - from the index: `Index::record`, `links_from`, `backlinks`, `children`, `findings_for`,
    `lens`, `ledger_of`, `field_texts`;
  - `read_record`.
- Produces:
  - `pub enum ReadTarget { Id(String), Path(String) }`.
  - `pub fn read(ws: &Workspace, target: &ReadTarget) -> Result<ReadView, StudioError>`.
    - An unknown id or path gives `UnknownRecord`.
    - A path must be a record path of the index; it is not normalised beyond `\` → `/`.
  - `ReadView { records: Vec<RecordView> }`.
  - `RecordView`: a flattened `RecordRef` plus:
    - `text: String`;
    - `links` and `backlinks`, both `Vec<LinkView>`;
    - `children: Vec<RecordRef>`;
    - `findings: Vec<FindingView>`;
    - `lens: Option<LensView>`;
    - `ledger: Option<TruthView>`.
  - `LinkView { slot: String (Slot::as_db), fragment: Option<String>, line: u32, column: u32,
    source: RecordRef, target: TargetRef }`. `TargetRef` is a flattened `RecordRef` plus
    `exists: bool`.
  - `LensView`:
    - `chain: Vec<RecordRef>`;
    - `fields: Vec<LensFieldView { field, value: LensValueView, from: RecordRef, overrides:
      Vec<OverrideView { value, from }> }>`;
    - `beliefs: Vec<BeliefView { belief: RecordRef, through: RecordRef, distorts:
      Option<TargetRef>, accuracy: Option<String> }>`.

    `LensValueView` is `#[serde(untagged)]`, either `Text(String)` or `Links(Vec<String>)`.
  - `TruthView` is P05's ledger element. Make its fields `pub(crate)` or add a constructor; no
    key changes.

  `distorts` and `accuracy` come from `field_texts("distorts")` and `field_texts("accuracy")`,
  by the belief's path.

- [ ] **Step 1: Write the failing tests** in `codex/read.rs`:
  - `read_mother_ilse_beliefs` (`// AC: CE-18.2`):
    - the chain is `mother-ilse`, `fates-cult`, `hearth-folk`;
    - the beliefs are B-911 through `fates-cult`, with `distorts` T-912 and `accuracy` `partial`,
      and B-912 through `mother-ilse`, with `distorts` T-911 and `never: true`;
    - each belief carries its title;
    - `text` contains `role: keeper of the Fates' shrine`.
  - `read_marks_never_everywhere` (`// AC: CE-18.3`):
    - `read(Id("T-911"))` has `never: true` and returns its full text with "Synthetic
      statement A";
    - its backlinks include B-912 as `source`, with `slot` `field:distorts`;
    - `read(Id("B-912"))` has a link whose `target` is T-911 with `never: true`.
  - `read_locked_records_return_text` (`// AC: CE-18.3`): fates-cult, the style and E91 each
    return their text and their `locked` reasons.
  - `read_missing_link_target` (`// AC: CE-18.1`): S-004's link to `ghost-911` has
    `exists: false`, `path: null` and `never: false`.
  - `read_duplicated_id_returns_both` (`// AC: CE-18.1`): two `records`.
  - `read_truth_has_ledger` (`// AC: CE-18.1`): T-912's `ledger` names C-031.
  - `read_unknown_is_error` (`// AC: CE-18.1`): `Id("nope")` and `Path("lore/nope.md")` give
    `UnknownRecord`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-studio codex::read`. Expected: FAIL.
- [ ] **Step 3: Implement**, with one helper that turns a `LinkRef` into a `LinkView`.
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(studio): read a record with links, lens and marks (P48)`.

### Task 6: `codex::graph`

**Files:**
- Create: `crates/theos-studio/src/codex/graph.rs`.
- Modify: `views/codex.rs`.

**Interfaces:**
- Produces:
  - `pub enum Direction { Out, In, Both }`, deserialised in lowercase;
  - `pub fn graph(ws: &Workspace, id: &str, depth: u8, direction: Direction) ->
    Result<GraphView, StudioError>`. `depth` is 1–3; an unknown root id gives `UnknownRecord`;
  - `GraphView { root: String, depth: u8, nodes: Vec<NodeView>, edges: Vec<EdgeView>,
    truncated: bool }`;
  - `NodeView` is a flattened `TargetRef` plus `distance: u8`;
  - `EdgeView { from, to, slot, fragment }`;
  - `pub const MAX_NODES: usize = 200`.

  The walk follows spec §5.9: breadth-first; out-edges from `links_from` of every path with the
  id; in-edges from `backlinks` and `children`. Nodes are ordered by distance, then path (a
  missing id sorts by id). A distance that would pass `MAX_NODES` is cut in that order, and
  `truncated` is set. A node at `depth` lists no edges to nodes outside the result. A missing id is
  never expanded. Edges are deduplicated on `(from, to, slot, fragment)`.

- [ ] **Step 1: Write the failing tests** (`// AC: CE-18.1`; `// AC: CE-18.3` on the marks
  test):
  - `graph_depth_one_both`: around `mother-ilse`, the nodes include fates-cult, B-912 and S-004,
    and an edge `S-004 → mother-ilse` with slot `beat:with`.
  - `graph_out_only` and `graph_in_only`: the edge sets differ as expected.
  - `graph_marks_never_node`: T-911 appears at distance 2 from mother-ilse with `never: true`.
  - `graph_missing_node_not_walked`: `ghost-911` is a node with `exists: false` and has no
    out-edges.
  - `graph_cycle_terminates`: two records that link to each other, depth 3. Each node appears
    once.
  - `graph_caps_at_max_nodes`: a generated temp project with one hub and 250 records linking to
    it. The result has ≤ 200 nodes and `truncated: true`.
  - `graph_depth_bounds`: depth 0 and 4 give `InvalidFilter`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-studio codex::graph`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(studio): the link graph around a record (P48)`.

### Task 7: `ledger_check`, `project::info` and the guides

**Files:**
- Create: `crates/theos-studio/src/codex/ledger.rs`, `crates/theos-studio/src/project.rs`,
  `crates/theos-studio/src/guides.rs` and `crates/theos-studio/src/views/project.rs`.
- Modify: `crates/theos-studio/Cargo.toml` (`pulldown-cmark.workspace = true`) and `lib.rs`.

**Interfaces:**
- Produces:
  - `pub fn ledger_check(ws: &Workspace, id: Option<&str>) -> Result<LedgerCheckView,
    StudioError>`:
    - `LedgerCheckView { truths: Vec<MarkedTruthView>, findings: Vec<FindingView>, errors:
      usize, warnings: usize }`, where `MarkedTruthView` is a flattened `TruthView` plus
      `MarkView`;
    - the findings are those whose rule starts with `lore/`, on the truths' and the clues'
      paths;
    - an `id` that is not a truth gives `UnknownRecord`.
  - `pub fn info(ws: &Workspace, tools: &[&str]) -> Result<ProjectInfoView, StudioError>`:
    - `ProjectInfoView { root, schema_version: u32, cache: CacheKind ("file" | "memory"),
      records, kinds: Vec<KindCountView { name, part, records }>, rules: Vec<RuleSeverityView {
      id, severity }>, errors, warnings, guides: Vec<GuideView>, tools: Vec<String> }`;
    - `rules` gives each rule's effective severity, from `schema::overview(registry,
      Some(root))`: `configured`, else the default.
  - `pub struct Guide { pub name: String, pub title: String, pub path: String }` (also the
    `GuideView`).
  - `pub fn guides::list(root: &Path) -> Vec<Guide>`:
    - it lists `tools/guides/*.md`, not recursively, sorted by name;
    - it skips a file it cannot read as UTF-8 with a `tracing::warn!`;
    - a missing folder, or a file in its place, gives an empty list;
    - `title` is the text of the first heading (`pulldown-cmark`), else the stem.
  - `pub fn guides::read(root: &Path, name: &str) -> Result<Option<String>, StudioError>`. It
    gives `None` for an unknown name, or a name with `/`, `\` or `..`. CRLF becomes LF.

- [ ] **Step 1: Write the failing tests**:
  - `ledger_check_marks_and_findings` (`// AC: CE-18.1`, `CE-18.3`):
    - T-911 is `never: true`;
    - T-912 has C-031's entry;
    - a temp copy with a clue that confirms T-911 yields the `lore/never-confirm` finding in
      `findings`;
    - `errors` counts it.
  - `ledger_check_one_truth` (`// AC: CE-18.1`):
    - `Some("T-912")` gives one truth;
    - `Some("B-911")` gives `UnknownRecord`.
  - `project_info_reports` (`// AC: CE-18.1`):
    - `records` matches the fixture count;
    - `kinds` includes `npc` with 1;
    - `rules` includes `lore/never-confirm` with severity `error`;
    - `guides` are `atlas` ("Placing on the atlas") and `quest` (title `quest`);
    - `cache` is `file`;
    - `tools` echoes its input.
  - `guides_edge_cases` (`// AC: CE-18.1`):
    - a CRLF guide reads with LF;
    - a non-UTF-8 file is skipped;
    - `tools/guides` as a file gives none;
    - `read("../theos")` gives `None`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-studio ledger_check project guides`.
  Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo nextest run -p theos-studio` and clippy. Expected: PASS.
- [ ] **Step 5: Commit** `feat(studio): ledger check, project info and guides (P48)`.

### Task 8: The `theos-mcp` server with `project_info`

**Files:**
- Modify: `Cargo.toml` (`rmcp`, `tokio`), `crates/theos-mcp/Cargo.toml` and
  `crates/theos-mcp/README.md` (usage, `.mcp.json`).
- Rewrite: `crates/theos-mcp/src/main.rs`, as the entry point only: clap, tracing to stderr
  filtered by `THEOS_LOG`, a current-thread runtime, `serve(stdio())`, exit codes.
- Create:
  - `crates/theos-mcp/src/state.rs`: the project and its lazy open;
  - `crates/theos-mcp/src/server.rs`: the `Server` and its `ServerHandler`;
  - `crates/theos-mcp/src/tools/mod.rs`: the router and the shared `run` helper;
  - `crates/theos-mcp/src/tools/project_info.rs`;
  - `crates/theos-mcp/tests/mcp/main.rs`, `common.rs` and `project_info.rs`.
- Modify: `crates/theos-mcp/tests/smoke.rs` (`.stdin(Stdio::null())`; the assertion is
  unchanged).

**Interfaces:**
- Consumes: `theos_studio::{Workspace, find_root, report, project::info}`.
- Produces, for Tasks 9–10:
  - `state::State`, which holds `Locator::{Given(PathBuf), Search(PathBuf)}` and an
    `Option<Workspace>`.
  - `State::with<T>(&mut self, f: impl FnOnce(&mut Workspace) -> Result<T, StudioError>) ->
    Result<T, String>`:
    - it opens the workspace when none is open (`Workspace::open` for `Given`; `find_root`,
      then `open`, for `Search`);
    - it syncs through `ws.write(|i| i.sync())`;
    - it runs `f`;
    - it maps any error through `report`.
  - `Server { state: Arc<Mutex<State>> }`.
  - `tools::run<T>(server, f) -> Result<Json<T>, String>`. Every tool calls it, so the
    lock-sync-query-report path exists once.
  - A failed call is a tool result with `isError: true` and one text block. Probe in Step 3
    that rmcp 3.12 turns `Err(String)` from a `#[tool]` fn into exactly that. If not, `run`
    returns `CallToolResult` built with `CallToolResult::error`, with the structured value set
    on success.
  - Every tool has `annotations(read_only_hint = true)`.
  - The server info carries the name `theos-mcp`, the crate version, and instructions that say
    the server is read-only and explain `never` and `locked`.
  - `tests/mcp/common.rs`:
    - `fn fixture_copy() -> (TempDir, PathBuf)`;
    - `async fn client(project: &Path, args: &[&str]) -> RunningService<RoleClient, ()>`, which
      starts `env!("CARGO_BIN_EXE_theos-mcp")` with `TokioChildProcess` in `project`;
    - `fn structured(result: &CallToolResult) -> serde_json::Value`.

- [ ] **Step 1: Write the failing tests**:
  - `lists_exactly_the_tools` (`// AC: CE-18.1`): `list_tools` gives exactly `project_info`
    for now, with an output schema and `readOnlyHint`. Task 9 extends the list to all six.
  - `project_info_over_stdio` (`// AC: CE-18.1`): the structured content has `cache` `file` and
    the fixture's record count.
  - `start_outside_project_reports_then_recovers` (`// AC: CE-18.1`):
    - the server is started in an empty temp folder;
    - `project_info` is `isError` with text containing `no theos.toml`;
    - the test then writes the fixture into that folder;
    - the next call succeeds.
  - `given_project_must_hold_theos_toml` (`// AC: CE-18.1`): `--project <empty>` gives an
    `isError` naming the folder.
  - `stdout_is_only_json_rpc` (`// AC: CE-18.1`):
    - the test spawns the binary with raw pipes and `THEOS_LOG=trace`;
    - it writes `initialize`, `notifications/initialized`, `tools/call project_info` and then
      EOF;
    - every stdout line must parse as a JSON-RPC 2.0 message;
    - the exit code must be 0.
- [ ] **Step 2: Run** `cargo nextest run -p theos-mcp`. Expected: FAIL. The smoke test still
  passes.
- [ ] **Step 3: Implement** with `#[tool_router]` on `Server` and a hand-written
  `ServerHandler` that keeps the router (Task 10 adds prompts to it).
- [ ] **Step 4: Run** `cargo nextest run -p theos-mcp`,
  `cargo clippy -p theos-mcp --all-targets -- -D warnings` and `cargo deny check`. Expected:
  PASS.
- [ ] **Step 5: Commit** `feat(mcp): the stdio server with project_info (P48)`.

### Task 9: The other five tools

**Files:**
- Create: `crates/theos-mcp/src/tools/{codex_search,codex_read,codex_list,codex_graph,
  ledger_check}.rs`, each with its `Params` struct (`Deserialize`, `JsonSchema`,
  `#[serde(deny_unknown_fields)]`) and one `#[tool]` fn. Each registers in `tools/mod.rs`.
- Create: `crates/theos-mcp/tests/mcp/{search,read,list,graph,ledger,marks,freshness}.rs`.

**Interfaces:**
- The parameters follow spec §5.8:
  - `codex_search`: `{ query: String, limit: Option<usize> = 20 }`;
  - `codex_read`: `{ id: Option<String>, path: Option<String> }`. Exactly one of them, else
    an `isError` "give exactly one of `id` and `path`";
  - `codex_list`: `{ kind, status, folder: Option<String>, offset: Option<usize> = 0, limit:
    Option<usize> = 100 }`;
  - `codex_graph`: `{ id: String, depth: Option<u8> = 1, direction: Option<Direction> = Both }`;
  - `ledger_check`: `{ id: Option<String> }`.
- Each description is one sentence on what the tool returns, plus "Records carry `never` (a
  truth Claude must never confirm) and `locked` (reasons it cannot be changed over MCP)."

- [ ] **Step 1: Write the failing tests** over stdio, on `fixture_copy()`:
  - `lists_exactly_the_six_tools` (`// AC: CE-18.1`): this replaces Task 8's one-tool list. The
    six names match `^[a-z][a-z0-9_]*$` (G7).
  - Each tool (`// AC: CE-18.1`): `structured(...)` equals the studio function's
    `serde_json::to_value` on the same fixture. The test opens a second `Workspace` on the same
    copy after the call. This proves that the server adds or loses nothing.
  - Parameter errors (`// AC: CE-18.1`):
    - an unknown key gives a protocol error (`deny_unknown_fields`);
    - `limit: 0` gives `isError`;
    - both `id` and `path` give `isError`;
    - `codex_read` of `nope` gives `isError` with "no record `nope`".
  - `mother_ilse_from_records_alone` (`// AC: CE-18.2`):
    - `codex_search "Mother Ilse"` → `codex_read {id: mother-ilse}` → `codex_read {id: B-911}`;
    - the assertions are those of spec §5.14's CE-18.2 row: the chain, both beliefs with
      `through` and `distorts`, and B-911's text "Synthetic belief A".
  - `never_and_locked_in_every_result` (`// AC: CE-18.3`): T-911 shows `never: true` and
    `locked: ["never"]` in the results of `codex_search`, `codex_read` (as itself, as B-912's
    `distorts` and as a link target), `codex_list {kind: truth}`, `codex_graph {id: B-912}` and
    `ledger_check`.
  - `every_record_object_carries_marks` (`// AC: CE-18.3`):
    - it calls every tool with representative parameters;
    - it walks each `structuredContent` recursively;
    - every object with a string `id` and a `path` key must also have `never` (bool) and
      `locked` (array).
  - `edits_on_disk_show_in_next_call` (`// AC: CE-18.1`): across five consecutive call pairs,
    the second call shows each change:
    - the title of B-911 changed;
    - T-912 deleted;
    - C-031 renamed to C-032;
    - `.theos/settings.toml` changing `lore/three-clue` to `warning`, which shows in
      `project_info`;
    - `target: never` added to T-912, which marks it.
- [ ] **Step 2: Run** `cargo nextest run -p theos-mcp`. Expected: FAIL.
- [ ] **Step 3: Implement** the five modules.
- [ ] **Step 4: Run** `cargo nextest run -p theos-mcp` and clippy. Expected: PASS.
- [ ] **Step 5: Commit** `feat(mcp): codex and ledger read tools (P48)`.

### Task 10: Guides as prompts

**Files:**
- Modify: `crates/theos-mcp/src/server.rs` (the prompts capability, `list_prompts` and
  `get_prompt`).
- Create: `crates/theos-mcp/src/prompts.rs` and `crates/theos-mcp/tests/mcp/prompts.rs`.

**Interfaces:**
- Consumes: `guides::list` and `guides::read` (Task 7), the project root from `State`.
- Produces:
  - `prompts/list`: one prompt per guide, with `name` = stem, `description` = title and no
    arguments.
  - `prompts/get`: one user message with the guide's text.
  - An unknown name gives the MCP error `invalid_params`.
  - Without a project, `prompts/list` gives an empty list.

- [ ] **Step 1: Write the failing tests** (`// AC: CE-18.1`):
  - `prompts_list_guides`: `atlas` with "Placing on the atlas", and `quest`.
  - `prompts_get_returns_text`: the text of `atlas.md`.
  - `prompts_unknown_is_invalid_params`.
  - `new_guide_appears_without_restart`: a third guide is written between two `list_prompts`
    calls.
- [ ] **Step 2: Run** `cargo nextest run -p theos-mcp prompts`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(mcp): serve the guides as prompts (P48)`.

### Task 11: Live session evidence and phase checks (controller, not a subagent)

- [ ] **Step 1:** Release-build `theos-mcp`.
- [ ] **Step 2:** Copy the fixture to a scratch folder with a `.mcp.json` that names the
  binary.
- [ ] **Step 3:** Run a Claude Code session there (`claude -p` with `--mcp-config` and only the
  `theos-mcp` tools allowed):
  - one prompt calls each of the six tools;
  - a second prompt asks "What does Mother Ilse believe?".
- [ ] **Step 4:** Save both transcripts in the PR as evidence for CE-18.1 and CE-18.2.
- [ ] **Step 5:** Run the phase checks:
  - `cargo xtask gate` on Windows;
  - `cargo xtask ac-trace --phase P48`, where each AC must show at least one passing test;
  - a Linux run in Docker, as for P05.
- [ ] **Step 6:** Update `docs/status.md`: P48 `in-progress` at plan time, `pr-open` at the PR.
