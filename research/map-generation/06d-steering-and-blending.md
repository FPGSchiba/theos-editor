# 06d · Steering and blending procedural maps (stage 5b, authoring)

## Summary

1. Steer through the **inputs of the physics**, never by editing its outputs: uplift, erodibility,
   climate and height-constraint layers, plus numbers. Schott et al. 2023 is the direct match
   (same stream-power family, MIT code, includes uplift inversion and consistent blending).
2. Any sketch or learned tool (cGAN, diffusion) stays compatible with G20 **if it runs outside the
   generator and its result is a hashed input layer**. The generator then only resamples it.
3. The 2-of-8 pass rate is a variance problem in three scalars (relief, lake share, vegetation).
   Fix it with calibration (solve a gain per target), early checks per stage and partial re-rolls,
   not with more seeds.
4. Lore demands ("castle on a hill above a river", "three Chimes an hour apart") are
   **constraint-first placement** on rasters and the travel-time field, with small repair stamps
   (flatten, bridge) as fallback. ASP and SAT solvers fit room graphs, not 384² rasters.
5. Published map metrics that work on a raster with a road graph: path lengths, chokepoint
   width, path overlap, area control, exploration, viewshed and exposure, distance to cover.
   Bot-simulation metrics do not. I found no peer-reviewed metric set for extraction games.
6. Blending: interpolate **parameters over floor number** and blend **fields with a mask before
   erosion**. Cross-fading finished height maps breaks drainage. Built kinds (city, halls) need
   mixed module pools or transition tiles, not interpolation.
7. Recipe format needs: typed inputs (`floor`), parameter curves, `extends`, a second parameter
   set with a mask, per-stage seed control, requirements, control layers. All are additions.
8. 100 maps: expand one family per band into 100 concrete recipes, run N seeds each, rank
   automatically, review on contact sheets and a metrics-versus-floor plot, pin the picks.
9. I found no postmortem that states designer hours per generated level; effort figures below are
   my estimates and are marked as such.

How to read source marks: **[opened]** = I read the page or PDF in this session.
**[hit]** = the search returned the record (title and URL confirmed) but I could not open it, or
did not; what I say about it comes from my prior knowledge of the paper. **[repo]** = a file in
this repository.

## 0. Starting point in the repo

| Fact | Source |
|---|---|
| Terrain recipes take no inputs; a run varies only by seed. "Inputs that set stage parameters can be added later; they would be additions to the recipe schema" | [repo] decision 0003 §9; P08 §5.5 ("recipe inputs are empty") |
| A designer sets: parametric sources by position (`orogen`, `fault`, `rift`, `dome`, `volcano`, `basin`), stage parameters, seed, `[constraints]` as `[min, max]` on summary paths | [repo] P08 §5.5, §5.9 |
| Constraints are evaluated by the runner **after** the whole pipeline | [repo] P08 §5.6 step 4 |
| 0.3.0 reference: `hearth-valley` 2 of 8 seeds pass, `hearth-valley-omega` 3 of 8 | [repo] P08 §5.14, Appendix A |
| The failures are relief (seeds 2, 8), lake share (2, 3, 6, 7, 8) and vegetation cover (5, 6); in the omega recipe also extraction walk time (7, 8). No seed failed a placement count | [repo] P08 Appendix A |
| Each stage has its own random stream derived from (run seed, stage id, occurrence); adding or moving a stage changes no other stage's stream | [repo] P08 §5.7 |
| `travel` already computes Tobler walking time on a 16-direction graph, multi-source Dijkstra from extractions, and the extraction-to-extraction time | [repo] P08 §5.9 |
| Kernels that exist: exact distance transform, connected components, grid Dijkstra, MST, marching squares, priority flood, Gaussian, morphology | [repo] P08 §5.8 |
| Protocol input types today: `feature` (child maps), `cameras`; `from` names a source output | [repo] `docs/generators.md` §3 |
| Package 1.0: f32/u8/u16 rasters, one frame, GeoJSON features; a minor version may add optional layers, kinds and properties | [repo] `docs/map-package.md` §4, §5, §6 |
| P08 research already ranks "inverse authoring (sketch → uplift)" as the first learned track, outside the generator | [repo] P08 §4.2 item 3 |

Two consequences I use below:

- The per-stage random streams mean a **partial re-roll** is already well defined: keep the
  terrain stages' streams and change only the placement stages' streams. The recipe only lacks a
  way to say it.
- Stage metric names are known before a run (P08 §5.3). So a constraint can be bound to the
  earliest stage that determines it and checked there.

## 1. Authoring terrain

### 1.1 Methods

| Method | What the designer gives | How terrain follows | Learned step | Source |
|---|---|---|---|---|
| Terrain sketching (Gain, Marais & Strasser 2009, I3D) | Silhouette, shadow and boundary strokes in 3D | Multiresolution surface deformation; noise propagated from the strokes | No | [prior knowledge only; no search record] |
| Diffusion curves (Hnaidi et al. 2010, Pacific Graphics / CGF 29(7)) | Bézier feature curves (ridges, river beds, cliffs) with constraint points: elevation + plateau radius, slope angle per side, noise amplitude and roughness | Constraints rasterised to elevation, gradient and noise grids, then spread by a multigrid diffusion solver (Jacobi relaxation, fixed iterations per level); detail noise added | No | [opened] |
| Hydrology first (Génevaux et al. 2013, SIGGRAPH / TOG) | Coast contour, river mouths, optional slope and river-density maps | A river graph is grown from the mouths, watersheds from a Voronoi partition, terrain from blended primitives along the rivers | No | [hit; HAL blocked the fetch; prior knowledge. P08 already cites it for river lines] |
| Feature primitives (Génevaux et al. 2015, CGF 34(6)) | A construction tree: disc, curve, contour and image primitives; operators blend, replace, add, warp | Each node has an elevation and a weight function; blending is the weight-averaged elevation | No | [opened] |
| Uplift as the control (Cordonnier et al. 2016, Eurographics / CGF) | An uplift map | Stream-power erosion on a stream graph until uplift and erosion balance | No | [hit; HAL blocked; prior knowledge; P08 §5.9 cites it for `tectonics`] |
| Ecosystem + erosion (Cordonnier et al. 2017, SIGGRAPH) | Painted layers and events | Coupled vegetation and erosion simulation | No | [prior knowledge only; not searched] |
| Sculpting Mountains (Cordonnier et al. 2018, TVCG) | Hand gestures that push tectonic plates; layered subsurface | Folded strata give an uplift field and rock layers, then erosion | No | [hit; prior knowledge] |
| **Large-scale authoring through interactive erosion (Schott, Paris, Fournier, Guérin & Galin 2023, TOG 42(5), art. 162)** | Edits in the **uplift domain**: copy and paste of ranges and valleys, warping for folds and faults, **point and curve elevation constraints** for ridges and river networks | Stream-power simulation turns uplift into terrain. Also: **inverse procedural modelling that reconstructs the uplift from an input elevation model**, and **hydrologically consistent blending between patches** | No | [opened: project page; code repo MIT, opened] |
| Analytical erosion (Tzathas, Gailleton, Steer & Cordonnier 2024, CGF 43(2)) | An input terrain and a time slider | Analytical solutions of the stream-power law; "time … acts as the parameter of a mathematical function"; multigrid | No | [opened: abstract]. The abstract describes forward ageing only; it does not claim inversion or positional constraints |
| River-profile inversion in geomorphology (Goren, Fox & Willett 2014, JGR Earth Surface) | Measured river long profiles | Linear inversion for uplift history; relies on the linear case n = 1 | No | [hit; prior knowledge] |
| Gradient terrain authoring (Guérin et al. 2022, CGF) | Edits to the gradient field | Poisson-type reconstruction | No | [hit; prior knowledge] |
| cGAN sketch to terrain (Guérin et al. 2017, SIGGRAPH Asia / TOG) | Sketched rivers, ridges, altitude cues | A conditional GAN trained on real elevation data; companion networks for erasing and erosion | Yes | [hit; both PDF and HAL fetches failed; prior knowledge] |
| StyleDEM / spatialised style (Perche et al. 2023, Pacific Graphics / CGF) | A sketch, a low-resolution map or a DEM; a mask where a second terrain's style applies | StyleGAN2 generator and encoder; feature maps of two generators mixed with the mask at every resolution | Yes | [opened] |
| Diffusion authoring (Lochner, Gain, Perche, Peytavie, Galin & Guérin 2023, CGF 42) | Style choice and feature sketches; editing of imported terrain; amplification | Conditional diffusion trained on real elevation data. Source code, a Blender add-on and trained models are announced | Yes | [opened: abstract]. Licence not checked |
| TerraFusion (2025, arXiv 2505.04050); Earthbender (MIG 2025) | Sketch-conditioned latent diffusion, height and texture together | — | Yes | [hit only; not opened; listed for completeness, no claims made] |
| Multi-scale erosion amplification (Schott et al. 2024, TOG 43(4), art. 145) | A low-resolution terrain | Fast thermal, stream-power and deposition approximations per scale | No | [opened: project page]. Already P08's refinement path |

### 1.2 Which stay deterministic when a learned step is kept outside and baked

Rule G20 forbids neural inference inside a run (BRIEF; P08 §5.7). The package promise is "same
generator version, recipe, seed **and inputs** give the same bytes" (`docs/map-package.md` §7).
So the question is only what crosses the boundary.

| What the outside tool hands over | Deterministic inside? | Physically consistent? | Verdict |
|---|---|---|---|
| A finished height map used as the result | Yes | No: learned height maps are not guaranteed to drain; rivers, lakes, roads and `flow_dir` would be computed on a surface the erosion never saw | Reject |
| A height map used as a **target**; the generator inverts it to uplift and then erodes (Schott 2023's inverse step) | Yes, when the inversion is a fixed-iteration CPU loop with exact operations | Yes: the physics has the last word | Good |
| An **uplift** layer (and optional erodibility-scale and precipitation-scale layers) | Yes | Yes | Best; this is P08 §4.2's "sketch → uplift" |
| Curves and points (vector constraints) | Yes | Yes, when applied before or during evolution | Good; small files, reviewable in git |

So **all** methods in §1.1 stay deterministic under that rule: Hnaidi-style diffusion and
Génevaux-style primitives can even run inside (plain numerics); cGAN, StyleGAN and diffusion
models run outside and deliver a layer. The conditions that make it hold:

1. The layer is stored as bytes with a SHA-256, and the hash is part of the run's `inputs`
   (so it reaches the manifest and the package).
2. The generator resamples it to its grid with exact operations only (bilinear weights are
   `+ − × ÷`).
3. The layer lives with the content (git or the binaries folder), like any authored asset. The
   model that made it does not need to be reproducible; the layer is the source of truth.

### 1.3 Recommendations

**A1 · Control layers as a run input.** A recipe may read named control rasters:
`uplift_add`, `uplift_scale`, `erodibility_scale`, `precipitation_scale`, `height_target`,
`height_weight`, `fixed_mask` (which cells are base level), and free masks for blending (§4).
A new first stage `controls` writes them into the world; `tectonics`, `geology` and
`climate_model` take them as optional reads (the contract already supports optional reads,
P08 §5.3).

**A2 · Uplift from a target (`uplift_from_target`).** Implements Schott 2023's inverse step for
the cases where a designer or a model can only give a shape. For the detachment-limited law at
steady state, P08's own physics test 1 states the identity S = (U/K)·A^(−m) for n = 1
(P08 §5.14), so U = K·A^m·S computed on the target is the first estimate; a fixed number of
simulate-and-correct rounds refines it. *The correction loop is my reading of the paper's
project page, which only says "reconstructing the uplift from an input elevation model"; the
paper's exact update rule must be read before the phase spec.* With deposition (G > 0) slopes
steepen (P08 §5.5 note), so the loop must run with the recipe's G.

**A3 · Point and curve constraints.** Two cheap forms that fit the existing sources:
- a **river-course curve** = a line of lowered uplift and raised erodibility (the existing
  `fault`/`rift` sources already do this; add a `path` given by the designer);
- a **height constraint** on a point or curve with a radius, applied each evolution step as a
  pull toward the value (Schott 2023 names point and curve elevation constraints; Hnaidi 2010
  gives the parametrisation: elevation, plateau radius, slope per side).

**A4 · Site preparation stamps (`site_prep`).** A flat platform of radius r at a site, by cut
and fill with a smooth shoulder, after erosion and before roads. This is Génevaux 2015's
*replace* operator and Hnaidi's plateau radius, used locally. It guarantees an arena or castle
ground exists when the terrain does not offer one.

**A5 · Learned tools: later, outside, optional.** A sketch → uplift model trained on the
generator's own snapshots (P08 §5.6 item 7 is the hook) produces control layers for A1. Nothing
in the generator depends on it.

| | A1 control layers | A2 uplift from target | A3 constraints | A4 site prep | A5 learned sketch tool |
|---|---|---|---|---|---|
| 1 Licence | None needed | Schott 2023 code MIT (C++/GPU; may be read) | Papers only | Papers only | Lochner 2023 code announced, licence not checked; StyleGAN2-based work inherits NVIDIA's licence terms (not checked) |
| 2 Bit-identical | Yes (hashed bytes, exact resampling) | Yes on CPU with fixed iteration counts | Yes | Yes | No, and it need not be: only its output layer is an input |
| 3 Stage contract | Yes: `controls` stage + optional reads | Yes: reads `height_target`, `erodibility`; writes `uplift` | Yes: sources in `tectonics`; a per-step process `constrain` | Yes: reads `height`, features; writes `height`, `erosion`, `deposition` | Not a stage |
| 4 Needs from format 1.0 | Names and dtypes for control layers; the input's hash in `manifest.inputs`. No new dtype | Nothing | Nothing | Optionally a `platform` polygon feature | Nothing |
| 5 Size | Small | Medium | Small | Small | Large, separate project |

Protocol side (`docs/generators.md` §3): A1 needs a new input type, for example
`{"type": "layers"}`, whose value is the path and hash of a control package. The simplest
carrier is a `.theosmap` with only control layers, read by `theos-map-format`; that reuses the
frame, the encoding and the per-layer hashes.

Open point: the floor frame (a rim over a central void) probably needs `fixed_mask` rather than
"drained edges on chosen sides" (P08 `grid`). I could not read `03-floor-frame.md` or
`decisions.md` (see "Could not do"), so I leave this as a question for the coordinator.

## 2. Constraint satisfaction over seeds

### 2.1 Methods

| Family | Idea | Strength | Limit | Source |
|---|---|---|---|---|
| Generate and test | Roll, evaluate, reject | Trivial | Cost grows fast when constraints are narrow. Dwarf Fortress does exactly this; its wiki warns that forcing exact amounts by rejection "is inefficient", advises broader settings, and offers a rejection log to find the parameter at fault | Togelius et al. 2011 (IEEE TCIAIG) [hit]; Dwarf Fortress wiki "World rejection" [opened] |
| Search-based PCG | Treat generator parameters as a genotype; optimise an evaluation function | Works with any generator as a black box | Needs many runs; evaluation must be cheap | Togelius, Yannakakis, Stanley & Browne 2011 [hit]; Togelius et al. 2010 (CIG, StarCraft maps, multi-objective) [opened] |
| Automatic tuning of generator parameters | A tool changes parameters, evaluates the outputs and moves the generator toward a wanted output range | Aims at the generator, not one map | — | Cook, Gow & Colton 2016 (CIG; Danesh) [opened: abstract] |
| Quality diversity (MAP-Elites) | Keep the best solution per cell of a grid of behaviour descriptors | Yields a spread of good maps, not one | Descriptors must be chosen | Mouret & Clune 2015 (arXiv 1504.04909) [prior knowledge only]; Gravina et al. 2019 survey [hit]; Alvarez et al. 2019 (constrained, interactive; IEEE CoG) [opened: abstract] |
| Generate then repair | Generate freely, then compute the smallest edit that satisfies playability | Keeps the generator's style and adds a guarantee | Repair model must exist for the domain | Zhang, Fontaine, Hoover, Togelius, Dilkina & Nikolaidis 2020 (AIIDE; MILP repair, minimum-cost edits) [opened: arXiv abstract] |
| Constraint-first, declarative (ASP) | Describe the design space as a logic program; a solver enumerates only valid artefacts; reachability and "shortest path ≥ N" are constraints | Guarantees by construction | Finite, small spaces. The paper reports 250 ms for a 6×6 maze, two hours for one 21×21 maze with a 114-step path, and says real-valued properties are hard to model | Smith & Mateas 2011 (IEEE TCIAIG 3(3)) [opened] |
| WFC as constraint solving; tile constraints with reachability | Tile adjacency is a constraint problem; later work adds global constraints such as paths | Good for tile layouts | Global distance constraints are costly | Karth & Smith 2017 (PCG workshop) [hit]; Cooper 2022 "Sturgeon" (AIIDE) [hit; the AAAI page refused the fetch]; Bucklew, GDC 2019 (Caves of Qud) [hit] |
| Declarative world modelling | Semantic layers (relief, water, roads, towns) kept consistent after each designer edit | A model for "edits propagate" | Research prototype | Smelik, Tutenel, de Kraker & Bidarra 2011 (Computers & Graphics 35) [hit] |
| Placement with playability constraints | Place content by constraint propagation over a fixed level | Fast, guaranteed | Needs a fixed geometry first | Horswill & Foged 2012 (AIIDE) [hit] |

### 2.2 What the reference run says

From P08 Appendix A [repo], for `hearth-valley` seeds 1–8:

- relief 1281–2809 m against a wanted 800–2200: two seeds too high;
- lake share 1.46–32.04 % against 0–8: five seeds too high;
- vegetation cover 37.8–82.4 % against 25–75: two seeds too high.

These are three scalars with a wide spread across seeds, each mostly driven by one or two
parameters (uplift rate against erodibility; deposition and pit handling; precipitation). None
is a combinatorial problem. P08 already attacks lake share at the cause (deposition fills
depressions, P08 §4.2). Rolling more seeds is the weakest tool here: at a pass rate of 2/8, the
chance that 11 seeds contain no pass is still 0.75¹¹ ≈ 4 %, and every roll pays the full
pipeline (about 17 s per seed in 0.3.0, P08 §4.3).

### 2.3 Recommendations

**C1 · Check each constraint at the earliest stage that fixes it.** Relief is final after
`landscape_evolution`, lake share after `hydrology`, vegetation after `ecology`. The runner can
bind each constraint path to the stage whose metric it is (metric names are known before the
run) and stop a seed there. This changes no output; it only saves time in batch runs. A recipe
flag keeps today's behaviour (run to the end and report) for single runs.

**C2 · Calibrate instead of re-rolling (outside the run).** For a scalar target with a
monotone driver, solve for the driver:

| Target | Driver | Why monotone |
|---|---|---|
| relief | a gain on all uplift rates | At steady state with n = 1, slope is proportional to U/K (P08 §5.14 test 1), so relief scales about linearly with the gain |
| vegetation cover | base precipitation | Density rises with actual evapotranspiration (P08 `ecology`) |
| lake share | `g` (deposition) and `min_lake_depth_m` | More deposition fills more depressions (P08 `fluvial`, `hydrology`) |

A batch driver runs a secant or bisection search (three to five runs) per seed and writes the
solved numbers **into the concrete recipe**. The canon run is then an ordinary run of that
recipe; nothing iterative happens inside a run. This is search-based PCG over a tiny genotype
(Togelius et al. 2011) and what Danesh automates (Cook et al. 2016). *The monotonicity claims
are my reading of P08's formulas; a probe on the Rust generator must confirm them.*

**C3 · Partial re-rolls.** A recipe (or a run input) may override the seed of named stages:
`seeds = { settlements = 7, pois = 7 }`. Terrain stays, placement changes. P08 §5.7's stream
derivation makes this a one-line rule. It turns "terrain is fine, the ruins are in the wrong
place" from a full re-roll into a cheap one.

**C4 · Constraint-first placement for what lore demands (`requirements`).** Requirements are
predicates over layers that already exist; the stage enumerates candidates and selects, with a
deterministic order and tie-break by cell index.

| Lore demand (from the task) | Formulation | Kernels (all exist, P08 §5.8) |
|---|---|---|
| "A castle on this hill above a river" | Candidate cells: local height maxima with prominence ≥ p, slope ≤ s inside radius r, a river cell of discharge ≥ q within d metres and at least h metres lower. If the designer pins the place, restrict candidates to a disc around the pin | `maximum`, `distance`, `labels` |
| "A guardian arena at the far end from the entry" | Among cells whose largest flat disc has radius ≥ r (distance transform of the mask slope ≤ s and dry), take the one with the largest walking time from the entry | `distance`, `paths` |
| "Three Chimes spaced an hour's walk apart" | Candidates from a site-quality map; choose 3 with every pairwise walking time in [50, 70] min. One Dijkstra field per shortlisted candidate; then exact enumeration of triples from a shortlist of a few dozen, or greedy farthest-point with swaps for larger counts | `paths` |
| Everything reachable | The road MST connects all places by construction (P08 `roads`); `travel` marks unreachable cells with −1; a requirement fails with the feature named | `paths`, `mst` |

When no candidate exists, the stage reports which predicate emptied the set (the Dwarf Fortress
rejection log is the model) and, if the recipe allows, falls back to **repair**: a `site_prep`
platform (A4), a bridge or a cut trail. That is generate-then-repair (Zhang et al. 2020) with
domain stamps instead of a MILP.

**C5 · Quality diversity across the tower, in the batch driver.** For 100 floors the goal is
a spread, not 100 near-identical optima. Keep an archive keyed by two or three descriptors
(for example relief class, water share class, openness) and store the best-scoring passing
(recipe, seed) per cell (MAP-Elites, Mouret & Clune 2015). Floors pick from the cell their
band's curve targets (§4). The archive is a plain table; no library is needed.

**C6 · Solvers only where the space is a small graph.** ASP or SAT suits room-and-lock graphs
(interiors, P29/P30), where Smith & Mateas show reachability and path-length constraints
directly. It does not suit terrain rasters (their own scale figures, §2.1). A tile solver with
backtracking for the built kinds is Rust code with a seeded stream; an external solver would be
a pinned binary dependency outside `Cargo.lock`.

| | C1 early checks | C2 calibration | C3 partial re-roll | C4 requirements | C5 QD archive | C6 tile / graph solving |
|---|---|---|---|---|---|---|
| 1 Licence | — | — | — | Papers only | Papers only | clingo is MIT (prior knowledge, not checked); mxgmn's WFC is MIT (prior knowledge, not checked); own code preferred |
| 2 Bit-identical | Yes (no output change) | Driver is outside; the solved recipe runs bit-identically | Yes | Yes (total order, ties by index) | Outside the run | Own backtracking: yes. External solver: only with a pinned version, and outside G20's audit |
| 3 Stage contract | Runner feature using `metric_names` | Not a stage (tool layer) | Runner/recipe rule | A stage: reads terrain layers, `reach` fields, `features`; writes `features`; metrics = achieved values | Not a stage | Stages of P29/P30 |
| 4 Needs from format 1.0 | Nothing | Nothing (the manifest already stores the recipe as run) | Nothing | New `poi` types (`chime`, `arena`, …) and a `requirement` property linking a feature to its rule; optional per-feature achieved values | Nothing | Per P29/P30 |
| 5 Size | Small | Small–medium | Small | Medium | Small | Medium–large |

Protocol side: requirements that pin positions are inputs. A new input type, for example
`{"type": "anchors"}`, carries a small GeoJSON in the map's frame (points, discs, curves with a
`role`). The editor can fill it from the map's pins, as it fills `cameras` today
(`docs/generators.md` §3).

## 3. Gameplay metrics for maps

### 3.1 Published metrics

| Metric | Definition | Source | On a 2D raster + road graph? |
|---|---|---|---|
| Base distance (fb1) | Shortest path between key points, normalised by map size (A* over passable cells) | Togelius et al. 2010 [opened] | Yes: `travel` already gives minutes |
| Base space (fb0) | Share of passable, reachable cells around a base; used as a hard constraint | same | Yes |
| Choke points (fp1) | Average narrowest gap along the paths between bases, measured across the path | same | Yes: distance transform of the walkable mask sampled along routes |
| Path overlap (fp2) | Average number of shortest paths through each cell | same | Yes: count over routes; on the road graph this is edge betweenness |
| Resource distance, ownership, safety, fairness (fr1–fr4) | Ratios and spreads of path lengths from bases to resources | same | Yes |
| Area control | How much of the map lies closer to one reference point than to the others | Liapis, Yannakakis & Togelius 2013 (AIIDE 9(1), 30–36) [opened: abstract; formulas from prior knowledge] | Yes: nearest-source label of a multi-source Dijkstra |
| Exploration | Search effort (flood fill) to get from one point to another | same | Yes |
| Balance | Evenness of the two measures across players or points | same | Yes |
| Checkpoint path lengths | Fitness built from shortest-path lengths between designated checkpoints | Ashlock, Lee & McGuinness 2011 (IEEE TCIAIG) [prior knowledge only] | Yes |
| Shortest path ≥ N, reachability | As constraints | Smith & Mateas 2011 [opened] | Yes |
| Region decomposition and choke detection | Regions and choke points from a Voronoi-style skeleton of the walkable area | Perkins 2010 (AIIDE) [hit; prior knowledge] | Yes: medial axis from the distance transform |
| Isovist and visibility graph | Area visible from a point; graph of mutually visible cells and its integration measures | Turner, Doxa, O'Sullivan & Penn 2001 (Environment and Planning B) [prior knowledge only] | Yes, but cost is cells × rays; sample points instead of all cells |
| Viewshed on terrain | Cells visible from an observer over a height field | Standard GIS practice [no source opened] | Yes |
| FPS design patterns | Sniper location, choke point, arena, stronghold, flanking route | Hullett & Whitehead 2010 (FDG) [hit; prior knowledge] | Partly: each pattern can be a detector over sightlines, width and flat area |
| Collision points, tactical choice | Where opposing routes meet; number of route choices | Güttler & Johansson 2003 [hit; prior knowledge] | Yes on the road graph (equal-time meeting points) |
| Fight time, kill balance | From bot matches | Cardamone et al. 2011 (EvoApplications); Lanzi, Loiacono & Stucchi 2014 (CIG) [first: hit; second: prior knowledge only] | **No**: needs a game simulation |
| Expressive range | 2D histograms of metric pairs over many outputs, compared across parameter settings (10,000 levels per plot in the paper) | Smith & Whitehead 2010 (PCGames) [opened] | Yes: it is a property of the batch |

**Extraction-style games.** I searched for published metric sets for extraction shooters and
battle-royale maps and found none I could verify. The rows below are derived from the published
metrics above and from the game's own demands; they are proposals, not citations.

### 3.2 Recommended metric set

| Group | Metric | How |
|---|---|---|
| Routes | Entry → each extraction walking time; spread between extractions | `travel` fields |
| Routes | Number of distinct routes entry → extraction (edge-disjoint paths on the road graph); detour ratio of the second route | Max-flow on a small graph |
| Routes | Path overlap / betweenness of road edges ("where everyone passes") | fp2 on the road graph |
| Chokepoints | Minimum walkable width along each main route; count of gaps narrower than w | Distance transform + route sampling |
| Sightlines | Visible area from each place; longest sightline; share of a route's length exposed to ground more than d away | Line-of-sight over `height` (+ canopy from `vegetation`), from sampled points |
| Cover | Distance to the nearest concealment (dense vegetation, steep break, building) per cell; its distribution along routes | Distance transform |
| Spaces | Largest flat disc per region (arena candidates); open-ground share | Distance transform of a slope mask |
| Fairness | Area control between entries or between extractions | Multi-source Dijkstra labels |
| Tower-wide | Expressive range of all of the above over floors and seeds | Batch tool |

Line-of-sight under G20: step along the ray with integer cell steps and compare slopes as
ratios (`Δh·d₀ > Δh₀·d`), so no trigonometry is needed.

| | Gameplay metric stages (`route_stats`, `chokepoints`, `visibility`, `cover`) |
|---|---|
| 1 Licence | Papers only; no code needed |
| 2 Bit-identical | Yes: integer stepping, exact comparisons, total-order ties |
| 3 Stage contract | Yes: pure stages reading `height`, `vegetation`, `water`, `road_network`, `features`; they write optional layers and report metrics that `[constraints]` can then name |
| 4 Needs from format 1.0 | Optional layers `exposure` (u8), `cover_dist` (f32, m), and `reach_min` if it is not already written; an optional feature kind `chokepoint` (point, `width_m`). All additions |
| 5 Size | Medium (four small stages) |

## 4. Blending across floors

### 4.1 Methods

| Method | What is blended | Source |
|---|---|---|
| Shared continuous parameters, biome as a lookup | Terrain height comes from splines over continuous noise fields (continentalness, erosion, peaks and valleys); the biome is chosen from the same fields plus temperature and humidity. The shape is continuous wherever the fields are | Article based on Henrik Kniberg's talks [opened]; the talk itself not opened. *That this is why borders do not cut the terrain is my inference; the article does not say it* |
| Blending old and new generators along a border | Minecraft 1.18 blends terrain of old chunks into new ones | Minecraft wiki, cited in [repo] decision 0003 §2 |
| Weighted blend of primitives | Elevation averaged by influence weights; commutative and associative, n-ary | Génevaux et al. 2015 [opened] |
| **Blend in the uplift domain, then erode** | "Hydrologically consistent blending between terrain patches" | Schott et al. 2023 [opened: project page] |
| Mask-based style mixing | Feature maps of two generators mixed with a user mask at each resolution | Perche et al. 2023 [opened] (neural; outside the generator) |
| Scattered-point biome blending | Blend weights from jittered sample points, normalised, to avoid grid-shaped borders | KdotJPG, "Scattered-Biome-Blender" [hit only; prior knowledge; licence not checked] |
| Tile sets with compatible edges | Transitions are extra tiles or adjacency rules | Karth & Smith 2017 [hit]; Bucklew GDC 2019 [hit]; Wang tiles, Cohen et al. 2003 [prior knowledge; not searched] |
| Authored content placed by pools | Designer-built pieces chosen from weighted pools | Minecraft jigsaw structures, Deep Rock Galactic, cited in [repo] decision 0003 §2 |

**Banded games.** I looked for talks on how games with themed bands of levels handle the
change between bands and found no primary source I could open. From general knowledge
(unverified): level-based roguelikes change theme at a level load and soften it by mixing
content from both themes near the border. Treat that as an assumption.

### 4.2 The principle

Blend **before** the physics, not after.

- A cross-fade of two finished height maps is not a drained surface: rivers stop at the seam
  and pits appear. Schott 2023 blends uplift and lets erosion make the seam consistent.
- For the simulated kinds (A river landscape, B needle peaks, F floating ground as far as it is
  a height field), everything that defines a type is a field or a number: uplift sources, rock
  mix, climate, process parameters. All of these interpolate.
- For the built kinds (D town, E city, G halls, C labyrinth, H galleries), a half-way value
  between two layouts means nothing. The transition is a **mixture**: weights of module pools,
  tile sets that contain transition pieces, or a zone mask that says which generator owns which
  part of the floor (the child-map mechanism of `docs/generators.md` §3 already puts a town
  inside a region).

Two axes need separate tools:

| Axis | Tool |
|---|---|
| Along the tower (floor 1 → 100) | Parameter curves over the floor number; weights that fade a source, a process or a pool in and out |
| Inside one floor | A mask that mixes two parameter sets' fields (terrain kinds) or assigns zones (built kinds) |

A caution: erosion is sensitive. Two neighbouring floors with the same seed and slightly
different parameters share their large shapes but not every river. Smooth parameters give a
smooth **character** (metrics that drift), not a morph. Since floors are separate worlds, that
is what the requirement asks for; the check is a plot of each metric against the floor number.

### 4.3 What the recipe format needs

| Need | Proposal | Why |
|---|---|---|
| A typed input | `[inputs] floor = { type = "number", min = 1, max = 100 }`; `describe` lists it; the request sends `"inputs": {"floor": 37}` | Decision 0003 §9 foresees inputs that set stage parameters |
| Parameter curves | Any numeric parameter may be a curve over an input: `rate = { over = "floor", keys = [[1, 2.6e-3], [10, 1.8e-3]], ease = "linear" \| "smoothstep" \| "step" }` | Interpolation over the sequence. Linear and smoothstep are polynomials, so exact operations suffice |
| Fading structure | A `weight` (0–1) on every source, process and pool; weight 0 means absent | Lists cannot interpolate; weights can |
| Inheritance | `extends = "band-1-base"` with overrides | One base per band, small files per floor or none at all |
| A second parameter set and a mask | `[[mix]] with = "<recipe id>"`, `mask = { ramp = { from = [x, y], to = [x, y] } }` or a control layer (A1), `amount` as a curve over `floor`; applies to the outputs of `tectonics`, `geology`, `climate_model` before `landscape_evolution` | Two terrain types on one map, consistently eroded |
| Seed control | `seed_group` (floors of a group share terrain streams) and per-stage seed overrides (C3) | "The same land, changed" across a band's border, or a fresh land, by choice |
| Curved targets | `[constraints]` and requirement counts may be curves too | Relief, water share or Chime count that drift over a band |
| Requirements | `[[require]]` entries (C4) | Lore demands |
| Zones for built kinds | A `zones` class layer and per-zone recipe ids (child maps) | Town into city, cavern into labyrinth; belongs to P29/P30 |

The expansion (resolve `extends`, evaluate curves at the given `floor`) is a pure function
that runs before validation. The manifest already carries "the full recipe as it ran"
(P08 §5.10), so a package stays self-describing and the family file is not needed to reproduce
it.

| | Parameter curves, inputs, `extends` | Field mixing (`mix`) | Zones and pool weights for built kinds |
|---|---|---|---|
| 1 Licence | — | Schott 2023 code MIT, as reference | Per P29/P30 |
| 2 Bit-identical | Yes: exact arithmetic on f64 | Yes: a weighted sum of fields | Yes with own code |
| 3 Stage contract | Not a stage; recipe expansion before validation | A stage `mix_fields` (reads two sets of `uplift`, `erodibility`, …; writes one), or the three field stages run twice under namespaced layer names | Stages |
| 4 Needs from format 1.0 | Nothing in the package; `inputs` already exists in the manifest | Nothing | A `zones` layer and legend (addition) |
| 5 Size | Small–medium (schema + expansion + validation messages) | Medium | Large (part of the new map kinds) |

Protocol side (`docs/generators.md` §3): input types `number` and `enum` are additions to
protocol 1's `inputs` map. `describe` must report each input's type and range so the editor can
draw a field for it.

## 5. Producing 100 maps

### 5.1 What comparable productions report

| Production | Practice | Source |
|---|---|---|
| Dwarf Fortress | Generate, test against rejection parameters, regenerate; a rejection log names the cause; the wiki advises against narrow rejection rules | Wiki "World rejection" [opened] |
| No Man's Sky | Automated probes roam generated planets so the team can inspect results and adjust the rules ("regain a small semblance of control") | GamesRadar, 8 Mar 2015 [opened]; GDC 2017 talk cited in [repo] decision 0003 |
| Far Cry 5 | Procedural tools generate biomes, terrain texturing, freshwater networks and cliff rocks for about 100 km²; artists do not touch every square metre | 80.lv summary of Carrier's talk [opened]; GDC 2018 [repo citation]. The summary does not describe baking or review, so I claim nothing about them |
| Ghost Recon Wildlands | Terrain and erosion simulated offline, results fixed ("no magic seeds") | [repo] decision 0003 §2 and its sources |
| Horizon Zero Dawn | World baked into 2D maps; placement at runtime, deterministic and locally stable | [repo] decision 0003 §2 |
| Caves of Qud | WFC from small authored samples for building layouts | Bucklew, GDC 2019 [hit; not opened] |
| Research tools | Expressive range plots; automatic parameter tuning toward a wanted range | Smith & Whitehead 2010 [opened]; Cook et al. 2016 [opened] |
| Houdini PDG | Parameter "wedges" and batch farms for variations | 80.lv, SideFX [hits; not opened] |
| This project | `theos gen run … --seeds …` writes outputs, results and a contact sheet; "change a recipe, run four seeds, look at the contact sheet, read the results" | [repo] decision 0003 §9 |

**Designer effort per level.** None of the sources I could open states hours per generated map
or per level. Anything more specific would be invention.

### 5.2 Recommended pipeline

1. **Author** one family per band: a base recipe, curves over `floor`, the requirements of
   each floor, optional control layers for floors whose lore fixes a shape.
2. **Expand** to 100 concrete recipes (pure function, §4.3).
3. **Calibrate** each floor's drivers on two or three seeds (C2).
4. **Run** N seeds per floor with early checks (C1). The adapter already runs seeds in
   parallel (P08 §5.7, CE-29.4).
5. **Rank** automatically: hard constraints and requirements first; then a score = distance of
   the floor's metrics from its targets, plus a bonus for distance from the picks of the
   neighbouring floors (so adjacent floors differ), using the archive of C5.
6. **Review** three views: a contact sheet per floor (top k), a band sheet (10 floors × top 3),
   and metric-versus-floor plots with the bands' borders marked. The plots are the test of "no
   hard cuts".
7. **Pick** through the editor or `gen_pick` (review entries for agents, decision 0003 §9).
   A pick pins (generator version, expanded recipe, seed, input hashes).
8. **Refine** a picked floor with partial re-rolls (C3), requirement edits and `site_prep`,
   without touching its terrain.

Cost, from repo figures only: 0.3.0 took about 17 s per seed at 384² (P08 §4.3). 100 floors ×
16 seeds = 1,600 runs ≈ 7.6 hours on one thread at that speed. The Rust generator's time is not
measured yet (P08 §5.14 "Budget"), so this is an upper-bound guess, not a forecast.

**My estimate of designer effort (not from a source):** the work moves from floors to bands.
Per band: one base recipe and its curves, tuned over a few batch rounds. Per floor: a list of
requirements, a look at one contact sheet, one pick, and for lore-heavy floors a sketch or a
few anchors. The floors that cost real time are the ones whose lore fixes a shape.

| | Batch driver, ranking, review sheets |
|---|---|
| 1 Licence | No outside code needed |
| 2 Bit-identical | Not required: the driver chooses (recipe, seed); only the chosen run must be reproducible, and it is |
| 3 Stage contract | Not a stage. Tool layer: `theos gen` (P09) or an example in the crate until then |
| 4 Needs from format 1.0 | Nothing. Renders are outside the package (`docs/map-package.md` §8) |
| 5 Size | Small–medium |

## 6. Summary table

| # | Recommendation | Strongest source | Licence | Bit-identical (G20) | Stage contract | Format 1.0 additions | Protocol (`generators.md` §3) | Size |
|---|---|---|---|---|---|---|---|---|
| A1 | Control layers as inputs (uplift, erodibility, climate scale, height target, masks) | Schott 2023; P08 §4.2 | — | Yes, hashed bytes + exact resampling | `controls` stage, optional reads | Control layer names; input hash in manifest | New input type `layers` | Small |
| A2 | `uplift_from_target` (inverse step) | Schott 2023 | MIT reference code | Yes on CPU, fixed iterations | Yes | — | — | Medium |
| A3 | Point and curve constraints, designer river courses | Schott 2023; Hnaidi 2010 | Papers | Yes | Sources + a per-step process | — | Via `anchors` | Small |
| A4 | `site_prep` platforms | Génevaux 2015; Hnaidi 2010 | Papers | Yes | Yes | Optional `platform` feature | — | Small |
| A5 | Learned sketch → uplift tool, outside | Guérin 2017; Lochner 2023; Perche 2023 | Not checked | Not needed (output is an input layer) | Not a stage | — | Uses A1 | Large, later |
| C1 | Constraints checked at the earliest stage | P08 §5.3 | — | Yes | Runner | — | — | Small |
| C2 | Calibration of scalar drivers by the batch driver | Togelius 2011; Cook 2016 | — | Driver outside; result is a plain recipe | Tool layer | — | — | Small–medium |
| C3 | Per-stage seed overrides (partial re-roll) | P08 §5.7 | — | Yes | Runner/recipe | — | Optional input | Small |
| C4 | `requirements`: constraint-first placement on rasters and travel time, with repair | Smith & Mateas 2011; Horswill & Foged 2012; Zhang 2020 | Papers | Yes | Yes | New `poi` types; `requirement` property | New input type `anchors` | Medium |
| C5 | Quality-diversity archive across floors | Mouret & Clune 2015; Gravina 2019 | Papers | Outside the run | Tool layer | — | — | Small |
| C6 | Tile or graph solving for built kinds only | Karth & Smith 2017; Smith & Mateas 2011 | Own code preferred | Yes with own code | Stages of P29/P30 | Per those phases | — | Medium–large |
| M1 | Gameplay metric stages (routes, chokepoints, visibility, cover) | Togelius 2010; Liapis 2013; Turner 2001 | Papers | Yes | Yes | Optional layers `exposure`, `cover_dist`; feature `chokepoint` | — | Medium |
| B1 | Inputs, parameter curves over `floor`, `extends`, weights | Decision 0003 §9 | — | Yes | Recipe expansion | — | Input types `number`, `enum` | Small–medium |
| B2 | Field mixing with a mask before erosion | Schott 2023; Génevaux 2015 | MIT reference | Yes | Yes | — | Mask via A1 | Medium |
| B3 | Zones and pool weights for built kinds | Decision 0003 §2 (pools); Karth & Smith 2017 | — | Yes with own code | Stages | `zones` layer | — | Large |
| P1 | Batch driver, ranking, contact and sequence sheets | Smith & Whitehead 2010; Dwarf Fortress; decision 0003 §9 | — | Not required | Tool layer | — | — | Small–medium |

Suggested order: B1 and C1/C3 first (cheap, and they unblock everything), then C4 and M1 (the
gameplay guarantees), then A1/A3/A4 (shape control), then B2 and A2, then P1 on top. A5, B3 and
C6 follow the new map kinds.

## Could not do

- **Project docs.** The brief asks for `01-capability-baseline.md`, `02-band-requirements.md`,
  `03-floor-frame.md` and `decisions.md` through the Projects tool. This session has no such
  tool. I worked from the brief's band table and the repo files only. Anything in
  `decisions.md` that overrides the above is not reflected; the rim and Bore geometry (A1's
  `fixed_mask`) and the real Chime rules are the likely places.
- **Sources not opened** (blocked, failed or not fetched): Génevaux 2013, Cordonnier 2016 and
  Guérin 2017 on HAL (bot protection) and the Purdue PDF of Guérin 2017 (client error); the AAAI
  page of Cooper's Sturgeon paper (access denied); Gain 2009, Cordonnier 2017 and 2018, Goren
  2014, Guérin 2022, Mouret & Clune 2015, Gravina 2019, Karth & Smith 2017, Perkins 2010,
  Turner 2001, Hullett & Whitehead 2010, Güttler & Johansson 2003, Ashlock 2011, Lanzi 2014,
  Cardamone 2011, Smelik 2011, Horswill & Foged 2012, the GDC talks, KdotJPG's blender,
  TerraFusion, Earthbender. For these I relied on the search record and on prior knowledge, as
  marked per row.
- **Abstract-level only:** Tzathas 2024, Lochner 2023, Cook 2016, Liapis 2013, Alvarez 2019,
  Zhang 2020, and the project pages of Schott 2023 and 2024. Details beyond the abstract
  (Schott's exact inversion update, Liapis's formulas) are from prior knowledge and must be
  read in full before a phase spec relies on them.
- **Licences not checked:** Lochner 2023's code, StyleDEM's code, clingo, mxgmn's WFC,
  KdotJPG's blender. Only Schott 2023's repository (MIT) was opened.
- **No source found** for: metrics specific to extraction shooters; transitions between themed
  bands in level-based games; designer hours per generated level.
- **No probes run.** The monotonicity behind calibration (C2) and the cost of visibility
  metrics need a probe on the Rust generator once it exists.

## Sources

Repo (read-only):

- `docs/specs/phases/P08-map-format-and-terrain-generator.md` §4.2,
  §4.3, §5.3, §5.5–5.10, §5.14, Appendix A
- `docs/decisions/0003-map-generator-in-rust.md` §2, §6, §9
- `docs/generators.md` §3
- `docs/map-package.md` §4–§9

Opened:

- Schott, Paris, Fournier, Guérin, Galin 2023, "Large-scale terrain authoring through
  interactive erosion simulation", ACM TOG 42(5):162.
  <https://h-schott.github.io/publications/uplift/publi_uplift.html>; code (MIT):
  <https://github.com/H-Schott/StreamPowerErosion>
- Schott, Galin, Guérin, Peytavie, Paris 2024, "Terrain Amplification using Multi-Scale
  Erosion", ACM TOG 43(4):145.
  <https://h-schott.github.io/publications/mserosion/publi_mserosion.html>
- Tzathas, Gailleton, Steer, Cordonnier 2024, "Physically-based Analytical Erosion for fast
  Terrain Generation", CGF 43(2). <https://diglib.eg.org/handle/10.1111/cgf15033>
- Hnaidi, Guérin, Akkouche, Peytavie, Galin 2010, "Feature based terrain generation using
  diffusion equation", CGF 29(7). <https://perso.liris.cnrs.fr/egalin/Articles/2010-terrain.pdf>
- Génevaux, Galin, Peytavie, Guérin, Briquet, Grosbellet, Benes 2015, "Terrain Modelling from
  Feature Primitives", CGF 34(6). <https://www.cs.purdue.edu/homes/bbenes/papers/Genevaux15CGF.pdf>
- Perche, Peytavie, Benes, Galin, Guérin 2023, "Authoring Terrains with Spatialised Style",
  Pacific Graphics / CGF. <https://www.cs.purdue.edu/homes/bbenes/papers/Perche23CGF.pdf>
- Lochner, Gain, Perche, Peytavie, Galin, Guérin 2023, "Interactive Authoring of Terrain using
  Diffusion Models", CGF 42. <https://pubs.cs.uct.ac.za/id/eprint/1583/>
- Togelius, Preuss, Beume, Wessing, Hagelbäck, Yannakakis 2010, "Multiobjective Exploration of
  the StarCraft Map Space", IEEE CIG.
  <https://www.um.edu.mt/library/oar/bitstream/123456789/29280/1/Multiobjective_exploration_of_the_starcraft_map_space_2010.pdf>
- Liapis, Yannakakis, Togelius 2013, "Towards a Generic Method of Evaluating Game Levels",
  AIIDE 9(1). <https://ojs.aaai.org/index.php/AIIDE/article/view/12680>
- Smith, Mateas 2011, "Answer Set Programming for Procedural Content Generation: A Design Space
  Approach", IEEE TCIAIG 3(3). <https://course.ccs.neu.edu/cs5150f14/readings/smith_asp4pcg.pdf>
- Smith, Whitehead 2010, "Analyzing the Expressive Range of a Level Generator", PCGames.
  <https://www.pcgworkshop.com/archive/smith2010analyzing.pdf>
- Cook, Gow, Colton 2016, "Towards the Automatic Optimisation of Procedural Content
  Generators", IEEE CIG. <https://research.gold.ac.uk/id/eprint/18950>
- Zhang, Fontaine, Hoover, Togelius, Dilkina, Nikolaidis 2020, "Video Game Level Repair via
  Mixed Integer Linear Programming", AIIDE. <https://arxiv.org/abs/2010.06627>
- Alvarez, Dahlskog, Font, Togelius 2019, "Empowering Quality Diversity in Dungeon Design with
  Interactive Constrained MAP-Elites". <https://arxiv.org/abs/1906.05175>
- Dwarf Fortress wiki, "World rejection". <https://dwarffortresswiki.org/index.php/World_rejection>
- GamesRadar, 8 Mar 2015, "No Man's Sky is so big, Hello Games uses probes to keep track of it
  all". <https://www.gamesradar.com/no-mans-skys-universe-so-big-hello-games-uses-probes-to-keep-track/>
- 80.lv, "Houdini: Procedural World Generation of Far Cry 5".
  <https://80.lv/articles/houdini-procedural-world-generation-of-far-cry-5/>
- "How Minecraft generates Worlds you want to explore" (based on Henrik Kniberg's talks).
  <https://dawnosaur.substack.com/p/how-minecraft-generates-worlds-you>

Search record only (not opened; see marks in the text):

- Génevaux, Galin, Guérin, Peytavie, Benes 2013, "Terrain Generation Using Procedural Models
  Based on Hydrology", ACM TOG. <https://hal.archives-ouvertes.fr/hal-01339224>
- Cordonnier et al. 2016, "Large Scale Terrain Generation from Tectonic Uplift and Fluvial
  Erosion", CGF. <https://hal.archives-ouvertes.fr/hal-01262376>
- Cordonnier et al. 2018, "Sculpting Mountains: Interactive Terrain Modeling Based on
  Subsurface Geology", IEEE TVCG. <https://hal.archives-ouvertes.fr/hal-01517343>
- Guérin et al. 2017, "Interactive Example-Based Terrain Authoring with Conditional Generative
  Adversarial Networks", ACM TOG. <https://hal.archives-ouvertes.fr/hal-01583706v2>
- Guérin et al. 2022, "Gradient Terrain Authoring", CGF. <https://diglib.eg.org/handle/10.1111/cgf14460>
- Goren, Fox, Willett 2014, "Tectonics from fluvial topography using formal linear inversion",
  JGR Earth Surface. <https://zendy.io/title/10.1002/2014jf003079>
- TerraFusion, 2025. <https://arxiv.org/abs/2505.04050>
- Togelius, Yannakakis, Stanley, Browne 2011, "Search-Based Procedural Content Generation: A
  Taxonomy and Survey", IEEE TCIAIG. <https://www.um.edu.mt/library/oar/handle/123456789/29609>
- Gravina, Khalifa, Liapis, Togelius, Yannakakis 2019, "Procedural Content Generation through
  Quality Diversity". <https://aitopics.org/doc/arxivorg:AFCC4B52>
- Cooper 2022, "Sturgeon: Tile-Based Procedural Level Generation via Learned and Designed
  Constraints", AIIDE. <https://ojs.aaai.org/index.php/AIIDE/article/view/21944>
- Karth, Smith 2017, "WaveFunctionCollapse is Constraint Solving in the Wild" (slides found at
  <https://creativecoding.soe.ucsc.edu/courses/cmpm202_w20/slides/W2_Tues_Karth_WaveFunctionCollapse.pdf>)
- Bucklew, GDC 2019, "Tile-Based Map Generation using Wave Function Collapse in Caves of Qud".
  <https://gdcvault.com/play/1025913>
- Perkins 2010, "Terrain Analysis in Real-Time Strategy Games", AIIDE.
  <https://ojs.aaai.org/index.php/AIIDE/article/view/12405>
- Güttler, Johansson 2003, "Spatial principles of level-design in multi-player first-person
  shooters". <https://researchr.org/publication/GuttlerJ03>
- Hullett, Whitehead 2010, "Design patterns in FPS levels", FDG.
  <https://researchr.org/publication/HullettW10>
- Cardamone, Yannakakis, Togelius, Lanzi 2011, "Evolving Interesting Maps for a First Person
  Shooter". <https://rd.springer.com/chapter/10.1007/978-3-642-20525-5_7>
- Smelik, Tutenel, de Kraker, Bidarra 2011, "A declarative approach to procedural modeling of
  virtual worlds", Computers & Graphics 35.
  <https://graphics.tudelft.nl/~rafa/myPapers/bidarra.CAG.2011.pdf>
- Emilien et al. 2015, "WorldBrush", ACM TOG. <https://hal.archives-ouvertes.fr/hal-01147913>
- Horswill, Foged 2012, "Fast Procedural Level Population with Playability Constraints", AIIDE.
  <https://ojs.aaai.org/index.php/AIIDE/article/view/12511>
- Liapis, Yannakakis, Togelius 2013, "Sentient Sketchbook".
  <https://www.um.edu.mt/library/oar/handle/123456789/29607>
- GDC 2018, "Procedural World Generation of Far Cry 5".
  <https://gdcvault.com/play/1025557/Procedural-World-Generation-of-Far>
- KdotJPG, Scattered-Biome-Blender. <https://relatedrepos.com/gh/KdotJPG/Scattered-Biome-Blender>

Cited from prior knowledge without a search record in this session: Gain, Marais & Strasser
2009 (I3D); Cordonnier et al. 2017 (SIGGRAPH); Mouret & Clune 2015 (arXiv 1504.04909); Ashlock,
Lee & McGuinness 2011 (IEEE TCIAIG); Lanzi, Loiacono & Stucchi 2014 (IEEE CIG); Turner, Doxa,
O'Sullivan & Penn 2001 (Environment and Planning B); Cohen et al. 2003 (Wang tiles, SIGGRAPH).
