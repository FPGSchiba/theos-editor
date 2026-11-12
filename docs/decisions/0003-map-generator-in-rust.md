# 0003 · Map generator in Rust, baked into packages

Jann decided this in a brainstorm on 10 Nov 2026; this record writes it down for approval. It
decides how maps are made: a Rust library in this repo that the editor runs in-process and the
engine can depend on later, replacing the Python toolset `theos_maps`. Canon floors are baked into
packages of a new format (`docs/map-package.md`), and the library is deterministic on every
platform. The same PR turns these decisions into umbrella spec version 3 (§10, §11).

## Status

accepted · 2026-11-10 · merged in #24 (after the gate PR #23). Amended by
[decision 0004](0004-map-generation-research.md) on 2026-11-12: §7 (no size limit for levels), the
phases of §10 (P29, P30), and the Horizon Zero Dawn quotation of §2.

Sources:

- the Python toolset `theos_maps` 0.3.0, about 3,000 lines, which Jann exported on 2026-11-10 from
  a skill in another Claude account. It was read from a local copy and is in no repository;
- two research passes of 2026-11-10 (§2, §3, §5), with the sources listed at the end;
- probes on the pinned toolchain (rustc 1.99.1, Windows 11) for the claims marked "measured".

## Goal

Jann's words (2026-11-10):

- "I would like to build a rust version of it."
- "I want to fully integrate it into the Engine later on. So its fine if we cannot reproduce
  maps."
- Why a binary at all? "Can we not build a generator implementation using the library? So its all
  in runtime, maybe on a different thread though."
- Floors: "It should stay the same" for every player, every time they enter it.
- Indoor floors: "Probably yes, so we will need stacked levels."
- "Lets leave the phase splitted. I think its better to have focused phases."
- On running generators from the existing CLI, as `theos check` runs the lore checks: "Lets move it
  to P09. I think its a good addition to already see some things in action and test it end to
  end."

**Non-goals:** reproducing maps made with `theos_maps` 0.3.0 bit for bit; generating canon floors
while the game runs; erosion on the GPU; exporting pins, spawns and quests to the engine now
(umbrella §5); porting the blockout generator (P23) to Rust.

**Anti-goals** (a failure even if everything else works):

- a generator change that silently moves a canon floor under its pins and quests;
- pins or other authored content inside a generated binary;
- GPL obligations on the editor or the game;
- a package the engine cannot read without pulling in the generator.

## Recommendation in short

| Question | Decision | § |
|---|---|---|
| Who makes maps | A Rust library, `theos-maps`, in this workspace; the Python toolset retires | 1 |
| How the editor runs it | In-process, on worker threads, through a third adapter `builtin` behind the existing `Generator` trait; no binary | 1, 9 |
| From the shell | `theos gen list \| describe \| run` in the existing CLI (P09, CE-30.4): runs into a folder with a contact sheet, never writes records | 9 |
| Bake or generate in the game | Bake. Every floor is one canon terrain, the same for every player; the game ships the package of the picked version. Runtime generation stays an option only for what varies inside a fixed floor | 2 |
| Determinism | The same recipe and seed give the same bytes on Windows, Linux and macOS (x64 and arm64), new global constraint G20 | 3 |
| Erosion | Implemented from the papers; fastscapelib is GPL-3.0 and is neither linked nor translated | 4 |
| Package format | `theos-map` 1.0 in `docs/map-package.md`: one ZIP file with a manifest, GeoJSON features and raw layers; read and written by its own small crate, `theos-map-format` | 6 |
| Generated and authored content | The package holds only what the generator made; pins, spawns and quests stay in records and are exported beside it later | 6 |
| Indoor floors | Stacked levels are part of format 1.0, of the frame and of pins; the generator for them is its own phase | 7 |
| Phases | P08 format and terrain; P29 settlements, interiors and child maps; P30 indoor floors with stacked levels; P09, P22 and P23 adjusted | 10 |

## 1. Who makes maps, and how the editor runs them

| | (A) Python process, as planned | (B) Rust library + protocol-1 binary | (C) Rust library in-process, as an adapter of the generator layer |
|---|---|---|---|
| G11, fake-generator ACs (CE-29.7, CE-45.4, CE-62.2) | Kept | Kept | Kept: the adapter implements the same `Generator` trait, so the UI and the tests stay generic |
| G1 (no engine dependency) | Kept | Kept; the engine may depend on the library | Kept; the engine may depend on the library |
| G4 (crate list) | Unchanged | Two new crates plus a binary | Two new crates |
| Determinism across OSes | numpy and fastscapelib, hard to control; CE-28.4 allowed a documented tolerance | Ours to control (§3) | Ours to control (§3) |
| macOS | Depends on wheels (CE-28.6) | Pure Rust | Pure Rust |
| Engine reuse | None: the engine would rewrite it | The library | The library |
| Install | uv, Python 3.12, venv on first use (CE-29.4) | A second binary to ship and find | Nothing: it is part of the editor, CLI and MCP server |
| Cancel | Kill the process tree | Kill the process tree | A cancel token checked between stages and inside long loops |
| A crash | Kills only the generator | Kills only the generator | A panic stays on its worker thread and becomes a failed run; an abort (out of memory) would take the editor down, unlikely at these sizes (§5) |
| Licence | fastscapelib GPL-3.0 in a separate program | Free of GPL | Free of GPL |

Jann chose (C), against my first proposal (B): the library already is the generator, so a binary
would only add a second program to ship. The process adapter stays for other generators (the
blockout generator of P23, perhaps the engine). Skills and scripts get the same runs from the shell
through a `theos gen` subcommand of the existing CLI (P09, CE-30.4; §9). Running maps no longer
needs uv or Python; the blockout generator still does.

## 2. Bake once, or generate while the game runs

**How shipped games do it.**

- **Baked at development time.** Ghost Recon Wildlands simulated its terrain and erosion offline
  ("Nothing is random… there's no 'magic' seeds"). Far Cry 5 generated biomes, rivers and cliffs
  inside the editor. Enshrouded is hand-made because its studio's earlier procedural world "didn't
  really feel like a place". Hunt: Showdown has fixed maps and varies only spawns, clues and
  extraction points from predefined sets.
- **Generated while the game runs.** Minecraft, No Man's Sky and Deep Rock Galactic. Even there,
  authored content is placed by rules (template pools, designer-built rooms), never pinned by hand.
- **Hybrid.** Horizon Zero Dawn bakes its world into 2D maps (about 4 MB/km²) and places detail at
  runtime; its slides name two separate goals for that placement, "deterministic" and "locally
  stable". Diablo IV has a hand-made world and random dungeon layouts.

**What breaks when a generator changes under existing worlds:**

- Minecraft 1.18 blends old chunks into new ones, and player buildings in them may come out
  buried or deteriorated.
- No Man's Sky and Valheim kept existing planets and explored land, and put new terrain only
  where nobody had been.
- 7 Days to Die made existing worlds incompatible.

I found no shipped game that runs stream-power erosion on a player's machine; Dwarf Fortress runs
erosion once, when the world is created.

**Decision.** Jann: a floor stays the same for every player, every time they enter it. So:

1. **Canon floors are baked.** The package of the picked version is what pins, quests and spawns
   bind to, and what the game ships. The engine reads packages and never re-runs a canon floor. A
   floor is 1–3 MB at today's 384² grid (about 10–40 MB at 1024² with every layer), so size is no
   reason to regenerate.
2. **"Rebuild at a new scale"** (Jann, 5 Nov 2026, #19) means refining the baked map, not running
   the recipe again at a finer grid: a finer grid gives different rivers and towns, and pins bound
   to features would break. The engine upsamples the canon map and adds detail on top, in a
   deterministic and locally stable way, as Horizon Zero Dawn does. That refinement is engine work
   (or a later stage of the library); this decision only keeps it possible.
3. **Generation while the game runs** stays an option for what varies inside a fixed floor:
   dungeons, encounter slots, spawns from predefined sets. Interiors are small graphs and grids;
   they take milliseconds.

## 3. Determinism

Determinism is what makes "regenerate and compare" work. A lost package or `features` file can be
rebuilt from its recipe and seed and checked against its hash (renders are pictures and are simply
drawn again), and later the engine can generate interiors or detail from a seed without shipping
them. Building it in from the first stage is cheap; adding it later means a rewrite. The rules,
each verified by the second research pass:

1. **Math functions through `libm`.** `f64::sin`, `cos`, `tan`, `exp`, `ln`, `log*`, `powf`,
   `powi`, `hypot`, `atan2` and their relatives call the platform's math library; the std docs
   call their precision "non-deterministic … varies by platform". Measured on Windows: std and
   `libm` differ in 1.6 % (sin) to 17.7 % (atan2) of a million inputs. The pure-Rust `libm` crate
   (rust-lang, MIT) gives the same bits everywhere and is pinned through `Cargo.lock`. Clippy's
   `disallowed-methods` rejects the std versions (verified on rustc 1.99.1). Because the gate
   allows only `gate/clippy.toml`, the ban covers the whole workspace (§12).
2. **Plain arithmetic is safe.** `+ - * /`, `sqrt` and `mul_add` are exact IEEE-754 operations,
   and rustc never fuses or reorders float operations (RFC 3514). No `fadd_fast`, no `algebraic_*`.
3. **FFT without runtime SIMD choices.** `rustfft`'s planner picks AVX, SSE or NEON code at
   runtime; measured, the SIMD and scalar paths differ in about 99 % of outputs. Even its scalar
   path computes its twiddle factors with std `cos`/`sin` (measured: 129 of 4,096 differ from
   `libm`). P08 uses a scalar FFT with twiddles from `libm`. Whether that is a patched `rustfft` or
   a small FFT of our own is P08's call, after a probe. The scalar FFT is 10–20 % slower.
4. **Random numbers.** Use `rand_pcg` or `rand_chacha`, which promise portable output and are
   tested against reference vectors. Never `StdRng` or `SmallRng`, whose algorithm may change.
   Samplers (uniform, normal) are our own and use `libm`, because `rand_distr` switches to std math
   when any crate in the build enables `num-traits/std`. Each stage gets its own sub-seed from
   (seed, stage name) through a fixed mixer, so adding a stage changes no other stage, as in
   0.3.0.
5. **Order.** No `HashMap` iteration order reaches an output. Sorts and priority queues compare
   with `total_cmp` and break ties by cell index (Barnes et al. 2014).
6. **Parallelism only where order cannot matter:** per cell, per row (D8 routing, the rows of the
   ADI diffusion), or per seed. Sums over many cells are taken in fixed chunks combined in index
   order. Drainage area is counted in whole cells, which is exact.
7. **The floating-point environment.** A host that sets flush-to-zero or denormals-are-zero
   changes results, and new threads inherit the setting from the thread that starts them; safe
   Rust cannot reset it (G3). The editor never sets it. An engine that calls the library must not
   either, or must call it from a thread with the default environment; the golden hashes, run in
   the engine's CI too, would show the difference.
8. **Proof.** Golden hashes of every layer after every stage, of `features` and of the package
   bytes, for fixed recipes and seeds. They run on Linux in every PR, and on Windows and macOS (x64
   and arm64) in the weekly `platforms` run and before releases (G2). Today `platforms` runs only
   on Apple silicon, so the gate PR before P08 adds the Intel target for these tests (§12). The
   renders are not hashed across platforms: the drawing crate has per-target SIMD and promises
   nothing, and pictures for people need not be bit-identical (G20 covers the package, the layers
   and `features`).

## 4. Parts and crates (engineering directive 2)

| Part | Crate or tool | Why |
|---|---|---|
| Recipes | `toml` and `serde` | Recipes become TOML like the project's other settings, workflows and pipelines; the 0.3.0 recipes are converted |
| Stage pipeline (needs and provides, sub-seeds) | own code, about 150 lines in 0.3.0 | It is the generator's core; no crate fits |
| Raster arrays | `ndarray` or plain `Vec` plus index helpers | P08's probe decides |
| Math functions | `libm` | §3 |
| FFT (spectral noise, orographic rain) | scalar FFT with `libm` twiddles | §3 |
| Random numbers | `rand_pcg` or `rand_chacha` plus our own samplers | §3 |
| Nearest neighbour (provinces, distances to curves) | `kiddo` (MIT/Apache, active) or `rstar` | Maintained, exact |
| Delaunay (road network) | `spade` (exact predicates) | Maintained; `delaunator` is the faster fallback |
| Graphs (spanning tree, Dijkstra, interior topology) | `petgraph`; `pathfinding` for grid paths | Maintained |
| Labelling, morphology, distance transforms | `imageproc` (MIT), with its rayon feature off | Avoids hand-written raster algorithms |
| Stream-power erosion, priority-flood, D8 routing, MST sink resolution, ADI diffusion | **own code, from the papers**: Braun & Willett 2013, Cordonnier, Bovy & Braun 2019, Barnes et al. 2014 | No mature crate exists (oxscape 0.1 has a few hundred downloads; fastlem is stale and MPL-2.0). fastscapelib and richdem are GPL-3.0, so their source is neither linked nor translated. The probe's core loop was about 200 lines |
| Renders | `tiny-skia` (paths, anti-aliasing), `image`/`png`; a font for labels (P08 chooses one with a licence that allows shipping) | Replaces matplotlib |
| GeoJSON | `geojson` | Maintained, matches RFC 7946 |
| ZIP | `zip`, with a deflate implementation that no cargo feature of another crate can swap (`docs/map-package.md` §1) | §6 |

P08's phase spec repeats the probes for the crates it picks, on the pinned toolchain.

## 5. Cost

Erosion with priority-flood, D8 routing, stream power and ADI diffusion, 240 steps. Measured on one
core in f64; the column with the near-linear MST sink resolver (Cordonnier et al. 2019) is an
estimate.

| Grid | One layer f32 | Working set | Measured (with naive priority-flood each step) | Estimate with MST resolver, 1 core | 8 cores |
|---|---|---|---|---|---|
| 384² | 0.59 MB | ~10 MB | 3–5 s | 2–4 s | ≤ 1 s |
| 1024² | 4.2 MB | ~72 MB | 26–67 s | 10–30 s | 3–8 s |
| 2048² | 16.8 MB | ~290 MB | 140–370 s | 45–125 s | 12–35 s |
| 4096² | 67 MB | ~1.2 GB | 16–33 min | 3.5–9 min | 1–2.5 min |

At today's grid a run takes seconds in the editor, and four seeds run side by side. From 2048²
up, a run belongs in the editor, never behind a loading screen.

## 6. The package and the two layers of a floor

`docs/map-package.md` defines format `theos-map` 1.0:

- one ZIP file per map version, `.theosmap`, with `manifest.json`, `features.geojson` and raw
  little-endian layers;
- the frame, the coordinates (x east, y south, heights above the base level, values at cell
  centres; (x, h, y) for a Y-up engine), the layer index with a SHA-256 per layer, and the
  standard layers and feature types;
- water as shapes: lakes as polygons with their surface level, rivers as lines with width,
  discharge and bed height (0.1 had only masks, so an engine or the blockout could not place a
  water surface);
- rooms as polygons, stacked levels (§7), and reading rules: an unknown major version is refused,
  unknown fields are ignored;
- reproducible bytes (0.1 wrote a timestamp);
- a frame-exact `base` render for the Atlas (0.1's renders have titles, legends and margins, so
  pins would sit in the wrong place).

Jann accepted this model on 2026-11-10. The data model's `package` output pointed at Confluence
Decision 221741058 for the format. Jann confirmed that the decision fixes nothing that differs
from this, so `docs/map-package.md` replaces it. A private session updates the Confluence copy
(`docs/status.md`).

**Two layers of a floor.**

| | Generated layer | Authored layer |
|---|---|---|
| What | Terrain, water, surfaces, places with ids, roads, buildings, rooms | Pins, spawns, the places of quest steps, cameras |
| Source | Generator, recipe and seed | Records in git (`## Pins`, `## Cameras`, spawns, quests) |
| Changes | When another version is picked | Whenever someone writes |
| Lives in | The `.theosmap` in the binaries folder | git, through the command layer and review |

The game loads both. The authored layer goes to the engine as GeoJSON in the same frame, pointing
at the package's feature ids (`docs/map-package.md` §9). That export stays in umbrella §5
("Later"); the format fixes its direction now, so it fits when it comes.

**Two crates.** The engine needs to read packages, not to generate them.

- `theos-map-format` holds the types, the reader and the writer, and depends on no other crate of
  this workspace.
- `theos-maps` is the generator. It depends on `theos-map-format` and on the generation crates of
  §4.

## 7. Stacked levels

Jann expects indoor floors with storeys on top of each other (2026-11-10). Every level shares the
frame's grid. On 2026-11-12 (decision 0004) Jann decided that the format sets no size limit:
indoor maps are expected to be considerably smaller than outdoor maps, and the generator's memory
budget is what limits a run. Per-level grids or tiles, if a phase needs them, come as a format
capability named in `requires`, a minor version that older readers refuse cleanly
(`docs/map-package.md` §6).

- **Format 1.0** carries them from the start. `frame.levels` lists the levels with their
  elevations. Layers and features carry `level`. Stairs, ladders, lifts and drops are features
  that connect two levels; a drop goes one way only.
- **Protocol and data model**, both additive.
  - The run manifest's `frame` gets an optional `levels` list with each level's id and elevation.
  - The version record's `frame` gets `levels` as a one-line list of level ids, bottom to top.
    The restricted YAML has no lists of maps (data model §4), and the editor needs only the ids;
    the elevations stay in the package.
  - Pins get an optional `level <id>`, required on a map whose frame lists levels.
  - A map with levels has one `base-<level id>` render per level.
- **Phases.**
  - P22's pin grammar and findings include `level` from the start (CE-61.2 gains the finding).
  - P30 builds the generator for indoor floors with stacked levels, and P45 the Atlas's level
    switcher (decision 0004).
  - Until P30, interiors (P34) have one level.

## 8. Versions

- **The generator reports its own version** in `describe` (`theos-maps@1.0.0` first). It is not
  the product version: under G17 every crate carries the workspace version, which changes with
  every editor release. A map version records `generator: name@version` (data model §8), and the
  version must mean "same package and `features`".
- **The golden hashes enforce it (CE-77.2).** Golden hashes are kept in one file per generator
  version. The test compares a run with the file of the current version, so a change of outputs
  fails until the version is raised. The tool that records golden hashes writes only the file of a
  version that has none yet. Editing an old version's file by hand is the kind of deliberate step
  any diff shows, so it is left to the reviews (engineering directive 4). A check against `main`
  in xtask would be stricter, but it is gate work that nothing else needs.
- **Format version** `MAJOR.MINOR` in the manifest. A new major is a breaking change of the
  product (CONTRIBUTING §5); a new generator version is not.

## 9. The editor side

- **Adapter `builtin`** in `theos-studio`, implementing the `Generator` trait (P09).
  - `describe` comes from the library.
  - `run` runs one seed on a worker thread of its own, sends the same progress events as the
    process protocol over a channel, and returns the manifest as a typed value. Inside a run the
    library may use more threads where order cannot matter (§3, rule 6).
  - Cancel sets a token that the library checks between stages and inside long loops. Partial
    outputs are discarded.
  - A panic ends as a failed run with its message.
- **Registration.** The Maps part registers the built-in generator in its part defaults (G14), so
  a project needs no entry. A project adds recipes in `tools/maps/recipes/` (or a folder its
  `.theos/settings.toml` names); they are added to the built-in recipes and win over a built-in of
  the same id. `docs/generators.md` §2 shows the entry.
- **Built-in recipes.** P08 converts the terrain recipes of 0.3.0 (`hearth-valley`,
  `hearth-valley-omega`, `fixture-small`) and ships them, so maps work in slice S4 before the
  content project has recipes of its own. P29 adds the settlement and interior recipes
  (`choir-hall-ruin`).
- **Recipe inputs.** As in 0.3.0, a recipe holds all its parameters, so the terrain recipes of P08
  take no inputs and a run varies only by its seed. Inputs that set stage parameters can be added
  later; they would be additions to the recipe schema, which P08's spec fixes and P23's recipes
  follow.
- **Outputs.** Every terrain run writes the standard set (`base`, `truth`, `inworld`, `geology`,
  `resources`, `features`, `package`). A recipe's `renders` adds extra renders, such as `omega`.
- **Child maps** (P29) are runs with `from` set to the parent version's package and an input of
  type `feature` naming the settlement or ruin (`docs/generators.md` §3). Their seed is the
  request's seed, as for every run, so several seeds give several candidates. 0.3.0 derived the
  seed from the parent's seed and the feature, which gave one child per place.
- **Extensions.** The drop-in `.py` stages of 0.3.0 become stages compiled into the library: one
  module plus one registration line each (engineering directive 1). The `omega_detuning` extension
  becomes a normal stage.
- **MCP** runs maps through the same layer (`gen_run`, `gen_pick`, CE-30.1–30.3); agents' picks
  land as review entries.
- **CLI** (P09, CE-30.4): `theos gen list`, `theos gen describe <generator>` and
  `theos gen run <generator> <recipe> --seeds … --out <dir> [--recipe-file <path>]`, over the same
  generator layer. It brings back 0.3.0's loop for working on recipes: change a recipe, run four
  seeds, look at the contact sheet, read the results. A skill calls it like `theos check`, and
  scripts and CI use it for batch runs and hash checks.
  - `run` writes each seed's outputs and results and a contact sheet to the folder it is given, and
    never writes records or the binaries folder. G7 binds MCP, but an agent in a shell that calls
    `theos` is not covered by it, so the CLI changes no content. Keeping a run stays in the editor
    (people) and `gen_pick` (agents, as a review entry).
  - `--recipe-file` runs a recipe that is not saved in the project yet.
  - Until P09, P08 runs its recipes for the parity tests and the contact sheet through a
    developer example inside the crate (`cargo run -p theos-maps --example …`), which ships
    nothing and adds no crate.

## 10. Phases and documents

**Phases.**

| Phase | Repo | Slice | Depends on | Scope |
|---|---|---|---|---|
| **P08** Map format and terrain generator (rewritten) | theos-editor | S4 | P00 | `theos-map-format` (format 1.0, levels included); `theos-maps` (stage core, determinism, every terrain stage of 0.3.0 including omega, terrain renders and `base`, frame and features, the library API with progress and cancel, built-in terrain recipes) |
| **P09** Generator layer and generator tools (adjusted) | theos-editor | S4 | P07, P08 | adds the `builtin` adapter; CE-29.4 now covers it; CE-29.5 is tested with a fixture process generator; `theos gen` in the CLI (CE-30.4) |
| **P22** Map pins, cameras and frames (adjusted) | theos-editor | S8 | P09 | pins with `level`; CE-61.2 gains the level finding |
| **P23** Map recipes and blockout generator (renamed) | tower-of-theos | S8 | P08, P29 | CE-63 now puts the project's map recipes into `tools/maps/recipes/`; the old CE-63 (frame and features) moved into P08 and P29; the blockout reads `.theosmap` 1.0 |
| **P29** Settlements, interiors and child maps (new) | theos-editor | S8 | P08 | the rest of the 0.3.0 port, one level |
| **P30** Indoor floors with stacked levels (new) | theos-editor | S10 | P25, P29 | the generator for indoor floors with stacked levels, and the Atlas's level switcher |

P29 is in S8 because the Atlas drills down into settlement and interior maps (CE-67.1), and P23's
blockout builds from them (CE-64.2). P30 is a new slice S10 after 1.0, because Jann expects
stacked levels but nothing earlier needs them.

Decision 0004 (2026-11-12) amends this table:
- P29 becomes "Settlements and child maps" at the state of the art, and the new P34 "Interiors"
  takes the interior half;
- P30 becomes "Levels in the generator" (after P34), and the new P45 "Atlas level switcher" (after
  P25 and P30) takes CE-79.3.

**Documents this PR changes:**

| Document | Change |
|---|---|
| `docs/specs/00-umbrella.md` | Version 3: G1, G4, G11, new G20; the phase table; P08 rewritten (CE-28, CE-76, CE-77); CE-29.1, CE-29.3, CE-29.4, CE-29.5 and CE-29.7; new CE-30.4; CE-61.2 and CE-62.2; P23 renamed, CE-63 replaced, CE-64.2; P29 (CE-78) and P30 (CE-79); slices S8 and S10; the order of phases |
| `docs/map-package.md` | New: format 1.0 |
| `docs/architecture.md` | Repos, tree, crates, §11 generators, §14 path to the engine |
| `docs/generators.md` | The `builtin` adapter, registration, the examples, `levels` in the frame, `base`, inputs of type `feature`, the keys of constraint results, conformance and versions |
| `docs/specs/data-model.md` | §8: `levels` in the frame, `base` as the second role the editor reads, the package's format; §9.6: pin `level`; §14: the example's generator name |
| `CONTRIBUTING.md` | §5: the generator's own version and the format version replace the `theos_maps` package version; milestone of S10 |
| `README.md` | Which repo holds the map generator |
| `CLAUDE.md`, `.claude/skills/theos-phase/SKILL.md` | Which phases work in tower-of-theos: those its repo column names |
| `docs/decisions/README.md`, `docs/status.md` | The log line; the phase rows, Needs Jann, Needs a private session |

**Not changed here:**

- The Maps screen reference `docs/ui/screens/11-maps.html` still says "theos_maps 0.3.0 · Python
  3.12 (uv)". P16's spec PR updates it, together with the screens it adds (G18).
- `CLAUDE.md` and the theos-phase skill named P01 and P08 as the tower-of-theos phases. This PR
  aligns both with the umbrella's repo column, so a session following the skill builds P08 here.

## 11. Acceptance criteria

This PR writes them into `docs/specs/00-umbrella.md`. Where each one comes from:

| Criteria | Phase | Design |
|---|---|---|
| CE-76.1 to CE-76.4 Map package format | P08 | §6, `docs/map-package.md` |
| CE-28.1 to CE-28.7 theos-maps terrain generator | P08 | §1, §4, §9 |
| CE-77.1, CE-77.2 Determinism | P08 | §3, §8 |
| CE-29.1, CE-29.3, CE-29.4, CE-29.5, CE-29.7 (changed) | P09 | §1, §9 |
| CE-30.4 `theos gen` (new) | P09 | §9 |
| CE-61.2, CE-62.2 (changed) | P22 | §7 |
| CE-63.1, CE-63.2 (replaced), CE-64.2 (changed) | P23 | §9, §6 |
| CE-78.1 to CE-78.4 Settlements, interiors and child maps | P29 | §6, §10 |
| CE-79.1 to CE-79.4 Indoor floors with stacked levels | P30 | §7 |

The old CE-28.1 to CE-28.6 (the Python package, uv, `python -m theos_maps.run`, macOS wheels) are
replaced, not carried over; no test was tagged with them.

**How we know the port is right.** Bit-identical output is not the bar, so:

- CE-28.5 compares behaviour. For each built-in recipe over seeds 1–8, the constraint checks pass
  at least as often as with 0.3.0, and the medians of the summary values (relief, lake share,
  river share, vegetation cover, precipitation) stay within tolerances that P08's spec sets. The
  0.3.0 reference numbers are run once and kept as fixtures, with the Python version noted.
- Jann judges the look from a contact sheet in the PR. As with P28's proof sheets, that is a
  review item, not an AC (G15).

## 12. What the gate needs before P08's build

Gate files fix things P08 changes, and the build cannot change them (CLAUDE.md, rule 3). One gate
PR before P08's build:

- adds `theos-map-format` and `theos-maps` to the expected members in `xtask/tests/workspace.rs`,
  with the two crates themselves, empty and documented, as P00 did for the others;
- classifies both crates as `library` in `gate/thresholds.toml` (floor 80 %);
- adds the std float methods of §3 to `disallowed-methods` in `gate/clippy.toml` (and its root
  copy), with the reason "not the same bits on every platform; use libm (G20)";
- makes the `platforms` workflow run the tests for the Intel macOS target too (CE-77.1): on an
  Intel runner while GitHub offers one, otherwise built for `x86_64-apple-darwin` and run under
  Rosetta on the Apple-silicon runner;
- if P08's spec measures that the parity runs (CE-28.5) are too slow in the unoptimised test
  profile, an optimised test profile for `theos-maps` or a separate job for those tests.

P08's phase spec lists the exact contents; the PR waits for Jann.

## 13. Risks

| Risk | Mitigation |
|---|---|
| The port looks different from 0.3.0 in ways the statistics miss | Contact sheet for Jann in each generator PR; recipes with constraints; Python reference numbers as fixtures |
| A dependency quietly uses std math or runtime SIMD (rustfft, `rand_distr`) | The workspace-wide clippy ban covers our code; dependencies are caught by the golden hashes on four platforms; the crate choices in §4 avoid the known cases |
| Golden hashes differ on a platform only the weekly run sees | The weekly `platforms` run and the release run cover Windows and macOS (G2); a red run blocks the release and is fixed before the next phase claims done |
| Implementing erosion from papers takes longer than porting | The probe's core loop was about 200 lines; the papers are precise; P08 is terrain only |
| A floor with stacked levels needs more than the format foresees | Additions are minor versions (`docs/map-package.md` §6); P30's spec can add what it needs |
| The engine wants a different runtime format | The package is the source format; the engine's asset pipeline converts it, and reads it with `theos-map-format` |
| A panic in a run takes down the editor | Runs are on their own threads and panics end as failed runs; aborts (out of memory) are not caught, so P08 refuses a run above its memory budget (decision 0004) |

## 14. If rejected

Jann names what to change: (A) or (B) instead of the in-process adapter (§1), generating canon
floors in the game (§2), the package format, the crate split, or the phase plan. This record, the
format and the umbrella changes are rewritten in the same PR. Gate PR #23 is harmless on its own.

## Research sources

Industry practice (§2):

- Ghost Recon Wildlands:
  [80.lv](https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/),
  [GDC](https://gdcvault.com/play/1024029/)
- Far Cry 5: [GDC](https://gdcvault.com/play/1025557/Procedural-World-Generation-of-Far)
- Enshrouded:
  [Wccftech](https://wccftech.com/enshrouded-qa-keen-games-explains-all-about-its-zelda-meets-valheim-game/)
- Hunt: Showdown: [wiki](https://huntshowdown.wiki.gg/wiki/Maps)
- Minecraft: [chunks](https://minecraft.wiki/w/Chunk),
  [jigsaw structures](https://learn.microsoft.com/en-us/minecraft/creator/documents/structures/introductiontojigsawstructures),
  [1.18](https://minecraft.wiki/w/Java_Edition_1.18)
- No Man's Sky:
  [GDC 2017](https://gamedeveloper.com/programming/video-how-continuous-world-generation-works-in-i-no-man-s-sky-i-),
  [Origins](https://www.gematsu.com/2020/09/no-mans-sky-origins-update-now-available)
- Deep Rock Galactic:
  [dev blog](https://devtrackers.gg/deep-rock-galactic/p/8cd5ca9c-below-decks-at-ghost-ship-cave-generation-in-deep-rock-galactic)
- Dwarf Fortress: [Wikipedia](https://en.wikipedia.org/wiki/Dwarf_Fortress)
- Horizon Zero Dawn:
  [slides](https://www.slideshare.net/slideshow/gpubased-procedural-placement-in-horizon-zero-dawn/72897079)
- Diablo IV:
  [Gematsu](https://www.gematsu.com/2022/03/diablo-iv-quarterly-update-details-environmental-art)
- Valheim:
  [Xbox Wire](https://news.xbox.com/en-us/2022/11/22/the-mist-stirs-join-the-valheim-public-test-to-preview-of-the-latest-biome/)
- 7 Days to Die: [Steam](https://steamcommunity.com/app/251570/discussions/0/3196989600441512302)
- Factorio: [Alt-F4](https://alt-f4.blog/ALTF4-26/)

Determinism and cost (§3, §5):

- Rust: [f64 docs](https://doc.rust-lang.org/std/primitive.f64.html),
  [RFC 3514](https://rust-lang.github.io/rfcs/3514-float-semantics.html)
- Box2D: [determinism](https://box2d.org/posts/2024/08/determinism/)
- rustfft: [docs](https://docs.rs/rustfft/latest/rustfft/),
  [twiddles](https://github.com/ejmahler/RustFFT/blob/master/src/twiddles.rs)
- rayon: [reductions](https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html)
- rand: [reproducibility](https://rust-random.github.io/book/crate-reprod.html)
- Rapier: [determinism](https://rapier.rs/docs/user_guides/rust/determinism/)
- Barnes et al. 2014: [Priority-Flood](https://arxiv.org/abs/1511.04463)
- Barnes 2019: [parallel stream power](https://arxiv.org/abs/1803.02977)
- Cordonnier, Bovy & Braun 2019:
  [sink routing](https://esurf.copernicus.org/articles/7/549/2019/)
- FastFlow: [CGF 2024](https://diglib.eg.org/handle/10.1111/cgf15243)
