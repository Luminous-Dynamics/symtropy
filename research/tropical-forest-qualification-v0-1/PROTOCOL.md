# Tropical Forest Ecological Qualification v0.1

**Tracking:** [Symtropy #5173 — TROPICAL-FOREST-ECO-QUAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Branch:** `research/tropical-forest-qualification-v0-1`  
**Date:** 2026-11-14  
**Status:** research protocol; not field-validated

## Purpose

This programme qualifies ecological state transitions and forecasts for tropical forests. It is not a rainforest renderer, a generic life simulator, a second GIS, or an assertion that current Symtropy models already reproduce Amazon, Congo, or Papua New Guinea ecosystems.

The first goal is modest and falsifiable: predict selected forest-structure and tree-demography variables at declared spatial and temporal supports, with calibrated uncertainty, against independent evidence. Wildlife and full trophic networks follow only when their own data and process models are available.

## System composition and authority

| Concern | Owner | Boundary |
|---|---|---|
| Canonical ecological state transitions and synthetic/retrospective execution | Symtropy | Reuse `symtropy-basin`, `symtropy-lifesim-core`, `symtropy-world`, `symtropy-terrain`, and Living World #252; do not create another general world or water solver |
| Source assets, spatial identity, CRS, support, acquisition/census time and EO change claims | Sol-Atlas | Reuse existing EO / Earth evidence contracts (#797, #798); imagery products are evidence, not ecological state truth |
| Hypotheses, uncertainty interpretation, causal questions and experiment prioritization | Symthaea | Recommendation and inference only; no invented priors, source labels or authority |
| Permissions, consent, contributor attribution, custody and community governance | Mycelix | Rights decision binds exact source, purpose and derivative; public URL does not imply unrestricted derivative rights |

### Existing Symtropy components: what they are and are not

- `symtropy-lifesim-core` is a CPU-first, deterministic multi-channel field substrate with diffusion and optional WGPU parity. Its channels are generic simulation variables; they are not a calibrated soil, water, carbon or fungal model by naming alone.
- `symtropy-basin` is a deterministic reference model that couples water, soil, atmosphere, heat, toxins, trophic-memory proxies, disturbance, and recovery. Current scenario defaults and heuristic updates must be classified as illustrative until parameterized and validated.
- `symtropy-colony` and `symtropy-mycelium` demonstrate field-mediated stigmergic and network dynamics. They are not yet validated ant ecology or fungal / mycorrhizal physiology.
- `symtropy-world` provides Earth H3 cell identity, generic biome / hydrology fields, and macro/meso/micro scale contracts. Grid identity does not itself establish geodetic accuracy, area support, or hydrological correctness.
- `symtropy-terrain` owns terrain/material and its existing physical boundaries. Soil biological state must not be inferred from a renderer voxel or unqualified terrain density.
- [Living World Cell #252](https://github.com/Luminous-Dynamics/symtropy/issues/252) owns the bounded causal ecological vertical slice; [Living Valley #1247](https://github.com/Luminous-Dynamics/symtropy/issues/1247) owns larger historical environmental causality. This programme supplies empirical profiles and validation criteria, not shortcuts around their prerequisite gates.
- Reuse [ABM runtime comparison #3787](https://github.com/Luminous-Dynamics/symtropy/issues/3787) and [collective cognition benchmark #4578](https://github.com/Luminous-Dynamics/symtropy/issues/4578) for runtime and interaction-topology comparisons.

## Initial model boundary

The first model family covers:

1. Forest stand / canopy structure and aboveground biomass (with units and spatial support).
2. Size- or functional-group cohort growth, establishment/recruitment, survival and mortality.
3. Soil-water stress and seasonal forcing only where the upstream hydrology and forcing variables are qualified.
4. Disturbance and recovery where independent labels distinguish degradation, temporary disturbance, permanent land conversion, fire, windthrow, landslide, or drought.
5. Deadwood / litter / decomposition and nutrient return only after the relevant stocks, fluxes and boundary terms can be accounted for.
6. Horizontal heterogeneity and canopy gaps only where they improve independent predictions over patch/cohort baselines.

A tree cohort is not an individual tree. A pixel is not a plot. A plot mean is not a global forest state. Every variable must declare its support and conversion model.

Later, separate evidence-qualified modules can add animal populations, movement, pollination, seed dispersal, disease, trophic interactions, decomposition guilds and symbiosis. A complete food web is not in v0.1.

## Candidate evidence sources

| Source | Spatial / temporal character | Appropriate role | Non-negotiable caveat |
|---|---|---|---|
| JRC Tropical Moist Forest (TMF) | Landsat, 30 m, annual tropical moist-forest change series (1990–2025 for current cited product) | Disturbance, degradation, deforestation and regrowth context | Pin dataset version and class definitions; not tree demography |
| UMD / Global Forest Watch tree-cover loss | Landsat, 30 m, annual (2001–2025) | Disturbance-event screening / change context | Loss includes natural events, harvest and plantations; it is not synonymous with permanent deforestation; preserve methodology-era boundaries |
| MapBiomas Collection 11 (Brazil) | Annual land cover, 30 m, 1985–2025 | Brazil / Amazon regional transition context | Brazil coverage is not full Amazon / Congo / New Guinea coverage; use collection-specific accuracy and class semantics |
| NASA GEDI L4A V2.1 | Sparse ISS lidar footprints; aboveground biomass density and prediction standard error | Biomass / canopy-structure constraints and calibration checks | Preserve footprint support, beam, orbit, quality flags, algorithm group and uncertainty; do not treat as wall-to-wall truth |
| ForestPlots.net and partner networks (including RAINFOR, AfriTRON, GEM, ForestGEO, etc.) | Plot / stem censuses with species, size, demography, traits and soil metadata | Growth, mortality, recruitment, species/trait and process calibration | Access is tiered, contributor governed and may be non-redistributable; request permission before use or derived publication |
| New Guinea / Papua New Guinea field networks | To be established case by case | Regional transfer and local calibration | Do not imply plot coverage or permission before confirmation from data custodians |

Reference pages:
- JRC TMF: https://forobs.jrc.ec.europa.eu/TMF/
- UMD / GFW 2025 data explanation: https://www.globalforestwatch.org/blog/data-and-tools/2025-tree-cover-loss-data-explained/
- MapBiomas: https://brasil.mapbiomas.org/en/
- GEDI L4A V2.1: https://catalog.data.gov/dataset/gedi-l4a-footprint-level-aboveground-biomass-density-version-2-1
- ForestPlots.NET: https://forestplots.net/en/about
- ForestPlots data conditions: https://cloudcms.forestplots.net/en/join-forestplots/working-with-data
- RAINFOR field manuals: https://rainfor.org/en/manuals/after-the-field/
- AfriTRON manuals: https://afritron.org/en/manuals

Data admission records the exact asset/version/digest, provider, access and derivative rights, purpose, temporal interval, spatial support, CRS/datum, units, quality flags, processing history, and evaluation split. A dataset can be referenced without copying its protected rows.

## Strong external comparators

### Process-based tropical forest dynamics

- **FATES / E3SM-FATES** is a serious comparator: DOE NGEE Tropics describes it as a process-rich ecosystem model representing forest demography and ecophysiology, including competition among tree sizes and functional types. The ModEx approach connects experiments, measurements, and model updates.
  - https://ess.science.energy.gov/ngee-tropics/
  - Technical guide: https://fates-users-guide.readthedocs.io/projects/tech-doc/en/latest/fates_tech_note.html
- **LPJ-GUESS** is a process-based dynamic global vegetation model representing vegetation dynamics and composition across climate, CO₂, nitrogen deposition and land management.
  - https://www.lss.ls.tum.de/en/lsai/lpj-guess/
- A multi-model forest demography study compares BiomeE, CABLE-POP, FATES, LPJ-GUESS, JULES-RED and ORCHIDEE against regrowth and natural forest observations, including tropical sites:
  - https://ntrs.nasa.gov/citations/20230012086
- A 2026 LPJ-GUESS paper adds a spatially-explicit canopy scheme to address horizontal heterogeneity and gap regeneration:
  - https://gmd.copernicus.org/articles/19/3595/2026/

The programme must reproduce strong references on matched tasks, rather than claiming superiority from a toy model. FATES literature reports that model biases can remain along tropical precipitation gradients and by functional type; the 2026 LPJ-GUESS work is explicitly trying to improve small-scale canopy-gap processes. Those are benchmark targets, not proof that either model is universally best.

### Observation / biodiversity references

- The NEON Ecological Forecasting Challenge evaluates forecasts by site and forecast horizon using CRPS and compares them to a null forecast: https://projects.ecoforecast.org/neon4cast-docs/Evaluation.html
- WildlifeBench V1 provides a task-specific camera-trap species-identification benchmark on 790 expert-labelled events from Europe and Ecuador. It is not an ecosystem simulator and does not yet cover Amazon/Congo/New Guinea as a balanced, all-taxa benchmark: https://wildlifebench.org/camera-trap/

## Evaluation protocol

### T0 — semantic and data admission

- Each variable declares unit, spatial support, temporal support, source, transformation, quality, uncertainty, and valid domain.
- Distinguish observations, derived labels, inferences, simulated states, interventions and decisions.
- Missing, cloud-covered, unobserved, below-detection and out-of-domain are not observed absence.
- Keep source-specific classes distinct: TMF degradation and UMD stand-replacing tree-cover loss are not interchangeable labels.

### T1 — synthetic model verification

- Deterministic replay for frozen model, input and seed.
- Non-negative stocks and bounded variables.
- Carbon / biomass / water / nutrient budget residuals with declared boundaries and explicit sources/sinks.
- Analytic or independently computed limiting cases.
- No mass creation/deletion hidden by clamping, rendering or a weighted summary.
- Paired intervention cases and disconnected negative controls prove that changes propagate only over declared causal paths.

### T2 — retrospective hindcast

Start with one region and one repeated-plot dataset that has an approved use path. Train on prior censuses/sites and predict held-out whole plots and future census intervals. Never randomly split neighbouring stems, pixels, repeated scenes, or rows from the same plot between train and test.

### T3 — cross-site / cross-region transfer

Report Amazon, Congo and New Guinea as separate strata. Use leave-site-out, leave-campaign-out, temporal holdout and leave-region-out transfer. A region without adequate data and rights is `NOT_RUN`, not pass, zero, or evidence of generalization.

### T4 — prospective forecasting

Before the next census or reference observation is revealed, bind the prediction, model, data manifest, horizon and forecast distribution to an immutable receipt. Score after independently governed observations arrive.

## Metrics (no single composite score)

| Task | Primary metrics |
|---|---|
| Biomass / canopy structure | MAE, RMSE, bias, CRPS, 90/95% interval coverage and width |
| Mortality / survival | Brier or log score, calibration, survival / hazard error, cohort-specific error |
| Growth / recruitment | MAE/RMSE or likelihood/deviance on observed increments and recruitment counts |
| Disturbance / forest transition | Per-class precision, recall, F1, IoU, event timing error, false alerts per area and time |
| Recovery | Recovery trajectory error, time-to-recovery distribution and uncertainty |
| Water / carbon / nutrients | Budget closure residuals and flux/state errors in physical units |
| Transfer | Per-region and worst-region score, calibration degradation and abstention |
| Runtime | Wall time, peak memory, energy / hardware only when measured (reuse #3787) |
| Evidence integrity | Provenance completeness, split leakage, rights violations and invalid absence promotions |

For probabilistic ecological forecasts, report proper scoring rules (CRPS / log score / Brier as appropriate) alongside calibration and sharpness. A point estimate alone is insufficient.

### Proposed stretch target

Once baseline variance, sample size and cluster structure are known, preregister a matched tournament aiming for at least 10% lower CRPS or normalized error than the strongest admissible baseline on at least two independent region/task strata, under equal data and compute budgets. Require nominal 90% predictive intervals to meet predeclared coverage tolerance, no degradation of mandatory conservation/evidence gates, and site/plot-block uncertainty analysis. This is a target, not an achieved result.

## First concrete sequence

1. Audit each current code variable/process as **measured**, **derived**, **illustrative**, or **unimplemented**.
2. Define a typed forest-state vocabulary and units; do not relabel generic `[0,1]` values as physical measurements.
3. Create a tiny known-truth synthetic forest patch manifest and verify growth / mortality / biomass budgets.
4. Implement a simple persistence/climatology and size-stratified statistical baseline before calibrating a complex model.
5. Reproduce or explicitly mark FATES / LPJ-GUESS baselines `NOT_RUN` with a reason.
6. Secure plot-network collaboration and derivative rights before ingesting restricted data; start with properly licensed EO source snapshots.
7. Connect a validated profile to Living World #252 and Living Valley #1247 only after their owning prerequisites are green.

## Claim ceiling

A passing synthetic fixture demonstrates software/model verification only. A retrospective hindcast demonstrates performance on that declared held-out dataset only. A prospective forecast demonstrates performance for that region, variable and horizon only. None alone establishes complete rainforest simulation, ecological understanding, conservation effectiveness, SOTA superiority, or permission to intervene in a real ecosystem.
