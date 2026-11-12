# 03 · What a floor physically is

Stage 2 of the map generation research. The world-frame questions the generator depends on.

Status: answered by Jann on 11 Nov 2026, see `decisions.md`. In short: the floor's physics are
magic and are not taken strictly, so rows 1 to 4 of the second table are not blockers; the Tower
proposal's floor model is the working assumption. Sources: the Tower proposal ("what a floor is",
"moving between floors"), Explaining Lore (Confluence TOT 171016193), P08 phase spec,
`docs/map-package.md`.

None of the Tower proposal's eight decision boxes was ticked when this was written.

## Where proposal and generator already agree

| Topic | Tower proposal | Generator (P08) | Fit |
|---|---|---|---|
| A map is part of a floor | "A run uses one region of it" | One rectangular frame per map | Fits |
| The rim | Land ends at a rim over the Bore; rivers fall off it | An edge can be drained at base level 0; rivers leave the map there | Fits: the rim is a drained edge |
| The outer wall | The fold ends at the wall of the register | The other edges are closed | Fits |
| Size | Free per floor, from one choir-hall to tens of kilometres | Grid and size are free per recipe, up to 2048² cells | Fits up to about 100 km |
| Machine ruins | Choir-halls, tuning galleries, echo-vaults on every floor | `ruin` places with interior child maps (P29) | Fits |
| Detuning | The light flickers when the Tower drifts (ω) | `detuning` writes ω around ruins | Fits |

## Where they do not agree

| # | Topic | Tower proposal | Generator (P08) | Problem |
|---|---|---|---|---|
| 1 | Vault height | Hearth vault "about 1–2 km up" | `hearth-valley` asks for 800–2,200 m of relief | The mountains would reach or pass the vault |
| 2 | Rain | Falls from the vault | Wind pushes air over mountains (one wind direction) | Under a vault there may be no wind system |
| 3 | Temperature | Not stated | Falls with height (lapse rate), which gives snow on peaks | Needs a reason under a vault, or another rule |
| 4 | Light | A strip along the vault's crown; none in the Underdeep | No model of light; vegetation follows water only | Caves would grow forest |
| 5 | Whole floor | A ring of land around the Bore | A rectangle | A map of a whole small floor needs a hole and a round outline |
| 6 | Broken rims | Crucible: drifting islands, soft gravity | Heightfield, normal gravity for water and walking | Hydrology and walking times mean nothing there |
| 7 | Chimes | Several per floor, one in the main settlement; extraction at any Chime | `extraction` points, `portal-in`, `portal-out` | The place types do not match: no Chime, no way up, no guardian arena |
| 8 | Underdeep | Registers whose vault came down | Terrain needs a sky; interiors are rooms | Unclear whether a floor is one cave level or several |
| 9 | Floor 0, the Hem | A ring-town across the field's edge | Settlement kind (P29) | Outside the 100 floors, but it needs a map too |

## Questions asked at checkpoint 1, and their outcome

| # | Question | Recommendation then | Outcome |
|---|---|---|---|
| 1 | Is the Tower proposal's floor model the working assumption? | Yes | Working assumption (not answered) |
| 2 | Does a map show a region of a floor or the whole floor? | A region by default; whole floor for small floors | Working assumption; open again as D11 of the synthesis |
| 3 | How does weather work under a vault? | By act | Dropped: physics are not strict |
| 4 | Is the vault a hard limit on relief? | Yes for the Hearth | Dropped: no |
| 5 | Does light drive vegetation? | Yes, as a factor | Dropped as a blocker; a cheap option for the Underdeep |
| 6 | Which generated places does every floor have? | Chime, gate up with guardian arena, gate down, hub | Working assumption; arena open as D10 |
| 7 | Is the Hem in scope? | Yes, as a city test case | Working assumption |

## Not decided here

- Why the outside is dying, and whether the outside is playable (Tower proposal, open questions).
- How many Chimes a floor has.
