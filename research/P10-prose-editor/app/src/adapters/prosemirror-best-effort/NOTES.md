# ProseMirror prototype, best-effort column: notes

Variant `prosemirror:best-effort` (column `best-effort`), `prosemirror-view` 1.42.9, `prosemirror-markdown` 1.13.9. The same editor as
`prosemirror:stock`: schema, parser, node views, plugins, keymap and history are imported from `../prosemirror/`, nothing is copied. Only what
Write mode saves and what the harness offsets refer to differ. State after Task 16 and its fix round 1 (the round-trip counts below are the
same before and after the fix round). `tests/e2e/prosemirror-best-effort-probes.spec.ts`
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
| Enter in the middle of a top-level paragraph, Backspace joining it again | span | `\n\n` in, `\n\n` out (probe; mid-paragraph only). |
| Enter at the end of a top-level paragraph, then typing: mid-file (before a heading) and on the last block of a file that ends with a newline, each with and without a save (`text()`) between Enter and typing | span, the same bytes in all four | The new paragraph is `\n\nNew one.` behind the old one and the final newline stays (probes: exact bytes, level, and the saved text parses to the editor's document). Between Enter and typing the empty paragraph owns a blank line between two blank lines. Fix round 1; before it, the save in between made the fallback merge the new paragraph into the next block. |
| The same on the last block of a file without a final newline, with a save between | block, a final newline added | Variant M1 of the separator class (finding below): `…graph.\n\nNew one.\n` instead of `…graph.\n\nNew one.`. Without a save between it is span. |
| Enter in the middle of a callout or quote line | span | A blank quote line with the line's prefix: `\n>\n> ` (probe). |
| Enter at the end of the last paragraph of a voice, design or note callout | block | The editor (stock `splitBlock`) makes a `callout_caption` there, so typing goes into a `— ` caption line; the callout is written again. For design the saved `> — New one.` does not parse back to the document (the design parser has no caption: it reloads as a paragraph), and the empty caption `> — ` saved between Enter and typing does not parse back for any type. Variant M2 (finding below). |
| Enter in a plain quote's last line, a save, typing | span, then block | The save is `> A quote.\n>\n> \n` (span); the keystroke after it is written again (a nested empty paragraph has no line of its own). |
| Enter in a callout title when the callout has nothing else (`leaveTitle` adds a paragraph), a save | block, a blank line leaks behind the callout | Variant O2 (finding below). |
| Enter at the end of a callout paragraph that has a paragraph after it | no change | Stock `splitBlock` refuses there, so there is nothing to save (Node check). |
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
blocks are gaps and belong to no block. An empty paragraph (Enter at the end of a paragraph makes one) has no Markdown and no token: it owns
a blank line of its gap, behind the blank line that separates it from the block before; an empty body has one empty range. `text()` returns
the text; nothing in it is written again unless a change touched it.

**Changes** (`changes.ts`). Every Write-mode transaction that changes the document is queued (typing does nothing more). At the next query
(`text()`, `caret()`, an offset lookup, `lastSaveLevel()`) the queue is mapped in order. A change is the range ProseMirror's step maps say
was replaced (`StepMap.forEach`, mapped through the later steps with `Mapping.slice`), so the select-all replace covers the whole body, edges
included, and a split at the end of a paragraph stays at that end. Only a step with an empty map (a mark step) brings in the document diff
(`findDiffStart`/`findDiffEnd`), whose overlap correction may move a range across a block boundary. Transactions whose ranges touch (typing
a word, `[[query` and the pick, typing and undoing it) are one change; no source text lies inside such a range.

**Span edit** (`spanedit.ts`, `spans.ts`). The start and end of a change are mapped onto source offsets: inside a textblock through the stock
`PositionMap` of the run of blocks around it (a run is the blocks not separated by a blank line), each position giving a range of candidates
from right behind the previous character to the next one (markers such as `*`, `_`, `**`, `]]` between, line prefixes left out); between
top-level blocks the end of the one before or the start of the one after (the body edges at the ends). The inserted content is written raw
(text as typed, a chip as its syntax), and as a second form with the stock serializer's inline rendering (marks, escapes). Across sibling
textblocks of one container (a split by Enter, pasted paragraphs) the parts are joined by a blank line of that container: `\n\n` at the top
level, `\n>\n> ` in a callout, taken from the prefix of the source line. Whole top-level blocks are written raw (paragraphs, headings) or by
the serializer, with blank lines to their neighbours.

**Verification** (`verify.ts`). Every candidate (offsets times forms) is spliced into the source of the run of blocks it falls in, that run
alone is parsed with the adapter's parser, and it must give the new document's blocks there (`Node.eq`), as far as Markdown can hold them:
empty paragraphs are left out on both sides. The first candidate that does is the edit. A second pass accepts a parse that differs only by
whitespace Markdown cannot hold (a space at a line end or start, several spaces): the typed bytes stay. Blocks separated by blank lines parse
independently in markdown-it (checked on every corpus and generated file), except for reference definitions and list continuation by
indentation, which the corpus does not have.

**The final newline** is a fixed separator: an edit that reaches the end of a file that ends with a newline leaves one there, unless it
replaces the whole body (select-all and typing, whose text is then the whole body, as the oracle has it).

**Fallback per block** (`fallback.ts`). When no candidate verifies, or a position cannot be mapped (a run whose tokens do not pair with its
textblocks), the changed top-level blocks alone are written by the stock serializer and put in place of their lines; the blocks around and
the blank lines keep their bytes. Such a block is `serialized`. The result is parsed again like a span edit; when it does not stand for the
document, the whole run around it is written again (and its blocks are `serialized` too). When neither stands for it (no Markdown holds the
document, such as a beat attribute block holding a line that is no attribute line), the smaller rewrite stays and its blocks get guessed
ranges, only the written lines and never the blank lines around them; they are `opaque`: a later change rewrites them together, and offsets
inside that run are guesses, but only there.

**Undo** keeps the bytes: a span edit is taken back as a span edit; a block equal to its block as loaded gets its loaded text back (also
after a fallback, as long as its run still parses with it), with the newline that ends its last line as it is now (the last line of a file
may have none, and a block added behind it gives it one); a document equal to the loaded one gets the loaded text back whole.

**`lastSaveLevel()`** maps the queue first, then says `block` while any block of the saved text was written by the serializer since the load
(or the last external change or mode switch), else `span`. A block a change makes from others (a join, a split) counts as written when any
of the blocks it came from was. Bytes in the gaps between blocks carry no flag (variant O2 of the finding below).

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
- A mode switch, an external change and a load start a new loaded state: the level and the undo restore count from there. So the `external`
  round-trip rows can never report `block`: `applyExternal` resets the level, so whatever the local edit did before it is not counted. Those
  rows are blind by construction; their bytes are still compared with the oracle.
- The loose check is broader than "spaces at the edges of a textblock": it strips the blanks at the start of the first text node that starts
  with blanks, wherever in the textblock that node is (a text behind a chip, for one), and it collapses runs of blanks, so `a  b` counts as
  `a b`. No case was found where it saves other bytes than the typed ones, but it accepts more than it says.
- A change is the range the steps replaced. A replace step wider than the actual change (a command that replaces a whole textblock to change
  one character) would have its whole range written again from the document, markers inside it included, and still count as `span`. Typing,
  the commands of this editor and undo make minimal steps, so this is theoretical here.
- Several pending changes are mapped one after another at the next query, each with a parse of its run; `text()` after a long burst of typing
  in one place is one change.
- What the editor does is the stock editor's: paste semantics (above), Enter in list items (no `prosemirror-schema-list`), the popover in
  attribute lines (above).
- Cost: `reset` tokenizes the body once more than stock (the block ranges); typing costs the same as stock (a queue push).

## Finding: the bytes between blocks have no node in a ProseMirror tree

**The class.** Blank lines, the newline that ends a block's last line and the file's final newline are bytes between blocks. A ProseMirror
document has no node for them, so the save path has to keep them as bookkeeping beside the tree, and every edit at a block boundary has to
decide where they go. The tree editor also makes blocks there that Markdown cannot hold (an empty paragraph after Enter, an empty caption).
The review rounds of Task 16 found six variants of this one class. Fix round 1 fixed the first three with one set of rules; controller ruling
26 then stopped further patching of the class, so the last three are recorded here and not fixed. This is evidence for criterion 1: on a
tree editor byte stability at block boundaries is not a property of the editor but bookkeeping that every new kind of boundary edit can break.

| Variant | What happened | Cost now |
|---|---|---|
| E1. Enter at the end of a paragraph, a save, typing | The empty paragraph's guessed range swallowed the blank line in front of the next block; typing merged the new paragraph into it (`…\n\nNew one.\nNext.`, which parses to another document). | Fixed (round 1): span, exact bytes, mid-file and on the last block (probes). |
| E2. A block added behind the last block | The diff's overlap correction stretched the change past the last block to the end of the file; the final newline was lost, at `span`. Also for a node or text inserted at the document end. | Fixed (round 1): span, final newline kept (probes and unit tests). |
| Undo line end | The undo restore put back a block's loaded line end, so the last line of a file without a final newline stayed without one after a block was added behind it (`Words here.\nNew one.`). | Fixed (round 1). |
| M1. A file without a final newline: Enter at the end of its last paragraph, a save, typing | `blankLine` counts only lines that end in `\n`, so the empty paragraph sits on the separator line, its run joins the paragraph before, and the rewrite adds a newline: `…graph.\n\nNew one.\n` instead of `…graph.\n\nNew one.`; offsets in that run are guesses (`toOffset` of "Last" 15 instead of 14). | block level, one byte beyond the edit (a final newline the file did not have). |
| M2. Captions in callouts | Enter at the end of the last paragraph of a voice, design or note callout makes a `callout_caption` (stock `splitBlock`). For design the saved `> — New one.` does not parse back to the document (the design parser has no caption; it reloads as a paragraph); the empty caption `> — ` saved between Enter and typing does not parse back for any type. In a plain quote the save is span (`> A quote.\n>\n> \n`) and only the keystroke after it is block. | block level; for design a saved file that reloads as a different document. |
| O2. Enter in a callout title when the callout holds nothing else, a save | `leaveTitle` adds an empty paragraph to the callout; the rewrite leaves a blank line behind the callout: `> [!design] Title\n\n\nAfter.` at block. An undo that cannot go back to the loaded document (after another edit) leaks a second blank line (`\n\n\n\n`) and reports span: the leaked bytes sit in a gap, which has no block and so no `serialized` flag. The O1 rule of fix round 2 does not change that (the callout block itself is back to its loaded text). An undo straight back to the loaded document restores the loaded bytes, at span. | block level after the save; after such an undo extra blank lines at a level that says span. |

The rules of fix round 1 (all variants above share them): a block is whole lines and separators stay outside block ranges, guessed ranges
included; an empty paragraph owns a blank line between blank separators; the change range comes from ProseMirror's step maps, not from the
diff; every rewrite is parsed again and widened to its run when it does not stand for the document; the final newline is a fixed separator;
a restored block keeps its current line end. The reparse turns most losses into a block-level rewrite; it does not see gap bytes (O2) or a
final newline (M1).

Existing APIs used: markdown-it's token `map` for the block lines, ProseMirror's `StepMap.forEach` and `Mapping.slice` for the change range,
`Fragment.findDiffStart`/`findDiffEnd` only for mark steps, `Node.eq` and the adapter's own `MarkdownParser` for the reparse, the stock
`MarkdownSerializer` for rewrites. Hand-written: where an empty paragraph's blank line is, and the final-newline rule.

## Budget

One implementation attempt and two fix rounds: round 1 the separator class (above), round 2 the level of joined and split rewritten
blocks (a new changed block is `serialized` when the edit wrote it or any old changed block was; before, a join kept only the first block's
flag and the new half of a split reported span; probes for both). Tried and kept: raw text first, serializer's inline form second; offsets on both sides of markers; the loose
whitespace pass; the container prefix for paragraph breaks in callouts; per-run offset maps (after a probe showed that one block that no
Markdown stands for turned every offset of the file into a guess); trimming the blank line markdown-it gives a list (after a unit test showed
the fallback lost the blank line behind a list). Not tried: paste as plain text and a soft-break node (editor and parser changes shared with
the stock column), rewriting emphasis markers (`_` to `*`) as a span edit.
