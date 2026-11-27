# The gate

Status: accepted with the umbrella spec; built in phase P00. Binding. Changing this file needs
Jann's approval.

The gate decides what may reach `main`. Code PRs merge on their own when every check is green.
PRs that touch protected paths wait for Jann. Claude works under Jann's GitHub account, so GitHub
cannot tell the two apart: the rules in `CLAUDE.md` that Claude never approves, never weakens
the gate and never merges a waiting PR are part of the gate.

## 1. Required checks

The required status checks on `main` are six CI jobs: `gate`, `test`, `coverage`, `pr`,
`approval` and `ui` (branch protection, administrators included). Each job runs the
checks of the Job column below through `cargo xtask`. A check that fails does not stop the others
of its job, and the job is red when any of its checks is. The job summary of a run is the table of
verdicts, one row per check: passed, failed with its findings, or not needed with the reason. A
check that is not needed counts as passed and is not run, and the job still reports, so a required
check is never missing. When every check runs for a reason other than a change to code (a push, a
manual or scheduled run, a `phase/` branch, a label, changed paths that cannot be read or are
none, lists that cannot be read, a detached head), the summary says why above the table.

| Check | Job | What it runs | OS |
|---|---|---|---|
| `fmt` | `gate` | `cargo fmt --all --check` | Linux |
| `clippy` | `gate` | `cargo clippy --workspace --all-targets --locked -- -D warnings`, and the same with `--features theos-editor/engine` | Linux |
| `test` | `test` | `cargo nextest run --workspace --locked` (profile `ci` in CI, `default` locally; both leave out the slow tests), the same with `--features theos-editor/engine` and `-E 'package(theos-editor)'`, and `cargo test --doc --workspace --locked` | Linux |
| `slow-tests` | `gate` | `cargo xtask slow-tests`: `cargo nextest run --workspace --locked --profile slow --no-tests=warn`, only the slow tests; on a pull request only when it can change their outcome (below) | Linux |
| `docs` | `gate` | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` | Linux |
| `deny` | `gate` | `cargo deny --locked check` (licences, advisories, bans, sources) | Linux |
| `engine-free` | `gate` | `cargo xtask engine-free`: no engine crate in the tree without feature `engine` | Linux |
| `coverage` | `coverage` | `cargo xtask coverage`: per-crate line coverage ≥ floor in `gate/thresholds.toml`, and no crate drops more than the allowed amount below `main` | Linux |
| `lints` | `gate` | `cargo xtask lints`: `[workspace.lints]` in the root `Cargo.toml` equals `gate/lints.toml`; the config files equal their `gate/` copies and exist nowhere else; library and binary code comes only from its crate's `src/`; `cfg` names only what `gate/cfg-names.toml` allows, and no `cfg` is an empty `any()`; no `allow` or `warn` attribute anywhere, outer or inner; no `allow` or `expect` of a guard lint or of `clippy::pedantic`/`clippy::restriction` (§2); no reasonless `#[expect]` or `#[ignore]` anywhere, macro input and code behind another OS's `cfg` included (P00 spec §5.10). | Linux |
| `ac-trace` | `gate` | `cargo xtask ac-trace`: every acceptance criterion of the PR's phase has at least one test tagged `AC: <id>`; no tag names an unknown criterion | Linux |
| `pr-body` | `pr` | `cargo xtask pr-body`: the PR description has every section of the template, filled | Linux |
| `pr-title` | `pr` | `cargo xtask pr-title`: the title is a Conventional Commit ending with `(PNN)`, `(gate)`, `(docs)` or a release title (`CONTRIBUTING.md` §3) | Linux |
| `commit-msg` | `gate` | `cargo xtask commit-msg`: every commit of the PR is a Conventional Commit (`CONTRIBUTING.md` §2) | Linux |
| `approval` | `approval` | fails while the PR touches a protected path and lacks the label `approved` | Linux |
| `ui` | `ui` | the versions of Node 24 and of pnpm (the `packageManager` pin), then `pnpm install --frozen-lockfile`, `svelte-check`, `eslint`, `vitest`, Playwright tests | Linux |

**Linux packages.** The Tauri crates need system libraries to build. `gate/linux-packages.txt`
lists them, one per line. The jobs `gate`, `test` and `coverage` install them with
`apt-get install --no-install-recommends`, through the input `gui-packages` of the setup action.
The downloaded `.deb` files are cached, keyed by the runner image and the list, so a run installs
them without downloading. The setup action asks `cargo xtask needs` first, which decides as the
gate does: a pull request that changes no code installs nothing, and nothing is installed until
`Cargo.lock` names `webkit2gtk`, the package that links the libraries. Only a job that saves its
build cache, on `main`, saves the cached files.

**Slow tests** are tests that take minutes, such as the map generator's parity and golden runs
(P08 spec §5.14). A test is slow when a segment of its name, the path nextest shows, starts with
`slow_`: `maps::parity::slow_recipes_match`, or every test in a module `slow_runs`. The nextest
profiles in `gate/nextest.toml` split the suite: `default` and `ci` leave the slow tests out, and
the profile `slow` runs only them, so every test runs in exactly one of `test` and `slow-tests`.
`slow-tests` runs in the job `gate`, beside `fmt`, `clippy` and the others, so on a pull request
that changes a map crate `gate` takes the time of the slow tests on top of its other checks.
`coverage` uses the profile `default` and so measures without the slow tests. A test that is not
slow has no name segment that starts with `slow_`, or `test` would leave it out.

`ac-trace` adds a result column to its table when nextest wrote reports: it merges the report of
the profile `ci` (`default` locally) with that of `slow`, so a criterion proven only by a slow
test shows `pass` or `FAIL` from the slow run. A report that is missing adds nothing, so a slow
test that did not run shows `–`; no pass is ever claimed that did not happen. `cargo xtask gate`
runs `ac-trace` after `test` and `slow-tests` for that reason. In CI, `ac-trace` runs in the
job `gate`, after `slow-tests` and without `test`, whose report stays in the job `test`. When the
slow tests ran in `gate`, the table has a result column in which they show `pass` or `FAIL` and
every other test shows `–`; when they did not run, the table has no result column. The verdict
never depends on the column, because `test` and `slow-tests` fail for a failing test.

On a pull request, `slow-tests` runs the slow tests only when the pull request changes, since its
merge base, a file of a workspace crate with slow tests (one of its `.rs` files contains `slow_`; the crate `xtask`
itself never counts), of a workspace crate such a crate depends on, or one of `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml`, `.cargo/`, `.config/nextest.toml` and `gate/nextest.toml` (dependencies,
the compiler and the nextest profiles that pick the slow tests); a change to the rest of the
gate, to `xtask/` or to the workflows does not count. Otherwise it is reported as not needed, with the reason, without building anything. The job `gate` runs and
reports on every pull request either way, so the required check is never missing. A push to
`main`, a manual run and the `platforms` schedule always run the slow tests; so does
`cargo xtask gate` outside CI, unless the change touches no code (the rule below). Inside a CI
job, `cargo xtask gate` hands `slow-tests` the event, so on a pull request, a `platforms` run
started by a label included, the rule above decides.

**Windows and macOS** are not in the per-PR checks, because their runner minutes cost about 1.7
and about 10 times as much as Linux on a private repository (GitHub's prices of November 2026:
$0.010 and $0.062 against $0.006 a minute). Instead the workflow `platforms` has one job per OS,
so each runner is set up and built once. The jobs `windows` (`windows-latest`) and `macos`
(`macos-latest`, Apple silicon) run `clippy`, `test`, `slow-tests`, `lints`, `ac-trace` and (from
P11) `ui`. The job `macos-intel` (`macos-15-intel`) runs `test` and `slow-tests` only, and only on
demand. The triggers: Monday 04:00 UTC for `windows` and `macos`; the label `windows` or `macos`
added to a PR (each label runs its own OS); a manual run with the inputs `windows` and `macos`
(on by default) and `macos_intel` (off). The rule for pull requests that change no code (below)
does not apply there: a label or a manual run is a deliberate request. The full test suite is
`test` and `slow-tests` together; `release.yml` runs both on all four runners before every release
(`docs/release.md`).
These runs are not required checks. A red run is fixed before the next release; it is listed in
`docs/status.md` under "Needs Jann" until then. `lints` and `ac-trace` run there too because
they read what the compiler builds on the machine they run on, and code behind `#[cfg(windows)]`
or `#[cfg(target_os = "macos")]` is only built there.

`cargo xtask gate` runs every check except `pr-body`, `pr-title` and `approval` locally, on
Windows and Linux, with the same results as CI (`commit-msg` checks the commits since `main`).
`slow-tests` runs the slow tests there whenever the change touches code, so the local gate is
never more lenient than CI. The rule for pull requests that change no code (below) applies
locally too, judged on the working tree: the current branch and every path changed since the merge
base with `origin/main`, committed or not, untracked files included, so uncommitted code always
counts. Claude runs the gate before claiming a phase is done.

### Pull requests that change no code

A pull request that changes only paths no check reads needs only `commit-msg`; every other check
is reported as not needed. `cargo xtask gate` decides, inside the job that always runs and
reports, so a skipped check is never a missing required check. It reads `gate/code-paths.toml`,
which holds two lists of globs. A path is non-code when it matches the first list and not the
second. The first list:

- `docs/**`
- `research/**`
- `design/**`
- `README.md`
- `CLAUDE.md`
- `.claude/**`

The second list is the paths that are code although the first list matches them. It wins in any
folder, and it ignores letter case, because the tool configs are found in any case:

- `docs/specs/00-umbrella.md`
- `docs/gate.md`
- `**/*.ts`
- `**/Cargo.toml`
- `**/Cargo.lock`
- `**/rust-toolchain`
- `**/rust-toolchain.toml`
- `**/clippy.toml`
- `**/.clippy.toml`
- `**/rustfmt.toml`
- `**/.rustfmt.toml`
- `**/.cargo/**`
- `**/.gitignore`
- `**/.gitattributes`
- `design/theos-ui/**`

The file `gate/code-paths.toml` itself is always code, whatever the lists say, so a pull request
cannot make its own change to the lists skip the tests that guard them.

Every other path is code: `CONTRIBUTING.md`, `.github/**`, `gate/**`, `xtask/**`, and every new
folder or root file, until someone lists it in a protected PR. The rule, in this order: a branch
called `phase/…` runs every check, because `ac-trace` reads the phase from the branch. Changed
paths that cannot be listed (a missing commit, git failing) run every check, with a note, and so
does no changed path at all. Only non-code paths run `commit-msg` and nothing else. Anything else
runs every check. The changed paths are those of the whole pull request since its merge base with
`main`, never those of the last push or of its base, and a rename counts as a deletion and an
addition. The base is not used because a base change starts no `ci` run, so a verdict made against
an old base could stay green. A push to `main`, a manual run, a `platforms` run started by a label
and a detached head have no rule: every check runs. Every doubt adds checks; none removes one.

A non-code change cannot alter a skipped check, because no check reads such a path. This is what
each check reads, with the place in the code:

| Check | What it reads | Why a non-code path cannot change its verdict |
|---|---|---|
| `fmt` | the workspace `.rs` files (`xtask/src/checks/cargo_cmds.rs:28`) | no non-code path is Rust code |
| `clippy` | the workspace, and by `include_str!` the umbrella (`xtask/src/umbrella.rs:287`) and the PR template (`xtask/src/checks/pr_body.rs:129`, `xtask/tests/pr_body.rs:10`) | the umbrella is always code, and `.github/**` is code |
| `test`, `coverage` | `docs/gate.md` (`xtask/tests/settings.rs:78`, `xtask/tests/workflows.rs:931`, `xtask/tests/approval.rs:579`), `CONTRIBUTING.md` (`xtask/tests/commit_msg.rs:436`), the workflows, and the files `clippy` includes; no crate takes a doc file into its docs (no `doc = include_str!`), so no doc test reads one | `docs/gate.md` is always code, and the others are code |
| `docs` | the workspace sources, through rustdoc (`xtask/src/checks/cargo_cmds.rs:114`) | no non-code path is Rust code |
| `deny` | the Cargo files, `deny.toml` and the advisory database (`xtask/src/checks/cargo_cmds.rs:123`) | Cargo files and `deny.toml` are code; a new advisory is not caused by the pull request and shows at the next code PR and the next push to `main` |
| `engine-free` | `cargo metadata` and `gate/engine-crates.toml` (`xtask/src/checks/engine_free.rs:42`) | both are code |
| `lints` | every file name, for `Cargo.toml` and the tool configs in any folder (`xtask/src/checks/lints/config_files.rs:168`, `:243`), and the mirrored files of `gate/` (`:39`) | those names are always code, whatever the folder and the case |
| `ac-trace` | the umbrella (`xtask/src/umbrella.rs:20`), every `.ts` file (`xtask/src/checks/ac_trace/mod.rs:311`, `:328`) and the branch (`:259`) | the umbrella and `.ts` files are always code, and a `phase/` branch runs every check |
| `tools` | `gate/tools.toml` (`xtask/src/tools.rs:21`) | code |
| `ui` | `editor-ui/**`, `editor-ui/e2e/ipc/*.json` and the design system `design/theos-ui/**`, which the UI imports (`xtask/src/checks/ui.rs`) | all of it is code: no path of `editor-ui/` is in the first list, and `design/theos-ui/**` is in the second |
| `slow-tests` | its own changed-paths rule (`xtask/src/checks/slow.rs:46`) | it only watches workspace crates and the files listed there |

`commit-msg` always runs. `pr-title`, `pr-body` and `approval` always run too, in jobs of their
own that the rule does not touch. They read the pull request, and the files they read are code:
`pr-body` reads `.github/pull_request_template.md`, and `approval` reads
`gate/protected-paths.toml` and `gate/repo-settings.toml` of the base commit. The tests
`gate_md_lists_the_code_paths_of_gate_toml` and
`gate_md_explains_every_check_a_docs_change_skips` keep the lists and the table in step with
`gate/code-paths.toml` and with the checks.

## 2. Lints (`gate/lints.toml`, mirrored into `Cargo.toml`)

At least: `unsafe_code = "forbid"`, `missing_docs = "deny"`, `clippy::all` and
`clippy::pedantic` as warnings (errors under `-D warnings`), `clippy::unwrap_used`,
`clippy::expect_used`, `clippy::panic`, `clippy::todo`, `clippy::unimplemented`,
`clippy::dbg_macro`, `clippy::print_stdout`, `clippy::print_stderr` denied outside tests and
binaries' `main` (use `#[expect(…, reason = "…")]` where one is justified). Four lints guard the
others: `unexpected_cfgs` is forbidden, and `clippy::allow_attributes`,
`clippy::allow_attributes_without_reason` and `clippy::ignore_without_reason` are denied (derives
such as clap's allow the lint groups that contain them, so they cannot be forbidden). `lints`
refuses every `allow` and `warn` attribute in our code, and every `allow` or `expect` that names
one of the four or the groups `clippy::pedantic` and `clippy::restriction` that contain them. It
also asks for a reason on every `#[expect]` and `#[ignore]`, also where clippy does not look, so
none of the four can be switched off. The result: no `#[allow]` or `#[warn]` anywhere, a reason
on every `#[expect]` and `#[ignore]`, and no unknown cfg name. Lint levels live only in
`gate/lints.toml`. P00 writes the final list; later changes need approval.

## 3. Thresholds (`gate/thresholds.toml`)

| Item | Start value |
|---|---|
| Line coverage, library crates | 80 % |
| Line coverage, binary crates (`theos`, `theos-mcp`, `theos-editor`) | 60 % |
| Largest allowed drop of a crate below `main` | 1.0 percentage point |

Raising a floor is welcome and still a protected change. Lowering one is never done to make a PR
pass.

## 4. Acceptance-criterion tags

A test proves a criterion by a comment directly above it:

```rust
// AC: CE-09.1
#[test]
fn imported_records_round_trip_byte_for_byte() { … }
```

Several: `// AC: CE-09.1, CE-09.3`. TypeScript tests use the same comment. Criteria are defined
only in `docs/specs/00-umbrella.md`.

## 5. Protected paths — Jann approves

- `CLAUDE.md`, `.claude/**`, `CONTRIBUTING.md`
- `docs/**`, except `docs/plans/**` and `docs/status.md`
- `.github/**`, `gate/**`, `xtask/**`, `deny.toml`, `rust-toolchain.toml`, `.cargo/**`
- `CHANGELOG.md`, `release-plz.toml`, `cliff.toml` (so every release PR waits for Jann)
- `design/**`
- the `[workspace.lints]` table of the root `Cargo.toml` (checked by `lints`)

Flow: CI adds the label `needs-jann` to such a PR and `approval` fails. Jann reviews and adds the
label `approved`; the check re-runs and auto-merge completes. Only Jann adds `approved`.
`needs-jann` stays on: CI adds it back, and removing it only starts another run.

Crate-level READMEs and rustdoc are not protected; they are reviewed in the phase reviews.

## 6. Merging

- Squash merge only, linear history, branch deleted after merge.
- Branches must be up to date with `main` before merging.
- No direct pushes, force pushes or deletions on `main`, for administrators too.
- Claude enables auto-merge on its PRs that do not touch protected paths; the PR merges when all
  checks pass.
- Release PRs are merged by Jann only. Tags are created only by the release pipeline
  (`docs/release.md`); nobody creates, moves or deletes a tag by hand.

## 7. Process requirements outside CI

- Every phase follows `.claude/skills/theos-phase/SKILL.md`.
- The final whole-branch review loops until a fresh reviewer finds nothing (CLAUDE.md).
- The PR description carries the final review summary and every ruling Claude made.
