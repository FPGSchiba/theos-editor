# 04 · Readiness analysis

> **After review round 1:** where this page differs from `00-map-generation-research.md` and `09-proposed-phases.md`, those two are right. In particular: upper band 4 is not a labyrinth after P29 and P30; bands 5 and 8 are larger and unproven; whether every floor's gate up has a guardian arena, or only every tenth floor, is open as decision D10.

Stage 3 of the map generation research: the ten bands against what the generator can do
(`01-capability-baseline.md`), with Jann's decisions of 11 Nov 2026 (`decisions.md`).

Status: first version from the specs. The probes (`05-probes.md`) and the state-of-the-art reports
(`06*.md`) may move single cells; the synthesis carries the final version.

## Summary

- **Right after P08, one band can be designed as regions:** band 1, and the open land of bands 2
  and 3. Nothing else.
- **P29 and P30 add towns, single interiors and stacked indoor floors.** That completes band 1,
  most of band 2, the labyrinth half of band 4 and small floors of band 9.
- **Six terrain types have no phase:** cavern networks (4), needle peaks (5), giant trees (3),
  cities (6, 7), broken floating ground (8) and ring galleries (10).
- **Three gaps cut across every band:** the places every floor needs (Chimes, gates, guardian
  arena), blending between bands, and steering a map towards what the lore asks for.

## Codes

| Code | Meaning |
|---|---|
| R | Works with a recipe and stages that are planned |
| P | Needs a parameter or a small change to a planned stage |
| S | Needs a new stage on an existing map kind |
| K | Needs a new map kind |
| F | Needs an addition to the map package (see `07-package-headroom.md`) |
| A | Not generated: authored by hand in the editor |
| E | Belongs to the engine |

A phase in brackets says when an R becomes true.

## Matrix

| Band | Ground | Water | Plant cover | Settlements and structures | Routes | Interiors | Scale |
|---|---|---|---|---|---|---|---|
| 1 Hearthlands | R (P08) | R (P08) | R (P08) | R (P08 points, P29 detail) | R (P08) | R (P29) | R |
| 2 Rune Kingdoms | R (P08) | R; moats S | R | Walled towns R (P29); castles S; siege approach A | R | Castle crawl R (P29), several storeys R (P30) | R |
| 3 Wildwood | R (P08) | R; swamps P | Density only: P for forest classes; **giant trees K + F** | Camps and groves P (new site types) | R; routes up the trees K | R (P29) | R, large maps near the 2048² limit: P |
| 4 Underdeep | **Caverns K + F** | Underground rivers S | Light factor P | Delver camps P | Inside the cave kind K | Labyrinth R (P29, P30); cave-to-labyrinth blend S | Up to 5 km against a 2 km limit for levels: F |
| 5 Thousand Peaks | **Needle peaks S**, with overhangs F; cloud sea P | Falls into cloud P | R | Halls on summits P (site rule); stairs S | **Rope bridges and stairs over gaps S + F** | R (P29) | R |
| 6 Brass Cities | Under the city R | Canals S | — | **Tiered city K or S + F** (levels on a settlement); docks A | Lifts and tier links S + F | Foundries and halls R (P30) | 2–8 km at 1–2 m cells exceeds the grid limit: F or child maps |
| 7 Metropolis | Under the city R | Embanked river P | — | **Street-grid city S** (grid streets, parcels, massing); storeys R | Trams and elevated lines S + F | Block interiors R (P29) as child maps | As band 6 |
| 8 Shatter | **Broken floating ground K + F**; soft gravity E | No normal flow: hydrology off P | — | **Ruined city S** (a destruction stage on a city) | Crossings between pieces S + F | R (P29) | R |
| 9 Works | — | Coolant channels P | — | **Machine halls S** on indoor floors (P30) | Lifts, gantries R (P30 links) | The floor itself: R (P30) up to 2 km | 1–4 km: above 2 km F |
| 10 Instrument | — | — | — | **Ring galleries S** on indoor floors, with a hole and round outline F | Strings of light as links P + F | R (P30) | R |

## Gaps that cut across all bands

| Gap | Today | Code | Note |
|---|---|---|---|
| Places every floor needs | `extraction`, `portal-in`, `portal-out`, `ruin` | P + F | Chime, gate up with guardian arena, gate down, hub settlement. Small change to `pois`, new feature types |
| Guardian arena on every tenth floor | None | S | A place with a required size, flatness and distance from the entry; then an interior or terrain child map |
| Blending between bands | A recipe describes one terrain type | S, partly K | Two parts: shifting parameters step by step over ten floors (recipe tooling), and two terrain types on one map (for example cavern into labyrinth, forest into cave mouths) |
| Steering | Recipes take no inputs | P to S | Fixed places, masks or a sketch as inputs; see `06d-steering-and-blending.md` |
| Rim and Bore | Drained edge per side | P | A drain inside the map and a round outline for whole-floor maps |
| Pass rate of seeds | 2 of 8 for `hearth-valley` in 0.3.0 | P | CE-28.5 only asks P08 to be no worse; 100 floors need better |

## When each band becomes designable

| Step | Bands | What is still missing there |
|---|---|---|
| After P08 | 1 as regions; open land of 2 and 3 | Town and interior detail; the floor places (Chime, gates) |
| After P29 | 1 complete; 2 without castles | Castle stage; giant trees |
| After P30 | Upper floors of 4 (labyrinth); small floors of 9 | Machine-hall and ring-gallery stages; floors above 2 km |
| New work, natural | 3 complete, lower 4, 5 | Giant trees, caverns, needle peaks, bridges over gaps |
| New work, built | 6, 7, the Hem | City kind or stages, tier links, scale |
| New work, broken | 8, 10 | Floating ground, ring outline |

## Largest gaps, ranked

Ranked by bands blocked, then by how early the band comes in the climb.

| # | Gap | Bands blocked | Code |
|---|---|---|---|
| 1 | Floor places: Chime, gates, guardian arena | all | P + F + S |
| 2 | Blending between bands | all | S, K |
| 3 | Steering and seed pass rate | all | P, S |
| 4 | City generation at 2–10 km | 6, 7, 8, the Hem | K or S, F |
| 5 | Cavern networks | 4 | K, F |
| 6 | Stacked or floating ground outdoors | 5 (bridges), 8, rims of 6–7 | K, F |
| 7 | Needle peaks | 5 | S |
| 8 | Giant trees as terrain | 3 | K, F |
| 9 | Machine halls and ring galleries beyond P30's interiors | 9, 10 | S, F |
| 10 | Castles | 2 | S |
