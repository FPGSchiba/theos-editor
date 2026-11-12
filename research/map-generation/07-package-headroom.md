# 07 · Map package headroom

> **After review round 1:** this page calls levels on outdoor maps, the `present` and `void` masks and sloping level heights "minor" additions. They are not: a 1.0 reader that ignored them would build a wrong map. `09-proposed-phases.md` §B1 lists them as changes of meaning, to amend into 1.0 now or to carry as a 2.0 (decision D3). The feature and layer additions below remain optional.

Stage 6 of the map generation research: `docs/map-package.md` (format `theos-map` 1.0) tested
against every band.

Status: first version from the format document and the band sheet. The state-of-the-art reports
may add needs; the synthesis carries the final list.

## Summary

- **Format 1.0 holds up better than expected.** Its reading rule (unknown fields, layers, feature
  kinds and properties are ignored, §6) makes almost every need an additive minor version.
- **Two things in 1.0 already reach further than the text says:** stacked levels, and features
  with a third coordinate plus values per vertex. Together they can describe caves, tiers,
  floating pieces, bridges, giant trees and strings of light without storing volumes.
- **Only two needs would force a new major version:** true 3D volumes as layers, and maps larger
  than one grid can hold (tiles, or a grid per level). Both can be avoided.
- **Recommendation:** plan a format 1.1 with the additions below, and reach the hard bands with
  levels, a ceiling layer and 3D lines instead of volumes.

## Findings

"Minor" means an optional addition under §6. "Text" means only the document's wording changes.

| # | What cannot be stored today | Bands | Proposed answer | Change |
|---|---|---|---|---|
| 1 | Ground above ground: overhangs, cave roofs, floating pieces. Terrain has one `height` per cell | 4, 5, 8 | Allow `levels` on any map kind, not only "an indoor map". Each level then has its own `height` layer and a mask of where it exists | Text + minor (a `present` mask layer) |
| 2 | Headroom above a walking surface (cave roof, hall ceiling, underside of a floating block) | 4, 8, 9 | New standard layers `ceiling` (m) and, for floating pieces, `underside` (m) | Minor |
| 3 | Things that are lines in space: rope bridges, stairs on a cliff, tree trunks and branches, strings of light, elevated tram lines | 3, 5, 7, 10 | Already possible: LineStrings with a third coordinate and per-vertex arrays (as rivers have). Add feature kinds and a per-vertex `radius_m` or `width_m` | Minor (new kinds) |
| 4 | Bridges over a gap that is not water | 5, 8 | `site`/`bridge` gets `over`: `water` or `gap`; `deck_m` already exists | Minor |
| 5 | The places every floor needs | all | New `poi` types `chime`, `gate-up`, `gate-down`; a `guardian-arena` polygon; `hub: true` on the main settlement | Minor |
| 6 | Where the rim, the wall and the Bore are | all | `frame.edges` with one of `rim`, `wall`, `open` per side; a `void` mask layer for a hole inside the map and for round floors | Minor |
| 7 | Plant cover beyond one density | 3 | A class layer `biome` with a legend, beside `vegetation` | Minor |
| 8 | Cities: building height and tier, rails, lifts, docks | 6, 7 | `building` gets `height_m`, `base_m`, `from_level`, `to_level`; new `street` types (`avenue`, `alley`, `rail`, `tram`); `link` types already cover lifts | Minor |
| 9 | Castles and other compound structures | 2 | A `structure` kind (polygon) that groups buildings, walls and gates by id | Minor |
| 10 | Ruined state | 8, ruins everywhere | `state` on buildings and structures (`intact`, `damaged`, `collapsed`) | Minor |
| 11 | Caves as chambers and passages | 4 | Reuse `room` and `connection` with cave types, plus `ceiling`; passages as LineStrings with width | Minor |
| 12 | Indoor floors wider than about 2 km | 4, 6, 7, 9 | Keep the limit per map. A large floor is a coarse parent map with child maps (districts, cave regions), which 1.0 already has | None |
| 13 | Child maps cannot be rotated | 10 | Accept: ring galleries are one whole-floor map, not rotated children | None |
| 14 | True volumes (voxels, signed distance fields) | 4, 8 if findings 1–3 prove too weak | A separate optional `volumes` index, so the 2D layer index keeps its meaning. If volumes had to go into `layers`, `shape` would change meaning: a new major | Minor if separate, major otherwise |
| 15 | Blend information (which two recipes, which share) | all | Nothing: `recipe` and `summary` are free form | None |

## What this means for P08's build

P08 writes format 1.0 and needs none of this. Two points are worth settling before a second
generator writes packages:

1. The wording of §4.3 ("an indoor map with stacked levels") and of decision 0003 §7. If levels
   are meant for outdoor maps too, `theos-map-format`'s validation must not tie levels to the
   interior kind. CE-76.3 already tests levels on a synthetic package without naming a kind.
2. The feature types of `pois`. P08 converts `extraction`, `portal-in` and `portal-out` from
   0.3.0. Renaming them later changes feature ids that pins bind to (P22), so the new place types
   are cheapest to add before canon maps exist.
