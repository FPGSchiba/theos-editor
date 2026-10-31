# P10 · Prose editor research — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to
> implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Choose the prose editor component for P12 with evidence: a comparison of five
candidates, prototypes of the top two that load and save a corpus, a byte report, and a decision
record Jann accepts or rejects.

**Architecture:** A standalone Vite + Svelte 5 app in `research/P10-prose-editor/app/` hosts every
prototype behind one `ProseAdapter` interface. Playwright drives it in Chromium and WebKit, feeds
it the corpus bytes from disk, compares the saved bytes with a source-level oracle and writes the
reports. Paper research (dossiers, comparison) and the decision record are Markdown, guarded by
document checks in Vitest.

**Tech Stack:** Node ≥ 24 (type stripping for scripts), pnpm through corepack, Vite, Svelte 5,
TypeScript strict, Vitest, Playwright (Chromium, WebKit), `diff` (jsdiff). Prototype libraries
are added by the prototype tasks.

**Spec:** `docs/specs/phases/P10-prose-editor.md` (approved). Read it with this plan; where the
two seem to differ, the spec wins.

## Global Constraints

Umbrella global constraints, verbatim (`docs/specs/00-umbrella.md` §2):

- **G1** Repositories: `theos-editor` (this repo) holds the editor. `tower-of-theos` holds the
  content project and the map toolset. `Theos-Engine` is never a dependency, except behind the
  cargo feature `engine`, which no phase here builds.
- **G2** Platforms: Windows 11 x64 (primary), Linux x64, and macOS 13 or later on Apple silicon
  and Intel (universal builds). Every PR is checked on Windows and Linux; macOS is tested weekly
  and before every release (`docs/gate.md` §1).
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

**CE-06 Choose the prose editor component**
- CE-06.1 Comparison of CodeMirror 6, ProseMirror/Tiptap, Milkdown, Lexical and Monaco on:
  byte-stable Markdown, front matter untouched, inline link chips, callouts with their own look,
  beat headings with attribute lines, spell check and IME, long documents, Svelte 5 integration,
  licence, maintenance. Sources cited.
- CE-06.2 Prototypes of the top two load and save the corpus; saved bytes are compared and
  reported.
- CE-06.3 A recommendation Jann can accept or reject.

Phase rules from the spec, one line each:

- Everything except the decision record lives in `research/P10-prose-editor/`; nothing there is imported by a later phase.
- `app/` is a standalone pnpm project (own lockfile, not in any workspace); every dependency pinned exactly (`pnpm add -E`); pnpm through corepack in `packageManager`; `"engines": { "node": ">=24" }`.
- Styles and fonts are imported from `design/theos-ui/` by relative path, never copied; `design/` is never edited.
- Corpus files: UTF-8, LF (except the one CRLF file), each with the front-matter comment line `# synthetic fixture (P10), not canon`; no lore beyond the data model and the reference screens.
- Tests need no network once dependencies and Playwright browsers are installed (CONTRIBUTING §4).
- Byte stability is judged at span level (spec §4.3, criterion 1); the measured rubric of spec §4.4 is fixed and may not be adjusted after results exist.
- Budget per feature and prototype: one implementation task and one fix round (spec §4.4). What is still missing after that is recorded, not chased.
- Commits: Conventional Commits, scope `ui`; type `chore` for research code, `test` for corpus and suites, `docs` for Markdown results; footers `Refs: <ACs>`, `Phase: P10`, `Claude-Session: <session id>`, `Co-Authored-By: Claude …`.
- Every test that proves an AC carries `// AC: <id>` directly above it.
- Never touch protected paths except `docs/decisions/0001-prose-editor.md` and the row in `docs/decisions/README.md` (Task 19).

## Review Focus

Inputs no suite would hit by accident, each pinned to a test:

1. Backspace at the first body character must leave the front matter untouched → manifest edit
   `body-start-backspace` on the S-004 record (Task 2), run by the round-trip suite (Task 6).
2. `[[` then Escape keeps the typed text and closes the popover → feature check
   `F-link-popover-escape` (Task 7).
3. Typing at the end of a file without a final newline must not add one → manifest edit
   `eof-insert` on the no-final-newline record (Task 2).
4. Undo after save must not reach past the save → integration check `N-history` (Task 7).
5. A `---` thematic break in the body is body, not the end of the front matter → unit test in
   Task 1 and manifest edit `after-thematic-break` (Task 2).

---

## Conventions for every task

- Working directory `research/P10-prose-editor/app/` unless a path says otherwise.
- `P10_VARIANTS=<a,b>` limits the browser suites to those variants (default: all registered).
  `P10_STRICT=1` makes a byte mismatch or a failed feature check fail its test; without it the
  suites record the result and pass, because a mismatch is a measurement, not a test failure.
  A harness error (exception, timeout) is recorded as `error` and fails only under `P10_STRICT=1`.
  `P10_REPORT=1` lets the global teardown overwrite `../report/`.
- Browser test titles carry a feature tag: `@base`, `@links`, `@callouts`, `@beats`, `@input`,
  `@integration`, `@long`. Run one feature for one variant:
  `P10_STRICT=1 P10_VARIANTS=<v> pnpm exec playwright test --grep @links`.
- Offsets are UTF-16 code-unit offsets into the LF form of the file (JS string indices).
- Commit after every task, message per the Global Constraints, e.g.
  `test(ui): add the P10 corpus and its manifest (P10)` with `Refs: CE-06.2` and `Phase: P10`.

## File map

```text
research/P10-prose-editor/
  .gitignore                    generated/, app/test-results/, app/playwright-report/, app/dist/
  README.md                     Task 19
  comparison.md                 Task 4, final in Task 18
  dossiers/<candidate>.md       Task 3 (codemirror-6, prosemirror-tiptap, milkdown, lexical, monaco, webviews)
  corpus/records/**.md          Task 2 (project-like paths, e.g. records/story/scripts/S-004.md)
  corpus/manifest.json          Task 2
  corpus/generate-long.ts       Task 2 → corpus/generated/{typical,large,stress}.md (ignored)
  report/*.json, report/*.md    written by Task 17's run
  app/
    package.json, pnpm-lock.yaml, tsconfig.json, vite.config.ts, svelte.config.js,
    playwright.config.ts, index.html                                          Task 1, Task 5
    src/harness/types.ts        shared types (below)                          Task 1
    src/harness/frontmatter.ts  splitFrontMatter                              Task 1
    src/harness/anchor.ts       resolveAnchor                                 Task 1
    src/harness/oracle.ts       expectedAfter, expectedAfterExternalUndo, toLf Task 1
    src/harness/compare.ts      compareBytes                                  Task 1
    src/harness/manifest.ts     Manifest types, readManifest (Node)           Task 2
    src/harness/{registry,bridge,mock-index,places,keymap,findings}.ts, styles.css   Task 5
    src/components/{LinkChip,LinkPopover,VoiceHead,NoteHead,BeatRow}.svelte          Task 5
    src/adapters/baseline/      plain <textarea>, validates the harness       Task 5
    src/adapters/<variant>/     prototypes                                    Tasks 8–16
    scripts/licences.ts         Task 17
    tests/docs/*.test.ts        document checks (Vitest)                      Tasks 2, 4, 6, 18, 19
    tests/e2e/*.spec.ts, helpers.ts, drivers.ts, record.ts, write-reports.ts  Tasks 5–7
```

## Shared types (`src/harness/types.ts`, Task 1)

These names are used by every later task.

```ts
export type Kind = 'tower' | 'band' | 'floor' | 'location' | 'people' | 'faction' | 'npc'
  | 'thing' | 'topic' | 'truth' | 'belief' | 'clue' | 'equation' | 'script' | 'dialogue'
  | 'template' | 'concept' | 'concept-version' | 'map' | 'map-version';
export type BeatKey = 'at' | 'with' | 'says' | 'reveals' | 'when';
export const BEAT_KEY_ORDER: readonly BeatKey[] = ['at', 'with', 'says', 'reveals', 'when'];
export type LinkPlace = { in: 'prose' } | { in: 'beat'; key: BeatKey };
export interface LinkInfo { id: string; kind: Kind; title: string }
export interface Finding { rule: string; severity: 'error' | 'warn' | 'info'; message: string;
  at: { line: number } | { beat: number } }
export type Mode = 'write' | 'source';
export interface Anchor { find: string; occurrence?: number /* 1-based, default 1 */;
  offset?: number /* added to the match start, default 0 */ }
export interface TextChange { from: number; to: number; insert: string }
export type ScriptedEdit = { id: string } & (
  | { kind: 'insert'; at: Anchor; text: string }
  | { kind: 'delete'; at: Anchor; length: number; backward?: boolean }
  | { kind: 'beat-param'; beat: number; key: BeatKey; value: string }
  | { kind: 'note-state'; note: string; state: string }
  | { kind: 'link-insert'; at: Anchor; query: string; pick: string }
  | { kind: 'link-delete'; link: Anchor /* points at the link's "[[" */ }
  | { kind: 'select-all-type'; text: string }
  | { kind: 'paste'; at: Anchor; plain: string; html?: string }
  | { kind: 'external'; local: { at: Anchor; text: string }; external: { at: Anchor; text: string } }
  | { kind: 'undo-all'; edits: Array<{ id: string } & ({ kind: 'insert'; at: Anchor; text: string }
      | { kind: 'delete'; at: Anchor; length: number; backward?: boolean })> });
export interface EditorContext {
  resolve(id: string): LinkInfo | undefined;
  allowedKinds(place: LinkPlace): Kind[];
  search(query: string, place: LinkPlace): LinkInfo[];
  findings: Finding[];
  developerWords: string[];
  keymap: Keymap;
  save(): void;
}
export interface Keymap { save: string; undo: string; redo: string; selectAll: string } // 'Mod-s' etc.
export interface ProseAdapter {
  mount(target: HTMLElement, text: string, ctx: EditorContext): void;
  text(): string;
  apply(edit: ScriptedEdit): void;          // through the editor's own commands
  setMode(mode: Mode): void;
  focus(): void;
  caretTo(offset: number): void;
  caret(): number;
  applyExternal(changes: TextChange[]): void; // minimal edit, keeps caret and local undo
  undo(): void;
  canUndo(): boolean;
  markSaved(): void;                         // clears the field undo history (CE-34.3)
  updateContext(ctx: EditorContext): void;   // index or findings changed; no re-mount
  lastSaveLevel?(): 'span' | 'block';        // best-effort variants only
  destroy(): void;
}
export interface VariantInfo { name: string /* e.g. 'codemirror', 'tiptap:stock' */;
  component: string; version: string; wysiwyg: boolean; column?: 'stock' | 'best-effort';
  baseline?: boolean; packages: string[] /* direct npm dependencies of this variant */ }
export interface Comparison { identical: boolean; firstDiff?: { line: number; column: number };
  extraBytes: number; withinBlock?: boolean; hunk?: string }
```

## DOM contract of the shared components (Task 5)

Both prototypes render through the same Svelte components, so the suites query one contract.

| Element | Contract |
|---|---|
| Link chip | `.tu-link[data-link-id=<id>]`, `.is-broken` when unresolved, `spellcheck="false"`, contains `.tu-kind[data-kind=<kind>]` and `.tu-link__title` (title or shown text) |
| `[[` popover | `.tu-pop[data-link-popover]`; rows `.tu-pop__row[data-link-option=<id>][data-kind=<kind>]`; Enter or click picks; Escape closes |
| Voice block | `[data-callout="voice"]` with class `tu-voice`; head `.tu-voice__head` (speaker title, beliefs, tier); caption `.tu-voice__by` |
| Design callout, `Design note:` line | `[data-callout="design"]`; the line gets class `tu-note` |
| Note callout | `[data-callout="note"][data-note=<n>]` with `[data-note-state=<open\|dropped\|resolved>]` |
| Developer word | `[data-dev-word]` around the word, only inside voice blocks (look: harness CSS, dotted underline in `sig-reject` plus `title="developer word"`) |
| Beat heading | `[data-beat=<n>]` showing `<n> · <name>` |
| Beat row | `[data-beat-row=<n>]` with `.tu-beat`; params `[data-beat-param=<key>]`; add button `[data-beat-add]`, key choice `[data-beat-add-key=<key>]`, value field `[data-beat-input]` (Enter commits); finding `[data-beat-finding][data-finding-rule=<rule>]`; the row has `spellcheck="false"` |

Chips are atomic: the caret steps over a chip in one press and Backspace right after a chip
removes the whole link; a link shows raw only while it is typed (`04-script.html`). Every other
construct (headings, emphasis, callout titles, beat headings, attribute lines) shows raw while
the caret is on its line (live preview).

---

### Task 1: Project scaffold and harness core

**ACs:** none directly; the oracle and comparison back CE-06.2. **Model:** mid-tier.

**Files:**
- Create: `research/P10-prose-editor/.gitignore`; `app/package.json`, `app/tsconfig.json`,
  `app/vite.config.ts` (Vitest config inside), `app/svelte.config.js`
- Create: `app/src/harness/{types,frontmatter,anchor,oracle,compare}.ts`
- Test: `app/src/harness/{frontmatter,anchor,oracle,compare}.test.ts`

**Interfaces:**
- Produces: the shared types above; `splitFrontMatter(text: string): { frontMatter: string; body: string }`;
  `resolveAnchor(text: string, anchor: Anchor): number`; `toLf(text: string): string`;
  `expectedAfter(original: string, edit: ScriptedEdit): string`;
  `expectedAfterExternalUndo(original: string, edit: ScriptedEdit & { kind: 'external' }): string`;
  `compareBytes(saved: string, expected: string, editOffset?: number): Comparison`.

- [ ] **Step 1: Scaffold.** `corepack enable`, `corepack use pnpm@latest` in `app/`, then
  `pnpm add -E -D svelte vite @sveltejs/vite-plugin-svelte typescript vitest @playwright/test diff`.
  `package.json`: `"private": true`, `"type": "module"`, `"engines": {"node": ">=24"}`, scripts
  `dev` (`vite --port 5179 --strictPort`), `test:unit` (`vitest run`), `test:e2e`
  (`playwright test`), `test` (`pnpm test:unit && pnpm test:e2e`). tsconfig: `strict`,
  `noUncheckedIndexedAccess`, `verbatimModuleSyntax`, `allowImportingTsExtensions`, `noEmit`,
  `moduleResolution: "bundler"`. Vite: Svelte plugin, `server.fs.allow` = the repo root
  (`../../..`), Vitest `include: ['src/**/*.test.ts', 'tests/docs/**/*.test.ts']`, environment node.
- [ ] **Step 2: Write the failing tests.**
  - `splitFrontMatter keeps both fence lines and the newline`: `'---\nid: x\n---\nBody\n'` →
    `{ frontMatter: '---\nid: x\n---\n', body: 'Body\n' }`.
  - `splitFrontMatter treats a thematic break in the body as body` (Review Focus 5):
    `'---\nid: x\n---\nA\n\n---\n\nB\n'` → body `'A\n\n---\n\nB\n'`.
  - `splitFrontMatter without front matter`: `'Text'` → `{ frontMatter: '', body: 'Text' }`;
    `splitFrontMatter on CRLF` keeps the `\r\n` bytes in both parts.
  - `resolveAnchor finds the nth occurrence plus offset`: `('a b a', {find:'a', occurrence:2, offset:1})` → `5`;
    `resolveAnchor throws when the text is missing`.
  - `expectedAfter insert` and `delete` (forward); `delete backward clips at the body start`:
    `'---\nid: x\n---\nBody'`, backward delete of 3 at offset of `Body` → unchanged.
  - `beat-param changes the value and keeps the spaces after ::`: `'## Beat 1 — A\nat::  [[x]]\n'`,
    `{beat:1, key:'at', value:'[[y]]'}` → `'## Beat 1 — A\nat::  [[y]]\n'`.
  - `beat-param adds a key after the last preceding key in BEAT_KEY_ORDER`: beat with `at::` and
    `when::`, add `with` → new line `with:: <value>` between them.
  - `beat-param adds after the heading when the beat has no attributes`.
  - `note-state replaces the last segment`: `'> [!note] n3 · [[f04-terrain-v2]] · open\n'` →
    `dropped` gives `'> [!note] n3 · [[f04-terrain-v2]] · dropped\n'`; and
    `'> [!note] n4 · resolved by [[fates-shrine-v5]]\n'` → `open` gives `'> [!note] n4 · open\n'`.
  - `link-insert inserts [[pick]] at the anchor`; `link-delete removes through the closing brackets`:
    `'see [[a|b]] now'` → `'see  now'`.
  - `select-all-type keeps the front matter and replaces the body`.
  - `paste inserts the plain text with LF line ends` (`'a\r\nb'` → `'a\nb'`; `html` ignored).
  - `external applies both inserts`; `expectedAfterExternalUndo keeps only the external insert`.
  - `undo-all expects the original`.
  - `compareBytes reports identical`; `reports the first differing line and column` (1-based);
    `counts extraBytes in UTF-8`: saved `'xéy'` vs expected `'xéy'` → `identical: false`,
    `extraBytes: 3` (the larger differing middle after common prefix and suffix);
    `withinBlock` is true when the differing range lies in the run of non-blank lines around
    `editOffset`, false otherwise, undefined without `editOffset`;
    `hunk shows whitespace`: spaces as `␣`, tabs as `⇥`, CR as `␍`, U+00A0 as `⍽`.
- [ ] **Step 3: Run** `pnpm test:unit` — expected: all new tests fail (modules missing).
- [ ] **Step 4: Implement** the five modules. Beat lines: heading `^## Beat <n>\b`, attribute
  lines are the lines directly after it matching `^[a-z]+::`. Note title line:
  `^> \[!note\] <note>( · .*)?$`, segments split on `' · '`. `compareBytes` compares
  `TextEncoder` bytes; the hunk comes from `diff`'s `createTwoFilesPatch` on whitespace-mapped text.
- [ ] **Step 5: Run** `pnpm test:unit` — expected: all pass.
- [ ] **Step 6: Commit** (`chore(ui): scaffold the P10 harness core and oracle (P10)`, `Refs: CE-06.2`).

### Task 2: Corpus and manifest

**ACs:** CE-06.2 (`// AC: CE-06.2` on `corpus covers every construct`). **Model:** mid-tier.

**Files:**
- Create: `corpus/records/**.md`, `corpus/manifest.json`, `corpus/generate-long.ts`
- Create: `app/src/harness/manifest.ts`
- Test: `app/tests/docs/corpus.test.ts`

**Interfaces:**
- Consumes: `ScriptedEdit`, `expectedAfter`, `splitFrontMatter` (Task 1).
- Produces: `interface CorpusFile { path: string /* relative to corpus/ */; origin: 'data-model §14' | 'synthetic';
  lineEnds: 'lf' | 'crlf'; covers: string[]; edits: ScriptedEdit[] }`,
  `interface Manifest { files: CorpusFile[] }`, `readManifest(): Manifest`,
  `readCorpusText(file: CorpusFile): string` (the file's bytes as UTF-8, unchanged),
  `export const CONSTRUCTS: readonly string[]` (the tags below);
  `generateLong(targetBytes: number): string` exported by `corpus/generate-long.ts`, which writes
  `corpus/generated/{typical,large,stress}.md` only when run as a script.

- [ ] **Step 1: Write the failing test** `corpus.test.ts`:
  - `corpus covers every construct` (`// AC: CE-06.2`): every tag of `CONSTRUCTS` is claimed by at
    least one file, and each claim is confirmed by that tag's detector (a regex or a small
    function per tag, in the test) on the file's text.
  - `every file has the synthetic marker`: the front matter contains the line
    `# synthetic fixture (P10), not canon`.
  - `line ends match the manifest`: `lf` files contain no `\r`; the `crlf` file has only `\r\n`.
  - `every edit kind is used` (all ten `ScriptedEdit` kinds) and `every edit resolves`:
    `expectedAfter` runs without throwing on the LF form of its file.
  - `the spec's edits exist` (spec §4.6): ids `word-insert`, `word-delete` (mid-paragraph),
    `voice-end-sentence`, `note-state-change`, `beat-param-add`, `beat-param-change`,
    `link-insert-popover`, `link-chip-delete`, `select-all-type`, `paste-crlf`, `paste-html`,
    `external-while-dirty`, `undo-all`, each of the matching kind.
  - `Review Focus edits exist`: ids `body-start-backspace` (S-004, `delete`, `backward: true`, at
    the first body character), `eof-insert` (insert at the end of the no-final-newline file) and
    `after-thematic-break` (insert in the paragraph after a body `---`).
  - `long documents are deterministic`: `generateLong` called twice per size gives identical
    strings; sizes within ±10 % of 5 000, 250 000 and 2 500 000 bytes.
- [ ] **Step 2: Run** `pnpm test:unit tests/docs/corpus.test.ts` — expected: FAIL.
- [ ] **Step 3: Write the corpus.** The seven examples of data model §14 as separate files
  (marker line inserted right after the opening `---`). Synthetic records (ids with a 9xx number
  or a `synthetic-` slug) so that every tag is covered: `CONSTRUCTS` = the rows of spec §4.2 as
  kebab-case tags, e.g. `fm-comment-line`, `fm-map`, `hard-wrapped-paragraph`, `setext-heading`,
  `emphasis-underscore`, `list-ones`, `gfm-table`, `hr-dashes`, `html-comment`,
  `hard-break-spaces`, `link-alias`, `link-dangling`, `link-in-beat-attribute`, `voice-caption`,
  `callout-lazy-line`, `note-resolved`, `beat-no-attributes`, `beat-extra-spaces`, `e-combining`,
  `nbsp`, `emoji`, `no-final-newline`, `empty-body`, `crlf` (one tag per item of the spec table).
  Prose comes from the data model examples and the reference screens; constructs without such
  text use neutral filler that names the construct, never new names or facts. Edits: at least
  one of every kind, spread over files, each with a unique `id`.
- [ ] **Step 4: Write** `manifest.ts` and `generate-long.ts` (Node built-ins only; run with
  `node corpus/generate-long.ts` from `research/P10-prose-editor/`; repeats a fixed block of
  beats, voice blocks and links until the target size).
- [ ] **Step 5: Run** the test — expected: PASS.
- [ ] **Step 6: Commit** (`test(ui): add the P10 corpus and its manifest (P10)`, `Refs: CE-06.2`).

### Task 3: Candidate dossiers

**ACs:** feeds CE-06.1 (no test). **Model:** mid-tier, six subagents in parallel, each with web
search and fetch.

**Files:** Create `research/P10-prose-editor/dossiers/{codemirror-6,prosemirror-tiptap,milkdown,lexical,monaco,webviews}.md`.

- [ ] **Step 1: One subagent per dossier**, briefed with spec §3 and §4.3 (criteria definitions)
  and this template: title and version researched (npm, with date); one `##` section per criterion
  of spec §4.3 in its order; under each, facts only (no scores, no verdicts), each fact ending in
  a source tag `[D<n>]`; known open issues with links; a `## Sources` list
  `- D<n> · <url> · <title> · read <YYYY-MM-DD>`; a `## Unknowns` list of what the sources do not
  settle. ProseMirror/Tiptap covers both forms. `webviews.md` has two sections, spell check and
  IME, each for WebView2, WKWebView and WebKitGTK as used by Tauri 2, including how spell check
  is switched on where it is off by default.
- [ ] **Step 2: Review each dossier** against its template (every fact sourced, every criterion
  present, dates present).
- [ ] **Step 3: Commit** (`docs(ui): add the P10 candidate dossiers (P10)`, `Refs: CE-06.1`).

### Task 4: Comparison on paper

**ACs:** CE-06.1 (`// AC: CE-06.1` on the tests below). **Model:** most capable (scoring).

**Files:**
- Create: `research/P10-prose-editor/comparison.md`
- Create: `app/tests/docs/markdown.ts` (`parseTable(md: string, heading: string): string[][]`,
  `parseSources(md: string): Map<string, { ref: string; date: string }>`)
- Test: `app/tests/docs/comparison.test.ts`

**Interfaces:**
- Consumes: the dossiers (Task 3).
- Produces: the fixed format below (Task 18 and Task 19 parse it).

`comparison.md` format:

```markdown
Stage: paper
Top two: <candidate>, <candidate>

## Matrix
| Criterion | Weight | CodeMirror 6 | ProseMirror/Tiptap | Milkdown | Lexical | Monaco |
|---|---|---|---|---|---|---|
| Byte-stable Markdown | 3 | 3 [S1, S4] | … |
…
| Licence | – | pass [S9] | … |
| Weighted total | | 44 | … | knockout | … |

## Sources
- S1 · <url or repo path> · <title> · read 2026-11-02
```

Cells: `<score> [S…]` or, from Task 18, `<paper>→<measured> [S…]`; scores 0–3, licence
`pass`/`fail`. Rows in spec §4.3 order with weights 3, 2, 2, 2, 2, 2, 1, 2, –, 2. A candidate
whose effective score on criterion 1 or 2 is 0, or whose licence is `fail`, has `knockout` as
its total. One `##` section per candidate argues each score with `S` references (dossier
sources are copied into the S list). A `## Top two` section applies spec §4.4.

- [ ] **Step 1: Write the failing tests** (`// AC: CE-06.1` above each):
  - `every criterion row exists once with its weight`.
  - `every cell has a valid score and at least one source` (all 50 cells).
  - `every cited source is in the source list with a date`.
  - `weighted totals match the cells` (Σ weight × effective score; effective = measured if
    present, else paper; `knockout` where spec §4.3 says so).
  - `the top two follow the rule of spec §4.4 on paper scores` (highest paper totals without
    knockout, tie to the higher byte-stability score; if fewer than two survive, the best
    knocked-out candidate fills the gap).
- [ ] **Step 2: Run** `pnpm test:unit tests/docs/comparison.test.ts` — expected: FAIL.
- [ ] **Step 3: Write `comparison.md`** from the dossiers with the scale of spec §4.3. A claim
  without a source is not scored.
- [ ] **Step 4: Run** — expected: PASS.
- [ ] **Step 5: Commit** (`docs(ui): compare the prose editor candidates on paper (P10)`, `Refs: CE-06.1`).
- [ ] **Step 6 (controller): Update this plan.** Replace "prototype A/B" in Tasks 8–15 with the
  two names and their variant slugs; if a prototype is WYSIWYG, its base task builds the
  `<slug>:stock` variant and Task 16 is kept for it; otherwise Task 16 is dropped. Commit the plan.

### Task 5: Harness app and baseline

**ACs:** none directly (enables CE-06.2). **Model:** mid-tier.

**Files:**
- Create: `app/index.html`, `app/src/main.ts`, `app/src/App.svelte` (file picker over
  `corpus/records`, variant picker, Write/Source switch)
- Create: `app/src/harness/registry.ts` (`variants: VariantInfo[]`, `createAdapter(name: string): ProseAdapter`),
  `bridge.ts`, `mock-index.ts`, `places.ts`, `keymap.ts`, `findings.ts`, `styles.css`
- Create: `app/src/components/{LinkChip,LinkPopover,VoiceHead,NoteHead,BeatRow}.svelte`
- Create: `app/src/adapters/baseline/index.ts`
- Create: `app/playwright.config.ts`, `app/tests/e2e/{helpers,record,write-reports}.ts`
- Test: `app/tests/e2e/smoke.spec.ts`

**Interfaces:**
- Consumes: shared types (Task 1), `readManifest`, `readCorpusText` (Task 2).
- Produces:
  - `window.__harness` (bridge): `variants(): VariantInfo[]`, `load(variant: string, text: string, fileId: string): Promise<void>`
    (destroys the previous adapter, mounts a fresh one), `text()`, `apply(edit)`, `setMode(m)`,
    `focus()`, `caretTo(o)`, `caret()`, `applyExternal(c)`, `undo()`, `canUndo()`, `markSaved()`,
    `saveCount(): number`, `updateIndex(patch: LinkInfo[]): void` (calls `updateContext`),
    `lastSaveLevel(): string | undefined`.
  - `mock-index.ts`: an index of every id and title in the corpus front matter plus the extra
    ids the screens use; `places.ts`: `allowedKinds` per data model §9.4 (`at` → location,
    floor; `with` → npc, faction, people; `says` → belief; `reveals` → clue; `when` → none;
    prose → every kind); `findings.ts`: `findingsFor(fileId: string): Finding[]`, where
    `fileId` is the manifest path (one `never-confirm` error on beat 2 of
    `records/story/scripts/S-004.md`); `keymap.ts`:
    `keymap: Keymap = { save: 'Mod-s', undo: 'Mod-z', redo: 'Mod-Shift-z', selectAll: 'Mod-a' }`;
    developer words `['T-012', 'never-confirm', 'tier', 'ledger', 'canon']`.
  - The shared components with the DOM contract above; `styles.css` imports
    `design/theos-ui/tokens.css` and `components/bundle.css` by relative path and adds the
    harness-only looks (note callout, developer word).
  - Test helpers: `variantsUnderTest(): VariantInfo[]` (honours `P10_VARIANTS`, excludes
    `baseline` unless named), `loadFile(page, variant, file)`, `edit(page, e, driver)`,
    `record(kind: 'round-trip' | 'features' | 'input' | 'integration' | 'long-docs', row: object)`
    (appends a JSON line to `test-results/raw/<kind>-<worker>.jsonl`), `strict(): boolean`.
  - `write-reports.ts` (global teardown, only with `P10_REPORT=1`): per kind writes
    `../report/<kind>.json` and `.md`, header with commit (`git rev-parse HEAD`), OS, browser
    versions and date.
  - Playwright projects `chromium` and `webkit`; `webServer` = `pnpm dev`.

- [ ] **Step 1: Write the failing test** `smoke.spec.ts` (`@base`): with the `baseline` variant,
  every corpus file loads and `text()` equals its LF form; `setMode('source')` keeps the text;
  `variants()` lists `baseline` with `baseline: true`.
- [ ] **Step 2: Run** `P10_VARIANTS=baseline pnpm exec playwright test smoke` — expected: FAIL.
- [ ] **Step 3: Implement** the app, bridge, registry, mock context, components and the baseline
  adapter (a `<textarea>`; caret through `setSelectionRange`; undo through its own stack).
- [ ] **Step 4: Run** — expected: PASS in both projects.
- [ ] **Step 5: Commit** (`chore(ui): add the P10 harness app and baseline adapter (P10)`, `Refs: CE-06.2`).

### Task 6: Round-trip suite and report

**ACs:** CE-06.2 (`// AC: CE-06.2` on the round-trip tests and the coverage check). **Model:** mid-tier.

**Files:**
- Create: `app/tests/e2e/round-trip.spec.ts`, `app/tests/e2e/drivers.ts`
- Modify: `app/tests/e2e/write-reports.ts` (round-trip Markdown section)
- Test: `app/tests/docs/report-coverage.test.ts`

**Interfaces:**
- Consumes: bridge and helpers (Task 5), oracle and `compareBytes` (Task 1), manifest (Task 2).
- Produces: round-trip rows `{ variant, engine, file, check: 'unedited' | <edit id>,
  driver: 'load' | 'api' | 'input', identical, firstDiff?, extraBytes, withinBlock?,
  level: 'span' | 'block' | 'none', adapterLevel?, hunk?, error? }` where `level` is `span` when
  identical, `block` when the mismatch stays inside the edited block, else `none`.

- [ ] **Step 1: Write the failing tests.**
  - `round-trip.spec.ts` (`// AC: CE-06.2`): per variant under test and file,
    `@base unedited · <variant> · <file>` (expected: the LF form); per edit and driver
    `@<tag> <edit id> via <driver> · <variant> · <file>` with the tag from the edit kind
    (`insert`, `delete`, `paste`, `select-all-type`, `external`, `undo-all` → `@base`;
    `link-*` → `@links`; `note-state` → `@callouts`; `beat-param` → `@beats`). `external` also
    checks `expectedAfterExternalUndo` after one undo. Each test records its row; it fails only
    under `P10_STRICT=1` (mismatch or error).
  - `drivers.ts`: `api` calls `apply`; `input` uses real input after `caretTo`: `insert` types;
    `delete` presses Delete (Backspace when `backward`) `length` times; `beat-param` clicks
    `[data-beat-param]` or `[data-beat-add]` + `[data-beat-add-key]`, fills `[data-beat-input]`,
    presses Enter; `note-state` puts the caret on the state segment, Shift+End, types;
    `link-insert` types `[[` + query and clicks `[data-link-option=<pick>]`; `link-delete` puts
    the caret after `]]` and presses Backspace once; `select-all-type` presses
    `ControlOrMeta+a` and types; `paste` dispatches a `paste` ClipboardEvent with `text/plain`
    (and `text/html`); `external` does its local part by input and the external part through
    `applyExternal`; `undo-all` presses `ControlOrMeta+z` until the text stops changing (at most 50).
  - `report-coverage.test.ts` (`// AC: CE-06.2`): `report/round-trip.json` has a row for every
    non-baseline variant × engine × file × (unedited + every edit × both drivers), and both
    columns for every WYSIWYG component.
- [ ] **Step 2: Run** `P10_STRICT=1 P10_VARIANTS=baseline pnpm exec playwright test round-trip --grep @base`
  — expected before Step 3: FAIL (spec missing). The coverage check fails until Task 17.
- [ ] **Step 3: Implement** the spec, drivers and the round-trip report section (counts per
  variant × engine: unedited identical N/M, edits matching N/M; then one entry per mismatch
  with file, edit, driver, line:column, extra bytes, level and the hunk).
- [ ] **Step 4: Run** the Step 2 command — expected: PASS for every `@base` test with `api` and
  `input` on `baseline` (a textarea keeps bytes; this validates the harness).
- [ ] **Step 5: Commit** (`test(ui): add the P10 round-trip suite and report (P10)`, `Refs: CE-06.2`).

### Task 7: Feature, input, integration and long-document suites

**ACs:** evidence for CE-06.1 criteria 3–8 (no tag). **Model:** mid-tier.

**Files:**
- Create: `app/tests/e2e/{features,input,integration,long-docs}.spec.ts`
- Modify: `app/tests/e2e/write-reports.ts` (one section per kind)

**Interfaces:**
- Consumes: bridge, helpers, DOM contract (Task 5); generated long files (Task 2).
- Produces: rows `{ variant, engine, check, ok, detail?, error? }` (features, input,
  integration) and `{ variant, engine, size, loadMs, typing: { start, middle, end: { median, p95 } }, scrollMs, heapBytes? }` (long-docs).

- [ ] **Step 1: Write the checks.** Each records `ok` and fails only under `P10_STRICT=1`.
  - `features.spec.ts`: `@links` `F-link-chip` (`[[mother-ilse]]` → chip with kind `npc`, title
    "Mother Ilse"), `F-link-alias`, `F-link-broken`, `F-link-atomic` (ArrowRight before `[[`
    moves the caret behind `]]`), `F-link-popover-prose` (options of several kinds),
    `F-link-popover-kind-filter` (in an `at::` line only `location`/`floor` options),
    `F-link-popover-escape` (Review Focus 2: `[[son` + Escape → no popover, text equals the
    oracle insert of `[[son`); `@callouts` `F-voice` (head names the speaker, caption shown),
    `F-design`, `F-note` (state attribute), `F-dev-word` (wrapped inside voice text, nowhere
    else); `@beats` `F-beat-heading` (`1 · Arrival`), `F-beat-row` (`at` param holds the
    `fates-shrine` chip), `F-beat-finding` (rule `never-confirm` on beat 2); `@base`
    `F-live-preview` (caret on the beat heading line shows `## Beat 1 — Arrival` raw; elsewhere
    rendered), `F-source-mode` (front matter visible, no chips, typing into the front matter
    leaves the text unchanged), `F-frontmatter-hidden` (Write mode shows no `id: S-004`).
  - `input.spec.ts` (`@input`): `I-spellcheck-prose` (a prose word's element is
    `isContentEditable` and `.spellcheck === true`), `I-spellcheck-off` (chips and beat rows
    `.spellcheck === false`); Chromium only through a CDP session: `I-ime-ja-<place>` (compose
    `にほんご`, commit `日本語`), `I-deadkey-circ-<place>` (`^`→`ê`) and `I-deadkey-uml-<place>`
    (`¨`→`ü`), with `<place>` = `prose`, `chip` (right after a chip) and `voice` (inside a voice
    block), bytes against the oracle; WebKit: `I-insert-<place>` with `keyboard.insertText` at the
    same places and one `I-ime-webkit` row `ok: false,
    detail: 'gap: no composition input in Playwright WebKit'`.
  - `integration.spec.ts` (`@integration`): `N-external-caret` (caret shifts only when the
    change is before it), `N-history` (Review Focus 4: edit, `markSaved()`, `canUndo()` false,
    `ControlOrMeta+z` changes nothing, a new edit makes `canUndo()` true), `N-keymap-save`
    (`ControlOrMeta+s` raises `saveCount()` by 1), `N-keymap-undo-redo`, `N-remount` (20 loads:
    chip count equals the current file's links, no console errors), `N-reactive` (`updateIndex`
    with a new title updates the chip; the editor's root node is the same object).
  - `long-docs.spec.ts` (`@long`, serial): per size, load to first paint (double
    `requestAnimationFrame`), 200 keystrokes at start, middle and end (latency from the
    keydown's `timeStamp` to the next animation frame after the editor's DOM changed), scroll
    to the end, heap via `performance.memory` in Chromium.
- [ ] **Step 2: Run** `P10_VARIANTS=baseline pnpm exec playwright test features input integration long-docs`
  — expected: the suites run; `baseline` records `ok: false` for every rendered feature (it
  has none) and real numbers for the long documents. That proves the checks can fail.
- [ ] **Step 3: Commit** (`test(ui): add the P10 feature, input and long-document suites (P10)`, `Refs: CE-06.1`).

### Tasks 8–11: Prototype A (named after Task 4)

**ACs:** CE-06.2 (through the suites). **Model:** mid-tier; most capable if the first attempt
fails. Budget: each task is one implementation attempt plus one fix round (spec §4.4); then
record what is missing in `src/adapters/<a>/NOTES.md` and stop.

Each task follows the same steps: (1) run the task's suites with `P10_STRICT=1
P10_VARIANTS=<a>` — expected: FAIL; (2) implement in `src/adapters/<a>/` (small files, one per
concern); (3) run again — expected: PASS, or the remaining failures listed in `NOTES.md` with
what was tried; (4) commit (`chore(ui): <feature> in the <a> prototype (P10)`, `Refs: CE-06.2`).

| Task | Feature | Suites (`--grep`) | Scope |
|---|---|---|---|
| 8 · A1 | Base | `@base\|@integration\|@input` (except the input checks ending in `-chip`) | `VariantInfo` entry in the registry; every `ProseAdapter` method; whole file loaded; front matter hidden and protected in Write mode (protected range or split, spec §4.5), raw and protected in Source mode; live preview for headings and emphasis; keymap from `ctx.keymap`; `Mod-a` selects the body only |
| 9 · A2 | Links | `@links`, and the `@input` checks ending in `-chip` (ids end in `-prose`, `-chip` or `-voice` for where they type) | chips through `LinkChip`, atomic caret and delete, `[[` popover through `LinkPopover` with `ctx.search` and the place (prose or beat key) |
| 10 · A3 | Callouts | `@callouts` | voice, design and note callouts through `VoiceHead`/`NoteHead` with editable text; `Design note:` lines; developer words |
| 11 · A4 | Beats | `@beats` | beat heading, `BeatRow` with edit, "+ add" and finding; raw lines under the caret |

### Tasks 12–15: Prototype B (named after Task 4)

As Tasks 8–11 for `<b>` in `src/adapters/<b>/`.

### Task 16 (only for a WYSIWYG prototype): best-effort column

**ACs:** CE-06.2. **Model:** most capable. Budget as above.

**Files:** Create `src/adapters/<slug>-best-effort/` (reuses the `<slug>` code; registry entry
`<slug>:best-effort`, `column: 'best-effort'`; the base variant becomes `<slug>:stock`).

- [ ] **Step 1: Run** every suite with `P10_STRICT=1 P10_VARIANTS=<slug>:best-effort` — expected: FAIL.
- [ ] **Step 2: Implement** spec §4.5 best effort: untouched blocks keep their source; inside an
  edited block the editor's change steps are mapped onto the source text as span edits; where
  the mapping fails, the block is serialized again; `lastSaveLevel()` reports which happened.
- [ ] **Step 3: Run** — expected: PASS or listed in `NOTES.md`.
- [ ] **Step 4: Commit** (`chore(ui): best-effort byte preservation for <slug> (P10)`, `Refs: CE-06.2`).

### Task 17: Measurement run and licences

**ACs:** CE-06.2 (coverage check passes). **Model:** cheapest.

**Files:** Create `app/scripts/licences.ts`; generated `report/*.json`, `report/*.md`,
`report/licences.md`.

- [ ] **Step 1:** `pnpm exec playwright install chromium webkit`, then
  `node ../corpus/generate-long.ts` (from `app/`, writes into `../corpus/generated/`).
- [ ] **Step 2:** `P10_REPORT=1 pnpm test` on Windows — expected: exit 0; reports for
  round-trip, features, input, integration and long-docs written.
- [ ] **Step 3:** `node scripts/licences.ts` — reads `pnpm licenses list --json` and
  `pnpm list --json --depth Infinity`, writes `report/licences.md` with the licence of every
  package in each variant's dependency closure (from `VariantInfo.packages`) and flags any
  licence outside MIT, ISC, BSD-2-Clause, BSD-3-Clause, Apache-2.0, 0BSD, BlueOak-1.0.0, CC0-1.0.
  The same script writes `report/adapter-size.md`: non-blank lines per file of every
  `src/adapters/<variant>/` folder, grouped by feature (the file names say the concern).
- [ ] **Step 4:** `pnpm test:unit` — expected: PASS, including `report-coverage.test.ts`.
- [ ] **Step 5: Commit** the reports (`docs(ui): record the P10 measurements (P10)`, `Refs: CE-06.2`).

### Task 18: Measured scores

**ACs:** CE-06.1. **Model:** most capable.

**Files:** Modify `research/P10-prose-editor/comparison.md`; Test: `app/tests/docs/comparison.test.ts`.

- [ ] **Step 1: Add the failing test** `at stage final the top two carry measured scores`
  (`// AC: CE-06.1`): `Stage: final`; the two top-two columns have `<paper>→<measured>` on
  criteria 1–8; measured cells cite at least one source whose ref is a `report/` file.
- [ ] **Step 2: Run** — expected: FAIL.
- [ ] **Step 3: Score** with the fixed rules only: spec §4.4 for criteria 1, 3–6 and 8
  (criterion 1 on the worse engine and the best column; `report/adapter-size.md` as evidence for
  2 vs 3), the targets of spec §4.8 for criterion 7, `report/licences.md` for criterion 9;
  criterion 10 keeps its paper score. Explain every change from paper in the candidate sections;
  update the totals. A knockout by measurement follows spec §4.4 (no third prototype; both knocked out →
  stop and tell Jann).
- [ ] **Step 4: Run** `pnpm test:unit` — expected: PASS.
- [ ] **Step 5: Commit** (`docs(ui): score the P10 prototypes from the measurements (P10)`, `Refs: CE-06.1`).

### Task 19: Decision record and research README

**ACs:** CE-06.3 (`// AC: CE-06.3`). **Model:** most capable.

**Files:**
- Create: `docs/decisions/0001-prose-editor.md`; Modify: `docs/decisions/README.md` (the `open` row)
- Create: `research/P10-prose-editor/README.md`
- Test: `app/tests/docs/decision.test.ts`

- [ ] **Step 1: Write the failing tests** (`// AC: CE-06.3`):
  - `the record has every section`: `## Status`, `## Decision`, `## Why`,
    `## Consequences for P12`, `## Alternative`, `## If rejected`; status line `proposed`.
  - `the record recommends exactly one of the top two` (names from `comparison.md`).
  - `every relative link in the record resolves`.
  - `the decision log row names the record`: the README has no `open` row for the prose editor
    and one row linking `0001-prose-editor.md`.
- [ ] **Step 2: Run** — expected: FAIL.
- [ ] **Step 3: Write** the record per spec §4.9, the README row (date of the PR, the component,
  link), and the research README: how to run (`corepack enable`, `pnpm install`,
  `pnpm exec playwright install chromium webkit`, `node ../corpus/generate-long.ts`, `pnpm test`),
  where the results are, a five-line summary, and the manual checklist of spec §4.7.
- [ ] **Step 4: Run** `pnpm test:unit` — expected: PASS.
- [ ] **Step 5: Commit** (`docs(ui): recommend the prose editor component (P10)`, `Refs: CE-06.3`).

## After the tasks (controller)

1. **Evidence check** (spec §4.10): one fresh reviewer on the most capable model gets the spec,
   `comparison.md`, the dossiers, `report/`, the decision record and the adapters; it checks every
   scored claim against its source, every score against spec §4.3–4.4, and the record against the
   reports. One fix subagent gets the complete findings list. No second round.
2. **Verify** (theos-phase step 5): `pnpm test` (without `P10_STRICT`) in `app/` on Windows,
   output read; `pnpm test:unit` again after the fix pass. If P00 is merged: `cargo xtask gate`
   and `cargo xtask ac-trace --phase P10`; otherwise a ruling.
3. **PR** per spec §4.12: status `pr-open`, PR `docs(ui): prose editor research (P10)`, template
   filled with the rulings; no auto-merge.

## Rulings in this plan

- Ruling: a byte mismatch or a missing feature is recorded and does not fail its test unless
  `P10_STRICT=1` — the suites measure, and `pnpm test` must be green to verify the run — cost if
  wrong: a broken harness hides behind green tests; the `baseline` checks in Tasks 5–7 guard it.
- Ruling: chips are atomic and show raw only while typed; every other construct shows raw on the
  caret's line — criterion 3 asks for one-unit caret steps, Jann asked for live preview, and
  `04-script.html` shows a link raw while it is typed — cost if wrong: P12 revisits how a link
  target is edited.
- Ruling: both prototypes render through the same Svelte components — the comparison measures
  the editors, not two sets of widgets — cost if wrong: a component that would need different
  widgets looks harder than it is.
- Ruling: the dossiers are written by six researchers who do not score, and one scorer reads
  them all — separates facts from verdicts against the bias risk of spec §7 — cost if wrong: six
  more subagent runs.
- Ruling: tests for criteria 3–8 carry no AC tag; only the comparison, round-trip, coverage and
  decision checks do — they are the ones that prove CE-06.1–06.3 — cost if wrong: none for the gate.
