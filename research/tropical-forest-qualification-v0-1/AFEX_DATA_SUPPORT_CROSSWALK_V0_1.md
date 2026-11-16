# AFEX multi-stream data support crosswalk v0.1

**Issue:** [#5408 — AFEX-FOREST-INTERVENTION-001](https://github.com/Luminous-Dynamics/symtropy/issues/5408)  
**Parent programme:** [#5173 — tropical forest ecological qualification](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Design gate:** [AFEX_FACTORIAL_DESIGN_V0_1.md](AFEX_FACTORIAL_DESIGN_V0_1.md)  
**Register:** [DATASET_CANDIDATE_REGISTER_V0_1.json](DATASET_CANDIDATE_REGISTER_V0_1.json)  
**Date:** 15 November 2026  
**Status:** metadata crosswalk only; no child archive bytes ingested or aligned

## 1. Why this crosswalk exists

The AFEX experiment collection links multiple observational streams to one randomized nutrient-manipulation programme. This is unusually valuable for studying coupled process responses, but a shared experiment name does not make every resource a common panel. The records have different dates, units, sampling operators, spatial supports and nested units.

The correct architecture is an **intervention-aligned evidence graph**:
- the randomized plot assignment is the common intervention authority;
- each source retains its own observation operator and support;
- cross-stream edges are admitted only after plot/block/treatment/time reconciliation;
- shared plots and campaigns are recorded as dependence, not counted as independent replications;
- mechanistic edges remain hypotheses until directly supported.

Do not concatenate the CSV files and assume row-to-row equivalence.

## 2. Candidate source matrix

| EIDC resource | Temporal support | Observation / sampling unit | Process signal | Important limitation |
|---|---|---|---|---|
| [Tree census and diameter increment](https://doi.org/10.5285/c2587e20-ba4a-4444-8ce9-ccdec15b0aa3) | Annual census 2017–2019; irregular dendrometer dates Apr 2018–Jan 2020 | Tree; repeated tree × date | DBH state, mortality, recruit threshold crossing, band-window distance | Raw dendrometer measure is circumference-window distance in mm, not DBH/biomass increment; rainy dates were not collected |
| [Stem and soil respiration](https://doi.org/10.5285/591e3708-7ff1-483b-9156-15c721c00daf) | Soil respiration Jun 2017–Oct 2019; stem respiration Oct 2019 only | Soil collar × date; tree stem chamber | Soil CO₂ efflux and stem respiration | Distinct sensors, time windows and units; verify plot linkage and chamber-area conversions |
| [Leaf area index](https://doi.org/10.5285/6e70665f-b558-4949-b42a-49fbaec7e7cc) | Oct 2017, Mar/Aug/Oct 2018 | 36 within-plot points × campaign | Canopy leaf area | Preserve point coordinates, time, ring configuration and above-canopy reference; 36 points do not create 36 treatment replicates |
| [Fine-root in-growth-core mass / productivity](https://doi.org/10.5285/b3a55011-bf46-40f5-8850-86dc8bc4c85d) | Aug/Nov 2017; Feb/Jun/Sep/Dec 2018; Mar/Jun/Sep/Dec 2019 | Plot × campaign × depth composite | Fine-root (<2 mm) mass recovered from in-growth cores; used as productivity input in the 2026 turnover paper | Five cores are bulked into one composite per plot and depth. The 2026 paper states its fine-root standing-stock denominator data are available from the corresponding author on request; do not assume the public EIDC file alone supplies those stock values |
| [Seedlings growth](https://doi.org/10.5285/2da56eb1-ff01-48de-ba2a-d3afceefc85f) | Bimonthly Feb 2019–Jan 2020 | Tagged seedling × sub-plot × campaign | Seedling height, DGH, leaf counts, herbivory and leaf mortality | Different inclusion criteria from tree-census recruitment; verify the stated BDFFP site and plot crosswalk before spatial joins |
| [Litterfall nutrient content](https://doi.org/10.5285/c2eaade3-5395-4c62-9733-d9dbed1546af) | Aug 2017, Aug 2018, Aug 2019 | Sample × campaign; exact plot pooling must be checked | C, N, P, K, Mg, Ca, Mn, Zn; lignin/cellulose/hemicellulose | Concentration/composition is not an areal nutrient flux without sample mass and area basis; catalogue spatial resolution is 1 km |

The AFEX parent collection also lists litterfall herbivory, ant abundance, root mycorrhizal colonisation, root nutrient concentration, phosphatase activity, root morphology, soil microbial enzyme activity and fine-litterfall production. These remain discovery candidates until each child record is separately audited.

### Belowground nutrient-acquisition traits (2018)

Four additional AFEX child records are now registered:
- [Fine-root AM colonisation](https://doi.org/10.5285/52f756a1-a0f6-4991-b21b-4042018a451f): February 2018, 0–10 cm only, five within-plot points averaged to a plot/community mean; percentage root length colonised by hyphae, arbuscules and vesicles. It is not a fungal nutrient-transfer flux.
- [Fine-root morphology](https://doi.org/10.5285/4ccaab66-2818-41b8-8b31-1a710766c5d6): February 2018, 0–10 and 10–30 cm, five within-plot points and plant-community means by plot/depth; scanned length, area, volume, diameter and derived SRL/SRA/RTD.
- [Fine-root potential phosphatase activity](https://doi.org/10.5285/6529004d-b6e8-429d-ac68-0f5324d7e5d0): February 2018, 0–10 and 10–30 cm, root-tip potential acid phosphomonoesterase assay within three days of sampling. Preserve its dry-mass-normalised fluorimetric units; it is not an in-situ P uptake flux.
- [Fine-root nutrient concentration](https://doi.org/10.5285/ba190356-1731-4558-be7a-3fedc570663a): fine roots <2 mm, plot/depth means bulked over four intervals from November 2017 to August 2018; preserve this temporal composite, not a synthetic single-date observation.

The 2021 AFEX study reports a cation-associated AM colonisation increase in 0–10 cm (41.90% to 50.40%, p=0.042), with no significant N or P main effect on colonisation. It reports P-associated root productivity/diameter increases and lower phosphatase activity, and cation-associated diameter/colonisation changes: https://doi.org/10.1111/nph.17154. These trait responses provide useful hypotheses for the 2026 turnover patterns, but the sources remain distinct campaigns and support.

The mechanism-discrimination child [#5445](https://github.com/Luminous-Dynamics/symtropy/issues/5445) now records direct-foraging, mycorrhizal-outsourcing, combined-strategy, no-mediation and omitted-mechanism alternatives. Its [protocol](AFEX_ROOT_STRATEGY_DISCRIMINATION_V0_1.md) and [machine-readable matrix](cases/afex-root-strategy-hypotheses-v0.1.json) deliberately preserve the 0–10 cm AMF support and prohibit treating AM colonisation as causal mediation of 0–30 cm later turnover without additional assumptions and aligned stock/flow evidence.

### New 2026 root-turnover evidence (published summary, not raw data)

Schmeisk-Rosa et al. (2026), [*Phosphorus and base cations drive contrasting root dynamics in a central Amazon forest*](https://doi.org/10.1007/s11104-026-08303-2), defines fine-root turnover as annual productivity divided by standing stock. It reports unchanged standing stock under nutrient additions, but P-associated turnover increases of 23% and 48% in years 1 and 2, a 24% year-2 turnover decrease with cation additions, a P×cation interaction that counteracts the P response, and a year-two N×cation grouping contrast. The paper's reported -N-CAT versus +N-CAT notation is retained rather than recast as a pure N main effect. The paper states that the EIDC in-growth-core resource above supplies productivity data, while the fine-root standing-stock data used in its turnover analysis are available from the corresponding author on reasonable request.

Therefore:
- the EIDC file is admitted as a productivity/in-growth-core candidate, not automatically as the full numerator+denominator dataset needed to reproduce the paper's turnover;
- published aggregate summaries can be checked for arithmetic, but no raw plot-level stock/productivity reanalysis is possible until the stock data are obtained;
- the synthetic stock/flow benchmark [#5431](https://github.com/Luminous-Dynamics/symtropy/issues/5431) is a counterexample and summary crosscheck, not an independent reproduction of the 2026 study.

## 2.1 Soil microbial and above-ground litter streams

Three further companion records have now been individually registered:
- [Soil microbial potential enzyme activities](https://doi.org/10.5285/b9a2523e-09ba-43c7-acfa-a894a231b133): January 2018, soil at 0–5 and 5–10 cm, five within-plot points averaged by plot/depth; potential BG, NAG and PHOS activities in nmol MUF g soil dry weight⁻¹ h⁻¹. This is an ex-situ potential enzyme assay, not an in-situ nutrient flux.
- [Fine litterfall production and nutrient composition](https://doi.org/10.5285/c0294ec9-45d6-464c-b543-ce9ece9fd968): repeated July 2017–July 2019 collection with five 50 cm × 50 cm traps per plot, 1 m above ground; preserve plot, trap, collection date, category and the 1.25 m² total trap area.
- [Litterfall herbivory / leaf-trait record](https://doi.org/10.5285/58b66cdf-451c-42e7-8153-c24ec30631fd): August campaigns in 2017, 2018 and 2019; the catalogue summary describes realized leaf area and LMA from leaves sampled across all litter traps in each plot. Do not infer that a direct herbivory rate is present until the archive schema confirms it.

These streams improve the opportunity to test a microbial C/N/P acquisition alternative and connect root traits with above-ground litter return. They also add more support types: microbial soil depth is 0–5/5–10 cm, fine-root trait data are 0–10/10–30 cm, and litter trap samples are plot × trap × collection-date. No cross-layer or cross-stream joins are admitted solely because these records belong to AFEX.

### Biotic community and seedling damage streams

Two remaining child records from the AFEX parent collection have now been registered:
- [Litterfall ant genus and abundance](https://doi.org/10.5285/60e77fd4-7a24-4545-8d90-08e9dfcbd16a): October 2018 and September 2019; five 1×1 m subplots inside each 50×50 m plot (20 subplots per treatment, 160 total); Winkler extraction. This is a biotic-community observation, not a decomposition or nutrient-mineralization flux.
- [Seedling leaf loss by herbivory](https://doi.org/10.5285/2b8029ff-ddf5-47b2-9231-5fa0cbb6cd41): repeated bimonthly observations; ten marked leaves per eligible seedling; visual leaf-loss percentage plus galls/pathogens/trail herbivory and leaf mortality. Preserve seedling × leaf × campaign nesting and six repeated visits per subplot.

Both catalogues describe the BDFFP site as approximately 100 km north of Manaus, whereas other AFEX records describe approximately 80 km. Shared experiment branding does not prove spatial identity; resolve plot/subplot keys before joins.

## 3. Non-negotiable support rules

### 3.1 Treatment and replication

- Primary treatment-assignment unit: plot.
- Blocking unit: randomized block.
- Tree, seedling, collar, LAI point, core and repeated date are nested observation units.
- Composite samples are one observation per declared composite, not independent measurements of the treatment.
- The expected 4 × 8 = 32 plot allocation is only a candidate design until actual plot IDs and treatment maps are reconciled.

### 3.2 Do not collapse different biological thresholds

The tree census describes DBH >10 cm and defines recruitment as a new individual reaching 10 cm DBH. The seedling resource describes tagged individuals >20 cm tall and DGH >1 cm. Those are different life-stage and measurement definitions. A seedling growth row is not interchangeable with a tree-census recruit event.

### 3.3 Do not collapse different measurement operators

- A dendrometer band window distance is not automatically circumference increment.
- Stem respiration ppm and soil respiration µmol CO₂ m⁻² s⁻¹ are not directly additive.
- LAI is an instrument-derived canopy estimate with ring and reference-reading semantics.
- Fine-root biomass stock is not automatically root production or nutrient uptake.
- Litterfall nutrient concentration is not nutrient return per unit area without mass and area support.

Every transformation must retain raw variable, source file, unit, method, support, formula/version, missingness and resulting unit.

### 3.4 Preserve temporal and spatial support

Before any cross-stream join, use the executable [AFEX source identity gate](AFEX_SOURCE_IDENTITY_GATE_V0_1.md) and [#5470](https://github.com/Luminous-Dynamics/symtropy/issues/5470). Its dispositions distinguish exact plot-context matches, documented aliases, unresolved identity and hard support conflicts. The gate does not adjudicate the actual 80 km / 100 km discrepancy; that remains unresolved until source-key or field-protocol evidence is admitted.

Before joining streams, compute the exact intersection of:
- block and plot identity;
- plot polygon / within-plot point or subplot support;
- observation date and interval;
- treatment assignment and dose;
- measurement protocol and sample frame.

Do not interpolate a sparse LAI campaign to a monthly series without an explicit model and uncertainty. Do not force tree measurements at the individual level to match a plot-composite root sample. Retain unmatched observations as valid source-native data rather than discarding them or inventing joins.

## 4. Hypothesized process graph

The following is a guide for experiment planning, not a validated causal DAG:

`N/P/cation assignment`
→ `soil nutrient availability and microbial activity`
→ `fine-root biomass, nutrient concentration, mycorrhiza and phosphatase`
→ `canopy LAI and seedling growth`
→ `stem growth, recruitment and mortality`
→ `litterfall and stem/soil respiration`

Potential feedbacks and common causes (water availability, seasonal phenology, disturbance, tree size/species and soil heterogeneity) must be represented rather than treated as residual noise. Some variables are mediators; conditioning on them changes the causal estimand. The primary treatment effect should remain intention-to-treat by randomized plot assignment, with mediation analyses clearly labeled secondary and assumption-dependent.

Ant abundance and herbivory can inform biotic pathways, but they should not be promoted to carbon-flux observations. The model-set adequacy lane should include plausible omitted pathways and test whether conclusions change.

## 5. Admission and evaluation sequence

1. Admit each child dataset independently: archive bytes, checksum, rights, citation, file schema and measurement metadata.
2. Resolve a shared plot/block/treatment crosswalk against the experiment protocol.
3. Preserve each source's native rows and raw values; create explicit typed observation adapters.
4. Calculate pairwise support intersections and dependence groups before fusion.
5. Build only the process-specific model cells whose variables are actually observed at compatible support.
6. Freeze plot-level train/test partitions and forecast issue dates; every tree, seedling and repeated measurement from a held-out plot remains held out.
7. Compare against block-adjusted factorial baselines and measurement-specific statistical models before mechanistic models.
8. Evaluate each response separately: treatment-effect bias, process error, proper probabilistic scores, interval coverage, pool closure, transfer and decision loss.
9. Use the existing observation-admission, support-intersection, variable-aware aggregation, consilience/dependence, process-trajectory, model-set adequacy and decision-value owners. Do not implement duplicate generic engines.

## 6. Claim ceiling

The records establish that multiple streams exist within the AFEX collection and provide their source-described sampling designs. They do **not** yet establish that every stream can be joined at the same plot, block, treatment and time support.

All resources remain metadata-only in the current register. No raw archive bytes, plot-level joins, treatment effects, calibrated process pathways, integrated digital twin or SOTA advantage are claimed.

The ratio's plot identity is part of its scientific meaning. Do not compute turnover after dropping plot IDs and joining only on treatment/year, and do not average stock and productivity before forming the plot-level response. See [AFEX_RATIO_ESTIMAND_V0_1.md](AFEX_RATIO_ESTIMAND_V0_1.md) and [#5443](https://github.com/Luminous-Dynamics/symtropy/issues/5443) for the synthetic same-marginal/opposite-pairing challenge.


### 16 November 2026 — EIDC archive endpoint and access status

The EIDC catalogue pages now expose the supporting-documentation package endpoints for the two highest-priority root trait records:
- phosphatase supporting docs: https://data-package.ceh.ac.uk/sd/6529004d-b6e8-429d-ac68-0f5324d7e5d0.zip
- morphology supporting docs: https://data-package.ceh.ac.uk/sd/4ccaab66-2818-41b8-8b31-1a710766c5d6.zip

Both endpoints are recorded in [DATASET_CANDIDATE_REGISTER_V0_1.json](DATASET_CANDIDATE_REGISTER_V0_1.json). Catalogue links and Open Government Licence statements are verified; direct binary ZIP retrieval from the current runtime failed. The full data endpoint is not admitted. EIDC's current programmatic guidance states that downloads via tools such as wget/curl require a Personal Access Token, following its 5 October 2026 migration. No token is available in this workflow. The register therefore keeps `downloaded=false`, `data_ingested=false`, no SHA-256 and no file manifest. Do not infer raw-data admission from the catalogue page or link.
