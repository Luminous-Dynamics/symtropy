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
| Amazon RAINFOR 2017–2019 EIDC record (DOI 10.5285/63d4b774-4e03-4db2-95ad-dcca18f0d681) | 8,729 stems in 29 Brazilian Amazon terra-firme plots across Acre, Pará and Mato Grosso; diameter census and live/dead state; 2017–2019; Open Government Licence | Best first rights-cleared Amazon structural / stand-state candidate; possible plot-level size-structure and dead/alive validation after checking package and support | The dataset is one 2017–2019 campaign record, not automatically a repeated time series. Use it for structural/stratified evaluation only unless an earlier matched census is separately admitted. Retain OGL attribution. It does not give full species demography across the Amazon |
| NASA / ORNL DAAC Sustainable Landscapes Brazil inventory (DOI 10.3334/ORNLDAAC/2007) | Selected Brazilian Amazon research sites, 2009–2018; DBH, tree heights, taxa, coordinates, canopy position, crown radius, dead-tree decomposition, AGB estimates at selected sites; point observations | Complementary source for structural allometry, canopy and dead-tree condition; candidate for a second independent source/operator | Sampling frequency varies by site: some sampled once and some resampled in later years. The catalog declares public access and USA-government-work license, but per-site/year guides must be inspected before treating any records as a longitudinal panel |
| DRC National Forest Inventory 2017–2021 (FAO FAM ref COD_2017-2021_NFI_v01_M_v01_A_ESS) | National sample design across DRC excluding former Bandundu: 321 planned 400m×400m sampling units, four 75m×75m plots each (1,284 planned); 300 units surveyed (93%); nested DBH subplots and biophysical/socioeconomic attributes | Public metadata and, if authorized, a Central African regional-strata / structural validation source | DRC-only, not the entire Congo Basin. 21 units were unsurveyed due to inaccessibility/conflict-related conditions; these are explicit missingness, never absence. Raw NFI database at DIAF requires Secretary General authorization; only content published on the national SNSF portal is accessible without specific authorization. The metadata says the inventory was to enable permanent plots; do not assume repeated census is already available |
| Papua New Guinea Multipurpose NFI 2016–2019 (FAO FAM ref PNG_2019_MNFI_v01_M_v01_A_ESS) | National two-phase stratified design; 4km grid (2km in three smaller provinces); 1,000 selected sampling points; four circular plots 300m apart per cluster; nested radius/DBH thresholds and regeneration quadrats | PNG regional strata and structure / regeneration sampling-frame context | PNG covers only the PNG jurisdiction, not the whole island of New Guinea. Precision / bias information is limited; some clusters were not surveyed due to security/logistics. Microdata access is controlled by the country-owned external repository and requires the applicable confidentiality/access process. The stated 5–10 year cycle is not proof that a completed repeat census is available |
| Brazil National Forest Inventory 2013 (FAO FAM ref BRA_2013_NFI_v01_EN_M_v01_A_OCS) | National 20km grid, denser in selected domains; Amazon biome uses four 10m×10m subplots in a Maltese-cross configuration (2,000m² total); intended repeat interval five years | Secondary Brazil national sampling-frame / structural context if the exact wave files and methods are admitted | Raw access requires confidentiality declaration and SNIF/Forest Service access terms. An intended five-year interval is not proof of an available repeated panel; confirm actual remeasurements and variable equivalence first |
| Other plot networks (ForestPlots.net, AfriTRON, ForestGEO, GEM and additional national or university networks) | Site-specific plot and stem censuses, traits, soil and ecological observations | Future calibration and regional transfer candidates | These are distinct networks and dataset licences, not one common repository or blanket licence. Assess every dataset's own rights, census protocol, taxonomic scheme, geolocation sensitivity and repeat-census availability before ingestion |

Reference pages:
- JRC TMF: https://forobs.jrc.ec.europa.eu/TMF/
- UMD / Global Nature Watch tree-cover loss data explanation (formerly Global Forest Watch): https://www.globalforestwatch.org/blog/data-and-tools/2025-tree-cover-loss-data-explained/\n- Global Nature Watch rebrand / Horizon preview announcement (21 Sep 2026): https://www.wri.org/news/release-global-forest-watch-becomes-global-nature-watch-expanding-cover-more-land-ecosystems
- MapBiomas: https://brasil.mapbiomas.org/en/
- GEDI L4A V2.1: https://catalog.data.gov/dataset/gedi-l4a-footprint-level-aboveground-biomass-density-version-2-1
- MapBiomas Collection 11 platform / accuracy information: https://plataforma.brasil.mapbiomas.org/
- Papua New Guinea NFI publication notice / FAO FAM Catalogue: https://www.fao.org/in-action/boosting-transparency-forest-data/news-and-events/news/detail/papua-new-guinea-publishes-national-forest-inventory-in-fam-catalogue/en
- Amazon RAINFOR EIDC 2017–2019 data record and OGL licence: https://catalogue.ceh.ac.uk/documents/63d4b774-4e03-4db2-95ad-dcca18f0d681
- Brazil National Forest Inventory 2013 (FAO FAM reference BRA_2013_NFI_v01_EN_M_v01_A_OCS): https://microdata.fao.org/index.php/catalog/2074
- NASA / ORNL DAAC Brazilian Amazon 2009–2018 inventory: https://doi.org/10.3334/ORNLDAAC/2007
- DRC NFI 2017–2021 metadata and access conditions: https://microdata.fao.org/index.php/catalog/2732
- PNG NFI 2016–2019 metadata and access conditions: https://microdata.fao.org/index.php/catalog/2943
- Brazil NFI 2013 metadata: https://microdata.fao.org/index.php/catalog/2553
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

Data admission records the exact asset/version/digest, provider, access and derivative rights, purpose, temporal interval, spatial support, CRS/datum, units, quality flags, processing history, and evaluation split. A dataset can be referenced without copying its protected rows.

### Initial data-admission recommendation

The machine-readable [Dataset Candidate Register v0.1](DATASET_CANDIDATE_REGISTER_V0_1.json) tracks exact source IDs, spatial/temporal support, rights state and next admission gates. It records metadata only; all sources remain marked not downloaded and not ingested.

Start the first Amazon structural pilot with the EIDC 2017–2019 RAINFOR diameter dataset because its official catalogue record explicitly provides an Open Government Licence and clear population, region, sample count and measurement dates. Treat this as a **single-campaign plot/stem structure reference** until an earlier matched census is separately admitted. Use the NASA/ORNL DAAC 2009–2018 dataset as a second source, but form training/test splits at site and year level because temporal frequency varies by site. Do not combine both sources into a single apparent panel without verifying plot identity, overlap and census dates.

For the DRC and PNG, start with public documentation, sampling frames, methods and approved aggregate outputs. Do not ingest the country-controlled microdata until the required government/custodian authorization is obtained. Neither national inventory record alone establishes a repeated demographic time series.

**Demographic hindcast gate:** before any growth, mortality or recruitment forecast is scored as a temporal prediction, require at least two compatible, independently dated measurements of the same permanent plot or an otherwise valid longitudinal cohort. A one-time inventory, dead/alive state, or national stock estimate can validate structure or levels; it cannot by itself validate growth rates, mortality hazards, recruitment rates or recovery trajectories.

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

A collection of models is not adequate merely because its members agree. Use the existing structural-uncertainty and model-set-adequacy owners (#2244 and #2334), specialized in [forest model-set adequacy issue #5198](https://github.com/Luminous-Dynamics/symtropy/issues/5198) and [its v0.1 challenge spec](FOREST_MODEL_SET_ADEQUACY_V0_1.md), to run four distinct tests:

1. **Parameter uncertainty:** the same equations, different plausible parameter sets.
2. **Structural disagreement:** plausible models fit history but imply different future or intervention responses.
3. **Omitted mechanism:** all registered models agree, but a blinded generating process contains a mechanism absent from every candidate model.
4. **Observation-process shift:** the ecological state is unchanged but sensor, sampling geometry, cloud, detection threshold or processing changes.

For each, score forecast calibration, decision regret, out-of-domain detection, false alarms and abstention. A high internal ensemble agreement must not increase confidence when residuals or held-out mechanism challenges indicate model-set inadequacy. Select the next observation or experiment to discriminate the hypotheses under the same time, cost, access and ecological-disturbance budgets. Use the forest-specific [decision-value tournament #5223](https://github.com/Luminous-Dynamics/symtropy/issues/5223) and [its worked examples](FOREST_DECISION_VALUE_V0_1.md) to compare information gain with actual decision value, reusing #2171 rather than implementing a duplicate EVSI kernel. The system recommends; the benchmark does not authorize real-world intervention.

## Evaluation protocol

### T0 — semantic and data admission

The next gates are specified in [OBSERVATION_ADMISSION_CONTRACT_V0_1.md](OBSERVATION_ADMISSION_CONTRACT_V0_1.md), [VARIABLE_AWARE_AGGREGATION_V0_1.md](VARIABLE_AWARE_AGGREGATION_V0_1.md), and [cross-modal consilience #5257](https://github.com/Luminous-Dynamics/symtropy/issues/5257). Admission, support conversion, variable semantics and source dependence are distinct checks. Multiple products are not independent evidence solely because their product IDs differ.

The record-level gate is specified in [OBSERVATION_ADMISSION_CONTRACT_V0_1.md](OBSERVATION_ADMISSION_CONTRACT_V0_1.md) and exercised by the eleven-case synthetic corpus. This is an admission contract for descriptors, not a production adapter or a rights adjudicator. The next support-conversion work is tracked separately in [FOREST-SUPPORT-INTERSECTION-QUAL-001 #5237](https://github.com/Luminous-Dynamics/symtropy/issues/5237).

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

### T5 — uncertainty calibration and transfer

Uncertainty is qualified at the true dependence unit, not at the raw pixel/stem count. The first block-aware conformal slice is documented in [FOREST_BLOCK_CONFORMAL_BENCHMARK_V0_1.md](FOREST_BLOCK_CONFORMAL_BENCHMARK_V0_1.md) and tracked in [#5285](https://github.com/Luminous-Dynamics/symtropy/issues/5285). It compares pooled observation residuals with equal-sized spatial/campaign block-maximum residuals, keeps point coverage distinct from whole-block coverage, and exposes a known-shift negative control.

- Freeze spatial/campaign/time block definitions before fitting; preserve independent training, calibration and test groups.
- Declare the exchangeability unit and its rationale. Group disjointness is necessary but does not prove exchangeability.
- Report finite-sample calibration-block rank, interval width/WIS, point coverage, all-observations-in-block coverage and realized coverage uncertainty separately.
- Evaluate leave-site, leave-campaign and leave-region transfer; report worst-stratum and rolling-window undercoverage, not only pooled long-run averages.
- Unknown shift or exchangeability means no distribution-free guarantee is claimed. A shifted negative control must remain visible.
- For a target 90% split-conformal level, at least 9 exchangeable calibration blocks are needed for a finite radius; for 95%, at least 19. These are mathematical minima, not enough by themselves for precise empirical coverage confidence intervals.

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
| Uncertainty | 50/80/90/95% point coverage, whole-block coverage, interval width/WIS at matched coverage, calibration block count and block-bootstrap coverage intervals |
| Runtime | Wall time, peak memory, energy / hardware only when measured (reuse #3787) |
| Evidence integrity | Provenance completeness, split leakage, rights violations and invalid absence promotions |

For probabilistic ecological forecasts, report proper scoring rules (CRPS / log score / Brier as appropriate) alongside calibration and sharpness. A point estimate alone is insufficient.

### Proposed stretch target

Once baseline variance, sample size and cluster structure are known, preregister a matched tournament aiming for at least 10% lower CRPS or normalized error than the strongest admissible baseline on at least two independent region/task strata, under equal data and compute budgets. Require nominal 90% and 95% predictive intervals to meet predeclared coverage tolerance on independent block units, no degradation of mandatory conservation/evidence gates, and block-bootstrap coverage intervals. Treat 10% CRPSS improvement and ±5 percentage-point block-coverage tolerance as future targets, not achieved results. This is a target, not an achieved result.

## First concrete sequence

0. Complete variable-aware aggregation over intensive fields, extensive stocks, probability-sample counts and categorical classes; preserve coverage and uncertainty with each value.
0. Benchmark cross-modal consilience and source dependence before quantitative data fusion (#5257); compose with existing observation lineage and EVSI owners.
0. Freeze and review the record-level observation-admission contract; keep production adapters, source authentication, geometry and rights adjudication explicitly separate.
0. Implement cross-scale support conversion in [#5237](https://github.com/Luminous-Dynamics/symtropy/issues/5237) before any plot/footprint/pixel score comparison.
1. Audit each current code variable/process as **measured**, **derived**, **illustrative**, or **unimplemented**.
2. Define a typed forest-state vocabulary and units; do not relabel generic `[0,1]` values as physical measurements.
3. Create a tiny known-truth synthetic forest patch manifest and verify growth / mortality / biomass budgets; run the standalone Rust verifier at `validator/` and preserve its receipt.
4. Implement a simple persistence/climatology and size-stratified statistical baseline before calibrating a complex model.
5. Reproduce or explicitly mark FATES / LPJ-GUESS (including SEC versus PPA canopy schemes) baselines `NOT_RUN` with a concrete reason.
6. Secure plot-network collaboration and derivative rights before ingesting restricted data; start with properly licensed EO source snapshots.
7. Connect a validated profile to Living World #252 and Living Valley #1247 only after their owning prerequisites are green.

## Claim ceiling

A passing synthetic fixture demonstrates software/model verification only. A retrospective hindcast demonstrates performance on that declared held-out dataset only. A prospective forecast demonstrates performance for that region, variable and horizon only. None alone establishes complete rainforest simulation, ecological understanding, conservation effectiveness, SOTA superiority, or permission to intervene in a real ecosystem.


## 15 November 2026 — additional qualification work

The programme now has three additional child lanes:

- [#5319 — realized block-coverage inference](https://github.com/Luminous-Dynamics/symtropy/issues/5319): uncertainty about observed whole-block coverage, kept distinct from prediction intervals and calibration-conditional coverage.
- [#5327 — spatial localization and conditional coverage](https://github.com/Luminous-Dynamics/symtropy/issues/5327): local weighted-residual diagnostics, effective local sample size, abstention, and Moran's I descriptive residual autocorrelation. Specialist external comparators include spconform, ERT and stable localized conformal (StCP); the small Rust kernel is not those methods.
- [#5328 — sealed blind model-set adequacy](https://github.com/Luminous-Dynamics/symtropy/issues/5328): separate public development fixtures from independently custodied hidden generators, immutable candidate submissions and signed evaluation receipts. No custodian/evaluator has yet been appointed.

The implementation and literature update are recorded in [FOREST_BLOCK_COVERAGE_INFERENCE_V0_1.md](FOREST_BLOCK_COVERAGE_INFERENCE_V0_1.md), [FOREST_SPATIAL_LOCALIZATION_V0_1.md](FOREST_SPATIAL_LOCALIZATION_V0_1.md), [FOREST_BLIND_MODEL_SET_CUSTODY_V0_1.md](FOREST_BLIND_MODEL_SET_CUSTODY_V0_1.md), and [INDUSTRY_AND_SOTA_LANDSCAPE_V0_1.md](INDUSTRY_AND_SOTA_LANDSCAPE_V0_1.md).

The core claim boundary remains unchanged: no real forest observations have been ingested, no hindcast or field calibration has been performed, no LPJ-GUESS reproduction has been executed, and no SOTA superiority is established.


### Executable synthetic receipts added 15 November 2026

The standalone validator CLI now dispatches the block-coverage and spatial-localization corpora through digest-bound receipts:

- `FOREST-BLOCK-COVERAGE-INFERENCE-001` → `PASS_SYNTHETIC_BLOCK_COVERAGE_INFERENCE`
- `FOREST-SPATIAL-LOCALIZATION-001` → `PASS_SYNTHETIC_SPATIAL_LOCALIZATION`

These paths reuse the library kernels, report exact source-manifest SHA-256 and per-case checks, and include mutation tests through the top-level `evaluate()` dispatcher. The CLI receipt adapters do not add new scientific guarantees: block independence remains a declared assumption; Gaussian spatial weighting remains diagnostic-only; Moran's I remains descriptive with no p-value. The 7 adapter unit tests are authored, bringing the package total to 173 authored Rust test functions. Cargo/Clippy/formatting execution remains unverified in the current runtime.


### 15 November 2026 — adaptive-neighbourhood diagnostic extension

The spatial localization library now supports an optional k-nearest-neighbour cap, stable distance/ID ordering, and a physical kth-neighbour radius in each local result. The public corpus includes a counterexample where a remote high-residual calibration observation drives the pooled radius to 50 while the k=12 local diagnostic returns 0.5 over an 11 m neighbourhood. This is only a toy diagnostic; it does not establish a coverage guarantee.

The work is tracked in [#5332 — FOREST-LOCALIZATION-STABILITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5332). The proper external comparison is spconform 0.1.2's own `k_neighbors` procedure plus calibration-set resampling / StCP stability, not a claim that our diagnostic duplicates it.


### Adaptive neighbourhood diagnostic update — 15 November 2026

The spatial localization kernel and its receipt corpus now also support an optional nearest-neighbour cap. Every capped prediction reports the selected neighbour count and physical kth-neighbour radius. The public reference case demonstrates the diagnostic distinction between a pooled radius of 50 and a k=12 local radius of 0.5 over an 11 m neighbourhood. This is a designed counterexample, not evidence of coverage improvement or validity. The new work is tracked in [#5332](https://github.com/Luminous-Dynamics/symtropy/issues/5332).

The spconform 0.1.2 comparator provides its own `k_neighbors` option under its published localized-conformal method. Luminous's implementation remains a diagnostic weighting baseline and must be compared head-to-head on matched splits and supports. It does not inherit spconform's local-exchangeability rationale.


### 15 November 2026 — process-resolved trajectories and compensating-error detection

The programme adds [#5340 — FOREST-PROCESS-TRAJECTORY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5340) and [FOREST_PROCESS_TRAJECTORY_V0_1.md](FOREST_PROCESS_TRAJECTORY_V0_1.md). The new bounded two-pool reference separates live and deadwood stocks, mortality as an internal transfer, growth/recruitment as live-pool inputs, and decomposition/exports as external losses. It computes per-period live, deadwood and total budget residuals and compares stocks at every census boundary as well as each primitive flux.

The public four-case synthetic fixture includes a deliberately adversarial candidate whose live/deadwood stock trajectory exactly matches the reference while growth, mortality and decomposition are each wrong by 20 units. Its disposition is `STOCK_FIT_WITH_PROCESS_MISMATCH`, rather than a stock-fit pass. This directly targets the compensating growth/mortality errors reported in the 2026 nine-model ForestPaths intercomparison. Twelve additional Rust unit tests are authored across the process kernel and top-level dispatcher; the package count is now 185 authored Rust test functions.

The synthetic receipt is SHA-256-bound and CLI-dispatched as `PASS_SYNTHETIC_PROCESS_TRAJECTORY`. Independent JavaScript reference arithmetic reproduces all four declared case dispositions and numeric oracles. This is not Rust execution: exact-head formatting, Clippy, unit tests and receipt replay remain pending unless separately evidenced. No field fluxes, longitudinal panel, ecological calibration, external VDM reproduction, hindcast, blind evaluation or SOTA superiority are established.


### 15 November 2026 — flux identifiability and measurement design

The programme adds [#5349 — FOREST-FLUX-IDENTIFIABILITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5349) and [FOREST_FLUX_IDENTIFIABILITY_V0_1.md](FOREST_FLUX_IDENTIFIABILITY_V0_1.md). An exact-rational kernel shows that two live/deadwood stock increments constrain only rank 2 of six declared fluxes, leaving nullity 4. Four independent nullspace witnesses prove the stock-only ambiguity. The kernel enumerates all 64 subsets of six direct measurement channels; 18 reach full rank, with two four-measurement designs tied at a synthetic effort cost of 14. These costs are synthetic, not field survey estimates. At the exact flux-identifiability increment the package inventory was 195 authored Rust tests; the subsequent ForestPaths crosswalk lane adds 11 more, raising the current total to 206.

The empirical bridge is tracked in [#5352 — ForestPaths benchmark reproduction](https://github.com/Luminous-Dynamics/symtropy/issues/5352). The paper's Zenodo record lists processed observation CSVs and multi-model netCDF outputs, with exact published MD5 values recorded in [DATASET_CANDIDATE_REGISTER_V0_1.json](DATASET_CANDIDATE_REGISTER_V0_1.json). The current container cannot resolve zenodo.org, so binary retrieval and ingestion remain NOT_RUN. The visible Zenodo rights field is not populated; license/attribution review is therefore a mandatory admission gate.

Independent JavaScript arithmetic reproduces exact expected rank, nullity, all 64 measurement subsets, all 18 full-rank combinations and the two tied minimum-cost designs. This is independent arithmetic, not Rust execution. Practical identifiability, observation error, field measurement costs, parameter recovery, empirical flux accuracy and forecast skill remain NOT_ESTABLISHED.


### 15 November 2026 — ForestPaths D-BEN source semantics

The source-code audit is documented in [FORESTPATHS_D_BEN_ADMISSION_V0_1.md](FORESTPATHS_D_BEN_ADMISSION_V0_1.md) and [forestpaths-dben-source-crosswalk-v0.1.json](cases/forestpaths-dben-source-crosswalk-v0.1.json), tracked by [#5357](https://github.com/Luminous-Dynamics/symtropy/issues/5357). The helper functions declare nine model-specific names, FIN/BIA/BCI, run-ID mappings, forcing identifiers and variable-specific conversions. The format file has multiple incompatible size-class systems. A cross-model benchmark must retain those original bins and transformations before any support conversion or scoring.

The most important semantic trap is that the label `cmort` describes loss due to mortality or other turnover processes, while BiomeE-Standalone's `WDgrowTot` includes reproduction and migration and `WDmortTot` includes density killing and disturbance loss. These must not be presented as pure growth/mortality observations. The R Markdown equilibrium periods were visually selected and hard-coded per site/model; they are reproduction parameters to hold fixed first and perturb in a separate sensitivity experiment, not a universal equilibrium algorithm.

The code repository has Apache-2.0 licensing, but this applies to repository source code, not automatically to the separately deposited Zenodo archive. A 14415142/14415143 DOI discrepancy remains unresolved; the Zenodo file license and binary checksums remain unverified because no archive bytes were downloaded. Thus this crosswalk does not imply data ingestion or paper reproduction.


### 15 November 2026 — executable ForestPaths source-crosswalk admission

The source crosswalk is now machine-readable at `cases/forestpaths-dben-source-crosswalk-v0.1.json` and checked by `validator/src/forestpaths_crosswalk_kernel.rs`. The CLI receipt verifies model/run/site/equilibrium mapping widths, source blob SHA syntax, 24 unique unit-declared quantities, model-specific forcing IDs, composite flux claims, five distinct size-class families, and data/code/Rmd DOI roles. It deliberately returns only `PASS_SYNTHETIC_FORESTPATHS_CROSSWALK`; no archive bytes are authenticated or ingested.

Ten kernel unit tests plus one top-level dispatch/receipt test are authored. The current raw source inventory is 229 authored Rust tests. Independent JavaScript crosswalk checks reproduce the source manifest's nine model IDs, eight run tokens per model, all 27 model/site equilibrium window pairs, 24 unique variables, five distinct size-class families, composite growth/mortality semantics and DOI-role separation. This is structural cross-checking, not Rust execution or empirical reproduction. All Rust tests remain unexecuted in the present local runtime.


### ForestPaths archive-role resolution

The Zenodo 14415143 dataset page's setup section links a separate Zenodo software release: [15870478 — DBEN_Eckes_Shephard_et_al v1.0.0](https://zenodo.org/records/15870478). The latter archives the analysis repository code as a separate software object (published MD5 `8a4d79b4a94dc58a67e36d7e7100cee1`). The dataset archive (14415143) and code archive (15870478) must be retrieved and hash-verified separately. The current v1.0.0 R Markdown still refers to 14415142 for the data; that reference remains unresolved and is not substituted for the dataset record.

The GitHub `v1.0.0` tag's inspected helper, format and R Markdown blobs match the inspected `main` versions. The observation processing script named by Zenodo metadata remains unavailable at the declared paths in both tag and main. The software record's visible license field is not populated; the GitHub `LICENSE` is Apache-2.0 for repository code, not an automatic data license.


### 15 November 2026 — noise-aware practical flux identifiability

The planned extension in [#5360](https://github.com/Luminous-Dynamics/symtropy/issues/5360) now has an executable bounded linear-Gaussian reference in `validator/src/practical_identifiability_kernel.rs` and the fixture `cases/forest-flux-practical-identifiability-synthetic-v0.1.json`. It reuses the exact structural-rank routine from #5349, adds explicitly declared measurement standard deviations and correlation pairs, validates covariance positive-definiteness, computes whitened Fisher information and GLS covariance metrics, and enumerates all 64 sensor subsets under D-, A-, E- and synthetic-cost criteria.

The frozen example exposes the difference between uniqueness and usable information: the cheapest rank-6 designs cost 14 synthetic effort units but are labelled `PRACTICALLY_WEAK` under declared synthetic thresholds (minimum eigenvalue 0.01; maximum condition number 30). All six direct channels cost 25 units and are D/A/E-optimal in this fixture; their minimum Fisher eigenvalue is 0.02099 and condition number 17.01. A separate exact-full-rank near-collinearity control is also labelled weak. These thresholds and noise/cost values are not calibrated to field instruments and must not be promoted into survey requirements.

The CLI returns `PASS_SYNTHETIC_PRACTICAL_IDENTIFIABILITY` only when the eight receipt oracles match. Twelve practical-identifiability kernel tests plus one top-level dispatcher/receipt test are authored. The current raw package inventory is 236 authored Rust test functions. All remain unexecuted until exact-head formatting, Clippy, build and test verification; no external forest data were ingested and no practical identifiability in a real forest is claimed. Compose future survey selection with #5223 expected value of sample information and #2171 active sensing, using field cost, permissions, seasonality and decision loss rather than Fisher information alone.


### 15 November 2026 — target-specific estimand identifiability

The exact structural flux benchmark now reports identifiable scientific targets separately from full six-flux identifiability. For a linear estimand (c^T x), the evaluator tests whether adding (c^T) to the observation matrix increases exact rank. The stock-only two-pool balances identify net live-pool increment, net deadwood-pool increment and total-pool increment, but not mortality transfer or gross live inputs. Across the 64 subset designs, mortality transfer becomes identifiable with one mortality channel (synthetic effort cost 3); gross live inputs need growth + recruitment (minimum cost 6). The kernel enumerates and binds per-estimand minimum-cost/minimum-count designs as part of its receipt. This does not establish practical precision: #5360 adds a separate synthetic covariance-aware information analysis.

The structural kernel now contains 16 authored tests. The raw Rust source inventory across the standalone validator is 236 authored tests, none yet executed in this runtime.
