# UI reference

The eleven screens of the Theos Content Studio concept, exported from the design canvas on
31 Oct 2026 at 1440 × 900.

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

The Concepts screen already shows a notes panel; the Maps screen gets the same panel in P16
(`docs/specs/data-model.md` §9.3).

The design system is in `design/theos-ui/` (tokens, component CSS, fonts under the SIL Open Font
License, component notes and previews).
