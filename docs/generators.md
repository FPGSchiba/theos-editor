# Generators — protocol v1

Status: accepted with the umbrella spec. Binding.

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

- **process** — an external program speaking the stdio protocol below (theos_maps first; any
  other program, including the game engine, can implement it);
- **comfyui** — the ComfyUI HTTP API; workflow templates with named slots are its recipes.

## 2. Registration

Per project, in `.theos/settings.toml`:

```toml
[generators.terrain]
adapter  = "process"
command  = ["uv", "run", "--project", "tools/theos_maps", "python", "-m", "theos_maps.gen"]
produces = ["map"]

[generators.comfy]
adapter   = "comfyui"
url       = "http://127.0.0.1:8188"
workflows = "tools/comfy/workflows"
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
versions listed here. Changes to this document need Jann's approval (`docs/gate.md`).
