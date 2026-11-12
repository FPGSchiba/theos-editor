# P06 · Command layer and repo lock — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (CLAUDE.md) to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** every write to a content project's files runs as one command through one pipeline in
`theos-studio`, checked as a whole, undoable to the exact bytes, and kept apart from other
processes' writes by the repo lock.

**Architecture:** a command builds a plan (the files before and after). The pipeline:
1. takes the repo lock;
2. checks the bases;
3. previews the plan with P03's index;
4. rejects it when it introduces an error (counted per path and rule);
5. writes each file whole, with rollback;
6. reindexes;
7. hands the result to a route.

The human route keeps an undo journal in memory. A rename is one command, built by participants
(versions, children, ids, links), with link edits from a new public function in `theos-content`.

**Tech stack:** Rust 2024 (toolchain 1.99.1). `theos-content` (P02–P04). Standard library:
`std::fs::File::try_lock` and `std::fs::rename`. Crates: `tempfile` (`persist_noclobber`), `sha2`,
`serde`, `serde_json`, `thiserror`, `tracing`, and `proptest` (dev). All of them are already in
`Cargo.lock`.

**Spec:** `docs/specs/phases/P06-command-layer-and-repo-lock.md` (the authority; read it with this
plan). Interfaces shared with P05: `docs/specs/phases/P05-cli-and-hook.md` §4.2, §5.3, §6.

## Global Constraints

Umbrella G1–G16, verbatim:

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

The phase's acceptance criteria, verbatim:

- CE-20.1 A blocking finding rejects the command and leaves files untouched.
- CE-20.2 Undo restores the bytes.
- CE-20.3 Renaming an id updates every inbound link in one command.
- CE-22.2 The repo lock keeps two writers apart across processes.

Project rules every task follows (CLAUDE.md, `docs/gate.md`):
- **Engineering directives** 1–4 of CLAUDE.md go into every subagent brief.
- **Lints and attributes:**
  - no `unwrap`/`expect` outside tests;
  - no `#[allow]`; an `#[expect]` or `#[ignore]` needs a real reason;
  - `print_stdout`, `print_stderr` and `clippy::exit` are denied, tests included;
  - clippy pedantic runs with `-D warnings`;
  - `missing_docs` is denied, so every public item is documented.
- **Coverage floor** for library crates: 80 % (`gate/thresholds.toml`). Error paths are tested.
- **Text and paths:** files we write use LF. Paths go through `std::path`. Code that differs per
  OS gets a test on each OS (`#[cfg(unix)]` / `#[cfg(windows)]`).
- **AC tests:** a test that proves an AC carries `// AC: <id>` directly above `#[test]`.
- **Commits:** Conventional Commits, scope `studio` or `content`. Footers:

  ```text
  Refs: <ACs>
  Phase: P06
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  ```

  A footer line is at most 100 characters.
- **Git in worktrees:** one git command per call.
- **TDD:** run the RED step for real and read its failure before writing the code.

## Review Focus

The inputs most likely to bite a user that the AC tests do not exercise. Each has its test in
the owning task.

1. A file changed by another program between the plan and the write. Expected: `Stale`, nothing
   written. Task 6, `a_file_changed_after_the_plan_is_stale`.
2. A target held open without delete sharing on Windows, or in an unwritable folder on Unix.
   Expected: the whole command rolled back, with a message naming the file. Task 5,
   `a_failure_rolls_back_every_written_file`.
3. Paths that leave the project, are absolute, enter `.git`, or use `\`. Expected: `Path` before
   any read; `\` is read as `/`. Task 4, `paths_outside_the_project_are_refused`.
4. A rename whose child folder holds a file that is not a record and a hidden file, and a new id
   that only an ignored file has. Expected: every file moves, and `IdTaken`. Task 8,
   `a_rename_moves_every_file_of_the_folder` and `an_id_taken_by_an_ignored_file_is_refused`.
5. An unchanged save of a CRLF record. Expected: nothing written, no journal entry, and the
   bytes stay CRLF. Task 6, `an_unchanged_save_writes_nothing`.

---

## File structure

```text
crates/theos-content/src/rename.rs            new: rename::edits (Task 1)
crates/theos-content/src/lib.rs               pub mod rename; crate docs
crates/theos-content/tests/index/rename.rs    behaviour tests of rename::edits
crates/theos-studio/Cargo.toml                dependencies
crates/theos-studio/README.md, src/lib.rs     crate docs, modules, re-exports
crates/theos-studio/src/lock.rs               repo lock (Task 2)
crates/theos-studio/src/workspace.rs          P05's (Task 3), + one field
crates/theos-studio/src/introduced.rs         FindingCounts, introduced (Task 3; P05 or P06)
crates/theos-studio/src/command/mod.rs        Command, Reads, PlanContext, Plan, FileChange, Outcome
crates/theos-studio/src/command/error.rs      CommandError
crates/theos-studio/src/command/base.rs       Base
crates/theos-studio/src/command/path.rs       project path checks
crates/theos-studio/src/command/{create,write,edit,patch,delete}.rs
crates/theos-studio/src/command/files.rs      whole-file writes, rollback (Task 5)
crates/theos-studio/src/command/state.rs      CommandState: journal, lock flag, lock wait
crates/theos-studio/src/command/route.rs      Route, Applied, HumanRoute
crates/theos-studio/src/command/verdict.rs    the verdict of a preview (Task 6)
crates/theos-studio/src/command/pipeline.rs   the steps of spec §5.4
crates/theos-studio/src/command/run.rs        impl Workspace { read, run, run_with, set_lock_wait }
crates/theos-studio/src/command/journal.rs    Journal, EntryId, EntrySummary (Tasks 6, 7)
crates/theos-studio/src/command/undo.rs       impl Workspace { undo, redo, … } (Task 7)
crates/theos-studio/src/command/rename/{mod,participant,versions,children,ids,links}.rs
crates/theos-studio/tests/studio/main.rs      one test binary, modules below
crates/theos-studio/tests/studio/{common,helper,lock,commands,verdict,pipeline,undo,rename,writers}.rs
crates/theos-studio/tests/fixtures/{commands,standing}/   synthetic projects (Task 6)
```

---

### Task 1: `theos_content::rename::edits`

The edits that rename ids in one record, spelling each front-matter scalar in its own style
(spec §5.8 step 5, ruling 15). Nothing in `theos-studio` yet.

**Files:**
- Create: `crates/theos-content/src/rename.rs`
- Modify: `crates/theos-content/src/lib.rs` (`pub mod rename;` and an "After P06" paragraph in
  the crate docs)
- Test: `crates/theos-content/tests/index/rename.rs` (+ `mod rename;` in
  `tests/index/main.rs`)

**Interfaces:**
- Consumes:
  - `Record` (`text()`, `front_matter()`, `body().links()`);
  - `LinkRef { slot, span, target, .. }` (`index::query`);
  - the crate-private `yaml::encode(text, Some(style), yaml::Slot::{Field, FlowItem, BlockItem,
    Entry})`;
  - `TextEdit { span, replacement }`.
- Produces:

```rust
/// Old id → new id.
pub type IdMap = std::collections::BTreeMap<String, String>;

/// The edits that rename the ids of `renames` in `record`: its `id` field when its id is a key,
/// and every link of `links` (the index's links of this record, `Index::links_from`) whose target
/// is a key. A front-matter link is the scalar alone, respelled in its own style for its place;
/// a body link changes only its `target_span`. Sorted by start, never overlapping.
pub fn edits(record: &Record, links: &[LinkRef], renames: &IdMap) -> Result<Vec<TextEdit>, RenameError>;

#[derive(Debug, thiserror::Error)]
pub enum RenameError {
    #[error(transparent)] Encode(#[from] EncodeError),
    /// The index names a front-matter link at bytes no scalar of the record has.
    #[error("no scalar of the front matter sits at bytes {}..{}", .span.start, .span.end)]
    NoScalar { span: Span },
    /// The index names a body link at bytes no link of the record has.
    #[error("no link of the body sits at bytes {}..{}", .span.start, .span.end)]
    NoLink { span: Span },
}
```

`Slot::Field(_)` and `Slot::LensEntry(_)` are front-matter links. Every other slot is a body link.
A scalar is found by span among: field values, flow and block list items, and map entry values
(scalars and flow list items). Its place gives the encoder slot.

- [ ] **Step 1: Write the failing tests** in `tests/index/rename.rs`. Each test:
  1. builds a record with `TempProject::new()` and `Index::in_memory` + `apply(&[Change::text(path,
     text)])`;
  2. takes `links_from(path)`;
  3. calls `edits` and `apply_edits`, and compares the whole text.

  Every test carries `// AC: CE-20.3`.
  - `renames_the_id_field_in_its_style`: `id: T-960`, `id: 'T-960'` and `id: "T-960"` become
    `T-970` in the same style.
  - `renames_a_field_link_and_keeps_its_quoting`: a clue's `reveals: "T-960"` becomes
    `reveals: "T-970"`, and `reveals: T-960` becomes `reveals: T-970`.
  - `renames_list_items_and_keeps_comments`:
    - `where: [f04-synthetic, f05-synthetic]` → `[f04-renamed, f05-synthetic]`;
    - a block list item `  - f04-synthetic  # synthetic comment` keeps its comment and
      indentation.
  - `renames_lens_entries`: an npc's `lens:` / `unaware_of: [T-960]` and `rejects: ['B-960']`.
  - `renames_body_links_and_keeps_fragment_and_shown_text`:
    - `[[T-960#2|the truth]]` → `[[T-970#2|the truth]]`;
    - `[[T-960]] and [[T-960]]` on one line, both rewritten;
    - a beat `at:: [[f04-synthetic]]`;
    - a voice block's speaker and belief (data model §9.1);
    - a note title's version link and `resolved by [[…]]` (§9.3).
  - `leaves_text_that_only_looks_like_an_id`: `title: T-960` and links to other ids are
    unchanged; the edits touch only the expected spans (compare the text outside them byte for
    byte).
  - `a_span_the_record_does_not_have_is_an_error`: a `LinkRef` with a moved span gives `NoScalar`
    (front-matter slot) or `NoLink` (body slot).

- [ ] **Step 2: Run them and read the failure**
  - Run: `cargo nextest run -p theos-content --locked rename`
  - Expected: compile error, "unresolved import `theos_content::rename`".

- [ ] **Step 3: Implement `rename::edits` and `RenameError`** in `src/rename.rs`. Find a scalar
  by an exact span match; a body `Link` by `link.span == link_ref.span`.

- [ ] **Step 4: Run the tests until they pass**
  - Run: `cargo nextest run -p theos-content --locked`
  - Expected: all pass, including the 1201 earlier tests of the workspace (`cargo nextest run
    --workspace --locked`).

- [ ] **Step 5: Commit** — `feat(content): rename ids in a record as span edits`, with
  `Refs: CE-20.3`.

### Task 2: The repo lock

`theos-studio` gets its first code: the lock of spec §5.9, and the two-process test harness.

**Files:**
- Modify: `crates/theos-studio/Cargo.toml`:
  - dependencies: `theos-content` (path), `serde` (derive), `serde_json`, `sha2`, `tempfile`,
    `thiserror` and `tracing`, all `workspace = true`;
  - dev-dependencies: `proptest` and `tempfile`.
- Modify: `crates/theos-studio/src/lib.rs` (crate docs: what P06 adds; `pub mod lock;`)
- Modify: `crates/theos-studio/README.md` (no longer "empty")
- Create: `crates/theos-studio/src/lock.rs`
- Create: `crates/theos-studio/tests/studio/main.rs`, `common.rs`, `helper.rs`, `lock.rs`

**Interfaces:**
- Produces:

```rust
/// How long a command waits for the lock.
pub const COMMAND_WAIT: Duration = Duration::from_secs(10);
/// Takes the repo lock `<project_root>/.theos/cache/lock` (folder and file created when
/// missing), trying again with a pause of 10 ms doubling up to 200 ms until `wait` has passed.
/// Writes `.theos/cache/lock.owner` after taking it.
pub fn acquire(project_root: &Path, purpose: &str, wait: Duration) -> Result<LockGuard, LockError>;
/// Holds the lock; dropping it deletes the owner file, then unlocks.
pub struct LockGuard { /* the locked File, the owner path */ }
/// Who holds the lock, as the owner file says.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Owner { pub pid: u32, pub program: String, pub purpose: String, pub since: u64 /* unix s */ }
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    /// "the project is busy: {program} (pid {pid}) has been running `{purpose}` for {n} s", or
    /// "the project is busy: another theos process holds .theos/cache/lock" without a readable owner.
    Busy { owner: Option<Owner>, waited: Duration },
    /// "this workspace already holds the repo lock"
    Reentrant,
    /// "cannot use the repo lock {path}: {source}"
    Io { path: PathBuf, source: std::io::Error },
}
```

- **Test harness (`tests/studio/`):**
  - `common.rs`:
    - `TempProject::new()` / `copy_of(name)` / `root()`, as `theos-content`'s
      `tests/index/common.rs` does;
    - `files(root) -> BTreeMap<String, Vec<u8>>` (every file under the root except
      `.theos/cache`);
    - `spawn_helper(task: &HelperTask) -> Child`. It runs `std::env::current_exe()` with the
      arguments `helper::helper --exact` and the environment variable `THEOS_STUDIO_HELPER` set
      to the task as JSON.
  - `helper.rs`:
    - `#[test] fn helper()` returns at once when `THEOS_STUDIO_HELPER` is unset; otherwise it runs
      the task;
    - `HelperTask::Hold { root, purpose, ready, release }`: acquire, write the file `ready`, wait
      until the file `release` exists (up to 30 s), then drop the guard;
    - Task 9 adds `Append`.

    The helper signals through files, never stdout, because `print_stdout` is denied.

- [ ] **Step 1: Write the failing tests** in `tests/studio/lock.rs`:
  - `// AC: CE-22.2` `a_second_process_waits_and_names_the_holder`:
    1. a helper holds the lock with purpose "synthetic hold";
    2. `acquire(root, "test", 300 ms)` is `Busy` with `owner.pid == child.id()` and the purpose
       "synthetic hold", and `waited >= 300 ms`;
    3. after the release file, `acquire(.., 5 s)` is `Ok`.
  - `// AC: CE-22.2` `a_killed_holder_frees_the_lock_at_once`: a helper holds the lock;
    `child.kill()` and `wait()`; then `acquire(root, "test", Duration::ZERO)` is `Ok`.
  - `the_owner_file_goes_with_the_guard` (untagged): while held, `lock.owner` parses as an
    `Owner` with this pid. After the drop it is gone.
  - `busy_without_an_owner_file_says_so` (untagged): a lock held with the owner file deleted
    gives `Busy { owner: None, .. }` and the second message.
  - `acquire_creates_the_cache_folder` (untagged).

- [ ] **Step 2: Run them and read the failure**
  - Run: `cargo nextest run -p theos-studio --locked lock`
  - Expected: compile error, "unresolved import `theos_studio::lock`".

- [ ] **Step 3: Implement** `acquire`, `LockGuard` (`Drop`), `Owner` and `LockError` with
  `File::try_lock`. `TryLockError::WouldBlock` means wait; `TryLockError::Error` is `Io`.
  `program` is the file stem of `std::env::current_exe()`, else `"theos"`.

- [ ] **Step 4: Run the tests until they pass** — `cargo nextest run -p theos-studio --locked`
  and `cargo clippy -p theos-studio --all-targets --locked -- -D warnings`.

- [ ] **Step 5: Commit** — `feat(studio): the repo lock across processes`, with
  `Refs: CE-22.2`.

### Task 3: The workspace and `introduced`

P05 builds `Workspace` (P05 spec §5.3) and `introduced` (P05 spec §4.2). Whichever phase is first
writes them. P06 then adds one field.

**Files:**
- `crates/theos-studio/src/workspace.rs` and `introduced.rs`: from `origin/main`, or written here.
- Create: `crates/theos-studio/src/command/mod.rs` (module skeleton) and `command/state.rs`.

**Interfaces:**
- Consumes (P05, exactly these names):

```rust
pub struct Workspace { project: Project, registry: KindRegistry, index: Index, in_memory: bool, commands: CommandState }
impl Workspace {
    pub fn open(root: &Path) -> Result<Workspace, StudioError>;
    pub fn find(start: &Path) -> Result<Workspace, StudioError>;
    pub fn find_root(start: &Path) -> Result<PathBuf, StudioError>;
    pub fn project(&self) -> &Project;
    pub fn registry(&self) -> &KindRegistry;
    pub fn index(&self) -> &Index;
    pub fn index_mut(&mut self) -> &mut Index;
    pub fn is_in_memory(&self) -> bool;
    /// Runs `f`; on `IndexError::Contended` switches to an in-memory index once (sync) and runs it again.
    pub fn write<T>(&mut self, f: impl FnMut(&mut Index) -> Result<T, IndexError>) -> Result<T, StudioError>;
}
#[non_exhaustive] pub enum StudioError { Project(..), Registry(..), Index(..), Io { path, source }, NotAProject { start }, /* P05 adds more */ }

pub struct FindingCounts { /* (path, rule) -> count; (path, rule, message) seen */ }
impl FindingCounts {
    pub fn of(findings: &[Finding], moved: &BTreeMap<String, String>) -> FindingCounts;
    pub fn max(&self, other: &FindingCounts) -> FindingCounts;
    pub fn count(&self, path: &str, rule: &str) -> usize; // P06 adds this accessor if P05's file lacks it
}
pub fn introduced(allowed: &FindingCounts, after: &[Finding]) -> Vec<Finding>;
```

- Produces: `pub(crate) struct CommandState` in `command/state.rs`, with `Default`. Tasks 6 and 7
  give it the journal, the lock flag and `lock_wait: Duration` (default `COMMAND_WAIT`).
  `Workspace` gets the field `commands: CommandState`: one line in the struct, one in `open`.

- [ ] **Step 1: Look at main.**
  - Run: `git fetch origin`, then `git ls-tree --name-only origin/main crates/theos-studio/src/`.
  - When `workspace.rs` is listed: run `git merge origin/main` (resolve `Cargo.toml` and `lib.rs`
    by keeping both sides' lines) and go to Step 5.
- [ ] **Step 2 (only when main lacks them): failing tests.**
  - `tests/studio/commands.rs`: `open_reads_a_project`, `find_walks_up_to_the_project` and
    `find_outside_a_project_is_not_a_project`.
  - Unit tests in `introduced.rs`, each `// AC: CE-20.1`:
    - `a_new_error_of_a_rule_on_a_path_is_introduced`;
    - `a_changed_message_of_a_standing_error_is_not_introduced` (three-clue's "No clue moves
      T-981 …" → "Only one clue moves T-981 …");
    - `a_raised_severity_is_not_introduced` (the same finding as a warning before, an error
      after);
    - `warnings_are_never_introduced`;
    - `a_moved_file_keeps_its_findings` (`moved` maps old → new);
    - `max_takes_the_larger_count_per_key`;
    - `unseen_messages_come_first` (two errors after, one of them a message seen before, one
      allowed: the unseen one is returned).
- [ ] **Step 3:** run `cargo nextest run -p theos-studio --locked` and read the failure
  (unresolved `Workspace` or `introduced`).
- [ ] **Step 4:** implement `workspace.rs`, `StudioError` and `introduced.rs` with the
  signatures above, per P05 spec §5.3 and §4.2. Run the tests until they pass.
- [ ] **Step 5:** add `command/state.rs` (`CommandState`, empty with `Default`), the
  `commands` field and its line in `open`. If main's `FindingCounts` lacks `count`, add it in
  `introduced.rs` with a unit test. `cargo nextest run --workspace --locked` passes.
- [ ] **Step 6: Commit** — `feat(studio): the workspace and the count of introduced errors`,
  with `Refs: CE-20.1`. When everything came from main, the commit message is
  `feat(studio): give the workspace its command state`.

### Task 4: Commands and their plans

The command model and the five plain commands (spec §5.3). This task builds plans only, with no
writes.

**Files:**
- Create: `command/{mod,error,base,path,create,write,edit,patch,delete}.rs`
- Test: unit tests in each file; `tests/studio/commands.rs` for the plans against a project.

**Interfaces:**
- Consumes:
  - `Record::parse`, `with_edits`, `patch`, `was_normalized`, `text`;
  - `theos_content::{decode, text::normalize, TextEdit, PatchOp, Order}`;
  - `KindRegistry::kind(name)?.order()`;
  - `RenameError` (Task 1);
  - `LockError` (Task 2).
- Produces:

```rust
pub trait Command {
    /// The line shown in the undo list and the log.
    fn label(&self) -> String;
    /// What the index must have fresh before `plan` runs.
    fn reads(&self) -> Reads;
    /// The files this command changes, read from the workspace as it is now.
    fn plan(&self, ctx: &PlanContext<'_>) -> Result<Plan, CommandError>;
}
pub enum Reads { Paths(Vec<String>), Everything }
pub struct PlanContext<'a> { /* root, &Index, &KindRegistry */ }
impl<'a> PlanContext<'a> {
    pub(crate) fn new(root: &'a Path, index: &'a Index, registry: &'a KindRegistry) -> Self;
    pub fn root(&self) -> &Path;
    pub fn index(&self) -> &Index;
    pub fn registry(&self) -> &KindRegistry;
    /// The bytes of a project file, `None` when absent; the path is checked first.
    pub fn read(&self, path: &str) -> Result<Option<Vec<u8>>, CommandError>;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileChange { pub path: String, pub before: Option<Vec<u8>>, pub after: Option<Vec<u8>> }
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan { pub changes: Vec<FileChange>, /** old path → new path */ pub moved: BTreeMap<String, String> }
impl Plan { pub fn is_empty(&self) -> bool /* every before == after */; pub fn paths(&self) -> Vec<String>; }
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Base(String); // lower-case SHA-256 hex
impl Base { pub fn of(bytes: &[u8]) -> Base; pub fn as_str(&self) -> &str; } // + Display
pub(crate) fn project_path(path: &str) -> Result<String, CommandError>;

pub struct Create { pub path: String, pub text: String }
pub struct Write  { pub path: String, pub base: Base, pub text: String }
pub struct Edit   { pub path: String, pub base: Base, pub edits: Vec<TextEdit> }
pub struct Patch  { pub path: String, pub base: Base, pub ops: Vec<PatchOp> }
pub struct Delete { pub path: String, pub base: Base }

#[non_exhaustive] #[derive(Debug, thiserror::Error)]
pub enum CommandError {
    Rejected { blocking: Vec<Finding>, findings: Vec<Finding> },
    Stale { path: String }, NotFound { what: String }, Exists { path: String },
    IdTaken { id: String, path: String }, Ambiguous { id: String, paths: Vec<String> },
    CaseOnly { id: String, to: String }, Path { path: String, reason: &'static str },
    Changed { path: String }, NothingToUndo, NothingToRedo,
    Read { path: String, source: ReadError }, Record { path: String, source: RecordError },
    Patch { path: String, source: PatchError }, Edit { path: String, source: RecordEditError },
    Rename { path: String, source: RenameError },
    Write { path: String, source: std::io::Error },
    Partial { path: String, source: std::io::Error, unrestored: Vec<String> },
    Index(#[from] IndexError), Lock(#[from] LockError), Studio(#[from] StudioError),
}
```

Every message names the path and what to do. `Stale` says "changed on disk since it was read;
reload it". `Write` says "another program may have it open".

- [ ] **Step 1: Write the failing tests** (untagged):
  - `paths_outside_the_project_are_refused` (Review Focus 3): `../outside.md`, `C:\x.md`,
    `C:/x.md`, `/abs.md`, `\\server\x.md`, `.git/config`, `.GIT/config`, `lore/./x.md` and the
    empty path each give `Path`; `lore\truths\T-960.md` becomes `lore/truths/T-960.md`.
  - `a_base_that_differs_is_stale`, for `Write`, `Edit`, `Patch` and `Delete`.
  - `create_refuses_an_existing_file` (`Exists`). Under `cfg(windows)`, also another case of the
    name.
  - `write_turns_line_ends_into_lf` and `an_unchanged_save_of_a_crlf_record_is_an_empty_plan`.
  - `edit_applies_span_edits` and `patch_keeps_the_kind_order` (a field inserted where
    `KindSpec::order` puts it).
  - `patch_of_an_unknown_kind_keeps_the_order_of_its_fields`.
  - `delete_plans_a_removal`.
  - `base_is_the_sha256_hex` (`Base::of(b"")` is
    `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`).
- [ ] **Step 2:** run `cargo nextest run -p theos-studio --locked commands` and read the
  failure.
- [ ] **Step 3:** implement the types and the five commands. Each `reads()` is
  `Reads::Paths(vec![path])`. A record that does not decode or parse is `Read` or `Record` for
  `Edit` and `Patch`; `Write`, `Create` and `Delete` take any file.
- [ ] **Step 4:** run the tests until they pass; clippy is clean.
- [ ] **Step 5: Commit** — `feat(studio): commands and their plans`.

### Task 5: Writing a plan to disk

The whole-file writes of spec §5.6.

**Files:**
- Create: `command/files.rs` (unit tests inside)

**Interfaces:**
- Produces:

```rust
pub(crate) const RETRIES: u32 = 5;
pub(crate) const RETRY_PAUSE: Duration = Duration::from_millis(100);
/// Writes `plan` under `root`: content first in plan order, removals last, then the folders those
/// removals emptied. On a failure it puts back what it wrote from each change's `before` and fails
/// with `Write`, or with `Partial` when that fails too.
pub(crate) fn write_plan(root: &Path, plan: &Plan) -> Result<(), CommandError>;
```

One file:
1. `tempfile::Builder::new().prefix(".theos-").suffix(".tmp").tempfile_in(parent)`, then write,
   then `sync_all`.
2. For an existing target (`before` is `Some`): copy the target's permissions to the temp file,
   `into_temp_path().keep()`, then `std::fs::rename` over the target. On an error, remove the
   kept temp file.
3. For a new file: on Unix, create the temp file with `Permissions::from_mode(0o666)` so the
   umask applies; then `persist_noclobber`.
4. A rename refused with `PermissionDenied`, or with raw OS error 32 on Windows, is tried again
   `RETRIES` times, `RETRY_PAUSE` apart.

- [ ] **Step 1: Write the failing tests** (untagged):
  - `writes_new_files_and_replaces_existing_ones`.
  - `a_create_never_replaces_a_file_that_appeared`: the plan says `before: None`, but the file
    exists. Expected: `Write`, the file unchanged, and the plan's other writes rolled back.
  - `removals_come_last_and_emptied_folders_go`: a moved file; its old folder, now empty, is
    gone; an unrelated empty folder stays.
  - `a_failure_rolls_back_every_written_file` (Review Focus 2): a plan of two files where the
    second fails:
    - `#[cfg(windows)]`: the second target is held by a handle opened with
      `OpenOptionsExt::share_mode(1 | 2)` (no delete sharing);
    - `#[cfg(unix)]`: the second file's folder is made `0o555`.

    The first file has its old bytes again; the error names the second path and says another
    program may have it open; no `.theos-*.tmp` file is left anywhere.
  - `#[cfg(unix)]` `replaced_files_keep_their_permissions` (`0o640` stays `0o640`) and
    `new_files_get_the_default_mode` (the same mode as a file made with `File::create` in the
    same folder).
- [ ] **Step 2:** run `cargo nextest run -p theos-studio --locked files` and read the failure.
- [ ] **Step 3:** implement `write_plan` as above. Log each retry and each rollback at `warn`.
- [ ] **Step 4:** run the tests until they pass, on Windows here. CI covers Linux; the
  `platforms` workflow covers macOS.
- [ ] **Step 5: Commit** — `feat(studio): write a plan whole, with rollback`.

### Task 6: The pipeline, the verdict and the human route

Spec §5.4, §5.5, §5.10 and §5.11 (logging). CE-20.1 end to end.

**Files:**
- Create: `command/{route,verdict,pipeline,run,journal}.rs`
- Modify: `command/state.rs`: `journal: Journal`, `locked: bool`, `lock_wait: Duration`.
- Create: `tests/fixtures/commands/`, `tests/fixtures/standing/` (below)
- Test: `tests/studio/verdict.rs` (CE-20.1), `tests/studio/pipeline.rs`

**Interfaces:**
- Consumes:
  - `Workspace::write` and `index()` (Task 3);
  - `FindingCounts` and `introduced` (Task 3);
  - `acquire` and `COMMAND_WAIT` (Task 2);
  - `write_plan` (Task 5);
  - `Index::{update, sync, preview, findings_for}`, `Change`, `Preview`, `Update`.
- Produces:

```rust
pub struct Outcome { pub changed: Vec<String>, pub update: Update, pub findings: Vec<Finding>,
                     pub introduced: Vec<Finding>, pub entry: Option<EntryId> }
pub trait Route {
    /// Whether the route holds the repo lock for the whole command.
    fn holds_lock(&self) -> bool;
    /// After the write and the reindex.
    fn finish(&mut self, workspace: &mut Workspace, applied: Applied) -> Result<Option<EntryId>, CommandError>;
}
pub struct Applied { pub label: String, pub changes: Vec<FileChange>,
                     pub counts_before: FindingCounts /* unmapped */, pub counts_after: FindingCounts }
pub struct HumanRoute; // holds the lock; finish pushes the entry into the journal
impl Workspace {
    pub fn read(&self, path: &str) -> Result<Option<(Vec<u8>, Base)>, CommandError>;
    pub fn run(&mut self, command: &dyn Command) -> Result<Outcome, CommandError>; // = run_with(.., &mut HumanRoute)
    pub fn run_with(&mut self, command: &dyn Command, route: &mut dyn Route) -> Result<Outcome, CommandError>;
    pub fn set_lock_wait(&mut self, wait: Duration);
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)] pub struct EntryId(u64);
pub struct EntrySummary { pub id: EntryId, pub label: String, pub paths: Vec<String> }
pub(crate) struct Journal { /* done: VecDeque<Entry>, undone: Vec<Entry>, next: u64 */ }
impl Journal { pub(crate) const LIMIT: usize = 100; pub(crate) fn push(&mut self, applied: Applied) -> EntryId; }
impl Workspace {
    /// Newest first.
    pub fn undo_entries(&self) -> Vec<EntrySummary>;
    pub fn redo_entries(&self) -> Vec<EntrySummary>;
}
pub(crate) enum Work<'a> { Command(&'a dyn Command), Replay { label: String, plan: Plan, allowance: FindingCounts, entry: EntryId } }
```

**Pipeline order** (`pipeline.rs`), exactly spec §5.4:
1. Take the lock if `route.holds_lock()`: `locked` already set → `Reentrant`; else
   `acquire(project root, label, lock_wait)`.
2. Freshen the index: `Reads::Paths` → `update`; `Reads::Everything` → `sync`. Then plan.
3. Compare every change's `before` with the disk: a difference is `Stale` (for `Replay`:
   `Changed`).
4. `preview` the changes; then read `findings_for` each affected path.
5. The verdict (`verdict.rs`):
   - `allowed` = `FindingCounts::of(before, &plan.moved)`, `.max(allowance)` for `Replay`;
   - `blocking` = `introduced(&allowed, &preview.findings)`; non-empty → `Rejected`;
   - the new warnings = warnings above `allowed.count(path, rule)`.
6. `write_plan`.
7. `update` every plan path; log the `Update` at `debug`, every list including `revalidated`.
   Each command applied or rejected is logged at `info`, with its label and paths.
8. `route.finish`.
9. Release the lock (also on every error) and return the `Outcome`.

An empty plan (`Plan::is_empty`) returns at step 2 with nothing changed and no entry. Every call
on the index goes through `Workspace::write`.

**Fixtures** (synthetic, the header comment `# synthetic fixture (P06), not canon` in every
record, ids in the 96x range). Every record has the fields its kind requires; the fixtures of
`crates/theos-content/tests/fixtures/projects/links/` show each kind. `commands/` has no error,
which `the_fixtures_have_no_errors` asserts.

| File | Holds |
|---|---|
| `theos.toml` | synthetic project, data model §13 |
| `world/tower.md`, `world/synthetic-band.md` | tower, band |
| `world/synthetic-band/f04-synthetic.md` | floor 4, parent the band |
| `world/synthetic-band/f04-synthetic/synthetic-hall.md` | location, parent `f04-synthetic` |
| `world/synthetic-band/f04-synthetic/synthetic-hall/synthetic-nook.md` | location, parent `synthetic-hall` |
| `world/synthetic-band/f04-synthetic/notes.txt`, `.synthetic-hidden` | a file that is not a record, a dot file |
| `people/synthetic-folk.md`, `people/synthetic-folk/synthetic-singer.md` | people; npc with `home: synthetic-hall`, `lens:` `unaware_of: [T-960]`, `rejects: ['B-961']` |
| `lore/truths/T-960.md`, `T-961.md` | targets `confirmed` and `never` |
| `lore/clues/C-960.md` | `reveals: T-960`, `moves_to: suspected`, `where:` block list `- f04-synthetic  # synthetic comment` |
| `lore/clues/C-961.md` | `reveals: "T-960"`, `moves_to: hinted`, `where: [f04-synthetic]` |
| `lore/beliefs/B-960.md`, `B-961.md` | `held_by: 'synthetic-singer'`, `distorts: T-960`; a second belief |
| `story/scripts/S-960.md` | `where: [f04-synthetic]`. The body has the mention `[[T-960#1\|the truth]]`, `[[T-960]] and [[T-960]]` on one line, beat 1 with `at:: [[f04-synthetic]]`, `with:: [[synthetic-singer]]` and `reveals:: [[C-960]]`, and a voice block of `[[synthetic-singer]]` naming `[[B-960]]` (data model §9.1). |
| `concepts/synthetic-shrine.md` | `for: [synthetic-hall]`, `picked: [synthetic-shrine-v3]`, note `n1 · [[synthetic-shrine-v2]] · resolved by [[synthetic-shrine-v3]]` |
| `concepts/synthetic-shrine/synthetic-shrine-v1.md` … `-v3.md` | versions; v2 `from: synthetic-shrine-v1`, v3 `from: synthetic-shrine-v2`, `addresses: [n1]` |

`standing/` has:
- `theos.toml`;
- `.theos/settings.toml` with `[validate.severity]` `"lore/three-clue" = "error"`;
- `T-980` (`target: hinted`) and `C-980` (`moves_to: confirmed`), a standing no-overshoot error
  on `C-980`;
- `T-981` with no clue, a standing three-clue error;
- `S-980` with a dangling `[[synthetic-gone]]`, a standing `link/dangling` error;
- every record the clues need (a floor for `where`).

- [ ] **Step 1: Write the failing tests** in `tests/studio/verdict.rs`. Each is `// AC:
  CE-20.1`. Each rejection asserts:
  - `Err(Rejected { blocking, .. })`, with the expected rule and path in `blocking`;
  - `files(root)` equal before and after;
  - `index().dump()` equal before and after;
  - `undo_entries()` empty.

  The tests:
  - `an_invalid_value_is_rejected`: `Patch` sets `T-960`'s `target` to `maybe` (`schema/value`).
  - `a_dangling_link_is_rejected`: `Edit` appends `[[synthetic-nowhere]]` to `S-960`
    (`link/dangling`).
  - `a_clue_that_reveals_a_never_truth_is_rejected`: `Patch` sets `C-961`'s `reveals` to
    `T-961` (`lore/never-confirm`).
  - `a_neighbour_that_overshoots_is_rejected`: `Patch` sets `T-960`'s `target` to `hinted`.
    `blocking` has `lore/no-overshoot` on `lore/clues/C-960.md`.
  - `a_new_record_with_an_error_is_rejected`: `Create` a truth without `target`.
  - `standing_errors_do_not_block`, in `standing/`:
    - `Patch` of `S-980`'s `title` is applied, and `findings` still lists its `link/dangling`;
    - `Patch` of `T-980`'s `target` to `suspected` is applied; `C-980` is in `update.revalidated`
      and still has its error.
  - `a_changing_standing_message_does_not_block`, in `standing/`: `Create` `C-981` moving `T-981`
    is applied, although the three-clue message changes.
  - `lowering_a_guardrail_is_rejected`: a `Write` of `.theos/settings.toml` with
    `"lore/never-confirm" = "warning"` (`settings/guardrail`).
  - `raising_a_rule_to_error_is_applied`: in `commands/`, a `Write` of the settings with
    `"lore/three-clue" = "error"` is applied.

  And in `tests/studio/pipeline.rs` (untagged):
  - `the_fixtures_have_no_errors`.
  - `an_applied_command_updates_the_index_and_returns_its_outcome`.
  - `a_file_changed_after_the_plan_is_stale` (Review Focus 1): a test `Command` whose `plan`
    reads the file, builds its plan, then writes other bytes to that file on disk. Expected:
    `Stale`, the other program's bytes kept, no entry.
  - `an_unchanged_save_writes_nothing` (Review Focus 5): a CRLF record written into a
    `TempProject`, then `Write` of its normalised text. Expected: `changed` empty,
    `entry: None`, the bytes still CRLF.
  - `a_held_lock_makes_the_command_busy`: `acquire` in the test, `set_lock_wait(200 ms)`, then
    `run` is `Lock(Busy)`.
  - `reentry_is_refused`: a unit test in `pipeline.rs` on the lock step with `locked` set.
- [ ] **Step 2:** run `cargo nextest run -p theos-studio --locked verdict pipeline` and read
  the failure.
- [ ] **Step 3:** implement `route.rs`, `verdict.rs`, `pipeline.rs`, `run.rs`, `journal.rs`
  (`push` only) and the fixtures.
- [ ] **Step 4:** run the tests until they pass; run `cargo nextest run --workspace --locked`.
- [ ] **Step 5: Commit** — `feat(studio): run commands through the verdict and the human route`,
  with `Refs: CE-20.1`.

### Task 7: Undo and redo

Spec §5.7. CE-20.2, except the rename case (Task 8).

**Files:**
- Modify: `command/journal.rs` (the entries, limit, scopes)
- Create: `command/undo.rs`
- Test: `tests/studio/undo.rs`

**Interfaces:**
- Consumes: `Work::Replay` and the pipeline (Task 6); `Applied`.
- Produces:

```rust
pub enum Scope { Newest, Record(String) }
impl Workspace {
    pub fn undo(&mut self, scope: Scope) -> Result<Outcome, CommandError>;
    pub fn redo(&mut self, scope: Scope) -> Result<Outcome, CommandError>;
}
```

The undo of an entry is `Work::Replay`, labelled `undo: <label>`:
- `plan.changes` are the entry's changes reversed (`before` ↔ `after`);
- `plan.moved` maps new → old for the moved pairs;
- `allowance` = the entry's `counts_before`.

The spec's `crlf/` fixture (§5.12) is written by the tests at runtime: git's line-end settings
could normalise a checked-in CRLF file.

Redo uses the entry's changes and `counts_after`. A replay does not push a new entry: success
moves the entry between done and undone, and `Outcome.entry` is its id. A new command clears
undone. Done keeps the newest `Journal::LIMIT` entries.

- [ ] **Step 1: Write the failing tests** in `tests/studio/undo.rs`. Each is `// AC: CE-20.2`
  except where marked:
  - `undo_gives_back_the_exact_bytes`:
    - `Create`, `Write`, `Edit`, `Patch` and `Delete`, each undone;
    - also a CRLF record and a record without a final newline, written into the project by the
      test, then edited;
    - after each undo: `files(root)` equal to before the command, and `index().dump()` equal to
      that of a fresh `Workspace::open` of a copy of the folder.
  - `an_undo_after_a_later_change_fails_with_changed`: the file edited on disk after the
    command. Expected: `Changed`, nothing written, the entry still in `undo_entries`.
  - `redo_gives_the_later_bytes_again` and `a_new_command_clears_redo`.
  - `scope_record_undoes_the_newest_entry_of_that_record`: an `Edit` of A, then an `Edit` of B;
    `undo(Scope::Record(A))` undoes A's, and B's stays.
  - `undoing_a_fix_brings_the_error_back`: in `standing/`, `Edit` removes `S-980`'s dangling
    link, then `undo`. Expected: applied, the link and its `link/dangling` back.
  - `undoing_a_linked_create_is_rejected`: `Create` `T-962`, then an `Edit` of `S-960` linking
    it, then `undo(Scope::Record("lore/truths/T-962.md"))`. Expected: `Rejected`
    (`link/dangling`).
  - `undoing_a_random_history_gives_the_first_bytes`: proptest, 16 cases, up to 8 commands drawn
    from the five kinds on `commands/`. Rejected ones are skipped. Then undo until
    `NothingToUndo`; `files(root)` equals the start.
  - Untagged:
    - `nothing_to_undo_and_redo`;
    - `the_journal_keeps_a_hundred_entries`: 101 commands; the oldest is gone. Use a tiny
      project so this stays fast.
- [ ] **Step 2:** run `cargo nextest run -p theos-studio --locked undo` and read the failure.
- [ ] **Step 3:** implement `journal.rs` and `undo.rs`.
- [ ] **Step 4:** run the tests until they pass.
- [ ] **Step 5: Commit** — `feat(studio): undo and redo to the exact bytes`, with
  `Refs: CE-20.2`.

### Task 8: Rename

Spec §5.8. CE-20.3, and the rename case of CE-20.2.

**Files:**
- Create: `command/rename/{mod,participant,versions,children,ids,links}.rs`
- Test: `tests/studio/rename.rs`

**Interfaces:**
- Consumes:
  - `theos_content::rename::{edits, IdMap}` (Task 1);
  - `Index::{record, children, backlinks, links_from}`;
  - `Plan.moved`;
  - the pipeline with `Reads::Everything`.
- Produces:

```rust
pub struct Rename { pub id: String, pub to: String }
pub(crate) trait Participant {
    /// Extends the rename: more ids, more moved files, more records to edit.
    fn extend(&self, ctx: &PlanContext<'_>, set: &mut RenameSet) -> Result<(), CommandError>;
}
pub(crate) struct RenameSet {
    pub ids: IdMap,                          // old id → new id
    pub moves: BTreeMap<String, String>,     // old file path → new file path
    pub sources: BTreeSet<String>,           // old paths of the records to edit
}
pub(crate) const PARTICIPANTS: &[&dyn Participant] = &[&Versions, &Children, &Ids, &Links];
```

**The plan** (`mod.rs`):
1. Checks: `id` names one record (`NotFound`, `Ambiguous`). `to != id`, and not equal ignoring
   ASCII case (`CaseOnly`).
2. Run the participants in order:
   - `versions`: for a `concept` or `map`, each `children(id)` of kind `concept-version` or
     `map-version` whose id is `<id>-v<digits>` maps to `<to>-v<digits>`;
   - `children`: for each renamed record with a folder `<folder>/<old>/`, every file under it
     (`std::fs::read_dir`, recursive, records or not, dot files too) moves to
     `<folder>/<new>/…`;
   - `ids`: each renamed record's file moves to its folder (after `children`) with the new stem;
   - `links`: `sources` = the renamed records ∪ the `source_path` of every `backlinks(old)`.
3. `IdTaken`:
   - a new id that `record(new)` finds;
   - or a new path, or a new child folder, that `std::fs::symlink_metadata` finds on disk.
4. Build the changes. For each path in `moves` ∪ `sources`, read the old bytes. A source gets
   `edits(record, &links_from(old path), &ids)` applied with `with_edits`. A moved file becomes a
   removal at the old path plus a creation at the new one; any other file is changed in place.
   `plan.moved` = `moves`.

- [ ] **Step 1: Write the failing tests** in `tests/studio/rename.rs`. Each is `// AC: CE-20.3`
  unless marked. After each rename, assert that `backlinks(old)` is empty for every old id, that
  the index has no `link/dangling`, that no finding is new, and that exactly one entry was added.
  - `renaming_a_truth_rewrites_every_link`: `T-960` → `T-970`. Expected:
    - `C-960` (plain), `C-961` (`"T-970"`) and `B-960`'s `distorts` rewritten;
    - the npc's `unaware_of`;
    - `S-960`'s `[[T-970#1|the truth]]` and `[[T-970]] and [[T-970]]`;
    - `T-970.md` with `id: T-970`.
  - `renaming_a_concept_renames_and_moves_its_versions`: `synthetic-shrine` →
    `synthetic-temple`. Expected:
    - `concepts/synthetic-temple.md` and `concepts/synthetic-temple/synthetic-temple-v1.md` …
      `-v3.md`, each with its `id`;
    - `concept: synthetic-temple` in each version;
    - `from` links, `picked: [synthetic-temple-v3]`, and the note's version and `resolved by`
      links;
    - the old folder gone.
  - `renaming_a_floor_moves_its_child_folder`: `f04-synthetic` → `f04-renamed`. Expected:
    - `synthetic-hall`, `synthetic-nook`, `notes.txt` and `.synthetic-hidden` under
      `world/synthetic-band/f04-renamed/`;
    - their `parent` fields rewritten;
    - `C-960`'s block list item with its comment kept;
    - `C-961`'s and `S-960`'s flow lists;
    - the beat's `at::`.
  - `renaming_an_npc_rewrites_voice_and_beat_links`: `synthetic-singer` → `synthetic-chanter`.
    Expected: the voice speaker, `with::` and `held_by: 'synthetic-chanter'`.
  - `a_rename_moves_every_file_of_the_folder` (Review Focus 4, untagged): the files of
    `f04-synthetic/` that are not records are byte-identical at their new paths.
  - `a_taken_id_is_refused`: `to` = `T-961` gives `IdTaken`.
  - `an_id_taken_by_an_ignored_file_is_refused` (Review Focus 4, untagged): a `.gitignore` with
    `lore/truths/T-975.md` and that file on disk. A rename to `T-975` gives `IdTaken`, and
    nothing is written.
  - `a_case_only_rename_is_refused`: `T-960` → `t-960` gives `CaseOnly` on every OS.
  - `an_id_that_breaks_the_kind_rule_is_rejected`: `T-960` → `x960` is `Rejected` with
    `schema/id`.
  - `// AC: CE-20.2` `undoing_a_rename_restores_every_path`: after the concept rename, `undo`.
    Expected: `files(root)` equal to before, the new folder gone, the old folder back.
  - Untagged: `an_unknown_id_is_not_found`, and `a_duplicate_id_is_ambiguous` (a second
    `T-960.md` written into another folder).
- [ ] **Step 2:** run `cargo nextest run -p theos-studio --locked rename` and read the failure.
- [ ] **Step 3:** implement the participants and `Rename`. Its `label()` is
  `rename <id> to <to>`; `reads()` is `Everything`.
- [ ] **Step 4:** run the tests until they pass; run `cargo nextest run --workspace --locked`.
- [ ] **Step 5: Commit** — `feat(studio): rename an id in one command`, with
  `Refs: CE-20.3, CE-20.2`.

### Task 9: Two writers in two processes

CE-22.2, second half: the lock and the base check together.

**Files:**
- Modify: `tests/studio/helper.rs` (`HelperTask::Append`)
- Create: `tests/studio/writers.rs`

**Interfaces:**
- Consumes: `Workspace::{open, read, run, set_lock_wait}` and `Edit`.
- Produces: `HelperTask::Append { root, path, tag, count, ready }`. It writes `ready`, then
  appends `count` lines `<tag> <n>` to the end of the body. Each line is one `Edit`: `read` →
  `Base`, an insertion at the text's end, `run`; on `Stale` it reads again and retries.

- [ ] **Step 1: Write the failing test** `// AC: CE-22.2`
  `two_processes_append_without_losing_a_line`:
  1. In `commands/`, a helper appends 20 lines tagged `helper` to `S-960`, while the test
     appends 20 lines tagged `test`. Both use `set_lock_wait(10 s)`.
  2. Wait for the helper to exit with success.
  3. Expected: the record holds all 40 lines, each exactly once, and it parses (`Record::parse`).
     No `.theos-*.tmp` file is left.
- [ ] **Step 2:** run `cargo nextest run -p theos-studio --locked writers` and read the failure
  (unknown helper task).
- [ ] **Step 3:** implement `Append` in the helper.
- [ ] **Step 4:** run it 5 times in a row (`cargo nextest run -p theos-studio --locked writers
  --retries 0`, repeated). It passes every time.
- [ ] **Step 5: Commit** — `test(studio): two writers in two processes`, with
  `Refs: CE-22.2`.

### Task 10: `theos new` onto `Create`

Only when P05's `theos new` is on `origin/main` when this task starts (`git fetch origin`, then
look for P05's `new` module in `crates/theos/src/`).

- [ ] **If it is there:**
  1. `git merge origin/main`.
  2. Replace `theos new`'s preview-and-write with `workspace.run(&Create { path, text })`.
     Map `Rejected` to P05's exit code and finding output, and `Exists` to its "already exists"
     message.
  3. P05's `theos new` tests pass unchanged. Run `cargo nextest run --workspace --locked`.
  4. Commit `refactor(cli): write theos new through the create command`.
- [ ] **If it is not:** add a ledger line, "P05 moves `theos new` onto `Create` (whichever
  merges second, P06 spec §6.1)", and send the P05 session that line.

---

## After the tasks

These steps follow the theos-phase skill and are not tasks for an implementer:
- the final review loop (CLAUDE.md);
- `cargo xtask gate` on Windows;
- `cargo xtask ac-trace --phase P06`: every AC has a passing test;
- `docs/status.md` → `pr-open`;
- the build PR with the labels `windows` and `macos`.

---

## Measurements

**One rename of 50 files** (spec §7, risk "A rename of a parent with many children"):
- The test `rename::renaming_a_parent_with_fifty_files_in_its_folder` renames the floor
  `f04-synthetic` of `commands/` to `f04-renamed`. Its folder holds 50 files: 46 generated child
  records that each link the floor, the hall, the nook, `notes.txt` and a hidden file.
- The plan has 105 file changes: the floor and the 50 files move (a removal and a creation each),
  and two clues and the script are edited in place. The test checks the result only.
- `Workspace::run` of that rename took 223–227 ms, median 224 ms, over five runs: the full
  pipeline, from the lock and the whole-project sync to the reindex.
- Measured on 12 Nov 2026 on Windows 11, toolchain 1.99.1, **dev profile** (unoptimized), with
  `cargo nextest run` running that test alone. The time came from a local, uncommitted `Instant`
  around the call; the committed test has no timing.
