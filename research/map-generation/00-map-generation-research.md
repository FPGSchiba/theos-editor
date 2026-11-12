# Map generation research — synthesis

11 Nov 2026 · for Jann · joins the P08 terrain generator with the ten themed bands.

Reports behind this page: `01` capability baseline · `02` band requirements · `03` floor frame ·
`decisions.md` (Jann, checkpoint 1) · `04` readiness matrix · `05` probes · `06a` audit ·
`06b` natural terrain · `06c` built environments · `06d` steering and blending · `07` package
headroom · `08` throughput and gameplay · `09` proposed phases · `review-round-1.md` and later
rounds (the reviews of this page).

## The answer in short

- **P08 builds the right foundation, for one band.** Its stage-and-layer model held up in the
  probes and the audit. Its content is a river landscape: band 1, and the open land of bands 2
  and 3.
- **Seven bands are blocked by terrain or structure the generator cannot make.** The research
  recommends new stages and two new map kinds (`cavern`, `tree`), not a new architecture. By
  design every recommended method fits the Stage contract and can be bit-identical (G20); none of
  that is proven until the probes listed in `09` have run, and levels need one extension of the
  contract (layers addressed by level).
- **Every band also lacks three things that are not terrain:** the floor's places (Chimes, gates,
  guardian arena), recipes that take a floor number or a requirement, and a way to review many
  seeds.
- **The package format needs one real change and many optional additions.** The real change is
  levels on outdoor maps ("slabs") and a mask for where there is no ground. That changes what
  existing fields mean, so it is not a minor version. It is cheapest as an amendment to 1.0 now,
  while no package and no reader exists.
- **The order has a dependency Jann should know about:** most band-specific work needs levels in
  the generator, which P30 builds, and P30 waits for the Atlas (P25). `09` proposes to split P30.

## Readiness per band

"After P29 and P30" assumes both are built as the umbrella describes them today.

| Band | After P08 | After P29 and P30 | Still needed | Size of the rest |
|---|---|---|---|---|
| 1 Hearthlands | Regions | Towns, ruins | Floor places, recipe family, metrics | Medium |
| 2 Rune Kingdoms | Open land | Towns of the old method | Castles, walls that follow terrain, moats | Medium |
| 3 Wildwood | Open land | Camps | Forest classes and swamp; giant trees (new kind) | Large |
| 4 Underdeep | — | Rooms on levels, not yet a labyrinth | Interior structure; cavern kind; the blend between them | Large |
| 5 Thousand Peaks | Steep cones only | Halls | Needle peaks, summit sites, cloud sea, links over gaps | Medium–large, unproven |
| 6 Brass Cities | — | — | Street core, tiers, canals, city scale | Large |
| 7 Metropolis | — | — | Street core, grid, transit, city scale | Medium–large |
| 8 Shatter | — | — | Street core, ruin pass, floating plates, and the two combined | Large, unproven |
| 9 Works | — | Small halls of the old method | Interior structure on many levels; floors above 2 km | Large |
| 10 Instrument | — | — | Ring layout on top of band 9's work | Small on top of 9 |

## What the evidence says

### The generator's design (audit, `06a`)

- 40 claims checked: 22 confirmed, 11 weaker than stated, 1 wrong, 6 not checkable. Several
  "confirmed" verdicts rest on abstracts or memory, and the "wrong" verdict rests on a paper seen
  at abstract level; the audit marks each.
- No constant was found misquoted. Two formulas could not be compared with their primary source
  (the plant-growth formula and the river width and depth exponents).
- Nothing from 2023–2026 forces a redesign. One paper (McDonald & Cordonnier 2026) suggests that
  alluvial forms need more than P08's fluvial law, so the spec should promise less there.
- To correct in the P08 spec: the fluvial law is ambiguous about how rain enters; four citations
  do not carry the method the spec builds; "keeps rivers narrow" does not hold on flat ground;
  "no neural inference is bit-identical" is too strong.

### What the old toolset shows (probes, `05`)

The probe run reports 62 runs of 17 recipes on `theos_maps` 0.3.0. That tool lacks deposition and
multi-neighbour flow, so lake and river results say nothing about P08's maps. The structural
findings carry over.

- **Absolute thresholds break at other scales.** At 4 km the river threshold, the mill rule (a fixed
  discharge) and the placement margin of 0.3.0 fail. P08 keeps the river threshold as an absolute
  discharge and takes over 0.3.0's site rules.
- **Seed variance was very large in 0.3.0:** one recipe gave 327–1,059 m of relief and 3–32 %
  lakes. The pass rate of 2 of 8 is 0.3.0's; P08's is not measured yet.
- **Needle peaks do not come out of river erosion.** The probe made fluted cones 5–7 km high.
- **The Bore can be added as a drain inside the map** (40 lines in the probe). Every later stage
  ignored it: places and roads landed in the hole in 3 of 4 seeds.
- **Interiors and settlements scale in size but not in kind.** 155 rooms on 1 km and 4,765
  buildings in a 3 km settlement ran, but as rooms and corridors on one level that fill 10–18 % of their footprint, and as organic lanes
  with at most three storeys. That settlement took about four minutes.
- **Interpolating recipe numbers gives a drift of character, not the same land changing.** Over
  five steps from band 1 to band 3, on two seeds, height correlation between steps was 0.71–0.95,
  rivers shared 1–9 % of their cells, and settlements moved 3.7–8.7 km on a 16 km map. Anchors
  and fixed river courses, which the research proposes, were not tested.

### State of the art for what is missing (`06b`, `06c`, `06d`)

| Need | Recommended method | Where it goes | Size | Basis |
|---|---|---|---|---|
| Streets for towns, grids, ring roads | Tensor-field streets, blocks, parcels, massing as attributes | Stages, settlement kind | Medium | Published (Chen 2008, Vanegas 2012); papers not opened |
| Castles and walls | Wall as a least-cost loop; castle site by score | Stages | Medium | Composition |
| Style districts | Districts that switch style by a threshold | Stage | Small | Own design |
| Ruined places | Generate intact, then a damage pass that keeps connectivity | Stages, both built kinds | Medium | Own design, no paper found |
| City tiers | Tiers as levels of a settlement, with links | Stages, after levels exist | Medium–large | Own design; "the least proven" (`06c`) |
| Interior structure | Cyclic mission graph, partition with multi-level halls | Stages, interior kind | Large | Published (Dormans); papers not opened |
| Ring galleries | Radial street field; polar mapping for interiors | Parameters | Small | Own design |
| Caverns | 3D graph of chambers and passages, drawn into slab levels | New kind `cavern` | Large | Graph: Paris 2021 (read at project-page level; its path search replaced by an own design); slab drawing and the blend to labyrinth: own design |
| Needle peaks | Rock joints, strata, wall retreat on a fine grid | Terrain stages | Medium–large | Geology: May 2018; the rule: own design |
| Floating ground | Plates cut by cracks, displaced, with undersides | Terrain stages on slabs | Medium | Own design |
| Giant trees | Grown branch skeleton with walkable tiers | New kind `tree` | Large | Skeleton: Runions 2007 (not opened); walkability: own design |
| Steering | Control layers, anchors, requirements placed first | Recipe, protocol, stages | Small to medium each | Schott 2023 (read at project-page level); placement and repair: own design |
| Blending land | Curves over the floor number; mixing input fields before erosion | Recipe; one stage | Small; medium | Schott 2023, as above |
| Map metrics | Routes, chokepoints, sightlines, cover | Stages | Medium | General level metrics; the set is a proposal, none exists for extraction shooters |
| 100 floors | Recipe family per band, batch, ranking, sheets | Tool layer | Small–medium | Practice reports; calibration is own design |

Before any of these becomes a phase spec, its main source must be read in full and its probe
run. `09` names the probe per phase.

### What no report answers

- **Band borders that change the kind of map** (3 to 4, 4 to 5, 5 to 6): how forest
  becomes cave, cave becomes peaks, peaks become city. The reports cover blends within terrain
  and within built places only.
- **A ruined city on floating plates** (band 8): the ruin pass and the plates are designed apart.
- **How regions of one floor relate** when a floor is larger than one map.
- **Cities larger than one grid** (bands 6 and 7 at 2–8 km): the answer on paper is a coarse
  parent map with district child maps; nothing tested it.

## Gap register

| # | Gap | Bands | Evidence | Answer | Proposed phase |
|---|---|---|---|---|---|
| M-1 | Floor places: Chime, gates, guardian arena, hub | all | `03`, `08` | Final type names now; placement rules later | P08 amendment, P31, P32 |
| M-2 | Void mask, rim and Bore that every stage honours | all | `05` probe 7 | A mask layer, edges, drain inside the map | Format amendment, P31 |
| M-3 | Thresholds as absolute numbers | 1 to 3 | `05` probe 1 | Thresholds relative to the map, or stated per recipe | P08 amendment (Jann) |
| M-4 | Recipes take no inputs; one file per floor | all | `08`, `06d` | `floor` input, curves, `extends`, weights | P32 |
| M-5 | Seed pass rate unknown for P08; no retry | all | P08 App. A | Measure after P08; batch checks, stage re-rolls | P32, P33 |
| M-6 | Lore cannot fix a place or a shape | all | `06d` | Requirements, anchors, control layers | P32, P36 |
| M-7 | No metrics for routes, chokepoints, sightlines, cover | all | `08` | Metric stages | P33 |
| M-8 | No tool for batches of floors | all | `08` | Batch run, ranking, sheets | P33 |
| M-9 | Ground above ground cannot be stored | 3, 4, 8; tiers of 6 | `07`, `06b` | Slabs | Format amendment |
| M-10 | Levels exist only for indoor maps up to 2 km | 4, 6, 7, 9 | `04`, `07` | Parent and child maps; or raise the limit | P42 for cities; none for bands 4 and 9 (D8) |
| M-11 | Settlement method stops at organic towns | 2, 6, 7, 8, Hem | `05` probe 6 | Street core | P35 |
| M-12 | Castles, moats, terrain-following walls | 2 | `05` probe 2 | Fortify stages | P35 |
| M-13 | Blending of built styles | borders 1–2, 6–7, 7–8; 8–9 and 9–10 claimed by `06c` §7 for interiors, with no criterion yet | `06c` §7 | Districts | P35; P41 for interiors |
| M-14 | Blending two terrain types on one map | borders within terrain | `05` probe 8, `06d` | Field mixing | P36 |
| M-15 | Borders that change map kind | 3–4, 4–5, 5–6 | none | Open; see "What no report answers" | none yet |
| M-16 | No ruin pass | 8, ruins everywhere | `06c` | Damage stages | P37 |
| M-17 | Needle peaks, summit sites, cloud sea | 5 | `05` probe 5, `06b` | Joints, strata, wall retreat; site rule | P38 |
| M-18 | Floating broken ground | 8 | `06b` | Plate stages and links | P39 |
| M-19 | Cavern networks | 4 | `05` probe 4, `06b` | Kind `cavern` | P40 |
| M-20 | Interiors are rooms and corridors on one level, thinly packed | 4, 9, 10 | `05` probe 4, `06c` | Mission graph, halls, ring layout | P41 |
| M-21 | City tiers, transit, canals, city scale | 6, 7 | `06c` | Settlement levels, rail, district child maps | P42 |
| M-22 | Forest classes, swamp | 3 | `05` probe 3 | A `biome` stage | P38 |
| M-23 | Giant trees | 3 | `06b` | Kind `tree` | P43 |
| M-24 | Band recipe families in the content project | all | umbrella G1, CE-63 | Each band phase delivers its family to `tower-of-theos` | with each phase |

## Proposed order

Existing phases are shown with their umbrella dependencies. "P30a" and "P30b" are the proposed
split of P30 (decision D6).

| Step | Phase | Needs | What it unlocks |
|---|---|---|---|
| Now | Amendments to the P08 spec and to format 1.0 | Jann | A correct spec; final place names; slabs and the void in the format before anything reads it. The format part adds work to P08's `theos-map-format` (`09` §A3) |
| — | P08 Map format and terrain generator | P00 | Band 1 regions |
| — | P09 Generator layer, `theos gen` | P07, P08 | Runs from the editor and the command line |
| 1 | P31 Floor frame and places | P08 | The Bore, the rim, Chimes and gates on terrain |
| 2 | P32 Recipe families and requirements | P09, P31 | One recipe family per band; lore demands; guardian arenas |
| 3 | P33 Map metrics and batch tool | P32 | Review of many floors; a test for "no hard cuts" |
| — | P29 Settlements, interiors, child maps | P08 | Towns and ruins |
| — | **Band 1 designable** | | with the band 1 recipe family |
| 4 | P35 Street core, fortification, districts | P29, P31, P32 | **Band 2**; the base for 6, 7, 8 and the Hem |
| 5 | P36 Steering by layers, field mixing | P32 | Shaped floors; two terrain types on one map |
| — | P30a Levels in the world, runner and generator | P29 | Steps 6, 7, 9, 10 (plates) and 11 |
| 6 | P41 Interior structure | P30a, P31, P32 | **Upper band 4 and band 9 on floors up to 2 km; band 10** |
| 7 | P40 Cavern kind | P30a, P41 | **Band 4 on floors up to 2 km** |
| 8 | P38 Needle peaks and forest classes | P36 | **Band 5**; forest classes and swamp for band 3 |
| 9 | P42 City tiers, transit, city scale | P30a, P35 | **Bands 6 and 7** |
| 10 | P37 Ruin pass, then P39 floating ground | P35, P32, P41 for interiors; P30a, P32 | **Band 8, ruins and plates apart**; a ruined city on plates has no design yet |
| 11 | P43 Giant trees | P30a, P29, P38 | **Band 3** |
| — | P30b Atlas level switcher | P25 | Showing levels in the editor |

The table shows the case where P29 stays as the umbrella describes it, which D5 recommends
against. If Jann says yes to D5, P29 itself takes the street core and the shared room graph, P35
shrinks to fortification and districts, single-level interiors stay a port until P41, and band 1
becomes designable later, because P29's spec then waits for the street probes. CE-78.1 and CE-78.3
change in that case.

Steps 4 to 11 follow the climb with two moves: interiors come before caverns because caverns hand
their graph to the interior stages, and giant trees come last because they are the largest piece
with the least support. Until then band 3 has forest land without its climbed trees, which does
not meet Jann's decision that the trees are terrain.

## Decisions for Jann

| # | Decision | Recommendation |
|---|---|---|
| D1 | Correct the P08 spec's text before its build (`09` §A1)? | Yes |
| D2 | Take the behaviour changes of `09` §A2 into P08 (final place type names, relative thresholds, a river-spread check)? | Place names: yes, now. P08 converts `extraction` and the portals from 0.3.0, and renaming later changes feature ids that pins bind to. The other two: your call |
| D3 | Slabs and the void mask: amend format 1.0 now, or a 2.0 later? | Amend 1.0 now. No package and no reader exists. Later it is a new major version, and the blockout tool and the engine would already read 1.0 |
| D4 | Stored volumes (voxels, distance fields) in the package | Not now. Decide after the cavern and plate probes show whether slabs are enough |
| D5 | Is P29 a port of 0.3.0, or state of the art as P08 became? This covers settlements and interiors, and changes CE-78.1 and CE-78.3 | State of the art for both. The probes show the old settlement and interior methods cannot reach bands 4 to 10 with any parameter. If P29 stays a port, P35 and P41 replace its methods later, after maps may have been picked |
| D6 | Split P30 into levels in the generator (after P29) and the Atlas level switcher (after P25)? | Yes. Otherwise bands 3 to 10 wait for most of the editor's screens |
| D7 | Make rooms and connections a value other kinds can write (in P29)? | Yes; the cavern-to-labyrinth blend needs it |
| D8 | The 2 km limit for levels (your statement of 10 Nov, decision 0003 §7) and the 2048² grid, against the scales you accepted for bands 4, 6, 7 and 9 | Keep both; large floors are a coarse parent map with child maps. Needs a probe |
| D9 | What "no hard cuts" means for maps: a) character drifts over floors; b) a border floor mixes two types; c) the same land carries on from floor to floor | a) and b). c) is not reached by parameter curves alone and is untested with anchors and fixed river courses |
| D10 | Guardians: an arena at every floor's gate up, and a larger one on every tenth floor? | That is how I read the Tower proposal and the bands decision together; please confirm |
| D11 | The working assumption since checkpoint 1, which you have not confirmed, is that a map shows a region of a floor. The drafted phases still test whole-floor maps in places. If a floor has several region maps: which carries the gates and the hub, and how do regions meet? | Keep "a region by default". Start with one region per floor that carries the hub and both gates; the `void` mask marks where the rim or the Bore crosses it. More regions per floor later |
| D12 | New input types in the generator protocol (`09` §C) | Yes, as additions to protocol 1 |
| D13 | Where band recipes live and who writes them | `tower-of-theos`, delivered with each band's phase |
| D14 | May giant trees come last? | Yes, if band 3 without them is acceptable for a while |
| D15 | The order above | By the climb, shared work first |

## Limits of this research

- Nothing new was built or run on the Rust generator, which does not exist yet.
- Core papers for streets, parcels, mission graphs and trees were found but not opened; Schott
  2023 was read at project-page level. The reports mark every source.
- The four research agents could not read `decisions.md`. Their reports cover its points (the
  cavern-to-labyrinth run, climbed trees, the Hem, loose physics), but `06b` assumed a 24 km
  floor and 2 km interiors from the repo, and `06d` flags the Chime rules and the rim's geometry
  as places where your decisions may override it. D10 and D11 are the two points where I found
  the reports and your decisions not yet aligned.
- Three probe sheets were not looked at (`p4c`, `p5x`, `p8` seed 2); bands 8 and 10 had no probe.
- Two fresh reviews checked this page and `09`. Round 1 found 41 points (2 critical), round 2 found
  21 new ones (none critical) and 14 of the first round only partly resolved. This version
  answers the important ones of both rounds. The fixes made after round 2 have not been reviewed
  again; the `README` lists what stayed open.
