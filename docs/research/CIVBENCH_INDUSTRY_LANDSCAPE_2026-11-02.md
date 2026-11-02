# CivBench Industry and Research Infrastructure Landscape

**Date:** 2026-11-02  
**Status:** Comparative strategy; this is not a benchmark result or ranking.  
**Purpose:** Identify mature infrastructure to reuse, the gaps CSS can address, and a fair path to demonstrate superiority on bounded tasks.

## 1. Strategic position

Civilizational Systems Science should not set out to replace every agent-based modelling environment, historical database, climate-impact intercomparison, or integrated-assessment scenario platform.

Those systems already solve important parts of the problem. Rebuilding them would waste effort and create a weak comparison target.

The defensible ambition is to create an **evidence-preserving composition and evaluation layer** that connects:
- source-grounded historical and contemporary knowledge;
- physical, ecological, demographic, economic, institutional and cultural mechanisms;
- versioned scenarios and exact run artifacts;
- causal intervention tests and uncertainty;
- cross-context and transfer evaluation;
- evidence-backed spatial and planetary projections.

The unit of competition must be a named scientific task, not "the whole industry."

## 2. Existing platforms and what they already do well

### Mesa

Official documentation: https://mesa.readthedocs.io/stable/

Mesa is an Apache-2.0 Python framework for agent-based modelling. Its current stable documentation describes built-in modelling components, AgentSet-based agent management, browser-based Solara visualization, data collection/analysis tools and example models.

**Strength:** accessible framework for constructing, instrumenting, visualizing and analysing agent-based models.

**Not implied:** a built-in theory or validation of civilization, a standard historical evidence ontology, or automatic physical conservation across domains.

**CSS response:** do not compete on a generic agent scheduler. Compare against Mesa implementations for bounded agent tasks; make the scientific model and evidence contract portable so a Mesa model could be used as an external baseline or data producer.

### GAMA

Official platform and version history:
- https://gama-platform.org/wiki/next/Home
- https://gama-platform.org/wiki/next/OlderVersions
- https://github.com/gama-platform/gama

GAMA is an open-source, spatially explicit agent-based modelling and simulation environment. Its documentation lists urban mobility, climate adaptation, epidemiology, disaster evacuation and urban planning among use cases. The documentation reports the 2026-06 release as current on the date of this audit. GAMA emphasizes spatial models, extensibility, plugins, interoperability and headless/server use.

**Strength:** spatially explicit agent-based modelling, GIS use cases and a mature modeller-facing environment.

**CSS response:** reuse spatial inputs and compare against GAMA for bounded regional or settlement experiments. CSS can contribute evidence provenance, chronology uncertainty, institutional causality, multi-domain stock-flow checks and common benchmark manifests around those models.

### NetLogo

Official documentation: https://docs.netlogo.org/

The official documentation describes NetLogo as a programmable environment for simulating natural and social phenomena. The current documentation identifies NetLogo 7.0.4 (April 2026); its release notes describe BehaviorSpace export recording the NetLogo version used.

**Strength:** accessible, established multi-agent modelling and experiment tooling; useful for transparent baseline models and teaching.

**CSS response:** include NetLogo reference models where a compact, interpretable ABM is more appropriate than a high-complexity Rust or cognitive model. Do not assume our stack is superior just because it is lower-level or faster.

### Seshat Global History Databank

Official data portal: https://seshatdatabank.info/data

Seshat provides historical variables and polity data through article replication datasets, curated data snapshots and an API. Its data portal identifies Polaris-2026 as the latest snapshot at the time of review and describes scripts for comparing it with an earlier release.

**Strength:** historical data curation and replication materials for comparative historical research.

**Limit:** variable coverage, temporal resolution, coding choices and representativeness depend on the curated data and individual research questions.

**CSS response:** consume and cite Seshat data rather than rebuilding its curation. Add explicit uncertainty, missingness, spatial/temporal intervals, alternative interpretations and mechanistic tests. Treat the data as a sample of coded history, not the totality of civilization.

### D-PLACE

Official data source guide: https://d-place.org/source  
About: https://d-place.org/about

D-PLACE connects cultural descriptions to language, time, geography and environmental context, and links coded observations to primary sources and coding comments. Its documentation specifically warns users to consider intracultural variation, uncertainty and the original sources behind cultural codes.

**Strength:** cross-cultural comparison with time, place, language and source context.

**CSS response:** use D-PLACE as a key source for culture/environment questions, but retain coding comments, source lineage, temporal focus, spatial focus and uncertainty. Never treat cultural codes as immutable traits of populations.

### World Values Survey / Integrated Values Surveys

Official time-series documentation: https://www.worldvaluessurvey.org/AJDocumentationSmpl.jsp?CndWAVE=-1&INID=&SAID=-1  
WVS/EVS trend data: https://www.worldvaluessurvey.org/WVSEVStrend.jsp

The WVS V3.0 time-series file covers waves from 1981–2022. The official documentation explicitly states that it is a repeated cross-sectional time series, not a panel following the same respondents. The WVS/EVS trend data provide repeated cross-national questions over a long period.

**Strength:** repeated cross-national measures of values and attitudes.

**Limit:** not a person-level longitudinal panel; countries, sampling designs, questions, translations and response structures require careful harmonization.

**CSS response:** model country/wave aggregates only where justified, preserve survey weights and methodology, and avoid claiming individual-level belief change from repeated cross-sections.

### ISIMIP

Official project: https://www.isimip.org/  
Protocols: https://www.isimip.org/protocol/  
ISIMIP4 protocol: https://protocol4.isimip.org/

ISIMIP provides common simulation protocols, forcings and output structures for inter-sectoral climate-impact model comparison. The project reports that ISIMIP4 was launched in August 2026 with the first forcing data and related protocol, and that ISIMIP4a/4b cover model evaluation and future climate-impact experiments.

**Strength:** structured intercomparison of climate impacts across sectors, scales, forcing datasets and experiments.

**CSS response:** do not write a replacement climate-impact model. Use ISIMIP-compatible data/outputs as an external environmental forcing or comparison source. CSS's additional hypothesis is how physical impacts connect to institutions, migration, trade, access and collective adaptation.

### IAMC and IIASA SSP scenario infrastructure

IAMC scenario databases: https://www.iamconsortium.org/resources/databases/  
IIASA SSP data: https://iiasa.ac.at/models-tools-data/ssp

The IAMC has coordinated collections of quantitative integrated-assessment scenarios used in IPCC and related policy assessments. IIASA's SSP database links population, economic development, technology and related socioeconomic assumptions with climate pathways. The IIASA portal reports updated GDP and population projections released in 2025 and further IAM quantifications under development during 2026.

**Strength:** structured, quantitative, cross-model scenario comparison for long-run socioeconomic and climate pathways.

**CSS response:** use SSP/IAMC scenarios as scenario families and boundary conditions, not as empirical forecasts or a single true future. Add local, institutional, cultural and distributional mechanisms while maintaining compatibility with shared scenario identifiers and units.

## 3. Comparative capability map

| Capability | Mature external infrastructure | CSS contribution |
|---|---|---|
| Generic ABM execution | Mesa, GAMA, NetLogo | Mechanism-specific models and portable task contracts |
| Spatial ABM / GIS | GAMA and geospatial toolchains | Historical location/boundary uncertainty plus causal spatial-resource links |
| Comparative historical variables | Seshat | Source lineage, time intervals, competing interpretations, and mechanism validation |
| Cross-cultural variables | D-PLACE, WVS/EVS | Situated cultural data and transfer tests with uncertainty; not cultural ranking |
| Climate-impact intercomparison | ISIMIP | Coupling climate impact to institutions, access, migration, trade and adaptation |
| Long-run socioeconomic scenarios | IAMC / IIASA SSP | Explicit institutional/cultural mechanisms and distribution-sensitive outcome vectors |
| Run provenance and evidence state | Existing Humanity Atlas contract and Mycelix evidence-plane work | Cross-project scientific evaluation receipts and independent re-execution |
| Cognitive / cultural transmission | Specialized cognitive and social-learning research | Falsifiable tests showing whether cognitive mechanisms add value over simple baselines |
| Historical/planetary visual exploration | Sol-Atlas / GIS / digital-twin systems | Observed vs inferred vs scenario display with source/time/space uncertainty |

This table is an architectural positioning, not a claim that every external platform lacks the CSS capability. Where capabilities overlap, benchmark the same task and compare.

## 4. The real research gaps CSS can own

### 4.1 Provenance continuity across model transformations

The source identity, source version, transformation, unit, uncertainty and time semantics must survive conversion from a historical dataset to an indicator, from an indicator to a model parameter, and from model input to scenario output.

A SHA-256 digest establishes byte identity. It does not authenticate a publisher, guarantee data accuracy, establish independence, or prove scientific truth.

### 4.2 Causal-path and mechanism qualification

A model should expose the path from an intervention to independently measured outcomes. The multiworld simulator's Phase 2 null result demonstrates why this matters: intervention-internal variables moved, but the stated aggregate outcome inputs did not. CivBench should identify disconnected pathways and preserve null results.

### 4.3 Physical closure coupled to social outcomes

Test mass/energy/material/water/population accounting independently from the social model, then test whether the social/institutional layer changes resource use and distribution through declared processes. Do not let a single aggregate welfare score hide resource depletion, unequal exposure or unaccounted flows.

### 4.4 Pattern-oriented multi-scale validation

A civilizational model should reproduce a pre-registered vector of spatial, temporal, distributional, institutional and physical patterns for its intended use—not just a single aggregate curve. Pattern selection, thresholds, source dependencies and acceptable discrepancy measures must be declared before fitting.

### 4.5 Cross-context transfer without universalizing local history

Hold out sites, source families, regions, periods, ecologies or institutional forms. A model that explains one case may not explain another. Failed transfer is a result, not a reason to add ad hoc parameters until the model fits.

### 4.6 Crisis-tail calibration

Ordinary calibration does not guarantee reliable estimates of low-frequency, high-impact tail events. Include exceedance and excess-severity calibration for crisis loss, service interruption and ecological thresholds when data support it. Report event rarity and confidence intervals; do not overclaim with small samples.

## 5. Where to aim higher than current practice

The following are proposed targets, not attained performance:

1. **Benchmark interoperation:** one shared manifest can wrap an existing Symtropy, Mesa, GAMA, NetLogo, ISIMIP or external scientific model output without erasing its native model authority.
2. **Exact provenance:** every scored run pins model commit, toolchain, resolved config, source datasets, transforms, seeds, evaluation split and output digests.
3. **Independent verifier:** a second implementation checks physical invariants, artifact binding and score calculation rather than trusting the producer.
4. **No hidden leakage:** historical origins, source publication cutoffs and spatial/source-family holdouts are machine-checkable.
5. **Matched-baseline competition:** complex models must beat simple statistical and fixed-rule alternatives on the identical task and budget.
6. **Mechanism evidence:** every added mechanism has positive controls, negative controls, ablations and a measured effect on a predeclared outcome vector.
7. **Distribution and power:** report who gains, who bears costs, access, autonomy and concentration, not only aggregate output.
8. **Open negative results:** null and failed models remain in the evidence record with explanations and reproducible runs.

## 6. How to claim superiority responsibly

A result may claim improvement only when it names:
- the exact benchmark task and version;
- the strongest relevant baseline(s);
- identical source snapshots and evaluation cases;
- matched compute/runtime budgets where relevant;
- the frozen holdout and source/time leakage controls;
- proper scoring rules and confidence intervals;
- calibration, tail behavior, and transfer performance;
- ablations showing which mechanism contributed;
- an independent exact-artifact rerun.

"Better than SOTA" is not a valid claim across a field. A defensible claim is narrow, for example:

"On HIST-001D v0.1, for the pre-registered held-out site/phase set, model family A improves the specified chronology/occupation pattern score over baseline family B by X, with interval Y, under the same source cut-off and compute budget; it does not transfer to case C."

If no credible baseline can be beaten, publish that. If the benchmark is too small to distinguish models, improve the task or collect more evidence rather than expanding the claim.

## 7. Immediate implementation consequence

CivBench should remain a task and evaluation layer over the existing Humanity Atlas run contract and multiworld simulator.

Do not create a second generic:
- agent-based runtime;
- evidence ledger;
- scenario/run envelope;
- historical database;
- climate model;
- spatial atlas.

The next technical milestone is an adapter/qualification matrix showing which existing external data/model systems can be used as sources, forcing inputs, baseline models, or independent validators, with exact version and license rules.
