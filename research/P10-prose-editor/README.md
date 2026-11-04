# P10 · Prose editor research

The evidence behind the decision record [`0001-prose-editor.md`][record] (phase P10, CE-06;
[phase spec][spec]). The research is throwaway: nothing here is imported by a later phase. P12
may port patterns and the byte-test method, and may delete this folder once the manuscript editor
exists (spec §5).

## Summary

1. Five candidates were scored on paper. CodeMirror 6 and ProseMirror/Tiptap (by Jann's decision
   on a tie with Milkdown) were prototyped and measured; ProseMirror was built as plain
   ProseMirror with `prosemirror-markdown`.
2. Bytes: CodeMirror 6 saved 17 of 17 unedited files identical and matched the oracle on 60 of 60
   edits, in Chromium and WebKit. ProseMirror reached 5 of 17 and 8 of 60 with its stock
   serializer, and 17 of 17 and 57 of 60 with the best-effort save path, at 3325 adapter lines
   against 1332.
3. Final totals: CodeMirror 6 44, ProseMirror/Tiptap 34 of 54, no knockout. The lead comes from
   byte stability, link chips and callouts.
4. Long documents: both meet spec §4.8 on the 250 KB file. ProseMirror's 2.5 MB stress file opens
   in 2.3 s and types at medians of 140–184 ms.
5. Recommendation: CodeMirror 6, proposed. Spelling marks and OS input methods in the real
   webviews were not automated; see the manual checklist below, then P11.

## Where things are

| Path | What |
|---|---|
| [`comparison.md`][comparison] | CE-06.1: the matrix, the argument for every score, the sources (`Stage: final`) |
| [`dossiers/`][dossiers] | the researchers' facts, one file per candidate and one for the webviews |
| [`corpus/`][corpus] | the corpus of CE-06.2 with `manifest.json`; `generate-long.ts` for the long documents |
| [`report/`][report] | results of the measurement run: round trip, features, input, integration and long documents (`.md` and `.json`), licences, adapter size |
| [`app/`][app] | the harness (Vite, Svelte 5, TypeScript), the prototypes in `app/src/adapters/` with a `NOTES.md` each, the tests in `app/tests/` |

## How to run

From `research/P10-prose-editor/app/`, with Node 24 or later (`engines`). Node 25 and later no
longer ship corepack ([Node.js docs][corepack]); there, run `npm install -g corepack` first.

```text
corepack enable
corepack pnpm install
corepack pnpm exec playwright install chromium webkit
node ../corpus/generate-long.ts
corepack pnpm test
```

- Where `corepack enable` lacks the rights, skip it: every command here already runs pnpm as
  `corepack pnpm …`.
- `generate-long.ts` writes `corpus/generated/{typical,large,stress}.md` (ignored by git) to open
  by hand. The long-documents suite builds the same text itself.
- `pnpm test` runs the unit and document checks (Vitest), every browser suite in Chromium and
  WebKit (Playwright), then the report checks; `test:unit`, `test:e2e` and `test:reports` run
  one part each. A byte mismatch or a failed feature check is recorded as a result, not as a
  test failure; `P10_STRICT=1` makes it fail. `P10_VARIANTS=a,b` limits the browser suites to the
  named variants (`codemirror`, `prosemirror:stock`, `prosemirror:best-effort`, `baseline`).
- The reports in `report/` are rewritten only when `P10_REPORT=1` is set (in PowerShell:
  `$env:P10_REPORT = '1'`). The committed ones come from two runs, because the timings need the
  machine to themselves:
  `P10_REPORT=1 corepack pnpm exec playwright test --grep-invert @long`, then
  `P10_REPORT=1 corepack pnpm exec playwright test long-docs --workers=1` (about 15 minutes).
  `node scripts/licences.ts` writes `licences.md` and `adapter-size.md`.
- By hand: `corepack pnpm dev` and open <http://localhost:5179>. Pick a corpus file and a variant;
  the toolbar switches between Write and Source. In the browser console, `__harness.text()`
  returns the file as the editor would save it.

## Manual checklist (spec §4.7)

For Jann before accepting the record. It checks what no automated test could: the spelling marks
the browser draws, and real OS input methods. Run it in the variant `codemirror`. That takes
about five minutes with the Swiss German layout and a Japanese IME already installed; adding
them, or repeating the steps in `prosemirror:best-effort` to compare, takes longer.

1. Start `corepack pnpm dev` and open <http://localhost:5179> in Microsoft Edge, the Chromium
   engine that WebView2 embeds. Pick `records/lore/topics/synthetic-links-explaining.md`. Click
   into the text before typing: a `contenteditable` focused only by script gets no spelling marks
   in WebView2 (WebView2Feedback #5728).
2. **Spelling.** Type a misspelled word, such as `recieve`, into a paragraph. Expected: a red
   underline, and a right-click offers suggestions; picking one replaces only that word. This is
   the CodeMirror risk the decision record lists for P11: reports of no marks in a CodeMirror
   editor on Chromium 152. Chips get no marks. Then open `records/story/scripts/S-004.md`. In
   Write mode the attribute lines of a beat are a row widget, and a click opens the row's input,
   so put the caret in the `## Beat 1` heading and press ArrowDown until the raw lines show. Go
   down to `when:: T1`, press End and type ` recieve`: no mark there either (Ctrl+Z takes it
   back).
3. **Dead keys, Swiss German layout.** Add the layout under Windows Settings, Time & language,
   Language & region, if needed, and switch to it with Win+Space. Type `^` then `e`, and `¨` then
   `u`, in three places: in a paragraph, right after the chip in
   `A link to a record: [[fates-shrine]]`, and in a voice block of
   `records/lore/topics/synthetic-voices-in-world.md`, in the line that begins
   `Sing it wrong`. Expected: `ê` and `ü` once each, no stray accent, and the chip still whole.
   Switch to Source to see the text as written. Whether WebView2 sends composition events for
   dead keys is unsettled: a 2016 statement says Windows browsers fire none, UI Events says they
   must, and no current source settles it ([webviews dossier][webviews]). The automated
   `I-deadkey-*` checks sent a CDP composition in Chromium, the shape WebKit uses on macOS (WebKit
   167880). Neither the Windows nor the WebKitGTK path was driven, so this step is the only test
   of the Windows path.
4. **Windows IME.** Add Japanese (Microsoft IME), switch with Win+Space, and type `nihongo`,
   Space, Enter in the same three places. Expected: `日本語` once, its first character present
   (WebView2Feedback #5625 drops it in some IMEs), nothing doubled, the chip still whole.
5. **Safari**, if a Mac is at hand: run the harness there and repeat steps 2 to 4 with the macOS
   input sources. Turn on Edit, Spelling and Grammar, Check Spelling While Typing if it is off.

Note what you saw in the P10 PR. A failure joins the list of what P11 checks in the real
webviews (decision record, Consequences for P12).

[record]: ../../docs/decisions/0001-prose-editor.md
[spec]: ../../docs/specs/phases/P10-prose-editor.md
[comparison]: comparison.md
[dossiers]: dossiers/
[webviews]: dossiers/webviews.md
[corepack]: https://nodejs.org/dist/latest/docs/api/corepack.html
[corpus]: corpus/
[report]: report/
[app]: app/
