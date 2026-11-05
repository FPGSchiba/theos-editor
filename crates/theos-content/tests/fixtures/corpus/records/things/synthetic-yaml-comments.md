---
# synthetic fixture (P02), not canon
# an own-line comment before the first field
id: synthetic-yaml-comments
kind: thing
# an own-line comment between fields
title: Synthetic YAML comments
type: document   # a comment after a plain scalar
status: "draft"  # a comment after a quoted scalar
where: [fates-shrine, f04-choir-hall]   # a comment after a flow list
held_by:   # a comment after a key with a block list below
  - throat-kin
  # a comment between block items

  - hearth-folk
# a comment between two fields
requires:
- T-012
- T-001
cites:
    - E7
    - E8
lens:
  # a comment before the first entry
  speaks_to: children   # a comment after an entry
  # a comment between entries
  role: keeper
  # a comment after the last entry
# an own-line comment after the last field
---
A synthetic body for a record whose front matter is full of comments.
