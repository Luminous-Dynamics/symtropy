# Forest Observation Admission Envelope v0.1

**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Branch:** `research/tropical-forest-qualification-v0-1`  
**Status:** synthetic record-level contract; no production adapter, external data ingestion, or rights adjudication

## 1. Purpose

The observation-operator specification describes how source measurements relate to latent ecological state. This companion contract makes the admission boundary executable at the **record descriptor** level.

The paradigm shift is from "load a dataset, then clean it" to **prove that each observation is admissible for a declared claim before it can enter calibration, evaluation, model fitting, or decision support**.

An admitted record must carry source identity, asset digest, measurement semantics, spatial and temporal support, operator identity, quality state, rights-use declaration, and leakage grouping. These fields are not optional metadata: they define what the measurement means and which claims it can support.

This validator evaluates synthetic descriptors only. It does not retrieve, inspect, license, or validate a real source asset.

## 2. Record envelope

The machine-readable development corpus is:

`cases/forest-observation-admission-contract-synthetic-v0.1.json`

Each record contains:

| Envelope | Required content | Why it matters |
|---|---|---|
| Identity | observation ID; subject ID | Stable reference to the measured support and its subject |
| Source | provider; product ID/version; asset SHA-256 | Prevent silent source or processing-version substitution |
| Measurement | variable; value; unit; validity state; detection and coverage qualification | Keep measurement semantics and missingness distinct |
| Spatial/temporal support | geometry ID; CRS; area; valid-time interval; repeat count | Prevent incompatible plot/footprint/pixel comparison and unsupported rates |
| Observation operator | operator ID and version | Bind the conversion from latent state to source-specific measurement |
| Rights | status; intended purpose; allowed purposes | Block use when the declared purpose is not cleared |
| Split lineage | independent group key; partition; related partition | Prevent plot/site/campaign leakage |
| Claim | target variable; longitudinal-rate flag | Ensure the output claim does not exceed the observation's semantics |

The sample asset digest is a synthetic 64-character hexadecimal value. It is not a digest of a real ecological dataset.

## 3. Deterministic refusal / admission rules

Rule precedence is deliberate. An observation with incomplete provenance is not made admissible by a high-quality score or a downstream model. A rights block is not overridden by statistical value.

1. **Provenance completeness.** Missing identity, source/product version, valid SHA-256 syntax, geometry/CRS, temporal support, positive support area, variable/unit, or operator identity/version causes `REJECT_INCOMPLETE_PROVENANCE`.
2. **Rights.** Any status other than `cleared_for_declared_use`, or an intended purpose absent from `allowed_purposes`, causes `BLOCK_RIGHTS_NOT_CLEARED`. This is a check of a declared prior decision—not a legal rights adjudicator.
3. **Missingness.** Cloud, no-acquisition, no-data, QA-invalid, below-detection and observed-absence records cannot carry a numeric zero as a substitute for missingness. Such promotion causes `REJECT_MISSINGNESS_PROMOTED_TO_ZERO`.
4. **Domain.** Source and target domains that differ without a qualified transfer profile cause `OUT_OF_DOMAIN`.
5. **Semantics.** A source measurement variable that differs from the claim target causes `REJECT_TARGET_VARIABLE_SEMANTIC_MISMATCH`. For example, a forest-degradation class is not a tree-mortality count.
6. **Absence.** `observed_absence` is admissible only when both detection and coverage are qualified for the declared support, target and interval. Otherwise it causes `REJECT_UNQUALIFIED_ABSENCE`. A qualified absence is still bounded to that detection envelope; it is not universal proof of ecological absence.
7. **Longitudinal claims.** A rate claim requires at least two matched repeat measurements. A single inventory can establish a state/stock, not a growth, mortality or recovery rate.
8. **Leakage.** A related view from the same independent group cannot cross partitions. The synthetic control represents a plot/campaign group appearing in both train and test.
9. **Unobserved states.** No acquisition remains `PRESERVE_UNOBSERVED`; it is not converted to no-change, zero biomass, healthy forest or absence.
10. **Valid measurement.** A fully described synthetic `observed_valid` record can produce `ADMIT_SYNTHETIC_ONLY`; that disposition is deliberately not a production-data admission.

## 4. Synthetic challenge coverage

The corpus contains eleven public development cases:

- complete synthetic plot observation;
- cloud-masked value incorrectly encoded as zero;
- uncleared rights status;
- unqualified negative/absence claim;
- qualified negative claim within a stated detection envelope;
- invalid asset digest and missing operator version;
- same plot/campaign represented across partitions;
- one census used to assert a longitudinal rate;
- Brazil source domain presented as a DRC target without transfer qualification;
- no acquisition;
- source class / target variable semantic mismatch.

The Rust verifier binds a deterministic receipt to the exact manifest bytes. Three additional tests mutate rights status, absence qualification and expected dispositions. **These test functions are authored, not Cargo-executed yet.**

## 5. Benchmark contract and stretch targets

These are hard-gate targets for this contract—not achieved performance and not a replacement for ecological forecast metrics.

| Dimension | Target before real-data admission |
|---|---:|
| Required provenance/support/operator fields present | 100% |
| Missingness promoted to zero | 0 accepted cases |
| Unqualified absence admitted | 0 cases |
| Rights-purpose mismatch admitted | 0 cases |
| Cross-partition dependent observations | 0 accepted cases |
| Unsupported longitudinal claims admitted | 0 cases |
| Unqualified regional transfer admitted | 0 cases |
| Deterministic receipt replay | byte-identical for identical input bytes |

A later benchmark should compare (a) naïve zero-fill ingestion, (b) schema-only validation, (c) manual source curation, and (d) this executable admission gate under the same mutated record corpus. Report false acceptance and false rejection **per failure family**; never conceal a rights or absence failure in a composite score.

For real observation quality, add source-specific detection/coverage calibration, geometric support-conversion error, uncertainty calibration, and per-region transfer performance. The present corpus does not measure any of those.

## 6. Industry and SOTA interpretation

This contract does not compete with FATES, LPJ-GUESS SEC, GEDI products, tree-crown segmentation or SpeciesNet on their native predictive tasks. It qualifies whether their outputs and field observations can be compared without semantic, support, rights or leakage errors.

The need is current:

- A 2026 systematic review of 190 forest computer-vision studies identifies non-standardized evaluation and cross-biome transfer as major deployment constraints; it reports substantial degradation across biomes and incomplete split/uncertainty reporting. The exact estimates belong to that review and should be treated as review findings, not universal constants: https://link.springer.com/article/10.1007/s10531-026-03412-x
- A 2026 DRC Luki Biosphere Reserve study compared Detectree2, SAM and a hybrid on 5 cm UAV imagery, with 1,882 manually interpreted crowns and a separate 360-tree field inventory. It found metric-dependent precision/recall and crown-area/tree-density trade-offs. The manually interpreted crowns are a human reference, not absolute ground truth: https://onlinelibrary.wiley.com/doi/10.1002/ece3.74260
- A 2026 camera-trap study found broadly similar multi-species occupancy estimates from AI-labelled and expert-labelled images in its study assemblages, but still reported consequential rare-species omissions and confusions. This supports scoring downstream ecological inference, not classifier accuracy alone: https://doi.org/10.1111/1365-2664.70370
- A September 2026 *Nature Geoscience* perspective argues that the central challenge is integrating satellite observations with field evidence, experiments and models across scales: https://www.nature.com/articles/s41561-026-02081-3

Our differentiation should be the **evidence-composition layer**: source-specific observation operators, independent support/uncertainty accounting, hard rights and missingness gates, leakage-resistant splits, and decision-aware selection of the next observation. Prediction superiority must still be established on matched, held-out ecological outcomes.

## 7. Non-goals and claim ceiling

This version does not:

- download or ingest RAINFOR, NASA/ORNL, GEDI, JRC TMF, MapBiomas, DRC NFI or PNG NFI data;
- cryptographically verify a real asset against a trusted source registry;
- adjudicate licence terms, consent, custody, access, revocation or community authority;
- implement a production observation adapter or geospatial geometry intersection;
- calibrate source detection probability or the false-absence risk;
- validate a field forecast, a regional transfer, a conservation decision or an SOTA win.

The current success state is `MANIFEST_VALID`: the public synthetic record descriptors satisfy the declared rule contract. Production adapter execution, real data, rights authority and empirical observation qualification remain `NOT_RUN` / `NOT_ESTABLISHED`.

## 8. Ownership

- Sol-Atlas owns canonical asset identity, geospatial geometry, CRS/time and EO evidence lineage.
- Symtropy owns model-specific observation operators and comparison of latent predictions to source-specific measurements.
- Symthaea may reason over admitted evidence and recommend discriminating observations; it cannot authorize access or interventions.
- Mycelix and the relevant source/community custodians retain rights, consent, attribution, custody and revocation authority.
