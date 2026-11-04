# Comments on text ranges: the same probe in both editors

Question (Jann): is CodeMirror 6 still the choice if comments on text sections must be easy? This
probe builds the same minimal feature in both editors at the state level (Node, no DOM) and
measures what each needed. Pinned versions: `@codemirror/state` 6.7.6, `@codemirror/view` 6.43.15,
`@codemirror/commands` 6.11.1; `prosemirror-state` 1.4.4, `prosemirror-view` 1.42.9,
`prosemirror-history` 1.5.1, `prosemirror-model` 1.25.12. No adapter, suite, oracle or doc was
changed; the ProseMirror side imports the stock adapter's `stockBody()` (`PositionMap` +
`Tracker`) read-only.

The feature: a comment is `{ id, from, to }` in file offsets (UTF-16, front matter included),
stored outside the file. Start-edge rule, both sides: text typed at the exact start stays
outside the range (the range shifts); text typed at the exact end stays outside it too.
Both sides run the identical tests (`behaviours.ts`), on three ranges: (a) `The binders call them
ancestors` (a block start), (a2) `call them ancestors` (inside its paragraph), (b)
`record: [[fates-shrine]]` (spans the link).

## Result

Every behaviour was run per range; all passed on both sides, on the first reasonable attempt
(no behaviour needed its second attempt, so none is recorded as failing). Run: 77 probe tests, 276
tests in all with the 199 existing ones, `tsc --noEmit` exit 0.

| Behaviour | CodeMirror 6 | ProseMirror |
|---|---|---|
| 1. add/remove leave the file text byte-identical | pass | pass |
| 2. insert before shifts, inside grows, delete over an end shrinks, typing at end does not extend, start rule | pass | pass |
| 3. delete all commented text drops it; undo restores text and comment | pass | pass (custom "grave", see below) |
| 4. undo of add removes, redo restores | pass | pass (custom action stack, see below) |
| 5. outside change (no history) maps the comment | pass | pass |
| 6. round trip list, fresh state, restore | pass | pass |
| 7. link range like prose range | pass | pass |

Test 2d (deletion over the start) runs for (a2) and (b) only. For (a) the start of the range is
the start of its paragraph, so such a deletion crosses a paragraph break; a separate limits test
covers that and passes on both sides.

## Size and what each side needed

Non-blank, non-comment lines, tests excluded: CodeMirror `codemirror.ts` 71; ProseMirror
`prosemirror.ts` 140 (the shared `shared.ts` types and corpus helper, 44 non-blank lines, and the
shared tests `behaviours.ts`, 126, are not counted on either side).

- CodeMirror, all library mechanism: `StateField` + `Decoration.mark` set (the library maps it
  through changes and draws it), `StateEffect` for add/remove, `invertedEffects` for undo/redo.
  Custom code: dropping emptied ranges (one `filter`), and one function that says "comments that
  were there before and are gone now come back on undo". The undo of both add/remove and of a
  text deletion that dropped a comment came from that one function.
- ProseMirror, library mechanism: plugin state, `Decoration.inline`/`DecorationSet.map`,
  `tr.mapping`, `isHistoryTransaction`, and the adapter's `PositionMap`/`Tracker` for offsets.
  Custom code: (1) the grave: a list of dropped comments (position, length) mapped through every
  change; a history transaction that re-inserts text of the same length at that spot revives
  the comment. (2) An own stack of comment actions, keyed by the history's undo depth, because
  `prosemirror-history` has no hook for plugin state and an add is not a history event. (3) The
  offset conversion in both directions on every add, list and edit.

## What was hard

- ProseMirror undo. `prosemirror-history` only offers `addToHistory` meta, `closeHistory`,
  `isHistoryTransaction` and the depths; there is no equivalent of CodeMirror's
  `invertedEffects`. Both undo behaviours are therefore code of our own, and both are
  heuristics: the grave revives by matching a length at a position (an undo that inserts
  exactly as many characters at that spot would revive it wrongly; not hit by the tests), and
  the action stack only takes back an action when no document edit happened since, by depth.
  It restores a removed comment at the position it had when removed.
- ProseMirror offsets. Every file offset must go through the position source, which the
  `Tracker` rebuilds by splicing the changed text in after each edit, and the saved text is not
  that source (see limits). CodeMirror needed no conversion: the document is the file.
- CodeMirror: nothing was hard; the one thing to read up was that `invertedEffects` positions are
  in the document before the transaction, which fits `startState` ranges.

## Limits found

- Saved text, ProseMirror: the stock serializer does not write the corpus file back as it was
  (soft line breaks are joined, `_x_` becomes `*x*`). This is already so before any comment, and
  comments do not change it (behaviour 1 holds), but offsets then refer to the `Tracker`'s
  position source, not to the saved bytes. Comments stored as offsets would drift against the saved
  file for ProseMirror whenever the serializer changes a line length. CodeMirror: text is the file.
- A range inside or across a link: ProseMirror has the link as one atom. A range that starts
  inside `[[...]]` snaps to the start of the chip; a range wholly inside one link collapses
  and is silently lost on add. CodeMirror keeps both as given (the link is plain text).
- A range across blocks works on both sides (offsets round-trip). A range at a block start
  and a deletion from the paragraph break into it shrinks it correctly on both.
- Both sides: undo of a partial deletion over the end of a range does not grow the range back
  (the mark is not inclusive). Same on both; tested as observed.
- `PositionMap` fell back to guesses nowhere on this file (`exact` is true; asserted). It
  has fallbacks on other documents (empty list items), not probed.
- Not probed: drawing, overlapping comments, many users, a comment inside a fenced block,
  selection-based UI.

## Open storage question for Jann (not decided here)

Where do the comments live? (1) A sidecar file beside the Markdown file (offsets plus a quote of
the commented text, so a comment can be re-anchored by searching for the quote when the file was
changed outside the editor), or (2) markers inside the Markdown file. With a sidecar the
Markdown stays byte-identical (G5) and a diff of the record shows no comment noise, but the sidecar
is a second file that can go out of step (the quote is what repairs it), and a copy of the record
without the sidecar loses the comments. With markers inside the file, comments travel with the
record and need no re-anchoring. But adding, editing or resolving a comment is an edit of the
file. G5 still holds, because the bytes the edit does not touch stay identical. Every comment edit
appears in the record's diff, and generators and other tools that read the file see the markers
unless they strip them. This probe is
valid for either storage: it measures the in-editor part (mapping through edits, undo, restore).
