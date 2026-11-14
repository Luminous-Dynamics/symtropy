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

### C. JRC Tropical Moist Forest v2025

**Observed:** 30 m Landsat Collection 2 wall-to-wall annual change layers for 1990–2025, including tropical moist forest extent, degradation, deforestation and post-disturbance regrowth. The product deliberately distinguishes temporary disturbance while forest remains forested from transition to non-forest.

**Operator requirements:**
- exact TMF v2025 asset ID / layer and class ontology;
- observation-validity mask and annual support;
- first observed disturbance timing and intensity/duration where available;
- distinction among degradation, deforestation, regrowth, plantation and water classes;
- explicit transition-mapping uncertainty and source version.

**Scoring:** compare predicted disturbance state and transition timing through the TMF class operator. TMF degradation is not tree mortality, nor is it automatically permanent conversion. Keep TMF class metrics separate from stem mortality and biomass metrics.

### D. MapBiomas Collection 11 (Brazil)

**Observed:** annual Brazil land-cover / land-use classes at 30 m for 1985–2025, with collection- and class/year-specific accuracy information. It is a Brazil product, not a pan-Amazon, Congo or New Guinea forest truth layer.

**Operator requirements:**
- exact collection, territory, year, class level and taxonomy;
- pixel support and the product's accuracy report for the relevant biome / class / year;
- known source-data and algorithm revisions;
- explicit geographic applicability;
- separate native vegetation, plantations, agriculture, water and other land uses.

**Scoring:** evaluate only inside its declared geographic coverage and with its class semantics. Cross-region use outside Brazil is `OUT_OF_DOMAIN`, not a low-confidence observation or a negative result.

### E. Papua New Guinea National Forest Inventory

FAO announced publication of Papua New Guinea's multipurpose national forest inventory in the Food and Agriculture Microdata (FAM) Catalogue on 15 March 2026. The announcement describes a nationally representative forest assessment covering forest resources, biodiversity, carbon stocks, land use and community socioeconomic conditions.

Before use, verify the actual catalogue package, sampling design, variables, row-level disclosure, release terms and permissions with the Papua New Guinea Forest Authority / FAO. Catalogue visibility is not blanket authorization to redistribute microdata or publish sensitive locations.

## 3. Canonical observation claim

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

## 4. Non-interchangeability and missingness

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

## 5. Leakage-resistant evaluation

Split by the unit of independent evidence, not by convenient rows or pixels:

- plot / permanent plot network;
- site / landscape;
- census campaign and time interval;
- disturbance event;
- sensor / processing version;
- region.

Where multiple products refer to the same event or plot, keep their dependent views in the same split. Spatial adjacency and temporal autocorrelation must be accounted for. Report how many independent sites, plots, events and campaigns actually contribute to each metric.

## 6. Negative-control corpus

The companion JSON file defines synthetic cases for:
1. plot-mean versus GEDI-footprint support mismatch;
2. GEDI ISS storage interval misread as forest loss;
3. TMF degradation mislabelled as tree mortality;
4. MapBiomas Brazil model transfer to Congo or Papua New Guinea;
5. cloud / no-acquisition converted into no-change;
6. plot/campaign leakage across train and blind test.

These cases currently define expected dispositions; they are not yet wired into the Rust accounting verifier. The accounting verifier's PASS must not be represented as passing these observation-operator cases.

## 7. Ownership

- Sol-Atlas owns canonical source asset identity, geospatial geometry, CRS/time, EO product semantics, source rights references and observation evidence.
- Symtropy owns latent ecological process state and the model-to-observation operator adapters for its own prediction outputs.
- Symthaea owns hypothesis comparison, uncertainty interpretation and experiment prioritization.
- Mycelix owns external rights decisions, contributor attribution, custody, revocation and community governance.

A model adapter cannot reinterpret source-native classes or override rights decisions. A visualization or resampling step cannot change canonical observation support.
