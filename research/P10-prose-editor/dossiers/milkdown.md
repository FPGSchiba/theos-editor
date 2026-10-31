# Milkdown — dossier

Researched: @milkdown/kit, @milkdown/core, @milkdown/crepe, @milkdown/preset-commonmark, @milkdown/preset-gfm, @milkdown/transformer, @milkdown/plugin-listener 7.23.0 (npm, read 2026-10-31; 7.23.0 published 2026-10-27) · Researcher: P10 Task 3

Scope note. Milkdown is a family: `@milkdown/kit` (headless re-export of core, ctx, prose, utils, presets, plugins, components) and `@milkdown/crepe` (a ready-made editor built on the kit, with its own UI features). All `@milkdown/*` packages are released together at one version [D1, D12]. Facts below say which of the two they apply to. "Measured" means a run I made on 2026-10-31 in a scratch directory outside the repo (D46 to D53); everything else comes from the cited documents, source files, issues or npm metadata. Every measured run is headless in Node with jsdom: it covers parse, serialize, schema and transaction behaviour, and no caret movement, composition input, spell check, layout or paint [D46]. Corpus files are read from `research/P10-prose-editor/corpus/`; front matter was split off before feeding the body to Milkdown (the adapter model of spec §4.3 criterion 2) [D46].

## Byte-stable Markdown

Document model
- The editor document is a ProseMirror document; the Markdown source text is not kept. Parse path: Markdown string → `remark.parse` (mdast with positions) → `runSync` → per-node `parseMarkdown` runners → ProseMirror nodes. Serialize path: ProseMirror doc → `SerializerState` builds a new mdast tree → `remark.stringify` for the whole tree [D6, D13].
- The serializer's `toString` is `remark.stringify(this.build())`: the whole document is re-serialized on every call; there is no per-block or per-span path [D13]. `getMarkdown()` and the listener plugin's `markdownUpdated` both call the serializer on the whole doc; `markdownUpdated` is debounced 200 ms [D7, D23].
- The remark instance is `unified().use(remarkParse).use(remarkStringify, options)`, options taken from the context slice `remarkStringifyOptionsCtx`, whose default is `{ handlers: remarkHandlers, encode: [] }` [D14].
- The serializer builds mdast nodes from the props each node's `toMarkdown` runner passes (`depth`, `ordered`, `spread`, `marker`, `lang`, `isInline` and similar); the runners read (paragraph, heading, lists, code, emphasis, strong, hardbreak, hr) pass no `position`, so the mdast given to `remark.stringify` has none [D13, D17, D18].

Source positions
- mdast nodes from `remark.parse` carry `position.start.offset` and `position.end.offset`, and the file text is passed to `runSync(tree, markdown)` so remark plugins can read it as `file.value` [D13].
- In the stock packages read, offsets are used in exactly two places: `remarkMarker` (reads the first character of each `strong` and `emphasis` node to store `*` or `_` as a `marker` attribute) and Crepe's LaTeX remark plugin (source slicing of math nodes) [D16, D53].
- Stock node schemas do not store source offsets or raw source text; `code_block` stores only `language`, `list_item` stores `label`, `listType`, `spread`, `bullet_list` stores `spread`, `ordered_list` stores `order`, `spread`, `heading` stores `id`, `level` [D18].
- A custom node schema can declare any `attrs`, and its `toMarkdown` runner can emit an mdast `html` node whose `value` is written verbatim (the preset itself emits `html` nodes with value `<br />` for empty paragraphs) [D17, D10]. Raw `html` blocks and inline html round-trip unchanged in a measured run [D47]. No document or example found that stores per-block source text and re-emits it [D6, D10, D12].

What the stock serializer changed in my measurements (stock `commonmark` + `gfm` presets, default options, kit 7.23.0, remark-stringify 11.0.0, mdast-util-to-markdown 2.2.1) [D46, D47]
- Corpus bodies (front matter split off), 17 files: the 4 files with empty bodies and 2 of 13 non-empty bodies (`synthetic-characters`, `f04-choir-hall`) came back byte-identical; 11 of 13 non-empty bodies differed [D46].
- `[[id]]` and `[[id|text]]` come back as `\[\[id]]` and `\[\[id|text]]`; `> [!voice] …`, `> [!note] …`, `> [!design]` come back as `> \[!voice] …` etc. (stock text handler escapes `[`) [D46, D15].
- Blank lines: a blank line is inserted between a heading and a directly following paragraph (the `key:: value` lines after `## Beat n — name`), and between a paragraph and a directly following blockquote; runs of 2 or more blank lines collapse to one [D46].
- Line ends and file end: CRLF becomes LF; a missing final newline is added [D46].
- Whitespace: trailing spaces at the end of a line or paragraph are removed; a hard break written as two trailing spaces comes back as backslash plus newline; a tab inside a line is kept [D46].
- Headings and rules: setext headings become ATX; closing `##` is removed; `---`, `___`, `- - -` become `***` [D46, D47].
- Lists: bullet markers `-`, `*`, `+` are replaced by `*` (and `-` for a list directly following another bullet list); ordered `1) ` becomes `1. `; a `1. 1. 1.` list becomes `1. 2. 3.` [D46, D47].
- Code: a `~~~js title="x"` fence becomes a backtick fence and the info text after the language (`title="x"`) is dropped; an indented code block becomes a fenced block [D47, D18].
- Other: tables are re-padded and the delimiter row widened (`|---|---|` becomes `| - | - |`; a 3-column table with alignment became `| -------- | :------: | -------: |`); entities are decoded (`&amp;` to `&`, `&copy;` to a copyright sign); a bare URL becomes `<https://…>`; a lazy continuation line in a blockquote gets a `> ` prefix; `\[bracket\]` comes back as `\[bracket]`; a line starting `--` inside a blockquote comes back as `\--` [D46, D47].
- Kept as written in the same runs: `*` versus `_` for each emphasis and strong span (via the `marker` attribute); soft line breaks inside a paragraph (the `at::`/`with::` lines stay on separate lines); Unicode including a decomposed é, a non-breaking space and an emoji; raw html [D46, D47, D16].
- Empty paragraphs are written as a line `<br />` (`remarkPreserveEmptyLine`, part of the commonmark preset); the maintainer's answer to the report is `editor.remove(remarkPreserveEmptyLinePlugin)` [D17, D36, D47].
- Size effect on the generated long files: typical 5,051 B → 5,213 B, large 249,980 B → 258,242 B, stress 2,499,632 B → 2,582,144 B (none identical) [D49].

Options that control the serializer
- Documented for Milkdown: set `remarkStringifyOptionsCtx` (example options `bullet`, `fences`, `incrementListMarker`) [D7]. The options are those of mdast-util-to-markdown: `bullet`, `bulletOther`, `bulletOrdered`, `closeAtx`, `emphasis`, `strong`, `fence`, `fences`, `incrementListMarker`, `listItemIndent`, `quote`, `resourceLink`, `rule`, `ruleRepetition`, `ruleSpaces`, `setext`, `tightDefinitions`, plus `handlers`, `join`, `unsafe`, `extensions` [D26].
- Each option is one value for the whole document. With `bullet: '-'` and `rule: '-'` a measured run wrote `-` bullets and `---` rules; with `incrementListMarker: false` the list code writes the list's start number on every item (`start + 0`) [D47, D26].
- `emphasis` and `strong` options are only the fallback; the per-node `marker` attribute wins in Milkdown's `emphasis` and `strong` handlers [D15].
- `handlers` entries replace the output of one node type; `join` functions are appended to the default join list and decide the number of blank lines between two sibling blocks (they receive mdast nodes, not source text); `unsafe` entries are appended to the default escape rules, so defaults are not removed through options [D26].
- A measured custom set (below) replaced the `blockquote` handler and added a `wikilink` handler and a `join` function through `remarkStringifyOptionsCtx` [D48].

Custom syntax: measured with a small plugin set (my own code, ~60 lines: `$remark` text-to-wikilink transform, `$nodeSchema` inline atom `wikilink`, `$remark` blockquote-to-`callout` transform, `$nodeSchema` block `callout`, a `wikilink` stringify handler, a `blockquote` handler that prefixes `[!kind] `, and a `join` returning 0 between a heading and a `key:: ` paragraph) [D48]
- Byte-identical after round trip: `S-004`, `synthetic-links-explaining`, `synthetic-notes`, `f04-terrain` (4 of the 8 records tried). These contain wikilinks with and without alias, wikilinks inside emphasis and next to punctuation, voice and note callouts, and beats with `key:: value` lines [D48].
- Still different: `S-901` (blank line inserted before a `> [!voice]` that directly followed `key::` lines), `synthetic-voices-in-world` (lazy continuation line gets `> `, `--` line escaped as `\--`, and a `> [!design]` callout with no title and no body came out as `>`, a limit of my quick plugin), `B-901` (setext to ATX, `\[bracket\]`, two-space hard break), `T-901` (bullets, numbering, table, rule) [D48].
- A `join` function cannot know whether the source had a blank line between two blocks, because the mdast it sees is built from the ProseMirror doc [D26, D13].

## Front matter untouched

- No front matter support in `@milkdown/kit`, `@milkdown/crepe`, the presets or Crepe's feature list; no node, handler or option for it in the installed packages [D24, D53]. A feature request is open since 2025-03-06 (#1712, 4 comments) [D33].
- Measured: a whole file with front matter fed to stock Milkdown comes back with the opening `---` as `***`, the `# comment` line as a heading, the YAML lines merged into one paragraph with a setext underline of dashes (`------…`), and `where: [f04-choir-hall]` as `where: \[f04-choir-hall]` (S-004); every front matter byte sequence changed [D51].
- Community route in #1712: `remark-frontmatter` for `yaml` plus a custom atom node whose `toMarkdown` emits a `yaml` mdast node [D33]. Measured with `remark-frontmatter` 5.0.0 and a `$nodeSchema` of about 8 lines (text content, `code: true`, `yaml` to text and back): the front matter block was byte-identical for `S-004`, `mother-ilse`, `C-031`; whole file identical for the latter two (S-004 differs by body wikilink escapes) [D51]. `$remark` passes `{}` as default options, which makes `remark-frontmatter` throw `Missing type in matter`; the plugin has to be wrapped to pass `['yaml']` (same wrapper as in #1712) [D51, D33].
- Adapter route (front matter split off, body only given to the editor): the API takes and returns plain strings (`defaultValueCtx`, `replaceAll`, `getMarkdown`) [D7]. The corpus bodies in the measurements above were fed this way [D46].
- Source mode: no whole-document source view in `@milkdown/crepe`'s feature list (CodeMirror is used for code blocks only) [D24]. CodeMirror 6 (`codemirror`, `@codemirror/*`) is a dependency of Crepe, and `@milkdown/components` lists `@codemirror/view|state|language` as peer dependencies [D2]. A community `@milkdown-lab/plugin-split-editing` 1.3.1 exists, last modified 2024-08-07 [D29].

## Inline link chips

- No wikilink node, plugin or option in `@milkdown/kit`, `@milkdown/crepe` or the main repository (code search); a GitHub repository search for "milkdown wikilink" returned no wikilink plugin, and the awesome list names none (2026-10-31) [D53, D30].
- Stock behaviour for `[[id]]`: parsed as plain text, serialized back as `\[\[id]]` [D46].
- Mechanism for a custom inline node: `$remark` (remark plugin), `$nodeSchema` (ProseMirror node spec with `parseMarkdown` and `toMarkdown`), `$view` (ProseMirror NodeView constructor), `$prose` (any ProseMirror plugin), `$inputRule`; documented with two worked examples (directive-based iframe node, `==mark==` mark). The docs' own note: a real custom syntax needs a micromark extension, the example uses a simplified text visitor [D10, D20].
- Measured: a text-visitor wikilink plugin plus an inline atom node (`group: 'inline'`, `atom: true`, attrs `id` and `alias`, `class: 'tu-link'` on the span) made `[[id]]`, `[[id|text]]`, wikilinks inside `*…*`, `**…**`, `_…_` and next to punctuation round-trip byte-identically in the records listed above [D48].
- `[[` popover: `@milkdown/plugin-slash` ships `SlashProvider`, a headless ProseMirror plugin view that takes an `HTMLElement` as content, positions it with `@floating-ui/dom`, and has `trigger` (default `/`, string or list), `shouldShow(view, prevState)` override and a `debounce` (default 200 ms) [D27]. The option list has no item search or filtering; the content element is supplied by the host [D27].
- Pushing index changes into the editor without re-creating it: `ctx.set`, `ctx.update`, `ctx.watch` on slices, and `editor.action(ctx => …)` are the documented routes for handing data to plugins [D6, D7].

## Callouts with their own look

- No callout or admonition node in the installed packages; Crepe has no callout feature (features: CodeMirror, ListItem, LinkTooltip, Cursor, ImageBlock, BlockEdit, Toolbar, Placeholder, Table, Latex, TopBar, AI) [D24, D53].
- Stock behaviour for Obsidian callouts: `> [!voice] …` is a blockquote whose first text starts with `[!voice]`; it comes back as `> \[!voice] …` [D46].
- Measured with my plugin set: a `$remark` visitor converting a `blockquote` whose first text starts with `[!kind]` into a custom mdast `callout` node, a block `callout` schema (`content: 'block+'`, attr `kind`, `class: 'tu-' + kind` on the wrapper), and a `blockquote` stringify handler that prefixes `[!kind] ` after the children are serialized; the callout text stays ordinary editable paragraphs inside the node (S-004, synthetic-notes, f04-terrain identical) [D48].
- `Design note:` lines are plain paragraph text with no special handling. ProseMirror decorations can be supplied through `$prose`, which accepts any ProseMirror `Plugin` [D20].
- A "show the Markdown of the construct under the caret as raw text" mode is not documented for v7.23.0: `@milkdown/plugin-automd` (inline sync) turns typed Markdown into marks on keystroke; issue #645 (always show syntax at the caret) was closed in 2022 pointing to RFC #819 "Hybrid Editing", closed 2023-02-14 with items for bold, italic, link and images ticked [D41].

## Beat headings with attribute lines

- `## Beat n — name` parses to the stock `heading` node (attrs `level`, `id`); `syncHeadingIdPlugin` writes an `id` derived from the text (default `textContent.toLowerCase().trim().replace(/\s+/g,'-')`) into the node on every doc change, with `addToHistory` false; measured DOM: `<h2 id="beat-1-—-arrival">` [D18, D19, D47].
- `key:: value` lines: no special support. They parse as one paragraph; each soft line break becomes an inline `hardbreak` node (`isInline: true`) whose DOM is `<span data-type="hardbreak" data-is-inline="true"> </span>` (a space), and whose Markdown is a newline; a hard break is `<br data-is-inline="false">` [D17, D47]. The colons are not escaped (only `[` is escaped in the measured files) [D46].
- Display as in `04-script.html` (heading plus `tu-beat` parameter row with chips): no built-in node for it; the extension points are a custom node with a node view, or decorations through `$prose` [D20, D53].
- Editing one parameter and changing only that line: serialization is whole-document, so other lines stay intact only to the extent that the stock normalizations (above) do not apply to them; measured with the `join` function, `S-004` came back identical, `S-901` did not (extra blank line before a callout that directly followed `key::` lines) [D48].
- A finding shown on its beat: no built-in support; slices and `editor.action` are the documented ways to hand data to plugins, decorations or node views [D6, D7, D20].

## Spell check and IME

- Editing surface (measured DOM dump): `<div contenteditable="true" translate="no" class="ProseMirror editor" role="textbox">`; prose is ordinary text nodes inside `<p>`/`<h2>`. The string `spellcheck` does not occur in the installed `@milkdown/*` 7.23.0 sources and styles, nor in a GitHub code search of the repository, so the webview default applies [D47, D53].
- Turning spell check off: ProseMirror `EditorProps.attributes` through `editorViewOptionsCtx` (maintainer's answer in #751; the FAQ shows `editorViewOptionsCtx` for `editable`) [D41, D7]. Per-node attributes come from a node's `toDOM`, which the docs' examples use for `contenteditable` and `style` [D10].
- IME composition is handled by ProseMirror (`view.composing` is true while a composition is active) [D45]. Milkdown-specific parts found:
  - `syncHeadingIdPlugin` returns early while `view.composing` is true [D19].
  - `@milkdown/preset-gfm` depends on `prosemirror-safari-ime-span` and uses it in `auto-insert-span-plugin.ts`; that package works around a Safari bug where IME text replacement removes empty nodes when the composition finishes (ProseMirror issue 934) [D25].
- Past IME reports, all closed: #1528 (Chinese IME, caret jumps to the start after input; fixed by PR #1568 merged 2024-11-28, issue closed 2024-12-09), #1542 (same family, closed 2024-10-20), #1615 (Chinese pinyin left in the document in a collaborative setup; maintainer: `y-prosemirror` force-flushes the view so `view.composing` turns false; workaround `editor.use(commonmark.filter(x => x !== syncHeadingIdPlugin))`; closed 2025-01-15) [D38].
- Search of the issue tracker (2026-10-31, terms IME, composition, input method, pinyin, Chinese, Japanese, Korean, dead key, umlaut, accent): no open issue on IME or dead keys. No report about Swiss or German dead keys found [D38].
- Open typing-order and caret issues in Crepe's list item component (`@milkdown/components` `list-item-block`, used by Crepe's ListItem feature, on by default in Crepe): #2506 (caret restore one animation frame after mount moves the caret back over text typed in that frame; example "zwei" becomes "weiz"), #2501 (text typed right after Enter, Tab or undo in a list lands out of order) [D40].
- Older WebKit and regular expressions: Milkdown's own source uses regex lookbehind (`remark-line-break.ts`: `(?<![\t ])`; reported earlier for `strongInputRule`), and #2190 (Tauri 2.0 + Crepe 7.17.2, closed same day, 2025-12-04) reports "invalid group specifier name" because the embedded WebKit lacked lookbehind; #1703 reports the same cause in `mdast-util-gfm-autolink-literal` on Safari iOS, closed by the maintainer as out of scope [D17, D39].
- Scope note: what WebView2, WKWebView and WebKitGTK do for spell check and IME is not covered here; spec §4.3 lists those webview facts once for all candidates.

## Long documents

- Milkdown renders the whole ProseMirror document into the DOM; I found no windowing or virtualization option in the docs, source or Crepe feature list read (absence of a mention, not a measured claim) [D6, D12, D24].
- Measured, headless (Node 24.19.0, jsdom 30.1.3, kit 7.23.0, Windows 11, Intel Core Ultra 7 270K Plus, 34 GB), single run per file, stock `parserCtx`/`serializerCtx` only, no browser layout or paint [D49]:
  - `typical.md` body 5,051 B: parse 12 ms, serialize 6 ms, doc size 4,946, heap used 79 MB.
  - `large.md` body 249,980 B: parse 277 ms, serialize 92 ms, doc size 244,475, heap used 140 MB.
  - `stress.md` body 2,499,632 B: parse 10,401 ms, serialize 1,393 ms, doc size 2,444,627, heap used 963 MB.
  - 10.0 times the input bytes gave 37.5 times the parse time and 15.1 times the serialize time between large and stress. Heap is `heapUsed` read after serialize, without forced GC.
- Work done on every document change, from source: `syncHeadingIdPlugin` calls `doc.descendants` over the whole document after each doc-changing update (and `doc.eq` first) [D19]; the listener plugin serializes the whole document after a 200 ms debounce when `markdownUpdated` listeners exist [D23].
- Reported and closed: #1601 (slugify cost on a ~150 kb document at paste and edit, closed 2025-02-14; workaround replaced the id generator; the current default generator is a lowercase/trim/replace), #2503 (`remarkLineBreak` quadratic on long runs of spaces, 50k spaces about 4 s; closed 2026-10-23; fix listed in 7.23.0) [D37, D19, D44].
- User report in #2396 (open feature request): `replaceAll` froze the UI on a "relatively large document"; a diff-based patch did not (reported by that user, not measured by me) [D32].

## Svelte 5 integration

Bindings
- Official integrations in the repo: `packages/integrations/react` and `packages/integrations/vue` only (`@milkdown/react` and `@milkdown/vue` exist on npm) [D29].
- Docs recipe for Svelte: "We don't provide Svelte support out of box, but you can use the vanilla version with it easily." Example: a Svelte action `use:editor` on a `div` calling `Editor.make().config(ctx => ctx.set(rootCtx, dom)).use(commonmark).create()`; no runes and no teardown shown; the example loads `@milkdown/theme-nord` [D8].
- Community: `semanticdata/svelte-milkdown` is an example app (8 stars, last push 2024-04-29, pins Milkdown 7.1.1), not a library; `npm search` for Milkdown plus Svelte returned only `@milkdown/*` packages (2026-10-31); the Milkdown awesome list names a community Angular integration and no Svelte one [D31, D30].
- The docs mention no Svelte 5 specific material. An issue search for "svelte" (2026-10-31) shows one open bug with a Svelte reproduction: #2121, caret offset when the editor sits inside a zoomed svelteflow container, not reproducible in the Milkdown playground [D8, D40].

Mount and tear down
- The core API is framework-neutral: `Editor.make().config(ctx => ctx.set(rootCtx, element))…create()` returns a promise; `await editor.destroy()` is documented ("always destroy the editor when the component unmounts"); Crepe has `new Crepe({ root })`, `create()`, `destroy()` [D7, D12].
- Open report for teardown: #2507, `destroy()` throws "Cannot read properties of undefined (reading 'unmount')" when CodeMirror failed to initialize (Crepe code block) [D40].

Design-system components inside the editor
- `$view(type, factory)` registers a ProseMirror NodeView or MarkView constructor (`nodeViewCtx`, `markViewCtx`) as written by the host, with no Milkdown wrapper around the view object [D20]. No Milkdown example mounts a Svelte component in a node view [D8, D10].
- Milkdown's own node views (`@milkdown/components`, Crepe) are written with Vue 3: `@milkdown/components` and `@milkdown/crepe` depend on `vue ^3.5.20`, and `@milkdown/kit` depends on `@milkdown/components`, so Vue is installed with the kit (210 packages in the production closure of kit plus crepe, measured) [D1, D2, D52].

Index and finding pushes, keymap, history
- Slices (`createSlice`, `ctx.get/set/update/watch`) and `editor.action` push host data in without re-creating the editor [D6, D7].
- Keymap: shortcuts are per-command slices, configurable with `ctx.set(<keymap>.key, { CommandName: 'Mod-Shift-b' })` or arrays; `Mod` is Ctrl on Windows and Linux and Cmd on macOS; commands can also be called with `commands.call(<command>.key)` [D9]. The docs describe no hook for a host-owned keymap; the documented routes are the per-command slices and `commands.call` [D9].
- External change as a minimal edit: no built-in API. Stock macros: `replaceAll(markdown)` is one `tr.replace(0, doc.content.size, Slice)`; `replaceAll(markdown, true)` re-creates the editor state [D21]. Measured on a 3-paragraph doc with the caret mid-paragraph 2 after one local edit (undo depth 1) and an external change in paragraph 1: `replaceAll` moved the caret from 40 to 97 (end of document) and left undo depth at 1; `replaceAll(…, true)` reset the selection to 1 and undo depth to 0; a block diff with `computeDocDiff` from `@milkdown/plugin-diff` plus `tr.replace` per change plus `selection.map` (the pattern in #2396) kept the caret in the same text (40 to 60, shifted by the 20 inserted characters) and raised undo depth to 2 [D50, D32, D28]. `@milkdown/plugin-diff` was first published 2026-04-01 [D28]. A built-in patch API is an open request (#2396) [D32].
- Marking and clearing undo history: `@milkdown/plugin-history` wraps `prosemirror-history` (`Undo`, `Redo` commands bound to `Mod-z`, `Mod-y`/`Shift-Mod-z`, config `{ depth, newGroupDelay }`), and `@milkdown/prose/history` re-exports all of `prosemirror-history` (`closeHistory`, `undoDepth`, `isHistoryTransaction`) [D22]. The only history reset found in the Milkdown macros read is the state re-creation of `replaceAll(…, true)` [D21, D50].

## Licence

- Package licence: MIT for `@milkdown/kit`, `core`, `crepe`, `preset-commonmark`, `preset-gfm`, `transformer`, `plugin-listener` (npm); repository LICENSE "The MIT License (MIT), Copyright (c) 2020-present Mirone" [D1, D42].
- Transitive closure, measured by walking `dependencies` from `@milkdown/kit` and `@milkdown/crepe` 7.23.0 in a fresh npm install on 2026-10-31 (peer dependencies not followed, `jsdom` and `remark-frontmatter` excluded): 210 packages: 206 MIT, `dompurify` 3.4.17 `(MPL-2.0 OR Apache-2.0)`, `entities` 7.0.1 BSD-2-Clause, `source-map-js` 1.2.2 BSD-3-Clause, `picocolors` 1.1.1 ISC [D52]. `npm view <pkg> license` for the direct dependencies of crepe, components, kit and the presets (vue, codemirror, katex, lodash-es, remark and unified family, prosemirror-*, `@floating-ui/dom`, nanoid, clsx) agrees (all MIT except dompurify) [D1, D2, D52]. `pnpm licenses list` was not run (no install into the repo).
- Paid tier or hosted service: none found for any feature; the docs say "open source and fully free" [D6]. The funding link is GitHub Sponsors for the maintainer [D42].
- Network and telemetry: among the installed `@milkdown/*` packages the only `fetch(` calls and external API hosts (`https://api.openai.com`, `https://api.anthropic.com`) are in `@milkdown/crepe`'s `llm-providers`; the Crepe AI feature is off by default (`[CrepeFeature.AI]: false`), as is TopBar [D24, D53]. No `XMLHttpRequest`, `sendBeacon` or `WebSocket(` in those packages [D53]. The scan covered `@milkdown/*` packages only, not third-party dependencies (vue, dompurify, katex, codemirror) [D53]. "Telemetry" in Milkdown's docs is an in-process plugin inspector (`editor.enableInspector()`, `editor.inspect()`), disabled by default, with no network call described [D11].
- No install scripts (`preinstall`, `install`, `postinstall`) in the npm metadata of `@milkdown/kit`, `core`, `crepe` [D1].

## Maintenance

Release cadence (npm and GitHub, read 2026-10-31)
- Latest: 7.23.0, published 2026-10-27. Releases in the last 12 months (2025-10-31 to 2026-10-31): 16 (7.17.2 on 2025-11-23, 7.17.3, 7.18.0 on 2026-01-05, 7.19.0 on 2026-03-03, 7.19.1, 7.19.2, 7.20.0 on 2026-03-30, 7.21.0 on 2026-05-12, 7.21.1, 7.21.2 on 2026-06-02, 7.21.3 on 2026-07-12, 7.22.0 on 2026-08-03, 7.22.1, 7.22.2 on 2026-09-23, 7.22.3 on 2026-10-10, 7.23.0). Longest gaps: 2026-01-05 to 2026-03-03 (57 days), 2025-10-11 to 2025-11-23 (43 days), 2026-03-30 to 2026-05-12 (43 days) [D1, D4].
- Major version 7 since 2023-02-21 (7.0.0); no 8.x; `@milkdown/core` has 162 published versions since 2021-04-26; `@milkdown/kit` first published as 7.4.0 on 2024-08-02. Releases are made with changesets; entries are labelled minor or patch and mix fixes and features; no stated breaking-change policy in the docs read [D1, D44].
- Output-affecting changes arrived in patch and minor releases: 7.22.2 "keep user-authored inline br when loading markdown" (#2463), 7.22.3 "update markdown serializer for stable autolinks" (#2482), 7.23.0 "keep remarkLineBreak linear" and "keep hardbreak attrs" [D44]. #2516 reports that Crepe 7.23.0 with mdast-util-to-markdown 2.2.1 changes saved output for inline maths followed by a digit, not reproducible on 7.22.3 with mdast-util-to-markdown 2.1.2; the packages use caret ranges for `remark`, `remark-stringify` and `unified` [D40, D14].

Activity
- Default branch `main`: last commit 2026-10-27; repository last pushed 2026-10-31. Commits to `main` since 2025-10-31: 260 (135 by bots such as renovate and github-actions, 125 by people): Saul-Mirone 89, 24 other people 36 [D5].
- Merged pull requests since 2025-10-31: 233 (bots included). Of 51 PRs merged since 2026-07-31 (first API page of 100 closed PRs), Saul-Mirone authored 10, bots 24, 13 other contributors 17 [D3, D5].
- Issues since 2025-10-31: 72 opened, 56 closed. Open now: 36 issues (8 labelled `bug`), 8 pull requests [D3].
- 12,034 GitHub stars, 563 forks; npm downloads for the week 2026-10-23 to 2026-10-29: `@milkdown/core` 725,991, `@milkdown/kit` 668,148, `@milkdown/crepe` 481,371 [D3, D43].

Maintainers and backing
- npm `maintainers` for `@milkdown/kit`: one account (`mirone`); GitHub organisation `Milkdown` has one public member (Saul-Mirone); all-time commits: Saul-Mirone 2,474, next human contributor 14 [D1, D5, D42].
- Backing: an individual maintainer (GitHub profile location Tokyo) funded through GitHub Sponsors (`FUNDING.yml`: `github: Saul-Mirone`); README thanks JetBrains, Vercel OSS, Cypress, Nx and Algolia for open-source programme support; no company owner or paid staff stated. The docs ask users to "fund me in order to help with the maintenance" [D42, D6].
- SECURITY.md: report by issue or email to the maintainer [D42].

## Known open issues

State of all items: open as of 2026-10-31 unless stated.
- #2396 Provide a way to "patch" the current markdown without doing a full replace (cursor position) · https://github.com/Milkdown/milkdown/issues/2396 · open, created 2026-06-15, 2 comments · external change as a minimal edit that keeps the caret (criterion 8, CE-34.2) [D32]
- #1712 Frontmatter/metadata support · https://github.com/Milkdown/milkdown/issues/1712 · open, created 2025-03-06, 4 comments · no front matter handling (criterion 2) [D33]
- #2349 Backslashes in autolink URLs double on every round-trip · https://github.com/Milkdown/milkdown/issues/2349 · open, created 2026-04-30; a measured round trip on 7.23.0 no longer doubles (3 passes stable) but drops the backslash from `<https://example.com/page.\>` · serializer behaviour in the byte report (criterion 1) [D34, D47]
- #2516 Crepe 7.23.0 saves inline maths followed by a digit as Markdown that reopens as plain text · https://github.com/Milkdown/milkdown/issues/2516 · open, created 2026-10-28 · output changed by a transitive dependency bump; applies with Crepe's Latex feature (criteria 1 and 10) [D40]
- #2505 `syncListOrderPlugin` uses a relative node offset as a document position in `tr.setNodeMarkup()`, corrupting the document root or throwing RangeError · https://github.com/Milkdown/milkdown/issues/2505 · open, created 2026-10-21, label bug · `@milkdown/preset-commonmark` list handling (criterion 1) [D40]
- PR #2487 fix(preset-commonmark): stop list items losing a leading block on save · https://github.com/Milkdown/milkdown/pull/2487 · open pull request, created 2026-10-10 · a list item starting with a code block or heading is saved as `* <br />` plus the block and re-parses differently (criterion 1) [D40]
- #2090 Regular expressions are not consistent with the state machine of commonmarkjs · https://github.com/Milkdown/milkdown/issues/2090 · open, created 2025-09-08, label bug · input rules around `**test.**a` (typing in Write mode; criterion 6) [D40]
- #1249 Limited support for commonly used HTML tags and entities in Markdown · https://github.com/Milkdown/milkdown/issues/1249 · open, created 2024-02-20, label enhancement · inline HTML and entity handling (criterion 1) [D40]
- #2506 list-item-block: caret restore after mount moves the caret back over text typed in that frame · https://github.com/Milkdown/milkdown/issues/2506 · open, created 2026-10-22 · typing correctness in Crepe lists (criterion 6) [D40]
- #2501 Text typed right after Enter, Tab or undo in a Crepe list lands out of order · https://github.com/Milkdown/milkdown/issues/2501 · open, created 2026-10-19 · typing correctness in Crepe lists (criterion 6) [D40]
- #2413 Inline styles cannot exit to normal text at paragraph end · https://github.com/Milkdown/milkdown/issues/2413 · open, created 2026-07-14 · caret and marks at the end of a paragraph (criterion 6); related #2380 (inline code exit at line end, open, enhancement) [D40]
- #2507 Code block destroy() throws when CodeMirror failed to initialize · https://github.com/Milkdown/milkdown/issues/2507 · open, created 2026-10-22, label bug · teardown of the editor with Crepe code blocks (criterion 8) [D40]
- #2121 Incorrect caret positioning when zoomed in specific library · https://github.com/Milkdown/milkdown/issues/2121 · open, created 2025-09-26, label bug; reproduction is a Svelte app with the editor inside a zoomed svelteflow container, not reproducible in the Milkdown playground · caret rendering inside a transformed container (criterion 6; criterion 8 for the Svelte setting) [D40]
- #2495 / #2414 table-cell click selection (deferred selection overwrites a newer ProseMirror selection; click enters edit mode instead of selecting a cell) · https://github.com/Milkdown/milkdown/issues/2495 · open, created 2026-10-15 and 2026-07-16 · Crepe table block, relevant only if the table feature is used (criterion 6) [D40]
- Closed with a bearing on the criteria, for reference: #2403 nested strong and emphasis corrupted on serialize (closed 2026-07-02); #2428 `remarkPreserveEmptyLine` deleted user `<br>` (closed 2026-08-29); #2503 `remarkLineBreak` quadratic (closed 2026-10-23); #1528 and #1615 IME reports (closed 2024-12-09 and 2025-01-15) [D35, D36, D37, D38]

## Unknowns

- Whether a per-block raw-source attribute plus a verbatim `html` re-emit keeps untouched spans byte-identical through typing, split, merge, paste and undo. The sources show the pieces (offsets in remark, free node attrs, verbatim `html` nodes) but no example. A prototype has to run the corpus edits of the manifest against the oracle and count changed bytes outside the edit. Without it, the measured stock result stands: 2 of 13 non-empty bodies unchanged, 4 of 8 records unchanged with the wikilink/callout/join plugins.
- Browser-level behaviour is unmeasured: caret stepping over a chip, delete removing a whole chip, `spellcheck="false"` on chips and attribute lines in WebView2, WKWebView and WebKitGTK, IME and dead keys (Swiss and German layouts) next to a chip, and the minimum WebKit version given Milkdown's lookbehind regexes (#2190). A prototype run in Chromium and WebKit with the input harness has to measure them.
- Large-document cost in a browser: open time, keystroke-to-paint latency (median and p95), scroll, JS heap. Headless parse and serialize are known (277 ms and 92 ms on `large.md`; 10.4 s and 1.4 s on `stress.md`), but the per-keystroke cost of `syncHeadingIdPlugin` (whole-document walk) and of the listener's whole-document serialize is not.
- Svelte 5 specifics: mounting `Editor.create()` and `destroy()` in a runes component, `mount`/`unmount` of design-system components from a NodeView, whether Vue ends up in the bundle when only `@milkdown/kit/core` and the two presets are imported, and the bundle size. A prototype with Vite has to measure them.
- Live-preview Write mode (raw Markdown of the construct under the caret): the sources do not show whether any Milkdown package renders it in 7.23.0 (RFC #819 items ticked in 2023; `plugin-automd` re-parses typed Markdown). A prototype has to check the playground behaviour or build it with decorations.
- Full licence scan of the prototype's resolved tree with `pnpm licenses list`, and a scan of third-party dependencies (vue, dompurify, katex, codemirror) for network calls.
- Front matter edge cases (delete at the body start, select-all and type, paste, undo) with the front matter as a node or split off in the adapter.
- Whether the history can be cleared or marked on save without re-creating the state, beyond `closeHistory` and `replaceAll(…, true)`.

## Sources

- D1 · https://www.npmjs.com/package/@milkdown/kit (registry data via `npm view @milkdown/kit version time dependencies maintainers license scripts`, same for core, crepe, preset-commonmark, preset-gfm, transformer, plugin-listener; `@milkdown/vue` and `@milkdown/react` exist at 7.23.0) · npm metadata, versions 7.23.0 · read 2026-10-31
- D2 · https://www.npmjs.com/package/@milkdown/crepe (`npm view` of dependencies and peerDependencies for @milkdown/crepe, @milkdown/components, @milkdown/prose, and `license` of each direct dependency) · npm metadata · read 2026-10-31
- D3 · https://github.com/Milkdown/milkdown (GitHub API: repo metadata, issue and PR search counts, org members, merged PR authors) · repository state · read 2026-10-31
- D4 · https://github.com/Milkdown/milkdown/releases (GitHub API releases list) · releases v7.12.1 to v7.23.0 with dates · read 2026-10-31
- D5 · https://github.com/Milkdown/milkdown/commits/main (GitHub API: commits since 2025-10-31 by author, contributors list) · commit activity · read 2026-10-31
- D6 · https://github.com/Milkdown/website/blob/main/docs/guide/architecture-overview.md and https://github.com/Milkdown/website/blob/main/docs/guide/why-milkdown.md · Architecture Overview; Why Milkdown (docs source of milkdown.dev) · read 2026-10-31
- D7 · https://github.com/Milkdown/website/blob/main/docs/guide/faq.md · FAQ · read 2026-10-31
- D8 · https://github.com/Milkdown/website/blob/main/docs/recipes/svelte.md · Svelte recipe · read 2026-10-31
- D9 · https://github.com/Milkdown/website/blob/main/docs/guide/keyboard-shortcuts.md · Keyboard Shortcuts · read 2026-10-31
- D10 · https://github.com/Milkdown/website/blob/main/docs/plugin/example-iframe-plugin.md and .../example-marker-plugin.md · plugin examples (custom syntax) · read 2026-10-31
- D11 · https://github.com/Milkdown/website/blob/main/docs/blogs/announcing-telemetry-inspector.md · Announcing Telemetry Inspector · read 2026-10-31
- D12 · https://github.com/Milkdown/website/blob/main/docs/guide/using-crepe.md, .../using-milkdown-kit.md, .../using-components.md · Using Crepe, Kit, Components · read 2026-10-31
- D13 · https://github.com/Milkdown/milkdown/blob/main/packages/transformer/src/serializer/state.ts and .../parser/state.ts · serializer and parser state · read 2026-10-31
- D14 · https://github.com/Milkdown/milkdown/blob/main/packages/core/src/internal-plugin/init.ts and .../atoms.ts · remark instance and `remarkStringifyOptionsCtx` · read 2026-10-31
- D15 · https://github.com/Milkdown/milkdown/blob/main/packages/core/src/__internal__/remark-handlers.ts · text, strong, emphasis stringify handlers · read 2026-10-31
- D16 · https://github.com/Milkdown/milkdown/blob/main/packages/plugins/preset-commonmark/src/plugin/remark-marker-plugin.ts · remarkMarker · read 2026-10-31
- D17 · https://github.com/Milkdown/milkdown/tree/main/packages/plugins/preset-commonmark/src (plugin/remark-line-break.ts, plugin/remark-preserve-empty-line.ts, node/paragraph.ts, node/hardbreak.ts, __internal__/serialize-text.ts) · line breaks and empty-line placeholder · read 2026-10-31
- D18 · https://github.com/Milkdown/milkdown/tree/main/packages/plugins/preset-commonmark/src/node (bullet-list.ts, list-item.ts, ordered-list.ts, code-block.ts, heading.ts, hr.ts) · node schemas and attrs · read 2026-10-31
- D19 · https://github.com/Milkdown/milkdown/blob/main/packages/plugins/preset-commonmark/src/plugin/sync-heading-id-plugin.ts · syncHeadingIdPlugin · read 2026-10-31
- D20 · https://github.com/Milkdown/milkdown/tree/main/packages/utils/src/composable (`$view.ts`, `$prose.ts`); `$remark`, `$nodeSchema`, `$inputRule` as used in the docs examples of D10 · composables for node views and ProseMirror plugins · read 2026-10-31
- D21 · https://github.com/Milkdown/milkdown/blob/main/packages/utils/src/macro/replace-all.ts · replaceAll · read 2026-10-31
- D22 · https://github.com/Milkdown/milkdown/blob/main/packages/plugins/plugin-history/src/index.ts and the installed `@milkdown/prose` 7.23.0 `lib/history.d.ts` (`export * from 'prosemirror-history'`) · history plugin and re-export · read 2026-10-31
- D23 · https://github.com/Milkdown/milkdown/blob/main/packages/plugins/plugin-listener/src/index.ts · listener (200 ms debounce, full serialize) · read 2026-10-31
- D24 · https://github.com/Milkdown/milkdown/tree/main/packages/crepe/src (feature/index.ts, core/builder.ts, core/crepe.ts, llm-providers) · Crepe features, defaults, AI providers · read 2026-10-31
- D25 · https://github.com/Milkdown/milkdown/blob/main/packages/plugins/preset-gfm/package.json and https://www.npmjs.com/package/prosemirror-safari-ime-span (README) · Safari IME span plugin · read 2026-10-31
- D26 · https://github.com/syntax-tree/mdast-util-to-markdown/blob/main/readme.md and installed mdast-util-to-markdown 2.2.1 `lib/configure.js`, `lib/handle/list-item.js`, `lib/join.js` · options, defaults, merge behaviour · read 2026-10-31
- D27 · https://github.com/Milkdown/milkdown/blob/main/packages/plugins/plugin-slash/src/slash-provider.ts · SlashProvider · read 2026-10-31
- D28 · installed `@milkdown/plugin-diff` 7.23.0 `lib/index.d.ts` and `npm view @milkdown/plugin-diff time.created` (2026-04-01) · computeDocDiff and diff plugin · read 2026-10-31
- D29 · https://github.com/Milkdown/milkdown/tree/main/packages/integrations (react, vue) and https://www.npmjs.com/package/@milkdown-lab/plugin-split-editing (1.3.1, modified 2024-08-07) · official integrations; split-editing plugin · read 2026-10-31
- D30 · https://github.com/Milkdown/awesome/blob/main/README.md · community list (Angular integration, plugins) · read 2026-10-31
- D31 · https://github.com/semanticdata/svelte-milkdown and `npm search` results for "milkdown svelte" variants · community Svelte example; no binding package · read 2026-10-31
- D32 · https://github.com/Milkdown/milkdown/issues/2396 · patch markdown without full replace · read 2026-10-31
- D33 · https://github.com/Milkdown/milkdown/issues/1712 · Frontmatter/metadata support · read 2026-10-31
- D34 · https://github.com/Milkdown/milkdown/issues/2349 · Backslashes in autolink URLs · read 2026-10-31
- D35 · https://github.com/Milkdown/milkdown/issues/2403 · Nested strong/emphasis not remerged · read 2026-10-31
- D36 · https://github.com/Milkdown/milkdown/issues/2428 and https://github.com/Milkdown/milkdown/issues/2379 · `<br />` empty-paragraph placeholder · read 2026-10-31
- D37 · https://github.com/Milkdown/milkdown/issues/2503 and https://github.com/Milkdown/milkdown/issues/1601 · performance reports · read 2026-10-31
- D38 · https://github.com/Milkdown/milkdown/issues/1528, #1542, #1615 and PR #1568; issue-search results for IME terms · IME reports · read 2026-10-31
- D39 · https://github.com/Milkdown/milkdown/issues/2190 and https://github.com/Milkdown/milkdown/issues/1703 · WebKit regular expression reports · read 2026-10-31
- D40 · https://github.com/Milkdown/milkdown/issues/2506, #2501, #2516, #2507, #2505, #2495, #2414, #2380, #2121, #2090, #2413, #1249 and pull request #2487 (issue list `repos/Milkdown/milkdown/issues?state=open`, titles and bodies) · open issues and PR · read 2026-10-31
- D41 · https://github.com/Milkdown/milkdown/issues/751, #645, #819 and https://github.com/Milkdown/milkdown/tree/main/packages/plugins/plugin-automd · spellcheck attribute answer; hybrid editing RFC; automd plugin · read 2026-10-31
- D42 · https://github.com/Milkdown/milkdown (LICENSE, README.md, .github/FUNDING.yml, SECURITY.md) and https://github.com/Saul-Mirone · licence, sponsors, security contact, maintainer profile · read 2026-10-31
- D43 · https://api.npmjs.org/downloads/point/last-week/@milkdown/kit (and /core, /crepe) · weekly downloads 2026-10-23 to 2026-10-29 · read 2026-10-31
- D44 · https://github.com/Milkdown/milkdown/blob/main/packages/core/CHANGELOG.md · changelog 7.22.0 to 7.23.0 · read 2026-10-31
- D45 · https://prosemirror.net/docs/ref/ · ProseMirror reference (`EditorView.composing`) · read 2026-10-31
- D46 · Local measurement (not in the repo): script `rt.mjs` in `C:\Users\janne\AppData\Local\Temp\claude\D--Projects-theos-editor\c97b1761-fd8c-4c8f-8231-1107b43666b8\scratchpad\mk-exp\`. Headless Milkdown (`Editor.make().use(commonmark).use(gfm)`, `defaultValueCtx` = body, `getMarkdown()`), Node 24.19.0, jsdom 30.1.3, `@milkdown/kit` 7.23.0, remark-stringify 11.0.0, mdast-util-to-markdown 2.2.1; input: the 17 files of `research/P10-prose-editor/corpus/records/` with front matter (`^---\n…\n---\n`) split off; compared as strings · run 2026-10-31
- D47 · Local measurement: scripts `rt2.mjs`, `rt8.mjs`, `rt9.mjs`, `rt10.mjs` in the same directory; construct matrix (bullets, emphasis, rules, fences, headings, ordered lists, hard breaks, entities, html, urls, blank lines, tables, callout-shaped blockquotes), option variants (`bullet`, `rule`, `emphasis`, `strong`, `fence`, `listItemIndent`, `setext`, `incrementListMarker`), DOM dump of the editor root, empty-paragraph serialization, autolink and nested-mark repros of #2349 and #2403 · run 2026-10-31
- D48 · Local measurement: script `rt3.mjs` (same directory); own plugin set for wikilinks, callouts and a heading-to-`key::` join, run against 8 corpus records (S-004, S-901, synthetic-links-explaining, synthetic-voices-in-world, B-901, synthetic-notes, f04-terrain, T-901) · run 2026-10-31
- D49 · Local measurement: script `rt4.mjs` (same directory); `parserCtx` and `serializerCtx` timings on the generated `typical.md`, `large.md`, `stress.md` bodies, single run each, same machine as D46 (Intel Core Ultra 7 270K Plus, 24 logical cores, 34 GB) · run 2026-10-31
- D50 · Local measurement: script `rt5.mjs` (same directory); `replaceAll`, `replaceAll(…, true)` and `computeDocDiff` patch against a local edit, caret and `undoDepth` read before and after · run 2026-10-31
- D51 · Local measurement: scripts `rt6.mjs`, `rt7.mjs` (same directory) with `remark-frontmatter` 5.0.0; whole files fed to stock Milkdown, then with a yaml atom node, on S-004, mother-ilse, C-031 · run 2026-10-31
- D52 · Local measurement: script `lic.mjs` (same directory); walk of `dependencies` from `@milkdown/kit` and `@milkdown/crepe` 7.23.0 in the fresh npm install of D46, reading each `package.json` license · run 2026-10-31
- D53 · Local searches: `grep` over the installed `@milkdown/*` 7.23.0 packages (`fetch(`, `XMLHttpRequest`, `sendBeacon`, `WebSocket(`, `position`, `spellcheck`, callout, admonition, wikilink, front matter; the "callout" hits are the identifier `ctxCallOutOfScope`) and GitHub code search in Milkdown/milkdown for `spellcheck`, `callout`, `admonition`, `wikilink`, `frontmatter` · run 2026-10-31
