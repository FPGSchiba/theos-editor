# Review round 2 — `00-map-generation-research.md` and `09-proposed-phases.md`

Reviewer: fresh, 12 Nov 2026. Read in full: `BRIEF.md`, `00`, `09`, `review-round-1.md`, `04`,
`07`, `08` (with their correction notes), `docs/map-package.md`. Read in part: `05` (summary,
method, probes 1b, 4, 6, 7b, 8, conclusions), `06a` (summary, §1.1, §1.4, §4, sources), `06b`
(§0–§1, §2.3–§4.3, §6–§8, sources), `06c` (summary, §0, §2, §4–§9), `06d` (summary, §0, §1.3,
§2.3, §3.2, §4.2–§4.3, §5.2, §6, "Could not do"), P08 spec (§5.5–§5.7, §5.9 table rows, §5.10,
§6, §7, grep for cited terms), umbrella (§2, §3 table, P08, P09, P16, P22, P23, P25, P29, P30),
decision 0003 (§6, §7, §9 recipe lines), `docs/generators.md` §3 and §6.

Not checked: `01`, `02`, `03`, `decisions.md` (not in the folder; the owner's decisions are taken
from the review task). No probe re-run, no contact sheet opened, no external source opened. The
claim in `00` that three probe sheets were not looked at has no source in the folder and was not
verified.

Counts of new findings: **0 Critical, 9 Important, 12 Minor.**
Round-1 findings: **27 resolved, 14 partly, 0 not resolved** (finding 34 is the weakest "partly").

Abbreviations: "MP" = `docs/map-package.md`; "R1-n" = round-1 finding n; "N-n" = new finding n
below.

---

## 1. Round-1 findings

| R1 | Topic | Status | Note |
|---|---|---|---|
| 1 | Slabs are not an optional addition | Partly | §B is split into B1 and B2 and D3 offers the real choice. Left open: no phase or criterion owns B1 (N-3); the mask rule is still circular (N-3, last point); `state` on ruins fails B1's own test (N-6) |
| 2 | Order hangs on P30 | Resolved | P08, P09, P29, P30a, P30b in the table; D6 added; needs corrected |
| 3 | "Three new map kinds" | Resolved | |
| 4 | "Every method fits the contract" | Resolved | "By design … none of that is proven" |
| 5 | Pass rate as blocker; "not terrain" | Resolved | |
| 6 | "No formula is misquoted" | Resolved | |
| 7 | Gap 3 bands and evidence | Resolved | M-3 and M-10 are separate; A2.2 names `river_q` |
| 8 | "The probe shows c) fails" | Resolved | D9 |
| 9 | Band 4 after P29 and P30 | Resolved | |
| 10 | Band 1 "Small", "complete after four phases" | Partly | Size and wording fixed. Band 1's towns are still made by P29 and replaced in P35 under the order shown (N-2); the band 1 recipe family has no owner (N-10) |
| 11 | Bands 5 and 8 "Medium" | Resolved | Sizes fixed. The order table still claims "Band 8" (N-8) |
| 12 | Scale gap of bands 6 and 7 | Resolved | P42 carries parent and district maps; canals in scope without a criterion (N-17) |
| 13 | Band 3 "complete" after P43 | Partly | Forest classes moved to P38. How the tree gets onto its region map is still not buildable as written (N-7) |
| 14 | P32 "a failing run stops there" | Resolved | Batch option; stages declare final paths |
| 15 | No criterion protects golden hashes | Partly | The blanket criterion covers only "a stage P08 builds" (N-13) |
| 16 | Hash of the feature id | Resolved | Integer centroid |
| 17 | Rename or add place types | Partly | A2.1 decides "rename". B2 still files the types as an optional minor addition, and the list of texts that follow is incomplete (N-5) |
| 18 | P31 needs later work | Partly | Arena and floor input moved to P32; "Needs B1". New: P31 has a criterion on kinds that P29 builds (N-4) |
| 19 | Protocol change has no phase | Partly | §C has owners and D12. No criterion in P32 or P36 covers the adapter, `theos gen run` or the Maps screen, and P16 is in no "Needs" (N-12) |
| 20 | Needs left out | Resolved | In `09`. `00` and `09` now differ for P37 (N-14) |
| 21 | Pieces in no phase | Partly | Districts, the Hem, moats, canals, summit sites placed. Still open: floor places on interior and cavern floors have scope words but no criterion (N-11); borders that change the map kind are disclosed as open, not planned |
| 22 | P34 criterion circular; names open | Partly | One name each. The rule itself is unchanged (N-3, last point) |
| 23 | P38 tower metric | Resolved | |
| 24 | "No wall crosses water" | Resolved | |
| 25 | Criteria with no test | Partly | All fixed except P37's amount-0 criterion (N-15) |
| 26 | Unopened sources and own designs | Partly | Basis column and limits added. Paris 2021 and the Dijkstra replacement are still not disclosed (N-16) |
| 27 | Missing decisions | Partly | All eight rows exist. D11 does not present Jann's decision as his and recommends against it (N-1) |
| 28 | Presented as settled | Resolved | The Chime-in-hub source is named; not verifiable here |
| 29 | "62 runs of 17 recipes" | Resolved | "The probe run reports" |
| 30 | Blend figures | Resolved | |
| 31 | G-1 to G-20 labels | Resolved | |
| 32 | §A against `06a` §4 | Resolved | |
| 33 | §B not the full list | Resolved | Leftovers in N-18 |
| 34 | Two blend inputs (`t`, `floor`) | Partly | P35 says "a blend input" and still does not say whether it is `floor`; it also has no phase that gives it the input (N-4) |
| 35 | Band 5 and slabs | Resolved | |
| 36 | Two phases define links | Resolved | A bridge over a gap now has two forms (N-18) |
| 37 | Control layer cannot be rebuilt | Resolved | |
| 38 | Order of the requirement stage | Resolved | |
| 39 | P43 has two criteria | Resolved | |
| 40 | "Nothing conflicts" | Resolved | |
| 41 | No repo, slice or needs column | Partly | Repo and needs added; no slice; the recipe-family row has no phase id (N-10) |

---

## 2. New findings

### Important

**N-1. Important — `00` D11, "What no report answers" (third bullet); `09` P31, P33.
The owner's decision "a map shows a region of a floor by default" is asked back as an open
question, and the recommendation goes against it.**

- What is wrong: D11 asks "Is a floor one map, or several region maps?" and recommends "One map
  per floor to start; regions later". The owner has decided the opposite default. Nowhere in `00`
  or `09` is that decision stated as his. `00` l. 205–206 says only that D10 and D11 are "not yet
  aligned".
- The phases follow the recommendation, not the decision: P31 tests "a round outline and a hole"
  and "`void` equals the cells outside the outline"; `frame.edges` is "per side" (B1), which
  cannot describe a rim that curves through a region; "exactly one settlement is marked `hub`"
  and the gate types are per map; P33's batch writes "a contact sheet per floor".
- Evidence: owner's decisions in the review task; `00` l. 191, l. 113; `09` l. 63, l. 123–130,
  l. 165; R1-27.2, which raised the same point.
- Suggested fix: state the decision in D11 as Jann's. Either ask him to confirm a temporary
  exception (whole-floor maps first) with its cost, or change P31: a region carries the part of
  the rim and Bore that crosses it (the `void` mask already can; `frame.edges` cannot), and say
  which region carries the gates and the hub.

**N-2. Important — `00` D5 against the order table and `09` P35, P41.
The order is drawn for the option D5 recommends against.**

- What is wrong: D5 recommends that P29 be "state of the art for both" settlements and
  interiors, and warns: "If P29 stays a port, P35 and P41 replace its methods later, after maps
  may have been picked". The order table and `09` show exactly that case: P29 first, "Band 1
  designable" after it, then P35 ("It replaces the settlement method of P29 (D5)") and P41 ("It
  replaces the interior method of P29 and P30 (D5)").
- Nothing shows the order if Jann says yes to D5. P35 then cannot "need P29" and replace it.
  P41 cannot move into P29 either: it needs P30a (multi-level halls), and P30a needs P29. P29
  would also inherit the probes and paper reading of P35 and P41 before its spec, so "Band 1
  designable" moves later.
- Evidence: `00` l. 185, l. 159–164; `09` l. 178–179, l. 275; umbrella P29 ("the rest of the
  port"), CE-78.1, CE-78.3.
- Suggested fix: show both cases. For "state of the art": P29 = child maps, the shared room
  graph (D7) and the street core; P35 = fortification and districts on top; interiors of one
  level stay a port in P29 until P41, or P41's single-level part moves into P29. Say which
  umbrella criteria change in each case.

**N-3. Important — `09` §B1, §D ("There is no P34"); `00` order, step "Now … (docs)".
The format change of B1 has no phase and no criterion.**

- What is wrong: R1-1 asked for a criterion on what a reader does with slab packages. The
  rewrite removed P34 and made B1 a document change. But someone must build it in
  `theos-map-format`: levels on any kind, a `height` per level, `present`, `void`, `underside`,
  `ceiling`, `frame.edges`, the Bore fields, and their validation.
- If 1.0 is amended before P08's build, P08 builds it, because CE-76.1 binds its bytes to MP
  §1–§5 and CE-76.3 to the levels rules. That widens P08's scope. §A lists no such change, and
  `00` calls the step "docs". P31 writes only `void` and the edges. No phase tests `present`,
  `underside`, `ceiling` or a sloping level before P39 and P40.
- If Jann picks "2.0 later" (the other half of D3), no phase exists for it at all.
- Decision 0003 §7, `docs/generators.md` §3 ("An indoor map with storeys … adds
  `frame.levels`") and the data model's `frame.levels` also word levels as indoor. B1 lists only
  MP.
- B1 keeps the rule R1-22 called circular: `present` is "required on every level that does not
  cover the frame". Without the mask nothing says that a level does not cover the frame, so a
  validator cannot apply it.
- Evidence: `09` l. 52–66, l. 115–116; umbrella CE-76.1, CE-76.3; P08 spec §5.13 (validation of
  levels); `07` "What this means for P08's build", point 1.
- Suggested fix: add to §A a list of what P08 must build if D3 is "amend now", with draft
  criteria (round trip of a two-slab terrain package with a partial level; rejection rules), or
  keep a small format phase. List all four documents B1 touches. Make `present` mandatory on
  every level of a map whose kind is not `interior`, or on every level.

**N-4. Important — `09` P31 and P35: needs that the criteria imply but the table omits.**

- P31 (needs P08, B1) has the criterion "Terrain, settlement and interior kinds can each carry
  `chime`, `gate-up` and `gate-down`". Settlement and interior kinds are P29 (CE-78.1). Step 1
  cannot prove it.
- P31: "one Chime lies inside [the hub settlement]". On a terrain map a settlement is a Point
  (MP §5), so "inside" has no measure.
- P35 (needs P29) has a Hem fixture "on an annulus with a `void` centre"; `void` is written by
  P31. It also has "With a blend input …"; P08 recipes take no inputs (P08 §5.5, decision 0003
  §9), and number inputs are owned by P32 (`09` §C). So P35 needs P31 and P32, and `00` step 4
  needs them too.
- P39 (needs P30a) tests "every plate that carries a required place"; requirements are P32.
- P41's scope has "floor places in interiors" (P31) and lock and key over requirements; it lists
  P30a only.
- Evidence: `09` l. 101, 104, 109–110, 127–130, 189–191, 252, 273–275; MP §5 table.
- Suggested fix: move the settlement and interior part of the P31 criterion to P29 or to P35 and
  P41; give "inside" a radius; add P31 and P32 to P35, P32 to P39, P31 to P41, and repeat it in
  `00`'s table.

**N-5. Important — `09` A2.1 and B2 first row; `00` D2.
The place-type rename is filed as an optional minor addition, and the texts that must change
with it are not all named.**

- A2.1 replaces `extraction`, `portal-out`, `portal-in`. Removing types that MP §5 lists is not
  "adds optional things" (MP §6). It is an amendment of 1.0, like B1, and only valid while 1.0 is
  unbuilt. B2 lists it under "a minor version whenever a phase needs them" with the phase
  "P08 amendment".
- A2.1 names the recipes' constraints, the travel metric and the parity fixture. It leaves out:
  MP §5 (`poi` row), `docs/generators.md` §3 (the type list names `portal-in`, `portal-out`,
  `extraction`; a protected, binding document), P08 §5.9 (`pois`, `travel`: "multi-source
  Dijkstra from extraction points"), P08 §5.11 (`truth`: "circle extractions, star portals").
- Evidence: MP §5, §6; `docs/generators.md` §3; P08 l. 451, 621, 626, 790.
- Suggested fix: move the three types to B1 (or to a "B0: amend the 1.0 type list") and complete
  the list in A2.1.

**N-6. Important — `09` B2, rows P37 and P39.
By B1's own test, the ruin properties are not optional, and P39's row mixes two designs.**

- B1's test is "A 1.0 reader that ignored these would build a wrong map". `06c` §4 keeps the
  footprint and `storeys` of a ruined building and adds `state`, `storeys_left`, `was_use`; a
  street that is `gone` stays a street feature with `street.state`. A reader that ignores them
  (the blockout, CE-64.2: "buildings, streets and walls match the package's features") builds
  the intact town.
- P39's scope and evidence are `06b` §3 (plates as slab levels with `underside` and `present`).
  Its B2 row is `06c` §4's other design ("Kind `fragment` with `offset_m`"). If the level's
  `height` carries the plate, `offset_m` repeats it; if it does not, ignoring `fragment` gives
  unbroken ground.
- Evidence: `06c` l. 281–309 and its format row; `06b` §3.2, §3.3 point 4; `09` l. 75, 77,
  248–252.
- Suggested fix: let existing fields carry the ruined state (`storeys` = what is left, removed
  streets are absent or typed `rubble`) and put the original in new optional fields
  (`was_storeys`, `was_use`). For P39 say that the level layers are the truth and `fragment` is
  an optional outline.

**N-7. Important — `09` P43 (l. 301–302), B2 rows P38 and P43.
A tree as "a child map of a `landform` feature on its region" cannot be built from what the
phases define.**

- `landform` is introduced by P38 with the types `pillar` and `arch`. P43 does not need P38, and
  no type is given for a tree site.
- No stage writes that feature on the region map. `06b` §4.2 did it in `tree_roots`, which also
  wrote the parent's `height`; `09` drops the height change and with it the stage. A child run
  cannot write its parent.
- Child maps take "a settlement or a ruin" (CE-78.2).
- B2 lists "types `trunk`, `limb`" without a kind (`06b` §4.3: kind `tree`).
- Evidence: `09` l. 76, 81, 112, 298–307; `06b` l. 357–362, 381; umbrella CE-78.2.
- Suggested fix: add a terrain stage in P43 (or in the forest part of P38) that places tree
  sites as a named feature type, a criterion that the child's trunk sits on it within a
  tolerance, and the widening of CE-78.2's feature types.

**N-8. Important — `00` order table, steps 6, 7 and 10; M-10; D8.
Three "unlocked" claims are stronger than the same page's own lists.**

- Step 10 says "**Band 8**". "What no report answers" and `09` "Not planned here" both say a
  ruined city on floating plates has no design; band 8 is types E and F together (brief).
- Step 6 says "band 9" and step 7 "**Band 4**". `04` gives band 9 as 1–4 km and band 4 as up to
  5 km against the 2 km limit for levels. D8 answers with "a coarse parent map with child maps",
  but only P42 builds and probes that, for cities. P40 and P41 have no scale criterion and no
  probe, and M-10 points at "Decision D8", not at a phase. The readiness table itself lists
  "floors above 2 km" as still needed for band 9.
- Evidence: `00` l. 45, 112, 130, 164–168, 188; `04` Scale column; `09` l. 256–283, 319–320.
- Suggested fix: write "Band 8, plates and ruins apart", "Bands 4 and 9 up to 2 km"; give M-10 a
  phase or a row in "Not planned here".

**N-9. Important — `00` l. 76–77, M-20; `09` P41 first criterion.
"One loop of rooms" disagrees with the probe's own table, and the cycle criterion does not
separate the new method from the old one.**

- `05` probe 4 measured 7 cycles at 62 rooms and 17 at 155 (table column "Cycles"; 68 − 62 + 1
  and 171 − 155 + 1 agree). `05`'s prose says "exactly one main cycle … regardless of size".
  `00` took the prose.
- P41: "A hall recipe yields a room graph with at least the asked number of independent
  cycles". 0.3.0 already yields 7 to 17, so the old method passes this criterion.
- Evidence: `05` l. 215–221, 233–236; `00` l. 76–77, 140; `09` l. 277.
- Suggested fix: in `00`, "one main cycle with a lock; the other cycles are short". In P41, test
  what the mission graph adds: nested cycles with their own locks, a bound on dead-end share,
  footprint use above the probe's 10–11 % (as a negative fixture, as P38 does).

### Minor

**N-10. Minor — `09` §D table, last row; `00` M-24, "Band 1 designable".**
The recipe families have no phase id, and band 1 has no "band phase" to deliver its family (its
work is P31–P33). P23, which the row needs and which itself needs P29, is not in `00`'s order
table. CE-63.1's folder is `tools/maps/recipes/`, not `tools/maps/`.

**N-11. Minor — `09` P40, P41: floor places on cavern and interior floors.**
Both scopes say "floor places"; neither has a criterion. "Every floor has a hub settlement" has
no form on an interior map (`hub` is "on a settlement", B2), and the arena requirement of P32 is
defined by slope on a terrain raster.

**N-12. Minor — `09` §C, P32, P36.**
§C says each input type needs the adapter, `theos gen run` (CE-30.4 has no input flag) and a
field on the Maps screen (CE-45.4). No criterion in P32 or P36 tests any of the three, and P16
is in no "Needs". `00` names the P25 chain as "a dependency Jann should know about" but not that
P32 (and through it P33, P36, P37, P38) sits behind P09, which needs P07.

**N-13. Minor — `09` intro, third bullet.**
The golden-hash criterion applies to "every phase that changes a stage P08 builds". P35, P41,
P37 and P42 change or replace stages of P29 and P30, whose hashes CE-78.4 and CE-79.4 cover.
Say "a stage an earlier phase builds".

**N-14. Minor — `00` order table against `09` §D.**
- P37: `00` gives "P35" only; `09` gives "P35; P41 for interiors; P32".
- P38: `00` "Needle peaks"; `09` "Needle peaks and forest classes". M-22 says "P38 or its own
  small phase"; `09` has decided. Step 11 credits band 3 to P43 alone although its forest
  classes come in step 8.
- Row P30a says "Everything below", and D6 says "bands 3 to 10 wait"; P38 (band 5) needs P36
  only.
- The `04` and `08` correction notes state the arena at every gate up as settled "(decision
  D10)"; D10 is still "please confirm".

**N-15. Minor — `09` P37, second criterion (l. 220–221).**
"Against the same recipe with damage amount 0, amount 0 gives equal layers and features"
compares a thing with itself. Meant: against the same recipe without the ruin stages. Also the
fourth criterion writes "a gap feature" for a broken street, while B2 gives `street.state`.

**N-16. Minor — `00` table row "Caverns", "Limits"; `09` P40, P43 "Probes first".**
- `06b` read Paris et al. 2021 at project-page level ("[opened: project page]") and replaced its
  gridless search by Dijkstra on a lattice ("is replaced by", §1.4 point 2). `00` gives "Graph:
  Paris 2021" with neither note; R1-26 asked for both.
- `00` l. 104 says each main source "must be read in full" before a spec and that `09` names the
  probe. P35, P36 and P41 list the reading; P40 (Paris 2021) and P43 (Runions 2007, "not
  opened") do not.
- McDonald & Cordonnier 2026, on which A1.6 changes the spec, was seen as a video page only
  (`06a` sources); `00` l. 57 does not say so.

**N-17. Minor — scope items without a criterion, and criteria true by construction.**
- No criterion: canals (P42), cloud-sea base level (P38), platforms at anchors (P43), early
  `[[require]]` versus the Chimes `pois` already writes (P31, P32).
- True by construction: "swamp appears where wetness and flatness pass thresholds" (P38); "a
  settlement class can be sited on summits" (P38).
- No measure: the arena polygon "next to `gate-up`" (P32), which must also lie "at a given
  walking time from `gate-down`" although `pois` has already placed both gates; "volume balance
  holds" (P38) without a tolerance; "stations at a spacing" (P42).
- P40's "between a third and two thirds" is a number of this page, against the intro's "this
  page does not invent it".

**N-18. Minor — `09` B2 leftovers.**
- A bridge over a gap is a `site` with `over: gap` in P38 and a `link` of type `bridge` in P39.
- P35's criteria use a "water gate", a "keep" and a moat as "a `water` feature"; B2 has no type
  for any of them (MP `water` types are `river` and `lake`).
- `landform` type `arch` is listed for P38, whose scope no longer has the stage that made
  arches.
- P42's `deck` mask and B1's `present` mask both mark where a partial level exists.

**N-19. Minor — `00` M-13, M-15, "What no report answers".**
`06c` §7 claims the built borders 8→9 ("interior districts replace outdoor ones") and 9→10
(ring districts first). `00` files 8–9 as "none" and leaves 9–10 out of both M-13 and M-15; P41
has no blend criterion for it.

**N-20. Minor — recommendations dropped without a note.**
`08` and `06d` recommend calibration (C2), the diversity archive (C5) and `seed_group`. `00`'s
table still says "calibration is own design", but no phase carries any of them and "Not planned
here" does not list them. `seed_group` is the tool for D9 c).

**N-21. Minor — numbers and attributions.**
- `09` P35 quotes "0.06–0.09 for 0.3.0's organic streets". That is the 4 km city; at default
  size the probe measured 0.09–0.22 (`05` l. 311). An organic-town fixture needs the second
  range.
- `00` l. 77–78 "4,765 buildings on 4 km" and "a 4 km settlement": the settlement is 3 km wide in a
  4 km window (`05` l. 296, 305).
- `00` l. 69–70 names the mill rule as failing at 4 km; A2.2 covers `river_q`, `big_river_q`
  and margins, not the mill's fixed 0.4 m³/s. "Placement margins" is no parameter in the P08
  spec (it is `features.py:43` of 0.3.0, taken over as "0.3.0's rules").
- D8 does not say that the 2 km limit is Jann's own statement of 10 Nov 2026 (decision 0003
  §7), which now meets the scales he accepted for bands 4 and 9.

---

## 3. Open after this round

Round-1 findings not fully resolved: 1, 10, 13, 15, 17, 18, 19, 21, 22, 25, 26, 27, 34, 41.

The five most important open points:

1. D11 recommends one map per floor against Jann's decision that a map shows a region, without
   naming that decision; P31 and P33 are built on it (N-1).
2. The order shows P29 as a port that P35 and P41 replace, which is the option D5 recommends
   against; the recommended case has no order (N-2).
3. The slab and void format change has no phase and no criterion; amended into 1.0 it silently
   widens P08 (N-3).
4. P31 and P35 test things that later or unnamed phases deliver (settlement and interior kinds,
   `void`, recipe inputs) (N-4).
5. The order table claims bands 4, 8 and 9 as unlocked although the ruin-on-plates case and
   floors above 2 km have no phase (N-8).
