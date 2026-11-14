# Tropical Forest Industry and SOTA Landscape v0.1

**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Branch:** `research/tropical-forest-qualification-v0-1`  
**Date:** 2026-11-14  
**Status:** desk research and benchmark planning; no competitor reproduced

## 1. Executive readout

There is no single direct competitor to a complete evidence-qualified ecological science platform. The current landscape is divided into specialist capabilities:

- Earth-observation products estimate canopy cover, height, aboveground carbon and disturbance.
- Forest process models simulate vegetation demography, physiology, succession and disturbance response.
- Biodiversity digital-twin prototypes compose landscape and species-community models.
- Wildlife AI labels camera-trap imagery.
- Ecological forecasting challenges evaluate probabilistic ecological outcomes at specific sites and horizons.

Luminous should not claim to replace these specialists. It should make them interoperable, challenge their assumptions, preserve observation/authority boundaries, and show whether a new measurement changes a decision.

The initial competitive strategy is therefore a **multi-axis, task-specific tournament**, not a single global leaderboard.

## 2. Competitive landscape

| System / family | What it actually does | Evidence / maturity signal | What it does not establish | Luminous comparison lane |
|---|---|---|---|---|
| **E3SM-FATES / NGEE Tropics** | Process-rich tropical forest demography and ecophysiology; tree size and functional-type competition; model–experiment integration | DOE's NGEE Tropics programme explicitly uses ModEx: field measurement targets model process gaps and parameter uncertainty | It is not a general evidence-custody fabric, remote-sensing change product, or autonomous survey authority | Primary reference for cohort growth, mortality, recruitment, PFT coexistence, drought/precipitation response and uncertainty |
| **LPJ-GUESS standard / PPA / SEC (2026)** | Dynamic global vegetation and forest demography; 2026 SEC scheme retains spatial cohort positions so mortality creates persistent canopy gaps and local light variation; PPA is lower complexity | Peer-reviewed 2026 GMD paper evaluates biomass, mortality/productivity by size class, functional coexistence and understory regeneration | SEC is not full individual-tree biology; it is one model architecture, with its own parameter and observation assumptions | Mandatory canopy-representation tournament: uniform patch vs PPA vs SEC-like vs small individual-tree reference |
| **BioDT Forest Biodiversity Dynamics** | Biodiversity digital-twin prototype composes LANDIS-II for forest succession/disturbance with HMSC for species–environment relationships | Official BioDT use-case page labels the forest pDT early access and research-only; initial focus is Finland forests and birds | It is not an Amazon/Congo/New Guinea qualified operational model, and the product explicitly says not to use for operational decisions | Benchmark orchestration, succession/species interaction, FAIR metadata, provenance and uncertainty transfer; test tropical-domain limitations honestly |
| **Planet Forest Carbon** | Global aboveground live carbon, canopy height and canopy cover; Forest Carbon Monitoring uses 3 m products, quarterly, archive back to 2021; Diligence includes 30 m archive | Current Planet product and technical documentation specify quarterly global land coverage and 3 m monitoring products | It is an EO-derived estimate product, not a mechanistic growth/mortality/recruitment model or independent plot census | Compare canopy/biomass observation accuracy, spatial support, update latency, uncertainty and cost; consume as a source with its own error model |
| **Space Intelligence DisturbanceTracker** | Vendor product for 10 m, monthly SAR-enabled tropical wet-forest/mangrove disturbance alerts; events are unconfirmed after two satellite passes and confirmed after four | Product overview reports monthly detection and published Gabon comparisons; product claims remain vendor-reported unless independently reproduced | It does not simulate ecosystem recovery, succession or population dynamics; its scope is a defined tropical forest/mangrove monitoring domain | Compare event detection, confirmation latency, selective logging sensitivity and false alarms using independent adjudicated events and equal area/time windows |
| **Global Nature Watch / Horizon** (formerly Global Forest Watch) | EO and AI platform expanding monitoring beyond forests into grasslands and other vegetated landscapes; existing forest tools/data continue during transition | WRI announced the rebrand on 21 Sep 2026 and Horizon preview | A monitoring/discovery platform is not a process-based forest digital twin or a plot-calibrated demography model | Use as a source discovery / monitoring comparator; compare source coverage, alerts, provenance, workflow usability and cross-ecosystem context |
| **SpeciesNet / Wildlife Insights** | Camera-trap image classification; SpeciesNet recognises nearly 2,500 animal categories using 65M labelled images; Wildlife Insights is the operational community platform | Google Research reports 99.4% animal-image finding on a held-out project test set; 83% of predictions at species level and 94.5% of those correct | Image-level animal identification is not abundance, occupancy, survival, population dynamics or causal ecology | Compare image/event classification and human-review burden only; downstream abundance/occupancy needs detection-probability and sampling models |
| **TropiCam-AI** | Camera-trap classification specifically for Neotropical arboreal mammals and birds; supports taxonomic backoff when species-level predictions are uncertain | 2026 Methods in Ecology and Evolution paper covers 84 taxa and reports 95.0% testing accuracy under its stated taxonomic aggregation setting | It is not a generic fauna model across Africa/New Guinea or a forest ecosystem model; headline accuracy depends on its test split and taxonomic backoff protocol | Reproduce on the same species/domain and test regional transfer, calibration, abstention and event-level leakage |
| **NEON Ecological Forecasting Challenge** | Site- and horizon-specific probabilistic forecast evaluation using CRPS and null baselines | Established public challenge methodology | It is not a tropical forest digital twin and does not directly validate forest demography | Adopt scoring discipline for prospective forecasts: proper scoring, site/horizon strata, baseline comparisons, calibration and transparent forecast archive |

## 3. Primary sources and references

### Process-based forest modelling
- DOE NGEE Tropics / E3SM-FATES: https://ess.science.energy.gov/ngee-tropics/
- FATES technical documentation: https://fates-users-guide.readthedocs.io/projects/tech-doc/en/latest/fates_tech_note.html
- LPJ-GUESS SEC / PPA 2026 paper: https://gmd.copernicus.org/articles/19/3595/2026/
- Multi-model forest demography reference: https://ntrs.nasa.gov/citations/20230012086

### Biodiversity and monitoring digital twins
- BioDT Forest Biodiversity Dynamics: https://biodt.eu/use-cases/forest-biodiversity-dynamics
- BioDT FAIR digital twins for biodiversity: https://doi.org/10.1038/s44185-026-00029-9
- Global Nature Watch transition announcement: https://www.wri.org/news/release-global-forest-watch-becomes-global-nature-watch-expanding-cover-more-land-ecosystems
- Global Nature Watch Horizon: https://horizon.globalnaturewatch.org/

### Earth observation and forest disturbance
- Planet Forest Carbon: https://www.planet.com/products/forest-carbon/
- Planet Forest Carbon Monitoring technical documentation: https://docs.planet.com/data/planetary-variables/forest-carbon-monitoring/
- Space Intelligence DisturbanceTracker: https://www.space-intelligence.com/disturbancetracker/
- GEDI L4A V2.1 Data.gov catalogue: https://catalog.data.gov/dataset/gedi-l4a-footprint-level-aboveground-biomass-density-version-2-1
- JRC Tropical Moist Forest: https://forobs.jrc.ec.europa.eu/TMF/

### Wildlife observation AI
- SpeciesNet research note: https://research.google/blog/where-wild-things-roam-identifying-wildlife-with-speciesnet/
- TropiCam-AI (2026): https://doi.org/10.1111/2041-210x.70213
- Wildlife Insights: https://www.wildlifeinsights.org/

### Forecast evaluation
- NEON Ecological Forecasting Challenge evaluation: https://projects.ecoforecast.org/neon4cast-docs/Evaluation.html

## 4. Where Luminous should differentiate

### A. Evidence-qualified model composition

The core product is not another source map or simulator. It is an interoperable experiment that can:

1. admit rights-cleared observations with source-native support and error;
2. map them to explicit latent ecological state variables through observation operators;
3. execute multiple pinned model families on the same initial condition and forcings;
4. measure disagreement and test whether the candidate model set is adequate;
5. identify the next observation that could distinguish models or alter a decision;
6. preserve source identity, rights, model revision, experiment and independent scoring receipts.

This composes FATES / LPJ-GUESS / LANDIS-II / HMSC and EO products without making them falsely interchangeable.

### B. Model-set criticism

A common but weak approach is to take an ensemble mean or report that all models agree. Agreement is not evidence that the model set contains the true mechanism.

The benchmark must include:
- models with same historical fit but divergent future forecasts;
- parameter uncertainty separated from structural uncertainty;
- a blinded generating mechanism omitted from all registered models;
- observation-process changes with unchanged ecological state;
- an explicit requirement to abstain or request evidence when model adequacy fails.

Reuse Symtropy #2244 and #2334 rather than building a second general model-uncertainty engine.

### C. Observation-aware causal forecasting

EO alerts and camera traps are observations, not the ecosystem itself. Luminous must keep the observation process explicit:
- sensor detects a target vs target exists;
- no acquisition vs observed absence;
- cover loss vs stem mortality;
- image classification vs abundance;
- footprint biomass estimate vs plot-wide biomass;
- disturbance alert vs mechanism and recovery outcome.

This is the central bridge between monitoring vendors and ecological process models.

### D. Decision value rather than map density

A product that renders more layers or reports more alerts is not automatically more useful. Compare whether the system changes a decision or reduces expected loss at equal field cost, crew-hours, delay, access, and ecological disturbance.

Reuse Sol-Atlas's existing EVSI and spatial-sampling work. Do not create another EVSI kernel.

### E. Community / rights / custody as first-class interfaces

Scientific usefulness must not override data rights. Plot, camera-trap, Indigenous and community records need source-specific permissions, retention, derivative, attribution, revocation and benefit-sharing. Use Mycelix as the authority/custody layer; it does not decide ecological truth.

## 5. Fair benchmark tournament

For each declared task, freeze:
- target variable and exact observation definition;
- spatial and temporal support;
- data-access rights, source version and training cut-off;
- model version, parameter-fitting method and hyperparameter budget;
- compute, runtime, memory and field-survey budget;
- test split and its unit of independence;
- metric definitions, uncertainty method and stopping rules.

Compare the strongest available baseline and the Luminous candidate on exactly the same evidence.

### Mandatory reporting rules

- Never compare a 3 m Planet grid against a 25 m GEDI footprint or a plot census without a declared support / observation operator.
- Never pool Brazil, Congo and New Guinea into one score that hides a missing region or a poor regional result.
- Never let tree-cover loss become permanent deforestation by default.
- Never let camera images become population estimates without detection and sampling models.
- Never claim a model outperforms FATES / LPJ-GUESS / BioDT on all tasks from a single synthetic fixture.
- Publish per-site, per-region, per-horizon and worst-stratum performance.
- Include uncertainty intervals, calibration, abstention and coverage alongside accuracy.
- Label unavailable comparison paths `NOT_RUN`, not inferior.

## 6. Proposed stretch targets (hypotheses, not achieved)

These are proposed tournament objectives after baseline variance and sample size are known; they are not current Luminous results.

| Task | Proposed target |
|---|---|
| Held-out forest demographic forecast | At least 10% lower CRPS than strongest matched null/statistical/process baseline on two independent site/task strata |
| Canopy-gap regeneration | Demonstrate lower held-out gap/light/recruitment error than uniform canopy and PPA on the published SEC challenge family; reproduce SEC before any superiority claim |
| Disturbance monitoring | At equal spatial support and review budget, improve event-level recall / confirmation latency without exceeding a pre-registered false-alarm ceiling |
| Biomass / structure | Match or improve error/calibration against Planet / GEDI reference on the same support and independent field validation; compare cost and latency separately |
| Fauna observations | Match task-specific SpeciesNet / TropiCam-AI baselines on their valid domains, then test calibration, abstention and cross-region transfer |
| Active investigation | Lower realized decision regret or survey cost at equal decision quality versus random, fixed-grid, cost-only and information-gain baselines |
| Model-set adequacy | Detect blinded omitted mechanisms above a preregistered sensitivity while controlling false alarms and unnecessary abstention |
| Evidence integrity | Zero rights violations, zero invalid absence promotions, zero split leakage and complete source/revision lineage on the frozen conformance corpus |

Do not collapse these into one weighted score. A major rights failure, invalid absence promotion, or severe worst-region calibration failure is a gate, not something to average away with fast runtime.

## 7. Execution sequence

1. Finish exact-head Rust verifier compilation / unit tests / receipt replay.
2. Add a validator for observation-operator expected dispositions and canopy geometry invariants.
3. Reproduce the 2026 LPJ-GUESS SEC / PPA challenge on the bounded canopy fixtures.
4. Implement a typed forest observation profile / latent-state projection adapter in the correct existing owner.
5. Admit one rights-cleared plot dataset and one independent EO / lidar reference; preserve support and uncertainty.
6. Run one plot-level hindcast and publish a baseline scorecard.
7. Expand to cross-site and cross-region transfer only where data, labels and rights are qualified.
8. Connect Symthaea model criticism and experiment selection to the existing model-adequacy and EVSI owners.

## 8. Claim ceiling

This is a desk-research comparison and tournament plan. It does not mean Luminous has reproduced any competitor, matched any competitor's data product, obtained any data licence, run any ecological hindcast, or demonstrated a performance advantage.
