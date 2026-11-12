# 0004 · What the map generation research changes

On 12 Nov 2026 Jann went through the map generation research (`research/map-generation/`,
merged in #29) point by point with Claude, before the P08 plan is written. This record writes
down what Jann decided and what he deferred. The same PR turns the decisions into umbrella spec
version 4 and amends the P08 spec, `docs/map-package.md`, `docs/generators.md` §3, decision 0003
and `CONTRIBUTING.md` §5.

## Status

proposed · 2026-11-12 · accepted when Jann merges the PR
`docs(spec): map generation research into P08 and the umbrella (docs)`.

## How the research was used

Jann's framing: nothing in the research is binding. Several of its recommendations are own
designs with no paper behind them, many sources were found but not opened, nothing ran on the
Rust generator, and its fixes after the second review were not reviewed again. Claude was to say
where the research is wrong. Each point below says what was decided and, where Claude disagreed
or found more, what and why.

Two papers were read in full for this record: Yuan et al. 2019 (the author's version from HAL,
hal-02136641) and Davy & Lague 2009 (J. Geophys. Res. 114, F03007). The audit (`06a`) could open
neither.

## Decisions

| # | Topic | Jann's decision | Where it lands |
|---|---|---|---|
| 1 | Text corrections (`09` §A1, `06a` §4) | "The changes to P08 sound good to me." | P08 §1, §4.2, §4.4, §5.9 Paper column, rulings; decision 0003 §2 |
| 1a | The fluvial law | "Yes 1a sounds good to me." | P08 §5.9: Yuan et al.'s eq. 11 with the runoff-weighted area Q = upstream sum of p̂ × area in both terms, Qs = upstream sum of (U − ∂h/∂t) × area; G = d\*·v_s / r̄; Yuan's tolerance 10⁻³ × max(U)·Δt; a note that Yuan solve single flow; physics test 14 |
| 2 | Place type names (D2) | "Sounds good to me." | `chime`, `gate-up`, `gate-down` replace `extraction`, `portal-out`, `portal-in`: format §5, generators §3, P08 recipes, constraints, `travel`, `truth`, the parity fixture's metric name. Placement rules stay 0.3.0's in P08 |
| 3 | Absolute thresholds, river spread (`09` §A2.2, §A2.3) | "Yes sounds good" | `river_q`, `big_river_q` absolute and required in every recipe; the mill rule relative to `river_q`; edge margins a share of the frame; spacing absolute; channel discharge over single flow instead of a spread check |
| 4 | The package format (D3, D4) | "Lets go your proposal c." | A `requires` list in the manifest (format §3, §6) and **CE-76.5**. Slabs, `void`, undersides and per-level grids come later as capabilities named in `requires`, each with its phase. No stored volumes now (D4) |
| 5a | P29's method (D5) | "state of the art for sure" | P29's spec chooses the method after reading its sources and running its probes; **CE-78.1 and CE-78.3 reworded** so they do not name 0.3.0's method |
| 5b | The shared room graph (D7) | "that sounds good to me. So yes." | `room_graph` as a typed world value; **CE-80.3** |
| 5c | Split P29 | "Yes I think focused phases make the most sense." | P29 Settlements and child maps; **P34 Interiors** (CE-80) |
| 6 | Split P30 (D6) | "I agree as well with the research so lets do the split." | P30 Levels in the generator (CE-79.1, 79.2, 79.4); **P45** Atlas level switcher (CE-79.3) |
| 7 | Which proposed phases enter the umbrella | "Sounds good to me." | **P31** Floor frame and places (CE-81), **P32** Recipe families (CE-82), **P33** Map metrics and batch tool (CE-83), P34; the rest waits in the research track (umbrella §5) |
| D8 | Size limits | "I think its right to remove the size from the format. Lets not set a limit now." · "Yes I agree with replacing it with a memory budget limit. The default should depend on the machines capability. So lets claim a chunk of memory of the machine with at least 3GB. And more if the machine allows." | No size in the format (§4.3), decision 0003 §7 amended, **CE-79.1 reworded** ("up to 2 km across" dropped); the P08 runner refuses a run above a memory budget (3 GiB by default in the library; P09 sets the machine-aware budget). Jann expects indoor maps to be considerably smaller than outdoor maps |
| D9 | "No hard cuts" | a) character drifts over floors and b) a border floor mixes two terrain types | P33's neighbour-floor check (CE-83.6) measures a); b) is field mixing, in the research track |
| D10 | A guardian arena at every gate up | "No not confirmed. This must change. A trial does not necessarily need guardian arena. It is an option, but not the only option." | Requirements and trial sites stay in the research track; an arena is one kind of trial site, not a rule for every floor |
| D11 | A map shows a region of a floor | Agreed: a region by default; one region per floor carries the hub and both gates; `void` marks where the rim or the Bore crosses it. Plus Jann's direction: "assign a recommended power for each region so a player needs to lineraly work through the regions to reach the end point of the trial. Additionally reions with higher power exist can exist as well as danger zones." · "Yes lets record it like that." | P31's scope and criteria; "Regions and power" in the research track; its shape under Needs Jann |
| D12 | New input types | `number` and `enum` now | `docs/generators.md` §3; P32 (CE-82.3) |
| D13 | Band recipes | "Defer who writes them. I agree with where they live." | They live in `tower-of-theos` `tools/maps/recipes/`; who writes them is deferred |
| D14 | Giant trees last | "Yes they may come last. Its not as important." | Research track, last |
| D15 | Order | P31 → P32 → P33 after P08 and P09; the rest when adopted | Umbrella §3 |
| Scope | `route_potential` | "Move it out of P08 I think P31 or P32 are better homes for it." | Out of P08 with the site type `market`; research track until P31's or P32's placement work |
| Scope | Extra metrics and snapshot export | Claude's call in the scope pass ("Move to P33"); Jann answered only its open question (`route_potential`) and did not object | Moved to P33 (CE-83.3, CE-83.4); Jann rules on it when he merges this PR |

## Where Claude disagreed with the research or found more

- **`requires` instead of amending 1.0 with all of B1 or a 2.0 later.** The research is right that
  slabs and `void` change how existing fields must be read. A list of required capabilities makes
  old readers refuse such packages instead of building them wrong (like PNG's critical chunks).
  So those features can come later as minor versions, designed after their probes, without
  freezing unprobed designs into 1.0 now.
- **The steady-state factor.** The P08 spec's "1 + G/p" is right when p means the relative runoff
  p̃, as in Yuan et al.'s eq. 24; it is now written as (1 + G/p̂)^(1/n).
- **Yuan et al. solve single flow.** Multiple flow with deposition is FastScape's documented
  practice, not the paper's. Physics test 14 runs with both partitions.
- **The tolerance.** The P08 spec's 10⁻⁶ m was about 5,000 times stricter than Yuan et al.'s
  rule.
- **River spread.** Rather than the research's spread check, channel discharge follows the single
  steepest receiver, as the river lines do. This also keeps CE-28.5's river share measured as
  0.3.0 measured it; spreading multi-neighbour discharge could have pushed it out of range.
- **Two overclaims in P08 §1** ("valleys get alluvial floors", "rivers no longer follow the grid").
- **P41 Interior structure dissolves:** state-of-the-art interiors are P34, levels P30. Only the
  ring layout stays in the research track.
- **A hub Chime conflicts with P08's placement rule.** P08 places Chimes where 0.3.0 placed
  extraction points, away from settlements. P31's hub Chime (CE-81.4) needs that rule changed;
  P31's spec does it.
- **Phase ids.** The gate accepts only `P` and two digits, so the research's "P30a/P30b" became P30
  and P45. Interiors took P34, the number the research left free.

## Deferred

Listed under "Needs Jann" in `docs/status.md`:

- the shape of regions with a recommended power (D11): areas inside the floor's map (the working
  assumption) or separate maps; the scale (working assumption 1–5, like `threat`); authored in the
  map body or generated as a proposal;
- who writes the band recipe families (D13);
- every research-track phase until its sources are read and its probe has run.

## The research track

Named in umbrella §5, with the research's numbers (`09-proposed-phases.md`):

| Proposal | Waiting for |
|---|---|
| P35 Fortification and districts (the street core is P29's) | Reading the street and district sources; own design for districts |
| P36 Steering by layers, field mixing | Reading Schott et al. 2023 in full; a mixing probe on the Rust generator |
| P37 Ruin pass | A damage-pass probe (own design) |
| P38 Needle peaks and forest classes | A wall-retreat probe (own design); strata |
| P39 Floating ground | A plate probe (own design) |
| P40 Cavern kind | A cave-skeleton probe; KarstNSim's licence |
| P42 City tiers, transit, city scale | A tier probe; a parent-and-districts probe |
| P43 Giant trees (last, D14) | A skeleton probe; Runions 2007 |
| P44 Band recipe families | The band's phases; who writes them (D13) |
| Requirements and trial sites (the other half of P32) | D10 as decided: an arena is one option; a placement probe |
| Promoting settlements at road junctions | P31's or P32's placement work |
| Regions and power | D11's shape; polygon areas on maps |
| Ring layouts (band 10) | A ring-layout probe |
| Larger places as a parent map with child maps; per-level grids; stored volumes | A probe each, and a format capability in `requires` |

## Effect on the criteria and the golden hashes

- **CE-28.5** (parity with 0.3.0) measures relief, lake share, river share, vegetation cover and
  median precipitation, and counts passing constraints. The renames change constraint names, not
  values. Single-flow channel discharge measures river share as 0.3.0 did.
- **CE-77** (golden hashes): none exist yet, so neither the renames nor `requires` invalidate any.
  P08's first golden file is recorded with this format.
- Changed or new criteria, each quoted with Jann's decision in the PR: CE-76.5 (new), CE-78.1–78.4
  (P29, reworded), CE-79.1 (reworded), CE-79.3 (moved to P45), CE-80.1–80.5 (P34, new), CE-81
  to CE-83 (P31–P33, new).

## Sources

- `research/map-generation/` (#29): `00` synthesis, `09` proposed phases, `06a` audit,
  `review-round-2.md`.
- Yuan, Braun, Guerit, Rouby & Cordonnier 2019, "A new efficient method to solve the stream power
  law model taking into account sediment deposition", J. Geophys. Res. Earth Surface 124,
  1346–1365, doi:10.1029/2018JF004867; author's version hal-02136641.
- Davy & Lague 2009, "Fluvial erosion/transport equation of landscape evolution models revisited",
  J. Geophys. Res. 114, F03007, doi:10.1029/2008JF001146.
