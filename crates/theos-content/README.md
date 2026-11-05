# theos-content

Library. Record model, front-matter patcher, body grammars (beats, voice, notes), kind registry,
index, search, effective lenses, validators, lore rules and ledger. Pure filesystem; no async; no
UI.

After phase P02 the crate provides the first half of that list: records and grammars. The rest
follows in P03 and P04.

## What it provides

| Module | What it does |
|---|---|
| `text` | `Span` (a byte range), `TextIndex` (byte offset, line and column, UTF-16), `TextEdit` and `apply_edits`, line-ending normalisation |
| `record` | `Record`: reads a record (`read_record`, `decode`, `Record::parse`), splits it into front matter and body, `was_normalized()` |
| `yaml` | The restricted YAML of the data model: one rule per forbidden form, the front-matter model with spans (`FrontMatter`, `Field`, `Scalar`) and the patcher (`Record::patch`, `set`, `remove`, `set_entry`, `remove_entry`, `Order`) |
| `body` | One pass over the body: blocks, wikilinks, callouts (`[!voice]`, `[!design]`, `[!note]`), design notes, beats of a script, and the malformed constructs with their line and column |

The patcher changes only the lines of the field it is asked to change: every edit is a span edit
on the text, the result is parsed again and checked, and nothing is returned when the check fails.
Bodies are never rewritten by the crate; callers build `TextEdit`s from the spans of the model.

## Coordinates

Every span is a byte range into the LF text of the whole record, front matter included
(`Record::text()`). For an LF file these are file offsets. For a CRLF file they are offsets into
the text a write would produce, never into the bytes on disk. The same text always gives the same
spans. Positions for people are a line and a column counted from 1 in Unicode scalar values.
`TextIndex` converts between the two and to UTF-16.

A write turns the whole file into LF; for LF files a write that changes nothing returns the same
bytes. `Record::was_normalized()` tells a caller that saving an unchanged CRLF record would change
its bytes.

## The fixture corpus

`tests/fixtures/corpus/` is one content project, shared by P03 to P05 and by the display tests of
P12:

- `records/` and `malformed/`: valid records, ids unique across both, file names equal ids;
- `crlf/`: CRLF, mixed and lone-CR inputs with their LF twins (`twins.toml`);
- `rejected/`: front matter the restricted YAML refuses, with the rule, line and column
  (`expected.toml`).

Every file says it is synthetic. The integration tests in `tests/corpus/` read the corpus: records
parse, bodies and records round-trip byte for byte through the patcher and through span edits
built from the model, and a field edit changes only that field's lines.
