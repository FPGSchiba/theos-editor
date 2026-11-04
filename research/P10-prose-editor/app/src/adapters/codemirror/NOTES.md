# CodeMirror 6 prototype: notes

Variant `codemirror`, `@codemirror/view` 6.43.15. State after Task 8 (the base), its fix round 1, Task 9 (links), Task 10 (callouts) and Task 11 (beats).
`tests/e2e/codemirror-probes.spec.ts` (`@harness`) probes what the measured suites cannot see.

## Under `P10_STRICT=1`

Run: `P10_STRICT=1 P10_VARIANTS=codemirror pnpm exec playwright test --grep "@beats|@base|@integration|@input|@links|@callouts"`. In the Task 17 run (`report/`)
every round-trip row is identical (60 of 60 edits per engine), and every feature, integration and input check is `ok` but the WebKit IME `gap`.
After Task 11 Chromium and WebKit failed the two checks below; they were the harness driver, and ruling 22 (Task 7c) made it scroll to the row.

| Check | Why it failed | Tried |
|---|---|---|
| `@beats beat-param-add-order via input` (S-901, beat 4), fixed | The `input` driver waited 3 s for `[data-beat-row="4"]` and did not scroll. CodeMirror draws the viewport only (about 13 lines of the 45 of the record, up to beat 3); beat 4 is below the fold, so its row is not in the DOM. | Nothing beyond the feature itself: the whole-document render of Task 9 was dropped on purpose (ruling 19), and the driver may not change (ruling 17). The `api` driver passes. |
| `@beats beat-param-add-no-dash via input` (S-901, beat 5), fixed | The same, for beat 5. | The same. |

## What the base does, and how

- **Front matter** (`frontmatter.ts`). It stays in the document, so every offset is a document offset and
  `text()` is `doc.toString()`. `bodyStart` is a state field that looks again only when a change reaches the front matter or its end.
  Write mode hides it with a `Decoration.replace` from a direct decoration set, makes `[0, bodyStart]` an
  atomic range, and a transaction filter keeps every selection in the body. Both modes share one
  `transactionFilter`: a change inside the front matter is dropped, a change that starts inside and ends in the body starts
  at the body instead. The dossier suggested `changeFilter`; it was not used because its measured behaviour
  lets an insertion at offset 0 through and loses the typed text of a select-all-and-type. Undo and redo
  run with `filter: false`, so no filter sees them; none is needed, because they only invert changes that
  already passed. The filter lets a change through that carries the `fromOutside` annotation (`applyExternal`).
  Write mode hides the lines of the front matter as one empty block, so the first body line is a line of its own and keeps its
  line decoration (an inline replace over the line breaks merged it with the front matter and lost the heading class). The
  caret starts at the body start, because a selection filter only sees transactions and a state created at offset 0 would be
  inside the hidden range. Front matter that is the last thing in the file, without a newline behind the closing fence, is hidden
  inline instead: WebKit puts the caret nowhere after a block that ends the document. Typing at its end gets a newline in front of it
  in the same change, so the closing fence keeps its line and one undo takes both back.
- **Source mode** (`modes.ts`). One compartment holds the Write extensions (hide, live preview) or the Source
  ones (front matter drawn raw with a line class). The same view, text, history and selection carry over;
  switching to Write moves a caret that is in the front matter to the body start.
- **Live preview** (`livepreview.ts`). A view plugin over the Lezer tree of `@lezer/markdown`: heading lines get the
  design system's `ms-title`/`ms-h2`/`ms-h3`, emphasis and strong get a mark, and the markers (`#`+space,
  `*`, `_`) are hidden on every line that no selection touches. The document is never changed.
- **History** (`history.ts`). `history()` sits in a compartment; `markSaved()` takes it out and puts a new one in
  (two dispatches), which leaves `undoDepth` at 0. `canUndo()` is `undoDepth(state) > 0`.
- **External changes** (`external.ts`). One transaction with `Transaction.addToHistory.of(false)`. CodeMirror maps the
  stored undo steps and the selection through it, so the caret keeps its text and undo removes only the local edit.
- **Keymap** (`keymap.ts`). `ctx.keymap` (`Mod-` notation) goes straight into `keymap.of`; it sits in a compartment so
  `updateContext` can swap it. `save` calls `ctx.save()`, `Mod-a` selects the body, and Enter, Backspace
  and Delete are the plain commands (`insertNewline`, `deleteCharBackwardStrict`), so Enter copies no indentation and Backspace never
  deletes an indent unit.
- **Input** (`spellcheck.ts`). `contentAttributes` sets `spellcheck="true"` on the content element (CodeMirror's default is `false`).
- **`apply`** (`edits.ts`, `index.ts`). Each kind is a transaction with the user event of typing, pasting or deleting. `external` is the
  local insert plus `applyExternal` at the shifted offset; `undo-all` applies each edit and then undoes until `canUndo()` is false.

## Links (Task 9)

Files: `links.ts` (chips), `typing.ts` (the link being typed), `popover.ts` (the `[[` popover), `context.ts`.
Chips, the typing state and the popover are part of the Write set of `modes.ts`; Source mode shows every link raw.

- **Chips** (`links.ts`). A view plugin scans the visible lines for `[[id]]` and `[[id|shown]]` and replaces each with a
  `Decoration.replace` whose widget mounts the shared `LinkChip` with `mount` from `svelte` (`toDOM`) and unmounts it in
  `destroy`. The widget is found by its element in a `WeakMap`, because CodeMirror keeps the element of an equal widget and
  hands it on to the newer instance. `eq` compares id, alias, kind and title, so a rebuild of the decorations (every document
  change, every new index) leaves a chip that draws the same thing alone. CodeMirror redraws the whole changed range, so a chip
  next to a changed one may be rebuilt with it (probed). Links in inline code and code blocks stay raw (found by the Lezer tree).
  The bytes are never touched; the chip is drawn over `[[...]]`. Chips appear in prose, callout titles and beat attribute lines.
- **Atomic.** The same decoration set is `EditorView.atomicRanges`. ArrowLeft and ArrowRight, Shift+Arrow, and the deletion commands
  of the keymap (`deleteCharBackwardStrict`, `deleteCharForward`) skip it, so one Backspace behind a chip or one Delete before it
  removes the whole link and one arrow press steps over it. A click on a chip puts the caret before or behind it
  (`ignoreEvent` is false). `caretTo` (programmatic) is not snapped out of a chip: a caret placed inside `[[...]]` stays there.
- **Typing** (`typing.ts`). A state field holds the `[[query` that ends at the caret. It starts when typed text (`input.type`)
  completes a `[[` whose query has no `]`, `[` or `|` and at most 80 characters, goes on while the caret stays in that `[[`, and
  ends otherwise. The text shows raw with the reference screen's `tu-typing__q` mark and no chip is drawn over it; a finished link
  (`]]` typed) becomes a chip at once. Escape sets `dismissed`: the typed text stays plain text and no popover comes back for that `[[`.
- **Popover** (`popover.ts`). One `LinkPopover` is mounted while the typing state is open and not dismissed, into an element on
  `document.body` (position fixed, below the caret from `coordsAtPos` in the measure phase, above it when the page ends too soon,
  moved again on scroll). Options come from `ctx.search(query, place)`; the place is `{ in: 'beat', key }` when the line starts
  with `at::`, `with::`, `says::`, `reveals::` or `when::`, else prose. Picking (Enter or a click) replaces `[[query` with `[[id]]`
  in one transaction (`input.complete`). The component swallows ArrowUp, ArrowDown, Enter and Escape in the capture phase; the editor
  keeps the focus. `onclose` (Escape) dispatches the dismissal and the plugin unmounts the component. The popover is also unmounted
  when typing ends (caret leaves, text changes shape), when the editor loses the focus (otherwise it would keep Enter), and in `destroy`.
- **`apply`** (`edits.ts`). `link-insert` is the typed `[[query` (`input.type`) and then the same completion the popover makes.
  `link-delete` removes `[[` through `]]` in one transaction (`delete.backward`) with the caret where the link was.
- **`updateContext`** (`index.ts`, `context.ts`). The context is a state field set by an effect; the plugin rebuilds the decorations
  on it, so an index change redraws the changed chips and keeps the editor, its history and its caret (`N-reactive`).

Known limits of the links: an IME composition inside `[[query` was not tried (the composition guard of the component only stops its keys); undoing a pick gives back the typed text without reopening the popover; the spell-check half of `I-spellcheck-off` for
beat rows is Task 11; nothing was run on WebView2, WKWebView or WebKitGTK.

## Callouts (Task 10)

Files: `callouts.ts` (the block lines, the heads, the hidden markers), `calloutlook.ts` (the box look), `devwords.ts`, `notestate.ts`
(`note-state`), `rawlines.ts` (which lines a selection touches; `livepreview.ts` uses it now too). The callout plugin
and its look are part of the Write set of `modes.ts`; Source mode shows every callout raw.

- **A block is a run of lines.** The blocks come from the Lezer tree (`Blockquote` whose first line is `> [!voice|design|note]`), so a lazy
  line (`and they will hear you.`, no `>`) belongs to its block, and a block in code is not one. Every line of the block gets one
  `Decoration.line` with `data-callout`, the classes `cm-callout`, `cm-callout-<kind>`, `-first`, `-last` and, for voice, `tu-voice`; the
  note lines also get `data-note` and `data-note-state` (`open` when the title has no state). The box is drawn per line in
  `calloutlook.ts` from the design tokens: the sides on every line, the top on the first, the bottom on the last.
- **Voice.** The title line is replaced (`Decoration.replace`, inline, the whole line content) by a widget that mounts the shared
  `VoiceHead` in `toDOM` and unmounts it in `destroy` (found by element in a `WeakMap`, as the chips are). The last line that starts
  with `— ` is the caption: it gets `tu-voice__by`, its `> — ` is hidden and the design system's `::before` draws the dash (raw, the dash
  is text and the `::before` is switched off). `-- text` is voice text. `VoiceHead` shows the speaker's title only, not chips, so the
  links of a voice title are chips only while the title is raw (the caret on it).
- **Design.** The title line becomes a small `design` label (no shared head exists); the text lines carry `data-callout="design"`.
  **`Design note:`.** A line that starts with it (outside code) gets the class `tu-note`, no `data-callout`.
- **Note.** The title line is replaced by `NoteHead` (id, version chip, state, resolving version chip). The state is the last segment when
  it is `open`, `dropped` or `resolved by [[id]]`; `data-note-state` is `open`, `dropped` or `resolved`.
- **Developer words** (`devwords.ts`). The words of `ctx.developerWords` are marked with `[data-dev-word]` and `title="developer word"` in the
  text lines of voice blocks (caption included) as whole words (`tier` is not marked in `frontier`), case-insensitive. Not in prose, in
  the title, in design or note callouts or in `Design note:` lines.
- **Live preview.** `touchedLines` (whole lines under every selection range) decides per line: untouched, the `> ` marker is a hidden
  replace decoration and the title line is its head; touched, the line shows raw (the title with its link chips, as Task 9 draws them).
  Because a selection always touches its lines, a marker that is hidden is never under the caret or inside a selection: Backspace at the
  start of a body line joins it to the line above (one newline, the `> ` stays and is drawn), Delete at the end of the line above takes
  one newline, and a Shift selection into a hidden line draws it first (probed). A click on a head puts the caret on the title line.
- **`note-state`** (`notestate.ts`). One transaction (`input.note-state`, never joined: one undo step) replaces the last ` · ` segment of the title line
  `> [!note] <note> …` when it is a state and appends ` · <state>` when it is not, as the oracle does; the caret ends behind it on the
  title line, which then shows raw. A note id that is not there throws.
- **The viewport.** CodeMirror draws the viewport only; the harness checks scroll the record to read the whole of it (ruling 19).

Known limits of the callouts: Enter in a callout does not continue the `> ` marker (the base's Enter copies nothing), so a new line is a
lazy line of the block until the author types `> `; a nested quote (`> > `) is one callout line with a hidden outer marker; the voice head
has no chips, see above; a `Design note:` line inside a callout is callout text; callouts in lists are not recognised (the title has to
start at column 0); an IME composition in a callout title was not tried (the `-voice` checks compose in a text line); nothing was run on
WebView2, WKWebView or WebKitGTK.

## Beats (Task 11)

Files: `beats.ts` (finds the beats, plans a `beat-param` change by the oracle's rule), `beatrow.ts` (the widgets and the state field).
`BeatRow.svelte` is keyed by `${index}:${key}` now (a second `at::` while typing no longer throws `each_key_duplicate`).

- **Block decorations come from a state field** (a view plugin cannot provide them); the field is in the Write set of `modes.ts` and is also
  `atomicRanges`. It scans the whole body for `## Beat <n>` (not in code) and the `key:: value` lines directly after it, on every document,
  selection or context change.
- **Heading.** `## Beat <n> — <name>` becomes an inline replace of the line content by `span[data-beat=<n>]` showing `<n> · <name>`. A heading
  without the dash (`## Beat 5 - …`) stays a plain heading line (raw), but its attribute lines still get the row.
- **Row.** One block `Decoration.replace` over the attribute lines (first line start to last line end) mounts the shared `BeatRow` (`mount` in
  `toDOM`, `unmount` in `destroy`); `ignoreEvent` is true, so keys and clicks in the row's input never reach the editor. A beat with no attribute line gets a
  block widget after its heading so that "+ add" has a row. `eq` compares beat, params and the context object: a new index or new findings redraw the
  row (findings come from `ctx.findings`, shown by the component).
- **Raw lines.** `touchedLines` decides: the heading is raw while a selection touches it, and the row gives way to the raw attribute lines while a selection
  touches any of them. A selection that crosses the block touches it, so no delete takes lines the author did not see (probed). The caret stepping into the
  row lands on its edge and turns it raw.
- **Edits.** `oncommit(key, value)` and `apply({ kind: 'beat-param' })` both call `setBeatParam`: one transaction (`input.beat-param`) that replaces the value of an
  existing line (keeping `key::` and its blanks) or inserts `key:: value` after the last present key that precedes it in at, with, says, reveals, when (else after the
  heading). Nothing else changes. The selection is not moved.

Known limits of the beats: a row that is redrawn (context change, edit elsewhere in its lines) loses an open input; the editor does not take the focus back after a
commit; an unknown key (`synthetic::`) is shown but cannot be edited from the row; the rows below the fold are not in the DOM until the editor is scrolled there (CodeMirror draws the viewport; the drivers scroll, ruling 22).

## Known limits of the base

- A setext heading is styled, but its `====` underline stays raw (hiding it would leave an empty line).
- An unclosed leading `---` is not front matter (as in `splitFrontMatter`), and `bodyStart` then looks again on every change; that costs a scan of the document.
- Nothing in this base has been run on WebView2, WKWebView or WebKitGTK; only the two Playwright engines.
