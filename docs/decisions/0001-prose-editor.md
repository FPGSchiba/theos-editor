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
and the reports of the run of 2026-11-03 (commit 6b22b2b, Windows, Chromium 156, WebKit 27.2).

- **Totals: CodeMirror 6 44, ProseMirror/Tiptap 34** of 54 (42 and 33 on paper), no knockout.
  The 10 points come from byte stability (6), link chips (2) and callouts (2); every other
  criterion scores the same. Milkdown (33), Lexical (31) and Monaco (25) were scored on paper.
- **Byte-stable Markdown** (criterion 1, weight 3; G5): CodeMirror 6 3, ProseMirror 1
  ([`round-trip.md`][round-trip]). CodeMirror 6 kept all 17 unedited corpus files and all 60
  edits identical to the oracle, through commands and real input, in both engines; its adapter
  writes nothing of its own. ProseMirror stock kept 5 of 17 and 8 of 60. ProseMirror best effort
  (untouched blocks keep their source text, change steps are mapped onto it) kept 17 of 17 and
  57 of 60; the misses are the pastes. It scores 1 for the class "the bytes between blocks have no
  node in a ProseMirror tree": six variants, three left unpatched by controller ruling 26. In one,
  Enter at the end of a voice or design callout's last paragraph, then typing, writes the whole
  callout again and turns a line break inside its paragraph into a space
  ([best-effort notes][pm-be-notes]).
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
  ProseMirror (spec §4.8: under 1 s and 50 ms). The 2.5 MB stress file opens in 80 ms and 2.3 s,
  with typing medians of 22–25 ms and 140–184 ms.
- **Licences** ([`licences.md`][licences]): all pass. One flag, `argparse` 2.0.1 (Python-2.0,
  OSI-approved and permissive), comes in through markdown-it, so ProseMirror only.
- **Not scored:** ProseMirror's stock schema holds a soft line break as a space, so its Write mode
  cannot show line breaks as written (spec §3).
- **Readings the scores rest on** (comparison, Top two and Measured scores; the rulings are
  listed in the P10 PR). ProseMirror was prototyped by Jann's decision of 2026-10-31 on a paper tie
  with Milkdown. Two rulings were made at scoring time, after the results: ruling 29 (a recorded
  defect is a known bug only on a construct the corpus holds, or an everyday edit of one) decided
  ProseMirror's criteria 3 and 4; ruling 30 (the stress file is usable if it opens, typing lands
  and the editor does not hang) decided its criterion 7. With every listed reading turned
  ProseMirror's way it has 43 against 46; each reading applied to both alike keeps the order.
- **Unequal probing:** three review rounds probed Enter at block boundaries in ProseMirror; none
  probed Enter in CodeMirror 6's callouts and lists. CodeMirror 6's record there is clean partly
  because nobody looked, though its Enter only inserts a newline into the text, which is the file.

## Consequences for P12

P12 ports the prototype's patterns ([CodeMirror notes][cm-notes]); it imports nothing (spec §5).

**What P12 builds on top**

- Front matter: a `transactionFilter` that drops a change inside it and moves a crossing change to
  the body start. The stock `changeFilter` is not enough: it lets an insertion at offset 0 through
  and loses the typed text of select-all-and-type. Source mode is a compartment swap in the same
  view, so text, history and selection carry over.
- Rendering: a view plugin over the `@lezer/markdown` tree hides markers on lines no selection
  touches; chips are `Decoration.replace` widgets mounting `LinkChip`, also passed as
  `atomicRanges`; callouts are line decorations with widget heads; beat rows are block widgets
  from a state field mounting `BeatRow`, and a row edit is one transaction on its line.
- Host: `history()` in a compartment, replaced on save; an external change as one transaction kept
  out of the history; the app's keymap in `keymap.of` inside a compartment.
- Byte tests: spec §5 recommends porting the method (whole-file oracle, edits through commands and
  real input, byte comparison) to prove CE-34.1 in `editor-ui/`, re-run on real records once they
  exist (spec §7) and on every upgrade of a pinned version.

**Known gaps and risks**

- CodeMirror draws only the viewport, so P12's UI tests must scroll before reading or acting off
  the caret (controller rulings 19 and 22; the scroll-aware pattern in [`checks.ts`][checks]).
- Enter in a callout does not continue `> `; the new line is a lazy continuation of the paragraph.
  The stock fix is `insertNewlineContinueMarkup` (`@codemirror/lang-markdown`, researched 6.5.2).
  P12 probes Enter in callouts and lists, which P10 did not.
- The editor does not regain focus after a beat-row commit; a redrawn row loses an open input; an
  unknown key cannot be edited from the row.
- Links in voice titles are chips only while the title is raw: `VoiceHead` has none.
- Not tried: a composition inside a typed `[[query` or in a callout title.
- Undo and redo pass no filter: harmless while the front matter is protected in both modes, a risk
  if P12 lets Source mode edit it.
- The editor parses for display only; links and findings come from `theos-content` (spec §7).
- Upstream: one maintainer wrote 120 of the 124 `@codemirror/view` commits in the 12 months
  researched.

**What P11 must check in the real webviews** (Playwright's engines are not the Tauri webviews;
facts in the [webviews dossier][webviews]):

- Spelling marks: WebView2 shows none in a script-focused `contenteditable` until a click
  (WebView2Feedback #5728). A forum report saw none in a CodeMirror editor under Electron, where
  the maintainer saw them in Linux Chromium (comparison, section CodeMirror 6). WKWebView has
  spell check off by default; the app must set `WebContinuousSpellCheckingEnabled` before the
  webview exists (Tauri #7705). WebKitGTK is off until enabled through `with_webview`.
- WKWebView's smart quote and dash substitution may rewrite typed characters (G5).
- IME: WebView2 can drop the first character of a CJK composition (WebView2Feedback #5625). wry
  turns WebKitGTK preedit off, so a composition may end in `compositionend` without
  `compositionstart`. Playwright could not drive WebKit composition (a gap row of
  [`input.md`][input]).
- Dead keys: on Windows a German-layout dead key can panic tao, Tauri's windowing layer
  (tao #1408). On Windows a dead key arrives as a `Dead` keydown plus plain text input, not as a
  composition (UI Events and a 2016 statement; the dossier found no current source). The
  `I-deadkey-*` checks cover the macOS and WebKitGTK path only; the manual checklist of the
  [research README][research] tries the Windows path once.

## Alternative

**ProseMirror/Tiptap**, 34 points, built as plain ProseMirror (`prosemirror-view` 1.42.9,
`prosemirror-markdown` 1.13.9, no Tiptap) with the best-effort save path. In its favour: every
manifest edit but the two pastes reached span level in both engines, and so did every
composition and dead key in Chromium; it matches CodeMirror 6 on front matter, beats, Svelte 5
integration and long documents; its document is a tree of the constructs P12 renders. It would
cost instead
([best-effort notes][pm-be-notes], [stock notes][pm-notes]):

- G5 resting on a 988-line save mapping beside the tree, whose separator class has a named cause
  and no named fix: three variants (M1, M2, O2) are open, and a new kind of boundary edit can add
  one that only the byte tests would find.
- Plain-text paste and a soft-break node in schema and parser, named but not built; the node is
  also what spec §3's line breaks as written need.
- Still to build: chips in raw callout titles, beat headings and attribute lines; Enter in
  callouts; an undo history that survives a Write/Source switch. Needed only because the document
  is a tree: a 322-line map from file offsets to tree positions.
- A whole-document DOM: the stress file opens in 2.3 s and types about seven times slower than in
  CodeMirror 6.
- The same single core maintainer as CodeMirror's, more open composition (13) and WebKit (18)
  issues, and `prosemirror-markdown` patch releases that changed the serializer's output.

## If rejected

Jann names the reason. If ProseMirror/Tiptap does not share it, this record is rewritten for
ProseMirror/Tiptap (plain ProseMirror with the best-effort save path) in the same PR. If it shares
the reason, P10 goes back to the comparison with the reason as a new criterion (spec §4.9).

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
