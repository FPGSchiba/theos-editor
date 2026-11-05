# P02 · Records and grammars — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (`CLAUDE.md` makes it the execution method) to implement this plan task by task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** `theos-content` reads the text of any record into a model with exact positions, writes
front-matter changes that touch only the changed field's lines, and parses the body grammars of
data model §9 (beats, voice blocks, design notes, notes, wikilinks).

**Architecture:** Text in, text out; no folders, kinds or index. `text` owns spans, positions,
line endings, UTF-16 offsets and span edits. `record` splits a record into front matter and body
and parses both independently. `yaml` runs `yaml-rust2`'s scanner and parser once per front
matter, checks the data model §4 allow-list with one module per rule, builds the model, and
patches it with span edits that a re-parse verifies. `body` runs `pulldown-cmark` once per body;
small grammars (links, one module per callout type, beats, design notes) read its events.

**Tech stack:** Rust 1.99.1 (edition 2024) · yaml-rust2 0.13.0 · pulldown-cmark 0.13.6
(`ENABLE_WIKILINKS` only) · similar 2.7 (new) · thiserror 2 · tracing 0.1. Dev: proptest 1.12
(new, `default-features = false, features = ["std"]`), toml 1, serde 1, tempfile 3.

**Spec:** `docs/specs/phases/P02-records-and-grammars.md` (approved and merged, #16). Read it with
this plan. Where this plan is silent, the spec decides; where they differ, the spec wins.
Decision 0002 (#15) is merged, so the UTF-16 conversion of spec §5.2 and §6 is in scope (Task 2).

**Probes run while planning (5 Nov 2026, toolchain 1.99.1, scratch crates outside the repo)**
- `similar` 2.7.0 (Apache-2.0) builds. `similar::capture_diff_slices(Algorithm::Myers, &old,
  &new)` returns `DiffOp::Equal { old_index, new_index, len }`, `Delete { old_index, old_len,
  new_index }`, `Insert { old_index, new_index, new_len }` and `Replace { old_index, old_len,
  new_index, new_len }`. `[a,b,c]→[c,a,b]` gives Insert(old 0), Equal(old 0, len 2), Delete(old
  2); `[a,b]→[x,y]` gives one Replace; `[a,a,b]→[a,b,a]` gives Equal, Insert, Equal, Delete. This
  is the probe the spec asked the plan's first task to run; Task 15 adds the dependency.
- `pulldown-cmark` 0.13.6 with `ENABLE_WIKILINKS`:
  - `[[a|b|c]]` is a link with `dest_url` `a`, `has_pothole: true`, and shown text `b|c`;
    `[[S-004#3]]` has `dest_url` `S-004#3`; `[[ x ]]` has `dest_url` ` x `.
  - `![[x]]` is a `Tag::Image` whose `link_type` is `WikiLink`.
  - `[[]]`, `[[|x]]` and an unclosed `[[open` arrive as split text events (`[`, `[`, …).
  - Code spans and inline HTML comments (`<!-- [[c]] -->`) contain no links.
  - In `> [!voice] …` the first `Paragraph` range starts right after `> `; its first line is the
    title line. A lazy continuation line belongs to that paragraph; a lone `>` line ends it.
  - A setext `Beat 1 — X` over `---` is an `H2` whose text event excludes the underline.
  - Block ranges include their final newline.

## Global Constraints

Umbrella global constraints, verbatim. CLAUDE.md asks for G1–G16; G17 and G18 were added to the
umbrella later and are copied too.

- **G1** Repositories: `theos-editor` (this repo) holds the editor. `tower-of-theos` holds the
  content project and the map toolset. `Theos-Engine` is never a dependency, except behind the
  cargo feature `engine`, which no phase here builds.
- **G2** Platforms: Windows 11 x64 (primary), Linux x64, and macOS 13 or later on Apple silicon
  and Intel (universal builds). Every PR is checked on Linux; Windows and macOS are tested
  weekly, on demand (a label on the PR) and before every release (`docs/gate.md` §1).
- **G3** Rust: stable toolchain pinned in `rust-toolchain.toml`, edition 2024, `unsafe_code`
  forbidden, `Cargo.lock` committed, all builds `--locked`.
- **G4** Crates: `theos-content`, `theos-vcs`, `theos-studio` (libs); `theos`, `theos-mcp`,
  `theos-editor` (bins); `xtask`. Paths `crates/<name>`, `xtask/`. UI in `editor-ui/`
  (Svelte 5, TypeScript, pnpm).
- **G5** Content files follow `docs/specs/data-model.md`; the restricted YAML parser is ours;
  any edit leaves untouched bytes identical.
- **G6** Git only through the `git` command line; credentials only through the user's
  credential helper.
- **G7** MCP: stdio only, `rmcp`, tool names `^[a-z][a-z0-9_]*$`, agents only propose.
- **G8** `never-confirm`, `no-overshoot`, `inworld-vocab` are always blocking; nothing lowers them.
- **G9** Text of a truth with target `never` never appears in an LLM prompt.
- **G10** Binaries never enter git; references are `path@sha256:<hex>` relative to the binaries
  folder.
- **G11** Generators only through `docs/generators.md`; no generator-specific field in the data
  model; the UI works for any generator that produces a kind.
- **G12** Network only to ComfyUI and the configured LLM endpoint, both opt-in; no telemetry.
- **G13** UI uses `design/theos-ui/` and follows `docs/ui/`; shortcuts use Ctrl on Windows and
  Linux and Cmd on macOS, from one keymap.
- **G14** Settings in three layers (part default → user → project); secrets in the OS
  credential store.
- **G15** Every AC below has at least one test tagged with its id (`docs/gate.md` §4).
- **G16** Errors in library crates are typed (`thiserror`); `anyhow` only in binaries. Logging
  with `tracing`.
- **G17** Branches, commits, PR titles, versions and tags follow `CONTRIBUTING.md`; releases
  follow `docs/release.md`. One product version for all crates; tags only from the pipeline.
- **G18** UI references are the HTML files in `docs/ui/screens/` (built only from
  `design/theos-ui/`); the PNG files next to them are previews rendered from that HTML.

Phase ACs, verbatim:

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

Jann's rulings that bind the build (spec §4.1): scalars are typed by the field, never by their
spelling (D1); a write turns the whole file into LF, and for LF files G5 and CE-09.1 hold byte for
byte (D2); "ours" in G5 means our subset rules and span patcher on `yaml-rust2`, never a
hand-written YAML lexer (D3); the four approaches of spec §4.3 are all (a).

## Build rules (every task, every subagent)

Put this section into every implementer and reviewer brief.

- **Engineering directives** (`CLAUDE.md`): small modules with one job; a new rule or grammar is
  one module plus one registry line. Use the crate before custom code; never hand-write a YAML or
  Markdown lexer: our code reads the tokens and events `yaml-rust2` and `pulldown-cmark` give, and
  one line at a time inside blocks they found. If reviews keep finding new variants of one
  problem, stop and name the class (directive 3).
- **Lints** (workspace, `-D warnings`): no `#[allow]` or `#[warn]` attributes; `#[expect]` only
  with `reason = "…"`; every public item documented (`missing_docs`); every public `Result` fn
  has an `# Errors` section (`missing_errors_doc`); no `unwrap`, `expect`, `panic!`, `todo!`,
  `unimplemented!`, `dbg!` or prints outside tests. Public conversions return errors instead of
  slicing blindly (`str::get`); indexing with spans the crate itself built is fine. Clippy
  pedantic is on: `#[must_use]` on pure public fns, `# Errors` sections, no needless casts.
- **No dead code in tests either**: a helper or a deserialised field nobody reads fails clippy;
  add each helper or field in the task whose test first uses it.
- **Docs**: code blocks in doc comments are ```` ```text ```` unless they are doctests that pass
  (`cargo test --doc` runs in the gate); intra-doc links must resolve (`cargo doc -D warnings`).
- **Tests**: units in `#[cfg(test)] mod tests` next to the code; behaviour in the one
  integration-test crate `crates/theos-content/tests/corpus/` (`main.rs` plus one module per
  file). A test proving an AC is a plain `#[test]` fn with `// AC: CE-xx.y` on the line directly
  above `#[test]`; never a tag inside `proptest!`, never `rstest` cases (ruling P1). Tests may
  return `Result<(), Box<dyn std::error::Error>>` and use `?`.
- **Fixtures** live in `crates/theos-content/tests/fixtures/corpus/`. Every file says it is
  synthetic (`# synthetic fixture (P02), not canon` as a front-matter comment, or the P10 marker
  the copies already carry). No lore beyond the data model's examples.
- **Files with backslashes** (Rust string escapes, fixtures) are written with the Write or Edit
  tool, never with a shell heredoc (heredocs here collapse `\\`).
- **Git**: one git command per Bash call in the worktree (no `&&` chains); never `--no-verify`.
  Commit message: Conventional Commit, scope `content` (or `build` for dependency-only commits),
  body saying why, then the footers:

  ```text
  Refs: CE-09.3
  Phase: P02
  Claude-Session: b279544b-000a-4306-95eb-c0af2f46d0ae
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  ```

  Leave out `Refs:` when a task proves no AC.
- **Per-task checks** (run from the worktree root, read the output):
  `cargo fmt --all --check`,
  `cargo clippy -p theos-content --all-targets --locked -- -D warnings`,
  `cargo nextest run -p theos-content --locked`. A task that adds a dependency also runs
  `cargo deny --locked check`.

## Review Focus

The five input classes the spec implies, no AC names, and that are most likely to bite. Each has
its test in the owning task.

1. **A record whose front matter is wrong.** Its body is still parsed (P12 shows it), and every
   patch on it fails with `PatchError::FrontMatter` and returns no text. Tests: Task 9
   (`body_is_parsed_when_the_front_matter_fails`), Task 14 (`patching_a_broken_front_matter_fails`).
2. **Values the encoder must quote.** Empty text, leading or trailing spaces, `: ` or ` #`
   inside, a leading indicator (`#`, `-`, `[`, `]`, `{`, `}`, `&`, `*`, `!`, `|`, `>`, `'`, `"`,
   `%`, `@`, `` ` ``, `?`, `:`), and `,`, `[`, `]` inside a flow item: each is quoted and reads
   back the same text. Test: Task 13 (`values_that_need_quotes_are_double_quoted`).
3. **Edits next to own-line comments and blank lines.** Removing the first block item when a
   comment line follows it; inserting a field after one that is followed by comment lines;
   removing the field directly below an own-line comment. The comment lines stay where they were.
   Tests: Task 14 (`own_line_comments_stay_put`), Task 15 (`block_list_comments_stay_put`).
4. **Lists with duplicates and replacements.** `[a, a, b]→[a, b]`, `[a, b]→[x, y]` (a `Replace`
   op), and a block list with a duplicate item: kept items keep their bytes, and the result reads
   back the new list. Test: Task 15 (`duplicates_and_replacements`).
5. **The edges of the split.** A closing `---` at the end of the file without a newline, an
   empty body, a body that is only `\n`, and a `---` thematic break or setext underline in the
   body: none ends the front matter early or breaks the tiling. Tests: Task 3
   (`split_edges`), Task 9 (`blocks_tile_the_body`).

## Rulings made while planning

These go into the ledger's "Rulings (build)" and the PR, in this form.

- P1. Ruling: AC tests are plain `#[test]` fns in one integration-test crate (`tests/corpus/`)
  plus unit tests — `ac-trace` matches a JUnit name equal to the fn name or ending in `::name`,
  so `rstest` cases (`…::case_1`) would show no result; one crate keeps shared helpers used
  without `#[allow(dead_code)]` — cost if wrong: loops instead of parameterised cases.
- P2. Ruling: `similar` 2.7 passed its probe while planning (see the header) and is added in
  Task 15 — cost if wrong: none.
- P3. Ruling: `proptest` 1.12 with `default-features = false, features = ["std"]` for the
  untagged property tests the spec asks for (§5.14) — the defaults add fork and timeout support
  nothing uses — cost if wrong: one dev-dependency.
- P4. Ruling: setting a block list to no items writes `key: []` (the field's lines are rewritten,
  the key-line comment kept) — `key:` alone is an empty value, so an empty block list cannot be
  written — cost if wrong: a style change.
- P5. Ruling: removing a field or entry that is not there is `PatchError::NotFound`;
  `set_entry` on a field that is not a map is `PatchError::NotAMap` — the patcher fails closed;
  P06 may treat `NotFound` as a no-op — cost if wrong: one match arm in P06.
- P6. Ruling: a field missing from the caller's `Order` is not inserted
  (`PatchError::NotInOrder`); an entry missing from the entry order goes last — spec §5.7 says
  "or last" for entries only; P03's registry knows every field — cost if wrong: P03 passes a
  fuller order.
- P7. Ruling: `Record::patch` applies its operations one after another; each is verified on the
  text the previous one produced — no overlap rules between operations — cost if wrong: one parse
  per operation.
- P8. Ruling: `Field.comment` is the trailing comment on the key line; block items and map
  entries carry the comment of their own line; a shape change keeps the key-line comment and
  writes it after one space — the spec's "value's last line" is ambiguous for block values —
  cost if wrong: the spacing of one comment.
- P9. Ruling: a malformed link is reported in `Body::malformed()` and is not in `Body::links()` —
  P06 must not rename inside it and P03 must not count it; one fault, one finding — cost if
  wrong: P04 reads both lists.
- P10. Ruling: a closing `---` at the end of the file without a newline closes the front matter,
  with an empty body — the line is exactly `---` — cost if wrong: such a file is refused instead.
- P11. Ruling: the round trips write back values built from the model (`Link::spelling()`,
  `NoteState::spelling()`, attribute and token text), never `text[span]` — copying the slice back
  would be the hollow test of spec §4.3 (b) — cost if wrong: none.
- P12. Ruling: in the corpus, the gaps between top-level body blocks hold only whitespace; the
  tiling test asserts it, which is what makes "blocks and gaps tile the body" mean something —
  cost if wrong: a record with a link reference definition widens the assertion.
- P13. Ruling: beat headings are top-level level-2 headings, ATX or setext; headings inside
  block quotes are ordinary — data model §9.4 structures a script body by them — cost if wrong:
  none for valid records.
- P14. Ruling: a link fragment is kept as its digit text (`#03` stays `03`), with `beat()` for the
  number; a beat number or fragment that does not fit `u32` is malformed — writing back what was
  read — cost if wrong: none.
- P15. Ruling: the template ids of the corpus are the ones in Task 8 — fresh, each matching its
  kind's pattern, colliding with no corpus id — cost if wrong: renaming fixtures.
- P16. Ruling: no operation switches a list between flow and block style; a list keeps its style
  through every edit (P4 is the one forced exception) — spec §5.7 allows a switch only when the
  caller asks, and no P02 AC or §6 interface asks — cost if wrong: one more `PatchOp` in P06.
- P17. Ruling: until Tasks 15 and 16 land, a list or map written over a list or map goes through
  the shape-change rewrite — each task stays shippable; the per-item and per-entry paths replace
  it — cost if wrong: none after Task 16.

## File structure

```text
crates/theos-content/
  Cargo.toml                         deps: thiserror, tracing, yaml-rust2, pulldown-cmark, similar
  README.md                          what the crate offers after P02 (Task 17)
  src/
    lib.rs                           crate docs, `pub mod`s, root re-exports
    test_support.rs                  #[cfg(test)] helpers for unit tests
    text/
      mod.rs                         re-exports
      span.rs                        Span
      index.rs                       Position, TextIndex (byte ↔ line/column), PositionError
      utf16.rs                       TextIndex byte ↔ UTF-16 (Task 2)
      normalize.rs                   normalize (CRLF, lone CR → LF)
      edit.rs                        TextEdit, apply_edits, EditError
    record/
      mod.rs                         Record (parse, accessors, with_edits)
      split.rs                       the two `---` lines
      read.rs                        decode, read_record, ReadError
      error.rs                       RecordError, RecordEditError
    yaml/
      mod.rs                         re-exports
      error.rs                       YamlError, YamlRule
      input.rs                       tokens + events + crate error, positions converted, once
      rules/mod.rs                   Rule trait, RULES registry, first_error
      rules/<rule>.rs                one file per rule of spec §5.4
      scalar.rs                      Scalar, ScalarStyle, Lexical, Decimal, spell, spelled_end
      model.rs                       FrontMatter, Field, Value, List, Item, Map, Entry, RawValue
      encode.rs                      encode, Slot, EncodeError
      patch/mod.rs                   PatchOp, Order, PatchError, Record::patch and helpers
      patch/field.rs                 scalar set, insert, remove, shape change
      patch/list.rs                  flow and block lists per item (similar)
      patch/map.rs                   map entries
      patch/verify.rs                re-parse and compare
    body/
      mod.rs                         Body, Block, BlockKind, parse orchestration
      events.rs                      the one pulldown-cmark pass
      links.rs                       Link, wikilinks, malformed links
      malformed.rs                   Malformed, MalformedKind
      callout/mod.rs                 Callout, CalloutTitle, Token, Fold, registry, captions
      callout/title.rs               title tokens split on ` · `
      callout/voice.rs               voice title grammar
      callout/design.rs              design title
      callout/note.rs                note title, NoteState, note text
      design_note.rs                 DesignNote (`[!design]` and `Design note:` paragraphs)
      beat.rs                        Beat, Attribute, AttributeValue
  tests/
    corpus/main.rs                   the integration-test crate; declares the modules below
    corpus/common.rs                 corpus paths, file walk, expected.toml, cut()
    corpus/rejected.rs               CE-09.3 over rejected/
    corpus/records.rs                corpus checks, CE-09.4 twins
    corpus/malformed.rs              CE-10.3 over malformed/
    corpus/grammars.rs               CE-10.1, CE-10.2 over corpus records
    corpus/round_trip.rs             CE-09.1, CE-10.4
    corpus/edits.rs                  CE-09.2 matrix, property tests
    fixtures/corpus/{records,malformed,crlf,rejected}/
```

---

### Task 1: Text — spans, positions, line endings, edits

**Files:**
- Modify: `crates/theos-content/Cargo.toml` (add `thiserror.workspace = true`)
- Modify: `crates/theos-content/src/lib.rs` (crate docs: what P02 provides; `pub mod text;`)
- Create: `src/text/{mod.rs, span.rs, index.rs, normalize.rs, edit.rs}`

**Interfaces — produces:**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Span { pub start: usize, pub end: usize }      // byte range, start <= end
impl Span {
    pub const fn new(start: usize, end: usize) -> Span;
    pub const fn len(self) -> usize;
    pub const fn is_empty(self) -> bool;
    pub fn get(self, text: &str) -> Option<&str>;          // None when out of range or off a boundary
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position { pub line: usize, pub column: usize } // from 1; column in Unicode scalar values
impl Display for Position                                    // "line:column"
pub fn normalize(input: &str) -> (Cow<'_, str>, bool);     // (LF text, whether anything changed)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextIndex { /* owns the text and its line starts */ }
impl TextIndex {
    pub fn new(text: String) -> TextIndex;
    pub fn text(&self) -> &str;
    pub fn position(&self, offset: usize) -> Result<Position, PositionError>;
    pub fn offset(&self, position: Position) -> Result<usize, PositionError>;
    pub fn line_span(&self, line: usize) -> Option<Span>;  // the line with its '\n', if any
}
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum PositionError { OutOfRange { offset: usize, len: usize }, NotCharBoundary { offset: usize },
                         NoSuchLine { line: usize }, NoSuchColumn { line: usize, column: usize } }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit { pub span: Span, pub replacement: String }
pub fn apply_edits(text: &str, edits: &[TextEdit]) -> Result<String, EditError>;
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum EditError { OutOfRange { span: Span }, NotCharBoundary { span: Span },
                     Overlap { first: Span, second: Span } }
```

`TextIndex` takes LF text (it is built after `normalize`). The end of the text is a valid
offset; on a line, the column after its last character is valid. `apply_edits` sorts a copy of
the edits by `(start, end)` with a stable sort, so zero-width inserts at one offset keep their
order and come before a deletion that starts there; an edit starting before the previous one's
end is `Overlap`.

- [ ] **Step 1: Write the failing unit tests**
  - `position_counts_unicode_scalar_values`: in `"title: Mère Ilse ö 😀 x\nkind: npc"` the `x`
    (byte 26) is `1:22` and `kind` (byte 28) is `2:1`; `offset(2:1) == 28`.
  - `tab_is_one_column`: in `"a\tb"`, byte 2 is `1:3`.
  - `offsets_and_positions_round_trip`: for every char boundary of
    `"è\u{00A0}e\u{0301}😀\n\tx\n"` and of a text without a final newline,
    `offset(position(o)) == o`; the end of the text is accepted.
  - `off_boundary_and_out_of_range_are_errors`: byte 9 of the first text (inside `è`) is
    `NotCharBoundary`; `len + 1` is `OutOfRange`; line 9 is `NoSuchLine`; `1:99` is
    `NoSuchColumn`.
  - `crlf_and_lone_cr_become_lf` with `// AC: CE-09.4`: `normalize("a\r\nb\rc\n")` is
    `("a\nb\nc\n", true)`; `normalize("a\nb")` is borrowed and `false`; `"\r\r\n"` becomes
    `"\n\n"`.
  - `edits_apply_in_span_order`: replace, insert and delete given out of order give the expected
    text; an insert at 3 and a delete of `3..5` both apply; `[]` returns the text unchanged;
    overlapping spans give `Overlap`; a span inside `è` gives `NotCharBoundary`; past the end
    gives `OutOfRange`.
- [ ] **Step 2: Run** `cargo nextest run -p theos-content --locked` — expect compile errors.
- [ ] **Step 3: Implement** the items above. `TextIndex` keeps the line-start byte offsets;
  columns are counted with `chars()` over the line prefix.
- [ ] **Step 4: Run** the per-task checks — all pass.
- [ ] **Step 5: Commit** `feat(content): add spans, positions and text edits` (Refs: CE-09.4).

### Task 2: UTF-16 offsets in `TextIndex` (decision 0002)

**Files:**
- Create: `src/text/utf16.rs` (an `impl TextIndex` block)
- Modify: `src/text/index.rs` (keep the UTF-16 start of each line), `src/text/mod.rs`

**Interfaces — produces:**

```rust
impl TextIndex {
    pub fn utf16_offset(&self, offset: usize) -> Result<usize, PositionError>;
    pub fn offset_from_utf16(&self, utf16: usize) -> Result<usize, PositionError>;
}
// PositionError gains: InsideSurrogatePair { utf16: usize }
```

Both run per line: find the line by its start (binary search over the stored starts), then walk
that line's chars. 0002 stores anchors in byte coordinates and converts with these at the UI.

- [ ] **Step 1: Write the failing unit tests**
  - `utf16_counts_surrogate_pairs`: in `"a😀b"` bytes `0, 1, 5, 6` are UTF-16 `0, 1, 3, 4`;
    `offset_from_utf16(2)` is `InsideSurrogatePair { utf16: 2 }`.
  - `utf16_round_trips`: for every char boundary of the P10 characters line
    (`"— · “d” ‘s’ café cafe\u{0301} 5\u{00A0}km 😀\n"`) and a second line,
    `offset_from_utf16(utf16_offset(o)) == o`; the end of the text works.
  - `utf16_errors`: byte inside `é` → `NotCharBoundary`; past the end → `OutOfRange` (both
    directions).
- [ ] **Step 2: Run** the tests — expect failures.
- [ ] **Step 3: Implement** the two methods.
- [ ] **Step 4: Run** the per-task checks.
- [ ] **Step 5: Commit** `feat(content): convert byte offsets to utf-16 and back` (no `Refs:`;
  body names decision 0002).

### Task 3: Records — split, read, normalise

**Files:**
- Create: `src/record/{mod.rs, split.rs, read.rs, error.rs}`, `src/test_support.rs`
- Modify: `src/lib.rs` (`pub mod record;`, `#[cfg(test)] mod test_support;`, root re-exports
  `Record, RecordError, ReadError, read_record, decode, Span, Position, TextIndex, TextEdit,
  apply_edits`), `Cargo.toml` (dev: `tempfile.workspace = true`)

**Interfaces — produces:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record { /* index: TextIndex, was_normalized, front_span, body_span; later tasks add front, body */ }
impl Record {
    pub fn parse(input: &str) -> Result<Record, RecordError>;
    pub fn text(&self) -> &str;
    pub fn index(&self) -> &TextIndex;
    pub fn was_normalized(&self) -> bool;
    pub fn front_span(&self) -> Span;   // the lines between the two `---` lines (may be empty)
    pub fn body_span(&self) -> Span;    // everything after the closing line (may be empty)
    pub fn with_edits(&self, edits: &[TextEdit]) -> Result<Record, RecordEditError>;
}
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum RecordError {
    #[error("1:1: the record starts with a byte order mark")] ByteOrderMark,
    #[error("1:1: a record starts with a `---` line")] NoOpeningLine,
    #[error("1:1: front matter has no closing `---` line")] NoClosingLine,
}
impl RecordError { pub fn rule(&self) -> &'static str /* "bom", "opening-line", "closing-line" */;
                   pub fn position(&self) -> Position /* 1:1 */ }
pub fn decode(bytes: &[u8]) -> Result<&str, ReadError>;
pub fn read_record(path: &Path) -> Result<Record, ReadError>;
#[derive(Debug, thiserror::Error)]
pub enum ReadError { Io { path: PathBuf, source: std::io::Error },
                     Utf8 { position: Position },       // rule "utf8": first bad byte
                     Record(#[from] RecordError) }
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum RecordEditError { Edit(#[from] EditError), Record(#[from] RecordError) }
// test_support (cfg(test)): pub(crate) fn record(front: &str, body: &str) -> Record
//   parses format!("---\n{front}---\n{body}") and panics on error (tests only)
```

`parse`: a leading U+FEFF is `ByteOrderMark` (spec §9: no second G5 exception); then
`normalize`; line 1 must be exactly `---`; the front matter ends at the next line that is exactly
`---`, followed by `\n` or the end of the text (ruling P10). `decode` reports the position of
the first invalid byte in the coordinates of the normalised valid prefix. `with_edits` applies
the edits to `text()` and parses the result, so its output is LF (CE-09.4).

- [ ] **Step 1: Write the failing unit tests** (`record/mod.rs`, `read.rs`)
  - `splits_front_matter_and_body`: `"---\nid: a\n---\nbody\n"` → front `4..10`, body `14..19`.
  - `split_edges`: `"---\n---\n"` (empty front `4..4`, empty body); `"---\nid: a\n---"` (body
    `13..13`); body `"\n"` only; body without a final newline; a body holding `---` and a setext
    underline (`"x\n---\n"`) keeps the first closing line as the delimiter.
  - `delimiters_are_exact`: `"--- \nid: a\n---\n"` → `NoOpeningLine`; `"---\nid: a\n--- \n"` →
    `NoClosingLine`; `""` → `NoOpeningLine`; each reports `1:1` and its rule string.
  - `byte_order_mark_is_refused`: `"\u{FEFF}---\n---\n"` → `ByteOrderMark` at `1:1`.
  - `crlf_record_reads_like_its_lf_twin` with `// AC: CE-09.4`:
    `"---\r\nid: a\r\n---\r\nb\r\n"` parses to the same `text()`, `front_span()` and
    `body_span()` as `"---\nid: a\n---\nb\n"`, and `was_normalized()` is true (false for the twin).
  - `edits_return_a_new_lf_record` with `// AC: CE-09.4`: `with_edits` on the CRLF record above
    replacing `a` with `b` gives `"---\nid: b\n---\nb\n"` and `was_normalized() == false`.
  - `invalid_utf8_reports_its_position`: `decode(b"---\nid: \xff\n")` → `Utf8 { 2:5 }`;
    `decode(b"---\r\nid: \xff")` → `Utf8 { 2:5 }` too.
  - `read_record_reads_a_file`: a temp file (tempfile) parses; a missing path is `Io`.
- [ ] **Step 2: Run** the tests — expect failures.
- [ ] **Step 3: Implement** `split.rs` (returns the two spans or a `RecordError`), `read.rs`,
  `Record`.
- [ ] **Step 4: Run** the per-task checks.
- [ ] **Step 5: Commit** `feat(content): split records into front matter and body` (Refs: CE-09.4).

### Task 4: YAML input, the rule registry, and the indicator rules

**Files:**
- Modify: `Cargo.toml` (`yaml-rust2.workspace = true`; dev: `serde.workspace = true`,
  `toml.workspace = true`), `src/lib.rs` (`pub mod yaml;`), `src/record/mod.rs`
- Create: `src/yaml/{mod.rs, error.rs, input.rs}`, `src/yaml/rules/{mod.rs, anchor.rs,
  alias.rs, tag.rs, flow_map.rs, complex_key.rs, document_marker.rs}`
- Create: `tests/corpus/{main.rs, common.rs, rejected.rs}`,
  `tests/fixtures/corpus/rejected/expected.toml` and the rejected files listed below

**Interfaces — produces:**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum YamlRule { Anchor, Alias, Tag, BlockScalar, OneLineScalar, Escape, FlowMap, ListOfMaps,
    NestedMap, ListInList, OneLineList, TopLevelMap, TabIndent, DuplicateKey, Key, ComplexKey,
    DocumentMarker, EmptyValue, Syntax /* the crate's own error */ }
impl YamlRule { pub fn as_str(self) -> &'static str }   // kebab names of spec §5.4; "syntax"
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{position}: {message}")]
pub struct YamlError { pub rule: YamlRule, pub message: String, pub span: Span, pub position: Position }

pub(crate) struct Marked<T> { pub value: T, pub offset: usize, pub position: Position }
pub(crate) struct Input<'a> {
    pub index: &'a TextIndex, pub span: Span,            // the front matter in record coordinates
    pub tokens: Vec<Marked<yaml_rust2::scanner::TokenType>>,
    pub events: Vec<Marked<yaml_rust2::Event>>,          // as delivered to MarkedEventReceiver
    pub crate_error: Option<YamlError>,                  // rule Syntax
}
impl<'a> Input<'a> { pub fn new(index: &'a TextIndex, span: Span) -> Input<'a> }
pub(crate) trait Rule { fn check(&self, input: &Input<'_>) -> Option<YamlError>; } // its first violation
pub(crate) const RULES: &[&dyn Rule];                    // one line per rule
pub(crate) fn first_error(input: &Input<'_>) -> Option<YamlError>;
impl Record { pub fn front_matter_error(&self) -> Option<&YamlError> }  // stays; Task 7 adds front_matter()
```

`Input::new` runs both on `&index.text()[span]`, once: `Scanner::new(text.chars())` with
`next_token()` until `None` or an error, and `Parser::new_from_str(text).load(&mut collector,
false)`. A `Marker` converts as: line = line of `span.start` + `marker.line() - 1`, column =
`marker.col() + 1`, offset through `index.offset`; never `marker.index()` (spec §4.2: it counts
characters). A marker that does not convert clamps to `span.end`. `first_error` takes every
rule's first violation and the crate error, in that order, and returns the one with the smallest
offset (`min_by_key` keeps the first of equals, so a rule beats the crate at the same place).
`Record::parse` stores `first_error` (until Task 7 replaces it with the model).

Rule positions (each rule's module doc names its position):
- `anchor`, `alias`, `tag`, `flow-map`: the `Anchor`, `Alias`, `Tag`, `FlowMappingStart` token.
- `complex-key`: a `Key` token whose position differs from the next scalar's (the `?`).
- `document-marker`: `DocumentEnd`, an explicit `DocumentStart`, or a directive token.

Unit tests use `test_support::record(front, "")`; the front matter's first line is record line 2.

- [ ] **Step 1: Write the failing unit tests** (in the rule modules and `input.rs`)
  - `yaml_rust2_markers_count_characters` (pin, spec §7): parsing
    `"title: Mère Ilse ö 😀 x\nkind: npc"` with `yaml-rust2` directly gives the `kind` key a
    marker with `index() == 23`, `line() == 2`, `col() == 0`; through `Input`, in a record, its
    offset is the byte offset of `kind`.
  - `anchors_are_rejected` `// AC: CE-09.3`: `"a: &x 1\n"` → (`anchor`, 2:4).
  - `aliases_are_rejected` `// AC: CE-09.3`: `"a: 1\nb: *x\n"` → (`alias`, 3:4).
  - `tags_are_rejected` `// AC: CE-09.3`: `"c: !t v\n"` → (`tag`, 2:4).
  - `flow_maps_are_rejected` `// AC: CE-09.3`: `"f: {a: 1}\n"` → (`flow-map`, 2:4);
    `"id: {{title}}\n"` → (`flow-map`, 2:5).
  - `complex_keys_are_rejected`: `"? a\n: b\n"` → (`complex-key`, 2:1).
  - `document_markers_are_rejected`: `"a: 1\n...\n"` → (`document-marker`, 3:1);
    `"%YAML 1.2\na: 1\n"` → (`document-marker`, 2:1).
  - `columns_count_unicode_scalar_values` `// AC: CE-09.3`: `"tags: [Mère, 😀, *x]\n"` →
    (`alias`, 2:17).
  - `the_first_error_in_document_order_wins`: `"a: *x\nb: &y 1\n"` → `alias` at 2:4;
    `"a: &x 1\nb: @x\n"` → `anchor`, not `syntax`.
  - `crate_errors_keep_the_crate_position`: `"a: @x\n"` → rule `syntax` on line 2 at the
    column the crate reports, converted (pin the observed column).
- [ ] **Step 2: Write the rejected corpus and its harness**
  - Files in `rejected/`, each `---`, `# synthetic fixture (P02), not canon`,
    `id: rejected-<name>`, then the offending line (so it sits on line 4), then `---`:
    `anchor.md` (`a: &x 1`), `alias.md` (`b: *x`), `tag.md` (`c: !t v`), `flow-map.md`
    (`f: {a: 1}`), `flow-map-template.md` (`title: {{title}}`), `complex-key.md` (`? a` /
    `: b`), `document-end.md` (`...`), `directive.md` (`%YAML 1.2` on line 2 instead of the
    comment, comment after it), `alias-after-unicode.md` (`tags: [Mère, 😀, *x]`),
    `syntax-reserved.md` (`a: @x`), `syntax-backtick.md` (``a: `x``), `syntax-nested-value.md`
    (`a: b: c`), `syntax-indent.md` (a map whose second key is indented one space more).
    Also `bom.md` (a BOM before `---`), `no-opening-line.md` (first line is the marker
    comment), `no-closing-line.md` (no closing `---`).
  - `expected.toml`: one `[[case]]` per file with `file`, `rule` (`YamlRule::as_str` or
    `RecordError::rule`), `line`, `column`, worked out by hand from the file. For `syntax-*`
    files, the column the crate reports (observed once, pinned).
  - `tests/corpus/main.rs`: crate docs, `mod common; mod rejected;`.
  - `common.rs`: `pub fn corpus() -> PathBuf` (`CARGO_MANIFEST_DIR/tests/fixtures/corpus`),
    `pub fn md_files(dir: &Path) -> Vec<PathBuf>` (recursive, sorted, std only),
    `pub fn rel(path: &Path) -> String` (relative to `corpus()`, `/` separators),
    `#[derive(Deserialize)] pub struct Case { file, rule: Option<String>, line, column }`
    (Task 9 adds `kind: Option<String>`), `pub fn cases(path: &Path) -> Vec<Case>` (reads
    `[[case]]` arrays).
  - `rejected.rs`: `rejected_files_fail_with_rule_line_and_column` `// AC: CE-09.3` — every
    `.md` in `rejected/` has exactly one case and every case's file exists; each file is read as
    bytes, `decode`d, parsed; the `RecordError` or `front_matter_error()` gives
    `(rule, line, column)` equal to the case. `invalid_utf8_is_rejected_with_its_position` builds
    `b"---\n# synthetic fixture (P02), not canon\nid: \xff\n---\n"` → `utf8` at 3:5.
- [ ] **Step 3: Run** the tests — expect failures.
- [ ] **Step 4: Implement** `error.rs`, `input.rs`, `rules/mod.rs` and the six rule modules.
- [ ] **Step 5: Run** the per-task checks and `cargo deny --locked check`.
- [ ] **Step 6: Commit** `feat(content): reject yaml anchors, aliases, tags and flow maps`
  (Refs: CE-09.3).

### Task 5: Shape and line rules

**Files:**
- Create: `src/yaml/rules/{block_scalar.rs, list_of_maps.rs, nested_map.rs, list_in_list.rs,
  one_line_list.rs, top_level_map.rs, tab_indent.rs, duplicate_key.rs, key.rs, empty_value.rs}`
- Modify: `src/yaml/rules/mod.rs` (one registry line each), `rejected/expected.toml`
- Create: the rejected files below

Positions:
- `block-scalar`: the scalar event of style `Literal` or `Folded` (the `|` or `>`).
- `list-of-maps`, `nested-map`, `list-in-list`: the inner `MappingStart` or `SequenceStart`
  event that breaks the allowed shape (top map → value: scalar, list of scalars, or a map whose
  values are scalars or flow lists).
- `one-line-list`: a `FlowSequenceStart` whose matching `FlowSequenceEnd` is on another line.
- `top-level-map`: the first node when it is not a mapping.
- `tab-indent`: the first tab before the first non-blank character of a front-matter line
  (lines that hold only whitespace are skipped).
- `duplicate-key`: the second occurrence, per map.
- `key`: a key that is not plain or does not match `^[a-z][a-z0-9_]*$` (fields and entries).
- `empty-value`: an empty plain scalar; the position is one character after the last `Value`
  (`:`) or `BlockEntry` (`-`) token before it.

- [ ] **Step 1: Write the failing unit tests**
  - `block_scalars_are_rejected` `// AC: CE-09.3`: `"d: |\n  x\n"` and `"e: >\n  x\n"` →
    (`block-scalar`, 2:4).
  - `deeper_nesting_is_rejected` `// AC: CE-09.3`: `"g:\n  - a: 1\n"` → (`list-of-maps`, 3:5);
    `"h:\n  i:\n    j: 1\n"` → (`nested-map`, 4:5); `"l:\n  - - a\n"` → (`list-in-list`,
    3:5); `"l: [a, [b]]\n"` → (`list-in-list`, 2:8).
  - `lists_stay_on_one_line`: `"w: [a,\n  b]\n"` and `"w: [a, # c\n  b]\n"` →
    (`one-line-list`, 2:4).
  - `the_front_matter_is_a_map`: `"just text\n"` and `"- a\n"` → (`top-level-map`, 2:1).
  - `tabs_do_not_indent`: `"a: 1\n\tb: 2\n"` → (`tab-indent`, 3:1); `"l:\n\t- a\n"` →
    (`tab-indent`, 3:1), before the crate's own tab error; `"a: x\ty\n"` is accepted.
  - `keys_are_unique_per_map`: `"l: 1\nl: 2\n"` → (`duplicate-key`, 3:1);
    `"m:\n  a: 1\n  a: 2\n"` → (`duplicate-key`, 4:3); `"a: 1\nm:\n  a: 2\n"` is accepted.
  - `keys_follow_the_key_rule`: `"Key: 1\n"`, `"k-2: 3\n"`, `"\"a\": 1\n"` → (`key`, 2:1);
    `"m:\n  B: 1\n"` → (`key`, 3:3).
  - `empty_values_are_rejected`: `"o:\n"` → (`empty-value`, 2:3); `"o:\np: 1\n"` →
    (`empty-value`, 2:3); `"l:\n  - \n"` → (`empty-value`, 3:4); `"e: \"\"\n"` is accepted.
  - `allowed_forms_pass`: one front matter with block items at indent 0 and 4, `[]`, a trailing
    comma, a one-level map with a flow-list value, comments on own lines and after values, and
    blank lines has no error.
- [ ] **Step 2: Add the rejected files** (offending construct from line 4, as in Task 4):
  `block-scalar-literal.md`, `block-scalar-folded.md`, `list-of-maps.md`, `nested-map.md`,
  `list-in-list-block.md`, `list-in-list-flow.md`, `one-line-list.md`,
  `one-line-list-comment.md`, `top-level-scalar.md`, `top-level-list.md` (these two have no `id`
  line; the construct is on line 3), `tab-indent-key.md`, `tab-indent-block.md`,
  `duplicate-key.md`, `duplicate-entry-key.md`, `key-upper-case.md`, `key-hyphen.md`,
  `key-quoted.md`, `entry-key-upper-case.md`, `empty-value.md`, `empty-item.md`; their rows in
  `expected.toml`.
- [ ] **Step 3: Run** the tests — expect failures.
- [ ] **Step 4: Implement** the ten rule modules and their registry lines.
- [ ] **Step 5: Run** the per-task checks.
- [ ] **Step 6: Commit** `feat(content): reject nesting, block scalars and other yaml forms`
  (Refs: CE-09.3).

### Task 6: Scalars and rule `one-line-scalar`

**Files:**
- Create: `src/yaml/scalar.rs`, `src/yaml/rules/one_line_scalar.rs`
- Modify: `src/yaml/rules/mod.rs`, `src/yaml/mod.rs`, `rejected/expected.toml`
- Create: rejected files `one-line-scalar-plain.md`, `one-line-scalar-single.md`,
  `one-line-scalar-double.md`, `escape-newline.md`, `escape-tab.md`

**Interfaces — produces:**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScalarStyle { Plain, SingleQuoted, DoubleQuoted }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scalar { pub text: String, pub style: ScalarStyle, pub span: Span, pub position: Position }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lexical { Integer, Decimal, Boolean, Text }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decimal { /* its text */ }
impl Decimal { pub fn as_str(&self) -> &str; pub fn to_f64(&self) -> f64 }
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ScalarError { NotPlain { position: Position }, NotInteger { text: String, position: Position },
    IntegerRange { text: String, position: Position }, NotDecimal { text: String, position: Position },
    NotBoolean { text: String, position: Position } }
impl Scalar {
    pub fn as_text(&self) -> &str;
    pub fn lexical(&self) -> Lexical;            // Text for any quoted scalar
    pub fn as_int(&self) -> Result<i64, ScalarError>;
    pub fn as_decimal(&self) -> Result<Decimal, ScalarError>;
    pub fn as_bool(&self) -> Result<bool, ScalarError>;
}
pub(crate) fn spell(style: ScalarStyle, text: &str) -> String;
pub(crate) fn spelled_end(source: &str, offset: usize, style: ScalarStyle, text: &str) -> Result<usize, usize>;
//   Ok(end) when `spell(style, text)` is the source at `offset`; Err(first differing byte offset)
```

`spell` (spec §5.4): plain = the text; single = `'` + text with `'` doubled + `'`; double = `"` +
text with `\` and `"` escaped + `"`. The lexicon (spec §5.5, P01): integer `-?[0-9]+` (`007` is
7; outside `i64` is `IntegerRange`), decimal `-?[0-9]+\.[0-9]+`, boolean `true`/`false`; matched
with `char` checks, no regex. Rule `one-line-scalar` runs `spelled_end` on every scalar event
(keys and values): on a mismatch it reports `escape` when the scalar is double-quoted and the
source byte at the first difference is `\`, else `one-line-scalar`, both at the scalar's start.

- [ ] **Step 1: Write the failing unit tests**
  - `multi_line_scalars_are_rejected` `// AC: CE-09.3`: `"m: a\n  b\n"`, `"q: 'a\n  b'\n"`,
    `"q: \"a\n  b\"\n"` → (`one-line-scalar`, 2:4).
  - `only_two_escapes_are_allowed`: `r#"a: "x\ny""#` and `r#"a: "\t""#` (each plus `"\n"`) →
    (`escape`, 2:4).
  - `spelled_scalars_end_where_their_spelling_ends`: `r#"a: "a \"b\" c""#` → text `a "b" c`,
    span covers both quotes; `"a: 'It''s'\n"` → `It's`; `"a: b#c\n"` → `b#c`;
    `"u: https://example.com/p#x\n"` → the whole URL; `"r: maps/a.png@sha256:00ff\n"` → the
    whole ref; `"t: draft   \n"` → span of 5 bytes; `"e: \"\"\n"` → empty, double-quoted;
    `"x: a\ty\n"` → `a\ty`.
  - `scalars_are_typed_by_the_field` (D1): plain `42` → `as_text() == "42"`, `as_int() == 42`,
    `lexical() == Integer`; `007` → 7; `-3`; `2.99` → `Decimal`, `to_f64() == 2.99`; `true`
    → `as_bool() == true`; `false` → `as_text() == "false"`; `1e5`, `~`, `null`, `yes`, `no`,
    `0x1F`, `+1`, `.5`, `E7`, `T-012` → `Text`, and `as_int()` is `NotInteger`; quoted `'42'` →
    `Text` and `as_int()` is `NotPlain`; `99999999999999999999` → `IntegerRange`.
- [ ] **Step 2: Add the five rejected files** and their rows.
- [ ] **Step 3: Run** the tests — expect failures.
- [ ] **Step 4: Implement** `scalar.rs` and the rule.
- [ ] **Step 5: Run** the per-task checks.
- [ ] **Step 6: Commit** `feat(content): read scalars and reject multi-line values` (Refs: CE-09.3).

### Task 7: The front-matter model

**Files:**
- Create: `src/yaml/model.rs`
- Modify: `src/yaml/mod.rs`, `src/record/mod.rs` (store `Result<FrontMatter, YamlError>`)

**Interfaces — produces:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontMatter { pub span: Span, pub fields: Vec<Field> }
impl FrontMatter {
    pub fn parse(index: &TextIndex, span: Span) -> Result<FrontMatter, YamlError>;
    pub fn get(&self, key: &str) -> Option<&Field>;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field { pub key: String, pub key_span: Span, pub value: Value, pub lines: Span,
                   pub comment: Option<Span> }       // the key line's trailing comment (P8)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value { Scalar(Scalar), List(List), Map(Map) }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List { pub style: ListStyle, pub items: Vec<Item>, pub span: Span }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListStyle { Flow, Block }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item { pub scalar: Scalar, pub lines: Option<Span> /* block items */, pub comment: Option<Span> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Map { pub entries: Vec<Entry> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry { pub key: String, pub key_span: Span, pub value: EntryValue, pub lines: Span,
                   pub comment: Option<Span> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryValue { Scalar(Scalar), List(List) }   // the list is always Flow
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawValue { Text(String), List(Vec<String>), Map(Vec<(String, RawEntry)>) }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawEntry { Text(String), List(Vec<String>) }
impl Value { pub fn to_raw(&self) -> RawValue }      // spans dropped: D1's raw texts
impl EntryValue { pub fn to_raw(&self) -> RawEntry }
impl Record { pub fn front_matter(&self) -> Result<&FrontMatter, &YamlError> }
```

The builder walks `Input::events` after `first_error` returned `None`; an event order it does not
expect returns a `YamlError` with rule `Syntax` (never a panic). Spans:
- a scalar's span ends at `spelled_end`; a flow list runs from `[` to after `]` (the
  `SequenceEnd` marker); a block list from its first item's line start to its last item's line
  end;
- `lines` of a field or entry: the start of the key's line to the end of its last value line,
  `\n` included; a block item's `lines` is its own line;
- a comment is the `#…` after the value (or after `key:` for block values) on the same line,
  without the newline: skip spaces and tabs, then expect `#` (the crate already accepted the
  line, so this only reads what is left).

Own-line comments and blank lines belong to no field, except those between the items of one block
list, which fall inside that field's `lines`.

- [ ] **Step 1: Write the failing unit tests**
  - `fields_keep_their_spans_and_lines`: the data model's `mother-ilse` front matter → keys `id,
    kind, title, parent, home, role, lens` in order; `role` is plain
    `keeper of the Fates' shrine`; `lens` is a map with one entry `speaks_to` whose text is
    `children and newcomers; calls the player "stair-child"`; `lens.lines` slices to
    `"lens:\n  speaks_to: …\n"`; `text[key_span] == key` for every field.
  - `comments_and_quotes_of_b901`: the P10 `B-901` front matter → the own-line comment is in no
    field's `lines`; `status.comment` slices to `# a comment after a value`; `held_by` is a block
    list `[throat-kin, hearth-folk]`, each item's `lines` its own line; `accuracy` is
    single-quoted `partial` with span `'partial'`; `topic` follows the blank line.
  - `flow_lists_and_odd_spacing`: `"w: ['a', \"b\",]\n"` → flow, 2 items, span `['a', "b",]`;
    `"w: []\n"` → empty flow list; `"w: [ a ]  # c\n"` → comment `# c`.
  - `values_on_the_next_line_and_key_comments`: `"title:\n  The Choir-Hall\n"` → `lines` is both
    lines; `"held_by:   # who\n  - a\n"` → `comment` slices to `# who`; block items at indent
    0; `"l:\n  - a\n  # note\n  - b\n"` → 2 items, field `lines` covers all three lines.
  - `empty_front_matter_has_no_fields`: `""` and `"# only a comment\n"` → no fields.
  - `raw_values_drop_spans`: `held_by.value.to_raw() == RawValue::List(vec!["throat-kin".into(),
    "hearth-folk".into()])`; `lens` → `RawValue::Map(vec![("speaks_to".into(),
    RawEntry::Text(…))])`.
  - `record_exposes_the_front_matter`: a record with a valid front matter gives `Ok`; one with
    `a: &x 1` gives `Err` with rule `anchor` and `front_matter_error()` returns the same.
- [ ] **Step 2: Run** the tests — expect failures.
- [ ] **Step 3: Implement** `model.rs`; `Record::parse` calls `FrontMatter::parse`.
- [ ] **Step 4: Run** the per-task checks (the rejected harness keeps passing).
- [ ] **Step 5: Commit** `feat(content): build the front-matter model with spans`.

### Task 8: The corpus — records and CRLF twins

**Files:**
- Create: `tests/fixtures/corpus/records/**`, `tests/fixtures/corpus/crlf/{.gitattributes,
  twins.toml, *.md}`, `tests/corpus/records.rs`
- Modify: `tests/corpus/main.rs`, `tests/corpus/common.rs`
  (`pub fn project_records() -> Vec<PathBuf>`: `records/` and `malformed/` together)

Every record is LF and valid for P02 (spec §5.13): front matter parses, body has no malformed
construct (asserted from Task 9 on). Ids are unique across `records/` and `malformed/`; file name
= id; paths follow data model §2 and §6.

**`records/` contents**
- **Data model §14 examples**, one file each, at their paths, each with
  `# synthetic fixture (P02), not canon` as its first front-matter line:
  `world/hearth/f04-choir-hall.md`, `people/hearth-folk/fates-cult/mother-ilse.md`,
  `lore/clues/C-031.md`, `story/scripts/S-004.md`, `maps/f04-terrain.md`,
  `maps/f04-terrain/f04-terrain-v1.md`, `concepts/fates-shrine/fates-shrine-v5.md`. The `…`
  hashes become 63 zeros and one digit: `…01`, `…02`, `…03` for the three outputs of
  `f04-terrain-v1`, `…04` for `fates-shrine-v5`.
- **The 20 P01 templates** from `../tower-of-theos/templates/records/` (read only), with
  `{{title}}` replaced by the id below and the marker comment added after `---` (P15):

  | Template | Id | Path |
  |---|---|---|
  | band | `synthetic-band` | `world/synthetic-band.md` |
  | belief | `B-902` | `lore/beliefs/B-902.md` |
  | clue | `C-902` | `lore/clues/C-902.md` |
  | concept | `synthetic-concept` | `concepts/synthetic-concept.md` |
  | concept-version | `synthetic-concept-v1` | `concepts/synthetic-concept/synthetic-concept-v1.md` |
  | dialogue | `synthetic-dialogue` | `dialogue/synthetic-dialogue.md` |
  | equation | `E901` | `canon/formalism/E901.md` |
  | faction | `synthetic-faction` | `people/synthetic-people/synthetic-faction.md` |
  | floor | `f99-synthetic-floor` | `world/synthetic-band/f99-synthetic-floor.md` |
  | location | `synthetic-location` | `world/synthetic-band/f99-synthetic-floor/synthetic-location.md` |
  | map | `synthetic-map` | `maps/synthetic-map.md` |
  | map-version | `synthetic-map-v1` | `maps/synthetic-map/synthetic-map-v1.md` |
  | npc | `synthetic-npc` | `people/synthetic-people/synthetic-faction/synthetic-npc.md` |
  | people | `synthetic-people` | `people/synthetic-people.md` |
  | script | `S-902` | `story/scripts/S-902.md` |
  | template | `synthetic-template` | `templates/synthetic-template.md` |
  | thing | `synthetic-thing` | `things/synthetic-thing.md` |
  | topic | `synthetic-topic-explaining` | `lore/topics/synthetic-topic-explaining.md` |
  | tower | `tower` | `world/tower.md` |
  | truth | `T-902` | `lore/truths/T-902.md` |

- **P10 copies** from `research/P10-prose-editor/corpus/records/` (copied, never imported), at
  the same relative paths: `lore/beliefs/B-901.md`, `lore/truths/T-901.md`,
  `lore/topics/synthetic-links-explaining.md`, `lore/topics/synthetic-voices-in-world.md`,
  `maps/synthetic-notes.md`, `things/synthetic-characters.md`,
  `things/synthetic-no-final-newline.md`, `things/synthetic-whitespace.md`. Reworded neutrally
  (spec §9): in `B-901`, `[[T-012|Listeners are the unwoven Tuners]]` becomes
  `[[T-012|the truth this belief distorts]]`; in `synthetic-voices-in-world`, both
  "the Cutter is near, so we light" become "we light"; in `synthetic-links-explaining`, "The
  binders call them ancestors, but the operator-plate names a Listener." becomes "A synthetic
  sentence without links sits between two paragraphs with links." `S-901` waits for Task 12
  (`malformed/`); `synthetic-crlf` goes to `crlf/`.
- **New synthetic records** (wording is the builder's, within the rules above):
  - `things/synthetic-yaml-comments.md`: own-line comments before the first field, between
    fields and after the last; comments after scalars, after a flow list and after `key:` with a
    block list below; comments and a blank line between block items; block items at indent 0
    and at indent 4; a one-level map with comments between and after its entries.
  - `things/synthetic-yaml-quoting.md`: plain, single-quoted with `''`, double-quoted with
    `\"` and `\\`, `""`, `''`; texts that look typed (`42`, `007`, `-3`, `2.99`, `1e5`, `true`,
    `null`, `~`, `yes`) as plain values; `b#c`; a URL with `#`; a binary ref
    `x/y.png@sha256:<64 hex>`; trailing spaces after a value; flow lists `[a,b]`, `[ a ]`,
    `['a', "b"]`, `[a, b,]`, `[]`.
  - `things/synthetic-yaml-unicode.md`: `è`, `ö`, U+00A0, a combining mark, an emoji, `·` and
    `—` in the title; a tab inside a value; a value on the line after its key.
  - `things/synthetic-no-body.md`: ends with `---` and no newline (P10).
  - `lore/topics/synthetic-callouts-in-world.md`: voice blocks `[!VOICE]`, `[!voice]-`,
    `[!Voice]+`; speaker only, speaker and beliefs, speaker, beliefs and tier; a voice block
    nested in a plain block quote; lazy continuation; empty `>` lines; a caption; a `-- ` line
    that is not a caption; `[!design] A design title with [[f04-choir-hall]]`; a bare
    `[!design]`; a `Design note:` paragraph inside a voice block and one hard-wrapped at the top
    level; an unknown type `[!tip] A tip`; links in a list, a block quote, a code span, fenced
    code and an HTML comment; `[[S-004#3]]`, `[[Mother-Ilse]]`, `[[mother-ilse|Ilse]]`.
  - `story/scripts/S-903.md`: beats with extra spaces after `::`, `with:: [[a]], [[b]]`, an
    unknown key, a beat without attributes, prose directly after the attributes in the same
    paragraph, a voice block directly after the attributes, a setext beat heading, a `## Beats`
    heading, `## Beat x — y` (an ordinary heading), a `---` thematic break after a blank line,
    and a `[[S-903#1]]` link.
  - `concepts/synthetic-concept-notes.md`: `## Notes` with `n1 · open`,
    `[!note]- n2 · [[synthetic-concept-v1]] · dropped`,
    `n3 · resolved by [[synthetic-concept-v1]]`, `n4 · closed` (an unknown state: data, not an
    error), a note of two paragraphs with a caption, and a note whose text holds a link.

**`crlf/` contents** — `.gitattributes` with the single line `* -text`, added and committed
before any CRLF file (the root has `* text=auto eol=lf`). CRLF twins are made from the LF files
with `sed -e 's/$/\r/' <lf> > <crlf>`; mixed and lone-CR files with `printf`. `twins.toml` maps
each input to its LF twin:
- `S-004.md` → `../records/story/scripts/S-004.md`; `B-901.md` →
  `../records/lore/beliefs/B-901.md`; `f04-terrain.md` → `../records/maps/f04-terrain.md`;
- `synthetic-crlf.md` (the P10 file, byte for byte) → `synthetic-crlf.lf.md` (its LF twin);
- `mixed.md` (CRLF, LF and one lone CR) → `mixed.lf.md`; `lone-cr.md` (only CR) →
  `lone-cr.lf.md`.

Check after `git add`: `git ls-files --eol crates/theos-content/tests/fixtures/corpus/crlf` shows
`i/crlf` (or `i/mixed`) for every CRLF input.

- [ ] **Step 1: Write the failing tests** in `records.rs`
  - `corpus_records_parse`: every file of `project_records()` parses and has an `Ok` front matter.
  - `corpus_ids_are_unique_and_match_file_names`: the `id` field's text equals the file stem, and
    no id appears twice.
  - `every_fixture_says_it_is_synthetic`: every `.md` under `corpus()` contains
    `synthetic fixture`.
  - `crlf_inputs_read_like_their_lf_twins` `// AC: CE-09.4`: for each twin pair, the input bytes
    contain `\r\n` (`lone-cr.md`: `\r` and no `\n`); the parsed record's `text()` equals the
    twin's text, `was_normalized()` is true, `front_matter()` equals the twin's, and the
    position of every field's `key_span.start` is the same.
- [ ] **Step 2: Run** them — expect failures (no files yet).
- [ ] **Step 3: Create** the files above.
- [ ] **Step 4: Run** the per-task checks; then `git ls-files --eol` as above after staging.
- [ ] **Step 5: Commit** `test(content): add the record corpus and crlf twins` (Refs: CE-09.4).

### Task 9: Body — blocks, wikilinks, malformed links

**Files:**
- Modify: `Cargo.toml` (`pulldown-cmark.workspace = true`), `src/lib.rs` (`pub mod body;`),
  `src/record/mod.rs` (store and expose the body)
- Create: `src/body/{mod.rs, events.rs, links.rs, malformed.rs}`,
  `tests/corpus/malformed.rs`, `tests/fixtures/corpus/malformed/expected.toml`,
  `tests/fixtures/corpus/malformed/things/synthetic-malformed-links.md`

**Interfaces — produces:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body { /* span, blocks, links, malformed; later: callouts, design_notes, beats */ }
impl Body {
    pub fn parse(index: &TextIndex, span: Span) -> Body;   // never fails: faults are data
    pub fn span(&self) -> Span;
    pub fn blocks(&self) -> &[Block];
    pub fn links(&self) -> &[Link];                        // well-formed links only (P9)
    pub fn malformed(&self) -> &[Malformed];
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block { pub span: Span, pub kind: BlockKind }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind { Heading, Other }                       // Tasks 10 and 12 add variants
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link { pub span: Span, pub target: String, pub target_span: Span,
                  pub fragment: Option<String>, pub fragment_span: Option<Span>,
                  pub shown: Option<String>, pub shown_span: Option<Span>, pub block: usize }
impl Link { pub fn beat(&self) -> Option<u32>; pub fn spelling(&self) -> String } // "[[t#f|s]]"
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Malformed { pub kind: MalformedKind, pub message: String, pub span: Span, pub position: Position }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MalformedKind { LinkUnclosed, LinkTarget, Embed, CalloutTitle, BeatHeading,
                         BeatAttribute, MisplacedAttributes }
impl MalformedKind { pub fn as_str(self) -> &'static str }  // "link-unclosed", "link-target", "embed",
    // "callout-title", "beat-heading", "beat-attribute", "misplaced-attributes"
impl Record { pub fn body(&self) -> &Body }
// events.rs: pub(crate) struct Events<'a> { pub source: &'a str /* whole record */, pub items: Vec<(Event<'a>, Span)> }
//   built once per body: Parser::new_ext(body, Options::ENABLE_WIKILINKS).into_offset_iter(),
//   offsets shifted by span.start; grammars read it, none re-parses.
```

- **Blocks:** the top-level events (depth 0) with their ranges; `Heading` for headings,
  `Other` for everything else.
- **Links** (spec §5.8): `Tag::Link` with `LinkType::WikiLink { has_pothole }`. `target_span`
  starts after `[[` and has the length of `dest_url`; a `#` splits off the fragment (its digits
  are `fragment_span`); with a pothole, `shown_span` runs from after `|` to before `]]`. `block`
  is the index of the top-level block it sits in. Valid: target `[A-Za-z0-9-]+`, fragment
  `[0-9]+` that fits `u32` (P14).
- **Malformed** (CE-10.3), each with the position of its span's start:
  - `link-target`: a target with a space, `/`, `^` or any other character outside the alphabet,
    or a fragment that is not a number;
  - `embed`: a `Tag::Image` with link type `WikiLink`;
  - `link-unclosed`: `[[` in the source range of a merged text event
    (`TextMergeWithOffset`); text events never hold code, so code is skipped.

- [ ] **Step 1: Write the failing unit tests**
  - `links_carry_exact_spans`: in `"x [[a|b|c]] y [[S-004#3]] [[mother-ilse]]\n"`: link 1 target
    `a`, shown `b|c` at `6..9` (body-relative), `spelling() == "[[a|b|c]]"`; link 2 target
    `S-004`, fragment `3`, `beat() == Some(3)`; link 3 target `mother-ilse`; every
    `target_span` slices to its `target`.
  - `links_everywhere_but_code`: links in a list, a block quote, a heading and a callout title are
    found; none in a code span, fenced code or an HTML comment.
  - `malformed_links_report_their_line` `// AC: CE-10.3`: the forms `[[`, `[[]]`, `[[|x]]`,
    `[[ x ]]`, `[[world/tower]]`, `[[a^b]]`, `[[S-004#x]]`, `![[x]]`, each its own paragraph
    (separated by blank lines), in a record whose body starts on line 4 → in order
    `link-unclosed` 4:1, `link-unclosed` 6:1, `link-unclosed` 8:1, `link-target` 10:1,
    `link-target` 12:1, `link-target` 14:1, `link-target` 16:1, `embed` 18:1; `links()` is
    empty. A paragraph `"è [[a b]]"` reports column 3.
  - `blocks_tile_the_body` `// AC: CE-10.4`: for bodies `""`, `"\n"`, `"x"` (no final newline),
    a setext heading, a `---` thematic break after a blank line, nested lists, fenced code and an
    HTML block: blocks are ordered, do not overlap, lie inside the body span, and every gap is
    whitespace only (P12).
  - `body_is_parsed_when_the_front_matter_fails` (Review Focus 1): a record with `a: &x 1` has
    an `Err` front matter and a body with its links.
- [ ] **Step 2: Write the malformed corpus harness**
  - `malformed/things/synthetic-malformed-links.md`: valid front matter (id
    `synthetic-malformed-links`, kind `thing`), a body with every malformed link form above,
    one per line, between prose.
  - `expected.toml`: `[[case]]` with `file` (relative to `malformed/`), `kind`, `line`, `column`.
  - `common.rs`: `Case` gains `kind: Option<String>`.
  - `malformed.rs`: `malformed_files_report_each_construct` `// AC: CE-10.3` — every `.md` in
    `malformed/` has at least one case; per file, the sorted `(kind, line, column)` of
    `body().malformed()` equals its cases exactly; the front matter is `Ok`.
    `records_have_no_malformed_constructs`: every file in `records/` has an empty `malformed()`.
- [ ] **Step 3: Run** the tests — expect failures.
- [ ] **Step 4: Implement** `events.rs`, `links.rs`, `malformed.rs`, `Body`.
- [ ] **Step 5: Run** the per-task checks and `cargo deny --locked check`.
- [ ] **Step 6: Commit** `feat(content): parse body blocks and wikilinks` (Refs: CE-10.3, CE-10.4).

### Task 10: Callouts — voice, design, other; `Design note:` paragraphs

**Files:**
- Create: `src/body/callout/{mod.rs, title.rs, voice.rs, design.rs}`, `src/body/design_note.rs`,
  `tests/corpus/grammars.rs`,
  `tests/fixtures/corpus/malformed/lore/topics/synthetic-malformed-voices-in-world.md`
- Modify: `src/body/mod.rs` (`callouts()`, `design_notes()`, `BlockKind::{Callout(usize),
  DesignNote(usize)}`), `malformed/expected.toml`, `tests/corpus/main.rs`

**Interfaces — produces:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Callout { pub span: Span, pub depth: usize, pub name: String /* lower case */,
    pub name_span: Span, pub fold: Option<Fold>, pub title_span: Span, pub content_span: Span,
    pub caption_span: Option<Span>, pub title: Option<CalloutTitle> /* None: malformed */ }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fold { Open /* + */, Closed /* - */ }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalloutTitle { Voice(Voice), Design(Design), Other }   // Task 11 adds Note(Note)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token { pub text: String, pub span: Span }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voice { pub speaker: Link, pub beliefs: Vec<Link>, pub tier: Option<Token> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Design { pub text: Option<Token>, pub links: Vec<Link> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesignNote { pub span: Span, pub form: DesignNoteForm }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesignNoteForm { Callout(usize) /* index into callouts() */, Paragraph { marker: Span } }
impl Body { pub fn callouts(&self) -> &[Callout]; pub fn design_notes(&self) -> &[DesignNote] }
// callout/mod.rs:
//   pub(crate) struct TitleInput<'a> { pub index: &'a TextIndex, pub title_span: Span,
//       pub tokens: &'a [TitleToken], pub links: &'a [Link] /* inside the title */,
//       pub paragraphs: &'a [Span] /* content paragraphs after the title line */,
//       pub caption: Option<Span>, pub depth: usize }
//   pub(crate) trait TitleGrammar { fn name(&self) -> &'static str;
//       fn parse(&self, input: &TitleInput<'_>) -> Result<CalloutTitle, Malformed>; }
//   const GRAMMARS: &[&dyn TitleGrammar] = &[&voice::VoiceGrammar, &design::DesignGrammar]; // Task 11 adds note
// title.rs: pub(crate) enum TitleToken { Link(Link), Bare(Token), Invalid(Token) }
//   pub(crate) fn split(index: &TextIndex, title_span: Span, links: &[Link]) -> Vec<TitleToken>
```

- **Callout** (spec §5.9): a block quote at any depth whose first child is a paragraph whose
  first line starts with `[!type]` (`type` = `[A-Za-z][A-Za-z0-9-]*`), then optionally `+` or
  `-`. The title is the rest of that line after one optional space, without the newline
  (`title_span` may be empty). `content_span` runs from the start of the next line to the end of
  the callout. `caption_span` (voice and note only): the last line of the last paragraph when it
  starts with `— ` (U+2014, space), from `—` to the line end. Unregistered types are
  `CalloutTitle::Other`.
- **Title tokens:** split at ` · ` (space, U+00B7, space) outside link spans; a token is a link
  when it is exactly one link's span, `Bare` when it is a non-empty run without whitespace,
  `[`, `]` or `·`, and `Invalid` otherwise (empty, two words, `·` without spaces, `•`, `|`).
- **Voice:** a link first (speaker), then links (beliefs), then at most one `Bare` (tier), last.
  Anything else is `callout-title` at the offending token (or at the title for an empty title).
- **Design:** never malformed; `text` is the whole title when not empty, `links` the links in it.
- **`Design note:` paragraphs:** a paragraph at the top level or anywhere inside a callout whose
  source starts with `Design note:`; its span is the paragraph's, `marker` the 12 bytes.
- Every `[!design]` callout also appears in `design_notes()` as `DesignNoteForm::Callout`.

- [ ] **Step 1: Write the failing unit tests**
  - `voice_blocks_parse_with_their_title_tokens` `// AC: CE-10.2`: the data model §9.1 example
    → speaker `throat-kin`, beliefs `[B-020]`, tier `T2`, caption slices to
    `— a binder, teaching the naming-song`, `content_span` starts on line 2 of the callout;
    `[!VOICE] [[a]]` → name `voice`; `[!voice]- [[a]] · T1` → `Fold::Closed`, tier `T1`;
    `[[a]] · [[B-1]] · [[B-2]]` → two beliefs, no tier.
  - `callouts_nest_and_continue_lazily`: `"> [!note] n1 · open\n> > [!voice] [[a]]\n> > x\n"`
    → two callouts, the voice at depth 2; a lazy continuation line lies in `content_span`;
    `-- a line` is not a caption and `— x` after it is.
  - `design_notes_parse_with_their_title_tokens` `// AC: CE-10.2`:
    `> [!design] A remark about [[f04-choir-hall]]` → text and one link; `> [!design]` → no
    text; `"Design note: the player first meets\nthis belief at F4.\n"` → one paragraph design
    note covering both lines, `marker` slices to `Design note:`; the same paragraph inside a voice
    block is found too.
  - `other_callouts_keep_their_title`: `> [!tip] A tip` → `Other`, name `tip`, title `A tip`.
  - `malformed_voice_titles_report_their_line` `// AC: CE-10.3`: `[!voice] T1`,
    `[!voice] [[a]] · T1 · [[b]]`, `[!voice] [[a]] ·  · T1`, `[!voice] [[a]]·[[b]]`,
    `[!voice] [[a]] • [[b]]`, `[!voice] [[a]] | T1`, `[!voice]` → one `callout-title` each, on
    the title's line; the callout is still listed with `title: None`.
  - `voice_contents_are_exposed`: the voice block's `content_span` of
    `synthetic-voices-in-world`'s first block slices to its speech line.
- [ ] **Step 2: Add** `malformed/lore/topics/synthetic-malformed-voices-in-world.md` (one
  paragraph of prose between the bad titles above) and its rows; `grammars.rs` with
  `corpus_voice_titles_parse` `// AC: CE-10.2`: every voice callout in
  `records/lore/topics/synthetic-voices-in-world.md` and
  `records/lore/topics/synthetic-callouts-in-world.md` has `Some(CalloutTitle::Voice(_))`, and
  the first of `synthetic-voices-in-world` has speaker `hearth-folk`, no beliefs, no tier.
- [ ] **Step 3: Run** the tests — expect failures.
- [ ] **Step 4: Implement** the callout modules and `design_note.rs`.
- [ ] **Step 5: Run** the per-task checks.
- [ ] **Step 6: Commit** `feat(content): parse voice and design callouts` (Refs: CE-10.2, CE-10.3).

### Task 11: Note callouts

**Files:**
- Create: `src/body/callout/note.rs`,
  `tests/fixtures/corpus/malformed/maps/synthetic-malformed-notes.md`
- Modify: `src/body/callout/mod.rs` (one registry line), `malformed/expected.toml`,
  `tests/corpus/grammars.rs`

**Interfaces — produces:**

```rust
// CalloutTitle gains Note(Note); GRAMMARS gains &note::NoteGrammar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note { pub id: Token /* "n3" */, pub number: u32, pub version: Option<Link>,
                  pub state: NoteState, pub state_span: Span, pub text: String }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoteState { Open, Dropped, ResolvedBy(Link), Unknown(String) }
impl NoteState { pub fn spelling(&self) -> String }   // "open", "dropped", "resolved by " + link.spelling(), the text
```

- **Title** (spec §5.9): tokens `n<k>` (`k` digits, fits `u32`), then optionally one version
  link, then the state token: `open`, `dropped`, `resolved by [[id]]` (exactly `resolved by `
  then one link and nothing else), or any other `Bare` word (`Unknown`). `state_span` covers the
  whole state token. Malformed (`callout-title`): no `n<k>`, no state, `resolved by` without a
  link, extra tokens, an `Invalid` token.
- **Text** (spec §5.9): the content without the title line and without the caption; per
  paragraph, its source lines with the block-quote prefix of the callout's depth removed (lazy
  lines have none), trimmed, joined by one space; paragraphs joined by `\n\n`. Links keep their
  source spelling.

- [ ] **Step 1: Write the failing unit tests**
  - `note_states_are_recognised` `// AC: CE-10.2`: `n3 · [[f04-terrain-v2]] · open` → number 3,
    version `f04-terrain-v2`, `Open`; `n4 · resolved by [[fates-shrine-v5]]` →
    `ResolvedBy(link with target fates-shrine-v5)`; `n5 · dropped` → `Dropped`;
    `n6 · closed` → `Unknown("closed")`; `[!note]- n2 · open` → fold `Closed`; every
    `state_span` slices to its state token.
  - `note_text_leaves_out_title_and_caption` `// AC: CE-10.2`: the data model §9.3 `n3`
    example → text `Lakes cover a tenth of the floor; the valley should drain east, toward the
    rim.`; a note `> a\n> b\nlazy\n>\n> c [[x|y]]\n> — Jann\n` → text `"a b lazy\n\nc [[x|y]]"`.
  - `malformed_note_titles_report_their_line` `// AC: CE-10.3`: `[!note] open`, `[!note] n1`,
    `[!note] n1 · [[v]]`, `[!note] n1 · resolved by`, `[!note] n1 · open · extra`,
    `[!note] n1·open` → one `callout-title` each on the title's line.
- [ ] **Step 2: Add** `malformed/maps/synthetic-malformed-notes.md` (kind `map`, the bad titles
  above under `## Notes`) and its rows; in `grammars.rs`, `corpus_note_states_parse`
  `// AC: CE-10.2`: `records/maps/synthetic-notes.md` gives `n1` open, `n2` open with version,
  `n3` resolved by `f04-terrain-v1`, `n4` dropped, `n5` dropped with version; `n1`'s text has
  no `— Jann`; `records/concepts/synthetic-concept-notes.md` gives `n4` as `Unknown("closed")`.
- [ ] **Step 3: Run** the tests — expect failures.
- [ ] **Step 4: Implement** `note.rs` and its registry line.
- [ ] **Step 5: Run** the per-task checks.
- [ ] **Step 6: Commit** `feat(content): parse note callouts with their states`
  (Refs: CE-10.2, CE-10.3).

### Task 12: Beats

**Files:**
- Create: `src/body/beat.rs`, `tests/fixtures/corpus/malformed/story/scripts/S-901.md` (the P10
  copy, unchanged), `tests/fixtures/corpus/malformed/story/scripts/S-904.md`
- Modify: `src/body/mod.rs` (`beats()`, `BlockKind::{BeatHeading(usize), BeatAttributes(usize)}`),
  `malformed/expected.toml`, `tests/corpus/grammars.rs`

**Interfaces — produces:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Beat { pub number: u32, pub number_span: Span, pub name: String, pub name_span: Span,
                  pub heading_span: Span, pub span: Span, pub attributes: Vec<Attribute> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute { pub key: String, pub key_span: Span, pub value: AttributeValue,
                       pub value_span: Span, pub line_span: Span }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttributeValue { Links(Vec<Link>), Token(Token), Raw { text: String, links: Vec<Link> } }
impl AttributeValue { pub fn spelling(&self) -> String }  // links joined by ", "; the token; the raw text
impl Body { pub fn beats(&self) -> &[Beat] }
```

Spec §5.10, read on the heading's and paragraph's source lines:
- **Heading:** a top-level `H2` (ATX or setext, P13) whose inline text (first to last inline
  event) starts with `Beat ` and a digit must read `Beat <n> — <name>` (digits fitting `u32`,
  space, U+2014, space, a non-empty name); otherwise `beat-heading`. A beat runs to the next
  top-level `H2` or the end of the body.
- **Attribute lines:** the paragraph starting on the line right after the heading's last line;
  its leading lines of the form `<key>::` (key rule) are attribute lines; the run ends at the
  first other line. After `::`: one or more spaces, then a non-empty value, else
  `beat-attribute`.
- **Shapes:** `at`, `with`, `says`, `reveals`: links (from `links()` inside the value span)
  separated by exactly `, `, covering the value (trailing spaces trimmed); `when`: one `Bare`
  token; anything else for these keys is `beat-attribute`. Other keys: `Raw` with their links.
- **Misplaced:** a paragraph after a blank line below a beat heading, whose first line has the
  attribute form, is `misplaced-attributes` at that line.

- [ ] **Step 1: Write the failing unit tests**
  - `s004_parses_into_its_beats` `// AC: CE-10.1` (in `grammars.rs`, on the corpus file
    `records/story/scripts/S-004.md`): two beats; Beat 1 `Arrival`: `at` [fates-shrine], `with`
    [mother-ilse], `says` [B-016], `when` `T1`; Beat 2 `The murmur`: `at` [fates-shrine],
    `reveals` [C-031], `when` `T1`; each link's `target_span` slices to that id.
  - `attribute_shapes_and_kept_data`: `at::   [[x]]` (extra spaces) → one link;
    `with:: [[a]], [[b]]` → two; `synthetic:: free text [[c]]` → `Raw` with one link; a beat
    without attributes; prose after the attribute lines in the same paragraph is not an
    attribute; a voice block right after the attributes is a callout; a setext beat heading is a
    beat; `## Beats` and `## Beat x — y` are ordinary headings; a beat ends at the next `H2`;
    a key given twice is kept as two attributes; `at:: [[a]], [[b]]` keeps both links.
  - `malformed_beat_attributes_report_their_line` `// AC: CE-10.3`: `at:: fates-shrine`,
    `with:: [[a]] [[b]]`, `when:: [[x]]`, `at::`, `at::[[x]]` → one `beat-attribute` each, on
    its line.
  - `malformed_beat_headings_report_their_line` `// AC: CE-10.3`: `## Beat 5 - x`,
    `## Beat 7 —`, `## Beat 8 —name`, `## Beat 9`, `## Beat 99999999999 — x` → one
    `beat-heading` each.
  - `misplaced_attributes_are_reported` `// AC: CE-10.3`: `"## Beat 1 — A\n\nat:: [[x]]\n"`
    → `misplaced-attributes` on the line of `at::`.
- [ ] **Step 2: Add** `malformed/story/scripts/S-901.md` (expects one `beat-heading` on the
  `Beat 5` line) and `malformed/story/scripts/S-904.md` (kind `script`, the bad attributes,
  headings and the misplaced case above), and their rows.
- [ ] **Step 3: Run** the tests — expect failures.
- [ ] **Step 4: Implement** `beat.rs`.
- [ ] **Step 5: Run** the per-task checks.
- [ ] **Step 6: Commit** `feat(content): parse beats and their attributes`
  (Refs: CE-10.1, CE-10.3).

### Task 13: The value encoder

**Files:**
- Create: `src/yaml/encode.rs`
- Modify: `Cargo.toml` (dev: `proptest.workspace = true`), root `Cargo.toml`
  (`[workspace.dependencies]` `proptest = { version = "1.12", default-features = false,
  features = ["std"] }`), `Cargo.lock`

**Interfaces — produces:**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Slot { Field, FlowItem, BlockItem, Entry }
pub(crate) fn encode(text: &str, keep: Option<ScalarStyle>, slot: Slot) -> Result<String, EncodeError>;
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError { #[error("a value cannot hold a line break or a control character other than a tab")]
                       Unrepresentable { text: String } }
```

Spec §5.7: a text with `\n`, `\r` or any other `char::is_control` except `\t` is
`Unrepresentable`. `keep == Some(SingleQuoted)` or `Some(DoubleQuoted)` keeps that style
(`spell`). Otherwise plain when the oracle reads it back as the same plain text, else
double-quoted. The oracle parses, with our own parser (`Record::parse`), a record whose front
matter is `k: <text>\n` (Field), `k: [<text>]\n` (FlowItem), `k:\n  - <text>\n` (BlockItem) or
`k:\n  e: <text>\n` (Entry), and checks one plain scalar with that text.

- [ ] **Step 1: Write the failing unit tests**
  - `plain_when_it_reads_back`: `draft`, `a b`, `42`, `true`, `null`, `a "b"`, `a\b`,
    `x, y` (Field), `a\tb` → unchanged.
  - `values_that_need_quotes_are_double_quoted` (Review Focus 2): `""` → `""` (two quotes),
    ` lead`, `trail `, `a: b`, `b #c`, `#x`, `- x`, `[x`, `]x`, `{x`, `}x`, `&a`, `*a`, `!a`,
    `|`, `>`, `'x`, `"x` (→ `"\"x"`), `%a`, `@a`, `` `a ``, `?`, `:` → double-quoted; in
    `FlowItem`: `x, y`, `a]`, `a[` → double-quoted; each result read back by the oracle in its
    slot gives the input.
  - `existing_styles_are_kept`: `SingleQuoted` + `It's` → `'It''s'`; `DoubleQuoted` + `a "b"` →
    `"a \"b\""`; `Plain` + `a: b` → `"a: b"`.
  - `line_breaks_and_controls_are_refused`: `a\nb`, `a\rb`, `\u{0}`, `\u{7f}`, `\u{85}` →
    `Unrepresentable`.
  - untagged property test `encoded_text_reads_back`: for texts from the regexes
    `[^\p{Cc}]{0,24}` and `[a-z\t #:,\[\]-]{0,12}`, in every slot, encode then oracle-read gives
    the text.
- [ ] **Step 2: Run** the tests — expect failures.
- [ ] **Step 3: Implement** `encode.rs`.
- [ ] **Step 4: Run** the per-task checks and `cargo deny --locked check`.
- [ ] **Step 5: Commit** `feat(content): encode front-matter values`.

### Task 14: The patcher — scalars, fields, verification

**Files:**
- Modify: `Cargo.toml` (`tracing.workspace = true`)
- Create: `src/yaml/patch/{mod.rs, field.rs, verify.rs}`, `tests/corpus/edits.rs`
- Modify: `tests/corpus/common.rs` (`pub fn cut(text: &str, span: Option<Span>) -> String`:
  the text without that span), `tests/corpus/main.rs`

**Interfaces — produces:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchOp { Set { key: String, value: RawValue }, Remove { key: String } } // Task 16 adds entry ops
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Order { /* fields: Vec<String>, entries: BTreeMap<String, Vec<String>> */ }
impl Order {
    pub fn new<I, S>(fields: I) -> Order where I: IntoIterator<Item = S>, S: Into<String>;
    pub fn with_entries<I, S>(self, key: &str, entries: I) -> Order where I: IntoIterator<Item = S>, S: Into<String>;
}
#[derive(Debug, thiserror::Error)]
pub enum PatchError { FrontMatter(YamlError), Key { key: String }, NotFound { key: String, entry: Option<String> },
    NotAMap { key: String }, NotInOrder { key: String }, Encode(#[from] EncodeError),
    Edit(#[from] RecordEditError), Verify { reason: String } }
impl Record {
    pub fn patch(&self, ops: &[PatchOp], order: &Order) -> Result<Record, PatchError>;
    pub fn set(&self, key: &str, value: RawValue, order: &Order) -> Result<Record, PatchError>;
    pub fn remove(&self, key: &str) -> Result<Record, PatchError>;
}
// field.rs (pub(crate)): edits for one op as Vec<TextEdit>; verify.rs: verify(old, new, op) -> Result<(), PatchError>
```

`patch` folds the ops (P7): each op makes `TextEdit`s, `with_edits` applies them, `verify`
checks the result, the next op runs on that record. Spec §5.7:
- **scalar → scalar:** replace the value span with `encode(text, Some(old style), slot)`; no
  "unchanged" short-cut.
- **new field** (key must match the key rule, else `Key`; must be in `order`, else
  `NotInOrder`): inserted at the end of `lines` of the last present field that precedes it in
  `order`; else at the start of the first field's `lines`; in an empty front matter at
  `front_span().start`. Text: `key: <encoded>\n`, a list as `key: [a, b]\n`, a map as
  `key:\n  e: v\n…` (entries in the given order).
- **remove:** delete the field's `lines`; `NotFound` when absent.
- **shape change** (and, until Tasks 15 and 16, a list or map written over a list or map):
  replace the field's `lines` with the new field text; a key-line comment is kept after one space
  (P8).
- **verify:** the new front matter parses; the body text is unchanged; every other field is
  present with the same `to_raw()` and the same bytes of `lines`; the touched field reads back
  the requested raw value (absent after a remove). A failure is `Verify` with a reason, logged
  with `tracing::debug!`, and no record is returned.

- [ ] **Step 1: Write the failing unit tests** (in `patch/mod.rs`; exact output texts)
  - `setting_a_scalar_changes_only_its_value` `// AC: CE-09.2`: f04-choir-hall, `title` →
    `The Choir Hall`: only that line differs; B-901 `status` → `canon` keeps
    `   # a comment after a value`; `title` → `a: b` gives `title: "a: b"`; `accuracy:
    'partial'` → `false` gives `accuracy: 'false'`; writing back the current value gives the same
    text.
  - `inserting_follows_the_callers_order` `// AC: CE-09.2`: with
    `Order::new(["id", "kind", "title", "status", "tags", "parent", "number"])` on
    f04-choir-hall (fields id, kind, title, parent, number, status): `tags` goes after `status`
    (the last present field preceding it); a key ordered before all present fields goes before
    `id`; into `"---\n---\n"` it goes on line 2; after a field followed by comment lines it goes
    directly after the field, above the comments (Review Focus 3).
  - `removing_a_field_removes_its_lines` `// AC: CE-09.2`: removing `parent` removes exactly
    its line.
  - `own_line_comments_stay_put` (Review Focus 3): in `"a: 1\n# about b\nb: 2\n# after b\nc: 3\n"`,
    removing `b` gives `"a: 1\n# about b\n# after b\nc: 3\n"`; inserting `bb` (ordered after
    `b`) gives `"a: 1\n# about b\nb: 2\nbb: x\n# after b\nc: 3\n"`.
  - `shape_changes_rewrite_the_field`: `role: x   # c` set to `List([a, b])` → `role: [a, b] # c`.
  - `refusals_return_no_text`: a line break → `Encode`; key `Bad` → `Key`; a key not in the
    order → `NotInOrder`; removing an absent key → `NotFound`.
  - `patching_a_broken_front_matter_fails` (Review Focus 1): a record with `a: &x 1` →
    `FrontMatter`.
  - `writes_turn_crlf_into_lf` `// AC: CE-09.4`: a CRLF record with `title` set to its own value
    → the result has no `\r` and equals the LF twin.
- [ ] **Step 2: Write** `edits.rs` with `scalar_edits_change_only_the_field_lines`
  `// AC: CE-09.2`: over the corpus — `C-031` `title` plain → plain and → a value needing quotes,
  `B-901` `status` (trailing comment); each asserts `cut(old, old_field.lines) ==
  cut(new, new_field.lines)` and that the field reads back the value.
- [ ] **Step 3: Run** the tests — expect failures.
- [ ] **Step 4: Implement** `patch/mod.rs`, `field.rs`, `verify.rs`.
- [ ] **Step 5: Run** the per-task checks.
- [ ] **Step 6: Commit** `feat(content): patch front-matter scalars and fields`
  (Refs: CE-09.2, CE-09.4).

### Task 15: The patcher — lists per item

**Files:**
- Modify: root `Cargo.toml` (`similar = "2.7"` in `[workspace.dependencies]`),
  `crates/theos-content/Cargo.toml` (`similar.workspace = true`), `Cargo.lock`
- Create: `src/yaml/patch/list.rs`
- Modify: `src/yaml/patch/field.rs` (lists over lists go through `list.rs`), `tests/corpus/edits.rs`

**Interfaces — produces:**

```rust
pub(crate) fn flow_edits(text: &str, list: &List, new: &[String]) -> Result<Vec<TextEdit>, PatchError>;
pub(crate) fn block_edits(text: &str, list: &List, new: &[String]) -> Result<Vec<TextEdit>, PatchError>;
```

The ops come from `similar::capture_diff_slices(Algorithm::Myers, &old_texts, &new)`; `Replace`
is a delete and an insert at the same place. Kept items produce no edit. New items are encoded
for their slot (`FlowItem`, `BlockItem`). Spec §5.7:
- **Flow:** a removed item goes with the separator before it (for item 0, with the separator
  after it); a new item is written as `, <v>` after the item that precedes it in the new list,
  as `<v>, ` before the first old item, or the whole `[…]` becomes `[<v>, …]` when the old list
  is empty; a list emptied of all items becomes `[]`. Nothing outside `[` … `]` changes.
- **Block:** a removed item's `lines` go; a new item is the line `<indent>- <v>\n` (the first
  item's indent) inserted right after the line of the item that precedes it in the new list, or
  at the start of the first old item's line; own-line comments and blank lines keep their place.
  A block list set to no items is rewritten as `key: []` (P4).

- [ ] **Step 1: Write the failing unit tests** (exact outputs)
  - `flow_lists_change_per_item` `// AC: CE-09.2`: `[a, b, c]` minus `b` → `[a, c]`, minus `a`
    → `[b, c]`, minus `c` → `[a, b]`; `[a,b]` plus `c` → `[a,b, c]`; `[ a ]` with `b` first →
    `[ b, a ]`; `['a', "b"]` written back → unchanged; `[a, b,]` minus `b` → `[a,]`, plus `c` →
    `[a, b, c,]`; `[]` → `[a, b]`; `[a]` → `[]`; `[a, b, c]` → `[c, a, b]`; `[a]` plus `x, y`
    → `[a, "x, y"]`; a comment after `]` stays.
  - `block_lists_change_per_item` `// AC: CE-09.2`: `"l:\n  - a\n  - b\n  - c\n"` minus `b` →
    `"l:\n  - a\n  - c\n"`; plus `d` after `a` → a line `  - d` after `a`; plus `z` first →
    before `a`; indent 0 keeps indent 0; reordering keeps each kept item's line bytes; set to
    `[]` → `l: []` with the key-line comment kept.
  - `block_list_comments_stay_put` (Review Focus 3): `"l:\n  - a  # first\n  # between\n  - b\n"`
    minus `a` → `"l:\n  # between\n  - b\n"`; plus `x` after `a` →
    `"l:\n  - a  # first\n  - x\n  # between\n  - b\n"`.
  - `duplicates_and_replacements` (Review Focus 4): `[a, a, b]` → `[a, b]` gives `[a, b]`;
    `[a, b]` → `[x, y]` gives `[x, y]`; the block list `a, a, b` → `a, b` removes one line.
- [ ] **Step 2: Extend** `edits.rs` with `list_edits_change_only_the_field_lines`
  `// AC: CE-09.2`: corpus `C-031` `where` (flow) add and remove; `B-901` `held_by` (block) add,
  remove, reorder; the block list with comments in `synthetic-yaml-comments`; each asserts the
  `cut` equality and the read-back.
- [ ] **Step 3: Run** the tests — expect failures.
- [ ] **Step 4: Implement** `list.rs` and route list-over-list sets to it.
- [ ] **Step 5: Run** the per-task checks and `cargo deny --locked check`.
- [ ] **Step 6: Commit** `feat(content): patch front-matter lists per item` (Refs: CE-09.2).

### Task 16: The patcher — maps per entry

**Files:**
- Create: `src/yaml/patch/map.rs`
- Modify: `src/yaml/patch/{mod.rs, field.rs, verify.rs}`, `tests/corpus/edits.rs`

**Interfaces — produces:**

```rust
// PatchOp gains: SetEntry { key: String, entry: String, value: RawEntry },
//                RemoveEntry { key: String, entry: String }
impl Record {
    pub fn set_entry(&self, key: &str, entry: &str, value: RawEntry, order: &Order) -> Result<Record, PatchError>;
    pub fn remove_entry(&self, key: &str, entry: &str) -> Result<Record, PatchError>;
}
```

`verify` for an entry op expects the field's old raw map with that entry set or removed (the
field absent when its last entry was removed).

Spec §5.7, by entry key:
- setting an entry's scalar replaces its value span (`Slot::Entry`, old style kept); a flow-list
  value goes through `flow_edits`; a shape change inside the entry rewrites the entry's line;
- a new entry is the line `<indent><key>: <v>\n` (the first entry's indent) after the last
  present entry that precedes it in `order`'s entries for the field (P6: missing → last);
- removing an entry deletes its `lines`; removing the last entry removes the field;
- `set` with `RawValue::Map` over a map runs per entry (remove the missing, set the kept, insert
  the new in the given order);
- `set_entry` on an absent field inserts the field (it must be in `order`); on a field that is
  not a map → `NotAMap`; `remove_entry` of an absent entry → `NotFound`.

- [ ] **Step 1: Write the failing unit tests** (exact outputs, on `mother-ilse`)
  - `map_entries_change_one_by_one` `// AC: CE-09.2`: `set_entry("lens", "speaks_to", …)`
    changes only the value; `set_entry("lens", "stance", Text("wary"))` with entry order
    `[stance, speaks_to]` goes before `speaks_to`; with no entry order it goes last;
    `remove_entry("lens", "speaks_to")` on a two-entry map removes that line; on a one-entry map
    removes the field; a flow-list entry `"m:\n  e: [a,b]\n"` plus `c` gives `[a,b, c]`.
  - `map_refusals`: `set_entry` on `title` → `NotAMap`; `remove_entry` of `nope` → `NotFound`;
    `set_entry` on an absent `lens` with `lens` in the order inserts `lens:\n  stance: wary\n`.
- [ ] **Step 2: Extend** `edits.rs` with `map_edits_change_only_the_field_lines`
  `// AC: CE-09.2`: `mother-ilse` `lens.stance` added and `lens.speaks_to` changed and removed;
  the map in `synthetic-yaml-comments` with comments between its entries.
- [ ] **Step 3: Run** the tests — expect failures.
- [ ] **Step 4: Implement** `map.rs`.
- [ ] **Step 5: Run** the per-task checks.
- [ ] **Step 6: Commit** `feat(content): patch front-matter maps per entry` (Refs: CE-09.2).

### Task 17: Corpus round trips, the edit matrix, CRLF writes, the crate README

**Files:**
- Create: `tests/corpus/round_trip.rs`
- Modify: `tests/corpus/{main.rs, edits.rs, records.rs}`, `crates/theos-content/README.md`,
  `src/lib.rs` (crate docs list the modules and what P03–P12 use, spec §6)

- [ ] **Step 1: Write the failing tests**
  - `corpus_records_round_trip_through_the_patcher` `// AC: CE-09.1` (`round_trip.rs`): for
    every file of `project_records()`, with `Order::new` of the record's own keys: the input
    bytes equal `record.text()`; `set(key, field.value.to_raw())` for every field, one at a
    time, returns the same text; `set_entry` for every map entry likewise; one `patch` with all
    those ops together returns the same text; `Record::parse` of the output has a front matter
    and body equal to the first.
  - `corpus_bodies_round_trip_through_span_edits` `// AC: CE-10.4`: for every body: blocks tile
    it (Task 9's assertions); then, built from the model (P11), each of these edits alone and all
    together as one `apply_edits` returns the same text: `target_span` ← `target`,
    `fragment_span` ← `fragment`, `shown_span` ← `shown`, `span` ← `Link::spelling()`, every
    note's `state_span` ← `NoteState::spelling()` and `id.span` ← `id.text`, every beat
    attribute's `value_span` ← `AttributeValue::spelling()`, every beat's `number_span` ←
    `number.to_string()` and `name_span` ← `name`, every voice tier and design title token's
    span ← its text.
  - `crlf_writes_return_lf` `// AC: CE-09.4` (`records.rs`): for every CRLF input of
    `twins.toml`, setting every field to its current value returns text without `\r` that equals
    the LF twin's text.
  - `edit_matrix_changes_only_the_field_lines` `// AC: CE-09.2` (`edits.rs`): over the corpus,
    the matrix of spec §5.14: a scalar plain → plain, → needing quotes, with a trailing comment;
    a flow list; a block list (add, remove, reorder, comments inside); `lens.stance`; inserting a
    missing field at the start, in the middle, at the end, and `tags` after `status`; removing a
    field and a map entry. Each asserts the `cut` equality (insert: only new lines; removal:
    only that field's lines gone) and that the result reads back the expected model.
  - untagged property tests next to it: `random_scalars_change_only_their_field` and
    `random_lists_change_only_their_field` — random texts without line breaks set on
    `f04-choir-hall`'s `title` and `C-031`'s `where`: the `cut` equality holds and the value
    reads back; texts with a line break give `Encode`.
- [ ] **Step 2: Run** them — fix any failure at its cause (the spans or the patcher, never the
  test), with superpowers:systematic-debugging.
- [ ] **Step 3: Write** the crate README (what `theos-content` provides after P02, the
  coordinates rule of spec §5.2, the corpus and who reuses it) and the `lib.rs` docs.
- [ ] **Step 4: Run** the per-task checks, then `cargo xtask ac-trace --phase P02`: every AC
  CE-09.1 to CE-10.4 shows at least one passing test.
- [ ] **Step 5: Commit** `test(content): prove the round trips over the corpus`
  (Refs: CE-09.1, CE-09.2, CE-09.4, CE-10.4).

## After the tasks

theos-phase steps 4 to 6: the final review loop of `CLAUDE.md` (fresh most-capable reviewer per
round, cap 5), then `cargo xtask gate` on Windows and `cargo xtask ac-trace --phase P02`, then
the PR `feat(content): records and grammars (P02)` with every ruling of the ledger (P1–P17 above
plus the build's). P02 has no OS-specific code: no `windows` or `macos` labels.
