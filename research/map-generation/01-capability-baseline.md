# 01 · Capability baseline

Stage 0 of the map generation research. What the generator can do, from the specs only.
Read 11 Nov 2026 at theos-editor `main` (0f50cf3): umbrella §3 (P08, P09, P22, P23, P29, P30),
P08 phase spec, decision 0003 §6–§8, `docs/map-package.md`. Nothing of this is built yet; P08's
spec is merged, its build has not started.

States: **P08**, **P29**, **P30** = planned in that phase · **later** = named in P08 §3 or §5.9 as
a later stage · **none** = in no spec.

## Map kinds

| Kind | What it makes | State |
|---|---|---|
| `terrain` | A region as a heightfield with water, climate, vegetation, places and roads | P08 |
| `settlement` | Buildings, streets, walls, gates, landmarks, fields; one level | P29 |
| `interior` | Rooms as polygons, doors, connections, a critical route, lock and key; one level | P29 |
| Child map | A settlement or interior made inside a parent map from one of its features, axis-aligned | P29 |
| Indoor floor | An interior up to about 2 km across with at least two stacked levels, linked by stairs, ladders, lifts and one-way drops | P30 |
| Blockout scene | A 3D scene from a settlement or interior package, for concept-art cameras | P23 (tower-of-theos) |

## Terrain stages (P08 §5.9)

| Capability | Stage | State |
|---|---|---|
| Uplift from parametric sources: orogen, fault, rift, dome, volcano, basin | `tectonics` | P08 |
| Rock provinces with erodibility, diffusivity and a critical slope per rock type | `geology` | P08 |
| Rain by linear orographic theory, temperature by lapse rate, evaporation, runoff | `climate_model` | P08 |
| River erosion with sediment deposition, flow to several neighbours | `fluvial`, `partition_flow` | P08 |
| Closed basins resolved; depressions fill with sediment | `resolve_sinks` | P08 |
| Hillslope diffusion and a slope limit per rock | `hillslope`, `slope_limit` | P08 |
| Lakes by water balance, discharge, river lines with width, depth and bed height | `hydrology` | P08 |
| Vegetation density from the water balance, snow, wetness | `ecology` | P08 |
| Ambient element mix; detuning ω around ruins | `elements`, `detuning` | P08 |
| Settlements by site quality, in classes city, town, village, hamlet | `settlements` | P08 |
| Ruins, extraction points, portals in and out | `pois` | P08 |
| Resource deposits | `resources` | P08 |
| Roads in four classes, costed by grade, with bridges over water and fords | `roads` | P08 |
| Promotion of settlements where roads meet; inns and a market | `route_potential` | P08 |
| Mills, mines, quarries, camps, shrines, watchtowers, caves (as points) | `sites` | P08 |
| Walking time from extraction points (Tobler) | `travel` | P08 |
| Glacial erosion, layered strata, flexure, water depths, several wind directions | — | later |
| Karst, wind, coast and lava landforms; biome or species classes | — | none |
| Learned methods (sketch to uplift, surrogate, amplification) | separate generators | later, outside `theos-maps` |
| Refinement to walking scale | the engine | later |

Every stage follows one contract (P08 §5.3): declared layers in and out, typed parameters, checks
and metrics. A new capability is one stage module plus one registry line.

## Limits that matter for the bands

| Limit | Source |
|---|---|
| Terrain is a heightfield: one height per cell. No overhangs, arches, caves or floating pieces | P08 §5.4, map-package §4.4 |
| The frame is an axis-aligned rectangle of square cells; child maps cannot be rotated | map-package §2 |
| Built-in recipes are 384² cells of 62.5 m (24 km). Grids are limited to 2048² | P08 §5.5, §7 |
| Edges are either drained at base level or closed, per side. No drain inside the map | P08 §5.9 `grid` |
| One wind direction; no seasons; no model of light | P08 §3, §5.9 |
| Vegetation is one density value per cell | P08 §5.9 `ecology` |
| Bridges cross water only, up to 400 m | P08 §5.9 roads |
| Terrain recipes take no inputs: no sketch, mask or fixed place. Steering is through source positions, parameters and constraints on summary numbers | P08 §5.5 |
| Stacked levels share one grid and are at most about 2 km across; larger needs a new major format version | decision 0003 §7, map-package §4.3 |
| Until P30 an interior has one level. The 0.3.0 interior recipe is 100 × 100 cells of 2 m (200 m) | decision 0003 §7, `choir-hall-ruin.yaml` |
| Layers are 2D rasters (f32, u8, u16). No volumes, no meshes | map-package §4 |
| Street types are `lane`, `street`, `main-street`; building is one type with `use` and `storeys` | map-package §5 |
| Pins, cameras, spawns and quest places are written by people, never generated | map-package §9 |

The limits of the old settlement and interior code (largest settlement, room counts) were measured
in stage 4; see `05-probes.md`, probes 4 and 6.
