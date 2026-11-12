# P08 · Map format and terrain generator — phase spec

Status: approved (#27, 11 Nov 2026) · amended by decision 0004 (12 Nov 2026) · repo of the build:
`theos-editor`
Source: `docs/specs/00-umbrella.md` §3 P08, decision 0003
(`docs/decisions/0003-map-generator-in-rust.md`) and decision 0004
(`docs/decisions/0004-map-generation-research.md`). Scope and acceptance criteria come from there;
this spec decides how.

## 1. Goal

After P08 the workspace has two new library crates:

- **`theos-map-format`** reads and writes map packages (`docs/map-package.md`, format 1.0). It is
  small and depends on no other crate of this workspace, so the engine can depend on it alone.
- **`theos-maps`** is the map generator for terrain maps. It turns a recipe and a seed into a
  package, the `features` output, a frame-exact `base` render and renders for people, and the
  same inputs give the same package bytes on every platform (G20).

The generator is not a port of the Python toolset `theos_maps` 0.3.0. Jann decided on 10 Nov 2026
that P08 builds a terrain generator at the current state of the art. 0.3.0 provides the starting
recipes and a sanity range for the numbers, not the target (§4.1). Three research passes (§4.2)
found 0.3.0's architecture current but its processes about ten years old. P08 therefore adds:

- sediment deposition, so basins and lakes fill with sediment and valley floors widen;
- flow routing to several neighbours in erosion, so valleys lose the grid's bias (river lines still
  follow the steepest receiver, smoothed);
- a lake water balance;
- a slope limit per rock type;
- vegetation from a water balance;
- roads costed by the grade along their way, in 16 directions, with bridges as single spans and
  networks measured in terrain distance.

Every step of the generator, now and in later phases, is a **`Stage`** with one contract (§5.3).
A stage declares the layers it reads and writes, takes typed parameters, and is checked after it
runs. Errors therefore name the stage they come from. A stage can be tested on its own, switched
on or off in a recipe, and tuned through its parameters and metrics.

What later phases build on it:

- P09 wraps the library in the built-in adapter and adds `theos gen` (CE-29.4, CE-30.4);
- P16 shows runs, results and constraints;
- P22 reads `features` and the frame;
- P23 moves the recipes into the content project;
- P25 shows `base` under the pins;
- P29, P34 and P30 add settlement, interior and indoor-floor stages through the same contract;
- P31, P32 and P33 add the floor's frame and places, recipe families, metrics and batches;
- the engine reads packages with `theos-map-format`.

## 2. Acceptance criteria (verbatim from the umbrella)

**CE-76 Map package format** (`theos-map-format`, `docs/map-package.md`)
- CE-76.1 A package written by `theos-map-format` and read back gives the same manifest, layers
  and features, and its bytes follow `docs/map-package.md` §1–§5 (checked against a fixture
  package).
- CE-76.2 A package whose `format_version` has an unknown major is refused with a message naming
  both versions; fields, entries, layers and feature kinds the reader does not know are ignored.
- CE-76.3 A synthetic package with two stacked levels round-trips with every layer and feature on
  its level; on a map with levels, a layer or feature without a level, or with a level the frame
  does not list, is rejected.
- CE-76.4 `theos-map-format` depends on no other crate of this workspace.
- CE-76.5 A package whose `requires` names a capability the reader does not implement is refused
  with a message naming it.

**CE-28 theos-maps terrain generator**
- CE-28.1 `describe` gives the name `theos-maps`, the generator's own version, `produces: ["map"]`
  and the recipes with their inputs: the built-in terrain recipes converted from `theos_maps`
  0.3.0 (`hearth-valley`, `hearth-valley-omega`, `fixture-small`) and the recipes of a given
  folder, where a recipe of the folder wins over a built-in one with the same id.
- CE-28.2 `run` reports a progress event per stage and ends with a manifest carrying the frame,
  the outputs `base`, `truth`, `inworld`, `geology`, `resources`, `features` and `package` (plus
  the extra renders a recipe asks for), and results with each constraint under the keys of
  `docs/generators.md` §3 and `pass`.
- CE-28.3 A recipe whose stage order leaves a stage without an input fails before the first stage
  runs, with a message naming the stage and the missing layer; an unknown stage or recipe input
  fails the same way.
- CE-28.4 Cancelling a run stops it at its next cancel point, within one second at 384², and
  leaves no files behind; a stage that panics ends the run as failed with the panic's message.
- CE-28.5 For each built-in recipe over seeds 1–8, the constraints pass at least as often as with
  `theos_maps` 0.3.0, and the medians of relief, lake share, river share, vegetation cover and
  median precipitation stay within the tolerances the phase spec sets around the 0.3.0 reference,
  recorded as fixtures.
- CE-28.6 Every feature has an `id`, `properties.kind` and `properties.type` as in
  `docs/map-package.md` §5; lakes carry `surface_m`, and rivers are lines with width, discharge
  and bed height; the `base` render lies exactly on the frame (a feature at a known position lands
  on the expected pixel).
- CE-28.7 The omega detuning stage of 0.3.0 is a stage of the generator; `hearth-valley-omega`
  writes the `omega` layer.

**CE-77 Determinism** (G20)
- CE-77.1 For fixed recipes and seeds, the hashes of every layer after every stage, of the
  `features` output and of the package bytes equal the golden hashes on Linux x64, Windows x64,
  and macOS on Apple silicon and Intel.
- CE-77.2 Golden hashes are kept per generator version: outputs that differ from the golden hashes
  of the current version fail a test, and the tool that records golden hashes refuses to overwrite
  those of an existing version.

## 3. Scope

**In P08:**

- `crates/theos-map-format`: the package types, the writer and the reader.
- `crates/theos-maps`:
  - the `Stage` contract, the registry, the world of layers and the runner;
  - recipes (TOML) and the built-in terrain recipes;
  - the numeric kernels;
  - every terrain stage of §5.9, the omega stage among them;
  - features, renders and the contact sheet;
  - the package writing;
  - the library API with progress and cancel;
  - the metrics the parity check needs, the memory budget, the golden hashes with their recording
    tool, and the physics tests;
  - a developer example that runs recipes.
- The parity fixture of the 0.3.0 reference numbers (§5.14).
- Additions to `docs/map-package.md` (still format 1.0, since no package exists yet) and the Stage
  rule in `docs/architecture.md`, both in this spec's PR (§5.18).
- The gate PR before the build (§5.16).

**Not in P08:**

| Item | Where |
|---|---|
| The `builtin` adapter, settings registration, `theos gen` | P09 |
| Settlement maps and their child maps | P29 |
| Interior maps on one level, ruins' child maps, the room graph | P34 |
| Indoor floors with stacked levels (the format already carries levels) | P30 (generator), P45 (Atlas) |
| The extra terrain metrics (hypsometry, slope histogram, drainage-area distribution, pits, breaching volume) and the snapshot export | P33 (decision 0004) |
| Promoting settlements at road junctions, markets at route centres | The placement work of P31 or P32; the research track until then (decision 0004) |
| Recipes in the content project | P23 |
| Glacial erosion, layered strata, flexure, water depths (GraphFlood), several wind directions | Later, when a floor needs them (§4.4) |
| Learned methods (surrogate, inverse authoring, amplification) | A research track outside the generator: a learned step is always a separate generator whose output is baked and hashed (decision 0003; §4.2) |
| Refinement to walking scale | The engine |
| An inference runtime, GPU code, threads inside a run | Not in P08 |

## 4. Decisions

### 4.1 Decided by Jann (10 and 12 Nov 2026)

- Decision 0003 and its umbrella changes, merged in #24: Rust, in-process, baked canon,
  deterministic, format 1.0, stacked levels, focused phases, `theos gen` in P09.
- **Renders:** "a simpler version with the same data richness (or more)". Every layer and feature
  kind stays visible and named; the styling is plain (§5.11).
- **A state-of-the-art generator in P08, not a port:**
  - Jann: "if its clinging to the python stuff its not necessary".
  - Physics tests are the proof of correctness.
  - The 0.3.0 numbers are a sanity range (CE-28.5).
  - The look-changing upgrades are part of P08, not a later phase.
- **The `Stage` contract is mandatory for every step, now and in later phases.** Jann: "so that
  this pipeline is testable and errors are visible where they come from and tweaks can happen
  better as well".
- The crate and approach choices of §4.3 (Jann, 10 Nov 2026, on the list of eight). Point 4 (pit
  handling) changed after the research: keep the MST resolver and add deposition.
- **12 Nov 2026, after the map generation research (decision 0004):**
  - the text corrections of the audit (§4.2, §5.9);
  - the fluvial law as Yuan et al. 2019 write it, with their tolerance and an extra physics test
    (§5.9, §5.14);
  - the place types `chime`, `gate-up` and `gate-down` instead of `extraction`, `portal-out` and
    `portal-in`;
  - absolute discharge thresholds that every recipe sets, a mill rule relative to `river_q`, edge
    margins as a share of the frame, and single-flow channel discharge in `hydrology`;
  - a `requires` list in the manifest and CE-76.5;
  - no size limit in the format; a memory budget instead of a fixed grid limit;
  - `route_potential` leaves P08; the extra metrics and the snapshot export move to P33.

### 4.2 Research (10 Nov 2026)

Three passes, sources in their reports (summarised in §5.9 by stage):

1. **Terrain formation.**
   - 0.3.0's architecture is the mainstream FastScape family: uplift, implicit stream power, MST
     sinks, diffusion. It is ahead of the commercial tools (Gaea, World Machine, Houdini,
     Unreal), which stack hydraulic and thermal passes.
   - Its processes are about 2013-era: no sediment deposition (Yuan et al. 2019; goSPL; Landlab
     SPACE) and single-flow routing only.
   - Both gaps plausibly explain the observed weaknesses (oversized lakes, grid-aligned valleys,
     no filled basins); only runs with G > 0 and multiple flow can prove it. Both fit the same
     implicit solver, O(n) per iteration.
2. **Placement, refinement and tools.**
   - 0.3.0's placement follows the 2010–2012 research: after Emilien 2012's interest maps (which
     place buildings inside a village) and Galin 2010's roads. Its network construction is not
     Galin 2011's method, and no paper has a promotion step (the audit, decision 0004).
   - It is behind Galin 2010 in the 8-neighbour Dijkstra and the per-cell slope cost, and it
     measures links in straight-line distance.
   - Refinement to walking scale later fits multi-scale erosion amplification (Schott et al. 2024,
     MIT code; the code needs GPU compute shaders, so only the method can be reused under G20).
     The package must carry flow, rivers, lakes, roads and erosion fields for it.
3. **Deep learning.** Worth pursuing beside the generator, never inside it:
   - Stock frameworks do not give bit-identical inference across platforms; reproducible inference
     exists (RepDL, 2025) but adds a dependency. The conclusion stands for cost and dependency
     reasons.
   - Strict physics-informed networks are a poor fit for stream power.
   - Surrogates trained on our own generator output are the better hybrid.
   - Ranked tracks: inverse authoring (sketch → uplift), an erosion preview surrogate, engine
     amplification.
   - It asks the first port for hooks: a layer export, resolution-independent parameters, a
     metrics module, and stages as pure, typed steps. P08 builds the last three: §5.5; §5.14, with
     the parity metrics that P33 extends; §5.3. The layer export comes with P33 (decision 0004).
4. **The map generation research of 11–12 Nov** (`research/map-generation/`, decision 0004)
   audited these claims. Later options it names: analytical stream power as a deterministic
   preview (Tzathas et al. 2024), an adjoint model for inverse authoring (ESurf 2025), and the
   terrain-descriptor survey (Argudo et al. 2025) to check the metrics against.

### 4.3 Probes (10 Nov 2026, rustc 1.99.1, Windows 11)

Probe sources are in the session scratchpad. Their results, with what P08 takes from them:

| Part | Finding | Consequence |
|---|---|---|
| ZIP | `zip` 9.5 with `flate2`: a feature enabled by an unrelated crate (`flate2/zlib-rs`) changed the package bytes. `zip`'s default `system` is DOS on Windows and Unix elsewhere, and its `time` feature stamps the current time. With every option set explicitly and miniz_oxide as the backend, two writes are byte-identical. zopfli uses std `log2`. `rawzip` always writes data descriptors | `zip` without default features + `deflate-flate2`; `flate2` with `rust_backend` (miniz_oxide); every header field set explicitly (§5.13); cargo-deny bans on the other backends (gate PR) |
| FFT | `rustfft` computes its twiddles with std `cos`/`sin`, even in its scalar planner | Own radix-2 FFT with `libm` twiddles: relative difference to rustfft 7e-16; 512² forward + inverse 8.6 ms vs 6.5 ms |
| Drawing | `tiny-skia` 0.12's `simd` feature uses approximate reciprocals that differ by architecture; `std` brings std trigonometry | `tiny-skia` without default features, with `no-std-float` |
| Text | `ab_glyph` uses only exact float operations and has a `libm` feature. The design system's fonts are WOFF2 (Inter, JetBrains Mono, Spectral; SIL OFL 1.1). `wuff` (MIT) decodes WOFF2 | `ab_glyph` + `wuff`, fonts from `design/theos-ui/fonts` |
| Rasters | `imageproc` without default features still pulls in nalgebra/matrixmultiply and moxcms, which pick CPU code at runtime; its morphology works on u8 images only | Own raster kernels in f64 with scipy's semantics (§5.8) |
| Geometry | `kiddo` pulls in `generator` (inline-assembly coroutines); `spade` has exact predicates and nearest-neighbour queries; `petgraph`'s Dijkstra returns a `HashMap` | `spade` for Delaunay and nearest neighbour; own Kruskal and grid Dijkstra |
| `num-traits/std` | Enabled by the default features of `image`, `kiddo`, `moxcms`, `spade` | `spade` without default features; the plan checks the feature tree (§7) |
| Clippy | `disallowed-methods` catches inherent, UFCS and path calls of the f32/f64 methods; `num_traits::Float` generics only when listed | The ban list of §5.16 |
| Licences | Every crate above is in `deny.toml`'s allow list | — |

The Python 0.3.0 reference (Python 3.12, fastscapelib 0.3.0, numpy 2.5.4, scipy 1.18.1 through
uv) ran seeds 1–8 of the three recipes: about 17 s per hearth-valley seed (§5.14 has the numbers).

### 4.4 Approaches considered

| Question | Chosen | Not chosen, and why |
|---|---|---|
| Port or state of the art | State of the art with 0.3.0 as a sanity range | A faithful port keeps the known defects (Jann) |
| Pipeline model | Named, typed layers with a `Stage` contract checked before and after each stage (CE-28.3) | A fixed typed pipeline per kind: compile-time safety, but recipes could not choose or switch stages, and CE-28.3 asks for the check by name |
| Erosion processes | Separate per-step stages in an evolution loop, switchable per recipe | One monolithic erosion function: untestable per process and not tunable |
| Fluvial law | Erosion–deposition after Davy & Lague 2009 with the implicit solution of Yuan et al. 2019 (O(n) per Gauss–Seidel iteration); deposition coefficient G; G = 0 is detachment-limited stream power (Braun & Willett 2013) | Transport-limited only: wrong for bedrock mountains; explicit schemes: unstable at 25 kyr steps |
| Routing | Multiple flow directions in erosion (FastScape's slope-dependent exponent); single flow in channels for discharge, rivers and their lines (decision 0004); the steepest receiver kept for the MST resolver | D8 only: grid bias in valleys; Qin et al. 2007's exponent for discharge: dispersive on low gradients, so rivers would spread across plains (the audit); D∞: harder to make implicit with deposition |
| Pits | MST of basins (Cordonnier, Bovy & Braun 2019), an ε-lifted routing surface (Barnes et al. 2014), sediment filling depressions (after goSPL's documentation; the combination is ours) | Priority-flood filling of the surface each step: removes lakes that should exist |
| ZIP backend | `zip` + `flate2`/miniz_oxide + cargo-deny bans | `rawzip`: writes a 1980-00-00 date or extra fields, which the format forbids; Stored: masks would not shrink (97.5 % saved by deflate) |
| FFT | Own radix-2, padding to powers of two | Patched `rustfft`: a fork to maintain |
| Raster kernels | Own code | `imageproc`: u8/f32 images, runtime CPU dispatch in its dependencies |
| Glacial erosion, strata, flexure, GraphFlood | Later, when a floor needs them | They pay off only on special floors (alpine, for example); each is one more stage when it comes |

## 5. Design

### 5.1 Parts, and the tool or crate each one uses

| Part | Tool or crate | Why |
|---|---|---|
| Recipes | `toml` + `serde` | Like the project's other configuration |
| Package JSON | `serde_json` through `serde_json::Value`, whose maps are sorted | The format fixes key order (`docs/map-package.md` §3) |
| Features | `geojson` (no default features) | P22 reads `features` with the same crate (CE-60.2) |
| ZIP | `zip` (no default features, `deflate-flate2`) + `flate2` (`rust_backend`) | §4.3 |
| Hashes | `sha2` | Already in the workspace |
| Math functions | `libm` | G20; std's are banned (§5.16) |
| Random numbers | `rand_pcg` + `rand_core`; own samplers | Portable bit streams; `rand_distr` switches to std math |
| FFT | own radix-2 | §4.3 |
| Raster kernels (Gaussian, distance transform, labelling, morphology, filters, gradient, percentiles) | own code | §4.3 |
| Delaunay, nearest neighbour | `spade` (no default features) | Exact predicates |
| Spanning tree, grid shortest paths, priority queues | own code with total order | §4.3 |
| Erosion, routing, pits, deposition, diffusion | own code from the papers | No usable crate; fastscapelib and goSPL are GPL-3.0, so only papers and documentation are read; Landlab (MIT) may be read as a reference |
| Lake outlines | marching squares, own code (about 80 lines) | The `contour` crate works on f64 fields but its licence and float use are unchecked; the plan may probe it instead |
| Drawing | `tiny-skia` (no default features, `no-std-float`) | §4.3 |
| PNG | `png`, `Compression::Fast` (fdeflate) | Independent of the flate2 backend; renders are outside G20 anyway |
| Text | `ab_glyph` (`libm`) + `wuff` | §4.3 |
| Errors, logging | `thiserror`, `tracing` | G16 |

### 5.2 Crates and modules

```text
crates/theos-map-format/
  src/lib.rs          re-exports, docs
  src/manifest.rs     Manifest, Frame, Level, LayerEntry, DType
  src/layer.rs        LayerData (F32, U8, U16), encoding, canonical bytes, sha256
  src/package.rs      Package { manifest, features, layers }, validation (levels, shapes, NaN)
  src/write.rs        ZIP writing with fixed headers (§5.13)
  src/read.rs         reading, version check, unknown-field tolerance
  src/json.rs         canonical JSON (sorted keys, pretty, final LF)
  src/error.rs        FormatError
crates/theos-maps/
  src/lib.rs          API: describe, run, RunRequest, RunManifest, Progress, CancelToken
  src/stage/          the Stage trait, StageContext, LayerSpec, Finding, Metrics, registry
  src/world.rs        World: the layer store
  src/runner.rs       validation, memory budget, execution, checks, hashes, progress, cancel
  src/recipe/         schema, parsing, built-in recipes (recipes/*.toml, include_str!)
  src/rng.rs          seeds per stage, Pcg64, samplers
  src/kernels/        fft, noise, filters, distance, labels, morphology, curves, paths, mst
  src/stages/terrain/ one module per stage (§5.9)
  src/stages/evolution/ one module per per-step process (§5.9)
  src/features/       feature building, ids, kinds and types
  src/metrics.rs      the metrics module (§5.14)
  src/render/         base, truth, inworld, geology, resources, omega, contact sheet, fonts
  src/package.rs      building the theos-map-format package from a world
  src/golden.rs       golden-hash recording and comparison
  recipes/            hearth-valley.toml, hearth-valley-omega.toml, fixture-small.toml
  examples/run.rs     developer runner: recipes × seeds → outputs + contact sheet
  examples/bless.rs   records golden hashes for a new generator version
  tests/              behaviour, physics, parity, golden, determinism
  tests/fixtures/     parity numbers, golden hashes, package fixture (text)
```

### 5.3 The Stage contract

Every step of the generator implements one trait. The registry is a single list, one line per
stage (engineering directive 1).

```rust
/// One step of map generation. Every generation capability is a Stage (architecture §11).
pub trait Stage: Send + Sync {
    /// The id recipes use, `^[a-z][a-z0-9_]*$`.
    fn id(&self) -> &'static str;
    /// One line for `describe` and the docs.
    fn summary(&self) -> &'static str;
    /// Map kinds the stage applies to (`terrain` in P08).
    fn kinds(&self) -> &'static [MapKind];
    /// Parses and checks the stage's parameters from the recipe; unknown keys are errors.
    fn params(&self, table: &toml::Table) -> Result<Box<dyn StageParams>, ParamError>;
    /// Layers the stage reads with these parameters (name, type, optional).
    fn reads(&self, params: &dyn StageParams) -> Vec<LayerSpec>;
    /// Layers the stage writes with these parameters.
    fn writes(&self, params: &dyn StageParams) -> Vec<LayerSpec>;
    /// Names of the metrics the stage reports, known before anything runs.
    fn metric_names(&self, params: &dyn StageParams) -> Vec<&'static str>;
    /// Runs the stage. The context hands out only the declared layers; the report carries
    /// values that are not layers (for example a walking time between two Chimes).
    fn run(&self, ctx: &mut StageContext<'_>, params: &dyn StageParams)
        -> Result<StageReport, StageError>;
    /// Invariants checked after the run (finite values, ranges, balances).
    fn checks(&self, world: &World, params: &dyn StageParams, report: &StageReport)
        -> Vec<Finding> { Vec::new() }
    /// Numbers the stage reports into the summary, under the names of `metric_names`.
    fn metrics(&self, world: &World, params: &dyn StageParams, report: &StageReport)
        -> Metrics { Metrics::default() }
}
```

The capabilities above are binding for P08, P29, P30, P31, P33 and P34. The plan may rename items or split the
trait, but not lose a capability:

- reads and writes that depend on the parameters (a `landscape_evolution` without `hillslope`
  reads no `kd`);
- metric names known before the run, so constraint paths can be validated;
- checks and metrics that see the parameters and the run's report.

**`StageParams`** is a typed struct per stage, deserialised with `serde` (`deny_unknown_fields`,
defaults per field) and downcast inside `run`. All lengths are in metres or kilometres, and all
rates are per year, so a recipe means the same at any grid size (§5.5).

**`StageContext`** gives the stage:

- read access to the layers it declared in `reads`, and write access to those in `writes`;
- its own random stream (§5.7);
- the cancel token and a progress callback for long loops;
- the frame (cell size, grid, edges).

Asking for an undeclared layer, writing a layer of the wrong type, or ending without writing a
declared layer is a `StageError` naming the stage and the layer.

**`LayerSpec`** is a name, a type, and whether a read is optional. An optional read (for
example `resources` using `omega` when the recipe runs `detuning`) is not required by validation;
the context returns `None` when the layer is absent. The types are:

- `F64` raster: heights, rates, fields;
- `U8` raster: classes;
- `Mask`;
- `U32` raster: indices such as receivers;
- typed non-raster values: the feature list, the road network, the climate parameters, the flow
  graph.

The world stores them in a `BTreeMap` by name, so iteration order is fixed.

**Two levels, one trait.**

- **The pipeline:** a recipe lists stages in order.
- **The `landscape_evolution` stage** holds its own list of per-step stages (`uplift`,
  `route_steepest`, `resolve_sinks`, `partition_flow`, `fluvial`, `hillslope`, `slope_limit`,
  `climate_update`) and runs them in order every time step.
- **The same rules apply at both levels:** declared layers, typed parameters, checks, metrics.
  The evolution stage's own reads and writes are the union of its processes' (minus what one
  process writes for a later one), so validation sees exactly what a recipe's process list needs.
- **The recipe switches a process on by listing it.** "No deposition" is `fluvial` with `g = 0`,
  and "single-flow routing" is `partition_flow` with `exponent = "steepest"`.
- **Per-step processes are cheap to check:** their `checks` run every `check_every` steps (a
  recipe option, default 10) and on the last step.

**The rule for later phases** (`docs/architecture.md` §11, this PR):

- Every generation capability is a `Stage`.
- Shared mathematics are pure functions in `kernels/`, with their own tests and no state.
- A stage reaches the world only through its context.
- A new capability is a new stage module plus one registry line.

### 5.4 The world and its layers

- Rasters are row-major `Vec`s with the frame's grid. Row 0 is north and column 0 is west, as in
  `docs/map-package.md` §2, behind a small `Grid<T>` type with index helpers. `ndarray` is not
  needed for these operations.
- Computation is in f64. The package stores f32 (or u8/u16 classes) as the format's standard
  layers prescribe.
- Before hashing or writing, values are canonicalised: −0.0 becomes +0.0. NaN and infinities
  are errors naming the stage and the layer. `f64::max` and `f64::min` may return either zero on
  ±0 ties (`clamp` does not), so canonicalising at storage keeps the stored bits stable. Inside
  computations, §5.7 keeps the sign of zero out of comparisons.
- Layer names are snake case. A world layer that has a standard package layer uses that name
  throughout: `height`, `vegetation`, `temperature`, `precipitation`, `discharge`, `roads` (the
  road-class raster). The road network as a value is `road_network`. Internal layers (`uplift`,
  `erodibility`, `diffusivity`, `sag_rate`, `receivers`, `routing_surface`, `stack`) are written
  to the package only as snapshot layers, which P33 adds (decision 0004).
- **Non-raster values** (features, the road network, the climate parameters, the flow graph) have
  a canonical byte form: their `serde` value written as canonical JSON (§5.7). The golden hashes
  use that form.

### 5.5 Recipes

A recipe is a TOML file. The built-in recipes are embedded with `include_str!`. `describe` and
`run` take a list of recipe folders whose recipes win over built-in ones by id (CE-28.1).

```toml
id       = "hearth-valley"
title    = "Hearth valley terrain"
kind     = "terrain"
grid     = [384, 384]             # [rows, columns], as the format's frame
size_km  = [24.0, 24.0]           # [width, height], as the format's size_m
renders  = []                     # extra renders beyond the standard set, e.g. ["omega"]

[[stage]]
id    = "grid"
edges = ["south"]

[[stage]]
id = "tectonics"
background_uplift = 2.2e-4
  [[stage.source]]
  type = "orogen"
  path = [[0.02, 0.14], [0.40, 0.08], [0.98, 0.20]]   # [x, y] fractions (G19)
  width = 0.10
  rate = 2.6e-3
  asymmetry = 0.5
  jitter = 0.06

[[stage]]
id          = "hydrology"
river_q     = 0.15            # m³/s; required, no default
big_river_q = 2.0             # m³/s; required, no default

[[stage]]
id     = "landscape_evolution"
steps  = 240
dt_yr  = 2.5e4
  [[stage.process]]
  id = "uplift"
  [[stage.process]]
  id = "route_steepest"
  [[stage.process]]
  id = "resolve_sinks"
  [[stage.process]]
  id = "partition_flow"
  exponent = "slope_adaptive"
  [[stage.process]]
  id = "fluvial"
  m = 0.45
  g = 1.0
  [[stage.process]]
  id = "hillslope"
  [[stage.process]]
  id = "slope_limit"
  [[stage.process]]
  id = "climate_update"
  every = 30

[constraints]
"elevation_m.relief" = [800, 2200]
lake_pct             = [0, 8]
vegetation_cover_pct = [25, 75]
"features.ruin"      = [2, 2]
"features.chime"     = [2, 2]

[render.inworld]                  # optional; built-in recipes carry no in-world texts (§9)
title  = ""
labels = []
```

- **Schema.** Top-level fields as above. `[[stage]]` tables in pipeline order, each with `id`
  and the stage's parameters. Nested `[[stage.process]]` tables for `landscape_evolution`.
  `[constraints]` maps a summary path (dotted) to `[min, max]`. `[render.<id>]` sets render
  options. `renders` lists extra renders.
- **Order of pairs.** Every position is `[x, y]` as a fraction of the frame (G19). 0.3.0 wrote
  `[y, x]`, so the conversion swaps every pair: orogen paths, dome, basin and volcano centres.
  Grids are `[rows, columns]` and sizes `[width, height]`, as in the format's frame.
- **Summary paths and metrics.** The metrics module (§5.14) writes 0.3.0's top-level paths, so
  0.3.0's constraints keep their meaning: `elevation_m.relief`, `lake_pct`, `rivers_pct`,
  `vegetation_cover_pct`, `precip_mm_yr.median`, `travel.chime_to_chime_min`. It
  follows 0.3.0's definitions: vegetation cover is the share of cells with density above 0.5;
  river share is the share of non-lake cells whose discharge exceeds `river_q`. Stage metrics go
  under `summary.stages.<id>`. A constraint's result keys follow `docs/generators.md` §3
  (`elevation_m.relief` → `elevation_m_relief`, `_min`, `_max`, `_ok`).
- **Placement checks.** 0.3.0 silently counted placed ruins and extraction points in its "all ok".
  The converted recipes state them as constraints (`features.ruin`, `features.chime`), so
  CE-28.5 compares like with like. `features.<type>` is the count of features of that type.
- **Validation before any stage runs** (CE-28.3):
  - every stage id is known and applies to the recipe's kind;
  - its parameters parse;
  - every stage's reads are written by an earlier stage;
  - every per-step process's reads are written by an earlier process or before the loop;
  - recipe inputs are empty: the terrain recipes take no inputs in P08 (decision 0003 §9), so any
    input is "unknown recipe input";
  - constraint paths name values the metrics module or a stage's `metric_names` reports.

  Each error names the stage (and the layer or parameter).
- **A broken recipe in a folder** does not hide the others: `describe` lists the valid recipes and
  reports each broken one as a problem with its file and the error; `run` of a broken recipe fails
  with that error.
- **Built-in recipes.** `hearth-valley`, `hearth-valley-omega` and `fixture-small` keep 0.3.0's
  intent: the same sources, rock mixes, climate, feature counts and constraints. Their process
  parameters are tuned so their seeds pass the recipe's constraints more often than 0.3.0 did
  (CE-28.5).
  - `hearth-valley-omega` stays as 0.3.0 had it: no settlements, resources, roads or sites. So
    `pois` reads `features` and `travel` reads `road_network` as optional reads; without roads,
    travel uses off-road walking only.
  - Deposition steepens channels at steady state (slope grows by (1 + G/p̂)^(1/n), Yuan et al.'s
    eq. 24), so G > 0 raises relief at the same uplift. Uplift rates and K are retuned so relief
    stays inside the recipe's range.
  - Every recipe sets `river_q` and `big_river_q` on `hydrology`; they have no default. The
    built-in recipes keep 0.3.0's values (0.15 and 2.0 m³/s; `fixture-small` 0.05 and 0.5).
- **Units.** Lengths in metres or kilometres, positions as fractions of the frame, rates per year.
  0.3.0 gave filter sizes and iterations in cells; P08 converts them at 0.3.0's cell size
  (62.5 m), so they mean the same at 384² and scale correctly at other sizes.

### 5.6 The runner

1. **Validate** the recipe (§5.5). Nothing runs on an error.
   - **Memory budget** (decision 0004): the runner estimates the run's memory as the cells times
     the bytes of every layer the recipe's stages declare, times 1.5 for temporaries. Above
     `RunOptions.memory_budget` the run is refused before any stage runs, naming the largest
     layers. Without a budget from the caller it is 3 GiB. The machine-aware budget (at least
     3 GiB, more where the machine allows) is the caller's to set; in the editor that is P09.
2. **For each pipeline stage, in order:**
   - report a progress event `{stage: <stage id>, done: <index>, total: <pipeline length>}`;
   - check the cancel token;
   - parse the parameters;
   - run the stage inside `std::panic::catch_unwind`. A panic becomes `RunError::StagePanicked
     { stage, message }` (CE-28.4).
3. **After each stage:**
   - check the contract (every declared write present, with the right type);
   - run the stage's `checks`: an error-level finding fails the run naming the stage, and a
     warning goes into the summary;
   - record its `metrics` under `summary.stages.<id>`. Its wall time goes into the
     `RunManifest`'s `timing` only, never into the package, whose bytes must not change between
     runs;
   - hash every layer it wrote (canonical bytes, SHA-256) into the golden record (CE-77.1).
4. **After the pipeline**, each step with its own progress event (`stage` = `constraints`,
   `features`, `renders`, `package`):
   - evaluate the constraints into results (`docs/generators.md` §3 keys and `pass`). The runner
     does this, not a stage;
   - assign the feature ids and build the features (§5.10);
   - draw the renders;
   - write the package and the outputs into the output folder;
   - return the manifest.
5. **Progress inside long stages.** `landscape_evolution` reports
   `{stage: "landscape_evolution", done: <step>, total: <steps>}` every 10 steps, and the road
   router reports per routed link. Every pipeline stage reports at least its starting event.
6. **Cancel:**
   - The token is checked between stages, every time step of `landscape_evolution`, every 4,096
     queue pops of a shortest-path search, every row of a render, and between package entries.
   - On cancel or failure, the runner deletes everything it wrote into the output folder before
     it returns, and only that: files that were there before stay (CE-28.4).
   - Outputs are first written under a temporary name in that folder and renamed at the end, so a
     crash never leaves a complete-looking output.
7. **Snapshots** of the layers a stage wrote (`docs/map-package.md` §4.5), the training-data and
   debugging hook, come with P33 (decision 0004).

### 5.7 Determinism (G20)

The rules of decision 0003 §3, made concrete:

- **Math functions:** only `libm` (version pinned through `Cargo.lock`). The workspace-wide clippy
  ban of §5.16 rejects the std methods. Exact operations (`+ − × ÷`, `sqrt`, `abs`, `floor`,
  `round`, `ceil`, `trunc`, `rem_euclid`) are allowed. `mul_add` is banned too: on x86-64 without
  the `fma` target feature it calls the platform's C library, outside `Cargo.lock`. A fused
  multiply-add, where one is wanted, is `libm::fma`.
- **The sign of zero in computations:** priority-queue and sort keys are normalised with `+ 0.0`
  before comparison, since `total_cmp` orders −0 before +0. Maxima and minima of floats go through
  `kernels::num::{max, min}`, which return +0 on ±0 ties, never through `f64::max`/`f64::min`. A
  clippy ban on those would flag ordinary code in other crates (engineering directive 4), so this
  is a code-review item (§8) and the golden hashes on four platforms catch what slips through.
- **FFT:** own iterative radix-2 FFT with `libm` twiddles.
  - Spectral noise is synthesised at the next power of two of each dimension, with wavenumbers
    scaled to cycles per map, then cropped.
  - The orographic precipitation pads the terrain to the next power of two of 1.5 × its size with
    a linear ramp to zero.
- **Random numbers:**
  - Each stage gets `Pcg64` (from `rand_pcg`), seeded by SplitMix64 over (run seed, FNV-1a 64 of
    the stage's id and its occurrence number among stages with that id, starting at 0), so adding
    or moving another stage changes no stage's stream. Per-step processes are seeded the same way
    inside their loop.
  - Seed derivation and hashing use explicit wrapping arithmetic, so debug builds (overflow
    checks on) and release builds compute the same values.
  - Uniform f64 comes from the top 53 bits; integers in a range use Lemire's method; normal
    variates use Box–Muller with `libm`.
  - The derivation rule is part of the generator version.
- **Order:**
  - No `HashMap` reaches an output: the world, summaries and features use `BTreeMap` and `Vec`.
  - Sorts and priority queues compare with `total_cmp` and break ties by cell index, then by
    feature id.
- **Threads:** none inside a run in P08. One run is one thread (the adapter runs seeds in
  parallel, CE-29.4). Parallel kernels can come later under the rules of decision 0003 §3.6.
- **Canonical layers:** −0.0 becomes +0.0; NaN and infinities are errors (§5.4).
- **Canonical JSON:** a small writer in `theos-map-format` sorts object keys itself before
  printing, so the order does not depend on whether some crate enables `serde_json`'s
  `preserve_order`. It prints exactly what `docs/map-package.md` §3 describes.
- **Package bytes:** JSON as `docs/map-package.md` §3; ZIP as §5.13.
- **Floating-point environment:** the generator never changes it, and the library docs tell
  engine callers not to set flush-to-zero (decision 0003 §3.7).

### 5.8 Numeric kernels

Pure functions with unit tests against known values (scipy semantics where 0.3.0 used scipy):

| Kernel | Notes |
|---|---|
| `fft` | Radix-2, forward and inverse, 1D and 2D |
| `spectral_noise` | 1/f^β synthesis, kmin/kmax in cycles per map, zero mean, unit variance |
| `warp` | Domain warp of normalised coordinates |
| `gaussian` | Separable, reflect mode, truncate at 4σ, σ in metres |
| `uniform`, `maximum` | Box filters, reflect mode |
| `laplace`, `gradient` | Central differences (numpy's `gradient` at edges) |
| `percentile` | Linear interpolation (numpy's default) |
| `distance` | Exact Euclidean distance transform (Felzenszwalb & Huttenlocher 2012), with nearest-index output |
| `labels` | Connected components, 4- or 8-connectivity, union–find, labels in scan order |
| `morphology` | Binary dilation, erosion, opening with a cross or a disk in metres |
| `curves` | Catmull–Rom; signed distance to a polyline through `spade`'s nearest neighbour |
| `voronoi` | Nearest seed through `spade` |
| `paths` | Grid Dijkstra (single and multiple sources), 16 directions, custom edge cost, total order |
| `mst` | Kruskal with total order |
| `marching_squares` | Lake outlines as rings in metres |
| `priority_flood` | Barnes et al. 2014 with ε; filled surface and labels |

### 5.9 Terrain stages

Every stage below is a `Stage` with its own tests. Reads and writes are layer names (optional
reads in brackets). "Paper" names what the stage implements from; "own" marks this spec's own
design. GPL code is never read.

| Stage | Reads → writes | Method | Paper |
|---|---|---|---|
| `grid` | – → `frame`, `fixed` | Raster with drained (fixed-value) edges on chosen sides, the rest closed | – |
| `tectonics` | `frame`, `fixed` → `uplift`, `sag_rate`, `belts`, `warp` | 0.3.0's parametric sources (orogen, fault, rift, dome, volcano, basin) on warped coordinates with spectral variation. Young subsidence becomes `sag_rate` (m/yr), applied **during** evolution, so subsiding basins fill with sediment | Uplift-domain authoring as in Cordonnier et al. 2016 and Schott et al. 2023; the parametric sources are 0.3.0's |
| `geology` | `frame`, `uplift`, `sag_rate`, `belts`, `warp` → `rock`, `erodibility`, `diffusivity`, `critical_slope` | 0.3.0's provinces, fold belt, plutons, and alluvium where `sag_rate` is high. Per rock type: erodibility K, diffusivity kd, critical slope (granite and quartzite steepest, shale and alluvium gentlest) | Own table; the idea of threshold slopes after Montgomery & Brandon 2002 |
| `climate_model` | `frame` → `climate` | Orographic precipitation by linear theory (FFT), base + scaled orographic part; temperature by lapse rate; potential evaporation PET = clamp(300 + 45·T, 200, 1800) mm/yr (0.3.0's lake evaporation); actual evapotranspiration by Fu's form of the Budyko curve; runoff = P − AET | Smith & Barstad 2004; Fu 1981; Bracht-Flyr et al. 2013 |
| `landscape_evolution` | the union of its processes → `height`, `erosion`, `deposition`, `receivers`, `drainage_area` | Initial surface from spectral noise, then `steps` time steps of its process list (below) | – |
| ↳ `uplift` | `height`, `uplift`, `sag_rate`, `fixed` → `height` | height += (uplift − sag_rate)·dt; drained edges stay at base level | – |
| ↳ `route_steepest` | `height`, `fixed` → `receivers` | Steepest-descent receiver per cell (D8 distances), ties by index | Braun & Willett 2013 |
| ↳ `resolve_sinks` | `height`, `receivers` → `receivers`, `routing_surface`, `depressions` | Connects each closed basin through its lowest pass by the minimum spanning tree of basins; `routing_surface` lifts depression cells to their spill level + ε along the priority-flood order, `height` itself unchanged; `depressions` labels routed depressions with their spill node | MST of basins: Cordonnier, Bovy & Braun 2019; the ε lift: Barnes et al. 2014 |
| ↳ `partition_flow` | `routing_surface`, `receivers` → `weights`, `stack` | Weights over the lower neighbours of the routing surface for erosion: `exponent = "steepest"` (all to the receiver) or `"slope_adaptive"`, p = 0.5 + 0.6·S with S the steepest slope (FastScape's slope-dependent partition; that S is the steepest slope is this spec's reading). A fresh topological order (`stack`, receivers before donors) of the resulting graph | FastScape documentation; Freeman 1991 |
| ↳ `fluvial` | `height`, `erodibility`, `receivers`, `weights`, `stack`, `depressions`, `fixed`, `climate` → `height`, `erosion`, `deposition`, `drainage_area` | The erosion–deposition law with n = 1 (below), solved implicitly in stack order with Gauss–Seidel iterations; sediment filling routed depressions; cumulative erosion and deposition | Yuan et al. 2019 (eq. 9, 11, 13–21), after Davy & Lague 2009; Braun & Willett 2013. Multiple flow with deposition as in FastScape's documentation (Yuan et al. solve single flow); the depression filling after goSPL's documentation, combined with the MST resolver as our own step |
| ↳ `hillslope` | `height`, `diffusivity`, `fixed` → `height`, `erosion`, `deposition` | Linear diffusion, implicit ADI (Thomas algorithm per row and column), after `fluvial` as in Yuan et al.; per cell, a net loss books as erosion and a net gain as deposition | – |
| ↳ `slope_limit` | `height`, `critical_slope`, `fixed` → `height`, `erosion`, `deposition` | Thermal step (algorithm below) | The classic thermal erosion step of graphics (Musgrave, Kolb & Mace 1989); threshold slopes after Montgomery & Brandon 2002 |
| ↳ `climate_update` | `height`, `climate` → `climate` | Recomputes precipitation, AET and runoff from the current surface every `every` steps | – |
| `hydrology` | `height`, `climate` → `height`, `lake`, `lake_id`, `lake_level`, `river`, `big_river`, `discharge`, `flow_dir`, `water_surface`, `water_features` | Lakes by water balance, channel discharge over single flow, rivers, river lines and lake outlines (below) | Barnes et al. 2014; Bracht-Flyr et al. 2013; Quinn et al. 1991 (multiple flow on hillslopes, single flow in channels); Leopold & Maddock 1953; Génevaux et al. 2013 |
| `ecology` | `height`, `climate`, `drainage_area` → `temperature`, `slope`, `vegetation`, `snow`, `wetness` | Net primary productivity from actual evapotranspiration (the Thornthwaite Memorial model, NPP = 3000·(1 − e^(−0.0009695·(AET − 20))) g dry matter/m²/yr), density = min(NPP / 2000, 1) × a slope factor (1 − smoothstep(0.45, 0.9, slope), as 0.3.0) × (1 + a wetness bonus from ln(a / tan β), capped); snow from temperature and slope as 0.3.0 | Lieth & Box 1972; Lieth 1975; Sørensen et al. 2006 |
| `elements` | `river`, `lake`, `climate`, `vegetation`, `slope`, `rock`, `temperature` → `elements` | 0.3.0's ambient element mix (water, wood, earth, metal, fire) | – |
| `settlements` | terrain layers → `features` (settlements), `site_quality` | 0.3.0's site quality with hinterland; greedy picking by class with spacing (absolute, set per recipe); names from a recipe list or the built-in syllable lists (generated, not canon) | After Emilien et al. 2012, whose interest maps place buildings inside a village; siting whole settlements with them is 0.3.0's adaptation |
| `pois` | terrain layers, [`features`] → `features` (ruins, Chimes, gates) | 0.3.0's rules: ruins on prominent hard ridges; `chime` where 0.3.0 placed extraction points; `gate-down` and `gate-up` where it placed the arrival and ascent portals. With no settlements, no distance to settlements is enforced | – |
| `resources` | terrain layers, `features`, [`omega`] → `features` (deposits), `resource_fields` | 0.3.0's suitability fields | – |
| `roads` | terrain layers, `features` → `road_network`, `roads`, `features` (bridges, fords) | Network and routing on terrain cost (below) | Routing after Galin et al. 2010 (16 directions instead of its 80, no curvature cost); the network construction is own |
| `sites` | terrain layers, `road_network`, `features` → `features` (sites), `road_network`, `roads` | 0.3.0's mills, mines, quarries, lumber camps, shrines, watchtowers, inns (at the midpoint of long main and road links away from settlements) and camps, with spur tracks routed by the `roads` router. A mill needs discharge of at least `mill_q_factor` × `river_q` (default 2.7, which is 0.3.0's 0.4 m³/s at its 0.15) | – |
| `travel` | `height`, [`road_network`], water layers, `features` → `reach_min` | Tobler's hiking function on the 16-direction graph, a road speed-up and an off-path factor of 0.6 off roads, ford and lake penalties; multi-source Dijkstra from the Chimes. Unreachable cells, or every cell when there is no Chime, hold −1 (finite, documented); the Chime-to-Chime time goes into the stage report | Tobler 1993 |
| `detuning` | `height`, `features` → `omega` | 0.3.0's ω field: baseline, hot spots around ruins, slow drift (CE-28.7) | – |

Constraints are evaluated by the runner after the pipeline (§5.6), not by a stage.

**Margins.** `settlements`, `pois`, `resources` and `sites` keep their places away from the frame's
edge by `edge_margin`, a share of the frame's shorter side (default 0.025, which is 0.3.0's 0.6 km
at 24 km; 0.0125 for `resources`, 0.3.0's 0.3 km). A map is a region of a floor, so its edge is a
cut, not a landform (decision 0004).

**The fluvial law** (Yuan et al. 2019, eq. 11, after Davy & Lague 2009). With elevation h, uplift
rate U, erodibility K and the slope S to the receivers:

- Q = the sum over the cell and every cell upstream of p̂ × cell area, split by the flow weights,
  where p̂ = local runoff ÷ map-mean runoff (dimensionless). Q is the runoff-weighted drainage area
  and stands in both terms; with uniform runoff, Q = p̃·A as in Yuan's eq. 6 and 11.
- Qs = the sediment volume flux leaving the cell: the upstream sum of (U − ∂h/∂t) × cell area,
  uplift included.

      dh/dt = U − K·Q^m·S + G·Qs / Q

- G = d\*·v_s / r̄ (Yuan's eq. 9; Davy & Lague's eq. 7 and 17, where G/p̃ is their dimensionless
  number), with r̄ the map-mean runoff. G is dimensionless. With G = 0 the law is detachment-limited
  stream power (Braun & Willett 2013), 0.3.0's process family. There is no porosity term.
- At steady state, slope and relief grow by the factor (1 + G/p̂)^(1/n) (Yuan's eq. 24 and 28;
  Davy & Lague's eq. 15). With n = 1 and mean runoff that is 1 + G, so G = 1 doubles slopes at equal
  K and uplift.
- **Solution, each time step** (Yuan's eq. 20, 21):
  1. Accumulate Qs from donors to receivers (reverse stack order, split by `weights`) using the
     current heights.
  2. Solve h implicitly from receivers to donors (stack order).
  3. Repeat both until the largest height change between iterations is below 10⁻³ × max(U)·Δt
     (Yuan's tolerance; 10⁻⁶ m when no cell uplifts) or 50 iterations have run. Non-convergence is
     a warning finding with the residual; the result stays deterministic. Convergence is proven for
     G/p̂ ≤ 1 and shown experimentally above it; the iteration count depends on G, not on the grid.
- **Depression filling** (part of step 1, only when G > 0): when the Qs accumulation reaches a
  routed depression, the sediment volume fills the depression's cells in height order, up to its
  spill level. Filled cells get an ε slope toward the spill node, so they are not pits next step.
  The excess passes on from the spill node in the same sweep: `resolve_sinks` connects the spill
  node downstream of the depression in the stack. With G = 0 there is no deposition and no
  filling.
- `erosion` books the thickness removed per cell and step, `deposition` the thickness added
  (deposition and filling); both are cumulative, non-negative and in metres.
- Deposition alone is not expected to give braided rivers, meanders or deltas: recent work needs
  more than this law for alluvial forms (McDonald & Cordonnier 2026, seen at video level). P08
  promises filled basins and wider valley floors, not those forms.

**Slope limit.**

- Slope = the largest downhill gradient to the 8 neighbours (height difference ÷ distance).
- Passes visit cells from highest to lowest (ties by index). Where a cell's slope to its steepest
  neighbour exceeds its rock's critical slope, half the excess height difference moves to that
  neighbour, booked as erosion and deposition.
- Passes repeat until no slope exceeds critical + 1e-6, or 100 passes (a warning finding if not
  reached).
- Drained edges are sinks: material moved onto them leaves the map and is counted in the stage
  report, so the volume balance holds with that term.

**Hydrology.**

- **Depressions:** by priority-flood with ε on the final height (Barnes et al. 2014).
- **Lake balance:** each maximal depression receives land runoff Q_land from its catchment. Its
  equilibrium lake area is A = Q_land / (E_lake − P) when lake evaporation E_lake (= PET) exceeds
  precipitation P on the lake; otherwise it fills to spill. This is Bracht-Flyr et al. 2013's
  steady-state balance (rain on the lake counted), rearranged for the area.
  - A terminal lake fills the depression's cells in height order until its area reaches A, at one
    level.
  - Nested sub-depressions are not merged separately in P08; Fill–Spill–Merge (Barnes et al.
    2021) is a later option.
  - In Hearth's climate most depressions reach spill. Lake share then depends mostly on deposition
    in `fluvial` and on the shallow-pit rule.
- **Shallow pits:** depressions shallower than `min_lake_depth_m` (default 2 m) or smaller than
  `min_lake_area_m2` (default 4 cells) are filled to spill + ε in `height`. That is why
  `hydrology` writes `height`; river beds then never cross dry pits.
- **Channel discharge:** runoff accumulated over the single steepest receiver on the
  depression-filled surface, as 0.3.0 did and as channels behave (Quinn et al. 1991: multiple flow
  on hillslopes, single flow in channels). The `river` raster, the river lines, `discharge` and the
  fords therefore agree by construction, and discharge cannot spread across plains. Multiple flow
  stays in erosion (`partition_flow`) and in `drainage_area` for the wetness index.
  - lakes route over their outlets; terminal lakes absorb their inflow;
  - rivers are where discharge exceeds `river_q`, big rivers above `big_river_q`, outside lakes.
    Both thresholds are absolute discharges (m³/s) because widths, depths, fords and the lake
    balance depend on them, and every recipe must set both: they have no default (decision 0004).
- **River lines** (`water_features`):
  - traced downstream over the steepest receivers on the filled surface;
  - cut at confluences and at lake shores (an outlet starts a new line);
  - smoothed by two Chaikin passes;
  - per vertex: width w = a·Q^0.5 and depth d = c·Q^0.4 (the downstream exponents commonly given
    after Leopold & Maddock 1953, not checked against its body text), discharge, and bed height =
    the terrain height at the vertex (water surface = bed + depth); Strahler order per line.
- **Lakes:**
  - outlines by marching squares on `lake_id`;
  - `surface_m` = the lake level;
  - `outlet` = the spill node's position, absent for terminal lakes;
  - `depth_max_m`.
- **Raster outputs:**
  - `water_surface` = the lake level on lakes, else `height`;
  - `flow_dir` = the D8 code of the steepest downhill neighbour on the priority-flood-filled
    surface, so every cell points along its path to an outlet. 0 marks cells that drain off the
    map and the lowest cell of each terminal lake.

**Roads.**

- **Edges:** 16 directions (the 8 neighbours plus the 8 knight moves). A step's cost:
  - length × (1 + (grade ÷ comfort_grade)² × 6) while the grade along the step is below the
    class's `max_grade` (default main 0.08, road 0.10, track 0.15, trail 0.25), infinite above it;
  - plus forest cost × vegetation;
  - plus a ford cost for each stream cell crossed.
- **Every cell a diagonal or knight step passes through is checked.** A step may not cross a lake
  or big-river cell; such water is crossed only by bridge edges.
- **Bridge edges:** from a land cell, a straight line in each of the 16 directions across water
  cells to the first land cell, up to `max_bridge_span_m` (default 400 m). Cost: span × bridge
  cost × (1 + an angle penalty growing as the crossing departs from perpendicular to the river's
  flow direction), plus a deck term. `deck_m` = the higher bank's height + clearance (default
  3 m), in metres above the base level. Lakes get bridges only within the span limit; streams
  (below `big_river_q`) are forded.
- **Network** (own design):
  - candidate links are the Delaunay edges over settlements and gates, each weighted by its routed
    terrain cost;
  - the MST of those costs, plus each further link whose cost through the current network
    exceeds `loop_detour` × its direct cost, between settlements of class village or higher;
  - links are routed in order of class, then cost, with a discount on cells that already carry a
    road, so trunks merge;
  - road class follows the endpoints' classes, as 0.3.0;
  - trails to ruins;
  - bridges and fords become `site` features.

**Not in P08** (§3):
- glacial erosion (Hergarten 2021), layered strata K(x, y, z) (Cordonnier et al. 2018), flexure,
  GraphFlood water depths, several wind directions;
- Yuan et al.'s second erodibility for deposited sediment (their eq. 22, a basement layer);
- the transport needed for braided rivers, meanders and deltas (McDonald & Cordonnier 2026);
- promoting settlements at road junctions and markets at route centres, moved to the placement work
  of P31 or P32 (decision 0004).

Each is a later stage under the same contract.

### 5.10 Features and package layers

- **Features** follow `docs/map-package.md` §5. While the pipeline runs, stages refer to features
  by internal handles. After the pipeline the runner assigns the ids `<type>-<n>`, numbered per
  final type in the order the stages added the features (deterministic), and resolves every
  reference (`from`, `to`, `road`, `serves`, `deposit`) to those ids. A later stage that changes
  a feature's type (a promotion, in P31 or P32) therefore renames no reference.
- **Kinds and types are lower case with hyphens:** `gate-up`, `iron-ore`, `lumber-camp`.
- **Settlement names** carry no claim to canon.
- **Water features:**
  - rivers: `water`/`river` LineStrings with (x, y, bed height), `width_m`, `depth_m`,
    `discharge_m3s` per vertex, and `order`;
  - lakes: `water`/`lake` Polygons with `surface_m`, `terminal`, `outlet` (a point in metres, or
    none for a terminal lake) and `depth_max_m`.
- **Roads** carry `class`, `from`, `to`, `length_m`, `width_m` and `grade_max`. Bridges carry
  `span_m` and `deck_m`.
- **Layers written to the package for terrain** (the standard layers of §4.4 plus this spec's
  additions, §5.18):
  - heights and flow: `height`, `water_surface`, `flow_dir`, `drainage_area`, `discharge`.
    `drainage_area` is the plain upstream area in m² over multiple flow directions, not weighted
    by runoff (the fluvial law weights it internally); the wetness index uses the same area;
  - erosion: `erosion`, `deposition` (cumulative thickness removed and added by every process,
    §5.9);
  - climate: `precipitation`, `temperature`;
  - classes and densities: `water`, `rock`, `vegetation`, `snow`, `roads`;
  - the five `element_*` layers, and `omega` when the recipe runs `detuning`;
- **The manifest** carries the frame, the full recipe as it ran, the seed, the inputs (empty) and
  the results. Its summary holds the stage metrics, the metrics module's values (§5.14) and the
  constraint details. Timing is only in the `RunManifest` (§5.6).

### 5.11 Renders

Plain styling with every layer and feature kind visible (Jann). All PNG.

| Render | Content |
|---|---|
| `base` | Frame-exact (2 pixels per cell, square pixels): shaded relief (three-light hillshade), rock and soil colours, vegetation, farmland around settlements, snow, rivers by width, lakes. No symbols, no text. On the Atlas it lies under the pins (data model §8) |
| `truth` | `base`, plus: roads by class (width, dashes); every feature kind as a plain shape per type (square settlements sized by class, triangle ruins, circle Chimes, star gates, small marks for sites and deposits); settlement names; a legend of what is present; a scale bar; a title (recipe, seed, relief, lakes, Chime walk). It may have margins |
| `geology` | Rock colours over hillshade, rivers and lakes, legend |
| `resources` | Grey relief, deposits as circles by type sized by radius, a ring for exceptional tier, legend |
| `inworld` | Paper tone, the known area around settlements and main roads, main roads, settlements with names, lakes, big rivers, simple ridge and forest marks. Texts (title, labels and their positions) only from the recipe's `[renders.inworld]`, never from code; everything is drawn inside the frame |
| `omega` | ω over hillshade, ruins marked (an extra render, asked for by `hearth-valley-omega`) |

- **Drawing:** `tiny-skia` (scalar), labels with `ab_glyph`. Inter is used for `truth`, legends
  and the contact sheet, Spectral for `inworld`. Both are read from `design/theos-ui/fonts` with
  `include_bytes!` and decoded with `wuff` once per process.
- **Licence:** the fonts are under SIL OFL 1.1. `theos-maps` exposes the licence texts, so the
  binaries that embed it can ship them (Needs Jann, §7).
- **Contact sheet:** `render::contact_sheet(paths, columns)` tiles the first render of several
  seeds with labels. The developer example uses it, and `theos gen run` will (CE-30.4).

### 5.12 Library API

What P09's adapter and `theos gen` call. The types mirror `docs/generators.md` §3 and derive
`serde`, so the adapter maps them one to one.

```rust
pub const GENERATOR: &str = "theos-maps";
pub const VERSION: &str = "1.0.0";           // the generator's own version (§5.14)

pub fn describe(recipe_dirs: &[PathBuf]) -> Result<Description, RecipeError>;

pub struct RunRequest { pub recipe: String, pub seed: u64, pub inputs: serde_json::Map<String, Value>,
                        pub from: Option<PathBuf>, pub notes: Vec<Note>, pub options: RunOptions }
pub struct RunOptions { pub memory_budget: Option<u64> }    // bytes; 3 GiB when None (§5.6)

pub fn run(request: &RunRequest, recipe_dirs: &[PathBuf], out_dir: &Path,
           progress: &mut dyn FnMut(&Progress), cancel: &CancelToken)
           -> Result<RunManifest, RunError>;
```

- `RunManifest` holds `generator` (`theos-maps@1.0.0`), `recipe`, `seed`, `inputs`, `frame`,
  `outputs` (role → file name in `out_dir`) and `results`.
- `RunError` has typed variants: recipe invalid (with stage, layer or parameter), cancelled,
  stage failed, stage panicked, finding failed, I/O.
- `describe` lists, for each recipe: its id, title, `inputs: {}`, `seeded: true` and its stages
  with their summaries.
- `accepts_notes` is `false` in P08.
- `from` must be `None` for terrain recipes (child maps are P29).

### 5.13 `theos-map-format`

- **Types:** `Package { manifest: Manifest, features: geojson::FeatureCollection, layers:
  BTreeMap<LayerKey, LayerData> }`, `Manifest` and `Frame` with optional `origin_m`, `parent` and
  `levels`, `Level`, `LayerEntry`, `LayerKey`, `DType`; `Manifest.requires: Vec<String>`.
- **`write(&Package, impl Write + Seek)`:**
  - validates the package (shapes against the grid, levels rules, no NaN or infinity, unique
    names);
  - computes each layer's SHA-256 into its entry;
  - writes `requires`, sorted: the capabilities the package uses (none in P08, so `[]`);
  - writes `manifest.json`, `features.geojson` and the layers in index order.
- **ZIP options for every entry:**
  - `CompressionMethod::Deflated`, level 6;
  - `last_modified_time(DateTime::DEFAULT)` (1980-01-01 00:00);
  - `system(System::Unix)`, `unix_permissions(0o644)`;
  - no extra fields, no comments, `large_file` only above 4 GiB.
- **`read(impl Read + Seek)`:**
  - checks `format` and the major of `format_version` (an unknown major is refused, naming both
    versions);
  - reads the manifest leniently: unknown fields, entries, layers and feature kinds are ignored;
  - refuses a package whose `requires` names a capability this reader does not implement, naming
    it (CE-76.5); in P08 the reader implements none;
  - checks each layer's size and hash;
  - applies the levels rules (CE-76.3).
- **JSON:** values go through `serde_json::Value` and the crate's canonical writer (§5.7), which
  sorts keys itself and prints serde_json's pretty layout with a final LF. A test compares its
  output with `serde_json::to_string_pretty` on a value with sorted keys.
- **Layers are keyed by level and name** (`LayerKey { level: Option<String>, name: String }`),
  since a name is unique only within its level (`docs/map-package.md` §4.1).

### 5.14 Validation

**Physics tests** (behaviour tests, not criteria of their own):

1. **Steady state:** uniform U and K, G = 0, steepest routing. After many steps, channel slopes
   follow S = (U/K)^(1/n)·A^(−m/n) within 2 % for drainage areas above 10 cells.
2. **Stability:** with G = 0 and steepest routing, the implicit solver at 10× the default time
   step stays monotone: no cell falls below its receiver and no oscillation appears. With G > 0,
   the same run converges within the iteration limit and stays finite.
3. **Diffusion:** the ADI solution of a Gaussian bump with constant kd matches the analytic heat
   kernel within 1 %.
4. **Mass balance with deposition:** over a run, the change in total volume equals uplifted
   volume − the sediment flux Qs leaving at drained-edge outlets (taken from the routed flux, not
   from the difference of the other terms), to 1e-9 relative.
5. **Lake filling:** sediment routed into a closed depression fills it up to its spill level and
   passes on the excess.
6. **Discharge routing:** on a surface without depressions, the runoff accumulated over
   multiple flow directions and leaving over drained edges equals the total runoff put in (the
   routing itself loses nothing; lake evaporation is tested in 8).
7. **Priority-flood:** the filled surface has no interior pit; every cell drains to an edge over
   non-increasing heights (+ε).
8. **Lake balance:** a terminal lake's surface evaporation equals its inflow within one cell's
   share.
9. **Smith–Barstad:** wind over a symmetric hill gives positive precipitation on the windward
   side and negative on the lee side.
10. **Slope limit:** after `slope_limit`, no slope exceeds the rock's critical slope + 1e-6;
    volume is conserved once the material moved onto drained edges is counted.
11. **Orientation:** row 0 is north, column 0 west; a source at a known fraction appears at the
    expected cell.
12. **Roads:** no road step exceeds `max_grade` except bridge edges; bridges span the water in
    one edge.
13. **Resolution independence:** the same recipe at 192² and 384², over seeds 1–4, gives median
    relief and lake share within 15 % and median river length per km² (not river cell share,
    which halves with the cell size) within 25 %.
14. **Steady state with deposition:** with uniform U, K and runoff, n = 1 and no hillslope
    processes, the steady-state slope at G > 0 is (1 + G) times the slope at G = 0 within 2 %, and
    so is the relief (Yuan et al. 2019, eq. 24 and 28). Run with both flow partitions.

**Metrics** (`metrics.rs`), deterministic and reported in every summary: relief, lake share, river
share, vegetation cover, median precipitation and the Chime-to-Chime walk, as 0.3.0 defined them
(§5.5). The parity test uses them. The further terrain metrics (hypsometry, slope histogram,
drainage-area distribution, pits, breaching volume) come with P33 (decision 0004).

**Parity with 0.3.0 (CE-28.5).** `tests/fixtures/parity/theos_maps-0.3.0.toml` holds 0.3.0's
per-seed values for seeds 1–8 of the three recipes. It records the provenance: toolset 0.3.0,
Python 3.12, fastscapelib 0.3.0, numpy 2.5.4, scipy 1.18.1, the command and the date. The
reference code is Jann's export and is in no repository. The numbers (min / median / max over
seeds 1–8):

| Metric | hearth-valley | hearth-valley-omega | fixture-small |
|---|---|---|---|
| relief (m) | 1281 / 1765.5 / 2809 | 1243 / 1786 / 2782 | 212 / 349 / 2292 |
| lake share (%) | 1.46 / 13.31 / 32.04 | 1.38 / 17.57 / 31.14 | 0 / 2.64 / 11.25 |
| river share (%) | 0.93 / 1.32 / 1.87 | 0.88 / 1.33 / 1.78 | 0 / 0.32 / 2.25 |
| vegetation cover (%) | 37.8 / 55.55 / 82.4 | 37.8 / 55.35 / 82.5 | 32 / 38.45 / 59.8 |
| median precipitation (mm/yr) | 735 / 911.5 / 1091 | 731 / 909.5 / 1104 | 623 / 695 / 852 |
| seeds passing all constraints | 2 of 8 | 3 of 8 | no constraints |

**The tolerance** (the phase spec's to set, CE-28.5): for each recipe and metric, the median of
the Rust generator over seeds 1–8 lies within the 0.3.0 range (min to max). When the recipe
constrains that metric, the recipe's wanted range is added to it. Lake share always has a floor
of 0: fewer lakes is the intended fix (§4.2), so hearth-valley-omega, which has no lake
constraint, may lie anywhere in [0, 31.14]. The number of seeds passing all constraints (the
placement checks included, §5.5) is at least 0.3.0's. Appendix A holds 0.3.0's per-seed values,
which the fixture copies.

**Golden hashes (CE-77.1, CE-77.2):**

- **File per version:** `tests/fixtures/golden/<version>.toml`, one file per generator version.
  It holds, for each golden run, the SHA-256 of every layer after every pipeline stage, of
  `features` and of the package bytes.
- **Golden runs:** `fixture-small` seeds 1 and 2 at full pipeline, and `hearth-valley` seed 1.
  Per-step processes inside `landscape_evolution` are covered by the evolution stage's hashes
  and their own physics tests.
- **The test** compares a fresh run with the file of `VERSION`. A mismatch names the first stage
  and layer that differ. A missing file fails the test with the instruction to run `bless`.
- **The recording tool** `cargo run -p theos-maps --example bless` writes the file of the current
  version only if it does not exist; otherwise it refuses with a message to raise `VERSION`. A
  test proves the refusal.
- **Before P08 merges,** the 1.0.0 file exists only on the branch. The last build task records
  it, after the outputs have settled. A review-loop fix that changes outputs deletes that file and
  records it again; the deletion shows in the diff. Once 1.0.0 is on `main`, any output change
  needs a new `VERSION`.
- **Platforms:** the tests run on Linux in every PR, and on Windows, macOS Apple silicon and
  macOS Intel in `platforms` (§5.16). The P08 PR carries the labels `windows` and `macos` and
  links their runs.
- **Budget:** an early plan task measures one 384² run with the full process list. If 24 parity
  runs take longer than five minutes in the test profile, the plan says so, and the parity test
  moves to a separate job through a gate PR of its own.

**Determinism within one platform:** two runs in one process, and runs in two processes, give
identical package bytes.

### 5.15 The developer example

`cargo run -p theos-maps --release --example run -- <recipe>… --seeds 1..8 --out <dir>
[--recipes <dir>]` runs recipes and writes each seed's outputs and a
contact sheet. The P08 PR carries a contact sheet of the three built-in recipes for Jann's review.
That review is a review item, not an AC (decision 0003 §11). The example is not a shipped binary
and adds nothing to G4.

### 5.16 The gate PR before the build, and the profile

One gate PR (`gate/map-crates`), waiting for Jann, before the build starts:

1. **The two crates:** `crates/theos-map-format` and `crates/theos-maps`, empty and documented
   (README, crate docs), as P00 did for the others.
2. **`xtask/tests/workspace.rs`:** both crates in the expected members.
3. **`gate/thresholds.toml`:** both crates as `library` (floor 80 %).
4. **`gate/clippy.toml` and its root copy:** `disallowed-methods` for `f32::` and `f64::` `sin`,
   `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`,
   `atanh`, `exp`, `exp2`, `exp_m1`, `ln`, `log`, `log2`, `log10`, `ln_1p`, `powf`, `powi`,
   `hypot`, `cbrt`, `sin_cos`, `mul_add` and `algebraic_add`, `algebraic_sub`, `algebraic_mul`,
   `algebraic_div`, `algebraic_rem`, each with the reason "not the same bits on every platform;
   use libm (G20)". No existing code calls them (checked 10 Nov 2026).
5. **`deny.toml`:** bans on `zlib-rs`, `libz-sys`, `libz-ng-sys` and `cloudflare-zlib-sys`, so no
   crate can swap the deflate backend. The gate PR proves the ban with a probe.
6. **`.github/workflows/platforms.yml`:** an Intel macOS job running the tests: on `macos-15-intel`
   while GitHub offers it, otherwise built for `x86_64-apple-darwin` and run under Rosetta on the
   Apple-silicon runner. Like the other macOS jobs, it runs weekly, on demand and when a PR gets
   the label `macos`.

**The profile:** `[profile.dev.package.theos-maps] opt-level = 2` in the root `Cargo.toml` (not a
gate file).

- The parity test (24 runs) and the golden runs then take minutes instead of the better part of
  an hour unoptimised; the plan's budget task measures it (§5.14).
- The root manifest's comment ("workspace crates keep the dev profile") gains the exception and
  its reason.
- Coverage is still measured: instrumentation comes before optimisation. If the coverage
  numbers suffer, the plan says so.

### 5.17 How each AC is proven

| AC | Test |
|---|---|
| CE-76.1 | Round trip of a package built from a text fixture (manifest JSON, features GeoJSON, layer values in TOML); the written bytes' SHA-256 equals the recorded one; the entry order, and every local-header and central-directory field (time, date, version made by, permissions, flags, extra-field lengths, comments), parsed from the bytes by the test itself and checked against §1 |
| CE-76.2 | A manifest with major 2 is refused with both versions in the message; a manifest with unknown fields, an unknown entry, an unknown layer and an unknown feature kind reads |
| CE-76.3 | A synthetic two-level package round-trips with every layer and feature on its level. Four rejections, each by the writer and by the reader: a layer without a level, a layer with an unknown level, a feature without a level, a feature with an unknown level. The reader tests use packages assembled by the test with the `zip` crate, bypassing the writer's validation |
| CE-76.4 | `cargo_metadata` (dev-dependency): `theos-map-format`'s dependencies contain no workspace member |
| CE-76.5 | A package whose `requires` lists `slabs` is refused with a message naming `slabs`; a package with an empty `requires` reads; the writer writes `requires` sorted |
| CE-28.1 | `describe` with no folder lists the three built-ins with `name`, `version`, `produces`; with a temporary folder holding a `fixture-small.toml` of another title, that one wins |
| CE-28.2 | A 64² test recipe with constraints and the extra render `omega` reports a starting progress event for every pipeline stage in order, and returns the standard outputs, the `omega` render and the constraint result keys of `docs/generators.md` §3 with `pass` |
| CE-28.3 | Recipes with a stage before its input, an unknown stage, an unknown parameter and a recipe input each fail before any stage runs, naming stage and layer or parameter |
| CE-28.4 | (a) A `hearth-valley` run cancelled from an evolution progress event returns `Cancelled` within one second of the cancel. (b) A run cancelled at the `package` progress event, in a folder that already holds an unrelated file, returns `Cancelled` and leaves only that file. (c) A test-only registry adds a stage that panics; the run fails with its message and leaves no files of its own |
| CE-28.5 | The parity test over seeds 1–8 of the three recipes against the fixture, with the tolerance of §5.14 |
| CE-28.6 | On a recipe and seed whose run has at least one lake and one river (the test asserts both exist first): every feature has `id`, `kind`, `type`; lakes have `surface_m`; rivers have per-vertex width, discharge and bed height. A synthetic world with a lake feature at a known position in metres renders lake colour at the pixel that position maps to in `base` (centre of the lake; edges may be anti-aliased) |
| CE-28.7 | `hearth-valley-omega` runs `detuning` and its package has the `omega` layer |
| CE-77.1 | The golden test (§5.14): on Linux in the PR; on Windows, macOS Apple silicon and macOS Intel through the `windows` and `macos` labels on the P08 PR, with the runs linked |
| CE-77.2 | Changing an output fails the golden test; `bless` refuses to overwrite the file of an existing version |

### 5.18 Document changes in this PR

- **`docs/map-package.md`**, still format 1.0 because no package or reader exists yet; all
  additions are optional:
  - §4.4 adds the standard terrain layers `water_surface`, `flow_dir`, `drainage_area`,
    `erosion` and `deposition`, defined as in §5.9 and §5.10;
  - §5 adds the river properties `depth_m` and `order`, the lake properties `outlet` and
    `depth_max_m`, the road properties `width_m` and `grade_max`, the bridge properties
    `span_m` and `deck_m` (the deck's height above the base level), and the site type `market`
    (placed by `route_potential`);
  - a new §4.5 defines snapshot layers (`<layer>@<stage>`, files under `layers/snapshots/<stage>/`,
    f32 or u8, ignored by readers that do not ask for them).
- **`docs/architecture.md` §11:** the Stage rule (§5.3).

Decision 0004 (12 Nov 2026) later removed the site type `market` again, with `route_potential`,
renamed the place types, added `requires` and dropped the size note of §4.3.

## 6. Interfaces other phases use

| Interface | Used by |
|---|---|
| `theos_map_format::{Package, Manifest, Frame, Level, LayerEntry, DType, LayerData, read, write}` | The engine; P22 (frame); P29, P30, P31, P34; the blockout (P23 reads packages in Python) |
| `theos_maps::{describe, run, RunRequest, RunOptions, RunManifest, Progress, CancelToken, RunError, GENERATOR, VERSION}` | P09 (`builtin` adapter, `theos gen`) |
| The recipe schema (§5.5) | P23 (content recipes), P29, P30, P31, P32 (families), P34 |
| The `Stage` trait, `StageContext`, `LayerSpec`, the registry | P29, P30, P31, P33, P34 and every later generation capability |
| `render::contact_sheet` | P09 (`theos gen run`) |
| The metrics (§5.14) | P29, P34 parity; P33 extends them |
| `RunOptions.memory_budget` | P09 (the machine-aware budget, seeds in parallel) |
| The constraint result keys | P16 (Maps screen) |
| The `base` render and feature types | P22, P25 |
| Golden files per version, `bless` | P29, P30, P31, P34 |

## 7. Risks

| Risk | Mitigation |
|---|---|
| Yuan et al. 2019 solve single flow; multiple flow with deposition is FastScape's practice, not the paper's | Physics tests 1, 4, 5 and 14 run with both flow partitions; FastScape's documentation and Landlab's ErosionDeposition (MIT) are readable references |
| Several new processes at once make defects hard to place | Each process is its own stage with its own tests; the recipe can switch each off; golden hashes per stage name where a change appears |
| Tuning the built-in recipes takes long | Tuning targets are the recipe constraints; the developer example's contact sheet and metrics make iterations quick; CE-28.5's tolerance is wide |
| A dependency quietly enables `num-traits/std`, `serde_json/preserve_order` or another deflate backend | `spade` without default features; the plan inspects `cargo tree -e features`; deny bans for backends; a test asserts sorted JSON keys; golden hashes on four platforms catch the rest |
| −0.0 from `max`/`min`/`clamp` differs between x86 and ARM | Canonicalisation before hashing and writing; a division by a value that can be −0 is a code-review item (§8) |
| Golden hashes differ on a platform only the weekly run sees | The release run covers all platforms; a red weekly run blocks the next phase from claiming done |
| A run at 1024² is slow (deposition iterations, 16-direction paths) | Budgets measured in the plan; P08's recipes are 384²; threads inside a run are a later option |
| The OFL fonts must ship with the binaries that embed them | Needs Jann: P09 (or the release workflow) adds the licence texts to releases |
| `catch_unwind` cannot catch an abort (out of memory) | The memory estimate before the run (§5.6) refuses a recipe above the budget |

## 8. Review Focus — inputs most likely to break it

1. **Recipe edges:**
   - no drained edge (all closed) or every edge drained;
   - a 64² grid with metre-sized kernels larger than the map;
   - a non-power-of-two grid;
   - flat terrain, where routing has ties;
   - zero precipitation or zero uplift;
   - a recipe without `river_q` or `big_river_q`.
2. **Erosion:**
   - G = 0 vs G > 0;
   - `steepest` vs `slope_adaptive` routing;
   - a depression that fills exactly to its spill level;
   - sediment exceeding a depression's volume;
   - very large time steps;
   - no cell uplifting (the tolerance falls back to 10⁻⁶ m);
   - G/p̂ above 1, where convergence is not proven, up to the 50-iteration limit;
   - `slope_adaptive` erosion on a plain, where channel discharge follows the single steepest
     receiver: river lines, fords and lake outlets must still agree.
3. **Lakes:**
   - a lake covering most of the map;
   - a lake touching a drained edge;
   - nested depressions;
   - a terminal lake smaller than one cell;
   - breaching thresholds at exactly the limit.
4. **Roads:**
   - fewer than two settlements;
   - all candidate links blocked by water;
   - a bridge longer than the allowed span;
   - two equal-cost paths (tie-breaking);
   - a road along a drained edge.
5. **Features:**
   - zero settlements, ruins or Chimes;
   - ids when a stage adds no feature;
   - rivers shorter than two vertices;
   - lakes whose outline touches the frame edge.
6. **Determinism:**
   - −0.0 and NaN anywhere in a layer; the sign of zero in priority-queue keys; any `f64::max`
     or `f64::min` outside `kernels::num`;
   - `HashMap` or unordered iteration anywhere near an output;
   - std float methods through a trait (the ban does not see `num_traits::Float`);
   - JSON key order;
   - ZIP headers on Windows;
   - FFT padding for sizes 64, 384 and 1024.
7. **Runner:**
   - cancel at every stage boundary and inside every long loop;
   - a panic in a per-step process;
   - a stage writing an undeclared layer;
   - a recipe whose memory estimate lies just above or below the budget;
   - an output folder that already holds files.
8. **Format:**
   - levels rules;
   - unknown fields at every level of the manifest;
   - a layer whose size or hash does not match;
   - NaN in an f32 layer;
   - a ZIP64 boundary;
   - `requires` naming a capability the reader does not know; an empty `requires`.

## 9. Rulings (Claude's calls)

1. Ruling: the evolution processes are per-step `Stage`s inside `landscape_evolution`, not
   pipeline stages — they must run every time step, and one trait keeps one contract — cost if
   wrong: a second trait later.
2. Ruling: CE-77.1's "every stage" means every pipeline stage; per-step processes are covered by
   the evolution stage's hashes and their physics tests — hashing 240 steps × 7 processes adds
   nothing a test does not — cost if wrong: more hashes.
3. Ruling: n = 1 in P08 — the implicit solutions for multiple flow directions and for deposition
   need it (Yuan et al. 2019; Braun & Willett 2013) — cost if wrong: Newton iterations for n ≠ 1
   later.
4. Ruling: the steepest receiver stays alongside the multiple-flow weights, for the MST resolver,
   river centrelines and `flow_dir` — the resolver needs single paths (Cordonnier et al. 2019) —
   cost if wrong: none.
5. Ruling: subsidence (`sag_rate`) acts during evolution, not after it — subsiding basins should
   fill — cost if wrong: basins look different from 0.3.0's.
6. Ruling: lake balance uses Fu's form of the Budyko curve for runoff and a temperature-based lake
   evaporation — the research's most likely cause of oversized lakes is gross precipitation as
   inflow — cost if wrong: retuning.
7. Ruling: the parity tolerance is the 0.3.0 range widened by the recipe's wanted range, with a
   floor of 0 for lake share — a better generator should not fail for having fewer lakes than
   0.3.0's least-lake seed — cost if wrong: CE-28.5 stricter in a later phase.
8. Ruling: built-in recipes carry no in-world texts; `[renders.inworld]` supplies them — no lore
   ships inside the editor, and labels stay inside the frame — cost if wrong: texts move from
   recipes to code.
9. Ruling: generated settlement names keep 0.3.0's syllable lists as defaults, overridable per
   recipe — they are Jann's and marked "generated, not canon" — cost if wrong: new lists.
10. Ruling: feature types become lower case with hyphens (`gate-up`, `iron-ore`) —
    `docs/map-package.md` §5 — cost if wrong: renames before P22.
11. Ruling: the fixture package of CE-76.1 is text (JSON, GeoJSON, TOML values) plus a recorded
    SHA-256, not a committed ZIP — G10 keeps binaries out of git — cost if wrong: none.
12. Ruling: snapshots are a run option, not a recipe input, and a package with snapshots is not
    canon; P33 builds them (decision 0004) — recipe inputs are empty in P08, and canon packages
    stay minimal — cost if wrong: an input later.
13. Ruling: no threads inside a run in P08 — determinism by construction; seeds already run in
    parallel — cost if wrong: slower 1024² runs.
14. Ruling: `theos-maps` is optimised in the dev profile — parity and golden runs would otherwise
    take about half an hour — cost if wrong: coverage numbers drift; then a separate job.
15. Ruling: a memory estimate before the run (cells × the declared layers' bytes × 1.5) and a budget
    replace the fixed grid limit (Jann, decision 0004) — aborts (out of memory) cannot be caught —
    cost if wrong: the factor 1.5 needs tuning.
16. Ruling: `docs/map-package.md` gains its additions as format 1.0 — no package or reader exists
    yet — cost if wrong: they become 1.1.
17. Ruling: the gate PR proves the cargo-deny ban with a probe crate before relying on it — the
    probe did not test the ban itself — cost if wrong: none.
18. Ruling: lake outlines use own marching squares (about 80 lines) unless the plan's probe clears
    the `contour` crate — its float use and licence are unchecked — cost if wrong: a little code.
19. Ruling: the hillslope stays linear; the critical-slope limiter covers what is visible at 62.5 m
    — nonlinear creep is sub-grid there (research pass 1) — cost if wrong: a nonlinear stage later.
20. Ruling: road curvature limits (heading in the search state) are not in P08 — 16 directions,
    grade along the step and bridges give most of the gain; curvature costs about 16× (Galin 2010)
    — cost if wrong: one more stage parameter later.

21. Ruling: the conversion swaps 0.3.0's `[y, x]` pairs to `[x, y]` — G19 and the format define
    positions that way — cost if wrong: recipes rewritten.
22. Ruling: feature ids are assigned by the runner after the pipeline, and stages use internal
    handles — a later stage that changes a type (a promotion, in P31 or P32) must not rename
    references, and P22's pins bind to the final ids — cost if wrong: none.
23. Ruling: 0.3.0's hidden ruin and extraction placement checks become explicit constraints
    (`features.ruin`, `features.chime`) — CE-28.5 then compares like with like — cost if
    wrong: none.
24. Ruling: metric paths and definitions follow 0.3.0's summary (`elevation_m.relief`,
    `lake_pct`, …), except that the walk between extraction points is `travel.chime_to_chime_min`
    (decision 0004) — the constraints keep their meaning — cost if wrong: renames before P16.
25. Ruling: the runner evaluates constraints after the pipeline; `constraints` is not a stage —
    constraints read the whole summary, not layers — cost if wrong: none.
26. Ruling: the routing order is steepest receivers → MST of basins with an ε-lifted routing
    surface → flow partition on the corrected surface with a fresh stack — fastscapelib's
    documented order; partitioning before the resolver would leave pits out of order — cost if
    wrong: none.
27. Ruling (Jann, decision 0004): erosion uses FastScape's dispersive exponent p = 0.5 + 0.6·S;
    channel discharge in `hydrology` uses the single steepest receiver — rivers, their lines and
    the fords agree by construction and do not spread across plains, and river share is measured
    as 0.3.0 measured it — cost if wrong: a channel exponent later.
28. Ruling: runoff enters the fluvial law once, as the runoff-weighted area Q (the upstream sum of
    p̂ × cell area, p̂ normalised by the map mean) in both terms — Yuan et al.'s eq. 11 with p̃·A,
    generalised to runoff that varies across a catchment; 0.3.0's K table keeps its meaning —
    cost if wrong: K retuned.
29. Ruling: G = 0 switches off deposition and depression filling together — G = 0 is meant to be
    0.3.0's process family — cost if wrong: one more switch.
30. Ruling: potential evaporation is 0.3.0's temperature formula, clamp(300 + 45·T, 200, 1800)
    mm/yr — only mean temperature exists, and it is the lake evaporation 0.3.0 tuned — cost if
    wrong: a better PET model later.
31. Ruling: lakes use the Bracht-Flyr balance with rain on the lake, at one level per maximal
    depression; nested sub-depressions are not merged — simplest form consistent with the paper;
    Fill–Spill–Merge later — cost if wrong: nested lakes look merged.
32. Ruling: shallow pits are filled in `height` by `hydrology` — river beds must not cross dry pits —
    cost if wrong: breaching instead.
33. Ruling: vegetation uses the Thornthwaite Memorial model (Lieth & Box 1972; Lieth 1975) on
    actual evapotranspiration — the water balance the research recommends; the Miami model uses
    precipitation — cost if wrong: one formula.
34. Ruling: river bed height is the terrain height at the vertex; rivers end at lake shores and
    outlets start new lines — the engine places the water surface at bed + depth — cost if wrong:
    a property's meaning before P22.
35. Ruling: maximum road grades are per class (main 0.08, road 0.10, track 0.15, trail 0.25) — ruins
    on peaks and quarries on steep slopes must stay reachable by trails and tracks — cost if wrong:
    recipe parameters.
36. Ruling: unreachable travel cells hold −1 — layers must be finite (§5.4) — cost if wrong: a
    documented sentinel changes.
37. Ruling: `describe` lists valid recipes and reports broken ones as problems instead of failing
    — one broken file must not hide the others — cost if wrong: none.
38. Ruling: `mul_add` joins the clippy ban, but `f64::max`/`min` do not (a helper and reviews
    instead) — `mul_add` is rare and leaves `Cargo.lock`; banning max/min would flag ordinary code
    elsewhere (engineering directive 4) — cost if wrong: a later ban.
39. Ruling: per-stage random streams are keyed by stage id and occurrence number, not position —
    inserting a stage must not shift others — cost if wrong: none.
40. Ruling: timing lives only in the `RunManifest`, never in the package — package bytes must not
    change between runs — cost if wrong: none.
41. Ruling: until P08 merges, the 1.0.0 golden file may be deleted and recorded again on the branch
    — nothing outside the branch depends on it yet; afterwards outputs change only with a new
    version — cost if wrong: none.
42. Ruling: the `Stage` trait's capabilities (parameter-dependent reads and writes, metric names
    before the run, checks and metrics with parameters and a report) are binding; its exact
    signatures are the plan's — the later map phases build on the capabilities — cost if wrong:
    none.

Rulings of decision 0004 (12 Nov 2026), Claude's calls inside Jann's decisions:

43. Ruling: the fluvial tolerance is Yuan et al.'s 10⁻³ × max(U)·Δt, with 10⁻⁶ m when no cell
    uplifts — the paper's rule; 10⁻⁶ m everywhere cost iterations for nothing — cost if wrong: a
    stricter constant.
44. Ruling: the deposition test (14) runs with both flow partitions — Yuan et al. prove the factor
    for single flow only, and the multiple-flow combination is ours — cost if wrong: none.
45. Ruling: the mill rule is `mill_q_factor` × `river_q` with default 2.7 — 0.4 m³/s at 0.3.0's
    `river_q` of 0.15 — cost if wrong: one default.
46. Ruling: `edge_margin` is a share of the frame's shorter side, 0.025 by default and 0.0125 for
    `resources` — 0.3.0's 0.6 km and 0.3 km at 24 km — cost if wrong: two defaults.
47. Ruling: `requires` is always written, as `[]` when empty — one canonical form, so golden
    hashes do not depend on whether a writer omits it — cost if wrong: none.
48. Ruling: the memory estimate counts the declared layers once each, times 1.5 for temporaries,
    and the library's default budget is 3 GiB — the library reads no operating-system values,
    which also suits the engine; P09 sets the machine-aware budget — cost if wrong: the factor
    needs tuning.
49. Ruling: the site type `market` leaves the format with `route_potential` — nothing writes it in
    P08 — cost if wrong: a minor version adds it back.
50. Ruling: feature ids stay assigned after the pipeline although `route_potential` left — P31 or
    P32 may bring promotion back, and the mechanism is already specified — cost if wrong: a little
    unused generality.

## 10. Left to the builder

- The exact parameter values of the built-in recipes after tuning (within §5.5's rules).
  Parameters that 0.3.0 accepted but never read (for example `settlements` on `pois` in
  `hearth-valley-omega`) are dropped in the conversion, since P08 rejects unknown parameters.
- The constants of the hydraulic geometry (a, c), the critical slopes per rock type and the Budyko
  parameter, from the cited papers' ranges.
- The palette and layout of the renders within §5.11.
- Module boundaries inside `stages/` and `kernels/`, as long as each stage is one module.
- The order of tasks in the plan, provided the format crate and the Stage contract come first and
  each process lands with its physics test.

## Appendix A. The 0.3.0 reference per seed

Run on 10 Nov 2026 with `theos_maps` 0.3.0 (Jann's export), Python 3.12, fastscapelib 0.3.0,
numpy 2.5.4 and scipy 1.18.1 through uv, without the engine export (`do_export=False`). The
parity fixture copies these values. The column "Extraction walk" is 0.3.0's
`travel.extraction_to_extraction_min`; P08 reports it as `travel.chime_to_chime_min`. "All
constraints" includes 0.3.0's ruin and extraction placement checks; no seed failed those, so the
failures listed are the recipe's own constraints.

| Recipe | Seed | Relief (m) | Lake share (%) | River share (%) | Vegetation cover (%) | Median precipitation (mm/yr) | Lakes | Extraction walk (min) | All constraints |
|---|---|---|---|---|---|---|---|---|---|
| hearth-valley | 1 | 1281 | 3.63 | 0.93 | 55.1 | 895 | 18 | 96.0 | pass |
| hearth-valley | 2 | 2809 | 10.09 | 1.87 | 56.0 | 914 | 22 | 102.2 | fail (relief, lake share) |
| hearth-valley | 3 | 1630 | 23.04 | 1.26 | 59.2 | 953 | 17 | 133.7 | fail (lake share) |
| hearth-valley | 4 | 2131 | 6.43 | 1.56 | 37.8 | 735 | 14 | 108.9 | pass |
| hearth-valley | 5 | 1754 | 1.46 | 1.51 | 77.3 | 988 | 6 | 121.9 | fail (vegetation cover) |
| hearth-valley | 6 | 1435 | 18.37 | 1.25 | 82.4 | 1091 | 17 | 174.0 | fail (lake share, vegetation cover) |
| hearth-valley | 7 | 1777 | 32.04 | 0.93 | 55.1 | 909 | 17 | 243.6 | fail (lake share) |
| hearth-valley | 8 | 2767 | 16.54 | 1.38 | 40.1 | 804 | 15 | 208.1 | fail (relief, lake share) |
| hearth-valley-omega | 1 | 1243 | 1.38 | 0.88 | 54.8 | 888 | 14 | 100.2 | pass |
| hearth-valley-omega | 2 | 2750 | 10.34 | 1.78 | 55.9 | 920 | 23 | 134.5 | fail (relief) |
| hearth-valley-omega | 3 | 1678 | 23.56 | 1.17 | 58.0 | 949 | 18 | 108.7 | pass |
| hearth-valley-omega | 4 | 2109 | 11.51 | 1.40 | 37.8 | 731 | 13 | 200.7 | pass |
| hearth-valley-omega | 5 | 1762 | 23.87 | 1.45 | 77.5 | 997 | 10 | 108.4 | fail (vegetation cover) |
| hearth-valley-omega | 6 | 1417 | 16.85 | 1.31 | 82.5 | 1104 | 20 | 163.9 | fail (vegetation cover) |
| hearth-valley-omega | 7 | 1810 | 31.14 | 0.97 | 54.8 | 899 | 17 | 435.5 | fail (extraction walk) |
| hearth-valley-omega | 8 | 2782 | 18.28 | 1.36 | 38.4 | 797 | 12 | 412.2 | fail (relief, extraction walk) |
| fixture-small | 1 | 267 | 1.05 | 2.08 | 37.5 | 656 | 1 | 18.8 | no constraints |
| fixture-small | 2 | 284 | 10.82 | 0.27 | 59.8 | 836 | 2 | 23.8 | no constraints |
| fixture-small | 3 | 212 | 11.25 | 0.27 | 36.4 | 683 | 1 | 40.3 | no constraints |
| fixture-small | 4 | 345 | 2.27 | 0.00 | 39.4 | 623 | 1 | 26.9 | no constraints |
| fixture-small | 5 | 992 | 2.91 | 0.34 | 32.0 | 707 | 2 | 23.8 | no constraints |
| fixture-small | 6 | 1327 | 2.37 | 0.29 | 48.3 | 852 | 3 | 34.4 | no constraints |
| fixture-small | 7 | 353 | 0.00 | 2.25 | 32.9 | 679 | 0 | 18.6 | no constraints |
| fixture-small | 8 | 2292 | 6.69 | 1.64 | 52.7 | 839 | 3 | 24.4 | no constraints |
