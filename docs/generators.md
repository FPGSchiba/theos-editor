# Generators — protocol v1

Status: accepted with the umbrella spec. Binding. Map frames and features, camera inputs,
blockouts, workflow sidecars and pipelines added with umbrella spec version 2.

A **generator** turns a recipe, inputs and a seed into output files and results: the map toolset
today, ComfyUI for concept art, and later perhaps a renderer inside the game engine. The editor
knows generators only through this interface, so a generator can be replaced or added without
touching the data model or the UI, and any other tool can drive the same generators or read their
results without the editor.

## 1. What the editor needs from a generator

| Operation | Purpose |
|---|---|
| `describe` | Name, version, what it produces (`map`, `concept`), its recipes and their inputs. |
| `run` | One run for one seed: progress events while it works, then a manifest. |
| cancel | Stop a run; partial outputs are discarded. |

Inside the editor this is the `Generator` trait in `theos-studio` with two adapters:

- **process** — an external program speaking the stdio protocol below (theos_maps first, then the
  blockout generator theos_blockout; any other program, including the game engine, can implement
  it);
- **comfyui** — the ComfyUI HTTP API; workflow templates with named slots are its recipes (§7).

Pipelines (§8) chain recipes into stages with a pick between them. They live in the editor, not in
the protocol: a generator never sees a pipeline, only single runs.

## 2. Registration

Per project, in `.theos/settings.toml`:

```toml
[generators.terrain]
adapter  = "process"
command  = ["uv", "run", "--project", "tools/theos_maps", "python", "-m", "theos_maps.gen"]
produces = ["map"]

[generators.blockout]
adapter  = "process"
command  = ["uv", "run", "--project", "tools/theos_blockout", "python", "-m", "theos_blockout.gen"]
produces = ["map"]

[generators.comfy]
adapter   = "comfyui"
url       = "http://127.0.0.1:8188"
workflows = "tools/comfy/workflows"
pipelines = "tools/comfy/pipelines"
produces  = ["concept"]
```

Paths are relative to the project root. The binaries folder comes from the user settings.

## 3. Process protocol (stdio, newline-delimited JSON)

Every line a generator writes to stdout is one JSON object. Anything on stderr is a log line.

### describe

```text
<command> describe
```

```json
{"protocol": 1, "name": "theos_maps", "version": "0.3.0", "produces": ["map"],
 "accepts_notes": false,
 "recipes": [{"id": "hearth-valley", "title": "Hearth valley terrain",
              "inputs": {"size_km": {"type": "number", "default": 4.0}},
              "seeded": true}]}
```

### run

```text
<command> run --request <request.json> --out <empty directory>
```

Request file:

```json
{"protocol": 1, "recipe": "hearth-valley", "seed": 1,
 "inputs": {"size_km": 4.0},
 "from": null,
 "notes": [{"id": "n1", "about": "f04-terrain-v2", "text": "Too much relief for the Hearth."}]}
```

`from` is the path of a source output for refinement runs; `notes` are the parent's open notes,
sent only when `describe` says `accepts_notes: true`.

An input of type `cameras` in `describe` asks for views: the editor fills it from the
`## Cameras` of the map the run is for (`docs/specs/data-model.md` §9.7), converted to metres in
that map's frame:

```json
{"cameras": [{"id": "gate-a6", "at_m": [14880, 17040], "h_m": 62,
              "look_m": [13200, 15360], "look_h_m": 115, "fov": 66, "portrait": false}]}
```

Events on stdout while running:

```json
{"event": "progress", "stage": "erosion", "done": 3, "total": 7}
{"event": "done", "manifest": "manifest.json"}
```

or, on failure, `{"event": "error", "message": "…"}` and a non-zero exit code.

Manifest (in `--out`):

```json
{"protocol": 1, "generator": "theos_maps@0.3.0", "recipe": "hearth-valley", "seed": 1,
 "inputs": {"size_km": 4.0},
 "outputs": {"truth": "s1-truth.png", "inworld": "s1-inworld.png",
             "heightfield": "s1-height.npy", "package": "s1_engine.zip"},
 "results": {"relief_m": 1234, "lakes_pct": 2.99, "pass": true}}
```

Output paths are relative to `--out`. Result values are numbers, booleans or short strings.

**Maps: frame and features.** A generator that produces maps adds a `frame` and, when it places
features, a `features` output:

```json
{"frame": {"size_m": [24000, 24000]},
 "outputs": {"truth": "s1-truth.png", "features": "s1-features.geojson", "…": "…"}}
```

- `frame.size_m` is the map's width and height in metres. A child map (a run whose `from` is the
  output of another map run, such as a settlement inside a region) adds `origin_m`, its north-west
  corner in the parent's frame; the editor records the parent version in the version's `frame`.
- `features` is a GeoJSON FeatureCollection whose coordinates are metres in the map's frame
  (x east, y south, origin north-west), not longitude and latitude: an alternative coordinate
  system by prior arrangement, as RFC 7946 §4 allows. Every feature has a string `id` that stays the
  same for the same recipe and seed, and `properties.type` (city, town, village, ruin, shrine,
  portal-in, portal-out, extraction, camp, watchtower, plot, lane, wall, …).
- Both are optional for protocol 1: a map without a frame is shown but takes no pins.

**Blockouts.** The blockout generator produces versions of a map whose `type` is `blockout`. Its
recipes build a clay scene from a settlement or interior map package (given as `from`) or from a
blockout script, then render every camera of its `cameras` input. Outputs: `scene` (glTF binary)
and, per camera, `<camera>-clay`, `<camera>-depth` and `<camera>-canny` (PNG).

### cancel

The editor kills the whole process tree. A generator must not leave anything outside `--out`.

## 4. What the editor does with a run

1. Copies each output into the binaries folder under `<concepts|maps>/<parent id>/`, hashes it,
   and writes a version record (`docs/specs/data-model.md` §8). Human runs go to the working
   tree; agent runs go to the session branch as review entries.
2. Keeps runs that nobody keeps in `.theos/cache/runs/` until they are discarded.

## 5. Conformance

`cargo xtask generator-check -- <command…>` runs `describe` and one small `run` against any
process generator and reports every protocol violation. A new generator — the game engine
included — is ready when this passes.

## 6. Versioning

The protocol version is the integer `protocol`. Breaking changes raise it; the editor supports the
versions listed here. Changes to this document need Jann's approval (`docs/gate.md`). `frame`,
`features` and inputs of type `cameras` were added to protocol 1 as optional parts; a generator
without them stays valid.

## 7. ComfyUI workflows

A recipe of the comfyui adapter is a workflow in ComfyUI's API format,
`tools/comfy/workflows/<id>.json`, with a sidecar `tools/comfy/workflows/<id>.toml`:

```toml
id       = "klein-ref-look"
title    = "Look transfer with FLUX 3 Klein references"
produces = "concept"
vram_peak_gb = 13.5            # measured at the default size

[slots.prompt]
node = "6"
input = "text"
type = "text"

[slots.image0]
node = "41"
input = "image"
type = "image"

[slots.seed]
node = "25"
input = "noise_seed"
type = "seed"

[[models]]
file   = "flux3-image-klein-int8-convrot.safetensors"
folder = "diffusion_models"
source = "https://…"
```

- A slot names the node id and the input it fills, and its type (`text`, `image`, `number`,
  `seed`). Slots are what `describe` lists as the recipe's inputs.
- `models` lists every model file the workflow loads, with its ComfyUI folder and where to get it.
- Readiness: before a run the adapter asks ComfyUI's `/object_info` for the nodes and `/models` for
  the model files, and reports a workflow as not ready with the missing names.

## 8. Pipelines

A pipeline chains workflows into stages; a pick between stages chooses what the next one starts
from. Pipelines belong to the editor (`theos-studio`), not to the protocol.
`tools/comfy/pipelines/<id>.toml`:

```toml
id    = "place-key-art"
title = "Place key art: blockout to 4K"
type  = "key-art"              # the concept type it makes
needs = ["location"]           # what the concept's `for` must contain

[[stage]]
id     = "compose"
recipe = "zimage-cn-compose"
seeds  = 4
slots  = { control = "camera.canny", prompt = "prompt" }

[[stage]]
id     = "look"
recipe = "klein-ref-look"
slots  = { image0 = "pick.compose", image1 = "style.refs.buildings", image2 = "style.refs.sky" }

[[stage]]
id     = "upscale"
recipe = "zimage-4k"
pick   = "auto"
slots  = { image = "pick.look" }
inputs = { denoise = 0.22 }
```

Slot sources:

| Source | Value |
|---|---|
| `prompt` | the prompt of the run, written on top of the brief (`docs/specs/data-model.md` §9.9) |
| `pick.<stage>` | the image of the version picked in that earlier stage |
| `style.refs.<role>` | a reference of the effective style |
| `camera.<render>` | `clay`, `depth` or `canny` of the chosen camera, from the newest blockout version |
| `ref.<kind>` | the canon pick of the concept for the record of that kind in the concept's `for` |

- `pick` is `you` (the default: the stage waits for a pick) or `auto` (one seed, runs on).
- Each stage run writes concept versions with `from` set to the previous pick and `pipeline` and
  `stage` in `inputs`.
- One stage runs at a time.
