# theos-map-format

Library. The map package format of `docs/map-package.md`: a `.theosmap` file is a ZIP with a
manifest, the raster layers and a GeoJSON feature file. The crate is the one place that reads and
writes it; the terrain generator (`theos-maps`) and the engine side use it, nobody else touches
the bytes.

- `manifest`: the manifest types (`Manifest`, `Frame`, `LayerEntry`, `DType`, `Level`, `Parent`).
- `layer`: `LayerData` (`f32`, `u8`, `u16` rasters), `LayerKey` and their on-disk encoding.
- `package`: `Package`, the manifest plus layers plus features, with `validate`.
- `write`: `write` and `write_cancellable`. The ZIP has fixed headers, so equal packages give
  equal bytes.
- `read`: `read`. Lenient towards what a later minor version adds, strict about integrity (sizes,
  hashes, names).
- `json`: the canonical JSON writer (object keys in lexicographic order, `serde_json`'s pretty
  printer, one LF at the end), and `to_checked_value`, the one conversion of a value into it.
  Everything that is hashed or compared as bytes goes through them.

## Determinism rules for contributors

The same package must give the same bytes on every platform (G20).

- Canonical JSON only: never serialize a manifest or feature file with a plain `serde_json` call.
- Every value becomes JSON through `json::to_checked_value`, never `serde_json::to_value`:
  `serde_json` writes NaN and the infinities as `null` without an error, and the checked
  conversion refuses them (`FormatError::JsonNotFinite`).
- No `HashMap` or `HashSet`; use `BTreeMap` and `Vec`, so no output depends on iteration order.
- No clock, no OS values, no locale in the bytes.
- ZIP entries are written in a fixed order with fixed timestamps and attributes.
- Deflate runs through `flate2` with the pure Rust backend; the workspace turns off the C
  backends, so a platform's zlib never changes the bytes.

## Tests

Units sit next to the code. Behaviour tests are the integration crate `tests/format/`. Run them
with `cargo nextest run -p theos-map-format --locked`.
