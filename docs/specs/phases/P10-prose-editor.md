# P10 · Prose editor research — phase spec

Status: proposed · waits for Jann · 31 Oct 2026
Umbrella entry: `docs/specs/00-umbrella.md` §3, P10 · slice S1 · depends on: –
Output: the decision record `docs/decisions/0001-prose-editor.md` (protected; Jann approves).

## 1. Goal

P12 builds the manuscript editor "on the component chosen in P10" (CE-34). P10 chooses that
component with evidence Jann can check: a comparison of five candidates, prototypes of the top two
that load and save a corpus, a byte report, and a recommendation Jann accepts or rejects.

P10 writes no production code. Everything except the decision record lives in
`research/P10-prose-editor/` and is throwaway: later phases may port patterns from it, never
import from it.

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-06 Choose the prose editor component**
- CE-06.1 Comparison of CodeMirror 6, ProseMirror/Tiptap, Milkdown, Lexical and Monaco on:
  byte-stable Markdown, front matter untouched, inline link chips, callouts with their own look,
  beat headings with attribute lines, spell check and IME, long documents, Svelte 5 integration,
  licence, maintenance. Sources cited.
- CE-06.2 Prototypes of the top two load and save the corpus; saved bytes are compared and
  reported.
- CE-06.3 A recommendation Jann can accept or reject.

## 3. What the chosen component must carry

The criteria of CE-06.1 are judged against what later phases need, not in the abstract.

| Need | From |
|---|---|
| Saving leaves untouched bytes identical | CE-34.1, G5 |
| An external change reloads a clean record and offers a merge for a dirty one | CE-34.2 |
| Field undo hands over to command undo on save | CE-34.3 |
| `[[` suggests ids of the kinds allowed at that place; a dangling link shows as an error | CE-35.1, CE-35.2 |
| Voice blocks in the in-world style with developer words flagged inline; note and design callouts with their own look | CE-35.3, data model §9.1–9.3 |
| Beat headings with attribute lines; a finding shows on the beat that causes it | data model §9.4, CE-41.2 |
| Read, Write and Source modes; front matter in the side panel, edited by the span patcher | `docs/ui/screens/01-main.html`, `04-script.html`, data model §4 |
| Write mode is a live preview: the Markdown of the construct under the caret shows raw, everything else looks like the screens; line breaks show as written | Jann's choice for P10 |
| The design system's classes: `tu-link`, `tu-voice`, `tu-beat`, `tu-note`, LinkAutocomplete | G13, `design/theos-ui/` |
| Ctrl on Windows and Linux, Cmd on macOS, from one keymap | G13 |
| No network, no telemetry | G12 |
| Runs in the Tauri 2 webviews: WebView2 (Windows), WKWebView (macOS), WebKitGTK (Linux) | G2, `docs/architecture.md` §3 |

## 4. Design

### 4.1 Order of work

1. Corpus (§4.2).
2. Comparison of all five candidates on paper (§4.3), then the top two (§4.4).
3. Harness and the two prototypes (§4.5).
4. Measurements: bytes (§4.6), input (§4.7), long documents (§4.8).
5. Measured scores replace paper scores for the top two; the comparison is final.
6. Decision record (§4.9).
7. Evidence check (§4.10), verification (§4.11), PR (§4.12).

The plan cannot name the top two before step 2 ends. It writes the prototype tasks for
"prototype A" and "prototype B" and is updated once the comparison names them (plans are not
protected).

### 4.2 Corpus

`research/P10-prose-editor/corpus/` is the corpus of CE-06.2. No real records exist yet
(tower-of-theos has no content before P01 and the lore import CE-04), so P10 builds it:

- The examples of `docs/specs/data-model.md` §14, one file each, bytes as in the data model.
- Synthetic records that together cover every construct in the table below. They reuse ids,
  names and texts from the data model and the reference screens and add no lore beyond them.
- Every file carries the comment line `# synthetic fixture (P10), not canon` in its front matter,
  which also tests that comments survive.
- `corpus/manifest.json` lists every file with its origin (`data-model §14` or `synthetic`), the
  constructs it covers and the edits run on it (§4.6).

| Area | Constructs, each in at least one file |
|---|---|
| Front matter | comments on their own line and after a value; plain, single- and double-quoted scalars; both list forms; one-level maps (`lens`, `outputs`, `results`) |
| Prose | paragraphs hard-wrapped over several lines; ATX headings 1–3 and a setext heading; `*`, `_`, `**`, `__` emphasis; inline code; a fenced code block; lists with `-`, `*`, `+`, `1.`, `1)`, nested lists, a list numbered `1. 1. 1.`; a plain block quote; a GFM table; thematic breaks `---` and `***` in the body; an HTML comment; hard breaks by two trailing spaces and by `\`; backslash escapes; an autolink and a Markdown link |
| Links | `[[id]]`, `[[id\|shown text]]`, a dangling `[[id]]`; links next to punctuation, at line start and end, inside emphasis, inside callout titles and beat attributes |
| Callouts | voice with the speaker only, with beliefs and tier, with and without a `— ` caption; empty `>` lines inside; a lazy continuation line; `> [!design]`; a `Design note:` line; note callouts in all three states under `## Notes` |
| Beats | every attribute of data model §9.4; a beat without attributes; a beat followed directly by a voice block; extra spaces after `::` |
| Characters and whitespace | `—`, `·`, curly quotes; `é` precomposed and as `e` + U+0301; U+00A0; an emoji (surrogate pair); tabs in prose; trailing spaces; two and three blank lines in a row; no final newline |
| Whole files | an empty body; one CRLF file (expected output: its LF form, as CE-09.4) |

Long documents are generated, not committed. `corpus/generate-long.ts` writes three files
deterministically into an ignored folder, built from the synthetic constructs (beats, voice
blocks, a link every few lines): **typical** ≈ 5 KB, **large** ≈ 250 KB (about 40 000 words, more
than any record we expect), **stress** ≈ 2.5 MB.

### 4.3 Comparison (CE-06.1)

`research/P10-prose-editor/comparison.md`: one section per candidate, a matrix of 5 candidates ×
10 criteria, and a source list.

**Candidates** are the five of CE-06.1. ProseMirror/Tiptap is one candidate: the comparison
scores the family and says which form it would prototype (plain ProseMirror with
`prosemirror-markdown`, or Tiptap) and why. Each candidate is judged at its current release on
the day of the research; the versions are recorded.

**Criteria**, each defined so it can be checked:

| # | Criterion | What is judged | Weight |
|---|---|---|---|
| 1 | Byte-stable Markdown | Load and save without edits gives the same bytes. After an edit, every byte outside the edit is unchanged (span level; G5 says "any edit leaves untouched bytes identical", so block level is not enough). Is the source text the document, or is it parsed into a tree and serialized again? | 3, knockout |
| 2 | Front matter untouched | The front matter bytes stay identical through every edit (typing, delete at the body start, select-all and type, paste, undo). In Write mode the adapter may split the front matter off and re-attach it on save; what tells candidates apart is Source mode: the same component shows the whole file raw with the front matter protected (up to 3), or Source mode needs a second component (at most 2) | 2, knockout |
| 3 | Inline link chips | `[[id]]` and `[[id\|text]]` as `tu-link` chips with kind glyph, title or shown text, and a broken state; the caret steps over a chip as one unit; delete removes the whole link; `[[` opens a popover filtered by the kinds allowed at that place | 2 |
| 4 | Callouts with their own look | Voice, design and note callouts shown with their looks while their text stays editable in place; `Design note:` lines; developer words flagged inline in voice text | 2 |
| 5 | Beat headings with attribute lines | `## Beat n — name` and its `key:: value` lines shown as in `04-script.html` (heading and a `tu-beat` parameter row with chips); editing a parameter changes only its line; a finding shows on its beat | 2 |
| 6 | Spell check and IME | Prose is real text in a `contenteditable` with native spell check, not a hidden textarea or a canvas; spell check is off for ids, chips and attribute lines; IME composition and dead keys (Swiss and German layouts) give the right text without loss or doubling, also next to a chip | 2 |
| 7 | Long documents | Open, type and scroll the typical, large and stress files (§4.8) | 1 |
| 8 | Svelte 5 integration | Integration into the Svelte 5 app: mount and tear down inside a Svelte 5 component with runes; render design-system Svelte components inside the editor (chip, voice head, beat row) with `mount` and `unmount`; push index or finding changes in without re-creating the editor; take its shortcuts from the app's one keymap (G13); apply an external change as a minimal edit that keeps the caret and the local undo history (CE-34.2); let the host mark and clear the undo history on save (CE-34.3); state of official or community bindings | 2 |
| 9 | Licence | Licence of the package and of every transitive dependency of the prototype (`pnpm licenses list`); no paid tier for a needed feature; no network service or telemetry (G12) | pass or knockout |
| 10 | Maintenance | Releases and commits in the last 12 months; maintainers and backing; open issues in the areas we depend on (Markdown, IME, decorations); API stability and breaking changes | 2 |

**Scores** per cell, 0 to 3: 0 = not possible, no credible path · 1 = possible with heavy custom
work or known open bugs · 2 = works with moderate custom work · 3 = works with stock features and
little glue. Licence cells are `pass` or `fail`. Weighted total = Σ weight × score, at most 54. A
knockout criterion at 0 (or licence `fail`) removes the candidate.

**Sources.** Every cell cites at least one source: documentation, a repository file, an issue or
pull request, a release note, or a measurement of the prototypes (`report/…`). Each entry of the
source list has a URL or repo path and the date it was read. A claim without a source is not
scored. Facts about the webviews (spell check and IME in WebView2, WKWebView and WebKitGTK) apply
to every `contenteditable` candidate alike and are listed once, with sources.

### 4.4 Choosing the top two

The top two are the two highest weighted totals among the candidates without a knockout; a tie
goes to the higher byte-stability score. If fewer than two survive, the missing prototype is the
highest-scoring knocked-out candidate, built to confirm the knockout by measurement.

After the prototypes, measured scores replace the paper scores for the top two. The matrix shows
both, and every change is explained. The measured scores follow rules fixed here, before any
result exists:

- **Byte stability (criterion 1)** is scored on the worse of the two engines and, for a WYSIWYG
  prototype, on the best-effort column (stock is reported beside it). 3 = every unedited file is
  identical and every edit matches the oracle · 2 = every unedited file is identical, and the
  mismatches are confined to rare constructs with a named fix · 1 = unedited files are identical,
  but edits change bytes beyond the edit in common constructs (block level only) · 0 = an
  unedited round trip is not identical (knockout).
- **Criteria 3–6 and 8** use the 0–3 scale on what the prototype achieved: 3 = works as defined
  with stock features and little glue · 2 = works with moderate custom code · 1 = works only in
  part within the budget, or with known bugs · 0 = not achieved and no credible path found. The
  lines of adapter code per feature are reported as evidence for the difference between 2 and 3.
- **Budget**, the same for every feature of both prototypes: one implementation task and one fix
  round. What is still missing after that is recorded with what was tried, and scored on what
  works.
- **A knockout by measurement** of one of the top two does not start a third prototype: the
  survivor is recommended, and the alternative is the next-ranked candidate (measured or on
  paper) with its costs. If both are knocked out, P10 stops and asks Jann.

The hypothesis, stated so the comparison can refute it: CodeMirror 6 as an Obsidian-style live
preview (the source text is the document; chips and callouts are decorations) against a
ProseMirror-family editor. The scores decide, not the hypothesis.

### 4.5 Harness and prototypes

```text
research/P10-prose-editor/
  README.md            how to run, where the results are, summary, manual checklist (§4.7)
  comparison.md        CE-06.1
  corpus/              §4.2, with manifest.json and generate-long.ts
  report/              written by the test run and committed (§4.6–4.8)
  app/                 Vite, Svelte 5, TypeScript (strict), pnpm
    src/harness/       adapter interface, corpus loader, mock index, Write/Source switch, styles
    src/adapters/      one folder per prototype
    tests/             Playwright tests and document checks
```

- A standalone pnpm project with its own lockfile and exact version pins. It is not part of
  `editor-ui/` (P11) or of any pnpm workspace. pnpm comes through corepack, pinned in
  `packageManager`; the Node version is in `engines`.
- Styles and fonts come from `design/theos-ui/` (tokens, `components/bundle.css`, fonts),
  referenced from the repo, never copied.
- **One adapter interface** for both prototypes, so the harness and every test are shared. A
  sketch; the plan fixes the exact shape:

  ```ts
  interface ProseAdapter {
    mount(target: HTMLElement, text: string, ctx: EditorContext): void;
    text(): string;                     // the whole file as the editor would save it
    apply(edit: ScriptedEdit): void;    // an edit through the editor's own commands
    setMode(mode: 'write' | 'source'): void;
    destroy(): void;
  }
  interface EditorContext {
    resolve(id: string): LinkInfo | undefined;  // mock index: kind, title
    allowedKinds(place: LinkPlace): Kind[];     // prose, or a beat attribute (data model §9.4)
    findings: Finding[];                        // mock findings: rule, severity, line or beat
    developerWords: string[];                   // mock word list for voice text
  }
  ```

  These shapes serve the research only; P11 generates the real IPC types from Rust (CE-32.4).
- **The same feature list for both prototypes** (a full Write-mode spike, Jann's choice):
  - The harness hands the adapter the whole file. In Write mode the front matter is hidden,
    either as a protected range or split off by the adapter and re-attached on save. Source mode
    shows the whole file raw with the front matter protected; whether Source mode may edit it is
    P12's call.
  - Write mode is a live preview (Jann's choice): the Markdown of the construct under the caret
    shows raw, everything else is rendered; line breaks show as written.
  - Link chips with the broken state, and the `[[` popover with its kind filter.
  - Voice, design and note callouts with their looks and editable text; developer words flagged
    in voice text.
  - Beat headings with their parameter row; a parameter can be edited through the row (and as
    its raw line under the caret); "+ add"; a mock finding on its beat.
  - The shortcuts come from one keymap object owned by the harness (Ctrl, or Cmd on macOS).
  - The harness can apply an external change and mark or clear the undo history (criterion 8).
  - Read mode is not built.
- Spike quality: no tests beyond §4.6–4.8, no polish. A feature that resists a reasonable attempt
  is recorded with what was tried; that is evidence, not a blocker.
- **A WYSIWYG prototype is measured twice** (Jann's choice), if one of the top two parses Markdown
  into a tree. *Stock*: the library's own Markdown parser and serializer. *Best effort*: untouched
  blocks keep their original source text; inside an edited block, the editor's change steps are
  mapped onto the source text as span edits; where the mapping fails, the block is serialized
  again. The report shows both columns and, per edit, which level the best effort reached.

### 4.6 Byte report (CE-06.2)

For each prototype (both columns for a WYSIWYG one), each engine (Playwright's Chromium and
WebKit) and each corpus file:

1. **Unedited.** Load, save, compare with the file's bytes. Expected: identical. The CRLF file is
   expected to equal its LF form.
2. **Edits.** The scripted edits of `manifest.json`, each run twice: through `apply` (the
   editor's commands) and through real input (Playwright keyboard and clicks on the rendered
   page). At least: insert and delete a word mid-paragraph; type a sentence at the end of a voice
   block; change a note's state; add and change a beat parameter; insert a link through the `[[`
   popover; delete a link chip; select-all and type in Write mode (replaces the body only); paste
   text with CRLF line ends (inserted with LF) and paste rich HTML (inserted as its plain text);
   an external change elsewhere in the file while a local edit is unsaved (the caret stays, and
   undo still removes the local edit); undo back to the start.
3. **Oracle.** The expected file is the original with the same text change applied to the source
   by the test, not by the editor. Undo back to the start expects the original bytes.
4. **Bytes, not strings.** The saved text is encoded as UTF-8 and compared byte by byte with the
   oracle, so normalization (NFC, U+00A0, line ends) shows up.

`report/round-trip.md` and `report/round-trip.json` are written by the run and committed. Per
prototype, column and engine: unedited files identical (N of M) and edits matching the oracle
(N of M). For every mismatch: file, edit, first differing line and column, bytes changed beyond
the edit, whether the damage stays inside the edited block, and a diff hunk with whitespace made
visible. The header records the commit, OS, browser versions and date.

### 4.7 Spell check and IME

- **Chromium, automated:** IME composition through the DevTools protocol
  (`Input.imeSetComposition`, `Input.insertText`): a Japanese composition and dead-key sequences
  (`^` then `ê`, `¨` then `ü`), each in prose, right after a link chip and inside a voice block;
  bytes checked against the oracle.
- **WebKit, automated:** text insertion only, because Playwright offers no composition input for
  WebKit; the gap is listed.
- **Both engines, automated:** prose sits in a `contenteditable` with real text nodes and
  `spellcheck` on; chips, ids and attribute lines carry `spellcheck="false"`.
- **Not automatable:** whether the webview draws spelling marks and offers suggestions. The
  comparison cites what WebView2, WKWebView and WebKitGTK do. The research README has a
  five-minute manual checklist (a misspelled word, its suggestions, dead keys on a Swiss German
  layout, a Windows IME) that Jann may run before accepting. P11 checks the same in the real
  Tauri shell.
- `report/input.md` collects the results.

### 4.8 Long documents

Per prototype, in Chromium (and WebKit for timings): time from load to first paint;
keystroke-to-paint latency, median and p95 over 200 keystrokes at the start, middle and end of
each file; scrolling to the end; JS heap size (Chromium only). `report/long-docs.md` records the
numbers and the machine. A score of 3 needs, on the large file, under 1 s to open and a p95 under
50 ms; the stress file only has to stay usable. The numbers are evidence, not a gate.

### 4.9 Decision record (CE-06.3)

`docs/decisions/0001-prose-editor.md`, the first numbered record of `docs/decisions/README.md`:

- **Status:** proposed; accepted when Jann merges the P10 PR.
- **Decision:** one component for P12 (for ProseMirror/Tiptap: which form), with the version
  researched.
- **Why:** the weighted totals, the byte-report counts and the evidence that decided, linked into
  `research/P10-prose-editor/`.
- **Consequences for P12:** what P12 has to build on top (parser extensions, decorations,
  widgets), known gaps and risks, and what P11 must check in the real webviews.
- **Alternative:** the runner-up and what it would cost instead.
- **If rejected:** Jann names the reason. If the runner-up does not share it, the record is
  rewritten for the runner-up in the same PR; otherwise P10 goes back to the comparison with the
  reason as a new criterion.
- The README row "open · Prose editor component — decided by phase P10." becomes a dated row that
  names the component and links `0001`.

### 4.10 Evidence check

Before the PR, one fresh reviewer on the most capable model checks every scored claim against
its source, every score against the definitions of §4.3, and the decision record against the
reports. One fix pass follows; there is no review loop (see §9).

### 4.11 Tests and verification

- **AC tags** (G15, `docs/gate.md` §4). The Playwright byte tests carry `// AC: CE-06.2`. Three
  document checks make the ACs testable as a whole:
  - `comparison.md` has all 50 cells filled, a score 0–3 (licence: `pass` or `fail`) in each,
    and at least one source per cell that resolves to a dated entry of the source list
    (`// AC: CE-06.1`);
  - `report/round-trip.json` covers every corpus file, both prototypes (and both columns of a
    WYSIWYG one) and both engines (`// AC: CE-06.2`);
  - the decision record has every section of §4.9, recommends exactly one of the top two and
    links only reports that exist (`// AC: CE-06.3`).
- **Verification** (theos-phase step 5): `pnpm test` in `app/` on Windows, output read; on Linux
  as well when a Linux machine is available. The report says where the tests ran.
- **Gate.** P00 builds the gate in parallel. If P00 is merged when P10 finishes, P10 runs
  `cargo xtask gate` and `cargo xtask ac-trace --phase P10`, which has to find the TypeScript tags
  under `research/` (§5). If it is not merged, that is recorded as a ruling and listed in the PR.
- No CI job runs the research tests: the `ui` check starts in P11 and covers `editor-ui/`.

### 4.12 Branches and PRs

- **Spec:** this document, branch `spec/P10-prose-editor`, PR `docs(spec): prose editor research
  (P10)`.
- **Build:** branch `phase/P10-prose-editor` in the worktree `.worktrees/P10-prose-editor`, one
  PR `docs(ui): prose editor research (P10)` with `research/P10-prose-editor/`,
  `docs/plans/P10-prose-editor.md`, `docs/status.md`, `docs/decisions/0001-prose-editor.md` and
  the README row. It touches a protected path, so it waits for Jann and is never auto-merged
  (Jann's choice: the research is the evidence for the decision and is reviewed with it; CLAUDE.md
  rule 3 is read as not applying to P10, whose result is a decision record by the theos-phase
  skill).

## 5. Interfaces other phases use

| Phase | Uses | Binding |
|---|---|---|
| P12 | The decision record: component, version, the patterns to build on, gaps | Yes, once accepted |
| P12 | The byte-test method (whole-file oracle, edits through commands and through real input, byte comparison) to prove CE-34.1 in `editor-ui/` | Recommended; the code is ported, not imported |
| P11 | The webview checks the research could not automate (spelling marks, OS input methods) | Listed in the decision record |
| P11, P12 | The `EditorContext` shapes as input for the IPC types (CE-32.4) | No; the Rust types win |
| P02 | The corpus files as a start for its fixture corpus | No; they follow the data model and are marked synthetic |
| P00 | `ac-trace` finds `// AC:` tags in TypeScript tests under `research/**` | Needed for P10 to pass `ac-trace` |

Nothing under `research/` is imported by a later phase; P12 may delete the folder once the
manuscript editor exists.

## 6. Out of scope

- Production code in `editor-ui/` or `crates/`; Tauri; Rust.
- Read mode; a real index, validation or IPC (mocks only).
- Proving CE-34 or CE-35 (P12).
- Editors other than the five of CE-06.1.

## 7. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| The researcher favours its hypothesis | A biased pick | Criteria, weights and knockouts fixed here before the research; every score needs a source; evidence check (§4.10) |
| Playwright's Chromium and WebKit are not WebView2, WKWebView and WebKitGTK | Spell check or IME behaves differently in the app | Webview facts cited; manual checklist; P11 checks in the real shell |
| The synthetic corpus misses constructs of the real lore import (CE-04) | Byte stability proven on the wrong inputs | Broad construct list (§4.2); P12 re-runs the byte tests on real records once they exist |
| Best-effort byte preservation hides costs (undo, edits across blocks) | A WYSIWYG editor looks cheaper than it is | Stock and best-effort columns; block or span level per mismatch |
| The editor's Markdown parse and `theos-content` (P02) disagree on beats or callouts | Wrong rendering, not wrong data | The editor parses for display only; links and findings come from `theos-content`; noted for P12 |
| Headless timings differ from the desktop app | Long-document scores are off | Machine recorded; numbers are evidence, not a gate |
| Spike code leaks into production | Unreviewed code in P12 | `research/` is never imported; P12 ports patterns |
| P00's `ac-trace` does not scan `research/` | P10 cannot pass `ac-trace` | Listed under "Needs Jann" for the P00 spec |
| The licence check covers only the pinned versions | A later version changes its licence | Versions pinned; licence checks for `editor-ui/` belong to P11 |

## 8. Review Focus

The inputs most likely to break a prototype; reviewers and tests look here first.

- **Front matter boundary:** a `---` thematic break in the body; delete at the first body
  character; select-all and type; paste at the start; undo past the first edit.
- **Whitespace:** trailing spaces that are hard breaks; several blank lines; tabs; U+00A0
  (`contenteditable` likes to create it); no final newline; CRLF.
- **Unicode:** `e` + U+0301 next to `é` (NFC normalization); an emoji; `—` and `·` in callout
  titles and beat headings.
- **Markdown with more than one spelling:** `*` and `_`, `-` and `*` lists, `1. 1. 1.`, setext
  headings, escapes, HTML comments, hard-wrapped paragraphs. Stock serializers rewrite these.
- **Links:** a chip next to punctuation, at line start and end, inside a callout title or a beat
  attribute; a dangling id; `[[` then Escape; moving the caret across a chip and deleting it.
- **Callouts:** lazy continuation lines; empty `>` lines; a caption `— ` next to `-- `; a voice
  block directly after beat attributes.
- **Beats:** extra spaces after `::`; an unknown attribute; a beat heading without `—`; a beat
  without attributes.
- **Live preview edges:** the caret entering and leaving a revealed construct (chip, callout
  title, beat heading) while composing with an IME or extending a selection.
- **Input:** IME composition and dead keys next to a chip; pasting rich HTML; undo after save.
- **The stress file:** edits at its end.

## 9. Rulings

Jann chose: browser and Playwright, no Tauri shell; a full Write-mode spike; stock, then best
effort for a WYSIWYG prototype; one build PR that waits for him; Write mode as a live preview.

The calls made in this spec, open to change in its review:

- Ruling: criterion weights and the 0–3 scale (§4.3) — byte stability and the rendered constructs
  are why P12 needs a prose editor; app integration carries CE-34.2 and CE-34.3; long documents
  are effort, not risk — cost if wrong: close candidates swap places.
- Ruling: knockouts on byte stability, front matter and licence — they protect binding constraints
  (CE-34.1, G5, G12) — cost if wrong: a fixable candidate is dropped early.
- Ruling: span level is the bar for byte stability, and the measured rubric of §4.4 is fixed
  before the research — G5 says any edit leaves untouched bytes identical; fixing the rubric first
  guards against bias — cost if wrong: a block-level editor scores lower than Jann would rate it.
- Ruling: best effort for a WYSIWYG prototype aims at span level (change steps mapped onto the
  source), not only block level — block level fails G5 by construction, so stopping there would
  not show what byte stability costs — cost if wrong: extra work in one prototype.
- Ruling: in Write mode the front matter may be split off by the adapter; criterion 2 is decided
  by Source mode — the side panel owns the front matter (`01-main.html`), so splitting is what P12
  would do — cost if wrong: a protected range inside the document goes untested.
- Ruling: CE-34.2, CE-34.3 and the one keymap are judged under criterion 8, weight 2 — CE-06.1
  names no criterion for them, and they are binding for P12 — cost if wrong: integration weighs
  too much.
- Ruling: one implementation task and one fix round per feature and prototype — the same budget
  keeps the measured scores fair — cost if wrong: a feature that needed one more round scores low.
- Ruling: a knockout by measurement starts no third prototype; both knocked out means stop and
  ask Jann — CE-06.2 asks for the top two only — cost if wrong: the alternative in the decision
  record rests on paper evidence.
- Ruling: P10 builds its own corpus from the data-model examples and synthetic records — no real
  records exist yet — cost if wrong: P12 re-runs the byte tests on real records.
- Ruling: long-document sizes and targets (§4.8) — larger than any expected record — cost if
  wrong: performance weighs too much or too little.
- Ruling: one evidence check instead of the final review loop (§4.10) — the theos-phase skill
  applies only steps 1, 2 and 5 to P10, and the result waits for Jann anyway — cost if wrong: an
  unsupported claim reaches Jann.
- Ruling: document checks as the tagged tests for CE-06.1 and CE-06.3 — G15 needs a tagged test
  per AC, and these ACs are documents — cost if wrong: the tests check completeness, not truth;
  the evidence check covers truth.
- Ruling: the decision record is `0001` — it is the first numbered record — cost if wrong: a
  rename.
