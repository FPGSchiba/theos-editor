---
# synthetic fixture (P02), not canon
id: synthetic-callouts-in-world
kind: topic
title: Synthetic callouts, in-world
status: draft
track: in-world
---
A synthetic paragraph before the first callout.

> [!VOICE] [[throat-kin]]
> A synthetic voice line with a speaker only.

> [!voice]- [[hearth-folk]] · [[B-014]] · [[B-020]]
> A synthetic voice line with a speaker and two beliefs, collapsed.

> [!Voice]+ [[mother-ilse]] · [[B-016]] · T1
> A synthetic voice line with a speaker, a belief and a tier, expanded.
>
> A second paragraph after an empty quote line.
>
> — a synthetic caption

> A plain synthetic block quote that holds a voice block.
>
> > [!voice] [[throat-kin]] · T2
> > A synthetic voice line nested in a block quote.

> [!voice] [[hearth-folk]]
> A synthetic voice line whose second line
continues lazily without a quote marker.

> [!voice] [[throat-kin]]
> -- a line that starts with two hyphens is not a caption.

> [!voice] [[mother-ilse]] · [[B-016]]
> A synthetic voice line that ends with a design note.
>
> Design note: the player meets this line before any clue is confirmed.

> [!design] A design title with [[f04-choir-hall]]
> A synthetic designer remark with a title.

> [!design]
> A synthetic designer remark without a title.

> [!tip] A tip
> A synthetic block of an unknown type, which is plain data.

Design note: a synthetic design note at the top level that is wrapped
over two lines to show a hard-wrapped paragraph.

- A list item with a link to [[fates-shrine]].
- A list item with the link [[S-004#3]] to a beat.

> A plain synthetic block quote with a link to [[mother-ilse|Ilse]].

Links with letters in other cases and aliases: [[Mother-Ilse]] and [[mother-ilse|Ilse]].

A code span holds no link: `[[` and `[[not a link|` stay as they are.

```text
[[ an open bracket pair inside fenced code
[[[triple]] and ![[an embed]] are not links here
```

<!-- a synthetic HTML comment with [[ an open pair and [[a-link]] inside -->
