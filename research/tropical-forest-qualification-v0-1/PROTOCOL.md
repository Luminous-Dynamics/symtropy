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

### Existing scientific-qualification owners — compose, do not fork

- [Structural ecological uncertainty #2244](https://github.com/Luminous-Dynamics/symtropy/issues/2244): compare plausible ecological model structures and their decision implications.
- [Model-set adequacy #2334](https://github.com/Luminous-Dynamics/symtropy/issues/2334): test whether all registered models can agree and still be wrong because the generating mechanism was omitted.
- [Ecological transfer and shock recovery #2214](https://github.com/Luminous-Dynamics/symtropy/issues/2214): hold out ecological regimes and measure transfer, resilience, and population invariants.
- [Closed-box aquatic process oracle #2206](https://github.com/Luminous-Dynamics/symtropy/issues/2206): reuse its process-accounting discipline and zero-third-party-dependency reference-kernel philosophy; its aquatic process equations are not forest equations.
- [Causal intervention validity #2186](https://github.com/Luminous-Dynamics/symtropy/issues/2186) and [active sensing under budgets #2171](https://github.com/Luminous-Dynamics/symtropy/issues/2171): reuse intervention and experiment-selection semantics.
- [Living World Cell #252](https://github.com/Luminous-Dynamics/symtropy/issues/252) remains the canonical structural-flora / ecological-consequence integration owner.

This tropical programme specializes those capabilities for forest demography, canopy structure, forest observation operators, and regional hindcasts. It does not create another model-uncertainty engine, general OOD detector, active-sensing policy, or water solver.

## Initial model boundary

The first model family covers:

1. Forest stand / canopy structure and aboveground biomass (with units and spatial support).
2. Size- or functional-group cohort growth, establishment/recruitment, survival and mortality.
3. Soil-water stress and seasonal forcing only where the upstream hydrology and forcing variables are qualified.
4. Disturbance and recovery where independent labels distinguish degradation, temporary disturbance, permanent land conversion, fire, windthrow, landslide, or drought.
5. Deadwood / litter / decomposition and nutrient return only after the relevant stocks, fluxes and boundary terms can be accounted for.
6. Horizontal heterogeneity and canopy gaps only where they improve independent predictions over patch/cohort baselines.
7. A **spatially explicit cohort-canopy profile**: cohorts have declared crown-area / position support, canopy gaps persist after mortality or disturbance, and the resulting local light field can affect recruitment. This is an explicit benchmark target because the 2026 LPJ-GUESS spatially explicit canopy scheme (SEC) demonstrates that cohort placement can recover gap and recruitment behaviour without fully resolving every individual tree. The matched competitors must include the standard horizontally uniform patch canopy, perfect-plasticity approximation (PPA), spatially explicit cohort canopy (SEC), and an individual-tree reference only on a small enough case to be tractable.

A tree cohort is not an individual tree. A pixel is not a plot. A plot mean is not a global forest state. Every variable must declare its support and conversion model.

Later, separate evidence-qualified modules can add animal populations, movement, pollination, seed dispersal, disease, trophic interactions, decomposition guilds and symbiosis. A complete food web is not in v0.1.

## Candidate evidence sources

The source layers measure different constructs. They are complementary, not interchangeable truth labels.

| Source | Spatial / temporal character | Appropriate role | Non-negotiable caveat |
|---|---|---|---|
| JRC Tropical Moist Forest (TMF) | v2025 product: 30 m wall-to-wall Landsat Collection 2, annual change collection for 1990–2025, with timing/intensity and transition classes | Disturbance, degradation, deforestation and regrowth context | Degradation is temporary disturbance while remaining forested; deforestation is forest-to-nonforest transition. Preserve the TMF ontology rather than collapsing both to tree loss |
| UMD / Global Nature Watch (formerly Global Forest Watch) tree-cover loss | Landsat, 30 m, annual (2001–2025) | Disturbance-event screening / change context | Loss includes natural events, harvest and plantations; it is not synonymous with permanent deforestation; preserve methodology-era boundaries |
| MapBiomas Collection 11 (Brazil) | Annual land cover and use, 30 m, 1985–2025; collection notice updated August 2026 | Brazil / Amazon regional transition context | Brazil coverage is not full Amazon / Congo / New Guinea coverage; preserve class/year accuracy information and collection-specific semantics |
| NASA GEDI L4A V2.1 | Footprint-level model estimates of aboveground biomass density (Mg/ha) and prediction standard error; nominal coverage 2019-04-18 to 2025-07-09 | Biomass / canopy-structure constraints and calibration checks | Approx. 25 m footprints sampled along sparse tracks, not wall-to-wall truth. Retain beam, orbit, algorithm group, input waveform metrics and all quality flags. There were no acquisitions while the instrument was stored from March 2023 to April 2024; do not interpret that interval as ecological absence |
| ForestPlots.net, RAINFOR, AfriTRON, ForestGEO, GEM and other plot networks | Plot / stem censuses with species, size, demography, traits and soil metadata | Growth, mortality, recruitment, species/trait and process calibration | Each network has its own access and contributor rules; some records are restricted or non-redistributable. Do not imply that these networks are one consortium or share a single licence; request the applicable permission before use or derived publication |
| Papua New Guinea National Forest Inventory (FAO FAM Catalogue) | Nationally representative multipurpose inventory published to the FAO Food and Agriculture Microdata Catalogue in March 2026 | Independent New Guinea / PNG regional evidence and sampling-frame context | Catalogue publication does not automatically grant unrestricted row-level access or derivative rights; confirm the exact download, variables, design weights, spatial disclosure, and licensing with PNGFA / FAO before ingestion |

Reference pages:
- JRC TMF: https://forobs.jrc.ec.europa.eu/TMF/
- UMD / Global Nature Watch tree-cover loss data explanation (formerly Global Forest Watch): https://www.globalforestwatch.org/blog/data-and-tools/2025-tree-cover-loss-data-explained/\n- Global Nature Watch rebrand / Horizon preview announcement (21 Sep 2026): https://www.wri.org/news/release-global-forest-watch-becomes-global-nature-watch-expanding-cover-more-land-ecosystems
- MapBiomas: https://brasil.mapbiomas.org/en/
- GEDI L4A V2.1: https://catalog.data.gov/dataset/gedi-l4a-footprint-level-aboveground-biomass-density-version-2-1
- MapBiomas Collection 11 platform / accuracy information: https://plataforma.brasil.mapbiomas.org/
- Papua New Guinea NFI publication notice / FAO FAM Catalogue: https://www.fao.org/in-action/boosting-transparency-forest-data/news-and-events/news/detail/papua-new-guinea-publishes-national-forest-inventory-in-fam-catalogue/en
- ForestPlots.NET: https://forestplots.net/en/about
- ForestPlots data conditions: https://cloudcms.forestplots.net/en/join-forestplots/working-with-data
- RAINFOR field manuals: https://rainfor.org/en/manuals/after-the-field/
- AfriTRON manuals: https://afritron.org/en/manuals

Data admission records the exact asset/version/digest, provider, access and derivative rights, purpose, temporal interval, spatial support, CRS/datum, units, quality flags, processing history, and evaluation split. A dataset can be referenced without copying its protected rows.

### The observation-operator rule

Keep a latent ecological state separate from each observation product:

`observation = H_source(latent_state, sampling_geometry, instrument, acquisition, processing) + observation_error`

- A plot census samples selected stems inside a plot, with census dates, size thresholds, taxonomic decisions and measurement error.
- GEDI L4A is a footprint-level model estimate derived from waveform metrics and field-plot calibration. It does not directly observe a plot-wide biomass surface.
- TMF is a Landsat-derived thematic change product with its own 30 m support, annual time bins and class ontology.
- MapBiomas is a separate annual land-cover/use classification product with its own training and accuracy profile.

The evaluator must apply the relevant observation operator to the model state before scoring. Do not compare a cohort's total biomass directly with one GEDI footprint, treat TMF degradation as tree mortality, or turn cloud/no-data into stable forest. Separate process uncertainty, parameter uncertainty, sampling uncertainty, sensor/algorithm uncertainty and label uncertainty.

## Industry and SOTA comparison

Use the companion [Industry and SOTA Landscape v0.1](INDUSTRY_AND_SOTA_LANDSCAPE_V0_1.md) for a task-by-task comparison of FATES, LPJ-GUESS SEC/PPA, BioDT Forest Biodiversity Dynamics, Planet Forest Carbon, Space Intelligence DisturbanceTracker, Global Nature Watch/Horizon, SpeciesNet, TropiCam-AI and NEON forecast evaluation.

Important current naming distinction: Global Forest Watch was renamed **Global Nature Watch** in 2026, and WRI introduced the AI-driven Horizon platform in preview. Existing forest data, tools and workflows continue through Global Nature Watch during the transition. Preserve the source dataset lineage and exact product version in all observation records; do not use the old platform brand as a substitute for the product identifier.

There is no single direct competitor or universal benchmark across these products. The programme must report task-specific comparisons and never compare a remote-sensing estimate product directly with a process-based ecology model without a declared observation operator and common target.

## Strong external comparators

See the companion [Industry and SOTA Landscape v0.1](INDUSTRY_AND_SOTA_LANDSCAPE_V0_1.md) for the broader comparison across process models, biodiversity digital twins, EO products, disturbance monitoring and wildlife-observation AI.


### Process-based tropical forest dynamics

- **FATES / E3SM-FATES** is a serious comparator: DOE NGEE Tropics describes it as a process-rich ecosystem model representing forest demography and ecophysiology, including competition among tree sizes and functional types. The ModEx approach connects experiments, measurements, and model updates.
  - https://ess.science.energy.gov/ngee-tropics/
  - Technical guide: https://fates-users-guide.readthedocs.io/projects/tech-doc/en/latest/fates_tech_note.html
- **LPJ-GUESS** is a process-based dynamic global vegetation model representing vegetation dynamics and composition across climate, CO₂, nitrogen deposition and land management.
  - https://www.lss.ls.tum.de/en/lsai/lpj-guess/
- A multi-model forest demography study compares BiomeE, CABLE-POP, FATES, LPJ-GUESS, JULES-RED and ORCHIDEE against regrowth and natural forest observations, including tropical sites:
  - https://ntrs.nasa.gov/citations/20230012086
- A 2026 LPJ-GUESS paper adds a spatially-explicit canopy scheme (SEC): cohorts retain positions in a patch, so mortality can create persistent gaps and change understory light and recruitment without resolving all trees individually. Its evaluation includes aboveground woody biomass, mortality and productivity by size class, coexistence, regeneration and sensitivity:
  - https://gmd.copernicus.org/articles/19/3595/2026/

The programme must reproduce strong references on matched tasks, rather than claiming superiority from a toy model. NGEE Tropics explicitly uses a model-experiment (ModEx) loop: site-specific meteorology and plant traits, field measurements designed to address process uncertainty, and multi-scale model evaluation. A published FATES assessment reports high aboveground woody productivity bias across much of the tropics under default parameters, overestimation of drought-deciduous abundance, and underestimation of evergreen functional types along a Neotropical precipitation gradient. These are specific published results and useful challenge dimensions, not proof that FATES is universally weak. The 2026 LPJ-GUESS SEC work is a strong canopy-structure comparator, not a baseline to dismiss.

The competitive aim is to outperform on a declared task and evidence class: e.g. a held-out mortality forecast, cohort size-structure trajectory, canopy-gap recruitment response, or decision-quality-per-survey-budget. Do not rank complete models with one pooled score.

### Observation / biodiversity references

- The NEON Ecological Forecasting Challenge evaluates forecasts by site and forecast horizon using CRPS and compares them to a null forecast. Adopt this scoring discipline, while recognizing that NEON challenge targets are not equivalent to tropical tree demography: https://projects.ecoforecast.org/neon4cast-docs/Evaluation.html
- WildlifeBench V1 provides a task-specific camera-trap species-identification benchmark on 790 expert-labelled events from Europe and Ecuador. It is not an ecosystem simulator and does not yet cover Amazon/Congo/New Guinea as a balanced, all-taxa benchmark: https://wildlifebench.org/camera-trap/

## Key paradigm shift: model-set criticism, not just model fitting

A collection of models is not adequate merely because its members agree. Use the existing structural-uncertainty and model-set-adequacy owners (#2244 and #2334) to run four distinct tests:

1. **Parameter uncertainty:** the same equations, different plausible parameter sets.
2. **Structural disagreement:** plausible models fit history but imply different future or intervention responses.
3. **Omitted mechanism:** all registered models agree, but a blinded generating process contains a mechanism absent from every candidate model.
4. **Observation-process shift:** the ecological state is unchanged but sensor, sampling geometry, cloud, detection threshold or processing changes.

For each, score forecast calibration, decision regret, out-of-domain detection, false alarms and abstention. A high internal ensemble agreement must not increase confidence when residuals or held-out mechanism challenges indicate model-set inadequacy. Select the next observation or experiment to discriminate the hypotheses under the same time, cost, access and ecological-disturbance budgets. The system recommends; the benchmark does not authorize real-world intervention.

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
3. Create a tiny known-truth synthetic forest patch manifest and verify growth / mortality / biomass budgets; run the standalone Rust verifier at `validator/` and preserve its receipt.
4. Implement a simple persistence/climatology and size-stratified statistical baseline before calibrating a complex model.
5. Reproduce or explicitly mark FATES / LPJ-GUESS (including SEC versus PPA canopy schemes) baselines `NOT_RUN` with a concrete reason.
6. Secure plot-network collaboration and derivative rights before ingesting restricted data; start with properly licensed EO source snapshots.
7. Connect a validated profile to Living World #252 and Living Valley #1247 only after their owning prerequisites are green.

## Claim ceiling

A passing synthetic fixture demonstrates software/model verification only. A retrospective hindcast demonstrates performance on that declared held-out dataset only. A prospective forecast demonstrates performance for that region, variable and horizon only. None alone establishes complete rainforest simulation, ecological understanding, conservation effectiveness, SOTA superiority, or permission to intervene in a real ecosystem.
