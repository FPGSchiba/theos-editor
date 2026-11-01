# P00 · Foundation and gate — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (`CLAUDE.md` makes it the execution method) to implement this plan task by task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** The G4 workspace skeleton plus the gate of `docs/gate.md`. Every check is a
`cargo xtask` command, GitHub Actions runs those commands, and the GitHub settings are kept as
files.

**Architecture:** `xtask` is a lib + bin. Each check is a module with a pure core (inputs →
`Verdict`) and a thin runner that gathers inputs from git, cargo, the CI event payload or
GitHub. Workflows only check out, run the shared setup action, and call `cargo xtask <check>`.
`cargo xtask gate` calls the same functions locally.

**Tech stack:** Rust 1.99.1 (edition 2024) · clap 4 · serde, toml 1, serde_json · thiserror 2,
anyhow · tracing, tracing-subscriber · cargo_metadata 0.23 · globset 0.4 · regex 1 · sha2 0.11 ·
roxmltree 0.21. Dev: tempfile 3, yaml-rust2 0.13. Tools: cargo-nextest 0.9.150, cargo-deny 0.20.2,
cargo-llvm-cov 0.9.1. Actions: checkout v7.0.1, cache v6.1.0, Swatinem/rust-cache v2.9.3,
taiki-e/install-action v2.88.1.

**Spec:** `docs/specs/phases/P00-foundation-and-gate.md` (approved, PR #5). Read it with this
plan. Where this plan is silent, the spec decides; where they differ, the spec wins.

## Global Constraints

Umbrella global constraints, verbatim. CLAUDE.md asks for G1–G16; G17 and G18 were added to the
umbrella later and are copied too.

- **G1** Repositories: `theos-editor` (this repo) holds the editor. `tower-of-theos` holds the
  content project and the map toolset. `Theos-Engine` is never a dependency, except behind the
  cargo feature `engine`, which no phase here builds.
- **G2** Platforms: Windows 11 x64 (primary), Linux x64, and macOS 13 or later on Apple silicon
  and Intel (universal builds). Every PR is checked on Windows and Linux; macOS is tested weekly
  and before every release (`docs/gate.md` §1).
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

- CE-08.1 The workspace with all crates of G4 (empty, documented) builds and tests in CI on
  Linux and Windows.
- CE-08.2 CI fails when a crate depends on an engine crate without feature `engine`.
- CE-08.3 Every check of `docs/gate.md` §1 (except `ui`) exists and is a required status check on
  `main`; branch protection and repo settings match `docs/gate.md` §6.
- CE-08.4 `cargo xtask gate` gives the same verdicts as CI on Windows and Linux.
- CE-08.5 `approval` fails for a PR that touches a protected path without the label `approved`
  and passes once it is added; CI adds `needs-jann` to such PRs.
- CE-08.6 `ac-trace` fails for a phase AC without a tagged test and for a tag naming an unknown AC.
- CE-08.7 `pr-body` fails for a description missing a template section.
- CE-08.8 `coverage` fails when a crate is below its floor or drops more than allowed below `main`.
- CE-08.9 `pr-title` fails for a title that is not a Conventional Commit ending with `(PNN)`,
  `(gate)`, `(docs)` or a release title.
- CE-08.10 `commit-msg` fails when any commit of the PR is not a Conventional Commit.
- CE-08.11 The workspace has one version in `[workspace.package]` that every crate inherits.
- CE-08.12 A scheduled workflow runs `clippy` and `test` on macOS weekly and on demand.

Rules every task follows (from `CLAUDE.md`, `CONTRIBUTING.md` and spec §5.2):

- **Lints in force from Task 1:** `unsafe_code` forbid, `missing_docs` deny, clippy `all` +
  `pedantic`, and denied `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`,
  `dbg_macro`, `print_stdout`, `print_stderr`, `allow_attributes`,
  `allow_attributes_without_reason`, `exit`, `mem_forget`.
  - Never `#[allow]`; use `#[expect(…, reason = "…")]` only with a real reason.
  - Output goes through `std::io::Write` on a passed-in writer, never `print!`.
  - Each integration test file starts with a `//!` line.
  - Test helpers outside `#[test]` functions return `Result` and use `?`.
- **No `unwrap`/`expect` outside tests.** Library errors use one `thiserror` enum,
  `xtask::Error` (Task 2); `anyhow` only in `xtask/src/main.rs`.
- **Processes:** `std::process::Command` with argument lists, never a shell. Paths go through
  `std::path`. Reported paths are repo-relative with `/`. Files we write are LF.
- **Scanner test inputs** that contain violations or tags live in `xtask/tests/fixtures/**`.
  Inside `.rs` sources, never put such text at the start of a line in a string literal: the
  `lints` and `ac-trace` scanners read every `.rs` file outside `tests/fixtures/`.
- **AC tags:** a test that proves an AC has `// AC: <id>` on the line directly above `#[test]`.
- **Commits:** Conventional Commits with these footers on every commit of this branch:

  ```text
  Refs: <the ACs the commit works on, if any>
  Phase: P00
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: 666a12cd-ea1d-44dd-bf09-077295de681d
  ```

  The summary is lower case, has no full stop and is at most 72 characters; body lines are at
  most 100 columns.
- **Before each commit:** `cargo fmt --all`, then `cargo clippy --workspace --all-targets
  --locked -- -D warnings`, then the task's tests. After Task 3: `cargo nextest run --workspace
  --locked`. All must be clean.
- **Never** change `docs/**` (except this plan and `docs/status.md`), `CLAUDE.md`,
  `CONTRIBUTING.md`, `.claude/**` or `design/**` on this branch (spec §10).

## Review Focus

The input classes below are the most likely to bite and are easy to miss; each has its pinning
test in the named task.

1. **CRLF text from Windows and the GitHub web editor** (commit messages, PR bodies, mirrored
   config files) is read as LF and judged the same. Tests in Tasks 4, 6 and 8.
2. **Protected changes in disguise:** a rename out of `docs/`, `DOCS/x.md`, a change only inside
   `[workspace.lints]`, and a whitespace-only or mode-only change after approval. Tests in Task 11.
3. **Text that looks like a tag or attribute but isn't one:** a tag above a helper fn, a tag
   separated by a blank line, and `#[allow` inside a doc comment. Tests in Tasks 7 and 8.
4. **A required check that never reports:** path filters, job-level `if:` on a required job, and a
   context name that differs from the ruleset. Tests in Task 14.
5. **The engine's `theos-editor` versus ours,** and optional engine deps reachable from `default`
   through another feature. Tests in Task 9.

---

## File map

```text
Cargo.toml, Cargo.lock, rust-toolchain.toml, .cargo/config.toml, .gitattributes   Task 1
clippy.toml, .config/nextest.toml, deny.toml                                       Tasks 1, 3
crates/{theos-content,theos-vcs,theos-studio}/  Cargo.toml README.md src/lib.rs    Task 1
crates/{theos,theos-mcp,theos-editor}/          Cargo.toml README.md src/main.rs
                                                tests/smoke.rs                     Task 1
gate/lints.toml clippy.toml                                                        Task 1
gate/nextest.toml tools.toml                                                       Task 3
gate/engine-crates.toml                                                            Task 9
gate/thresholds.toml                                                               Task 10
gate/protected-paths.toml                                                          Task 11
gate/ruleset-main.json ruleset-tags.json repo-settings.toml                        Task 12
xtask/Cargo.toml, xtask/README.md                                                  Task 1
xtask/src/main.rs            CLI: clap, dispatch, exit codes                       Tasks 2, 13
xtask/src/lib.rs             module tree, crate docs                               Task 2
xtask/src/error.rs           xtask::Error                                          Task 2
xtask/src/verdict.rs         Check, Finding, Verdict                               Task 2
xtask/src/report.rs          text table, annotations, step summary                 Task 2
xtask/src/ci.rs              CI event payload                                      Task 2
xtask/src/branch.rs          branch kinds, Phase                                   Task 2
xtask/src/git.rs             git CLI wrapper                                       Task 2
xtask/src/files.rs           repo file listing                                     Task 2
xtask/src/process.rs         running cargo children                                Task 2
xtask/src/checks/cargo_cmds.rs  fmt, clippy, test, docs, deny                      Task 3
xtask/src/tools.rs           tool pins                                             Task 3
xtask/src/conventional.rs    Conventional Commit parser                            Task 4
xtask/src/checks/commit_msg.rs                                                     Task 4
xtask/src/checks/pr_title.rs                                                       Task 5
xtask/src/checks/pr_body.rs                                                        Task 6
xtask/src/umbrella.rs        ACs and phase table                                   Task 7
xtask/src/checks/ac_trace/{mod,rust_tags,ts_tags,junit}.rs                         Task 7
xtask/src/checks/lints.rs                                                          Task 8
xtask/src/checks/engine_free.rs                                                    Task 9
xtask/src/checks/coverage/{mod,llvm_json,baseline}.rs                              Task 10
xtask/src/checks/approval/{mod,protected,fingerprint,decide}.rs                    Task 11
xtask/src/github/{mod,gh_cli,fake}.rs                                              Task 11
xtask/src/github/settings.rs                                                       Task 12
xtask/src/gate.rs                                                                  Task 13
xtask/tests/*.rs, xtask/tests/fixtures/**                                          per task
.github/actions/setup/action.yml, .github/workflows/*.yml                          Task 14
README.md (root)                                                                   Task 13
```

---

### Task 1: Workspace skeleton, toolchain and lint configuration

Suggested implementer model: mid-tier. ACs: CE-08.1, CE-08.11.

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `.gitattributes`,
  `clippy.toml`, `gate/lints.toml`, `gate/clippy.toml`, six crates under `crates/` (Cargo.toml,
  README.md, `src/lib.rs` or `src/main.rs`), `crates/{theos,theos-mcp,theos-editor}/tests/smoke.rs`,
  `xtask/Cargo.toml`, `xtask/README.md`, `xtask/src/main.rs`, `xtask/src/lib.rs`,
  `xtask/tests/workspace.rs`, `Cargo.lock`.

**Interfaces:**
- Produces: the workspace that every later task builds in; `cargo xtask` alias; root
  `[workspace.lints]` equal to `gate/lints.toml`.

Fixed values:
- `rust-toolchain.toml`: `channel = "1.99.1"`, `components = ["rustfmt", "clippy",
  "llvm-tools-preview"]`, `profile = "minimal"`.
- Root `Cargo.toml`:
  - `[workspace]`: `resolver = "3"`, `members = ["crates/*", "xtask"]`.
  - `[workspace.package]`: `version = "0.0.0"`, `edition = "2024"`, `rust-version = "1.99.1"`,
    `publish = false`, `repository = "https://github.com/FPGSchiba/theos-editor"`.
  - `[workspace.lints]`: the spec §5.2 list, byte-for-byte the same tables as `gate/lints.toml`
    (which holds them as `[rust]`, `[rustdoc]`, `[clippy]`).
  - `[workspace.dependencies]`: every third-party crate any member uses, added as later tasks
    need them.
- Every member's `Cargo.toml`: `version.workspace = true`, `edition.workspace = true`,
  `rust-version.workspace = true`, `publish.workspace = true`,
  `repository.workspace = true`, `[lints] workspace = true`.
- `crates/theos-editor/Cargo.toml`: `[features] engine = []`, with a comment saying it is
  reserved for the engine part and must stay empty until then.
- `.cargo/config.toml`: `[alias] xtask = "run --locked --package xtask --target-dir
  target/xtask-bin --"`. The running `xtask.exe` lives in its own build directory, so that the
  cargo commands it starts (`test`, `clippy`) never relink it (Windows cannot replace a running
  executable).
- `.gitattributes`: `* text=auto eol=lf`.
- `gate/clippy.toml`, with the root `clippy.toml` byte-identical: the clippy test exemptions for
  unwrap, expect, panic, print and dbg (keys valid for clippy 1.99; drop a key clippy reports as
  unknown and say so in the commit body).
- Crate docs (`//!`) and READMEs state each crate's role, taken from `docs/architecture.md` §3,
  and say "empty until phase PNN". The phases: content P02–P04, vcs P06, studio P06/P07, theos
  P05, theos-mcp P05, theos-editor P11.
- Binaries: `fn main() {}`.
- xtask: `lib.rs` with crate docs only; `main.rs` calls nothing yet and exits 0.

- [ ] **Step 1:** Run `rustup toolchain install` in the worktree, which installs 1.99.1 from the
  file once that file exists, and write the manifests and crate sources above.
- [ ] **Step 2: Write the failing tests**

  | Test (file) | Assertion | Tag |
  |---|---|---|
  | `workspace_members_are_the_g4_crates` (`xtask/tests/workspace.rs`) | `cargo metadata --no-deps --format-version 1` members, by name and manifest dir relative to the root, equal `theos-content@crates/theos-content`, `theos-vcs@…`, `theos-studio@…`, `theos@…`, `theos-mcp@…`, `theos-editor@…`, `xtask@xtask` | CE-08.1 |
  | `every_member_has_readme_and_crate_docs` | each member dir has `README.md`; its `src/lib.rs` or `src/main.rs` starts with `//!` | CE-08.1 |
  | `every_member_inherits_the_workspace_version` | each member's version equals `[workspace.package].version` (`0.0.0`); each manifest's `package.version` is the table `{ workspace = true }` | CE-08.11 |
  | `theos_binary_runs`, `theos_mcp_binary_runs`, `theos_editor_binary_runs` (`crates/<bin>/tests/smoke.rs`) | `Command::new(env!("CARGO_BIN_EXE_<bin>")).status()` is success | CE-08.1 |

- [ ] **Step 3:** Run `cargo test --workspace --locked`. Expected: the three workspace tests
  PASS once the manifests are right; fix the manifests until they do. Then flip one member to a
  literal `version = "0.0.1"`, see `every_member_inherits_the_workspace_version` FAIL, and revert.
- [ ] **Step 4:** Run `cargo clippy --workspace --all-targets --locked -- -D warnings` and
  `cargo fmt --all --check`. Expected: clean.
- [ ] **Step 5:** Commit: `build: add the workspace skeleton with the g4 crates`
  (`Refs: CE-08.1, CE-08.11`).

---

### Task 2: xtask core — verdicts, reports, CI context, branches, git, CLI

Suggested implementer model: mid-tier. ACs: none (infrastructure; later tasks prove ACs through
it).

**Files:**
- Create: `xtask/src/{error,verdict,report,ci,branch,git,files,process}.rs`.
- Modify: `xtask/src/lib.rs`, `xtask/src/main.rs`, `xtask/Cargo.toml`.

**Interfaces (produces; exact names later tasks use):**
- `error.rs`: `pub enum Error` (thiserror) with variants `Io { path: PathBuf, source:
  std::io::Error }`, `Command { program: String, args: Vec<String>, status: Option<i32>,
  stderr: String }`, `Parse { what: String, message: String }`, `Config { file: String,
  message: String }`, `GitHub { message: String }`, `Missing { what: String }`.
  `pub type Result<T> = std::result::Result<T, Error>;`
- `verdict.rs`:
  - `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Check { Tools, Fmt, Clippy, Test,
    Docs, Deny, EngineFree, Coverage, Lints, AcTrace, CommitMsg, PrTitle, PrBody, Approval }`
    with `pub const fn name(self) -> &'static str`. The names are `tools`, `fmt`, `clippy`,
    `test`, `docs`, `deny`, `engine-free`, `coverage`, `lints`, `ac-trace`, `commit-msg`,
    `pr-title`, `pr-body` and `approval`.
  - `pub const GATE_ORDER: [Check; 11] = [Tools, Fmt, Lints, EngineFree, AcTrace, CommitMsg,
    Clippy, Docs, Deny, Test, Coverage];`
  - `pub const PR_ONLY: [Check; 3] = [PrTitle, PrBody, Approval];`
  - `pub struct Finding { pub file: Option<String>, pub line: Option<u32>, pub message: String }`
    with `Finding::new(msg)` and `Finding::at(file, line, msg)`.
  - `pub struct Verdict { pub check: Check, pub findings: Vec<Finding>, pub notes: Vec<String> }`
    with `Verdict::pass(check)`, `Verdict::with(check, findings)` and `fn passed(&self) -> bool`
    (true when there are no findings).
- `report.rs`:
  - `pub fn write_table(verdicts: &[Verdict], out: &mut dyn Write) -> io::Result<()>`: one line
    per check with `pass`/`FAIL`, then each finding as `  - path:line: message` and each note
    as `  note: …`. Every line after the first line of a finding or note is `    | …`. The text
    is split at every kind of line break, and no line starts with the text itself, so the runner
    cannot read a line of a pull request as a workflow command.
  - `pub fn annotation(check: Check, f: &Finding) -> String`: `::error
    file=<f>,line=<n>,title=<check>::<msg>` with `%`, CR and LF escaped as `%25`, `%0D` and
    `%0A`; without a file it is `::error title=<check>::<msg>`.
  - `pub fn summary_markdown(verdicts: &[Verdict]) -> String`.
  - `pub fn emit(verdict: &Verdict, ci: bool, out: &mut dyn Write) -> Result<()>`: in CI it
    also writes the annotations and appends the summary to `$GITHUB_STEP_SUMMARY`.
- `ci.rs`:
  - `pub enum CiEvent { PullRequest(PullRequestEvent), Push(PushEvent), Other }`.
  - `pub struct PullRequestEvent { pub action: String, pub number: u64, pub title: String, pub
    body: Option<String>, pub base_sha: String, pub head_sha: String, pub head_ref: String, pub
    label: Option<String> }`.
  - `pub struct PushEvent { pub before: String, pub after: String }`.
  - `pub fn from_env() -> Result<Option<CiEvent>>`: `None` unless `GITHUB_ACTIONS=true`; reads
    `GITHUB_EVENT_NAME` and parses `GITHUB_EVENT_PATH`.
  - `pub fn parse_event(name: &str, json: &str) -> Result<CiEvent>`.
- `branch.rs`:
  - `pub struct Phase(u8)` with `Display` giving `P07`, and `FromStr` that accepts exactly
    `P\d\d`.
  - `pub enum BranchKind { Phase(Phase), Spec(Phase), Gate, Docs, Fix, Release, Main,
    Other(String) }`.
  - `pub fn parse_branch(name: &str) -> BranchKind`, accepting `phase/PNN-<slug>`,
    `spec/PNN-<slug>`, `gate/<slug>`, `docs/<slug>`, `fix/<slug>` (slug `[a-z0-9][a-z0-9-]*`),
    `release-plz-*` and `main`.
  - `pub fn phase_of(kind: &BranchKind) -> Option<Phase>`, which is `Some` only for `Phase`.
- `git.rs`: `pub struct Git { root: PathBuf }`. Every call runs `git -C <root> -c
  core.quotepath=false …` and returns stdout or `Error::Command`. Methods: `rev_parse(&str)`,
  `merge_base(&str, &str)`, `rev_list(range: &str) -> Vec<String>`, `parents(sha) ->
  Vec<String>`, `message(sha) -> String`, `diff_name_only_no_renames(base, head) ->
  Vec<String>`, `diff_raw(base, head) -> String` (`--raw --no-renames --no-abbrev`),
  `show(rev, path) -> Option<String>` (None if absent), `current_branch() -> Option<String>`,
  `worktree_add_detached(dir, sha)`, `worktree_remove(dir)`.
- `files.rs`: `pub fn repo_files(root: &Path) -> Result<Vec<String>>`. In a git work tree it is
  `git ls-files --cached --others --exclude-standard`; otherwise a directory walk that skips
  `.git`, `target` and `node_modules`. Paths are relative with `/`.
- `process.rs`: `pub struct CargoCommand { pub args: Vec<String>, pub env: Vec<(String,
  String)> }` and `pub fn run_cargo(root: &Path, cmd: &CargoCommand) -> Result<bool>`, which
  inherits stdio and returns whether the exit was 0.
- `main.rs`:
  - clap derive `Cli { #[arg(long, global = true)] root: Option<PathBuf>, command: Command }`,
    with one subcommand per `Check` name plus `gate`, `github {check|apply}` and `tools`. The
    default root is the parent of xtask's `CARGO_MANIFEST_DIR`.
  - Exit codes: 0 pass, 1 fail, 2 error.
  - `tracing_subscriber` writes to stderr, filtered by `XTASK_LOG`.
  - Subcommands that aren't built yet return `Error::Missing`.

- [ ] **Step 1: Write the failing unit tests** (in each module's `#[cfg(test)]`):
  - `annotation_escapes_newlines_and_percent`: `"a%b\nc"` becomes `a%25b%0Ac`.
  - `annotation_without_file`.
  - `table_lists_failures_with_locations`.
  - `parses_pull_request_event` with fixture `xtask/tests/fixtures/events/pull_request.json` (a
    trimmed real payload containing title, body `null`, labels, base and head SHAs, head ref
    and action `labeled` with label `approved`).
  - `parses_push_event`.
  - `branch_kinds`: `phase/P02-records-and-grammars` → `Phase(P02)`; `spec/P10-prose-editor`
    → `Spec(P10)`; `gate/x` → `Gate`; `docs/x` → `Docs`; `fix/x` → `Fix`;
    `release-plz-2026-10-31` → `Release`; `main` → `Main`; `feature/x`, `phase/P2-x` and
    `phase/P02` → `Other`.
  - `phase_display_and_parse`.
  - `git_reads_messages_and_parents`: a temp repo made with `git init -b main` and commits
    under `-c user.name=t -c user.email=t@t`.
  - `repo_files_walks_non_git_dirs`.
  - `verdict_passed_iff_no_findings`.
- [ ] **Step 2:** Run `cargo test -p xtask`. Expected: FAIL (missing items).
- [ ] **Step 3:** Implement the interfaces above.
- [ ] **Step 4:** Run `cargo test -p xtask` and `cargo run --locked -p xtask -- --help`.
  Expected: PASS, and the help lists every subcommand.
- [ ] **Step 5:** Commit: `feat(xtask): add the verdict, report, ci, branch and git core`.

---

### Task 3: Cargo-command checks and tool pins

Suggested implementer model: cheapest. ACs: CE-08.4 (commands identical to `docs/gate.md` §1).

**Files:**
- Create: `xtask/src/checks/mod.rs`, `xtask/src/checks/cargo_cmds.rs`, `xtask/src/tools.rs`,
  `gate/tools.toml`, `gate/nextest.toml`, `.config/nextest.toml` (byte copy), `deny.toml`.

**Interfaces:**
- Consumes: `CargoCommand`, `run_cargo`, `Verdict`, `Check` (Task 2).
- Produces:
  - `cargo_cmds::{fmt, clippy, docs, deny}() -> Vec<CargoCommand>` and `cargo_cmds::test(ci:
    bool) -> Vec<CargoCommand>`;
  - `pub fn run_cargo_check(root: &Path, check: Check, cmds: &[CargoCommand]) -> Result<Verdict>`;
  - `tools::{ToolPin, load_pins(root) -> Result<Vec<ToolPin>>,
    parse_version(tool, output) -> Option<String>, install_spec(&[ToolPin]) -> String,
    check_installed(root) -> Result<Verdict>, install_missing(root) -> Result<()>}`.

Fixed values:
- Commands (exact argument lists):
  - `fmt`: `fmt --all --check`.
  - `clippy`: `clippy --workspace --all-targets --locked -- -D warnings`.
  - `test`: `nextest run --workspace --locked --profile <ci|default>`, then `test --doc
    --workspace --locked`.
  - `docs`: `doc --workspace --no-deps --locked` with env `RUSTDOCFLAGS=-D warnings`.
  - `deny`: `deny --locked check`. If cargo-deny 0.20.2 rejects the global `--locked`, use
    `deny check`, adjust the test, and say why in the commit body.
- `gate/tools.toml`: `[tools]` with `cargo-nextest = "0.9.150"`, `cargo-deny = "0.20.2"` and
  `cargo-llvm-cov = "0.9.1"`.
- Version output parsing: nextest `cargo-nextest 0.9.150 (…)`, deny `cargo-deny 0.20.2`,
  llvm-cov `cargo-llvm-cov 0.9.1`.
- `install_spec` gives `cargo-nextest@0.9.150,cargo-deny@0.20.2,cargo-llvm-cov@0.9.1`.
- `cargo xtask tools`:
  - plain: checks the installed versions; a mismatch is a finding that includes `cargo install
    --locked <tool> --version <v>`;
  - `--install`: runs those installs;
  - `--print-install-spec`: writes the spec line to stdout.
- `gate/nextest.toml`: profiles `default` and `ci`, both with `retries = 0`, `fail-fast =
  false` and `junit.path = "junit.xml"`.
- `deny.toml` (cargo-deny 0.20 keys):
  - graph: default features only;
  - advisories: yanked deny;
  - licences: allow exactly the spec §4.2 list, and ignore private (unpublished) crates;
  - bans: `multiple-versions = "warn"`, `wildcards = "deny"`, wildcard paths allowed;
  - sources: unknown registries and git deny, the crates.io index allowed, and allow-git
    `https://github.com/FPGSchiba/theos-engine`.

- [ ] **Step 1: Write the failing tests:**
  - `commands_match_gate_md` (tag CE-08.4): asserts each argument list and env above exactly.
  - `parses_tool_versions`, for all three output formats plus garbage → None.
  - `install_spec_lists_pins_in_order`.
  - `pins_load_from_gate_tools_toml`, reading the real file.
- [ ] **Step 2:** Run `cargo test -p xtask`. Expected: FAIL.
- [ ] **Step 3:** Implement, and wire `fmt`, `clippy`, `test`, `docs`, `deny` and `tools` into
  `main.rs`. `test` uses the `ci` profile when `ci::from_env()` is `Some`.
- [ ] **Step 4:** Run `cargo xtask tools --install` (installs the pinned tools locally), then
  `cargo xtask tools`, `cargo xtask fmt`, `cargo xtask clippy`, `cargo xtask test`,
  `cargo xtask docs` and `cargo xtask deny`. Expected: each exits 0. Fix licence or advisory
  findings by changing dependencies, never by loosening `deny.toml`.
- [ ] **Step 5:** Commit: `feat(xtask): run fmt, clippy, test, docs and deny with pinned tools`
  (`Refs: CE-08.4`).

---

### Task 4: Conventional Commit parser and `commit-msg`

Suggested implementer model: mid-tier. ACs: CE-08.10.

**Files:**
- Create: `xtask/src/conventional.rs`, `xtask/src/checks/commit_msg.rs`, `xtask/tests/commit_msg.rs`.

**Interfaces:**
- Consumes: `Git`, `BranchKind`, `Phase`, `Verdict` (Task 2). It takes the set of known AC ids
  as a `&BTreeSet<String>` parameter (Task 7 supplies it; tests pass literals).
- Produces:
  - `pub const TYPES: [&str; 10] = ["feat","fix","perf","refactor","test","docs","build","ci",
    "chore","revert"];`
  - `pub const SCOPES: [&str; 14] = ["content","vcs","studio","cli","mcp","editor","ui",
    "generators","xtask","gate","spec","docs","design","release"];`
  - `pub struct Header { pub kind: String, pub scope: Option<String>, pub breaking: bool, pub
    summary: String }`.
  - `pub fn parse_header(line: &str) -> std::result::Result<Header, String>`: the grammar
    `type(scope)!: summary`, where the scope and `!` are optional and exactly one space follows
    the colon.
  - `pub fn header_problems(h: &Header) -> Vec<String>`: the type and scope must be in the
    lists; the summary has at most 72 chars (counted as `char`s), does not start with an
    upper-case letter, and does not end with `.`.
  - `pub struct Message { pub header: Header, pub body: Vec<String>, pub footers: Vec<(String,
    String)> }`.
  - `pub fn parse_message(text: &str) -> std::result::Result<Message, String>`:
    - CRLF is normalised to LF first;
    - the blank line after the header is required when a body exists;
    - footers are the last paragraph whose first line matches `^(BREAKING CHANGE|[A-Za-z-]+):
      `; continuation lines join the previous footer.
  - `pub fn message_problems(m: &Message, branch: &BranchKind, known_acs: &BTreeSet<String>)
    -> Vec<String>`:
    - body and footer lines are at most 100 chars unless the line has no space;
    - `!` if and only if a `BREAKING CHANGE` or `BREAKING-CHANGE` footer exists;
    - `Phase:` equals the branch phase on `Phase` branches, is required there, and must be a
      valid `Phase` elsewhere;
    - `Refs:` holds comma-separated ids that are all in `known_acs`.
  - `checks::commit_msg::check(git: &Git, range: &CommitRange, branch: &BranchKind,
    known_acs: &BTreeSet<String>) -> Result<Verdict>`, where `pub enum CommitRange {
    Between { base: String, head: String } }`. It lists `git rev-list base..head`, skips
    commits with two or more parents, and gives one finding per problem, with the short SHA as
    the file and no line.
  - Range selection in `main.rs`:
    - pull request event: `base_sha..head_sha`;
    - push event: `before..after`, skipped when `before` is all zeros;
    - local: `merge_base(HEAD, origin/main)..HEAD`, with `--base`/`--head` overrides.
  - The branch comes from the event's `head_ref`, else `current_branch()`, else `--branch`.

- [ ] **Step 1: Write the failing tests.** Unit tests in `conventional.rs`:
  `parses_header_with_scope_and_bang`, `rejects_unknown_type`, `rejects_unknown_scope`,
  `summary_of_72_chars_passes_73_fails`, `rejects_uppercase_start_and_full_stop`,
  `crlf_message_parses_like_lf`, `bang_requires_breaking_footer_and_back`,
  `both_breaking_spellings_count`, `long_body_line_fails_but_url_line_passes`,
  `multi_line_footer_joins`.

  Integration tests in `xtask/tests/commit_msg.rs` build temp repos with `git init -b main`.
  Every commit uses `-c user.name=t -c user.email=t@t`, so the tests run on CI runners with no
  git identity. Each test below is tagged CE-08.10:

  | Test | Commits after the base | Expected |
  |---|---|---|
  | `good_range_passes` | two valid commits with `Phase: P00` on branch `phase/P00-x` | pass |
  | `bad_header_fails` | `Added stuff` | fail, finding names the commit |
  | `missing_phase_footer_fails_on_phase_branch` | a valid header without `Phase:` | fail |
  | `unknown_ref_fails` | `Refs: CE-99.1` with known `{CE-08.1}` | fail |
  | `merge_commit_is_skipped` | `Merge branch 'main' into phase/P00-x` merge | pass |
  | `fixup_commit_fails` | `fixup! feat(xtask): x` | fail |

- [ ] **Step 2:** Run `cargo nextest run -p xtask`. Expected: FAIL.
- [ ] **Step 3:** Implement and wire up `cargo xtask commit-msg`. Known ACs come from Task 7
  later; until then, wire an empty set and add a `// AC ids come from umbrella.rs (Task 7)` note
  that Task 7 replaces.
- [ ] **Step 4:** Run `cargo nextest run -p xtask`, then `cargo xtask commit-msg` on this branch.
  Expected: PASS, and this branch's own commits pass.
- [ ] **Step 5:** Commit: `feat(xtask): check commit messages against contributing rules`
  (`Refs: CE-08.10`).

---

### Task 5: `pr-title`

Suggested implementer model: cheapest. ACs: CE-08.9.

**Files:** Create `xtask/src/checks/pr_title.rs`.

**Interfaces:**
- Consumes: `parse_header`, `header_problems` (Task 4); `BranchKind` (Task 2).
- Produces: `pub fn check_title(title: &str, branch: &BranchKind) -> Verdict`. In CI the title
  and branch come from the event; locally from `--title` and `--branch`.

Rules (spec §5.4):
- A release title matches `^chore\(release\): v\d+\.\d+\.\d+(-rc\.\d+)?$` and is required on
  and only allowed on `Release`.
- Otherwise the header parses, `header_problems` is empty, and the summary ends with ` (PNN)`,
  ` (gate)` or ` (docs)`.
- Branch match:
  - `Phase(p)` → ` (p)`;
  - `Spec(p)` → type `docs`, scope `spec`, ` (p)`;
  - `Gate` → ` (gate)`; `Docs` → ` (docs)`;
  - `Fix` → any of the three suffixes;
  - `Main` or `Other` → fail with a finding naming `CONTRIBUTING.md §1`.

- [ ] **Step 1: Write the failing unit tests**, each tagged CE-08.9:

  | Test | Title / branch | Expected |
  |---|---|---|
  | `phase_title_passes` | `feat(content): records and grammars (P02)` / `phase/P02-records-and-grammars` | pass |
  | `spec_title_passes` | `docs(spec): foundation and gate (P00)` / `spec/P00-foundation-and-gate` | pass |
  | `spec_title_needs_docs_spec` | `feat(spec): x (P00)` / `spec/P00-x` | fail |
  | `gate_and_docs_titles_pass` | `ci(gate): x (gate)` / `gate/x`; `docs: x (docs)` / `docs/x` | pass |
  | `release_titles_pass` | `chore(release): v0.1.0`, `chore(release): v0.2.0-rc.1` / `release-plz-2026-11-01` | pass |
  | `not_conventional_fails` | `Records and grammars (P02)` / `phase/P02-x` | fail |
  | `missing_suffix_fails` | `feat(content): records` / `phase/P02-x` | fail |
  | `suffix_must_match_branch` | `feat(content): x (P03)` / `phase/P02-x` | fail |
  | `unknown_branch_fails` | `feat(content): x (P02)` / `feature/x` | fail |

- [ ] **Step 2:** Run the tests. Expected: FAIL.
- [ ] **Step 3:** Implement and wire up `cargo xtask pr-title`.
- [ ] **Step 4:** Run the tests. Expected: PASS.
- [ ] **Step 5:** Commit: `feat(xtask): check pull request titles` (`Refs: CE-08.9`).

---

### Task 6: `pr-body`

Suggested implementer model: cheapest. ACs: CE-08.7.

**Files:** Create `xtask/src/checks/pr_body.rs`.

**Interfaces:**
- Produces:
  - `pub fn template_sections(template: &str) -> Vec<String>`: the `## ` headings, trimmed.
  - `pub fn check_body(template: &str, body: Option<&str>) -> Verdict`: one finding per missing
    or empty section.
  - The runner reads `.github/pull_request_template.md` from `--root`, and the body from the
    event or `--body-file`.

Rules:
- Normalise CRLF to LF.
- A section is the text from its heading to the next `## ` line.
- A section is filled when it is non-empty after removing `<!-- … -->` (also multi-line) and
  whitespace.
- Order and extra sections don't matter; headings match after trimming trailing spaces.

- [ ] **Step 1: Write the failing unit tests**, each tagged CE-08.7, against the real template
  text loaded with `include_str!("../../../.github/pull_request_template.md")`:
  - `full_body_passes`;
  - `missing_section_fails` (drop `## Gate`; the finding names `Gate`);
  - `placeholder_only_section_fails` (keep the template comment only);
  - `crlf_body_passes`;
  - `reordered_sections_pass`;
  - `null_body_fails_every_section`.
- [ ] **Step 2:** Run the tests. Expected: FAIL.
- [ ] **Step 3:** Implement and wire up `cargo xtask pr-body`.
- [ ] **Step 4:** Run the tests. Expected: PASS.
- [ ] **Step 5:** Commit: `feat(xtask): check pull request descriptions against the template`
  (`Refs: CE-08.7`).

---

### Task 7: Umbrella parser and `ac-trace`

Suggested implementer model: mid-tier. ACs: CE-08.6.

**Files:**
- Create: `xtask/src/umbrella.rs`, `xtask/src/checks/ac_trace/{mod,rust_tags,ts_tags,junit}.rs`,
  `xtask/tests/ac_trace.rs`, and fixtures `xtask/tests/fixtures/ac-trace/<case>/` (each holds a
  `docs/specs/00-umbrella.md` excerpt and source files).
- Modify: `main.rs`, and the known-AC wiring for `commit-msg` (Task 4).

**Interfaces:**
- Produces:
  - `umbrella::{Umbrella, PhaseInfo { id: Phase, name: String, repo: String, acs:
    Vec<String> }, parse(text: &str) -> Result<Umbrella>, Umbrella::all_ids() ->
    BTreeSet<String>, Umbrella::phase(Phase) -> Option<&PhaseInfo>}`:
    - `repo` comes from the §3 table;
    - AC ids match `CE-\d{2}[A-Z]?\.\d+` at the start of list items (`- CE-…`) under each
      `### PNN ·` heading.
  - `pub struct Tag { pub file: String, pub line: u32, pub ids: Vec<String>, pub test:
    Option<String>, pub counts: bool }`.
  - `rust_tags::scan(file: &str, text: &str) -> (Vec<Tag>, Vec<Finding>)` and
    `ts_tags::scan(file: &str, text: &str) -> (Vec<Tag>, Vec<Finding>)`.
  - `junit::parse(xml: &str) -> Result<BTreeMap<String, bool>>`: test name → passed.
  - `ac_trace::check(umbrella: &Umbrella, phase: Option<Phase>, tags: &[Tag], junit:
    Option<&BTreeMap<String, bool>>, out: &mut dyn Write) -> Verdict`.

Rules (spec §5.4):
- **Tag line:** exactly `^\s*// AC: <id>(, <id>)*\s*$`. Any other line starting with `\s*//\s*AC`
  is a malformed finding.
- **Rust:** after the tag, only attribute lines (a multi-line attribute runs until its brackets
  balance) and `///` doc lines may appear before an `fn` line (`pub`/`async` allowed).
  - One attribute must be a test attribute: `#[test]`, `#[<path>::test…]`, `#[rstest…]` or
    `#[test_case…]`. Anything else is a dangling finding.
  - The tag does not count when an attribute is `#[ignore…]`, or is a `#[cfg(…)]` whose
    predicate names anything except `windows`, `unix`, `target_os`, `target_family`, `any`,
    `all`, `not` and string literals.
- **TypeScript:** the next line, after whitespace, starts with `test(`, `it(`, `test.each`,
  `it.each`, `test.only(` or `it.only(` (counts), or with `test.skip(`, `it.skip(` or
  `test.fixme(` (no dangling finding, but doesn't count). Anything else is dangling.
- **Files:** every `.rs` and `.ts` file from `repo_files`, except paths containing
  `/tests/fixtures/` or starting with `tests/fixtures/`, and `target/` and `node_modules/`.
  Research folders are included.
- **Verdict:**
  - unknown ids → findings;
  - with a phase whose `repo` is `theos-editor`: every AC id of that phase needs one counting
    tag; phases in another repo demand nothing (they get a note);
  - the phase comes from `--phase`, else the CI `head_ref`, else `current_branch()`, through
    `phase_of`.
- **Output:** a table of AC → `file:line test` rows, with a `pass`/`FAIL`/`–` column when
  `target/nextest/<profile>/junit.xml` exists. Match testcases by the trailing `::<fn name>`.

- [ ] **Step 1: Write the failing tests.** Unit tests:
  - `parses_phases_repos_and_ids`, on the real umbrella through
    `include_str!("../../docs/specs/00-umbrella.md")`: P00 has CE-08.1…CE-08.12, P05 contains
    `CE-08R.1`, and P01's repo is `tower-of-theos`;
  - `rust_tag_above_test_counts`;
  - `rust_tag_with_doc_and_attrs_counts`;
  - `rust_tag_above_helper_is_dangling`;
  - `rust_tag_with_blank_line_is_dangling`;
  - `ignored_test_does_not_count`;
  - `cfg_any_test_does_not_count_cfg_windows_does`;
  - `malformed_tags_are_findings` (`//AC: x`, `// AC - CE-08.1`, `// AC: CE-08.1,CE-08.2`);
  - `ts_playwright_test_counts_inside_describe`;
  - `ts_skip_and_fixme_do_not_count`;
  - `junit_parses_pass_and_fail`.

  Integration tests on fixture dirs, each tagged CE-08.6:

  | Test | Fixture | Expected |
  |---|---|---|
  | `phase_ac_without_tag_fails` | umbrella with P90 (`theos-editor`, CE-90.1, CE-90.2), one test tagged CE-90.1 | fail, finding names CE-90.2 |
  | `unknown_id_fails` | a test tagged CE-99.9 | fail |
  | `ignored_test_does_not_prove_an_ac` | the only CE-90.2 test is `#[ignore = "x"]` | fail |
  | `dangling_tag_fails` | tag above a non-test fn | fail |
  | `complete_phase_passes` | both ACs tagged (one Rust, one Playwright `.spec.ts`) | pass |
  | `other_repo_phase_demands_nothing` | phase P91 with repo `tower-of-theos`, no tags | pass |

- [ ] **Step 2:** Run `cargo nextest run -p xtask`. Expected: FAIL.
- [ ] **Step 3:** Implement. Wire up `cargo xtask ac-trace [--phase PNN]` and give
  `commit-msg` the real `Umbrella::all_ids()`.
- [ ] **Step 4:** Run the tests, then `cargo xtask ac-trace --phase P00`. Expected: tests PASS;
  the command lists the CE-08 ids and FAILs only for those not tagged yet. That is correct at
  this point, and Task 14 completes them.
- [ ] **Step 5:** Commit: `feat(xtask): trace acceptance criteria to tagged tests`
  (`Refs: CE-08.6`).

---

### Task 8: `lints`

Suggested implementer model: mid-tier. ACs: none directly (gate check of spec §5.4).

**Files:** Create `xtask/src/checks/lints.rs`, `xtask/tests/lints.rs`, and
`xtask/tests/fixtures/lints/<case>/` mini repos.

**Interfaces:**
- Produces: `pub fn check(root: &Path, files: &[String]) -> Result<Verdict>`, plus the pure
  helpers `scan_attributes(file: &str, text: &str) -> Vec<Finding>`,
  `workspace_lints_match(root_manifest: &str, gate_lints: &str) -> Result<bool>`, and
  `same_text(a: &str, b: &str) -> bool` (CRLF-insensitive).

Rules (spec §5.4):
- `[workspace.lints]` of the root `Cargo.toml` equals `gate/lints.toml`, compared as parsed
  `toml::Value`.
- Every member manifest has `[lints] workspace = true`.
- `clippy.toml` equals `gate/clippy.toml`, and `.config/nextest.toml` equals
  `gate/nextest.toml`, both by `same_text`.
- None of these exist anywhere else in `files`: a `clippy.toml` or `.clippy.toml` outside the
  root, any `rustfmt.toml` or `.rustfmt.toml`, or any `.cargo/` outside the root.
- In every `.rs` file except `tests/fixtures`, only attribute lines count, meaning lines whose
  trimmed text starts with `#[` or `#![`, joined until the brackets balance:
  - any `allow(` in the attribute (including inside `cfg_attr`) is a finding;
  - any `expect(` without `reason =` is a finding;
  - an `ignore` attribute without `= "…"` is a finding.
  - Lines starting with `//` are skipped.

- Code stays in `src/` (final review round 3): a `#[path]` or `include!` in a file under `crates/<name>/src/` or `xtask/src/`, and a `[lib]` or `[[bin]]` `path` of such a crate's manifest, must lead into that `src/` (`src_bounds.rs`); `include_str!` and `include_bytes!` stay allowed.
- [ ] **Step 1: Write the failing tests.** Unit tests:
  - `allow_attribute_found`, `inner_allow_found`, `cfg_attr_allow_found`;
  - `expect_without_reason_found`, `expect_with_reason_passes`;
  - `bare_ignore_found`, `ignore_with_reason_passes`;
  - `allow_in_doc_comment_ignored`, `multi_line_attribute_joined`;
  - `lints_tables_compare_parsed` (reformatted equal content passes; a changed level fails);
  - `crlf_mirror_equals_lf`.

  Integration tests on fixture mini repos:
  - `clean_repo_passes`;
  - `crate_without_workspace_lints_fails`;
  - `nested_clippy_toml_fails`;
  - `rustfmt_toml_fails`;
  - `crate_level_cargo_dir_fails`;
  - `mirror_drift_fails`.
- [ ] **Step 2:** Run the tests. Expected: FAIL.
- [ ] **Step 3:** Implement and wire up `cargo xtask lints`.
- [ ] **Step 4:** Run the tests, then `cargo xtask lints` on the repo. Expected: PASS for both.
- [ ] **Step 5:** Commit: `feat(xtask): check lint tables, config mirrors and lint attributes`.

---

### Task 9: `engine-free`

Suggested implementer model: mid-tier. ACs: CE-08.2.

**Files:** Create `xtask/src/checks/engine_free.rs`, `gate/engine-crates.toml`,
`xtask/tests/engine_free.rs`, and `xtask/tests/fixtures/engine-free/` holding:
- `engine/theos-core/`: a fake engine crate, outside every case's workspace;
- one case dir per test, each with its own `[workspace]` root.

**Interfaces:**
- Produces:
  - `pub struct EngineCrates { pub sources: Vec<String>, pub names: Vec<String> }` with
    `load(root)`.
  - `pub fn declared_findings(meta: &cargo_metadata::Metadata, engine: &EngineCrates) ->
    Vec<Finding>`.
  - `pub fn resolved_findings(meta: &cargo_metadata::Metadata, engine: &EngineCrates) ->
    Vec<Finding>`.
  - `pub fn check(root: &Path, engine: &EngineCrates, locked: bool, offline: bool) ->
    Result<Verdict>`. It runs `cargo metadata --no-deps` for the declared check, then
    `cargo metadata` with default features and no `--filter-platform` for the resolved check.

Fixed values:
- `gate/engine-crates.toml`:
  - `sources = ["github.com/fpgschiba/theos-engine"]`, matched case-insensitively as a
    substring of the source URL;
  - `names` lists the 16 engine workspace members (`theos-shared`, `theos-math`, `theos-core`,
    `theos-ecs`, `theos-assets`, `theos-render`, `theos-physics`, `theos-ai`, `theos-magic`,
    `theos-scripting`, `theos-window`, `theos-plugin-api`, `theos-plugin-loader`,
    `theos-launcher`, `theos-editor`, `theos-game`) plus `theos-editor-api`.

Rules (spec §5.4):
- **Engine crate:** a crate whose source matches, or a path dependency or package outside the
  workspace root, or a crate whose name is listed and which is not a workspace member (by path
  inside the root).
- **Declared:** every dependency (any kind, any target, also renamed through `package`) that is
  an engine crate must be `optional`.
  - Every feature that references it (`dep:<key>`, `<key>`, `<key>/<f>`, `<key>?/<f>`, where
    `<key>` is the rename or the name) must be `engine`. This also catches implicit features.
  - The features reachable from `default` must not include `engine`.
  - A dependency on a workspace member with `features` containing `engine` counts as an engine
    dependency.
- **Resolved:** no engine crate among the resolve's packages.

- [ ] **Step 1: Write the failing integration tests.** The tests copy a case dir together with
  `engine/` into a tempdir (keeping their relative layout) and run `check(…, locked = false,
  offline = true)`, so nothing is written into the repo and no network is used. Each test is
  tagged CE-08.2:

  | Test | Case | Expected |
  |---|---|---|
  | `non_optional_engine_dep_fails` | `theos-core = { path = "../engine/theos-core" }` | fail |
  | `optional_dep_behind_other_feature_fails` | optional, enabled by feature `render` | fail |
  | `engine_reached_from_default_fails` | `default = ["full"]`, `full = ["engine"]` | fail |
  | `renamed_engine_dep_fails` | `core = { package = "theos-core", path = … }` non-optional | fail |
  | `dev_dependency_on_engine_fails` | `[dev-dependencies]` non-optional | fail |
  | `implicit_feature_fails` | optional without any `dep:` reference | fail |
  | `dep_behind_engine_only_passes` | optional, `engine = ["dep:theos-core"]` | pass |
  | `own_member_named_theos_editor_passes` | workspace member `theos-editor`, depended on by path | pass |

  Unit test `source_match_is_case_insensitive`.
- [ ] **Step 2:** Run the tests. Expected: FAIL.
- [ ] **Step 3:** Implement and wire up `cargo xtask engine-free`, which runs `locked = true`
  and `offline = false` on the real workspace.
- [ ] **Step 4:** Run the tests and `cargo xtask engine-free`. Expected: PASS for both.
- [ ] **Step 5:** Commit: `feat(xtask): keep engine crates behind the engine feature`
  (`Refs: CE-08.2`).

---

### Task 10: `coverage`

Suggested implementer model: mid-tier. ACs: CE-08.8.

**Files:** Create `xtask/src/checks/coverage/{mod,llvm_json,baseline}.rs`,
`gate/thresholds.toml`, `xtask/tests/coverage.rs`, and fixture
`xtask/tests/fixtures/coverage/llvm-cov.json` (small, synthetic, marked as synthetic in a
`README` next to it).

**Interfaces:**
- Produces:
  - `pub struct CrateCoverage { pub covered: u64, pub count: u64 }` with `fn percent(&self) ->
    Option<f64>` (None when count is 0).
  - `pub fn parse_llvm_cov(json: &str, root: &Path) -> Result<BTreeMap<String,
    CrateCoverage>>`. It sums `summary.lines` per crate over files under
    `<root>/crates/<name>/src/` and `<root>/xtask/src/`. Separators are normalised and
    comparison is case-insensitive on Windows.
  - `pub enum CrateClass { Library, Binary }` and `pub struct Thresholds { pub max_drop: f64,
    pub library: f64, pub binary: f64, pub crates: BTreeMap<String, CrateClass> }` with
    `load(root)`.
  - `pub fn verdict(head: &BTreeMap<String, CrateCoverage>, baseline: Option<&BTreeMap<String,
    CrateCoverage>>, t: &Thresholds, members: &[String]) -> Verdict`.
  - `baseline::measure(root: &Path, sha: &str, cache_dir: &Path) -> Result<Option<BTreeMap<…>>>`.
  - `pub fn check(root: &Path, base_rev: &str) -> Result<Verdict>`.

Fixed values:
- `gate/thresholds.toml`: `max_drop = 1.0`, `[floors] library = 80.0` and `binary = 60.0`.
  `[crates]` maps `theos-content`, `theos-vcs`, `theos-studio` and `xtask` to `"library"`, and
  `theos`, `theos-mcp` and `theos-editor` to `"binary"`.

Rules:
- **Fails** when:
  - a member is missing from `[crates]`;
  - a crate with a `percent` is below its class floor;
  - a crate present in both head and baseline with percents dropped by `> max_drop` points.
- A crate with `count == 0` passes with the note "no instrumented lines".
- **Head:** `cargo llvm-cov nextest --workspace --locked --json --output-path
  target/xtask/coverage/head.json`.
- **Baseline:**
  - the base rev comes from the event `base_sha`, else `origin/main`, else `--base`;
  - the baseline is the merge base of `HEAD` and that rev;
  - if it equals `HEAD`, the baseline is the head result;
  - otherwise `git worktree add --detach target/xtask/coverage/wt <sha>`, run the same command
    there (with `CARGO_TARGET_DIR=target/xtask/coverage/target`), and always remove the
    worktree afterwards;
  - a baseline without `xtask/Cargo.toml` gives `None` (P00's own PR);
  - cache: `target/xtask/coverage/cache/<sha>-<toolchain>-<llvm-cov version>.json`, where
    `<toolchain>` is the first 12 hex digits of the sha256 of the `release:` and `commit-hash:`
    lines of `rustc -vV`. It is the toolchain that measures, which is the one of the cargo that
    runs the gate, and not the `rust-toolchain.toml` of the baseline commit.
- On a push to `main`, also copy the head result into the cache under `HEAD`'s sha, so the next
  PRs find it.

- [ ] **Step 1: Write the failing tests.** Unit tests on `verdict` with literal maps, each tagged
  CE-08.8:
  - `crate_below_floor_fails` (theos-content 79.9 % of a library);
  - `binary_floor_is_60` (theos at 60.0 passes, 59.9 fails);
  - `drop_over_max_fails` (82.0 → 80.9 fails);
  - `drop_within_max_passes` (82.0 → 81.0 passes);
  - `unlisted_member_fails`;
  - `empty_crate_passes`;
  - `crate_new_in_pr_has_no_drop_check`.

  Integration test `parses_llvm_cov_fixture`.
- [ ] **Step 2:** Run the tests. Expected: FAIL.
- [ ] **Step 3:** Implement and wire up `cargo xtask coverage`.
- [ ] **Step 4:** Run the tests, then `cargo xtask coverage` locally. Expected: PASS. The
  baseline for this branch is `None`, because `main` has no xtask yet. xtask itself must
  reach 80 %; add tests where it doesn't.
- [ ] **Step 5:** Commit: `feat(xtask): enforce per-crate coverage floors and drops`
  (`Refs: CE-08.8`).

---

### Task 11: `approval` with fingerprints, and the GitHub client

Suggested implementer model: mid-tier; the task reviewer must check D2/D3 line by line. ACs:
CE-08.5.

**Files:** Create `xtask/src/checks/approval/{mod,protected,fingerprint,decide}.rs`,
`xtask/src/github/{mod,gh_cli,fake}.rs`, `gate/protected-paths.toml`,
`xtask/tests/approval.rs`.

**Interfaces:**
- Produces:
  - `github::Comment { pub author: String, pub body: String, pub created_at: String, pub
    updated_at: String }` and `github::LabelSpec { pub name: String, pub color: String, pub
    description: String }`.
  - `pub trait PullRequests`, with `labels(&self, pr: u64) -> Result<Vec<String>>`,
    `add_label(&self, pr: u64, name: &str) -> Result<()>`, `remove_label(&self, pr: u64,
    name: &str) -> Result<()>`, `ensure_label(&self, spec: &LabelSpec) -> Result<()>`,
    `comments(&self, pr: u64) -> Result<Vec<Comment>>` (all pages) and `comment(&self, pr:
    u64, body: &str) -> Result<()>`.
  - `gh_cli::GhCli { repo: String }`, which implements it through `gh api`. The repo comes from
    `GITHUB_REPOSITORY`.
  - `fake::FakeGitHub` (`#[cfg(test)]`), which records calls.
  - `protected::ProtectedPaths::load(root) -> Result<Self>` and `matches(&self, path: &str)
    -> bool`:
    - globs use globset with `literal_separator(true)`;
    - `protected` patterns are case-insensitive and `except` patterns are case-sensitive.
  - `protected::touched(git: &Git, rules: &ProtectedPaths, merge_base: &str, head: &str) ->
    Result<Vec<String>>`: the protected paths from `diff_name_only_no_renames`, plus
    `Cargo.toml [workspace.lints]` when the parsed table differs between `git show
    <merge_base>:Cargo.toml` and `git show <head>:Cargo.toml`.
  - `fingerprint::Fingerprint(pub String)` and `fingerprint::compute(git, merge_base, head) ->
    Result<Fingerprint>`: the sha256 hex of `diff_raw`.
  - `decide::MARKER` = `<!-- theos-gate:approval`.
  - `decide::marker(fp, head) -> String`, which gives `<!-- theos-gate:approval
    fingerprint=<hex> head=<sha> -->` plus the sentence "Approval recorded for this content of
    the PR.".
  - `decide::recorded(comments: &[Comment]) -> Option<Fingerprint>`: the newest comment whose
    author is `github-actions[bot]`, with `created_at == updated_at`, that contains the marker.
  - `pub enum Event { Labeled(String), Other }`, from the PR event's action and label.
  - `pub enum Action { EnsureLabel(LabelSpec), AddLabel(String), RemoveLabel(String),
    Comment(String) }`.
  - `decide::decide(touched: &[String], event: &Event, labels: &[String], recorded:
    Option<&Fingerprint>, current: &Fingerprint, head: &str) -> (Verdict, Vec<Action>)`.
  - `approval::run(git: &Git, rules: &ProtectedPaths, pr: &PullRequestEvent, gh: &dyn
    PullRequests) -> Result<Verdict>`:
    1. compute the merge base;
    2. if the head sha is missing locally, run `git fetch --no-tags origin
       refs/pull/<n>/head`;
    3. take `touched`, the fingerprint, the fresh labels and the comments;
    4. `decide`, then execute the actions in order.

Fixed values:
- `gate/protected-paths.toml`:
  - `protected = ["CLAUDE.md", ".claude/**", "CONTRIBUTING.md", "docs/**", ".github/**",
    "gate/**", "xtask/**", "deny.toml", "rust-toolchain.toml", ".cargo/**", "CHANGELOG.md",
    "release-plz.toml", "cliff.toml", "design/**"]`;
  - `except = ["docs/plans/**", "docs/status.md"]`;
  - `[[tables]] file = "Cargo.toml"`, `table = "workspace.lints"`.
- Label specs: `needs-jann` (colour `d93f0b`), `approved` (`0e8a16`), `macos` (`5319e7`).
  Task 12 reads the same values from `gate/repo-settings.toml`; until then
  `decide::NEEDS_JANN` holds them.

Decision table (`decide`), as spec §5.7. Rows are evaluated top to bottom; the first match wins.
Every protected row starts with `EnsureLabel(needs-jann)` and `AddLabel(needs-jann)` if the
label is missing.

| # | Condition | Verdict | Further actions |
|---|---|---|---|
| 1 | `touched` is empty | pass | none |
| 2 | `approved` not in the fresh labels (any event, also a `Labeled("approved")` already undone) | fail "waits for Jann: add the label `approved` after review" | none |
| 3 | event is `Labeled("approved")` | pass | `Comment(marker(current, head))` |
| 4 | `recorded == Some(current)` | pass | none |
| 5 | otherwise (differs, or none recorded) | fail "approval removed: the PR changed after approval (old → new)" | `RemoveLabel("approved")`, `Comment(<same reason>)` |

- [ ] **Step 1: Write the failing tests.** Unit tests in `decide.rs`, one per table row, each
  tagged CE-08.5:
  - `unprotected_pr_passes` (row 1);
  - `protected_without_label_fails_and_adds_needs_jann` (row 2);
  - `adding_approved_records_fingerprint_and_passes` (row 3);
  - `approved_with_same_fingerprint_passes` (row 4);
  - `changed_fingerprint_removes_approval_and_fails` (row 5);
  - `approved_without_recorded_fingerprint_fails` (row 5);
  - `labeled_event_with_label_already_removed_fails` (row 2).

  More unit tests:
  - `recorded_ignores_comments_not_by_bot`;
  - `recorded_ignores_edited_comments`;
  - `recorded_takes_newest`.

  In `protected.rs`:
  - `docs_protected_plans_and_status_not` (`docs/gate.md` yes; `docs/plans/P00-x.md` no;
    `docs/status.md` no);
  - `case_variants_are_protected` (`DOCS/x.md`, `Claude.md`);
  - `star_does_not_cross_slash`.

  Integration tests in `xtask/tests/approval.rs`, on temp git repos:
  - `rename_out_of_docs_is_protected` (tag CE-08.5);
  - `lints_table_change_is_protected` (tag CE-08.5);
  - `reformatted_lints_table_is_not_a_change`;
  - `fingerprint_changes_on_whitespace_only_edit`;
  - `fingerprint_changes_on_mode_change`, which uses `git update-index --chmod=+x`;
  - `fingerprint_stable_across_rebase_onto_unrelated_main_commit`;
  - `protected_paths_toml_matches_gate_md_section_5`: every pattern in `protected` and `except`
    appears backticked in `docs/gate.md` §5.
- [ ] **Step 2:** Run the tests. Expected: FAIL.
- [ ] **Step 3:** Implement and wire up `cargo xtask approval` (CI only: it needs the PR event
  and `GH_TOKEN`; locally it reports `Error::Missing`).
- [ ] **Step 4:** Run the tests. Expected: PASS.
- [ ] **Step 5:** Commit: `feat(xtask): gate protected paths on an approval tied to content`
  (`Refs: CE-08.5`).

---

### Task 12: GitHub settings as code

Suggested implementer model: mid-tier. ACs: CE-08.3 (file side; the workflow side is in Task 14).

**Files:** Create `xtask/src/github/settings.rs`, `gate/ruleset-main.json`,
`gate/ruleset-tags.json`, `gate/repo-settings.toml`, `xtask/tests/settings.rs`.

**Interfaces:**
- Consumes: `GhCli`, the `LabelSpec` (Task 11), and `decide::NEEDS_JANN`, which now reads from
  `repo-settings.toml`.
- Produces:
  - `pub trait RepoAdmin`, with `rulesets(&self) -> Result<Vec<(u64, String)>>`,
    `ruleset(&self, id: u64) -> Result<serde_json::Value>`, `create_ruleset(&self, body:
    &Value) -> Result<()>`, `update_ruleset(&self, id: u64, body: &Value) -> Result<()>`,
    `repo(&self) -> Result<Value>`, `patch_repo(&self, body: &Value) -> Result<()>`,
    `repo_labels(&self) -> Result<Vec<LabelSpec>>` and `upsert_label(&self, spec: &LabelSpec)
    -> Result<()>`. `GhCli` and `FakeGitHub` implement it. The name `repo_labels` avoids a
    clash with `PullRequests::labels`.
  - `settings::Desired { rulesets: Vec<Value>, repo: toml::Table, labels: Vec<LabelSpec> }`
    with `load(root)`.
  - `settings::differences(desired: &Desired, live: &Live) -> Vec<String>`:
    - rulesets match by `name`, and only keys present in the desired JSON are compared;
    - `rules` compare as sets keyed by `type`;
    - `required_status_checks` compare as sets of `(context, integration_id)`;
    - repo fields absent from the live JSON are reported as "not checked" notes, not
      differences.
  - `settings::check(gh: &dyn RepoAdmin, root) -> Result<Verdict>` and `settings::apply(gh:
    &dyn RepoAdmin, root) -> Result<Vec<String>>` (create or update; idempotent).
  - `pub fn required_contexts_from_gate_md(text: &str) -> BTreeSet<String>`. It parses the
    `docs/gate.md` §1 table, skips `ui`, and turns OS `Linux, Windows` into `<check> (linux)`
    and `<check> (windows)`; Linux-only checks keep their plain name.

Fixed values:
- `ruleset-main.json`:
  - name `main`, target `branch`, enforcement `active`, `bypass_actors: []`;
  - `conditions.ref_name.include = ["~DEFAULT_BRANCH"]` and `exclude = []`;
  - rules `deletion`, `non_fast_forward` and `required_linear_history`;
  - rule `pull_request` with `required_approving_review_count: 0`,
    `dismiss_stale_reviews_on_push: false`, `require_code_owner_review: false`,
    `require_last_push_approval: false`, `required_review_thread_resolution: false` and
    `allowed_merge_methods: ["squash"]`;
  - rule `required_status_checks` with `strict_required_status_checks_policy: true`,
    `do_not_enforce_on_create: false`, and these 15 contexts, each with `integration_id:
    15368`: `fmt`, `clippy (linux)`, `clippy (windows)`, `test (linux)`, `test (windows)`,
    `docs`, `deny`, `engine-free`, `coverage`, `lints`, `ac-trace`, `commit-msg`, `pr-title`,
    `pr-body`, `approval`.
- `ruleset-tags.json`: name `tags`, target `tag`, enforcement `active`, `bypass_actors: []`,
  include `["refs/tags/v*"]`, and rules `update` and `deletion`.
- `repo-settings.toml`:
  - `[repo]` with `allow_squash_merge = true`, `allow_merge_commit = false`,
    `allow_rebase_merge = false`, `allow_auto_merge = true`, `delete_branch_on_merge = true`,
    `allow_update_branch = true`, `squash_merge_commit_title = "PR_TITLE"` and
    `squash_merge_commit_message = "COMMIT_MESSAGES"`;
  - `[[labels]]` for `needs-jann` (`d93f0b`, "Touches a protected path; waits for Jann"),
    `approved` (`0e8a16`, "Reviewed by Jann; only Jann adds this label") and `macos`
    (`5319e7`, "Run the macOS jobs on this pull request").

- [ ] **Step 1: Write the failing tests,** each tagged CE-08.3:
  - `ruleset_contexts_equal_gate_md_checks`: `required_contexts_from_gate_md(docs/gate.md)`
    equals the 15 contexts in `ruleset-main.json`, with `integration_id` 15368 on every one;
  - `main_ruleset_encodes_section_6`: no bypass, squash only, linear history, strict, and no
    force push or deletion;
  - `tag_ruleset_blocks_update_and_deletion`;
  - `repo_settings_encode_section_6`.

  Untagged:
  - `differences_ignore_extra_live_keys_and_order`;
  - `missing_ruleset_is_a_difference`;
  - `apply_creates_then_updates` (FakeGitHub: the first apply creates, the second updates, and
    the third has no difference).
- [ ] **Step 2:** Run the tests. Expected: FAIL.
- [ ] **Step 3:** Implement and wire up `cargo xtask github check|apply [--repo owner/name]`.
  The repo defaults to `GITHUB_REPOSITORY`, else the `origin` URL.
- [ ] **Step 4:** Run the tests, then `cargo xtask github check` against the live repo, read
  only. Expected: tests PASS; the live check FAILs listing every difference, which is correct
  before `apply`. Do **not** run `apply` (spec §10: it needs Jann's go-ahead).
- [ ] **Step 5:** Commit: `feat(xtask): keep rulesets, repo settings and labels as code`
  (`Refs: CE-08.3`).

---

### Task 13: `cargo xtask gate` and the root README

Suggested implementer model: cheapest. ACs: CE-08.4.

**Files:** Create `xtask/src/gate.rs`. Modify `main.rs` and the root `README.md`.

**Interfaces:**
- Consumes: every check runner (Tasks 3–10).
- Produces: `pub fn run(root: &Path, out: &mut dyn Write) -> Result<Vec<Verdict>>`. It runs
  `GATE_ORDER`, keeps going after a failure, then writes `write_table`. `main` exits 1 if any
  verdict failed.
  - When the `tools` verdict fails, the other checks are skipped, with the note "install the
    pinned tools first" (versions decide verdicts).
  - A check that errors becomes a failing verdict carrying the error text.

- [ ] **Step 1: Write the failing test** `gate_runs_every_ci_check_except_pr_only` (tag
  CE-08.4). It asserts that `GATE_ORDER` without `Tools`, joined with `PR_ONLY`, equals every
  `Check` except `Tools`, so no check is missing from both lists. Task 14 adds the matching test
  against the workflow files.
- [ ] **Step 2:** Run it. Expected: FAIL until `gate.rs` exists.
- [ ] **Step 3:** Implement. Write the README section "Building and the gate":
  `rustup toolchain install`, `cargo xtask tools --install`, `cargo xtask gate`, the list of
  checks, and `cargo xtask ac-trace --phase PNN`.
- [ ] **Step 4:** Run `cargo xtask gate` locally on Windows. Expected: every check passes except
  `ac-trace` (CE-08.1/3/4/12 tags complete in Task 14); record the table in the task report.
- [ ] **Step 5:** Commit: `feat(xtask): run every local check with cargo xtask gate`
  (`Refs: CE-08.4`).

---

### Task 14: Workflows, the setup action, and the workflow-shape tests

Suggested implementer model: mid-tier. ACs: CE-08.1, CE-08.2, CE-08.3, CE-08.4, CE-08.12.

**Files:** Create `.github/actions/setup/action.yml`, `.github/workflows/{ci,pr,approval,macos,
settings}.yml`, `xtask/tests/workflows.rs`.

**Interfaces:**
- Consumes: the check names (Task 2), `required_contexts_from_gate_md` (Task 12) and
  `install_spec` (Task 3).

Fixed values:
- Action pins, as `owner/repo@<sha> # <tag>`:
  - `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1`
  - `actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9 # v6.1.0`
  - `Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0 # v2.9.3`
  - `taiki-e/install-action@68a389c788b2cd04016ce8c80ee23b80d2ab3d68 # v2.88.1`
- Setup action (composite, `shell: bash` on every step):
  1. `rustup toolchain install` (without arguments it installs the toolchain of
     `rust-toolchain.toml`; since rustup 1.28 `rustup show` no longer does);
  2. rust-cache;
  3. a step `id: tools` that runs `cargo xtask tools --print-install-spec` and writes
     `spec=<line>` to `$GITHUB_OUTPUT`;
  4. install-action with `tool: ${{ steps.tools.outputs.spec }}`.
- **Jobs are explicit, with no matrix.** The job `name` is the context, and its `runs-on`
  follows the OS suffix: `(linux)` → `ubuntu-latest`, `(windows)` → `windows-latest`,
  `(macos)` → `macos-latest`, none → `ubuntu-latest`.
  - `ci.yml`:
    - jobs `fmt`, `clippy (linux)`, `clippy (windows)`, `test (linux)`, `test (windows)`,
      `docs`, `deny`, `engine-free`, `coverage`, `lints`, `ac-trace` and `commit-msg`;
    - `commit-msg` and `coverage` check out with `fetch-depth: 0`;
    - `coverage` restores and saves `actions/cache` for `target/xtask/coverage/cache` with key
      `coverage-${{ github.event.pull_request.base.sha || github.sha }}-${{
      hashFiles('rust-toolchain.toml', 'gate/tools.toml') }}` and restore-keys `coverage-`.
  - `pr.yml`: jobs `pr-title` and `pr-body`.
  - `approval.yml`: job `approval`. Its steps are, in order:
    1. checkout with `ref: ${{ github.event.pull_request.base.sha }}` and `fetch-depth: 0`;
    2. checkout of the PR head with `fetch-depth: 0` and `if: hashFiles('xtask/Cargo.toml') ==
       ''` (the P00-only fallback, D3);
    3. setup;
    4. `cargo xtask approval`.

    Job `env: GH_TOKEN: ${{ github.token }}`; permissions `contents: read`, `pull-requests:
    write` and `issues: write`.
  - `macos.yml`: jobs `clippy (macos)` and `test (macos)`. Triggers: `schedule: cron "0 4 * *
    1"`, `workflow_dispatch`, and `pull_request: types: [labeled]`, with job-level `if:
    github.event_name != 'pull_request' || github.event.label.name == 'macos'`.
  - `settings.yml`: job `settings` with `cargo xtask github check`, triggered weekly by `cron "0
    5 * * 1"` and by `workflow_dispatch`, with `GH_TOKEN` as above. Its permissions are
    `contents: read` and `issues: read`: the check lists the labels of the repository, which
    needs `issues: read` in a private repository, and a job-level `permissions:` replaces the
    top-level one. It writes nothing.
- **Triggers and permissions:**
  - triggers as spec §5.6;
  - top-level `permissions: contents: read`;
  - `ci.yml` and `pr.yml` have `concurrency: { group: ${{ github.workflow }}-${{
    github.event.pull_request.number || github.ref }}, cancel-in-progress: ${{
    github.event_name == 'pull_request' }} }`;
  - `approval.yml` has the group per PR with `cancel-in-progress: false`.

- [ ] **Step 1: Write the failing tests** in `xtask/tests/workflows.rs`. They parse the YAML
  with yaml-rust2.

  | Test | Assertion | Tag |
  |---|---|---|
  | `ci_builds_and_tests_on_linux_and_windows` | `clippy (linux)`/`test (linux)` on `ubuntu-latest` and `(windows)` on `windows-latest`, running `cargo xtask clippy`/`test` | CE-08.1 |
  | `engine_free_runs_on_every_pr` | `ci.yml` job `engine-free` runs `cargo xtask engine-free` and the workflow triggers on `pull_request` | CE-08.2 |
  | `required_contexts_are_pr_jobs` | the union of job names in `ci.yml`, `pr.yml` and `approval.yml` equals `required_contexts_from_gate_md(docs/gate.md)` and the contexts of `gate/ruleset-main.json` | CE-08.3 |
  | `every_job_only_runs_its_own_check` | every job in every workflow has exactly one `run:` step, equal to `cargo xtask <name without OS suffix>` (`settings` → `cargo xtask github check`); `approval` has the two checkouts described above and nothing else extra | CE-08.4 |
  | `gate_list_equals_ci_jobs_minus_pr_only` | the CI check names (OS suffix stripped, deduplicated) minus `pr-title`, `pr-body` and `approval` equal `GATE_ORDER` without `tools` | CE-08.4 |
  | `macos_runs_weekly_and_on_demand` | `macos.yml` has a `schedule` cron, `workflow_dispatch`, and jobs on `macos-latest` running `cargo xtask clippy` and `cargo xtask test` | CE-08.12 |
  | `required_jobs_always_report` | the PR workflows (`ci.yml`, `pr.yml`, `approval.yml`) have no `paths`/`paths-ignore`/`branches` filters on `pull_request` and no job-level `if:` | – |
  | `no_pull_request_target_and_least_privilege` | no `pull_request_target`; top-level permissions are `contents: read`; only `approval` has write scopes | – |
  | `actions_pinned_by_sha` | every `uses:` that isn't local matches `@[0-9a-f]{40}` | – |
  | `no_expressions_in_run_steps` | no `run:` value contains `${{` | – |

- [ ] **Step 2:** Run the tests. Expected: FAIL.
- [ ] **Step 3:** Write the action and the workflows.
- [ ] **Step 4:** Run the tests, then `cargo xtask ac-trace --phase P00` and `cargo xtask gate`.
  Expected: PASS everywhere; `ac-trace` lists CE-08.1…CE-08.12, each with at least one test.
- [ ] **Step 5:** Commit: `ci(gate): run every check as github actions jobs over xtask`
  (`Refs: CE-08.1, CE-08.2, CE-08.3, CE-08.4, CE-08.12`).

---

## After the tasks (theos-phase steps 4–6, not implementer tasks)

1. Final review loop as `CLAUDE.md` defines it: a fresh whole-branch reviewer each round, one
   fix subagent per round, until a round is clean; cap 5 rounds.
2. `cargo xtask gate` on Windows (output read), and `cargo xtask ac-trace --phase P00`.
3. Push, and open `feat(gate): foundation and gate (P00)` with the template filled.
   - The workflows run for the first time on that PR, and `approval` adds `needs-jann` through
     its fallback.
   - Add the label `macos` to get the macOS run.
   - Ask Jann for the go-ahead to `cargo xtask github apply`, then put the `github check`
     output into the PR (spec §10).
   - No auto-merge: the PR touches protected paths.
