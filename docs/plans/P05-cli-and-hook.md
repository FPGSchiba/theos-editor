# P05 · CLI and hook — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (`CLAUDE.md` makes it the execution method) to implement this plan task by task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** `theos` becomes the command line of a content project:
- `theos check` (with `--staged`), `ledger`, `new`, `schema` and `hook install`, each with `--json`,
  and `--json` for `formalism`;
- the pre-commit hook runs `theos check --staged` and refuses a commit whose staged snapshot holds
  an error.

**Architecture:** three crates, so the editor, MCP (P48) and P47's commit share the CLI's code:
- **`theos-vcs`:** the git runner (`Git`), the repository queries (`repo`) and the reader of the
  staged snapshot (`staged`), through the git command line only (G6).
- **`theos-studio`:**
  - `Workspace`, the project, kinds and index opened once, with the contended-cache fallback;
  - the checks: `check`, `check_staged`, `create` for `theos new`, `install_hook`;
  - the shared verdict `introduced`, which P06 uses too;
  - the JSON views and the error report.
- **`theos`:** argument parsing (clap) and printing only.

`theos-content` gains `Index::paths`, `Index::snapshot`, `IdRule::number_prefix` and
`Placement::under_parent`.

**Tech stack:**
- Rust 1.99.1, edition 2024.
- New workspace dependencies: `which` 8.0 (Task 1) and `dirs` 7.0 (Task 7), both MIT (`deny.toml`
  allows MIT and MPL-2.0).
- Already pinned: `clap` 4.6 (derive), `serde`, `serde_json`, `toml` 1.1, `thiserror`, `tracing`,
  `anyhow` (binary only), `tempfile`, `rusqlite` and `ignore` (as dev-dependencies of
  `theos-studio`, to build library errors in tests).

**Spec:** `docs/specs/phases/P05-cli-and-hook.md` (approved and merged, #35). Read it with this
plan. Where this plan is silent, the spec decides; where they differ, the spec wins.

**Probes run while planning** (12 Nov 2026, git 2.56.1 on Windows 11, isolated with
`GIT_CONFIG_NOSYSTEM=1` and an empty `GIT_CONFIG_GLOBAL`), in addition to spec §4.3:
- `git rev-parse --git-path hooks` prints `.git/hooks` by default. With a repo-local
  `core.hooksPath` it prints that value, `githooks` or `.git/myhooks`, relative to the top folder.
- In a linked worktree it prints the absolute `<common>/.git/hooks`.
- `git rev-parse --path-format=absolute --git-common-dir` prints the main repository's `.git`
  there, with `/`.

## Global Constraints

Umbrella global constraints, verbatim. `CLAUDE.md` asks for G1–G16. G17 to G20 were added to the
umbrella later and are copied too, as in the P02 to P04 plans.

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

Phase ACs, verbatim:

**CE-17 CLI and hook**
- CE-17.1 `theos check | ledger | new | schema | hook install` with `--json` output.
- CE-17.2 The hook refuses a commit with a never-confirm violation and checks the staged snapshot.

**What binds the build beyond the ACs:**
- **Spec §4.2:** the agreement with P06's spec, approved with #35:
  - the staged check covers every finding of the staged snapshot;
  - it never takes P06's repo lock;
  - `theos new` blocks only on errors it introduces, counted per path and rule;
  - the signatures of spec §6 are a contract with P06, P46, P47 and P48.
- **Agreed with P06's plan session on 12 Nov 2026:**
  - **`introduced`:** P06's signature (`FindingCounts`, Task 3), built by whichever phase is first.
  - **P05 writes** `workspace.rs` as spec §5.3 says, `check_staged`, `views/`, `report`,
    `user_settings` and all of `theos-vcs`.
  - **P06 writes** `command/`, `lock.rs` and `theos-content/src/rename.rs`. It adds one field
    `commands: CommandState` to `Workspace`: one line in the struct and one in `open`.
  - **Merging:** both sides add lines to `lib.rs`, `error.rs` and the manifests. Whichever PR
    merges second merges `origin/main`, never a rebase.

## Build rules (every task, every subagent)

Put this section into every implementer and reviewer brief.

- **Engineering directives** (`CLAUDE.md`):
  - Small modules with one job. `theos` only parses and prints: every decision lives in
    `theos-studio`, so P48 serves the same results.
  - **Use what exists first:**
    - `clap` parses arguments, `serde_json` writes JSON, `toml` reads the user settings and
      `dirs` finds their folder;
    - `which` finds git, and `std::process::Command` runs it, with no shell;
    - P02's front-matter parser reads `--set` values, and P02's patcher writes them;
    - `Index::preview` and `Index::snapshot` judge, and `settings::parse` reads severities.
  - **The only custom readers** are for git's documented `-z`, `ls-files -s` and `cat-file
    --batch` formats (spec §5.1). Never hand-write a parser for TOML, JSON or YAML.
  - If reviews keep finding new variants of one problem, stop and name the class (directive 3).
- **Lints** (workspace, `-D warnings`):
  - No `#[allow]` or `#[warn]`. `#[expect]` only with `reason = "…"`.
  - Every public item is documented (`missing_docs`). Every public fn returning `Result` has an
    `# Errors` section.
  - No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, `print!` or `exit`
    outside tests. `theos` writes with `writeln!` to the `out` and `err` writers it is given.
  - Clippy pedantic is on.
- **Coverage:**
  - `src/**` counts, with its error paths: test them.
  - `theos-vcs` and `theos-studio` need 80 %, `theos` 60 %.
  - Integration tests and the binary runs of `CARGO_BIN_EXE_theos` count.
- **Code per OS:** follow `theos-content/src/walk.rs`:
  - one `#[cfg(unix)]` and one `#[cfg(windows)]` fn with the same signature;
  - each with a test under the same `cfg`;
  - only `unix` and `windows` are allowed cfg names (`gate/cfg-names.toml`).
- **Dead code between tasks.** A module that a later task wires in carries
  `#[cfg_attr(not(test), expect(dead_code, reason = "wired in by Task N"))]` on its `mod` line.
  The task that wires it in removes the attribute. A test helper is added in the task whose test
  first uses it.
- **Docs:** code blocks in doc comments are ```` ```text ```` unless they are doctests that pass.
  Intra-doc links must resolve. Each crate's `lib.rs` docs say what it provides after P05.
- **Tests:**
  - Units go in `#[cfg(test)] mod tests` next to the code.
  - **Behaviour** goes in one integration-test crate per crate, `main.rs` plus one module per
    file:
    - `crates/theos-vcs/tests/vcs/`;
    - `crates/theos-studio/tests/studio/`;
    - `crates/theos/tests/cli/` (Task 8 moves `tests/cli.rs` there; `tests/smoke.rs` stays);
    - `crates/theos-content/tests/index/` (P03's).
  - A test proving an AC is a plain `#[test]` fn with `// AC: CE-17.x` on the line directly
    above `#[test]`.
  - Tests may return `Result<(), Box<dyn std::error::Error>>`.
- **Git in tests** always runs isolated (spec §5.13):
  - `GIT_CONFIG_GLOBAL` points at an empty file in the test's temp folder, and
    `GIT_CONFIG_NOSYSTEM=1`;
  - `GIT_DIR`, `GIT_WORK_TREE` and `GIT_INDEX_FILE` are removed from every git and `theos` child
    a test starts;
  - repositories are made with `git init -q -b main`;
  - commits use `-c user.name=Synthetic -c user.email=synthetic@example.invalid`;
  - code under test gets an isolated runner through `Git::find()?.with_env(…)` (L1).
- **The `theos` binary in tests:**
  - runs from `env!("CARGO_BIN_EXE_theos")`;
  - always gets `THEOS_CONFIG_DIR` set to a temp folder, empty unless the test writes
    `settings.toml` there;
  - always gets the git isolation above, which the hook's `theos` and git inherit.
- **Fixtures:**
  - Every new record starts its front matter with `# synthetic fixture (P05), not canon`.
  - Texts are neutral ("Synthetic statement A"), never lore.
  - Ids are `T-900`… `C-900`… `S-900`… and `synthetic-…` slugs.
  - The npc example is data model §14's `mother-ilse`, under synthetic parents.
  - P05's fixture project lives in `crates/theos-studio/tests/fixtures/` (Task 6). `theos`'s tests
    read it by the relative path `../theos-studio/tests/fixtures/…`, as they already read
    `theos-content`'s.
- **Files with backslashes** (Rust string escapes, Windows paths in tests) are written with the
  Write or Edit tool, never with a shell heredoc.
- **Git:**
  - One git command per Bash call in the worktree (no `&&` chains). Never `--no-verify`.
  - **Commit message:** a Conventional Commit with scope `vcs`, `studio`, `content` or `cli`, or
    `build` for a dependency-only commit. The summary starts lower case. A body says why.
  - **Footers:** one block. No footer line is over 100 characters. Leave `Refs:` out when a task
    proves no AC.

  ```text
  Refs: CE-17.2
  Phase: P05
  Claude-Session: 023e0398-6eb2-4e8e-b783-b170ea6f8b28
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

1. **A project folder whose path has a space, non-ASCII letters or glob characters**, such as
   `[draft] Inhalt ä/`.
   - Expected: the staged read lists exactly its entries (`--literal-pathspecs`), and paths map
     back to project paths.
   - Tests: Task 5 (`a_folder_with_glob_characters_and_umlauts_is_read_literally`), Task 6
     (`a_project_in_a_subfolder_maps_its_paths`).
2. **Entries at record paths that are not regular files:** a symbolic link (mode 120000) or a
   submodule (160000).
   - Expected: they are no record of the snapshot, even when the working tree holds a regular
     file with a violation at that path.
   - Tests: Task 5 (`tracked_keeps_the_modes_of_links_and_submodules`), Task 6
     (`links_and_submodules_at_record_paths_are_not_records`).
3. **`.theos/settings.toml` staged differently from the working copy.**
   - Expected: the staged settings decide every severity of the snapshot, also on paths the
     commit does not touch.
   - Tests: Task 4 (`a_settings_change_applies_to_untouched_paths`), Task 6
     (`staged_settings_decide_the_snapshot_severities`).
4. **The first commit of a repository** (unborn `HEAD`).
   - Expected: every staged file is checked, and the hook refuses a violation as later.
   - Tests: Task 5 (`the_first_commit_lists_every_staged_file_as_added`), Task 12
     (`the_first_commit_is_checked`).
5. **An installing `theos` whose path holds a space and a `'`,** run by Git for Windows' `sh`.
   - Expected: the hook starts that `theos`, and falls back to the PATH when the file is gone.
   - Tests: Task 11 (`the_script_quotes_the_path`), Task 12
     (`a_theos_path_with_a_space_and_a_quote_runs`).

## Rulings made while planning

These go into the ledger's "Rulings (build)" and the PR, in this form.

- L1. Ruling: `Git::with_env(self, &[(&str, &str)]) -> Git` adds variables to every command the
  runner starts — tests isolate git's configuration through it, and P47 can set
  `GIT_TERMINAL_PROMPT` once — cost if wrong: one method.
- L2. Ruling: `GitError` gets a fourth variant, `Output { args, message }`, for git output that
  cannot be read — a failed command and an unreadable answer are different causes — cost if
  wrong: one variant P46 keeps.
- L3. Ruling: `repo::top` reads git's answer as UTF-8 and fails with `Output` otherwise — one
  code path on every OS, and git on Windows prints UTF-8 — cost if wrong: a repository under a
  non-UTF-8 Unix path cannot use the hook.
- L4. Ruling: `repo::place(top, folder)` gives the folder's place in the repository, both
  canonicalized, with `/`, empty at the top; the staged read and the path mapping both use it —
  one computation of a shared input — cost if wrong: none.
- L5. Ruling: only stage-0 entries of `ls-files -s` become `tracked` — git refuses a commit with
  unmerged paths before any hook runs — cost if wrong: none.
- L6. Ruling: `Git::run` writes stdin from a scoped thread while `wait_with_output` reads stdout
  and stderr — a `cat-file --batch` answer fills the pipe before its input ends — cost if wrong:
  none.
- L7. Ruling: the staged reads pass `--literal-pathspecs` — a project folder named `[draft]` is
  a glob otherwise — cost if wrong: none.
- L8. Ruling: every studio write to the index goes through `Workspace::write`. On
  `IndexError::Contended` it switches to the in-memory index once and runs the closure again —
  spec §5.3 names `open` and later writes; one place handles both — cost if wrong: none.
- L9. Ruling: `introduced` takes P06's signature, `FindingCounts::of`, `max` and
  `introduced(allowed, after)`, agreed with P06's plan session — undo needs the larger of two
  counts — cost if wrong: none for P05.
- L10. Ruling: the JSON `binaries` object always has `checked`, `folder` and `reason`, null when
  absent — spec §5.11: every key is always there — cost if wrong: none.
- L11. Ruling: `--json` is one global flag of `Cli` — every command takes it the same way — cost
  if wrong: none.
- L12. Ruling: `--binaries` conflicts with `--staged` (clap, exit 2) — the staged check reads no
  binaries — cost if wrong: none.
- L13. Ruling: a kind is numbered when its id pattern is exactly `^<capitals>-[0-9]{3,}$`;
  `IdRule::number_prefix` derives the prefix from the pattern — one source, and a later `Q-` kind
  counts by itself — cost if wrong: a numbered kind written another way asks for an id.
- L14. Ruling: the next number counts every record id with the prefix, of any kind — spec §5.9,
  "in the index" — cost if wrong: none.
- L15. Ruling: a `--set` value is read by parsing `---\n<key>: <value>\n---\n` with P02's
  front-matter parser and taking `Value::to_raw` — no hand parser for `[a, b]` — cost if wrong:
  none.
- L16. Ruling: the skeleton is `id`, `kind`, then `title` (the id) and `status: draft` where the
  kind has them, then an empty body — spec §5.9 — cost if wrong: none.
- L17. Ruling: a hook is "for the same project" when it holds the marker line and the exact
  `exec` line this install would write. `--force` stops with exit 2 when
  `pre-commit.before-theos` exists already — a backup is never destroyed — cost if wrong: the
  user deletes an old backup by hand.
- L18. Ruling: a hooks folder that does not exist is judged by its nearest existing ancestor,
  canonicalized, plus the rest of its path, and is created only after it passes — a repo-local
  `core.hooksPath` may name a new folder — cost if wrong: none.
- L19. Ruling: `theos schema` reads the configured severities from the project's settings file
  with `settings::parse` and `rules::effective`, without opening the index — schema needs no
  index — cost if wrong: none.
- L20. Ruling: each test crate has its own small git helper — G4 allows no test crate — cost if
  wrong: a few duplicated lines.
- L21. Ruling: a relative `binaries_root` is used as written — the user layer holds absolute
  paths (architecture §13) — cost if wrong: one join in P14.
- L22. Ruling: `check` merges binary findings into index order: path, then position (none
  first), rule, message and span start — the index's own order (P03) — cost if wrong: an order.
- L23. Ruling: `check_staged` and `install_hook` call `repo::top` first and turn its failure
  into `StudioError::NotInRepository` — one clear message for a folder outside git — cost if
  wrong: one more `rev-parse`.
- L24. Ruling: the snapshot's changes list the `Content` changes in `modified` order, then the
  `Removed` changes in cache order — deterministic input — cost if wrong: none.
- L25. Ruling: the hook names the installing `theos` by `std::env::current_exe`, with `\` written
  as `/` — Git for Windows' `sh` reads `C:/…` — cost if wrong: a re-install.
- L26. Ruling: P04's test `a_folder_without_theos_toml_exits_with_1` becomes `…_exits_with_2` —
  spec §5.4 moves failures to 2; the test is changed to the spec, not weakened — cost if wrong:
  none.

## File structure

```text
Cargo.toml                                + which (Task 1), dirs (Task 7) in workspace.dependencies
crates/theos-vcs/
  Cargo.toml                              thiserror, which; dev: tempfile (Task 1)
  README.md                               what the crate provides after P05 (Task 13)
  src/lib.rs                              docs, `pub mod repo, staged`, re-exports Git, GitError
  src/git.rs                              Git, GitError (Task 1)
  src/repo.rs                             top, place, hooks_dir, common_dir (Task 1)
  src/staged.rs                           Staged, StagedPath, Status, Tracked, read (Task 5)
  src/staged/parse.rs                     the -z, ls-files -s and cat-file --batch readers (Task 5)
  tests/vcs/{main,common,git,repo,staged}.rs
crates/theos-studio/
  Cargo.toml                              theos-content, theos-vcs, thiserror, tracing, serde,
                                          toml, dirs; dev: tempfile, serde_json, rusqlite, ignore
  README.md                               (Task 13)
  src/lib.rs                              docs, modules, re-exports
  src/error.rs                            StudioError (Task 2; variants added in 7, 9, 10, 11)
  src/workspace.rs                        Workspace, find_root (Task 2)
  src/report.rs                           report (Task 2)
  src/introduced.rs                       FindingCounts, introduced (Task 3)
  src/staged.rs                           StagedCheck, check_staged (Task 6)
  src/check.rs                            BinariesFolder, Binaries, Check, check, order (Task 7)
  src/user_settings.rs                    folder, binaries_root (Task 7)
  src/schema.rs                           overview, kind (Task 9)
  src/create.rs                           NewRequest, Created, CreateError, create (Task 10)
  src/hook.rs                             Installed, HookError, install_hook (Task 11)
  src/views/{mod,check,ledger,schema,new,hook}.rs                    (Tasks 7–11)
  tests/studio/{main,common,workspace,staged,check,new,hook}.rs
  tests/fixtures/projects/staged/         the clean CE-17.2 project (Task 6)
  tests/fixtures/staged-variants/         C-901.md, T-900-lowered.md, settings-lowering.toml,
                                          settings-raising.toml (Task 6)
crates/theos-content/
  src/index/sync.rs                       + Index::snapshot (Task 4)
  src/index/query.rs                      + Index::paths, all_findings_in (Task 4)
  src/kinds/spec.rs                       + IdRule::number_prefix, Placement::under_parent (Task 10)
  src/index/resolve/placement.rs          uses Placement::under_parent (Task 10)
  src/lib.rs                              crate docs: the P05 additions (Tasks 4, 10)
  tests/index/snapshot.rs                 (Task 4)
crates/theos/
  Cargo.toml                              + theos-studio, theos-vcs, serde_json; dev: serde_json,
                                          toml (Task 8)
  README.md                               the commands, exit codes and line format (Task 13)
  src/main.rs                             Cli, Command, Exit, run, main (Task 8; arms in 9–11)
  src/output.rs                           workspace, findings, json, summary, notes (Task 8)
  src/{check,ledger,formalism,schema,new,hook}.rs                      (Tasks 8–11)
  tests/cli/{main,common,check,ledger,formalism,schema,new,hook,commit}.rs
```

---

### Task 1: The git runner and the repository queries

**ACs:** none directly; the groundwork of CE-17.2.

**Files:**
- Modify: `Cargo.toml` (`which = "8.0"` in `[workspace.dependencies]`), `crates/theos-vcs/Cargo.toml`
  (`thiserror`, `which`; dev `tempfile`), `crates/theos-vcs/src/lib.rs`
- Create: `crates/theos-vcs/src/{git,repo}.rs`, `crates/theos-vcs/tests/vcs/{main,common,git,repo}.rs`

**Interfaces — produces:**

```rust
// theos_vcs (git.rs, re-exported at the root)
pub struct Git { program: PathBuf, env: Vec<(String, String)> }
impl Git {
    pub fn find() -> Result<Git, GitError>;                     // which::which("git")
    pub fn with_env(self, env: &[(&str, &str)]) -> Git;         // L1
    pub fn run(&self, args: &[&OsStr], cwd: &Path, stdin: Option<&[u8]>,
               env: &[(&str, &str)]) -> Result<Vec<u8>, GitError>;   // stdout
}
fn find_in(path: Option<OsString>, cwd: &Path) -> Result<Git, GitError>; // which::which_in

#[non_exhaustive] #[derive(Debug, thiserror::Error)]
pub enum GitError {
    NotFound,                                                        // "git was not found on the PATH"
    Failed { args: Vec<String>, code: Option<i32>, stderr: String }, // stderr whole, lossy, trimmed
    Io { source: std::io::Error },                                   // spawn or pipe failed
    Output { args: Vec<String>, message: String },                   // L2
}

// theos_vcs::repo
pub fn top(git: &Git, folder: &Path) -> Result<PathBuf, GitError>;      // rev-parse --show-toplevel in folder (L3)
pub fn place(top: &Path, folder: &Path) -> Result<String, GitError>;    // L4; "" at the top, "a/b" below
pub fn hooks_dir(git: &Git, top: &Path) -> Result<PathBuf, GitError>;   // rev-parse --git-path hooks, joined to top when relative
pub fn common_dir(git: &Git, top: &Path) -> Result<PathBuf, GitError>;  // rev-parse --path-format=absolute --git-common-dir
```

- `run` runs `program` with `args` in `cwd`, `self.env` then `env` added to the inherited
  environment, stdin piped when given (else null), stdout and stderr piped. Stdin is written per
  L6. A non-zero exit is `Failed`.
- `place` fails with `Io` when either path cannot be canonicalized, and with `Output` when the
  folder is not under the top. Git's answers end in `\n`; trim exactly one.

- [ ] **Step 1: Write the failing tests.**
  - `tests/vcs/common.rs`: `Sandbox::new()` makes a temp folder holding an empty `gitconfig`, and
    gives `git()`, the isolated `Git::find()?.with_env(&[("GIT_CONFIG_GLOBAL", …),
    ("GIT_CONFIG_NOSYSTEM", "1")])`. `init(rel) -> PathBuf` runs `init -q -b main`. It also has
    `write(path, text)`, `run(cwd, args) -> String` and `commit(cwd, message)`.
  - `tests/vcs/git.rs`:
    - `run_returns_stdout`: `--version` starts with `git version`;
    - `a_failing_command_is_failed_with_its_stderr`: `rev-parse --show-toplevel` in a folder
      outside any repository gives `Failed { code: Some(128), .. }`, and `stderr` contains
      `not a git repository`;
    - `stdin_reaches_git`: `hash-object --stdin` of `abc` gives
      `f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f\n`;
    - `a_large_answer_does_not_block`: a 3 MiB blob is written with `hash-object -w --stdin`,
      then read with `cat-file --batch`. The output is longer than 3 MiB;
    - `with_env_reaches_every_command`: `with_env(&[("GIT_AUTHOR_NAME", "Synthetic Author"),
      ("GIT_AUTHOR_EMAIL", "a@example.invalid")])`, then `var GIT_AUTHOR_IDENT` contains
      `Synthetic Author <a@example.invalid>`.
  - Unit tests in `git.rs`:
    - `an_empty_path_finds_no_git`: `find_in(Some("".into()), cwd)` is `NotFound`;
    - `a_missing_program_is_io`: `Git { program: <temp>/no-such-git, env: vec![] }.run(…)` is
      `Io`.
  - `tests/vcs/repo.rs`:
    - `top_of_a_subfolder_is_the_repository_folder`, compared after canonicalizing both;
    - `place_is_empty_at_the_top_and_a_slashed_path_below`: the top gives `""`, and
      `content/Inhalt ä b` gives `content/Inhalt ä b`;
    - `top_outside_a_repository_fails`: `Failed`;
    - `hooks_dir_is_dot_git_hooks_by_default`;
    - `hooks_dir_honours_a_local_hooks_path`: `config core.hooksPath githooks` gives
      `<top>/githooks`;
    - `a_linked_worktree_shares_the_common_hooks`: `worktree add`. In the worktree,
      `hooks_dir` equals `common_dir/hooks`, and `common_dir` is the main repository's `.git`.
- [ ] **Step 2: Run them.** `cargo nextest run -p theos-vcs`. Expected: FAIL, nothing is defined.
- [ ] **Step 3: Implement** `git.rs`, `repo.rs` and the `lib.rs` docs and modules.
- [ ] **Step 4: Run** the per-task checks and `cargo deny --locked check`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(vcs): run git and query the repository`.

### Task 2: StudioError, Workspace and the error report

**ACs:** none directly; spec §5.3 and §5.12, used by every later task.

**Before you start:** run `git fetch origin`.
- **If `origin/main` already holds `crates/theos-studio/src/workspace.rs`:** run
  `git merge origin/main` and write only what is missing here.
- **Otherwise P06 has written these files on its pushed branch,**
  `origin/phase/P06-command-layer-and-repo-lock`. P06 copied this plan's shapes. Read P06's
  `crates/theos-studio/src/{error,workspace}.rs`, `Cargo.toml` and
  `tests/studio/{main,common,workspace}.rs` with `git show origin/phase/P06-…:<path>`. Write
  P05's files to match them line for line wherever the interfaces agree, so the later merge has
  only small add/add conflicts:
  - P05 leaves out P06's `commands: CommandState` field, its line in `open`, and P06's
    `command`, `lock` and helper modules and tests;
  - P05 adds what P06 lacks: `StudioError::Git` and `NotInRepository`, and `report.rs`;
  - P05's `tests/studio/common.rs` uses P06's helper names, `TempProject::{new, copy_of, root}`
    and `files(root)`, and adds only P05's own helpers;
  - `Cargo.toml` keeps the dependency lines in the same order as P06's, plus P05's
    `theos-vcs`.

  Never cherry-pick or merge P06's branch: its command layer is P06's to merge.

**Files:**
- Modify: `crates/theos-studio/Cargo.toml` (`theos-content` and `theos-vcs` by path, `thiserror`,
  `tracing`; dev `tempfile`, `rusqlite`, `ignore`), `crates/theos-studio/src/lib.rs`
- Create: `crates/theos-studio/src/{error,workspace,report}.rs`,
  `crates/theos-studio/tests/studio/{main,common,workspace}.rs`

**Interfaces — produces:**

```rust
// error.rs
#[non_exhaustive] #[derive(Debug, thiserror::Error)]
pub enum StudioError {
    #[error(transparent)] Project(#[from] ProjectError),
    #[error(transparent)] Registry(#[from] RegistryError),
    #[error(transparent)] Index(#[from] IndexError),
    #[error(transparent)] Git(#[from] GitError),
    Io { path: PathBuf, source: std::io::Error },          // "{path}: {source}"
    NotAProject { start: PathBuf },   // "no theos.toml in `{start}` or any folder above it"
    NotInRepository { folder: PathBuf }, // "`{folder}` is not in a git repository"
}
// workspace.rs
pub struct Workspace { project: Project, registry: KindRegistry, index: Index, in_memory: bool }
impl Workspace {
    pub fn open(root: &Path) -> Result<Workspace, StudioError>;
    pub fn find(start: &Path) -> Result<Workspace, StudioError>;   // open(find_root(start)?)
    pub fn project(&self) -> &Project;
    pub fn registry(&self) -> &KindRegistry;
    pub fn index(&self) -> &Index;
    pub fn index_mut(&mut self) -> &mut Index;
    pub fn is_in_memory(&self) -> bool;
    pub fn write<T>(&mut self, f: impl FnMut(&mut Index) -> Result<T, IndexError>)
        -> Result<T, StudioError>;                                  // L8
}
pub fn find_root(start: &Path) -> Result<PathBuf, StudioError>;
fn fallback(opened: Result<Index, IndexError>,
            in_memory: impl FnOnce() -> Result<Index, IndexError>) -> Result<(Index, bool), IndexError>;
// report.rs
pub fn report(error: &dyn std::error::Error) -> String;
```

- **`open`:** `Project::open`, then `parts::builtin()`, then `fallback(Index::open(…).map(|(i, _)|
  i), || { Index::in_memory(…) then sync() })`. A fallback logs `tracing::warn!` with the cache
  path.
- **`find_root`:** makes `start` absolute with `std::path::absolute`, then takes the first of its
  ancestors that holds a `theos.toml` file. None is `NotAProject { start }`.
- **`write`:** runs `f`. On `Err(IndexError::Contended { .. })`, while not yet in memory, it
  builds and syncs the in-memory index, sets `in_memory`, and runs `f` once more.
- **`report`:** starts with `error.to_string()`. It walks `source()` and adds `": "` and each
  cause's text, unless the text so far already contains it (spec §5.12).

- [ ] **Step 1: Write the failing tests.**
  - Unit tests in `report.rs`:
    - `each_cause_appears_once`, for each of the nine variants of spec §5.12, built directly:
      - `FormalismError::Source(SourceError { message: "m", position: None, span: None })`;
      - `FormalismError::Io { path, source: io::Error::other("disk full") }`;
      - `IndexError::Sqlite(rusqlite::Error::QueryReturnedNoRows)`;
      - `IndexError::Cache { .. }` and `IndexError::CacheInUse { .. }`;
      - `ProjectError::Io { .. }` and `ReadError::Io { .. }`;
      - `WalkError::from(ignore::Error::Io(io::Error::other("denied")))`;
      - `FactError::Sqlite(..)`.

      Each is wrapped in a `StudioError` where a variant exists. The innermost cause's text
      appears in `report` exactly once.
    - `a_cause_the_text_lacks_is_added`: a two-level test error whose outer message omits the
      inner one gives `outer: inner`.
  - Unit tests in `workspace.rs`, on a temp project holding only `theos.toml`:
    - `a_contended_cache_falls_back_to_memory`: `fallback(Err(Contended { .. }), || in_memory)`
      gives `(index, true)`;
    - `other_errors_pass_through`: `Err(Busy { .. })` stays `Busy`, and the closure is not
      called;
    - `an_opened_index_is_kept`: `(index, false)`;
    - `a_contended_write_runs_again_in_memory`: a `write` closure returns `Contended` on its
      first call and `Ok(7)` on its second. The result is `Ok(7)`, and `is_in_memory()` is true.
  - `tests/studio/workspace.rs`. `common.rs` copies a `theos-content` fixture project into a
    temp folder.
    - `open_syncs_the_index`: P04's `lore` copy lists its records;
    - `find_walks_up_to_theos_toml`: from `<root>/lore/clues`;
    - `find_without_a_project_is_not_a_project`;
    - `open_of_a_folder_without_theos_toml_fails_with_the_project_error`.
- [ ] **Step 2: Run them** — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the per-task checks — PASS.
- [ ] **Step 5: Commit** `feat(studio): open a workspace and report errors once`.

### Task 3: The shared verdict `introduced`

**ACs:** none directly; Task 10's `theos new` proves CE-17.1 through it.

**Before you start:** run `git fetch origin`.
- If `origin/main` holds `crates/theos-studio/src/introduced.rs`, run `git merge origin/main`,
  note "Task 3 taken from P06 (#PR)" in the ledger, and go to Task 4.
- Otherwise read P06's version on `origin/phase/P06-command-layer-and-repo-lock` and write
  P05's line for line like it, with its unit tests, as in Task 2.

**Files:** create `crates/theos-studio/src/introduced.rs`; modify `src/lib.rs`.

**Interfaces — produces** (P06's signature, L9):

```rust
/// Findings counted per (path, rule), with the messages seen (P06 spec §5.5).
pub struct FindingCounts { counts: BTreeMap<(String, String), usize>,
                           messages: BTreeSet<(String, String, String)> }
impl FindingCounts {
    /// Counts `findings` of every severity; `moved` maps old path -> new path.
    pub fn of(findings: &[Finding], moved: &BTreeMap<String, String>) -> FindingCounts;
    /// The larger count per key; messages united.
    pub fn max(&self, other: &FindingCounts) -> FindingCounts;
    /// The count of (path, rule); 0 when absent. P06's verdict uses it for new warnings.
    pub fn count(&self, path: &str, rule: &str) -> usize;
}
/// The errors of `after` above `allowed`'s count for their (path, rule).
pub fn introduced(allowed: &FindingCounts, after: &[Finding]) -> Vec<Finding>;
```

**Selection:** count the errors of `after` (severity `Error`) per (path, rule). Where `e` errors
exceed the allowed count `a`, take `e − a` of them:
- first, in `after` order, those whose (path, rule, message) `allowed` has not seen;
- then the rest, also in `after` order.

Return the taken findings in `after` order. Warnings are never introduced.

- [ ] **Step 1: Write the failing unit tests:**
  - `every_error_on_a_new_path_is_introduced`;
  - `a_standing_error_is_not_introduced`, also when its message changed;
  - `a_second_error_of_a_rule_is_introduced` gives exactly the one with the new message;
  - `a_warning_is_never_introduced`;
  - `a_raised_severity_is_not_introduced`: before one warning, after one error of the same rule
    and path;
  - `moved_paths_count_under_their_new_path`;
  - `max_takes_the_larger_count_and_unites_messages`;
  - `count_is_zero_for_an_unknown_key`. Match P06's test names where its file has them.
- [ ] **Step 2: Run them** — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the per-task checks — PASS.
- [ ] **Step 5: Commit** `feat(studio): count introduced errors per path and rule`.

### Task 4: The snapshot preview and the cached paths

**ACs:** CE-17.2 (the whole-snapshot check of spec §5.7).

**Files:**
- Modify: `crates/theos-content/src/index/sync.rs` (`snapshot`), `src/index/query.rs`
  (`paths`, and `all_findings_in`, which `findings()` uses too), `src/lib.rs` (crate docs),
  `tests/index/main.rs` (`mod snapshot;`)
- Create: `crates/theos-content/tests/index/snapshot.rs`

**Interfaces — produces:**

```rust
impl Index {
    /// Every path the cache holds a file or an input-file row for, sorted.
    pub fn paths(&self) -> Result<Vec<String>, IndexError>;
    /// Applies `changes` as `apply` does, reads every finding of the result with the
    /// severities the result's own settings configure, and rolls back.
    pub fn snapshot(&mut self, changes: &[Change]) -> Result<Vec<Finding>, IndexError>;
}
pub(super) fn all_findings_in(conn: &Connection, registry: &KindRegistry) -> Result<Vec<Finding>, IndexError>;
```

- `paths` runs `SELECT path FROM files UNION SELECT path FROM inputs ORDER BY path`. An input row
  exists only while its file does (`settle_input`).
- `snapshot` is `self.transact(|work| apply_changes(work, changes), |conn, registry, _| all_findings_in(conn, registry), false)`.
  It reads all findings, configured, in index order, inside the transaction.

- [ ] **Step 1: Write the failing tests** in `tests/index/snapshot.rs`, on `TempProject` copies of
  P04's `lore` and `settings` fixtures:
  - `paths_lists_records_and_input_files`: holds `.theos/settings.toml` and every record path,
    and nothing else;
  - `// AC: CE-17.2` `a_snapshot_holds_every_finding_of_the_changed_state`:
    - with no change it equals `findings()`;
    - with a truth's target lowered (`Change::text`), an untouched clue of it shows
      `lore/no-overshoot`;
    - it includes findings on paths the change did not touch;
  - `a_settings_change_applies_to_untouched_paths`: settings content that sets a configurable
    rule to `error` turns that rule's findings on untouched records into errors;
  - `a_staged_guardrail_lowering_is_an_error`: settings content lowering `lore/never-confirm`
    gives `settings/guardrail` as an error;
  - `a_snapshot_leaves_the_cache_unchanged`: `dump()` is the same before and after.
- [ ] **Step 2: Run them** — FAIL.
- [ ] **Step 3: Implement**, and add the P05 additions to the crate docs.
- [ ] **Step 4: Run** the per-task checks — PASS.
- [ ] **Step 5: Commit** `feat(content): preview the findings of a whole snapshot` with
  `Refs: CE-17.2`.

### Task 5: The staged snapshot through git

**ACs:** none directly; the reader behind CE-17.2.

**Files:** create `crates/theos-vcs/src/staged.rs`, `src/staged/parse.rs` and
`tests/vcs/staged.rs`; modify `src/lib.rs`.

**Interfaces — produces** (spec §5.2):

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Staged { pub top: PathBuf, pub changed: Vec<StagedPath>, pub tracked: Vec<Tracked>,
                    pub modified: Vec<(Vec<u8>, Vec<u8>)> }        // (path, staged bytes)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedPath { pub path: Vec<u8>, pub status: Status }
#[non_exhaustive] #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status { Added, Deleted, Modified, TypeChanged, Unmerged, Other(u8) }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tracked { pub path: Vec<u8>, pub mode: u32 }            // mode parsed as octal
impl Tracked { pub fn is_regular(&self) -> bool }                  // 0o100644 or 0o100755
pub fn read(git: &Git, folder: &Path) -> Result<Staged, GitError>;
// parse.rs (private, each Err(String) becomes GitError::Output with the command's args)
fn name_status(out: &[u8]) -> Result<Vec<StagedPath>, String>;
fn ls_files(out: &[u8]) -> Result<Vec<Entry>, String>;              // Entry { path, mode, object }, stage 0 only (L5)
fn batch(out: &[u8], count: usize) -> Result<Vec<Vec<u8>>, String>; // `<oid> <type> <size>\n<bytes>\n`
```

**`read`** runs these commands in this order. Every command after the first runs in the top
folder. The pathspec is `place`, or `.` when that is empty.
1. `repo::top(git, folder)` and `repo::place(&top, folder)`.
2. `--no-optional-locks diff --cached --name-status --no-renames -z` gives `changed`.
3. `--no-optional-locks --literal-pathspecs ls-files -s -z -- <pathspec>` gives `tracked`.
4. `--no-optional-locks --literal-pathspecs diff --name-only --no-renames -z -- <pathspec>`. Its
   paths that are regular entries are read with one `cat-file --batch`, which gets
   `<object>\n` per path. That gives `modified`. When no path is modified, `cat-file` is not
   run.

A `<oid> missing` answer, a short read or a wrong count is `Output`.

- [ ] **Step 1: Write the failing tests.**
  - Unit tests in `parse.rs`:
    - `name_status_reads_letters_and_paths`, including a path with a newline and a space;
    - `ls_files_reads_mode_object_and_path_and_keeps_stage_zero`;
    - `batch_reads_each_blob`;
    - `batch_rejects_a_missing_object_and_a_short_blob`.
  - `tests/vcs/staged.rs`:
    - `a_file_staged_and_edited_again_gives_its_staged_bytes`;
    - `a_file_deleted_on_disk_but_staged_gives_its_staged_bytes`;
    - `a_staged_deletion_is_not_tracked`: `rm --cached`, file still on disk. `changed` holds it
      as `Deleted`, and `tracked` does not;
    - `another_index_is_read_through_the_environment`: copy `.git/index` to `.git/other-index`,
      stage a change with `GIT_INDEX_FILE=.git/other-index`, then `read` with
      `git.with_env(&[("GIT_INDEX_FILE", ".git/other-index")])` sees it. The plain index does
      not;
    - `tracked_keeps_the_modes_of_links_and_submodules`: two entries made with `update-index
      --add --cacheinfo`, `120000,<blob>,lore/clues/C-901.md` and
      `160000,<HEAD commit>,lore/sub`. They come back with those modes, `is_regular()` is false,
      and neither is in `modified`;
    - `a_project_in_a_subfolder_reads_only_its_folder`: `tracked` lists only `content/…`,
      repository-relative, and `changed` lists both folders;
    - `the_first_commit_lists_every_staged_file_as_added`: an unborn `HEAD`;
    - `a_folder_with_glob_characters_and_umlauts_is_read_literally`: a project in
      `[draft] Inhalt ä/` with a sibling `d/` lists exactly its own entries.
- [ ] **Step 2: Run them** — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the per-task checks — PASS.
- [ ] **Step 5: Commit** `feat(vcs): read the staged snapshot`.

### Task 6: The staged check

**ACs:** CE-17.2 (library level; Task 12 runs it through git's hook).

**Files:**
- Create: `crates/theos-studio/src/staged.rs`, `tests/studio/staged.rs`, the fixtures below
- Modify: `src/lib.rs`, `tests/studio/{main,common}.rs` (an isolated repository helper as in Task
  1)

**Fixtures.** `crates/theos-studio/tests/fixtures/projects/staged/` is a clean project:
- `theos.toml` and a `.gitignore` of `/.theos/cache`;
- the place records a clue's `where` and `gate` need, modelled on P04's `lore` fixture
  (`world/`);
- `lore/truths/T-900.md` (target `confirmed`), `lore/clues/C-900.md` (reveals T-900,
  `moves_to: confirmed`) and `lore/truths/T-901.md` (target `never`);
- `.theos/settings.toml` holding only a comment that says it is synthetic.

`tests/fixtures/staged-variants/` holds:
- `C-901.md`: reveals T-901 with `moves_to: hinted`, a never-confirm violation;
- `T-900-lowered.md`: T-900 with target `suspected`, so C-900 overshoots;
- `settings-lowering.toml`: sets `lore/never-confirm` to `warning`, which gives
  `settings/guardrail`;
- `settings-raising.toml`: sets one configurable rule that the clean project reports as a warning
  to `error`. Pick it by running `check` on the fixture.

**Interfaces — produces:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedCheck { pub findings: Vec<Finding>, pub changed: Vec<String> }
impl StagedCheck {
    pub fn blocks(&self) -> bool;          // any Severity::Error
    pub fn errors(&self) -> usize;
    pub fn warnings(&self) -> usize;
}
pub fn check_staged(workspace: &mut Workspace, git: &Git) -> Result<StagedCheck, StudioError>;
fn project_path(place: &str, repo_path: &[u8]) -> Option<String>;
fn changes(cached: &[String], tracked: &BTreeMap<String, u32>,
           modified: Vec<(String, Vec<u8>)>) -> Vec<Change>;            // L24
```

**Steps of `check_staged`** (spec §5.7):
1. `repo::top`, whose failure is `NotInRepository` (L23).
2. `staged::read(git, root)`, then `repo::place(&staged.top, root)`.
3. Map every path with `project_path`. It drops paths outside the place and paths that are not
   UTF-8.
4. `changes(&index.paths()?, …)`:
   - `Change::Content` for each modified path;
   - `Change::Removed` for each cached path that is not a regular file in `tracked`;
   - nothing for the rest.
5. `workspace.write(|index| index.snapshot(&changes))`.
6. Put the findings on `changed` paths first, then the rest, keeping index order in each group.

- [ ] **Step 1: Write the failing tests.**
  - Unit tests in `staged.rs`:
    - `project_path_strips_the_place`: place `""` and `content`. `content2/x.md` is outside,
      and a non-UTF-8 path is dropped;
    - `changes_cover_modified_untracked_and_irregular`: cached `[a, b, c, d,
      .theos/settings.toml]`, tracked `{a: 0o100644, b: 0o120000, c: 0o100644}` and modified
      `[c]` give `[Content c, Removed b, Removed d, Removed .theos/settings.toml]`.
  - `tests/studio/staged.rs`. Each test copies the fixture into a fresh repository, commits it,
    then stages its variant:
    - `the_clean_fixture_has_no_error`: `check_staged` is clean, and `blocks()` is false;
    - `// AC: CE-17.2` `a_staged_never_confirm_violation_blocks`: `C-901` added. It blocks, a
      finding has rule `lore/never-confirm` and path `lore/clues/C-901.md`, the first finding is
      on a `changed` path, and `changed` holds that path;
    - `// AC: CE-17.2` `a_fix_only_in_the_working_tree_does_not_unblock`: `C-901` staged, then
      deleted on disk. It still blocks;
    - `// AC: CE-17.2` `a_violation_only_in_the_working_tree_does_not_block`: `C-901` untracked
      on disk, then a clean staged `C-901` with a violating working copy. Neither blocks;
    - `// AC: CE-17.2` `a_lowered_target_blocks_through_the_committed_clue`: `T-900` lowered and
      staged in full. It blocks on `lore/no-overshoot` at `lore/clues/C-900.md`, which is not in
      `changed`;
    - `// AC: CE-17.2` `a_staged_guardrail_lowering_blocks`: on `settings/guardrail`;
    - `a_locally_excluded_record_does_not_block`: `C-901` on disk and listed in
      `.git/info/exclude`;
    - `a_staged_deletion_of_a_file_on_disk_is_checked_as_gone`: `rm --cached
      lore/truths/T-900.md`. C-900's `reveals` no longer resolves in the findings, read from
      `check_staged`, not the working tree;
    - `links_and_submodules_at_record_paths_are_not_records`: a violating `C-901.md` on disk,
      with a 120000 entry at that path. It does not block;
    - `a_project_in_a_subfolder_maps_its_paths`: the project in `[draft] Inhalt ä/`, a violating
      record outside it in `other/` (not checked), and `C-901` inside (blocks, with a
      project-relative path);
    - `staged_settings_decide_the_snapshot_severities`: `settings-raising.toml` staged, with the
      working copy unchanged. It blocks on that rule, on a record the commit does not touch;
    - `a_force_added_ignored_record_is_not_checked`: the project's `.gitignore` lists
      `lore/clues/C-901.md`, and a violating `C-901` is staged with `git add -f`. It does not
      block (spec §9: the index never reads such a file);
    - `a_folder_outside_git_is_not_in_a_repository`.
- [ ] **Step 2: Run them** — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the per-task checks — PASS.
- [ ] **Step 5: Commit** `feat(studio): check the staged snapshot` with `Refs: CE-17.2`.

### Task 7: Check, the user settings and the finding views

**ACs:** none directly; CE-17.1 is proven through the binary in Task 8.

**Files:**
- Modify: `Cargo.toml` (`dirs = "7.0"`), `crates/theos-studio/Cargo.toml` (`dirs`, `toml`,
  `serde`; dev `serde_json`), `src/{lib,error}.rs` (`UserSettings { path, message }`, displayed
  `{path}: {message}`)
- Create: `src/{check,user_settings}.rs`, `src/views/{mod,check}.rs`, `tests/studio/check.rs`

**Interfaces — produces:**

```rust
// check.rs
pub enum BinariesFolder { Folder(PathBuf), NotSet, Unreadable(String) }
pub fn binaries_folder(flag: Option<PathBuf>) -> BinariesFolder;  // flag, else user_settings::binaries_root()
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Binaries { Checked { folder: PathBuf }, NotChecked { reason: String } }
pub struct Check { pub findings: Vec<Finding>, pub binaries: Binaries }
pub fn check(workspace: &mut Workspace, binaries: BinariesFolder) -> Result<Check, StudioError>;
pub fn order(findings: &mut [Finding]);                            // L22
pub fn counts(findings: &[Finding]) -> (usize, usize);             // (errors, warnings)
// user_settings.rs
pub fn folder() -> Option<PathBuf>;              // THEOS_CONFIG_DIR, else dirs::config_dir()/theos
pub fn binaries_root() -> Result<Option<PathBuf>, StudioError>;
fn binaries_root_in(folder: &Path) -> Result<Option<PathBuf>, StudioError>;
// views/check.rs, all #[derive(Serialize)], snake case, None serialised as null
pub struct FindingView { rule: String, severity: &'static str, path: String, id: Option<String>,
    line: Option<usize>, column: Option<usize>, span: Option<SpanView>, message: String }
pub struct SpanView { start: usize, end: usize }
pub struct BinariesView { checked: bool, folder: Option<String>, reason: Option<String> }   // L10
pub struct CheckView { findings: Vec<FindingView>, errors: usize, warnings: usize, binaries: BinariesView }
pub struct StagedView { findings: Vec<FindingView>, errors: usize, warnings: usize, changed: Vec<String> }
// From<&Finding>, From<&Check>, From<&StagedCheck>
```

- **`check`:**
  - takes the findings from `index().findings()`;
  - for `Folder(f)`, adds `workspace.write(|i| i.binary_findings(&f))`'s findings and calls
    `order`;
  - `NotSet` gives `NotChecked { reason: "no binaries folder set" }`;
  - `Unreadable(r)` gives `NotChecked { reason: r }`.
- **`binaries_folder(None)`** maps `binaries_root()`:
  - `Ok(Some(f))` to `Folder(f)`;
  - `Ok(None)` to `NotSet`;
  - `Err(e)` to `Unreadable(report(&e))`.
- **`binaries_root_in`:**
  - reads `<folder>/settings.toml`; a missing file is `None`;
  - parses it with `toml::from_str::<toml::Table>`;
  - takes a top-level string `binaries_root`;
  - invalid TOML or a non-string value is `UserSettings`, and another read error is `Io`.

- [ ] **Step 1: Write the failing tests.**
  - Unit tests in `user_settings.rs`:
    - `no_file_means_no_folder`;
    - `the_key_gives_the_folder`;
    - `invalid_toml_names_the_file`: the message starts with the file's path;
    - `a_value_that_is_not_text_is_an_error`.
  - Unit tests in `views/check.rs`:
    - `a_finding_without_a_position_has_null_line_column_and_span`;
    - `a_finding_on_an_input_file_has_a_null_id`;
    - `the_check_view_has_every_key`: the key sets of `serde_json::to_value`, also inside
      `binaries`;
    - `the_staged_view_has_changed_and_no_binaries`.
  - `tests/studio/check.rs`, on a copy of P04's `binaries` project with the folder
    `theos-content/tests/fixtures/binaries`:
    - `a_binaries_folder_adds_its_warnings_in_index_order`;
    - `no_folder_says_why`;
    - `an_unreadable_setting_is_the_reason`.
- [ ] **Step 2: Run them** — FAIL.
- [ ] **Step 3: Implement.** Run `cargo deny --locked check`.
- [ ] **Step 4: Run** the per-task checks — PASS.
- [ ] **Step 5: Commit** `feat(studio): check a project with its binaries folder`.

### Task 8: The command line: exit codes, `check`, `ledger` and `formalism`

**ACs:** CE-17.1 (check, ledger, `--json`), CE-17.2 (`theos check --staged`).

**Files:**
- Modify: `crates/theos/Cargo.toml` (`theos-studio`, `theos-vcs`, `serde_json`; dev `serde_json`,
  `toml`), `src/{main,ledger,formalism}.rs`, `crates/theos-studio/src/views/mod.rs` (`ledger`,
  `formalism`)
- Create: `crates/theos/src/{output,check}.rs`, `crates/theos-studio/src/views/ledger.rs`,
  `crates/theos/tests/cli/{main,common,check,ledger,formalism}.rs`
- Delete: `crates/theos/tests/cli.rs`. Its three tests move to `ledger.rs` and `formalism.rs`
  (L26).

**Interfaces — produces:**

```rust
// main.rs
#[derive(Parser)] #[command(version, about)]
struct Cli { #[arg(long, global = true)] json: bool, #[command(subcommand)] command: Command }   // L11
enum Command {
    Check { folder: Option<PathBuf>, #[arg(long)] staged: bool,
            #[arg(long, conflicts_with = "staged")] binaries: Option<PathBuf> },           // L12
    Ledger { folder: Option<PathBuf> },
    Formalism { folder: Option<PathBuf> },
    // Schema (Task 9), New (Task 10), Hook (Task 11)
}
enum Exit { Ok, Errors, Refused }          // ExitCode 0, 1, 2
fn run(cli: Cli, out: &mut impl Write, err: &mut impl Write) -> anyhow::Result<Exit>;
fn main() -> ExitCode;                     // Err(e): writeln!(stderr, "error: {}", report(e.as_ref())), exit 2
// output.rs
pub(crate) fn workspace(folder: Option<&Path>) -> Result<Workspace, StudioError>;  // open(folder) or find(current_dir)
pub(crate) fn findings(out: &mut impl Write, findings: &[Finding]) -> io::Result<()>;  // one Display line each
pub(crate) fn json(out: &mut impl Write, view: &impl Serialize) -> anyhow::Result<()>;   // pretty, then "\n"
pub(crate) fn summary(errors: usize, warnings: usize) -> String;   // "1 error, 2 warnings", "0 errors, 1 warning"
pub(crate) fn in_memory_note(err: &mut impl Write, workspace: &Workspace) -> io::Result<()>;
// theos_studio::views::ledger
pub struct LedgerView(Vec<TruthView>);   // [{ id, path, title, target, entries: [{ state, floor, place,
                                         //    gate, clue, clue_path, placed_by: [{ script, beat }] }] }]
pub struct FormalismView { written: Vec<String> }
```

**Behaviour** (spec §5.4–§5.7):
- **Text:**
  - stdout gets only the finding lines (or P04's ledger and formalism text);
  - stderr gets `summary`, the binaries note `binaries not checked: <reason>`, and the
    in-memory note when `is_in_memory()`;
  - `--staged` prints `commit refused: N errors in the staged snapshot` or
    `staged snapshot: no errors, N warnings`.
- **`--json`:** stdout gets one document (`CheckView`, `StagedView`, `LedgerView`,
  `FormalismView`). The notes stay on stderr.
- **Exit:** `check` with any error is `Errors` (1); every failure is 2.

- [ ] **Step 1: Write the failing tests.**
  - **`tests/cli/common.rs`:**
    - `theos(args, cwd) -> Command`, with the isolation of the build rules and a fresh
      `THEOS_CONFIG_DIR`;
    - copies of fixture projects;
    - the repository helper.
  - **`check.rs`:**
    - `// AC: CE-17.1` `theos_check_prints_finding_lines_and_exits_1_on_errors`: the `staged`
      fixture plus `C-901`. A stdout line starts with `lore/clues/C-901.md:` and contains
      ` error lore/never-confirm: `. stderr holds the summary, and the exit is 1;
    - `// AC: CE-17.1` `theos_check_json_has_findings_counts_and_binaries`: every key of spec
      §5.11; `binaries` is `{checked: false, folder: null, reason: "no binaries folder set"}`;
    - `// AC: CE-17.1` `theos_check_reads_the_binaries_folder_from_user_settings`:
      `settings.toml` in `THEOS_CONFIG_DIR` points at P04's binaries folder, so `checked` is
      true and binary warnings appear. `--binaries` overrides it;
    - `theos_check_of_a_clean_project_exits_0`;
    - `an_unreadable_user_setting_is_a_note_not_a_failure`;
    - `theos_check_finds_the_project_above_the_current_folder`;
    - `// AC: CE-17.1` `theos_check_outside_a_project_exits_2`;
    - `staged_and_binaries_together_exit_2`;
    - `// AC: CE-17.2` `theos_check_staged_judges_the_index_not_the_working_tree`: `C-901`
      staged and then deleted on disk exits 1, with the refusal line. A clean index with
      `C-901` untracked exits 0. `--json` gives `changed` and no `binaries` key.
  - **`ledger.rs`:**
    - the moved `theos_ledger_prints_the_ledger` (`// AC: CE-15.3`, unchanged);
    - `// AC: CE-17.1` `theos_ledger_json_is_the_ledger`: P04's `lore` copy, compared field by
      field with `expected-ledger.toml`, read with `toml`.
  - **`formalism.rs`:**
    - the moved `theos_formalism_writes_the_mirror_once`;
    - `theos_formalism_json_lists_the_written_paths`;
    - `a_folder_without_theos_toml_exits_with_2`, both commands (L26);
    - `a_source_error_is_printed_once`: an invalid `_source.json`. Its message appears once on
      stderr (P04's follow-up).
- [ ] **Step 2: Run them** — FAIL.
- [ ] **Step 3: Implement.** Run `cargo deny --locked check`.
- [ ] **Step 4: Run** the per-task checks — PASS.
- [ ] **Step 5: Commit** `feat(cli): check, staged check and json output` with
  `Refs: CE-17.1` and `Refs: CE-17.2` on separate lines.

### Task 9: `theos schema`

**ACs:** CE-17.1.

**Files:**
- Modify: `crates/theos-studio/src/{lib,error}.rs` (`UnknownKind { kind, known: Vec<String> }`:
  "unknown kind `{kind}`; the kinds are: a, b, …"), `crates/theos/src/main.rs` (`Schema { kind:
  Option<String>, #[arg(long)] project: Option<PathBuf> }`)
- Create: `crates/theos-studio/src/schema.rs`, `src/views/schema.rs`, `crates/theos/src/schema.rs`,
  `crates/theos/tests/cli/schema.rs`

**Interfaces — produces:**

```rust
// theos_studio::schema
pub fn overview(registry: &KindRegistry, project: Option<&Path>) -> Result<SchemaView, StudioError>;
pub fn kind(registry: &KindRegistry, name: &str) -> Result<KindView, StudioError>;  // UnknownKind
// views/schema.rs, #[derive(Serialize)]
pub struct SchemaView { kinds: Vec<KindSummaryView>, rules: Vec<RuleView> }
pub struct KindSummaryView { name: String, part: String, id_rule: String, placement: PlacementView }
#[serde(rename_all = "lowercase")] pub enum PlacementView { Folder(String), Under(String) }
pub struct RuleView { id: String, severity: &'static str, class: &'static str, configured: Option<&'static str> }
pub struct KindView { name, part, id_rule, placement: PlacementView, fields: Vec<FieldView>, body: BodyView }
pub struct FieldView { key: String, #[serde(rename = "type")] ty: String, required: bool }  // ty: FieldType's Display
pub struct BodyView { beats: Option<Vec<BeatView>>, notes: Option<Vec<String>>, verbatim: bool }
pub struct BeatView { key: String, targets: Vec<String> }                                  // Target's Display
```

- **The configured severity** (L19) is `rules::effective(info, severities.get(id))`:
  - `severities` comes from `settings::parse` of `<project>/.theos/settings.toml`;
  - a missing file means an empty map, and a file that is not UTF-8 is `parse(None, …)`.
- **The project** is `--project` if given, else `find_root(current_dir)`. Outside a project,
  `configured` is `null` and the text leaves the column out.
- **Text:**
  - `Kinds:` then one line per kind: name, part, placement (`lore/truths/` or
    `under its parent`) and the id rule;
  - `Rules:` then one line per rule: id, default severity, class and, in a project,
    `configured <severity>`;
  - columns are padded to their widest cell, two spaces apart;
  - `theos schema <kind>` prints the kind's name and part, `id:`, `placement:`, `fields:` (one
    line each, `FieldSpec`'s Display) and `body:` (beats with targets, notes, verbatim).

- [ ] **Step 1: Write the failing tests.** Unit test in `views/schema.rs`:
  `placement_is_folder_or_under`, which checks the JSON
  `{"folder":"lore/truths"}` and `{"under":"parent"}`. In `tests/cli/schema.rs`, comparing text
  lines with whitespace collapsed:
  - `// AC: CE-17.1` `theos_schema_lists_every_kind_and_rule`: run outside a project. The JSON
    kinds equal `parts::builtin()`'s names in order, the rule ids equal `rules::all`'s, and
    every `configured` is null. The text holds a `truth` line and a `lore/never-confirm` line;
  - `// AC: CE-17.1` `theos_schema_shows_the_configured_severities`: P04's `settings` project.
    One configurable rule's `configured` differs from its `severity`, and every guardrail is
    `error`;
  - `// AC: CE-17.1` `theos_schema_of_a_kind_lists_fields_and_body`: `truth` has fields in
    front-matter order with `required`, and placement `{folder: "lore/truths"}`. `script` has
    non-null `beats` with targets;
  - `theos_schema_of_an_unknown_kind_exits_2_and_lists_the_kinds`.
- [ ] **Step 2: Run them** — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the per-task checks — PASS.
- [ ] **Step 5: Commit** `feat(cli): theos schema` with `Refs: CE-17.1`.

### Task 10: `theos new`

**ACs:** CE-17.1.

**Files:**
- Modify:
  - `crates/theos-content/src/kinds/spec.rs`: `number_prefix` and `under_parent`;
  - `src/index/resolve/placement.rs`: the check calls `Placement::under_parent`;
  - `crates/theos-studio/src/{lib,error}.rs`: `Create(#[from] CreateError)`, transparent;
  - `crates/theos/src/main.rs`: `New { kind, id: Option<String>, #[arg(long)] title:
    Option<String>, #[arg(long = "set", value_parser = field_value)] set: Vec<(String, String)>,
    #[arg(long)] project: Option<PathBuf> }`.
- Create: `crates/theos-studio/src/create.rs`, `src/views/new.rs`, `crates/theos/src/new.rs`,
  `tests/studio/new.rs`, `crates/theos/tests/cli/new.rs`

**Interfaces — produces:**

```rust
// theos_content
impl IdRule { pub fn number_prefix(&self) -> Option<&'static str> }   // "^T-[0-9]{3,}$" -> "T-" (L13)
impl Placement { pub fn under_parent(parent_path: &str, parent_id: &str) -> String }  // "<parent folder>/<parent id>"
// theos_studio::create
pub struct NewRequest<'a> { pub kind: &'a str, pub id: Option<&'a str>, pub title: Option<&'a str>,
                            pub set: &'a [(String, String)] }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Created { pub path: String, pub id: String, pub findings: Vec<Finding>, pub introduced: Vec<Finding> }
impl Created { pub fn written(&self) -> bool }   // introduced.is_empty()
pub fn create(workspace: &mut Workspace, request: &NewRequest<'_>) -> Result<Created, StudioError>;
#[non_exhaustive] #[derive(Debug, thiserror::Error)]
pub enum CreateError { IdDoesNotFit { kind, id, says }, NeedsId { kind }, UnknownField { kind, field },
    Value { field, message }, Template { path, message }, NoParent { field }, ParentMissing { id },
    Exists { path } }
fn next_id<'a>(ids: impl Iterator<Item = &'a str>, prefix: &str) -> String;   // L14
fn fill_title(template: &str, id: &str) -> String;
fn skeleton(spec: &KindSpec, id: &str) -> String;                               // L16
// views/new.rs
pub struct NewView { path: String, id: String, findings: Vec<FindingView>, introduced: Vec<FindingView> }
```

**Steps of `create`** (spec §5.9):
1. **Kind:** an unknown one is `UnknownKind`.
2. **Id:**
   - a given id must fit `id_fits`;
   - with no id, `number_prefix` gives the prefix, and `next_id` runs over `index().records()`'
     ids. Without a prefix it is `NeedsId`.
   - `next_id` adds one to the highest number. It pads to the widest existing width and at least
     three digits: `T-012` → `T-013`, `T-999` → `T-1000`, none → `T-001`.
3. **Text:** `templates/records/<kind>.template.md` with `fill_title`, else `skeleton`. Parse it
   with `Record::parse`; a failure is `Template`.
4. **Flags:**
   - `--title` becomes `PatchOp::Set { key: "title", value: RawValue::Text }`;
   - each `--set k=v` needs `spec.field(k)`, else `UnknownField`;
   - its value is read per L15, and a failure is `Value`;
   - `record.patch(&ops, &spec.order())`, where a `PatchError` is `Value` with `report`.
5. **Path:**
   - `Flat(folder)` gives `<folder>/<id>.md`, or `<id>.md` at the root;
   - `UnderParent(field)` takes the patched front matter's scalar `field` (missing is
     `NoParent`), then `index().record(parent)` (empty is `ParentMissing`), and takes the first
     record's path into `Placement::under_parent`.
6. **Verdict:**
   - `preview = workspace.write(|i| i.preview(&[Change::text(path, record.text())]))`;
   - before is `findings_for` each of `update.reparsed`, `relinked`, `revalidated`, `removed`
     and the new path;
   - `introduced = introduced(&FindingCounts::of(&before, &BTreeMap::new()), &preview.findings)`;
   - `findings = preview.findings`;
   - when `introduced` is not empty, return without writing.
7. **Write:**
   - `create_dir_all` of the parent, then `OpenOptions::new().write(true).create_new(true)`,
     where `AlreadyExists` is `Exists`;
   - write `record.text()` (LF);
   - then `workspace.write(|i| i.update(&[&path]))`.

**The CLI:**
- `field_value` splits at the first `=`; an empty key is a clap error.
- Text:
  - prints the findings;
  - when written, prints the path to stdout and exits 0;
  - when refused, prints `refused: the new record would introduce N errors` to stderr and exits
    2.
- `--json`: `NewView`, with the same exit codes.

- [ ] **Step 1: Write the failing tests.**
  - Unit tests in `kinds/spec.rs`:
    - `number_prefixes_of_the_builtin_kinds`: truth `T-`, belief `B-`, clue `C-`, script `S-`;
      npc, equation and floor give `None`;
    - `under_parent_joins_the_parent_folder_and_id`: `("lore/factions/x.md", "x")` gives
      `lore/factions/x`, and `("x.md", "x")` gives `x`.

    P03's placement tests must stay green.
  - Unit tests in `create.rs`:
    - `next_id_is_one_above_the_highest`: widths, the first id, other prefixes and `T-01a`
      ignored;
    - `fill_title_replaces_only_title`: `{{speaker}}` stays;
    - `the_truth_skeleton`: equals
      `"---\nid: T-001\nkind: truth\ntitle: T-001\nstatus: draft\n---\n"`.
  - `tests/studio/new.rs`:
    - `a_standing_error_elsewhere_does_not_refuse`: the `staged` fixture plus `C-901`, then a new
      truth. It is written, `findings` holds the standing error, and `introduced` is empty;
    - `a_clue_revealing_a_never_truth_is_refused`: every required field given by `set`.
      `introduced` holds `lore/never-confirm`, and no file exists;
    - `an_existing_id_is_never_overwritten`;
    - `an_id_in_another_case_is_refused`: `t-900` for kind truth fails `id_fits`. A slug id that
      exists in another case is refused with `Exists` (case-insensitive file systems) or with an
      introduced `index/duplicate-id` (Linux). Either way no second file exists;
    - `a_missing_parent_is_parent_missing`;
    - `a_value_the_restricted_yaml_rejects_is_a_value_error`: `--set title=a: b`.
  - `tests/cli/new.rs`. The project is the `staged` fixture plus a synthetic
    `templates/records/truth.template.md` shaped like tower-of-theos's, and a synthetic faction
    the npc kind's `parent` accepts (check `parts/codex/npc.rs`):
    - `// AC: CE-17.1` `theos_new_numbers_a_truth_from_the_template`: `theos new truth --title
      "Synthetic statement B"` writes `lore/truths/T-902.md`, with `id: T-902` and that title,
      and prints its path;
    - `// AC: CE-17.1` `theos_new_places_an_npc_under_its_parent`: `theos new npc mother-ilse
      --set parent=<faction> --title "Mother Ilse" --json`. `path` is
      `Placement::under_parent`'s folder plus `mother-ilse.md`, and `introduced` is `[]`;
    - `// AC: CE-17.1` `theos_new_refuses_an_introduced_error_and_writes_nothing`: exit 2, the
      finding printed, no file;
    - `theos_new_without_a_template_builds_a_skeleton`;
    - `unknown_kinds_fields_and_ids_exit_2`, and `a_slug_kind_without_an_id_exits_2`.
- [ ] **Step 2: Run them** — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the per-task checks — PASS.
- [ ] **Step 5: Commit** `feat(cli): theos new` with `Refs: CE-17.1`.

### Task 11: `theos hook install`

**ACs:** CE-17.1.

**Files:**
- Modify: `crates/theos-studio/src/{lib,error}.rs` (`Hook(#[from] HookError)`, transparent),
  `crates/theos/src/main.rs` (`Hook { #[command(subcommand)] action: HookAction }`, with
  `HookAction::Install { folder: Option<PathBuf>, #[arg(long)] force: bool }`)
- Create: `crates/theos-studio/src/hook.rs`, `src/views/hook.rs`, `crates/theos/src/hook.rs`,
  `tests/studio/hook.rs`, `crates/theos/tests/cli/hook.rs`

**Interfaces — produces:**

```rust
pub const MARKER: &str = "# theos pre-commit hook";
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed { pub hook: PathBuf, pub replaced: bool, pub backup: Option<PathBuf> }
pub fn install_hook(project_root: &Path, git: &Git, theos: &Path, force: bool) -> Result<Installed, StudioError>;
#[non_exhaustive] #[derive(Debug, thiserror::Error)]
pub enum HookError { OutsideGitFolder { hooks: PathBuf, git_folder: PathBuf },  // explains both cases of spec §5.8
    Foreign { path: PathBuf },  // "… exists and is not theos's hook for this project; --force keeps it as pre-commit.before-theos"
    BackupExists { path: PathBuf } }   // L17
fn script(theos: &str, folder: &str) -> String;     // spec §5.8 text, LF
fn quoted(text: &str) -> String;                   // '…' with ' written '\''
#[cfg(unix)] fn make_executable(path: &Path) -> std::io::Result<()>;     // mode 0755
#[cfg(windows)] fn make_executable(path: &Path) -> std::io::Result<()>;  // nothing to do
// views/hook.rs
pub struct HookView { hook: String, replaced: bool, backup: Option<String> }
```

**Steps of `install_hook`** (spec §5.8):
1. `repo::top`, whose failure is `NotInRepository`.
2. `place`.
3. `hooks_dir` and `common_dir`. The hooks folder is checked per L18 against the canonical common
   folder, and created after it passes.
4. `script(<theos with \ as />, <place or .>)`, with the exact lines of spec §5.8.
5. An existing `pre-commit`:
   - holding `MARKER` and the same `exec` line: replace it, and set `replaced`;
   - otherwise without `force`: `Foreign`;
   - with `force`: `BackupExists` if the backup is there, else rename the hook to
     `pre-commit.before-theos`.
6. Write the script (LF), then `make_executable`.

The CLI passes `std::env::current_exe()` (L25). It prints the hook's path, plus a line naming the
backup, or `HookView` with `--json`.

- [ ] **Step 1: Write the failing tests.**
  - Unit tests in `hook.rs`:
    - `the_script_quotes_the_path`: `C:/Program Files/it's/theos.exe` and folder
      `[draft] Inhalt ä` give the exact five-line script, with `'it'\''s'`;
    - `#[cfg(unix)]` `the_hook_is_executable`: mode `0o755`;
    - `#[cfg(windows)]` `the_hook_is_written_without_a_mode`: the file exists, and
      `make_executable` is `Ok`.
  - `tests/studio/hook.rs`:
    - `a_reinstall_replaces_its_own_hook`;
    - `a_hook_for_another_project_folder_is_foreign`;
    - `force_keeps_the_foreign_hook_as_a_backup`;
    - `force_never_overwrites_a_backup`;
    - `a_local_hooks_path_inside_git_is_used`: `core.hooksPath .git/myhooks`, which does not
      exist yet;
    - `a_tracked_hooks_path_is_refused`: `core.hooksPath githooks`;
    - `a_global_hooks_path_is_refused`: `core.hooksPath` set to an outside folder in the
      isolated global config file;
    - `a_linked_worktree_installs_into_the_common_hooks`;
    - `outside_git_is_not_in_a_repository`.
  - `tests/cli/hook.rs`:
    - `// AC: CE-17.1` `theos_hook_install_writes_the_hook`: the file starts with `#!/bin/sh`,
      holds `MARKER`, and its `exec` line names `'.'`. Exit 0, and stdout is the path;
    - `// AC: CE-17.1` `theos_hook_install_json`: `{hook, replaced: false, backup: null}`, then a
      second run gives `replaced: true`;
    - `// AC: CE-17.1` `theos_hook_install_refuses_a_foreign_hook_with_2`: `--force` then gives
      `backup` and exit 0;
    - `a_project_in_a_subfolder_names_its_folder`: the `exec` line names
      `'[draft] Inhalt ä'`.
- [ ] **Step 2: Run them** — FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the per-task checks — PASS.
- [ ] **Step 5: Commit** `feat(cli): theos hook install` with `Refs: CE-17.1`.

### Task 12: Commits through the installed hook

**ACs:** CE-17.2.

**Files:** create `crates/theos/tests/cli/commit.rs`; modify `tests/cli/main.rs`.

Every test copies the `staged` fixture into a fresh isolated repository and commits it before the
hook is installed (no hook exists yet, so nothing is skipped). Then the test runs `theos hook
install`, stages its variant and runs `git commit -m …` with the isolation env. "Refused" means: `git commit` fails,
its stderr names `lore/never-confirm` (or the rule in question) and the file, and `git rev-list
--count HEAD` is unchanged. "Commits" means: it succeeds and the count grows.

- [ ] **Step 1: Write the failing tests:**
  - `// AC: CE-17.2` `a_staged_never_confirm_violation_is_refused`;
  - `// AC: CE-17.2` `a_fix_only_in_the_working_tree_is_refused`;
  - `// AC: CE-17.2` `a_violation_only_in_the_working_tree_commits`: both shapes from Task 6;
  - `// AC: CE-17.2` `commit_a_and_commit_path_check_their_own_snapshots`:
    - `commit -a` with a tracked clue edited into a violation is refused;
    - `commit <path>` of a clean file commits while another staged path holds a violation that
      the path commit leaves out;
    - `commit <path>` of the violating path is refused;
  - `// AC: CE-17.2` `a_lowered_target_is_refused`;
  - `// AC: CE-17.2` `a_staged_guardrail_lowering_is_refused`;
  - `// AC: CE-17.2` `a_locally_excluded_record_does_not_block`;
  - `// AC: CE-17.2` `a_project_in_a_subfolder_commits_through_its_hook`: the project in
    `[draft] Inhalt ä/`; a clean commit passes, and `C-901` is refused;
  - `the_first_commit_is_checked`: the hook is installed before any commit, and a first commit
    holding `C-901` is refused;
  - `a_commit_with_autocrlf_is_checked`: `-c core.autocrlf=true`, CRLF working files; the clean
    commit passes, and `C-901` is refused;
  - `a_theos_path_with_a_space_and_a_quote_runs`: copy the binary to `<temp>/bin dir/it's/`, run
    `hook install` from there, and a violation is refused with `lore/never-confirm` named.
  - `a_moved_theos_falls_back_to_the_path`: install from such a copy, then delete it. Commit a
    violation with `PATH` set to the folder of `env!("CARGO_BIN_EXE_theos")` followed by the
    test's own `PATH`. It is refused with `lore/never-confirm` named, which shows that the
    PATH's `theos` ran.
- [ ] **Step 2: Run them.** Expected: PASS on the first run, since Tasks 6 to 11 built the
  behaviour; this task proves it end to end. Any failure is a bug: find it with
  superpowers:systematic-debugging, fix it in the owning code with its own unit test, and never
  change the test to pass.
- [ ] **Step 3: Run** the per-task checks — PASS.
- [ ] **Step 4: Commit** `test(cli): commits through the installed hook` with `Refs: CE-17.2`.

### Task 13: READMEs, crate docs and the trace

**ACs:** none new.

**Files:** modify `crates/{theos,theos-studio,theos-vcs}/README.md` and the `lib.rs` /
`main.rs` docs. In the `theos-content` crate docs, the "P05 uses …" line names the new functions.

- [ ] **Step 1: Write the READMEs** from spec §1, §5.4, §5.11 and §6:
  - `theos`: the commands, the finding line format, the exit codes, and that P49's problem
    matcher reads the lines;
  - `theos-studio`: `Workspace` and the contended fallback, the checks, the views, `report`, and
    what P06, P47 and P48 use;
  - `theos-vcs`: `Git`, `repo`, `staged`, and what P46 and P47 add.
- [ ] **Step 2: Run** the per-task checks, then `cargo xtask ac-trace --phase P05`. Expected:
  CE-17.1 and CE-17.2 each show passing tests.
- [ ] **Step 3: Commit** `docs(cli): describe the command line, workspace and git crates`.

## After the tasks

theos-phase steps 4 to 6:
1. **The final review loop** of `CLAUDE.md`: a fresh reviewer on the most capable model each
   round, cap 5. The brief includes this plan's rulings L1–L26, the P06 agreement and the spec's
   Review Focus (§8).
2. **Verify:**
   - `git fetch origin`. If P06 merged theos-studio files meanwhile, run `git merge origin/main`
     and resolve `lib.rs`, `error.rs` and the manifests by keeping both sides;
   - `cargo xtask gate` on Windows;
   - `cargo xtask ac-trace --phase P05`.
3. **The PR** `feat(cli): command line and hook (P05)`. It:
   - lists every ruling: L1–L26 above and the build's;
   - gets the labels `windows` and `macos` at creation, since the hook's mode and the Windows
     `sh` path are per OS, and links their `platforms` runs (re-dispatch
     `gh workflow run platforms --ref <branch>` after pushes);
   - sets P05 to `pr-open` in `docs/status.md` in the same push;
   - auto-merges once green, since it touches no protected path (`docs/status.md` and
     `docs/plans/**` are exceptions).
