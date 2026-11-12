# 06a · Audit of the state-of-the-art claims in the P08 spec (stage 5a)

Audited 11 Nov 2026. Scope: P08 spec §4.2, §4.4, §5.9 (Paper column; fluvial law, slope limit,
hydrology, roads, route potential; plus the licence line in §5.1 that the task names) and the
"Research sources" of decision 0003. Nothing under this repository was changed. The
other reports in `research/map-generation/` were not read.

## Summary

- 40 claims checked: 22 confirmed, 11 weaker than stated, 1 wrong, 6 could not be checked
  (§1.1–1.3; the decision 0003 sources are counted separately in §1.4).
- **Wrong:** "No neural inference is bit-identical across platforms." Bit-level reproducible
  inference across platforms exists (RepDL, Microsoft, 2025). The spec's conclusion (keep
  inference out of the generator) can stand, but for cost and dependency reasons, not as a law.
- **Formulas:** Qin's exponent, FastScape's `p = 0.5 + 0.6·S`, Tobler's 0.6 and the licences
  are quoted correctly. No constant was found misquoted. Two formulas could not be compared with
  their primary source (Lieth's NPP formula, Leopold & Maddock's exponents).
- **The fluvial law as written is ambiguous** on precipitation: "A weighted by runoff" and
  `p̂ · A` together either count runoff twice or use local instead of upstream runoff.
- **Citations that do not carry what the spec hangs on them:** Galin 2011 (the spec's Delaunay +
  MST + detour network is not that paper's method), Emilien 2012 for route potential (no
  promotion in the paper), Montgomery & Brandon 2002 for per-rock critical slopes, Cordonnier
  2019 "fill mode" (the spec describes a different fill than the paper).
- **"Rivers stay narrow" with Qin's exponent is weaker than stated:** on low gradients, where big
  rivers run, Qin gives p ≈ 1.1, which is dispersive.
- **Missed 2023–2026 work:** nothing forces a redesign of P08. Four items deserve a line in the
  spec: McDonald & Cordonnier 2026 (alluvial forms), Tzathas et al. 2024 (analytical stream
  power, a deterministic preview), the 2025 adjoint model (deterministic inverse authoring), and
  the Eurographics 2025 terrain-descriptor survey (metrics).
- Limits of this audit: Yuan et al. 2019 full text, Lieth 1975, Leopold & Maddock's body text and
  the SIGGRAPH 2026 list beyond its first 58 papers could not be opened. Most game sources of
  decision 0003 were not checked (2 of 14 were).

## 1. Claims and verdicts

Verdicts: **C** confirmed · **W** weaker than stated · **X** wrong · **?** could not check.
"Memory" means the auditor knows the source but could not open it in this session.

### 1.1 §4.2 Research

| # | Claim | V | Evidence |
|---|---|---|---|
| 1 | 0.3.0's architecture is the mainstream FastScape family (uplift, implicit stream power, MST sinks, diffusion) | C | fastscapelib-fortran documents exactly this set: `U − K_f A^m S^n + (G/A)∫… + K_d ∇²h`. [FastScape docs](https://fastscape.org/fastscapelib-fortran/). Graphics uses the same family (Cordonnier 2016, Schott 2023) |
| 2 | It is ahead of Gaea, World Machine, Houdini, Unreal, which stack hydraulic and thermal passes | W | The description is right: Houdini "simulates hydraulic and thermal erosion" ([SideFX](https://www.sidefx.com/docs/houdini/heightfields/erosion.html)); Gaea documents hydraulic and thermal erosion, no uplift or stream power ([Gaea](https://docs.quadspinner.com/Guide/Using-Gaea/Erosion.html)); Unreal has Erosion and Hydro Erosion sculpt tools ([Epic](https://dev.epicgames.com/documentation/unreal-engine/landscape-hydroerosion-tool-in-unreal-engine)). "Ahead" is a judgement: those tools deposit sediment (Gaea outputs "Deposits"), which 0.3.0 does not. 0.3.0 is ahead in large-scale drainage structure only. World Machine's page was found but not opened |
| 3 | 0.3.0's processes are about 2013-era: no deposition, single-flow routing only | W | Braun & Willett 2013 is single-flow detachment-limited, so the label fits. But single-flow is 0.3.0's choice, not the library's: fastscapelib has a [MultiFlowRouter](https://fastscapelib.readthedocs.io/en/latest/api_python/_api_generated/fastscapelib.MultiFlowRouter.html) |
| 4 | Yuan et al. 2019 adds sediment deposition | C | JGR Earth Surface 124, 1346. Abstract: "efficient (i.e., O(N) and implicit)", unconditionally stable. [ADS](https://ui.adsabs.harvard.edu/abs/2019JGRF..124.1346Y) |
| 5 | goSPL models deposition | C | README: "simulates sediment erosion, transport and deposition". [goSPL](https://github.com/Geodels/gospl) |
| 6 | Landlab SPACE models deposition | ? | Not opened. Memory: Shobe et al. 2017, GMD 10, 4577, yes |
| 7 | Both gaps explain oversized lakes, grid-aligned rivers, no alluvial plains | ? | An inference about 0.3.0's output, not a literature claim. Plausible; only a run with G > 0 and multiple flow proves it |
| 8 | Deposition and multiple flow both fit the same implicit O(n) solver | W | FastScape's Fortran library does combine them, so it is feasible. But the abstract of Yuan et al. claims O(N) for the method, and with G > 0 the solve needs iterations (the spec itself caps them at 50). Cost is O(n × iterations), and the iteration count grows with G (memory of the paper; full text not reachable) |
| 9 | 0.3.0's placement is the 2010–2012 recipe: Emilien 2012 interest maps | W | Paper confirmed: Visual Computer 28, 809–818; interest values from sociability, worship, accessibility, slope, water, fortification, domination. [PDF](https://perso.liris.cnrs.fr/egalin/Articles/2012-villages.pdf). But its interest maps place **buildings inside a village**, in a loop with road growth. Using them to site whole settlements on a region map is an adaptation, not the paper's recipe |
| 10 | Galin 2010 roads | C | Eurographics 2010, CGF 29(2). A* on a grid with segment masks, slope cost along the direction of travel, curvature, bridges and tunnels as long arcs. [PDF](https://perso.liris.cnrs.fr/egalin/Articles/2010-roads.pdf) |
| 11 | Galin 2011 networks | C | Pacific Graphics 2011, CGF 30(7); Galin, Peytavie, Guérin, Beneš. [PDF](https://perso.liris.cnrs.fr/eric.guerin/download/pg2011.pdf) |
| 12 | 0.3.0 is behind that recipe: 8-neighbour Dijkstra, per-cell slope cost, straight-line distances | C | Galin 2010: masks beyond 8 neighbours (k = 5, 80 directions recommended), slope in travel direction. Galin 2011: Euclidean proximity graphs "not effective" |
| 13 | …and no route-potential promotion | W | Neither Galin paper nor Emilien 2012 has a promotion step. It is not part of "that recipe"; it is the spec's own addition |
| 14 | Schott et al. 2024, multi-scale erosion amplification, MIT code | C | ACM TOG 43(4), art. 145; Schott, Galin, Guérin, Peytavie, Paris. [Paper page](https://h-schott.github.io/publications/mserosion/publi_mserosion.html). Repository licence MIT. [GitHub](https://github.com/H-Schott/MultiScaleErosion). Caveat the spec omits: the code needs OpenGL 4.3 compute shaders, so it cannot be reused under G20; only the method can |
| 15 | No neural inference is bit-identical across platforms | **X** | RepDL gives bit-level reproducible training and inference across platforms by fixing operation order and correctly rounded functions. [arXiv 2510.09180](https://arxiv.org/abs/2510.09180v1). Integer-only inference is another route. A small network evaluated with the spec's own rules (fixed order, exact operations, `libm`) would be bit-identical too. True only for stock frameworks on mixed hardware |
| 16 | Strict physics-informed networks are a poor fit for stream power | ? | No source found for or against. Roberts 2025 does not discuss it |
| 17 | Surrogates trained on our own output are the better hybrid | W | The one published test is modest: a Fourier neural operator trained on stream-power runs gets large-scale form but "loses fine detail such as valley networks" and leaves more sinks. [Roberts 2025, ESurf 13, 563](https://esurf.copernicus.org/articles/13/563/2025/). Rivers are exactly what a preview must show |

### 1.2 §4.4 Approaches considered, and §5.1 licences

| # | Claim | V | Evidence |
|---|---|---|---|
| 18 | Erosion–deposition after Davy & Lague 2009, implicit O(n) solution of Yuan et al. 2019 | C | FastScape docs attribute the G term to Yuan et al. 2019. Davy & Lague 2009 (JGR 114, F03007) not opened; memory |
| 19 | G = 0 is detachment-limited stream power (Braun & Willett 2013) | C | Follows from the equation: the G term vanishes. Braun & Willett 2013, Geomorphology 180–181; memory |
| 20 | FastScape's slope-dependent exponent | C | "p = -1 … corresponds to an exponent that varies with slope according to: p = 0.5 + 0.6*slope". [FastScape docs](https://fastscape.org/fastscapelib-fortran/) |
| 21 | Qin et al. 2007's convergent exponent | C | IJGIS 21(4), 443–458. See #31 |
| 22 | D∞ is harder to make implicit with deposition | ? | No source. An engineering opinion |
| 23 | MST resolver, Cordonnier, Bovy & Braun 2019 | C | ESurf 7, 549. MST of the basin graph; Kruskal O(n log n) or planar Borůvka O(n). [ESurf](https://esurf.copernicus.org/articles/7/549/2019/) |
| 24 | Sediment filling depressions is goSPL's rule, from its documentation | C | Sediment below the depression volume is deposited; above it "the elevation of each node i is increased to its filling elevation" and the surplus goes to the spill node. [goSPL docs](https://gospl.readthedocs.io/en/stable/tech_guide/dep.html). Note: goSPL does this on priority-flood depressions (Barnes 2016), not on the MST resolver; the combination is the spec's own |
| 25 | fastscapelib and goSPL are GPL-3.0; Landlab is MIT | C | [fastscapelib](https://github.com/fastscape-lem/fastscapelib), [goSPL](https://github.com/Geodels/gospl), [Landlab](https://github.com/landlab/landlab) READMEs |
| 26 | Glacial erosion: Hergarten 2021 | C | "Modeling glacial and fluvial landform evolution at large scales using a stream-power approach", [ESurf 9, 937](https://esurf.copernicus.org/articles/9/937/2021/) |
| 27 | Layered strata: Cordonnier et al. 2018; GraphFlood | C | "Sculpting Mountains", IEEE TVCG 2018 ([HAL](https://hal.archives-ouvertes.fr/hal-01517343)); GraphFlood 1.0, [ESurf 12, 1295, 2024](https://esurf.copernicus.org/articles/12/1295/2024/). Titles seen, bodies not opened |

### 1.3 §5.9 Paper column and paragraphs

| # | Claim | V | Evidence |
|---|---|---|---|
| 28 | `tectonics`: Cordonnier et al. 2016; Schott et al. 2023 | C | CGF 35(2) "Large Scale Terrain Generation from Tectonic Uplift and Fluvial Erosion" ([EG](https://diglib.eg.org/handle/10.1111/cgf12820)); TOG 42(5) "Large-scale Terrain Authoring through Interactive Erosion Simulation" ([ACM](https://dl.acm.org/doi/10.1145/3592787)). Both author uplift maps; neither defines 0.3.0's parametric sources |
| 29 | `geology` and `slope_limit`: Montgomery & Brandon 2002 for critical slope per rock type | W | The paper (EPSL 201, 481) relates erosion rate to relief and argues for threshold hillslopes. It gives no critical slope per rock type (memory; abstract not opened). The per-rock table is 0.3.0's own |
| 30 | `resolve_sinks` "fill" mode after Cordonnier 2019 | W | The paper's "depression filling" reassigns receivers across the depression by distance to the spill. The spec's fill lifts a `routing_surface` to spill + ε in priority-flood order. Same goal, different algorithm; the spec's is closer to Barnes 2014 with ε |
| 31 | Qin: p = 8.9·min(tan β_max, 1) + 1.1, weights 0.5 cardinal, 0.354 diagonal | C | "f(e) = 8.9 × min(e, 1) + 1.1", e = tangent of the maximum downslope gradient; "Li is 0.5 … cardinal … 0.354 … diagonal" (after Quinn et al. 1991). [PDF](https://geography.wisc.edu/solim/axing/publication/030_IJGIS-Qin07_Published.pdf) |
| 32 | Qin's exponent "is convergent on slopes and keeps rivers narrow"; FastScape's is dispersive | W | On steep ground Qin gives p up to 10 (convergent). On valley floors tan β → 0 gives p ≈ 1.1, Freeman's dispersive value. Big rivers run on low gradients, so discharge spreads there. The spec keeps lines narrow only because river lines follow the single steepest receiver; the `river` raster from thresholded discharge may still braid or thin out on plains |
| 33 | `partition_flow`: Freeman 1991 | C | Freeman 1991, Computers & Geosciences 17: flow ∝ slope^p, p = 1.1 (memory, and consistent with Qin's lower bound) |
| 34 | Priority-flood with ε, Barnes et al. 2014 | C | Barnes, Lehman & Mulla, Computers & Geosciences 62, 2014. [arXiv 1511.04463](https://arxiv.org/abs/1511.04463). The ε variant is in the paper body (memory) |
| 35 | Lake area A = Q_land / (E_lake − P), Bracht-Flyr et al. 2013 | W | J. Hydrology 486, 376–383: a steady-state lake water balance with a Budyko runoff term. Its result is the ratio of basin area to lake area, not this formula. [UNL](https://digitalcommons.unl.edu/geosciencefacpub/377). The spec's formula is the same balance rearranged, so it is consistent but derived, not quoted |
| 36 | Fill–Spill–Merge, Barnes et al. 2021 | C | ESurf 9, 105–121. [ESurf](https://esurf.copernicus.org/articles/9/105/2021/esurf-9-105-2021-metrics.html) |
| 37 | Width w = a·Q^0.5, depth d = c·Q^0.4, Leopold & Maddock 1953 | ? | USGS Professional Paper 252 exists and states power laws; its abstract gives no numbers. [USGS](https://pubs.usgs.gov/publication/pp252). 0.5 and 0.4 are the standard downstream values (memory; velocity 0.1 closes the sum to 1) |
| 38 | Lieth's Montreal model: NPP = 3000·(1 − e^(−0.0009695·(AET − 20))) g/m²/yr; cited "Lieth 1975; Adams et al. 2004" | ? | The formula matches the auditor's memory of Lieth & Box 1972 / Lieth 1975 (dry matter, AET in mm). No source opened shows it or the name "Montreal model"; the literature found calls it the **Thornthwaite Memorial model** ([Zhou et al., PLOS ONE](https://journals.plos.org/plosone/article?id=10.1371%2Fjournal.pone.0252149)). Adams, White & Lenton 2004 (Ecol. Modelling 177, 353) covers the Miami model only, so it does not support this formula |
| 39 | Tobler 1993, off-path factor 0.6 | C | "For off-path travel, this value should be multiplied by 3/5". [Wikipedia, citing Tobler 1993](https://en.wikipedia.org/wiki/Tobler%27s_hiking_function) |
| 40 | `roads`: Galin et al. 2010, 2011; `route_potential`: Emilien et al. 2012 | W | Routing: in the spirit of Galin 2010, but 16 directions against the paper's recommended 80, and no curvature cost. Network: Galin 2011 uses a generalised proximity graph on path cost (γ-neighbourhood between Gabriel and relative-neighbour graphs), then merges nearby roads by Fréchet distance. The spec's Delaunay candidates + MST + `loop_detour` loops are a different construction. Promotion: not in Emilien 2012 (see #13) |

Not individually checked (bibliographic, memory only): Smith & Barstad 2004 (linear orographic
precipitation, J. Atmos. Sci. 61); Fu 1981 (Budyko form); Génevaux et al. 2013 (SIGGRAPH,
hydrology-based terrain); Sørensen et al. 2006 (HESS, topographic wetness index). The titles
match their use in the spec.

### 1.4 Decision 0003, "Research sources"

| Source | V | Evidence |
|---|---|---|
| Ghost Recon Wildlands, 80.lv | C | Both quotes are in the article; terrain built "from scratch in World Machine, using noise and erosions", on 80 computers. [80.lv](https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/) |
| Horizon Zero Dawn slides | W | "~4MB/km2" is there. "Deterministic" and "locally stable" are two separate goals on a slide; the decision joins them into one quotation. [Slides](https://www.slideshare.net/slideshow/gpubased-procedural-placement-in-horizon-zero-dawn/72897079) |
| Barnes et al. 2014 (arXiv 1511.04463) | C | See #34 |
| Barnes 2019 (arXiv 1803.02977) | C | "Accelerating a fluvial incision and landscape evolution model with parallelism", Geomorphology 2019. [arXiv](https://arxiv.org/abs/1803.02977) |
| Cordonnier, Bovy & Braun 2019 | C | See #23 |
| FastFlow, CGF 2024 | C | Jain, Kerbl, Gain, Finley, Cordonnier; GPU flow routing in O(log n) and depression routing in O(log² n) iterations. [EG](https://diglib.eg.org/handle/10.1111/cgf15243) |
| The other twelve game sources and the Rust, Box2D, rustfft, rayon, rand, Rapier links | – | Not checked in this audit |

## 2. Question 1 — work from 2023–2026 that the spec misses

None of it forces a change to P08's design. The family the spec chose (implicit stream power with
deposition, multiple flow, MST sinks) is still what the geoscience codes and the graphics papers
build on. Four items should be named in the spec, because each bears on a decision it records.

| Work | What it does | Bearing on the spec | Licence · G20 · Stage fit · format needs · size |
|---|---|---|---|
| McDonald & Cordonnier 2026, "Stochastic Geomorphological Transport for Terrain Erosion Simulation", ACM TOG (SIGGRAPH 2026, honourable mention). [Video](https://www.youtube.com/watch?v=Eu5ATKUWyWw); DOI 10.1145/3811336 (page not reachable) | Parallel, stochastic, point-based transport over geological time. Drops the stream-power velocity assumptions for momentum conservation. Shows braided rivers, meanders, deltas | These are the alluvial forms the spec hopes to get from G > 0. The newest graphics result says they need more than the ξ–q law. Lower the expectation in §4.2 ("both gaps explain … no alluvial plains") and name this as the later option beside GraphFlood | No code found. A stochastic parallel scheme can be made bit-identical only with fixed sub-seeds and a fixed order, single-threaded. Fits as a stage. No format change. Medium to large |
| Tzathas, Gailleton, Steer & Cordonnier 2024, "Physically-based Analytical Erosion for fast Terrain Generation", CGF 43(2). [EG](https://diglib.eg.org/items/0a412929-2e29-4bcb-a597-1a4b2cc558e2) | Analytical solutions of the stream power law; time becomes a slider; multigrid iteration in 2D; adds landslides and hillslopes | A deterministic erosion preview without a neural surrogate. It competes with deep-learning track 2 in §4.2. Detachment-limited only (no deposition) | Licence of any code not checked. Exact operations only, so G20 is reachable. Fits as a stage or a preview mode. No format change. Medium |
| "Reconstructing landscapes: an adjoint model of the stream power and diffusion erosion equation", ESurf 13, 1263, 2025. [ESurf](https://esurf.copernicus.org/articles/13/1263/2025/) (title seen, body not opened) | Gradient of a landscape misfit with respect to uplift and parameters through the stream-power solver | A deterministic route to inverse authoring (sketch → uplift), which §4.2 ranks first among deep-learning tracks. Also Unerosion (Yang et al., CGF 2024, [EG](https://diglib.eg.org/handle/10.1111/cgf15182)) runs erosion back in time for the same purpose | Not checked. Would run beside the generator as a tool, not as a stage. Medium |
| Argudo et al. 2025, "Terrain Descriptors for Landscape Synthesis, Analysis and Simulation", Eurographics 2025 state-of-the-art report, CGF. [Code](https://github.com/oargudo/terrain-descriptors); [Wiley](https://onlinelibrary.wiley.com/doi/10.1111/cgf.70080) (not reachable) | A survey and implementation of terrain metrics | The spec builds a metrics module (§5.14) from 0.3.0's summary. This is the current reference list to check it against | Licence not checked; read the survey, not the code, until it is. Small |

Seen and judged not to change the design:

- Roberts 2025, neural operators for landscape evolution (ESurf): supports caution on surrogates (#17).
- FastFlow 2024 and Bangerth 2026 ([arXiv 2606.12800](https://arxiv.org/abs/2606.12800)): GPU or
  cluster flow routing. Not usable under "no threads inside a run".
- Perche et al. 2025, "Vector-Based Terrain Modelling" (CGF): a representation, not a process.
- TerraFusion 2025 ([arXiv 2505.04050](https://arxiv.org/abs/2505.04050)) and other diffusion
  models: neural, outside the generator by the spec's own rule.
- "Flexible Terrain Erosion" 2024 ([HAL](https://hal.inrae.fr/LISTIC/hal-04603075v1)): title seen
  only. It may matter for the non-heightfield bands (caverns, floating ground), not for P08.

Gaps in this search: the SIGGRAPH 2026 paper list was read only up to paper 58 of several
hundred; GMD was not searched separately; SIGGRAPH Asia 2025 and Eurographics 2026 were covered
only by keyword search.

## 3. Question 2 — formulas and constants in §5.9

| Item | Result |
|---|---|
| Qin exponent and contour weights | Correct (#31). The weights are Quinn et al. 1991's, used by Qin |
| FastScape `p = 0.5 + 0.6·S` | Correct (#20). The docs say "slope"; that S is the **steepest** slope is the spec's reading and was not confirmed |
| Tobler off-path 0.6 | Correct (#39) |
| Lieth NPP formula | Constants match memory; not verified against a source. Name and second citation doubtful (#38). State the unit as dry matter |
| Leopold & Maddock 0.5 and 0.4 | Standard values; not verified against the paper (#37) |
| Lake balance | Consistent with the cited model, but derived (#35) |
| PET = clamp(300 + 45·T, 200, 1800) | 0.3.0's own formula, as the spec says. No literature claim |
| Road cost `1 + (grade ÷ comfort_grade)² × 6`, bridge and score rules | The spec's own. Not from Galin; do not read the Paper column as covering them |

**The fluvial law.** No constant is wrong, but the equation is ambiguous in one place:

    dh/dt = U − K · (p̂ · A)^m · S + G · Qs / (p̂ · A)

- The text defines A as "drainage area … weighted by runoff" and then multiplies by p̂ again. If A
  is already runoff-weighted, runoff is counted twice. If A is plain area and p̂ is the local
  value, a dry cell below a wet catchment gets too little water. The quantity that belongs in
  both terms is the upstream sum of p̂ × cell area. FastScape's docs describe precipitation as
  "relative precipitation rate … with respect to a mean value already contained in Kf and g",
  which agrees with the normalisation by the mean, but they do not print where it enters. How
  Yuan et al. write it could not be checked (full text not reachable; the spec's §7 already
  lists this risk).
- FastScape's printed deposition term is `(G/A) ∫_A (U − ∂h/∂t) dA`. The spec's Qs must be that
  integral, uplift included. The spec's step 1 ("accumulate Qs … using the current heights")
  should say so, or Qs will be read as eroded volume only.
- "Implicit O(n)" holds per iteration. With G > 0 the total cost depends on the iteration count
  (#8).

**Slope limit.** The algorithm (move half the excess to the steepest neighbour, repeat) is the
classic thermal erosion step of graphics. Neither cited paper defines it in this form:
Montgomery & Brandon 2002 is a geomorphology paper on relief and erosion rate; Schott et al. 2024
uses thermal erosion as one of its processes. Cite it as the standard thermal step and keep
Montgomery & Brandon only for the idea of threshold slopes.

## 4. What to change in the spec (for the coordinator)

1. §4.2 item 3: replace "No neural inference is bit-identical across platforms" with a statement
   that stock frameworks are not, and that reproducible inference exists but adds a dependency.
2. §5.9 fluvial law: define the discharge term once (upstream sum of p̂ × area) and define Qs.
3. §5.9 hydrology: drop or qualify "keeps rivers narrow"; add a check that the `river` raster does
   not spread on low gradients.
4. §5.9 Paper column: `roads` and `route_potential` are "after" Galin and Emilien at most; the
   network construction and the promotion are the spec's own. `slope_limit` and `geology`: see §3.
5. §5.9 ecology: name the model "Thornthwaite Memorial model (Lieth & Box 1972; Lieth 1975)" unless
   a source for "Montreal model" is found; drop Adams et al. 2004 as its source or say what it is
   cited for.
6. §4.2 or §4.4: add the four works of section 2.
7. Decision 0003: split the Horizon Zero Dawn quotation.

## Sources

Opened in this audit:

- FastScape Fortran library documentation — https://fastscape.org/fastscapelib-fortran/
- Yuan, Braun, Guerit, Rouby & Cordonnier 2019, JGR Earth Surface 124, 1346 (abstract only) — https://ui.adsabs.harvard.edu/abs/2019JGRF..124.1346Y
- Cordonnier, Bovy & Braun 2019, ESurf 7, 549 — https://esurf.copernicus.org/articles/7/549/2019/
- Qin et al. 2007, IJGIS 21(4), 443–458 — https://geography.wisc.edu/solim/axing/publication/030_IJGIS-Qin07_Published.pdf
- Schott, Galin, Guérin, Peytavie & Paris 2024, ACM TOG 43(4) — https://h-schott.github.io/publications/mserosion/publi_mserosion.html ; code https://github.com/H-Schott/MultiScaleErosion
- Repositories (licence lines): https://github.com/fastscape-lem/fastscapelib , https://github.com/Geodels/gospl , https://github.com/landlab/landlab
- goSPL technical guide, inland depressions — https://gospl.readthedocs.io/en/stable/tech_guide/dep.html
- Galin et al. 2010, CGF 29(2) — https://perso.liris.cnrs.fr/egalin/Articles/2010-roads.pdf
- Galin et al. 2011, CGF 30(7) — https://perso.liris.cnrs.fr/eric.guerin/download/pg2011.pdf
- Emilien et al. 2012, Visual Computer 28 — https://perso.liris.cnrs.fr/egalin/Articles/2012-villages.pdf
- Adams, White & Lenton 2004, Ecol. Modelling 177 (first 100,000 characters) — https://bio-math10.biology.kyushu-u.ac.jp/~ben/adams_et_al_2004.pdf
- Zhou et al. 2021, PLOS ONE (Thornthwaite Memorial model naming) — https://journals.plos.org/plosone/article?id=10.1371%2Fjournal.pone.0252149
- Bracht-Flyr, Istanbulluoglu & Fritz 2013, J. Hydrology 486 (abstract) — https://digitalcommons.unl.edu/geosciencefacpub/377
- Tobler's hiking function — https://en.wikipedia.org/wiki/Tobler%27s_hiking_function
- Leopold & Maddock 1953, USGS PP 252 (abstract) — https://pubs.usgs.gov/publication/pp252
- Hergarten 2021, ESurf 9, 937 — https://esurf.copernicus.org/articles/9/937/2021/
- Roberts 2025, ESurf 13, 563 — https://esurf.copernicus.org/articles/13/563/2025/
- RepDL, arXiv 2510.09180 (search result, abstract level) — https://arxiv.org/abs/2510.09180v1
- Jain et al. 2024, FastFlow, CGF — https://diglib.eg.org/handle/10.1111/cgf15243
- Barnes et al. 2014 — https://arxiv.org/abs/1511.04463 ; Barnes 2019 — https://arxiv.org/abs/1803.02977
- Tzathas et al. 2024, CGF — https://diglib.eg.org/items/0a412929-2e29-4bcb-a597-1a4b2cc558e2
- McDonald & Cordonnier 2026 (video page) — https://www.youtube.com/watch?v=Eu5ATKUWyWw
- Bangerth 2026 — https://arxiv.org/abs/2606.12800
- Houdini heightfield erosion — https://www.sidefx.com/docs/houdini/heightfields/erosion.html
- Gaea erosion guide — https://docs.quadspinner.com/Guide/Using-Gaea/Erosion.html
- 80.lv, Ghost Recon Wildlands — https://80.lv/articles/procedural-technology-in-ghost-recon-wildlands/
- Horizon Zero Dawn placement slides — https://www.slideshare.net/slideshow/gpubased-procedural-placement-in-horizon-zero-dawn/72897079

Could not be opened (relied on abstracts, search-result titles or memory, as marked): Yuan et al.
2019 full text (HAL blocked the fetch); Lieth 1975 and Lieth & Box 1972; Leopold & Maddock 1953
body; Montgomery & Brandon 2002; Davy & Lague 2009; Braun & Willett 2013; Freeman 1991; Shobe et
al. 2017; ACM page of McDonald & Cordonnier 2026; Wiley page of Argudo et al. 2025; the GitHub API
(licences were read from the repository web pages instead); the World Machine help page.
