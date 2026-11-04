# long-docs report

- commit: `6b22b2b32d8c3669f09c8fb170aaa2b4173a4c88`
- os: Windows_NT 10.0.26300 x64
- chromium: 156.0.8078.4
- webkit: 27.2
- date: 2026-11-03T18:04:47.191Z

Machine: Intel(R) Core(TM) Ultra 7 270K Plus, 24 logical cores, 31.3 GiB RAM.

Load is the time from `load()` to the second animation frame after it. Typing (median / p95 in ms) is
the time from the keydown's `timeStamp` until the main thread has styled, laid out and painted the
frame after the editor's DOM changed (a form field's `input` event counts as a change), before GPU
raster and present. `typingFrame` is the same keystroke ended earlier, in the animation frame callback
of that frame, which runs before its style, layout and paint: a lower bound for `typing`. WebKit
delivers the message that ends `typing` about two frames after that callback, so its `typing` has a
floor of about 30 ms (see the typical file): compare it between variants, and read the cost of the
editor itself from `typingFrame`. There are 200 keystrokes per position, typed into prose: the start (the intro
paragraph), the middle and the end (the paragraph of the last beat), in the order end, middle, start so
that earlier typing does not move the later places. Each keystroke is sent to an idle page after a
pause of 0 to 16 ms.
Scroll is the time from jumping to the end of the document until its last line is drawn (`>` when it never
was within the limit). Heap is `performance.memory.usedJSHeapSize` after a garbage collection, Chromium
only. The numbers are evidence, not a gate (spec §4.8); a headless run on a loaded machine is slower than
the app.

## Results

| variant | engine | file | load ms | typing start (median / p95 ms) | typing middle | typing end | typingFrame start | typingFrame middle | typingFrame end | scroll ms | heap MB |
|---|---|---|---|---|---|---|---|---|---|---|---|
| codemirror | chromium | typical (5 KB) | 31 | 2.4 / 14 | 2.7 / 14 | 2.7 / 14 | 1.7 / 13 | 1.9 / 13 | 1.9 / 14 | 21 | 21.7 |
| codemirror | chromium | large (250 KB) | 38 | 5.1 / 16 | 4.5 / 16 | 4.4 / 15 | 4.4 / 15 | 3.9 / 16 | 3.7 / 14 | 30 | 24.5 |
| codemirror | chromium | stress (2.5 MB) | 80 | 25 / 29 | 24 / 29 | 22 / 27 | 24 / 29 | 23 / 28 | 22 / 26 | 21 | 50.4 |
| codemirror | webkit | typical (5 KB) | 44 | 30 / 32 | 30 / 32 | 31 / 37 | 3.0 / 4.0 | 3.0 / 4.0 | 3.0 / 10 | 27 | - |
| codemirror | webkit | large (250 KB) | 55 | 30 / 44 | 30 / 32 | 30 / 44 | 4.0 / 13 | 5.0 / 7.0 | 5.0 / 14 | 39 | - |
| codemirror | webkit | stress (2.5 MB) | 101 | 46 / 48 | 46 / 48 | 46 / 62 | 25 / 28 | 24 / 28 | 26 / 31 | 42 | - |
| prosemirror:best-effort | chromium | typical (5 KB) | 32 | 1.7 / 14 | 1.9 / 13 | 1.9 / 14 | 1.0 / 13 | 1.1 / 12 | 1.2 / 13 | 4.4 | 21.7 |
| prosemirror:best-effort | chromium | large (250 KB) | 273 | 13 / 22 | 16 / 21 | 18 / 24 | 8.1 / 16 | 10 / 15 | 12 / 19 | 1.7 | 56.8 |
| prosemirror:best-effort | chromium | stress (2.5 MB) | 2279 | 140 / 184 | 156 / 181 | 184 / 228 | 87 / 111 | 104 / 117 | 130 / 159 | 25 | 364.0 |
| prosemirror:best-effort | webkit | typical (5 KB) | 45 | 30 / 32 | 30 / 32 | 30 / 32 | 2.0 / 2.0 | 2.0 / 3.0 | 2.0 / 3.0 | 2.0 | - |
| prosemirror:best-effort | webkit | large (250 KB) | 916 | 30 / 32 | 30 / 32 | 31 / 32 | 4.0 / 7.0 | 4.0 / 7.0 | 5.0 / 7.0 | 5.0 | - |
| prosemirror:best-effort | webkit | stress (2.5 MB) | 7892 | 62 / 77 | 62 / 78 | 63 / 79 | 31 / 39 | 34 / 42 | 36 / 47 | 29 | - |
| prosemirror:stock | chromium | typical (5 KB) | 30 | 2.3 / 15 | 2.5 / 16 | 2.2 / 14 | 1.2 / 14 | 1.5 / 15 | 1.4 / 14 | 13 | 21.7 |
| prosemirror:stock | chromium | large (250 KB) | 286 | 15 / 21 | 18 / 26 | 22 / 27 | 9.3 / 14 | 12 / 18 | 15 / 20 | 25 | 56.8 |
| prosemirror:stock | chromium | stress (2.5 MB) | 2234 | 147 / 191 | 175 / 222 | 200 / 245 | 90 / 120 | 117 / 146 | 142 / 172 | 26 | 364.0 |
| prosemirror:stock | webkit | typical (5 KB) | 51 | 30 / 32 | 30 / 32 | 30 / 32 | 2.0 / 3.0 | 2.0 / 3.0 | 2.0 / 3.0 | 2.0 | - |
| prosemirror:stock | webkit | large (250 KB) | 938 | 30 / 32 | 30 / 31 | 30 / 32 | 4.0 / 7.0 | 6.0 / 10 | 5.0 / 8.0 | 7.0 | - |
| prosemirror:stock | webkit | stress (2.5 MB) | 8924 | 62 / 77 | 62 / 78 | 62 / 78 | 32 / 38 | 35 / 44 | 36 / 45 | 29 | - |

## Errors

No errors.

Every row is in the JSON file next to this one.
