# ProseMirror prototype, best-effort column: notes

Variant `prosemirror:best-effort` (column `best-effort`), `prosemirror-view` 1.42.9, `prosemirror-markdown` 1.13.9. The same editor as
`prosemirror:stock`: schema, parser, node views, plugins, keymap and history are imported from `../prosemirror/`, nothing is copied. Only what
Write mode saves and what the harness offsets refer to differ. State after Task 16. `tests/e2e/prosemirror-best-effort-probes.spec.ts`
(`@harness`) and `sourcebody.test.ts` (Vitest) probe what the measured suites cannot see.

## Result

Round trip (`P10_VARIANTS=prosemirror:best-effort`, non-strict), 77 rows per engine, the same on both engines:

| Column | Identical | Block level | Beyond the block | Error | `adapterLevel` |
|---|---|---|---|---|---|
| best effort | 74 | 3 | 0 | 0 | 77 `span` |
| stock (for comparison) | 13 | 10 | 54 | 0 | none |

Every unedited file is identical, the CRLF file to its LF form. Under `P10_STRICT=1` (every tag but `@long`) 4 checks fail per engine, listed
below; every feature, input and integration check that compares bytes passes (`F-source-mode`, `F-link-popover-escape`,
`N-keymap-undo-redo`, `N-external-caret`, all IME, dead-key and `I-insert-*` rows).

## Still failing under `P10_STRICT=1`

- **`paste-crlf`, api and input (both engines).** Block level, `adapterLevel` `span`. ProseMirror's plain-text paste makes every pasted line
  a paragraph, so the document holds two paragraphs where the oracle has one line break inside one paragraph. The saved text is the span edit
  of that document (`line.\n\nSecond`); writing the oracle's `line.\nSecond` would save a file that parses to one paragraph, not to what the
  editor shows. Not fixable in the save path: the stock schema and parser have no soft line break (it is a space in the tree), so no
  document can say "line break here". Named fix: a soft-break inline node in the schema and the parser (shared with stock), which a pasted
  line break would become and which is written as `\n`.
- **`paste-html` via input (both engines).** Block level, `adapterLevel` `span`. The editor pastes the HTML as rich text: `<b>rich</b>` stays
  bold and the DOM parser drops the leading space, so the document is `lists.Pasted **rich** text.`, and that is what is saved (a span edit,
  the bold written by the serializer's inline form). Via api (`pasteText`, plain) the row is identical. Named fix: paste as plain text
  (`transformPastedHTML` or a `handlePaste` that inserts `text/plain`), an editor behaviour shared with stock, not a save-path one.
- **`F-link-popover-kind-filter` (both engines).** Not a byte check: typing `[[` in an `at::` line opens no popover. It fails in the stock
  column too, since Task 15: the attribute lines are a code-like `beat_attrs` node, and the typing plugin opens no popover in code. Same
  editor, so the same result; left as it is.

## Level per edit kind (corpus and probes)

| Edit kind | Level reached | Why |
|---|---|---|
| `insert` (typing, IME, dead keys) | span | Text raw at the mapped offset. Probe: a keystroke in every third word of six corpus files is the oracle's insert, undo gives the bytes back. |
| `delete` (Delete, Backspace) | span | The source range of the deleted characters; a deleted emphasis run takes its markers with it. |
| `paste` plain, one line | span | As typing. |
| `paste` plain, several lines | span edit, differs from the oracle | See `paste-crlf` above. |
| `paste` HTML (input) | span edit, differs from the oracle | See `paste-html` above. |
| `select-all-type` (api and keyboard) | span | The replace covers the whole body, so it is the body range of the source, final newline included. |
| `external` | span | The change goes into the source text; one undo removes the local edit as a span edit. |
| `undo-all`, undo, redo | span | Undo is a transaction like any other; a document equal to the loaded one gets the loaded bytes back whole. |
| `link-insert` (popover) | span | The typed `[[query` and the pick are one change; the chip is written as `[[id]]`. |
| `link-delete` | span | The chip's source range is its syntax. |
| `note-state` | span | The title is raw text of the callout; the change is a span of its line. |
| `beat-param` (row and api) | span | The attribute lines are raw text; the first line of a bare beat gets its own line (`at:: x\n`). |
| Enter, Backspace in a top-level paragraph | span | `\n\n` in, `\n\n` out (probe). |
| Enter in a callout or quote paragraph | span | A blank quote line with the line's prefix: `\n>\n> ` (probe). |
| Enter in an item of a tight list | block | No Markdown holds a tight list whose item has two paragraphs; the list is written again (probe). |
| A letter that changes what a line is (`Beat` of a beat heading, an upper-case letter in an attribute key) | block | The document still holds the beat nodes, which no source with that line stands for; the block is written again, the bytes are the typed ones (probe). |
| A letter between a word and an `_` emphasis run (`Y_x_`) | block | An `_` run cannot open inside a word; the serializer writes `Y*x*` (unit test). |

## What it does, and how

**Seam in the stock adapter** (`../prosemirror/body.ts`). The adapter gets a `BodyModel`: how the saved file comes from the document and which
text the offsets refer to. `stockBody()` is exactly what the stock adapter did inline before (the stock serializer for `text()`, the `Tracker`
for offsets), and a model that defines `observe` gets a `dispatchTransaction`; stock does not, so its view is created as before. The stock
rows of a full non-strict run are identical to the ones before the seam (all 154 round-trip rows, hunks included, and every feature, input
and integration row). `PositionMap` got one public method, `offsets(pos)`; its alignment is unchanged.

**The saved truth is the source text** (`sourcebody.ts`, `blocks.ts`). `SourceBody` keeps the file text, the document it stands for, and for
every top-level child of that document the source range of its block: markdown-it's `map` of the top-level tokens, whole lines, the newline
of the last line included, blank lines behind a list left out (markdown-it counts the one behind a list to the list). The blank lines between
blocks are gaps. An empty body has one empty block for the empty paragraph. `text()` returns the text; nothing in it is written again unless
a change touched it.

**Changes** (`changes.ts`). Every Write-mode transaction that changes the document is queued (typing does nothing more). At the next query
(`text()`, `caret()`, an offset lookup, `lastSaveLevel()`) the queue is mapped in order. A change is the range the steps replaced (the step
maps joined with the document diff), so the select-all replace covers the whole body, edges included. Transactions whose ranges touch
(typing a word, `[[query` and the pick, typing and undoing it) are one change; no source text lies inside such a range.

**Span edit** (`spanedit.ts`, `spans.ts`). The start and end of a change are mapped onto source offsets: inside a textblock through the stock
`PositionMap` of the run of blocks around it (a run is the blocks not separated by a blank line), each position giving a range of candidates
from right behind the previous character to the next one (markers such as `*`, `_`, `**`, `]]` between, line prefixes left out); between
top-level blocks the end of the one before or the start of the one after (the body edges at the ends). The inserted content is written raw
(text as typed, a chip as its syntax), and as a second form with the stock serializer's inline rendering (marks, escapes). Across sibling
textblocks of one container (a split by Enter, pasted paragraphs) the parts are joined by a blank line of that container: `\n\n` at the top
level, `\n>\n> ` in a callout, taken from the prefix of the source line. Whole top-level blocks are written raw (paragraphs, headings) or by
the serializer, with blank lines to their neighbours.

**Verification** (`verify.ts`). Every candidate (offsets times forms) is spliced into the source of the run of blocks it falls in, that run
alone is parsed with the adapter's parser, and it must give the new document's blocks there (`Node.eq`). The first candidate that does is the
edit. A second pass accepts a parse that differs only by whitespace Markdown cannot hold (a space at a line end or start, several spaces):
the typed bytes stay. Blocks separated by blank lines parse independently in markdown-it (checked on every corpus and generated file),
except for reference definitions and list continuation by indentation, which the corpus does not have.

**Fallback per block** (`fallback.ts`). When no candidate verifies, or a position cannot be mapped (a run whose tokens do not pair with its
textblocks), the changed top-level blocks alone are written by the stock serializer and put in place of their lines; the blocks around and
the blank lines keep their bytes. Such a block is `serialized`. When the written run does not parse into one token per block (a document no
Markdown stands for, such as a beat attribute block holding a line that is no attribute line), the rewritten blocks get guessed ranges and are
`opaque`: a later change rewrites them together, and offsets inside that run are guesses, but only there.

**Undo** keeps the bytes: a span edit is taken back as a span edit; a block equal to its block as loaded gets its loaded text back (also
after a fallback, as long as its run still parses with it); a document equal to the loaded one gets the loaded text back whole.

**`lastSaveLevel()`** maps the queue first, then says `block` while any block of the saved text was written by the serializer since the load
(or the last external change or mode switch), else `span`.

**Offsets** (`offsets.ts`). `caretTo`, `caret` and `apply` use the stock `PositionMap`, built per run on demand on the source text, which is
the exact file. A query reads one run instead of tokenizing the whole file, and a run that does not pair only guesses inside itself. The
bare-beat newline loss of the stock position source does not exist here: the source has the newline (probe: offsets behind the first line of
a bare beat stay right).

**External changes, modes, history.** `applyExternal` applies the change to the source text, parses it, dispatches the minimal replace outside
the history (the stock code path) and makes the result the new loaded state. Source mode shows the source text (probe: after an edit it shows
`_underscores_` and the setext underline as written); switching back parses it and starts from it. `markSaved` only clears the history.

## Limits

- The verification parses a run alone; a reference definition elsewhere in the file, or a block that becomes a list continuation by its
  indentation, is not seen (none in the corpus). A run containing reference-style links never verifies, so every change there falls back.
- A mode switch, an external change and a load start a new loaded state: the level and the undo restore count from there.
- Several pending changes are mapped one after another at the next query, each with a parse of its run; `text()` after a long burst of typing
  in one place is one change.
- What the editor does is the stock editor's: paste semantics (above), Enter in list items (no `prosemirror-schema-list`), the popover in
  attribute lines (above).
- Cost: `reset` tokenizes the body once more than stock (the block ranges); typing costs the same as stock (a queue push).

## Budget

One implementation attempt. Tried and kept: raw text first, serializer's inline form second; offsets on both sides of markers; the loose
whitespace pass; the container prefix for paragraph breaks in callouts; per-run offset maps (after a probe showed that one block that no
Markdown stands for turned every offset of the file into a guess); trimming the blank line markdown-it gives a list (after a unit test showed
the fallback lost the blank line behind a list). Not tried: paste as plain text and a soft-break node (editor and parser changes shared with
the stock column), rewriting emphasis markers (`_` to `*`) as a span edit.
