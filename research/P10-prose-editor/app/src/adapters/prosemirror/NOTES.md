# ProseMirror prototype: notes

Variant `prosemirror:stock` (column `stock`), `prosemirror-view` 1.42.9, `prosemirror-markdown` 1.13.9 with its own parser and serializer, unchanged.
State after Task 12 (the base). `tests/e2e/prosemirror-probes.spec.ts` (`@harness`) probes what the measured suites cannot see:
caret mapping, typing position, front matter protection, undo, external changes, live preview, composition.

## Still failing under `P10_STRICT=1`

Run: `P10_STRICT=1 P10_VARIANTS=prosemirror:stock pnpm exec playwright test --grep "@base|@integration|@input" --grep-invert "I-[a-z-]*-chip"`.
Chromium fails 55 of 72 rows, WebKit 51 of 69 (the 3 skipped rows of each engine belong to the other engine). Every failure is one of three kinds.

**1. Byte mismatches of the stock serializer (measurements, not chased).** 42 round-trip rows per engine (6 at block level, 36 beyond it, 0 errors; 13 are identical),
and the checks that compare saved bytes: `F-source-mode` (the body saved differs, the front matter is intact), `N-keymap-undo-redo` (undo returns to the
serialization of the file, not to its bytes), `N-external-caret` (the caret rows pass, the text row compares bytes), and the IME, dead key and
`I-insert-*` rows (the composition happens and the probes show the text lands next to the same characters; the rest of the file differs).
What the stock parser and serializer change, file by file (`fm + serialize(parse(body))` against the original):

| Change | Seen in |
|---|---|
| Soft line breaks become a space: a paragraph of several lines is one long line (attribute lines `at::`/`with::`/`says::`, callout heads and bodies, wrapped prose) | every file that has a wrapped paragraph or a callout or beats: B-901, links, voices, f04-terrain, synthetic-notes, S-004, S-901, synthetic-crlf, whitespace |
| `[[id]]` and `[[id\|text]]` are escaped as `\[\[id\|text\]\]`; `[!voice]` as `\[!voice\]` | every file with links or callouts |
| The final newline of the file is dropped | every file that ends with one (all but the 5 identical ones) |
| Setext headings become ATX (`# Title`, `## Title`) | B-901 |
| `_x_` becomes `*x*`, `__x__` becomes `**x**` | B-901 |
| A hard break (two trailing spaces) becomes a backslash break; trailing spaces of a line are lost | B-901, whitespace |
| Bullets `-`, `+` and `*` all become `*`, `1)` becomes `1.`, a list of ones is renumbered 1, 2, 3, adjacent lists are separated by two blank lines | T-901 |
| Several blank lines between blocks collapse to one; trailing spaces of a paragraph line are lost | whitespace |
| A table (not CommonMark) stays a paragraph of text | T-901 |

Identical in all rows: fates-shrine-v5, C-031, f04-terrain-v1, mother-ilse (empty body) and synthetic-no-final-newline (each short, plain, or without a body). `synthetic-characters` differs only by the dropped final newline (its edit rows are block level for that reason).

**2. Features that belong to later tasks (ruling 18).** `F-live-preview` fails only on the beat headings (`2 · The murmur` rendered, Task 15); the raw `## Beat 1 — Arrival` under the caret passes.
`I-spellcheck-off` (no chips, no beat rows), `N-reactive` and `N-remount` (no chips), Tasks 13 and 15.

**3. The WebKit gap row** `I-ime-webkit` is a recorded gap and never fails.

Nothing else fails: no harness error, no timeout, no console error in `N-remount`.

## What the base does, and how

- **Schema** (`schema.ts`). Write mode is the stock schema of `prosemirror-markdown`. Source mode has its own schema: a document with one code-like block that holds the whole file as text.
- **Parse and serialize** (`markdown.ts`). `defaultMarkdownParser.parse` and `defaultMarkdownSerializer.serialize`. `text()` is the front matter bytes plus the serializer's output; no source text is returned.
  A front matter without its final newline gets one when a body follows.
- **Positions** (`positions.ts`). The harness speaks offsets into the whole file, ProseMirror positions in a tree. `PositionMap` pairs each textblock with the line range markdown-it gives its block token
  (`map`, read from `defaultMarkdownParser.tokenizer`) and aligns the block's text with its source lines character by character: the characters no text character matches (`## `, `> `, `- `, `**`, `\`) are the markers.
  Line prefixes (quote marks, list markers, ATX hashes) are skipped by a regular expression, a soft break (a space in the tree) matches a source line end, a fence's first line is skipped.
  `Tracker` keeps the position source, the file text the offsets refer to, in step with the document: at the next query it takes the minimal diff between the document it last saw and the current one
  (`findDiffStart`/`findDiffEnd`) and splices the changed text (`textBetween`) into it, as the oracle does with the file. The position source is used for offsets only, never returned.
  Limits: the alignment is greedy (a text character without a source twin, such as the character an entity stands for, is placed at the cursor); the map is rebuilt, with a full markdown-it parse, at the first query after a change
  (typing is free, an offset query after each edit costs one parse of the body, so long files pay per query); changes that fall in two places between two queries are spliced as one range, which loses the markup in
  between; blocks that markdown-it and the schema do not pair one to one (an empty list item) fall back to proportional guesses; a paragraph line that starts like a list item (`2. x` inside a paragraph) is read as a marker.
  Probes show offsets round-trip (`caretTo`, `caret`) in headings, soft-wrapped paragraphs, quote lines, setext and fenced blocks, and typing lands next to the same characters.
- **Front matter** (`frontmatter.ts`, `source.ts`). Write mode keeps it outside the ProseMirror document: split off on load, put back in front of the serializer's output. Source mode shows the whole file raw in the same view, as one block, and protects
  the front matter with a `filterTransaction` (any step that starts inside it is rejected) and a decoration for its look. Chosen over a second component so that the comparison counts one component; the cost is that the document, the history and the
  position source are replaced at every mode switch, so undo history does not survive a switch and Write to Source writes the serialized text, not the loaded bytes.
- **Live preview** (`livepreview.ts`). A plugin whose `decorations` add widgets for the blocks and runs the selection touches: `## ` before a heading, `*`, `**` or `` ` `` around emphasis, strong and code runs. The tree renders everything else.
  The markers are those of the stock serializer, so source written with `_` shows `*`.
- **History** (`history.ts`). `prosemirror-history`; `markSaved()` creates the state again with the same document and selection and a new history plugin (the library has no reset, issue 1042).
- **External changes** (`external.ts`). The change goes into the position source, the new source is parsed, and the minimal replace between the two documents is dispatched with `addToHistory: false`; the history maps its steps and the
  transaction maps the selection. Probe: the caret shifts for a change before it and stays for one behind it, and one undo removes only the local edit.
- **Keymap** (`keymap.ts`). `ctx.keymap` (`Mod-` notation) into `prosemirror-keymap`, then `baseKeymap`; `save` calls `ctx.save()`; `Mod-a` is `selectAll` in Write mode (the body is the whole document) and the body range in Source mode.
  `updateContext` swaps the plugins with `state.reconfigure`, which keeps the history instance.
- **Input** (`index.ts`). The editable root has `spellcheck="true"` in Write mode.
- **`apply`** (`edits.ts`). `insert` is `tr.insertText`, `delete` is `tr.delete`, `paste` is `view.pasteText` (plain text), `select-all-type` is an `AllSelection` replaced by the text, `external` is the local insert plus `applyExternal`,
  `undo-all` applies each edit and then undoes until `canUndo()` is false. Link, callout and beat kinds throw until Tasks 13 to 15.

## Not done in the base

Enter in a list item splits the block instead of the item (no `prosemirror-schema-list`); Tab, input rules and marks by shortcut are not bound. Neither is part of the measured suites.
