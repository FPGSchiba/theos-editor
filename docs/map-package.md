# Map package — format `theos-map` 1.1

Status: proposed with decision 0003 (`docs/decisions/0003-map-generator-in-rust.md`) and umbrella
spec version 3; binding once Jann merges it. Changes need Jann's approval (`docs/gate.md`).

A **map package** is the generated layer of one map version: what a generator made from one
recipe and seed that a game needs to build the place: terrain, water, surfaces, places with their
ids, roads, buildings and rooms. The editor bakes it once (decision 0003 §2); the game ships it
and reads it with the crate `theos-map-format`, which also writes it. It is the `package` output
of a map version (`docs/specs/data-model.md` §8).

What people write about a map (pins, cameras, spawns, the places of quest steps) is not in the
package; it stays in records in git (§9).

Format 1.0 replaces the package of the Python toolset `theos_maps` 0.3.0 (`theos-map` 0.1). What
changed: one file instead of a folder; raw layers instead of PNG; water as shapes (lake levels,
river lines); rooms as polygons; stacked levels; no timestamp, so the bytes are reproducible; a
hash per layer.

Format 1.1 (P29 and P34) only adds: the settlement and interior feature properties and types
marked "1.1" in §4.4 and §5. It defines no capability, so 1.0 readers read 1.1 packages (§6).

## 1. Container

- One file per map version with the extension `.theosmap`: a ZIP archive.
- Entries, in this order: `manifest.json`, `features.geojson`, then the layer files in the order
  of the manifest's layer index (§4.1). No directory entries.
- Every entry: deflate compression, modification time 1980-01-01 00:00:00 (the earliest time ZIP
  can hold), no extra fields, no comments, UTF-8 names with `/`. ZIP64 only for an entry over
  4 GiB.
- The deflate implementation is one that no cargo feature of another crate in the build can swap
  for another backend (a plain `flate2` can be switched to zlib-ng by any crate). P08's spec names
  it after a probe.
- A reader ignores entries it does not know (§6).

## 2. Coordinates

- **Frame.** The rectangle the map covers, `size_m = [width, height]` in metres. Origin at the
  north-west corner, x east, y south (G19). A position as a fraction of the frame is
  `[x / width, y / height]`.
- **Heights** are metres above the map's base level. For terrain the generator sets it so the
  drained edges of the map lie at 0 m. A child map uses its parent's base level.
- **Grid.** `grid = [rows, columns]` of square cells of `cell_m` metres, so
  `size_m = [columns · cell_m, rows · cell_m]`. Row 0 is the northern edge, column 0 the western
  edge. The value of cell (r, c) belongs to its centre at
  `x = (c + 0.5) · cell_m`, `y = (r + 0.5) · cell_m`.
- **Engines with Y up** map a point (x, y, h) to (X, Y, Z) = (x, h, y): east, up, south. That is
  right-handed.
- **Child maps** (a settlement inside a region, an interior inside a ruin) are axis-aligned with
  their parent. `frame.origin_m` is the child's north-west corner in the parent's frame. There is
  no rotation.
- **Levels** of an indoor map stack in the same frame (§4.3).

## 3. `manifest.json`

| Field | Type | Meaning |
|---|---|---|
| `format` | `"theos-map"` | |
| `format_version` | `"MAJOR.MINOR"` | `"1.1"` for this document (§6) |
| `generator` | `name@version` | As the generator's `describe` reports it, for example `theos-maps@1.0.0` (`docs/generators.md` §6) |
| `requires` | array of strings | The capabilities a reader must implement to build this map correctly (§6), sorted; empty when the package uses none. Format 1.0 defines none yet |
| `kind` | text | `terrain`, `settlement`, `interior`, … (the map's `type` in the data model) |
| `recipe_id` | text | The recipe's id |
| `recipe` | object | The full recipe as it ran, so the package can be regenerated after the recipe file changed |
| `seed` | integer | |
| `inputs` | object | The run's inputs (`docs/generators.md` §3) |
| `frame` | object | `size_m`, `cell_m`, `grid`; for a child map `origin_m` and `parent`; for an indoor map with stacked levels `levels` (§4.3) |
| `layers` | array | The layer index (§4.1) |
| `features` | `"features.geojson"` | §5 |
| `results` | object | The run's results: numbers, booleans or short strings, with `pass` and the constraint keys of `docs/generators.md` §3 |
| `summary` | object | Everything else the generator reports about the run; free form |

`frame.parent` names what a child map was made from: `package_sha256` (the parent package's
SHA-256) and `feature` (the parent feature it details, such as `town-2`). The editor records the
parent map version in the version record's `frame` (data model §8).

**Writing JSON** (`manifest.json` and `features.geojson`): exactly what `serde_json`'s pretty
printer writes for a value whose maps keep their keys in lexicographic order: UTF-8, two-space
indentation, one array element or object member per line, numbers in the shortest form that reads
back to the same value; then one final LF.

A shortened example (`…` marks left-out parts):

```json
{
  "features": "features.geojson",
  "format": "theos-map",
  "format_version": "1.0",
  "frame": {
    "cell_m": 62.5,
    "grid": [
      384,
      384
    ],
    "size_m": [
      24000.0,
      24000.0
    ]
  },
  "generator": "theos-maps@1.0.0",
  "inputs": {},
  "kind": "terrain",
  "layers": [
    {
      "description": "terrain height",
      "dtype": "f32",
      "file": "layers/height.f32",
      "max": 1874.2,
      "min": 0.0,
      "name": "height",
      "sha256": "…",
      "shape": [
        384,
        384
      ],
      "units": "m"
    }
  ],
  "recipe": …,
  "recipe_id": "hearth-valley",
  "requires": [],
  "results": {
    "elevation_m_relief": 1234,
    "elevation_m_relief_max": 2200,
    "elevation_m_relief_min": 800,
    "elevation_m_relief_ok": true,
    "pass": true
  },
  "seed": 1,
  "summary": …
}
```

## 4. Layers

### 4.1 Layer index

| Field | Req. | Meaning |
|---|---|---|
| `name` | yes | Unique within its level |
| `file` | yes | `layers/<name>.<dtype>`, or `layers/<level id>/<name>.<dtype>` on a map with levels |
| `dtype` | yes | `f32`, `u8` or `u16` |
| `shape` | yes | `[rows, columns]`, the frame's `grid` |
| `sha256` | yes | SHA-256 of the file's bytes, lower-case hex |
| `description` | yes | One line |
| `units` | no | `m`, `m3/s`, `degC`, `mm/yr`, `min`, … for measured values |
| `scale`, `offset` | no | For a quantised field: value = `offset + raw · scale` |
| `legend` | no | For a class layer: raw value (as a string) → class name; the reader takes classes from here, never from a fixed list |
| `min`, `max` | no | Of the decoded values |
| `level` | on maps with levels | The level id (§4.3) |

### 4.2 Encoding

Raw values without a header: little-endian, row by row from north to south, each row from west to
east, `rows · columns` values. `f32` is IEEE 754 binary32; NaN and infinities are not allowed. A
mask is `u8` with 0 for no and 1 for yes.

### 4.3 Levels

An indoor map with stacked levels lists them in `frame.levels`, bottom to top:
`{"id": "<slug>", "elevation_m": <number>}`, where `elevation_m` is the walking surface above the
map's base level, plus an optional `name`. When `levels` is present:

- every layer entry and every feature carries `level`;
- layer files sit under `layers/<level id>/`;
- a feature that connects levels (`stair`, `ladder`, `lift`, `drop`) carries `from_level` and
  `to_level`; a `drop` goes one way only, from `from_level` to `to_level`.

A map without `levels` has one implicit level; its layers and features carry no `level`.

All levels share the frame's grid. The format sets no size limit; the generator refuses a run that
would not fit its memory budget. A later capability (§6) may give levels a grid of their own.

### 4.4 Standard layers

A generator writes the layers that fit its map. Readers rely on these names and meanings and
ignore layers they do not know.

| Kind | Layer | dtype | Meaning |
|---|---|---|---|
| terrain | `height` | f32 | Height, m |
| terrain | `water_surface` | f32 | The lake level where there is a lake, else the ground height, m |
| terrain | `flow_dir` | u8 | The D8 code of the steepest downhill neighbour on the depression-filled surface, so every cell points along its path to an outlet; 0 for cells that drain off the map and for the lowest cell of a terminal lake (legend) |
| terrain | `drainage_area` | f32 | Upstream area draining through the cell over multiple flow directions, m², not weighted by runoff |
| terrain | `erosion`, `deposition` | f32 | Thickness removed and thickness added by all processes over the whole run, each cumulative and non-negative, m |
| terrain | `discharge` | f32 | River discharge, m3/s |
| terrain | `precipitation` | f32 | Annual precipitation, mm/yr |
| terrain | `temperature` | f32 | Mean annual temperature, degC |
| terrain | `water` | u8 | Class: 0 dry, 1 stream, 2 river, 3 lake (legend) |
| terrain | `rock` | u8 | Bedrock class (legend) |
| terrain | `vegetation`, `snow` | u8 | Density 0–1 (`scale` 1/255) |
| terrain | `roads` | u8 | Road class per cell (legend) |
| terrain | `element_water`, `element_wood`, `element_earth`, `element_metal`, `element_fire` | u8 | Ambient element affinity 0–1 (`scale` 1/255) |
| terrain | `omega` | u8 | Detuning ω 0–1 (`scale` 1/255), when the recipe runs that stage |
| settlement | `height`, `water` | f32, u8 | As for terrain |
| settlement | `streets` | u8 | Street class (legend) |
| settlement | `buildings` | u8 | Mask of building footprints |
| settlement | `fields` | u8 | Land use: 0 none, 1 field, 2 orchard, 3 pasture, 4 garden, 5 yard, 6 open ground, 7 plaza (legend; 1.1) |
| interior | `walkable` | u8 | Mask of walkable floor |
| interior | `rooms` | u16 | Room number + 1 per cell, 0 for none; the room features carry the number |
| interior | `corridors`, `cover` | u8 | Masks |

### 4.5 Snapshot layers

A run may ask the generator to keep the layers a stage wrote, for debugging or as training data.
Such a layer is named `<layer>@<stage>`, its file is `layers/snapshots/<stage>/<layer>.<dtype>`,
its dtype is f32 or u8, and the manifest's `summary.snapshots` lists them. A package with
snapshot layers is a debugging or training artefact, never a canon output. Readers that do not
ask for snapshots ignore them.

## 5. Features

`features.geojson` is a GeoJSON FeatureCollection (RFC 7946), the same bytes as the run's
`features` output (`docs/generators.md` §3). Coordinates are metres in the frame (§2), not
longitude and latitude: an alternative coordinate reference by prior arrangement, which RFC 7946
§4 allows. A third coordinate, where present, is a height in metres above the base level.

- Every feature has a string `id`, unique in the package and the same for the same generator
  version, recipe, seed and inputs. Ids are opaque to readers.
- Every feature has `properties.kind` (the group) and `properties.type`.
- Values per vertex of a line (a river's width or discharge) are arrays in `properties` with one
  value per coordinate.
- On a map with levels every feature has `properties.level` (§4.3).

| Kind | Types | Geometry | Properties besides kind and type |
|---|---|---|---|
| `settlement` | `city`, `town`, `village`, `hamlet` | Point | `name` (generated, not canon) |
| `poi` | `ruin`, `chime`, `gate-up`, `gate-down` | Point | |
| `site` | `mill`, `mine`, `quarry`, `lumber-camp`, `shrine`, `watchtower`, `inn`, `cave`, `camp`, `bridge`, `ford` | Point | links to related features by id (`road`, `serves`, `deposit`); bridges also `span_m` and `deck_m` (the deck's height above the base level, m) |
| `resource` | one type per resource (`iron-ore`, `timber`, …) | Point | `richness`, `tier`, `element`, `harvest`, `radius_m` |
| `road` | `trail`, `track`, `road`, `main` | LineString | `from`, `to` (feature ids), `length_m`, `width_m`, `grade_max` |
| `water` | `river` | LineString (x, y, bed height) | `width_m`, `depth_m`, `discharge_m3s` per vertex; `order` (Strahler) |
| `water` | `lake` | Polygon, or MultiPolygon when one lake has several outlines | `surface_m` (water level), `terminal` (no outflow), `outlet` (a point in metres; absent for a terminal lake), `depth_max_m` |
| `street` | `lane`, `street`, `main-street` | LineString | `from`, `to` (street node ids, opaque strings), `width_m`, `length_m`, `bridge` (boolean), `entry` (the parent road's id, main streets only) (1.1) |
| `parcel` | `house`, `landmark`, `yard`, `garden`, `plaza`, `open` | Polygon | `street` (the street it faces; absent for none), `area_m2` (1.1) |
| `building` | `building` | Polygon | `use` (`house` or a landmark type), `storeys`; 1.1 adds `height_m` and `parcel` |
| `landmark` | free text (P29: `market`, `well`, `shrine`, `temple`, `keep`, `inn`, `smithy`, `guardhouse`, `granary`, `archive`, `mill`, `harbour`) | Point | `name` (generated, not canon); 1.1 adds `building` or `parcel` |
| `wall` | `wall`; `tower` (1.1) | Polygon; Point | wall: `height_m`, `thickness_m` (1.1); tower: `wall` (1.1) |
| `gate` | `gate`; `water-gate` (1.1) | Point | `wall`, and for `gate` the `street` it carries (1.1) |
| `room` | room type from the recipe | Polygon | `number` (as in the `rooms` layer), `role`, `keys`, `locked` |
| `door` | `open`, `locked`, `drop`, … | Point | `rooms` (the two room ids), `key` |
| `connection` | `open`, `locked`, `oneway` | null | `from`, `to` (room ids), `key` |
| `route` | `critical` | LineString | `rooms` (ids in order) |
| `link` | `stair`, `ladder`, `lift`, `drop` | Point or LineString | `from_level`, `to_level` |

The generator's phase specs (P08, P29, P30) define each type's properties in full. A new kind,
type or property is an addition (§6).

## 6. Versions and reading rules

- `format_version` is `MAJOR.MINOR`. A reader refuses a package whose major it does not know,
  with a message naming both versions.
- Within a major, a reader ignores fields, entries, layers, feature kinds and properties it does
  not know, with one exception: a reader refuses a package whose `requires` names a capability it
  does not implement, with a message naming the capability.
- A minor version only adds things. An addition that changes how existing fields must be read
  (ground above ground, cells that are not part of the floor) is a **capability**: the minor
  version that defines it gives it a name, and every package that uses it lists that name in
  `requires`. An older reader then refuses such a package instead of building a wrong map.
  Anything that cannot be expressed this way is a new major.
- The crate `theos-map-format` is the reference reader and writer. It depends on no other crate
  of this workspace, so the engine can depend on it alone (G1).

## 7. Reproducibility

The same generator version, recipe, seed and inputs give the same package bytes on every
platform of G2 (G20):

- no timestamps, host names, paths or random ids in any entry;
- JSON as in §3, ZIP as in §1, with a deflate implementation no feature can swap and its version
  pinned through `Cargo.lock`;
- layer values from deterministic computation (decision 0003 §3).

The renders (§8) are not part of this promise: they are pictures for people, and the drawing
crates promise nothing across platforms.

The `sha256` of each layer lets a reader check one layer without the others, and lets tests
compare the layers of two platforms.

## 8. Renders

Images for people are outputs of the run, not package entries (`docs/generators.md` §3). One of
them, `base`, lies exactly on the frame: its north-west pixel corner is the frame's origin, and
its width and height in pixels cover `size_m`, with square pixels. A map with levels has one
`base-<level id>` per level instead. The Atlas shows `base` under the pins (data model §8). Other
renders (`truth`, `inworld`, `geology`, …) may have titles, legends and margins.

## 9. The authored layer

Pins, cameras, spawns and the places of quest steps are written by people in records
(`docs/specs/data-model.md` §9.5–9.7) and are never in a package: moving a pin must not change a
binary or its hash.

When the engine part exports them (umbrella §5, "Later"), the export is a GeoJSON
FeatureCollection in the same frame and coordinates as the package it belongs to. It names that
package by its `package_sha256`, and each feature carries the record id, the record's kind, the
package feature it is bound to (when a pin has one), its level, and its radius for an area. This
section fixes only the direction; the export's own format comes with the engine part.
