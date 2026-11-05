# P02 · Records and grammars — phase spec

Status: waiting for Jann's approval · 5 Nov 2026 · repo of the build: `theos-editor`
Source: `docs/specs/00-umbrella.md` §3 P02. Scope and acceptance criteria come from there; this
spec decides how. It closes the 17 gaps of the readiness check of 4 Nov 2026 (appendix A).

## 1. Goal

After P02, `theos-content` reads the text of any record into a model with exact positions. It
writes front-matter changes that touch only the changed field's lines, and parses the body
grammars of data model §9: beats, voice blocks, design notes, notes and wikilinks. It works on
text in and text out: no folders, kinds, index or findings yet.

Every later core phase builds on it:
- P03 types the fields by kind and indexes the links;
- P04 turns P02's malformed constructs into findings and checks the in-world text P02 marks;
- P06 edits records through the patcher and renames ids through the link spans;
- P09 reads and resolves notes;
- P12 shows what P02 parses (decision 0001: "links and findings come from `theos-content`").

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-09 Record model and front-matter patcher**
- CE-09.1 Every record in the fixture corpus round-trips byte for byte.
- CE-09.2 Setting one field changes only that field's lines.
- CE-09.3 Anchors, aliases, tags, multi-line scalars, flow maps and deeper nesting are rejected
  with line and column.
- CE-09.4 CRLF input is read; output is LF.

**CE-10 Body grammars**
- CE-10.1 S-004 of the corpus parses into its beats with their attribute links.
- CE-10.2 Voice blocks, design notes and note callouts parse with their title tokens; note states
  `open`, `resolved by [[id]]` and `dropped` are recognised.
- CE-10.3 A malformed beat attribute, callout title or wikilink reports its line.
- CE-10.4 Bodies round-trip byte for byte.

The fixture corpus starts with the examples of `docs/specs/data-model.md` and synthetic records;
real records are added once the lore import exists.

## 3. Scope

**Goals**
- One crate, `theos-content`, with a record model, the restricted-YAML rules, a front-matter
  patcher, the body grammars, positions, and the fixture corpus that P03 to P05 reuse.
- Every part built on a crate where one exists (engineering directive 2): `yaml-rust2` for YAML,
  `pulldown-cmark` for Markdown. Our code is the subset rules, the patcher, and the small
  grammars of data model §9 that no Markdown parser knows.
- Exact positions everywhere: byte spans for code, line and column for people.

**Non-goals**
- Walking folders, reading `theos.toml`, kinds, schemas, the registry, the index (P03).
- Findings with rule and severity, and judging meaning: dangling links, a wrong tier, beat
  numbering, note ids, link counts per attribute (P03, P04).
- Commands, undo, file writes (P06). P02 returns new text; it never writes a file.
- Typed body edits such as "rename this link" or "resolve this note". P02 exposes the exact
  spans and one generic span edit; P06 and P09 build the edits they need on them.
- Comments on text (proposal 0002). P02 builds only the positions 0002 asks of it (§6).
- A CLI (P05).

**Anti-goals** (P02 fails if any of these holds, even with every test green)
- A hand-written YAML or Markdown lexer (engineering directive 2).
- A write that changes a byte outside the edited field. The one exception is CE-09.4's CRLF
  rule, which Jann ruled on (§4.1).
- Round-trip tests that pass only because the parser keeps the source text and returns it.
- Something that data model §4 forbids, accepted silently: the rules fail closed.
- Lore invented in fixtures.

## 4. Decisions

### 4.1 Decided by Jann (5 Nov 2026)

- **Ruling (Jann): scalars are typed by the field, not by their spelling.** The parser keeps
  every plain scalar as raw text, with its style. The field's type decides what a value means:
  an id is always text, and `accuracy: false` is the enum value `false`. The id pattern of data
  model §3 stays as it is. Quotes are never required. (Readiness decision D1.)
- **Ruling (Jann): a write turns the whole file into LF.** CE-09.4 says "CRLF input is read;
  output is LF". Reading without writing changes nothing. Precedence:
  - for CRLF input, CE-09.4 wins over G5's "any edit leaves untouched bytes identical": the CR
    bytes are the only bytes a write changes outside the edited field;
  - for LF files, G5 and CE-09.1 hold byte for byte.

  G5 and the ACs are unchanged. CE-09.1's corpus is LF, and CE-09.4 has CRLF fixtures of its own
  (§5.13). (D2.)
- **Ruling (Jann): "the restricted YAML parser is ours" (G5) means our subset rules and our span
  patcher on top of `yaml-rust2`**, which is already pinned (0.13.0 in `Cargo.lock`). It does
  not mean a hand-written YAML lexer (directive 2). The probe Jann asked for passed (§4.2). (D3.)
- **Comments on text sections** are not decided yet. Proposal 0002 is open as its own PR
  (#15). This spec builds only what its ACs need, plus the interfaces 0002 asks of P02,
  marked "depends on 0002" (§6). (D4.)

### 4.2 Probe: the pinned crates on toolchain 1.99.1 (5 Nov 2026)

A scratch crate outside the repo ran about 60 inputs through `yaml-rust2` 0.13.0 (its `Parser`
with `MarkedEventReceiver`, and its public `Scanner`) and `pulldown-cmark` 0.13.6
(`Options::ENABLE_WIKILINKS`, `into_offset_iter`).

**`yaml-rust2` 0.13.0: positions**
- Every event and token carries a `Marker`: `line` from 1, `col` from 0, `index`.
- `index` and `col` count **characters (Unicode scalar values), not bytes.** The doc comment says
  "bytes", but the probe disagrees: in `title: Mère Ilse ö 😀 x\nkind: npc`, `kind` is at index
  23, its character offset; its byte offset is 28. P02 therefore uses `line` and `col` only, and
  converts them to bytes through its own line table (§5.2). A unit test pins this behaviour, so
  an upgrade that changes it fails loudly.
- Starts are given for every key, scalar and collection. A flow list's `]` has its own marker
  (`SequenceEnd`). A scalar's end is not given. P02 derives it by checking the decoded text
  against the source (§5.4, rule `one-line-scalar`): it never lexes.
- CRLF counts as one line break. P02 normalises line endings first anyway (§5.2).

**`yaml-rust2` 0.13.0: what CE-09.3 must reject**

| Construct | How the crate shows it |
|---|---|
| anchor `&a` | `Anchor` token at the `&`; the node's anchor id is not 0 |
| alias `*a` | `Alias` token and `Alias` event at the `*` |
| tag `!t` (YAML tags, not the `tags` field) | `Tag` token at the `!`; the node carries `Some(Tag)` |
| multi-line scalars `\|` and `>` | scalar style `Literal` or `Folded` |
| flow map `{…}` (also `{{title}}`) | `FlowMappingStart` token at the `{` |
| deeper nesting: a list of maps, a map in a map, a list in a list | event nesting (`MappingStart` inside `SequenceStart`, and so on) |
| tab indentation of a block | the crate's own error, "tabs disallowed within this context", with its position |

**What the crate does not report**, so a small rule of ours does (§5.4):
- duplicate keys (the events list them twice);
- a tab before a top-level key (accepted as separation);
- multi-line plain and quoted scalars (silently folded into one line);
- `...` (the parser stops and silently drops the rest);
- `?` complex keys (only the `Key` token's position differs from its scalar's);
- empty values `key:` (an empty plain scalar, positioned on the next line).

**`pulldown-cmark` 0.13.6**
- Ranges are byte offsets.
- `[[id]]` and `[[id|text]]` come as `Link { link_type: WikiLink { has_pothole } }` with exact
  ranges. `[[id#x]]`, `[[id^x]]`, `[[world/tower]]` and `[[ bad ]]` come as wiki links with the
  raw target; `![[x]]` comes as an `Image`.
- `[[]]` and an unclosed `[[` come as plain text.
- No link is produced inside code spans or code blocks.
- `> [!voice] …` arrives as a plain `BlockQuote(None)` whose first paragraph begins with the text
  events `[`, `!voice`, `]`. GFM alerts know only five upper-case types, and only when the
  marker stands alone.
- An attribute paragraph directly after a heading, and a voice block directly after attribute
  lines (S-901 Beat 4), parse as separate blocks.
- `utils::TextMergeWithOffset` merges split text events and keeps their offsets.

**Verdict: the probe passes.** The crates give every position the patcher needs, and every
construct CE-09.3 names is visible with its exact position. The six gaps above are each one small
rule over the crate's output.

### 4.3 Approaches (Jann picks in this review; the recommendation comes first)

**Where the YAML rules get their positions**
- (a) **Tokens and events** (recommended). The rules read the `Scanner`'s tokens for the
  constructs that have an indicator (`&`, `*`, `!`, `{`, `?`, `%`, `---`, `...`), and the
  `Parser`'s events for shape. Every error points at the offending character. Both come from the
  same crate; the parser runs its own scanner over the same text.
- (b) Events only. Simpler, but an anchor or tag error points at the node after it, not at the
  `&` or `!`.

**Body grammar**
- (a) **`pulldown-cmark` for blocks, links and code, plus three small grammars of ours**
  (recommended): callout titles, beat headings, and attribute lines. They read one line each,
  inside blocks the crate has already found.
- (b) `comrak`, which has an AST with source positions. It would add a second Markdown crate
  (not pinned, not probed), and the three grammars would still be ours: no Markdown crate knows
  the grammars of data model §9.
- (c) A line scanner of our own. Ruled out by directive 2.

**Patcher granularity**
- (a) **Replace the value's span; patch lists of both styles per item, and maps per entry**
  (recommended). Untouched items keep their bytes, their comments and their spacing, so data
  model §4 ("comments … survive") holds inside lists too, and writing back the current value
  rebuilds the same bytes (§5.7).
- (b) Rewrite the field's lines as a whole. This still meets CE-09.2, but drops own-line
  comments inside a block list, changes the spacing, and makes CE-09.1's round trip through the
  patcher fail on any list not written in the canonical form.

**What the round trips prove** (CE-09.1, CE-10.4)
- (a) **Through the model and the edit paths** (recommended). Setting every field to its current
  value through the patcher leaves the bytes identical; the body's blocks and gaps tile it
  exactly; writing back each link, note state and attribute through the span edit leaves the
  bytes identical.
- (b) `parse` then return the source. Trivially true, and proves nothing (readiness gap 5).

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Module (indicative) | Crate or tool | Our code, and why |
|---|---|---|---|
| Line endings, line table, positions | `text` | std | A line table over a `&str`; no crate does byte ↔ line/column ↔ UTF-16 for us without pulling in an editor crate |
| Span edits | `text::edit` | std | Apply sorted, non-overlapping replacements; ten lines |
| Splitting front matter and body | `record::split` | std | The two `---` lines of data model §2 (P01's rule) |
| YAML tokens and events | `yaml::input` | `yaml-rust2` 0.13 (`Scanner`, `Parser`) | Collect them once, with converted positions |
| Subset rules | `yaml::rules::*` | – | Data model §4 is our subset; one module per rule, one line each in the registry |
| Front-matter model | `yaml::model` | – | Fields, values, spans |
| Scalar ends, lexical class | `yaml::scalar` | – | Check the decoded text against the source; the lexicon of §5.5 |
| Patcher | `yaml::patch` | `similar` (item diff of lists, probed in the plan) | The edit rules of data model §4 |
| Value encoding | `yaml::encode` | our own parser as the oracle | Plain if it reads back the same, else double-quoted |
| Markdown blocks, links, code | `body::blocks` | `pulldown-cmark` 0.13 (`ENABLE_WIKILINKS`, offsets, `TextMergeWithOffset`) | – |
| Callout titles | `body::callout::{voice, note, design}` | – | Data model §9 grammars; one module per callout type, one line each in the registry |
| Beats and attributes | `body::beat` | – | Data model §9.4 grammar; one table of attribute shapes |
| Design-note paragraphs | `body::design_note` | – | Data model §9.2 |
| Errors | `error` | `thiserror` | Typed errors (G16) |
| Corpus manifest in tests | tests only | `toml`, `serde` | – |

New dependencies: `yaml-rust2` and `pulldown-cmark` for `theos-content` (both already in
`[workspace.dependencies]`), and `similar` (Apache-2.0; added to `[workspace.dependencies]` by
the plan after a probe). Dev-dependencies such as `rstest`, `proptest` or `insta` are the plan's
call. `[workspace.dependencies]` is not a protected path.

### 5.2 Text, line endings and positions

- **Input** is `&str`. A thin `read_record(path)` reads the bytes and decodes UTF-8. Invalid
  UTF-8 is an error with the line and column of the first bad byte. There is no folder walk.
- **A UTF-8 byte order mark** is an error at 1:1, "the record starts with a byte order mark".
  See the ruling in §9: dropping it on write would be a second exception to G5, and Jann has made
  only the CRLF one.
- **Normalisation.** `\r\n` and a lone `\r` both become `\n` before anything else. The model
  remembers that it normalised (`was_normalized()`), so P06 knows that saving an unchanged record
  would change its bytes. A lone `\r` is treated like CRLF so that positions agree with
  CodeMirror, which splits lines the same way.
- **Coordinates.** Every span is a byte range into the normalised (LF) text of the **whole**
  record, front matter included. For an LF file these are file offsets. For a CRLF file they are
  offsets into the text a write would produce, never into the bytes on disk. The same text always
  gives the same spans.
- **Positions for people:** line from 1, column from 1, counted in Unicode scalar values (a tab
  is one). This is `yaml-rust2`'s unit (plus one) and P01's Python column, so `theos check` and
  the P01 checker give the same columns. Lines and columns of a CRLF file equal those of its LF
  twin.
- **`TextIndex`**, built once per text:
  - byte offset ↔ (line, column);
  - byte offset ↔ UTF-16 offset (**depends on 0002**; P12's CodeMirror and 0002's anchors use
    UTF-16);
  - converting a position that is not on a character boundary is an error, never a panic.

### 5.3 Splitting a record

Data model §2 and P01's checker:
- Line 1 must be exactly `---`, else the error "a record starts with a `---` line" at 1:1.
- The front matter ends at the next line that is exactly `---`. Trailing spaces make it no
  delimiter. Without one, the error "front matter has no closing `---` line" at 1:1.
- The body is everything after the closing line. It may be empty and may lack a final newline.
- The front matter and the body parse independently. A body is parsed even when the front matter
  has an error, so P12 can show it, and `Body::parse` works on its own span.

### 5.4 Restricted YAML: the subset rules

Data model §4 is read as an **allow-list**: what it does not allow is an error. The rules run
over three inputs built once per front matter: the scanner's tokens, the parser's events, and
the lines. Each rule is a module with one job, registered with one line. The front matter
reports **the first error in document order**, with its line, column and span. The crate stops
at its first error too, so collecting more would only produce cascades. A crate error becomes
`YamlError` with its message and converted position.

**Allowed** (data model §4):
- keys `^[a-z][a-z0-9_]*$`, plain, each once per map;
- scalars: plain, single-quoted, or double-quoted with the escapes `\"` and `\\` only (P01's
  choice);
- lists of scalars, either one-line `[a, b]` (also `[]`, and a trailing comma, as P01 accepts)
  or `- item` lines at any indent;
- one level of map, whose values are scalars or one-line lists; its keys follow the key rule;
- comments on their own line or after a value; blank lines.

**Rejected**, each with line and column:

| Rule | Example | Detected from | CE-09.3 |
|---|---|---|---|
| `anchor` | `a: &x 1` | `Anchor` token | yes |
| `alias` | `b: *x` | `Alias` token | yes |
| `tag` | `c: !t v` | `Tag` token | yes |
| `block-scalar` | `d: \|`, `e: >` | scalar style; the position is the indicator after the key's `:` | yes (multi-line) |
| `one-line-scalar` | `m: a` / `  b`, or a quoted value over two lines | the decoded text does not match the source at its start (below) | yes (multi-line) |
| `flow-map` | `f: {a: 1}`, `id: {{title}}` | `FlowMappingStart` token | yes |
| `list-of-maps` | `g:` / `  - a: 1` | events | yes (nesting) |
| `nested-map` | `h:` / `  i:` / `    j: 1` | events | yes (nesting) |
| `list-in-list` | `- - a`, `[a, [b]]` | events | yes (nesting) |
| `one-line-list` | `w: [a,` / `  b]` (also a comment inside `[…]`) | `SequenceStart` and `SequenceEnd` on different lines | – |
| `top-level-map` | a scalar or a list as the whole front matter | events | – |
| `tab-indent` | a tab before the first non-blank character of a line | lines (the crate catches only the block cases) | data model §4 |
| `duplicate-key` | `l: 1` / `l: 2` | events, per map | data model §4 |
| `key` | `Key: 1`, `k-2: 3`, `"a": 1` | events: plain style, key pattern | data model §4 |
| `complex-key` | `? a` / `: b` | the `Key` token's position differs from its scalar's | – |
| `document-marker` | `...`, `%YAML 1.2` | `DocumentEnd`/`DocumentStart`/directive tokens | – |
| `empty-value` | `o:` with nothing below, `- ` with nothing after | an empty plain scalar; the position is right after the `:` or `-` | – |
| `escape` | `a: "x\ny"` | the double-quoted rule of `one-line-scalar` (below) | – |
| (crate) | `a: @x`, `` a: `x ``, `a: b: c`, inconsistent map indentation | the crate's error and position | – |

**`one-line-scalar`, the one rule that checks the crate's result.** For each scalar, P02
rebuilds the source spelling the decoded text must have, and compares it with the source at the
scalar's start:
- plain: the text itself;
- single-quoted: `'` + the text with `'` doubled + `'`;
- double-quoted: `"` + the text with `\` and `"` escaped + `"`.

If they differ, the scalar ran over a line break (the crate folded it), or used an escape the
subset does not allow. Both are errors at the scalar's start; the message names both causes, or
the one the plan can tell apart cheaply. If they match, the end of the rebuilt spelling is the
scalar's end, which the patcher needs. This compares two strings; it does not lex.

### 5.5 Scalars and their meaning (Jann's ruling D1)

- A `Scalar` holds its decoded `text`, its `style` (plain, single-quoted, double-quoted), its
  `span` (quotes included) and its line and column.
- `as_text()` always works. An id, a link and an enum value read the text: `id: 42`, `id: true`
  and `accuracy: false` are the texts `42`, `true` and `false`.
- `as_int()`, `as_decimal()` and `as_bool()` read the text of a **plain** scalar by P01's
  lexicon, and return a typed error otherwise:
  - integer `-?[0-9]+` (so `007` is 7; a value outside `i64` is an error);
  - decimal `-?[0-9]+\.[0-9]+`, kept as its text with `to_f64()`;
  - boolean `true` / `false`.

  `null`, `~`, `yes`, `no`, `1e5`, `0x1F`, `+1`, `.5`, `E7` and `T-012` are text. A quoted
  scalar is never a number or a boolean: quoting means text.
- `lexical()` gives the class of a plain scalar (integer, decimal, boolean, text), for open maps
  without a schema (`inputs`, `results`).
- Which accessor a field uses is P03's schema. P02 never decides it.

### 5.6 The front-matter model

```text
FrontMatter { fields: [Field], span }
Field  { key, key_span, value, lines, comment }     lines: start of the key's line to the end of
                                                    the last value line, newline included
Value  = Scalar(Scalar)
       | List { style: Flow | Block, items: [Item], span }
       | Map  { entries: [Entry] }
Item   { scalar, lines (block), comment }
Entry  { key, key_span, value: Scalar | flow List, lines, comment }
```

- `comment` is the span of a `# …` after the value, on the value's last line.
- Own-line comments and blank lines belong to no field. They keep their place.
- Fields are found by key and iterated in file order.

### 5.7 The patcher (CE-09.2)

**Operations:** `set(key, value)`, `remove(key)`, `set_entry(key, entry, value)`,
`remove_entry(key, entry)`. A value is text, a list of texts, or a map of entries; the caller
gives raw text (D1), and P03 turns typed values into it. `set` and `set_entry` insert what is
missing, using a **field order the caller passes**. The order comes from P03's registry, never
from a list in P02 (CE-11.5). The data model gives the order (§5 then the kind's table), and P01
says the templates follow it, with `tags` after `status`.

**What each operation changes**
- **Scalar → scalar:** only the value's span. The spacing before it and a trailing comment stay.
- **Lists, both styles: per item**, through an item diff (`similar`). Items that keep their value
  and order keep their bytes.
  - **Block list:** a kept item keeps its line, comments included. A removed item's line goes.
    A new item gets the line `<indent>- <value>`, with the indent of the list's first item,
    after the item that precedes it in the new list. Own-line comments and blank lines inside the
    list stay after the line they followed.
  - **Flow list:** a kept item keeps its spelling and the separator before it, so `[a,b]`,
    `[ a ]`, `['a', "b"]` and a trailing comma survive. A removed item goes together with the
    separator before it (for the first item, the one after it). A new item is written as
    `, <value>` after the item that precedes it, or as `<value>, ` before the first kept item,
    or as `<value>` into `[]`. Nothing outside `[` … `]` changes.
- **Map:** per entry, by key. Setting an entry replaces its value span. A new entry goes after
  the last present entry that precedes it in the caller's entry order, or last. Removing an entry
  deletes its line; removing the last entry removes the field, since `key:` alone is an empty
  value.
- **Changing a value's shape** (a scalar to a list, a list to a scalar): the field's lines are
  rewritten. A trailing comment on the key line is kept.
- **New field:** inserted after the last line of the last present field that precedes it in the
  caller's order (data model §4). If none precedes it, it goes directly before the first field.
  If the front matter is empty, it goes on the first line. A new list is written as a one-line
  flow list `[a, b]`, as in the data model's examples. A new map is block style, indented two
  spaces.
- **Remove a field:** its lines go. Own-line comments above it stay: P02 does not guess what they
  belong to.
- A block list never turns into a flow list, nor the other way round, unless the caller asks
  for it.

**Encoding a value**
- Keep the existing style if it can hold the new text.
- Otherwise write it plain if it reads back as the same plain text. Our own parser is the oracle:
  `key: <text>`, or `key: [<text>]` inside a flow list, is parsed and compared.
- Otherwise write it double-quoted, with `\` and `"` escaped.
- A text with a line break, or with a control character other than a tab, cannot be
  represented. It is a typed error, and nothing is written. A tab inside a value is kept: YAML
  allows it, and the corpus has one.

**Writing back the current value is not a short-cut.** There is no "value unchanged, skip"
branch. A scalar written in its existing style rebuilds exactly its source spelling: this is the
same rebuilding that rule `one-line-scalar` (§5.4) checked when the record was parsed. A kept
list item keeps its bytes through the item diff. So the CE-09.1 round trip runs the real edit
path, and the bytes come out identical because the encoder and the item diff are exact.

**Verification.** After the edits are applied, the new front matter is parsed again:
- it must parse;
- every other field must keep its value and the bytes of its lines;
- the changed field must read back the requested value.

If any check fails, the patch fails with `PatchError::Verify` and returns no text. The output is
LF, because the model's text is.

**Mechanics.** The patcher produces `TextEdit { span, replacement }` lists, which the shared
`text::edit` applies. `Record::with_edits` returns the new text and a new `Record`.

### 5.8 Body: blocks and wikilinks

`pulldown-cmark` runs on the body with `Options::ENABLE_WIKILINKS` only. GFM tables would split
`[[id|text]]` at its pipe, and the other options change nothing P02 reads.

- **Blocks.** The body model is the list of top-level blocks with their spans. Blocks and the
  gaps between them tile the body exactly; that tiling is what CE-10.4 proves. Block kinds:
  - beat heading, beat attributes;
  - callout (voice, design, note, other);
  - design-note paragraph;
  - other heading;
  - other (paragraph, list, code, HTML, block quote, thematic break, table text).
- **Wikilinks.** Every link in the body appears once in `body.links()`: in prose, callout titles,
  beat attributes, lists, block quotes and callouts. Each link has:
  - `span` (the whole `[[…]]`), `target_span` (the id), `fragment` and `fragment_span`
    (`#3`), `shown_span` (`|text`);
  - the block it sits in.

  P06's rename (CE-20.3) rewrites exactly the `target_span`s. Links in the front matter are
  plain ids (data model §2), and their spans are the scalars' spans.
- **What a link may be** (the shape only; whether the id exists is P03's, CE-12.3):
  - `[[target]]` or `[[target|shown text]]`;
  - `target` is `[A-Za-z0-9-]+`, the union of the id alphabets of data model §3, optionally
    followed by `#` and a beat number (`[[S-004#3]]`, data model §9.4).
- **Malformed links** (CE-10.3), with line and column:
  - an unclosed `[[`, or an empty `[[]]` or `[[|x]]` (found as `[[` left in merged text outside
    code);
  - a target with spaces, `/` (the path form `[[world/tower]]`: a link names an id, not a
    path), `^` (block references), or any other character outside the alphabet;
  - a fragment that is not a number;
  - an embed `![[…]]` (not in the data model).

### 5.9 Callouts: voice, design, note

- **What a callout is.** A block quote, at any depth, whose first paragraph starts with
  `[!type]`, optionally followed by Obsidian's fold marker `+` or `-`. `type` is
  `[A-Za-z][A-Za-z0-9-]*`, matched without regard to case (as Obsidian does) and stored in
  lower case.
- **The title** is the rest of the first line. Its tokens are separated by ` · `: a space,
  U+00B7, a space.
- **The content** is everything after the title line. Lazy continuation lines belong to it, as
  CommonMark says.
- **The caption** (voice and note) is the last line of the callout's last paragraph, when it
  starts with `— ` (U+2014 and a space). A line starting with `-- ` or `- ` is not a caption.
- **Registered types**, one module each:

| Type | Title grammar | Exposed | Malformed title (CE-10.3) |
|---|---|---|---|
| `voice` | speaker link, then any number of belief links, then optionally one bare token (the tier) | speaker, beliefs, tier token, caption, content spans | no speaker link first; a bare token that is not last; an empty token; a token that is neither a link nor one bare word; a wrong separator (`·` without spaces, `•`, `\|`) |
| `note` | `n<k>`, then optionally one version link, then the state: `open`, `dropped`, `resolved by [[id]]`, or another bare word | id `k` and its span, version link, `NoteState` (`Open`, `Dropped`, `ResolvedBy(link)`, `Unknown(text)`) and its span, caption, text | no `n<k>`; no state; `resolved by` without a link; extra tokens; a wrong separator |
| `design` | free text, optional; its links are tokens | title text and links, content span | never |
| any other | kept as text | type, title span, content span | never |

- **Unknown is data, not an error.** An unknown note state (`· closed`) is `Unknown`. P04
  reports it (CE-15.5).
- **Tier tokens** are kept as written; P03 and P04 check them.
- **A note's text** (P09 sends it to generators, `docs/generators.md` §3) is its content without
  the title line and without the caption. Rules:
  - block-quote markers removed;
  - soft breaks become one space;
  - paragraphs are separated by a blank line;
  - links keep their source spelling (`[[id]]`, `[[id|text]]`).

  The caption is the author's signature (`— Jann`), not part of the note.

### 5.10 Beats

- **A beat heading** is a level-2 heading whose text starts with `Beat ` and a digit. It must
  read `Beat <n> — <name>`: `n` digits, then a space, U+2014 and a space, then a name that is
  not empty. Anything else that starts this way is a malformed beat heading, reported with its
  line (S-901 Beat 5: `Beat 5 - …`). Other level-2 headings are ordinary headings.
- **A beat** runs from its heading to the next level-2 heading, or to the end of the body.
- **Attribute lines** are the lines directly after the heading, with no blank line between, of
  the form `key:: value`:
  - `key` follows the key rule;
  - one or more spaces after `::` (S-901 has three);
  - the value is not empty.

  The run ends at the first line not of that form. The rest of that paragraph is prose.
- **Value shapes** of the five keys of data model §9.4:
  - `at`, `with`, `says`, `reveals`: one or more wikilinks, separated by `, ` (a comma and a
    space);
  - `when`: one bare token.

  A value that does not fit its key's shape is a malformed beat attribute (CE-10.3): for
  example `at:: fates-shrine`, `with:: [[a]] [[b]]`, `when:: [[x]]`, or `at::` with no value.
- **Unknown keys** (`synthetic::`, S-901) keep their raw value and any links in it. They are not
  an error; P04 judges them (P10 ruling 34 reads the same way).
- **Misplaced attributes.** A blank line after the heading, and then a paragraph whose first
  line has the attribute form, is reported as "attribute lines must directly follow the beat
  heading".
- **Kept as data:** a beat without attributes (S-901 Beat 3); a key twice; any number of links in
  `at::`. Counts, numbering and uniqueness are P03's and P04's. A beat is referred to as
  `S-004#3` (data model §9.4).
- Beats are parsed in every body. Whether a kind may hold them is P03's call.

### 5.11 Design notes

Two forms, both exposed with their spans, because P04's word check skips them:
- `> [!design]` callouts (§5.9), at any depth;
- a paragraph whose text starts with `Design note:`, at the top level or inside a callout. The
  note runs to the end of its paragraph, soft-wrapped lines included. Its "title token" (CE-10.2)
  is the `Design note:` marker.

P02 also exposes the voice blocks' content spans. P04 builds the in-world text from them
(data model §9.4, last paragraph).

### 5.12 Errors, and the line between P02 and P04

- **P02 reports what it cannot parse.**
  - The front matter: the first error (§5.4).
  - The body: every malformed construct, as `Malformed { kind, message, span, line, column }`.
    Kinds: unclosed or empty link, bad link target, embed, callout title, beat heading, beat
    attribute, misplaced attributes.
- **P02 keeps what is unknown but well-formed, as data:** unknown callout types, unknown
  attribute keys, unknown note states, tier tokens, beat numbers, duplicate beat keys. P04 judges
  them, so one fault never yields two findings.
- **Error types** use `thiserror` (G16). Every public function that returns `Result` documents
  its errors (clippy `missing_errors_doc`). No `unwrap`, `expect` or `panic` outside tests.

### 5.13 Fixture corpus

`crates/theos-content/tests/fixtures/corpus/`, shared with P03 to P05 (CE-11.1 validates "every
corpus record"):

- **`records/` and `malformed/` form one content project**, which P03 indexes as a whole
  (CE-11.1, CE-12.3):
  - ids are unique across both folders;
  - file names equal ids;
  - paths follow data model §2 and §6.

  "Valid" here means valid for P02: the front matter parses, and the body has no malformed
  construct. Dangling links and schema errors are allowed: CE-11.1 accepts a corpus record that
  "reports exact errors", and P03 reports them.
- **`records/`**: valid records, all LF; CE-09.1 and CE-10.4 run over all of them.
  - The data model §14 examples, one file each, at their paths. `S-004` is CE-10.1's record. The
    map example becomes two files. Its `…` hashes become 64 hex digits of an obviously synthetic
    value.
  - The 20 P01 templates (tower-of-theos `templates/records/`), with `{{title}}` replaced by a
    fresh id per kind that matches the kind's pattern and collides with no other corpus id (not
    the sample ids of P01's tests, which reuse example ids).
  - Copies of the P10 corpus files (`research/P10-prose-editor/corpus/records/`), copied and
    never imported. Left out are the seven that repeat a data model example (f04-choir-hall,
    mother-ilse, C-031, S-004, f04-terrain, f04-terrain-v1, fates-shrine-v5). Elsewhere:
    - S-901, whose Beat 5 heading is malformed, goes to `malformed/`;
    - synthetic-crlf goes to `crlf/`.

    Three sentences that read like lore claims are reworded neutrally (ruling in §9): B-901's
    `[[T-012|Listeners are the unwoven Tuners]]`, synthetic-voices-in-world's "the Cutter is
    near", and synthetic-links-explaining's "the operator-plate names a Listener".
  - Synthetic records for every allowed form of §5.4, and for every block form of §5.8 to §5.11:
    - comments in every place, and blank lines;
    - every quoting style, and flow lists with odd spacing, mixed quotes and a trailing comma;
    - a tab inside a value, and no final newline;
    - non-ASCII text, U+00A0, combining marks, emoji, and `·` and `—` in titles.
- **`crlf/`**: CRLF twins of some records, plus mixed and lone-CR files. A nested
  `.gitattributes` with `* -text` keeps them byte-exact (the root has `* text=auto eol=lf`; P10
  did the same). The tests assert that the input really holds `\r\n`.
- **`rejected/`**: one file per row of the rejection table (§5.4), plus BOM, no opening line,
  no closing line, and invalid UTF-8 (built in the test). `expected.toml` lists each file's rule,
  line and column.
- **`malformed/`**: records with valid front matter whose bodies hold malformed links, callout
  titles, beat headings and attributes. `expected.toml` lists their kinds and lines. They are
  records in CE-09.1's sense, and their bodies round-trip like any other (CE-10.4). The files in
  `rejected/` are not records: the parser refuses them.
- Every file says it is synthetic: `# synthetic fixture (P02), not canon` in its front matter,
  or the plan's equivalent for files whose front matter is the subject.

### 5.14 How each AC is proven

Tests carry `// AC: <id>` directly above a `#[test]` or `#[rstest]` function (P00's `ac-trace`).
Property tests run inside a tagged `#[test]`, or stay untagged next to the tagged example tests:
a tag inside `proptest!` has no owner and fails `ac-trace`.

| AC | Proven by |
|---|---|
| CE-09.1 | For every file in `records/` and `malformed/`: the parse succeeds; the text written back equals the input bytes; setting every field and every map entry to its current value through the patcher, one at a time and all together, leaves the bytes identical; the model parsed from that output equals the first one. |
| CE-09.2 | An edit matrix over the corpus. Cases: a scalar (plain to plain, to a value that needs quotes, with a trailing comment); a flow list; a block list (adding, removing and reordering items, with comments inside); a map entry (`lens.stance`); inserting a missing field at the start, in the middle, at the end, and `tags` after `status`; removing a field and a map entry. Each case asserts that every line outside the field's lines is byte-identical (for an insert: only new lines; for a removal: only that field's lines gone), and that the result reads back the expected model. A property test with random values runs untagged next to it. |
| CE-09.3 | Every file in `rejected/` fails with its expected rule, line and column. One test per construct the AC names (anchors, aliases, tags, multi-line scalars, flow maps, deeper nesting), plus the other rows. A case with non-ASCII text before the error proves that columns count Unicode scalar values. |
| CE-09.4 | Each CRLF fixture (its `\r\n` asserted) parses to the same model, lines and columns as its LF twin; every write returns text without a CR; mixed and lone-CR inputs too; `was_normalized()` is true. |
| CE-10.1 | `S-004` (the data model example) parses into Beat 1 (at `fates-shrine`, with `mother-ilse`, says `B-016`, when `T1`) and Beat 2 (at `fates-shrine`, reveals `C-031`, when `T1`). Each link's `target_span` slices to its id. |
| CE-10.2 | Voice blocks (speaker, beliefs, tier, caption, all title forms of the corpus), `[!design]` callouts and `Design note:` paragraphs, and notes with `n<k>`, a version link and the three states, each state recognised. An unknown state is kept as `Unknown`. |
| CE-10.3 | Every file in `malformed/` reports each construct with its line (and column): malformed beat attributes, callout titles of every registered type, beat headings, and every malformed link form of §5.8. |
| CE-10.4 | For every body in `records/` and `malformed/`: blocks and gaps tile it exactly; for every link target, fragment and shown text, note state, beat attribute value and callout title token, a span edit that writes back its own text leaves the bytes identical, which proves the spans exact. |

The gate as merged applies: no `allow` or `warn` attributes; a reason on every `expect` and
`ignore`; 80 % line coverage of `crates/theos-content/src/**`, error paths included; `missing_docs`.
P02 needs no OS-specific code, so no `windows` or `macos` labels. `cargo xtask gate` still runs
on Windows before the phase is claimed done (CLAUDE.md).

## 6. Interfaces other phases use

| Phase | Uses |
|---|---|
| P03 | `Record`, `FrontMatter`, `Scalar` with its accessors (D1), field and entry spans, field-order injection for inserts (the registry provides the order), body links with spans, beats, notes, and the corpus (CE-11.1) |
| P04 | `Malformed` items (kind, span, line, column) as the input for findings; design-note, design-callout and voice-content spans for the word check; note states including `Unknown` (CE-15.5); beat attributes for reveal checks |
| P05 | Errors print as `path:line:column: message`, the same positions everywhere |
| P06 | The patcher (CE-20); `TextEdit` and `apply_edits`; link `target_span`s in the body and scalar spans in the front matter for renaming an id (CE-20.3); `was_normalized()` for saving CRLF records |
| P09 | Notes: id, version link, state with its span (to resolve a note, CE-30.1), and text without the caption (run requests, CE-29.6) |
| P12 | The body model as the authority for links, beats and callouts (decision 0001); positions in UTF-16 through `TextIndex` (**depends on 0002**, see below); the corpus for its display tests |
| 0002 (comments) | **Depends on 0002.** Spans are byte offsets into the LF text of the whole record, the same for the same text; line and column as in §5.2; `TextIndex` converts to UTF-16 and back. Anchors (P05 under 0002) are stored in these coordinates, so they must not change after P02. If 0002 is not accepted before P02's plan is written, the UTF-16 conversion moves to P12, and everything else stays. |

## 7. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| `yaml-rust2` changes `Marker.index` or `col` in an update (its doc says bytes; it counts characters) | Wrong spans everywhere | Use `line` and `col` only, never `index`; a unit test pins the unit on non-ASCII; `Cargo.lock` and `--locked` |
| The crate's folding or escaping changes | `one-line-scalar` rejects valid values | It fails closed: a change shows up as rejections in the corpus tests, never as silent wrong spans |
| Hollow byte-stability tests | CE-09.1 and CE-10.4 pass without proving the edit paths | §4.3: round trips through the patcher and the span edit, tiling of blocks, an edit matrix |
| The body grammar and CodeMirror's display parse disagree (P12) | Wrong rendering, not wrong data | P02 is the authority (decision 0001); P12 reuses the corpus |
| The synthetic corpus misses constructs of the lore import (CE-04) | Gaps appear later | Broad synthetic forms; real records added after the import (umbrella) |
| Git normalises a CRLF fixture | CE-09.4 passes without testing anything | Nested `-text`, and the test asserts `\r\n` |
| `ac-trace` cannot see tags inside `proptest!` | A dangling tag fails the gate | Tags only above `#[test]` / `#[rstest]` functions (§5.14) |
| Drift from P01's Python checker until `theos check` replaces it (P05) | Templates or records judged differently | Same lexicon, escapes, tab rule, delimiters and empty-value rule. Known differences: P02 accepts `id: 42` and `title: 1984` (D1), and its messages differ for crate errors |
| The item diff of lists | A rare list edit touches a neighbour item or separator | Verification after every patch (§5.7); the CE-09.2 matrix covers reorders and duplicates |
| Directive 3: fixes keep leaking (P00's lexer history) | Review rounds without end | The rules sit on the crate's tokens and events; if reviews keep finding new YAML forms that slip through, the class gets named and the approach revisited, not patched case by case |

## 8. Review Focus — inputs most likely to break it

- **Positions:** non-ASCII before a token (`è`, emoji, combining marks, U+00A0); a tab inside a
  value; the first and last line; a CRLF file's columns; UTF-16 at a surrogate pair.
- **Line endings:** CRLF, mixed, lone CR; no final newline; a body that is empty or only a
  newline; `---` with trailing spaces; `---` as a thematic break in the body; a setext heading
  underline in the body.
- **Scalars:** `true`, `false`, `42`, `007`, `-3`, `2.99`, `1e5`, `~`, `null`, `yes` as ids,
  titles and enums; `'It''s'`; `"a \"b\" c"`; `"\n"`; a `#` without a space (`b#c`); a URL with
  `#`; a binary ref with `@`; `{{title}}`; trailing spaces after a value; an empty `""`.
- **Lists and maps:** `[]`; a trailing comma; `[a,b]`, `[ a ]`, `['a', "b"]` written back
  unchanged and with one item added or removed; block items at indent 0; comments between items;
  a comment after `key:` with a block below; `lens` with one entry removed; duplicate entry keys;
  inconsistent indents.
- **Patcher:** inserting before the first field, after a field followed by comment lines, into
  an empty front matter; a field whose value is on the next line; changing a scalar to a list;
  a value that needs quotes; a value with a line break (must fail); a new value that equals the
  old one.
- **Links:** `[[a|b|c]]`; a link at line start and end and next to punctuation; links in code
  spans, fenced code, HTML comments, callout titles, attribute lines, lists; `[[`, `[[]]`,
  `[[ x ]]`, `[[world/tower]]`, `![[x]]`, `[[S-004#3]]`, `[[S-004#x]]`, upper-case slugs.
- **Callouts:** `[!VOICE]`, `[!voice]-`; nested callouts; a voice block directly after
  attributes; lazy continuation; empty `>` lines; a caption next to `-- `; a note caption; a
  `·` without spaces; `n3` missing; `resolved by` without a link; an unknown state; a design
  callout with a title.
- **Beats:** extra spaces after `::`; a blank line after the heading; an unknown key; a beat
  without attributes; `Beat 5 - x`; `## Beats`; `## Beat x — y`; several links with and
  without commas; prose directly after the attributes in the same paragraph.

## 9. Rulings (Claude's calls; open to change in this review)

- Ruling: the YAML rules use the scanner's tokens and the parser's events (§4.3) — exact
  positions for every indicator, both from the pinned crate — cost if wrong: one extra pass over
  a few hundred bytes per record.
- Ruling: positions use `Marker.line` and `col` only, never `index` — the probe shows the unit
  is characters while the doc says bytes — cost if wrong: none.
- Ruling: columns count Unicode scalar values from 1; spans are bytes of the LF text of the
  whole record — this matches `yaml-rust2` and P01's checker; bytes are Rust's string unit;
  whole-record spans let one span address the front matter and the body — cost if wrong: one
  conversion layer for UTF-16 users (P12, 0002), which `TextIndex` provides.
- Ruling: a lone `\r` is normalised like CRLF — CodeMirror splits lines the same way, so
  positions agree — cost if wrong: a classic-Mac file reads instead of failing.
- Ruling: a UTF-8 BOM is an error at 1:1 — dropping it on write would be a second exception to
  G5 that Jann has not made — cost if wrong: a record with a BOM needs one manual fix.
- Ruling: the front matter reports its first error only — the crate stops at its first error,
  and more would be cascades — cost if wrong: a file with three errors takes three runs to fix.
- Ruling: data model §4 is an allow-list, with P01's lexicon, its escapes `\"` and `\\` only,
  `[]`, a trailing comma, and block items at any indent — P01's checker accepted the templates by
  these rules, and divergence would be confusing — cost if wrong: a form Jann wants needs a data
  model change.
- Ruling: an empty value (`key:` with nothing below, `- ` with nothing after) is an error — as in
  P01; the data model has no empty value — cost if wrong: optional fields must be left out
  rather than left empty, which the templates already do.
- Ruling: the patcher keeps the existing quote style when it can, writes plain when the value
  reads back the same, and otherwise double-quotes; text with a line break is refused — quotes
  are never required (D1), and the parser is its own oracle — cost if wrong: a stylistic
  preference for single quotes is not followed.
- Ruling: new lists are written as one-line flow lists, new maps as block maps with two spaces —
  as in the data model's examples — cost if wrong: a style change the plan can flip.
- Ruling: lists of both styles are patched per item, with `similar` for the item diff, and maps
  per entry — data model §4 says comments survive, which a rewrite would break inside a block
  list; a flow list re-encoded as a whole would turn `[a,b,]` into `[a, b]` and fail CE-09.1's
  round trip — cost if wrong: one dependency more.
- Ruling: there is no "value unchanged" short-cut in the patcher; the round trip runs the real
  encoder and item diff — a short-cut would make CE-09.1 hollow — cost if wrong: none.
- Ruling: a tab inside a value is kept; line breaks and other control characters are refused —
  YAML allows the tab, and P02 must write back what it reads — cost if wrong: none.
- Ruling: `records/` and `malformed/` form one content project with unique ids; P10 copies that
  repeat a data model example are left out, and the filled templates get fresh ids — P03 indexes
  the corpus as a whole (CE-11.1, CE-12.3), and duplicates would fill its tests with findings —
  cost if wrong: none.
- Ruling: every patch is verified by parsing its result — a wrong edit fails instead of being
  written — cost if wrong: a few microseconds per edit.
- Ruling: `pulldown-cmark` runs with `ENABLE_WIKILINKS` only — tables would split `[[id|text]]`
  at its pipe; other extensions change nothing P02 reads — cost if wrong: a table renders as a
  paragraph in P02's block list, which no grammar reads.
- Ruling: a link target is `[A-Za-z0-9-]+` with an optional `#<number>`; paths, block
  references, other fragments and embeds are malformed — the data model defines `[[id]]`,
  `[[id|text]]` and beat references only; whether an id exists is P03's (CE-12.3) — cost if
  wrong: an Obsidian habit (`#heading`, `![[…]]`) is reported until the data model allows it.
- Ruling: callout types match without regard to case, as in Obsidian, and fold markers are
  accepted — Obsidian renders `[!Voice]-` as a voice block, so the word check must see it — cost
  if wrong: P04 may add a style warning.
- Ruling: callouts are recognised at any depth — Obsidian renders nested callouts — cost if
  wrong: none for valid records.
- Ruling: the voice title is a speaker link, any number of belief links, and an optional final
  bare token (the tier), separated by ` · ` — the data model's example and its text ("the
  speaker, optionally the beliefs voiced and the tier") — cost if wrong: a title form the data
  model meant is reported as malformed.
- Ruling: captions are recognised on voice and note callouts, and a note's text excludes its
  caption — the data model's note example ends with `— Jann`, a signature, not feedback for a
  generator — cost if wrong: generators miss who wrote a note.
- Ruling: `[!design]` takes optional free text as its title; a `Design note:` paragraph runs to
  the end of its paragraph — data model §9.2 defines neither; the word check needs a span; a
  hard-wrapped note continues on the next line — cost if wrong: P04's skipped span is a paragraph
  instead of a line.
- Ruling: several links in an attribute are separated by `, ` — the data model's examples show
  single links; a comma list is the common Markdown habit — cost if wrong: a data model note
  fixes the separator, and P02 changes one shape.
- Ruling: attribute lines must follow the beat heading directly; attribute-like lines after a
  blank line are reported — data model §9.4 says "followed directly", and silent prose would hide
  the mistake — cost if wrong: a writer must remove one blank line.
- Ruling: P02 reports what it cannot parse and keeps the unknown but well-formed as data
  (unknown callout types, attribute keys, note states, tier tokens) — one fault yields one
  finding, in P04 (CE-15.5) — cost if wrong: none.
- Ruling: beats and callouts are parsed in every body, whatever the kind — P02 knows no kinds;
  P03 decides where beats are allowed — cost if wrong: none.
- Ruling: three P10 corpus sentences that read like lore claims are reworded neutrally in the
  copies — CLAUDE.md forbids inventing lore canon in fixtures, and this session cannot check
  canon — cost if wrong: Jann confirms them and they are restored.
- Ruling: the corpus lives in `crates/theos-content/tests/fixtures/corpus/` and is shared by
  P03 to P05 — CLAUDE.md's fixture rule; P03's CE-11.1 runs on the same records in the same
  crate — cost if wrong: none.
- Ruling: P02 builds no typed body edits (rename a link, resolve a note) — no P02 AC needs them;
  exact spans and one generic span edit are what P06 and P09 build on — cost if wrong: P06 and
  P09 each add a few lines.
- Ruling: the UTF-16 conversion in `TextIndex` depends on 0002 — Jann asked that the interfaces
  0002 needs be marked so — cost if wrong: P12 builds it instead.

## 10. Left to the builder

- Module and type names (the ones above are indicative), and the public API surface, as long as
  §6 is served.
- The exact error messages, and whether `one-line-scalar` tells the two causes apart.
- Test layout: a loop per AC or `rstest` `#[files]`, snapshot tests or not, and the property
  tests.
- The synthetic fixtures' wording, within the rules of §5.13.
- Whether `similar` passes its probe. If it does not, the plan names the alternative and why.

## Appendix A. Readiness gaps and where they are settled

| Gap (readiness check, 4 Nov 2026) | Settled in |
|---|---|
| 1 CE-09.4 vs CE-09.1 and G5 | §4.1 (Jann, D2) |
| 2 G5 "ours" vs directive 2 | §4.1 (Jann, D3), §4.2 |
| 3 CE-09.3 "tags"; tabs; duplicates; nesting | §5.4 |
| 4 CE-09.2 coverage (insert, remove, lists, maps, quoting, comments) | §5.7, §5.14 |
| 5 Hollow round trips | §4.3, §5.14 |
| 6 Corpus location, parts, sharing | §5.13 |
| 7 Design note title tokens and span | §5.9, §5.11 |
| 8 Voice title grammar, fold markers, captions | §5.9 |
| 9 Note title and the caption in its text | §5.9 |
| 10 P02/P04 boundary | §5.12 |
| 11 Beats (blank line, no attributes, hyphen, unknown key, numbers, separators) | §5.10 |
| 12 Wikilink forms, code, malformed vs dangling | §5.8 |
| 13 The YAML lexicon | §5.4, §5.5 |
| 14 Scalar typing | §4.1 (Jann, D1), §5.5 |
| 15 Field order | §5.7 |
| 16 Position units | §5.2 |
| 17 Scope edges (disk, UTF-8, folders) | §3, §5.2 |
