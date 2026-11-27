# P29 probes

The probes behind the P29 spec (`docs/specs/phases/P29-settlements-and-child-maps.md` §4.3), run
on 27 Nov 2026.

- `settle_baseline.py`, `table.py`: the driver that runs `theos_maps` 0.3.0's settlement child
  runs with seeds set directly, and the table builder. Environment as in the P08 spec §4.3
  (Python 3.12, fastscapelib 0.3.0, numpy 2.5.4, scipy 1.18.1 through uv), with `PYTHONPATH`
  pointing at a copy of the 0.3.0 export and `MPLBACKEND=Agg`:

  ```text
  python settle_baseline.py <parent engine folder> <feature id> <out folder> --seeds 1..8
  ```

- `probe-0.3.0-baseline.md`: the probe's per-seed table for `fixture-small` and `hearth-valley`
  seed 1 parents. Its metric definitions are the probe's; the P29 build changes the driver to the
  definitions of the spec's §5.13 and records the CE-78.3 fixture from it.

The geometry-crate probe and the street-tracer probe left only their results, quoted in the spec;
their sources stayed in the session's scratchpad.
