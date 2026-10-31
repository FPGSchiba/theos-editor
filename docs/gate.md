# The gate

Status: accepted with the umbrella spec; built in phase P00. Binding. Changing this file needs
Jann's approval.

The gate decides what may reach `main`. Code PRs merge on their own when every check is green.
PRs that touch protected paths wait for Jann. Claude works under Jann's GitHub account, so GitHub
cannot tell the two apart: the rules in `CLAUDE.md` that Claude never approves, never weakens
the gate and never merges a waiting PR are part of the gate.

## 1. Required checks

All checks are required status checks on `main` (branch protection, administrators included).

| Check | What it runs | OS |
|---|---|---|
| `fmt` | `cargo fmt --all --check` | Linux |
| `clippy` | `cargo clippy --workspace --all-targets --locked -- -D warnings` | Linux, Windows |
| `test` | `cargo nextest run --workspace --locked` and `cargo test --doc --workspace --locked` | Linux, Windows |
| `docs` | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` | Linux |
| `deny` | `cargo deny check` (licences, advisories, bans, sources) | Linux |
| `engine-free` | `cargo xtask engine-free`: no engine crate in the tree without feature `engine` | Linux |
| `coverage` | `cargo xtask coverage`: per-crate line coverage ≥ floor in `gate/thresholds.toml`, and no crate drops more than the allowed amount below `main` | Linux |
| `lints` | `cargo xtask lints`: `[workspace.lints]` in the root `Cargo.toml` equals `gate/lints.toml`; every `#[expect]` carries a `reason`; no `#[allow]`; every `#[ignore]` carries a reason | Linux |
| `ac-trace` | `cargo xtask ac-trace`: every acceptance criterion of the PR's phase has at least one test tagged `AC: <id>`; no tag names an unknown criterion | Linux |
| `pr-body` | `cargo xtask pr-body`: the PR description has every section of the template, filled | Linux |
| `approval` | fails while the PR touches a protected path and lacks the label `approved` | Linux |
| `ui` | from P11: `pnpm install --frozen-lockfile`, `svelte-check`, `eslint`, `vitest`, Playwright component tests | Linux, Windows |

`cargo xtask gate` runs every check except `pr-body` and `approval` locally, on Windows and
Linux, with the same results as CI. Claude runs it before claiming a phase is done.

## 2. Lints (`gate/lints.toml`, mirrored into `Cargo.toml`)

At least: `unsafe_code = "forbid"`, `missing_docs = "deny"`, `clippy::all` and
`clippy::pedantic` as warnings (errors under `-D warnings`), `clippy::unwrap_used`,
`clippy::expect_used`, `clippy::panic`, `clippy::todo`, `clippy::unimplemented`,
`clippy::dbg_macro`, `clippy::print_stdout`, `clippy::print_stderr` denied outside tests and
binaries' `main`, `clippy::allow_attributes` and `clippy::allow_attributes_without_reason` denied
(use `#[expect(…, reason = "…")]`). P00 writes the final list; later changes need approval.

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

- `CLAUDE.md`, `.claude/**`
- `docs/**`, except `docs/plans/**` and `docs/status.md`
- `.github/**`, `gate/**`, `xtask/**`, `deny.toml`, `rust-toolchain.toml`, `.cargo/**`
- `design/**`
- the `[workspace.lints]` table of the root `Cargo.toml` (checked by `lints`)

Flow: CI adds the label `needs-jann` to such a PR and `approval` fails. Jann reviews and adds the
label `approved`; the check re-runs and auto-merge completes. Only Jann adds `approved`.

Crate-level READMEs and rustdoc are not protected; they are reviewed in the phase reviews.

## 6. Merging

- Squash merge only, linear history, branch deleted after merge.
- Branches must be up to date with `main` before merging.
- No direct pushes, force pushes or deletions on `main`, for administrators too.
- Claude enables auto-merge on its PRs that do not touch protected paths; the PR merges when all
  checks pass.

## 7. Process requirements outside CI

- Every phase follows `.claude/skills/theos-phase/SKILL.md`.
- The final whole-branch review loops until a fresh reviewer finds nothing (CLAUDE.md).
- The PR description carries the final review summary and every ruling Claude made.
