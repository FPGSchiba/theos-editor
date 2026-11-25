# P50 · Leaner CI — phase spec

Status: waiting for Jann's approval · 25 Nov 2026 · repo of the build: `theos-editor`
Source: `docs/specs/00-umbrella.md` §3 P50, added by umbrella version 7 together with this spec.
Scope and acceptance criteria come from there; this spec decides how.

## 1. Goal

The gate keeps every check and every verdict, but stops paying for the same setup thirteen times.
On 3–12 Nov 2026 the gate used the $18 of GitHub Actions usage included in Jann's plan in ten
days; GitHub has refused every job since 13 Nov 07:01 UTC. At that pace the gate needs about
$38 a month. After P50 the same activity is estimated at about $13 a month (§4.2).

After P50:
- one CI job `gate` runs `cargo xtask gate`, the command Claude already runs locally; one job
  `pr` runs `pr-title` and `pr-body`; `approval` stays as it is. These three are the required
  status checks of `main`;
- a pull request that changes only paths no check reads (docs, research, design) runs only the
  checks those paths can affect;
- `pr` and `approval` build xtask from a cache that `main` saves;
- `platforms` runs one job per OS, and tests macOS on Intel only on demand and before releases.

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-08C Leaner CI**
- CE-08C.1 CI runs every check of `cargo xtask gate` in one job `gate`, and `pr-title` and
  `pr-body` in one job `pr`; the job summary of each lists every check it ran with its verdict.
- CE-08C.2 On a pull request or a local branch that is not a `phase/` branch and whose changed
  paths all match the non-code list of `gate/code-paths.toml` and none of its always-code list,
  `cargo xtask gate` runs `commit-msg` only and reports every other check as not needed; every
  other pull request and branch, every push, and every run whose changed paths cannot be read run
  every check.
- CE-08C.3 Every path outside the code that a check reads counts as code: the umbrella spec,
  `docs/gate.md`, `CONTRIBUTING.md`, `.github/**`, every `*.ts` file, and every `Cargo.toml`,
  `Cargo.lock`, toolchain, clippy, rustfmt or cargo configuration file in any folder.
- CE-08C.4 `docs/gate.md` lists the non-code and always-code paths exactly as
  `gate/code-paths.toml` does, and says for every check that a non-code pull request does not
  run why such a change cannot affect it.
- CE-08C.5 `pr` and `approval` build xtask from a cache that `main` saves, and no job of a pull
  request saves a build cache.
- CE-08C.6 `platforms` runs the checks of one OS in one job; its weekly and label runs test macOS
  on Apple silicon; the Intel macOS tests run only on demand; a newer label run for the same pull
  request and OS cancels the older one.

**Reworded by the same umbrella version** (Jann, 25 Nov 2026):
- CE-08.3 Every check of `docs/gate.md` §1 (except `ui`) runs in a required status check on
  `main` (`gate`, `pr` or `approval`); branch protection and repo settings match `docs/gate.md`
  §6.
- G2 (second sentence) Every PR is checked on Linux; Windows and macOS on Apple silicon are
  tested weekly, on demand (a label on the PR) and before every release, and macOS on Intel on
  demand and before every release (`docs/gate.md` §1).
- CE-08R.6 The release workflow runs the full test suite on Windows, Linux and macOS (Apple
  silicon and Intel) first and creates no release if any platform fails.

## 3. Scope

**Goals**
- `ci.yml` with the one job `gate`; `pr.yml` with the one job `pr`; `approval.yml` with the
  cache change only (§5.2, §5.5).
- In xtask: the docs-only decision inside `gate` (§5.3), the changed-paths reader shared with
  `slow-tests` and `approval`, the job summary (§5.2), `gate --only` for `platforms` (§5.6).
- `gate/code-paths.toml`, `gate/ruleset-main.json` (required checks `gate`, `pr`,
  `approval`), `docs/gate.md` §1 rewritten, `platforms.yml` one job per OS.
- The xtask tests that keep workflows, `docs/gate.md` and the ruleset equal, updated to the new
  shape (§5.9).
- `CONTRIBUTING.md` and the theos-phase skill: when to update a branch from `main` (§5.8).

**Non-goals**
- No check changes what it checks; no threshold, lint or test changes.
- No self-hosted runner (§11).
- No change to `approval`'s logic or triggers; no change to `settings.yml`.
- `release.yml` is P49's; P50 only states what it must run (§6).

**Anti-goals** (a failure even if the minutes drop)
- A required check that can report success without its checks having run.
- A docs-only rule that skips a check a docs change can affect.
- A local `cargo xtask gate` that gives a different verdict than the CI job.

## 4. Decisions

### 4.1 Decided by Jann (25 Nov 2026)

- Build stage 1 (docs-only skip, shared xtask cache, `platforms` one job per OS) and stage 2
  (fewer CI jobs) together as one phase.
- Stage 2 with one required check `gate` that runs `cargo xtask gate`, and CE-08.3 reworded,
  instead of 13 named checks posted through the GitHub API.
- The Intel macOS tests only on demand and before releases (G2 and CE-08R.6 reworded).
- Self-hosted runners on Jann's homelab (Proxmox) are an idea for later (§11).

### 4.2 Measurements (runs of 3–24 Nov 2026, all of the repository's history)

Data: every run and job from the REST API, the log of every billed job, and GitHub's prices of
24 Nov 2026 (docs.github.com/en/billing/reference/actions-runner-pricing: Linux $0.006, Windows
$0.010, macOS $0.062 a minute; each job rounded up to a whole minute; the included usage is a
dollar amount). "lmin" = Linux-equivalent minutes ($ / 0.006).

| Where the 3,080 billed lmin went | lmin | Share |
|---|---|---|
| Linux job time outside the check step: runner, checkout, toolchain download, cache, xtask build, tools, rounding | 1,893 | 61% |
| `platforms` (macOS 527, Windows 380; manual runs 376, labels 386, Monday 85, Intel test 103) | 847 | 28% |
| `ci` on push to `main` (same tree the PR tested) | 375 | 12% |
| Merges of `main` into PR branches | 296 | 10% |
| Rust checks on docs PRs (8.3 points skippable, 6.9 must run) | 469 | 15% |
| Jobs cancelled by a newer push, up to the cancel | 233 | 8% |

The check steps themselves took 280 raw minutes, of which 171 compiling and 109 running checks.
A median Linux job ran 41 s. Estimated per month on the same activity: 6,300 lmin ($38) today,
about 2,100 lmin ($13) with this design (merged CI job measured as about 27% of the separate
jobs' minutes, `platforms` per OS about 56%).

### 4.3 Approaches considered

- **Docs-only skip in the workflow** (a classifier job, `needs:` and a job-level `if:`): a skipped
  job reports success, so a failing classifier would turn every required check green unless the
  condition is written fail-safe; the gate's tests forbid `needs:` and `if:` on required jobs for
  that reason, and PR #43 decided the same for `slow-tests`. **Chosen instead:** the decision
  inside xtask, in the job that always runs and reports.
- **A workflow `paths-ignore` filter:** a skipped workflow never reports, so the required checks
  stay pending. Rejected.
- **13 named checks posted from one job through the Checks API:** keeps CE-08.3 as it was, but
  needs custom reporting code and an unproven assumption (that GitHub accepts such runs as the
  required checks bound to the GitHub Actions app). Rejected by Jann.
- **Skipping the Rust checks on docs-only pushes to `main`:** saves about 80 lmin a month, but
  the push run fills the caches and the coverage baseline that pull requests use and checks the
  squash commit's message. Rejected (Ruling R4).
- **Fewer event types for `approval` and `pr`:** under 20 lmin a month, and a job-level skip on a
  required check would overwrite a red verdict with green. Rejected.
- **A monthly `platforms` schedule:** changes CE-08.12 and G2; not asked for. Rejected.

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Tool or crate | Custom code, and why |
|---|---|---|
| Changed paths of a pull request | `git` CLI: merge base of the event's base and head, `git diff --name-only --no-renames` (what `approval` and PR #43's `slow-tests` do) | One shared module instead of three copies |
| Path lists and matching | `toml` + `globset` with `literal_separator(true)`, as `gate/protected-paths.toml` | Only the two-list rule (§5.3) |
| The docs-only decision | xtask, in `gate`'s runner | The rule itself; small and unit-tested |
| Job summary | GitHub's `$GITHUB_STEP_SUMMARY` (Markdown) | Writing the existing verdict table as Markdown |
| Running a subset of checks | xtask `gate --only <checks>` over the existing runner | A filter over `GATE_ORDER` |
| Workflow shape tests | the existing `xtask/tests/workflows.rs` and `settings.rs` (YAML and JSON crates) | Updated expectations |
| Caches | `Swatinem/rust-cache` through `.github/actions/setup` (`shared-key`, `save-if`) | none |

### 5.2 CI after P50

| Workflow | Job (= check name) | Runs | Triggers | Required |
|---|---|---|---|---|
| `ci` | `gate` | `cargo xtask gate`: every check of `GATE_ORDER`, with the docs-only rule on a pull request | `pull_request` (opened, synchronize, reopened), push to `main`, `workflow_dispatch` | yes |
| `pr` | `pr` | `cargo xtask pr-title`, then `cargo xtask pr-body` in the same xtask call or as two steps that both always run | `pull_request` (opened, edited, synchronize, reopened) | yes |
| `approval` | `approval` | unchanged | unchanged | yes |
| `platforms` | `windows`, `macos`, `macos-intel` | `cargo xtask gate --only clippy,test,lints,ac-trace` (plus `slow-tests` once PR #43 is in) | §5.6 | no |
| `settings` | `settings` | unchanged | unchanged | no |

- `gate` checks out with the whole history (`coverage`, `commit-msg` and the changed paths need
  it), restores the coverage baseline cache as `coverage` does today, and has a timeout that
  covers the sum of today's jobs on a cold cache (the builder measures it; `slow-tests` from PR
  #43 brings its 120 minutes).
- A check that fails does not stop the others (the runner already works this way), so one run
  shows every verdict. The job fails when any check fails.
- The job summary is the verdict table with one row per check: passed, failed (with the
  findings), or not needed (with the reason). `pr` writes the same for its two checks.
- `ci` keeps `cancel-in-progress` for pull requests; `pr` as well.

### 5.3 The docs-only rule (CE-08C.2, CE-08C.3)

`gate/code-paths.toml` holds two lists of globs:

- **non-code:** `docs/**`, `research/**`, `design/**`, `README.md`, `CLAUDE.md`, `.claude/**`;
- **always-code** (wins over non-code, in any folder): `docs/specs/00-umbrella.md`,
  `docs/gate.md`, `**/*.ts`, `**/Cargo.toml`, `**/Cargo.lock`, `**/rust-toolchain`,
  `**/rust-toolchain.toml`, `**/clippy.toml`, `**/.clippy.toml`, `**/rustfmt.toml`,
  `**/.rustfmt.toml`, `**/.cargo/**`, `**/.gitignore`, `**/.gitattributes`.

Always-code globs ignore letter case (`lints` finds tool configs in any case); non-code globs
match exactly, as the `protected` and `except` lists of `gate/protected-paths.toml` do.

Everything not on the non-code list is code: `crates/**`, `xtask/**`, `gate/**`, `.github/**`,
`.config/**`, `CONTRIBUTING.md`, every root file not listed, and every folder added later (for
example `editor-ui/` in P11). A new path is code until someone adds it to the non-code list in a
protected PR.

The same rule decides in CI and locally, so `cargo xtask gate` gives the same verdicts in both
places (CE-08.4). Its inputs:
- **pull request in CI:** the event's head branch, and the paths changed between the merge base
  of the event's base and head and the head;
- **local run:** the current branch (`git symbolic-ref`), and every path changed since the merge
  base with `origin/main`: committed, staged, unstaged and untracked (not ignored) — a local run
  judges the working tree, so uncommitted code counts;
- **push, `workflow_dispatch`, detached HEAD:** no rule; every check runs.

Before any check runs, `gate` decides:
1. the head branch starts with `phase/` → every check (ac-trace reads the phase from it);
2. the changed paths cannot be listed (a missing commit, git failing) → every check, with a note;
3. no changed path at all → every check;
4. every changed path matches non-code and none matches always-code → `commit-msg` only; every
   other check is reported as not needed, naming `gate/code-paths.toml`;
5. otherwise → every check.

The changed paths are those of the whole pull request or branch against the merge base, never
those of the last push, with renames as a deletion and an addition. `tools` counts as a check
that is not needed: `commit-msg` needs only xtask. Every uncertain case adds checks; none removes
one.

### 5.4 Why a non-code pull request cannot change a skipped check (CE-08C.4)

From reading every check on `main` at `0a6f480` (file:line evidence goes into `docs/gate.md`):

| Check | What it reads outside `crates/` and `xtask/` | Why non-code paths cannot change its verdict |
|---|---|---|
| `fmt` | nothing | formats only workspace `.rs` files |
| `clippy` | the umbrella and `.github/pull_request_template.md` (`include_str!` in tests) | both are code |
| `test`, `coverage` | the umbrella, `docs/gate.md`, `CONTRIBUTING.md`, `.github/**` | all code; no doctests; no crate includes a doc file in its docs |
| `docs` | nothing (rustdoc does not compile `cfg(test)`) | — |
| `deny` | Cargo files, `deny.toml`, the advisory database | Cargo files are code; a new advisory is not caused by the PR and shows at the next code PR and the next push to `main` |
| `engine-free` | `cargo metadata`, `gate/engine-crates.toml` | code |
| `lints` | every file name (looks for `Cargo.toml` and tool configs anywhere) and the two `include_str!` files | those names and files are always-code |
| `ac-trace` | the umbrella, every `*.ts` file anywhere, the branch name | umbrella and `*.ts` are code; `phase/` branches run every check |
| `tools` | `gate/tools.toml` | code |
| `slow-tests` (PR #43) | its own changed-paths rule | no workspace crate changes |

`commit-msg`, `pr-title`, `pr-body` and `approval` always run: they read the commits and the
pull request, not files.

### 5.5 Caches (CE-08C.5)

- `gate` restores the build cache of `main` and saves it only on `main` (`save-if`). A pull
  request saves no build cache, so open pull requests cannot evict `main`'s cache from the 10 GB.
- `pr` and `approval` restore a cache of `main` that holds a built xtask and save nothing. Which
  cache that is (the `gate` cache, or a small xtask cache saved next to it on `main`) is the
  builder's choice after measuring the restore time.
- The coverage baseline cache keeps its key and is saved as today.

### 5.6 `platforms` (CE-08C.6)

- Jobs `windows` (`windows-latest`) and `macos` (`macos-latest`, Apple silicon), each running
  `cargo xtask gate --only clippy,test,lints,ac-trace` (and `slow-tests` with PR #43's rule) and
  writing the verdict table to the job summary. Not required checks, as today.
- Job `macos-intel` (`macos-15-intel`) runs `--only test` (and `slow-tests`), on demand only.
- Triggers: Monday 04:00 UTC → `windows` and `macos`; label `windows` → `windows`; label `macos`
  → `macos`; `workflow_dispatch` with three boolean inputs `windows` (default on), `macos`
  (default on) and `macos-intel` (default off).
- Concurrency per pull request and OS with `cancel-in-progress: true` for label runs; scheduled
  and manual runs are never cancelled.
- The docs-only rule does not apply here: a label or a manual run is a deliberate request.
- Only the scheduled runs on `main` save a build cache; label and manual runs save none.

### 5.7 `docs/gate.md`, the ruleset and the switch-over

- `docs/gate.md` §1: the table lists every check with the required job that runs it (`gate`,
  `pr`, `approval`; `ui` from P11); a new subsection gives the two lists and the table of §5.4;
  the Windows and macOS paragraph gives today's prices (Windows about 1.7×, macOS about 10× Linux)
  and the new `platforms` triggers.
- `gate/ruleset-main.json`: required checks `gate`, `pr`, `approval`.
- Switching over is Jann's, because it changes the live ruleset: when the P50 PR is approved,
  Jann runs `cargo xtask github apply`; the PR then shows its three required checks and merges.
  Open pull requests merge `main` afterwards (the ruleset asks for up-to-date branches anyway).
- Nothing runs while GitHub refuses jobs; the proofs of §5.9 need billing to be unblocked first.

### 5.8 Working practice

`CONTRIBUTING.md` and the theos-phase skill gain two rules:
- update a pull request branch from `main` only when it is ready to merge and next in line, not
  after every merge to `main` (237 of 296 lmin of such merges were superseded by another one);
- push when a review round or task is finished, not after every commit.

### 5.9 How each AC is proven

| AC | Tests (tagged `// AC:`) | On the PR |
|---|---|---|
| CE-08C.1 | workflow shape: `ci` has exactly `gate`, `pr.yml` exactly `pr`; `gate` runs `cargo xtask gate`; unit tests of the Markdown summary | the run's summary |
| CE-08C.2 | unit tests of the decision over a fixture git repository: docs-only, mixed, `phase/` branch, unreadable range, empty diff, rename out of `docs/` | probe PRs below |
| CE-08C.3 | a test that classifies every input of §5.4 as code | probe PRs below |
| CE-08C.4 | a test that the lists in `docs/gate.md` equal `gate/code-paths.toml` | — |
| CE-08C.5 | workflow shape: `save-if` only on `main`; `pr` and `approval` restore `main`'s cache with saving off | — |
| CE-08C.6 | workflow shape: jobs, runners, triggers, inputs, concurrency of `platforms` | a manual run with `macos-intel` on |
| CE-08.3 (new wording) | `settings.rs`: required contexts equal the job column of §1 and the jobs | — |

**Probe pull requests**, opened against the P50 branch and closed unmerged, linked from the P50
PR: (1) only `docs/status.md` → `gate`, `pr` and `approval` finish green and `gate`'s summary
shows `commit-msg` run and every other check not needed; (2) the umbrella; (3) `research/x.ts`;
(4) `docs/x/Cargo.toml`; (5) only `docs/` on a `phase/` branch → (2)–(5) run every check. The P50
PR itself changes code and runs every check. The probes target the P50 branch, which has no
ruleset, so the merge box of `main` is seen on the first docs-only PR into `main` after the
switch-over; that observation goes into `docs/status.md`, not into P50's proof.

**Locally**, the same five cases run as unit tests over a fixture repository, plus an uncommitted
code change and an untracked `.rs` file on a docs-only branch (both → every check).

## 6. Interfaces

- **PR #43 (`slow-tests`)**, open: if it merges first, `slow-tests` becomes one more check of
  `gate` and of the `platforms` jobs, keeps its own changed-paths rule, and uses the shared
  reader. If P50 merges first, #43 is rebased onto P50's shape. P50's build starts from the
  `main` of that day.
- **P49 (`release.yml`)**: runs the full test suite (with `slow-tests`) on Windows, Linux,
  macOS on Apple silicon and macOS on Intel before every release (CE-08R.6).
- **P11 (`ui`)**: adds `ui` as its own required job; `editor-ui/**` is code under §5.3.

## 7. Risks

- **A `gate` run takes longer to report than today's slowest job** (estimated 2–3 minutes on a
  warm cache instead of about 1.6; longer on map PRs with `slow-tests`). Accepted for the minutes.
- **One red check gives one red `gate`.** The job summary names the failing check.
- **The rule misses an input a later phase adds** (a check that starts reading a doc file). The
  default is code, and CE-08C.3's test lists the known inputs; the reviews of the phase that adds
  such a read must add it to always-code.
- **Cold caches after the billing block**: caches unused for 7 days are gone; the first runs
  build from scratch.

## 8. Review Focus — inputs most likely to break it

- A pull request that renames a file from `crates/` to `docs/` (must be code).
- A docs PR on a `phase/` branch, or with an `.ts` file in `docs/`.
- A first push whose base commit is not fetched yet (shallow history, force-pushed base).
- A `pull_request` event without a head branch name, and `workflow_dispatch` on a PR branch.
- The job summary when a check cannot run at all (`could not run: …`).
- `platforms` on a label that is neither `windows` nor `macos` (no job may start).
- Windows paths in the changed-paths list (always `/`).
- A local docs-only branch with an uncommitted or untracked code file; a clone without
  `origin/main`.

## 9. Rulings (Claude's calls)

- **R1** The docs-only decision lives in xtask inside `gate`, not in the workflow — no job-level
  `if:` on a required job; same as PR #43 — cost if wrong: one billed minute per docs PR that a
  skipped job would not cost.
- **R2** An unreadable change list runs every check instead of failing — the safe direction is
  more checks; #43's `slow-tests` fails closed because it cannot fall back to running its tests
  cheaply — cost if wrong: a broken checkout shows as a slow run instead of a red one.
- **R3** The rule decides locally as in CI, with the working tree's changes — otherwise the local
  gate and CI give different verdicts on a docs-only branch (CE-08.4) — cost if wrong: a local
  run on a docs-only branch checks less than before, which is exactly what CI checks.
- **R4** Pushes to `main` always run every check — they feed the caches and the coverage baseline
  and check the squash commit's message — cost if wrong: about 80 lmin a month.
- **R5** `coverage` runs inside `gate`, not as its own job — one setup instead of two; `cargo
  xtask gate` already includes it — cost if wrong: about a minute more of latency per run.
- **R6** Pull requests save no build cache — `main`'s cache must survive the 10 GB limit — cost
  if wrong: a PR that changes `Cargo.lock` rebuilds the changed dependencies on every push.
- **R7** The non-code list starts small (docs, research, design, README, CLAUDE.md, `.claude/`) —
  every other path keeps today's full run — cost if wrong: some cheap PRs still run every check.
- **R8** The phase is P50, a new umbrella phase with criteria CE-08C, because every piece of work
  is a phase and ACs live only in the umbrella — cost if wrong: renumbering in the spec PR.
- **R9** The probes prove CE-08C.2 on the P50 branch; the merge box of `main` is observed after
  the switch-over, outside P50 — a branch other than `main` has no ruleset — cost if wrong: one
  more docs-only PR to watch.
- **R10** While GitHub refuses jobs, the build does everything that runs locally, sets the `gate`
  timeout from a cold local run with a wide margin, and opens the PR with the CI proofs listed as
  pending; none is claimed before it has run — the PR waits for Jann anyway — cost if wrong: the
  timeout is adjusted after the first CI runs.

## 10. Left to the builder

- Which cache `pr` and `approval` restore (§5.5), from measured restore times; without a
  measurement, the `gate` cache of `main`.
- Whether `pr` runs its two checks in one xtask call or two always-running steps.
- The `gate` job's timeout, from a measured cold run.
- The exact Markdown of the job summary.

## 11. Later (not in P50)

Self-hosted runners on Jann's homelab (Proxmox) would cost no runner minutes. Windows and Linux
virtual machines fit; macOS virtual machines are licensed only on Apple hardware, so the macOS
jobs would stay on GitHub. Before any job moves there: a dedicated runner VM per OS with no
access to Jann's other machines and credentials, used only by labelled `platforms` jobs (never by
a required check unless the homelab is always on: a queued job fails after 24 hours), and the
runner labels read from a repository variable so moving back is one setting.
