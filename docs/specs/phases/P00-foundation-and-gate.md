# P00 · Foundation and gate — phase spec

Status: waiting for Jann's approval (spec PR) · 31 Oct 2026
Phase: P00 · Repo: theos-editor · Depends on: – · Slice: S2
Branches: `spec/P00-foundation-and-gate` (this spec), `phase/P00-foundation-and-gate` (build)
Sources: `docs/specs/00-umbrella.md` (P00, G1–G18), `docs/gate.md`, `docs/architecture.md` §3,
`CONTRIBUTING.md`, `docs/release.md`, `CLAUDE.md`.

## 1. Goal

P00 builds the repo skeleton and the gate of `docs/gate.md`. Every later phase lands through
this gate, so the gate has to be trustworthy and must not get in the way. Code PRs merge on their
own when the gate is green, and PRs that touch protected paths wait for Jann. Claude works under
Jann's GitHub account, so GitHub cannot tell the two apart. The gate therefore makes the right
thing automatic and the wrong thing visible. It cannot make the wrong thing impossible: that part
rests on the rules in `CLAUDE.md`.

After P00:

- the workspace holds every crate of G4 (empty and documented), and it builds and tests on
  Linux and Windows in CI, plus macOS weekly;
- every check of `docs/gate.md` §1 except `ui` runs on every PR and is required on `main`;
- `cargo xtask gate` gives the same verdicts on a developer machine (Windows or Linux) as CI;
- the GitHub ruleset, repo settings and labels are kept as files in `gate/` and can be checked
  against the live repo.

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-08 Workspace, CI and gate**
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

## 3. Scope

**Goals**

- One implementation of every check, in Rust, tested offline, run in the same way by CI and by
  developers.
- Findings that show up where people look: on the PR's diff lines and in the run summary.
- A gate that has no quiet way around it: no check that can be skipped by editing an unprotected
  file, and no required check that silently never reports.

**Non-goals**

- Any content logic (records, grammars, index): P02 onwards.
- The release pipeline, `CHANGELOG.md`, `release-plz.toml`, `cliff.toml`: P05.
- The `ui` check and anything in `editor-ui/`: P11.
- `cargo xtask generator-check`: P09. The xtask command structure leaves room for it.
- Tauri in `theos-editor`: P11. In P00 it is a plain binary.

**Anti-goals** (it would be a failure even if every AC passed)

- Check logic written in workflow YAML, or a second implementation that has to be kept in step
  with xtask.
- A required check that only reports for some PRs (path filters, skipped jobs), which blocks those
  PRs forever.
- A check whose verdict depends on the machine: tool versions, line endings, path separators,
  locale.
- An approved protected PR that can merge changes Jann has not seen.

## 4. Decisions

### 4.1 Decided with Jann (brainstorming, 31 Oct 2026)

- **D1 · Thin CI over xtask.** GitHub Actions runs everything; xtask decides every verdict. Each
  CI job is: check out, shared setup, `cargo xtask <check>`. Workflows hold triggers, the OS
  matrix, caching and permissions, and nothing else. `cargo xtask gate` runs the same
  subcommands locally.
- **D2 · Approval follows content.** When Jann adds `approved`, CI records a fingerprint of the
  PR's changes. A later push that leaves the fingerprint unchanged (a rebase onto `main`, or a
  merge from `main`, that changes nothing in the PR's own diff) keeps the approval. A push that
  changes the fingerprint makes CI remove `approved`, add `needs-jann` and leave a comment
  saying why. CI never adds `approved`.
- **D3 · `approval` runs `main`'s gate.** The approval job checks out the PR's base commit (the
  tip of `main`) and runs that commit's xtask with that commit's `gate/protected-paths.toml`
  against the PR's changes. A PR that changes the approval code or the protected-path list is
  judged by the gate in force until it merges, and a PR that breaks the xtask build still gets
  `needs-jann`. One exception: when the base commit has no xtask (only the P00 build PR), the job
  runs the PR's own. This guards against accidents, not intent: a PR can still edit
  `approval.yml` itself (§8).

### 4.2 Claude's calls

Claude showed these to Jann during brainstorming and they were not objected to. They are still
Claude's calls, and Jann can overturn any of them in this review.

- Ruling: GitHub settings are a **ruleset** on the default branch (not classic branch protection),
  stored in `gate/ruleset-main.json` with no bypass actors and required checks pinned to the
  GitHub Actions app — rulesets are readable with the normal CI token (so drift can be checked
  weekly), and pinning to the app means a status posted through the API cannot satisfy a check —
  cost if wrong: switching to classic protection later rewrites `xtask github` and one file.
- Ruling: `cargo xtask github apply` changes live repo settings only after Jann says so —
  changing protection is security-sensitive (`CLAUDE.md`, "When to stop") — cost if wrong: one
  extra question per change.
- Ruling: required check names are `clippy (linux)`, `clippy (windows)`, `test (linux)`,
  `test (windows)`; every other check uses its name from `docs/gate.md` §1 — GitHub names matrix
  jobs this way, and the name stays readable — cost if wrong: renaming means a ruleset update.
- Ruling: the coverage baseline of `main` is measured at the merge base in the same run and cached
  per commit and toolchain — the local and CI verdicts are then the same by construction, with no
  artifacts that expire — cost if wrong: slower coverage runs when the cache is cold.
- Ruling: crates with no instrumented lines pass `coverage`; each binary gets a smoke test that
  runs it — P00's crates are empty by definition — cost if wrong: none, because the floors apply
  as soon as code arrives.
- Ruling: `xtask` gets the library floor (80 %) in `gate/thresholds.toml`; every workspace member
  must be listed there, and an unlisted crate fails `coverage` — xtask is the gate, so it needs
  tests, and a new crate must not escape the floors — cost if wrong: Jann can lower it in this
  review; after P00, lowering it is a protected change.
- Ruling: `commit-msg` enforces everything in `CONTRIBUTING.md` §2 that a machine can check (§5.4)
  and skips merge commits — "Update branch" creates merge commits on PR branches, and commitlint
  skips them too — cost if wrong: an unconventional merge message could pass, but squash merging
  keeps it off `main`.
- Ruling: Claude's trailers (`Co-Authored-By`, `Claude-Session`) are not enforced — the gate
  cannot tell who wrote a commit — cost if wrong: a missing trailer goes unnoticed.
- Ruling: `ac-trace` takes the phase from the branch name (`phase/PNN-…`), and `pr-title` checks
  that the title's suffix matches the branch — the branch of an open PR cannot change, and it is
  available locally and in CI alike — cost if wrong: none known.
- Ruling: a tagged test marked `#[ignore]` (or `.skip` in TypeScript) does not count as proof — it
  never runs in CI — cost if wrong: none.
- Ruling: `ac-trace` already scans TypeScript test files — P11 then needs no gate change — cost
  if wrong: a little unused code until P11.
- Ruling: engine crates are recognised by where they come from (the theos-engine git repo, or a
  path outside this workspace), plus a name list in `gate/engine-crates.toml` that skips our own
  workspace members — every engine crate is named `theos-*`, and the engine has a crate called
  `theos-editor` too — cost if wrong: a renamed or moved engine repo needs a list update.
- Ruling: `theos-editor` gets an empty feature `engine` now — `engine-free` then checks a real
  feature — cost if wrong: none (CE-32.3 in P11 needs it anyway).
- Ruling: `lints` also requires `clippy.toml`, `rustfmt.toml` and `.config/nextest.toml` to
  equal their copies in `gate/`, forbids those files (and `.cargo/`) anywhere else, and requires
  `lints.workspace = true` in every crate — otherwise these unprotected files could weaken
  clippy, rustfmt or the test run without touching a protected path — cost if wrong: one extra
  copy per file.
- Ruling: `deny` allows MIT, MIT-0, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause,
  BSD-3-Clause, ISC, Zlib, Unicode-3.0, BSL-1.0, CC0-1.0, CDLA-Permissive-2.0 and MPL-2.0, and
  nothing else (so no GPL family). Crates come from crates.io only, and git only from
  theos-engine. Our crates are `publish = false` — MPL-2.0 arrives with Tauri in P11 — cost if
  wrong: a later phase needs a gate PR to add a licence.
- Ruling: tool versions are pinned in `gate/tools.toml`, and CI installs exactly those;
  `cargo xtask gate` stops when a local version differs and prints the install command —
  otherwise the same code could give different verdicts — cost if wrong: developers update tools
  when the pin moves.
- Ruling: actions are pinned by full commit SHA; no Dependabot in P00 — the gate's own supply
  chain should not move silently, and Dependabot PRs would fail `pr-body` — cost if wrong: action
  updates are manual gate PRs.
- Ruling: the workspace version starts at `0.0.0` — the first real version comes from the release
  pipeline (0.1.0 at S2) — cost if wrong: none.
- Ruling: the toolchain pin is the newest stable at build time, and `rust-version` equals it —
  G3 asks for stable and pinned — cost if wrong: none.
- Ruling: the D2 fingerprint is the SHA-256 of `git diff --raw --no-renames --no-abbrev
  <merge-base> <head>` (the modes and blob ids on both sides of every changed path), not
  `git patch-id` as said in brainstorming — `patch-id` ignores whitespace by default, so a
  whitespace-only change after approval would keep it; the raw diff catches every byte, every mode
  and every binary — cost if wrong: approval also resets when `main` changed a file that the PR
  changes too, which is arguably correct.
- Ruling: PRs from `fix/<slug>` branches end their title with the phase whose code they fix
  `(PNN)`, or with `(gate)` / `(docs)` — `CONTRIBUTING.md` §3 defines no suffix for fixes outside
  a phase — cost if wrong: a docs PR to `CONTRIBUTING.md` that settles it (listed in the spec PR).
- Ruling: a second ruleset `gate/ruleset-tags.json` protects `refs/tags/v*` against update and
  deletion, with no bypass, from P00 on. Restricting tag *creation* waits for P05, where the
  release pipeline's own identity (a GitHub App) becomes the only bypass actor — `docs/gate.md`
  §6 says nobody moves or deletes a tag, and CE-08.3 asks the settings to match §6, but blocking
  creation now would block the release pipeline, which uses the same account — cost if wrong: a
  tag created by hand before P05 goes unnoticed (it still cannot be moved or deleted).
- Ruling: `macos.yml` also runs on a PR when the label `macos` is added — GitHub can dispatch a
  workflow only once it is on the default branch, so without this the build PR could not show a
  macOS run before Jann merges it, and later phases with OS-specific code (P09 process trees, P18
  bundles) get macOS on demand — cost if wrong: one more label, and macOS minutes when it is
  used.

## 5. Design

### 5.1 Workspace skeleton

```text
Cargo.toml              workspace root (members crates/*, xtask; resolver 3)
Cargo.lock              committed (G3)
rust-toolchain.toml     pinned stable; components rustfmt, clippy, llvm-tools-preview
.cargo/config.toml      [alias] xtask = "run --locked --package xtask --"
.gitattributes          * text=auto eol=lf  (LF on Windows checkouts too)
clippy.toml             copy of gate/clippy.toml
.config/nextest.toml    copy of gate/nextest.toml
deny.toml
crates/theos-content/   lib   ┐
crates/theos-vcs/       lib   │ each: Cargo.toml, README, src/lib.rs or src/main.rs
crates/theos-studio/    lib   │ with crate docs stating its role (architecture §3);
crates/theos/           bin   │ binaries exit 0 and have a smoke test
crates/theos-mcp/       bin   │
crates/theos-editor/    bin   ┘ feature `engine = []`, documented as reserved
xtask/                  lib + bin: the gate
gate/                   the gate's data (§5.2)
.github/                workflows and the shared setup action (§5.6)
```

- `[workspace.package]`: `version = "0.0.0"`, `edition = "2024"`, `rust-version` = the pin,
  `publish = false`, `repository`. Every crate inherits these with `*.workspace = true`
  (CE-08.11).
- `[workspace.lints]` is a copy of `gate/lints.toml`. Every crate has `[lints] workspace = true`.
- `[workspace.dependencies]` holds every shared dependency version; crates use
  `dep.workspace = true`.
- The root README gets the build commands and the tool setup (`cargo xtask tools`); it is not a
  protected path.
- `.gitattributes` is needed so that rustfmt, the mirror comparisons and the fingerprint never see
  CRLF on Windows.

### 5.2 `gate/` — the gate's data

| File | Holds | Used by |
|---|---|---|
| `lints.toml` | The final lint list (below); mirrored into `[workspace.lints]` | `lints` |
| `clippy.toml` | The test exemptions for the denied lints (`allow-unwrap-in-tests` and the like) | `lints` (mirror) |
| `nextest.toml` | Nextest profiles: no retries, no fail-fast in CI, JUnit output | `lints` (mirror), `test`, `coverage` |
| `thresholds.toml` | Coverage floors per crate class, the class of every workspace member, the largest allowed drop | `coverage` |
| `tools.toml` | Exact versions of cargo-nextest, cargo-deny and cargo-llvm-cov | setup action, `tools`, `gate` |
| `protected-paths.toml` | `docs/gate.md` §5 as globs with exceptions, plus the `[workspace.lints]` table rule | `approval` |
| `engine-crates.toml` | Engine sources and names (§5.4 `engine-free`) | `engine-free` |
| `ruleset-main.json` | The ruleset for the default branch, in the GitHub API format | `github` |
| `ruleset-tags.json` | The ruleset for `refs/tags/v*` (no update, no deletion) | `github` |
| `repo-settings.toml` | Merge settings and labels | `github` |

**Final lint list** (`docs/gate.md` §2 asks P00 to write it):

```toml
[rust]
unsafe_code = "forbid"
missing_docs = "deny"
rust_2018_idioms = { level = "warn", priority = -1 }

[rustdoc]
broken_intra_doc_links = "deny"

[clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
print_stdout = "deny"
print_stderr = "deny"
allow_attributes = "deny"
allow_attributes_without_reason = "deny"
exit = "deny"
mem_forget = "deny"
```

`gate/clippy.toml` turns on clippy's test exemptions for `unwrap_used`, `expect_used`, `panic`,
`print_*` and `dbg_macro`, which is what "denied outside tests" in §2 means. A binary that prints
from `main` carries `#[expect(clippy::print_stdout, reason = "…")]` there. Output elsewhere goes
through `std::io::Write`.

### 5.3 xtask

- **Lib + bin.** `xtask/src/lib.rs` holds every check as a module with a pure core: inputs in, a
  `Verdict` (pass/fail plus findings with file, line and message) out. `main.rs` only parses
  arguments and wires up the real git, cargo and GitHub. Library errors use `thiserror`; `main`
  may use `anyhow` (G16).
- **Commands.** `cargo xtask <check>` for `fmt`, `clippy`, `test`, `docs`, `deny`, `engine-free`,
  `coverage`, `lints`, `ac-trace`, `commit-msg`, `pr-title`, `pr-body`, `approval`. Also
  `cargo xtask gate`, `cargo xtask github check|apply`, and `cargo xtask tools [--install]`. Room
  for `generator-check` (P09) and `ui` (P11).
- **Processes.** git, cargo and gh run through `std::process::Command` with argument lists, never
  through a shell (G6; Windows). Paths go through `std::path`; reports show paths relative to
  the repo with `/`.
- **GitHub.** Behind a small trait (labels, comments, ruleset, repo settings). The real adapter
  calls `gh api`, and the tests use a fake. Only `approval`, `github` and nothing else talk to
  GitHub.
- **CI context.** When `GITHUB_ACTIONS=true`, xtask reads the event payload from
  `GITHUB_EVENT_PATH`: PR title, body, number, base and head commits, and the head branch. It
  writes findings as workflow annotations (`::error file=…,line=…::…`) and a verdict table to
  `GITHUB_STEP_SUMMARY`. Workflow `run:` steps therefore contain no `${{ }}` expressions, so no
  PR text can be injected into a shell. Locally the same commands take flags (`--title`,
  `--body-file`, `--base`, `--head`, `--branch`) for testing by hand.

### 5.4 The checks

| Check | Verdict |
|---|---|
| `fmt` | `cargo fmt --all --check`. |
| `clippy` | `cargo clippy --workspace --all-targets --locked -- -D warnings`. |
| `test` | `cargo nextest run --workspace --locked --profile ci` (CI) / `default` (local), then `cargo test --doc --workspace --locked`. |
| `docs` | `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS=-D warnings` set on the child process. |
| `deny` | `cargo deny --locked check` (licences, advisories, bans, sources) with `deny.toml`. Advisory ignores need a reason. |
| `engine-free` | Fails when either level finds an engine crate. **Declared:** every dependency of every workspace member (normal, dev, build, target-specific, renamed) that is an engine crate must be optional, and enabled only by the feature `engine` (`dep:x`, `x`, `x/…`, `x?/…`), and `default` must not reach `engine`. **Resolved:** `cargo metadata --locked` with default features contains no engine crate. An engine crate is a package from a source matching `gate/engine-crates.toml` (the theos-engine repo, case-insensitive), a path package outside the workspace root, or a package named in the list that is not a workspace member. |
| `coverage` | Measures `cargo llvm-cov nextest --workspace --locked` per crate (lines in `crates/<name>/src/**` and `xtask/src/**`; `tests/` and fixtures excluded). Fails when a crate is below the floor of its class, when a workspace member has no class, or when a crate present on the baseline drops by more than the allowed amount. The baseline is the merge base with `main`, measured in a temporary worktree and cached by commit, toolchain and llvm-cov version (`target/xtask/coverage/` locally, `actions/cache` in CI). A crate with no instrumented lines passes and is reported as such. |
| `lints` | `[workspace.lints]` equals `gate/lints.toml` (compared as parsed TOML). Every crate has `lints.workspace = true`. Root `clippy.toml` and `.config/nextest.toml` equal their `gate/` copies. No `clippy.toml`, `.clippy.toml`, `rustfmt.toml`, `.rustfmt.toml` or `.cargo/` exists anywhere else. Scanning every `.rs` file outside `**/tests/fixtures/**` and `target/`: no `allow(` in any attribute (`#[allow]`, `#![allow]`, inside `cfg_attr`); every `expect(` attribute has `reason = "…"`; every `#[ignore]` has a reason (`#[ignore = "…"]`). |
| `ac-trace` | Reads the ACs from `docs/specs/00-umbrella.md`: ids matching `CE-\d{2}[A-Z]?\.\d+` in list items under each `### PNN ·` heading. Finds tags: a `// AC: <id>[, <id>]*` comment followed only by attributes or doc comments up to a test (`#[test]`, `#[<path>::test]`, `#[rstest]`, `#[test_case(…)]`) in Rust, or `test(`, `it(` (and `.each`) in `*.test.ts`, `*.spec.ts`. Scans all tracked `.rs`/`.ts` files except `**/tests/fixtures/**`, `target/` and `node_modules/`. **Fails** when: an AC of the branch's phase has no counting test; a tag names an id not in the umbrella; a comment starting with `// AC` is malformed or not directly above a test. A test does not count when it is `#[ignore]`d or `.skip`ped, or carries a `#[cfg(…)]` that names anything besides the target OS. Without a phase branch (`main`, `spec/`, `gate/`, `docs/`, `fix/`), only the id and placement rules apply. It prints AC → tests, and adds a pass/fail column when nextest JUnit output exists. |
| `commit-msg` | Commits `base..head` (CI: from the event; locally: merge base with `origin/main` .. `HEAD`), skipping commits with two or more parents. Each message must have: a header `type(scope)!: summary` with a type and scope from `CONTRIBUTING.md` §2 (scope optional); a summary of at most 72 characters that does not start with an upper-case letter and does not end with `.`; a blank line before the body; body lines of at most 100 characters (a line with no spaces, such as a URL, is exempt); `!` if and only if there is a `BREAKING CHANGE:` (or `BREAKING-CHANGE:`) footer; `Phase: PNN` equal to the branch's phase on `phase/` branches; `Refs:` naming only ACs of the umbrella. CRLF in messages is read as LF. |
| `pr-title` | A Conventional Commit header (same parser) whose summary ends with ` (PNN)`, ` (gate)` or ` (docs)`, or a release title `chore(release): vX.Y.Z` / `vX.Y.Z-rc.N`. It must match the branch: `phase/PNN-…` → `(PNN)`; `spec/PNN-…` → `docs(spec): … (PNN)`; `gate/…` → `(gate)`; `docs/…` → `(docs)`; `fix/…` → `(PNN)`, `(gate)` or `(docs)`; `release-plz-…` → release title. Any other branch name fails, naming `CONTRIBUTING.md` §1. |
| `pr-body` | The sections are the `## ` headings of `.github/pull_request_template.md` at the checked-out commit. Each must appear in the body, in any order, with content once HTML comments and whitespace are removed. A missing body counts as empty. CRLF is read as LF. |
| `approval` | §5.7. |

### 5.5 `cargo xtask gate`

`cargo xtask gate` runs `tools` (the version check), then `fmt`, `lints`, `engine-free`,
`ac-trace`, `commit-msg`, `clippy`, `docs`, `deny`, `test` and `coverage`. It runs every check
even after a failure, prints the verdict table, and exits non-zero if anything failed. It
calls the same functions as the CI jobs; nothing in it is specific to running locally except
where the inputs come from (git instead of the event payload). It leaves out `pr-title`,
`pr-body` and `approval`, as `docs/gate.md` §1 says.

### 5.6 Workflows

| File | Triggers | Jobs (= check names) |
|---|---|---|
| `ci.yml` | `pull_request` (opened, synchronize, reopened), `push` to `main`, `workflow_dispatch` | `fmt`, `clippy (linux)`, `clippy (windows)`, `test (linux)`, `test (windows)`, `docs`, `deny`, `engine-free`, `coverage`, `lints`, `ac-trace`, `commit-msg` |
| `pr.yml` | `pull_request` (opened, edited, synchronize, reopened) | `pr-title`, `pr-body` |
| `approval.yml` | `pull_request` (opened, edited, synchronize, reopened, labeled, unlabeled) | `approval` |
| `macos.yml` | `schedule` (weekly, Monday 04:00 UTC), `workflow_dispatch`, `pull_request` (labeled, only for the label `macos`) | `clippy (macos)`, `test (macos)` (not required) |
| `settings.yml` | `schedule` (weekly), `workflow_dispatch` | `settings` (`cargo xtask github check`) |

Every job is the same three steps: `actions/checkout` (full history where commits or the merge
base are needed), `./.github/actions/setup`, and `cargo xtask <the job's check>`. The `approval`
job differs only in its checkout (D3). It checks out the base commit with full history, so the
PR's head commits are present as git objects. A second checkout step of the PR head runs only
when the base commit has no `xtask/Cargo.toml`.

The setup action installs the toolchain from `rust-toolchain.toml`, restores `Swatinem/rust-cache`,
and installs the tools at the versions from `gate/tools.toml` (prebuilt binaries).

Rules for every workflow, checked by a test:

- every job's `run:` step is exactly `cargo xtask <its check name>`, and it has no other `run:`
  steps;
- the `approval` job checks out the base commit, plus the P00-only fallback step and nothing
  else;
- no `paths`, `paths-ignore` or branch filters on `pull_request`, and no job-level `if:` on
  required jobs, so every required check reports on every PR;
- no `pull_request_target`;
- the top-level `permissions: contents: read`; only `approval` adds `pull-requests: write` and
  `issues: write`;
- actions are pinned by full SHA with the version as a comment;
- `concurrency` cancels outdated runs for `ci.yml` and `pr.yml`; `approval.yml` queues and never
  cancels, so label changes are never cut off halfway.

`push` to `main` runs `ci.yml` so the coverage baseline cache and the build caches exist for the
next PRs. Scheduled workflows run on `main`.

### 5.7 Approval end to end (CE-08.5, D2)

The job runs `cargo xtask approval` from the base commit (D3) with `GH_TOKEN` set to the
workflow token. xtask does all of the following; the YAML only provides the token. It reads the
PR's side only as git objects (`git diff`, `git show <head>:Cargo.toml`). It never builds,
runs or reads configuration from the PR's working tree.

1. **Changed paths:** `git diff --name-only --no-renames <merge-base> <head>`. A rename counts as
   both the old and the new path, so moving a file out of `docs/` still counts as touching
   `docs/`. Matching against `gate/protected-paths.toml` ignores case, so `DOCS/x.md` is
   protected too (on Windows it is the same folder).
2. **Lints table:** the PR is also protected when the parsed `[workspace.lints]` of the root
   `Cargo.toml` differs between the merge base and head.
3. **Not protected** → pass.
4. **Protected** → ensure the label `needs-jann` exists and is on the PR (create the label if it
   is missing). Read the PR's current labels from the API (not from the event, which may be
   stale on re-runs).
5. **Fingerprint:** compute it (§4.2). When the event is `labeled` with `approved`, record it as
   a new comment by the workflow: a marker line `<!-- theos-gate:approval fingerprint=<hex>
   head=<sha> -->` plus one human sentence.
6. **Recorded fingerprint:** the newest comment with the marker that was written by
   `github-actions[bot]` and never edited (`created_at == updated_at`). Comments by anyone else
   are ignored, because they could be written by the shared account.
7. **Verdict:**
   - without `approved`: fail with "waits for Jann: add the label `approved` after review";
   - with `approved` and a recorded fingerprint equal to the current one: pass;
   - with `approved` but a different fingerprint, or none recorded: remove `approved`, keep
     `needs-jann`, comment "approval removed: the PR changed after approval (old → new)", and
     fail.

Label and comment changes made with the workflow token start no new workflow runs, so there is no
loop. `needs-jann` is never removed by CI either: it marks the PR as Jann's for good. The decision
logic (steps 3–7) is a pure function over (protected?, event, labels, recorded and current
fingerprint) returning a verdict and a list of actions. The tests drive it through the fake
GitHub.

### 5.8 GitHub settings as code (CE-08.3)

- **`gate/ruleset-main.json`:** target `~DEFAULT_BRANCH`, enforcement `active`, `bypass_actors:
  []` (administrators included). Rules:
  - `deletion` and `non_fast_forward`;
  - `required_linear_history`;
  - `pull_request` with 0 required approvals and `allowed_merge_methods: ["squash"]`;
  - `required_status_checks` with `strict_required_status_checks_policy: true`, every context of
    §5.6's `ci.yml`, `pr.yml` and `approval.yml` jobs, each with the GitHub Actions app id
    (15368).
- **`gate/repo-settings.toml`:**
  - merge settings: squash only (merge commits and rebase off), `squash_merge_commit_title =
    PR_TITLE`, `squash_merge_commit_message = COMMIT_MESSAGES` (so `Refs:`, `Phase:` and Claude
    trailers reach `main`), `allow_auto_merge = true`, `delete_branch_on_merge = true`,
    `allow_update_branch = true`;
  - labels `needs-jann`, `approved` and `macos` with colours and descriptions.
- **`gate/ruleset-tags.json`:** target `refs/tags/v*`, enforcement `active`, `bypass_actors: []`,
  rules `update` and `deletion`.
- **`cargo xtask github check`** compares the live rulesets (by name), the repo settings and the
  labels with the files, prints every difference and fails on any. In `settings.yml` it runs with
  the workflow token. Fields that token cannot read are reported as "not checked" rather than
  failing, and Jann's local run is the authoritative one.
- **`cargo xtask github apply`** creates or updates both rulesets, patches the settings and
  creates or updates the labels. It is idempotent. Claude runs it only after Jann agrees.

### 5.9 Tools

`gate/tools.toml` pins cargo-nextest, cargo-deny and cargo-llvm-cov (current releases at build
time). `cargo xtask tools` checks the installed versions; `--install` runs `cargo install
--locked <tool> --version <v>` for each mismatch. CI installs the same versions as prebuilt
binaries. `cargo xtask gate` fails early, with the install command, when a version differs.

## 6. How each AC is proven

Every test below carries `// AC: <id>`. The evidence column lists what the build PR shows in
addition.

| AC | Tagged tests | Evidence in the build PR |
|---|---|---|
| CE-08.1 | The workspace members equal the G4 set at `crates/<name>` and `xtask/`, each with a README and crate docs. Smoke tests run `theos`, `theos-mcp` and `theos-editor` and expect exit 0. | `clippy` and `test` green on Linux and Windows |
| CE-08.2 | `engine-free` on fixture workspaces: a non-optional engine dependency, one behind another feature, one reached from `default`, a renamed one, and a dev/build one all fail; a dependency behind `engine` only passes. The `ci.yml` job exists. | — |
| CE-08.3 | The contexts derived from `docs/gate.md` §1 (minus `ui`, expanded by OS) equal the ruleset's required contexts and the job names of the PR workflows. The rulesets and repo-settings files encode §6 (squash only, linear, strict, no force push or deletion, no bypass, branch deletion, auto-merge, `v*` tags neither moved nor deleted). | `cargo xtask github check` output after `apply` |
| CE-08.4 | The check list of `gate` equals the CI jobs minus `pr-title`, `pr-body` and `approval`. Every CI job runs `cargo xtask <its name>`. Runs in both test legs (Linux and Windows). | `cargo xtask gate` output on Windows, and CI on Linux, for the same commit |
| CE-08.5 | Approval decisions through the fake GitHub: protected without the label fails and adds `needs-jann`; with the label and a matching fingerprint passes; a changed fingerprint removes `approved` and fails; not protected passes; a change to the lints table only is protected; a rename out of `docs/` is protected; a comment not by the bot or edited is ignored. The protected-path list and the lints table are read from the base commit and from git objects, so a PR that edits `gate/protected-paths.toml` is still judged by the base's list. `approval.yml` checks out the base commit (D3). | The build PR itself: `approval` red with `needs-jann` until Jann adds `approved` |
| CE-08.6 | `ac-trace` fixtures: a phase AC without a tag fails; an unknown id fails; an ignored test does not count; a dangling tag fails; a passing set passes. | `cargo xtask ac-trace --phase P00` lists CE-08.1–CE-08.12 |
| CE-08.7 | `pr-body`: a missing section fails, a placeholder-only section fails, a full body passes, a CRLF body passes. | — |
| CE-08.8 | Coverage verdicts over per-crate numbers: below floor fails; a drop over 1.0 point fails; within passes; an unlisted crate fails; an empty crate passes. Parsing of a llvm-cov JSON fixture. | — |
| CE-08.9 | `pr-title`: non-conventional, a missing suffix, a suffix not matching the branch, and an unknown branch fail; phase, spec, gate, docs and release titles pass. | — |
| CE-08.10 | `commit-msg` over temporary git repos made in the test: a bad header, an unknown type or scope, an over-long summary, `!` without a footer, and a missing `Phase:` on a phase branch fail; a merge commit is skipped; a good range passes. | — |
| CE-08.11 | `cargo metadata`: every member's version equals `[workspace.package]`, and every manifest uses `version.workspace = true`. | — |
| CE-08.12 | `macos.yml` has a weekly `schedule` and `workflow_dispatch` and runs `cargo xtask clippy` and `cargo xtask test` on `macos-latest`. | A run on the build PR started with the label `macos`, linked in the PR; the first `workflow_dispatch` run after merge, linked in `docs/status.md` |

The tests need no network. Tests that make git repos call the `git` command line in a temporary
directory.

## 7. Interfaces other phases use

- **Every phase:** branch names decide the phase for `ac-trace` and `pr-title`. Tests carry
  `// AC: <id>`. `cargo xtask gate` before claiming done; `cargo xtask ac-trace --phase PNN`
  for the AC table.
- **New crates (G4 is fixed, but any later crate):** `version.workspace = true`,
  `lints.workspace = true`, dependencies from `[workspace.dependencies]`, and an entry in
  `gate/thresholds.toml`. That entry is a protected change, so a new crate needs Jann.
- **P05 (release):**
  - release-plz must run with a GitHub App or personal token: PRs and pushes made with the
    workflow token start no workflows, so a release PR would never get its required checks;
  - the release PR's body must contain the template sections (release-plz `pr_body` template),
    its title `chore(release): vX.Y.Z[-rc.N]`, and its branch `release-plz-…`;
  - `release.yml` reuses `cargo xtask test` on all three OS;
  - P05 adds a `creation` rule to `gate/ruleset-tags.json`, with the release identity as its only
    bypass actor.
- **P09:** adds `cargo xtask generator-check` as one more xtask command (not a required check
  unless a gate PR says so).
- **P10:** `ac-trace` requires tagged tests for CE-06.1–CE-06.3 on `phase/P10-…`. Tests count
  wherever they are, but only CI-run tests prove anything. The P10 spec must say how its tests
  run in CI.
- **P11:** adds the `ui` check as jobs `ui (linux)` and `ui (windows)` calling `cargo xtask ui`,
  in a gate PR that also updates the ruleset. `ac-trace` already reads TypeScript tests. Linux CI
  then needs the WebKitGTK packages for Tauri in the setup action. MPL-2.0 is already allowed.
- **Engine part (later):** engine crates come in as optional dependencies behind `engine` only.
  The engine has its own crate named `theos-editor`; two packages with that name in one tree work
  in Cargo but confuse tooling and reports.

## 8. Risks

- **Strict "up to date" stalls auto-merge.** When `main` moves, every open PR is behind and
  GitHub does not update it by itself. An update made with the workflow token would not start the
  checks. Sessions rebase their own PRs; otherwise Jann clicks "Update branch". With D2 this keeps
  approvals intact.
- **Coverage may differ by OS.** CI measures on Linux. A local run on Windows measures the
  Windows build, so `#[cfg(windows)]` code can move a crate's number. This is reported as a
  known difference, and the CI verdict counts.
- **Advisories change daily.** `deny` can turn red on `main` without a code change. That is fixed
  quickly, never by an ignore without a reason.
- **Workflow token limits.** The weekly `settings` run may not be able to read every repo setting
  with the workflow token. It then checks the ruleset and labels, and Jann's local
  `github check` covers the rest.
- **Plan support.** Rulesets on a private repo of a personal account need GitHub Pro. The API
  answered "branch not protected" (not "upgrade"), which suggests it is available; `apply` will
  show for sure.
- **Shared identity.** Everything the gate does can be undone by the account that Claude uses.
  Pinning checks to the Actions app, trusting only bot comments, and running `main`'s approval
  code stop accidents and shortcuts, not intent. For `pull_request` events GitHub runs the PR's
  own `approval.yml`, so a PR could change which code the job runs. The rest is `CLAUDE.md`.
- **Gate changes take effect after merge.** Under D3, a gate PR cannot show its new approval
  behaviour on itself. Its tests prove it, and the next PR uses it.
- **First gated PR.** The build PR is the first to run the workflows. If a required context name
  is wrong, the PR cannot merge until it is fixed. Hence the rollout order in §10.
- **Runner minutes.** Windows jobs cost double and macOS ten times on a private repo; caching
  and the weekly macOS schedule keep this bounded.

## 9. Review Focus — inputs most likely to break it

- **approval:**
  - renames and deletions of protected files;
  - case variants (`DOCS/`, `Claude.md`);
  - a change only inside `[workspace.lints]`, a reformatted table (same content: not a change),
    and a moved table;
  - whitespace-only, mode-only and binary changes after approval;
  - a rebase with conflict resolution, a merge from `main`, and a base branch change (`edited`);
  - a forged or edited fingerprint comment;
  - the label added while the workflow was failing;
  - re-runs of old runs;
  - a PR that edits `gate/protected-paths.toml`, the approval code, or breaks the xtask build
    (D3: still protected, still `needs-jann`);
  - the P00 fallback triggering when the base does have an xtask;
  - anything read from the PR's working tree instead of git objects.
- **ac-trace:**
  - a tag above a helper that is not a test;
  - a blank line between tag and test;
  - several ids, extra spaces, `//AC:` and `// AC -` variants;
  - `#[ignore]`, `#[cfg(any())]` and `#[cfg(windows)]` tests;
  - `it.skip`;
  - ids with a letter (`CE-08R.1`);
  - an id listed under another phase;
  - tags inside fixtures.
- **commit-msg:**
  - merge, `fixup!`, `squash!` and git's own `Revert "…"` messages;
  - `BREAKING CHANGE` vs `BREAKING-CHANGE`;
  - multi-line footers;
  - CRLF messages from Windows;
  - a summary of exactly 72 characters;
  - unicode summaries;
  - a missing scope;
  - `!` in the wrong place.
- **pr-title:** `-rc.N` releases, a title that doesn't match the branch, and unknown branch
  prefixes.
- **pr-body:**
  - a body with only the template comments;
  - headings with trailing spaces;
  - CRLF from the GitHub web editor;
  - sections in a different order;
  - extra sections;
  - a `null` body.
- **coverage:**
  - an empty crate;
  - a crate new in the PR;
  - a crate absent on the baseline;
  - a stale cache from another toolchain;
  - a branch behind `main`.
- **engine-free:**
  - `dep:` vs implicit features;
  - weak features (`x?/f`);
  - renamed dependencies (`package = …`);
  - target-specific, dev and build dependencies;
  - `default` enabling `engine` through another feature;
  - the engine's `theos-editor` vs ours.
- **lints:**
  - `#![allow]`;
  - `cfg_attr(…, allow(…))`;
  - `allow` inside `macro_rules!`;
  - `#[expect]` without a reason;
  - `#[ignore]` without a reason;
  - a crate-level `clippy.toml`;
  - `.cargo/config.toml` in a crate folder;
  - fixture files with deliberate violations being compiled through `#[path]` or `include!`.
- **Workflows:**
  - a required job that does not run on some event;
  - matrix names that don't match the ruleset contexts;
  - `${{ }}` in a `run:` step.
- **Windows:**
  - backslashes in reports;
  - CRLF checkouts;
  - long paths in temporary worktrees;
  - `git` and `gh` found through `PATH` without a shell.

## 10. Rollout of the build PR

1. The build PR from `phase/P00-foundation-and-gate` runs the new workflows for the first time.
   `main` has no xtask yet, so `approval` uses its fallback and runs the PR's own (D3).
   `approval` fails and adds `needs-jann`. It must, because the PR touches `.github/**`,
   `gate/**`, `xtask/**`, `.cargo/**`, `deny.toml`, `rust-toolchain.toml` and the lints table.
   Under the umbrella's P00 entry, this is the one implementation PR that changes protected
   paths. It changes no protected document (`docs/**`, `CLAUDE.md`, `CONTRIBUTING.md`,
   `.claude/**`, `design/**`); a doc change found necessary goes in its own docs PR.
2. When every other check is green, Claude adds the label `macos` to run the macOS jobs once, and
   asks Jann for the go-ahead to run `cargo xtask github apply`. It then puts the
   `cargo xtask github check` output and the macOS run into the PR.
3. Jann reviews and adds `approved`. `approval` turns green, and Jann merges.
4. PRs opened before the merge (P10, docs) must rebase onto the new `main` to get their checks.

## 11. Left to the builder

- Exact crate choices for xtask: argument parsing, TOML, JSON, a maintained YAML parser for the
  workflow tests (one that passes `deny`), and globbing.
- Exact keys of `deny.toml`, `gate/nextest.toml` and the clippy test-exemption options for the
  pinned versions.
- Lint names that the pinned toolchain renamed (keep the intent of §5.2).
- Colours and descriptions of the labels, wording of messages, and the layout of the summary
  table.
- How the setup action passes tool versions to the installer.
