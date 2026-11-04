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
roxmltree 0.21. Revision 1 (Part 2) adds syn 2 (`full`, `visit`, `extra-traits`), proc-macro2 1
(`span-locations`), pulldown-cmark 0.13, quick-xml 0.41 and git-conventional 0.12, and removes
roxmltree and regex where nothing uses them any more. Dev: tempfile 3, yaml-rust2 0.13. Tools:
cargo-nextest 0.9.150, cargo-deny 0.20.2, cargo-llvm-cov 0.9.1. Actions: checkout v7.0.1, cache
v6.1.0, Swatinem/rust-cache v2.9.3, taiki-e/install-action v2.88.1.

**Spec:** `docs/specs/phases/P00-foundation-and-gate.md` (approved, PR #5; revision 1 approved
and merged, PR #9, 3 Nov 2026). Read it with this plan. Where this plan is silent, the spec
decides; where they differ, the spec wins. Part 1 (Tasks 1–14) was built against the original
spec; Part 2 (Tasks 15–25) builds revision 1 on the same branch.

## Global Constraints

Umbrella global constraints, verbatim. CLAUDE.md asks for G1–G16; G17 and G18 were added to the
umbrella later and are copied too.

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
- In every `.rs` file except `tests/fixtures`, every attribute counts (final review round 7: read from the token stream by `attribute_reader.rs`, inner and outer, anywhere in a line, across lines, in function and macro bodies; strings, character literals and comments never count; block comments nest):
  - any `allow(` in the attribute (including inside `cfg_attr`) is a finding;
  - any `expect(` without `reason =` is a finding;
  - an `ignore` attribute without `= "…"` is a finding.
  - Lines starting with `//` are skipped.

- Code stays in `src/` (final review round 3): a `#[path]` or `include!` in a file under `crates/<name>/src/` or `xtask/src/`, and a `[lib]` or `[[bin]]` `path` of such a crate's manifest, must lead into that `src/` (`src_bounds.rs`); `include_str!` and `include_bytes!` stay allowed.
- No legacy `rust-toolchain` file anywhere (final review round 5): rustup reads it instead of the protected `rust-toolchain.toml`.
- A `cfg(…)` or `cfg_attr(…)` predicate may use only `test`, `doc`, `unix`, `windows`, `feature = "…"` and `target_*` keys, with `all`, `any` and `not`; any other name, a predicate that cannot be read, and an attribute named `coverage` are findings (final review round 7, replacing the round 5 `coverage` denylist; `cfg_names.rs`); `#[cfg(not(test))]` stays allowed.
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

## Part 2 — Revision 1 (3 Nov 2026)

> **For agentic workers:** the same sub-skill as Part 1 (superpowers:subagent-driven-development).
> Tasks continue the numbering: 15–25. Each task's implementer sees only its own task and the
> sections of Part 2 above the tasks; the Interfaces blocks are how tasks learn each other's names.

**Goal:** The checks read code through the compiler, cargo and real parsers instead of the
hand-written scanners of Part 1 (spec §4.3 D4–D8, §5.10), per-PR checks run on Linux only, and
Windows joins macOS in the `platforms` workflow.

**Architecture:** One module (`xtask/src/view/`) builds the *workspace view*: the artifacts of
`cargo check --workspace --all-targets --locked --message-format=json`, the files of each one from
its dep-info, and every Rust file among them parsed once with `syn` (positions through
`proc-macro2` span locations, comments as the text between tokens). `lints` is four rule modules
over the view and the file list, registered in one list; `ac-trace` reads tests, attributes and
tags from the view. Markdown goes through one shared module over `pulldown-cmark`, JUnit through
`quick-xml`, commit messages through `git-conventional`. The view is built at most once per xtask
run and shared by `lints` and `ac-trace` in `cargo xtask gate`.

**Tech stack added:** syn 2 (`full`, `visit`, `extra-traits`) · proc-macro2 1 (`span-locations`)
· pulldown-cmark 0.13 (no default features) · quick-xml 0.41 · git-conventional 0.12.
Everything else as in Part 1. Each new crate must pass `cargo xtask deny` (licence allowlist in
spec §4.2; all five are MIT and/or Apache-2.0).

**Spec:** `docs/specs/phases/P00-foundation-and-gate.md` revision 1 (#9), §4.3, §5.2, §5.4,
§5.6, §5.8, §5.10, §6, §9; `docs/gate.md` §1–§2 as revised; umbrella G2 as revised.

### Global Constraints (Part 2)

Part 2 inherits every line of Part 1's "Global Constraints" (the umbrella's G1–G18, with G2 in
its revised wording above, the twelve ACs verbatim, and the rules every task follows), and adds
the engineering directives of `CLAUDE.md`, verbatim:

1. **Maintainability and extendability come first.** Code is easy to read, change and extend:
   small modules with one job, shared inputs built in one place, and a new rule or check is one
   new module plus one line that registers it.
2. **Use what exists before building anything.** Before writing a parser, checker or tool, find
   out whether rustc, clippy, cargo, an existing tool or a maintained crate already does it, and
   confirm that with a quick probe on the pinned toolchain. Prefer, in this order: configuring a
   standard tool, a well-known crate, custom code. Never hand-write a lexer or parser for a
   format that has a real one (Rust: rustc, clippy, `syn`; TOML, YAML, JSON, Markdown, XML: their
   crates). Every design and plan names, for each part, the tool or crate that does it, or says
   why custom code is needed.
3. **When fixes keep leaking, change the approach.** If reviews keep finding new variants of the
   same problem, stop patching single cases. Step back, name the class, and run the full
   superpowers loop for a new approach (brainstorming, written design, plan, build, review)
   instead of improvising.
4. **The gate keeps code quality high; it is not a fortress.** It must reliably catch the everyday
   shortcuts (silencing a lint, hiding code from coverage, skipping a test) and never flag
   ordinary code. Deliberate, exotic evasion that any diff shows is left to the reviews.

Changes to Part 1's task rules, for Part 2:

- **Scanner test inputs.** Part 1 forbade `#[allow` or `// AC:` text at the start of a line in a
  string literal of our sources, because the line scanners read every `.rs` file. From Task 24
  (lints) and Task 25 (ac-trace) on, a string is one token and is never an attribute or a
  comment, so that rule ends. Until then it still holds. Fixture repositories still live under
  `xtask/tests/fixtures/**`; each one is a workspace with no dependency outside itself (tests need
  no network), carries a `Cargo.lock` (Ruling 13) and a one-line `README` saying it is synthetic.
- **Commits.** As Part 1, with `Co-Authored-By` naming the model that wrote the commit (ledger
  ruling) and `Phase: P00`. Before each commit: `cargo fmt --all`, `cargo clippy --workspace
  --all-targets --locked -- -D warnings`, `cargo nextest run --workspace --locked`, and from
  Task 16 on the lints of spec §5.2 at `forbid` are in force: no `#[allow]` anywhere, a reason on
  every `#[expect]` and `#[ignore]`, no cfg name outside `gate/cfg-names.toml`.
- **Branch stays green.** Every task leaves `cargo nextest run --workspace --locked` green, with
  one exception: on the merged head a6b6659 two tests already fail
  (`settings::ruleset_contexts_equal_gate_md_checks`, `workflows::required_contexts_are_pr_jobs`)
  because `docs/gate.md` §1 changed under them; Task 15 goes first and makes them pass.
- **Model.** Implementers on a mid-tier model for every task (the plan is prose; ledger ruling
  R-pre3), task reviewers mid-tier, the final loop on the most capable model.

### Rulings (Part 2)

Recorded here and in the ledger; every one goes into the PR description.

1. Ruling: an artifact's dep-info is the file next to its first `filenames` entry whose name is
   the entry's stem without a leading `lib` plus `.d` (`libxtask-<hash>.rmeta` → `xtask-<hash>.d`,
   `build_script_build-<hash>[.exe]` → `build_script_build-<hash>.d`, `liblibfoo-<hash>.rmeta` →
   `libfoo-<hash>.d`), and its files are the dep-info lines that end with `:` (rustc's one target
   per input file, which cannot be mistaken for the output line), with `\ ` read as a space and
   `#` lines skipped — probed on 1.99.1: under `cargo check` every workspace artifact, lib, bin
   and test alike, has one `.rmeta` filename and that `.d` beside it — cost if wrong: a cargo
   release that moves dep-info makes the view fail loudly (an error, never a pass).
2. Ruling: cargo's test builds are the test targets: an artifact with `profile.test == true`
   (the `--test` build of a lib or bin, every `test` and `bench` target) is a test target for
   `ac-trace`; rule 1 of `lints` applies to lib, bin and build-script artifacts that are not test
   builds — cost if wrong: a `#[cfg(test)] #[path = "../tests/x.rs"] mod t;` in a lib is
   test-only code outside `src/`, which coverage does not measure anyway.
3. Ruling: the Rust files of a dep-info are those with the extension `.rs`; every other file is
   data — a module file with another extension (`#[path = "m.txt"]`) is a construction any diff
   shows (D6) — cost if wrong: such a file escapes rules 1–3 of `lints` and `ac-trace`.
4. Ruling: dep-info paths are normalised lexically: joined with the root when relative (cargo
   runs rustc from the workspace root), the `\\?\` prefix stripped, `.` and `..` resolved by
   components, `/` as separator, compared without case on Windows, never canonicalised; a file
   outside the root keeps its absolute normal form — cost if wrong: a symlink into `src/` is not
   seen through (D6).
5. Ruling: the view is shared through `view::Shared`, one object per process created in `main`
   and in `gate::run` and passed to the `lints` and `ac-trace` runners, whose signature becomes
   `run(root, ci, shared, out)` (amends ledger ruling R-pre1 for these two) — a process-wide
   static keyed by root would hand stale views to tests that change a fixture between two runs —
   cost if wrong: two runners take one parameter more than the others.
6. Ruling: a macro metavariable in a `cfg`/`cfg_attr` predicate (`#[cfg($p)]` in a `macro_rules!`
   body, or any predicate in macro tokens that `syn::Meta` cannot read because of `$`) is not
   judged; the invocation is judged where it writes `#[cfg(…)]` itself; otherwise
   `unexpected_cfgs` refuses unknown names and the known-but-refused ones are left to the reviews
   (D6) — cost if wrong: `m!(not(clippy))` through a metavariable passes `lints`.
7. Ruling: `cfg!(…)` in expressions is not judged — spec §5.10 rule 2 names `cfg` and `cfg_attr`
   attributes, and the spec puts it out of scope — cost if wrong: none beyond the spec's scope.
8. Ruling: a predicate that is not a bare name, a `key = "value"` pair, or `all`/`any`/`not` over
   those (a literal `true`/`false`, a path with `::`, an unknown list head) is a finding — fail
   closed, and `cfg(false)` would hide code from every check — cost if wrong: a later phase that
   needs such a predicate needs `gate/cfg-names.toml` or the rule changed in a gate PR.
9. Ruling: in macro tokens an attribute whose bracket content `syn::Meta` cannot parse is judged
   by its identifiers: an ident `allow` anywhere in it is a finding, anything else is not judged,
   so `#[$m]` and `$(#[$m:meta])*` pass, as spec §5.10 leaves them to the reviews — cost if wrong:
   an `allow` ident in a non-attribute meaning inside a macro is over-reported.
10. Ruling: `ac-trace` reads TypeScript from `repo_files` (tracked and unignored `.ts` files),
    skipping any path with a `node_modules` or `target` component and any `tests/fixtures/`
    folder — spec §5.4 names only `node_modules/`, but the fixture `.ts` files of this repository
    carry deliberate tags, so without the fixture exclusion `ac-trace` fails here; P11 decides the
    reader — cost if wrong: a TypeScript test under a `tests/fixtures/` folder is not read until
    P11.
11. Ruling: the Rust files `ac-trace` reads are exactly the view: a file nothing compiles is not
    read at all, not even for dangling tags (spec §5.10) — cost if wrong: an orphan `.rs` file
    with a stale tag is noticed only in review.
12. Ruling: probe facts and regression tests — the dep-info facts (`#[path]`, `include!`,
    `include_str!` data, library code under `tests/`, a byte-order mark) get tests over the
    view's fixture workspaces, which run `cargo check` on a tiny offline workspace (about a second
    each); `gate/cfg-names.toml` gets one test against `rustc --print cfg` of the running
    toolchain for the four G2 targets (milliseconds, no target installation needed); the clippy
    `forbid` facts and the llvm-cov facts get no suite test, because each would add a clippy or
    llvm-cov run of minutes; they are re-probed at every toolchain pin change (spec §8) — cost if
    wrong: a clippy behaviour change is noticed at the next pin bump, not before.
13. Ruling: in-process tests build fixture views with `ViewOptions::FIXTURE` (`--offline`, no
    `--locked`), the CLI tests run the real options (`--locked`), so every fixture workspace
    carries a `Cargo.lock` — cost if wrong: a fixture that gains a crate must regenerate its lock.
14. Ruling: `syn` 2 with `full`, `visit` and `extra-traits`, and `proc-macro2` 1 with
    `span-locations` — syn 2 is already in `Cargo.lock` (tracing-attributes) and its API is the
    one this plan names; syn 3, which serde and clap now use, is a later dependency update — cost
    if wrong: one dependency bump in a gate PR.
15. Ruling: `pulldown-cmark = { version = "0.13", default-features = false }` (tables enabled per
    parse), `quick-xml = "0.41"`, `git-conventional = "0.12"`; the exact patch versions are what
    `cargo update -p <crate>` resolves at build time, and the lock pins them — cost if wrong:
    none.
16. Ruling: the final review loop restarts at round 1 for revision 1 — it is new work on a new
    design, and rounds 1–7 reviewed code that revision 1 deletes; their findings and rulings stay
    in the ledger and bind wherever their subject survives — cost if wrong: up to five more
    rounds.
17. Ruling: `required_contexts_from_gate_md` returns the first column of `docs/gate.md` §1 without
    `ui` and no longer reads the OS column (spec §4.2, revision-1 ruling on check names) — cost
    if wrong: none.
18. Ruling: the label `windows` is `1d76db` with the description "Run the Windows jobs of the
    platforms workflow on this pull request"; `macos` keeps `5319e7` with the same wording for
    macOS — spec §11 leaves colours and wording to the builder — cost if wrong: none.
19. Ruling: the `lints` and `ac-trace` jobs need nothing new from the setup action: `cargo check`
    runs with the toolchain and the rust-cache it already provides — cost if wrong: a slow first
    `lints` job while the cache is cold.
20. Ruling: fixture workspaces hold no dependency outside themselves; where spec §9 says "serde
    derives", a std derive (`#[derive(Debug, Clone)]`) stands in — cost if wrong: none.
21. Ruling: the comment block above a test is the run of consecutive `//` line comments (not
    `///` or `//!`, each alone on its line) whose last line is directly above the test's first
    attribute or doc comment; a blank line or a code line ends the run — keeps Part 1's placement
    rule, and resolves the Task 7 deferred minor about a trailing comment after an attribute (a
    comment after `#[test]` is inside the attributes, not above them) — cost if wrong: none.
22. Ruling: `cfg_attr(…, ignore …)` on a test still makes its tag not count, as in Part 1 (the
    over-reach on `cfg_attr(miri, ignore)` stays a deferred minor) — cost if wrong: such a test
    does not prove an AC.
23. Ruling: rule 1 also flags an `include!` written in macro tokens (ident `include` followed by
    `!`), over-reporting a macro that writes the word without expanding it — fail closed — cost if
    wrong: one false finding in an exotic macro.
24. Ruling: when `cargo check` fails, the view's error names the command to run and does not
    echo the diagnostics (they can quote the pull request's code and the runner reads what xtask
    prints); cargo's own stderr is shown as it runs — same choice as `engine_free::metadata` —
    cost if wrong: a reader opens the job log one step up.

**Rulings of Part 1 that revision 1 retires** (they bound the code that Tasks 23–25 delete):

- "lints also forbids compiling fixtures into crates" and "close the fixture-compile class with
  three fail-closed rules (`mod fixtures;`, `main.rs`/`mod.rs` in `tests/fixtures/`, manifest
  paths)" (Task 8): replaced by rule 1 (library, binary and build-script targets compile Rust
  only from their `src/`; test targets are not limited, clippy lints what they compile).
- "lints gets its own file predicate" (Task 8): `lints` reads the view; only rule 4 reads file
  names.
- "final round 3 #1: close the coverage bypass with a lints rule (`#[path]`/`include!` from
  `src/`, `[lib]`/`[[bin]]` paths)": same intent, now rule 1 over dep-info.
- "round 6 fix #2: lints rejects `coverage`/`coverage_nightly` … `#[cfg(not(test))]` stays
  allowed", "final fix round 7 A: the cfg allowlist with `target_*`", and "ruling A's `target_*`
  means the 13 keys in `TARGET_KEYS`": replaced by rule 2 over `gate/cfg-names.toml`, whose keys
  come from `rustc --print cfg` (`not(test)` stays allowed because `test` is listed).
- "final fix round 7 B: every rule reads attributes from the token stream": replaced by `syn`
  and `proc-macro2` tokens.
- "macro-forwarded attributes stay unjudged": now what the spec says (§5.10, "left to the
  reviews"); rule 3 judges the invocation's own `#[…]` tokens (Ruling 9).
- "final #3: a tag in a `.rs` file no build compiles still counts, deferred to a follow-up gate
  PR": built now (Ruling 11).
- Deferred minors that revision 1 resolves: junit.rs hand-rolled XML scanner (Task 19); ac_trace
  mod.rs size and the duplicated scan loop (Task 25); attribute_end and block comments,
  attributes read only at line start (Task 24, modules deleted); a trailing `//` comment after an
  attribute makes a tag dangling (Ruling 21); the real-umbrella tests hard-code phases (Task 17
  rewrites them against the phase table it reads).
- Every other ruling of the ledger still binds: R-pre1 (amended by Ruling 5), R-pre2, R-pre3,
  the alias target directory, the marker-prefixed report format, `repo_files` dropping absent
  paths, `tests/fixtures/` exempt from the config-file rule, `dep:x` for engine deps, the
  coverage cache key, "not checked" notes, `rustup toolchain install`, the approval concurrency
  group, the settings job's `issues: read`, final round 2 #1/#2, final round 4 #1/#2, the legacy
  `rust-toolchain` file rule (spec §5.4 keeps it), and the three revision-1 rulings on the
  `platforms` schedule, labels and single file.

### Review Focus (Part 2)

From spec §9 (revision 1) and the build so far; each line names the task whose tests pin it.

1. **Ordinary code must pass `lints`:** `#[cfg(test)]`, `#[cfg(not(test))]`, `#[cfg(windows)]`,
   `#[cfg(target_os = "…")]`, `cfg_attr(test, derive(…))`, `#[expect(…, reason = "…")]`,
   `#[ignore = "…"]`, std derives, doc comments and the `$(#[$m:meta])*` idiom with ordinary
   attributes. The `clean` fixture holds all of them: Tasks 23 and 24.
2. **Code that leaves coverage:** library code taken from `tests/` or from outside `src/` through
   `#[path]`, `include!` or a manifest `path`; `src/../x.rs` in the llvm-cov export. Tasks 21,
   22 and 23.
3. **Attributes clippy does not see:** an inner `#![allow(…, reason = "…")]` in a one-line module
   and at crate level, `#![cfg_attr(…, allow(…))]`, an `#[allow]` forwarded through a macro
   invocation, `#[expect]`/`#[ignore]` without a reason in a `macro_rules!` body. Task 24.
4. **Tags that are not tags:** a `// AC:` line inside a string, a tag in a file that no target
   compiles, a tag above a test in an inline module, a CRLF source, a trailing comment after an
   attribute. Task 25.
5. **Workflows and settings after D8:** the 13 contexts of the ruleset equal the PR job names and
   `docs/gate.md` §1; every `platforms` job runs on its label's OS only and the other label does
   not start it; no required job has an `if:`. Task 15.

Also pinned: a heading inside a code block or an HTML comment is no section and a CRLF body
passes (Task 18); `BREAKING-CHANGE` vs `BREAKING CHANGE`, CRLF messages and multi-line footers
through git-conventional (Task 20); `cargo check` failing and a file `syn` cannot parse (Task 22).

### File map (Part 2)

```text
.github/workflows/ci.yml                 jobs clippy, test on Linux; names without suffix   Task 15
.github/workflows/platforms.yml          replaces macos.yml: 8 jobs (windows, macos)         Task 15
gate/ruleset-main.json, repo-settings.toml  13 contexts; label windows                        Task 15
xtask/src/github/settings.rs             WINDOWS label; required_contexts (Task 18 rewrites) Tasks 15, 18
xtask/tests/{workflows,settings}.rs      shape tests for D8                                  Task 15
gate/lints.toml, Cargo.toml              four lints at forbid                                Task 16
xtask/src/markdown.rs                    pulldown-cmark: headings, sections, items, tables  Task 17
xtask/src/umbrella.rs                    over markdown.rs                                    Task 17
xtask/src/checks/pr_body.rs              over markdown.rs                                    Task 18
xtask/tests/approval.rs                  gate.md §5 through markdown.rs                      Task 18
xtask/src/checks/ac_trace/junit.rs       quick-xml                                           Task 19
xtask/src/conventional.rs                git-conventional                                    Task 20
xtask/src/checks/coverage/llvm_json.rs   lexical path normalisation                          Task 21
xtask/src/view/{mod,cargo_check,dep_info,source,comments}.rs  the workspace view            Task 22
xtask/src/process.rs                     cargo_metadata moved here; stdout capture           Task 22
xtask/tests/view.rs, tests/fixtures/view/<case>/                                             Task 22
gate/cfg-names.toml                      the cfg names code may test                         Task 23
xtask/src/checks/lints/{mod,config_files,sources,cfg_names}.rs  rules 1, 2, 4; registry      Task 23
xtask/src/checks/lints/attributes.rs     rule 3; deletes attribute_reader, tokens,
                                         fixture_routes, src_bounds                          Task 24
xtask/src/checks/ac_trace/{mod,rust_tags}.rs  over the view; is_fixture/extension → files.rs Task 25
xtask/src/{main,gate}.rs                 Shared wiring (lints in 23, ac-trace in 25)         Tasks 23, 25
xtask/tests/fixtures/{lints,ac-trace}/   workspaces with Cargo.toml and Cargo.lock           Tasks 23–25
```

**Already built by Part 1 — verify, do not rebuild.** Revision 1 took these into the spec from
the build: the alias's own target directory (§5.1, `.cargo/config.toml`), the `settings` job's
`issues: read` (§5.6), the earlier-commit check and the "none recorded" comment of `approval`
(§5.7 steps 4 and 8; ledger rulings of final round 4), `cargo deny --locked check`, the nextest
profiles, the legacy `rust-toolchain` file rule, and the `exit`/`mem_forget` lints. The final
reviewer checks them against the revised text; no task touches them.

---

### Task 15: D8 — Linux per PR, the `platforms` workflow, contexts and labels

Suggested implementer model: mid-tier. ACs: CE-08.1, CE-08.3, CE-08.4, CE-08.12.

**Files:**
- Delete: `.github/workflows/macos.yml`.
- Create: `.github/workflows/platforms.yml`.
- Modify: `.github/workflows/ci.yml`, `gate/ruleset-main.json`, `gate/repo-settings.toml`,
  `xtask/src/github/settings.rs:44-60,174-210` (label constants, `check_labels`),
  `xtask/tests/workflows.rs`, `xtask/tests/settings.rs:57-105,190-210`, root `README.md` and
  `xtask/README.md` (only if a line says PRs run on Windows; today none does).

**Interfaces:**
- Consumes: `required_contexts_from_gate_md` (Task 12; with the revised `docs/gate.md` it already
  returns plain names), `check_of`, `runner_of`, `command_of` in `xtask/tests/workflows.rs`.
- Produces: `settings::WINDOWS: &str = "windows"` beside `MACOS`; `check_labels` requires
  `needs-jann`, `approved`, `windows` and `macos`.

Fixed values:
- `ci.yml`: jobs `fmt`, `clippy`, `test`, `docs`, `deny`, `engine-free`, `coverage`, `lints`,
  `ac-trace`, `commit-msg`, every one `runs-on: ubuntu-latest` with `name:` equal to its check
  (the job keys `clippy-linux`, `clippy-windows`, `test-linux`, `test-windows` become `clippy` and
  `test`). Triggers, permissions, concurrency, timeouts, the coverage cache step and the
  `fetch-depth: 0` checkouts stay as Task 14 built them. The header comment says per-PR checks
  run on Linux (D8).
- `platforms.yml`: `name: platforms`; `on:` `schedule: [{ cron: "0 4 * * 1" }]`,
  `workflow_dispatch:`, `pull_request: { types: [labeled] }`; `permissions: contents: read`; no
  `concurrency`. Eight jobs, keys `clippy-windows`, `test-windows`, `lints-windows`,
  `ac-trace-windows`, `clippy-macos`, `test-macos`, `lints-macos`, `ac-trace-macos`, with
  `name: <check> (windows)` / `<check> (macos)`, `runs-on: windows-latest` / `macos-latest`,
  `timeout-minutes: 60`, job-level `if: github.event_name != 'pull_request' ||
  github.event.label.name == 'windows'` (or `'macos'`), and the three steps checkout, setup,
  `run: cargo xtask <check>`. The header comment: why these four checks run here (`docs/gate.md`
  §1: `lints` and `ac-trace` read what the compiler builds on the machine).
- `gate/ruleset-main.json`: `required_status_checks` holds exactly 13 contexts, each with
  `integration_id: 15368`: `fmt`, `clippy`, `test`, `docs`, `deny`, `engine-free`, `coverage`,
  `lints`, `ac-trace`, `commit-msg`, `pr-title`, `pr-body`, `approval`.
- `gate/repo-settings.toml`: `[[labels]]` `windows` (`1d76db`, "Run the Windows jobs of the
  platforms workflow on this pull request") after `approved`; `macos` keeps `5319e7` and gets
  "Run the macOS jobs of the platforms workflow on this pull request"; the comment above each
  names `platforms.yml`.
- The setup action (`.github/actions/setup/action.yml`) is unchanged (Ruling 19).

- [ ] **Step 1: Write the failing tests** (`xtask/tests/workflows.rs` unless said otherwise).
  `ALL_WORKFLOWS` lists `platforms.yml` instead of `macos.yml`; `check_of`, `runner_of`,
  `command_of` stay.

  | Test | Assertion | Tag |
  |---|---|---|
  | `ci_builds_and_tests_on_linux` (replaces `ci_builds_and_tests_on_linux_and_windows`) | `ci.yml` jobs `clippy` and `test` run on `ubuntu-latest` with `cargo xtask clippy` / `cargo xtask test` | CE-08.1 |
  | `platforms_builds_and_tests_on_windows_and_macos` | `platforms.yml` jobs `clippy (windows)`, `test (windows)` on `windows-latest` and `clippy (macos)`, `test (macos)` on `macos-latest`, each running `cargo xtask <check_of(name)>` | CE-08.1 |
  | `platforms_runs_weekly_on_demand_and_by_label` (replaces `macos_runs_weekly_and_on_demand`) | `on` keys are exactly `schedule`, `workflow_dispatch`, `pull_request`; one cron `0 4 * * 1`; `pull_request.types == ["labeled"]`; the job names are exactly the eight above; every job's `if` is `github.event_name != 'pull_request' \|\| github.event.label.name == '<os>'` where `<os>` is the word in its suffix; every job's `runs-on` is `runner_of(name)` | CE-08.12 |
  | `required_contexts_are_pr_jobs` (unchanged) | passes once the job names and the ruleset agree | CE-08.3 |
  | `every_job_only_runs_its_own_check`, `gate_list_equals_ci_jobs_minus_pr_only` (unchanged) | still pass with `platforms.yml` in `ALL_WORKFLOWS` | CE-08.4 |
  | `no_key_lets_a_job_pass_while_xtask_fails` (Part 1, final round 1) | the job-level `if` is allowed on exactly the eight `platforms.yml` jobs (was: `macos.yml`); every other exception stays as it is | CE-08.4 |
  | `ruleset_contexts_equal_gate_md_checks` (`xtask/tests/settings.rs`) | the expected set becomes the 13 names | CE-08.3 |
  | `repo_settings_encode_section_6` (`xtask/tests/settings.rs`) and the unit tests of `settings.rs` that list the labels | four labels in the order `needs-jann`, `approved`, `windows`, `macos`; a file without `windows` fails `check_labels` | CE-08.3 |

- [ ] **Step 2:** Run `cargo nextest run -p xtask`. Expected: the new and changed tests FAIL (and
  the two that fail on the merged head still do).
- [ ] **Step 3:** Write the workflow, the ruleset, the settings file and the label constant.
- [ ] **Step 4:** Run `cargo nextest run -p xtask`. Expected: PASS, including the two tests that
  failed on the merged head. Run `cargo xtask lints` (the YAML and JSON are not Rust, so nothing
  else is affected). Do **not** run `cargo xtask github apply` (spec §4.2; the renamed contexts
  wait for Jann's go-ahead, "After the tasks").
- [ ] **Step 5:** Commit: `ci(gate): check pull requests on linux and other platforms weekly`
  (`Refs: CE-08.1, CE-08.3, CE-08.4, CE-08.12`).

---

### Task 16: The four guarding lints at `forbid`

Suggested implementer model: mid-tier. ACs: none directly (spec §5.2; the clippy run of every
later task proves the levels hold).

**Files:** Modify `gate/lints.toml`, `Cargo.toml` (`[workspace.lints]`), `xtask/tests/lints.rs`.

**Interfaces:** none new. `lints::workspace_lints_match` (Task 8) keeps the mirror equal.

Fixed values: the list of spec §5.2, byte for byte in both files: `[rust]` gains
`unexpected_cfgs = "forbid"` after `missing_docs`; `[clippy]` has `allow_attributes = "forbid"`,
`allow_attributes_without_reason = "forbid"` and, new, `ignore_without_reason = "forbid"` after
it; every other line as today. The fixture `xtask/tests/fixtures/lints/clean/gate/lints.toml`
mirrors only a subset and needs no change.

- [ ] **Step 1: Write the failing test** `the_gate_forbids_the_lints_that_guard_the_others` in
  `xtask/tests/lints.rs`: parse the real `gate/lints.toml` as `toml::Table` and assert
  `rust.unexpected_cfgs`, `clippy.allow_attributes`, `clippy.allow_attributes_without_reason`
  and `clippy.ignore_without_reason` are each the string `"forbid"`.
- [ ] **Step 2:** Run it. Expected: FAIL.
- [ ] **Step 3:** Edit both files.
- [ ] **Step 4:** Run `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo xtask lints`, `cargo nextest run -p xtask`. Expected: all clean. If clippy 1.99.1 does
  not know `ignore_without_reason` under that name, stop and report (spec §11 lets the builder
  follow a rename, but a rename is a plan change to record, not a silent one). If our own code
  trips one of the four, fix the code (a reason, a declared cfg), never the level.
- [ ] **Step 5:** Commit: `build(gate): forbid the lints that guard the others`.

---

### Task 17: The shared Markdown module and the umbrella over it

Suggested implementer model: mid-tier. ACs: CE-08.6, CE-08.10 (the umbrella's ids feed `ac-trace`
and `commit-msg`; `parses_phases_repos_and_ids` keeps its tag).

**Files:**
- Create: `xtask/src/markdown.rs`.
- Modify: `Cargo.toml` (`[workspace.dependencies]`), `xtask/Cargo.toml`, `xtask/src/lib.rs`
  (`pub mod markdown;`), `xtask/src/umbrella.rs`.

**Interfaces (produces):**
- `markdown.rs`, over `pulldown_cmark::Parser::new_ext(text, Options::ENABLE_TABLES)` with
  `into_offset_iter()`:
  - `pub fn normalise(text: &str) -> Cow<'_, str>`: a leading byte-order mark removed, CRLF read
    as LF. Every function below calls it first.
  - `pub struct Heading { pub level: u8, pub text: String, pub line: u32 }` and
    `pub fn headings(text: &str) -> Vec<Heading>`.
  - `pub struct Section<'a> { pub heading: Heading, pub body: &'a str }` and
    `pub fn sections(text: &'a str, level: u8) -> Vec<Section<'a>>`: one per heading of exactly
    `level`; `body` runs from the end of the heading to the start of the next heading whose level
    is `<= level`, else to the end of the text.
  - `pub struct ListItem { pub text: String, pub line: u32 }` and
    `pub fn list_items(text: &str) -> Vec<ListItem>`: every `Item` at any depth, in document
    order, with its inline text.
  - `pub struct Table { pub line: u32, pub header: Vec<String>, pub rows: Vec<Vec<String>> }` and
    `pub fn tables(text: &str) -> Vec<Table>`.
  - `pub fn code_spans(text: &str) -> Vec<String>`: the inline `Code` events.
  - `pub fn has_content(text: &str) -> bool`: some `Text` or `Code` event has a non-whitespace
    character; `Html` and `InlineHtml` never count.
  - `pub fn line_of(text: &str, offset: usize) -> u32`: 1 plus the LFs before `offset`.
  - "Inline text" everywhere: `Text` and `Code` events concatenated (code without its
    backticks), `SoftBreak` and `HardBreak` as one space, trimmed.
- `umbrella.rs` keeps `PATH`, `PhaseInfo`, `Umbrella`, `load`, `parse` and every error case, and
  reads with `markdown`: the phase table is the first `Table` whose header has `Phase`, `Name` and
  `Repo`; a row counts when its first cell parses as `Phase`; the criteria are the `list_items`
  of each `sections(text, 3)` body whose heading text is `PNN · …`, keeping the items whose text
  starts with `CE-` (id = first whitespace-separated word, checked with `is_ac_id`). `heading`,
  `section_phase`, `table_cells` are deleted.

Fixed values: `[workspace.dependencies]` gains `pulldown-cmark = { version = "0.13",
default-features = false }`; `xtask/Cargo.toml` `pulldown-cmark.workspace = true`.

- [ ] **Step 1: Write the failing unit tests** in `markdown.rs`:
  `headings_come_with_level_text_and_line`; `a_heading_in_a_code_block_or_an_html_comment_is_not_one`
  (a fenced block with `## Gate` and a multi-line `<!-- ## Gate -->`); `a_heading_with_trailing_spaces_or_a_closing_sequence_has_clean_text`
  (`## Gate   ` and `## Gate ##`); `sections_end_at_the_next_heading_of_the_same_or_a_higher_level`
  (`###` sections run over `####` and stop at `##`); `list_items_join_their_lines`
  (`- CE-90.2 The second,\n  which goes on` → one item); `tables_give_cells_as_plain_text`
  (`` | `fmt` | x | Linux | `` → `["fmt", "x", "Linux"]`); `code_spans_are_collected`;
  `content_ignores_html_comments_and_whitespace` (`<!-- x -->\n\n` → false; a code block → true);
  `crlf_and_a_byte_order_mark_are_read_as_plain_text` (same headings and lines as the LF text).

  In `umbrella.rs`, keep every existing test and add `a_criterion_that_spans_two_lines_is_one_id`
  and `a_level_four_heading_stays_inside_its_phase`. Rewrite the tests on the real umbrella so
  they compare against what the phase table lists (every phase of the table has a section or is
  built elsewhere) instead of hard-coding `0..=18` (Task 7 deferred minor).
- [ ] **Step 2:** Add the dependency (`cargo add` resolves the patch version; `cargo xtask deny`
  must pass), then run `cargo nextest run -p xtask`. Expected: the new tests FAIL.
- [ ] **Step 3:** Implement `markdown.rs` and rewrite `umbrella.rs` over it.
- [ ] **Step 4:** Run `cargo nextest run -p xtask`, `cargo xtask ac-trace --phase P00` and
  `cargo xtask commit-msg`. Expected: PASS; `ac-trace` lists CE-08.1–CE-08.12 as before.
- [ ] **Step 5:** Commit: `refactor(xtask): read markdown with pulldown-cmark`
  (`Refs: CE-08.6, CE-08.10`).

---

### Task 18: `pr-body` and the `docs/gate.md` readers over the Markdown module

Suggested implementer model: mid-tier. ACs: CE-08.7, CE-08.3.

**Files:** Modify `xtask/src/checks/pr_body.rs`, `xtask/src/github/settings.rs:575-615`,
`xtask/tests/settings.rs:209-240`, `xtask/tests/approval.rs:575-616` (the `section` and
`backticked` helpers).

**Interfaces:**
- Consumes: `markdown::{normalise, headings, sections, has_content, code_spans, tables}` (Task 17).
- Produces: `pr_body::template_sections`, `check_body`, `read_template`, `run` with unchanged
  signatures; `settings::required_contexts_from_gate_md(text) -> BTreeSet<String>` unchanged in
  signature, now: the first cell of every row of the first table under the level-2 section whose
  heading starts with `1.`, except `ui`, filtered to `[a-z0-9-]` (Ruling 17).

Rules (spec §5.4 `pr-body`): the sections are the level-2 headings of the template; each must be
a level-2 section of the body (heading text equal after inline parsing) whose body
`has_content`. A `null`, empty or whitespace body lacks every section. `sections`,
`strip_comments` and the local `normalise` of `pr_body.rs` are deleted.

- [ ] **Step 1: Write the failing tests.** Keep every existing `pr_body` test (they encode the
  spec's cases) and add, tagged CE-08.7: `a_heading_inside_a_code_block_is_no_section`,
  `a_heading_inside_an_html_comment_is_no_section`,
  `a_heading_with_a_closing_sequence_or_trailing_spaces_matches`,
  `a_section_whose_only_content_is_a_code_block_is_filled`. In `tests/settings.rs` replace
  `required_contexts_skip_ui_and_split_the_os_matrix` by
  `required_contexts_are_the_first_column_of_section_1_without_ui`: the fixture text's OS column
  is `Linux` everywhere, the expected set is `{fmt, clippy, docs}`, a table under `## 2.` is
  ignored, and `"no table here"` gives an empty set. In `tests/approval.rs` the helpers become
  `markdown::sections(&gate_md, 2)` (heading starting `5.`) and `markdown::code_spans(body)`; the
  assertions stay.
- [ ] **Step 2:** Run `cargo nextest run -p xtask`. Expected: the new tests FAIL.
- [ ] **Step 3:** Rewrite the three readers.
- [ ] **Step 4:** Run `cargo nextest run -p xtask`; then `cargo xtask pr-body --body-file
  <a file with PR #7's current description>`. Expected: PASS.
- [ ] **Step 5:** Commit: `refactor(xtask): read pr template and gate tables with pulldown-cmark`
  (`Refs: CE-08.7, CE-08.3`).

---

### Task 19: The JUnit report through `quick-xml`

Suggested implementer model: mid-tier. ACs: none directly (the result column informs; the
`ac-trace` tests of Task 25 exercise it).

**Files:** Modify `xtask/src/checks/ac_trace/junit.rs`, `Cargo.toml`, `xtask/Cargo.toml`.

**Interfaces:** `junit::parse(xml: &str) -> Result<BTreeMap<String, bool>>` unchanged. Inside:
`quick_xml::Reader::from_str(xml)` with `config_mut().trim_text(true)`, reading `Event::Start`,
`Event::Empty`, `Event::End`, `Event::Eof`; the root must be `testsuites` or `testsuite`; a
`testcase` opens a case from its `name` attribute (`try_get_attribute("name")`, unescaped);
`failure` or `error` inside it marks it failed, `skipped` leaves it out, `flakyFailure` and
`rerunFailure` are ignored; the case is recorded at its end (or at once when `Empty`), and a name
seen twice passed only if it passed every time. A `testcase` inside a `testcase`, a case without
a name, and any `quick_xml::Error` are `Error::Parse { what: "the JUnit report", message }` with
the reader's position and never the text.

Fixed values: `quick-xml = "0.41"` in `[workspace.dependencies]`; no features.

- [ ] **Step 1:** Keep every existing test of `junit.rs` and the fixture
  `xtask/tests/fixtures/ac-trace/nextest-junit.xml`; add
  `cdata_comments_and_a_declaration_inside_a_case_do_not_change_its_result` if no existing test
  covers all three, and `an_error_names_the_position_and_not_the_text`.
- [ ] **Step 2:** Add the dependency (`cargo xtask deny` passes), run `cargo nextest run -p xtask
  junit`. Expected: the new tests FAIL.
- [ ] **Step 3:** Rewrite `parse` over the reader; delete the `SKIPPED` scanner.
- [ ] **Step 4:** Run the tests, then `cargo xtask test` followed by `cargo xtask ac-trace --phase
  P00`. Expected: PASS, and the table shows `pass` for the tagged tests.
- [ ] **Step 5:** Commit: `refactor(xtask): read the junit report with quick-xml`.

---

### Task 20: Commit messages and titles through `git-conventional`

Suggested implementer model: mid-tier. ACs: CE-08.10, CE-08.9 (the integration tests of Tasks 4
and 5 keep their tags and must still pass unchanged).

**Files:** Modify `xtask/src/conventional.rs`, `Cargo.toml`, `xtask/Cargo.toml`.

**Interfaces:** `TYPES`, `SCOPES`, `Header`, `Message`, `parse_header`, `parse_message`,
`header_problems`, `wording_problems`, `message_problems` keep their signatures; `commit_msg.rs`
and `pr_title.rs` do not change. Inside, `git_conventional::Commit::parse(text)` reads the
structure: `Header { kind: commit.type_().to_string(), scope: commit.scope().map(ToString::to_string),
breaking: <the text before the first ": " of the first line ends with '!'>, summary:
commit.description().to_owned() }`; `Message { header, body: commit.body() split into lines,
footers: commit.footers() as (token, value) }`. The crate's error becomes our one-sentence
`Err(String)`; the `fixup!`, `squash!` and `Revert "…"` hints stay (they are ours and not a
parser). CRLF is still normalised to LF before parsing. The rules in `header_problems` and
`message_problems` stay exactly as spec §5.4 lists them; the blank-line rule is the crate's.
`split_footer`, `footers_of`, `trim_blank_lines` and the hand-written header reading are deleted.

Fixed values: `git-conventional = "0.12"` in `[workspace.dependencies]`.

- [ ] **Step 1:** Keep every existing unit test of `conventional.rs` and every integration test
  of `tests/commit_msg.rs` and `tests/pr_title.rs`. A case in `headers_with_the_wrong_shape_do_not_parse`
  that git-conventional accepts (its grammar decides the shape, spec §5.4) keeps failing through
  a rule on top of the crate when a §5.4 rule refuses it; when no §5.4 rule refuses it, stop and
  report NEEDS_CONTEXT with the case: a test is never deleted to make the new parser pass (hard
  rule 4), and the controller rules on it. The task reviewer checks that every rule of §5.4 keeps
  a test. Add
  `the_bang_comes_from_the_header_not_from_the_footer` (a message with a `BREAKING CHANGE:`
  footer and no `!` has `breaking == false` on its header and a problem from
  `message_problems`).
- [ ] **Step 2:** Add the dependency (`cargo xtask deny` passes); run `cargo nextest run -p xtask`.
  Expected: the new test FAILS.
- [ ] **Step 3:** Rewrite the two parse functions over the crate.
- [ ] **Step 4:** Run `cargo nextest run -p xtask`, then `cargo xtask commit-msg` on this branch
  and `cargo xtask pr-title --title "feat(gate): foundation and gate (P00)"`. Expected: PASS.
- [ ] **Step 5:** Commit: `refactor(xtask): parse commit messages with git-conventional`
  (`Refs: CE-08.10, CE-08.9`).

---

### Task 21: `coverage` maps normalised paths

Suggested implementer model: mid-tier. ACs: CE-08.8.

**Files:** Modify `xtask/src/checks/coverage/llvm_json.rs:157-190`.

**Interfaces:** `normal(path: &str) -> String` also resolves `.` and `..` components lexically
after turning `\` into `/` (a `..` with nothing before it is kept); `crate_of` is unchanged.

- [ ] **Step 1: Write the failing unit test** `a_file_that_climbs_out_of_src_is_not_in_the_crate`
  (tag CE-08.8): with root `/r`, `/r/crates/theos-content/src/../x.rs` → `None`,
  `/r/xtask/src/../tests/x.rs` → `None`, `/r/crates/theos-content/src/./a/../b.rs` →
  `Some("theos-content")`, `/r/crates/theos-content/src/x.rs` → `Some("theos-content")`.
- [ ] **Step 2:** Run it. Expected: FAIL (`..` is read as a file name today).
- [ ] **Step 3:** Implement with `std::path::Component` over the `/` form.
- [ ] **Step 4:** Run `cargo nextest run -p xtask coverage`. Expected: PASS.
- [ ] **Step 5:** Commit: `fix(xtask): normalise coverage paths before mapping them to crates`
  (`Refs: CE-08.8`).

---

### Task 22: The workspace view

Suggested implementer model: mid-tier. ACs: CE-08.8 (the dep-info facts that rule 1 rests on;
Ruling 12).

**Files:**
- Create: `xtask/src/view/mod.rs`, `xtask/src/view/cargo_check.rs`, `xtask/src/view/dep_info.rs`,
  `xtask/src/view/source.rs`, `xtask/src/view/comments.rs`, `xtask/tests/view.rs`, fixtures
  `xtask/tests/fixtures/view/<case>/` (`plain`, `path-and-include`, `bom`, `does-not-check`), each
  a workspace (`[workspace] resolver = "3", members = ["crates/*"]`, `Cargo.lock`, `README`).
- Modify: `Cargo.toml`, `xtask/Cargo.toml`, `xtask/src/lib.rs` (`pub mod view;`),
  `xtask/src/process.rs` (`cargo_metadata` moved here from `engine_free.rs:177-213`;
  `capture_cargo_showing_stderr`), `xtask/src/checks/engine_free.rs` (calls the moved function).

**Interfaces (produces):**
- `view/mod.rs`:
  - `#[derive(Clone, Copy)] pub struct ViewOptions { pub locked: bool, pub offline: bool }` with
    `pub const REPOSITORY: Self` (`locked: true, offline: false`) and `pub const FIXTURE: Self`
    (`locked: false, offline: true`).
  - `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum TargetKind { Lib, Bin, Test, Bench,
    Example, BuildScript }` (cargo_metadata's `Lib`, `ProcMacro`, `CDyLib`, `DyLib`, `RLib`,
    `StaticLib` → `Lib`; `CustomBuild` → `BuildScript`).
  - `pub struct Artifact { pub package: String, pub manifest_dir: String, pub kind: TargetKind,
    pub name: String, pub src_path: String, pub test: bool, pub files: Vec<String> }`: paths
    repo-relative with `/` (`manifest_dir` is `""` for a root package); `test` is
    `profile.test`; `files` are the dep-info files, sorted, deduplicated.
  - `pub struct View { pub root: PathBuf, pub artifacts: Vec<Artifact>, pub findings:
    Vec<Finding>, sources: BTreeMap<String, Source> }` with `pub fn build(root: &Path, options:
    ViewOptions) -> Result<Self>`, `pub fn source(&self, path: &str) -> Option<&Source>`,
    `pub fn sources(&self) -> impl Iterator<Item = &Source>` (every `.rs` file of every artifact,
    once, in path order) and `pub fn test_files(&self) -> BTreeSet<&str>` (the files of the
    artifacts with `test == true`).
  - `pub struct Shared { options: ViewOptions, cell: OnceCell<View> }` with `pub fn new() -> Self`
    (`REPOSITORY`), `pub fn with(options: ViewOptions) -> Self` and `pub fn view(&self, root:
    &Path) -> Result<&View>`, which builds on the first call and returns the same view after.
- `view/source.rs`:
  - `pub struct Source { pub path: String, pub text: String, pub file: Option<syn::File>,
    pub tokens: Option<proc_macro2::TokenStream> }`; `file` and `tokens` are `None` when the
    text does not parse.
  - `pub fn parse_source(path: &str, bytes: &[u8]) -> (Source, Option<Finding>)`: not UTF-8, or
    `syn::parse_file` / `TokenStream::from_str` failing, gives the finding "this file does not
    parse as Rust (line N); rustc would not compile it either" (N from the error's span; the
    message never quotes the text). `syn::parse_file` skips a leading byte-order mark and a
    shebang line itself; before `TokenStream::from_str`, blank them (replace their bytes with
    spaces of the same byte length), so the tokens' byte ranges and lines still match `text`.
  - `pub fn line_of(span: proc_macro2::Span) -> u32` (`span.start().line`).
- `view/comments.rs`: `pub struct LineComment { pub line: u32, pub text: String, pub alone: bool }`
  (`text` is the whole source line; `alone` when only whitespace precedes `//`) and
  `pub fn line_comments(text: &str, tokens: &TokenStream) -> Vec<LineComment>`: the `//` comments
  (not `///`, `//!`, not block comments) found in the text between the byte ranges of the leaf
  tokens (`Span::byte_range`), so a `//` inside a string or a doc comment is not one.
- `view/dep_info.rs`: `pub fn parse_dep_info(text: &str) -> Vec<String>` (Ruling 1),
  `pub fn repo_relative(root: &Path, file: &str) -> String` (Ruling 4) and
  `pub fn dep_info_of(artifact_file: &Path) -> PathBuf` (Ruling 1).
- `view/cargo_check.rs`: `pub fn check_command(options: ViewOptions) -> CargoCommand`
  (`check --workspace --all-targets --message-format=json`, plus `--locked` and `--offline` when
  set) and `pub fn artifacts(root: &Path, options: ViewOptions) -> Result<Vec<Artifact>>`: runs
  `process::cargo_metadata(root, true, locked, offline)` for the members (package id → name and
  manifest dir), runs the check through `process::capture_cargo_showing_stderr`, parses stdout
  with `cargo_metadata::Message::parse_stream`, keeps `CompilerArtifact` messages of members,
  and reads each one's dep-info. A non-zero exit, or `BuildFinished { success: false }`, is
  `Error::Command` whose `stderr` says "run `cargo check --workspace --all-targets --locked` in
  the repository to read the diagnostics" (Ruling 24); a missing dep-info file is `Error::Io`.
- `process.rs`: `pub(crate) fn cargo_metadata(root, no_deps, locked, offline) ->
  Result<cargo_metadata::Metadata>` (the body of `engine_free::metadata`) and
  `pub fn capture_cargo_showing_stderr(root: &Path, cmd: &CargoCommand) -> Result<Option<String>>`
  (like `capture_cargo`, stderr inherited).

Fixed values: `[workspace.dependencies]` gains `syn = { version = "2.0", features = ["full",
"visit", "extra-traits"] }` and `proc-macro2 = { version = "1.0", features = ["span-locations"] }`.
Fixture `plain`: crate `a` (lib: `src/lib.rs`, `src/sub.rs`, `build.rs`, `tests/it.rs`) and
crate `b` (bin `src/main.rs`); `path-and-include`: crate `a` whose `src/lib.rs` has
`#[path = "../extra/m.rs"] mod m;`, `include!("../extra/inc.rs");` and
`const DATA: &str = include_str!("../data.txt");`; `bom`: crate `a` whose `src/lib.rs` starts
with U+FEFF; `does-not-check`: crate `a` with `fn {` in `src/lib.rs`.

- [ ] **Step 1: Write the failing tests.** Integration tests in `xtask/tests/view.rs` copy a
  fixture to a temporary directory and call `View::build(dir, ViewOptions::FIXTURE)`:

  | Test | Assertion | Tag |
  |---|---|---|
  | `the_view_lists_the_artifacts_of_the_members` (`plain`) | artifacts include (`a`, Lib, test false, files `{crates/a/src/lib.rs, crates/a/src/sub.rs}`), (`a`, Lib, test true), (`b`, Bin, test false, `{crates/b/src/main.rs}`), (`it`, Test, test true, `{crates/a/tests/it.rs}`), (`a`, BuildScript, `{crates/a/build.rs}`); `manifest_dir` is `crates/a` / `crates/b` | – |
  | `dep_info_lists_path_modules_includes_and_data` (`path-and-include`) | the non-test Lib artifact's files are exactly `crates/a/src/lib.rs`, `crates/a/extra/m.rs`, `crates/a/extra/inc.rs`, `crates/a/data.txt` | CE-08.8 |
  | `test_files_are_what_test_builds_compile` (`plain`) | `test_files` contains `crates/a/tests/it.rs` and `crates/a/src/lib.rs`, not `crates/a/build.rs` | – |
  | `a_byte_order_mark_changes_nothing` (`bom`) | the file is in the view and `source(..).file.is_some()`, no findings | – |
  | `a_workspace_that_does_not_check_is_an_error` (`does-not-check`) | `Err(Error::Command { .. })` whose text does not contain `fn {` | – |
  | `shared_builds_the_view_once` (`plain`) | two `view()` calls give the same `*const View`; a file added between them is not in the view | – |

  Unit tests: `dep_info.rs` — `files_are_the_targets_without_dependencies` (a text with the
  `.d:` line, `xtask\src\main.rs:` lines, a `c\ d.rs:` line and a `# env-dep:` line gives
  `["c d.rs", "xtask/src/main.rs"]`), `repo_relative_resolves_dots_and_strips_the_verbatim_prefix`
  (`crates/a/src/../x.rs` → `crates/a/x.rs`; `\\?\<root>\clippy.toml` → `clippy.toml`;
  `/elsewhere/x.rs` → `/elsewhere/x.rs`), `dep_info_path_strips_lib_and_the_extension` (the three
  examples of Ruling 1). `source.rs` — `a_file_that_does_not_parse_is_a_finding_with_its_line`,
  `a_file_that_is_not_utf8_is_a_finding`. `comments.rs` —
  `line_comments_are_the_text_between_tokens` (a `// AC: x` inside a string and a `///` line are
  not listed; a block comment is not; a trailing comment has `alone == false`),
  `crlf_sources_give_the_same_lines_as_lf`.
- [ ] **Step 2:** Add the dependencies (`cargo xtask deny` passes), run `cargo nextest run -p
  xtask`. Expected: the new tests FAIL.
- [ ] **Step 3:** Implement the module. `Shared::view` may use `OnceCell::get` then
  `get_or_init` after a `build`, since `get_or_try_init` is not stable.
- [ ] **Step 4:** Run `cargo nextest run -p xtask`. Expected: PASS. (The repository's own view is
  first built for real by `cargo xtask lints` in Task 23; commit no ignored scratch test.)
- [ ] **Step 5:** Commit: `feat(xtask): build the workspace view from cargo check and syn`
  (`Refs: CE-08.8`).

---

### Task 23: `lints` rules 1, 2 and 4 over the view, the rule registry and `gate/cfg-names.toml`

Suggested implementer model: mid-tier. ACs: CE-08.8 (rules 1 and 2 keep the code that clippy,
`test` and `coverage` see the same; the rule-4 and registry tests prove no AC directly).

**Files:**
- Create: `gate/cfg-names.toml`, `xtask/src/checks/lints/config_files.rs`,
  `xtask/src/checks/lints/sources.rs`, fixtures `xtask/tests/fixtures/lints/{outside-src,
  include-in-src, bad-cfg}/`, `Cargo.lock` and `README` in `xtask/tests/fixtures/lints/clean/`.
- Modify: `xtask/src/checks/lints/mod.rs`, `xtask/src/checks/lints/cfg_names.rs` (rewritten),
  `xtask/tests/lints.rs`, `xtask/src/checks/lints/fixture_routes.rs:1-45` (its manifest helpers
  are still called from `config_files.rs` until Task 24 deletes them), `xtask/src/process.rs`
  (`rustc_program` becomes `pub(crate)`).

**Interfaces:**
- Consumes: `View`, `Shared`, `Source`, `line_of` (Task 22); `repo_files`, `is_fixture` (Part 1).
- Produces (`lints/mod.rs`):
  - `pub struct Inputs<'a> { pub root: &'a Path, pub files: &'a [String], pub view: &'a View,
    pub cfg_names: &'a CfgNames }`.
  - `pub struct Rule { pub name: &'static str, pub run: fn(&Inputs<'_>) -> Result<Vec<Finding>> }`
    and `pub const RULES: &[Rule] = &[sources::RULE, cfg_names::RULE, config_files::RULE]`
    (Task 24 inserts `attributes::RULE` before `config_files::RULE`). A new rule is one module
    and one entry here (D4).
  - `pub fn check(root: &Path, files: &[String], view: &View) -> Result<Verdict>`: the view's
    findings first, then each rule's findings in list order, each rule's sorted by (file, line);
    `pub fn run(root: &Path, ci: Option<&CiEvent>, shared: &Shared, out: &mut dyn Write) ->
    Result<Verdict>` (Ruling 5), which loads `CfgNames`, lists `repo_files` and takes
    `shared.view(root)?`.
  - `config_files.rs` holds what `mod.rs` held: `workspace_lints_match`, `same_text`, `parse`
    (re-exported from `lints` as today), `lint_tables`, `crate_manifests`, `mirror`,
    `stray_config`, behind `pub const RULE` and `pub fn findings(root: &Path, files: &[String])
    -> Result<Vec<Finding>>` (so the config tests need no view).
  - `sources.rs`: `pub const RULE`; for every artifact with `kind` in `{Lib, Bin, BuildScript}`
    and `test == false`, every `.rs` file in `files` must start with `<manifest_dir>/src/`
    (`src/` for a root package) or, for a `BuildScript`, equal its `src_path`; the finding is at
    the file, no line: "compiled into the <kind> `<name>` of `<package>` from outside its
    `src/`, where `coverage` measures library and binary code; move it under `src/`". For those
    files, every `include!` is a finding at its line: in the syntax tree (`syn::visit::Visit`,
    `visit_macro`, `mac.path.is_ident("include")`) and in macro tokens (Ruling 23).
  - `cfg_names.rs`: `pub struct CfgNames { pub names: BTreeSet<String>, pub keys:
    BTreeSet<String> }` with `pub fn load(root: &Path) -> Result<Self>` (`gate/cfg-names.toml`;
    missing or malformed is `Error::Config`); `pub const RULE`; `pub fn judge(predicate:
    &syn::Meta, names: &CfgNames) -> Result<(), String>` (a bare path → ident in `names`; a
    `name = "…"` → ident in `keys`; `all(…)`/`any(…)` → every inner; `not(…)` → exactly one
    inner; anything else refused, Ruling 8; the message repeats the rejected word as ASCII
    letters, digits and `_`, other characters as `?`, at most 40 characters, as Part 1 did).
    The rule walks every `Source` of the view: in the tree, every attribute `cfg(p)` and
    `cfg_attr(p, …)` (nested `cfg`/`cfg_attr` inside the `…` too); in macro tokens, every `#`
    [`!`] bracket group whose content parses as `Meta` with path `cfg` or `cfg_attr`, skipped
    when a `$` token is in the predicate (Ruling 6).

Fixed values, `gate/cfg-names.toml`:

```toml
# The cfg names code may test (P00 spec §5.10, rule 2). `unexpected_cfgs` at `forbid` already
# refuses every name rustc and cargo do not know; this list refuses the known names that differ
# between the builds the checks make (`clippy`, `debug_assertions`, `miri`, …) and any name a
# build script declares. `names` may stand alone, `keys` take a value, which rustc checks.
# `keys` are the target keys the pinned toolchain (rustc 1.99.1) sets itself on stable for the
# G2 targets: the union of `rustc --print cfg --target <t>` for x86_64-pc-windows-msvc,
# x86_64-unknown-linux-gnu, aarch64-apple-darwin and x86_64-apple-darwin, without
# `debug_assertions` and `panic`. Re-run the four commands at every toolchain pin change.
names = ["test", "doc", "unix", "windows"]
keys = [
    "feature",
    "target_abi",
    "target_arch",
    "target_endian",
    "target_env",
    "target_family",
    "target_feature",
    "target_has_atomic",
    "target_has_atomic_primitive_alignment",
    "target_os",
    "target_pointer_width",
    "target_vendor",
]
```

The implementer obtains the keys by running those four `rustc --print cfg --target <t>`
commands with the pinned toolchain (`--print cfg` needs no installed target); the eleven target
keys above are what 1.99.1 printed on 3 Nov 2026 — if the output differs, the output wins and the
commit body says so. The test below keeps the file honest.

- [ ] **Step 1: Write the failing tests.**

  `xtask/tests/lints.rs`: `lint(root)` becomes `lints::check(root, &repo_files(root)?,
  &View::build(root, ViewOptions::FIXTURE)?)`; the config-rule tests
  (`crate_without_workspace_lints_fails`, `nested_clippy_toml_fails`, `rustfmt_toml_fails`,
  `crate_level_cargo_dir_fails`, `mirror_drift_fails`, `lints_table_drift_fails`,
  `a_mirror_with_crlf_line_ends_is_the_same`, `a_missing_file_is_a_finding`,
  `a_manifest_that_is_not_toml_is_a_finding`, `a_legacy_rust_toolchain_file_fails`,
  `rust_toolchain_toml_and_fixture_copies_pass`) call `lints::config_files::findings` directly
  and need no view. New, each copying a fixture:

  | Test | Fixture | Expected | Tag |
  |---|---|---|---|
  | `library_code_outside_src_fails` | `outside-src`: `crates/a/src/lib.rs` with `#[path = "../tests/helper.rs"] mod helper;`, and a `[[bin]] name = "tool", path = "tools/main.rs"` in `crates/a/Cargo.toml` | two findings, at `crates/a/tests/helper.rs` and `crates/a/tools/main.rs`, no line | CE-08.8 |
  | `test_targets_may_compile_anything` | `outside-src` plus `crates/a/tests/it.rs` with `#[path = "fixtures/case/lib.rs"] mod case;` | no finding about `crates/a/tests/fixtures/case/lib.rs` | CE-08.8 |
  | `include_in_library_code_fails` | `include-in-src`: `src/lib.rs` with `include!("part.rs");`, `src/part.rs` present, and `include_str!("../data.txt")` | one finding at the `include!` line; none for `include_str!` | CE-08.8 |
  | `a_build_script_compiles_from_build_rs` | `plain` of the view fixtures, copied over `clean` | no finding | CE-08.8 |
  | `refused_cfg_names_fail_with_their_line` | `bad-cfg`: a lib with `#[cfg(not(clippy))]`, `#[cfg(debug_assertions)]`, `#[cfg(miri)]`, `#[cfg(coverage)]`, `#[cfg(docsrs)]`, `#[cfg(target_has_atomic_equal_alignment = "8")]`, `#[cfg(false)]`, `#[cfg_attr(not(clippy), allow(dead_code))]` (the cfg_attr predicate), and `macro_rules! m { () => { #[cfg(not(clippy))] fn g() {} } }` | one finding per line, in that order, each naming the word | CE-08.8 |
  | `clean_repo_passes` (extended) | `clean` gains: `#[cfg(not(test))]`, `#[cfg(windows)]`, `#[cfg(target_os = "macos")]`, `#[cfg(all(unix, not(target_env = "musl")))]`, `#[cfg(feature = "extra")]` with `extra = []` declared, `#[cfg_attr(test, derive(Debug))]` on a struct with `#[derive(Clone)]`, a `macro_rules! m { ($(#[$m:meta])* $v:vis fn $n:ident()) => { $(#[$m])* $v fn $n() {} } }` with `m! { #[inline] fn h() }`, and `macro_rules! p { ($p:meta) => { #[cfg($p)] fn k() {} } }` with `p!(windows);` | pass | – |
  | `every_rule_is_registered_once` (unit, `lints/mod.rs`) | `RULES` names are unique and equal `["sources", "cfg-names", "config-files"]` (Task 24 adds `attributes`) | – |
  | `cfg_names_toml_is_what_the_pinned_rustc_prints` (`xtask/tests/lints.rs`) | for each of the four G2 targets run `<rustc_program> --print cfg --target <t>`; the union of the keys (the part before `=`) minus `debug_assertions`, `panic`, `unix`, `windows`, plus `feature`, equals `CfgNames::load(repo root)?.keys`; `names` equals `{test, doc, unix, windows}` | CE-08.8 |
  | `the_command_passes_a_clean_repository_and_fails_a_dirty_one` (existing CLI test) | still runs `cargo xtask lints --root <copy>` with the repository options, which is why `clean` carries `Cargo.lock` (Ruling 13); the dirty case stays `bad-attributes` until Task 24 changes it | – |

  Unit tests in `cfg_names.rs` over `parse_source` text: `ordinary_predicates_pass`,
  `a_literal_or_a_path_with_segments_is_refused`, `nested_cfg_attr_and_combinators_are_judged`,
  `a_metavariable_in_a_predicate_is_not_judged`, `a_name_in_a_string_or_a_comment_is_not_judged`,
  `the_echoed_word_is_ascii_and_short` (`testé` → `test?`).
- [ ] **Step 2:** Run `cargo nextest run -p xtask`. Expected: the new tests FAIL.
- [ ] **Step 3:** Write `gate/cfg-names.toml`, `config_files.rs` (moved code), `sources.rs`,
  the new `cfg_names.rs`, the registry in `mod.rs`; remove the call of the old `scan_cfg_names`
  from the old `sources()` loop (the loop and the other old scans stay until Task 24). Wire
  `lints::run` in `main.rs` and `gate.rs` with a `Shared::new()` created in `main`'s `run` and in
  `gate::run` (`gate::run_check` takes `shared: &Shared`); `ac_trace::run` keeps its old
  signature until Task 25.
- [ ] **Step 4:** Run `cargo nextest run -p xtask`, then `cargo xtask lints` on the repository
  (it now runs `cargo check` first) and `cargo xtask gate`. Expected: PASS everywhere; the
  repository has no file outside `src/` in a lib, bin or build script and no refused cfg name.
- [ ] **Step 5:** Commit: `feat(xtask): lint sources and cfg names over the workspace view`
  (`Refs: CE-08.8`).

---

### Task 24: `lints` rule 3 over the syntax tree and macro tokens; the old scanners go

Suggested implementer model: mid-tier. ACs: none directly (spec §5.4 `lints`, §5.10 rule 3).

**Files:**
- Rewrite: `xtask/src/checks/lints/attributes.rs`.
- Delete: `xtask/src/checks/lints/attribute_reader.rs`, `tokens.rs`, `fixture_routes.rs`,
  `src_bounds.rs`; fixtures `xtask/tests/fixtures/lints/{fixture-compiled-through-path,
  manifest-path-into-fixtures, coverage-cfg}/`.
- Modify: `xtask/src/checks/lints/mod.rs` (the old `sources()` loop, `is_source` and the
  `pub use` of the old scans go; `attributes::RULE` joins `RULES`; the module doc is rewritten
  for the four rules), `xtask/src/checks/lints/config_files.rs` (the manifest-path-into-fixtures
  and `src_bounds` manifest rules go), `xtask/tests/fixtures/lints/bad-attributes/crates/a/src/lib.rs`,
  `xtask/tests/lints.rs`, `xtask/src/checks/ac_trace/rust_tags.rs:216-332` (`attribute_end` and
  `literal_end` lose their last users outside `rust_tags`; make them private again — Task 25
  deletes them).

**Interfaces:**
- Consumes: `Inputs`, `Rule`, `RULES` (Task 23); `Source`, `line_of` (Task 22).
- Produces: `attributes::RULE` (name `attributes`) and `pub fn findings(source: &Source) ->
  Vec<Finding>`. In the tree (`syn::visit::Visit::visit_attribute`, inner and outer, at every
  depth): an attribute whose path is `allow` is a finding at its line, with or without a reason;
  `cfg_attr(p, a, b, …)` is parsed with `parse_args_with(Punctuated::<Meta,
  Token![,]>::parse_terminated)` and each of `a, b, …` is judged the same way, recursively. In
  macro tokens (`visit_macro`, recursing into every group): each `#` [`!`] bracket group is
  parsed with `syn::parse2::<Meta>`; on success, the same judgement, plus a finding for
  `expect(…)` without a `reason = …` entry among its nested metas and for a bare `ignore`
  (`ignore = "…"` passes); on failure, Ruling 9. Messages are fixed sentences: "an `allow`
  attribute: use `#[expect(…, reason = \"…\")]`, or fix the code", "an `#[expect]` without a
  reason, inside a macro where clippy does not see it", "an `#[ignore]` without a reason, inside
  a macro where clippy does not see it".

Fixed values, `bad-attributes/crates/a/src/lib.rs` (its module doc lists the lines):
`#![allow(missing_docs)]`; `#[allow(dead_code)] fn silenced() {}`;
`#[cfg_attr(test, allow(unused))] fn cfg_attr_allow() {}`;
`mod m { #![allow(unused, reason = "in one line") ] }`;
`macro_rules! q { () => { #[expect(dead_code)] fn a() {} #[test] #[ignore] fn b() {} } }`;
`macro_rules! fwd { ($(#[$m:meta])* fn $n:ident()) => { $(#[$m])* fn $n() {} } } fwd! { #[allow(dead_code)] fn c() }`;
and, to pass, `#[expect(dead_code, reason = "kept")] fn kept() {}` and
`#[test] #[ignore = "slow"] fn slow() {}` — six findings, none on the last two lines.

- [ ] **Step 1: Write the failing tests.** Unit tests in `attributes.rs` over `parse_source` text:
  `outer_and_inner_allow_are_findings_with_their_line`, `allow_inside_cfg_attr_is_a_finding`,
  `an_inner_allow_with_a_reason_in_a_one_line_module_is_a_finding`,
  `allow_in_a_macro_invocation_and_in_a_macro_rules_body_are_findings`,
  `expect_and_ignore_without_reason_in_macro_tokens_are_findings_and_with_a_reason_pass`,
  `expect_and_ignore_in_the_tree_are_left_to_clippy`, `a_forwarded_attribute_is_not_judged`
  (`$(#[$m:meta])*` and `#[$m]`), `derives_doc_comments_and_cfg_attr_derive_pass`,
  `text_in_strings_and_comments_is_not_an_attribute`. In `tests/lints.rs`:
  `bad_attributes_in_sources_fail` asserts the six places above;
  `every_rule_is_registered_once` expects `["sources", "cfg-names", "attributes",
  "config-files"]`. Remove, naming each in the commit body with the test that replaces it:
  `fixture_compiled_through_path_fails`, `manifest_path_into_fixtures_fails`,
  `a_main_rs_or_mod_rs_directly_in_fixtures_fails`, `a_mod_fixtures_declaration_fails`,
  `source_in_a_folder_named_target_or_node_modules_is_scanned` (replaced by rule 1 and
  `test_targets_may_compile_anything`), and the `src_bounds` tests
  `code_reached_from_src_but_outside_it_fails`, `code_reached_from_src_inside_it_passes`,
  `xtask_source_leaving_src_fails`, `manifest_target_outside_src_fails`,
  `manifest_target_inside_src_or_not_measured_passes` (replaced by `library_code_outside_src_fails`,
  `test_targets_may_compile_anything` and `include_in_library_code_fails`, Task 23).
- [ ] **Step 2:** Run `cargo nextest run -p xtask`. Expected: the new tests FAIL.
- [ ] **Step 3:** Rewrite `attributes.rs`, delete the four modules and the fixtures, register the
  rule, rewrite the module doc of `lints/mod.rs` (the four rules, what the view is, what is left
  to the reviews).
- [ ] **Step 4:** Run `cargo nextest run -p xtask`, `cargo xtask lints`, `cargo xtask gate` on
  Windows. Expected: PASS. `cargo xtask coverage` must still show `xtask` at or above 80 %.
- [ ] **Step 5:** Commit: `feat(xtask): lint attributes over the syntax tree and macro tokens`.

---

### Task 25: `ac-trace` over the view

Suggested implementer model: mid-tier. ACs: CE-08.6.

**Files:**
- Rewrite: `xtask/src/checks/ac_trace/rust_tags.rs`.
- Modify: `xtask/src/checks/ac_trace/mod.rs` (`run`, `run_phase`, `scan_files`, `is_scanned`),
  `xtask/src/files.rs` (`is_fixture`, `extension` move here as `pub fn`), `xtask/src/main.rs`,
  `xtask/src/gate.rs`, `xtask/tests/ac_trace.rs`, fixtures `xtask/tests/fixtures/ac-trace/<case>/`
  (each gains `Cargo.toml` — package `fixture`, lib — `Cargo.lock` and `README`; new cases
  `uncompiled-file`, `tag-in-inline-module`, `string-looks-like-a-tag`).

**Interfaces:**
- Consumes: `Shared`, `View`, `Source`, `line_of`, `line_comments` (Task 22); `Tag`, `classify`,
  `malformed`, `dangling`, `TARGET_WORDS` (Part 1).
- Produces:
  - `ac_trace::run(root: &Path, ci: Option<&CiEvent>, shared: &Shared, out: &mut dyn Write) ->
    Result<Verdict>` and `run_phase(root, phase: Option<Phase>, ci: bool, shared: &Shared, out)
    -> Result<Verdict>` (Ruling 5); `main.rs` and `gate.rs` pass the `Shared` of Task 23.
  - `rust_tags::scan(source: &Source, in_test_target: bool) -> (Vec<Tag>, Vec<Finding>)`: the
    tests are the `ItemFn`s found by `syn::visit::Visit` at every depth (inline modules, fn
    bodies) with an attribute whose path's last segment is `test`, `rstest` or `test_case`
    (`#[tokio::test]` is `<path>::test`); the tag block of a test is the run of consecutive
    `LineComment`s with `alone == true` ending at the line before the test's first attribute or
    doc comment (Ruling 21); each line of the block goes through `classify`: a `Tag` becomes a
    `Tag` value, `Malformed` a finding; `counts` is false when an attribute's path is `ignore`,
    when a `cfg_attr` attribute's tokens contain the ident `ignore` (Ruling 22), when a `cfg`
    predicate names an ident outside `TARGET_WORDS`, or when it is `any()`; every other line
    comment that `classify` does not call `Other` is a malformed or dangling finding; when
    `in_test_target` is false, every tag found is a dangling finding "in a file that no test
    target compiles" instead (Ruling 11). `attribute_end` and `literal_end` are deleted.
  - `ac_trace::scan_files(root, view) -> Result<(Vec<Tag>, Vec<Finding>)>`: the view's findings
    (files that do not parse) are returned as findings; Rust tags come from `view.sources()`
    with `in_test_target = view.test_files().contains(path)`; TypeScript tags come from
    `repo_files(root)` filtered by `is_typescript(path)` (`.ts`, no `node_modules` or `target`
    component, not `is_fixture`, Ruling 10) through the unchanged `ts_tags::scan`.

- [ ] **Step 1: Write the failing tests.** Rewrite the unit tests of `rust_tags.rs` over
  `parse_source` text, keeping every name and expectation of Part 1
  (`rust_tag_above_test_counts`, `rust_tag_with_doc_and_attrs_counts`,
  `the_attribute_and_the_function_may_share_a_line`, `crlf_line_ends_do_not_matter`,
  `test_attributes_are_recognised_by_their_last_path_segment`, `rust_tag_above_helper_is_dangling`,
  `rust_tag_with_blank_line_is_dangling`, `ignored_test_does_not_count`,
  `cfg_any_test_does_not_count_cfg_windows_does`, `malformed_tags_are_findings`,
  `comments_that_are_not_tags_are_left_alone`, `every_tag_of_a_file_is_found_with_its_line`), and
  add `a_tag_line_inside_a_string_is_not_a_tag`, `a_tag_above_a_test_in_an_inline_module_counts`,
  `a_tag_above_a_test_in_a_function_body_counts`,
  `a_trailing_comment_after_an_attribute_keeps_the_tag` (`// AC: x` / `#[test] // slow` / `fn`),
  `tags_in_a_file_no_test_target_compiles_are_dangling`. In `ac_trace/mod.rs` replace
  `scanned_files_are_rust_and_typescript_outside_fixtures_and_builds` by
  `typescript_files_are_tracked_outside_fixtures_node_modules_and_target`.

  Integration tests (`xtask/tests/ac_trace.rs`; `trace` builds `Shared::with(ViewOptions::FIXTURE)`
  and calls `run_phase`), each tagged CE-08.6, the six of Task 7 kept and:

  | Test | Fixture | Expected |
  |---|---|---|
  | `a_tag_in_a_file_no_target_compiles_does_not_count` | `uncompiled-file`: `src/lib.rs` without tags, `src/orphan.rs` with `// AC: CE-90.1` above a `#[test]`, no `mod orphan;` | fail: CE-90.1 has no counting test; no dangling finding |
  | `a_tag_above_a_test_in_an_inline_module_counts` | `tag-in-inline-module`: the tag inside `#[cfg(test)] mod tests { … }` | pass |
  | `a_tag_inside_a_string_is_not_a_tag` | `string-looks-like-a-tag`: `const S: &str = "\n// AC: CE-99.9\n";` and a real tag for CE-90.1 | pass (no unknown-id finding) |
  | `the_command_line_takes_the_phase` (existing) | `complete-phase`, run through the binary with the repository options | pass (the fixture carries `Cargo.lock`) |

- [ ] **Step 2:** Run `cargo nextest run -p xtask`. Expected: the new tests FAIL.
- [ ] **Step 3:** Rewrite `rust_tags.rs` and `scan_files`, move `is_fixture` and `extension`,
  wire `Shared` into `ac_trace::run` in `main.rs` and `gate.rs`.
- [ ] **Step 4:** Run `cargo nextest run -p xtask`, then `cargo xtask ac-trace --phase P00` and
  `cargo xtask gate` on Windows. Expected: PASS; `ac-trace` lists CE-08.1–CE-08.12 with at least
  one counting test each, and the table shows the view's files only (no fixture paths); the gate
  prints `==> lints` building the view and `==> ac-trace` reusing it (no second `cargo check`).
- [ ] **Step 5:** Commit: `feat(xtask): trace acceptance criteria over the workspace view`
  (`Refs: CE-08.6`).

---

## After the tasks (theos-phase steps 4–6, not implementer tasks)

1. Final review loop as `CLAUDE.md` defines it, restarted at round 1 for revision 1 (Ruling 16):
   a fresh whole-branch reviewer on the most capable model each round, with the review package,
   the revised spec, the umbrella constraints and ACs, the ledger's deferred and parked lines and
   the list of every earlier finding with its resolution or ruling (rounds 1–7 of Part 1
   included, marked as reviewing deleted code where they do); one fix subagent per round with the
   complete list; stop when a round is clean; after 5 rounds without a clean review, stop and
   list the open findings in the PR.
2. `cargo xtask gate` on Windows, output read, and `cargo xtask ac-trace --phase P00`; record
   the table in the ledger.
3. Push. Update PR #7's description from the template: every section, the AC table, the final
   review summary, every ruling of the ledger (Part 1's and Part 2's), the gate summary, and the
   protected paths it changes.
   - Claude adds the labels `windows` and `macos` once each for one `platforms` run per OS and
     links both runs in the description (spec §6, CE-08.1 and CE-08.12 evidence).
   - The ruleset on GitHub still requires the Part 1 contexts (`clippy (linux)`, …), so the PR
     shows as blocked until the contexts are renamed: Claude asks Jann for the go-ahead to run
     `cargo xtask github apply` (renamed contexts, the `windows` label; spec §4.2), runs it only
     after that, then posts the `cargo xtask github check` output in the PR.
   - Set `docs/status.md` (P00 row, the PR #7 entry under "Needs Jann", the `platforms` runs).
   - Jann adds `approved` only when Claude reports the branch final. No auto-merge: the PR
     touches protected paths.
