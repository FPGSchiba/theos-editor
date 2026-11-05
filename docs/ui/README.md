# UI reference

The fifteen screens of the Theos Content Studio concept, exported from the design canvas at
1440 × 900: screens 01–11 on 31 Oct 2026, screens 12–15 (atlas, quests, generating from lore) on
5 Nov 2026.

- **`screens/NN-<name>.html` is the reference.** Each file is plain HTML that uses only
  `design/theos-ui/tokens.css`, `design/theos-ui/components/bundle.css` and a short block of page
  styles. Open it in a browser from a clone, or read it: the class names (`tu-*`), the structure
  and the exact texts are what the editor should reproduce with Svelte components.
- **`screens/NN-<name>.png` is a preview** rendered from that HTML, so GitHub and reviewers can see
  the screen without a browser. If the two ever disagree, the HTML wins; regenerate the PNG.
- **`assets/`** holds the sample images the screens show.

Where the sample content disagrees with `docs/specs/data-model.md` (for example paths like
`floor.md`, the id `F04`, or the "Perspectives" and "Chronicles" folders), the data model wins.

| Screen | Name | Phase |
|---|---|---|
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

Screens 01–11 show the rail without Atlas; screens 12–15 show it after Codex. P25 adds the entry
to every screen. The camera thumbnails on screen 15 are clay renders of the floor-0 blockout (the
Tower, the Gate and the Hem); the map on screen 12 is the hearth-valley sample of screen 11.

The Concepts screen already shows a notes panel; the Maps screen gets the same panel in P16
(`docs/specs/data-model.md` §9.3).

The design system is in `design/theos-ui/` (tokens, component CSS, fonts under the SIL Open Font
License, component notes and previews).
