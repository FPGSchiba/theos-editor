---
name: theos-phase
description: Use to run the next phase of the Theos content editor build (or a phase Jann names) end to end — phase spec, plan, subagent-driven build, final review loop, PR — following the umbrella spec in docs/specs/00-umbrella.md.
---

# Run one phase

Announce: "Running phase PNN with the theos-phase skill." Read `CLAUDE.md` first; its rules and
its changes to the superpowers loop apply to every step below.

## 0. Pick the phase

1. Read `docs/status.md` and `docs/specs/00-umbrella.md` §3.
2. Take the phase Jann named, or else the lowest-numbered phase whose dependencies are `done`
   and whose state is `todo` or `spec-approved`.
3. If its phase spec PR is open and not merged, stop: tell Jann the spec waits for approval.

## 1. Phase spec (state `todo` → `spec-pr`)

1. Use superpowers:brainstorming, starting from the phase's umbrella entry, the documents it
   names and the code that exists. Scope and ACs are fixed by the umbrella; the questions are
   about how. Present the approaches to Jann and let him choose.
2. Write `docs/specs/phases/PNN-<slug>.md` with: goal, ACs copied verbatim, design, interfaces
   other phases will use, risks, and the "Review Focus" inputs most likely to break it.
3. Branch `spec/PNN-<slug>`, commit, open a PR titled `[PNN-spec] <name>`. Do not enable
   auto-merge (protected path). Set the phase to `spec-pr` in `docs/status.md` in the same PR.
4. Stop and tell Jann the spec is ready for review.

## 2. Plan (state `spec-approved` → `in-progress`)

Start only when the spec PR is merged.

1. superpowers:using-git-worktrees: worktree on branch `phase/PNN-<slug>` from current `main`.
2. superpowers:writing-plans → `docs/plans/PNN-<slug>.md`. Global Constraints = umbrella G1–G16
   plus the phase ACs, verbatim. Each task lists the AC ids its tests prove.
3. Self-review the plan against the phase spec, then continue without waiting (the approved spec
   is the authority; the plan is not a protected path).
4. Set the phase to `in-progress` in `docs/status.md`; commit.

## 3. Build

superpowers:subagent-driven-development over the plan, with superpowers:test-driven-development
in every implementer. Tests carry `// AC: <id>`. Never change protected paths on this branch.

## 4. Final review loop

As defined in `CLAUDE.md` ("Final review is a loop"): fresh whole-branch reviewer each round,
one fix subagent per round with the complete findings list, until a fresh reviewer finds nothing;
cap 5 rounds.

## 5. Verify

superpowers:verification-before-completion: run `cargo xtask gate` (and the `ui` checks from P11
on) and read the output. Every phase AC must show at least one passing test in
`cargo xtask ac-trace --phase PNN`.

## 6. PR

1. Set the phase to `pr-open` in `docs/status.md`; add anything for Jann under "Needs Jann".
2. Push, open the PR `[PNN] <name>` with the template filled: ACs met, final review rounds and
   last verdict, every ruling, gate output summary.
3. Enable auto-merge if the PR touches no protected path; otherwise leave it for Jann.
4. When it is merged, set the phase to `done` in a follow-up commit on the next branch, and
   start over at step 0 if Jann asked for more than one phase.

## Phases in tower-of-theos (P01, P08)

Same steps, but the implementation branch and PR are in `../tower-of-theos`, which has no
auto-merge: every PR there waits for Jann. Track the state in this repo's `docs/status.md`.

## Research phase (P10)

Steps 1, 2 and 5 apply; the build is research and prototypes in `research/P10-prose-editor/`;
the result is a decision record in `docs/decisions/`, which waits for Jann.
