# Canopy-Gap Representation Competition v0.1

**Tracking:** [FOREST-CANOPY-GAP-TOURNAMENT-001 #5196](https://github.com/Luminous-Dynamics/symtropy/issues/5196)  
**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Canonical plant / ecological integration:** [Living World Cell #252](https://github.com/Luminous-Dynamics/symtropy/issues/252)  
**Status:** analytical synthetic benchmark design; not an implemented canopy model

## 1. The hypothesis

A forest patch's total leaf area or biomass does not uniquely determine its spatial light environment. Two patches with equal aggregate leaf-area index (LAI) can differ in gap size, gap connectivity, understory illumination and recruitment opportunities.

A model that predicts total biomass but erases horizontal canopy arrangement may therefore fit aggregate observations while getting local regeneration wrong.

This benchmark asks whether preserving cohort position and crown support improves predictions of gap / understory observables at a declared computational cost.

## 2. Representations under comparison

- **Uniform patch:** aggregate LAI / cohort leaf area distributed horizontally across the entire patch.
- **PPA:** cohorts ordered by height with a patch-filling canopy approximation.
- **SEC-like:** cohort crown-area support has spatial location; mortality can create persistent gaps.
- **Individual-tree reference:** explicit trees/crowns only on compact synthetic cases or a small independently mapped plot.

The first three are the key competition. The individual model is a bounded comparator, not a priori ground truth. The published LPJ-GUESS SEC scheme is the current external reference for spatially explicit cohort canopies.

## 3. Published comparison target and cost boundary

The 2026 LPJ-GUESS paper provides a concrete, reproducible challenge rather than a vague canopy inspiration:

- At its Barro Colorado Island case, standard LPJ did not maintain a recruitment layer in the reported harvest experiment.
- PPA formed a recruitment layer at 20% harvest but did not retain it to 60 years in the reported cases.
- SEC retained a recruitment layer beyond 60 years even at 20% harvest.
- SEC averaged 2.26× the runtime of standard LPD in the article's patch-count experiment; PPA averaged 1.54×.
- The article notes SEC was not part of the cited 2025 demographic vegetation model intercomparison.

These results are specific to the article's model version, sites, assumptions and experiment. The Luminous comparison must reproduce the source case where possible and must compare runtime and ecological output together. It must not imply that SEC universally dominates PPA or every cohort canopy.

## 4. Analytical toy fixture (not a biological light model)

Use an 8 × 8 grid of equal-area cells. Each of two hypothetical canopy arrangements has exactly 32 occupied cells and local LAI = 2 on occupied cells; all other cells have LAI = 0. Both therefore have identical total LAI = 64 and mean LAI = 1.

- **Arrangement A — checkerboard:** occupied and open cells alternate. With four-neighbour connectivity, all 32 open cells are isolated.
- **Arrangement B — clustered block:** occupied cells fill a 4 × 8 block; the other half is one contiguous 32-cell open region.

For a deliberately simple *analytical test only*, define direct-light transmittance:

`T(x,y) = e^{-k \, LAI(x,y)}`

with synthetic extinction coefficient `k = 0.5`. This does not constitute a calibrated radiative-transfer or plant-physiology model.

Expected values:
- Uniform-patch approximation: LAI = 1 in all 64 cells, hence `T = exp(-0.5) ≈ 0.60653066` everywhere.
- Spatial model: 32 open cells have `T = 1`; 32 occupied cells have `T = exp(-1) ≈ 0.36787944`.
- The two spatial arrangements have the same transmittance histogram but different gap component topology: 32 isolated open components versus one 32-cell open component under four-neighbour connectivity.

This fixture is designed to falsify the assumption that equal aggregate LAI implies equal local gap structure. It does not prove which arrangement better predicts real seedling recruitment.

## 5. Public challenge-case manifest

The companion JSON manifest now names ten public development case families: equal-aggregate/different-topology; single-cohort mortality; clustered versus dispersed mortality; local-light recruitment; causal gap closure; translation symmetry; rotation symmetry; disconnected control; renderer independence; and process-driven fidelity promotion.

The Rust verifier checks that all ten case IDs and expected dispositions match the declared development contract. It does **not** execute the associated growth, mortality, light, recruitment or fidelity processes. These cases are specifications for later model runners, not completed model experiments and not a blind holdout.

## 6. Required process-level tests

1. **Gap creation:** remove one declared cohort / crown contribution; local gap light rises only in cells whose canopy support changed.
2. **Clustered disturbance:** remove equal biomass in two different spatial patterns; compare gap geometry and understory light.
3. **Regeneration response:** add a synthetic shade-response process; keep its equations separate from the geometric light test and label all parameters synthetic until calibrated.
4. **Gap closure:** growth / crown expansion closes a gap only through canonical growth and spatial allocation; no renderer decal or hidden timer.
5. **Translation / rotation:** under symmetric boundaries, translate or rotate a canopy arrangement and test the corresponding output invariants.
6. **Control region:** a disconnected patch stays unchanged.
7. **Fidelity promotion:** when recruitment depends on local light, a representation with only global LAI must be marked insufficient or refined before authoritative use.
8. **Budget parity:** all representations receive identical initial state, forcing, candidate actions, duration, parameter-fitting data and compute budget declarations.

## 7. Scorecard

- gap area distribution, component count and connectivity;
- understory light mean, quantiles, variance and spatial autocorrelation;
- canopy light-field error against synthetic exact truth or independent lidar-derived support;
- recruitment / survival by size or functional group, only once the demographic process is independently qualified;
- canopy closure and gap lifetime;
- aboveground woody biomass, basal area, size-class mortality and productivity;
- runtime, peak memory and energy where measured.

A spatially explicit model should only be selected when the process and task need its additional state. If only aggregate biomass is being predicted, simpler representations may be sufficient.

## 8. Admissible SOTA comparison

Reproduce a pinned LPJ-GUESS baseline and the published SEC / PPA semantics on the same frozen cases. Report implementation differences and unavailable features. Do not claim to have reproduced SEC merely because a model places cohort centroids in a grid.

Use LPJ-GUESS, FATES and an individual-tree reference only where their scientific target, parameters, forcing, and observed outputs can be made semantically comparable. The Luminous benchmark should report a multi-axis Pareto frontier rather than hide ecological error inside a runtime win.

## 9. Claim ceiling

A pass on the analytical toy fixture proves only that the implementation distinguishes aggregate-equal but spatially different canopy arrangements and preserves declared invariants. It does not validate canopy radiation, tropical allometry, tree mortality, recruitment, species coexistence or forestry decisions.
