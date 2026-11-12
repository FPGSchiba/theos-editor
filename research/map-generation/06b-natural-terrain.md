# 06b · Natural terrain beyond the heightfield (stage 5b)

Research of 11 Nov 2026 for the `theos-maps` generator (P08 spec, format `theos-map` 1.0).
Question: how to make the natural terrain a 2D stream-power heightfield model cannot make.

## Summary

1. **One storage idea covers caves, floating ground, overhangs and tree tiers: slabs.** A slab is
   a level with two rasters, a top surface and an underside (for a cave: floor and ceiling).
   Format 1.0 already has stacked levels; it lacks a per-cell second surface and the rule that
   levels may have their own sloping `height`. That is a minor-version addition, not a new major.
   Voxels and stored SDFs are not needed and do not fit the format.
2. **Caves (band 4):** build a 3D graph of chambers and passages with cost-driven shortest paths
   (Paris et al. 2021, MIT code), keep the graph as features, rasterise it into slab levels. The
   cavern-to-labyrinth blend is a `worked` value 0–1 on the same graph. New map kind `cavern`.
3. **Needle peaks (band 5):** a heightfield can hold pillars; stream power alone cannot make
   them. Real pillars need jointed, flat-bedded hard rock, incision along joints, then wall
   retreat (May et al. 2018). Add a `joints` stage, strata K(x, y, z) and a `wall_retreat`
   stage, on a grid of 10 m or finer. No new map kind.
4. **Floating and broken ground (band 8):** constructive, not simulated: cut plates from a
   normal terrain, give each an underside, store as slabs. Arches and small overhangs go into
   the package as parametric features (Paris et al. 2019), realised by the engine.
5. **Giant trees (band 3):** space colonization (Runions et al. 2007) under walkability rules;
   the skeleton graph with a radius per vertex is the canon; walkable tiers are slab levels. I
   found no paper on trees as playable levels. New map kind `tree`.
6. **Other landforms:** each is one heightfield stage: glacial by Hergarten 2021 (Cordonnier et
   al. 2023 uses a neural network for ice flow, so G20 rules it out); dunes by Desertscapes;
   lava by stochastic lobes; shores by a small wave-exposure stage.
7. **Neural 3D terrain** (EarthCrafter, Dream Cubed, InfiniteDiffusion) is real in 2025–2026 but
   cannot be inside the generator (G20, decision 0003).
8. **Not done:** the project docs (`01`–`03`, `decisions.md`) could not be read, because this
   session has no Projects tool. Floor sizes and Jann's answers are therefore assumed from the
   repo only (§0).

## 0. What I read, and what I assumed

Read in full: `BRIEF.md`; P08 spec §1–§5.11 (Stage contract §5.3, determinism §5.7, stages
§5.9); `docs/map-package.md`; decision 0003. From the umbrella only the P29 and P30 entries'
headings and ACs' first lines.

**Not read:** the four project docs named in the brief. No `project_read` tool exists in this
session (a tool search found none). Consequences:

- I assume a terrain floor like `hearth-valley`: 24 km across at 62.5 m cells (P08 §5.5).
- I assume indoor maps of at most about 2 km at 1–2 m cells (decision 0003 §7).
- If `03-floor-frame.md` or `decisions.md` fix other sizes, or say what a rim or the Bore is in
  metres, the size notes below must be rechecked. Nothing here depends on lore.

How sources are marked in this report:

- **[opened]** the page, abstract or PDF was opened in this run.
- **[title]** a search confirmed the paper exists with these authors and title; content is from
  my knowledge of the paper.
- **[memory]** cited from memory only; check before relying on a detail.

What a 2D stream-power heightfield cannot do, in one table:

| Limit | Why | Topics |
|---|---|---|
| One height per cell | No ground above ground: no caves, overhangs, arches, floating land, branches | 1, 3, 4 |
| Erosion follows surface water | No dissolution underground, no ice, wind, waves or lava | 1, 5 |
| Uniform rock strength with depth, no fractures | Valleys are V-shaped and dendritic; no cliffs, mesas or pillars | 2 |
| Cell size 62.5 m | Pillars, passages and limbs are 2–100 m wide | 1, 2, 4 |

## 1. Caves and cavern networks (band 4, type C)

### 1.1 The strongest methods

| Method | Source | What it does | Result data |
|---|---|---|---|
| **Geology-driven skeleton + implicit geometry** | Paris, Guérin, Peytavie, Collon, Galin 2021, *Synthesizing Geologically Coherent Cave Networks*, CGF 40(7) [opened: project page] | Inlets (sinkholes) and outlets (springs) are joined by shortest paths in 3D. The cost comes from geology: inception horizons, faults and fractures, permeability, the water table. The path search is anisotropic and gridless. Conduits are then built as a construction tree of signed-distance primitives with blending and warping | A **graph** (nodes with position, edges), then an SDF tree, then a mesh |
| **Karst network simulators from hydrogeology** | Borghi, Renard, Jenni 2012, *J. Hydrology* 414–415 (SKS) [title]; Fandel et al. 2022, *Hydrogeology J.* 30 [title]; pyKasso [opened: PyPI page]; Gouy, Collon et al. 2024, KarstNSim, *J. Hydrology* [title; HAL blocked the fetch] | The same idea from the other field. SKS and pyKasso walk fast-marching fronts from inlets to outlets through a cost volume. KarstNSim uses a nearest-neighbour graph over sampled points with a geological cost, separate vadose (downward) and phreatic (along the water table) parts | A graph of conduits. No walkable geometry |
| **Game methods: cellular automata, 3D noise, L-systems** | Johnson, Yannakakis, Togelius 2010, *Cellular automata for real-time generation of infinite cave levels*, PCGames [memory]; Cui, Chow, Zhang 2011, *A procedural technique for generating 3D caves*, Computers & Graphics [memory]; Mark, Berechet, Mahlmann, Togelius 2015, *Procedural Generation of 3D Caves for Games on the GPU*, FDG [memory]; Deep Rock Galactic dev blog (decision 0003, sources) | CA: a 2D grid smoothed by a 4–5 neighbour rule gives organic single-level caves. 3D noise thresholds give "swiss cheese". L-system turtles carve tunnels into a voxel or metaball field | 2D mask, or dense voxels |
| **Skeleton graph skinned to a mesh** | Garcia, Richard, Olivares-Mendez 2025, PLUME, arXiv 2508.20926 [opened] | A multi-layer skeleton graph from a Gaussian–Perlin walk; Blender geometry nodes skin it into tubes. Made for robot simulation (lava tubes) | Graph nodes (id, parent, position, radius), mesh |
| **Speleogenesis simulation** | Dreybrodt, Gabrovšek, Romanov 2005, *Processes of Speleogenesis: A Modeling Approach* [memory] | Dissolution widens fractures; flow concentrates in the widest; breakthrough makes conduits. Physically right, slow, and gives fracture apertures, not walkable caves | Fracture-network apertures |

What they agree on: **the network is a graph first.** Paris et al. and the hydrogeology tools
build the same thing by the same means (least-cost paths through a geological cost). Geometry
comes second and is a matter of taste and storage.

What is state of the art: Paris et al. 2021 is still the reference in graphics. My searches for
2024–2026 found PLUME and KarstNSim but no newer graphics paper that replaces it. Full
speleogenesis is not used for content anywhere I found.

### 1.2 How a cave system can be stored

| Storage | Size for 2 km × 2 km × 300 m | Overlaps | Walk mesh, pins, Atlas | Fit with format 1.0 |
|---|---|---|---|---|
| Dense voxels at 2 m | 150 M voxels, 150 MB at one byte | Any | Needs a 3D viewer; pins need 3D | None: no volume type |
| Sparse voxels / stored SDF | 10–50 MB | Any | As above | None |
| SDF construction tree (parameters only) | Kilobytes | Any | The reader must evaluate it; reader and generator must agree bit for bit | New entry type; puts generator logic into `theos-map-format` |
| **Graph of chambers and passages** | Kilobytes | Any | Good for routes and playability; no surface | Fits: features with 3D coordinates and per-vertex arrays (§5 of the format) |
| **Stacked 2.5D levels (slabs)** | 1000² cells × (f32 floor + f32 ceiling + u8) ≈ 9 MB per level raw, far less deflated | Only between levels, not inside one | Each level is a map: `base-<level>` render, pins with `level`, 2D path search | Close: levels exist; needs sloping floors and a ceiling layer |

Why slabs fit real caves: karst caves develop along **inception horizons**, a few favourable
bedding planes, and along past water-table positions, so real systems are tiered: near-flat
storeys joined by shafts and ramps (the premise of Paris et al. 2021 and KarstNSim). A tier is a
level. A shaft is a `link`.

### 1.3 Recommendation

**Graph first, slabs second, in a new map kind `cavern`.** Stages:

| Stage | Reads → writes | Method |
|---|---|---|
| `cave_geology` | frame, [parent package layers] → `horizons`, `fracture_field`, `cave_cost` | A few inception horizons as gently warped surfaces (spectral noise kernel), two fracture sets as direction fields, a water table. From a parent terrain package: sinks become inlets, springs and river banks outlets (a child map, as P29 defines) |
| `cave_samples` | `horizons` → `cave_nodes` | Jittered 3D lattice of candidate nodes, dense near horizons. A lattice instead of Poisson-disk sampling keeps neighbour search trivial and ordered |
| `cave_skeleton` | `cave_nodes`, `cave_cost` → `cave_graph` | Dijkstra with total order from each inlet to its outlets over the k nearest neighbours; edge cost = length × geology terms (cheap along horizons and fractures, costly across beds), with a discount on edges already used so passages merge into trunks, as the road router does (P08 §5.9). Extra loops by the road network's `loop_detour` rule. This is Paris et al. 2021 §skeleton on our kernels |
| `cave_chambers` | `cave_graph` → `cave_graph` | Chambers at junctions and where horizons meet fractures; radius by node degree and discharge through the node (pipe-model sum). Assign every node to a tier |
| `cave_worked` | `cave_graph` → `cave_graph` | The `worked` value per node, 0 natural to 1 built (below) |
| `cave_surfaces` | `cave_graph` → per level `height`, `ceiling`, `walkable` | Rasterise per tier: a swept profile around each edge, unioned; floor flattened by a fill rule (sediment floors are flat, like the lake rule); noise on walls and roof. Checks: clearance ≥ a minimum, levels do not intersect |
| `cave_features` | graph, surfaces → `features` | `chamber`, `passage`, `link` (shaft, ramp, drop), entrances |
| playability | as P29/P30 | Every chamber reachable; routes across levels (CE-79.2's analysis reused) |

**The transition from cavern to built labyrinth.** I found no paper on this blend. Dungeon
generation is its own literature (van der Linden, Lopes, Bidarra 2014, *Procedural generation of
dungeons*, IEEE TCIAIG [memory]) and treats caves and built dungeons as separate generators. My
proposal uses the fact that both are a graph of spaces and connections:

- One scalar per node, `worked` ∈ [0, 1], from a recipe field (for example rising toward the
  labyrinth's core, or with the floor number inside the band).
- It drives three things at once:
  1. **Layout:** above a threshold, edges snap to the labyrinth's axes (Manhattan paths on a
     coarse lattice) and chambers snap to rectangles. Below it, they stay as routed.
  2. **Profile:** the cross-section blends from a warped ellipse to a rectangle or vault; wall
     noise amplitude scales with 1 − `worked`.
  3. **Features:** above a threshold, a chamber is emitted as a `room` and a passage as a
     corridor in P29's vocabulary, so P29/P30's interior stages (doors, keys, locks) can run on
     the built part.
- Middle values give what the band asks for: hewn passages that follow old cave lines, natural
  chambers with cut stairs.

This needs P29's interior generator to accept a graph it did not make. That is a design
constraint to raise before P29's spec is written: **rooms and connections as a shared typed
value (`space_graph`) both kinds read.**

### 1.4 The five points

1. **Licence.** Paris et al.'s skeleton code (`aparis69/Karst-Synthesis`) is **MIT** [opened];
   it may be read. It has no implicit-surface part. **pyKasso is GPL-3.0** [opened]: do not
   read its code; its papers are fine. KarstNSim's code licence: not verified (HAL blocked);
   treat as do-not-read until checked. PLUME's code licence: not stated in the paper.
2. **Bit-identical.** Yes. Lattice sampling, Dijkstra with total order, distance and noise
   kernels: exact operations and `libm` only. The gridless anisotropic search of the paper is
   replaced by Dijkstra on a fixed neighbour graph, which is easier to make ordered.
3. **Stage contract.** Yes. The graph is a typed non-raster value like `road_network` (§5.3).
   Per-level rasters need the world to hold a layer per level; P30 needs that anyway.
4. **Format needs** (all additions, minor version):
   - kind `cavern`; standard layers `height` (floor, sloping) and `ceiling` per level, plus
     `walkable`;
   - a clear statement that `frame.levels[].elevation_m` is a nominal elevation when the level
     has its own `height` layer;
   - feature kinds `chamber` (Polygon) and `passage` (LineString with z, `radius_m` and
     `worked` per vertex); `link` types `shaft` and `ramp`;
   - the "at most about 2 km" note of §4.3 revisited if a cave floor is larger (8 km at 4 m
     cells is the same 2,000² cells).
5. **Size.** Large: a new map kind, about eight stages. It shares levels, links and
   playability with P30 and the router and network rules with P08, so it should come after P30.

Not recommended: cellular automata as the main method (one level, no control over where
passages go, no geology); they remain useful as a wall-roughening kernel. 3D noise caves: no
guarantee of connection or walkability, and they need voxels.

## 2. Needle peaks, tower karst, pinnacles above a void (band 5, type B)

### 2.1 How the real ones form

| Landform | Rock and structure | Process | Source |
|---|---|---|---|
| Sandstone peak forest (Zhangjiajie) | Thick, near-flat quartz sandstone with dense vertical joints | Four stages: slow lowering after uplift; fast dissection of the plateau as knickpoints migrate headward **along joints and faults**; then **wall retreat** dominates; finally the pillars are consumed. Rates: 83–122 mm/kyr in the slots at pillar bases, 30–84 on walls, 2.5 on the tops. Walls up to 350 m | May et al. 2018, *Earth Surf. Process. Landforms* 43(3) [opened: abstract] |
| Tower karst (fenglin), cockpit karst | Massive limestone, fractured | Dissolution, focused where water gathers; the plain lowers to the water table and leaves residual towers. A landscape-evolution model (CHILD) reproduced cockpit karst only when dissolution was **anisotropic in space and coupled to infiltration into fractures**; uniform dissolution failed | Fleurant, Tucker, Viles 2008, *Geol. Soc. London Spec. Publ.* 296 [opened: abstract]; Kaufmann & Braun 2001, *Terra Nova* [memory] |
| Stone forest pinnacles (metre scale) | Soluble rock under water | Dissolution with gravity-driven flow sharpens tips by itself | Huang, Tong, Shelley, Ristroph 2020, *PNAS* 117(38) [opened] |
| Tepuis, mesas | Flat hard caprock over weaker beds | Scarp retreat by undercutting | Beneš & Arriaga 2005, *Table mountains by virtual erosion*, EG Workshop on Natural Phenomena [memory] |

The common cause: **strong rock that fails along vertical planes, a weakness pattern (joints)
that decides where incision goes, and a low base level.** The tops barely erode. None of this is
in P08's model: its erodibility is 2D per rock province, and its channels go where area
accumulates.

### 2.2 Can a heightfield with strata and a slope limit make them?

| Question | Answer |
|---|---|
| Can a heightfield **represent** pillars? | Yes, while walls are vertical or lean outward. Not bulbous tops, undercut bases or natural bridges (those are §3 features) |
| At 62.5 m cells? | No. Pillars are 10–100 m wide. They need cells of 10 m or finer: a 24 km floor at 10 m is 2,400², inside what decision 0003 §5 costs at 45–125 s for 2048² |
| Does stream power with `slope_limit` make them? | No. With uniform K it gives dendritic V-valleys. A high critical slope only allows steep walls; it does not create isolated columns |
| With strata K(x, y, z) (Cordonnier et al. 2018, *Sculpting Mountains*, IEEE TVCG [memory]; named as a later stage in P08 §5.9)? | Gives cliffs, benches and mesas. Still no pillars: nothing breaks a mesa into columns |
| With strata **and** a joint pattern in K? | Gives the first two of May et al.'s stages: a plateau cut by a lattice of slots into blocks and walls. This is the part an LEM is good at |
| Wall retreat into pillars? | Not a stream-power process. Diffusion rounds walls instead of moving them back. It needs its own rule |

So: **representable yes, simulated only in part.** The missing parts are a fracture pattern and
a retreat rule.

### 2.3 Methods in graphics

| Method | Source | Use here |
|---|---|---|
| Layered rock with hardness per layer, heightfield erosion | Beneš & Arriaga 2005 [memory]; Cordonnier et al. 2018 [memory] | Strata K(z): caprock and benches |
| Rock joints as the driver of weathering | Ito, Fujimoto, Muraoka, Chiba 2003, *Modeling rocky scenery taking into account joints*, CGI [memory]; Jones, Farley, Butler, Beardall 2010, *Directable weathering of concave rock using curvature estimation*, IEEE TVCG [memory] | Voxel methods for single outcrops (hoodoos, goblins). Right physics, wrong scale and storage |
| Implicit hoodoos and 3D features on a heightfield | Paris, Galin, Peytavie, Guérin, Gain 2019, *Terrain Amplification with Implicit 3D Features*, ACM TOG 38(5) [opened: repo page; title] | Detail on pillars (caps, undercuts), engine side |
| Peak statistics as control | Argudo, Galin, Peytavie, Paris, Gain, Guérin 2019, *Orometry-based terrain analysis and synthesis*, ACM TOG [memory] | Metrics: prominence and isolation of peaks as constraints for a "thousand peaks" recipe |

I found no paper that simulates a Zhangjiajie-type peak forest at landscape scale, in graphics
or in geomorphology. The recommendation below is built from May et al.'s stages and existing
kernels; it is a design, not a published method.

### 2.4 Recommendation

Three additions, all heightfield, all in map kind `terrain`:

1. **`joints` stage** (before evolution): two or three joint sets, each a family of near-parallel
   lines with jittered spacing (tens to hundreds of metres) and a warped direction. Writes
   `joint_field` (distance to the nearest joint, set id). `geology` multiplies erodibility near
   joints in rock types flagged as jointed. Kernels: `curves`, `distance`, `warp`.
2. **`strata` in `geology`** (the stage P08 already defers): rock as K(x, y, z) through a small
   stack of beds per province, each with K, kd and critical slope; the per-step processes look up
   the bed at the current surface height. Caprock beds get a critical slope near vertical.
3. **`wall_retreat` per-step process:** where a cell's local relief exceeds the bed's cliff
   threshold, the cliff foot moves back at a rate per year (May et al. give 30–84 mm/kyr), faster
   along joints. Implemented as a height drop of cliff-edge cells to the level of their lower
   neighbour when their accumulated retreat reaches one cell, with the material booked as
   erosion and passed to `fluvial` as sediment. Tops stay (2.5 mm/kyr). Run long enough, blocks
   shrink to walls, then pillars, then nothing, which is the real sequence; the recipe's `steps`
   picks the stage.

For tower karst instead of sandstone, swap process 3 for a **`dissolution`** process: lowering
rate proportional to runoff reaching the cell and to a solubility per rock, stopped at a
water-table height, stronger near joints (Fleurant et al.'s anisotropic case; Kaufmann & Braun).
Same reads and writes.

Around it:

- **Above a void or cloud sea:** drained edges at a low base level already express this; the
  valley floors are hidden, so their fidelity matters least. The `water_surface` idea extends to
  a `cloud_surface` constant in the recipe; no generator work.
- **Features:** each pillar becomes a `landform`/`pillar` feature (Polygon footprint, `top_m`,
  `base_m`, `height_m`) by labelling connected cells above a prominence threshold (kernel
  `labels`). Roads, sites and band-5 settlements need them as places.
- **Bridges between pillars** are `site`/`bridge` features with `deck_m` and already exist.
- **Metrics for constraints:** pillar count, median height, median footprint, share of area
  above the cloud level.

### 2.5 The five points

1. **Licence.** No code needed. `aparis69/Implicit-Volumetric-Terrains` is MIT [opened] if
   hoodoo detail is wanted later; its hoodoo grammar is not in the repo.
2. **Bit-identical.** Yes: line distance fields, table look-ups, ordered cell passes like
   `slope_limit`.
3. **Stage contract.** Yes. `wall_retreat` and `dissolution` are per-step processes; `joints` is
   a pipeline stage; strata change `geology`'s outputs from 2D fields to a bed table plus 2D
   fields.
4. **Format needs.** None that is breaking. Additions: feature kind `landform`; optionally the
   `rock` legend per bed. A finer grid is a recipe matter.
5. **Size.** Medium: three stages, one of them (strata) already planned. The cost is the grid:
   2,048² or more for a whole floor, or pillars only in child maps of a coarse floor.

Risk: at fine grids the implicit fluvial solver is fine, but 240 steps of ordered retreat passes
at 2,400² need measuring. Fallback: run evolution coarse, then run `wall_retreat` once as a
constructive pass on an upsampled grid (morphological erosion of the plateau mask by a distance
that grows with exposure time).

## 3. Volumetric and floating terrain (band 8, type F; rims)

### 3.1 The strongest methods

| Method | Source | What it does | Result data |
|---|---|---|---|
| **Material stacks (Arches)** | Peytavie, Galin, Grosjean, Mérillou 2009, *Arches: a Framework for Modeling Complex Terrains*, CGF 28(2) [title] | Each grid column is a list of runs: bedrock, sand, water, air. An implicit surface is convolved from the stacks for rendering. Sculpting tools and a stabilisation step. Makes arches, overhangs, caves | Run-length columns; compact; 2D-indexed |
| **Implicit 3D features on a heightfield** | Paris et al. 2019, ACM TOG 38(5) [opened: repo; title] | The heightfield becomes an implicit surface. Overhangs, arches, caves, hoodoos and sea cliffs are added as SDF primitives placed where erosion-like analysis says they belong. "Amplification": the base terrain stays the truth | Heightfield + a list of placed primitives with parameters |
| **Sparse construction tree of feature primitives** | Génevaux, Galin, Peytavie, Guérin, Briquet, Grosbellet, Benes 2015, *Terrain Modelling from Feature Primitives*, CGF 34(6) [title] | Terrain as a tree of primitives (ridges, hills, rivers, cliffs) combined by blend, replace and add operators. Note: its primitives are elevation functions, so the result is still a heightfield. Its value is the authoring model | A tree of parameters |
| **Feature curves diffused into a volume** | Becher, Krone, Reina, Ertl 2017, *Feature-based volumetric terrain generation*, I3D [memory] | 3D curves and surfaces with constraints, diffused on the GPU into a density volume | Dense volume |
| **Erosion on volumes** | Beneš, Těšínský, Hornyš, Bhatia 2006, *Hydraulic erosion*, CAVW (voxel fluid) [memory]; Jones et al. 2010 [memory]; Hartley, Mellado, Fiorio, Faraj 2024, *Flexible terrain erosion*, The Visual Computer [opened: thin page; title] | Erosion that does not assume a heightfield. Hartley et al.'s particle method runs on heightfields, voxels and implicit terrain alike | Voxels or implicit |
| **Procedural SDF scenes** | Raistrick et al. 2023, *Infinigen*, CVPR (BSD-3) [memory] | Terrain as composed SDF elements (caves, warped rocks, floating islands), meshed by marching cubes | Meshes |
| **Neural 3D terrain** | EarthCrafter, arXiv 2507.16535, 2025 (sparse latent diffusion of 3D earth) [title]; Merino et al. 2026, *Dream Cubed*, arXiv 2604.22847 (3D diffusion transformer over 32³ Minecraft chunks, caves in the data) [opened]; Goslin 2025, *InfiniteDiffusion*, arXiv 2512.08309 (unbounded terrain from a stack of diffusion models) [opened: abstract] | Learned volumes or heightfields | Voxels or heightmaps |

Reading of the field: there is **no physical model of floating islands**, because there are
none. Volumetric terrain in research is either authored (Arches, Becher), amplified from a
heightfield (Paris 2019), or learned. The newest neural work is real but excluded here by G20
and decision 0003: a learned step is a separate generator whose output is baked.

### 3.2 Recommendation: slabs, plus parametric features

**Slabs.** A slab is a level with a top raster and an underside raster and a mask. It is Arches'
material stack with a small fixed number of runs, written in the format's own terms (levels).

| Case | As slabs |
|---|---|
| Floating island | One level: `height` = its top, `underside` = its bottom, `solid` mask |
| Broken ground (Shatter) | The ground level plus one level per lifted or tilted plate |
| Overhanging rim over the Bore | The ground level gets an `underside` along the rim |
| Large arch, land bridge | A small level spanning two supports |
| Cave (§1) | The same pair read the other way: `height` = floor, `ceiling` |
| Tree tier (§4) | Top of limbs and platforms, underside |

Stages for band 8, in kind `terrain`, after the normal pipeline:

| Stage | Method |
|---|---|
| `fracture` | Split the floor into plates: a crack network from Voronoi cells of jittered seeds (`voronoi` kernel), biased to follow `joint_field` and river lines. Writes `plate_id` |
| `displace_plates` | Per plate: a lift, a tilt (a plane) and a drift, from the recipe and the stage's random stream; plates above a lift threshold leave the ground level and become their own level. The ground gets a crater or scar where a plate left |
| `undersides` | Underside = top − thickness, thickness = f(distance to the plate edge) × noise: thick in the middle, tapering to the edge, with hanging roots of rock. The exact Euclidean distance kernel does this |
| `slab_links` | Gaps between plates measured (jump distance, drop height); `link` features (`drop`, bridge, climb) and the playability graph across levels |

Erosion on floating plates is not simulated. The plates carry the eroded surface they were cut
from, which is what "broken ground" should look like.

**Parametric 3D features** for what is smaller than a slab: `landform` features of type `arch`,
`overhang`, `hoodoo`, `sea-stack` with position, size, orientation and a seed, placed by rules
on slope, rock and exposure as in Paris et al. 2019. The package stores only the parameters; the
engine realises them. The heightfield stays walkable truth, so pins and paths do not depend on
how the engine draws an arch.

Not recommended: voxels or a stored SDF in the package (size, a volume type, 3D pins, and no
`base` render); the SDF construction tree as package content (it would put evaluation code into
`theos-map-format`, which must stay small).

### 3.3 The five points

1. **Licence.** `Implicit-Volumetric-Terrains` MIT [opened]; Infinigen BSD-3 [memory]. No code
   is needed for slabs.
2. **Bit-identical.** Yes: Voronoi through `spade` (exact predicates), planes, distance
   transforms, spectral noise.
3. **Stage contract.** Yes, with one extension P30 also needs: layers addressed by level.
4. **Format needs** (minor version):
   - levels allowed on kind `terrain` (today §4.3 speaks of indoor maps);
   - standard layers `underside` (f32) and `solid` (u8 mask) per level;
   - a level may cover only part of the frame (mask), all on the shared grid;
   - feature kind `landform` with parametric types; `link` between terrain levels.
5. **Size.** Medium for band 8 (four stages) once levels exist in the world. The format change
   is small but needs Jann (protected document).

## 4. Climbable giant trees as terrain (band 3, type I)

### 4.1 The strongest methods

| Method | Source | What it does | Fit |
|---|---|---|---|
| **Space colonization** | Runions, Lane, Prusinkiewicz 2007, *Modeling Trees with a Space Colonization Algorithm*, EG Workshop on Natural Phenomena [title] | Attraction points fill a crown volume. Each step, every point pulls its nearest branch tip; tips grow one segment toward the mean pull; reached points are removed. Radii afterwards by the pipe model (r_parent^n = Σ r_child^n, n ≈ 2–3) | **Best.** The crown envelope and the points are direct controls: put points where a platform or a village must be |
| **Self-organising trees** | Palubicki, Horel, Longay, Runions, Lane, Měch, Prusinkiewicz 2009, *Self-organizing tree models for image synthesis*, SIGGRAPH [title] | Buds compete for space or light; resources flow by a priority model (Borchert–Honda); shedding | More natural, less direct control |
| **Parametric and L-system trees** | Weber & Penn 1995, SIGGRAPH [memory]; Prusinkiewicz & Lindenmayer 1990, *The Algorithmic Beauty of Plants* [memory] | Recursive rules with angles and ratios per level | Easy and deterministic; hard to steer to a place |
| **Recent: strands, roots, learned** | Li et al. 2024, *Interactive Invigoration: Volumetric Modeling of Trees with Strands*, SIGGRAPH [memory]; Li et al. 2023, *Rhizomorph*, SIGGRAPH Asia [memory] | Bark, branch junction and root geometry of high quality | Engine-side geometry, not layout |

**Trees as levels:** I found no academic work on a tree as a playable level structure. Games
build such places by hand. So the layout rules below are a proposal.

A real tree's proportions do not scale to a kilometre (the brief says the physics are magic), so
realism of growth matters less than three gameplay facts: limbs wide enough to walk, slopes a
player can take, and a connected route.

### 4.2 Recommendation

**Space colonization under walkability rules, graph as canon, tiers as slab levels.** New map
kind `tree`.

| Stage | Method |
|---|---|
| `tree_envelope` | Crown volume and tiers from the recipe: tier heights, radius per tier, attraction points in a thin shell per tier, plus designer points (platform sites). From a parent terrain: the trunk's position and ground height |
| `tree_skeleton` | Space colonization per tier from the trunk outward. Rules: segment inclination on walkable limbs below a limit (for example 30°); limbs of one tier do not cross in plan (so a tier is a valid slab); a minimum fork spacing. Nearest-tip search over a fixed 3D hash grid in `BTreeMap` order |
| `tree_radii` | Pipe model from the tips inward; a floor on limb width for walkable class |
| `tree_classify` | Each edge: `limb` (walkable top), `climb` (too steep: becomes a `link`), `twig` (decoration, features only). Crotches and designer points become `platform` polygons |
| `tree_tiers` | Rasterise each tier: `height` = top of limbs and platforms (a swept half-profile), `underside`, `solid`. The trunk's inside can be a normal indoor map (P30) as a child |
| `tree_roots` | On the parent terrain: buttress roots as ridges added to `height` around the trunk, and a `landform`/`trunk` polygon, so type A terrain and type I meet without a seam |
| playability | Route from the ground to every platform; widths and slopes checked |

Storage:

- **Canon: the skeleton graph**, as features. `tree`/`limb` LineStrings with z and `radius_m`
  per vertex, `parent` by id, `tier`. Format 1.0 already allows 3D coordinates and per-vertex
  arrays; a 1 km tree with 5,000 segments is well under 1 MB.
- **Walkable surfaces: slab levels per tier** (§3.2), so the Atlas, pins with `level`, and 2D
  path search work unchanged.
- **Not stored:** bark mesh, twigs, leaves. The engine sweeps generalised cylinders or an SDF
  of capsules along the graph.

### 4.3 The five points

1. **Licence.** Both key papers are public; no code needed. The algorithm is about 150 lines.
2. **Bit-identical.** Yes. Vector sums, `sqrt` normalisation and ordered neighbour search are
   exact operations; the removal order of attraction points must be by index.
3. **Stage contract.** Yes. The skeleton is a typed non-raster value like `road_network`.
4. **Format needs.** Kind `tree`; feature kind `tree` (types `trunk`, `limb`, `platform`);
   `link` type `climb`; slab layers as in §3. No mesh, no volume.
5. **Size.** Large: a new map kind of about seven stages. Small trees as scenery on normal
   terrain are a different thing and stay with vegetation density.

## 5. Other landforms stream power does not make

**Glacial.** The best-looking current method is Cordonnier et al. 2023, *Forming Terrains by
Glacial Erosion* (ACM TOG 42(4)) [opened]: U-valleys, hanging valleys, cirques, arêtes, horns
and fjords from ice flow, abrasion and quarrying, coupled to fluvial and hillslope erosion. But
its ice flow is a **convolutional network** trained on a higher-order ice model, and it took
9.8 min at 1024² on a GPU workstation; G20 rules it out, and its code licence is not stated in
the paper. The fit for this generator is Hergarten 2021, *Modeling glacial and fluvial landform
evolution at large scales using a stream-power approach* (Earth Surf. Dynam.) [memory; already
named in P08 §5.9]: ice flux replaces water discharge above an equilibrium line and the channel
"width" becomes the glacier's, inside the same implicit solver. One per-step process `glacial`
next to `fluvial`; deterministic; no format change beyond an `ice` layer. Small to medium.
Moraines are missing in both.

**Volcanic.** P08's `tectonics` already has a volcano source (a cone of uplift). What is
missing is lava: flows, fields and tubes. Volcanology's fast models put stochastic lobes of lava
down the steepest slope and add their thickness to the surface (MrLavaLoba: de' Michieli Vitturi
& Tarquini 2018, *J. Volcanol. Geotherm. Res.* [title; licence not checked, papers only]). That
is a heightfield process: one stage `lava_flows` that writes thickness into `height` and a
basalt class into `rock`, run between evolution phases so young flows sit on eroded ground.
Calderas and cones are constructive primitives in the sense of Génevaux et al. 2015.
Deterministic with the stage's random stream. Small. Lava tubes belong to §1 as a cave recipe
with a different cost field (PLUME targets exactly those).

**Arid and aeolian.** The reference is Paris, Peytavie, Guérin, Argudo, Galin 2019,
*Desertscapes: Procedural Generation of Desert Landscapes* (CGF 38(7)) [title; repo opened]:
sand moves by saltation hops, reptation and avalanching over bedrock in a two-layer heightfield
under a terrain-aware wind field, after Werner's 1995 cellular model [memory]. It gives
barchans, transverse, linear and star dunes, nabkhas, and yardangs by abrasion. The recoded
source is MIT [opened] (no wind roses). One stage `aeolian` after evolution, with a `sand`
thickness layer; hop order must be fixed and random draws come from the stage stream, so it can
be bit-identical. P08 lists "several wind directions" as later; this stage needs that. Small to
medium. Mesas and canyons of arid lands come from §2's strata, not from wind.

**Coastal.** A floor that ends at a rim over a void may have no sea, so only lakes matter
unless the floor frame says otherwise. Research offers two levels. For plan-view shorelines,
the coastline evolution model of Ashton & Murray 2006 (*J. Geophys. Res.*) [memory] moves sand
alongshore by wave angle and makes spits and capes; it is more than a lake shore needs. For the
profile, sea cliffs and stacks are among the implicit features of Paris et al. 2019 (§3).
Recommended: a small stage `shoreline` after `hydrology`: wave exposure per shore cell from
fetch (distance across open water along the wind directions, a ray walk on the grid), a wave-cut
bench and beach deposit scaled by exposure, cliffs where rock is hard. Deltas need no new
stage: P08's deposition already fills lakes from river mouths. Deterministic; no format change
beyond a `beach` class. Small.

## 6. Table

| Topic | Recommended method | New stage or new map kind | Package format need (beyond 1.0) | Size |
|---|---|---|---|---|
| Caves, cavern network (band 4) | Geology-cost shortest-path skeleton (Paris et al. 2021), rasterised into tiers | New map kind `cavern`, ~8 stages | Sloping `height` + `ceiling` per level; kinds `chamber`, `passage`; `link` types `shaft`, `ramp` | Large |
| Cavern → labyrinth | `worked` value 0–1 on the same graph, driving layout, profile and feature vocabulary (own proposal) | One stage in `cavern`; a shared `space_graph` value with P29/P30 | `worked` property | Small, but constrains P29's design |
| Needle peaks, sandstone pillars (band 5) | Joint pattern + strata K(x, y, z) + wall-retreat process (after May et al. 2018; own design) | Stages `joints`, strata in `geology`, process `wall_retreat`; kind `terrain` | Feature kind `landform`/`pillar`; fine grid (≤ 10 m) | Medium |
| Tower karst | Same, with a `dissolution` process instead of wall retreat (Fleurant et al. 2008) | One per-step process | None | Small |
| Floating and broken ground (band 8), rims | Plates cut by Voronoi cracks, displaced, with distance-based undersides: slabs (Arches' stacks, bounded) | Stages `fracture`, `displace_plates`, `undersides`, `slab_links`; kind `terrain` | Levels on `terrain`; `underside`, `solid` layers; partial levels | Medium |
| Arches, overhangs, hoodoos | Parametric features placed by rules (Paris et al. 2019), realised by the engine | One stage `landforms_3d` | Feature kind `landform` with parameters | Small |
| Giant trees (band 3) | Space colonization (Runions et al. 2007) under walkability rules; tiers as slabs | New map kind `tree`, ~7 stages | Kind `tree`; limbs as 3D lines with `radius_m`; `link` type `climb`; slab layers | Large |
| Glacial | Hergarten 2021 stream-power glacier | Per-step process `glacial` | `ice` layer (optional) | Small–medium |
| Volcanic | Stochastic lava lobes + cone and caldera primitives | Stage `lava_flows` | None (rock legend) | Small |
| Aeolian | Desertscapes sand transport on bedrock + sand | Stage `aeolian` | `sand` layer | Small–medium |
| Coastal (lake shores) | Fetch-based wave exposure, bench and beach | Stage `shoreline` | `beach` class | Small |

## 7. What this means for the order of work

1. **Decide slabs first.** One format addition (levels on any kind, `underside`/`ceiling`,
   `solid`, sloping level `height`) serves bands 3, 4 and 8 and the rims. It is a change to a
   protected document (`docs/map-package.md`), so it is Jann's call, and it is cheapest before
   P30 fixes how the world holds levels.
2. **Heightfield stages come first in cost order:** strata, `joints`, `wall_retreat` (band 5),
   then `glacial`, `aeolian`, `lava_flows`, `shoreline` when a floor needs them. All fit P08's
   contract unchanged.
3. **`cavern` and `tree` after P30**, since both reuse levels, links and cross-level
   playability.
4. **Raise with P29's spec:** rooms and connections as a value other kinds can write
   (`space_graph`), or the cavern-to-labyrinth blend needs a second interior generator.

## 8. Open points and limits of this report

- Project docs not read (§0): band requirements, the floor frame and `decisions.md` may change
  sizes and priorities.
- No probe was run. Cost figures are taken from decision 0003 §5 or estimated.
- KarstNSim's paper and code licence were not opened (bot wall). PLUME's and the glacial code's
  licences are not stated in their papers. MrLavaLoba's licence was not checked.
- Sources marked [memory] were not opened in this run: authors, year and venue are as I know
  them and should be checked before they go into a spec.
- §1.3's `worked` blend, §2.4's wall-retreat rule, §3.2's plate method and §4.2's walkability
  rules are my designs from the cited parts, not published methods. Each needs a probe and a
  contact sheet before a spec relies on it.

## Sources

Repo (read-only):

- `docs/specs/phases/P08-map-format-and-terrain-generator.md` §3,
  §4.2, §4.4, §5.3–§5.10
- `docs/map-package.md` §2–§6
- `docs/decisions/0003-map-generator-in-rust.md` §2, §3, §5, §7

Opened in this run:

- Paris, Guérin, Peytavie, Collon, Galin 2021. Synthesizing Geologically Coherent Cave Networks.
  CGF 40(7). <https://perso.liris.cnrs.fr/aparis/public_html/projects/paris2021_Karsts.html>,
  <https://diglib.eg.org/handle/10.1111/cgf14420>; code (MIT):
  <https://github.com/aparis69/Karst-Synthesis>
- Paris, Galin, Peytavie, Guérin, Gain 2019. Terrain Amplification with Implicit 3D Features.
  ACM TOG 38(5). <https://hal.archives-ouvertes.fr/hal-02273097>; code (MIT):
  <https://github.com/aparis69/Implicit-Volumetric-Terrains>
- Garcia, Richard, Olivares-Mendez 2025. PLUME: Procedural Layer Underground Modeling Engine.
  <https://arxiv.org/html/2508.20926v1>
- pyKasso (GPL-3.0; Fandel et al. 2022; Borghi, Renard, Jenni 2012).
  <https://pypi.org/project/pykasso/>
- May, Huang, Fujioka, Fink, Codilean, Yu, Ma, Wulf, Gu 2018. Evolution of sandstone
  peak-forest landscapes. Earth Surf. Process. Landforms 43(3), doi 10.1002/esp.4273.
  <https://apo.ansto.gov.au/dspace/handle/10238/12499>
- Fleurant, Tucker, Viles 2008. Modelling cockpit karst landforms. Geol. Soc. London Spec.
  Publ. 296, doi 10.1144/SP296.4. <https://digitalcommons.usf.edu/kip_articles/9564>
- Huang, Tong, Shelley, Ristroph 2020. Ultra-sharp pinnacles sculpted by natural convective
  dissolution. PNAS 117(38). <https://pmc.ncbi.nlm.nih.gov/articles/PMC7519309>
- Cordonnier, Jouvet, Peytavie, Braun, Cani, Benes, Galin, Guérin, Gain 2023. Forming Terrains
  by Glacial Erosion. ACM TOG 42(4).
  <https://people.cs.uct.ac.za/~jgain/wp-content/papercite-data/pdf/cordonnier2023.pdf>
- Desertscapes code (MIT): <https://github.com/aparis69/Desertscapes-Simulation>; paper: Paris,
  Peytavie, Guérin, Argudo, Galin 2019, CGF 38(7), <https://diglib.eg.org/handle/10.1111/cgf13815>
- Merino, Earle, Iwai, Togelius, Cetin 2026. Dream Cubed. <https://arxiv.org/html/2604.22847v1>
- Goslin 2025. InfiniteDiffusion. <https://arxiv.org/abs/2512.08309>
- Hartley, Mellado, Fiorio, Faraj 2024. Flexible terrain erosion. The Visual Computer.
  <https://marchartley.github.io/publication/Flexible-terrain-erosion> (page gave little detail)

Found by search, not opened ([title]):

- Gouy, Collon, et al. 2024. KarstNSim: A graph-based method for 3D geologically-driven
  simulation of karst networks. J. Hydrology. <https://hal.univ-lorraine.fr/hal-04468018>
  (fetch blocked)
- Peytavie, Galin, Grosjean, Mérillou 2009. Arches: a Framework for Modeling Complex Terrains.
  CGF 28(2). <https://perso.liris.cnrs.fr/egalin/Articles/2009-arches.pdf>
- Génevaux, Galin, Peytavie, Guérin, Briquet, Grosbellet, Benes 2015. Terrain Modelling from
  Feature Primitives. CGF 34(6). <https://diglib.eg.org/handle/10.1111/cgf12530>
- Runions, Lane, Prusinkiewicz 2007. Modeling Trees with a Space Colonization Algorithm.
  <https://www.algorithmicbotany.org/papers/colonization.egwnp2007.html>
- Palubicki et al. 2009. Self-organizing tree models for image synthesis.
  <https://www.algorithmicbotany.org/papers/selforg.sig2009.html>
- EarthCrafter 2025. <https://arxiv.org/html/2507.16535v1>
- de' Michieli Vitturi & Tarquini 2018, MrLavaLoba; see
  <https://nhess.copernicus.org/articles/23/3147/2023/> for a later use

From memory, not opened ([memory]): Johnson, Yannakakis, Togelius 2010; Cui, Chow, Zhang 2011;
Mark, Berechet, Mahlmann, Togelius 2015; Dreybrodt, Gabrovšek, Romanov 2005; van der Linden,
Lopes, Bidarra 2014; Kaufmann & Braun 2001; Beneš & Arriaga 2005; Cordonnier et al. 2018; Ito et
al. 2003; Jones et al. 2010; Argudo et al. 2019; Becher et al. 2017; Beneš et al. 2006;
Raistrick et al. 2023 (Infinigen); Weber & Penn 1995; Prusinkiewicz & Lindenmayer 1990; Li et
al. 2023 and 2024; Hergarten 2021; Werner 1995; Ashton & Murray 2006.
