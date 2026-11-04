# adapter size report

Non-blank lines per file of every `src/adapters/<folder>/`, grouped by feature from the file name.
`NOTES.md` and tests (`*.test.ts`) are excluded. `prosemirror-best-effort` is built on `prosemirror`, so the
best-effort variant costs both folders; `prosemirror:stock` costs only `prosemirror`.

## baseline

Total: 301 non-blank lines in 2 files.

| feature | lines | files |
|---|---|---|
| core | 205 | index.ts (205) |
| history | 96 | history.ts (96) |

## codemirror

Total: 1332 non-blank lines in 21 files.

| feature | lines | files |
|---|---|---|
| beats | 198 | beatrow.ts (108), beats.ts (90) |
| callouts | 359 | calloutlook.ts (67), callouts.ts (214), devwords.ts (30), notestate.ts (48) |
| core | 151 | context.ts (14), index.ts (120), theme.ts (17) |
| external changes | 15 | external.ts (15) |
| front matter | 109 | frontmatter.ts (109) |
| history | 14 | history.ts (14) |
| input and keys | 99 | keymap.ts (28), spellcheck.ts (3), typing.ts (68) |
| links | 203 | links.ts (102), popover.ts (101) |
| live preview | 72 | livepreview.ts (61), rawlines.ts (11) |
| modes | 35 | modes.ts (35) |
| scripted edits | 77 | edits.ts (77) |

## prosemirror

Total: 2314 non-blank lines in 27 files.

| feature | lines | files |
|---|---|---|
| beats | 437 | beatparam.ts (21), beatraw.ts (80), beats.ts (203), beatview.ts (133) |
| callouts | 516 | calloutraw.ts (75), callouts.ts (178), calloutview.ts (148), devwords.ts (51), notestate.ts (64) |
| core | 683 | body.ts (49), index.ts (198), markdown.ts (38), positions.ts (322), schema.ts (34), theme.ts (42) |
| external changes | 30 | external.ts (30) |
| front matter | 11 | frontmatter.ts (11) |
| history | 12 | history.ts (12) |
| input and keys | 120 | keymap.ts (26), typing.ts (94) |
| links | 282 | linkkeys.ts (44), links.ts (58), linkview.ts (66), popover.ts (114) |
| live preview | 58 | livepreview.ts (58) |
| modes | 46 | source.ts (46) |
| scripted edits | 119 | edits.ts (119) |

## prosemirror-best-effort

Total: 1011 non-blank lines in 10 files.

| feature | lines | files |
|---|---|---|
| core | 23 | index.ts (23) |
| save mapping | 988 | blocks.ts (116), changes.ts (71), edit.ts (51), fallback.ts (90), offsets.ts (54), sourcebody.ts (249), spanedit.ts (95), spans.ts (195), verify.ts (67) |
