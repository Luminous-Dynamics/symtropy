# Forest Observation Operators and Support Qualification v0.1

**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Branch:** `research/tropical-forest-qualification-v0-1`  
**Status:** specification and synthetic negative-control corpus; no real data ingested

## 1. Purpose

A forest model has a latent ecological state. A plot census, GEDI lidar footprint, TMF classification and MapBiomas land-cover product are different measurement processes applied to that state. They do not observe the same variables, at the same support, or with the same uncertainty.

The benchmark must never compare raw model state directly with an incompatible data product. It must first apply a source-specific observation operator:

`y_s = H_s(x, g_s, t_s, q_s, p_s) + \epsilon_s`

where:
- `x` is the latent ecological state;
- `H_s` is the source-specific measurement / sampling / processing operator;
- `g_s` is spatial support and geometry;
- `t_s` is acquisition or census time;
- `q_s` is the source's quality / validity state;
- `p_s` is the product, instrument, algorithm and processing profile;
- `epsilon_s` is observation error (which is not the same as model-process error).

This is an admission and comparison contract, not a claim that the operators are already implemented.

## 2. Source-specific operators

### A. Permanent plot census

**Observed:** selected stems meeting a declared inclusion rule, with measured diameter, taxonomic identification, status and census time; growth, mortality and recruitment are derived between successive censuses under explicit rules.

**Operator requirements:**
- plot geometry, area, boundary version and nested subplots;
- census date and actual elapsed time between repeated censuses;
- minimum diameter / inclusion threshold and how recruits enter;
- stem identity matching, death classification and ambiguity;
- taxonomy and trait version;
- measurement protocol and uncertainty;
- plot-level access / contributor / derivative rights.

**Scoring:** aggregate the model's corresponding stems or cohorts through the same plot boundary, census interval and size threshold. Report plot-block/site-block uncertainty; stems within a plot are not independent test replicates.

### B. NASA GEDI L4A V2.1

**Observed:** footprint-level model estimates of aboveground biomass density (AGBD, Mg/ha) with prediction standard error and quality / algorithm metadata. The product uses GEDI waveform metrics and field-plot calibration. The nominal coverage is 18 April 2019 to 9 July 2025; no acquisitions occurred while GEDI was stored on the ISS from March 2023 to April 2024. Each footprint is approximately 25 m, with sparse along-track sampling.

**Operator requirements:**
- exact GEDI collection/version and granule/sub-orbit;
- beam, shot, time, footprint geometry and location uncertainty;
- algorithm setting group and all relevant L2/L4 quality flags;
- AGBD, standard error and any prediction interval;
- land-cover / plant-functional-type model inputs;
- actual footprint support and sampling density;
- separate source-model error and Symtropy model error.

**Scoring:** integrate the latent model AGBD over the exact footprint support and compare to the L4A estimate using the L4A observation uncertainty. Do not compare one plot mean to a single footprint without a declared support conversion. Do not interpolate sparse footprints into wall-to-wall truth. The March 2023–April 2024 acquisition gap is `UNOBSERVED`, never zero biomass or no change.

### C. Amazon RAINFOR / EIDC 2017–2019 plot census dataset

The official NERC Environmental Information Data Centre record (DOI 10.5285/63d4b774-4e03-4db2-95ad-dcca18f0d681) lists the Open Government Licence. It describes 8,729 trees across 29 RAINFOR permanent plots in Acre, Pará and Mato Grosso, all terra-firme non-flooded lowland forest, with measurements collected during 2017–2019. The record identifies diameter, alive/dead state and recruited stems at DBH ≥10 cm.

**Appropriate role:** first rights-cleared Amazon structural / stand-state reference, subject to its exact per-plot protocol, fields, geometry and citation terms.

**Critical limitation:** the catalogue record describes the 2017–2019 census campaign; it does not by itself give us a matched earlier baseline census for every plot. Do not call it a temporal growth / mortality panel until earlier census rows are separately admitted and stem identity / protocol equivalence verified. The Open Government Licence does not waive the duty to cite the dataset and respect its stated spatial support.

### D. NASA / ORNL DAAC Sustainable Landscapes Brazil inventory (2009–2018)

NASA's public catalogue lists DOI 10.3334/ORNLDAAC/2007, point data from selected Brazilian Amazon research sites, 2009–2018. Variables include DBH, commercial and total tree height, taxa, coordinates, canopy position, crown radius, dead-tree decomposition status and aboveground biomass at selected sites. The NASA record reports public access and a USA-government-works license.

**Appropriate role:** complementary structural allometry, canopy position / crown radius and dead-tree condition reference, potentially valuable for canopy-gap model observation operators.

**Critical limitation:** temporal frequency varies by site; some sites were sampled once while others were resampled. The dataset must be partitioned by exact site/year/method record. It is not a uniform 2009–2018 annual panel.

### E. DRC National Forest Inventory 2017–2021

FAO FAM reference COD_2017-2021_NFI_v01_M_v01_A_ESS; primary producer DIAF. Its national sample design used 321 planned sampling units, each a 400m × 400m square containing four 75m × 75m plots (1,284 plots planned). Nested subplots have different DBH thresholds: 5 cm, 10 cm and 20 cm. The survey reports 300/321 units (93%) surveyed; 21 units were not surveyed due to inaccessibility, including conflict-affected areas.

**Appropriate role:** DRC-specific national forest strata, structure and inventory-design context, and an important missingness / access adversarial benchmark.

**Critical limitation:** DRC is not the entire Congo Basin. The 21 unsurveyed units must be UNOBSERVED_INACCESSIBLE_OR_CONFLICT_AFFECTED, not forest absence or zero biomass. FAM states that the raw NFI database is housed at DIAF and requires an authorization request approved by the Secretary General for the Environment and Sustainable Development; published content on the national SNSF portal is the exception. The FAM metadata says the NFI assessment was to enable permanent plots for future monitoring; this record alone does not establish a repeated census panel.

### F. Papua New Guinea Multipurpose NFI 2016–2019

FAO FAM reference PNG_2019_MNFI_v01_M_v01_A_ESS; producer PNG Forest Authority. The national two-phase stratified design began from 4 km × 4 km grid cells (with a denser 2 km grid in three provinces); 1,000 sampling points were selected. Each selected cluster comprises four circular plots 300 m apart, with nested radii and DBH thresholds of 1, 10, 20 and 40 cm plus regeneration quadrats. Some clusters were not surveyed for security/logistics, and the FAM record says precision/bias information is limited.

**Appropriate role:** PNG-specific forest structure, regeneration sampling-frame and region-stratified validation context.

**Critical limitation:** PNG is not the entire island of New Guinea. The actual microdata live in a country-owned external repository; confidentiality and access conditions are set by that repository. A catalogue record or public method description is not permission to use row-level data. The listed 5–10 year inventory cycle is not proof that a compatible follow-up census has been completed and admitted.

### G. JRC Tropical Moist Forest v2025

**Observed:** 30 m Landsat Collection 2 wall-to-wall annual change layers for 1990–2025, including tropical moist forest extent, degradation, deforestation and post-disturbance regrowth. The product deliberately distinguishes temporary disturbance while forest remains forested from transition to non-forest.

**Operator requirements:**
- exact TMF v2025 asset ID / layer and class ontology;
- observation-validity mask and annual support;
- first observed disturbance timing and intensity/duration where available;
- distinction among degradation, deforestation, regrowth, plantation and water classes;
- explicit transition-mapping uncertainty and source version.

**Scoring:** compare predicted disturbance state and transition timing through the TMF class operator. TMF degradation is not tree mortality, nor is it automatically permanent conversion. Keep TMF class metrics separate from stem mortality and biomass metrics.

### H. MapBiomas Collection 11 (Brazil)

**Observed:** annual Brazil land-cover / land-use classes at 30 m for 1985–2025, with collection- and class/year-specific accuracy information. It is a Brazil product, not a pan-Amazon, Congo or New Guinea forest truth layer.

**Operator requirements:**
- exact collection, territory, year, class level and taxonomy;
- pixel support and the product's accuracy report for the relevant biome / class / year;
- known source-data and algorithm revisions;
- explicit geographic applicability;
- separate native vegetation, plantations, agriculture, water and other land uses.

**Scoring:** evaluate only inside its declared geographic coverage and with its class semantics. Cross-region use outside Brazil is `OUT_OF_DOMAIN`, not a low-confidence observation or a negative result.

## 3. Source metadata references

- Amazon RAINFOR / EIDC 2017–2019 record and Open Government Licence: https://catalogue.ceh.ac.uk/documents/63d4b774-4e03-4db2-95ad-dcca18f0d681
- NASA / ORNL DAAC Sustainable Landscapes Brazil 2009–2018: https://doi.org/10.3334/ORNLDAAC/2007
- DRC NFI 2017–2021 FAM record: https://microdata.fao.org/index.php/catalog/2732
- Papua New Guinea Multipurpose NFI FAM record: https://microdata.fao.org/index.php/catalog/2943
- FAO PNG inventory publication notice: https://www.fao.org/in-action/boosting-transparency-forest-data/news-and-events/news/detail/papua-new-guinea-publishes-national-forest-inventory-in-fam-catalogue/en

## 4. Canonical observation claim

Every admitted observation must retain:

- `observation_id` and `subject_id` (canonical site / plot / footprint / raster support identity);
- `source_provider`, exact product / collection / instrument / algorithm version;
- original asset identifier, checksum/digest and retrieval time;
- source-native valid time or census interval;
- spatial support geometry, area, CRS, datum and positional uncertainty;
- measured variable, unit, value, quality state and uncertainty;
- observation operator identity/version;
- transformations, resampling, masking and aggregation steps;
- label lineage (field measured, product-derived, expert adjudicated, model-derived, unknown);
- rights decision, permitted purpose, derivative rights, retention and attribution obligations;
- train / validation / test grouping and leakage exclusions.

Observation, source-derived label, model inference, simulated state, intervention and decision remain distinct record types.

## 5. Non-interchangeability and missingness

The following states are not synonyms:

- `OBSERVED_VALID`
- `OBSERVED_CONFLICTED`
- `UNOBSERVED_NO_ACQUISITION`
- `UNOBSERVED_CLOUD_OR_QA_MASK`
- `BELOW_DETECTION_LIMIT`
- `OUT_OF_DOMAIN`
- `INVALID_GEOMETRY_OR_REGISTRATION`
- `UNKNOWN_LABEL`
- `OBSERVED_ABSENCE_WITHIN_QUALIFIED_ENVELOPE`

Only the final state can support a negative ecological claim, and only when the source's detectability and coverage are independently qualified for the target class, support and time interval. No-data, cloud, sparse sampling, or a missing plot census cannot become "forest absent", "no disturbance", "zero biomass" or "healthy forest".

## 6. Leakage-resistant evaluation

Split by the unit of independent evidence, not by convenient rows or pixels:

- plot / permanent plot network;
- NFI sampling unit / cluster (never split the four within-cluster plots as independent regional tests);
- site / landscape;
- census campaign and time interval;
- disturbance event;
- sensor / processing version;
- region.

Where multiple products refer to the same event or plot, keep their dependent views in the same split. Spatial adjacency and temporal autocorrelation must be accounted for. Report how many independent sites, plots, events and campaigns actually contribute to each metric.

## 7. Negative-control corpus

The companion JSON file defines nine synthetic expected-disposition cases for:
1. plot-mean versus GEDI-footprint support mismatch;
2. GEDI ISS storage interval misread as forest loss;
3. TMF degradation mislabelled as tree mortality;
4. MapBiomas Brazil model transfer to Congo or Papua New Guinea;
5. cloud / no-acquisition converted into no-change;
6. plot/campaign leakage across train and blind test;
7. the DRC 21 inaccessible/conflict-affected sampling units promoted to forest absence;
8. the PNG security/logistics omissions promoted to healthy/absent forest;
9. a single NFI / plot census treated as a longitudinal growth or mortality panel without a matched prior census.

These nine synthetic descriptors are now evaluated by the standalone Rust rule engine. Its PASS_SYNTHETIC_RULES receipt means the declared missingness/support/semantics inputs produced the expected dispositions. It does not mean production observation adapters are implemented, nor that real GEDI, TMF, MapBiomas, plot or NFI observations have been processed. Production adapter execution, external data ingestion and rights adjudication remain NOT_IMPLEMENTED / NOT_RUN.

## 8. Record-level admission envelope

The operator requirements above are now paired with a machine-readable record admission contract:
[OBSERVATION_ADMISSION_CONTRACT_V0_1.md](OBSERVATION_ADMISSION_CONTRACT_V0_1.md) and
`cases/forest-observation-admission-contract-synthetic-v0.1.json`.

The standalone Rust verifier evaluates eleven synthetic records. It checks the presence and consistency of source/product version, a SHA-256-shaped asset digest, spatial/temporal support, variable/unit, operator identity/version, declared rights purpose, independent split grouping, and claim type. It refuses cloud-as-zero, uncleared rights, unqualified absence, incomplete provenance, cross-partition leakage, unsupported longitudinal inference, unqualified domain transfer, and source/target variable mismatch. No-acquisition remains `PRESERVE_UNOBSERVED`.

This is record-descriptor validation only. A 64-character digest string is not proof that a source registry is trusted or that a real asset was retrieved correctly. A `cleared_for_declared_use` value is a declared upstream rights state, not a decision made by this validator. Geospatial intersection, real data adapters, source-specific detection calibration and Mycelix rights adjudication remain NOT_IMPLEMENTED / NOT_RUN.

## 9. Ownership

- Sol-Atlas owns canonical source asset identity, geospatial geometry, CRS/time, EO product semantics, source rights references and observation evidence.
- Symtropy owns latent ecological process state and the model-to-observation operator adapters for its own prediction outputs.
- Symthaea owns hypothesis comparison, uncertainty interpretation and experiment prioritization.
- Mycelix owns external rights decisions, contributor attribution, custody, revocation and community governance.

A model adapter cannot reinterpret source-native classes or override rights decisions. A visualization or resampling step cannot change canonical observation support.
