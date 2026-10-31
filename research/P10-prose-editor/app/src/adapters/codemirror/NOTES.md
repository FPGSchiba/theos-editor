# CodeMirror 6 prototype: notes

Variant `codemirror`, `@codemirror/view` 6.43.15. State after Task 8 (the base) and its fix round 1.
`tests/e2e/codemirror-probes.spec.ts` (`@harness`) probes what the measured suites cannot see.

## Still failing under `P10_STRICT=1`

Run: `P10_STRICT=1 P10_VARIANTS=codemirror pnpm exec playwright test --grep "@base|@integration|@input" --grep-invert "I-[a-z-]*-chip"`.
Chromium and WebKit fail the same four checks, and every one of them asks for a feature that a later
task of this prototype builds. None of them is a defect of the base, and nothing was tried against
them here, because the feature does not exist yet.

| Check | Why it fails | Closed by |
|---|---|---|
| `@base F-live-preview` | The caret-on-the-line half passes (the raw `## Beat 1 — Arrival` shows). It also wants beat 2 rendered as `[data-beat="2"]` showing `2 · The murmur`, and beat 1 as `1 · Arrival` once the caret leaves. Headings are rendered with their markers hidden, but not as beat headings. | Task 11 (beats) |
| `@input I-spellcheck-off` | Wants `.tu-link` chips and `[data-beat-row]` rows with `spellcheck` off; there are none. `I-spellcheck-prose` passes. | Tasks 9 and 11 |
| `@integration N-remount` | Wants one `.tu-link` chip per link of the file after 20 loads (0 now, 13 expected). Mount, teardown, one editor in `.hx-adapter` and a clean console pass. | Task 9 (links) |
| `@integration N-reactive` | Wants the chip of `[[mother-ilse]]` to show the title the index pushes in. Text, caret and undo depth are not judged until a chip exists. | Task 9 (links) |

The four `ScriptedEdit` kinds the base does not make (`link-insert`, `link-delete`, `note-state`,
`beat-param`) throw from `apply`; the `@links`, `@callouts` and `@beats` round-trip edits are not in this
task's suites.

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

## Known limits of the base

- A setext heading is styled, but its `====` underline stays raw (hiding it would leave an empty line).
- An unclosed leading `---` is not front matter (as in `splitFrontMatter`), and `bodyStart` then looks again on every change; that costs a scan of the document.
- Nothing in this base has been run on WebView2, WKWebView or WebKitGTK; only the two Playwright engines.
