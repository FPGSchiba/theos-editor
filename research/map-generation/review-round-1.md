# Review round 1 — `00-map-generation-research.md` and `09-proposed-phases.md`

Reviewer: fresh, 11 Nov 2026. Read in full: `BRIEF.md`, `00`, `09`, `04`, `05`, `06a`–`06d`, `07`,
`08`, `docs/map-package.md`. Read in part: P08 spec (§3–§5.14, §6, §7, Appendix A), umbrella
(§2, §3 table, P08, P09, P16, P22, P23, P29, P30, §5), decision 0003 (§3, §6–§8, §9 recipe
lines), `docs/generators.md` §3–§6, `docs/gate.md` §5.

Not checked: `01`, `02`, `03` and `decisions.md` (not in the folder; the owner's decisions were
taken from the review task). No probe was re-run, no contact sheet was opened, no external source
was opened. Line numbers refer to the files as they stood at review time.

Counts: **2 Critical, 24 Important, 13 Minor.** Every one of the eight categories has findings;
none is empty.

Abbreviations: "00" = the synthesis, "09" = proposed phases, "MP" = `docs/map-package.md`.

---

## Critical

**1. Critical — 09 §B (title "additive"), §D P34; 00 "The package format survives" (l. 17–19), D3, D4.
The slab part of format 1.1 is not an optional addition under MP §6.**

- What is wrong: MP §6 says "A minor version only adds optional things; anything else is a new
  major", and a 1.0 reader ignores what it does not know. Several items in §B change the meaning
  of things a 1.0 reader already reads, so ignoring them gives a wrong map, not a poorer one:
  - `frame.levels[].elevation_m` is defined in MP §4.3 as "the walking surface above the map's
    base level". With a sloping per-level `height` it becomes "nominal" (06b §1.4 point 4 asks for
    exactly that restatement; 09 §B does not carry it). A 1.0 reader builds flat floors.
  - `solid`/`present` and `void` masks are needed to read `height` correctly. MP §4.2 forbids NaN,
    so cells where a partial level or the Bore has no ground still hold a finite height. A reader
    that ignores the unknown mask sees land there. That is probe 7b's failure moved to the reader
    (05 l. 365–369).
  - Putting `levels` on a terrain map switches on every "when `levels` is present" rule of MP
    §4.3 and §8 for existing content: every layer and feature must carry `level`, `base` is
    replaced by `base-<level id>`, and pins need `level` (CE-61.2). The Atlas level switcher is
    P30 (CE-79.3).
  - MP §4.3 and decision 0003 §7 tie levels to "at most about 2 km" and say a larger indoor floor
    "would need a new major format version". 04 lists band 4 at up to 5 km and band 9 at up to
    4 km (matrix, Scale column); 06b §1.4 says the note must be "revisited". 09 §B is silent.
- Two more readers exist besides `theos-map-format`: the Python blockout (CE-64.2, builds terrain
  from `height` and `water`) and the engine. Both would be "1.0 readers".
- Evidence: MP §4.2, §4.3, §6, §8; 06b §1.4 point 4, §3.3 point 4; 07 l. 16–17 (which itself
  names "a grid per level" as a major-version trigger); umbrella CE-61.2, CE-64.2, CE-79.3.
- Suggested fix: split §B into (a) truly optional additions (new feature kinds, types,
  properties, optional layers such as `biome`, `exposure`) and (b) changes of meaning (levels on
  outdoor kinds, `elevation_m`, partial levels, `void`). For (b) give Jann the real choice, which
  00 does not offer: amend format 1.0 now, while no reader or package exists (P08 is not built),
  or accept a 2.0. Add to P34 an AC that states what a 1.0 reader must do with a slab package
  (refuse, or read correctly), not only that it "ignores the additions".

**2. Critical — 00 "Proposed order" (l. 132–151); 09 P38–P43 "Needs" lines.
Half the order hangs on P30, which is not in the order, and P30 sits behind the Atlas.**

- What is wrong: 09 names P30 as a need only for P40 and P41. The stage reports say more phases
  need it, because P30 is where the world and the runner learn about levels:
  - 06b §1.4 point 3: "Per-level rasters need the world to hold a layer per level; P30 needs that
    anyway"; §3.3 point 3: "Yes, with one extension P30 also needs: layers addressed by level";
    §3.3 point 5: band 8 is Medium "once levels exist in the world"; §7 point 3: "`cavern` and
    `tree` after P30".
  - 06c §2: tiers reuse "the levels mechanism P30 builds", the runner needs "run this stage for
    each level", and "It should come after P30".
  - So P39 (plates), P42 (tiers), P43 (trees) and the slab links of P38 need P30, not only P34.
    P34 is the file format; it gives the world nothing. 09 lists "Needs P34" for P39 and P43 and
    "Needs P34, P35" for P42. 00's own table says "City tiers … Stages, after P30" (l. 80), so 00
    and 09 disagree.
- The umbrella makes P30 depend on P25 and P29 (phase table), and P25 on P16, P22, P24. As
  written, steps 7 to 12 of the order (bands 3 to 10) cannot start before most of the editor UI
  exists. 00 never says this and offers no decision to re-cut P30 (generator part against Atlas
  part).
- P29, P08 and P09 are also missing from the order table; P29 appears only as "(with P29)".
- Evidence: 09 l. 171, 183, 196, 206, 216; 06b and 06c as quoted; umbrella §3 table rows P25, P30.
- Suggested fix: put P08, P09, P29 and P30 into the order table with their umbrella dependencies.
  Add "Needs P30" to P39, P42, P43 (and P38 if it keeps slab links). Add a decision for Jann:
  split P30 into "levels in the world, runner and generator" (needs P29 only) and "Atlas level
  switcher" (needs P25), or accept that bands 3–10 wait for the Atlas.

---

## Important

### Category 1 — unsupported or misstated claims in 00

**3. Important — 00 l. 15: "three new map kinds".**
The reports give two: `cavern` and `tree` (06b §6). 06c says "No topic needs a new map kind"
(summary point 3). 00's own table (l. 83, 86) and 09 name only those two. Fix: "two new map
kinds", or name the third and its source.

**4. Important — 00 l. 15–16: "Every method the research recommends fits the Stage contract and
can be bit-identical (G20)."**
Nothing was built or run on the Rust generator (00 l. 169), so every "Yes" in the reports is an
argument, not a result. The reports themselves carry conditions that 00 drops:
- streets: `i_overlay` must pass a probe on the pinned toolchain, and a tracer probe is needed
  "to confirm bit-identity" (06c §0.2, §9 point 3);
- slabs, tiers, caverns, trees: the contract needs an extension (layers addressed by level, a
  per-level run), 06b §3.3 point 3, 06c §2 point 3;
- several recommendations are not stages at all: early checks and seed overrides are runner
  rules, recipe expansion runs before validation, calibration, archive and batch are a tool layer
  (06d §6 table, column "Stage contract").
Fix: say "by design; unproven until the probes of 09 run", and name the contract extension.

**5. Important — 00 l. 19–21 and G-5: "seeds that mostly fail their own constraints" as a blocker
of all 100 floors; "What blocks all 100 floors is not terrain."**
- The pass rates are 0.3.0's (P08 Appendix A: 2 of 8, 3 of 8). P08 changes the processes to fix
  the main failure (lake share) and tunes the built-in recipes to pass more often (P08 §5.5,
  "Built-in recipes"). 05's own caveat says not to read the lake results as findings about the
  new generator (l. 15–16), and its constraint ranges were set by the probe agent (l. 42–43).
- "Not terrain" contradicts 04: "Six terrain types have no phase" (l. 15–16). Seven of ten bands
  are blocked by missing generators.
- 06a adds a doubt 00 leaves out: claim 7 ("both gaps explain oversized lakes") is "?", and
  McDonald & Cordonnier 2026 suggests alluvial forms "need more than the ξ–q law. Lower the
  expectation in §4.2" (06a §2). 09 §A.6 turns that into "add … as known later options".
Fix: state the pass rate as 0.3.0's and unmeasured for P08; carry 06a's "lower the expectation"
into §A; reword the headline to "what every floor lacks, beyond terrain".

**6. Important — 00 l. 45: "No formula or constant is misquoted."**
06a says less: "No constant was found misquoted. Two formulas could not be compared with their
primary source (Lieth's NPP formula, Leopold & Maddock's exponents)" (summary), the lake formula
is "derived, not quoted" (#35), and "S is the steepest slope … was not confirmed" (§3). Of the 22
"confirmed", several rest on memory or titles (#18, #19, #27, #33). The one "wrong" verdict rests
on a source seen at "search result, abstract level" (06a Sources, RepDL). Fix: use 06a's wording
and add one line on how many verdicts rest on unopened sources.

**7. Important — 00 l. 108 (G-3): bands "1, 4, 9, 10", evidence "`05` probe 1".**
Probe 1 is band 1 at 4 km and 16 km only. Bands 4, 9 and 10 have no rivers, mills or placement
margins; their scale problem is a different one (the 2 km limit for levels, 04 Scale column).
Two gaps are merged and the second has no entry in the register (see finding 22). Also the fix is
filed as a "P08 amendment", but P08 keeps `river_q` as an absolute discharge (P08 §5.9
hydrology), so "P08 already moves recipes to metres" (00 l. 59) does not cover the threshold the
probe broke on.

**8. Important — 00 D7 (l. 163): "The probe shows c) fails."**
The probe shows that interpolating every recipe number in 0.3.0 does not keep rivers and
settlements. It did not test the tools the research itself proposes for "the same land":
`seed_group`, designer river courses and anchors (06d §4.3, A3, C4). 05 says single-flow routing
"makes that worse (the caveat applies)" and the result rests on two seeds (l. 376, 401–406). Fix:
"c) is not reached by parameter curves alone; untested with anchors and fixed courses", and let
Jann decide with that wording.

### Category 2 — readiness table

**9. Important — 00 l. 32: band 4, "After P29 and P30: Labyrinth floors (upper band)". Too kind,
and contradicted by 00's own order.**
P29 is a port of 0.3.0's interior. Probe 4 found one loop plus leaves at any size, 10–11 %
footprint use, "a mine or bunker plan, not a labyrinth of halls" (05 l. 232–238), and 05's
conclusion 6: "Settlement and interior generators need a different model for bands 4–10, not more
parameters". 00 l. 142 then lists "Upper band 4" as unlocked by P41. Fix: "Rooms on levels, not
yet a labyrinth" in the table.

**10. Important — 00 l. 29: band 1 "Small"; l. 23 and 139: "Band 1 is complete after four
phases".**
- The four phases include P34, which band 1 does not need, and exclude P08 and P29, which it
  does.
- The "rest" for band 1 is P31 (every placement, routing and render stage retrofitted), P32
  (B1 Small–medium, C4 Medium by 06d §6) and P33 (M1 Medium). That is not Small.
- D5 recommends replacing the settlement method (P35, step 5). Band 1 has towns (type D), so its
  towns would be made by a method that is then replaced, after canon maps may have been picked.
  That is the same cost D2 warns about for feature ids.

**11. Important — 00 l. 33, 36: bands 5 and 8 "Medium". Too kind.**
- Band 5 rests on a wall-retreat rule that is the agent's own design; "I found no paper that
  simulates a Zhangjiajie-type peak forest at landscape scale" (06b §2.3). It needs strata, which
  P08 defers ("Later", P08 §3, §4.4) while 06b counts it as "already planned". It needs 10 m
  cells; 06b's example of 2,400² exceeds P08's 2048² limit (P08 §7) and its cost "need[s]
  measuring" (06b §2.5). Summit settlements and the cloud sea have no phase (see 21).
- Band 8 needs P35, P37, P39, P34 and P30. Two of those are own designs with no probe and no
  paper, and band 8 was not probed (00 l. 175). A ruined city on broken plates (streets and
  buildings cut at plate borders, 06c §4 point 4) is in no phase.
Fix: "Medium–large, unproven" for 5; "Large in total, unproven" for 8.

**12. Important — 00 l. 34–35: bands 6 and 7. The scale gap is dropped.**
04 marks both: "2–8 km at 1–2 m cells exceeds the grid limit: F or child maps". Probe 6 took
215–255 s for 4 km at 5 m. 07 #12 answers "parent map with child maps (districts)". 09 moves
"zones for built kinds" to "Later". So no phase lets a city larger than one grid exist, yet step
10 claims "Bands 6 and 7". Band 6 also needs canals (04: S), docks and halls; none is in P42.

**13. Important — 00 l. 31 and 147: band 3 "complete" after P43.**
"Forest classes" is in the table's "still needed" column, and probe 3 found no swamp and forest
that thins with slope (05 l. 197–203; 04: swamps P, forest classes P). 09 has a `biome` layer in
§B but no stage and no phase that writes it. P43 has no AC on how a tree sits on its region map
(06b's `tree_roots` writes the parent's `height`, which a child map cannot do). The remark that
band 3 "is already playable as forest land" (l. 151) is the synthesis's judgement and sits badly
with Jann's decision that the trees are terrain, not scenery.

### Category 3 — G20, the Stage contract, the format's reading rules

**14. Important — 09 P32, third criterion (l. 73–74): "a failing run stops there".**
P08 §5.6 has the runner evaluate constraints after the pipeline, and CE-28.2 requires a run to
end with a manifest whose results carry every constraint and `pass`. `docs/generators.md` §3
reports a failed check as `pass: false` with the four keys, which the Maps screen shows; an early
stop turns it into an error event with no results. 06d C1 avoided this: "A recipe flag keeps
today's behaviour … for single runs", batch only. 09 dropped the flag. Also "the earliest stage
that fixes its value" cannot be derived: the constrained paths (`elevation_m.relief`, `lake_pct`)
come from the metrics module, not from a stage's `metric_names` (P08 §5.5), and later stages
write `height` again (`hydrology` fills shallow pits; `site_prep`, `wall_retreat`). Fix: make it
a batch option, and require each constraint to name its stage or each stage to declare which
summary paths it finalises.

**15. Important — 09 §D as a whole: no criterion protects existing outputs (G20, CE-77.2).**
P31 changes the reads of every placement, routing and render stage; P32 changes the runner and
seed derivation ("The derivation rule is part of the generator version", P08 §5.7); P36 adds
optional reads to `tectonics`, `geology`, `climate_model`; P38 changes `geology`'s outputs
(06b §2.5 point 3). Each needs either "recipes that do not use the new thing keep their golden
hashes" or a new generator version with new golden files. No draft AC says which. 00 raises the
matching concern only once, for feature ids (D2). Fix: one AC per phase that touches an existing
stage.

**16. Important — 09 P37 with 06c §4 point 2: damage thresholds from "a hash of the feature id".**
P08 §5.9 and §5.10: feature ids are assigned by the runner after the pipeline; stages see
internal handles only. A stage cannot hash an id. Fix: hash a stable handle or the footprint's
integer centroid, and say so in P37.

**17. Important — 00 D2 against 09 §B: rename or add?**
D2 speaks of "renaming `extraction` and the portals". 09 §B adds `chime`, `gate-up`, `gate-down`
as new types. Removing documented types is not an addition under MP §6; keeping both gives two
types for one thing. Either way P08's built-in recipes (`"features.extraction" = [2, 2]`), the
`travel.extraction_to_extraction_min` path, the parity fixture and `docs/generators.md` §3 name
the old types. The cheap option is missing from D2: put only the final type names into the P08
amendment now (P08 is unbuilt), and leave the void mask and the arena for P31.

### Category 4 — dependencies and ordering

**18. Important — 09 P31 (step 1) needs work from steps 2, 3, 4 and 6.**
- The guardian arena: 00 G-1 calls it "a requirement" (P32). 08 builds it from "Metric 'largest
  flat disc', a requirement, and `site_prep`" = M1 (P33), C4 (P32), A4 (P36).
- "A floor marked as a guardian floor" needs a recipe input; P08 allows none (§5.5) and the
  `floor` input is P32.
- `void`, `frame.edges`, the Bore fields and the new `poi` types are format 1.1, which is P34
  (step 4). P31 would write them into packages marked 1.0, or 1.1 arrives in pieces across P31,
  P34, P35, P37, P38, P40, P43.
Fix: either move the format part first, or define several minor versions and say which phase
writes which.

**19. Important — 09 §C and P32, P33, P36: the protocol change has no phase, and P09 and P16 are
not named.**
`docs/generators.md` is protected and binding. New input types need: the adapter and
`theos gen run` to pass inputs (CE-30.4 has no such flag), and the Maps screen to draw a field
per type (CE-45.4, G11). A `layers` input needs a file picker or painter; `anchors` need a map.
08 says the batch tool "belongs [in P09] or directly after it"; 09 P33 names no dependency. Fix:
give §C an owning phase with ACs, and add P09 (and P16 for the UI) to the needs of P32, P33, P36.

**20. Important — 09 P35, P37, P38, P40, P43: needs left out.**
P35 builds on the settlement kind and child maps (P29) and names no need. P37 needs P35, P29 or
P41 for interiors, and P32 for "marked as required". P38's route requirement needs P32. P40
"hands the graph to the interior stages" and 00 l. 149–150 justifies the order by it, but P40
does not name P41. P43 needs child maps (P29).

**21. Important — pieces the reports call for that sit in no phase.**
- The `districts` stage and style blending for built kinds (06c §7; 06d B3, "Large"). 00 G-13
  files "districts" under P37, whose scope is damage only; 09 defers "zones for built kinds" to
  "Later". Yet 06d §4.2 says built kinds cannot blend by interpolation at all. The band borders
  3→4, 4→5 and 5→6 (changes of map kind) have no answer in any report; 06c §7 covers 1→2, 6→7,
  7→8, 8→9, 9→10 only. Jann's "no hard cuts" is therefore unplanned for most of the tower.
- Band recipes. G1 and CE-63 put the project's recipes in `tower-of-theos` (P23). Every "Band N
  complete" needs a recipe family that someone writes there; no phase does.
- The Hem. Jann named it a test case for cities. It is in G-10 and step 5's "base for", but no
  phase has a Hem fixture or AC. P35's "ring road" is not a ring town masked to an annulus.
- Floor places on non-terrain floors. P31 extends `pois`, a terrain stage. Bands 4, 9 and 10 are
  interiors or caverns; nothing places Chimes, gates, a hub or an arena there.
- Summit siting and the cloud sea for band 5 (05 l. 290–292; 04: P), moats and compound castle
  structures for band 2 (04: S; 07 #9), canals for band 6.

### Category 5 — draft criteria that cannot be tested as written

**22. Important — 09 P34, third criterion: circular.** "Validation rejects a level layer without
a mask where the level does not cover the frame": without the mask, nothing says the level does
not cover the frame. Also "`underside` or `ceiling`", "`solid` or `present`" leave names open in
a format text, and `void` is used by 06c §5 for shafts and atria inside interiors and by 09 for
the Bore. Fix: one name each; make the mask mandatory on every level of an outdoor kind, or
define a sentinel.

**23. Important — 09 P38, first criterion: built on a metric the probe showed to be wrong.**
"Metrics from the probe's tower count" — that metric counted 10–13 "towers" on the fluted cones
the probe rejects, and 75–151 on plain ridges (05 l. 281–282). With "recipe-given" thresholds
the failing probe output would pass. "Flat tops" and "separate" have no measure. Fix: define
towers by footprint width, wall height and top flatness (06b §2.4 proposes pillar count, median
height, median footprint), with thresholds in the phase spec, and a negative fixture.

**24. Important — 09 P35: "no wall crosses water".** Band 2's stated case is a walled town on a
river with bridges (05 probe 2; the city sits on water in all four seeds). A closed wall around a
town astride a river must cross it. The defect to forbid is the convex-hull wall running 2.5 km
over a lake (05 l. 308). Also "the best-scoring defensible site" is true by construction, "a
wall that follows the terrain" has no measure, and "a measured grid index for each" has no
bounds.

**25. Important — other criteria with no test.**
- P31 l. 56–57: "the Bore probe's seeds" are numpy seeds of 0.3.0; the Rust generator has other
  random streams (P08 §5.7), so those seeds mean nothing there. The scope's rim edges and "drain
  inside the map" have no criterion.
- P33 l. 87: cites "`06d` §3.2", a research file outside the repo, which also lists more than
  the four named stages. l. 93–94: "a spec-set share … after calibration" — 06d C2 solves the
  drivers per seed and writes them into the recipe, so the criterion measures recipes that were
  just fitted to pass, and they are no longer the built-in recipes. C2's monotonicity "must [be]
  confirm[ed]" by a probe (06d l. 207–208); 09 lists only the visibility probe.
- P37 l. 143: "amount 0 leaves the package unchanged" — unchanged against what? The recipe in
  the manifest differs from one without the stage, and every building gains `state`.
- P39 l. 167–168: plates "cover the source terrain's area" after cracks and drift cannot hold
  literally.
- P40 l. 179: "no straight walls" has no measure; `worked` 0.5, the blend Jann asked for, has a
  golden hash but no behaviour.
- P41 l. 193: "wraps without a seam" has no measure.
- No criterion anywhere tests "no hard cuts". 06d §5.2 names the test (metric-versus-floor
  plots); P33 writes the table but sets no bound on the step between neighbouring floors.

### Category 6 — own designs and unopened sources

**26. Important — 00 l. 93–100: the "published" list hides that its core papers were not
opened.**
Chen et al. 2008 and Vanegas et al. 2012 are "[hit]" (found by search, body not opened); Dormans
2010 and 2017 are "[hit]" (a blog post about Unexplored was opened); Runions et al. 2007 is
"[title]" (06c and 06b source lists). 00 singles out only Schott 2023 as "must be read in full".
P35, P41 and P43 would be specified from memory of their main source. The phrase "matching P08's
family" is true for Schott only. The list of own designs is also short: add the cave graph drawn
into slab tiers and the Dijkstra replacement of the paper's search (06b §1.3, §1.4), polar ring
interiors (06c §6: "no paper"), the requirement placement and its repair, the calibration
drivers, the uplift-inversion loop ("my reading of the paper's project page", 06d A2), field
mixing known from a project page, and the gameplay metric set ("proposals, not citations", 06d
§3.1; 08 says so, 00 does not).
Also 00 l. 176 says probes "are listed per phase in `09`", but P41 and P42 list none, although
ring layouts and tiers are named own designs (06c §2: "the least proven design in this report").

### Category 7 — decisions for Jann

**27. Important — decisions that are missing.**
1. Guardian arena: on every floor or on marked floors? The owner's decision as given to this
   review is a gate up with a guardian arena on every floor. 04 has both "gate up with guardian
   arena" and "on every tenth floor"; 08 says every tenth; 09 P31 says "a floor marked as a
   guardian floor"; 00 step 1 says "Every floor has … an arena".
2. Floor and map. Jann decided a map shows a region of a floor by default. P31's criteria are
   for a whole-floor map (round outline and a hole). `frame.edges` "per side" cannot describe a
   rim that curves through a region. Nothing says how regions of one floor meet, which region
   carries the gates and the hub, or whether a region is a child of a coarse floor map (P29's
   child maps take a settlement or a ruin only). The batch estimate assumes one map per floor.
3. P29's interior and P30: port or state of the art? D5 asks this for settlements only and
   answers "keep" for interiors, against 05's conclusion 6 and although 06c §5 says the mission
   graph "is P30". D5 also does not say that replacing the settlement method changes CE-78.1 and
   CE-78.3 (tolerances around 0.3.0), which only Jann can change (06c §3, last paragraph).
4. Amend format 1.0 now, or 1.1 or 2.0 later (finding 1).
5. Re-cut P30 (finding 2).
6. The 2 km limit for levels and the 2048² grid against bands 4, 6, 7, 9.
7. The protocol additions of §C (a protected document; 09 states them without a decision row).
8. Where the band recipes live and which phase writes them.

**28. Important — presented as settled.**
- 09 §B l. 42: "Ruled out: volumes … and meshes". D4 is still open, and no slab probe exists
  (06b §8: "No probe was run"). 07 #14 kept a separate optional `volumes` index as the fallback
  "if findings 1–3 prove too weak". Recommend "not now; decide after the cavern and plate
  probes".
- 09 P31 l. 58: "one Chime lies in the hub settlement" is a design rule; its source (`03`) could
  not be checked here. Mark where it comes from.
- 00 D1: "Docs only, no change of scope". §A.7 adds a validation rule to P08 and §A.2 offers a
  new check; both are behaviour with tests. §A.7 has no rule a validator could apply ("thresholds
  assume another map scale"), and it comes from the probes, not from "the audit's fixes".

---

## Minor

**29. Minor — 00 l. 55: "62 runs of 17 recipes".** `probes/results/` holds 63 result records;
`probes/recipes/` holds 13 probe recipes plus 5 blend steps, with 2 baseline recipes in the
toolset. Neither number can be rebuilt from the files. State how they were counted.

**30. Minor — 00 l. 69–71.** The blend figures come from two seeds; say so. "4–9 km" rounds
3.7–8.7 km (05 l. 381–390), on a 16 km map.

**31. Minor — 00 l. 104–125.** The register is labelled G-1 to G-20 beside the umbrella's
constraints G1 to G20 (the brief calls G20 "the generator's rule"). "G-20 Giant trees" will be
misread. Rename, for example "M-1".

**32. Minor — 09 §A against 06a §4.** 06a's item 7 (split the Horizon Zero Dawn quotation in
decision 0003) is dropped without a note. §A.4 softens 06a's item 5 (name the model
"Thornthwaite Memorial model", drop Adams et al. 2004). 06a's note that Schott 2024's code needs
OpenGL compute shaders and cannot be reused under G20 (#14) is missing.

**33. Minor — 09 §B is not the full list.** Missing against the reports: `room` spanning levels
(`level_top`, `height_m`), `link.rooms`, `connection.via`, link type `gantry` (06c §5); `deck`
mask and level `thickness_m` (06c §2); `tower`, `ward`, `wall.height_m`, `gate.street` (06c §1)
or 07's `structure` kind; `dock`, `station`; `street.state`, `storeys_left`, `was_use` (06c §4);
`platform` and `requirement` (06d); tree types `trunk`, `limb`, `platform` (06b §4.3); the map
kinds `cavern` and `tree` themselves. P41's "multi-level halls" and P35's "keep" have nothing to
be stored in. 07 said "the synthesis carries the final list".

**34. Minor — 06c and 06d propose two blend inputs.** 06c §7 uses `t` in 0–1 per band; 06d §4.3
uses `floor` with curves. 09 takes `floor` without saying `t` is dropped.

**35. Minor — 00 G-9 and G-15 against 06b.** 06b §2.4 keeps band 5 on a heightfield and says
bridges between pillars "already exist" as `site`/`bridge` with `deck_m`. 00 lists band 5 under
"ground above ground" and P38 speaks of "links between slabs". P38's scope also carries
`dissolution` and `landforms_3d` with no criterion.

**36. Minor — 09 P39 and P38 both define links** (`slab_links`; "bridges and stairs as links
between slabs"). One stage should own them.

**37. Minor — 09 P36.** A package made with a control layer can no longer be rebuilt from its
own manifest (decision 0003 §3 opening); only the hash is kept. Say where the layer lives (06d
§1.2 condition 3) and add an AC that a missing or changed layer fails the run by name.
`uplift_from_target` is in scope "later" with no criterion.

**38. Minor — 09 P32, requirement example.** Chimes placed by walking time need the travel field,
which needs roads, which are routed between settlements and portals (P08 §5.9). The order of the
requirement stage against `roads` and `travel` is not stated.

**39. Minor — 09 P43.** Two criteria for a new map kind of about seven stages (06b §4.2): nothing
on limb width, tiers that do not cross in plan, platforms at designer points, or the parent map.

**40. Minor — 00 l. 170–172.** "I checked their reports against [`decisions.md`]: nothing
conflicts" is stronger than the reports allow: 06b assumed a 24 km whole-floor terrain and 2 km
interiors "from the repo only" (§0), 06d flags "the real Chime rules" and the rim geometry as
likely overrides, and findings 27.1 and 27.2 look like conflicts.

**41. Minor — 09 intro.** New phases carry no repo, slice or "Depends on" column, which the
umbrella's phase table requires; "spec-set" and "recipe-given" thresholds appear in most criteria,
so the criteria defer their own content.
