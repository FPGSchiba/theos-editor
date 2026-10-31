# CodeMirror 6 prototype: notes

Variant `codemirror`, `@codemirror/view` 6.43.15. State after Task 8 (the base), its fix round 1, and Task 9 (links).
`tests/e2e/codemirror-probes.spec.ts` (`@harness`) probes what the measured suites cannot see.

## Still failing under `P10_STRICT=1`

Run: `P10_STRICT=1 pnpm exec playwright test --grep "@base|@integration|@input" --grep-invert "I-[a-z-]*-chip"` with
`P10_VARIANTS=codemirror`. Chromium and WebKit fail the same two checks, and both ask for a feature that a later task of this
prototype builds. Neither is a defect of the links, and nothing was tried against them here, because the feature does not exist yet.
The links suites (`@links`, `I-[a-z-]*-chip`, `N-remount`, `N-reactive`) pass in both engines (30 passed, 4 skipped by engine).

| Check | Why it fails | Closed by |
|---|---|---|
| `@base F-live-preview` | The caret-on-the-line half passes (the raw `## Beat 1 — Arrival` shows). It also wants beat 2 rendered as `[data-beat="2"]` showing `2 · The murmur`, and beat 1 as `1 · Arrival` once the caret leaves. Headings are rendered with their markers hidden, but not as beat headings. | Task 11 (beats) |
| `@input I-spellcheck-off` | The chip half passes now (every `.tu-link` has `spellcheck` off). It still wants `[data-beat-row]` rows with `spellcheck` off; there are none. | Task 11 (beats) |

The two `ScriptedEdit` kinds that are not made yet (`note-state`, `beat-param`) throw from `apply`; their `@callouts` and
`@beats` round-trip edits are not in the suites of Tasks 8 and 9.

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

## Known limits of the base

- A setext heading is styled, but its `====` underline stays raw (hiding it would leave an empty line).
- An unclosed leading `---` is not front matter (as in `splitFrontMatter`), and `bodyStart` then looks again on every change; that costs a scan of the document.
- Nothing in this base has been run on WebView2, WKWebView or WebKitGTK; only the two Playwright engines.
