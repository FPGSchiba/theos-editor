Theos UI is the one design system for the Theos Editor and every tool that runs inside its shell: scene editing, the review queue, the AI monitor, the Content Studio and the TensionGraph (Magic) sandbox. It is Grid Noir, extended. The base is a cold blue-black, square and lined, with colour that always means something. On top of that base sit two registers: **Instrument**, for operating things, and **Manuscript**, for reading and writing them.

## Principles

1. **One shell, two registers.** Title bar, rail, windows, tabs and status bar never change. Only the content inside a window switches register.
2. **Colour is meaning.** The seven signal colours are sacred and mean the same thing everywhere. Kind colours, element colours and the in-world parchment never borrow them.
3. **Never colour alone.** Every signal also carries a word or a glyph: `PENDING`, `AI`, ✓, ✕, a kind glyph or a band name.
4. **Square and lined.** 1px `border` lines, no shadows, radius 0 in all chrome. The only softness is 2px on Manuscript content.
5. **Each face has one job.** JetBrains Mono for data and IDs, Inter for UI and developer prose, Spectral only for in-world voices.
6. **Dense where you operate, generous where you read.** Instrument packs 10–12px rows. Manuscript gives text 15–17px, a 1.6 line height and a 70ch measure.

## The two registers

| | Instrument | Manuscript |
| --- | --- | --- |
| For | scene editing, review, AI monitor, debugging, front matter, validation, Magic sandbox chrome | lore codex, story scripts, perspectives: any long text a person reads or writes |
| Faces | Inter (labels, buttons) + JetBrains Mono (values) | Inter for developer prose, Spectral for in-world voices, Mono for IDs |
| Case | UPPERCASE labels, tabs and buttons | sentence case everywhere |
| Sizes | 8–14px (`inst-*`, `data*`) | 13–26px (`ms-*`, `iw-*`) |
| Line height | ~1.3 (data 16px on 10.5px) | 1.6 (24px on 15px, 28px on 17px) |
| Measure | the panel | `measure-prose` 70ch, `measure-inworld` 62ch |
| Radius | `radius-instrument` (0) | `radius-manuscript` (2px) on content surfaces |
| Ground | `panel`, `panel2`, `bg` wells | `panel`; voices on `panel-inworld` |

**When to use which**
- If a person scans it, compares it or acts on it, it is Instrument. If they read it from start to end, it is Manuscript.
- Chrome is always Instrument, in every window, even one that shows a lore record.
- Set the register on the content, not the window: `<div class="tu-window__body tu-manuscript">`.
- Some parts of a record are data and stay Instrument even inside Manuscript: front matter (`tu-fm`), validation (`tu-val`), beat parameters (`tu-beat`), the top line of the record header.
- The record title (`ms-title`) is the hinge: a Manuscript title on top of an Instrument ID line.
- "Mono everywhere" (`data-tu-font="mono"` on the root) switches the whole Instrument register to JetBrains Mono. Manuscript keeps its proportional faces.

## Colour

### Signals (sacred)

| Token | Alias | Means | Typical use |
| --- | --- | --- | --- |
| `amber` #ff8b2e | `sig-attention` | **Primary: human attention needed** | primary button, active tab/rail/selection, pending review, focus ring, hot window |
| `magenta` #ff3ea5 | `sig-ai` | **AI-origin data** | AI-proposed commands, AI-written values, the `AI` marker |
| `cyan` #3ef0ff | `sig-info` | info and schema | type names, components, INFO, auto-approved |
| `green` #29e8a5 | `sig-approve` | approve, healthy | approve, approved, canon, pass, keep |
| `red` #ff516f | `sig-reject` | reject, error | reject, danger, ERR, broken link, kill |
| `yellow` #ffd54a | `sig-warn` | warn, idle | WARN, idle pips, mid significance |
| `violet` #7a6cff | `sig-script` | scripts and behaviour trees | BT nodes, script badges |

- Components reference the `sig-*` aliases, never the hue tokens directly. Text on any solid signal fill is `onFill` (= `bg`).
- Amber is the scarcest colour on screen. One primary button per panel, one hot window per shell.
- Magenta never appears without the AI marker (`tu-ai`) or an AI source next to it. A significance meter can turn an AI entry amber: high significance raises human attention, it does not change who wrote it.
- Record status reuses the review set: `proposed` is amber (it waits for a human), `canon` is green, `draft` is an outline.

### Surfaces, lines and text
`bg` (shell, wells, tab strips) → `panel` (windows) → `panel2` (headers, filter bars, popovers) → `panel3` (empty states). Lines are `border`, or `borderHi` for hover and floating edges. Text is `text`, `textDim` for secondary, and `textMute` for tertiary text only (see Known flags). Manuscript body text is `textProse`, a step below `text` to cut halation over long reading.

### In-world parchment
`ink-inworld` #dfd3bf on `panel-inworld` #19140e, with `ink-inworld-dim` for attribution and `line-inworld` for edges. It is warm and low in saturation (OKLCH C 0.03) and it is not amber. Use it only inside the voice block, and never for UI.

### Kind palette
Record kinds get nine quiet tones: OKLCH L 0.80, C 0.065, spread round the hue wheel, each about 10:1 on `panel` and at least ΔE 0.10 from every signal colour. The **glyph** identifies the kind. The colour only helps you scan.

`kind-world` (all ten world entities) · `kind-truth` (cold, like the shell) · `kind-belief` (warm, the in-world side of a truth) · `kind-clue` · `kind-perspective` · `kind-chronicle` · `kind-beat` · `kind-concept` · `kind-template` (near-neutral).

- Kind colours appear only as glyphs, hairlines and washes of 8–10%. Never as a solid fill.
- A kind is never a status. A proposed Belief shows the Belief glyph *and* an amber `PROPOSED`.

### Element bands: data only
The five Spell Appearance bands (`el-earth-*`, `el-water-*`, `el-fire-*`, `el-air-*`, `el-lightning-*`) colour the 3D stage and data visualisation. They never colour chrome, a control or a status. Several of them sit right next to signal hues:

| Element | Sits next to | ΔE |
| --- | --- | --- |
| Lightning hi | AI magenta | 0.06 |
| Fire hi | amber | 0.08 |
| Lightning lo | script violet | 0.09 |
| Air lo | cyan | 0.12 |

**How Lightning and AI-magenta stay apart.** Hue cannot separate them, so three other things do:
1. **Place.** Lightning lives on the stage and in charts. Magenta lives in chrome.
2. **Shape.** Lightning is always a ramp or a glowing round sample with its gold-white core. Magenta is always flat and square.
3. **Words.** Lightning is always labelled with its band name. Magenta always carries `AI`.

The same three rules separate Fire from amber and Air and Water from cyan.

## Typography

- **Inter** (variable, 100–900): UI labels, buttons and developer prose. Its fallback stack goes to JetBrains Mono next, so UI glyphs Inter lacks (▸ ▾ ⋯ ◇) still render from a shipped font.
- **JetBrains Mono** (full build, 400–700): every value, ID, path, log line and glyph icon.
- **Spectral** (400–600, with italics): in-world voices only. It was drawn for screens but keeps a book face's calligraphic stress, so an NPC's words read as written by someone and stay legible at 17px on a dark ground, where Garamond-style faces thin out.

Instrument scale: `inst-title` 32 · `inst-section` 14 · `inst-heading` 14 · `inst-body` 12 · `inst-panel-title` 10.5 · `inst-tab` 10 · `inst-button` 10.5 · `inst-label` 10.5/10/9 · `inst-chip` 9.5. Data: `data-lg` 14 · `data` 10.5 · `data-sm` 10 · `data-xs` 9 · `rail-label` 8.
Manuscript scale: `ms-title` 26/32 · `ms-h2` 19/26 · `ms-h3` 15/24 · `ms-body` 15/24 · `ms-meta` 13/20 · `ms-mono` 13/20. In-world: `iw-title` 22/28 · `iw-body` 17/28 · `iw-attribution` 14/20 italic.

All three families are under the SIL Open Font License 1.1.

## Spacing, radius, lines, elevation

- Spacing runs on a 2px base (`sp-2` … `sp-48`). Instrument uses 2–16px; Manuscript uses 12–48px. Tree indent is `sp-14` per level.
- Fixed sizes live in tokens: `h-control` 26, `h-tab` 28, `h-titlebar` 30, `h-toolbar` 42, `h-statusbar` 22, `w-rail` 54, `h-rail-item` 48.
- **Radius.** `radius-instrument` is 0 for all chrome in both registers, so the shell never changes shape when its content does. `radius-manuscript` is 2px and goes only on Manuscript content surfaces: the voice block, link chips in prose, inline code and the `[[` popover. 2px softens the HUD edge over long reading and stays inside the Grid Noir family. The Magic editor's 4–7px is retired because it read as a different app.
- Lines are 1px everywhere. Dashed lines separate property rows and mark "add" and "pending". Dotted lines indent review bodies.
- Elevation is colour, not shadow. The only shadows are `shadow-hot` (the one window that needs you) and `shadow-popover` (floating layers).

## Iconography

Theos UI draws its icons from Unicode glyphs in JetBrains Mono, as Grid Noir always has. There are no icon fonts, no SVG sets and no emoji.
- Ship the **full** JetBrains Mono (as in `fonts/`), not a Latin subset. Subsets drop the geometric shapes the UI relies on.
- Kind glyphs are set by CSS from `data-kind` (see KindGlyph): ▲ ≣ ⊟ ⌂ ∴ ◧ ◉ ◆ ⌸ ✶ for world entities, and ■ □ ◔ ◎ § ¶ ◫ ⊡ for Truth, Belief, Clue, Perspective, Chronicle, Script/Beat, Concept/Version, Template.
- UI glyphs: ▸ breadcrumb, ▾ collapse, ⋯ more, − ▢ window controls, ⤷ component child, ✓ ✕ verdicts, ⊕ mix, ⊘ locked, ⌘ ⇧ ⏎ keys.
- Flagged: the source rail glyphs ◴ (Review), ✱ (AI) and ⚙ (Settings) and the transport glyphs ⏸ ⏭ are not in JetBrains Mono. They fall back to the OS symbol font and may look different per platform. Keep them for now; replace them when the rail gets its final icon pass.

## Writing in the UI

- **Instrument labels** are short uppercase nouns: `REVIEW`, `FRONT MATTER`, `DESIGN REVIEW QUEUE`. **Buttons** are uppercase verbs with an optional shortcut: `APPROVE`, `SAVE SCENE ⌘S`, `OPEN IN BT VIEWER`.
- **Data** is mono and literal: `e#4f12`, `B-020`, `codex/beliefs/b-020.md`, `sig 0.91`. Join items with ` · `.
- **Manuscript** uses sentence case and full sentences. Headings name the point ("What the binders believe"), not the section type.
- **In-world text** never contains developer vocabulary: no IDs, tiers, truths or system words. Developer prose can name in-world things; in-world text cannot name developer things.
- **Validation messages** say what is wrong and what fixes it: "Broken link: “Song-cellar” does not exist." plus the rule `no-broken-links` and a `Create record` action.
- **Empty states** say what will appear and what to do: "New AI proposals land here. Nothing changes the world until you approve it."

## The Magic sandbox

The TensionGraph sandbox stays a separate tool inside the same shell.
- **It adopts** the surfaces, lines, signal colours, type, rail, windows, tabs, inspector rows and status bar. It drops its own graphite palette, its 4–7px corners and IBM Plex.
- **It keeps** its 3D stage and the element bands, as data.
- Token types (SpatialAnchor ◆, TemporalFrequency ◇, KinematicVector →) are told apart by glyph in `text` colour, not by amber and cyan.
- Readouts follow the signal set: S and η healthy in green, volatile in yellow or red. The material band of a node shows as a `tu-swatch-el` with its band name.
- Its status bar uses the editor format: `solver OK · 6 nodes · 7 edges · S 0.77 · η 0.82 · ω 0.30`.

## Do and don't

| Do | Don't |
| --- | --- |
| Put `tu-ai` beside every magenta element | Use magenta for decoration, emphasis or a gradient |
| Show a proposed Belief as □ glyph + amber `PROPOSED` | Tint the status chip with `kind-belief` |
| Draw Lightning as a labelled ramp with its core | Fill a button, tab or badge with an element colour |
| Set an NPC's words in the voice block (Spectral, parchment) | Set in-world text in Inter, or developer truth in Spectral |
| Use `ink-inworld` only inside `tu-voice` | Use amber for in-world text, or parchment for UI |
| Keep chrome square in a Manuscript window | Round the tabs or the window because the content is prose |
| Make one primary (amber) action per panel | Put two amber fills side by side |
| Mark the scene root bold `text` | Paint a row amber to make it look important |
| Colour type annotations cyan (schema) | Colour type annotations magenta |
| Keep the slider neutral and turn it amber only while dragging | Fill sliders with an amber-to-magenta gradient |
| Pair every pip with a label | Rely on a coloured square alone |

## Changes from the mockups

These fix places where the Grid Noir mockups broke their own colour rules:
- Inspector type annotations (`vec3`, `f32`): magenta → cyan.
- Console source column: magenta for every source → `textDim`, with magenta only for AI sources.
- Slider fill: amber-to-magenta gradient → neutral, amber while active.
- Scene root in the hierarchy: amber → bold `text`.
- Review entries: the AI-origin command name stays magenta, and now carries the `AI` marker.

## Known flags

Kept from the source and flagged:
- `textMute` is 2.7:1 on `panel`, which fails AA. Use it only for redundant text such as indices, timestamps and tags. Front-matter keys use `textDim` instead.
- `violet` is 4.5:1 on `panel2`, borderline. Prefer it on `panel`.
- `border` is 1.34:1. Lines are structural; nothing relies on a border alone to carry meaning.

## Consuming

Load `tokens.css` (generated from `tokens.json`), the fonts and `components/bundle.css`, then use the `tu-*` classes. The layer is framework-neutral: each `components/<Name>` maps to one Svelte component, and the classes, `is-*` states and `data-*` attributes are the contract. Use `is-attention`, `is-ai`, `is-info`, `is-approve`, `is-reject`, `is-warn` and `is-script` to set a signal on any chip, badge, pip, meter or filter.
