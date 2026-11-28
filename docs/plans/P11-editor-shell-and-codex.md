# P11 · Editor shell and Codex — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to
> implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a read-only Tauri 2 desktop shell that opens a content project, shows the rail of its
enabled parts and the Codex of `docs/ui/screens/01-main.html`, with search, one keymap and a
command palette, typed IPC generated from Rust, and the gate's `ui` check.

**Architecture:**
- Rust gives the data:
  - `theos-content` builds a record body's block tree;
  - `theos-studio` holds the shared Codex queries (`codex`, P48's shape), the Codex-only views
    (`codex_view`) and the editor-part registry (`parts`);
  - `theos-editor` maps them to tauri-specta IPC types.
- The Svelte 5 UI in `editor-ui/` only draws, with Theos UI's classes and tokens.
- Page tests replay IPC answers that Rust tests record from the real commands.
- A gate PR merges first. It adds the `ui` job, the Linux WebKit packages, the `engine`-feature
  builds and the `deny` ignores.

**Tech stack:**
- Rust: Tauri =2.12.5, tauri-build =2.7.4, tauri-plugin-dialog 2, tauri-specta =2.0.0-rc.29,
  specta =2.0.0-rc.28, specta-typescript =0.0.15, schemars 1.
- UI: Svelte 5.57.3, Vite 8.3.8, TypeScript 6.0.3, pnpm 12.23.0, Node 24, @tauri-apps/api
  2.12.4.
- Tests and checks: Vitest 5.1.0, @playwright/test 1.65.1, svelte-check 4.7.8, ESLint 10,
  fuzzysort.

**Spec:** `docs/specs/phases/P11-editor-shell-and-codex.md` (approved, #56). Read it with this
plan. P48's spec `docs/specs/phases/P48-mcp-read.md` §5.4–§5.7 defines the shared `codex`
shapes.

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

The phase ACs, verbatim:

**CE-32 Editor shell**
- CE-32.1 The app opens a project and shows the rail of `docs/architecture.md` §6.
- CE-32.2 Shortcuts (Ctrl on Windows and Linux, Cmd on macOS) and a command palette.
- CE-32.3 Builds with and without the `engine` feature (the feature is empty).
- CE-32.4 IPC types for the UI are generated from Rust; a mismatch fails the build.

**CE-33 Codex view**
- CE-33.1 A floor shows as on `docs/ui/screens/01-main.html`.
- CE-33.2 Backlinks are derived.
- CE-33.3 Search jumps to a record.

From the spec, binding on every task:

- **Read-only.** No task runs a P06 command or writes a content file. The only file the app
  writes is the state file.
- **Pins.** Tauri and specta crates take `=` pins in `[workspace.dependencies]`. npm versions
  are exact in `package.json`, with `pnpm-lock.yaml` committed. Newer versions only if at least
  two weeks old.
- **No HTML from records.** The body is a typed tree from Rust. No `{@html}` anywhere in
  `editor-ui/src`.
- **Tokens only.** No colour, font or size literal and no `opacity` on text in a component.
  Theme on the root: `data-theme="grid-noir"`.
- **One keymap.** Key combinations are written only in `editor-ui/src/lib/keymap.ts`.
- **Generated IPC.** `invoke` only inside `editor-ui/src/lib/ipc/`. View types never use `u64`,
  `i64`, `usize` or `isize`; use `u32`, or a string where a value could exceed it.
- **Disabled, not dead.** Every unbuilt control is disabled and says "arrives with PNN".
- **No network** (G12): the CSP is `default-src 'self'`; no remote font, script or image.
- **Lints.** Workspace lints apply. The one allowed exception is
  `#[expect(clippy::needless_pass_by_value, reason = "Tauri injects State and AppHandle by value")]`
  on commands that take `State` or `AppHandle`. No `#[allow]`.
- **Protected paths** (`docs/**` except plans and status, `.github/**`, `gate/**`, `xtask/**`,
  `deny.toml`, `design/**`) change only in the gate PR (Part A), never on the phase branch.
- **Directives** (CLAUDE.md, put them in every brief):
  - one module per job;
  - a new check, part, preset or command is one module plus one registering line;
  - use what exists;
  - when fixes leak, change the approach.

## Review Focus

The five inputs most likely to bite a person, each with its test in the owning task:

1. **Text that looks like HTML or a script** in a title, a field value or the body shows as
   text. Test in Task 12 (page) and Task 3 (the tree keeps it as `Text`).
2. **A record changed, deleted or renamed outside the app** while its tab is open, then the
   window gets the focus back. The tab reloads, or closes with a status line. Test in Task 14.
3. **A parent cycle or a dangling `parent`** (`a` → `b` → `a`) does not hang or drop the tree.
   Cyclic records sit at the top level, once. Test in Task 5.
4. **Search with only a prefix** (`#`, `:`, `@`), with a prefix in another case (`:Floor`), with
   FTS operators as text (`AND`, `*`, `-`), and typing faster than the debounce. It gives sane
   results and no error, and the last query wins. Tests in Tasks 5 and 13.
5. **A project path with spaces or non-ASCII letters**, and a cache held by another process.
   The app opens it, and the status bar says the index is in memory. Test in Task 6.

---

## Part A — the gate PR (`gate/P11-ui`, its own worktree `.worktrees/P11-gate`)

Branch from `origin/main`. Title `ci(gate): the ui check and the editor's build needs (gate)`.
It touches protected paths, so it waits for Jann: no auto-merge. It merges before Part B's PR.
Commit footers: `Phase: P11` and the Claude trailers (CONTRIBUTING.md §2).

### Task A1: the `ui` check in xtask

**Files:**
- Create: `xtask/src/checks/ui.rs`
- Modify: `xtask/src/checks/mod.rs`, `xtask/src/verdict.rs` (`Check::Ui`, name `ui`),
  `xtask/src/gate.rs` (dispatch and order: `ui` after `ac-trace`), `xtask/src/tools.rs` (node
  and pnpm)
- Test: `xtask/tests/ui.rs`

**Interfaces:**
- Produces: `Check::Ui`; `checks::ui::run(root, ci, out) -> Result<Verdict>`; the pure core
  `ui::plan(root: &Path) -> UiPlan`, where `UiPlan` is `NotNeeded(&'static str)` or
  `Steps(Vec<Step>)`, and `Step { name: &'static str, program: &'static str, args: Vec<String> }`.

- [ ] **Step 1: Write the failing tests** in `xtask/tests/ui.rs`:
  - `not_needed_without_editor_ui`: a temp root without `editor-ui/package.json` gives
    `NotNeeded("no editor-ui yet")`;
  - `steps_in_order`: with the file present, the step names are `install`, `svelte-check`,
    `eslint`, `vitest`, `playwright`, and their argv are exactly:
    - `pnpm install --frozen-lockfile`;
    - `pnpm exec svelte-check --fail-on-warnings`;
    - `pnpm exec eslint .`;
    - `pnpm exec vitest run`;
    - `pnpm exec playwright test`;

    all run in `editor-ui/`;
  - `every_step_runs_and_each_failure_is_a_finding`: a fake runner that fails `eslint` and
    `vitest` gives a verdict with exactly two findings, names included, and five steps run;
  - `check_name_round_trips`: `Check::from_name("ui") == Some(Check::Ui)`.
- [ ] **Step 2:** `cargo nextest run -p xtask --test ui`. Expected: compile failure, then FAIL.
- [ ] **Step 3:** Implement `ui.rs`.
  - Inject the runner as a closure (`FnMut(&Step) -> std::io::Result<Output>`) so the tests
    need no Node.
  - `run` uses `std::process::Command` from `editor-ui/`, with `pnpm` resolved through `which`.
  - Each failed step is a finding with the last 40 lines of its output.
  - `tools` checks `node --version` (major 24) and `pnpm --version` against the
    `packageManager` pin of `editor-ui/package.json`, but only when that file exists.
- [ ] **Step 4:** `cargo nextest run -p xtask`. Expected: PASS, every earlier xtask test
  included.
- [ ] **Step 5:** Commit `feat(xtask): the ui check`.

### Task A2: `engine` in clippy and test (CE-32.3)

**Files:**
- Modify: `xtask/src/checks/cargo_cmds.rs`
- Test: its unit tests, `xtask/tests/settings.rs` if it lists the commands

- [ ] **Step 1: Failing tests:**
  - `clippy_also_builds_the_engine_feature`: the clippy command list has, after the workspace
    run, `cargo clippy -p theos-editor --all-targets --features engine --locked -- -D warnings`;
  - `test_also_runs_the_engine_feature`: the test command list has
    `cargo nextest run -p theos-editor --features engine --locked` with the same profile
    argument as the workspace run.
- [ ] **Step 2:** Run them. Expected: FAIL.
- [ ] **Step 3:** Add the two commands. Their verdicts join their check's verdict.
- [ ] **Step 4:** `cargo nextest run -p xtask`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(xtask): build theos-editor with the engine feature`.

### Task A3: workflows, Linux packages, ruleset

**Files:**
- Create: `gate/linux-packages.txt`, with one package per line: `build-essential`,
  `pkg-config`, `libwebkit2gtk-4.1-dev`
- Modify: `.github/actions/setup/action.yml` (input `gui-packages`), `.github/workflows/ci.yml`
  (job `ui`; `gui-packages: true` on `gate`, `test` and `coverage`),
  `.github/workflows/platforms.yml` (`ui` in the `--only` lists of `windows` and `macos`;
  `actions/setup-node` before it), `gate/ruleset-main.json` (`ui` among the required checks)
- Test: `xtask/tests/workflows.rs`

- [ ] **Step 1: Failing tests** in `workflows.rs`:
  - `ci_has_a_ui_job`: the job is named `ui` and runs on `ubuntu-latest`. It uses
    `.github/actions/xtask`, `actions/setup-node` with `node-version: 24` and
    `cache: pnpm`, `cache-dependency-path: editor-ui/pnpm-lock.yaml`, and an `actions/cache` of
    `~/.cache/ms-playwright` keyed by `hashFiles('editor-ui/pnpm-lock.yaml')`. Its last step is
    `cargo xtask gate --only ui`;
  - `rust_jobs_install_the_gui_packages`: `gate`, `test` and `coverage` pass
    `gui-packages: true`;
  - `setup_installs_packages_from_the_list`: when `gui-packages` is `true`, the setup action
    restores `actions/cache` of `/var/cache/apt/archives/*.deb`, keyed by
    `runner.os`, `ImageOS`, `ImageVersion` and `hashFiles('gate/linux-packages.txt')`. It runs
    `apt-get install --no-install-recommends` with the file's packages, and saves the cache only
    when `save-cache` is `true`;
  - `platforms_run_ui_on_windows_and_macos`;
  - `ruleset_requires_ui`.

  Pin each new action by its full SHA with the version comment, as the file does.
- [ ] **Step 2:** `cargo nextest run -p xtask --test workflows`. Expected: FAIL.
- [ ] **Step 3:** Edit the workflow files to pass the tests.
- [ ] **Step 4:** `cargo nextest run -p xtask`. Expected: PASS.
- [ ] **Step 5:** Commit `ci(gate): the ui job and the Linux packages for Tauri`.

### Task A4: `deny.toml` and `docs/gate.md`

**Files:**
- Modify: `deny.toml`, `docs/gate.md` §1
- Test: `xtask/tests/settings.rs` (`gate_md_lists_the_code_paths_of_gate_toml`,
  `gate_md_explains_every_check_a_docs_change_skips` must keep passing, and the new `ui` row
  must be covered)

- [ ] **Step 1:** `deny.toml` `[advisories] ignore` gets two entries with the spec's reasons
  (§5.13): `RUSTSEC-2024-0370` and `RUSTSEC-2024-0436`. The licence list is unchanged.
- [ ] **Step 2:** `docs/gate.md` §1:
  - `ui` among the required checks of the first paragraph;
  - the `ui` row's text becomes "`pnpm install --frozen-lockfile`, `svelte-check`, `eslint`,
    `vitest`, Playwright tests";
  - one paragraph on the Linux packages (`gate/linux-packages.txt`, cached `.deb` files, the
    three jobs);
  - `clippy` and `test` name the extra `--features engine` run of `theos-editor`;
  - a row `ui` in the table of what a non-code change cannot alter: it reads `editor-ui/**` and
    `editor-ui/e2e/ipc/*.json`, all code, at `xtask/src/checks/ui.rs`.
- [ ] **Step 3:** `cargo nextest run -p xtask`. Expected: PASS, including the gate.md tests.
- [ ] **Step 4:** `cargo xtask gate` on Windows. Expected:
  - `ui`: not needed, "no editor-ui yet";
  - `deny` passes (the ignores are unused on this branch; `cargo deny` warns about unused
    ignores, which is not a failure);
  - every other check passes.
- [ ] **Step 5:** Commit `ci(gate): deny ignores for Tauri and the ui row of gate.md`.
- [ ] **Step 6:** Push once and open the PR with the template filled. Do not enable auto-merge.
  Tell Jann that merging needs `cargo xtask github apply` for the ruleset.

---

## Part B — the build (`phase/P11-editor-shell-and-codex`, `.worktrees/P11-editor-shell-and-codex`)

Tasks 1–5 need no Tauri. Tasks 6–14 add Tauri crates to `Cargo.lock`, so the local `deny` check
fails on this branch until Part A is merged into `main` and `main` is merged here. Until then,
verify each task with its own commands. Run the whole `cargo xtask gate` after that merge.

**Before Task 1:** check `origin/main` for `theos_studio::codex`. If P48's build has merged it,
skip Tasks 1 and 2, merge `origin/main`, and use it as is.

### Task 1: `Index::field_texts` (P48 §5.4)

**Files:**
- Modify: `crates/theos-content/src/index/query.rs`
- Test: in the same file's tests, with the index test fixtures

**Interfaces:**
- Produces:
  `pub fn field_texts(&self, field: &str) -> Result<Vec<FieldText>, IndexError>`;
  `pub struct FieldText { pub path: String, pub text: String }`, exported from `index`.

- [ ] **Step 1: Failing test `field_texts_returns_valid_scalars_by_path`:**
  - two truths with `target: never` and `target: full` give both rows, ordered by path;
  - a list field and an invalid scalar give no row.
- [ ] **Step 2:** Run it. Expected: FAIL.
- [ ] **Step 3:** Query `field_values` with `shape = 'scalar'`, `valid = 1` and no map entry,
  ordered by path.
- [ ] **Step 4:** `cargo nextest run -p theos-content`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(content): the valid scalar values of a field`, with
  `Refs: CE-33.1`.

### Task 2: `theos_studio::codex` — `RecordRef`, `Marks`, `search`, `read`, `list` (P48 §5.5–§5.7)

**Files:**
- Create: `crates/theos-studio/src/codex/{mod.rs,marks.rs,record_ref.rs,search.rs,read.rs,list.rs}`
- Modify: `crates/theos-studio/src/lib.rs`, `crates/theos-studio/Cargo.toml` (`schemars`),
  `Cargo.toml` (`schemars = "1"` in workspace dependencies)
- Test: `crates/theos-studio/tests/codex.rs` and fixture `crates/theos-studio/tests/fixtures/codex/`
  (synthetic; copy of Task 6's fixture, see there)

**Interfaces:**
- Produces, exactly the JSON of P48 §5.7, every type `Serialize + schemars::JsonSchema`,
  snake case, every key present, `null` for absent:
  - `Marks::load(&Index) -> Result<Marks, StudioError>`, `Marks::of_path(&str) -> Mark`,
    `Marks::of_id(&str) -> Mark`, `Mark { never: bool, locked: Vec<LockReason> }`, with
    `LockReason` = `canon` | `never` | `formalism` | `style`, in that order;
  - `RecordRef { id, path, kind, title, status, never, locked }`, with the mark fields
    flattened;
  - `codex::search(&mut Workspace, query: &str, limit: u32) -> Result<SearchView, StudioError>`,
    where `SearchView { query, hits: Vec<SearchHit>, truncated }`,
    `SearchHit { #[flatten] record: RecordRef, score: f64, matches: Vec<MatchView> }` and
    `MatchView { column, line, column_number, r#match, text }`;
  - `codex::read(&mut Workspace, by: ReadBy) -> Result<ReadView, StudioError>`, where
    `ReadBy::Id(String) | ReadBy::Path(String)`, `ReadView { records: Vec<Read> }` and
    `Read { #[flatten] record, text, links: Vec<LinkView>, backlinks: Vec<LinkView>,
    children: Vec<RecordRef>, findings: Vec<views::Finding>, lens: Option<LensView>,
    ledger: Option<views::LedgerEntry> }`;
  - `LinkView { slot: String, fragment, line, column, source: RecordRef, target: TargetRef }`,
    where `slot` is `Slot::as_db` text and `TargetRef` is `RecordRef` plus `exists: bool`;
  - `codex::list(&mut Workspace, filter: ListFilter, offset: u32, limit: u32) -> Result<ListView, StudioError>`.

- [ ] **Step 1: Failing tests,** one per P48 §5.14 shape that P11 relies on:
  - `marks_name_never_and_canon`;
  - `search_hits_carry_marks_and_matches`;
  - `read_has_text_links_backlinks_with_slots`: a backlink from a clue's `where` has slot
    `field:where`;
  - `read_dangling_target_has_exists_false`;
  - `list_filters_by_kind_with_total_and_truncated`;
  - `every_object_with_an_id_has_never_and_locked`: the schema walk over the JSON of all
    results.
- [ ] **Step 2:** `cargo nextest run -p theos-studio --test codex`. Expected: FAIL.
- [ ] **Step 3:** Implement as P48 §5.5–§5.7 says, each function in its file:
  - the sync comes first through `Workspace::write(|i| i.sync())`;
  - `text` comes from `read_record` after the sync;
  - `lens` and `ledger` come from the index's `lens` and P05's ledger view.
- [ ] **Step 4:** `cargo nextest run -p theos-studio`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(studio): the shared Codex queries`, with `Refs: CE-33.2, CE-33.3`.
  Tell the P48 session (theos-editor-88) that `codex` is built on this branch and will reach
  `main` with P11's PR.

### Task 3: `theos_content::body::tree` — the body as a block tree

**Files:**
- Create: `crates/theos-content/src/body/tree.rs`
- Modify: `crates/theos-content/src/body/mod.rs` (`pub mod tree;` plus re-exports)
- Test: unit tests in `tree.rs`

**Interfaces:**
- Produces, all `Clone + Debug + PartialEq + Eq`:
  - `pub fn tree(record: &Record) -> Vec<Node>`;
  - `pub enum Node`:
    - `Heading { level: u8, inlines: Vec<Inline> }`;
    - `Paragraph { inlines: Vec<Inline> }`;
    - `List { ordered: bool, items: Vec<Vec<Node>> }`;
    - `Quote { nodes: Vec<Node> }`;
    - `Code { text: String }`;
    - `Rule`;
    - `Voice { speaker: Link, beliefs: Vec<Link>, tier: Option<String>, nodes: Vec<Node>, caption: Option<Vec<Inline>> }`;
    - `Design { nodes: Vec<Node> }`;
    - `Note { state: Option<String>, title: Vec<Inline>, nodes: Vec<Node> }`;
    - `Callout { name: String, title: Vec<Inline>, nodes: Vec<Node> }`, for types without a
      grammar;
    - `Beat { heading: Vec<Inline>, attributes: Vec<(String, Vec<Inline>)>, nodes: Vec<Node> }`;
    - `Raw { text: String }`;
  - `pub enum Inline`: `Text(String)`, `Emphasis(Vec<Inline>)`, `Strong(Vec<Inline>)`,
    `Code(String)`, `Break`, `Link(Link)` (P02's `body::Link`), and
    `External { text: String, url: String }` for Markdown links and images, never fetched.

- [ ] **Step 1: Failing tests,** each a small record string:
  - `paragraph_with_emphasis_and_two_wikilinks`: the shown text, fragment and target are kept;
  - `heading_levels_and_lists_nested`;
  - `voice_block_has_speaker_beliefs_tier_and_caption`, from the data model §9.1 example
    shape, synthetic;
  - `design_and_note_callouts`;
  - `beat_heading_with_attribute_lines`;
  - `malformed_callout_title_is_raw_text`;
  - `link_inside_inline_code_is_code_not_link`;
  - `html_and_script_stay_text` (Review Focus 1): `<script>x</script>` and `<b>` in the body
    become `Text` or `Raw` with the exact characters, never dropped;
  - `crlf_record_gives_the_same_tree_as_lf`.
- [ ] **Step 2:** `cargo nextest run -p theos-content tree`. Expected: FAIL.
- [ ] **Step 3:** Implement over the crate's one pulldown-cmark pass (`body::events::Events`,
  which stays private) and `Body`'s blocks, callouts, beats, links and malformed lists:
  - wikilinks become `Inline::Link` by matching spans with `Body::links`;
  - callout titles and contents come from `Callout` spans;
  - a block covered by a `Malformed` entry becomes `Raw` with its source text;
  - HTML events become `Text` with their source.

  No second parser. That is why this lives in `theos-content` (P06 ruling 15 set the same
  precedent with `rename::edits`).
- [ ] **Step 4:** `cargo nextest run -p theos-content`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(content): a record body as a block tree`, with
  `Refs: CE-33.1`.

### Task 4: `theos_studio::parts` — the editor-part registry and the rail

**Files:**
- Create: `crates/theos-studio/src/parts/{mod.rs,codex.rs,atlas.rs,graph.rs,scripts.rs,concepts.rs,maps.rs,review.rs,changes.rs,mcp.rs,code.rs,settings.rs}`
- Modify: `crates/theos-studio/src/lib.rs`
- Test: unit tests in `parts/mod.rs`

**Interfaces:**
- Produces:
  - `pub struct RailEntry { pub id: &'static str, pub label: &'static str, pub glyph: &'static str, pub order: u8, pub phase: Option<&'static str> }`;
  - `pub struct Part { pub id: &'static str, pub rail: Option<RailEntry>, pub always: bool }`,
    where `always` is true for `code` and `settings`;
  - `pub fn content() -> Vec<Part>`, one line per module;
  - `pub fn rail(enabled: &[String]) -> Vec<RailEntry>`, which logs a `warn` for each unknown
    name.

- [ ] **Step 1: Failing tests:**
  - `rail_of_tower_of_theos`, with tower-of-theos's `enabled` (`codex`, `changes`, `scripts`,
    `graph`, `concepts`, `maps`, `review`, `mcp`), gives ids `codex`, `graph`, `scripts`,
    `concepts`, `maps`, `review`, `changes`, `code`, `settings`. The glyphs are ◈ ⋈ ¶ ◫ △ ◴ ±
    `</>` ⚙, and the phases are `None`, P15, P15, P16, P16, P13, P13, P18, P14;
  - `atlas_follows_codex_when_enabled`: Atlas comes right after Codex, with phase P25 and
    glyph `⌾` (screen 12's rail);
  - `unknown_names_are_skipped`;
  - `empty_enabled_gives_code_and_settings`.

  Tag the first two `// AC: CE-32.1`.
- [ ] **Step 2:** Run them. Expected: FAIL.
- [ ] **Step 3:** Implement. Each part module holds one `pub const PART: Part`.
- [ ] **Step 4:** `cargo nextest run -p theos-studio parts`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(studio): the editor-part registry and the rail`, with
  `Refs: CE-32.1`.

### Task 5: `theos_studio::codex_view` — tree, record view, query

**Files:**
- Create: `crates/theos-studio/src/codex_view/{mod.rs,tree.rs,record.rs,front_matter.rs,lore.rs,relation.rs,query.rs}`
- Test: `crates/theos-studio/tests/codex_view.rs`, on the Task 2 fixture

**Interfaces:**
- Consumes: `codex::{read, search, list}`, `Marks`, `RecordRef` (Task 2), `body::tree` (Task 3).
- Produces (plain Rust types, `Serialize`; `theos-editor` maps them):
  - `pub enum TreeTab { Where, Who, Lore, Things }`;
    `pub fn tree(&mut Workspace, TreeTab) -> Result<Vec<TreeNode>, StudioError>`, where
    `TreeNode { record: RecordRef, tag: Option<String>, children: Vec<TreeNode> }` or
    `TreeNode::Group { label, children }`;
  - `pub fn record(&mut Workspace, id: &str) -> Result<RecordPage, StudioError>`, where
    `RecordPage { header: Header, body: Vec<ViewNode>, lore: Vec<LoreRow>, pictures: Vec<PictureRow>, front_matter: Vec<FieldRow>, links_in: Vec<LinkView>, used_by: Vec<UsedBy>, findings: Vec<views::Finding> }`.
    `ViewNode` mirrors `body::tree::Node`, but each link is
    `ViewLink { id, fragment, shown, target: Option<RecordRef> }`;
  - `Header { record: RecordRef, parent: Option<RecordRef>, links_in: u32, children: u32 }`;
  - `LoreRow { record: RecordRef, verb: String, touches: Option<RecordRef>, tier: Option<String> }`;
  - `PictureRow { concept: RecordRef, picked: Option<String> }`;
  - `FieldRow { key: String, value: FieldValue }`, where `FieldValue` is one of
    `Text(String)`, `Link(ViewLink)`, `List(Vec<FieldValue>)`, `Tier(String)` or
    `Map(Vec<FieldRow>)`;
  - `UsedBy { source: RecordRef, relations: Vec<String> }`;
  - `pub fn relation(slot: &str) -> String`;
  - `pub fn query(&mut Workspace, text: &str, limit: u32) -> Result<QueryView, StudioError>`,
    where `QueryView { hits: Vec<SearchHit>, truncated: bool, note: Option<String> }`.

- [ ] **Step 1: Failing tests** in `codex_view.rs`:
  - **`tree`:**
    - `where_tab_nests_tower_band_floor_location`: the band's tag is `1–20` (en dash), and the
      floor's tag is `draft`;
    - `who_tab_nests_people_faction_npc`;
    - `lore_tab_groups_by_kind_sorted_by_id`;
    - `things_tab_has_only_registered_kinds`;
    - `dangling_parent_and_cycle_sit_at_top_level_once` (Review Focus 3);
    - `empty_project_gives_empty_tabs`.
  - **`record`**, tagged `// AC: CE-33.1`, on the floor `f04-synthetic`:
    - the header says `floor`, the parent band's title and `draft`;
    - the body order is paragraph (chips with target titles and kinds), heading, list (a lore
      chip `C-…`), voice, design, a dangling chip (`target: None`), then `Raw`;
    - the lore rows have the verbs `hints` and `distorts` and the `never` mark;
    - there is a picture row;
    - the front-matter keys are in file order, the `parent` value is a `Link`, and `tiers` is a
      `Tier`;
    - the findings include `link/dangling`.
  - **`record`**, tagged `// AC: CE-33.2`:
    - `used_by_is_derived`: the clue shows `where`, and the floor's bytes are hashed equal
      before and after;
    - after the clue's file is deleted and the workspace synced, it is gone;
    - after a new belief linking the floor is written and synced, it appears.
  - `relation_names`: the mapping of spec §5.8 for `field:found_at`, `field:where`,
    `field:about`, `field:parent` ("child of"), `beat:at`, `voice:speaker`, `lens:about`,
    `note:version` and `mention`.
  - **`query`**, tagged `// AC: CE-33.3`:
    - `#f04` gives the floor;
    - `:clue` gives both clues;
    - `:clue murmur` gives one;
    - `@` plus the npc's name gives the npc;
    - `:Floor` matches `floor`;
    - `:nokind` gives no hits and the note "no kind `nokind`".
  - **`query`** (Review Focus 4): `#`, `:` and `@` alone and `AND * -` give no error; an empty
    query gives no hits.
- [ ] **Step 2:** `cargo nextest run -p theos-studio --test codex_view`. Expected: FAIL.
- [ ] **Step 3:** Implement each file with one job:
  - `tree.rs` builds the tabs from `records_of_kind` and `children`, skips unregistered kinds,
    and guards against cycles with a visited set;
  - `record.rs` assembles the page;
  - `front_matter.rs` walks P02's `FrontMatter` in order;
  - `lore.rs` builds the rows from backlinks whose source kind is truth, belief or clue, with
    the verb from the source's linking field: `reveals` → `hints`, `distorts` → `distorts`,
    `topic` → `about`;
  - `relation.rs` maps slots to relations;
  - `query.rs` parses the prefixes (`#`, `:`, `@`) by splitting at whitespace and hands the
    rest to `codex::search`.
- [ ] **Step 4:** `cargo nextest run -p theos-studio`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(studio): the Codex views of the editor`, with
  `Refs: CE-33.1, CE-33.2, CE-33.3`.

### Task 6: the Tauri app crate

**Files:**
- Create in `crates/theos-editor/`: `build.rs`, `tauri.conf.json`, `capabilities/main.json`,
  `icons/icon.ico`, `icons/icon.png`, `windows-app-manifest.xml`, `.gitignore` (`gen/`),
  `src/lib.rs`, `src/state.rs` (the state file), `src/ipc/{mod.rs,error.rs,app.rs,project.rs}`,
  `tests/fixtures/codex/**` (the synthetic project, its README says synthetic),
  `tests/support/mod.rs`, `tests/app.rs`
- Modify: `crates/theos-editor/Cargo.toml`, `src/main.rs`, `README.md`; root `Cargo.toml`
  (workspace dependencies)

**Interfaces:**
- Consumes: `parts::rail` (Task 4), `Workspace::open`.
- Produces:
  - `pub fn builder<R: Runtime>(tauri::Builder<R>) -> tauri::Builder<R>`;
  - `pub fn specta_builder<R: Runtime>() -> tauri_specta::Builder<R>`, with every command;
  - `pub struct AppState { workspace: Mutex<Option<Workspace>>, last_sync: Mutex<Option<Instant>> }`;
  - `pub enum IpcError`, serialised `{ "kind": …, "message": … }`, with the variants
    `NoProject`, `NotAProject { path, message }`, `NotFound { id }`, `Index { message }` and
    `Io { path, message }`;
  - the commands `app_info() -> AppInfo { version, os, engine }`,
    `project_open(path) -> Result<ProjectView, IpcError>`, `project_last() -> Option<String>`
    and `project_sync() -> Result<SyncView, IpcError>`;
  - `ProjectView { name, root, kind, rail: Vec<RailView>, records: u32, errors: u32, warnings: u32, in_memory: bool }`;
  - `SyncView { project: ProjectView, changed: Vec<String>, removed: Vec<String> }`;
  - `pub fn engine_presets() -> Vec<&'static str>`, `#[cfg(feature = "engine")]` and empty,
    with a non-engine twin `content_presets()`;
  - `tests/support`: `app() -> (App<MockRuntime>, WebviewWindow<MockRuntime>)` and
    `call(&window, cmd, json) -> Result<Value, Value>`.
- The fixture holds what spec §5.12 lists (no spawn).

- [ ] **Step 1: Failing tests** in `tests/app.rs`:
  - `open_returns_the_rail_of_enabled_parts` (`// AC: CE-32.1`), on the fixture;
  - `open_a_folder_without_theos_toml_is_not_a_project` (`// AC: CE-32.1`): the message
    contains "has no theos.toml";
  - `commands_before_open_fail_with_no_project`;
  - `engine_feature_state` (`// AC: CE-32.3`): `app_info().engine == cfg!(feature = "engine")`,
    and with the feature `engine_presets()` is empty. The manifest's `engine` feature is `[]`,
    read with `cargo_metadata` (dev-dependency, already in the lock);
  - `path_with_spaces_and_non_ascii_opens` (Review Focus 5): a fixture copy at
    `tmp/"Projekt Ä"/`;
  - `held_cache_opens_in_memory`: a second `Workspace` holds the cache, and
    `in_memory == true`;
  - `state_file_round_trips`, in a temp data dir passed by an env var that only tests set.
- [ ] **Step 2:** `cargo nextest run -p theos-editor`. Expected: FAIL.
- [ ] **Step 3:** Implement as spec §5.2, §5.4 and §4.2 say:
  - `build.rs` with `WindowsAttributes::new_without_app_manifest()`, plus `/MANIFEST:EMBED` and
    `/MANIFESTINPUT` on windows-msvc;
  - `tauri.conf.json` with `devUrl` `http://localhost:5179`, `frontendDist`
    `../../editor-ui/dist`, the CSP and one 1440 × 900 window;
  - capabilities with only the app's commands and `dialog:allow-open`;
  - `main.rs` holds only the subscriber and `builder(...).run(generate_context!())`;
  - the dev-dependency is `tauri` with feature `test`;
  - `tracing` lines as spec §5.4 says: `info` for an opened project and a sync, with counts;
    `debug` for each command and its time; `warn` for skipped `enabled` names; `error` for every
    `IpcError` returned.
- [ ] **Step 4:** Run these and expect clean output and PASS:
  - `cargo clippy -p theos-editor --all-targets --locked -- -D warnings`;
  - the same with `--features engine`;
  - `cargo nextest run -p theos-editor`;
  - `cargo nextest run -p theos-editor --features engine`.
- [ ] **Step 5:** Commit `feat(editor): the Tauri app, project commands and the rail`, with
  `Refs: CE-32.1, CE-32.3`.

### Task 7: Codex commands

**Files:**
- Create: `crates/theos-editor/src/ipc/{codex.rs,views.rs}`
- Test: `crates/theos-editor/tests/codex.rs`

**Interfaces:**
- Consumes: `codex_view::{tree, record, query}` (Task 5), `support` (Task 6).
- Produces:
  - the commands `codex_tree(tab: TreeTabView) -> Result<Vec<TreeNodeView>, IpcError>`,
    `record_view(id: String) -> Result<RecordView, IpcError>` and
    `codex_search(query: String, limit: u32) -> Result<QueryResult, IpcError>`;
  - the view types in `views.rs`: specta mirrors of Task 5's types, built with `From` impls,
    with no `u64` and `f64` scores kept as `f64`.

- [ ] **Step 1: Failing tests,** each through IPC:
  - `record_view_of_the_floor_matches_screen_01` (`// AC: CE-33.1`): the same assertions as
    Task 5's record test, read from JSON;
  - `used_by_is_derived_through_ipc` (`// AC: CE-33.2`): delete the clue, call `project_sync`,
    and it is gone;
  - `search_prefixes_through_ipc` (`// AC: CE-33.3`);
  - `unknown_id_is_not_found`.
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement the thin commands.
- [ ] **Step 4:** `cargo nextest run -p theos-editor` (both feature states). Expected: PASS.
- [ ] **Step 5:** Commit `feat(editor): the Codex commands`, with
  `Refs: CE-33.1, CE-33.2, CE-33.3`.

### Task 8: generated bindings and recorded IPC

**Files:**
- Create: `crates/theos-editor/tests/bindings.rs`, `crates/theos-editor/tests/recordings.rs`,
  `editor-ui/src/lib/ipc/bindings.ts` (generated), `editor-ui/e2e/ipc/*.json` (generated)

**Interfaces:**
- Produces:
  - `bindings.ts`, exported with `ErrorHandlingMode::Result`;
  - a recording file per call: `{ "cmd": …, "args": …, "result": "ok" | "error", "value": … }`,
    named `<cmd>[-<variant>].json`. Variants: `app_info-windows`, `app_info-macos`,
    `app_info-linux`, `project_open`, `codex_tree-where|who|lore|things`,
    `record_view-<id>` for every fixture record, and `codex_search-<slug>` for each page-test
    query (`f04`, `clue`, `murmur`, the npc's name, `nokind`).
  - Variants for the edge states the page tests need:
    - `project_last-none`;
    - `project_open-in-memory`, with the cache held by a second `Workspace` during the call;
    - `project_sync-changed` and `project_sync-removed`;
    - `codex_tree-who-empty`, from a second, empty synthetic fixture
      `tests/fixtures/empty/` with only `theos.toml`.

    Later tasks that need a new variant add it here and regenerate.
  - Project roots in recordings are replaced by the placeholder `<project>` with `/`
    separators, so they are equal on every OS.

- [ ] **Step 1: Failing tests:**
  - `bindings_are_current` (`// AC: CE-32.4`): export to `CARGO_TARGET_TMPDIR` and compare
    bytes with the checked-in file. On a difference, fail with "editor-ui/src/lib/ipc/bindings.ts
    is stale: run `UPDATE_BINDINGS=1 cargo test -p theos-editor bindings`". With
    `UPDATE_BINDINGS=1`, write the file and pass;
  - `a_changed_byte_is_stale` (`// AC: CE-32.4`): the comparison function on two strings
    differing in one byte reports stale;
  - `ipc_recordings_are_current`: the same pattern over `e2e/ipc/`, which also fails on an
    extra file no call writes.
- [ ] **Step 2:** Run without `UPDATE_BINDINGS`. Expected: FAIL (no files).
- [ ] **Step 3:** Implement both tests. Then generate the files with
  `UPDATE_BINDINGS=1 cargo nextest run -p theos-editor --test bindings --test recordings`.
- [ ] **Step 4:** Run without the variable. Expected: PASS. Run it twice and expect the same
  bytes.
- [ ] **Step 5:** Commit `feat(editor): IPC bindings and recordings generated from Rust`, with
  `Refs: CE-32.4`.

### Task 9: `editor-ui` scaffold and its checks

**Files:**
- Create in `editor-ui/`:
  - `package.json` (scripts `dev` on port 5179 strict, `build`, `preview`, `app`; exact versions;
    `packageManager` `pnpm@12.23.0`; `engines.node` `>=24`), `pnpm-lock.yaml`,
    `pnpm-workspace.yaml`;
  - `vite.config.ts`, `svelte.config.js`, `tsconfig.json` (strict), `eslint.config.js`,
    `vitest.config.ts`, `playwright.config.ts`;
  - `index.html` (`<html data-theme="grid-noir">`), `src/main.ts`, `src/App.svelte`;
  - `src/lib/ipc/client.ts`;
  - `e2e/support/replay.ts`;
  - `tests/styles.test.ts`, `tests/lint.test.ts`;
  - `README.md`.

**Interfaces:**
- Produces:
  - `client.ts` with `call<T>(r: Promise<Result<T, IpcError>>): Promise<T>`, which throws
    `IpcFailure { kind, message }`;
  - `replay.ts` with `replay(page, { os })`, which installs `mockIPC` before the app loads and
    answers each call from `e2e/ipc/`, matched on cmd and args. An unmatched call fails the
    test with its cmd and args.

- [ ] **Step 1: Failing tests:**
  - `tests/styles.test.ts`: every `.svelte` file's `<style>` and `style=` attribute has no hex
    colour, `rgb(`, `hsl(`, named colour, `font-family` other than `var(--…)`, `opacity`, or
    `px` font size. It also fails on a fixture string that has one of each;
  - `tests/lint.test.ts` (`// AC: CE-32.4`): ESLint's API on a fixture file that imports
    `invoke` from `@tauri-apps/api/core` outside `src/lib/ipc/` gives the
    `no-restricted-imports` error, and the same import inside `src/lib/ipc/` gives none;
  - a third case: `{@html` in a fixture `.svelte` gives an error from
    `svelte/no-at-html-tags`.
- [ ] **Step 2:** `pnpm exec vitest run`. Expected: FAIL.
- [ ] **Step 3:** Configure ESLint with `typescript-eslint`, `eslint-plugin-svelte`, the
  restricted import and `svelte/no-at-html-tags`; `bindings.ts` is ignored for stylistic rules.
  `main.ts` imports `../../design/theos-ui/tokens.css` and `components/bundle.css`, and
  `vite.config.ts` allows that folder in `server.fs.allow`.
- [ ] **Step 4:** Run these and expect each to pass or be clean:
  - `pnpm install --frozen-lockfile`;
  - `pnpm exec svelte-check --fail-on-warnings`;
  - `pnpm exec eslint .`;
  - `pnpm exec vitest run`;
  - `pnpm build`.
- [ ] **Step 5:** Commit `feat(ui): the editor-ui package and its checks`, with
  `Refs: CE-32.4`.

### Task 10: the shell — title bar, toolbar, rail, dock, status bar, placeholders

**Files:**
- Create:
  - `editor-ui/src/shell/{Shell.svelte,TitleBar.svelte,Toolbar.svelte,Rail.svelte,Dock.svelte,StatusBar.svelte,EmptyWork.svelte}`;
  - `src/parts/presets.ts`;
  - `src/parts/placeholder/{atlas,graph,scripts,concepts,maps,review,changes,code,settings}.ts`;
  - `src/parts/placeholder/EmptyWindow.svelte`;
  - `src/lib/state/project.svelte.ts`;
  - `src/components/{Window.svelte,TabBar.svelte,Button.svelte,Seg.svelte,KindGlyph.svelte,Pip.svelte}`;
  - `e2e/shell.spec.ts`.

**Interfaces:**
- Produces:
  - `Preset { id: string; windows: Split }`, where
    `Split = { dir: 'row' | 'column'; items: Array<{ flex: string; child: Split | WindowSpec }> }`
    and `WindowSpec = { id: string; tabs: string[]; component: Component; props?: object }`;
  - `presets: Record<string, Preset>`, one line per preset;
  - `project` state: `{ view: ProjectView | null; active: string; error: string | null }`.

- [ ] **Step 1: Failing page tests** in `shell.spec.ts`, all with `replay`:
  - `opens_a_project_and_shows_the_rail` (`// AC: CE-32.1`): the rail labels in order are
    Codex, Graph, Scripts, Concepts, Maps, Review, Changes, Code, Settings; Codex has
    `aria-current="true"`; the title bar shows the project name and "layout · Codex";
  - `unbuilt_entries_show_their_layout` (`// AC: CE-32.1`): for each unbuilt entry, clicking it
    shows its windows, and each window's text names its phase (e.g. "Arrives with P13");
  - `no_project_offers_open` (`// AC: CE-32.1`): a recording with `project_last` returning
    `null` shows "Open a content project" and the platform key;
  - `disabled_controls_name_their_phase`: Write, Source, Save, New record and Open in graph are
    disabled with "arrives with P12" or "P15" in their title;
  - `bars_show_the_counts`:
    - the title bar's validator pip reads "validator · N errors" (or "· N warn", or "clean")
      from the recording;
    - the status bar reads `THEOS//EDITOR`, the version, "Codex layout" and
      "N records · N errors · N warnings";
    - a recording with `in_memory: true` shows "index in memory: another program holds the
      cache".
- [ ] **Step 2:** `pnpm exec playwright test shell`. Expected: FAIL.
- [ ] **Step 3:**
  - Build the components with the `tu-*` classes of screen 01.
  - Write the placeholder presets from the reference screens listed in spec §5.9: read each
    screen for its windows and tab titles, and write each empty state as Theos UI's "Empty
    states" asks.
  - `App.svelte` calls `project_last` and then `project_open` on start.
- [ ] **Step 4:** Run all ui steps of Task 9's Step 4 plus `pnpm exec playwright test`.
  Expected: PASS.
- [ ] **Step 5:** Commit `feat(ui): the shell, the rail and the layouts of the unbuilt parts`,
  with `Refs: CE-32.1`.

### Task 11: keymap, commands, palette

**Files:**
- Create: `editor-ui/src/lib/keymap.ts`, `src/lib/commands.ts`, `src/shell/Palette.svelte`,
  `tests/keymap.test.ts`, `e2e/palette.spec.ts`
- Modify: `Shell.svelte` (one key listener), `Toolbar.svelte` (Save label from the keymap)

**Interfaces:**
- Produces:
  - `type Os = 'windows' | 'macos' | 'linux'`;
  - `keymap: ReadonlyArray<{ command: CommandId; keys: string[] }>`;
  - `label(keys: string, os: Os): string` (`Ctrl+K` or `⌘K`);
  - `match(e: KeyboardEvent, os: Os): CommandId | null`;
  - `commands: Record<CommandId, { label: string; run: () => void; enabled: () => boolean; phase?: string }>`.
  - The command ids and keys are exactly those of spec §5.11's table.

- [ ] **Step 1: Failing tests:**
  - `keymap.test.ts` (`// AC: CE-32.2`):
    - `Mod` is Control on `windows` and `linux` and Meta on `macos`;
    - `Ctrl+Tab` is Control on all three;
    - no two commands share keys on any OS;
    - every command with keys is in `commands`;
  - `palette.spec.ts` (`// AC: CE-32.2`):
    - with `os = windows`: Control+K opens the palette; typing `lore` narrows it to "Tree tab:
      Lore"; Enter switches the tree tab; Escape closes the palette and returns the focus;
      Control+P focuses the find field; Control+2 switches the rail to Graph; Control+W closes
      the active record tab;
    - with `os = macos`: Meta+K opens the palette and Control+K does not;
    - Save is listed, dimmed, with "arrives with P12", and Enter on it does nothing;
    - Mod+K while the find field has focus still opens the palette;
    - Control+Tab with one tab and Mod+W with none do nothing and raise no error.
- [ ] **Step 2:** Run vitest and playwright. Expected: FAIL.
- [ ] **Step 3:** Implement. Use `tinykeys` only if its `$mod` can be set from `Os`. Otherwise
  write `match` over the table, about 30 lines, and note the choice in the commit body. Filter
  the palette with `fuzzysort`.
- [ ] **Step 4:** All ui steps. Expected: PASS.
- [ ] **Step 5:** Commit `feat(ui): one keymap, the commands and the palette`, with
  `Refs: CE-32.2`.

### Task 12: the Codex screen

**Files:**
- Create in `editor-ui/src/parts/codex/`: `preset.ts`, `CodexTree.svelte`, `TreeRow.svelte`,
  `RecordWindow.svelte`, `RecordHeader.svelte`, `Body.svelte`, `Inline.svelte`, `VoiceBlock.svelte`,
  `Callout.svelte`, `BeatRow.svelte`, `LinkChip.svelte`, `LoreTable.svelte`, `Pictures.svelte`,
  `FrontMatter.svelte`, `UsedBy.svelte`, `Validation.svelte`, `tabs.svelte.ts`
- Create: `editor-ui/e2e/codex.spec.ts`

**Interfaces:**
- Consumes: `commands.codexTree`, `commands.recordView` (bindings), `replay`.
- Produces: `openRecord(id: string)` in `tabs.svelte.ts`, which opens or focuses a tab, sets
  the tree tab and selection, and expands the ancestors. Task 13 uses it.
  `LinkChip`, `VoiceBlock`, `Callout` and `BeatRow` take plain props and can be mounted alone,
  because P12's CodeMirror widgets will mount them.

- [ ] **Step 1: Failing page tests** in `codex.spec.ts`:
  - `floor_shows_as_on_screen_01` (`// AC: CE-33.1`): click the floor in Where, and check:
    - `.tu-rechead__id` has the id and `.tu-rechead__title` the title;
    - the paragraph's `.tu-link` chips have the targets' titles;
    - the lore chip has its id and title;
    - `.tu-voice` has the speaker and the caption;
    - the lore table has the rows with "hints", "distorts" and "· never";
    - the picture caption has "Images arrive with P16";
    - the `.tu-fm__key` order equals the recording's order;
    - `.tu-val` has the `link/dangling` message;
    - the dangling chip has the class `is-reject` and ✕;
    - the theme attribute is `grid-noir`;
    - the Where tab's rows have their tags (`1–20`, `draft`), and its footer has screen 01's
      three lines;
  - `backlinks_show_and_open` (`// AC: CE-33.2`): Used by lists the clue with "where", and
    Links in · N has N rows; clicking the clue opens its tab;
  - `html_in_records_is_text` (Review Focus 1): a fixture record whose title and body hold
    `<img src=x onerror=…>` and `<script>` shows them as literal text, and no `img` or `script`
    element exists in the record window;
  - `empty_tab_explains`: a recording of an empty `who` tab shows the empty-state text;
  - `never_truth_is_marked_everywhere`: the tree row, chip and lore row of the `never` truth
    carry the mark.
- [ ] **Step 2:** `pnpm exec playwright test codex`. Expected: FAIL.
- [ ] **Step 3:** Implement with screen 01's structure and classes:
  - chrome is Instrument, the record body is `tu-manuscript`;
  - the widths are 248 px and 320 px, set as tokens or flex values, never as colours or fonts;
  - empty sections are not drawn.

  Add the Review Focus 1 record to the Rust fixture (Task 6), regenerate the recordings
  (Task 8), and commit them with this task.
- [ ] **Step 4:** Run all ui steps, `cargo nextest run -p theos-editor` and the recordings test.
  Expected: PASS.
- [ ] **Step 5:** Commit `feat(ui): the Codex screen`, with `Refs: CE-33.1, CE-33.2`.

### Task 13: find a record

**Files:**
- Create: `editor-ui/src/shell/Find.svelte`, `editor-ui/e2e/search.spec.ts`,
  `tests/find.test.ts`
- Modify: `Toolbar.svelte`

- [ ] **Step 1: Failing tests:**
  - `search.spec.ts` (`// AC: CE-33.3`):
    - typing `:clue` shows two hits;
    - ArrowDown twice and Enter opens the second clue in a tab, switches the tree to Lore and
      selects its row with the group expanded;
    - Escape closes the results;
    - `:nokind` shows "no kind `nokind`";
  - `find.test.ts`: with a fake call that resolves out of order, only the last query's hits are
    shown (Review Focus 4), and the debounce is 120 ms (fake timers).
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement. Hits show the glyph, id, title and the first match's line, with the
  match marked by a `<mark>` element around the text slice (no HTML from data). Show "more…"
  when `truncated`.
- [ ] **Step 4:** All ui steps. Expected: PASS.
- [ ] **Step 5:** Commit `feat(ui): find a record`, with `Refs: CE-33.3`.

### Task 14: freshness, opening, README, evidence

**Files:**
- Modify: `crates/theos-editor/src/ipc/project.rs` (sync throttle), `editor-ui/src/App.svelte`,
  `editor-ui/src/lib/state/project.svelte.ts`, `crates/theos-editor/README.md`,
  `editor-ui/README.md`
- Create: `crates/theos-editor/tests/sync.rs`, `editor-ui/e2e/freshness.spec.ts`

- [ ] **Step 1: Failing tests:**
  - `sync.rs`:
    - `focus_sync_at_most_once_a_second`: two `project_sync` calls within 1 s; the second
      returns the cached view without syncing (a counter in the test state);
    - `deleted_record_is_reported_removed`;
    - `changed_record_is_reported_changed`;
  - `freshness.spec.ts` (Review Focus 2): a recording sequence where `project_sync` reports the
    open record removed, then the window focus event fires. The tab closes, and the status bar
    says "`<id>` was deleted outside the editor". For a changed record, the view reloads.
  - Ctrl+O calls the dialog plugin: mocked, answering a path, then `project_open` with it.
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement:
  - the throttle lives in `AppState.last_sync`;
  - the UI listens to `tauri://focus` through `@tauri-apps/api/window`;
  - "Open project…" uses `@tauri-apps/plugin-dialog`'s `open({ directory: true })`.

  The READMEs say how to run the app: `pnpm --dir editor-ui app`, or the two terminals.
- [ ] **Step 4:** Run everything:
  - `cargo nextest run -p theos-editor` in both feature states;
  - all ui steps;
  - start the app on Windows with `pnpm --dir editor-ui app` against the fixture, and take two
    screenshots, the floor and the Review placeholder, into
    `C:\Users\janne\AppData\Local\Temp\claude\…\scratchpad\p11-shots\` for the PR.
- [ ] **Step 5:** Commit `feat(editor): sync on focus and open a project`, with
  `Refs: CE-32.1`.

### Task 15: whole-gate run and status

- [ ] **Step 1:** When Part A has merged: `git merge origin/main` (keep the newest state of
  every `docs/status.md` row).
- [ ] **Step 2:** `cargo xtask gate` on Windows. Expected: every check passes, `ui` included.
  `cargo xtask ac-trace --phase P11` shows a test for each of CE-32.1–32.4 and CE-33.1–33.3.
- [ ] **Step 3:** The final review loop (CLAUDE.md), then the PR. Use the labels `windows` and
  `macos`, set P11 to `pr-open` in `docs/status.md`, and attach the screenshots beside
  `docs/ui/screens/01-main.png`. Under "Needs Jann": a manual start on macOS if none was
  possible.
