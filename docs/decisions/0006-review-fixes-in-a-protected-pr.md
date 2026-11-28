# 0006 · Review fixes inside a protected PR need no separate approval

Follows theos-loom's decision 0009 (`docs/decisions/0009-review-fixes-in-a-protected-pr.md` in
`FPGSchiba/theos-loom`), which Jann took on 28 Nov 2026.

## Status

proposed · 2026-11-28 · accepted when Jann merges #65.

## Decision

A PR that already touches protected paths (`docs/gate.md` §5) waits for Jann's approval anyway.
During its own final review loop, its fix waves may change protected paths without asking Jann
first. The PR lists every protected path it changes, and Jann's approval of the PR covers them.

Everything else stays: an implementation PR still never starts touching protected paths
(`CLAUDE.md` hard rule 3), and no fix ever weakens a check (hard rule 4).

## Why

In theos-loom's P00 review, four of seven questions to Jann were approvals of such fix waves, and
they took most of his time.

## Options considered

- Ask Jann before every fix wave that touches a protected path (the rule until now).

## Consequences

`CLAUDE.md` (hard rule 3, "When to stop and ask Jann"), the theos-phase skill, the PR template's
"Protected paths" section, `docs/gate.md` §7 and `CONTRIBUTING.md` §3 say so. Review fixes still
go through the full loop, so a fresh reviewer reads each of them.
