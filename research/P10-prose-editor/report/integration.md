# integration report

- commit: `0f70ac5adb7e54da59846352592bb4c6a451502e`
- os: Windows_NT 10.0.26300 x64
- chromium: 156.0.8078.4
- webkit: 27.2
- date: 2026-11-04T12:26:24.274Z

Checks of how the editor fits the app. A check is `ok` or `FAIL`; `error` means it could not run (the error is listed),
and `gap` is a gap of the tooling that no variant can close. A failed check is a result, not a test failure.

## Summary

| variant | engine | checks | ok | failed | errors | gaps |
|---|---|---|---|---|---|---|
| codemirror | chromium | 6 | 6 | 0 | 0 | 0 |
| codemirror | webkit | 6 | 6 | 0 | 0 | 0 |
| prosemirror:best-effort | chromium | 6 | 6 | 0 | 0 | 0 |
| prosemirror:best-effort | webkit | 6 | 6 | 0 | 0 | 0 |
| prosemirror:stock | chromium | 6 | 4 | 2 | 0 | 0 |
| prosemirror:stock | webkit | 6 | 4 | 2 | 0 | 0 |

## Checks

| check | codemirror · chromium | codemirror · webkit | prosemirror:best-effort · chromium | prosemirror:best-effort · webkit | prosemirror:stock · chromium | prosemirror:stock · webkit |
|---|---|---|---|---|---|---|
| `N-external-caret` | ok | ok | ok | ok | FAIL | FAIL |
| `N-history` | ok | ok | ok | ok | ok | ok |
| `N-keymap-save` | ok | ok | ok | ok | ok | ok |
| `N-keymap-undo-redo` | ok | ok | ok | ok | FAIL | FAIL |
| `N-reactive` | ok | ok | ok | ok | ok | ok |
| `N-remount` | ok | ok | ok | ok | ok | ok |

## Failed checks

### prosemirror:stock · chromium

- `N-external-caret`: the text is not the original with both external inserts
- `N-keymap-undo-redo`: ControlOrMeta+z did not take the edit back

### prosemirror:stock · webkit

- `N-external-caret`: the text is not the original with both external inserts
- `N-keymap-undo-redo`: ControlOrMeta+z did not take the edit back

Every row is in the JSON file next to this one.
