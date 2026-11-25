# P50 · Leaner CI — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (`CLAUDE.md` makes it the execution method) to implement this plan task by task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** the gate keeps every check and verdict, runs on far fewer runner minutes, and reports
at least as fast as before: three parallel CI jobs (`gate`, `test`, `coverage`), one `pr` job,
`approval` on a cached xtask, a docs-only rule inside xtask, and `platforms` with one job per OS.

**Architecture:** the decision of which checks run lives in xtask, in the gate's runner, so CI
and the local gate decide alike:
- `changes` lists the paths a pull request or the working tree changed;
- `code_paths` reads the two lists of `gate/code-paths.toml`;
- `scope` decides "every check" or "docs only";
- `gate` applies it to the checks a job selected with `--only` or `--skip`.

The workflows stay thin: checkout, setup (or the cached xtask), one `run`. A built xtask is
cached by `main` under a key that hashes every input of its build, and `pr`, `approval` and
`settings` run that binary when the key matches.

**Tech stack:** Rust 2024 xtask (`clap`, `globset`, `toml` + `serde`, `thiserror`), the `git`
CLI, GitHub Actions (`actions/checkout`, `Swatinem/rust-cache`, `actions/cache/restore` and
`actions/cache/save`, `taiki-e/install-action`), and `yaml-rust2` and `serde_json` in the
workflow and settings tests.

**Spec:** `docs/specs/phases/P50-leaner-ci.md` (approved in #45, amended in #46). Read it with
this plan; its §5 is the design, and its §9 holds the rulings R1–R12.

**Starting point:** `main` at `2a4c459` (with #43 `slow-tests` and #46). Before Task 1, merge
`origin/main`. If #44 (P08 build) has merged by then, it changes `.github/`, `gate/` and
`xtask/`: re-read every file a task lists before starting that task.

## Global Constraints

Umbrella §2, verbatim:

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

Phase ACs, verbatim (umbrella §3 P50, version 8):

- CE-08C.1 CI runs the checks of `cargo xtask gate` in three jobs side by side: `test` runs
  `test`, `coverage` runs `coverage`, and `gate` runs every other check; `pr-title` and `pr-body`
  run in one job `pr`; the job summary of each job lists every check it ran with its verdict.
- CE-08C.2 On a pull request or a local branch that is not a `phase/` branch and whose changed
  paths all match the non-code list of `gate/code-paths.toml` and none of its always-code list,
  `cargo xtask gate` and the CI jobs run `commit-msg` only and report every other check as not
  needed; every other pull request and branch, every push, and every run whose changed paths
  cannot be read run every check.
- CE-08C.3 Every path outside the code that a check reads counts as code: the umbrella spec,
  `docs/gate.md`, `CONTRIBUTING.md`, `.github/**`, every `*.ts` file, and every `Cargo.toml`,
  `Cargo.lock`, toolchain, clippy, rustfmt or cargo configuration file in any folder.
- CE-08C.4 `docs/gate.md` lists the non-code and always-code paths exactly as
  `gate/code-paths.toml` does, and says for every check that a non-code pull request does not
  run why such a change cannot affect it.
- CE-08C.5 `pr` and `approval` take xtask from a cache that `main` saves, and no job of a pull
  request saves a build cache.
- CE-08C.6 `platforms` runs the checks of one OS in one job; its weekly and label runs test macOS
  on Apple silicon; the Intel macOS tests run only on demand; a newer label run for the same pull
  request and OS cancels the older one.

ACs of P00 this phase must keep true (reworded by versions 7 and 8; their tests change here):

- CE-08.3 Every check of `docs/gate.md` §1 (except `ui`) runs in a required status check on
  `main` (`gate`, `test`, `coverage`, `pr` or `approval`); branch protection and repo settings
  match `docs/gate.md` §6.
- CE-08.4 `cargo xtask gate` gives the same verdicts as CI on Windows and Linux.
- CE-08.5 `approval` fails for a PR that touches a protected path without the label `approved`
  and passes once it is added; CI adds `needs-jann` to such PRs.
- CE-08.12 A scheduled workflow runs `clippy` and `test` on macOS weekly and on demand.

Speed budgets (spec §5.10, Jann): on a warm cache, `gate`, `test` and `coverage` each finish a
code pull request within 1.8 minutes of the run's start (slow tests excluded); `approval` and
`pr` on a cached xtask each within 20 seconds; a docs-only pull request's jobs within 1.8
minutes. A missed budget stops the build before review (Task 10).

## Build rules (every task, every subagent)

Put this section into every implementer and reviewer brief.

- **Engineering directives** (`CLAUDE.md`):
  - Small modules with one job; shared inputs built in one place. A new rule is one module plus
    one line that registers it.
  - Use what exists: `git` for changed paths, `globset` for globs, `toml` + `serde` for
    `gate/code-paths.toml`, `clap` for arguments, `yaml-rust2` and `serde_json` in tests, GitHub's
    own actions for caches. Never hand-write a parser for TOML, YAML, JSON or Markdown; Markdown
    goes through `xtask/src/markdown.rs`.
  - If reviews keep finding new variants of one problem, stop and name the class (directive 3).
- **This phase changes protected paths only** (`xtask/**`, `gate/**`, `.github/**`, `docs/**`,
  `CONTRIBUTING.md`, `.claude/**`). That is P50's scope; the PR waits for Jann (rule 2). Never
  touch `crates/**`.
- **The gate rules stand.** No threshold, lint or test is lowered or skipped. A workflow test
  that encodes the old shape (one job per check, `cargo xtask <check>` commands) is rewritten to
  assert the new shape at least as strictly; it is never deleted without a replacement that
  checks the same property. Name the replacement in the commit body.
- **Lints** (workspace, `-D warnings`): no `#[allow]` or `#[warn]`; `#[expect]` only with
  `reason = "…"`; every public item documented, every public fn returning `Result` has an
  `# Errors` section; no `unwrap`, `expect`, `panic!`, `todo!`, `dbg!` or `print!` outside tests;
  clippy pedantic is on. Coverage counts `xtask/src/**` with its error paths: test them.
- **Dead code between tasks:** a module a later task wires in carries
  `#[cfg_attr(not(test), expect(dead_code, reason = "wired in by Task N"))]` on its `mod` line;
  the wiring task removes it.
- **Tests:** units in `#[cfg(test)] mod tests` next to the code; git fixtures through
  `xtask::git::testing` (`init_repo`, `commit`, `git`); workflow and settings tests in
  `xtask/tests/workflows.rs` and `xtask/tests/settings.rs`. A test proving an AC has
  `// AC: <id>` on the line directly above `#[test]`.
- **Workflow files** keep the comment style of the existing ones: a comment says why, not what.
  Actions stay pinned by full SHA with the version in a comment; take the SHA of
  `actions/cache/restore` and `actions/cache/save` from the `actions/cache` SHA already pinned in
  `ci.yml` (same repository, same tag).
- **Files with backslashes or `${{ }}` expressions** are written with the Write or Edit tool,
  never with a shell heredoc.
- **Git:** one git command per Bash call in the worktree. Never `--no-verify`. Commit messages:
  Conventional Commits, scope `xtask` for `xtask/src`, `ci` for `.github/` and `gate/`, `docs`
  for documentation; a body that says why; the footer block:

  ```text
  Refs: CE-08C.1
  Phase: P50
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: 4d318670-ff1b-4e4a-a319-7c5a18949528
  ```

  No footer line over 100 characters; split long `Refs:` lines; leave `Refs:` out when a task
  proves no AC.
- **Per-task checks** (worktree root; read the output): `cargo fmt --all --check`;
  `cargo clippy --workspace --all-targets --locked -- -D warnings`;
  `cargo nextest run -p xtask --locked`.

## Review Focus

The inputs most likely to break P50 that no single task's main tests cover. Each line names the
task that pins it.

1. A pull request that renames a file from `crates/` into `docs/`: both the old and the new path
   count, so it is code (Task 3, `a_rename_out_of_code_counts_both_paths`).
2. Docs-only changes on a `phase/` branch, and a `.ts` file inside `docs/`: every check runs
   (Task 4, `a_phase_branch_runs_every_check`, `a_ts_file_in_docs_is_code`).
3. A base commit that is not in the clone (shallow history): every check runs, with a note, never
   an error that skips checks (Task 4, `an_unreadable_change_list_runs_every_check`).
4. `approval` when the base commit's xtask differs from `main`'s newest: the key comes from the
   base checkout, so it restores the base's binary or builds the base's xtask, never the head's
   (Task 7, `approval_keys_the_cached_xtask_on_the_base_checkout`).
5. A label other than `windows` or `macos` on a pull request: no `platforms` job starts (Task 8,
   `platforms_starts_no_job_for_another_label`).

## Rulings made while planning

Recorded in the ledger as `Ruling:` lines and listed in the PR.

- **P1** "Not needed" is a field of `Verdict` (`not_needed: Option<String>`), not a note — the
  summary and the table must tell it apart from a pass with notes — cost if wrong: two struct
  literals to touch.
- **P2** `pr` runs both checks in one xtask call, `xtask pr` — one `run` step, no step-level
  `if:` — cost if wrong: none.
- **P3** The cached xtask is restored by a new composite action `.github/actions/xtask` and saved
  by the setup action when its input `save-xtask` is `true` — both compute the key with the same
  expression, which a workflow test compares — cost if wrong: one more action file.
- **P4** `tools` runs first in every job that installs the toolchain (`gate`, `test`,
  `coverage`, `platforms`); `pr`, `approval` and `settings` never run `tools` and, on a cached
  xtask, install nothing — they use none of the pinned tools — cost if wrong: none.
- **P5** Glob building moves from `checks/approval/protected.rs` into `xtask/src/globs.rs`,
  shared with `code_paths` — one place builds globs — cost if wrong: none.
- **P6** Timeouts until measured: `gate` 150 minutes (`slow-tests` brings 120), `test` and
  `coverage` 60, `pr`, `approval` and `settings` 15 — cost if wrong: adjusted in Task 10.

## File structure

| File | Responsibility | Task |
|---|---|---|
| `xtask/src/verdict.rs` | `Verdict::not_needed`, `Check::from_name` | 1 |
| `xtask/src/report.rs` | "not needed" in the table and the summary; `publish` for many verdicts | 1 |
| `xtask/src/globs.rs` (new) | `glob_set`, moved from `protected.rs` | 2 |
| `gate/code-paths.toml` (new) | the non-code and always-code lists | 2 |
| `xtask/src/code_paths.rs` (new) | load and match `gate/code-paths.toml` | 2 |
| `xtask/src/git.rs` | `diff_name_only_worktree`, `untracked` | 3 |
| `xtask/src/changes.rs` (new) | the changed paths and branch of a pull request or the working tree | 3 |
| `xtask/src/checks/slow.rs` | `needed_by` on `changes`; `Verdict::not_needed` | 3 |
| `xtask/src/scope.rs` (new) | every check or docs only | 4 |
| `xtask/src/gate.rs` | `Selection`, the scope applied, `run_pr` | 5 |
| `xtask/src/main.rs` | `gate --only/--skip`, `pr`, `publish` in CI | 5 |
| `.github/workflows/ci.yml`, `.github/actions/setup/action.yml` | three jobs; the cached xtask saved | 6 |
| `.github/actions/xtask/action.yml` (new), `pr.yml`, `approval.yml`, `settings.yml` | the cached xtask restored | 7 |
| `.github/workflows/platforms.yml` | one job per OS | 8 |
| `xtask/tests/workflows.rs` | the workflow shape | 6, 7, 8 |
| `gate/ruleset-main.json`, `xtask/src/github/settings.rs`, `xtask/tests/settings.rs` | the five required checks | 9 |
| `docs/gate.md`, `CONTRIBUTING.md`, `.claude/skills/theos-phase/SKILL.md` | the new behaviour and practice | 9 |
| `xtask/src/checks/approval/protected.rs` | `globs::glob_set` (Task 2), `changes::paths_between` (Task 3) | 2, 3 |

---

### Task 1: "Not needed" verdicts and a report for many checks

**Files:**
- Modify: `xtask/src/verdict.rs`, `xtask/src/report.rs`, `xtask/src/checks/coverage/mod.rs:128`,
  `xtask/src/checks/engine_free.rs:169`

**Interfaces:**
- Produces:
  - `Verdict.not_needed: Option<String>` (the reason); `Verdict::not_needed(check: Check, reason:
    impl Into<String>) -> Verdict` (no findings, so `passed()` stays true);
    `Verdict::is_not_needed(&self) -> bool`.
  - `Check::from_name(name: &str) -> Option<Check>`, the inverse of `Check::name`.
  - `report::publish(verdicts: &[Verdict], out: &mut dyn Write) -> Result<()>`: the annotations of
    every finding to `out`, and `summary_markdown(verdicts)` appended to `$GITHUB_STEP_SUMMARY`
    when it is set. `report::emit` uses it for its single verdict.

- [ ] **Step 1: Write the failing tests** in `report.rs` and `verdict.rs`:
  - `// AC: CE-08C.1` `summary_lists_every_check_with_its_verdict`: verdicts pass (`fmt`),
    failed (`clippy`, one finding), not needed (`test`, reason "docs only") →
    `summary_markdown` has the rows `` | `fmt` | pass | ``, `` | `clippy` | **FAIL** | `` and
    `` | `test` | not needed | ``, and a `#### `test`` section with the reason;
  - `a_not_needed_verdict_passes_and_says_so_in_the_table`: `write_table` prints `not needed`
    for it and `passed()` is true;
  - `check_names_round_trip`: `Check::from_name(c.name()) == Some(c)` for every `Check`, and
    `Check::from_name("nope")` is `None`;
  - `publish_appends_one_summary_for_all_verdicts` (a temp file as `$GITHUB_STEP_SUMMARY`,
    through a `publish_with(verdicts, Some(path), out)` helper like `emit_with`).
- [ ] **Step 2: Run** `cargo nextest run -p xtask --locked report verdict` — expected: FAIL (no
  `not_needed`, no `from_name`, no `publish`).
- [ ] **Step 3: Implement** the field (`None` in `pass`, `with` and the two struct literals), the
  constructor, `from_name` (a match over every variant), the table and summary status
  `not needed`, and `publish`/`publish_with`; `emit_with` calls `publish_with` for its one
  verdict.
- [ ] **Step 4: Run** the per-task checks — expected: PASS.
- [ ] **Step 5: Commit** `feat(xtask): report checks that a change does not need`.

### Task 2: The code-path lists

**Files:**
- Create: `xtask/src/globs.rs`, `gate/code-paths.toml`, `xtask/src/code_paths.rs`
- Modify: `xtask/src/checks/approval/protected.rs` (use `globs::glob_set`), `xtask/src/lib.rs`

**Interfaces:**
- Produces:
  - `globs::glob_set(list: &str, patterns: &[String], ignore_case: bool) -> Result<GlobSet>`,
    moved unchanged (`literal_separator(true)`, `backslash_escape(true)`).
  - `code_paths::FILE: &str = "gate/code-paths.toml"`; `CodePaths::load(root: &Path) ->
    Result<CodePaths>`; `CodePaths::parse(text: &str) -> Result<CodePaths>`;
    `CodePaths::is_non_code(&self, path: &str) -> bool` (matches `non_code`, exactly, and no
    `always_code`, ignoring case); `CodePaths::non_code(&self) -> &[String]` and
    `CodePaths::always_code(&self) -> &[String]` in file order.
- `gate/code-paths.toml`: keys `non_code` and `always_code`, values exactly spec §5.3, with a
  comment block saying what the file is, that it is protected, that a path not listed as non-code
  is code, and that always-code ignores case (as `gate/protected-paths.toml` explains its lists).

- [ ] **Step 1: Write the failing tests** in `code_paths.rs`:
  - `// AC: CE-08C.3` `every_input_a_check_reads_is_code` over `CodePaths::load(root)` of the
    real file: `docs/specs/00-umbrella.md`, `docs/gate.md`, `CONTRIBUTING.md`,
    `.github/workflows/ci.yml`, `.github/pull_request_template.md`, `research/app/src/x.ts`,
    `docs/x/Cargo.toml`, `research/Cargo.lock`, `docs/CLIPPY.TOML`, `research/rust-toolchain`,
    `design/.cargo/config.toml`, `docs/.gitignore`, `research/.gitattributes`,
    `crates/theos/src/main.rs`, `xtask/src/main.rs`, `gate/tools.toml`, `Cargo.toml`,
    `editor-ui/src/App.svelte` → `is_non_code` is false for each;
  - `docs_research_design_and_notes_are_non_code`: `docs/status.md`,
    `docs/specs/phases/P50-leaner-ci.md`, `docs/plans/P50-leaner-ci.md`, `research/notes.md`,
    `design/theos-ui/index.html`, `README.md`, `CLAUDE.md`, `.claude/skills/theos-phase/SKILL.md`,
    `docs/ui/screens/01-start.png` → true;
  - `non_code_matches_exactly`: `Docs/status.md` and `readme.md` → false;
  - `// AC: CE-08C.3` `a_ts_file_in_docs_is_code`: `docs/x.ts` → false (Review Focus 2);
  - `a_bad_glob_is_named_by_its_list` (parse error names `non_code` or `always_code`);
  - `missing_keys_are_an_error`.
- [ ] **Step 2: Run** `cargo nextest run -p xtask --locked code_paths protected` — expected:
  FAIL to compile.
- [ ] **Step 3: Implement** `globs.rs` (moved; `protected.rs` imports it, its tests stay green),
  the TOML file, and `code_paths.rs` with a `#[derive(Deserialize)] #[serde(deny_unknown_fields)]`
  struct for the file (as `protected.rs` does) and `thiserror` errors through the crate's
  `Error::Config`.
- [ ] **Step 4: Run** the per-task checks — expected: PASS.
- [ ] **Step 5: Commit** `feat(xtask): list the paths no check reads`.

### Task 3: Changed paths of a pull request or the working tree

**Files:**
- Create: `xtask/src/changes.rs`
- Modify: `xtask/src/git.rs`, `xtask/src/checks/slow.rs`, `xtask/src/lib.rs`

**Interfaces:**
- Consumes: `Git::merge_base`, `Git::diff_name_only_no_renames`, `Git::current_branch`,
  `ci::PullRequestEvent` (`base_sha`, `head_sha`, `head_ref`).
- Produces:
  - `Git::diff_name_only_worktree(&self, base: &str) -> Result<Vec<String>>` (`git diff
    --name-only --no-renames <base>`: committed, staged and unstaged changes since `base`);
    `Git::untracked(&self) -> Result<Vec<String>>` (`git ls-files --others --exclude-standard`).
  - `changes::Changes { pub branch: Option<String>, pub paths: Vec<String> }` (paths sorted,
    unique, `/`-separated);
    `changes::paths_between(git: &Git, merge_base: &str, head: &str) -> Result<Vec<String>>`, the
    one reader that `of_pull_request`, `slow-tests` and `approval`'s `protected::touched` use;
    `changes::of_pull_request(git: &Git, pr: &PullRequestEvent) -> Result<Changes>`;
    `changes::of_worktree(git: &Git) -> Result<Changes>` (merge base of `HEAD` and
    `origin/main`, plus untracked files; branch from `current_branch`).
  - `slow::needed_by` reads `changes::of_pull_request(…)?.paths`; its "not needed" result becomes
    `Verdict::not_needed(Check::SlowTests, NOT_NEEDED)`.

- [ ] **Step 1: Write the failing tests** in `changes.rs` over `git::testing` repositories:
  - `// AC: CE-08C.2` `a_pull_request_lists_the_paths_since_the_merge_base` (base moves on after
    the branch point; its own changes are not listed);
  - `// AC: CE-08C.2` `a_rename_out_of_code_counts_both_paths` (`git mv crates/a/src/x.rs
    docs/x.rs` → both paths; Review Focus 1);
  - `the_worktree_counts_uncommitted_and_untracked_files` (a committed `docs/a.md`, a modified
    tracked `crates/b/src/lib.rs`, an untracked `crates/c/src/new.rs`, an ignored file under
    `target/` → the first three, not the ignored one);
  - `the_worktree_needs_origin_main` (no `refs/remotes/origin/main` → `Err`);
  - `paths_use_forward_slashes`.
- [ ] **Step 2: Run** `cargo nextest run -p xtask --locked changes` — expected: FAIL.
- [ ] **Step 3: Implement** the two `Git` methods (same style and errors as
  `diff_name_only_no_renames`) and `changes.rs`; move `slow::needed_by` and
  `protected::touched` onto `paths_between`; `xtask/tests/slow.rs` and `xtask/tests/approval.rs`
  stay green (adjust only the slow-tests "not needed" assertion to `is_not_needed()`).
- [ ] **Step 4: Run** the per-task checks — expected: PASS.
- [ ] **Step 5: Commit** `feat(xtask): read the changed paths in one place`.

### Task 4: The scope decision

**Files:**
- Create: `xtask/src/scope.rs`
- Modify: `xtask/src/lib.rs`

**Interfaces:**
- Consumes: `changes::{Changes, of_pull_request, of_worktree}`, `code_paths::CodePaths`,
  `branch::{parse_branch, BranchKind}`, `ci::CiEvent`.
- Produces:
  - `scope::Scope { Every { note: Option<String> }, DocsOnly }`;
  - `scope::decide(changes: Result<Changes>, paths: &CodePaths) -> Scope` (pure; the rules of spec
    §5.3 in order: `phase/` branch → Every; `Err` → Every with a note naming the error; no path →
    Every; every path non-code → DocsOnly; else Every);
  - `scope::for_run(root: &Path, ci: Option<&CiEvent>) -> Scope`: push, `workflow_dispatch`
    (any non-PR event) → Every; PR event → `decide(of_pull_request(…), …)`; no event →
    `decide(of_worktree(…), …)`, but a detached HEAD (`branch: None`) → Every with a note;
    `CodePaths::load` failing → Every with a note;
  - `scope::NOT_NEEDED_REASON: &str` = "the change touches only paths of the non-code list of
    gate/code-paths.toml, which no check reads (docs/gate.md §1)".

- [ ] **Step 1: Write the failing tests** (pure ones on `decide`, two on `for_run` over
  `git::testing` repositories):
  - `// AC: CE-08C.2` `docs_only_changes_on_a_docs_branch_need_only_commit_msg` → `DocsOnly`;
  - `// AC: CE-08C.2` `a_phase_branch_runs_every_check` (only `docs/x.md`, branch
    `phase/P50-leaner-ci`; Review Focus 2);
  - `// AC: CE-08C.2` `a_code_path_runs_every_check` (`docs/a.md` + `xtask/src/x.rs`);
  - `// AC: CE-08C.2` `an_unreadable_change_list_runs_every_check` (`Err` → `Every` whose note
    contains the error text; Review Focus 3);
  - `// AC: CE-08C.2` `no_change_runs_every_check`;
  - `// AC: CE-08C.2` `a_push_runs_every_check` (`for_run` with a `CiEvent::Push`);
  - `// AC: CE-08C.2` `uncommitted_code_on_a_docs_branch_runs_every_check` (`for_run` with no
    event, branch `docs/x`, committed `docs/a.md`, untracked `crates/a/src/b.rs`);
  - `a_pull_request_event_decides_from_its_commits` (`for_run` with a PR event over a fixture).
- [ ] **Step 2: Run** `cargo nextest run -p xtask --locked scope` — expected: FAIL.
- [ ] **Step 3: Implement** `scope.rs`.
- [ ] **Step 4: Run** the per-task checks — expected: PASS.
- [ ] **Step 5: Commit** `feat(xtask): decide which checks a change needs`.

### Task 5: The gate runner selects checks, applies the scope, and runs `pr`

**Files:**
- Modify: `xtask/src/gate.rs`, `xtask/src/main.rs`, `xtask/tests/cli.rs` (if it calls `gate`)

**Interfaces:**
- Consumes: `scope::{for_run, Scope, NOT_NEEDED_REASON}`, `Verdict::not_needed`,
  `Check::from_name`, `report::publish`, `pr_title::run`, `pr_body::run`.
- Produces:
  - `gate::Selection { pub only: Vec<Check>, pub skip: Vec<Check> }` (`Default` = every check);
    `Selection::checks(&self) -> Vec<Check>`: `Tools` first, then `GATE_ORDER` without `Tools`,
    kept when `only` is empty or names it, dropped when `skip` names it;
  - `gate::run(root: &Path, ci: Option<&CiEvent>, selection: &Selection, out: &mut dyn Write) ->
    Result<Vec<Verdict>>`: decides `scope::for_run` once; under `DocsOnly` every selected check
    but `CommitMsg` gets `Verdict::not_needed(check, NOT_NEEDED_REASON)` without running (`Tools`
    included); under `Every` the runner runs as today, and a note of `Every` is printed before
    the table;
  - `gate::PR_CHECKS: [Check; 2] = [Check::PrTitle, Check::PrBody]`;
    `gate::run_pr(root, ci, out) -> Result<Vec<Verdict>>` runs both through `run_checks` (no
    `tools`, no scope);
  - CLI: `gate --only <a,b>` and `gate --skip <a,b>` (`value_delimiter = ','`, mutually
    exclusive, names through `Check::from_name`; `tools`, `pr-title`, `pr-body`, `approval` and
    `github` are rejected with a message); new subcommand `pr` ("Run pr-title and pr-body and
    print one table"); under GitHub Actions both call `report::publish` with all verdicts.

- [ ] **Step 1: Write the failing tests** in `gate.rs` (with the injected runner the existing
  tests use) and `main.rs`/`cli.rs`:
  - `// AC: CE-08C.1` `only_and_skip_choose_the_checks_after_tools`: `only [Test]` → `[Tools,
    Test]`; `skip [Test, Coverage]` → every other check of `GATE_ORDER` with `Tools` first;
  - `// AC: CE-08C.1` `the_three_ci_jobs_cover_the_gate_once`: the selections of the `ci` jobs
    (`skip test,coverage`, `only test`, `only coverage`) together hold every check of
    `GATE_ORDER` but `Tools` exactly once;
  - `// AC: CE-08C.2` `docs_only_runs_commit_msg_and_marks_the_rest_not_needed`: under `DocsOnly`
    the runner is called for `CommitMsg` only, every other verdict `is_not_needed()`;
  - `// AC: CE-08C.2` `docs_only_in_a_test_job_runs_nothing`: `only [Test]` under `DocsOnly` →
    `Tools` and `Test` not needed, runner never called;
  - `// AC: CE-08C.1` `pr_runs_title_and_body_in_one_table`;
  - `unknown_or_pr_only_names_are_rejected` (`--only approval`, `--skip nope`).
  - Keep the existing CE-08.4 tests of `gate.rs` green under `Selection::default()`.
- [ ] **Step 2: Run** `cargo nextest run -p xtask --locked gate cli` — expected: FAIL.
- [ ] **Step 3: Implement** `Selection`, the scope in `run`, `run_pr`, and the CLI.
- [ ] **Step 4: Run** the per-task checks, then `cargo xtask gate --only fmt` and
  `cargo xtask gate --skip test,coverage,slow-tests` in the worktree (a phase branch: expect
  every selected check to run) — read the table.
- [ ] **Step 5: Commit** `feat(xtask): select the checks of a CI job and skip what a change does not need`.

### Task 6: `ci.yml` with three jobs, and the cached xtask saved on `main`

**Files:**
- Modify: `.github/workflows/ci.yml`, `.github/actions/setup/action.yml`, `xtask/tests/workflows.rs`

**Interfaces:**
- Consumes: `gate --only/--skip` (Task 5).
- Produces:
  - `ci.yml` jobs, each `runs-on: ubuntu-latest`, checkout with `fetch-depth: 0` and
    `persist-credentials: false`, the setup action, one run step, no `needs:`, no `if:`:
    - `gate`: setup `cache-key: gate`, `save-cache: ${{ github.ref == 'refs/heads/main' }}`,
      `save-xtask: ${{ github.ref == 'refs/heads/main' }}`; run `cargo xtask gate --skip
      test,coverage`; `timeout-minutes: 150`;
    - `test`: setup `cache-key: test`, saves on `main` only; run `cargo xtask gate --only test`;
      60 minutes;
    - `coverage`: setup `cache-key: coverage`, saves on `main` only; the baseline
      `actions/cache` step as today; run `cargo xtask gate --only coverage`; 60 minutes.
    - triggers and concurrency unchanged.
  - Setup action input `save-xtask` (default `'false'`) and a last step
    `actions/cache/save@<sha>` with `if: inputs.save-xtask == 'true'`, `path:
    target/xtask-bin/debug/xtask`, `key: xtask-bin-${{ runner.os }}-${{ hashFiles('xtask/**',
    'Cargo.lock', 'rust-toolchain.toml', '.cargo/**') }}` (the binary exists once the "Read the
    pinned tools" step has built xtask).
  - `workflows.rs` helpers: a table of the expected jobs per workflow (name, runner, command)
    replaces `check_of`/`command_of`; `step_kind` knows `actions/cache/save` and
    `actions/cache/restore` and the local actions.

- [ ] **Step 1: Rewrite the tests** of `workflows.rs` that encode one job per check, each
  keeping its property:
  - `// AC: CE-08C.1` `ci_runs_three_jobs_side_by_side` (jobs exactly `gate`, `test`,
    `coverage`; their commands; no `needs:`) — replaces `ci_builds_and_tests_on_linux` and
    `every_job_only_runs_its_own_check` for `ci.yml`;
  - `// AC: CE-08.4` `ci_jobs_run_the_gate_runner` (each `ci` command is `cargo xtask gate` with
    `--only`/`--skip`, and their selections cover `GATE_ORDER` once — through
    `xtask::gate::Selection`) — replaces `gate_list_equals_ci_jobs_minus_pr_only`;
  - `// AC: CE-08C.5` `no_pull_request_job_saves_a_build_cache` (every `save-cache` and
    `save-xtask` input of a PR workflow is `false` or the `refs/heads/main` expression);
  - `// AC: CE-08C.5` `only_gate_on_main_saves_the_cached_xtask`;
  - keep and update: `required_jobs_always_report`, `no_key_lets_a_job_pass_while_xtask_fails`,
    `every_job_has_a_timeout` (P6 values), `history_is_fetched_where_commits_or_the_merge_base_are_read`
    (all three `ci` jobs), `coverage_restores_and_saves_the_baseline_cache` (in `coverage`),
    `triggers_and_concurrency_follow_the_spec`, `setup_action_installs_the_toolchain_the_cache_and_the_pinned_tools`
    (plus the save step), `actions_pinned_by_sha`, `no_expressions_in_run_steps`.
- [ ] **Step 2: Run** `cargo nextest run -p xtask --locked --test workflows` — expected: FAIL.
- [ ] **Step 3: Change** `ci.yml` and the setup action; update the header comments.
- [ ] **Step 4: Run** the per-task checks — expected: PASS.
- [ ] **Step 5: Commit** `ci(gate): run the checks in three jobs side by side`.

### Task 7: `pr`, `approval` and `settings` on the cached xtask

**Files:**
- Create: `.github/actions/xtask/action.yml`
- Modify: `.github/workflows/pr.yml`, `.github/workflows/approval.yml`,
  `.github/workflows/settings.yml`, `xtask/tests/workflows.rs`

**Interfaces:**
- Consumes: the key expression of Task 6; `xtask pr` (Task 5).
- Produces:
  - `.github/actions/xtask` (composite, no inputs): step `actions/cache/restore@<sha>` (`id:
    restore`, the same `path` and key expression, no `restore-keys`); then, with `if:
    steps.restore.outputs.cache-hit != 'true'`, the setup action with `cache-key: gate` and
    `save-cache: 'false'` (installs the toolchain, restores `main`'s `gate` cache and builds
    xtask as today). Afterwards `target/xtask-bin/debug/xtask` exists either way.
  - `pr.yml`: one job `pr` (checkout, the xtask action, run `target/xtask-bin/debug/xtask --root
    . pr`), timeout 15, triggers and concurrency unchanged.
  - `approval.yml`: both checkouts as today, then the xtask action (its key hashes the base
    checkout), run `target/xtask-bin/debug/xtask --root . approval` with `GH_TOKEN`; timeout 15;
    triggers, permissions and concurrency unchanged.
  - `settings.yml`: the xtask action, run `target/xtask-bin/debug/xtask --root . github check`.

- [ ] **Step 1: Write and update the tests**:
  - `// AC: CE-08C.5` `pr_and_approval_take_xtask_from_main_s_cache` (both use the xtask action,
    which restores by the exact key, has no `restore-keys`, and saves nothing);
  - `// AC: CE-08C.5` `the_cached_xtask_has_one_key` (the key expression in the setup action's
    save step and in the xtask action's restore step are the same string);
  - `// AC: CE-08.5` `approval_keys_the_cached_xtask_on_the_base_checkout` (the xtask action
    comes after both checkouts and after nothing that checks out the head unconditionally;
    Review Focus 4);
  - `// AC: CE-08C.1` `pr_runs_both_checks_in_one_job` — replaces the per-check `pr.yml` jobs;
  - update `required_contexts_are_pr_jobs` (now `gate`, `test`, `coverage`, `pr`, `approval`),
    `token_reaches_only_the_steps_that_call_github`,
    `only_the_approval_checkouts_keep_the_git_credentials`,
    `no_key_lets_a_step_of_the_setup_action_fail_silently` (also the xtask action), and replace
    `approval_restores_the_xtask_cache_that_main_saves` by the tests above.
  - Step-level `if:` stays forbidden in workflows; inside the two local actions only the
    conditions named above are allowed (`no_step_condition_except_the_cache_ones`).
- [ ] **Step 2: Run** `cargo nextest run -p xtask --locked --test workflows` — expected: FAIL.
- [ ] **Step 3: Create and change** the action and the three workflows.
- [ ] **Step 4: Run** the per-task checks — expected: PASS.
- [ ] **Step 5: Commit** `ci(gate): run pr, approval and settings on the xtask main cached`.

### Task 8: `platforms` with one job per OS

**Files:**
- Modify: `.github/workflows/platforms.yml`, `xtask/tests/workflows.rs`

**Interfaces:**
- Produces jobs (each: checkout, setup with `cache-key` = the job name and `save-cache: ${{
  github.event_name == 'schedule' }}`, one run step writing the summary through `gate`):
  - `windows` (`windows-latest`) and `macos` (`macos-latest`): `cargo xtask gate --only
    clippy,test,slow-tests,lints,ac-trace`; timeout 150;
  - `macos-intel` (`macos-15-intel`): `cargo xtask gate --only test,slow-tests`; timeout 150;
  - triggers: `schedule` (Monday 04:00 UTC, unchanged), `workflow_dispatch` with boolean inputs
    `windows` (default `true`), `macos` (default `true`), `macos_intel` (default `false`),
    `pull_request: types: [labeled]`;
  - job conditions: `windows` runs on `schedule`, on dispatch with `inputs.windows`, and on a
    label `windows`; `macos` likewise with `macos`; `macos-intel` only on dispatch with
    `inputs.macos_intel`;
  - job-level `concurrency`: group `platforms-${{ github.event.pull_request.number ||
    github.run_id }}-<job>`, `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`.

- [ ] **Step 1: Write the tests** (replacing `platforms_builds_and_tests_on_windows_and_macos`
  and `platforms_runs_weekly_on_demand_and_by_label`):
  - `// AC: CE-08C.6` `platforms_runs_one_job_per_os` (jobs, runners, commands);
  - `// AC: CE-08C.6, CE-08.12` `macos_runs_weekly_on_demand_and_by_label_on_apple_silicon`;
  - `// AC: CE-08C.6` `intel_runs_only_on_demand`;
  - `// AC: CE-08C.6` `a_newer_label_run_cancels_the_older_one`;
  - `// AC: CE-08C.6` `platforms_starts_no_job_for_another_label` (every job's condition is false
    for `event_name == 'pull_request'` with a label other than its own; evaluate the condition
    strings with a tiny table of cases, not by parsing expressions in general; Review Focus 5);
  - `// AC: CE-08.1` keep the Windows build-and-test property.
- [ ] **Step 2: Run** `cargo nextest run -p xtask --locked --test workflows` — expected: FAIL.
- [ ] **Step 3: Rewrite** `platforms.yml` with its header comment.
- [ ] **Step 4: Run** the per-task checks — expected: PASS.
- [ ] **Step 5: Commit** `ci(gate): run each platform's checks in one job`.

### Task 9: The ruleset, `docs/gate.md` and the working practice

**Files:**
- Modify: `gate/ruleset-main.json`, `xtask/src/github/settings.rs` (`required_contexts_from_gate_md`),
  `xtask/tests/settings.rs`, `docs/gate.md`, `CONTRIBUTING.md`,
  `.claude/skills/theos-phase/SKILL.md`, `docs/status.md`
- Test: `xtask/tests/settings.rs`, a new test in `xtask/src/code_paths.rs` or
  `xtask/tests/workflows.rs` for CE-08C.4

**Interfaces:**
- Consumes: `CodePaths::{non_code, always_code}`, `xtask::markdown` (sections, tables, list items).
- Produces:
  - ruleset contexts exactly `gate`, `test`, `coverage`, `pr`, `approval` (integration 15368);
  - `required_contexts_from_gate_md(text) -> BTreeSet<String>` reads the column `Job` of §1's
    first table (distinct values, `ui` left out) instead of its first column;
  - `docs/gate.md` §1: the table `| Check | Job | What it runs | OS |` with every check (`ui`'s
    job is `ui`); a subsection "Pull requests that change no code" with the two lists as Markdown
    list items of backticked globs in file order, the rule of spec §5.3, and the table of spec
    §5.4 with `file:line` evidence on the current `main` (take it from the code, not from the
    spec); the paragraph on Windows and macOS with today's prices (Windows about 1.7×, macOS
    about 10× Linux) and the new `platforms` jobs and triggers; the local gate paragraph: the
    docs-only rule applies there too; the slow-tests paragraph: it runs in `gate`.
  - `CONTRIBUTING.md` and the skill: the two practice rules of spec §5.8.

- [ ] **Step 1: Write the failing tests**:
  - `// AC: CE-08C.4` `gate_md_lists_the_code_paths_of_gate_toml` (the list items of the
    subsection equal `CodePaths::load(root)` lists, in order);
  - `// AC: CE-08C.4` `gate_md_explains_every_check_a_docs_change_skips` (the subsection's table
    has a row for every check of `GATE_ORDER` but `CommitMsg`);
  - `// AC: CE-08.3` update `ruleset_contexts_equal_gate_md_checks`,
    `required_contexts_are_the_first_column_of_section_1_without_ui` (rename to
    `required_contexts_are_the_job_column_of_section_1_without_ui`), and the workflow test
    `required_contexts_are_pr_jobs` (contexts = jobs of `ci.yml`, `pr.yml`, `approval.yml`).
- [ ] **Step 2: Run** `cargo nextest run -p xtask --locked --test settings --test workflows code_paths`
  — expected: FAIL.
- [ ] **Step 3: Change** the ruleset, the parser and the four documents.
- [ ] **Step 4: Run** the per-task checks, then `cargo xtask lints` and `cargo xtask ac-trace
  --phase P50` — expected: every P50 AC has a test.
- [ ] **Step 5: Commit** twice: `ci(gate): require the five checks of the leaner CI` (ruleset,
  parser, settings tests) and `docs(gate): describe the leaner CI and the docs-only rule` (the
  documents and the CE-08C.4 tests).

### Task 10: Verify, open the PR, and prove it on GitHub

Not a subagent task: the controller runs it after the final review loop (CLAUDE.md) is clean.

- [ ] **Step 1:** `cargo xtask gate` on Windows in the worktree — read the table; every check
  passes (a phase branch runs every check).
- [ ] **Step 2:** `cargo xtask ac-trace --phase P50` — every CE-08C AC has a passing test.
- [ ] **Step 3:** push, open the PR `ci(gate): leaner CI (P50)` with the template filled (ACs,
  review rounds, every ruling P1–P6 and the ledger's, gate output), labels `windows` and `macos`
  are not needed (no OS-specific code). It runs on the old required checks until Jann switches.
- [ ] **Step 4:** first runs warm the caches: note the cold times and set the timeouts of P6
  from them if they differ by more than half (commit `ci(gate): …`).
- [ ] **Step 5:** probe PRs against `phase/P50-leaner-ci` (spec §5.9): (1) only
  `docs/status.md`; (2) the umbrella; (3) `research/x.ts`; (4) `docs/x/Cargo.toml`; (5) only
  `docs/` from a `phase/P50-probe` branch. Read each run's summaries: (1) every job green with
  only `commit-msg` run; (2)–(5) every check run. Close them unmerged; link them in the PR.
- [ ] **Step 6:** speed (spec §5.10): from `gh run view --json jobs` of the P50 PR's last run
  and probe (1) on warm caches, the time of `gate`, `test`, `coverage`, `pr`, `approval` against
  the budgets. A missed budget: stop, write the numbers into the PR and ask Jann.
- [ ] **Step 7:** one manual `platforms` run with only `macos_intel` on the P50 branch; link it.
- [ ] **Step 8:** set P50 to `pr-open` in `docs/status.md`, list the switch-over under "Needs
  Jann", push.
