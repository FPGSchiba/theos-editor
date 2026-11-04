# 0001 · Prose editor component

The component the manuscript editor of P12 is built on (CE-34: "on the component chosen in P10").
This is the recommendation of phase P10 (CE-06.3, [phase spec][spec] §4.9). The evidence is in
[`research/P10-prose-editor/`][research]; no later phase imports from it.

## Status

proposed · 2026-11-04 · accepted when Jann merges the P10 PR,
`docs(ui): prose editor research (P10)`.

## Decision

P12 builds the manuscript editor on **CodeMirror 6** as a live preview. The Markdown source text
is the document; link chips, callouts and beat rows are decorations and widgets drawn over it.

Versions, as researched and measured (variant `codemirror` in [`variants.ts`][variants], pinned in
[`package.json`][package]): `@codemirror/view` 6.43.15, `@codemirror/state` 6.7.6,
`@codemirror/commands` 6.11.1, `@codemirror/language` 6.13.1, `@lezer/markdown` 1.8.1.

## Why

Quoted from [`comparison.md`][comparison] (`Stage: final`, sections Matrix and Measured scores)
and the reports (Windows, Chromium 156, WebKit 27.2): round trip, input and long documents from
Task 17's run of 2026-11-03 on commit 6b22b2b; features and integration from the rerun of
2026-11-04 on commit 0f70ac5 (ruling 37), with verdicts identical to Task 17's.

- **Totals: CodeMirror 6 42, ProseMirror/Tiptap 32** of 54 (42 and 33 on paper), no knockout.
  The 10 points come from byte stability (6), link chips (2) and callouts (2); every other
  criterion scores the same. Milkdown (33), Lexical (31) and Monaco (25) were scored on paper
  only. ProseMirror stays the runner-up although its measured 32 is below Milkdown's paper 33
  (ruling 33). Spec §4.4 replaces paper scores with measured ones for the top two only, so the
  totals do not stand on one footing, and it names a paper candidate only on a knockout. Milkdown
  sits on ProseMirror's tree model, so the separator class below would likely meet it too, and
  likely the raw-line findings; neither is measured.
- **Byte-stable Markdown** (criterion 1, weight 3; G5): CodeMirror 6 3, ProseMirror 1
  ([`round-trip.md`][round-trip]). CodeMirror 6 kept all 17 unedited corpus files and all 60
  edits identical to the oracle, through commands and real input, in both engines; its adapter
  writes nothing of its own. ProseMirror stock kept 5 of 17 and 8 of 60. ProseMirror best effort
  (untouched blocks keep their source text, change steps are mapped onto it) kept 17 of 17 and
  57 of 60; the 17 hold by construction, because it saves the loaded text while no change is
  pending, so they do not measure its parsing; the misses are the pastes. It scores 1 for the
  class "the bytes between blocks have no node in a ProseMirror tree": six variants, three left
  unpatched by controller ruling 26. In one, Enter at the end of a voice or design callout's last
  paragraph, then typing, writes the whole callout again and turns a line break inside its
  paragraph into a space ([best-effort notes][pm-be-notes]).
- **Cost:** 1332 adapter lines for CodeMirror 6; 3325 for ProseMirror best effort, 988 of them
  the save mapping ([`adapter-size.md`][adapter-size]).
- **Chips and callouts:** every feature and integration check passes for CodeMirror 6 and for
  ProseMirror best effort ([`features.md`][features], [`integration.md`][integration]), but
  ProseMirror scores 1 on both, CodeMirror 6 2. ProseMirror's chips exist only in prose: a raw
  callout title, beat heading or attribute line shows its links as text. Enter at the end of a
  callout paragraph does nothing, or adds a caption nobody asked for ([stock notes][pm-notes],
  [best-effort notes][pm-be-notes]).
- **Long documents** (both 3; [`long-docs.md`][long-docs], Chromium): the 250 KB file opens in
  38 ms with a typing p95 of 15–16 ms in CodeMirror 6, and in 273 ms with 21–24 ms in
  ProseMirror (spec §4.8: under 1 s and 50 ms). The 2.5 MB stress file, ten times the large file,
  which is already more than any expected record (spec §4.2), opens in 80 ms and 2.3 s, with
  typing medians of 22–25 ms and 140–184 ms.
- **Licences:** all pass ([`licences.md`][licences]); the one flag, `argparse` 2.0.1
  (Python-2.0, permissive), is ProseMirror's.
- **Not scored:** ProseMirror's stock schema holds a soft line break as a space, so its Write mode
  cannot show line breaks as written (spec §3).
- **Readings and rulings the scores rest on** (comparison, Top two and Measured scores; the rulings
  are listed in the P10 PR). ProseMirror was prototyped by Jann's decision of 2026-10-31 on a paper
  tie with Milkdown. Five rulings were made at scoring time, after the results (rulings 29, 30, 32,
  34 and 36, all of 2026-11-04). Ruling 29 (a recorded defect is a known bug only on a construct the
  corpus holds, or an everyday edit of one) set ProseMirror's criteria 3 and 4 to 1. Ruling 30 (the
  stress file is usable if it opens, typing lands and the editor does not hang) raised its criterion
  7 to 3. Ruling 32 decides a part the harness could have checked but did not on a reading of the
  code: both prototypes leave spell check on in raw attribute lines and raw callout titles (where
  ProseMirror's link ids are plain text), which `I-spellcheck-off` did not see, because it reads
  only chips and rendered rows. Under ruling 29 that set criterion 6 of both to 1; the first
  measured totals were 44 and 34. Ruling 34 keeps criterion 5 at 2 for both: "editing a parameter
  changes only its line" covers the five keys of data model §9.4, so the unknown key `synthetic::`
  of S-901, which the shared `BeatRow` does not open, is not a parameter, and its raw line stays
  editable; the focus lost after a row commit is a missing convenience; no everyday edit triggers
  the redraw that loses an open input. Ruling 36 keeps ProseMirror's criterion 2 at 3: criterion 2
  judges the front matter bytes through the edits it names (typing, delete at the body start,
  select-all and type, paste, undo) and Source mode in one component. Two defects of the Source-mode
  guard need steps the criterion does not name, so they are recorded costs, by the standard that
  makes CodeMirror 6's filter-free undo a recorded risk: after an external change in Source mode the
  guard keeps its old length, and text typed over a selection that starts in the front matter is
  lost. The comparison lists what other readings would move, in weighted points: ProseMirror's
  criterion 1 at 2 if only the pastes decide, +3; its criteria 3 and 4 at 2 if those defects count
  as limits, +2 each; criterion 6 at 2 for both if the raw-line spell check counts as a limit, +2
  each, or at 3 if the unmeasured parts are also left to P11, +4 each; criterion 5 at 1 for both if
  the beat-row limits count as known bugs, −2 each (CodeMirror 6 40, ProseMirror 30); ProseMirror's
  criterion 2 at 2 if the undo history lost at a mode switch counts, −2, or at 1 if the two guard
  defects count as known bugs, −4 (CodeMirror 6 42, ProseMirror 28); its criterion 7 at 2 if usable
  means a latency bound, −1. With the four that favour ProseMirror (criteria 1, 3 and 4 at 2,
  criterion 6 at 3 for both) it has 43 against 46; with known bugs read broadly for both, 26
  against 36. Each reading applied to both alike keeps the order.
- **Unequal probing:** three review rounds probed Enter at block boundaries in ProseMirror; none
  probed Enter in CodeMirror 6's callouts and lists. CodeMirror 6's record there is clean partly
  because nobody looked, though its Enter only inserts a newline into the text, which is the file.
- **Comments on text ranges** (Jann, 2026-11-04: an important feature, not yet in the umbrella).
  A probe made after the scoring, so not scored ([comments probe][comments]), built the same
  minimal feature in both editors. A comment covers a text range, is stored outside the file in
  file offsets, and must survive edits, undo and redo, outside changes and a reload. All seven
  behaviours pass in both editors.
  - CodeMirror 6 needs 71 lines, almost all library mechanism: a `StateField` of `Decoration.mark`
    ranges that the library maps through every change, and `invertedEffects` for undo. Its
    positions are file offsets.
  - Plain ProseMirror needs 140 lines. Every add, list and edit converts offsets through the
    stock adapter's `PositionMap` and `Tracker`. `prosemirror-history` does not track plugin
    state, so undo takes two heuristics of our own: one brings a deleted comment back when an undo
    puts the same length of text back at its spot, and the other is a separate stack of comment
    actions.
  - In ProseMirror, a range that starts inside a link snaps to the chip, and a range inside one
    link is lost.
  - Tiptap's ready-made Comments extension needs a paid plan, from a private registry
    ([Tiptap Comments install][tiptap-comments], read 2026-11-04).
  - Where comments are stored is open (`docs/status.md`, Needs Jann).

## Consequences for P12

P12 ports the prototype's patterns ([CodeMirror notes][cm-notes]); it imports nothing (spec §5).

**What P12 builds on top**

- Front matter: a `transactionFilter` that drops a change inside it and moves a crossing change to
  the body start. The dossier's probe showed that the stock `changeFilter` is not enough: it lets
  an insertion at offset 0 through and loses the typed text of select-all-and-type; the prototype
  did not test it again. Source mode is a compartment swap in the same view, so text, history and
  selection carry over.
- Rendering: no parser extension was needed. Chips come from a scan of the visible lines,
  callouts from Lezer's `Blockquote`, beats from a scan of the body. A view plugin hides markers on
  lines no selection touches; chips are `Decoration.replace` widgets mounting `LinkChip`, also
  passed as `atomicRanges`; callouts are line decorations with widget heads; beat rows are block
  widgets from a state field mounting `BeatRow`, and a row edit is one transaction on its line.
- `[[` popover (CE-35.1): a state field follows the typed `[[query`; the shared `LinkPopover`
  component is mounted on `document.body` and placed with `coordsAtPos`; the allowed kinds
  come from the line's prefix (`at::` and the other keys). Not `@codemirror/autocomplete`, whose
  list DOM belongs to the library.
- Index and findings (CE-35.2): a context state field set by an effect, so a new index or new
  findings redraw the widgets without re-creating the editor.
- Spell check: CodeMirror defaults to `spellcheck="false"`, so prose turns it on through
  `contentAttributes` and widget hosts carry `spellcheck="false"`. Raw attribute lines and raw
  callout titles are not opted out, although spec §4.7 asks for spell check off on attribute
  lines and ids, so P12 adds `spellcheck="false"` to them, for example with a line decoration.
- External change (CE-34.2): the prototype got it from the harness as ready changes and applied
  them as one transaction kept out of the history (`external.ts`). P12 must compute the changes
  from the reloaded file: `@codemirror/merge` `diff()` was researched at 6.12.3 but not used, so
  it is not in the licence report. The merge offer for a dirty record was never tried.
- History and keymap (CE-34.3, G13): `history()` in a compartment, replaced on save; the app's
  keymap in `keymap.of` inside a compartment.
- Read mode (spec §3) was not built in P10 (spec §4.5); P12 builds it without research behind it.
- Byte tests: spec §5 recommends porting the method (whole-file oracle, edits through commands and
  real input, byte comparison) to prove CE-34.1 in `editor-ui/`, re-run on real records once they
  exist (spec §7) and on every upgrade of a pinned version.

**Known gaps and risks**

- CodeMirror draws only the viewport, so P12's UI tests must scroll before reading or acting off
  the caret (controller rulings 19 and 22; the scroll-aware pattern in [`checks.ts`][checks]).
- Enter in a callout does not continue `> `; the new line is a lazy continuation of the paragraph.
  The stock fix is `insertNewlineContinueMarkup` (`@codemirror/lang-markdown`, researched 6.5.2),
  not tried with the prototype's own `Language` (`index.ts`). P12 probes Enter in callouts and
  lists, which P10 did not.
- Spell check is on in raw attribute lines and raw callout titles (criterion 6 at 1, ruling 32).
  `I-spellcheck-off` read only chips and rendered rows, so P12's test must also read the raw
  lines.
- Beat rows: the editor does not regain focus after a row commit, and a redrawn row loses an open
  input. The beat field rescans the whole body on every document, selection or context change, a
  cost the long-document numbers already include (459 beats in the large file). The shared
  `BeatRow` lets no unknown key be edited from the row (`isBeatKey`); the fix belongs in that
  component, not in the editor.
- Editing an existing link: Write mode has no way to do it (Jann's manual check, 2026-11-04, in
  Chrome and WebView2; reproduced with Playwright in both prototypes). Arrow keys step over a
  chip and a click puts the caret beside it, as criterion 3 asks ("the caret steps over a chip as
  one unit"), so the raw `[[id|text]]` never shows. Only Source mode, or deleting the link and
  typing it again, changes a link. A voice title behaves the same: its links stay chips while the
  title is raw, and `VoiceHead` draws none. The rule comes from the P10 spec, not from the
  umbrella (CE-35 does not ask for it), so P12's spec can replace it. A chip would then show raw
  while the caret is inside it, the way Obsidian's live preview, also built on CodeMirror 6, shows
  links, and a modifier-click would open the target. In the prototype this means leaving out the
  chip's `Decoration.replace` and its atomic range on the lines a selection touches
  (`rawlines.ts`), as the headings already do; P12 then decides what Backspace after a chip does.
  ProseMirror's chips are atom nodes and step the same way.
- Live preview covers few constructs (found the same way). The view plugin styles headings and
  emphasis only (`livepreview.ts`). Plain block quotes, Markdown links and autolinks, inline code,
  lists, code blocks, tables and thematic breaks show as raw Markdown on every line. No criterion
  scores this. P12 draws them with the same mechanism: Lezer's `QuoteMark`, `LinkMark`, `URL`,
  `CodeMark` and similar marks hidden off the caret, plus line or mark classes. ProseMirror draws
  them as nodes.
- A `Design note:` line inside a callout gets no `tu-note`, although the data model allows it.
- Not tried: a composition inside a typed `[[query` or in a callout title. Undoing a pick does
  not reopen the popover.
- Undo and redo pass no filter: harmless while the front matter is protected in both modes, a risk
  if P12 lets Source mode edit it.
- The editor parses for display only; links and findings come from `theos-content` (spec §7).
- Upstream: one maintainer wrote 120 of the 124 `@codemirror/view` commits in the 5½ months to
  the GitHub archive (2026-04-15); later commits are on Forgejo and could not be read.
  ProseMirror's core has the same maintainer.

**What P11 must check in the real webviews** (Playwright's engines are not the Tauri webviews;
facts in the [webviews dossier][webviews]):

- The manual checklist of the [research README][research], run by Jann on 2026-11-04 in the
  variant `codemirror`, in Chrome and in WebView2, passed the steps it ran. Misspelled words got
  marks and suggestions, and Swiss German dead keys and the Japanese Windows IME gave the expected
  text, also right after a chip, which stayed whole. The Safari step was not run. P11 repeats all
  of them in the Tauri shell.
- Spelling marks in a CodeMirror editor on Chromium. A forum report saw none in a CodeMirror 6
  editor under Electron (Chromium 152) while a plain `contenteditable` had them; the maintainer saw
  them in Chrome 152 on Linux; a second user's report of none in Chrome 152 is unanswered
  (comparison, section CodeMirror 6). Jann's check saw marks in Chrome and WebView2, so the report
  did not reproduce there; P11 still checks the Tauri shell.
- WebView2: no marks in a script-focused `contenteditable` until a click (WebView2Feedback
  #5728); the spelling language follows the Windows UI language and ignores `lang` (#5294), which
  matters for English prose on a Windows with a German UI.
- WKWebView has spell check off by default; the app must set `WebContinuousSpellCheckingEnabled`
  before the webview exists (Tauri #7705). Its smart quote and dash substitution may rewrite
  typed characters (G5). WebKitGTK spell check is off until enabled through `with_webview`.
- IME: WebView2 can drop the first character of a CJK composition (WebView2Feedback #5625), and
  CJK IMEs can freeze on the first focus of a field that holds text (Tauri #15436). wry turns
  WebKitGTK preedit off, so by code reading, not run, a composition may arrive as `compositionend`
  without `compositionstart`. Playwright could not drive WebKit composition (a gap row of
  [`input.md`][input]).
- Dead keys: a German-layout dead key can panic tao, Tauri's windowing layer, on Windows (tao
  #1408, open, one report). Whether WebView2 sends composition events for dead keys is unsettled:
  a 2016 statement says Windows browsers fire none, UI Events says they must, and no current
  source settles it. The `I-deadkey-*` checks sent a CDP composition in Chromium, the shape WebKit
  uses on macOS (WebKit 167880). The Windows path was tried only by hand (Jann's check above),
  the WebKitGTK path not at all.

## Alternative

**ProseMirror/Tiptap**, 32 points, the runner-up by ruling 33 although Milkdown has 33 on paper
(see Why), built as plain ProseMirror (`prosemirror-view` 1.42.9, `prosemirror-markdown` 1.13.9,
no Tiptap) with the best-effort save path. In its favour: every
manifest edit but the two pastes reached span level in both engines, and so did every
composition and dead key in Chromium. It matches CodeMirror 6 on front matter, beats, Svelte 5
integration and long documents, and its front matter and modes took 57 lines against 144
([`adapter-size.md`][adapter-size]). The whole document is DOM, so tests and DOM reads need no
scrolling (CodeMirror 6's cost under rulings 19 and 22), and the document is a tree of the
constructs P12 renders. It would cost instead ([best-effort notes][pm-be-notes],
[stock notes][pm-notes]):

- G5 resting on the 988-line save mapping and its separator class (above): three variants (M1,
  M2, O2) are open, and a new kind of boundary edit can add one that only the byte tests find.
- Plain-text paste and a soft-break node in schema and parser, named but not built; the node is
  also what spec §3's line breaks as written need.
- Still to build: chips in raw lines and Enter in callouts (above), spell check off on raw
  attribute lines and raw titles (CodeMirror 6 needs that too), an undo history that survives
  a Write/Source switch, and a Source-mode guard that follows the front matter's length after an
  external change and keeps text typed over a selection that starts in it (above, ruling 36).
- Already paid in the prototype, and needed only because the document is a tree: a 322-line map
  from file offsets to tree positions.
- Slower on very long files: the stress-file numbers above.
- More open composition (13) and WebKit (18) issues, counted on 2026-10-31, than CodeMirror 6
  had in its GitHub snapshot of 2026-04-15, its last readable count (12 open in all, one on IME);
  and `prosemirror-markdown` patch releases that changed the serializer's output.

## If rejected

Jann names the reason. If ProseMirror/Tiptap does not share it, this record is rewritten for
ProseMirror/Tiptap (plain ProseMirror with the best-effort save path) in the same PR. If it shares
the reason, P10 goes back to the comparison with the reason as a new criterion (spec §4.9); there
Milkdown is the named next candidate (comparison, Measured scores).

[spec]: ../specs/phases/P10-prose-editor.md
[research]: ../../research/P10-prose-editor/README.md
[comparison]: ../../research/P10-prose-editor/comparison.md
[variants]: ../../research/P10-prose-editor/app/src/harness/variants.ts
[package]: ../../research/P10-prose-editor/app/package.json
[round-trip]: ../../research/P10-prose-editor/report/round-trip.md
[features]: ../../research/P10-prose-editor/report/features.md
[integration]: ../../research/P10-prose-editor/report/integration.md
[input]: ../../research/P10-prose-editor/report/input.md
[long-docs]: ../../research/P10-prose-editor/report/long-docs.md
[licences]: ../../research/P10-prose-editor/report/licences.md
[adapter-size]: ../../research/P10-prose-editor/report/adapter-size.md
[cm-notes]: ../../research/P10-prose-editor/app/src/adapters/codemirror/NOTES.md
[pm-notes]: ../../research/P10-prose-editor/app/src/adapters/prosemirror/NOTES.md
[pm-be-notes]: ../../research/P10-prose-editor/app/src/adapters/prosemirror-best-effort/NOTES.md
[webviews]: ../../research/P10-prose-editor/dossiers/webviews.md
[checks]: ../../research/P10-prose-editor/app/tests/e2e/checks.ts
[comments]: ../../research/P10-prose-editor/app/src/probes/comments/NOTES.md
[tiptap-comments]: https://tiptap.dev/docs/comments/getting-started/install
