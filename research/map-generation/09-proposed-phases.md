# 09 · Proposed phases and document changes

Draft text for theos-editor. Nothing here is binding. Phase numbers, scope and criteria are
proposals for an umbrella version 4 and a decision document, which Jann decides on.

How to read the criteria:

- They are written in the umbrella's style but carry no CE numbers.
- Where a criterion says "a threshold the phase spec sets", the number comes from that phase's
  probe; this page does not invent it.
- **Every phase that changes a stage of P08, P29 or P30 carries this criterion without repeating
  it:**
  recipes that do not use the new capability keep their golden hashes, or the generator version
  is raised and new golden files are recorded (G20, CE-77.2).
- "Own design" marks a method with no paper behind it.

## A. Amendment to the P08 spec

### A1. Corrections to the text (no change of behaviour)

From the audit (`06a` §4):

1. §5.9 fluvial law: define the discharge term once, as the upstream sum of p̂ × area, and define
   Qs.
2. §5.9 hydrology: drop "keeps rivers narrow".
3. §5.9 Paper column: `roads` and `route_potential` are the spec's own designs "after" Galin 2011
   and Emilien 2012; correct the sources of `slope_limit`, `geology` and the `resolve_sinks` fill
   mode.
4. §5.9 ecology: name the model "Thornthwaite Memorial model (Lieth & Box 1972; Lieth 1975)"
   unless a source for "Montreal model" is found; say what Adams et al. 2004 is cited for.
5. §4.2: replace "no neural inference is bit-identical across platforms" with "stock frameworks
   are not; reproducible inference exists but adds a dependency".
6. §4.2: lower the expectation that deposition alone gives alluvial plains (McDonald & Cordonnier
   2026), and name Tzathas et al. 2024, the 2025 adjoint model and the Eurographics 2025
   terrain-descriptor survey as later options. Note that Schott et al. 2024's code needs GPU
   compute shaders and cannot be reused under G20.
7. Decision 0003, "Research sources": split the Horizon Zero Dawn quotation (`06a` §1.4).

### A2. Changes of behaviour (each is Jann's decision, D2)

1. **Final place type names.** `pois` writes `chime`, `gate-up` and `gate-down` where 0.3.0 had
   `extraction`, `portal-out` and `portal-in`. This removes types the format lists, so it is an
   amendment of format 1.0, valid only while nothing reads it. Texts that follow: map-package §5
   (`poi` row), `docs/generators.md` §3 (type list), P08 §5.5 (recipe constraints), §5.9 (`pois`,
   `travel`), §5.11 (`truth` render), the travel metric's name and the parity fixture. Placement
   rules stay 0.3.0's; only the names change.
2. **Thresholds.** `river_q`, `big_river_q` and placement margins are either relative to the
   map (a share of the largest discharge, a share of the frame) or required in every recipe
   without a default.
3. **A river-spread check** in `hydrology`: a finding when the `river` raster is wider than the
   river line's `width_m` allows on low gradients.

### A3. What P08 builds if the format is amended now (D3)

If Jann amends 1.0 before P08's build, `theos-map-format` implements section B1, because CE-76.1
and CE-76.3 bind it to the format text. That widens P08. Draft criteria to add to CE-76:

- A terrain package with two levels, one of them partial, round-trips with each level's `height`,
  `present` and `underside`.
- A package with levels whose level lacks `present` is rejected; so is a `void` or `present`
  layer that is not a mask.
- `frame.edges` and the Bore fields round-trip.

Documents that word levels as indoor and change with B1: `docs/map-package.md` §4.3,
decision 0003 §7, `docs/generators.md` §3, and the data model's `frame.levels`.

If Jann picks "2.0 later" instead, a format phase with these criteria comes before P39 and P40,
and the blockout tool and the engine reader move to 2.0 with it.

## B. Map package (`docs/map-package.md`)

### B1. Changes of meaning: amend 1.0 before P08's build, or a 2.0 later (D3)

A 1.0 reader that ignored these would build a wrong map, so they are not a minor version.

| Change | Why it is not optional |
|---|---|
| `levels` on every map kind, not only indoor maps | Switches on `level` for every layer, feature and pin, and `base-<level id>` renders |
| A level may have its own `height` layer; `elevation_m` becomes the level's nominal height | A reader that uses `elevation_m` alone builds flat floors |
| A `present` mask on every level of a map that has levels | Layers may not hold NaN, so cells without ground still carry a number |
| `poi` types `chime`, `gate-up`, `gate-down` replace `extraction`, `portal-out`, `portal-in` (A2.1) | Removes listed types |
| A `void` mask for the frame: cells that are not part of the floor (the Bore, outside a round rim) | The same reason |
| `underside` (m): the bottom of a floating slab. `ceiling` (m): headroom above a walking surface | Needed to read a slab as a solid |
| `frame.edges` (`rim`, `wall`, `open` per side) and an optional Bore centre and radius | Tells a reader where the land ends |

Still open inside B1 (decision D8): the note that levels share one grid and stay under about
2 km.

### B2. Optional additions (a minor version whenever a phase needs them)

| Addition | Phase |
|---|---|
| Polygon `guardian-arena`; `hub` on a settlement; `platform`; `requirement` on a placed feature | P31, P32 |
| Street node ids with `from` and `to`; kinds `block`, `parcel`, `district`; `building.height_m`, `base_m`, `roof`; `structure` (groups buildings, walls, gates), types `tower`, `ward`; `wall.height_m`; `gate.street` | P35 |
| Layers `biome`, `exposure`, `cover_dist`; kind `chokepoint` | P33, P38 |
| `state`, `was_storeys`, `was_use` on buildings; layers `damage`, `rubble`. Existing fields carry the ruined result (`storeys` is what is left; a street that is gone is absent), so a reader that ignores the new fields still builds the ruin | P37 |
| `bridge.over` (`water` or `gap`); kind `landform` (`pillar`, `arch`) | P38 |
| Link types `ramp`, `bridge`; an optional `fragment` outline per plate. The level layers are the truth | P39 |
| Map kind `cavern`; kinds `chamber`, `passage`; link types `shaft`, `ramp`; `worked` | P40 |
| `room.level_top`, `room.height_m`; `link.rooms`; `connection.via`; link type `gantry` | P41 |
| A `deck` mask and `thickness_m` per level; kind `rail` (`tram`, `elevated`); sites `station`, `dock` | P42 |
| Map kind `tree`; feature kind `tree` with types `trunk`, `limb` as 3D lines with `radius_m`; `landform` type `tree-site`; link type `climb` | P43 |

Volumes (voxels, distance fields) and meshes: not proposed. Decision D4 keeps them open until the
cavern and plate probes.

## C. Generator protocol (`docs/generators.md` §3)

Additions to protocol 1. Each needs the built-in adapter and `theos gen run` to pass inputs (P09)
and the Maps screen to draw a field for them (P16).

| Input type | Owner | Note |
|---|---|---|
| `number`, `enum` | P32 | `describe` reports type and range |
| `anchors` (fixed places with a type and a position) | P32 | The Maps screen needs a map to set them on |
| `layers` (control layers by hash) | P36 | The layer file lives in the project's binaries folder beside the packages |

Draft criterion for each owner: an input of the new type set in the Maps screen, in
`theos gen run` and through the adapter reaches the generator unchanged and is recorded in the
manifest's `inputs`. Both owners therefore need P09 and P16.

## D. Phases

| Phase | Name | Repo | Needs |
|---|---|---|---|
| P31 | Floor frame and places | theos-editor | P08, B1 |
| P32 | Recipe families and requirements | theos-editor | P09, P16, P31 |
| P33 | Map metrics and batch tool | theos-editor | P32 |
| P35 | Street core, fortification, districts | theos-editor | P29, P31, P32 |
| P36 | Steering by layers, field mixing | theos-editor | P32, P16 |
| P37 | Ruin pass | theos-editor | P35; P41 for interiors; P32 |
| P38 | Needle peaks and forest classes | theos-editor | P36 |
| P39 | Floating ground | theos-editor | P30a, P32 |
| P40 | Cavern kind | theos-editor | P30a, P41 |
| P41 | Interior structure | theos-editor | P30a, P31, P32 |
| P42 | City tiers, transit, city scale | theos-editor | P30a, P35 |
| P43 | Giant trees | theos-editor | P30a, P29, P38 |
| P44 | Band recipe families, one delivery per band | tower-of-theos | the band's phases, P23 |

All would form a new slice after S10. "P30a" is the generator half of P30 (levels in the world, the runner and the interior
generator), if Jann splits it (D6). There is no P34: the format changes are sections A3 and B.

### P31 · Floor frame and places

Scope: a `void` mask and rim edges written by `grid`; a drain inside the map; every placement,
routing and render stage of the terrain kind reads `void`; the floor's places on terrain. A map
may be a region of a floor: the rim or the Bore then crosses it as `void` cells.

- A recipe can declare a round outline and a hole. In the package, no feature, road or site lies
  in a `void` cell, and rivers that reach the hole or a rim edge end there (fixture: a synthetic
  recipe with a central hole, seeds 1–8 of the Rust generator).
- `frame.edges` names each side's kind, and `void` equals the cells outside the outline.
- A terrain recipe asks for counts of `chime`, `gate-up` and `gate-down`; a missing place fails
  the constraint `features.<type>`. (Settlement, interior and cavern kinds get their places in
  P35, P41 and P40.)
- Exactly one settlement is marked `hub` when the recipe asks for one, and one Chime lies within
  a recipe-given distance of it. (The rule comes from the Tower proposal: "one in its main
  settlement".)

Evidence: `03`, `05` probe 7. Probe first: none.

### P32 · Recipe families and requirements

Scope: recipe inputs, curves, inheritance, weights; per-stage seed overrides; an early-stop
option for batches; requirements, including the guardian arena.

- A recipe with `extends` and a `floor` input expands to a concrete recipe by a pure function. The
  package's `recipe` is the expanded one, and the same floor gives the same bytes.
- A numeric parameter may be a curve over an input, with linear, smoothstep and step easing.
- A stage declares which summary paths are final after it. With the run option `stop_early`, a
  run ends after the first stage that finalises a failing constraint and still returns results
  with every evaluated constraint and `pass: false`. Without the option a run behaves as in P08.
- A stage's seed can be overridden; only that stage's and later stages' layer hashes change.
- A `[[require]]` entry places a feature by rules on rasters and on walking time (for example
  three Chimes 40–70 walking minutes apart). The requirement stage runs after `roads`, and
  `travel` runs after it. An unmet requirement is a constraint result with its name.
- A `guardian-arena` requirement finds a disc of a given radius whose slope stays under a given
  limit, at a given walking time from `gate-down`, and writes the polygon next to `gate-up`.

Evidence: `06d` B1, C1, C3, C4 (placement and repair: own design); `08`. Probe first: none.

### P33 · Map metrics and batch tool

Scope: metric stages `route_stats`, `chokepoints`, `visibility`, `cover`; a batch command over
floors and seeds with ranking and sheets.

- The stages report, under `summary.stages.<id>`: walking time from entry to each Chime and their
  spread; the number of edge-disjoint routes and the detour of the second; the narrowest walkable
  width on each main route; the visible area and the longest sightline from each place; the
  distance to cover along routes; the largest flat disc. Each can be named in `[constraints]`.
- Line of sight uses integer stepping and exact comparisons; golden hashes cover the metric
  layers.
- The batch command writes, for a band: a contact sheet per floor, a band sheet, and a table of
  every metric by floor. Its ranking is reproducible.
- The table flags every pair of neighbouring floors whose metrics differ by more than a bound the
  family sets. This is the test of "no hard cuts".
- For P08's built-in recipes over seeds 1–16, the batch reports the share of seeds that pass all
  constraints, as a recorded baseline. (No target: P08's pass rate is not known yet.)

Evidence: `06d` M1, P1 (the metric set is a proposal). Probe first: the cost of visibility at
384².

### P35 · Street core, fortification, districts

Scope: tensor-field streets, planar blocks, parcels, massing attributes, integer geometry; walls
as least-cost loops; castle and moat; a `districts` stage for style blends. It replaces the
settlement method of P29 (D5).

- One stage makes an organic town, a grid district and a ring road from parameters. The phase
  spec sets a grid-index range for each fixture (the probe measured 0.09–0.22 for 0.3.0's
  organic town at default size and 0.06–0.09 for its 3 km city).
- Every parcel touches a street; every building lies in one parcel.
- A wall is a closed loop around the built area; it crosses a river only at a water gate or a
  bridge feature and never crosses a lake.
- A castle recipe writes a `structure` with a keep, a wall and a gate on a site whose score the
  package reports; a moat is a `water` feature joined to the river.
- With a blend input, the share of districts in the second style grows monotonically with the
  input (fixtures at five values).
- A Hem fixture: a ring town on an annulus with a `void` centre.
- A settlement recipe can ask for a `chime` and gates; each lies on a street.
- Golden hashes cover a town, a grid city, a castle and the Hem on four platforms.

Evidence: `06c` §1, §3, §6, §7 (Chen 2008 and Vanegas 2012 not opened; districts: own design);
`05` probes 2 and 6. Probes first: `i_overlay` on the pinned toolchain; a streamline tracer for
bit-identity and look; reading both papers.

### P36 · Steering by layers, field mixing

Scope: control layers as inputs; point and curve constraints; `site_prep`; `mix_fields`.
`uplift_from_target` only if its probe supports it.

- A run with a control layer records the layer's hash and file name in the manifest. A missing or
  changed layer fails the run and names the layer.
- An anchor that asks for a platform yields flat ground of the asked size within a tolerance the
  phase spec sets, and every cell still drains.
- A recipe that mixes two parameter sets by a mask yields one drainage network with no pits along
  the seam.

Evidence: `06d` A1–A4, B2 (the inversion loop is the agent's reading of a project page). Probe
first: read Schott et al. 2023 in full; a mixing probe on the Rust generator.

### P37 · Ruin pass

Scope: `damage_field`, `ruin_buildings`, `break_streets`, for settlements and interiors. Own
design.

- Damage thresholds are drawn per building from its footprint's integer centroid, so the result
  does not depend on feature ids.
- A recipe with the damage stages at amount 0 gives the same layer hashes as the same recipe
  without them; its features differ only by `state: intact`.
- At any amount, every feature a requirement placed stays reachable from the entry.
- Buildings carry `state`; a broken street carries a gap feature.

Probe first: a damage pass on a P35 fixture. Not covered: a ruined city on floating plates (cuts
at plate borders).

### P38 · Needle peaks and forest classes

Scope: a `joints` stage, strata in `geology`, a `wall_retreat` process; a summit site rule and a
cloud-sea base level; bridges and stairs over gaps as `site` features; a `biome` stage with
forest classes and swamp. Band 5 stays a heightfield. Wall retreat is an own design.

- The phase spec defines a pillar by footprint width, wall height and top flatness. A needle-peak
  recipe yields at least the asked number of pillars; the probe's fluted cones (`05` probe 5) are
  a negative fixture and yield none.
- Volume balance holds for wall retreat.
- A settlement class can be sited on summits; a route requirement between two summits is met by
  `bridge` features with `over: gap` whose spans stay under a limit.
- `biome` writes a class per cell; swamp appears where wetness and flatness pass thresholds.

Evidence: `06b` §2; `05` probes 3 and 5. Needs strata, which P08 defers. Probes first: a
wall-retreat probe with a contact sheet; run time at 10 m cells, where an 8 km map is 800² but a
24 km map would pass the 2048² limit.

### P39 · Floating ground

Scope: `fracture`, `displace_plates`, `undersides`, and links between plates. Own design.

- Plates do not overlap in space; each has an `underside` and a `present` mask; the plates'
  footprints before displacement cover the source terrain.
- Every plate that carries a required place is reachable by links.

Evidence: `06b` §3. Probe first: a plate probe with a contact sheet.

### P40 · Cavern kind

Scope: map kind `cavern`: a chamber and passage graph by geology-cost paths, tiers as slab
levels, underground water, floor places, and a `worked` value from natural cave to built
labyrinth. Drawing the graph into tiers and the `worked` blend are own designs.

- Every chamber is reachable from the entry; passages keep a minimum width and headroom.
- A cavern recipe can ask for `chime`, `gate-up` and `gate-down`; each lies in a chamber.
- The phase spec defines a wall-straightness measure. `worked` = 0 stays under its lower bound;
  `worked` = 1 hands the graph to the interior stages and yields rooms and doors; at 0.5 the
  share of worked chambers lies between a third and two thirds.
- Golden hashes cover a cavern at `worked` 0, 0.5 and 1.

Evidence: `06b` §1; `05` probe 4. Needs the shared room graph (D7). Probes first: a
cave-skeleton probe; KarstNSim's licence.

### P41 · Interior structure

Scope: cyclic mission graphs by rewriting, partition with multi-level halls, key analysis over
levels, floor places in interiors, polar layout for rings (own design). It replaces the interior
method of P29 and P30 (D5).

- A hall recipe yields at least the asked number of halls that span two or more levels, and
  walkable floor covers a share of the footprint above a threshold the phase spec sets (0.3.0:
  10–18 %, `05` probe 4).
- An interior recipe can ask for `chime`, `gate-up` and `gate-down`; each lies in a room.
- Lock and key is solvable on seeds 1–8 across at least three levels.
- In a ring recipe, rooms and corridors on both sides of the seam angle connect, and no wall lies
  on it.

Evidence: `06c` §5, §6 (Dormans not opened); `05` probe 4. Probes first: a ring-layout probe;
reading the mission-graph sources.

### P42 · City tiers, transit, city scale

Scope: settlement levels with decks and links; `rail` over main streets; canals; a city as a
coarse parent map with district child maps. Tiers are an own design and the least proven one.

- Every tier is reachable from every other by links; every deck cell lies over a support feature
  or within a span limit of one.
- A tram line follows main streets within a curvature limit and has stations at a spacing.
- A city parent of 8 km yields district child maps whose streets meet at the district borders.

Evidence: `06c` §2, §3; `04` scale column. Probes first: a tier probe; a parent-and-districts
probe for run time and seams.

### P43 · Giant trees

Scope: map kind `tree`: a space-colonization skeleton under walkability rules, limbs as 3D lines,
tiers as slabs, `climb` links, platforms at anchors; a terrain stage `tree_sites` that places
`landform` features of type `tree-site` on the region. Walkability rules are an own design. A
tree is a child map of a tree site and does not change the region's height. CE-78.2's list of
parent features (a settlement or a ruin) is widened by `tree-site`.

- Every tier is reachable from the ground; limb slope on walk routes stays under a limit; limbs
  on walk routes keep a minimum width.
- Tiers do not overlap in plan within one level.
- The child's trunk base lies on its tree site within one cell of the region map.
- Golden hashes cover one tree.

Evidence: `06b` §4 (Runions 2007 not opened). Probe first: a skeleton probe with a contact sheet.

### Band recipe families (tower-of-theos)

Each band's phases end with that band's recipe family in the content project's `tools/maps/`
folder (P23 creates it). Band 1's family is delivered with P32: a base recipe, curves over the floor number, and the requirements of its
floors.

### Not planned here

- Borders where the kind of map changes (3 to 4, 4 to 5, 5 to 6): no method researched.
- Calibration of recipe drivers, the diversity archive across floors and shared seed groups
  (`06d` C2, C5, §4.3). Seed groups are the tool to test "the same land carries on" (D9 c).
- A ruined city on floating plates.
- Caverns and machine halls wider than about 2 km (bands 4 and 9); only cities get a
  parent-and-children answer, in P42.
- Glacial, wind, lava and shoreline stages; a learned sketch-to-uplift tool outside the
  generator.
