# P29 · Settlements and child maps — phase spec

Status: proposed · 27 Nov 2026 · repo of the build: `theos-editor`
Source: `docs/specs/00-umbrella.md` §3 P29, decision 0004 (5a) and the P08 spec. Scope and
acceptance criteria come from there; this spec decides how.

## 1. Goal

After P29, `theos-maps` makes settlement maps: a hamlet, a village, a market town or a planned
grid town, with streets, blocks, plots, buildings, landmarks and, where the recipe asks, a wall
with gates. A settlement map runs either from a region's settlement feature, as a child map of the
region package, or on its own over a small synthetic terrain.

What Jann uses it for (27 Nov 2026): planning quests and the feel of the world, and setting up
concept art, mostly to see where players can be and what they can do. The first playable version
covers floors 1–10, so the first towns are early starter towns. Jann's bar: "only if there are no
buildings" would make him wince; he would like the range "from simple settlements to more
medieval towns to more modern city layouts".

So the generator aims at:

- **readable places** that can carry pins (P22): a market square, named landmarks, gates, quarters
  of different character;
- **believable fabric** for concept frames and blockouts (P23): streets that follow the terrain,
  plots that face a street, buildings that stand on their plots in rows;
- **candidates per seed**: several seeds give several towns for the same place, and the parent
  region decides where the town sits and where its roads come in;
- **one pipeline, two street styles** (Jann chose approach B on 27 Nov 2026): organic growth and
  planned tracing feed the same blocks, plots, buildings, wall and gates, so a third style later
  is one more stage.

It also builds the **child-map plumbing** that P29 and P34 share (§5.2): new map kinds, runs that
start from a parent package and one of its features, the child's place in the parent's frame, the
parent's layers in the child's world, tables per kind and golden runs with a parent.

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-78 theos-maps settlements and child maps**
- CE-78.1 Settlement maps run from a built-in settlement recipe, with the renders `base` and
  `plan`, and packages whose layers and features follow `docs/map-package.md` §4.4 and §5
  (buildings, streets, walls, gates, landmarks).
- CE-78.2 A child map runs from a parent package (`from`) and one of its settlement features (an
  input of type `feature`); its frame carries `origin_m` in the parent's frame and `parent` with
  the parent package's hash and the feature id; the same parent, feature, seed and inputs give the
  same child.
- CE-78.3 For seeds 1–8, the built-in settlement recipe passes its constraints at least as often as
  0.3.0's settlement recipe did on the same seeds, recorded as a fixture.
- CE-78.4 The golden hashes of CE-77.1 cover a settlement and a settlement's child map.

Every P29 stage also keeps CE-77.1 and CE-77.2 (golden hashes per version, determinism on the four
platforms of G2) and the Stage contract of CE-28.3.

## 3. Scope

**In P29:**

- the shared child-map plumbing (§5.2), built by whichever of P29 and P34 starts its build first;
- settlement stages for local terrain, the site, main streets, the two street styles, the wall
  and gates, blocks, plots, buildings and landmarks, fields and metrics (§5.5–§5.13);
- the integer-lattice geometry module with `i_overlay` (§5.4);
- the settlement layers, features (format 1.1, §5.14), the renders `base` and `plan` (§5.15);
- three built-in recipes, `village`, `market-town` and `grid-town` (§5.16);
- the CE-78.3 fixture of 0.3.0's per-seed values (§5.17) and the golden runs (§5.18);
- the format 1.1 additions to `docs/map-package.md`, in this spec's PR (§5.20).

**Not in P29:**

| Item | Where |
|---|---|
| Interiors, ruins' child maps, the room graph | P34 (it reuses §5.2) |
| Fortification beyond one wall ring with gates and towers: castles, keeps as wards, moats, bastions, districts | P35 in the research track (decision 0004: "the street core is P29's") |
| City scale, tiers, decks, transit, canals | P42 in the research track |
| Ruin damage on settlements | P37 in the research track |
| A hub settlement and a Chime on a street | P31 (CE-81.4) |
| Stacked levels inside a settlement | P30 |
| The `builtin` adapter and `theos gen` | P09; P29 extends the library API it wraps |
| Recipes in the content project | P23 |
| Settlement maps in the Maps screen and Atlas | P16, P25 |

## 4. Decisions

### 4.1 Decided by Jann (27 Nov 2026)

| Topic | Jann's words | Where |
|---|---|---|
| What the maps are for | "I will plan my quests around them and see how the world could feel like and design concept art around that. Mostly for planning where players can be and what they can do." | §1 |
| First target | "an early starter town on the first 10 floors. As the first version that will be playable will only feature the first 10 floors." | The built-in recipes, §5.16 |
| Bar and range | "I think only if there are no buildings? I have not really high expectations so if we could do somewhat everything from simple settlements to more medival towns to more modern city layouts that would be great" | Two street styles; a run without buildings fails (§5.13) |
| Method | Approach B, "B sounds really good to me." | §4.4 |
| Shared plumbing with P34 | Ruled in the P34 session: the plumbing is shared, specified the same way in both specs; whichever build starts first builds it | §5.2 |

### 4.2 Sources read (27 Nov 2026)

The main sources were opened in full this time; decision 0004 and the research `06c` had seen
only their abstracts.

| Source | What P29 takes |
|---|---|
| Chen, Esch, Wonka, Müller & Zhang 2008, "Interactive procedural street modeling" | The planned style: a tensor field blended from grid, radial, height and boundary basis fields (Gaussian decay); streamlines traced along the major and minor eigenvectors with a separation distance, a lookahead that joins nearby streets, stops at degenerate points and water; major streets before minor ones |
| Vanegas, Kelly, Weber, Halatsch, Aliaga & Müller 2012, "Procedural generation of parcels in urban modeling" | Plots: street-front strips cut into parcels by lines across the strip at a frontage drawn per parcel; recursive splits of the minimum-area oriented box with street access kept; child seeds drawn before recursion. Its straight skeleton is replaced by an inset band (§5.10) |
| Emilien, Bernhardt, Peytavie, Cani & Galin 2012, "Procedural generation of villages on arbitrary terrains" | The organic style: seeds sampled by an interest value (slope, water, closeness to roads, domination), each connected to the network by a least-cost path that reuses existing streets; loops searched in a cone ahead; buildings grown from the frontage and snapped to the plot's sides, which gives terraced rows |
| Galin, Peytavie, Maréchal & Guérin 2010, "Procedural generation of roads"; Galin et al. 2011 | Least-cost routing with a slope limit and bridges, which P08's `roads` router already follows (16 directions); the reuse discount |
| Parish & Müller 2001 | The three snapping rules for street ends (cross, extend to a crossing, extend to a street) |
| Beneš, Wilkie & Křivánek 2014, "Procedural modelling of urban road networks" | An old organic core with planned extensions around it, the shape `market-town`'s outer zone can take |
| Yang, Wang, Vouga & Wonka 2013, "Urban pattern" | Read; not used: its cross-field solve and template warping cost much and do not fit irregular old towns |
| Mas, Martin & Patow 2020 (fortified cities) | Abstract only; no source read generates walls, so the wall is own design (§5.8) |

The texts are cited by their published versions; no code of them is used (Chen and Vanegas have no
public code; ProbableTrain's MapGenerator and Watabou's generators are GPL or LGPL and were not
read, as the research advised).

### 4.3 Probes (27 Nov 2026, rustc 1.99.1, Windows 11)

| Probe | Finding | Consequence |
|---|---|---|
| 0.3.0's settlements, seeds 1–8 (Python 3.12, fastscapelib 0.3.0, numpy 2.5.4, scipy 1.18.1 through uv, from `D:\Projects\theos_maps-export`) | Runs, deterministic across processes. The settlement recipe has no constraints. On fixture-small seed 1: town-1 203–258 buildings, 2.3–4.2 s; village-1 68–99, under 1 s. On hearth-valley seed 1: city-1 621–683 buildings, 10–14 s. Defects: a lake-shore village (hearth-valley `village-4`) gets **no streets and no buildings on 5 of 8 seeds**, silently; enlarged landmarks land on water, streets and other buildings; the city wall (a convex hull) crosses rivers without a gate on 7 of 8 seeds; gates do not match the road entries; the mill is missing when the window has no river | CE-78.3 is measured with P29's constraint set applied to 0.3.0's recorded outputs (§5.17); each defect is a check or a constraint here |
| Geometry crates | `i_overlay` 9.2.0 (MIT OR Apache-2.0, deps only its own `i_*` crates and `libm`, threads only behind a feature that stays off) works on an i32 lattice: booleans, offsets in integer math, slicing by lines and paths. 400 random polygons through union, difference, intersection, inset, outset, slicing: the same SHA-256 in debug and release, under every solver strategy and with `-C target-cpu=native`. `geo` brings an old `i_overlay`, `num-traits` and std `hypot`; `clipper2-rust` uses std trigonometry; `geo-straight-skeleton` 0.1.1 works on the five test polygons but is pure f64 with std `hypot`, `sin_cos`, `atan2`; the other skeleton crates are GPL | `i_overlay` without default features for booleans, offsets, faces and splits (§5.4); own minimum-area box (about 110 lines, integer); no straight skeleton (§5.10) |
| Street tracer (about 400 lines after Chen 2008: four basis fields with `libm` decay, fixed-step RK4 at 2 m, separation and lookahead, lattice snapping, noding and faces in i64/i128) | 1.6 km at 4 m: 0.16 s in release. The same SHA-256 over two runs in debug, release and release with `-C target-cpu=native`. Reads as a town: streets follow contours around a hill, rings form around the radial centre, streets run along the river bank; blocks of median 1,419 m². Perturbing a field parameter by 1e-12 or 1e-9 changed nothing, by 1e-6 about 9 % of the edges: once a keep-or-stop decision flips, the change spreads, so a small edit is not a small change. Defects: every minor street meeting the river steeply bridged it; lookahead left doglegs and stubs; degenerate points (the hilltop, the radial centre) gave crowded slivers; the raw graph had 70,000 edges of 2 m; naive rounding of crossings can bend segments | Planned style as in §5.7 with a bridge spacing rule, polylines simplified before noding, snap rounding for the crossings, and no seeds near degenerate points; the golden runs carry `grid-town` (§5.18) |

### 4.4 Approaches considered

| Question | Chosen | Not chosen, and why |
|---|---|---|
| Method (Jann) | B: one pipeline, two street styles (organic growth after Emilien 2012; planned tracing after Chen 2008) feeding shared blocks, plots, buildings, wall and gates | A, organic only: no grid towns until P35/P42. C, a tensor field for everything with noise for an organic look: Chen's field ignores slope and gives two near-orthogonal families, wrong for old towns |
| Port or state of the art | State of the art (decision 0004, 5a); 0.3.0's method only where no better source exists: the local terrain crop and the fields around the town (§5.5, §5.12) | A port keeps the defects of §4.3 |
| Main streets | P08's `roads` router (16 directions, slope cost, bridges, merge discount) on the child's grid | A second router: the same method twice |
| Geometry numbers | i32 lattice of 1/16 m, i64 and i128 predicates; floats only inside the tensor field and the terrain | f64 geometry: polygon booleans and face extraction break on near-degenerate input, and results differ by operation order |
| Polygon operations | `i_overlay` | `geo`, `clipper2-rust`: std float functions; own booleans and noding: 350–450 lines with the hard part (snapping) still to solve |
| Plot strips | Inset band of the block, cut at corners by bisectors | Straight skeleton: no allowed crate is deterministic; a patched fork of a one-month-old crate is a maintenance cost |
| Streets in one value | A `street_graph` world value that every street stage reads and extends | Streets only as a raster: blocks and plots need exact centrelines |
| Several styles in one town | Zones: a recipe gives each street stage a zone (a disc or ring around the centre, or the whole frame) | One style per recipe: no old core with planned extensions (Beneš 2014) |

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Tool or crate | Why |
|---|---|---|
| Stages, recipes, runner, world, package, golden files | P08's (`theos-maps`, `theos-map-format`) | One contract (CE-28.3) |
| Parent package reading | `theos_map_format::read` | It checks hashes and `requires` |
| Resampling parent layers | own `kernels::resample`, Catmull-Rom from P08's `kernels::curves` | About 60 lines; no crate keeps to the float ban |
| Polygon booleans, offsets, faces, slices | `i_overlay` 9.2 without default features, `Solver` fixed (precision high, no threads), `FillRule::NonZero`, integer offset math | Probe §4.3 |
| Minimum-area box, convex hull | own, integer (monotone chain, rotating calipers with i128 comparisons) | About 110 lines; no usable crate (§4.3) |
| Main-street routing | P08's `roads` `Router` and `StepCost` | Galin 2010's method, already built |
| Organic growth | own, after Emilien 2012 | No crate; about 400 lines |
| Tensor field and streamlines | own, after Chen 2008, with `libm` | No crate; about 500 lines (the probe's tracer was 400) |
| Noding of the street graph | own snap rounding on the lattice (Hobby 1999) | `i_overlay` keeps no graph; about 250 lines |
| Wall ring | own: least-cost closed path around the centre on P08's `kernels::paths` | No source; §5.8 |
| Fields | 0.3.0's method on P08's `kernels::voronoi` | No better source read; §5.12 |
| Drawing | P08's `tiny-skia`, `skrifa`, fonts | Same renderer |
| Names of landmarks | P08's `features::names` (generated, not canon) | Same rule |

### 5.2 Shared child-map plumbing (identical in the P29 and P34 specs)

P29 (settlements) and P34 (interiors) both run maps of a new kind from a parent package. Jann
ruled (27 Nov 2026) that this plumbing is shared: whichever build starts first builds it, and the
other merges `main` and reuses it. This section is word for word the same in both specs; a change
to it is made in both, in agreement between the two phases.

1. **Kinds.** `MapKind` gets `Settlement` and `Interior` next to `Terrain`; a recipe's `kind` takes
   `settlement` and `interior`. A stage lists the kinds it applies to, as in P08.
2. **Inputs.** The recipe's `[inputs]` declares inputs in the form of `docs/generators.md` §3, and
   `describe` reports them. Planning accepts declared inputs and refuses undeclared ones and wrong
   types before the run. A child kind's recipe declares exactly one input of type `feature`, with
   `parent_types`, the parent feature types it accepts (`["city", "town", "village", "hamlet"]`
   for settlements, `["ruin"]` for interiors). A feature of another type, or an id the parent does
   not have, is refused before the run.
3. **`from`.** `RunRequest.from` is the path of the parent package (`.theosmap`). The run reads it
   with `theos_map_format::read` (hashes and `requires` are checked) and takes the SHA-256 of the
   file's bytes as `frame.parent.package_sha256`. A recipe that declares an input of type
   `feature` is a child recipe: without `from` it is refused before the run, and a recipe without
   such an input given `from` is refused too.
4. **Child frame.** One function per kind, `child_frame(kind, recipe, parent, feature) ->
   ChildFrame { origin_m, rows, cols, cell_m }`, places the child in the parent's frame:
   axis-aligned (`docs/map-package.md` §2), centred on the feature's point (a Polygon's
   bounding-box centre), with `origin_m` snapped to whole child cells. The size comes from the
   kind's rule: a settlement's from its class table (P29), an interior's from the recipe's
   `size_km` and `grid`, which stay required for interiors (P34). The window may reach past the
   parent's edge.
5. **Parent layers in the world.** Before the first stage the runner puts into the world only the
   parent values that some stage of the plan `reads`:
   - `parent.<layer>`: a parent package layer resampled into the child frame by
     `kernels::resample` (bicubic, Catmull-Rom, every sample point clamped to the parent's grid,
     so cells past the parent's edge read the nearest edge cell);
   - `parent.features`: the parent's features that touch the window, in the child's frame;
   - `parent.feature`: the chosen feature (id, kind, type, geometry, properties, in the child's
     frame) with the parent's frame (`size_m`, `cell_m`, `grid`) and `package_sha256`.
   Planning treats them like layers written before the first stage, so a stage reading a parent
   value the kind does not offer is refused before the run. The memory estimate counts them.
6. **Seed and repeatability.** The child's seed is the request's seed (`docs/generators.md` §3);
   the stage RNG streams are P08's. The parent's package hash and the feature id are inputs of
   the run, so the same parent, feature, seed and inputs give the same package bytes.
7. **Package and manifest.** The package writes `kind`, `frame.origin_m` and
   `frame.parent {package_sha256, feature}`; the run's manifest `frame` adds `origin_m`. The
   package layer table, the standard renders, the feature types (the constraint paths
   `features.<type>`) and the `needs` check are looked up per kind: one table per kind, chosen by
   `MapKind`, with terrain's unchanged. `base` is standard for every kind and lies on the child's
   frame. Settlement: P29 §5.14–§5.15; interior: P34 §….
8. **Features without geometry.** `Geometry` gets `None`, written as GeoJSON `"geometry": null`
   (`docs/map-package.md` §5, `connection`).
9. **Golden runs.** A golden run is `{recipe, seed, inputs, from: Option<(recipe, seed)>}`; the
   parent is generated in the same test from its own golden run, so no package fixture is kept.
   Both phases use `fixture-small` seed 1 as the parent (it places one ruin and its settlements).
   Each phase adds its runs under a new `VERSION`: the first to merge raises it to 1.1.0 and
   records `1.1.0.toml`; the second merges `main`, raises it to 1.2.0 and records `1.2.0.toml`.
10. **Integer lattice.** Geometry that later code compares or splits is snapped to a lattice of
    1/16 m and compared with i64 predicates, in `kernels::geometry`. Whoever needs it first
    builds it.
11. **Format 1.1.** Format 1.1 holds the settlement and interior additions;
    `theos-map-format` writes 1.1 from the first build that adds them.

### 5.3 The settlement pipeline

A settlement recipe is a P08 recipe with `kind = "settlement"`. Its stages, in the order of a
built-in recipe; a recipe switches stages on and off and gives each its parameters:

| Stage | Reads → writes | Method |
|---|---|---|
| `local_terrain` (child) | `parent.height`, `parent.water`, `parent.features`, `parent.feature` → `height`, `water`, `entries`, `site` | §5.5 |
| `site_terrain` (standalone) | – → `height`, `water`, `entries`, `site` | §5.5 |
| `site_plan` | `height`, `water`, `entries`, `site` → `centre`, `interest`, `zones` | §5.6 |
| `main_streets` | `height`, `water`, `entries`, `centre` → `street_graph`, `streets` | §5.6 |
| `wall` | `height`, `water`, `street_graph`, `centre`, `zones` → `wall`, `street_graph` | §5.8 |
| `organic_streets` | `height`, `water`, `interest`, `zones`, `street_graph`, [`wall`] → `street_graph`, `streets` | §5.7 |
| `planned_streets` | `height`, `water`, `zones`, `street_graph`, [`wall`] → `street_graph`, `streets` | §5.7 |
| `blocks` | `street_graph`, `water`, `zones`, [`wall`] → `blocks` | §5.9 |
| `parcels` | `blocks`, `street_graph`, `height` → `parcels` | §5.10 |
| `buildings` | `parcels`, `street_graph`, `height`, `water`, `centre`, [`wall`] → `buildings`, `features` | §5.11 |
| `fields` | `height`, `water`, `buildings`, `street_graph`, `centre` → `fields` | §5.12 |
| `settlement_metrics` | everything above → summary metrics | §5.13 |

- `street_graph`, `blocks`, `parcels`, `wall`, `entries`, `site`, `centre` and `zones` are world
  values (P08's `WorldValue`, hashed as canonical JSON), all geometry on the lattice of §5.4.
- `streets` (u8, street class), `buildings` (u8 mask), `fields` (u8 land use), `height` (f64) and
  `water` (u8, terrain's classes) are rasters and the package layers of `docs/map-package.md`
  §4.4. `interest` stays in the world only.
- The features (§5.14) are built from the values after the pipeline, like P08's roads.
- Each stage fails with a message naming what is missing (no entry reaches the centre, no block
  is left, no building fits) instead of returning an empty town (0.3.0's `village-4`, §4.3).

### 5.4 Geometry on a lattice

- **Lattice.** Positions are `i32` in units of 1/16 m from the child frame's north-west corner,
  x east, y south. A 2.4 km city window spans 38,400 units; i32 leaves room for windows up to
  134 km. Conversion to metres is exact (n / 16), so features carry exact f64 values.
- **Predicates.** Orientation and intersection tests in i64, area comparisons and box areas in
  i128. No float enters a geometric decision.
- **Module** `kernels::geometry` (shared item 10): `Point`, `Ring`, `Polygon` (outer plus holes,
  counter-clockwise outer), `snap(f64 m) -> i32` (round half away from zero through
  `kernels::num`), the convex hull, the minimum-area box, the bisector cut of §5.10, and a thin
  wrapper over `i_overlay` that fixes the solver, fill rule and offset mode in one place.
- **Floats.** The terrain, the interest field and the tensor field are f64 under P08's rules
  (`libm`, no std float methods, no `mul_add`); every street vertex is snapped to the lattice the
  moment it is created, so all later geometry is integer.

### 5.5 Local terrain

**Child (`local_terrain`).** The window is the child frame of §5.2 item 4. The settlement rule:

| Class | Window | Cell | Built radius `r_built` |
|---|---|---|---|
| hamlet | 600 m | 2 m | 70 m |
| village | 1,000 m | 3 m | 170 m |
| town | 1,600 m | 4 m | 330 m |
| city | 2,400 m | 5 m | 560 m |

These are 0.3.0's values; the recipe's `[child.class.<class>]` tables override them.

- `height`: `parent.height` resampled (§5.2 item 5) plus fine relief: P08's spectral noise with an
  amplitude (default 1.2 m, 0.3.0's) and a correlation length set per recipe.
- `water`: rivers drawn from the parent's `water`/`river` LineStrings clipped to the window, each
  at its `width_m` per vertex (not rebuilt from discharge, as 0.3.0 did); lakes from
  `parent.water` class 3, outlined by marching squares (P08's `kernels::contour`) and smoothed
  without random noise, so a lake shore cannot change by seed and cut a town off (§4.3).
- `entries`: every parent `road` LineString that crosses the window's edge gives one entry: the
  crossing point on the lattice, the road's class and id, and the direction inwards. Roads that
  end inside the window (at the settlement itself) are followed to the window's edge the other
  way.
- `site`: the parent feature's point in the child frame, its class, name and population.

**Standalone (`site_terrain`).** For fixtures and for a town without a region: `height` from a
tilted plane plus P08's spectral noise; an optional river as a polyline of frame fractions with a
width; entries as a list of edge points with classes; a site class. All parameters come from the
recipe.

### 5.6 Site plan and main streets

**`site_plan`:**

- `interest` (after Emilien 2012, own weights): flatness (a bell on slope with a hard limit,
  default 0.3), dryness (water cells are −1, so excluded), closeness to water (decreasing with
  distance, for wells and mills), closeness to the entries' least-cost paths, and closeness to the
  parent's site point.
- `centre`: the market square's position, the highest interest within `0.35 · r_built` of the
  site point among cells that a least-cost path reaches from at least one entry. When no cell is
  reachable, the stage fails naming the site (0.3.0 built nothing silently).
- `zones`: discs and rings around the centre that the recipe names (`core`, `outer`, …), each
  with a radius in m or as a share of `r_built`. Street stages, the wall and the fields read them.

**`main_streets`:** one street of class `main-street` from each entry to the centre, routed by
P08's `Router` over a `StepCost` built from the child's `height` and `water` (the vegetation term
is zero; bridges as in P08, at most `bridge_max_m`). Each route after the first reuses earlier
ones at the merge discount, so main streets join before the square, as old roads do. The square
is a polygon around the centre (radius per recipe; 14 m for a town and 24 m for a city in 0.3.0)
and becomes the `plaza` parcel. Every entry reaches the centre, or the stage fails naming the
entry.

`street_graph`: nodes (lattice points, with an id) and edges (two node ids, the polyline between
them on the lattice, class `main-street`, `street` or `lane`, width in m, a bridge flag, the
stage that laid it). Edges are split at every crossing (noded), so the graph is planar. The
`streets` raster marks each cell a street covers with its class.

### 5.7 The two street styles

Each style fills the zones the recipe gives it and joins the graph that is already there.

**`organic_streets`** (after Emilien 2012):

1. Draw candidate points in the zone: cells sampled in a fixed order from the stage's RNG, kept
   with a probability that grows with `interest`, at least `spacing_m` from existing streets.
   The attempt budget is fixed (`attempts`), so the cost is bounded (Emilien's rejection sampling
   was not).
2. Connect each kept point to the nearest street by the least-cost path of §5.6, with the reuse
   discount; its class is `street` while the town's street length is below `street_km`, then
   `lane`.
3. Close loops: from a new street's end, look in a cone ahead (`loop_angle`, `loop_reach_m`) for a
   street node that is not on its own access path, and connect it with probability `loop_p`
   (0.3.0's per class: hamlet 0, village 0.4, town 0.75, city 0.9).
4. Snap ends after Parish & Müller 2001: an end that crosses a street makes a crossing; an end
   within `snap_m` of a node or street extends to it.
5. Never cross the wall except at a gate (§5.8); never cross a lake; cross a river only as a bridge.

**`planned_streets`** (after Chen 2008):

- The tensor field on the grid's vertices is a Gaussian-decay blend of basis fields from the
  recipe: `grid` (an angle, or `align = "main-street"`: the direction of the nearest main
  street), `radial` (a centre), `height` (along the contours) and `boundary` (along water shores
  and the wall). Values between vertices are bilinear.
- Streamlines follow the major eigenvector, then the minor one, with a **fixed-step** fourth-order
  Runge–Kutta integrator (step `step_m`; Chen's adaptive Cash–Karp step is not used, since its
  step choice amplifies float differences). The eigenvector's sign is kept continuous with the
  previous step.
- A line stops at the zone's edge, a degenerate point (tensor norm below `eps`), a lake, a river
  (unless the crossing is shorter than `bridge_max_m`), its maximum length, or closer than the
  separation `d_sep` to another street of the same family. It then looks ahead `lookahead_m` to
  join a street, as Chen does.
- Seeds come from a priority queue (closeness to the centre, then distance from degenerate points
  and water), ties broken by lattice position. Major lines get `d_sep_major`, minor lines
  `d_sep_minor`; the first major lines start from the main streets, so the grid hangs off them.
- Major lines are class `street`, minor ones `lane`.
- Every vertex is snapped to the lattice as it is made (§5.4).
- **From the probe (§4.3):**
  - a bridge needs `bridge_spacing_m` (default 120 m) to every other bridge along the river,
    otherwise the line stops at the bank;
  - no seed lies within `degenerate_clear_m` (default `d_sep_major`) of a degenerate point, and a
    line entering that disc stops, so hilltops and radial centres are left to the square or to
    organic streets;
  - a finished line is simplified (Douglas–Peucker at `simplify_m`, default 0.5 m, on the
    lattice) before it joins the graph;
  - a stub shorter than `stub_min_m` after snapping is removed.

**Noding** (both styles): a new edge is split where it crosses the graph. Crossings are rounded by
snap rounding on the lattice (Hobby 1999: every segment passing through a lattice cell that holds
a crossing or an end point is routed through that cell's centre), so rounding never creates a new
crossing. Own code, about 250 lines, since `i_overlay` slices polygons but does not keep a graph;
collinear overlaps merge into one edge, keeping the higher class.

### 5.8 Wall and gates

No source read generates walls; this is own design. 0.3.0's convex hull is not used: it ignores
terrain and water and crossed rivers without a gate on 7 of 8 seeds (§4.3).

- **Ring.** The wall is the least-cost closed path around the centre inside the recipe's wall zone
  (a ring between `inner_m` and `outer_m`). It is found on P08's `kernels::paths`: the ring is cut
  by a seam from the centre to the outer edge; the shortest path from each cell on one side of the
  seam to its neighbour on the other side is computed, and the cheapest closes the loop.
- **Cost.** Lower on high ground (Emilien's domination term) and on gentle slope, a constant per
  step, lakes impassable, rivers at a high cost.
- **Gates.** Every main street crosses the wall at a gate of type `gate`, which the wall's cost
  makes cheap to place square to the street. A river the wall crosses gets a gate of type
  `water-gate`. No other street crosses the wall; organic and planned streets inside and outside
  join through gates. Towers stand at the gates and at a spacing (`tower_spacing_m`), as points
  on the wall line.
- **Geometry.** The wall is a ring on the lattice with a thickness (`thickness_m`), written as a
  polygon (the band between the ring offset outwards and inwards by half the thickness).
- **Checks** (stage `checks`, errors): the ring is closed and simple, encloses the centre, crosses
  no lake cell, crosses a river only at a `water-gate`, and every main street crosses it exactly
  once, at a gate.

### 5.9 Blocks

- The zone the town fills (the union of the street stages' zones) is sliced by every street
  centreline, the wall ring and the river and lake outlines (`i_overlay` `slice_by_paths`, probe
  §4.3). Each face is a block; it is inset by half the width of the street on each side
  (`i_overlay` offset; a face side on the wall is inset by half the wall's thickness).
- A block below `min_block_m2` or with no street frontage becomes open ground; one that contains
  water is cut by the water outline.
- Each block keeps, per side, the street edge it faces (or none), for the plots.

### 5.10 Plots

After Vanegas et al. 2012, with an inset band instead of the straight skeleton:

1. **Street-front strips.** The band is the block minus the block inset by the plot depth
   (`depth_m`, drawn per block from a range). The band is cut at each convex corner along the
   corner's bisector, so each piece faces one street. At a corner the wider street keeps the
   piece (Vanegas' "street width" rule).
2. **Parcels.** Each strip is cut by lines across it, perpendicular to its street, at frontages
   drawn from a normal distribution around the middle of `[frontage_min_m, frontage_max_m]`
   (Vanegas' choice), clipped to the range. Remnants below the minimum join their neighbour along
   the longest shared edge.
3. **The inner rest** of a deep block (the inset core) is split by the oriented-box recursion:
   the minimum-area box's long side is cut through its middle, shifted by a random share
   `irregularity` of the short side; a piece that would lose street access is cut the other way
   (Vanegas' ω = 1, so access is kept); recursion stops when area and frontage are within range.
   Pieces without access become yards or gardens of the plots around them.
4. **Randomness** per parcel comes from a child RNG whose seed is drawn before the recursion, as
   Vanegas does, so a change in one block does not shift the others.

Parameters are per style: organic plots are narrow and deep (burgage plots: frontage 5–9 m, depth
25–45 m by default); planned plots are wider and shallower (10–18 m, 15–25 m).

### 5.11 Buildings and landmarks

- **Footprints** (after Emilien 2012): on each house parcel, a rectangle starts at the frontage,
  aligned with the street, and grows into the parcel up to `building_depth_m`; a side within
  `snap_m` of the parcel's side snaps to it, so neighbours form rows. The footprint is the
  rectangle intersected with the parcel (`i_overlay`), so no building leaves its parcel, touches
  a street or overlaps another (the 0.3.0 defects of §4.3).
- A footprint on water or on slope above `max_slope` is dropped (the parcel stays a yard).
- **Use and storeys:** a density field falls off with distance from the centre and the main
  streets; storeys are 1 to `storeys_max` by density (0.3.0 stopped at 3), `height_m` is
  storeys × `storey_m` plus a roof allowance. `use` is `house` unless a landmark takes it.
- **Landmarks** come from the recipe's list, each with a placement rule:

| Landmark | Rule |
|---|---|
| `market` | the square (§5.6) |
| `well` | in or beside the square, or the open block nearest the centre |
| `shrine`, `temple` | the parcel of highest domination within `0.8 · r_built` |
| `keep` | the parcel of highest domination inside the wall |
| `inn` | a main-street parcel nearest an entry or gate |
| `smithy`, `guardhouse`, `granary`, `archive` | a main-street parcel, in that priority, nearest the square |
| `mill` | the parcel nearest a river cell whose parent discharge is at least `mill_q` |
| `harbour` | a parcel on a lake shore |

  A landmark needs a bigger plot: it merges its parcel with neighbours along the same street until
  the area reaches its minimum (`landmark_area_m2`), instead of scaling a house (0.3.0's 1.2–1.6×,
  which put buildings on water and streets). A landmark whose rule finds no place (a mill without
  a river) is reported as `landmarks_absent` with its reason and fails a constraint only when the
  recipe marks it required.
- **Names** of landmarks and of the town on the `plan` render are generated (P08's
  `features::names`), never canon; the parent feature's name is kept for the town.

### 5.12 Fields

Outside the built area: 0.3.0's method, ported because no source on field systems was read and it
looked right in 0.3.0. Voronoi cells (`parcel_m`, default 55 m) out to `reach_m` (700 m), typed
`field`, `orchard` or `pasture` by slope and distance from the town, on P08's `kernels::voronoi`.
Written as the `fields` layer (legend: 0 none, 1 field, 2 orchard, 3 pasture, 4 garden, 5 yard,
6 open ground, 7 plaza).

### 5.13 Metrics and constraints

`settlement_metrics` reports under `summary.stages.settlement_metrics`:

| Metric | Meaning |
|---|---|
| `buildings`, `houses`, `landmarks` | counts |
| `population_est` | houses × `persons_per_house` (5) |
| `street_km_main`, `street_km_street`, `street_km_lane` | lengths by class |
| `street_components` | connected parts of the `streets` raster, 8-connected (1 when every street joins) |
| `entries_connected` | share of entries joined to the centre through the `streets` raster |
| `street_access_pct` | share of buildings with a street cell within `access_m` (default 8 m) of a footprint cell |
| `buildings_on_water` | buildings with a footprint cell on a `water` cell other than 0 |
| `buildings_on_street` | buildings with a footprint cell on a street cell |
| `building_overlap_cells` | cells covered by more than one footprint |
| `buildings_steep` | buildings whose footprint's mean slope exceeds `max_slope` |
| `landmarks_missing_required` | required landmarks without a place |
| `gates`, `water_gates`, `towers`, `wall_lake_cells` | the wall's counts, 0 without a wall |
| `built_area_ha` | area of the building footprints |
| `grid_index` | the street-orientation order of the research (`05-probes.md`: the mean resultant length of 4θ over 40 m chords; 0 organic, 1 grid) |

**One definition for both generators.** The defect metrics are measured on rasters of the map's
own grid, so they mean the same on 0.3.0's output (CE-78.3): a footprint covers the cells whose
centres lie inside its polygon (0.3.0's buildings are drawn the same way from its rectangles);
streets are the `streets` raster (0.3.0's street mask); slope is P08's `kernels::diff` slope of
`height`.

A run with no building fails before the constraints (Jann's bar, §4.1).

### 5.14 Features (format 1.1)

Built after the pipeline from the values, ids `<type>-<n>` as in P08:

| Kind | Type | Geometry | Properties |
|---|---|---|---|
| `street` | `main-street`, `street`, `lane` | LineString | `from`, `to` (node ids, opaque strings `node-<n>`), `width_m`, `length_m`, `bridge` (boolean), `entry` (the parent road id, main streets only) |
| `parcel` | `house`, `landmark`, `yard`, `garden`, `plaza`, `open` | Polygon | `street` (id of the street it faces, absent for none), `area_m2` |
| `building` | `building` | Polygon | `use` (`house` or the landmark type), `storeys`, `height_m`, `parcel` (id) |
| `landmark` | `market`, `well`, `shrine`, `temple`, `keep`, `inn`, `smithy`, `guardhouse`, `granary`, `archive`, `mill`, `harbour` | Point | `name` (generated), `building` or `parcel` (id) |
| `wall` | `wall` | Polygon | `height_m`, `thickness_m` |
| `wall` | `tower` | Point | `wall` (id) |
| `gate` | `gate`, `water-gate` | Point | `wall` (id), `street` (id, `gate` only) |

`block` is not a feature: blocks follow from the streets and parcels carry their street. The
settlement feature of the parent is not repeated in the child; the frame's `parent` names it.

### 5.15 Renders

| Render | Content |
|---|---|
| `base` | Frame-exact, 2 pixels per cell: hillshade of `height`, water, fields by land use, street surfaces by width, building roofs as fills, the wall. No symbols, no text |
| `plan` | `base` plus building and parcel outlines, streets by class, the wall with gates and towers, landmarks as symbols with generated names, the town's name, a legend, a scale bar and a title (recipe, seed, buildings, population). It may have margins |

`base` and `plan` are the settlement kind's standard renders (§5.2 item 7); terrain's renders do
not run for settlements.

### 5.16 Built-in recipes

| Recipe | Feature types (child) | Shape |
|---|---|---|
| `village` | `village`, `hamlet` | Organic streets only, no wall; landmarks `well`, `shrine`, `smithy`, `inn`, `mill` |
| `market-town` | `town`, `city` | Organic core inside a wall with gates and towers; organic streets outside the gates; landmarks `market`, `well`, `shrine`, `inn`, `smithy`, `guardhouse`, `granary`, `mill`, of which `market`, `shrine`, `inn`, `smithy`, `guardhouse` and `granary` are required (0.3.0's town set; `well` and `mill` depend on water); the CE-78.3 recipe |
| `grid-town` | `town`, `city` | Main streets from the entries, planned streets aligned with them around an organic market core; no wall |

Each recipe declares its `feature` input and runs as a child, or runs standalone through
`site_terrain` when given no `from`: one recipe file per shape, with `site_terrain`'s table used
only when there is no parent (§5.19 ruling 4). Constraints of each recipe include those of §5.17.

### 5.17 CE-78.3: the 0.3.0 reference

0.3.0's settlement recipe has no constraints (§4.3), so "passes its constraints" is measured with
`market-town`'s constraint set applied to both generators' outputs:

| Constraint | Range |
|---|---|
| `buildings` | [100, 5000] |
| `street_access_pct` | [100, 100] |
| `street_components` | [1, 1] |
| `entries_connected` | [1, 1] |
| `buildings_on_water`, `buildings_on_street`, `building_overlap_cells`, `buildings_steep` | [0, 0] each |
| `landmarks_missing_required` | [0, 0] |

- **The reference:** 0.3.0 run as a child of its own `fixture-small` seed 1, feature `town-1`,
  with child seeds 1–8 set directly (0.3.0's crc32 child seed is skipped). The P29 build runs it
  with the probe's driver (`research/P29-settlements/settle_baseline.py`), changed to the
  definitions of §5.13, and records, per seed, every metric of the table above as
  `crates/theos-maps/tests/fixtures/parity/settlement-0.3.0.toml`, with the driver's source and the
  tool versions in a comment. On the probe's numbers, whose definitions were close to these,
  0.3.0 passes 4 of 8 (overlaps on seeds 1, 5, 7, buildings on streets on 7 and 8); the recorded
  fixture, not this estimate, sets the bar.
- **The check:** `market-town` as a child of the Rust `fixture-small` seed 1, feature `town-1`,
  seeds 1–8, passes at least as many seeds as the fixture records. The parents differ (two
  generators); the class and the parent recipe and seed are the same (§5.19 ruling 3).
- Wall checks are stage errors (§5.8) rather than constraints, so a wall-less 0.3.0 town is not
  counted as failing them.

### 5.18 CE-78.4: golden runs

Added under the next `VERSION` (§5.2 item 9):

- `village` standalone, seed 1 ("a settlement");
- `market-town` from `fixture-small` seed 1, feature `town-1`, seed 1 ("a settlement's child
  map");
- `grid-town` from the same parent and feature, seed 1, so the planned style's streamlines are
  under the platform comparison.

### 5.19 How each AC is proven

| AC | Test |
|---|---|
| CE-78.1 | each built-in recipe runs standalone and as a child; the package reads back with `theos_map_format::read`, holds `height`, `water`, `streets`, `buildings`, `fields` and the feature kinds of §5.14; `base` and `plan` exist and `base` is frame-exact |
| CE-78.2 | a child of `fixture-small` seed 1 `town-1`: `origin_m` equals the snapped window corner, `parent.package_sha256` equals the parent file's SHA-256, `parent.feature` is `town-1`; two runs give the same package bytes; another feature or seed gives other bytes; refusals for a missing `from`, a wrong feature type and an unknown id |
| CE-78.3 | the parity test of §5.17 against the fixture |
| CE-78.4 | the golden test with the runs of §5.18 |

Unit tests cover each stage, the geometry kernels (hull, box, bisector cuts, lattice snapping)
and the checks of §5.8, each against a hand-made case.

### 5.20 Document changes in this PR

- **`docs/map-package.md`:** format 1.1 (§6) with the settlement additions of §5.14: the street,
  building, wall and gate properties, the kind `parcel`, the landmark types, the gate type
  `water-gate`, the wall type `tower`, and the `fields` legend of §5.12. P34's spec adds its
  interior rows to the same 1.1 (shared item 11).
- **`docs/generators.md` §3:** an input of type `feature` may carry `parent_types`, the feature
  types it accepts; the editor offers only features of those types (shared item 2).
- **`research/P29-settlements/`:** the 0.3.0 baseline driver and the probe's table.

## 6. Interfaces other phases use

| Interface | Used by |
|---|---|
| The shared child-map plumbing (§5.2) | P34, P30, P35, P42, P23 (blockouts read settlement packages as `from`) |
| `kernels::geometry` and the `i_overlay` wrapper | P34 (lattice rules), P35 (wards, bastions), P37 (ruin damage) |
| `street_graph`, `blocks`, `parcels`, `wall` world values | P35 (fortification and districts), P42 (city tiers), P37 (ruins), P31 (a Chime on a street, CE-81.4) |
| Settlement feature kinds and properties (§5.14) | P22 (pins on parcels and landmarks), P23 (blockouts), P25 (Atlas settlement view) |
| The street-style stage pattern: a zone, the graph in and out | Every later street style |
| `base` and `plan` for settlements | P16, P25 |

## 7. Risks

| Risk | Mitigation |
|---|---|
| Streamlines near degenerate points are chaotic, so a last-bit difference changes many streets | Fixed-step integration, `libm`, snapping each vertex; the tracer probe measured the sensitivity (§4.3); golden runs on four platforms include `grid-town` |
| `i_overlay` is a new dependency group (5 crates) | Pinned in `Cargo.lock`, no default features; one wrapper module; the probe's polygons become a unit test with a fixed hash |
| The inset band gives rougher corner plots than a straight skeleton | Corner pieces go to the wider street; remnants merge; parcels are checked for access and area. A skeleton can replace the band later behind the same function |
| Tuning three recipes takes long | Constraints and the `plan` contact sheet make iterations quick; Jann's bar is buildings, believable streets and readable landmarks |
| CE-78.3 compares towns on two different parents | Same parent recipe, seed and class; the constraints measure defects, not sizes |
| P29 and P34 change the same files (kinds, tables, golden file, format version) | §5.2, agreed word for word; the second to merge rebases |
| Speed: 0.3.0 took 2–4 s per town, 10–14 s per city | Budgets in the plan: a 1,600 m town at 4 m (400²) in under 10 s in release; the golden and parity tests use the test profile's budget |

## 8. Review Focus — inputs most likely to break it

1. **Parents:** a settlement at the parent's edge or corner (window past the edge); a parent with
   no roads (no entries: the stage must fail with a message or the recipe must allow it); a road
   that enters and leaves the window twice; a lake that covers the site point; a parent written by
   another generator version; a `.theosmap` that fails its hashes.
2. **Feature input:** an id that does not exist, a ruin given to a settlement recipe, a feature
   with a Polygon geometry.
3. **Streets:** two entries at the same cell; an entry on a river; a centre on a river island;
   a planned zone with a degenerate point inside; streamlines that run parallel at exactly
   `d_sep`; a street ending exactly on another street's vertex; collinear overlapping streets.
4. **Wall:** a wall zone crossed by a lake (no closed ring without crossing it); a river crossing
   the ring twice; a main street crossing the ring twice; a ring zone narrower than one cell.
5. **Blocks and plots:** a block that is a sliver, a triangle, or has a hole (a street loop
   inside); a block on both sides of a river; a corner with an angle near 180°; frontage bounds
   larger than the block; a landmark that cannot grow to its area.
6. **Lattice:** coordinates at the window's edge; values that round half-way; windows near the
   i32 limit.
7. **Determinism:** ordering of nodes and edges after noding; ties in priority queues; the RNG
   order when a stage is switched off; −0 in the tensor field.

## 9. Rulings (Claude's calls)

1. Ruling: the shared child-map plumbing is specified once and copied word for word into P34's
   spec — Jann ruled it shared — cost if wrong: two specs to edit together.
2. Ruling: a settlement recipe also runs standalone over `site_terrain` — CE-78.1 asks for maps
   "from a built-in settlement recipe" and CE-78.4 for "a settlement and a settlement's child
   map"; fixtures need no parent — cost if wrong: one small stage.
3. Ruling: CE-78.3 applies `market-town`'s defect constraints to 0.3.0's outputs on its own
   `fixture-small` seed 1 parent, with child seeds 1–8 set directly — 0.3.0's recipe has no
   constraints and its parents cannot be read by the Rust generator — cost if wrong: re-recording
   one fixture with another parent.
4. Ruling: one recipe file per shape serves child and standalone runs; `site_terrain` runs only
   when there is no parent, `local_terrain` only when there is one — two files per shape would
   drift — cost if wrong: a recipe split later.
5. Ruling: rivers in a child come from the parent's river lines with their widths, not from
   discharge — the lines are the parent's truth and need no threshold — cost if wrong: a
   discharge-based rebuild later.
6. Ruling: lake shores are not perturbed by seed — 0.3.0's seed-dependent shore cut a village off
   on 5 of 8 seeds — cost if wrong: smoother shores than 0.3.0.
7. Ruling: plots use an inset band cut at corner bisectors instead of a straight skeleton — no
   allowed crate is deterministic — cost if wrong: a skeleton module later behind the same
   function.
8. Ruling: the planned style integrates with a fixed step, not Chen's adaptive step — adaptive
   step choice amplifies float differences — cost if wrong: a smaller step and longer runs.
9. Ruling: the wall is a least-cost ring around the centre (own design) — no source read
   generates walls; 0.3.0's hull failed — cost if wrong: P35 replaces it.
10. Ruling: landmarks merge parcels instead of scaling a house — scaling put buildings on water and
    streets in 0.3.0 — cost if wrong: smaller landmarks.
11. Ruling: fields port 0.3.0's Voronoi method — no source on field systems was read — cost if
    wrong: a new `fields` stage later.
12. Ruling: `parcel` becomes a feature kind and `block` does not — pins and blockouts need plots;
    blocks follow from streets — cost if wrong: one more kind in a later 1.x.
13. Ruling: a run with no building fails — Jann's bar — cost if wrong: none.
14. Ruling: the street graph is noded by own snap rounding — the probe's naive rounding can bend
    segments into new crossings, and no allowed crate keeps a planar graph — cost if wrong: about
    250 lines replaced by a crate later.
15. Ruling: the planned style leaves discs around degenerate points to the square or organic
    streets — the probe's hilltop and radial centre gave slivers — cost if wrong: fewer planned
    streets near hilltops.
16. Ruling: the package writer writes format 1.1 for every kind, terrain included, so terrain
    package bytes change under the new `VERSION` — one writer path, and CE-77.2 allows changed
    outputs under a new version — cost if wrong: a per-kind version choice later.
17. Ruling: CE-78.3's defect metrics are raster measures on the map's own grid with one
    definition for both generators (§5.13) — 0.3.0 has no parcels or street graph — cost if
    wrong: re-recording the fixture.
18. Ruling: `market-town`'s required landmarks are 0.3.0's town set without `well` and `mill` —
    a stricter set would fail 0.3.0 on every seed and make the bar empty — cost if wrong: one
    recipe line.
19. Ruling: the town's population comes from the parent feature's `population` when present,
    otherwise from the class — cost if wrong: none.

## 10. Left to the builder

- Default values of every parameter not named here, tuned against the constraints and the `plan`
  contact sheet.
- The module split inside `stages/settlement/`, within the file-size habits of P08.
- The exact interest and domination weights and the tower spacing.
- Whether `planned_streets` stores the tensor field as a world value (for a debug render) or only
  inside the stage.
