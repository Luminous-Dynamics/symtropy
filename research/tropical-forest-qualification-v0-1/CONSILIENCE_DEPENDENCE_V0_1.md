# Cross-Modal Consilience and Evidence-Dependence Qualification v0.1

**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Focused work:** [FOREST-CONSILIENCE-001 #5257](https://github.com/Luminous-Dynamics/symtropy/issues/5257)  
**Status:** synthetic pairwise lineage rules; no source registry, probabilistic dependence model or evidence fusion

## 1. Purpose

A forest evidence system may consume satellite products, lidar, field plots, UAV imagery, camera traps, passive acoustic monitoring and eDNA. These are different observation processes. They are not automatically independent confirmations.

Two products can carry different names but share the same raw image, upstream calibration data, processing pipeline, validation set, site-selection bias, sensor deployment or environmental disturbance. Treating such records as independent can make confidence increase without new evidence.

The paradigm shift is **consilience before fusion**:
- preserve each source's native claim and uncertainty;
- establish support, time, validity and rights for each source separately;
- inspect source lineage and possible dependence;
- compare agreement and conflict before combining;
- fuse only when dependence is modeled or bounded.

An August 2026 *Remote Sensing in Ecology and Conservation* perspective describes six nature-technology integration paradigms—survey design, tip-and-cue, validation, consilience, interpolation/extrapolation and data fusion. In its seven case studies, consilience appeared in every case and often preceded later quantitative integration. It also emphasizes scale/time mismatches, sparse coincident multimodal evidence and rights/metadata/scientific-rigor requirements: https://doi.org/10.1002/rse2.70100

## 2. Pairwise dependency descriptor

The first synthetic descriptor declares:
- product/source identity;
- upstream asset IDs;
- acquisition/census campaign;
- site/deployment identity;
- sensor/platform identity;
- calibration dataset IDs;
- training dataset IDs;
- common-cause IDs (weather, access, power, regional event, observer effort, etc.);
- processing pipeline IDs;
- a list of dependency dimensions explicitly assessed;
- a declared lineage-completeness state.

The required dimensions in this version are:
`upstream_assets`, `campaign`, `site`, `sensor`, `calibration_data`, `training_data`, `common_causes`, and `processing_pipeline`.

A declared complete descriptor is not proof of completeness. Production use requires Sol-Atlas/source-registry lineage verification, not merely trusting these fields.

## 3. Dispositions and precedence

The pairwise rule engine emits one of four conservative states:

| Disposition | Meaning |
|---|---|
| `SHARED_UPSTREAM_EVIDENCE` | Both products derive from at least one same upstream asset; do not count as independent evidence |
| `SHARED_CAMPAIGN` | Both observations belong to the same declared field / acquisition campaign |
| `DEPENDENCE_MODEL_REQUIRED` | A common site, sensor, calibration/training set, common cause or processing pipeline is detected |
| `DEPENDENCE_UNKNOWN` | Lineage is incomplete or one or more required dependence dimensions were not assessed |
| `DECLARED_INDEPENDENT_GIVEN_CONDITIONS` | All required dimensions are declared assessed and no shared identity is found; this is conditional on the declared lineage being accurate, not a proof of universal statistical independence |

Precedence is deliberately conservative:
1. incomplete lineage or unassessed dimensions → `DEPENDENCE_UNKNOWN`;
2. common upstream asset → `SHARED_UPSTREAM_EVIDENCE`;
3. common campaign → `SHARED_CAMPAIGN`;
4. other detected shared dependencies → `DEPENDENCE_MODEL_REQUIRED`;
5. otherwise → `DECLARED_INDEPENDENT_GIVEN_CONDITIONS`.

This version is a deterministic lineage screen, not a numerical correlation estimator and not a probability model. Unknown dependence is never mapped to independent.

## 4. Public synthetic corpus

`cases/forest-consilience-dependence-synthetic-v0.1.json` provides twelve pairwise cases:
1. two products share an upstream scene;
2. plot and UAV observations share a campaign;
3. GEDI and a biomass model share calibration data;
4. two classifiers share training data;
5. optical and acoustic observations share a regional storm;
6. plot and camera observations share a site;
7. two products share a sensor platform;
8. two classifiers share a processing pipeline;
9. fully declared distinct lineages with no shared dependency;
10. incomplete lineage;
11. a required dependence dimension is unassessed;
12. different product names derive from the same raw satellite asset.

The corpus is synthetic. It does not encode actual product dependencies or prove that two real sensors are independent.

## 5. Benchmark plan

Compare:
- naïve independence inferred from different product IDs;
- source-aware but dependency-blind fusion;
- lineage-aware consilience without fusion;
- explicit dependence-aware fusion after an independent dependency model is qualified;
- leave-one-source and leave-one-lineage-out checks;
- human adjudication where a suitable independent expert reference exists.

Report:
- false corroboration / pseudo-replication rate;
- dependence-detection recall by shared dependency family;
- false dependence alarms on declared independent controls;
- change in uncertainty when shared sources are correctly treated as dependent;
- leave-one-lineage-out stability;
- conflicts retained for review or abstention;
- decision-value of a targeted independent observation at matched survey cost.

A source pair can be conditionally independent only under a stated set of assumptions. If dependence cannot be measured, do not multiply likelihoods or confidence scores as though it were absent.

## 6. Future evidence workflow

The proposed operational composition is:
1. **Admit** individual records under the observation-admission contract.
2. **Convert support** through #5237 and the variable-aware aggregation semantics.
3. **Compare** source-native claims with time, support, detection and uncertainty intact.
4. **Trace** common ancestors, site/campaign effects and calibration/training reuse.
5. **Classify** concordance, conflict, shared evidence, dependence-required or unknown.
6. **Select** an independent follow-up observation using existing #2171 / #5223 decision-value work.
7. **Fuse** only when dependence and observation operators are qualified; otherwise preserve a set of claims or abstain.

No system may authorize data access, field entry or conservation intervention through this benchmark.

## 7. Claim ceiling

A successful synthetic corpus receipt means only that these public lineage descriptors receive the frozen deterministic dispositions. It does not authenticate real asset lineage, establish statistical independence, calibrate multimodal posterior uncertainty, qualify ecological truth, or demonstrate conservation impact.
