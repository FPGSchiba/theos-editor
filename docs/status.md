# Build status

Updated by Claude at every phase transition. Not a protected path.

States: `todo` → `spec-pr` → `spec-approved` → `in-progress` → `pr-open` → `done`.

| Phase | Name | Repo | State | Spec PR | Build PR |
|---|---|---|---|---|---|
| P00 | Foundation and gate | theos-editor | done | #5, #9, #10 | #7 |
| P01 | Content layout and templates | tower-of-theos | done | #4 | tower-of-theos#2 |
| P02 | Records and grammars | theos-editor | done | #16 | #17 |
| P03 | Kinds, index, search, lenses | theos-editor | done | #18 | #20 |
| P04 | Validation, lore rules, ledger, formalism mirror | theos-editor | done | #21 | #28 |
| P05 | CLI and hook | theos-editor | done | #35, #41 | #42 |
| P06 | Command layer and repo lock | theos-editor | done | #36 | #39 |
| P07 | Agent sessions, review, MCP propose | theos-editor | todo | | |
| P08 | Map format and terrain generator | theos-editor | done | #27 | #44 |
| P09 | Generator layer and generator tools | theos-editor | todo | | |
| P10 | Prose editor research | theos-editor | done | #3 | #11 |
| P11 | Editor shell and Codex | theos-editor | done | #56 | #67 |
| P12 | Manuscript editor | theos-editor | todo | | |
| P13 | Changes and Review screens | theos-editor | todo | | |
| P14 | Start and Settings | theos-editor | todo | | |
| P15 | Lenses, Scripts, Graph | theos-editor | todo | | |
| P16 | Generative screens | theos-editor | todo | | |
| P17 | AI drafts and semantic search | theos-editor | todo | | |
| P18 | Installer, first run, Code preset | theos-editor | todo | | |
| P19 | Comments | theos-editor | todo | | |
| P20 | Content layout for world and art kinds | tower-of-theos | done | #22 | tower-of-theos#3 |
| P21 | World kinds: enemies, spawns, quests | theos-editor | todo | | |
| P22 | Map pins, cameras and frames | theos-editor | todo | | |
| P23 | Map recipes and blockout generator | tower-of-theos | todo | | |
| P24 | Quest screen and flow editor | theos-editor | todo | | |
| P25 | Atlas | theos-editor | todo | | |
| P26 | Style bible and briefs | theos-editor | todo | | |
| P27 | Asset pipelines | theos-editor | todo | | |
| P28 | Workflow catalogue and asset guide | tower-of-theos | todo | | |
| P29 | Settlements and child maps | theos-editor | in-progress | #54 | |
| P30 | Levels in the generator | theos-editor | todo | | |
| P31 | Floor frame and places | theos-editor | todo | | |
| P32 | Recipe families | theos-editor | todo | | |
| P33 | Map metrics and batch tool | theos-editor | todo | | |
| P34 | Interiors | theos-editor | done | #55 | #69 |
| P45 | Atlas level switcher | theos-editor | todo | | |
| P46 | Git read | theos-editor | done | #52, #58 | #62 |
| P47 | Git write | theos-editor | spec-pr | | |
| P48 | MCP read | theos-editor | done | #53 | #61 |
| P49 | Releases and the check action | theos-editor | todo | | |
| P50 | Leaner CI | theos-editor | done | #45, #46 | #48 |
| P51 | Window chrome | theos-editor | spec-pr | | |

Rows P20–P28 come from umbrella version 2 (#19); P29, P30 and the new P08 and P23 from version 3
(decision 0003, #24). Rows P31–P34 and P45, and the new P29 and P30, come from umbrella version 4
(decision 0004, #31). Rows P46 and P47 and the new P06 come from umbrella version 5 (the split of
P06, #33). Rows P48 and P49 and the new P05 come from umbrella version 6 (the split of P05,
#34). Row P50 comes from umbrella version 7 (#45), amended by version 8. Row P51 comes from
umbrella version 9.

## Needs Jann

- P34 is merged (#69). Its spec erratum, `docs(spec): interiors as built (docs)`, waits for Jann:
  the drop check holds the start room's and the drop room's keys (§5.9 item 4, ruling 7; R54),
  `locked` is set for every room behind a lock (§5.4; R55), the leaf rule reads "non-locked"
  as the program's flag (§5.6; R57), and §5.12 shows the recipe values as built (R45, R48,
  R50). P29 records its golden hashes as `theos-maps` 1.2.0, since #69 recorded 1.1.0.
- P48 follow-up, seen on macOS in P34's PR run (28 Nov 2026): `theos-mcp`'s
  `project_info_over_stdio` fails there. The project root is reported as
  `/private/var/folders/…` while the test expects `/var/folders/…`, the same folder through
  macOS's `/var` link. Comparing canonical paths in the test fixes it. P34 leaves it to P48
  (Jann, 28 Nov).
- P48 is merged (#61). Open:
  - The recorded Claude Code session that spec §5.14 asks for as CE-18.1/CE-18.2 evidence is
    blocked: "MCP server blocked by enterprise policy: theos". Jann allows the server in the
    managed MCP policy, or runs the session himself. The stdio tests prove all three ACs.
  - P11 merged P48's early commits (257fd87); `main` now holds them, so P11 merges `main`.
- P11 (build PR, 28 Nov 2026):
  - Theos UI's light mode is on `main` as the theme `grid-light` (#63). P11 takes every colour
    from tokens and sets `data-theme="grid-noir"` on the root; the switch (P14) sets `grid-light`
    (P11 spec §5.7).
  - P11's gate PR #60 merged on 27 Nov 2026. The live ruleset switch that adds `ui` to the
    required checks (`cargo xtask github apply`) is Jann's.
  - macOS needs one manual start of the editor before P11 is done: the app menu (Quit, Edit, no
    "Close Window" on Cmd+W) and Cmd+W closing a record tab, not the window. The `platforms` run
    covers the menu's tests; the start itself cannot run in CI.
  - Spec follow-up (protected): spec §5.12 names the fixture
    `crates/theos-editor/tests/fixtures/codex/`; it lives in
    `crates/theos-studio/tests/fixtures/projects/editor-codex/`, since the studio's Codex views
    test on it too. A docs PR moves the path in the spec.
  - Spec proposal (protected): spec §5.11 gives the tree tabs Lore and Things Mod+Shift+3 and
    Mod+Shift+4. On macOS, Cmd+Shift+3 and Cmd+Shift+4 are the system's screenshot keys, which
    the system takes before the editor sees them. Proposal: other keys for the four tree tabs in
    §5.11, changed in a docs PR and then in `keymap.ts`; not Ctrl+Alt+digits, which are AltGr
    characters on many Windows layouts.
  - Spec erratum (protected): the build's sync reports files, not only ids (`SyncView`: changed
    and removed as `{id, path}`, moves as `{id, from, to}`), and `record_view` takes an optional
    path, so each file of a duplicated id opens on its own. Spec §5.4 and §5.9 still describe ids
    only; a docs PR brings them in line.
  - The manual run of the app on 27 Nov 2026 rewrote Jann's `%APPDATA%/theos/editor-state.json`
    (the last project); it names a scratchpad copy of the fixture until Jann opens another project.
- P08's build PR #44 is a draft labelled `needs-jann`. The final review loop reached its cap of
  five rounds with 6 Minor findings open. A targeted fix and review round, which Jann asked for
  on 25 Nov 2026, closed them. Its spec changes reached main with #40 (rulings 51–75) and #47
  (76–77), its gate change with #43. macOS stays unverified until the `platforms` run on #44's
  head.
- The SIL OFL 1.1 fonts (Inter, Spectral) that `theos-maps` embeds must ship with their licence
  texts (P08 §5.11, §7). `theos_maps::render::font_licences` hands them out; P09 or the release
  workflow adds them to the releases of the binaries that embed the crate.
- GitHub refused every Actions job from 13 Nov 2026 to 25 Nov: the included $18 was used up on
  3–12 Nov. Jann set an Actions budget of $15 on 25 Nov. P50 (Leaner CI) cuts the gate's
  minutes.
- P50 is merged (#48), and the `main` ruleset requires `gate`, `test`, `coverage`, `pr` and
  `approval`. Its proofs after the merge remain: the probe PRs and the speed measurements (spec
  §5.9, §5.10) and one manual Intel macOS run of `platforms`.
- Follow-up from P06: the class "a command changes which files are records without the verdict
  judging it" (m4-4, m5-1, and N1–N3 of PR #39) needs a general design. One option is a check
  after the write that compares the reindex with the preview and heals the cache; another is a
  P03 preview over the planned disk state. Rulings R16, R21 and R23 refuse the common cases
  today. R24–R26 accept the rare rest: a `.gitignore` behind a symbolic link, a hidden file
  restored visible by a Windows rollback, and new folders that differ only in case. The commit
  check stops any error those cases leave.
- For P49 (umbrella version 6):
  - before its gate PR: Jann creates a GitHub App for release-plz (contents and pull requests
    write), installs it on this repository and stores its id and key as secrets; the tag ruleset
    names it as the only actor that creates `v*` tags;
  - after the first release: Jann allows access to this repository's actions from his private
    repositories, adds a read-only token for this repository's releases as a tower-of-theos
    secret, and adds the check job to tower-of-theos;
  - Jann's first release, `v0.1.0-rc.1`, is the end-to-end proof of CE-08R.2–08R.4.
- Deferred by Jann on 12 Nov 2026 (decision 0004):
  - the shape of regions with a recommended power (D11): areas inside the floor's map (the
    working assumption) or separate maps; the power scale (working assumption: 1–5, like
    `threat`); authored in the map body or generated as a proposal;
  - who writes the band recipe families (D13); they live in `tower-of-theos`
    `tools/maps/recipes/`;
  - the research-track phases of umbrella §5 (P35–P40, P42–P44, requirements and trial sites,
    promotion at road junctions, regions and power), each until its sources are read and its
    probe has run.
- For P09's spec (decision 0004): the machine-aware memory budget for map runs (at least 3 GiB,
  more where the machine allows) covers all seeds running at once, so the built-in adapter starts
  only as many seeds in parallel as fit.
- The Maps screen reference (`docs/ui/screens/11-maps.html`) still says "theos_maps 0.3.0 ·
  Python 3.12 (uv)", describes a Python venv, and shows "2 extractions" and "extraction ↔
  extraction 99 min". P16's spec PR updates it to the built-in generator and the place types
  `chime`, `gate-up`, `gate-down` (decision 0004; G18). The Atlas tower reference
  (`13-atlas-tower.html`) shows `portal-in-1` and `portal-out-1`; P25's spec PR renames them.
- P00's macOS `platforms` evidence is still open: the run on #7 got no GitHub runner (macOS
  capacity). The weekly Monday run, or a re-run, gives it.
- Comments on text sections (decision 0002, #15, phase P19): before P19 starts, Jann settles the
  wording for G7 and CE-23.1 proposed in 0002 §10, which P19's agent tools need. P02 builds the
  UTF-16 positions 0002 asks for.
- Three sentences of the P10 corpus read like lore claims (B-901's link text, "the Cutter is
  near", "the operator-plate names a Listener"). P02's fixture copies reword them neutrally until
  Jann, or a session that can read Confluence, confirms they are canon.
- tower-of-theos#3 (P20, merged) locks `markdown-it-py` 4.3.0 in `tools/repo_checks`, which was
  released the day it was locked. Pinning an older 4.x is a one-line follow-up if Jann prefers a
  cool-down.
- Theos UI with Grid Light (branch `docs/theos-ui-grid-light`, 27 Nov 2026):
  - A user-facing theme setting is new scope. G13 makes the UI use `design/theos-ui/` and follow
    `docs/ui/`, so the editor must render in both themes, but no AC lets the user choose the
    theme. Proposed AC, under CE-42 as Jann asked: "CE-42.3 The user picks Grid Noir, Grid Light
    or the system's preference; the choice is a user setting, applies to every window at once,
    and 3D viewports and map stages stay Grid Noir." CE-42 is the Graph story of P15, settings
    are CE-39 (P14, `10-settings.html`), and P11's spec already leaves the light theme switch to
    P14, so CE-39 is the likelier home (as CE-39.5). Jann decides the place and the wording; no
    phase builds it before that.
  - Four component states in the design system's `bundle.css` put signal-coloured text on a
    tint of the same signal and miss 4.5:1: the active rail label (amber 4.30:1 in Grid Light),
    tinted chips (magenta 4.13 and cyan 4.27 in Grid Light), the review summary (amber 4.29 and
    cyan 4.27, "12 AUTO", in Grid Light) and a violet filter (4.22:1 in Grid Noir). The violet
    filter sits on the toolbar's `panel`, where plain violet reaches 4.86:1; its 13 % violet
    tint lightens the ground toward the violet text. The design system's own previews show the
    rail label, the magenta chip and the review summary. Only screens 12 and 13 use a violet
    filter, and the cyan chip fails only on the selected row's amber tint on screen 13 (Q-104);
    Q-101 and Q-103 on plain `panel` pass at 4.74:1. Tested fix for the artifact: text on those
    tints mixes 10 % toward `text`, for example `color-mix(in srgb, var(--c) 90%, var(--text))`.
    With it no text outside `textMute` fails in any screen or preview, in either theme, and the
    fixed states reach at least 4.78:1 (the AI-drafted chip is the lowest). `design/theos-ui/`
    mirrors the artifact, so the fix goes into the artifact and reaches the repo with the next
    refresh.
  - Measured per `bundle.css` rule in the screens and the previews (Grid Noir / Grid Light; text
    needs 4.5:1, icons and controls 3:1), the design system's components that set text in `textMute`
    miss 4.5:1 in both themes: inactive tabs (`.tu-tab`) 2.84 / 3.01; section-header actions and
    counts (`.tu-section__more`) 2.46 / 3.29; off filters (`.tu-filter.is-off`) and property keys
    (`.tu-prop__key`) 2.68 / 3.56; popover kind words and key hints (`.tu-pop__kind`)
    2.02–2.46 / 2.74–3.29 and the popover footer (`.tu-pop__foot`) 2.68 / 3.56; tree tags
    (`.tu-tree__tag`) and tree indices (`.tu-tree__idx`) 2.23–2.68 / 2.96–3.56; the empty beat value
    (`.tu-beat__param.is-empty .tu-beat__val`) 2.68 / 3.56; review and console timestamps
    (`.tu-review__time`, `.tu-console__time`) 2.42–2.68 / 3.19–3.56; input placeholders
    (`.tu-input input::placeholder`) 2.84 / 3.01; and the `.tu-mute` utility 2.46–2.84 / 3.01–3.56.
    The components' icons in `textMute` miss 3:1 in Grid Noir only, at 2.42–2.84 / 3.01–3.56: the
    window controls (`.tu-tabbar__ctrls`), the select caret (`.tu-select::after`), the input icon
    (`.tu-input__icon`), the ⋯ of section headers and the glyphs in tree tags and in empty beat
    values; the separators (`.tu-titlebar__sep`, `.tu-beat__sep`) are 2.68–2.84 / 3.01–3.56. Control
    edges in `border` miss 3:1 in both themes: unchecked check boxes, toggles that are off, inputs
    and selects, 1.24–1.34 / 1.38–1.50; the `textMute` knob of a toggle that is off misses it in
    Grid Noir only (2.46 / 3.29). No screen or preview uses the other `textMute` rules
    (`.tu-section__note`, `.tu-empty__mark`, the console's `DBG` level), so they are not measured.
    The README allows `textMute` only for redundant text, but a component gets whatever text a
    screen puts in it: the screens and the cards' previews put state words, reasons, hints and
    counts into these components ("needs an npc" in a tree tag on screen 15, "stage all" in a
    section header on screen 08) as well as indices, timestamps and tags that repeat an id. Whether
    a use is redundant depends on its text, not on the component, so the flag cannot settle a
    component as a whole. The previews also set text in `textMute` themselves, with `.tu-mute` and
    with KindGlyph's page style (its kind labels, 2.84 / 3.01). The PR of this branch lists every
    `textMute` text below its need, by screen and preview. Proposed for the artifact: these
    components set their text and icons in `textDim` and keep `textMute` for the `.tu-mute` utility
    and separators, or the README's flag names the components and the content each may hold. The
    locked tier option (`.tu-tier__opt.is-locked`, 2.68 / 3.56) is an inactive control, which WCAG
    1.4.3 exempts. Like the states above, these go to the artifact and are not patched in
    `design/theos-ui/`.
  - P12 should not copy the P10 prototype's `textMute` for front matter, raw source lines and the
    beat row's "+ add" (2.68 / 3.56 on `panel`, where `textDim` gives 5.56 / 6.71), in
    `research/P10-prose-editor/app/src/`: `adapters/codemirror/theme.ts`,
    `adapters/prosemirror/theme.ts` and `harness/styles.css`.

## Needs a private session (Confluence, Jira, plugin)

- CE-16S: write `tower-of-theos/canon/formalism/_source.json` from Shared Formalism, in the
  format of the P04 spec §5.14, then run `theos formalism` to generate `E1`…`E18`. P04 proves
  the mirror with a synthetic source until then.
- inworld-vocab: if Confluence or the theos-design plugin holds a list of developer words, copy
  it into `words` of `[validate.inworld-vocab]` in `tower-of-theos/.theos/settings.toml`
  (P04 spec §5.11, §5.12). Built in are only ids, tier tokens and rule names.
- CE-04 / CE-05: lore import and Seal split (umbrella §4).
- Readable copies in Confluence: the umbrella spec (version 2), the data model and the
  architecture page need the world and art additions once the docs PR is merged.
- Decision 0003 (map generator in Rust, merged in #24): the readable copies of the umbrella spec
  (version 3), the architecture, the data model and the generator protocol; a decision page for
  0003; and Confluence Decision 221741058 (the engine package format), which `docs/map-package.md`
  replaces (Jann confirmed on 10 Nov 2026 that it fixes nothing different).
- Decision 0004 (map generation research, merged in #31): the readable copies of the umbrella
  spec (version 4) and the generator protocol, and a decision page for 0004.
- The P05 split (merged in #34): the readable copies of the umbrella spec (version 6) and of the
  release pipeline (`docs/release.md`).
- The `theos_maps` skill in Jann's other Claude account: once P09 is merged, point it at the
  editor's map generator (`theos gen run` in a shell, or `gen_run` over MCP) or retire it; its
  recipes move into the content project in P23.
- CE-75: move the style bible (`style.yaml` v2 and its reference crops) from Drive into `art/`
  style records, after P20.
- Jira: slices S8 and S9 have no epics yet (create only on Jann's explicit yes).
