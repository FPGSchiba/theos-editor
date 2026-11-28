# UI reference

The screens of the Theos Content Studio concept, exported from the design canvas at
1440 × 900: screens 01–11 on 31 Oct 2026, screens 12–15 (atlas, quests, generating from lore) on
5 Nov 2026. On 27 Nov 2026 they moved to theme tokens and their previews were rendered again from
the HTML, in both themes.

- **`screens/NN-<name>.html` is the reference.** Each file is plain HTML that uses only
  `design/theos-ui/tokens.css`, `design/theos-ui/components/bundle.css` and a short block of page
  styles, plus `design/theos-ui/components/bundle.js` and a one-line script that switch the theme
  (see "Themes"). Open it in a browser from a clone, or read it: the class names (`tu-*`), the
  structure and the exact texts are what the editor should reproduce with Svelte components.
- **`screens/NN-<name>.png` and `screens/NN-<name>-light.png` are previews** rendered from that
  HTML at 1440 × 900, in Grid Noir and Grid Light, so GitHub and reviewers can see the screen
  without a browser. If the two ever disagree, the HTML wins; regenerate the PNGs.
- **`assets/`** holds the sample images the screens show.

## Themes

Every screen works in both Theos UI themes (`design/theos-ui/README.md`, "Themes"). A screen opens
in Grid Noir; add `?theme=grid-light` to its URL to see Grid Light. The switch is
`design/theos-ui/components/bundle.js` (`TheosUI.setTheme`), loaded only so a reviewer can flip
the theme. It is not the editor's theme setting: P11's spec leaves the switch to P14, and no AC
specifies it yet.

The page styles follow the rules of the design system:

- Colours come from tokens only, and tints are `color-mix()` on a token, so one attribute switches
  the whole screen.
- No `opacity` or `filter` dims text. Dimmed rows use `textDim`; opacity stays only on pictures
  (a killed concept version, a T3 reading not picked yet) and on a disabled control.
- A stage keeps Grid Noir in both themes, as the 3D viewport does: the map canvas on screen 12
  carries `data-theme="grid-noir"`, because its labels and routes sit on a rendered map. The
  legend in the window bar above it follows the theme.

Contrast was checked in both themes against the design system's rules (text 4.5:1; large text,
icons and control borders 3:1). The check covered every text and input value that is not on a
picture, the placeholders, the glyphs that `::before` and `::after` draw, and the borders of
check boxes, toggles, inputs and selects. What does not pass is redundant `textMute` text such
as indices, timestamps and tags (the design system's known flag), an inactive control (the
locked tier option, which WCAG 1.4.3 exempts), or a state of the design system's own components,
listed in `docs/status.md` under "Needs Jann".

Where the sample content disagrees with `docs/specs/data-model.md` (for example paths like
`floor.md`, the id `F04`, or the "Perspectives" and "Chronicles" folders), the data model wins.

| Screen | Name | Phase |
|---|---|---|
| `00-chrome` | Window chrome — title bar, menus, window controls, scrollbar | P51 |
| `01-main` | Codex — floor record | P11 |
| `02-lens` | Codex — npc and lens | P15 |
| `03-truth` | Codex — truth and reveal ledger | P15 |
| `04-script` | Scripts — beats | P15 |
| `05-graph` | Graph — links around a truth | P15 |
| `06-concepts` | Concepts — versions, pipeline, notes | P16 |
| `07-review` | Review — agent proposals | P13 |
| `08-git` | Changes — source control | P13 |
| `09-start` | Start — projects | P14 |
| `10-settings` | Settings — git page | P14 |
| `11-maps` | Maps — runs, seeds, pick | P16 |
| `12-atlas` | Atlas — floor map, layers, place panel | P25 |
| `13-atlas-tower` | Atlas — the Tower view | P25 |
| `14-quest` | Quests — flow editor | P24 |
| `15-generate` | Concepts — generate from lore | P27 |

Every screen shows the frameless title bar of Windows and Linux: the mark, the menus File, Edit, View,
Go and Help, the breadcrumb as the drag region, the validator pip and the window controls. `00-chrome`
draws the chrome itself: the menus open, the keyboard focus, the window controls restored and
maximized, macOS, and the themed scrollbar. It is one tall page, not a 1440 × 900 screen, so its
previews are full-page renders.

Screens 01–11 show the rail without Atlas; screens 12–15 show it after Codex. P25 adds the entry
to every screen. The camera thumbnails on screen 15 are clay renders of the floor-0 blockout (the
Tower, the Gate and the Hem); the map on screen 12 is the hearth-valley sample of screen 11.

The Concepts screen already shows a notes panel; the Maps screen gets the same panel in P16
(`docs/specs/data-model.md` §9.3).

The design system is in `design/theos-ui/` (tokens, component CSS, fonts under the SIL Open Font
License, component notes and previews).
