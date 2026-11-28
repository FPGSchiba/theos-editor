# 0005 · The final review loop judges its findings and ends as best effort

Follows theos-loom's decision 0008 (`docs/decisions/0008-judged-review-loop.md` in
`FPGSchiba/theos-loom`), which Jann took on 28 Nov 2026. It replaces the cap rule of the
2026-10-31 entry "The final whole-branch review loops with fresh reviewers until clean (cap 5
rounds)".

## Status

proposed · 2026-11-28 · accepted when Jann merges #65.

## Decision

- Every finding of a final reviewer is judged: fix it, or rule it not worth fixing, with a
  `Ruling: <what> — <why> — <cost if wrong>` line. Minor findings that change no behaviour and
  risk nothing are ruled, not fixed. The loop stops when a reviewer reports nothing worth
  fixing. A reviewer may reopen a ruled finding only with new evidence.
- After 5 rounds: one last fix wave and a targeted review of it, then stop. The review was best
  effort; the PR stays ready (not draft) with its open findings listed. A review loop that hit
  its cap is no longer a reason to stop and ask Jann, and no reason to label the PR `needs-jann`.

## Why

In theos-loom's P00, fresh reviewers found new minor issues in every round, so the loop always
ran to its cap, and every round cost Jann's time. Five fresh reviews give high enough quality.
theos-editor saw the same: P08's loop hit its cap with 6 Minor findings open (#44), and P05's
with one Minor finding (#42).

## Options considered

- Loop until a reviewer reports no findings at all, and turn the PR into a draft labelled
  `needs-jann` after 5 rounds (the rule until now).

## Consequences

`CLAUDE.md` (the final review loop, the `needs-jann` rule under "CI minutes", "When to stop and
ask Jann"), the theos-phase skill, the PR template's "Final review" section and `docs/gate.md` §7
say so. No acceptance criterion of `docs/specs/00-umbrella.md` states the old rule.
