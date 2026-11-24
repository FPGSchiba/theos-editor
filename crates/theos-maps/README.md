# theos-maps

Library. The deterministic terrain generator for maps, built on `theos-map-format`. A run takes a
recipe (a TOML list of stages with parameters), a seed and inputs, writes a map package, the
standard renders and the features file, and returns the run manifest (`RunManifest`); the
developer example `run` also saves that manifest as `manifest.json`. The same inputs give the
same bytes on every platform.

Every step of generation is a `Stage` (`stage`): it declares the world layers it reads and writes
for its parameters, runs in a `StageContext` that hands out only those layers, and is one line in
`stages::register`. Other parts:

- `api`: `run`, `describe`, `RunRequest`, `RunManifest`, `GENERATOR`, `VERSION`.
- `recipe` and `recipes/`: recipe parsing and the built-in recipes `hearth-valley`,
  `hearth-valley-omega` and `fixture-small`.
- `constraints`, `metrics`: what a recipe demands of a result and the parity metrics.
- `kernels`: the numeric kernels (FFT, flooding, distance transforms, noise and more), with
  `kernels::num` as the float and cast helpers.
- `grid`, `rng`, `world`, `features`, `render`, `progress`, `golden`.

## The developer example

```text
cargo run -p theos-maps --release --example run -- <recipe>... --seeds 1..8 --out <dir> [--recipes <dir>]
```

For each recipe and seed it writes the outputs and `manifest.json` to `<out>/<recipe>/s<seed>/`
and a contact sheet to `<out>/<recipe>/contact-sheet.png`. On stdout it prints a Markdown table of
the parity metrics per seed, their medians and how many seeds pass the recipe's constraints.
`--seeds` includes both ends, `--jobs` sets how many runs go at once, and `--recipes` adds a
folder of recipes that win over the built-in ones by id.

## Golden hashes and `bless`

`tests/fixtures/golden/<VERSION>.toml` holds the hashes of every stage and output of the golden
runs (`golden::GOLDEN_RUNS`). The test `maps::golden` compares a fresh run with it.

```text
cargo run -p theos-maps --example bless
```

records the file for the current `VERSION` (in `src/api.rs`). It refuses when the file exists:
hashes of a released version are never overwritten, so a change of any output needs a new
`VERSION`. Before P08 merges, the 1.0.0 file may be deleted and recorded again (spec ruling 41).

## Determinism rules for contributors (G20)

Clippy enforces most of these (`-D warnings`).

- Math functions go through `libm`; the std float methods (`sin`, `exp`, `powf`, ...) are denied.
  Exact operations (`+ - * /`, `sqrt`, `abs`, `floor`, `round`, `ceil`, `trunc`, `rem_euclid`)
  are fine.
- Maxima and minima of floats go through `kernels::num::{max, min}`, never `f64::max`/`min`, and
  sort or queue keys go through `kernels::num::key`.
- `as` casts between integers and floats live only in `kernels::num`; call its helpers.
- No `HashMap` or `HashSet`; use `BTreeMap` and `Vec`.
- Sorts and priority queues use `total_cmp` and break ties by cell index, then by feature handle.
- No threads inside a run; seed and hash arithmetic uses `wrapping_*`.
- JSON that is hashed or compared goes through the canonical writer of `theos-map-format`.
- Callers must not set flush-to-zero or change the floating-point environment of the thread.

## Tests

Units sit next to the code. Behaviour tests are the integration crate `tests/maps/`. Tests at map
scale (a full 384 by 384 run, parity, golden runs) are named `slow_...`: the default and `ci`
nextest profiles leave them out, and

```text
cargo nextest run -p theos-maps --locked --profile slow
```

runs only them. Every test that proves an AC carries `// AC: <id>` on the line above `#[test]`.

## How to add a stage

1. Add a module under `src/stages/terrain/` that implements `TypedStage`: `ID`, `SUMMARY`,
   `KINDS`, a `Params` type (`Deserialize` and `Serialize`; a field `validate` fills is
   `#[serde(skip)]`), `reads`, `writes`, `run` and, when it reports metrics, `metric_names`. Check
   the parameters' domains in `validate` with `stage::Check` (one wording per rule); every number
   of the parameters also keeps, by its type, the bounds `stage::MAX_MAGNITUDE` and
   `MIN_MAGNITUDE` (a float field) or `MAX_COUNT` (an integer field), and the frame's cell edge
   lies between `recipe::MIN_CELL_M` and `MAX_CELL_M`. Lengths in metres become cells through
   `MapFrame` (`cells_of_m`, `centre_m`, `index_centre_m`, ...), never by hand. A loop whose
   length a recipe count sets reaches a cancel point (`StageContext::check_cancel`, or the
   `cancel` callback of a kernel or of `features::pick`) at least once per pass over the grid.
2. Add one line to `stages::register` in `src/stages/mod.rs`:
   `registry.add(Box::new(terrain::my_stage::MyStage));`.
3. Name it in a recipe's `[[stage]]` list. The runner checks the declared reads and writes, and
   `describe` lists the stage by itself.

## How to add an evolution process

A per-step process of `landscape_evolution` is a stage with the same contract.

1. Add a module under `src/stages/evolution/` that implements `TypedStage`.
2. Add one line to `PROCESSES` in `src/stages/evolution/processes.rs`.
3. A recipe switches it on with a `[[stage.process]]` table of its `landscape_evolution` stage.

## Adding a kernel

A kernel is one module in `src/kernels/`, listed in `kernels/mod.rs`.
