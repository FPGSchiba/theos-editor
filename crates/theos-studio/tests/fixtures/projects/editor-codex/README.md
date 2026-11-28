# Synthetic Codex fixture

A synthetic content project for the tests of `theos-editor` and of `theos-studio`'s Codex views
(P11 §5.12). It is not canon: its ids and texts come from the examples of
`docs/specs/data-model.md` §14, or say "synthetic".

It holds a tower, a band with `floors: [1, 20]`, a floor with two locations (no spawn: the kind
arrives with P21), people, a faction and an npc with a lens, a truth with target `never` and one
with another target, two clues, a belief that distorts a truth, a script whose beat is at the
floor, a concept for a location with a picked version, and a floor body with a voice block, a
design callout, a dangling link and an unclosed link.

For the Codex views it also holds a second belief, `B-017`, that distorts the `never` truth and
mentions the floor; a concept for the floor with no pick; a heading, a list and an HTML block in
the floor's body; two locations whose parents form a cycle (`synthetic-loop-a` and
`synthetic-loop-b`); a location whose parent names no record (`synthetic-stray`); a caption on
the floor's voice block; a voice callout at the end of the floor's body whose title does not
parse; and a thing, `synthetic-html`, whose title and body hold HTML that the Codex must show as text.

The concept is `fates-shrine-art`, not `fates-shrine` as in §14, whose location has that id too:
ids are unique across a project.

The tests copy it to a temporary folder before they open it, so no index cache is written here.
