# 05 — Probes of `theos_maps` 0.3.0 against the ten bands

> **Caveat (applies to every result in this file):** theos_maps 0.3.0 has no sediment
> deposition and no multiple-flow routing (the planned Rust generator adds both). A probe shows
> what the architecture and recipe model can express, not what the new generator's maps will
> look like.

## Summary

- The toolset runs unchanged on Linux (Python 3.12, fastscapelib 0.3.0 wheel). `hearth-valley`
  seed 1: 22.8 s; `choir-hall-ruin` seed 1: 2.4 s. 17 recipes, 62 runs, no crash.
- Bands 1–3 are expressible as recipes. The model is not scale-free: at 4 km the default river
  thresholds, mills and the scale bar break. Seed variance is very large (relief 327–1059 m and
  lakes 3–32 % from one recipe).
- Lakes are the main defect on low-relief floors: 9–36 % of the map in most gentle recipes.
  This is the no-deposition caveat showing; do not read it as a finding about the new generator.
- Band 5: the model makes 5–7 km high fluted cones with slopes to 88°, not needle towers. A
  heightfield of stream-power erosion has no vertical walls, overhangs or pillars.
- Bands 4 and 9: interiors scale to 155 rooms on 1 km × 1 km (44 s) with no hard limit, but
  are one level, room-and-corridor only, and fill 10–18 % of the footprint. "Cavern" is
  ragged rectangles joined by straight corridors.
- Bands 6 and 7: a settlement can be pushed to 4 km and 4765 buildings (255 s), but streets
  are organic (grid index 0.06–0.09 of 1), storeys are capped at 3 by a formula, and the wall
  is a convex hull that crosses lakes.
- The Bore: not expressible in a recipe (`grid` takes whole edges only), but a 40-line
  extension stage does it; rivers then drain into the hole. Every later stage ignores the
  hole and puts extractions, portals, mines, roads and farmland into it.
- Blending: interpolating recipe numbers changes relief and vegetation gradually, but rivers,
  lakes and settlements jump at every step (river overlap 1–9 %, settlements move 4–9 km).
- Report, recipes, scripts: `research/map-generation/05-probes.md`, `probes/`. 15 contact
  sheets: `Drive `Design/_lab/2026-11-map-research/``.

## Method

- Toolset: unchanged copy of `Design/_system/tools/theos_maps` 0.3.0 in `probes/toolset/`.
  Environment and commands: `probes/README.md`.
- Driver `probes/scripts/probe.py` calls the toolset's own `run_one` and adds metrics the
  toolset does not report: slope percentiles in degrees, a tower count (local maxima that stand
  ≥ 100 m above the ground 150 m away in all eight directions), storey histogram, and a street
  grid index (mean resultant length of 4θ over 40 m street chords; 0 = organic, 1 = a grid).
- Seeds 1–4 unless stated. Grids were not lowered. Run times are wall time per seed on 2 cores.
- "Constraints" are the recipe's own `constraints` stage. I set the ranges to state the
  intent of the probe, so a failure means "the recipe did not produce the intent for this seed".
- Raw numbers: `probes/results/<recipe>_results.json`.

## Baseline

| Recipe | Seed | Run time | Result |
|---|---|---|---|
| `toolset/recipes/hearth-valley.yaml` (24 km, 384²) | 1 | 22.8 s (erosion 14.7 s) | relief 1234 m, lakes 3.0 %, veg 53.7 %, 14 settlements, all three constraints pass |
| `toolset/recipes/choir-hall-ruin.yaml` (100², 2 m) | 1 | 2.4 s | 14/14 rooms, solvable, critical path 246 m; fails its own sightline constraint (54 m, limit 45 m) |

`hearth-valley-omega.yaml` and `map-stages/omega_detuning.py` were staged but not run (not needed
for the probes).

## What a recipe can express (from the code)

| Kind | Recipe controls | Fixed in code |
|---|---|---|
| terrain | size, grid, drained edges; uplift sources (orogen, fault, rift, dome, volcano, basin); rock mix; climate numbers; erosion steps, `m`, `n`; river thresholds; counts and spacing of settlements, POIs, resources, sites; road costs | seven rocks with K calibrated for `n = 1` (`terrain.py:22`); single-flow routing (`terrain.py:211`, `:246`); 0.6 km feature margin (`features.py:43`); mill needs Q > 0.4 m³/s (`features.py:388`); vegetation is one density (`terrain.py:295`) |
| settlement | `window_m`, `cell_m` (`local_terrain`); `radius_m`, `lanes`, `plot_m`, `wall` (`settlement_plan`, filter at `settlement.py:176`); `parcel_m`, `reach_m` | class table (`settlement.py:28–33`); landmarks per class (`:34`); storeys formula (`:283`); building `size` is a tuple and cannot be overridden from YAML |
| interior | grid, cell size, envelope (circle, rect, ring), room list (type, role, count, size, shape), extra cycles, layout and corridor numbers, cover | one main cycle with one lock and one drop (`interior.py:74–97`); orthogonal corridors (`:269`); one level (`:29`, `export.py:152`) |

## Probe 1 — Band 1 at other scales

### 1a. Small pastoral valley, 4 km

- Recipe `probes/recipes/p1a-small-valley-4km.yaml` (256², 15.6 m cells). Seeds 1–4, 5.8–7.5 s.
- Sheet: `p1a-small-valley-4km_contact.png`.

| Seed | Relief m | Lakes % | Rivers % | Max Q m³/s | Veg % | Slope p90 ° | Extraction↔extraction min | Failed constraints |
|---|---|---|---|---|---|---|---|---|
| 1 | 274 | 1.2 | 0.45 | 0.1 | 85.9 | 20.3 | 26.0 | vegetation |
| 2 | 244 | 9.1 | 0.75 | 0.3 | 83.6 | 15.6 | 28.5 | lakes, vegetation |
| 3 | 504 | 0.9 | 0.17 | 0.0 | 86.4 | 17.0 | 23.1 | relief, vegetation |
| 4 | 208 | 3.1 | 0.12 | 0.0 | 92.7 | 11.1 | 24.5 | vegetation |

- **Worked:** one village, three hamlets, tracks, two extractions 23–29 walking minutes apart.
  That is the run scale `refs/recipes.md` ("Scale") asks for.
- **Seen:** a green bowl with a farmed patch and a pond. Streams are short, thin and broken.
  The two side ridges I asked for are not readable; the seed's noise decides the shape.
- **Broke:**
  - River thresholds are absolute discharges (`terrain.py:277`, default 0.15 m³/s). A 4 km
    catchment reaches 0.0–0.3 m³/s, so I had to lower `river_q` to 0.02, and two seeds still
    have almost no river.
  - No mill in any seed: `features.py:388` hard-codes Q > 0.4 m³/s.
  - Scale bar is fixed at 5 km (`render_terrain.py:55`) and runs off a 4 km map.
  - The feature margin of 0.6 km (`features.py:43`) removes 51 % of a 4 km map from placement.
  - Vegetation ignores that this is pasture; cover is 84–93 % because rain is the only control.

### 1b. Wide farmland floor, 16 km, three villages and a market town

- Recipe `probes/recipes/p1b-farmland-16km.yaml` (384², 41.7 m cells). Seeds 1–4, 16–18 s.
- Sheet: `p1b-farmland-16km_contact.png`.

| Seed | Relief m | Lakes % | Max Q m³/s | Veg % | Slope p90 ° | Roads km (trail/track/road) | Failed constraints |
|---|---|---|---|---|---|---|---|
| 1 | 340 | 3.4 | 0.4 | 45.8 | 7.7 | 0.7 / 38.1 / 10.2 | none |
| 2 | 1059 | 23.5 | 1.7 | 58.0 | 31.8 | 3.3 / 56.6 / 37.2 | relief, lakes |
| 3 | 327 | 32.0 | 5.5 | 51.5 | 7.3 | 34.8 / 15.3 / 5.7 | lakes |
| 4 | 627 | 10.4 | 3.8 | 85.1 | 13.9 | 5.9 / 25.5 / 2.9 | lakes, vegetation |

- **Worked:** town 1, villages 3, hamlets 6 in every seed; roads join them; farmland appears
  around settlements. Seed 1 is a believable open farmland floor.
- **Seen:** seed 1 is a dry, flat plain with one lake. Seeds 2–4 are dominated by one huge
  lake; seed 2 also grew a full mountain block in the north. Seed 3 shows dead-straight
  diagonal "rivers".
- **Broke:**
  - Seed variance is far larger than the recipe suggests. Each source's rate is jittered by
    15 % and each path point by `jitter` (`terrain.py:71`, `:74`), the uplift field is warped,
    and rock provinces are redrawn per seed (`terrain.py:121–126`). Hard rock under the orogen
    triples relief (seed 2 has 23.9 % granite, seed 1 has 0.7 %).
  - Lakes: `basin` sags are added after erosion (`terrain.py:230`), and flat lowlands have no
    sediment to fill closed depressions. This is the caveat, not a recipe error. The recipe
    note "raise `background_variation`" did not help at 0.8.
  - Straight rivers are single-flow routes across filled flats (`terrain.py:246`).
  - There is no "market town" type; `town` gets a market only in its detail map
    (`settlement.py:34`).

## Probe 2 — Band 2: walled town on a river, with a keep

### 2a. Region, 12 km

- Recipe `probes/recipes/p2-rune-region-12km.yaml` (384², 31.3 m cells). Seeds 1–4, 17–19 s,
  exported as engine packages. Sheet: `p2-rune-region-12km_contact.png`.

| Seed | Relief m | Lakes % | Max Q m³/s | Roads km (main/road/track) | Watchtowers (asked 4) | Failed constraints |
|---|---|---|---|---|---|---|
| 1 | 764 | 26.5 | 1.2 | 8.8 / 25.2 / 18.4 | 4 | lakes |
| 2 | 1859 | 15.0 | 2.8 | 0 / 14.9 / 22.0 | 2 | relief, lakes, watchtowers |
| 3 | 712 | 30.5 | 4.1 | 0 / 15.1 / 13.8 | 2 | lakes, watchtowers |
| 4 | 964 | 9.1 | 3.2 | 0 / 7.4 / 12.1 | 2 | lakes, watchtowers |

- **Worked:** one city, one town, four villages, five hamlets in every seed; the city sits on
  or near water in all four; mines, quarries and mills attach to deposits and rivers.
- **Seen:** readable medieval marches, but each has one or two very large lakes. In seed 2 the
  ascent portal lies in the middle of a lake.
- **Broke:**
  - "Castle" is not a regional feature. The only fortification types are `watchtower` (a
    point) and the keep inside a city detail map. No type for castle, keep, border fort.
  - Watchtower count is not honoured (2 of 4) because the pick needs a 5 km spacing near a
    `road`-class road (`features.py:408`); no warning except through a constraint.
  - Portal placement does not exclude lakes for `portal_out` (`features.py:214`).
  - Region population (city 9000 × jitter, `features.py:111`) disagrees with the detail map
    (about 3200); `details.py` overwrites the region value after the fact.

### 2b. Settlement detail of the walled seat

- Recipe `probes/recipes/p2-walled-town-detail.yaml` (class defaults: 2400 m window, 5 m
  cells, 480²). One detail per region seed, 8.8–11.1 s. Sheet: `p2-walled-town-detail_contact.png`.

| Seed | Buildings | Population | Streets km (main/street/lane) | Gates | Bridges | Storeys 1/2/3 | Grid index | Constraints |
|---|---|---|---|---|---|---|---|---|
| 1 | 648 | 3200 | 2.6 / 1.7 / 5.8 | 2 | 1 | 542 / 52 / 54 | 0.09 | pass |
| 2 | 638 | 3150 | 0 / 4.1 / 5.2 | 3 | 2 | 495 / 90 / 53 | 0.13 | pass |
| 3 | 681 | 3365 | 3.6 / 2.6 / 5.8 | 5 | 2 | 581 / 56 / 44 | 0.13 | pass |
| 4 | 697 | 3445 | 0 / 7.7 / 5.7 | 3 | 4 | 572 / 57 / 68 | 0.22 | pass |

- **Worked:** streets follow the regional roads to a market; lanes branch; houses line the
  streets; wall with gates where streets cross it; keep, temple, archive, granary, smithy,
  inn, mill, guardhouse; a harbour when a lake is near; bridges where streets cross the river.
- **Seen:** a branching, tree-like street pattern inside a polygonal wall, most of the walled
  area is green space. It reads as a large walled village, not a dense town.
- **Broke:**
  - Wall and keep exist only for class `city` (`settlement.py:32`, `:34`). A `town` can get a
    wall through `wall: true` but never a keep.
  - The keep is an ordinary house scaled by 1.6 (`settlement.py:316`), on the most
    prominent point. No bailey, no moat, no castle plan.
  - The wall is the convex hull of the buildings (`settlement.py:343`). It ignores terrain and
    water (see probe 6).
  - Building density is low: plots only line streets, there are no blocks or back plots.
- **Largest size the settlement code allows:** there is no check. `window_m` and `cell_m` are
  free; the window may even exceed the region because the crop uses `mode="nearest"`
  (`settlement.py:78–82`). The practical limit is time: every lane attempt runs a distance
  transform and a Dijkstra over the whole grid (`settlement.py:212–226`). See probe 6: 800² with
  400 lanes takes 215–255 s; cost grows with cells × lanes.

## Probe 3 — Band 3: dense forest, strong relief, swamp, few roads

- Recipe `probes/recipes/p3-wildwood-16km.yaml` (384²). Seeds 1–4, 16–18 s.
  Sheet: `p3-wildwood-16km_contact.png`.
- A first version with an orogen of 2.8e-3 m/yr gave 2934–4165 m relief and p90 slopes of
  59–68° (`results/p3-wildwood-16km_first-untuned_results.json`). The table is the tuned recipe.

| Seed | Relief m | Lakes % | Rivers % | Veg % | Slope p50 / p90 ° | Roads km total | Peat-reed deposits (asked 4) | Failed constraints |
|---|---|---|---|---|---|---|---|---|
| 1 | 1469 | 7.4 | 1.87 | 71.7 | 20.3 / 47.8 | 41.5 | 4 | none |
| 2 | 1324 | 18.4 | 1.75 | 86.4 | 6.9 / 39.2 | 40.2 | 4 | lakes |
| 3 | 2285 | 9.7 | 1.69 | 74.0 | 10.6 / 51.0 | 45.8 | 4 | none |
| 4 | 2104 | 11.8 | 1.81 | 84.5 | 10.4 / 43.3 | 53.3 | 4 | none |

- **Worked:** dark continuous forest, dissected mountain blocks, many rivers, small clearings
  with one village and three hamlets, trails and tracks only (one short road in seed 1), camps
  and caves away from roads. Of all probes this is the closest to its band.
- **Seen:** deep green everywhere below the bare grey ridges; valleys with lakes; farmland only
  as small pale patches around the hamlets.
- **Broke:**
  - No swamp. Wet flats exist only as the resource field `peat_reeds` (`features.py:147`,
    `:161`) and as open lakes. There is no wetland layer or land-cover class.
  - Forest thins on steep ground (`terrain.py:295`), so more relief means less forest: the
    untuned version dropped to 50–67 % cover.
  - "Few roads" is not a control. Road length follows from the settlement count; `forest_cost`
    and `loop_detour` only bend routes. 40–53 km of trail and track remain.
  - Giant trees (terrain type I) have no representation: vegetation is one density value per
    cell, no objects, no height, no canopy level.

## Probe 4 — Bands 4 and 9: interiors

### 4a. Largest built interior

- Recipes `probes/recipes/p4a-underdeep-max.yaml` (300², 2 m cells, 600 m square; seeds 1–4,
  7.7–9.3 s) and `p4c-underdeep-scale-500.yaml` (500², 1 km square; seed 1, 44.1 s).
  Sheets: `p4a-underdeep-max_contact.png`, `p4c-underdeep-scale-500_contact.png`.

| Recipe, seed | Rooms carved | Connections | Cycles | Locks | Critical path m | Longest sightline m | Footprint used % | Failed constraints |
|---|---|---|---|---|---|---|---|---|
| 4a, 1 | 62/62 | 68 | 7 | 4 | 854 | 78 | 10.6 | sightline, footprint |
| 4a, 2 | 65/65 | 71 | 7 | 4 | 2480 | 82 | 10.8 | sightline, footprint |
| 4a, 3 | 60/60 | 66 | 7 | 4 | 1578 | 68 | 10.2 | footprint |
| 4a, 4 | 63/63 | 69 | 7 | 4 | 1114 | 70 | 10.6 | footprint |
| 4c, 1 | 155/155 | 171 | 17 | 9 | 1150 | 96 | 9.8 | sightline, footprint |

- **Limits found:** none is enforced. Grid and room count are free (`interior.py:29`, `:55`).
  Cost: layout is a Python double loop over room pairs for 600 iterations (`interior.py:182`),
  corridors run one full-grid Dijkstra per connection (`interior.py:269–279`). 62 rooms: 8 s;
  155 rooms: 44 s. Cover stops at 120 items by default (`interior.py:357`), so sightlines stay
  over the limit in large maps.
- **Levels: one.** The world has a single `shape` and one `walk` mask; the export writes one
  `walkable` layer (`export.py:152`). The "one-way drop" is an edge attribute in the plane, not
  a height difference (`interior.py:97`).
- **Worked:** all rooms carved and connected, lock-and-key solvable in every run.
- **Seen:** small rooms on long thin corridors, with most of the square empty. It reads as a
  mine or bunker plan, not a labyrinth of halls. Rooms cluster in clumps; labels overlap.
- **Broke:**
  - Footprint use is 10–11 %. The layout spreads rooms by spring forces; nothing packs them.
  - Topology has exactly one main cycle with one lock and one drop regardless of size
    (`interior.py:74–97`); more rooms only lengthen the two arcs and add leaves. Dead ends stay
    near 50 %. The critical path varies 854–2480 m for the same recipe.
  - No halls on many levels, no stairs, shafts, bridges, galleries (band 9, type G).

### 4b. Cavern-like interior

- Recipe `probes/recipes/p4b-cavern.yaml` (160², ragged and round rooms, eroded circle).
  Seeds 1–4, 1.3–1.9 s. Sheet: `p4b-cavern_contact.png`. **One level.**

| Seed | Rooms | Cycles | Critical path m | Sightline m | Footprint used % | Failed constraints |
|---|---|---|---|---|---|---|
| 1 | 22/22 | 4 | 438 | 58 | 17.1 | footprint |
| 2 | 19/19 | 4 | 354 | 40 | 18.4 | footprint |
| 3 | 22/22 | 4 | 418 | 40 | 17.8 | footprint |
| 4 | 20/20 | 4 | 436 | 40 | 17.1 | footprint |

- **Worked:** solvable, connected, ragged outlines, round sumps.
- **Seen:** rectangles with nibbled edges joined by straight one- or two-cell corridors with
  right-angle bends, floating in an empty circle. It does not read as a cave.
- **Broke:**
  - `ragged` only erodes the edge of a rectangle (`interior.py:223–229`); `round` is an ellipse.
  - Corridors are orthogonal and of constant width (`interior.py:269`, `:290–292`): no
    winding passages, no squeezes, no widening.
  - No cave process (no noise field, no cellular or erosion model), no height, no water level,
    and no transition from natural cavern to built labyrinth (type C).

## Probe 5 — Band 5: needle peaks

- Recipe `probes/recipes/p5-needle-peaks-8km.yaml` (384², 20.8 m cells; granite and quartzite
  only; ten small domes at 9e-3 m/yr; all four edges drained). Seeds 1–4, 16–17 s.
  Sheet: `p5-needle-peaks-8km_contact.png`.
- Failed variant `p5x-needle-peaks-n2-8km.yaml` (`n = 2`, `m = 0.9`): relief 229–276 m. The
  rock table's K values are calibrated for `n = 1` (`terrain.py:22–30`); with another `n` the
  same numbers erode everything flat. Sheet: `p5x-needle-peaks-n2-8km_contact.png`.

| Seed | Relief m | Slope p50 ° | p90 ° | p99 ° | max ° | Cells > 45° % | Local peaks | Towers | Lakes % |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 4816 | 8.6 | 68.5 | 86.3 | 88.1 | 15.2 | 118 | 10 | 19.0 |
| 2 | 6860 | 10.1 | 78.7 | 87.2 | 88.2 | 21.7 | 92 | 13 | 1.2 |
| 3 | 5392 | 9.3 | 75.2 | 86.1 | 87.9 | 19.2 | 100 | 10 | 4.0 |
| 4 | 6531 | 10.0 | 75.6 | 86.6 | 88.1 | 20.1 | 84 | 11 | 12.8 |

- **Slope distribution:** bimodal. Half the map is under 10° (the lowland between the
  sources), 15–22 % is over 45°, and the steep tail reaches 88°.
- **Isolated towers:** yes by the metric, 10–13 per map, which is the number of dome sources.
  The untuned probe 3 produced 75–151 "towers" by the same metric, as sharp ridge points.
- **Seen:** ten separate mountains, each about 1 km wide and 5–7 km high, with radial gullies
  like a volcano or a fluted cone. They are steep cones with a point, not needles: no vertical
  faces, no flat tops, no stacks, no arches. The ground between is forest, farmland and lakes.
- **Why:** a tower appears only where the recipe places a point source of uplift; erosion then
  cuts radial gullies. The model has no joints, no cliff retreat, no rock strength threshold,
  and a heightfield stores one height per cell. Relief is set by uplift ÷ erodibility and comes
  out several times higher than wanted for the needed steepness.
- **Cloud sea:** not expressible. There is no layer or datum for it; lakes fill the lowland
  instead. Settlements are placed on the flat lowland, not on peaks (site quality prefers flat
  ground near water, `features.py:92–97`).

## Probe 6 — Bands 6 and 7: a settlement at city size

- Recipe `probes/recipes/p6-city-size-4km.yaml`: `city-1` of the probe 2 region with
  `window_m: 4000`, `cell_m: 5` (800²), `radius_m: 1500`, `lanes: 400`. Seeds 1 and 2 only
  (215–255 s each). Sheet: `p6-city-size-4km_contact.png`.

| Seed | Run time s | Buildings | Population | Streets km (main/street/lane) | Built area ha | Storeys 1/2/3 | Max storeys | Grid index | Failed constraints |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 255.4 | 4765 | 23 785 | 4.1 / 9.9 / 48.5 | 53.0 | 4002 / 481 / 282 | 3 | 0.064 | grid, storeys, population |
| 2 | 215.5 | 3792 | 18 920 | 7.1 / 3.2 / 38.2 | 42.3 | 3051 / 431 / 310 | 3 | 0.087 | grid, storeys, population |

- **Worked:** the same code scales to a 3 km wide settlement without error.
- **Seen:** the walled-village pattern, enlarged: a dendritic web of lanes with houses on both
  sides and green between them. No blocks, no districts, no squares apart from one market. In
  seed 2 the wall runs straight across a lake for about 2.5 km.
- **Can streets form a grid? No.** Streets are least-cost paths from random points to the
  nearest street over a noisy cost field (`settlement.py:140–148`, `:212–232`). Grid index
  0.06–0.09 (0.09–0.22 at default size). There is no block, parcel or grid generator and no
  recipe switch for one.
- **Can buildings carry many storeys? No.** `storeys = 1 + (near centre) + (city and very near
  centre)` (`settlement.py:283`): at most 3, not a recipe parameter. Footprints are 7–15 m
  rectangles; `size` is a tuple that YAML cannot override.
- **Other blocks:** population is houses × 5 (`settlement.py:397`); built area is 5–8 % of the
  walled area; the wall is a convex hull (`settlement.py:343`); no elevated or underground
  levels, no rail, no canals, no zoning. A vertical city (type E) is outside the model.

## Probe 7 — The rim and the Bore

### 7a. One drained edge (the rim)

- Recipe `probes/recipes/p7a-rim-east-12km.yaml` (`grid.edges: [east]`). Seeds 1–4, 18–20 s.
  Sheet: `p7a-rim-east-12km_contact.png`.

| Seed | Relief m | Lakes % | Max Q m³/s | Extraction↔extraction min | Failed constraints |
|---|---|---|---|---|---|
| 1 | 832 | 11.4 | 1.3 | 61.9 | lakes |
| 2 | 655 | 17.4 | 3.4 | 92.5 | lakes |
| 3 | 1459 | 19.1 | 2.9 | 514.9 | lakes |
| 4 | 965 | 36.1 | 2.9 | 123.4 | lakes |

- **Worked:** the drained edge is the base level; rivers leave over it; the other edges act as
  walls. This is what the shipped recipe already does with `south`.
- **Seen:** nothing marks the rim. The land simply ends at the picture's edge. Large lakes;
  dead-straight diagonal river lines across flats (seeds 3, 4); in seed 3 an extraction sits on
  a lake island (515 minutes away), in seed 4 a portal lies in the lake.
- **Broke:** the rim is always a straight map edge at height 0 (`terrain.py:44–47`). No rim
  shape, no rim height profile, no cliff, no waterfall feature, nothing in the export that says
  "this edge is the rim".

### 7b. A central hole (the Bore)

- **Not expressible in a recipe.** `grid` passes only four edge states to
  `fs.RasterGrid.from_length` (`terrain.py:42–47`); there is no parameter for interior cells.
- **The architecture allows it as an extension.** fastscapelib 0.3.0 accepts
  `nodes_status={(row, col): FIXED_VALUE}`. `probes/ext/bore_grid.py` adds a stage `grid_bore`
  that provides the same layers as `grid`; all later core stages run unchanged.
- Recipe `probes/recipes/p7b-bore-centre-12km.yaml` (no drained outer edge, ragged hole of
  radius 0.14). Seeds 1–4, 19–20 s. Sheet: `p7b-bore-centre-12km_contact.png`.

| Seed | Relief m | Lakes % | Streams reaching the rim | Rim height m (min / median / max) | Features placed inside the Bore | Road cells inside |
|---|---|---|---|---|---|---|
| 1 | 515 | 24.8 | 1 | −15 / 17 / 61 | extraction, arrival portal, iron ore, mine | 118 |
| 2 | 1543 | 12.1 | 1 | −31 / 72 / 263 | extraction, 3 stone deposits, 3 quarries | 174 |
| 3 | 665 | 1.0 | 2 | −30 / 0 / 70 | none | 0 |
| 4 | 914 | 12.3 | 4 | −13 / 8 / 78 | hamlet, extraction | 87 |

- **Worked:** the whole floor drains inward; main rivers run to the hole; the rim gets a ragged
  outline and a real height profile cut by erosion.
- **Seen:** a round pit with a scalloped edge in the middle of farmland. Towns sit beside it
  and rivers end at it. In seeds 1, 2 and 4 roads cross the pit and an extraction point sits in
  its middle; the pit floor is drawn as farmland or forest.
- **Broke:** every stage after `grid` treats fixed cells as ordinary flat, dry land, which is
  the best building ground by the site score (`features.py:92–97`, `:199–203`). Vegetation,
  farmland, roads and POIs go in. Lakes still fill closed basins elsewhere. A real Bore needs a
  "void" mask that every placement and render stage respects, and a rim feature in the export.
  The `bore_check` stage sinks the hole by 300 m only for the picture.

## Probe 8 — Blending between band 1 and band 3

- Script `probes/scripts/blend.py`; endpoints `p1b-farmland-16km.yaml` (t = 0) and
  `p3-wildwood-16km.yaml` (t = 1), which share stage list and source list. Every number is
  interpolated; counts are rounded. Generated recipes: `probes/recipes/p8-blend/`.
  Seeds 1 and 2, five steps, 18–21 s per run.
  Sheets: `p8-blend-band1-to-band3_s1_contact.png`, `…_s2_contact.png`.

| Seed | t | Relief m | Lakes % | Veg % | Slope p90 ° | Settlements | Height corr. to previous step | River overlap | Lake overlap | Settlement move km (median) |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 0.00 | 340 | 3.4 | 45.8 | 7.7 | 10 | – | – | – | – |
| 1 | 0.25 | 428 | 3.6 | 90.6 | 10.2 | 8 | 0.94 | 0.08 | 0.14 | 8.7 |
| 1 | 0.50 | 930 | 27.2 | 80.3 | 30.7 | 6 | 0.88 | 0.01 | 0.11 | 7.6 |
| 1 | 0.75 | 1264 | 4.8 | 81.2 | 41.4 | 6 | 0.79 | 0.05 | 0.17 | 7.8 |
| 1 | 1.00 | 1469 | 7.4 | 71.7 | 47.8 | 4 | 0.95 | 0.09 | 0.46 | 5.8 |
| 2 | 0.00 | 1059 | 23.5 | 58.0 | 31.8 | 10 | – | – | – | – |
| 2 | 0.25 | 1225 | 13.9 | 83.0 | 41.6 | 8 | 0.86 | 0.01 | 0.38 | 3.8 |
| 2 | 0.50 | 1439 | 15.7 | 87.0 | 37.2 | 6 | 0.71 | 0.05 | 0.40 | 4.3 |
| 2 | 0.75 | 1358 | 16.8 | 87.7 | 31.1 | 6 | 0.83 | 0.05 | 0.60 | 6.2 |
| 2 | 1.00 | 1324 | 18.4 | 86.4 | 39.2 | 4 | 0.94 | 0.08 | 0.80 | 3.7 |

- **Smooth:** the large shape of the land. Height correlation between neighbouring steps is
  0.71–0.95; relief rises almost monotonically in seed 1 (340 → 1469 m); the mountain belt
  stays in the north and grows.
- **Jumps:** everything discrete. Rivers share 1–9 % of their cells between neighbouring steps.
  Lakes jump (seed 1: 3.6 % → 27.2 % → 4.8 %). Settlements with the same id move 4–9 km per
  step (the map is 16 km), names and roads change with them. Vegetation jumps from 46 % to 91 %
  in the first step because base rain crosses the vegetation threshold.
- **Seen:** five different maps of similar character, not one map changing. Seed 1 at t = 0.5
  is half lake.
- **Why:** (1) river incision is a threshold process, so small changes in uplift re-route
  drainage; single-flow routing makes that worse (the caveat applies). (2) Geology picks rocks
  with `rng.choice` over the rock mix (`terrain.py:126`), so a changed weight redraws provinces.
  (3) Feature placement is greedy arg-max over a score (`features.py:51–66`); any change moves
  the first pick and all later ones. (4) Sub-streams are keyed by seed and stage name only
  (`core.py:76–79`), which keeps noise fields fixed (good) but does nothing for (1)–(3).
- **Consequence for "bands blend into each other":** parameter interpolation gives a gradual
  change of character across floors, which is enough if each floor is a separate map. It does
  not give spatial continuity. Blending inside one map needs per-cell parameter fields; 0.3.0
  has global scalars only, except for uplift and rock, which are already fields.

## Band → what 0.3.0's model can and cannot express

| Band | Can express (shown by a probe) | Cannot express, and what blocks it |
|---|---|---|
| 1 Hearthlands (A, D) | River landscape with farmland, hamlet-to-town hierarchy, roads, mills, detail plans (P1, P2). | Scale-free behaviour: discharge thresholds, mill rule, margin and scale bar are absolute (`terrain.py:277`, `features.py:388`, `:43`, `render_terrain.py:55`). Stable lakes on flat floors (no deposition). Pasture or field as land cover at region scale (farmland is drawn by the renderer only). |
| 2 Rune Kingdoms (A, D town and castle) | Walled seat with gates, keep, temple, market, bridges; watchtowers along roads (P2). | Castle as its own site or plan; keep is a scaled house (`settlement.py:316`). Wall follows a convex hull, not terrain (`:343`). Dense town fabric (no blocks). Wall and keep tied to class `city`. |
| 3 Wildwood (A, I) | Dense forest on strong relief, trails, camps, caves, wet-flat deposits (P3). | Swamp as a layer; forest density independent of slope; giant trees or any climbable object (vegetation is a density, `terrain.py:295`); a road budget. |
| 4 Underdeep (C, G) | One-level room-and-corridor complex up to 155 rooms, lock-and-key loop, extractions (P4a, P4c). | Cavern networks (ragged = eroded rectangle, `interior.py:223`); winding passages; cavern-to-labyrinth transition; levels (`interior.py:29`, `export.py:152`); dense packing (10–18 % footprint use). |
| 5 Thousand Peaks (B, D) | Very steep fluted cones where the recipe puts point uplift; slopes to 88° (P5). | Needle towers, cliffs, stacks, overhangs: heightfield plus stream-power erosion only, K table valid for `n = 1` only (`terrain.py:22`). Cloud sea datum. Settlements on peaks (site score wants flat ground near water, `features.py:92`). |
| 6 Brass Cities (E, G) | A 3 km organic walled settlement, 4765 buildings (P6). | Vertical city: one ground level, storeys ≤ 3 by formula (`settlement.py:283`). Industry, rail, canals, districts. Interiors linked to buildings. |
| 7 Metropolis (E) | Nothing beyond band 6's organic plan. | Street grid (grid index 0.06–0.09; streets are least-cost lanes, `settlement.py:212`). Blocks, parcels, high-rise, zoning, transit. Size limited by time: 255 s for 4 km at 5 m cells. |
| 8 Shatter (E, F) | The `ruin` skin of the interior renderer and an eroded envelope (baseline only; not probed further). | Ruined city (no city to ruin, no damage stage). Floating broken ground: the model is one continuous heightfield on one grid. |
| 9 Works (G) | Large rectangular halls and galleries on one level (P4a). | Many levels, shafts, bridges, machine layouts; topology is one loop plus leaves at any size (`interior.py:74`). |
| 10 Instrument (H) | A `ring` envelope for interiors exists (`interior.py:38`, not probed). With the probe extension, terrain drains into a central hole (P7b). | Ring galleries stacked around the Bore (no levels, no curved corridors). A Bore that features respect: fixed cells are treated as building land (`features.py:92`, `:199`). |
| The frame (rim, Bore, vault) | One or more straight drained edges as rim (P7a). | Rim shape or profile, a void mask, the vault. Needs a new grid stage (shown feasible in 40 lines) plus a mask all later stages honour. |
| Blending between bands | Gradual change of relief and cover between whole maps (P8). | Continuity of rivers, lakes and settlements across steps; spatial blending inside one map (global scalar parameters). |

## What this means for the Rust generator (architecture only)

1. The stage-and-layer model held up: a new grid stage with interior base-level cells slotted in
   without touching other stages. Keep that property (Stage contract, P08 §5.3).
2. Make every threshold relative to map scale or state it in the recipe. 0.3.0 mixes kilometres,
   cells and absolute discharges.
3. A void or "not land" mask must be a first-class layer that placement, routing and render
   stages declare as input. The Bore probe shows what happens without it.
4. Constraints reported, but nothing retried. Seed variance was large enough that one recipe
   gave relief from 327 to 1059 m. The generator needs either tighter coupling between recipe
   and outcome or a seed-search step with checks.
5. Count requests can fail silently (watchtowers 2 of 4). Checks should cover every requested
   count, as `pois` already does for ruins and extractions.
6. Settlement and interior generators need a different model for bands 4–10, not more
   parameters: grid streets, blocks, storeys as data, levels, and cave processes are all absent
   as concepts, so no recipe can reach them.

## Could not do

- Probe 6 ran on two seeds, not four (about 4 minutes per seed).
- Band 8 and band 10 were not probed with their own recipes; their rows rest on the code and on
  probes 4 and 7.
- `hearth-valley-omega.yaml` with the `omega_detuning` extension was not run.
- No pyproject file exists for the toolset; only `requirements.txt`.

## Sources

- Runs: `probes/results/*.json` (this session, 2026-11-11), contact sheets in
  `Drive `Design/_lab/2026-11-map-research/``.
- Code: `probes/toolset/theos_maps/{core,terrain,features,settlement,details,interior,export,render_terrain,run}.py`
  (theos_maps 0.3.0, staged from `Design/_system/tools/`), line numbers as cited.
- Toolset docs: `probes/toolset/README.md`, `CHANGELOG.md`, `refs/recipes.md`, `refs/extending.md`.
- fastscapelib 0.3.0 Python API (`RasterGrid.from_length` docstring, read in the venv) for
  `nodes_status`.
- Brief: `research/map-generation/BRIEF.md` (band table and terrain types).
