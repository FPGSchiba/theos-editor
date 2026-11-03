# theos-editor — instructions for Claude

This repo builds the Theos content editor for Tower of Theos. Claude builds it; Jann owns the
architecture and approves documentation. Read this file fully at the start of every session.

## Sources of truth

1. `docs/specs/00-umbrella.md` — phases, acceptance criteria (ACs), global constraints.
2. `docs/architecture.md`, `docs/specs/data-model.md`, `docs/generators.md`, `docs/gate.md`,
   `CONTRIBUTING.md` (branches, commits, PRs, versions, tags), `docs/release.md`.
3. `docs/specs/phases/PNN-*.md` — the approved spec of a phase.
4. `docs/plans/PNN-*.md` — the plan of a phase.
5. `docs/status.md` — where the build stands.

Higher beats lower. This session has no access to Confluence or Jira; never ask for it and never
guess what a Confluence page says. If something can only be answered there, add it to
"Needs a private session" in `docs/status.md` and continue with what the repo defines.

The content project is the sibling clone `../tower-of-theos`. Work there only in the phases that
name it (P01, P08) and for reading fixtures.

## How work is done

- Every piece of work is a phase of the umbrella spec, run with the project skill
  `.claude/skills/theos-phase/SKILL.md`. Use superpowers for every step it names.
- One phase = one phase spec PR (waits for Jann) + one implementation branch and PR.
- Never work on `main`. Never push to `main`.
- **Every phase runs in its own git worktree from its first step on**, the spec included, so
  several sessions can run phases in parallel from the same clone. Consent for
  superpowers:using-git-worktrees is hereby given; use the native worktree tool if the session
  has one, otherwise `.worktrees/<name>` in the repo the step works in. Never switch branches in
  the main checkout.
- Parallel PRs touch `docs/status.md` at the same time. When a rebase conflicts there, keep the
  newest state of every row and every list entry from both sides.

## Hard rules — the gate

These rules are part of the gate (`docs/gate.md`). Claude works under Jann's GitHub account, so
GitHub cannot enforce them; breaking one is never acceptable, also not to unblock work.

1. Never add or remove the label `approved`. Never remove `needs-jann`.
2. Never enable auto-merge, merge, or ask anyone to merge a PR that touches a protected path
   (`docs/gate.md` §5). Those PRs wait for Jann.
3. Never change protected paths inside an implementation PR. Gate, spec or doc changes go in their
   own PR.
4. Never lower a threshold, disable or skip a check, delete or weaken a test to make it pass,
   add `#[allow]`, or add `#[ignore]` / `#[expect]` without a real reason in the attribute.
5. Never use `--no-verify`, force-push to a shared branch, or rewrite history on `main`.
6. Never claim a check passed without having run it and read its output
   (superpowers:verification-before-completion).
7. Never change an AC. If an AC is wrong or impossible, stop and say so (see "When to stop").
8. Never merge a release PR, and never create, move or delete a tag. Releases are Jann's.

## Changes to the superpowers loop (they override the skills)

- **Locations.** Phase specs: `docs/specs/phases/PNN-<slug>.md` (not `docs/superpowers/specs/`).
  Plans: `docs/plans/PNN-<slug>.md` (not `docs/superpowers/plans/`).
- **Brainstorming** for a phase starts from the phase's entry in the umbrella spec. Its scope and
  ACs are fixed; the phase spec decides how. Lay out the approaches, write the phase spec, open
  the spec PR and stop until Jann approves it.
- **Plans** copy the umbrella's global constraints (G1–G16) and the phase ACs verbatim into
  "Global Constraints". Every task names the ACs its tests prove; tests carry `// AC: <id>`.
- **Execution** is superpowers:subagent-driven-development.
- **Final review is a loop** (replaces the single fix wave in the skill's final review):
  1. Dispatch a fresh whole-branch reviewer (most capable model) with the review package, the
     phase spec, the umbrella constraints and ACs, the ledger's deferred and parked lines, and the
     list of all earlier findings with their resolutions and rulings.
  2. If it reports any finding — Critical, Important or Minor — dispatch one fix subagent with the
     complete list, then go back to step 1 with a new, fresh reviewer (never a scoped re-review).
  3. Stop when a fresh reviewer reports no findings.
  4. A reviewer may not reopen a finding that an earlier round resolved or ruled on unless it
     brings new evidence; it says so explicitly.
  5. After 5 rounds without a clean review, stop: open the PR as a draft, label it `needs-jann`,
     and list the open findings in the description.
- **Finishing** is always a PR (never a local merge). Fill the PR template completely, including
  every ruling from the ledger. Enable auto-merge only when rule 2 above allows it.
- **Model selection** as in the skill: plans and final reviews on the most capable model; task
  reviewers on a mid-tier model; implementers on the cheapest model that fits the task.

## Commands

```text
cargo xtask gate                 # every local check of docs/gate.md, same verdicts as CI
cargo xtask ac-trace --phase P02 # ACs of a phase and their tests
cargo nextest run --workspace
cargo xtask generator-check -- <command…>
```

## Engineering directives

These decide how something gets built, before any code is written. They bind every session and
every subagent; put them in subagent briefs.

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

## Code conventions

- Rust edition 2024, toolchain from `rust-toolchain.toml`. Library errors with `thiserror`;
  `anyhow` only in binaries. Logging with `tracing`. No `unwrap`/`expect` outside tests.
- Small files with one responsibility. Public items documented. Tests next to the code
  (`#[cfg(test)]`) for units, `tests/` for behaviour, fixtures in `crates/<crate>/tests/fixtures/`.
- Content fixtures are real-looking records following `docs/specs/data-model.md`. Never invent
  lore canon in fixtures beyond what the examples show; mark synthetic fixtures as such.
- Windows matters most, and macOS ships too: paths through `std::path`, no shell assumptions, LF
  in files we write, Ctrl in the UI (Cmd on macOS), processes stopped as whole trees. Code paths
  that differ per OS get a test on each OS; macOS is tested by the weekly `macos` workflow.
- Branches, commits, PR titles, versions and tags follow `CONTRIBUTING.md`: Conventional
  Commits with the `Refs:` and `Phase:` footers and the Claude trailers; PR titles like
  `feat(content): records and grammars (P02)` and `docs(spec): records and grammars (P02)`.

## When to stop and ask Jann

Only for: an irreversible or destructive action; a security-sensitive action; anything that needs
a protected-path change; an AC that is wrong or impossible; a final review loop that hit its cap.
Everything else is decided, recorded as `Ruling: <what> — <why> — <cost if wrong>` in the ledger,
and listed in the PR. Put open questions for Jann in `docs/status.md` under "Needs Jann".

## Language

English for code, docs, commits and PRs. README files are named README.
