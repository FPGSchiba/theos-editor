# 08 · Throughput and gameplay fit

> **After review round 1:** the pass rates on this page are those of `theos_maps` 0.3.0; P08's own rate is not measured. Whether every floor's gate up has a guardian arena is open as decision D10. Early constraint checks are a batch option, not the default (`09` P32).

Stage 7 of the map generation research: can 100 floors be produced, and can a map be made to play
the way a floor should?

Sources: P08 phase spec (Appendix A, §5.5, §5.9), umbrella CE-28.5, CE-78.3, CE-79.2, the probes
(`05-probes.md`) and the steering report (`06d-steering-and-blending.md`), whose recommendation
ids (A1, B1, C1 …) are used below.

## Summary

- **Throughput is the weaker side today.** The generator can make one good valley; nothing in the
  specs helps a designer make 100 floors that differ on purpose. Recipes take no inputs, every
  floor would be its own hand-tuned file, and most seeds fail their own constraints.
- **Gameplay fit is closer than it looks.** Walking times, lock and key, and reachability are
  already measured. What is missing is placement that aims at those numbers, and four kinds of
  metric: routes, chokepoints, sightlines, cover.
- **Both are fixed by additions around the generator, not by changes to P08's stages:** recipe
  families with curves over the floor number, requirements placed constraint-first, metric
  stages, and a batch tool that runs, ranks and shows seeds.

## Throughput

| Question | Finding | Source |
|---|---|---|
| How often does a seed pass? | 0.3.0: `hearth-valley` 2 of 8, `hearth-valley-omega` 3 of 8. Lake share fails most often (5 of 8), then relief and vegetation cover | P08 Appendix A |
| Will P08 fix that? | Partly. Sediment deposition should cut lake share. CE-28.5 only asks P08 to pass "at least as often" as 0.3.0 | P08 §4.2, umbrella CE-28.5 |
| How much do seeds vary at other scales? | On a 16 km farmland probe, lake share ran from 3 % to 32 % over four seeds; on a 4 km valley the default river thresholds no longer apply | `05-probes.md` |
| What can a designer steer? | Source positions, stage parameters, a seed, and min/max limits on summary numbers. No fixed places, no masks, no sketch | P08 §5.5 |
| How are ten floors of a band told apart? | Only by writing ten recipes by hand. Nothing measures whether neighbours differ or drift smoothly | — |
| How long does a batch take? | 0.3.0: 17–23 s per seed at 384². 100 floors × 16 seeds is about 7.6 hours on one thread as an upper bound; the Rust generator is not measured yet | P08 §4.3, `05-probes.md` |

Recommended answers (all from `06d`):

| Need | Answer | Id | Size |
|---|---|---|---|
| One recipe family per band, not one file per floor | A `floor` input, parameter curves over it, `extends`, weights | B1 | Small–medium |
| Fewer wasted runs | Check each constraint at the earliest stage that fixes it; re-roll single stages | C1, C3 | Small |
| Seeds that hit their targets | Calibrate one driver per target (uplift for relief, and so on) before the batch | C2 | Small–medium |
| Review of many maps | Batch tool: run, rank, contact sheet per floor, band sheet, metric-versus-floor plots | P1 | Small–medium |
| Neighbouring floors that differ | Ranking bonus for distance from the neighbours' picks | C5 | Small |

Designer effort then moves from floors to bands: per band a base recipe and its curves; per floor
a list of requirements, one contact sheet and one pick. This is an estimate, not a measured
figure.

## Gameplay fit

| What a floor needs | What exists | Gap | Answer |
|---|---|---|---|
| Runs of a chosen length between Chimes | `travel` gives walking time from extraction points; `travel.extraction_to_extraction_min` can be constrained | Placement does not aim at a time; it is tested after the fact | Requirements placed on the travel-time field (C4) |
| A hub settlement with a Chime | Settlement classes; no hub, no Chime type | Place types | New types and a requirement "Chime in the hub" (C4, format 1.1) |
| The way up far from the entry | `portal-in`, `portal-out` placed by 0.3.0's rules | No distance rule | Requirement on travel time (C4) |
| A guardian arena on every tenth floor | Nothing | No arena concept | Metric "largest flat disc", a requirement, and `site_prep` to level the ground (M1, C4, A4) |
| Quest routes and spawns | Written by people as pins bound to features (P22) | None for the generator | Route metrics tell the author where routes run (M1) |
| Extraction-shooter feel outdoors: chokepoints, sightlines, cover | Nothing | Four metric groups | Metric stages `route_stats`, `chokepoints`, `visibility`, `cover` (M1) |
| Playable interiors | Lock and key, reachability, one-way drops, a sightline limit in recipes | Scaling to many levels | Hierarchical key analysis over levels (`06c` §5) |
| Lore that fixes a shape ("a castle on this hill above a river") | Nothing | No inputs | Anchors and control layers (A1, A3) |

No published source was found for map metrics specific to extraction shooters. The metric set is
assembled from general level-design metrics and must be tuned on real play.

## What this asks of the specs

- `docs/generators.md` §3: input types `number`, `enum`, `layers`, `anchors`.
- Recipe format (P08 §5.5): inputs, curves, `extends`, weights, requirements. Additions; P08's
  three built-in recipes stay valid.
- P09 (`theos gen`): the batch tool belongs there or directly after it.
- P16 (Maps screen): the band sheet and the metric-versus-floor plot are review views.
