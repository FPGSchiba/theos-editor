# Map generation research

Research of 11–12 Nov 2026 that joins the terrain generator of P08 (`theos-maps`, spec merged in
#27) with the ten themed bands of the Tower. It answers three questions: which floors the
generator can design, what must be added or changed, and in which order.

Nothing here is binding. Proposed changes to the umbrella, the P08 spec, `docs/map-package.md`
and `docs/generators.md` are draft text in `09-proposed-phases.md`; Jann decides which become
their own docs PRs. Like `research/P10-prose-editor`, no later phase imports this folder.

## Start here

1. [`00-map-generation-research.md`](00-map-generation-research.md): the synthesis, with the
   readiness per band, the gap register, the proposed order and 15 decisions for Jann.
2. [`09-proposed-phases.md`](09-proposed-phases.md): amendments to the P08 spec, the package
   format changes, protocol additions, and twelve proposed phases with draft criteria.

## Where things are

| Path | What |
|---|---|
| `01-capability-baseline.md` | What the generator can do in P08, P29 and P30, and its limits |
| `02-band-requirements.md` | The ten bands in generator terms; nine terrain types; sample floors |
| `03-floor-frame.md` | What a floor physically is, against the generator's assumptions |
| `decisions.md` | Jann's answers at the first checkpoint |
| `04-readiness-matrix.md` | Bands against capabilities (first version; see its note) |
| `05-probes.md` | The old Python toolset run with band-like recipes |
| `06a-audit.md` | Independent audit of the research claims in the P08 spec |
| `06b-natural-terrain.md` | State of the art: caves, needle peaks, floating ground, giant trees |
| `06c-built-environments.md` | State of the art: towns, cities, ruins, interiors, ring layouts |
| `06d-steering-and-blending.md` | State of the art: steering, constraints, metrics, blending, batches |
| `07-package-headroom.md` | Format 1.0 against the bands (first version; see its note) |
| `08-throughput-and-gameplay.md` | Producing 100 floors, and gameplay fit |
| `review-round-1.md`, `review-round-2.md` | Two fresh reviews of `00` and `09` |
| `probes/` | Probe recipes, scripts, the Bore extension and result numbers |

## How it was done

- Stages 0–2 (baseline, band sheet, floor frame) from the specs, the ten-bands decision and the
  Tower proposal; then a checkpoint with Jann.
- Stages 3–7 in parallel: readiness and format headroom from the specs; an audit and three
  research reports by separate agents with web search; probes on `theos_maps` 0.3.0.
- Stage 8, the synthesis; stage 9, two rounds of review by agents that had seen none of the work.

## What to keep in mind when reading

- **Nothing was built or run on the Rust generator.** It does not exist yet. The probes ran on
  `theos_maps` 0.3.0, which lacks sediment deposition and multi-neighbour flow.
- **Sources.** Every report marks each source as opened, found by search only, or cited from
  memory. Core papers for streets, parcels, room graphs and trees were not opened. GPL code was
  not read.
- **Own designs.** Several recommended methods have no paper behind them; `00` marks each. Each
  needs a probe before a phase spec relies on it.
- **Review.** Round 1 found 41 points (2 critical). Round 2 found 21 new points (none critical)
  and 14 of round 1 only partly resolved. `00` and `09` answer the important ones of both rounds.
  The fixes after round 2 were not reviewed again.

## Open after round 2

- Draft criteria that still have no measure or are true by construction: the arena's position
  next to `gate-up`, the volume-balance tolerance and the summit and swamp rules in P38, station
  spacing and canals in P42, the cloud-sea base level, platforms at anchors in P43
  (`review-round-2.md` N-17).
- Format leftovers: a bridge over a gap is a `site` in P38 and a `link` in P39; no feature type
  yet for a keep, a water gate or a moat; `deck` and `present` overlap in meaning (N-18).
- The phase table has no slice column in the umbrella's sense (round 1, finding 41).

## Probes

`probes/README.md` says how to run them. Two things are not in this repository:

- The toolset itself (`theos_maps` 0.3.0 with its recipes). Copy it from Drive,
  `Design/_system/tools/` and `Design/_system/recipes/`, into `probes/toolset/`.
- The 15 contact sheets. They are in Drive under `Design/_lab/2026-11-map-research/`.
