The glyph and quiet colour that identify a content record's kind; the glyph carries the identity, the colour is a second cue.

**Use** `<span class="tu-kind" data-kind="belief"></span>`. The glyph comes from CSS, so markup never hard-codes it. For a labelled kind use `<span class="tu-kindtag" data-kind="belief"><span class="tu-kind" data-kind="belief"></span>Belief</span>`. Any element with `data-kind` gets `--k` (kind colour) and `--g` (glyph), so link chips and popover rows pick them up too.

| Kind | Glyph | Token |
| --- | --- | --- |
| World entity (untyped) | ◇ | `kind-world` |
| Tower · Band · Floor · Location · People | ▲ ≣ ⊟ ⌂ ∴ | `kind-world` |
| Faction · NPC · Artifact · Document · Creature | ◧ ◉ ◆ ⌸ ✶ | `kind-world` |
| Enemy · Spawn | ⍟ ⊛ | `kind-world` |
| Truth | ■ | `kind-truth` |
| Belief | □ | `kind-belief` |
| Clue | ◔ | `kind-clue` |
| Perspective | ◎ | `kind-perspective` |
| Chronicle | § | `kind-chronicle` |
| Script / Beat | ¶ | `kind-beat` |
| Quest | ↝ | `kind-beat` |
| Concept / Version | ◫ | `kind-concept` |
| Map / Version | △ | `kind-concept` |
| Template | ⊡ | `kind-template` |
| Style | ◊ | `kind-template` |

**Why these glyphs**: Truth and Belief are a filled and a hollow square, the same fact solid and as held. A clue is a quarter-filled circle, partial knowledge. A perspective is a lens. Chronicle and Script use the two text marks (section and paragraph). A concept is a split square, the two-state pair. A quest is a winding arrow, a route through the world; an enemy a circled star; a spawn a circled asterisk, a place where enemies gather; a map the triangle of the Maps rail; a style a cut facet.

**Rules**
- Kind colours never fill a surface and never stand in for a status. A Belief that is `proposed` shows a Belief glyph *and* an amber `proposed` status.
- All world entities share `kind-world`; their glyph tells them apart. Never invent a tenth world colour.
- Kind colours are low-chroma on purpose (OKLCH C 0.065, L 0.80; about 10:1 on `panel`). The saturated status set stays at least ΔE 0.10 away.
- All glyphs are in JetBrains Mono (full build), so they render the same on every OS.
