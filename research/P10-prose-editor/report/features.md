# features report

- commit: `7e0d7281b5f036b71e72895b6b6d8bbd6a1d2e2e`
- os: Windows_NT 10.0.26300 x64
- chromium: 156.0.8078.4
- webkit: 27.2
- date: 2026-11-04T12:13:53.671Z

Checks of a rendered feature of the editor. A check is `ok` or `FAIL`; `error` means it could not run (the error is listed),
and `gap` is a gap of the tooling that no variant can close. A failed check is a result, not a test failure.

## Summary

| variant | engine | checks | ok | failed | errors | gaps |
|---|---|---|---|---|---|---|
| codemirror | chromium | 17 | 17 | 0 | 0 | 0 |
| codemirror | webkit | 17 | 17 | 0 | 0 | 0 |
| prosemirror:best-effort | chromium | 17 | 17 | 0 | 0 | 0 |
| prosemirror:best-effort | webkit | 17 | 17 | 0 | 0 | 0 |
| prosemirror:stock | chromium | 17 | 15 | 2 | 0 | 0 |
| prosemirror:stock | webkit | 17 | 15 | 2 | 0 | 0 |

## Checks

| check | codemirror · chromium | codemirror · webkit | prosemirror:best-effort · chromium | prosemirror:best-effort · webkit | prosemirror:stock · chromium | prosemirror:stock · webkit |
|---|---|---|---|---|---|---|
| `F-beat-finding` | ok | ok | ok | ok | ok | ok |
| `F-beat-heading` | ok | ok | ok | ok | ok | ok |
| `F-beat-row` | ok | ok | ok | ok | ok | ok |
| `F-design` | ok | ok | ok | ok | ok | ok |
| `F-dev-word` | ok | ok | ok | ok | ok | ok |
| `F-frontmatter-hidden` | ok | ok | ok | ok | ok | ok |
| `F-link-alias` | ok | ok | ok | ok | ok | ok |
| `F-link-atomic` | ok | ok | ok | ok | ok | ok |
| `F-link-broken` | ok | ok | ok | ok | ok | ok |
| `F-link-chip` | ok | ok | ok | ok | ok | ok |
| `F-link-popover-escape` | ok | ok | ok | ok | FAIL | FAIL |
| `F-link-popover-kind-filter` | ok | ok | ok | ok | ok | ok |
| `F-link-popover-prose` | ok | ok | ok | ok | ok | ok |
| `F-live-preview` | ok | ok | ok | ok | ok | ok |
| `F-note` | ok | ok | ok | ok | ok | ok |
| `F-source-mode` | ok | ok | ok | ok | FAIL | FAIL |
| `F-voice` | ok | ok | ok | ok | ok | ok |

## Failed checks

### prosemirror:stock · chromium

- `F-link-popover-escape`: the text differs from the oracle insert of "[[son" at 12:69
- `F-source-mode`: typing into the front matter changed the text at 23:10

### prosemirror:stock · webkit

- `F-link-popover-escape`: the text differs from the oracle insert of "[[son" at 12:69
- `F-source-mode`: typing into the front matter changed the text at 23:10

Every row is in the JSON file next to this one.
