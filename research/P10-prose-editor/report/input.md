# input report

- commit: `6b22b2b32d8c3669f09c8fb170aaa2b4173a4c88`
- os: Windows_NT 10.0.26300 x64
- chromium: 156.0.8078.4
- webkit: 27.2
- date: 2026-11-03T17:50:21.734Z

Checks of spell check, IME or dead-key input. A check is `ok` or `FAIL`; `error` means it could not run (the error is listed),
and `gap` is a gap of the tooling that no variant can close. A failed check is a result, not a test failure.

## Summary

| variant | engine | checks | ok | failed | errors | gaps |
|---|---|---|---|---|---|---|
| codemirror | chromium | 11 | 11 | 0 | 0 | 0 |
| codemirror | webkit | 6 | 5 | 1 | 0 | 1 |
| prosemirror:best-effort | chromium | 11 | 11 | 0 | 0 | 0 |
| prosemirror:best-effort | webkit | 6 | 5 | 1 | 0 | 1 |
| prosemirror:stock | chromium | 11 | 2 | 9 | 0 | 0 |
| prosemirror:stock | webkit | 6 | 2 | 4 | 0 | 1 |

## Checks

| check | codemirror · chromium | codemirror · webkit | prosemirror:best-effort · chromium | prosemirror:best-effort · webkit | prosemirror:stock · chromium | prosemirror:stock · webkit |
|---|---|---|---|---|---|---|
| `I-deadkey-circ-chip` | ok | - | ok | - | FAIL | - |
| `I-deadkey-circ-prose` | ok | - | ok | - | FAIL | - |
| `I-deadkey-circ-voice` | ok | - | ok | - | FAIL | - |
| `I-deadkey-uml-chip` | ok | - | ok | - | FAIL | - |
| `I-deadkey-uml-prose` | ok | - | ok | - | FAIL | - |
| `I-deadkey-uml-voice` | ok | - | ok | - | FAIL | - |
| `I-ime-ja-chip` | ok | - | ok | - | FAIL | - |
| `I-ime-ja-prose` | ok | - | ok | - | FAIL | - |
| `I-ime-ja-voice` | ok | - | ok | - | FAIL | - |
| `I-ime-webkit` | - | gap | - | gap | - | gap |
| `I-insert-chip` | - | ok | - | ok | - | FAIL |
| `I-insert-prose` | - | ok | - | ok | - | FAIL |
| `I-insert-voice` | - | ok | - | ok | - | FAIL |
| `I-spellcheck-off` | ok | ok | ok | ok | ok | ok |
| `I-spellcheck-prose` | ok | ok | ok | ok | ok | ok |

## Failed checks

### codemirror · webkit

- `I-ime-webkit`: gap: no composition input in Playwright WebKit

### prosemirror:best-effort · webkit

- `I-ime-webkit`: gap: no composition input in Playwright WebKit

### prosemirror:stock · chromium

- `I-deadkey-circ-chip`: compositionstart 1, compositionend 1; the saved text differs from the oracle insert of "ê" at 12:69, 389 byte(s) beyond it
- `I-deadkey-circ-prose`: compositionstart 1, compositionend 1; the saved text differs from the oracle insert of "ê" at 12:69, 391 byte(s) beyond it
- `I-deadkey-circ-voice`: compositionstart 1, compositionend 1; the saved text differs from the oracle insert of "ê" at 16:72, 880 byte(s) beyond it
- `I-deadkey-uml-chip`: compositionstart 1, compositionend 1; the saved text differs from the oracle insert of "ü" at 12:69, 389 byte(s) beyond it
- `I-deadkey-uml-prose`: compositionstart 1, compositionend 1; the saved text differs from the oracle insert of "ü" at 12:69, 391 byte(s) beyond it
- `I-deadkey-uml-voice`: compositionstart 1, compositionend 1; the saved text differs from the oracle insert of "ü" at 16:72, 880 byte(s) beyond it
- `I-ime-ja-chip`: compositionstart 1, compositionend 1; the saved text differs from the oracle insert of "日本語" at 12:69, 389 byte(s) beyond it
- `I-ime-ja-prose`: compositionstart 1, compositionend 1; the saved text differs from the oracle insert of "日本語" at 12:69, 398 byte(s) beyond it
- `I-ime-ja-voice`: compositionstart 1, compositionend 1; the saved text differs from the oracle insert of "日本語" at 16:72, 887 byte(s) beyond it

### prosemirror:stock · webkit

- `I-ime-webkit`: gap: no composition input in Playwright WebKit
- `I-insert-chip`: the saved text differs from the oracle insert of "日本語êü" at 12:69, 389 byte(s) beyond it
- `I-insert-prose`: the saved text differs from the oracle insert of "日本語êü" at 12:69, 402 byte(s) beyond it
- `I-insert-voice`: the saved text differs from the oracle insert of "日本語êü" at 16:72, 891 byte(s) beyond it

Every row is in the JSON file next to this one.
