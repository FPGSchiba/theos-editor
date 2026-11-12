# 06c · Built environments — state of the art and recommendations

Stage 5b of the map generation research. Scope: settlements (P29) and interiors (P29, P30) for
bands 2, 6, 7, 8, 9, 10, the Hem (floor 0), and the blends between bands.

## Summary

1. **One street engine can serve bands 1, 2, 6, 7, 10 and the Hem:** tensor-field streets (Chen et
   al. 2008). Its basis fields (grid, radial, terrain-following, boundary-following) blend with
   weights, so organic lanes, a street grid, and ring roads around the Bore are the same stage
   with different parameters. It is bit-identical with `libm`.
2. **Everything after the streets is shared too:** blocks from the planar street graph, parcels by
   oriented-bounding-box splitting (Vanegas et al. 2012), massing as attributes (`storeys`,
   `height_m`, `roof`), not meshes.
3. **Walls, castles, tiers, transit, ruin damage and style districts are stages** on the settlement
   kind. **No topic needs a new map kind.** The megastructure is large, but it is P30's interior
   kind with more stages.
4. **Interiors:** a cyclic mission graph built by graph rewriting (Dormans 2010, 2017), laid out by
   hierarchical subdivision, with tile solving (model synthesis, Merrell 2007) only as optional
   room fill. Lock-and-key checks scale if keys are never lost: a fixed point over the room graph,
   not a search over key sets.
5. **Ruins are a post-pass:** generate the intact place, then damage it with a seeded field. That
   keeps determinism and lets the same stage ruin any floor.
6. **Format 1.0 gaps, all additive (1.1):** a street graph (nodes, edge ids), `block` and `parcel`
   polygons, building height and roof, `levels` on settlements, a `deck` mask per level, rooms
   that span levels, a `district` kind with `style`, a `rail` kind, damage state, and a frame
   descriptor for the Bore.
7. **Learned city generators (diffusion, LLM agents) stay outside the generator,** as P08 §4.2
   already rules: no neural inference is bit-identical.
8. **Weak spots in the literature:** tiered cities, ring towns and deterministic ruin passes have
   no strong papers. The recommendations there are compositions of proven parts, and say so.

## 0. What I read, and what I could not

| Source | State |
|---|---|
| `BRIEF.md` | Read |
| P08 phase spec §4–§5.8 (Stage contract, recipes, runner, determinism, kernels) | Read |
| `docs/map-package.md` (all) | Read |
| Decision 0003 §3–§7 and "Research sources" | Read |
| Umbrella §3 entries P29, P30 | Read |
| Project docs `01-capability-baseline`, `02-band-requirements`, `03-floor-frame`, `decisions.md` | **Not read.** This agent has no Projects tool (`project_read` is not among its tools). I relied on the band table in the brief and on the task text. "The Hem, floor 0" is taken from the task only. Anything Jann decided in `decisions.md` overrides this report |
| `theos_maps` 0.3.0 settlement and interior code | Not available locally; P29's entry (CE-78) is what I know of it |
| Web | Search worked. Page fetches hit a usage limit part of the way through. Each source in the list at the end is marked **[opened]**, **[hit]** (found by search, title and venue confirmed, body not opened) or **[memory]** (cited from my own knowledge, not checked in this session) |

Licences of public code were checked on the repository pages where marked [opened]. Where I could
not open the page, the licence is marked "unverified" and must be probed before anyone reads the
code.

## 0.1 What the generator and format give today

From the files above:

- **Stage contract** (P08 §5.3): a stage declares typed layers in and out, typed parameters, checks
  and metrics. Layers may be rasters or "typed non-raster values" (the feature list, the road
  network). So a street graph, a parcel set or a room graph is an allowed layer type.
- **Kernels that exist after P08** (§5.8): Delaunay and nearest neighbour through `spade`, Voronoi
  by nearest seed, grid Dijkstra with custom cost, Kruskal, exact distance transform, labelling,
  morphology, marching squares, Catmull–Rom, spectral noise.
- **Settlement in format 1.0** (§4.4, §5): layers `height`, `water`, `streets`, `buildings`,
  `fields`; features `street` (LineString; `lane`, `street`, `main-street`), `building` (Polygon;
  `use`, `storeys`), `wall` (Polygon), `gate` (Point), `landmark`.
- **Interior in format 1.0:** layers `walkable`, `rooms`, `corridors`, `cover`; features `room`
  (Polygon; `number`, `role`, `keys`, `locked`), `door`, `connection` (no geometry; `from`, `to`,
  `key`), `route`, `link` (`stair`, `ladder`, `lift`, `drop`; `from_level`, `to_level`).
- **Levels** (§4.3) are defined for "an indoor map". All levels share one grid, at most about 2 km
  across at 1–2 m cells.
- **Missing for built environments:** street connectivity (a `street` has no `from`/`to` and no
  node ids), blocks, parcels, building height above `storeys`, levels outside interiors, rooms
  taller than one level, styles, rails, damage.

## 0.2 A rule for all geometry: integer coordinates

Polygon work (offsets, splits, block extraction) is where float results differ between
platforms and where predicates fail on near-degenerate input. Recommendation for every stage below:

- Snap every vertex to a fixed lattice (for example 1/16 m, exact in binary) when it is created.
- Run predicates in integers (`i64` cross products are exact at 2 km / (1/16 m) = 32,000 units).
- Use `spade` (already chosen, exact predicates) for triangulation, and probe `i_overlay`
  (MIT or Apache-2.0; integer core; boolean operations, polygon offsetting, slicing a polygon with
  a polyline) for boolean and offset operations. Its README recommends integer mode "for
  cross-platform deterministic construction" of strokes; its optional multithreading must stay
  off. A probe on the pinned toolchain must check its feature tree for `std` math, as P08 §4.3
  did for other crates.

This makes rule G20 hold by construction instead of by testing.

---

## 1. Medieval walled towns and castles (band 2)

### Methods

| Method | Source | What it does | Code licence |
|---|---|---|---|
| Ward / Voronoi patches | Watabou, Medieval Fantasy City Generator (2017–) | Voronoi patches around a centre; inner patches become wards; the wall follows the outline of the inner patches; gates sit on wall vertices; streets run along patch edges from gates to the plaza; each ward is cut into lots by recursive bisection. (Method from public descriptions and my memory; the devlog page I opened describes no algorithm) | `watabou/TownGeneratorOS`: **GPL-3.0** [opened]. Must not be read |
| Villages on terrain | Emilien, Bernhardt, Peytavie, Cani, Galin 2012 | Interest maps per building type place seeds; roads grow to connect them with a terrain-aware cost; parcels are cut along roads; buildings fit the slope | No public code known |
| Geometric growth over time | Weber, Müller, Wonka, Gross 2009 | Streets grow outward step by step, driven by land use and traffic; old cores are dense and irregular, later rings more regular | No public code known |
| Fortified cities through history | Mas, Martin, Patow 2020 | A loop over years with events: growth, wall building, wall extension, battles. Battle simulation finds weak wall sections and strengthens them. Tested on Carcassonne, Girona and Barcelona's Ciutadella | No public code known |

Also relevant: Galin et al. 2010 and 2011 (roads as least-cost paths, hierarchical networks),
which P08 already uses for the regional roads.

### Data the result needs

Street graph (nodes, edges with class and width), blocks, parcels, building footprints with use and
storeys, wall polygon with towers and gates, wards (keep, inner ward, outer ward for a castle), the
terrain `height` of the parent map under the frame.

### Recommendation

Do not copy the ward method's fixed Voronoi look. Build band 2 from four stages on the settlement
kind:

1. **`street_field` + `street_trace`** (see topic 3): a tensor field made from the terrain gradient
   (streets follow contours and fall lines), the incoming regional roads, and rotation noise. That
   gives the irregular, terrain-led net of an old town. The incoming roads of the parent map become
   the main streets, so the child map fits its parent (CE-78.2).
2. **`blocks` and `parcels`** (topic 3), with small, deep parcels.
3. **`fortify`**: the wall is a closed least-cost loop around the built core. Cost favours crests
   and steep outer slopes and penalises length, using P08's `paths` kernel on a ring-shaped search
   band. Gates go where main streets cross the wall; towers at a spacing parameter and at corners.
4. **`castle_site`**: an interest map (Emilien 2012) from prominence, slope and distance to water
   picks the site. Wards are nested polygons along contours around it (keep on the top, inner and
   outer ward below).

**Siege approaches as a check, not as a generator.** Borrow the idea of Mas et al., not the battle
simulation: a multi-source Dijkstra from the frame edge to the wall with an attacker's cost (slope,
water, distance under towers). Report as metrics: the number of distinct easy approach corridors,
the share of wall length reachable below a cost, the weakest gate. Recipes constrain them
(`siege.approaches = [1, 3]`), which gives designers "one obvious way in" on purpose.

| Point | Answer |
|---|---|
| 1 Licence | Papers only. The one public code base (Watabou) is GPL-3.0 and is not read |
| 2 Bit-identical | Yes: grid Dijkstra with total order, integer geometry, `libm` for the field |
| 3 Stage contract | Yes. `fortify` reads `height`, `street_graph`, `parcels`; writes `walls`, `gates` and updates `street_graph`. Checks: wall closed, every gate on a street, every block reachable |
| 4 Format needs | `wall` gets `height_m`, `thickness_m`; new types `tower` (Point), `ward` (Polygon; `keep`, `inner`, `outer`); `gate` gets `street` (edge id). All additive |
| 5 Size | Medium: two new stages on top of the shared street and parcel stages |

**Stage or kind:** stages on the settlement kind.

---

## 2. Vertical, tiered cities (band 6)

### Methods

I found **no paper that generates a city on several stacked tiers with lifts and docks.** The
closest pieces:

| Method | Source | Use here |
|---|---|---|
| Roads with bridges and tunnels as path costs | Galin, Peytavie, Maréchal, Guérin 2010 | A line may leave the ground at a cost; gives viaducts and ramps between tiers |
| Layered procedural content | Lipp, Scherzer, Wonka, Wimmer 2011 | City layouts as layers that are merged and edited independently; the data model for "one street net per tier" |
| Tile solving in 3D | Merrell 2007; Merrell & Manocha 2011; Gumin 2016; Stålberg's Townscaper and Bad North (talks, not papers) | Vertical massing from a tile set with stacking rules. Good for building volumes on a tier, weak for global structure such as "every tier reaches a lift" |
| Multilayer networks | Kivelä et al. 2014 | The clean model for analysis: one graph per tier plus inter-layer edges (lifts, stairs, ramps) |
| Terracing | Emilien et al. 2012 (buildings and roads adapt to slope) | Tiers as height bands of real terrain, for a city climbing a slope instead of stacked decks |

### Data the result needs

Per tier: a deck outline (a tier is not the whole frame), a street graph, parcels, buildings.
Between tiers: links (lift, stair, ramp, bridge) with both ends tied to street nodes. On the rim:
docks. For the engine: the deck's elevation and thickness.

### Recommendation

Treat a tier as a **level of a settlement**, reusing the levels mechanism P30 builds for interiors:

1. **`tiers`**: makes the deck outlines. Two modes: `terrace` (height bands of the parent terrain,
   cleaned with morphology and marching squares, both in P08's kernels) and `stacked` (nested or
   offset polygons with supports). Writes one `deck` mask per level and the level elevations.
2. **Per tier, the shared `street_field` → `street_trace` → `blocks` → `parcels`** run inside the
   deck mask.
3. **`tier_links`**: places lifts, stairs and ramps. A greedy cover: add a link where the walking
   distance to the nearest link is largest, until a bound holds (for example at most 250 m to a
   lift). Ramps and viaducts are least-cost paths in 3D over the tier graphs (Galin 2010's cost).
4. **`docks`**: sites on deck edges that face the open side or the Bore, chosen by an interest
   map; each is tied to a main street.
5. **Check:** the multi-level reachability of P30 (CE-79.2), run on the street graph instead of the
   room graph: every block reaches every tier.

Tile solving in 3D is not recommended for the tier structure. Use it later, if at all, for
building massing inside parcels.

| Point | Answer |
|---|---|
| 1 Licence | Papers only; own code |
| 2 Bit-identical | Yes: rasters, integer geometry, Dijkstra with total order |
| 3 Stage contract | Yes. The per-tier stages run once per level; the runner needs "run this stage for each level", which P30 needs for interiors anyway |
| 4 Format needs | `frame.levels` allowed on settlements (1.0 words it for "an indoor map"); a `deck` mask layer per level; `link` types `ramp`, `bridge`; `link` gets `from`/`to` (street node or room ids); site type `dock`; level gets `thickness_m`. All additive |
| 5 Size | Medium to large. Mostly reuse, but it depends on P30's level machinery and is the least proven design in this report |

**Stage or kind:** stages on the settlement kind, with levels. Not a new kind. It should come after
P30, because it shares the level runner and the cross-level analysis.

---

## 3. Modern street-grid cities (band 7)

### Methods

| Method | Source | Strength | Weakness | Code licence |
|---|---|---|---|---|
| L-system growth | Parish & Müller 2001 | Simple; global goals (grid, radial, follow population) plus local snapping rules | Order-dependent, hard to control, patterns do not blend | Many ports of unknown licence; none needed |
| Tensor-field streets | Chen, Esch, Wonka, Müller, Zhang 2008 | Streets are streamlines of a tensor field built from basis fields (grid, radial, boundary, height). Fields blend smoothly; major and minor directions give the two street families | Needs care at degenerate points; dead ends need cleanup | `ProbableTrain/MapGenerator` (TypeScript): **LGPL-3.0, with a GPL-3.0 file as well** [opened]. Do not read |
| Hierarchical domain splitting | Yang, Wang, Vouga, Wonka 2013 | Splits a district into blocks and parcels directly with streamline and template splits; very regular, good-looking suburbs | More complex; less suited to organic towns | None known |
| Example-based | Aliaga, Vanegas, Beneš 2008; Nishida, Garcia-Dorado, Aliaga 2016 | Copies the statistics or patches of real street maps | Needs example data and its licence; canon would depend on a data set | None needed |
| Learned road graphs | Chu et al. 2019 (Neural Turtle Graphics) | A neural model emits a road graph node by node | Neural inference: not allowed inside the generator | — |
| Parcels | Vanegas, Kelly, Weber, Halatsch, Aliaga, Müller 2012 | Two schemes: recursive oriented-bounding-box (OBB) splits, and straight-skeleton strips for perimeter parcels | Skeleton needs a robust straight-skeleton implementation | None needed |
| Massing | Müller, Wonka, Haegler, Ulmer, Van Gool 2006 (CGA shape); Schwarz & Müller 2015 (CGA++) | Split grammars turn a footprint into volumes, façades and details | A whole language; its output is meshes | CityEngine is commercial |
| Diffusion and LLM layout | CityDreamer (Xie et al. 2024), CityGen (Deng et al. 2023), CityCraft (2024), UrbanWorld (2024), CityGenAgent (2026) | Text- or image-conditioned layouts; LLM agents that write procedural programs | Neural; not reproducible bit for bit; trained on real cities | Not relevant inside the generator |

### Data the result needs

Street graph with classes and widths; blocks; parcels with frontage edge; building footprints with
height; transit lines and stops.

### Recommendation

**Tensor-field streets, OBB parcels, massing as attributes.** In stages:

1. **`street_field`** writes a tensor field (two f64 rasters, the components of a symmetric
   traceless 2×2 tensor). Basis fields are typed parameters: `grid` (angle, decay), `radial`
   (centre), `polyline` (follow a river, a wall, the rim), `terrain` (from `height`), plus rotation
   noise from P08's `spectral_noise`. Weights decay with distance (`libm::exp`).
2. **`street_trace`** traces streamlines of the major and minor eigenvectors with a fixed-step
   Runge–Kutta integrator, seeds in a deterministic priority order, stops at a separation distance,
   snaps to existing nodes. Main streets first with wide spacing, then streets, then lanes: the
   three classes the format already has. Writes `street_graph`.
3. **`blocks`**: faces of the planar graph (half-edge traversal in integer coordinates), inset by
   half the street width.
4. **`parcels`**: recursive OBB splits until area and frontage bounds hold; parcels without street
   access are merged or become courtyards. The skeleton scheme is left out until a straight-skeleton
   crate passes a probe.
5. **`buildings`**: footprint from the parcel by setbacks; `use` from a land-use field; `storeys`
   and `height_m` from a density field that falls off from centres; `roof` from the style.
   Setback tiers for towers as a short list of `[height_m, inset_m]`.
6. **`transit`**: lines are shortest paths over main streets between stops chosen by an interest
   map (the hierarchical idea of Galin et al. 2011). A tram runs at grade. An elevated line is the
   same route with a deck height, like the `deck_m` a bridge already has.

Why not Parish & Müller: it gives one family of patterns per rule set and no smooth blending, and
topics 1, 6 and 7 would each need their own rules. Why not full CGA: the map package should say
*what* stands where; turning a footprint and a height into a mesh is the engine's or the asset
pipeline's job (decision 0003 §6 keeps the package to "what a game needs to build the place").

**Learned methods** fit only the track P08 §4.2 names "inverse authoring": outside the generator,
a model may turn a sketch or a text into *recipe parameters* (basis fields, density centres). The
recipe then runs deterministically.

| Point | Answer |
|---|---|
| 1 Licence | Papers only. The known tensor-field code is LGPL/GPL and is not read. `spade` and (after a probe) `i_overlay` are MIT/Apache |
| 2 Bit-identical | Yes. The eigenvector of a 2×2 symmetric tensor needs `sqrt` (exact) and one `libm::atan2`, or no trigonometry at all with the half-angle formulas. Fixed step count, fixed seed order, integer snapping |
| 3 Stage contract | Yes, cleanly: each step has one typed output. Checks: graph planar, no node of degree 1 unless marked a dead end, every parcel has frontage, block area within bounds |
| 4 Format needs | `street` gets `id`-linked `from`/`to` node ids, `width_m`; new `junction` (Point) or node ids inside a `summary` graph; new kinds `block`, `parcel` (Polygon; `block`, `frontage`); `building` gets `height_m`, `roof`, `tiers`, `parcel`; new kind `rail` (LineString with heights; `tram`, `elevated`; `deck_m`) and site type `station` |
| 5 Size | Medium. `street_field` and `street_trace` are the core of the whole settlement kind, so most of the cost is paid once |

**Stage or kind:** stages on the settlement kind. P29 is specified as a port of 0.3.0's settlement
tool. If Jann applies P08's rule here too ("a state-of-the-art generator, not a port"), these
stages *are* the settlement kind and the 0.3.0 numbers become a sanity range. That is a decision
for Jann, because CE-78.3 names tolerances around the 0.3.0 reference.

---

## 4. Ruined and broken cities (band 8, and ruins on every floor)

### Methods

The research literature on *generating* ruins is thin. What exists:

| Method | Source | Use here |
|---|---|---|
| Rule-based destruction of a procedural city | Esri CityEngine "destroyed city" case study; von Tunzelmann 2013 (Man of Steel, SIGGRAPH talk) | Industry proof of the order of work: build the intact city procedurally, then apply destruction rules per building |
| Aging and weathering | Mérillou & Ghazanfarpour 2008 (survey) | Damage as fields that accumulate over time: exposure, moisture, growth |
| Fracture by Voronoi cells | Standard practice in effects tools (Houdini, Blender); no single paper | Breaking a slab or ground plate into fragments |
| Battle damage on walls | Mas, Martin, Patow 2020 | Damage concentrated where attacks happen |
| A journal article found by search (Multimedia Tools and Applications 2022, `s11042-022-13049-x`) | Not opened; I do not know its content | — |

### Data the result needs

For every building: state (intact, damaged, shell, rubble) and what it was. For every street edge:
passable, blocked, or gone. A rubble raster and a changed `height` (craters, debris). For broken
ground: fragments as polygons with an offset in height.

### Recommendation

**"Generate intact, then ruin"**, as one family of post-stages that work on any settlement or
interior:

1. **`damage_field`**: an f64 raster 0–1. Sum of: an age term (constant), low-frequency noise,
   seeded blast points with falloff, and optional gradients (toward the Bore, along a fault line).
2. **`ruin_buildings`**: each building gets a state from the field value at its centroid and a
   per-feature threshold. The threshold comes from a hash of the feature id and the stage seed, not
   from the order of a random stream, so one building's fate does not depend on another's. The
   footprint is kept; `storeys_left`, `state` and `was_use` are added. Rubble is rasterised around
   collapsed buildings by a dilation.
3. **`break_streets`**: edges are blocked or removed where damage is high, under a guarantee: a
   named set of nodes (gates, extraction points, landmarks) stays connected. Compute a spanning
   tree over those first, protect it, then cut. The check is the same reachability analysis the
   interiors use.
4. **`fracture`** (broken ground): Voronoi cells of seeded points (`spade`), merged into plates by
   a threshold on the damage field; each plate gets a height offset and optionally a gap. Streets
   and buildings are cut at plate borders with integer polygon clipping.
5. **For interiors:** the same pattern on the room graph: collapse rooms (not on the critical
   route), turn doors into `drop`s or rubble blocks, open walls between neighbours. The playability
   analysis runs after the pass, so a ruin can never be unsolvable.

Because the pass is a stage, the same floor can be baked intact and ruined from one seed, and
"ruins on every floor" is one more line in any recipe.

| Point | Answer |
|---|---|
| 1 Licence | No code used. Own design from general practice |
| 2 Bit-identical | Yes. Hash thresholds, rasters, integer clipping |
| 3 Stage contract | Yes, and it is the best fit of all topics: pure functions from an intact world to a damaged one. Metrics: share of buildings per state, share of edges cut, connectivity kept |
| 4 Format needs | `building` gets `state`, `storeys_left`, `was_use`; `street` gets `state` (`open`, `blocked`, `gone`); layer `rubble` (u8 density) and `damage` (u8); new kind `fragment` (Polygon; `offset_m`, optional `tilt`). A tilted fragment conflicts with "there is no rotation" (§2) only if fragments become child maps; as features they are fine |
| 5 Size | Small per stage; medium in total. `fracture` overlaps with terrain type F (floating broken ground), which another report may cover: the two should share one stage |

**Stage or kind:** stages, usable on both kinds.

---

## 5. Megastructure interiors (band 9, and ruins on every floor)

### Methods

| Method | Source | What it gives | Limits | Code licence |
|---|---|---|---|---|
| Mission and space grammars | Dormans 2010; Dormans & Bakkes 2011 | A graph grammar writes the mission (tasks, locks, keys); a shape grammar builds space for it. Structure first, geometry second | Rule sets are hand-made and take time to tune | Ludoscope is closed |
| Cyclic generation | Dormans 2017 (Unexplored), described by Boris the Brave 2021 | Levels start from a loop between entrance and goal, not a tree. One of 24 major cycle types sets the pacing; minor cycles and dead ends follow. Nodes sit on a coarse grid (about 5×5) and are only annotated, never moved. About 5,000 rules over about 50 grammar modules in the shipped game. Lock/key pairs stay abstract until resolved | The rule count shows the authoring cost | Closed |
| Layout from a planar graph | Ma, Vining, Lefebvre, Sheffer 2014; Nepožitek & Gemrot 2018 (Edgar) | Given a room graph and room shapes, finds a non-overlapping layout with configuration spaces and simulated annealing | Stochastic search with restarts; slow for thousands of rooms | Edgar: licence unverified |
| Binary space partition | Classic roguelike practice (RogueBasin) | Recursive splits fill a rectangle with rooms; fast, complete coverage | No structure by itself; tree-like connections | — |
| Model synthesis / wave function collapse in 3D | Merrell 2007; Merrell & Manocha 2011; Gumin 2016; Karth & Smith 2017; Merrell 2021 (comparison) | Fills a grid with tiles under adjacency rules; 3D works the same as 2D | Local rules only: no reachability, no lock-and-key. Fails more on large outputs. Merrell's comparison reports plain WFC success rates near 0 % on several 200×200 2D cases where model synthesis, which re-solves in blocks, nearly always succeeds | WaveFunctionCollapse: MIT by my memory; the page I opened did not show the licence text. Unverified |
| Space syntax | Hillier & Hanson 1984 | Measures on the room graph: depth from the entrance, integration, choice, ringiness | Analysis, not generation | — |
| Constraint solving | Smith & Mateas 2011 (answer set programming) | Declares playability as constraints and solves | A solver dependency; hard to make bit-identical and fast at this size | — |

### How lock-and-key and playability scale

- **Work on the room graph, not on cells.** A 2 km level at 1 m cells has 4 million cells; its
  room graph has at most a few thousand rooms. Cell-level checks (every room carved, doors open
  onto floor) stay per level and use P08's `labels` kernel.
- **Keys that are never lost make the check polynomial.** Start with no keys; flood the graph
  through edges the current keys open; add the keys found; repeat until nothing changes. That is at
  most K floods for K keys, each O(rooms + connections). A search over key subsets is not needed.
- **One-way drops need a no-dead-end check:** from the target of every drop, with the keys the
  player is sure to hold there, the goal must still be reachable. "Sure to hold" is the set of keys
  on every path to the drop (a dominator computation on the graph), which is conservative and
  cheap.
- **Levels form a second graph.** Solve inside a level between its links (stairs, lifts, drops),
  then solve on the graph of levels and links. This is the idea of hierarchical pathfinding
  (Botea, Müller, Schaeffer 2004) applied to reachability, and it keeps a 2 km × many-level floor
  in milliseconds.
- **Correct by construction first, checked second.** A grammar that only ever adds a lock together
  with its key on the entrance side cannot produce an unsolvable graph. The analysis then is a
  check (CE-78.3, CE-79.2), not a filter that throws seeds away.
- **Space-syntax measures as metrics:** mean depth, share of rooms on cycles, longest dead end,
  critical-route length in metres and in level changes. Recipes can constrain them.

### Data the result needs

Levels; rooms as polygons with a height, some spanning several levels (a machine hall with
galleries); voids (shafts, atria); the connection graph; links tied to rooms; the critical route
across levels; cover.

### Recommendation

Three layers, each a group of stages on the interior kind:

1. **Structure: `mission_graph`.** Cyclic graph rewriting on a coarse 3D lattice of nodes (levels ×
   rows × columns), in the manner of Unexplored: one main cycle, then minor cycles, shortcuts,
   locks with keys, one-way drops as valves. The rules are data in the recipe (TOML), few to start
   with (tens, not thousands), and the rewriting engine is own code. `petgraph` is not needed;
   P08 already avoids it for determinism reasons (its Dijkstra returns a `HashMap`).
2. **Space: `partition`.** Each lattice node owns a box of space. Inside, a BSP or treemap split
   makes rooms of the node's type: one node may become a single hall spanning three levels, another
   a block of small rooms. Halls are the megastructure's signature, so room height is a parameter
   of the room type. Doors and links are placed on shared walls of connected nodes.
3. **Fill: `room_fill`, optional.** Machines, columns, catwalks and cover inside a room by tile
   solving. Use model synthesis's order (scanline, block by block) with integer weights, a fixed
   retry count and a fallback to an empty room, so there is no entropy logarithm, no unbounded
   retry and no failure of the run. It may never change walkable connectivity; the check re-runs
   the labelling.

Tile solving alone is not recommended as the generator: it cannot promise reachability or pacing,
and its failure rate grows with size.

| Point | Answer |
|---|---|
| 1 Licence | Papers and articles only. Ludoscope and Unexplored are closed. Edgar and WaveFunctionCollapse licences are unverified and their code is not needed |
| 2 Bit-identical | Yes. Graph rewriting with matches in a total order and a per-stage stream; integer rectangles; tile solving with integer weights |
| 3 Stage contract | Yes. `mission_graph` writes a typed graph value; `partition` reads it and writes `rooms`, `walkable`, features; `room_fill` reads and writes rasters. The playability analysis is a stage whose findings fail the run |
| 4 Format needs | `room` gets `level_top` (or `levels`) and `height_m`; a `void` mask per level for shafts and atria; `link` gets `rooms` (the two room ids) so the graph is closed across levels; `connection` gets `via` (a link id); `route` gets a per-vertex level; link types `ramp`, `gantry`. All additive. The 2 km limit of 1.0 holds |
| 5 Size | Large. This is P30. The mission graph and the hierarchical analysis are the costly parts |

**Stage or kind:** stages on the interior kind (P29 for one level, P30 for many). Not a new kind.

---

## 6. Ring galleries around a void (band 10), and the Hem (floor 0)

### Methods

I found **no paper on annular settlements or ring-shaped dungeons.** The usable parts:

| Method | Source | Use here |
|---|---|---|
| Radial basis field | Chen et al. 2008 | A tensor field whose major direction circles a centre gives ring streets; the minor direction gives spokes. It blends with other fields |
| Radial rule | Parish & Müller 2001 | The same pattern as a growth rule; no blending |
| Periodic tile solving | Gumin 2016 (periodic output option, by my memory) | Solve on a strip that wraps in one direction, then bend it |
| Cycle as the base structure | Dormans 2017 | A ring gallery *is* the main cycle; the ring gives two ways round between any two points for free |
| Unrolling | Standard mapping (θ, r) → strip; the log-polar form keeps cells square | Lets every rectangular stage work on a ring |

### Data the result needs

The Bore's centre and radius in the frame; rooms or parcels as annular sectors (polygons with
enough vertices on the arcs); ring streets and spokes; balconies and bridges at the inner rim.

### Recommendation

- **The Hem (a ring-shaped town):** no new method. `street_field` with a `radial` basis at the
  Bore's centre, plus a `polyline` basis along the inner rim, masked to the annulus. Ring streets
  become main streets, spokes become streets, the wall (if any) is the outer ring, and the inner
  rim gets a promenade edge. Blocks and parcels then work unchanged; OBB splits of a ring block
  give wedge parcels.
- **Ring galleries (interiors):** a small kernel `polar` that maps a strip to the annulus and back.
  The interior stages run on the strip, with the lattice of topic 5 wrapping in θ, so the main
  cycle goes around the Bore. Room rectangles map to sectors; vertices are added on arcs so no
  chord leaves a tolerance (for example 0.25 m). The log-polar form needs `libm::exp`, `log`, `sin`
  and `cos`, which is allowed. Rasters stay in the square frame: cells are filled by mapping each
  cell centre back to the strip.
- **Stacked rings** are levels (topic 5), with bridges across the Bore as links of type `bridge`.

Cost of the square frame: the Bore's cells are empty. At 2 km and 1–2 m cells that is a few
megabytes per level before deflate and close to nothing after, so the format's shared grid holds.

| Point | Answer |
|---|---|
| 1 Licence | Papers only |
| 2 Bit-identical | Yes, with `libm` for the mapping and integer snapping after it |
| 3 Stage contract | Yes. `polar` is a kernel, not a stage; a stage parameter (`layout = "ring"`) selects it. The wrap in θ must be a property of the lattice, so neighbour queries are the only code that knows about it |
| 4 Format needs | A frame descriptor for the void: centre and radius of the Bore (probably defined by `03-floor-frame.md`, which I could not read). Optional `arc` hints are not needed; polygons suffice. Link type `bridge` |
| 5 Size | Small: one basis field that topic 3 needs anyway, one kernel, one layout option |

**Stage or kind:** stage parameters on both existing kinds.

---

## 7. Gradual transitions across ten floors

### Methods

| Method | Source | Works for | Limit |
|---|---|---|---|
| Parameter interpolation | General practice; Weber et al. 2009 vary parameters over time | Continuous values: street spacing, parcel size, storeys, damage level, room size | Cannot blend discrete choices (wall or no wall) |
| Field blending in one map | Chen et al. 2008 (weighted basis fields) | Street patterns that change across a map: organic core, grid outskirts | Streets only |
| Layers and masks | Lipp et al. 2011; Aliaga et al. 2008 | Regions of different style merged along seams | Seams need stitching rules |
| District-wise styles | Watabou's ward and district types; common in games | Any discrete style: each district is built by one style | Needs enough districts for a smooth share |
| Mixed tile sets | Merrell 2007; Gumin 2016 (tile weights) | Interior fill: one tile set per style, weights shift | Needs bridge tiles that are valid next to both sets, or the solver fails more |
| Grammar rule weights | Dormans 2010 | Interior structure: the share of room and cycle types shifts | Needs both rule sets in one grammar |

### Recommendation

Two devices, used together:

1. **A blend value `t` per floor** (0 at the band's first floor, 1 at the next band's), given to the
   recipe as an input. Every continuous parameter may be written as a pair, and the generator
   interpolates. This is a recipe feature, not a stage: P08's recipes take no inputs yet (§5.5), so
   it needs the `inputs` mechanism P29 introduces for child maps.
2. **Districts with styles.** A `districts` stage partitions the settlement (Voronoi of seeded
   centres, or the major blocks of the main-street net) or the interior (the lattice nodes of topic
   5). Each district takes style B if `hash(district rank) < t`, else style A, and later stages
   read the district's style for their parameters. If the district centres come from a seed shared
   by the ten floors of a band, the flip is monotone: a district that has turned stays turned on
   the floors above. Players see the new style spread from floor to floor.

Inside a floor, `street_field` weights can follow the district styles, so a grid district and an
organic one meet in a smooth bend instead of a seam. For interior fill, mix tile sets only through
explicit bridge tiles, and keep the fallback of topic 5.

Band pairs this covers: 1→2 (village to walled town: `fortify` appears per district as a partial
wall, then closes), 6→7 (tiers flatten: tier count and deck height interpolate, grid weight
rises), 7→8 (`damage_field` level rises from 0), 8→9 (interior districts replace outdoor ones),
9→10 (the lattice becomes a ring: the `polar` layout cannot be half on, so blend by district:
ring-shaped districts near the Bore first).

| Point | Answer |
|---|---|
| 1 Licence | No code |
| 2 Bit-identical | Yes. Interpolation is exact arithmetic; the hash is integer |
| 3 Stage contract | Yes. `districts` writes a typed value and a `district` raster; other stages read it as an optional layer, the pattern P08 uses for `omega` |
| 4 Format needs | New kind `district` (Polygon; `style`, `rank`); `style` as an optional property on buildings, streets and rooms; `blend` recorded in `inputs`, which the manifest already has |
| 5 Size | Small for the stage; the real cost is that every style-aware stage must take its parameters per district |

**Stage or kind:** one stage plus a recipe feature.

---

## 8. Summary table

| Topic | Recommended method | Stage or kind | Package format need (beyond 1.0) | Size |
|---|---|---|---|---|
| Shared core | Tensor-field streets (Chen 2008) → planar blocks → OBB parcels (Vanegas 2012) → massing attributes; integer geometry | Stages on the settlement kind | Street node ids and `from`/`to`, `width_m`; kinds `block`, `parcel`; `building.height_m`, `roof`, `tiers` | Medium |
| 1 Medieval towns, castles | Terrain-led tensor field; wall as a least-cost loop; castle site by interest map; siege approaches as metrics | Stages (`fortify`, `castle_site`) | `wall.height_m`, `thickness_m`; types `tower`, `ward`; `gate.street` | Medium |
| 2 Tiered cities | Tiers as settlement levels; per-tier street stages; link placement by cover; multi-level reachability check | Stages, after P30 | `levels` on settlements; `deck` mask per level; link types `ramp`, `bridge` with `from`/`to`; site `dock`; level `thickness_m` | Medium–large |
| 3 Street-grid cities | Grid basis fields; OBB parcels; density-driven heights; transit as paths over main streets | Stages (`transit` is new) | Kind `rail` (`tram`, `elevated`, `deck_m`); site `station` | Medium (core) + small (transit) |
| 4 Ruined cities | Generate intact, then `damage_field`, `ruin_buildings`, `break_streets`, `fracture`; connectivity protected | Stages, both kinds | `state`, `storeys_left`, `was_use`; layers `rubble`, `damage`; kind `fragment` with `offset_m` | Small each, medium in total |
| 5 Megastructure interiors | Cyclic mission graph by rewriting → BSP partition with multi-level halls → optional block-wise tile fill; fixed-point key analysis, hierarchical over levels | Stages on the interior kind (P30) | `room.level_top`, `height_m`; `void` mask; `link.rooms`; `connection.via`; route levels | Large |
| 6 Ring galleries, the Hem | Radial basis field for the town; `polar` strip mapping with a wrapping lattice for interiors | Stage parameters | Bore centre and radius in the frame; link type `bridge` | Small |
| 7 Transitions | Blend input `t` for continuous parameters; `districts` with hash-threshold styles, monotone across floors | One stage + recipe inputs | Kind `district`; optional `style` property | Small |

All format needs are additive, so they fit a minor version (map-package §6): `theos-map` 1.1.
None needs meshes or volumes in the package, and none breaks the 2 km shared-grid rule.

## 9. Open points for the coordinator

1. **P29 as a port or as state of the art.** The umbrella calls P29 "the rest of the port" and
   CE-78.3 sets tolerances around 0.3.0. The street core recommended here replaces 0.3.0's
   settlement method. Jann's P08 rule suggests the same choice, but it is his call.
2. **Order.** Shared street core → medieval stages → ruin stages → districts and blends → transit
   → (after P30) tiers → ring layouts. The interior track (mission graph, partition, analysis)
   runs beside it.
3. **Probes to run before a spec:** `i_overlay` on the pinned toolchain (feature tree, integer
   mode, byte-equal results on four platforms); a 200-line streamline tracer to confirm
   bit-identity and the look at 1–2 m; a straight-skeleton crate, only if perimeter parcels are
   wanted.
4. **Not done:** the project docs were not readable here; no code was run; several sources were
   confirmed by search hit only, as marked below.

## Sources

Marks: **[opened]** page read in this session; **[hit]** found by search with matching title, body
not opened; **[memory]** cited from my own knowledge, unchecked.

Local files:

- `research/map-generation/BRIEF.md`
- `docs/specs/phases/P08-map-format-and-terrain-generator.md` §4.2–§5.8
- `docs/map-package.md` §1–§9
- `docs/decisions/0003-map-generator-in-rust.md` §3–§7
- `docs/specs/00-umbrella.md` §3, P29 (CE-78) and P30 (CE-79)

Streets, parcels, buildings:

- Parish, Müller 2001. Procedural Modeling of Cities. SIGGRAPH 2001. doi:10.1145/383259.383292 [memory]
- Chen, Esch, Wonka, Müller, Zhang 2008. Interactive Procedural Street Modeling. ACM TOG 27(3)
  (SIGGRAPH). doi:10.1145/1360612.1360702.
  <https://www.sci.utah.edu/~chengu/street_sig08/street_project.htm> [hit]
- Vanegas, Kelly, Weber, Halatsch, Aliaga, Müller 2012. Procedural Generation of Parcels in Urban
  Modeling. Computer Graphics Forum 31(2). <https://diglib.eg.org/handle/10.1111/v31i2pp681-690> [hit]
- Yang, Wang, Vouga, Wonka 2013. Urban Pattern: Layout Design by Hierarchical Domain Splitting.
  ACM TOG 32(6) (SIGGRAPH Asia) [memory]
- Aliaga, Vanegas, Beneš 2008. Interactive Example-Based Urban Layout Synthesis. ACM TOG 27(5).
  <https://www.cs.purdue.edu/cgvlab/www/resources/papers/Aliaga-2008-Interactive_Example-Based_Urban_Layout_Synthesis.pdf> [hit]
- Nishida, Garcia-Dorado, Aliaga 2016. Example-Driven Procedural Urban Roads. Computer Graphics
  Forum 35(6) [memory]
- Chu et al. 2019. Neural Turtle Graphics for Modeling City Road Layouts. ICCV 2019.
  <https://openaccess.thecvf.com/content_ICCV_2019/html/Chu_Neural_Turtle_Graphics_for_Modeling_City_Road_Layouts_ICCV_2019_paper.html> [hit]
- Müller, Wonka, Haegler, Ulmer, Van Gool 2006. Procedural Modeling of Buildings. ACM TOG 25(3).
  <https://history.siggraph.org/?p=113547> [hit]
- Schwarz, Müller 2015. Advanced Procedural Modeling of Architecture. ACM TOG 34(4).
  <https://history.siggraph.org/?p=112748> [hit]
- Lipp, Scherzer, Wonka, Wimmer 2011. Interactive Modeling of City Layouts using Layers of
  Procedural Content. Computer Graphics Forum 30(2).
  <https://diglib.eg.org/handle/10.1111/v30i2pp345-354> [hit]
- Weber, Müller, Wonka, Gross 2009. Interactive Geometric Simulation of 4D Cities. Computer
  Graphics Forum 28(2) [memory]
- Galin, Peytavie, Maréchal, Guérin 2010. Procedural Generation of Roads. Computer Graphics Forum
  29(2) [memory; also cited in P08 §4.2]
- Galin, Peytavie, Guérin, Beneš 2011. Authoring Hierarchical Road Networks. Computer Graphics
  Forum 30(7). <https://diglib.eg.org/xmlui/handle/10.1111/v30i7pp2021-2030> [hit]
- `ProbableTrain/MapGenerator`, LGPL-3.0 with a GPL-3.0 file.
  <https://github.com/ProbableTrain/MapGenerator> [opened; code not read]

Medieval towns and castles:

- Watabou, Medieval Fantasy City Generator. <https://watabou.itch.io/medieval-fantasy-city-generator>;
  source `watabou/TownGeneratorOS`, GPL-3.0, <https://github.com/watabou/TownGeneratorOS>
  [repository page opened; code not read; the method description is from memory]
- Emilien, Bernhardt, Peytavie, Cani, Galin 2012. Procedural Generation of Villages on Arbitrary
  Terrains. The Visual Computer 28. <https://hal.archives-ouvertes.fr/hal-00694525> [hit]
- Mas, Martin, Patow 2020. Simulating the Evolution of Ancient Fortified Cities. Computer Graphics
  Forum 39(1), 650–671. doi:10.1111/cgf.13897.
  <https://diglib.eg.org/items/9a5a8549-7ca9-4e05-ac56-0c548beed419/full> [opened, abstract]
- Colyar, Matthews. Procedural Modeling of Medieval Castles. SIGGRAPH poster.
  <https://history.siggraph.org/?p=136742> [hit; not used for a claim]

Learned city layout (context only):

- Xie et al. 2024. CityDreamer: Compositional Generative Model of Unbounded 3D Cities. CVPR 2024 [memory]
- Deng et al. 2023. CityGen: Infinite and Controllable 3D City Layout Generation.
  <https://arxiv.org/abs/2312.01508v1> [hit]
- CityCraft: A Real Crafter for 3D City Generation, 2024. arXiv:2406.04983 [hit]
- UrbanWorld: An Urban World Model for 3D City Generation, 2024. <https://arxiv.org/html/2407.11965v2> [hit]
- Imagine a City: CityGenAgent for Procedural 3D City Generation, 2026.
  <https://arxiv.org/html/2602.05362v1> [hit]

Ruins:

- Esri, CityEngine "destroyed city" case study.
  <https://www.esri.com/en-us/arcgis/products/arcgis-cityengine/success-stories/destroyed-city> [hit]
- von Tunzelmann 2013. Man of Steel: Procedural City Building and Destruction. SIGGRAPH talk.
  <https://history.siggraph.org/?p=139803> [hit]
- Mérillou, Ghazanfarpour 2008. A Survey of Aging and Weathering Phenomena in Computer Graphics.
  Computers & Graphics 32(2) [memory]
- Multimedia Tools and Applications 2022, <https://link.springer.com/article/10.1007/s11042-022-13049-x>
  [hit; not opened, content unknown, no claim rests on it]

Interiors:

- Dormans 2010. Adventures in Level Design: Generating Missions and Spaces for Action Adventure
  Games. PCG Workshop, FDG 2010. doi:10.1145/1814256.1814257.
  <https://pcgworkshop.com/archive/dormans2010adventures.pdf> [hit]
- Dormans, Bakkes 2011. Generating Missions and Spaces for Adaptable Play Experiences. IEEE
  Transactions on Computational Intelligence and AI in Games 3(3) [memory]
- Dormans 2017. Unexplored's Secret: "Cyclic Dungeon Generation".
  <https://gamedeveloper.com/design/unexplored-s-secret-cyclic-dungeon-generation-> [hit]
- Boris the Brave 2021. Dungeon Generation in Unexplored.
  <https://www.boristhebrave.com/2021/04/10/dungeon-generation-in-unexplored/> [opened]
- Ma, Vining, Lefebvre, Sheffer 2014. Game Level Layout from Design Specification. Computer
  Graphics Forum 33(2). <https://hal-univ-tlse3.archives-ouvertes.fr/INRIA/hal-00927311> [hit]
- Nepožitek, Gemrot 2018. Fast Configurable Tile-Based Dungeon Level Generator. FDG 2018.
  <https://artemis.ms.mff.cuni.cz/main/papers/Fast_Configurable_Tile_Based_Dungeon_Level_Generator.pdf> [hit]
- Merrell 2007. Example-Based Model Synthesis. I3D 2007, 105–112. <https://paulmerrell.org/model-synthesis/> [hit]
- Merrell, Manocha 2011. Model Synthesis: A General Procedural Modeling Algorithm. IEEE TVCG 17(6) [memory]
- Merrell 2021. Comparing Model Synthesis and Wave Function Collapse.
  <https://paulmerrell.org/wp-content/uploads/2021/07/comparison.pdf> [opened]
- Gumin 2016. WaveFunctionCollapse. <https://github.com/mxgmn/WaveFunctionCollapse>
  [README opened; licence not shown in what I read]
- Boris the Brave 2021. Model Synthesis and Modifying in Blocks.
  <https://www.boristhebrave.com/2021/10/26/model-synthesis-and-modifying-in-blocks/> [hit]
- Karth, Smith 2017. WaveFunctionCollapse is Constraint Solving in the Wild. FDG 2017 [memory]
- Smith, Mateas 2011. Answer Set Programming for Procedural Content Generation. IEEE TCIAIG 3(3) [memory]
- Hillier, Hanson 1984. The Social Logic of Space. Cambridge University Press [memory]
- Botea, Müller, Schaeffer 2004. Near Optimal Hierarchical Path-Finding. Journal of Game
  Development 1(1) [memory]
- Kivelä et al. 2014. Multilayer Networks. Journal of Complex Networks 2(3) [memory]
- Stålberg, talks on Bad North and Townscaper [memory; no claim beyond "uses tile solving in 3D"]

Crates:

- `i_overlay`, MIT or Apache-2.0, integer core. <https://github.com/iShape-Rust/iOverlay> [opened]
- `spade`: P08 §4.3 (probe result in the repo)
