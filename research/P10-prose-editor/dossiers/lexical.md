# Lexical — dossier

Researched: lexical 0.53.0 with @lexical/markdown, @lexical/mdast, @lexical/history, @lexical/extension, @lexical/rich-text, @lexical/list, @lexical/link, @lexical/react, all 0.53.0 (npm, read 2026-10-31) · Researcher: P10 Task 3
Researcher probes (supporting evidence only, not reproduced): D39, D42

Notes on method. `lexical` 0.53.0 is `latest` on npm, published 2026-10-28 [D1]. I read the published tarballs (source and CHANGELOG), the docs at lexical.dev, the GitHub repository (it now lives at `github.com/meta/lexical`; `facebook/lexical` redirects) and its issues. Facts marked [D39] come from small experiments I ran in the researcher scratch directory, not retained, against the 17 corpus records of `research/P10-prose-editor/corpus/records/` (HEAD 2c7906b). They are not the P10 harness: the bodies were imported after splitting the front matter off, in headless Node, with no DOM.

## Byte-stable Markdown

Document model
- The document is an `EditorState`: an immutable tree of nodes, locked after each update; reads and writes happen in `editor.read` / `editor.update` callbacks [D9].
- Formatting is stored as a property of each text node, not as nesting, so one content has one representation [D9].
- `editorState.toJSON()` serializes the tree only (no selection, no node keys) and is the native persistence format [D9]. Markdown is an import/export conversion, not the stored text [D3, D11].
- Nodes carry no source offsets or character-index mapping; the docs page on editor state does not mention any [D9]. Node keys are runtime-only and opaque [D9].
- Typed per-node data can be attached with NodeState (`createState`, `$getState`, `$setState`), serialized under a `"$"` key in the JSON [D2, D3].

How `@lexical/markdown` converts
- `$convertFromMarkdownString(markdown, transformers, node?, shouldPreserveNewLines=false, shouldMergeAdjacentLines=false)` clears the target root (`root.clear()`), imports it line by line with regex transformers, and calls `root.selectStart()` if a selection exists [D3].
- Import splits the string on `'\n'` only [D3].
- `$convertToMarkdownString(transformers, node?, shouldPreserveNewLines=false)` rebuilds a string from the node tree and joins blocks with `'\n'`; it adds no trailing newline [D3].
- Transformer kinds: element (`regExp`, `replace`, `export`), multiline element (`regExpStart`, `regExpEnd`, `replace`, `export`), text format (`tag`, `format`), text match (`importRegExp`, `regExp`, `replace`, `export`, `trigger`, `getEndIndex`). Transformers are passed explicitly, so a custom list replaces the stock one [D3, D11].
- Stock `TRANSFORMERS`: HEADING, QUOTE, UNORDERED_LIST, ORDERED_LIST; CODE (multiline); INLINE_CODE, BOLD_ITALIC_STAR, BOLD_ITALIC_UNDERSCORE, BOLD_STAR, BOLD_UNDERSCORE, HIGHLIGHT, ITALIC_STAR, ITALIC_UNDERSCORE, STRIKETHROUGH; LINK. The package exports no table, thematic-break, setext-heading or HTML-comment transformer; `CHECK_LIST` is exported but is not in the default set [D3].
- `shouldPreserveNewLines=true` skips `normalizeMarkdown`, keeps blank lines as empty paragraphs and escapes only `* _ ` ~` on export; the default mode runs `normalizeMarkdown` and escapes `[ ] * _ ` ~ \`, plus `<` and `&` in some positions, as `\x` or `&#NN;` [D3].
- Syntax that the package remembers in NodeState: list marker (`mdListMarker`), code fence (`mdCodeFence`), code meta (`mdCodeMeta`), hard line break style (`mdHardLineBreak`) [D3].
- 0.53.0 changed the export escaping (#9481: "escapes literal text so it does not become HTML, character references or links when parsed again") and the import unescaping (#9396) [D2]. Behaviour of earlier versions was not measured.

Measured on the corpus, stock `TRANSFORMERS`, 0.53.0, front matter split off (17 files) [D39]
- Default mode: 5 of 17 bodies come back byte-identical, 12 differ.
  - `[[id]]` becomes `&#91;&#91;id&#93;&#93;`, and `> [!voice]` becomes `> &#91;!voice&#93;`.
  - A blank line is inserted between `## Beat n — name` and the first `key:: value` line.
  - Runs of two or three blank lines collapse to one.
  - The final newline is dropped (`f04-choir-hall.md`, `synthetic-characters.md`).
  - The `-`, `*` and `+` lists in `T-901.md` come out as one run with `+`.
  - In the CRLF file `\r\n` followed by `\n` appears.
- `shouldPreserveNewLines=true`: 14 of 17 are identical (including the CRLF file, because `\r` stays in the text, the no-final-newline file and the whitespace file). 3 differ:
  - `_x_` becomes `*x*` and `__x__` becomes `**x**`.
  - Nested list indentation is rewritten to 4 spaces per level (`  - a` becomes `    - a`).
  - A `1. 1. 1.` list is renumbered `1. 2. 3.`.
  - `***` is escaped to `\*\*\*`.
  - `\[bracket\]` loses its backslashes.
- A one-word edit in preserve mode changed that one line and exactly the same other lines as the unedited round trip, and nothing else (tested on `T-901.md` and `B-901.md`). The unedited round trip is the floor of the diff [D39].
- With a custom inline `DecoratorNode` (`WikiLinkNode`, `getTextContent()` returns the raw `[[id|text]]`) and a text-match transformer, preserve mode stays at 14 of 17 and default mode at 5 of 17. In this naive version `*[[id]]*` and `_[[id|text]]_` lose their emphasis markers, because a plain DecoratorNode has no text format [D39].
- The whole file, front matter included, through the stock path in preserve mode: 11 of 17 identical; 4 changed in the front matter (`held_by:` becomes `held\_by:`, `  - x` list items lose their indent), 2 changed in the body for the causes above [D39].

`@lexical/mdast` (alternative converter)
- Built on micromark and mdast; marked `@experimental` ("may change between any two Lexical releases"); the README says `@lexical/markdown` "remains the supported default" [D4]. Introduced in 0.47.0 [D2].
- The README says re-serializing "stays close to the source, though not exact": list bullet, code fence and hard-break style are kept, tables are re-padded and some constructs are normalized [D4].
- It reads `node.position.start.offset` only transiently, to recover list bullets and for shortcuts; offsets are not stored on Lexical nodes [D4].
- Measured with `MdastCommonMarkExtension` + `MdastGfmExtension` + `MdastExtension`: 5 of 17 identical. Differences: `[[` becomes `\[\[`, `*x*` becomes `_x_`, `**x**` becomes `__x__`, trailing spaces are trimmed, blank-line runs collapse, CRLF becomes LF, the final newline is dropped, `<!-- -->` becomes `\<!-- -->`, `*` and `+` lists become `-`, and a blank line is inserted after a heading [D39].

## Front matter untouched

- Neither `lexical`, `@lexical/markdown` nor `@lexical/mdast` has a front-matter concept; `@lexical/mdast` depends on no front-matter micromark/mdast extension [D3, D4, D1]. The adapter has to split it off (see the whole-file measurement above) [D39].
- Per-node or per-range read-only: the docs pages on nodes, editor state and commands describe only the global `editor.setEditable(boolean)`, TextNode `token` mode ("can't change its content and is deleted all at once") and decorators [D9, D10, D12]. Decorator DOM is always set to `contenteditable="false"` by the reconciler [D5].
- For a Source mode in the same component: `@lexical/plain-text` exists [D1]. Headless data-model check: a file stored as one `ParagraphNode` of `TextNode` lines separated by `LineBreakNode`s returns `getTextContent()` equal to the file for 17 of 17 corpus files (CRLF and no-final-newline included). Typing, paste and select-all in a real DOM were not tested [D39].
- Not settled by sources: front matter surviving typing, delete at body start, select-all-and-type, paste and undo (see Unknowns).

## Inline link chips

- `DecoratorNode`: "wrapper node to insert arbitrary view (component) inside the editor"; rendering is framework-agnostic [D10].
  - `decorate(editor, config)` returns a value stored in the editor's decorator map (`getDecorators()`), delivered by `registerDecoratorListener` [D5].
  - `isInline()` defaults to true, `isKeyboardSelectable()` to true, `isIsolated()` to false. Default: the caret can step over the decorator and select it. An isolated decorator "can not be traversed, extended over, selected as a node, or deleted by an adjacent caret operation" [D5].
- Markdown export calls `getTextContent()` for decorator children (after text-match transformers), so a chip node that stores its source string exports it verbatim [D3, D39].
- Markdown import of a chip goes through a text-match transformer (`importRegExp` + `replace(textNode, match)`) [D3].
- `DecoratorTextNode` (`@lexical/extension`, since 0.41.0) gives a decorator an inline text format through NodeState [D7, D2]. The Markdown exporter writes `getTextContent()` for decorators and does not apply a format to it [D3].
- `registerMutationListener(klass, listener)` reports created/updated/destroyed nodes; `getElementByKey(key)` returns the DOM element; its doc comment names attaching DOM listeners as a use [D5].
- A `[[` popover: `LexicalTypeaheadMenuPlugin` and `useBasicTypeaheadTriggerMatch` are in `@lexical/react` only; the other packages I read ship no typeahead trigger matcher [D8].
- Composition next to a chip-like node:
  - #6296 (Chinese input after a mention corrupted the entity) closed 2026-07-05, fixed by #8741 (`$handleCompositionStart` redirects composition away from token and segmented nodes) [D31].
  - #8507 (Japanese IME data lost right after a token-mode TextNode) closed 2026-05-13 as not reproducible on macOS Chrome 148; the closing comment says the `$handleCompositionStart` heuristic does not cover that caret position [D31].
- 0.53.0 lists decorator fixes: mouse drag across inline decorators (breaking-change entry #9329), caret after an inline decorator on list removal (#9300), deleting a line that starts with an inline decorator (#9301), deleting to a soft-wrapped boundary (#9321) [D2].

## Callouts with their own look

- `ElementNode` override points: `createDOM`, `updateDOM`, `getDOMSlot` ("control where its children are inserted into the DOM, e.g. to add a wrapping node or accessory nodes before or after the children"), `canBeEmpty`, `canInsertTextBefore/After`, `isInline`, `isShadowRoot`, `insertNewAfter`, `collapseAtStart`, `excludeFromCopy`, `extractWithChild` [D5, D10].
- A shadow root is an ElementNode that behaves like the root; `getTopLevelElement()` inside a `TableCellNode` returns the first child under the cell [D5].
- Stock `QuoteNode` (`@lexical/rich-text`) is the target of the `QUOTE` transformer; `[!voice]` stays literal text in it. The stock packages ship no callout or admonition node [D3, D2].
- `@lexical/mdast` offers `MdastShadowRootQuoteExtension`, "opt-in: blockquotes as block containers (full-fidelity nested content)" [D4].
- Round trip of stock `QuoteNode` callouts (voice and note files) in preserve mode: byte-identical; default mode escapes `[!voice]` [D39].
- Developer words flagged inline: no decoration or overlay API (styling ranges without changing the tree) appears in the docs read. Range-level marking in Lexical is by nodes: `MarkNode` (`@lexical/mark`, an inline `ElementNode`) and `HashtagNode` (`@lexical/hashtag`, a `TextNode` subclass); `registerNodeTransform` exists to produce such nodes [D44, D5].
- `Design note:` lines are ordinary paragraph text in the stock converter [D3].

## Beat headings with attribute lines

- `## …` imports as `HeadingNode`; `key:: value` lines are plain paragraph text in the stock converter [D3].
- Default mode inserts a blank line between a beat heading and its attribute lines on export; preserve mode leaves `S-004.md` and `S-901.md` identical [D39].
- Typed per-node data: NodeState on any node, serialized under `"$"` [D2].
- Stable block ids: experimental `BlockIdExtension` in `@lexical/extension` 0.53.0 gives each block an id (NodeState `blockIdState`) that "survives serialization, reloads, undo/redo, the Lexical clipboard and collaboration" [D2].
- Showing something on a block: `registerMutationListener` plus `getElementByKey`, or custom DOM in `createDOM` / `getDOMSlot` [D5].

## Spell check and IME

Spell check
- The root is a native `contenteditable`; text is real DOM text [D5].
- The only `spellcheck` attribute in `lexical` core source is in `TextNode.createDOM`, which sets `spellcheck="false"` on text with the `code` format; `@lexical/rich-text`, `-extension`, `-markdown` and `-link` have none. The React `ContentEditable` element has a `spellCheck` prop defaulting to `true` [D5, D8].
- No documented per-node or per-range spell-check switch was found in the docs read [D10].
- Before 0.53.0 the reconciler wrote text with `nodeValue`; in Chromium this dropped the spelling underlines of the whole text node on Backspace, after accepting a suggestion, after a transform or any programmatic update. Fixed in PR #9354 (merged 2026-10-19), shipped in 0.53.0 [D30, D2].
- 0.53.0 text write path (`$setTextContent`): while composing, and in Firefox and Chrome, only the changed part of the text is written (`replaceData`). Other engines keep the full `nodeValue` replacement; the source comment gives the reason for WebKit ("Safari, every iOS browser, WebKitGTK"): it "doesn't drop a marker that an insertion lands inside, so a partial write could leave an underline on a word that is now spelled correctly" [D5].
- Open: #4103 "No way to add Grammarly-like contextual spell-check" (2023-03-12), #4792 "Spellcheck clears all misspelled words" (2023-07-17), #5086 embedded link removed when text is corrected with Grammarly on Safari (2023-10-05) [D21].

IME and composition
- Lexical handles composition itself: `COMPOSITION_START_COMMAND`, a composition key set at the start, and a ZWSP heuristic when composition starts on an element anchor (an empty paragraph) [D7]. `@lexical/extension` `IMEExtension` exposes `compositionKey` and `composingTextNode` signals [D7].
- History tags composition updates (`COMPOSITION_START_TAG`, `COMPOSITION_END_TAG`), discards composing-character updates and merges a composition into one entry [D6].
- `composition` label: 51 issues, 3 open as of 2026-10-31: #4753 (Korean cursor position in some fonts, 2023-07-07), #6377 (CJK in Android Firefox, 2024-07-08), #6940 (iOS dictation deletes words, 2024-12-11). Open and unlabelled: #8834 (committing a Korean composition by mouse click scrolls the viewport, 2026-07-14) [D32, D21, D25].
- Dead keys and German or Swiss layouts:
  - #9289: composing into a `TextNode` subclass replaced it with a plain `TextNode`, reproduced with Japanese IME and with Option+E then E on macOS; closed 2026-10-01, fixed by #9291 in 0.53.0 [D28, D2].
  - #7026: German keyboard backtick (a dead key) broke inline-code shortcuts; closed 2026-05-13. A maintainer comment in it says tests for non-US layouts are hard "since playwright does not support that well" [D26].
  - #8697: dead keys on a new paragraph were removed in Firefox; closed 2026-06-15 [D27].
  - #8503 (0.45.0, labelled breaking) applies Markdown shortcuts on composition-committed triggers [D2].
- WebKit:
  - PR #9362 (merged 2026-10-20, in 0.53.0): cancelling an unconfirmed macOS system Pinyin composition with Backspace or Escape could leave desktop WebKit composing so the next Enter never reached `KEY_ENTER_COMMAND` [D29].
  - Closed: #8153 (Safari macOS, selected text not deleted after IME input, 2026-02-20), #8596 (Tab with Korean IME froze the editor in Safari, 2026-05-30) [D33].
  - WebKit bug 304068 (aborting a dead-key composition in desktop Safari gave an empty `compositionend` `event.data`) was filed 2025-12-12 and is marked RESOLVED FIXED [D43]. It applies to any `contenteditable` editor.
- Browser detection: `IS_SAFARI = /Version\/[\d.]+.*Safari/ && !IS_ANDROID`; `IS_APPLE_WEBKIT = /AppleWebKit\/[\d.]+/ && IS_APPLE && !IS_CHROME`; `IS_APPLE = /Mac|iPod|iPhone|iPad/` on `navigator.platform`. Composition code branches on `IS_SAFARI || IS_APPLE_WEBKIT` and `keyCode === 229` [D5].
- Stated browser support: Chrome 86+, Edge 86+, Firefox 115+, Safari 15+; the page does not mention WebView, WKWebView, WebKitGTK, Electron or Tauri [D16].
- Embedded-webview issues: #9065 is the only Tauri one (open; see below). A search for WKWebView / WebKitGTK / webview / WebView2 returns 7 issues, none about WebKitGTK; the open ones are #9065, #7886 (Android WebView) and #7106 (Safari selection) [D33].

## Long documents

- Maintainer statement on #8743 (2026-06): "Lexical currently has no built-in virtualization. If it's in an editor attached to the DOM as a node then the node has DOM associated with it." and "Most people use lexical with much smaller documents" [D22].
- Numbers posted in #8743:
  - Contributor benchmark in real Chromium, 107k nodes (1000 pages): EditorState construction 146 ms; with reconciler (DOM creation) 211 ms; with transforms and history 225 ms; browser reflow 269 ms [D22].
  - User report: ~35 MB, 519,352 nodes, import about 30 s; one typing trace spent about 2.3 s in browser layout and about 75 ms in `$reconcileRoot` [D22].
  - Pasting 20,000 identical paragraphs took about 4 s in Chrome on a MacBook M4 (playground) [D22].
  - The maintainer suggested checking CSS layout cost and hiding off-screen blocks with `display: none` before any reconciler change; he called the playground a poor testbed [D22].
- Headless Node 24.19 timings, no DOM, so they exclude reconcile and layout. Input is the S-901 and voices bodies repeated; stock `TRANSFORMERS` [D39]:

  | Size | Top-level blocks | Mode | Import | Export |
  |---|---|---|---|---|
  | 5 KB | 117 | preserve | 6 ms | 1 ms |
  | 245 KB | 5,685 | preserve | 61 ms | 10 ms |
  | 2,444 KB | 56,783 | preserve | 509 ms | 59 ms |
  | 245 KB | 2,352 | default | 46 ms | 10 ms |
  | 2,444 KB | 23,496 | default | 534 ms | 69 ms |

- Reconciliation is batched with `queueMicrotask`; `editor.update(fn, {discrete: true})` reconciles synchronously [D19].
- 0.45.0 changelog performance items: reconciler children fast path (#8482), copy-on-write maps for the node map and key-to-DOM map (#8481); 0.49.0 added editor-operation benchmarks (#8856) [D2].
- History keeps one `EditorState` per undo entry; `maxDepth` defaults to `null` (unbounded) [D6].
- Open: #5683 "Slow Performance in Safari with Large Character Counts" (Lexical 0.13.1 report, 2024-03-04, last update 2025-10-14); #5139 "Optimize the time to delete a large number of custom nodes" (2023-10-18) [D21].

## Svelte 5 integration

Bindings and examples
- The core is framework-agnostic and `@lexical/react` is the official binding; no official Svelte package exists (`@lexical/svelte` is a 404 on npm) [D15, D20, D1].
- Vanilla usage: `defineExtension` and `buildEditorFromExtensions`, `editor.setRootElement(el)` / `setRootElement(null)`, `editor.dispose()` for teardown; `RichTextExtension`, `HistoryExtension` as dependencies [D14, D17].
- Official Svelte 5 example in the repo, `examples/extension-sveltekit-ssr-hydration` (Svelte ^5.53.5, package version 0.53.0): calls `editor.setRootElement(editorRef)` inside `$effect`; Extension signals "comply with the svelte store API" and are read with a `$` prefix in the template; the editor is built outside `.svelte` files "to avoid the svelte/dollar_prefix_invalid compiler error" because Lexical's `$`-prefixed functions clash with Svelte's `$` namespace; `.svelte` files use `import * as` to reach them [D37].
- `@lexical/extension` depends on `@preact/signals-core` ^1.14.4 [D1].
- Community `svelte-lexical` 0.6.5 (npm 2026-07-15), peer `svelte ^5.0.0`, MIT, 601 stars, 18 open issues [D34, D35].
  - The npm package pins `lexical` and every `@lexical/*` at 0.40.0; the master branch pins 0.41.0 [D34, D35].
  - 266 commits since 2025-10-31, 207 by one author and 58 by a second [D35].
  - 5,202 npm downloads in the week 2026-10-23 to 2026-10-29, against 8,356,301 for `lexical` [D41].
  - Its docs describe Svelte 5 decorator rendering with `mount` from `svelte`, a mutation listener plus `registerDecoratorListener`, a component cache keyed by node key, and props held in a `$state` rune [D35].
  - Open issues include "Support for Slash Command & Mentions" (#140, 2025-11-05), "Hashtags and mentions" (#48, 2023-07-05), "History Persistence" (#167, 2026-07-01) [D35].
- Other community packages: `@hvniel/lexical-svelte` 0.1.1 (2026-08-09, lexical 0.49.0, peer `svelte ^5.56.0`); `lexical-svelte-runes` 0.1.4-beta.89.1 (2025-05-17, lexical ^0.30.0); `lexical-svelte` 1.0.0-beta.4 (2022-05-17) [D36].
- Lexical's upgrade guide: all packages are released at one version and "an app must resolve exactly one copy of each package" [D18].

Mount and unmount design-system components inside the editor
- A decorator's DOM host comes from `createDOM`; `decorate()` supplies the data; the host (here a Svelte component) renders via `registerDecoratorListener` and `registerMutationListener`, and `getElementByKey` returns the element [D5, D10, D35].
- svelte-lexical's doc records that rendering a component fully inside `createDOM` or `decorate` failed when a nested editor (an image caption) started a reconcile inside a reconcile, because the reconciler keeps the active editor in a global; it then moved to the decorator-listener route [D35].

Keymaps and commands
- Core dispatches `UNDO_COMMAND` on Ctrl+Z (Cmd+Z on Apple), `REDO_COMMAND` on Shift+Ctrl/Cmd+Z and on Ctrl+Y (non-Apple) via a `CONTROL_OR_META` mask (`ctrlKey: !IS_APPLE`, `metaKey: IS_APPLE`) [D5].
- `KEY_MODIFIER_COMMAND` is dispatched for modifier keydowns [D5].
- Experimental `KeyboardShortcut` data (key, modifier mask with `'any'`, command, `$disabled`, `$dispatch`) and `KeyboardShortcutsExtension` in `@lexical/extension`, with an `event.code` fallback for non-Latin layouts [D5, D7].
- Commands: `createCommand`, `editor.registerCommand(cmd, handler, priority)` (return `true` to stop propagation), `editor.dispatchCommand`; ten priority levels [D12].
- Not settled by sources: whether the native Ctrl/Cmd+Z handling can be switched off so the app keymap owns it (see Unknowns).

External change as a minimal edit
- No built-in text diff or minimal-edit apply. `$convertFromMarkdownString` clears the root and moves the selection to the start; `editor.setEditorState(editor.parseEditorState(json))` replaces the whole state [D3, D14].
- The Svelte example applies a replaced state with `HISTORY_MERGE_TAG` [D37].

History
- `HistoryExtension` / `registerHistory(editor, historyState, delay, now, onChange, maxDepth)`: merge window `delay` defaults to 300 ms, `maxDepth` defaults to unbounded [D6, D14].
- `HistoryState` is a plain object `{current, undoStack, redoStack}` that the host can create (`createEmptyHistoryState`) and hold [D6].
- `CLEAR_HISTORY_COMMAND` empties undo, redo and current and dispatches `CAN_UNDO_COMMAND(false)` and `CAN_REDO_COMMAND(false)`; `CLEAR_EDITOR_COMMAND` also clears it [D6].
- Update tags: `HISTORY_MERGE_TAG`, `HISTORY_PUSH_TAG` (always its own entry, since #9218), `HISTORIC_TAG` (set on undo/redo) [D6, D2].
- `canUndo` / `canRedo` signals on `HistoryExtension`; `CAN_UNDO_COMMAND` and `CAN_REDO_COMMAND` are deprecated [D13].
- The history source has no named save-point or marker API [D6].

## Licence

- `lexical` is MIT (npm and GitHub SPDX); the LICENSE reads "Copyright (c) Meta Platforms, Inc. and affiliates" [D1, D20, D5].
- Installing `lexical`, `@lexical/markdown`, `history`, `rich-text`, `list`, `link`, `extension`, `utils`, `selection` and `code` at 0.53.0 pulls 19 packages, all MIT (including `@preact/signals-core` and `@types/trusted-types`). Adding `@lexical/mdast` brings it to 75 packages, all MIT. I read the `license` field in each `package.json`; I did not run `pnpm licenses list` [D42].
- `@lexical/react` adds `@floating-ui/react` ^0.27.19; its licence was not read [D1].
- A grep for `fetch(`, `XMLHttpRequest`, `sendBeacon` and `WebSocket(` over the `src/` of lexical, markdown, mdast, history, extension, link, rich-text, list, html, code-core, utils, clipboard, react, hashtag, headless and internal returned no match; no other search for network or telemetry code was run [D42].
- The Lexical website and playground added Vercel Analytics (#8796); that is site code, not a published package [D2].
- Collaboration needs a Yjs provider chosen by the app; no paid tier or hosted service was found for any feature in this dossier [D2, D20].
- Published as ESM only since 0.51.0 (#9127) [D2].

## Maintenance

Releases
- Stable releases in the 12 months to 2026-10-31: 0.39.0 (2025-12-10), 0.40.0 (2026-02-02), 0.41.0 (02-25), 0.42.0 (03-19), 0.43.0 (04-09), 0.44.0 (04-27), 0.45.0 (05-28), 0.46.0 (06-26), 0.47.0 (07-09), 0.48.0 (07-16), 0.49.0 (07-30), 0.50.0 (09-02), 0.51.0 (09-17), 0.52.0 (09-28), 0.53.0 (10-28): 15 releases. 0.38.2 (2025-10-31T00:52Z) sits at the boundary. Gaps: 40 days from 0.38.2 to 0.39.0, 54 days to 0.40.0, then 7 to 34 days between releases [D1].
- Nightly builds are published on weekdays (`0.53.1-nightly.20261030.0` is the newest) [D1].

Versioning and breaking changes
- Still 0.x. The upgrade guide: "Lexical is still 0.x, so breaking changes ship in minor releases"; all packages share one version [D18].
- Counting changelog lines labelled "breaking" per release, 0.39.0 to 0.53.0: 0, 0, 1, 2, 1, 2, 7, 6, 2, 0, 2, 2, 2, 3, 3 (total 33; 12 of the 15 releases have at least one). 0.53.0 also has a structured "Breaking changes" section with 4 entries (#9329, #9478, #9316, #9479) [D2].
- Recent examples: ESM-only publishing (0.51.0, #9127), removal of `LexicalNode.getCommonAncestor` (0.52.0, #9238), removal of deprecated extensions (0.53.0, #9316), node classes ported to the `config()` protocol (#8640), React 17 dropped (#8782) [D2].
- From 0.23.0 each changelog version lists Breaking changes (with Detect and Migration), Behavior changes and Deprecations [D18].
- Several newer APIs are marked experimental: `@lexical/mdast`, `KeyboardShortcut`, `BlockIdExtension`, node `$config` schemas, named slots [D4, D5, D2, D10].
- 1.0: no issue with "1.0" in its title; the 2023 roadmap wiki calls moving to 1.0 "really a judgment call by the core team" [D33, D40].

Activity and backing
- The repository is under the `meta` GitHub org; the npm `repository` field still says `facebook/lexical`; copyright is Meta Platforms. CODEOWNERS lists 7 handles; npm lists 6 maintainers [D20, D1].
- 951 commits on `main` since 2025-10-31 by 151 distinct author names. Top: Bob Ippolito 208, Gerard Rovira 114, mayrang 97, Sherry 79, dependabot 65 [D20].
- 23,990 stars, 2,277 forks. npm downloads for 2026-10-23 to 2026-10-29: `lexical` 8,356,301, `@lexical/markdown` 7,806,299, `@lexical/mdast` 19,504 [D20, D41].

Issues
- As of 2026-10-31: 352 open issues (144 without a label) and 93 open pull requests [D21].
- Closed issues: at least 61 in the last 30 days, 145 in 90 days, 510 in 365 days (a lower bound: counted among the 1,500 most recently created closed issues) [D21].
- Open issues by label: markdown 5, composition 3, decorators 3, history 4, selection 9, copy+paste 4, safari-bug 2, mac-bug 1, windows-bug 1, serialization 2, all-platforms-bug 23 [D21].

## Known open issues

All states as of 2026-10-31.
- Bold or italic text ending in punctuation before a letter exports as Markdown that imports as plain text · https://github.com/meta/lexical/issues/9660 · open, opened 2026-10-31 · export fidelity, criterion 1 [D23]
- New lines in tables are not properly transformed to Markdown · https://github.com/meta/lexical/issues/9323 · open, opened 2026-10-04 · GFM table in the corpus, criterion 1 [D33]
- Preserve alignment when converting to Markdown · https://github.com/meta/lexical/issues/7555 · open, opened 2025-05-21 · criterion 1 [D33]
- Customize Markdown nested list spacing · https://github.com/meta/lexical/issues/2213 · open, opened 2022-05-18 · nested-list indentation is rewritten on export, criterion 1 [D33]
- Support for Markdown directive element syntax · https://github.com/meta/lexical/issues/8903 · open, opened 2026-08-01 · no native callout syntax, criterion 4 [D33]
- Committing an IME composition with a mouse click scrolls the viewport back · https://github.com/meta/lexical/issues/8834 · open, opened 2026-07-14 · IME, criterion 6 [D25]
- Cursor positioning problem during composition in specific fonts (Korean) · https://github.com/meta/lexical/issues/4753 · open, opened 2023-07-07, updated 2026-05-05 · IME, criterion 6 [D32]
- iOS dictation deletes words after completion · https://github.com/meta/lexical/issues/6940 · open, opened 2024-12-11 · composition label; mobile, not a P10 target, criterion 6 [D32]
- CJK composition broken in Android Firefox · https://github.com/meta/lexical/issues/6377 · open, opened 2024-07-08 · composition label; not a P10 target, criterion 6 [D32]
- Text selection on Safari has an extra empty selection area · https://github.com/meta/lexical/issues/7106 · open, opened 2025-01-28 · WebKit selection, criteria 3 and 6 [D33]
- Can't scroll on block dragging in macOS webview (Tauri) · https://github.com/meta/lexical/issues/9065 · open, opened 2026-08-12, Lexical 0.49 and Tauri CLI 2.11.4 · WKWebView, criteria 6 and 8 [D24]
- Scaling Lexical for large documents (no built-in virtualization) · https://github.com/meta/lexical/issues/8743 · open, opened 2026-06-24 · criterion 7 [D22]
- Slow performance in Safari with large character counts · https://github.com/meta/lexical/issues/5683 · open, opened 2024-03-04 · criterion 7 [D21]
- Optimize the time to delete a large number of custom nodes · https://github.com/meta/lexical/issues/5139 · open, opened 2023-10-18 · criterion 7 [D21]
- Decorator block nodes selection selects all nodes or nothing · https://github.com/meta/lexical/issues/3748 · open, opened 2023-01-18, updated 2026-10-28 · decorator selection, criterion 3 [D21]
- Caret at the beginning of an inline DecoratorNode with up/down keys · https://github.com/meta/lexical/issues/5312 · open, opened 2023-12-25 · chips, criterion 3 [D21]
- Unable to move between consecutive decorator nodes on a mobile device · https://github.com/meta/lexical/issues/4859 · open, opened 2023-08-04 · chips, criterion 3 [D21]
- Let an inline ElementNode opt out of having a range's end moved into it when selection is read back · https://github.com/meta/lexical/issues/9290 · open, opened 2026-09-30 · inline element behaviour, criteria 3 and 4 [D33]
- Arrow-key navigation for DecoratorNodes differs between RichTextPlugin and PlainTextPlugin · https://github.com/meta/lexical/issues/4214 · open, opened 2023-03-27, updated 2026-09-30 · Source mode vs Write mode, criteria 2 and 3 [D21]
- An arrow key on a node-selected decorator sends typed text to the top (plain-text) · https://github.com/meta/lexical/issues/9282 · open, opened 2026-09-30 · criteria 2 and 3 [D21]
- No way to add Grammarly-like contextual spell-check · https://github.com/meta/lexical/issues/4103 · open, opened 2023-03-12 · criterion 6 [D21]
- Spellcheck clears all misspelled words · https://github.com/meta/lexical/issues/4792 · open, opened 2023-07-17 · criterion 6 [D21]
- Async updates cause unwanted history · https://github.com/meta/lexical/issues/4171 · open, opened 2023-03-21 · history, criterion 8 [D21]
- Undo of a step that started from a node selection puts back a caret, not the node selection · https://github.com/meta/lexical/issues/9572 · open, opened 2026-10-30 · history, criterion 8 [D21]
- svelte-lexical: Support for Slash Command and Mentions · https://github.com/umaranis/svelte-lexical/issues/140 · open, opened 2025-11-05 · `[[` popover on the Svelte binding, criterion 8 [D35]

## Unknowns

- IME and dead keys in WKWebView and WebKitGTK. Lexical's tests and issues name Safari, iOS, desktop WebKit and Chromium, but no source says whether Tauri's WKWebView or WebKitGTK user agents match `IS_SAFARI` / `IS_APPLE_WEBKIT` (WebKitGTK is not Apple, so `IS_APPLE_WEBKIT` is false there; the source comment on text writes does name WebKitGTK [D5]), and the issue search found no WebKitGTK issue. A prototype must type Swiss and German dead keys and a macOS IME next to a chip and at a callout head in Tauri on macOS and Linux, and log `navigator.userAgent` there.
- How much custom code makes the stock Markdown path byte-identical at span level. Measured here: stock converter and one naive chip. Not measured: `_` and `__` emphasis, nested-list indentation, renumbered lists, `***`, backslash escapes, emphasis around a chip, callout and beat nodes as custom nodes, and the harness oracle under typing, paste and undo.
- Real typing and scroll latency on the large (≈250 KB) and stress (≈2.5 MB) files in each webview. The headless numbers exclude reconcile and layout, which maintainers and a user report as the dominant costs.
- Front-matter protection inside one editor (typing, delete at the body start, select-all-and-type, paste, undo): no source settles how a token node, a decorator or a transform behaves in each case.
- Applying an external change as a minimal edit that keeps caret and undo history: no built-in diff; whether a text diff mapped to nodes is practical was not tried. Also whether the host can mark a save point in history beyond clearing it.
- Whether the app keymap can own Ctrl/Cmd+Z, Ctrl+Y and Shift+Ctrl/Cmd+Z, which the core handles natively; no source read shows a switch to turn that off.
- Live preview in Write mode (raw Markdown of the construct under the caret): no Lexical feature found; the only Markdown example is a split view (WYSIWYG left, Markdown preview right) [D38]. The cost of swapping nodes on selection change was not measured.
- Whether `spellcheck="false"` set in the `createDOM` of a custom text or element node (ids, attribute lines) survives reconciler updates in each webview (core sets it only for code-format text), and whether native spell check skips chips (decorator DOM is `contenteditable="false"`).
- How well Lexical 0.53 and the Svelte community packages combine: `svelte-lexical` pins 0.40.0 to 0.41.0 while one copy of each Lexical package is required; whether using only the core with our own glue (as the official example does) is enough was not tried.
- Meta's staffing and funding: sources show the `meta` org and Meta copyright but not the employers of the top committers.

## Sources

- D1 · https://registry.npmjs.org/lexical (read with `npm view lexical version time dist-tags license repository.url maintainers --json`; also `npm view` for `@lexical/*@0.53.0`, `svelte-lexical`, `@hvniel/lexical-svelte`, `lexical-svelte-runes`, `lexical-svelte`, `@lexical/svelte`) · npm registry metadata · read 2026-10-31
- D2 · `package/CHANGELOG.md` in the npm tarball of lexical@0.53.0 (`npm pack lexical@0.53.0`) · Lexical CHANGELOG v0.38.0 to v0.53.0 · read 2026-10-31
- D3 · `package/src/` (index.ts, MarkdownImport.ts, MarkdownExport.ts, MarkdownTransformers.ts) in the tarball of @lexical/markdown@0.53.0 · source · read 2026-10-31
- D4 · `package/README.md`, `src/MdastExtension.ts` and `package.json` in the tarball of @lexical/mdast@0.53.0 · source · read 2026-10-31
- D5 · lexical@0.53.0 tarball `package/src/`: nodes/LexicalDecoratorNode.ts, nodes/LexicalElementNode.ts, LexicalReconciler.ts, LexicalEditor.ts, LexicalKeyboardShortcuts.ts, LexicalEvents.ts, environment.ts, LICENSE · source · read 2026-10-31
- D6 · `package/src/index.ts` in the tarball of @lexical/history@0.53.0 · source · read 2026-10-31
- D7 · `package/src/` (IMEExtension.ts, KeyboardShortcutsExtension.ts, DecoratorTextExtension.ts) in the tarball of @lexical/extension@0.53.0 · source · read 2026-10-31
- D8 · `package/src/` (LexicalContentEditableElement.tsx, LexicalTypeaheadMenuPlugin.tsx, LexicalTypeaheadMenuPluginUtils.ts) in the tarball of @lexical/react@0.53.0 · source · read 2026-10-31
- D9 · https://lexical.dev/docs/concepts/editor-state · Editor State · read 2026-10-31
- D10 · https://lexical.dev/docs/concepts/nodes · Nodes · read 2026-10-31
- D11 · https://lexical.dev/docs/packages/lexical-markdown · @lexical/markdown · read 2026-10-31
- D12 · https://lexical.dev/docs/concepts/commands · Commands · read 2026-10-31
- D13 · https://lexical.dev/docs/packages/lexical-history · @lexical/history · read 2026-10-31
- D14 · https://lexical.dev/docs/getting-started/quick-start · Quick Start (vanilla JS) · read 2026-10-31
- D15 · https://lexical.dev/docs/intro · Introduction · read 2026-10-31
- D16 · https://lexical.dev/docs/getting-started/supported-browsers · Supported Browsers · read 2026-10-31
- D17 · https://lexical.dev/docs/extensions/intro · Extensions introduction · read 2026-10-31
- D18 · https://github.com/meta/lexical/blob/main/packages/lexical-website/docs/upgrading.md · Upgrading Lexical (read through the GitHub contents API) · read 2026-10-31
- D19 · https://github.com/meta/lexical/blob/main/packages/lexical-website/docs/faq.md · FAQ · read 2026-10-31
- D20 · https://github.com/meta/lexical (repository metadata, README.md, .github/CODEOWNERS, commit list since 2025-10-31, via the GitHub API) · Lexical repository · read 2026-10-31
- D21 · https://github.com/meta/lexical/issues (open issues list, 352 items, exported with `gh issue list --state open --json number,title,labels,createdAt,updatedAt`; closed issues exported the same way, limit 1500; open PR count with `gh pr list`) · issue tracker data · read 2026-10-31
- D22 · https://github.com/meta/lexical/issues/8743 · Feature: Scaling Lexical for large documents using chunked multi-editor architecture · read 2026-10-31
- D23 · https://github.com/meta/lexical/issues/9660 · Bug: [lexical-markdown] Bold or italic text that ends in punctuation … · read 2026-10-31
- D24 · https://github.com/meta/lexical/issues/9065 · Bug: can't scroll on block dragging in macOS webview (tauri) · read 2026-10-31
- D25 · https://github.com/meta/lexical/issues/8834 · Bug: Committing an IME composition with a mouse click scrolls the viewport … · read 2026-10-31
- D26 · https://github.com/meta/lexical/issues/7026 · Bug: Broken inline code when using the German keyboard · read 2026-10-31
- D27 · https://github.com/meta/lexical/issues/8697 · Bug: Dead keys on a new paragraph get removed · read 2026-10-31
- D28 · https://github.com/meta/lexical/issues/9289 · Bug: Composing into a TextNode subclass replaces it with a plain TextNode … · read 2026-10-31
- D29 · https://github.com/meta/lexical/pull/9362 · Bug Fix: Finish cancelled desktop WebKit compositions · read 2026-10-31
- D30 · https://github.com/meta/lexical/pull/9354 · Bug Fix: Keep spellcheck underlines when a text node's text changes · read 2026-10-31
- D31 · https://github.com/meta/lexical/issues/6296, /8507 and /6492 (closing comments) · closed composition issues · read 2026-10-31
- D32 · https://github.com/meta/lexical/issues?q=is%3Aissue+label%3Acomposition · issues with label composition (51 results) · read 2026-10-31
- D33 · GitHub issue search pages on meta/lexical (queries: "dead key", Tauri, WKWebView OR WebKitGTK OR webview OR WebView2, Safari composition OR IME label:safari-bug, composition with decorator OR mention OR inline, `"1.0" in:title`, `is:open label:markdown`) · issue search results · read 2026-10-31
- D34 · https://www.npmjs.com/package/svelte-lexical (registry metadata via `npm view svelte-lexical --json`) · svelte-lexical 0.6.5 · read 2026-10-31
- D35 · https://github.com/umaranis/svelte-lexical (README.md, docs/decorator-node.md, packages/svelte-lexical/package.json on `master`, open issues, commit list since 2025-10-31) · svelte-lexical repository · read 2026-10-31
- D36 · npm registry metadata for `@hvniel/lexical-svelte`, `lexical-svelte-runes`, `lexical-svelte` (repository https://github.com/hanielu/lexical-svelte) · community Svelte packages · read 2026-10-31
- D37 · https://github.com/meta/lexical/tree/main/examples/extension-sveltekit-ssr-hydration (README.md, package.json, src/routes/+page.svelte, src/lib/buildEditor.ts) · Svelte 5 + Lexical Extension demo · read 2026-10-31
- D38 · https://github.com/meta/lexical/pull/8464 · Feature: Add markdown-editor split-view example · read 2026-10-31
- D39 · researcher probe, not reproduced; supporting evidence only · scripts rt.mjs, rt2.mjs, md.mjs, wl.mjs, edit.mjs, fm.mjs, plain.mjs and perf.mjs in the researcher scratch directory, not retained; Node v24.19.0; lexical, @lexical/headless, markdown, rich-text, list, link, code, table, mdast, extension at 0.53.0 (jsdom only for `createDOM` in the chip test). Input: the 17 files of `research/P10-prose-editor/corpus/records/` at HEAD 2c7906b, body = text after the second `---` line. Editor created with `createHeadlessEditor` or `buildEditorFromExtensions`, import in `editor.update(..., {discrete: true})`, export in `editor.read`. Modes: default and `shouldPreserveNewLines=true`; stock `TRANSFORMERS`; chip test adds one `WikiLinkNode` plus one text-match transformer · run 2026-10-31
- D40 · https://github.com/facebook/lexical/wiki/Lexical-Roadmap · Lexical Roadmap (wiki, last edited 2023-03-14) · read 2026-10-31
- D41 · https://api.npmjs.org/downloads/point/last-week/lexical (and `/@lexical/markdown`, `/@lexical/mdast`, `/svelte-lexical`) · npm download counts, 2026-10-23 to 2026-10-29 · read 2026-10-31
- D42 · researcher probe, not reproduced; supporting evidence only · `npm install` of the lexical and @lexical/* package sets named under Licence (0.53.0) into the researcher scratch directory, not retained, reading each `package.json` `license`; `grep` for `fetch(`, `XMLHttpRequest`, `sendBeacon`, `WebSocket(` over the extracted `package/src/` of the 15 packages listed · run 2026-10-31
- D43 · https://bugs.webkit.org/show_bug.cgi?id=304068 · Safari Desktop: When aborting a composition after pressing a dead key (e.g. ^) in oncompositionend · read 2026-10-31
- D44 · `package/src/` in the tarballs of @lexical/hashtag@0.53.0 (LexicalHashtagNode.ts, `class HashtagNode extends TextNode`) and @lexical/mark@0.53.0 (MarkNode.ts, `class MarkNode extends ElementNode`) · source · read 2026-10-31
