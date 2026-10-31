# Manufacturing assurance benchmark protocol (MFG-BENCH-001)

**Parent research:** [MFG-TWIN-VVUQ-001 #3121](https://github.com/Luminous-Dynamics/symtropy/issues/3121).  
**Purpose:** provide fair, external, reproducible comparisons for Symtropy's physical, metrology, conformity, interoperability and intervention-assurance claims.  
**Status:** research protocol. No benchmark result, product advantage, SOTA ranking or factory-impact result is claimed by this document.

## 1. The competitive thesis and its limit

Symtropy should not attempt to recreate a complete CAD/CAM/PLM/MES/CMM/SPC/multiphysics product suite. Established vendors already provide substantial capabilities in those areas:

| Baseline family | Representative current capability | What we should compare |
|---|---|---|
| Siemens digital manufacturing / lifecycle twin stack | Product, process and factory digital-twin workflows and lifecycle integration | Digital-thread completeness, cross-domain provenance, change propagation and closed-loop evidence |
| Ansys Twin Builder / TwinAI / CoSim (2026 R1) | Reduced-order and nonlinear model workflows, hybrid analytics, FMU and multiphysics co-simulation | Physical prediction, model-order reduction, co-simulation fidelity, runtime and uncertainty qualification |
| Autodesk Fusion Manufacturing | CNC probing and part inspection workflows, imported and live inspection results, in-process alignment | Tool/machine process evidence, inspection-to-CNC linkage and dimensional data handling |
| Hexagon Q-DAS / qs-STAT / solara.MP / O-QIS | Measurement data management, real-time SPC, Cp/Cpk/Pp/Ppk, machine/process qualification and MSA | Metrology, MSA, capability, drift, data quality and quality-system interoperability |
| NIST Smart Manufacturing Systems Test Bed | CAD/CAM/CAI/PDM/V&V, CNC milling/turning, inspection resources and MTConnect data | External digital-thread and machine-to-inspection integration |
| NIST AM-Bench | Published, carefully measured additive-manufacturing benchmark datasets and challenge problems | Independent physical-model validation and thermal/process observables |

This competitor list is a set of comparison lanes, not a claim that every product has the same intended use, exposes the same data, or should be replaced by Symtropy.

The differentiator to test is narrower:

> Can a solver-neutral, vendor-neutral assurance layer reproduce the link from a declared physical-model claim through machine and metrology evidence to uncertainty-aware conformity, authorized intervention and independently verified postconditions?

The product is an independent *assurance and evidence-composition layer* over existing engineering systems—not a promise to outperform every specialist CAD, CAM, CAE, SPC, CMM or MES function.

## 2. Benchmark evidence tiers

Never merge these evidence classes into one leaderboard.

| Tier | Evidence | What it can establish | What it cannot establish by itself |
|---|---|---|---|
| T0 — contract fixtures | Synthetic known-answer, boundary, mutation and failure fixtures | Parser, arithmetic, state-machine and verifier behavior | Physical prediction accuracy or factory benefit |
| T1 — analytical references | Closed-form / semi-analytical equations and independently coded oracles | Equation and numerical implementation verification in a declared mathematical domain | Real-process validity |
| T2 — public measured data | Rights-cleared NIST / published experimental datasets | Validation against independent physical observations in the dataset's domain | Generalisation to every material, machine or production line |
| T3 — independent laboratory | Separately measured materials/processes/instruments and holdout campaigns | Replication under an independently controlled experiment | Factory-wide economics or sustained production effect |
| T4 — industrial pilot | Real production with frozen protocol, matched comparison and quality oversight | Context-specific process and economic impact | Universal superiority outside the measured plant/product population |

A result must include its tier, exact context of use, source-data identity, split, model/configuration, metric definition, sample count, confidence interval and claim ceiling. A T0 pass must never be promoted into T2/T3/T4 language.

## 3. External benchmark admissions

### 3.1 NIST Smart Manufacturing Systems Test Bed — first integration anchor

Use the NIST SMS Test Bed as the first external interoperability/digital-thread target. NIST describes a CAx Lab spanning CAD, CAM, CAI, PDM and V&V; a Manufacturing Lab with CNC milling/turning and inspection equipment such as CMMs and digital micrometers; and MTConnect data collected from the manufacturing lab. It publishes access routes for volatile data streams (VDS), a query-able data repository (QDR) and technical data packages (TDP).

Admission plan:
1. Select one specific public machine/device and one bounded observation period or TDP.
2. Freeze source URL, retrieval timestamp, licence/reuse terms, document/schema version, raw byte count and SHA-256 of every exact artifact.
3. Capture MTConnect document type (Probe, Streams, Assets), device/component/data-item identity, instance/sequence and source timestamps.
4. Admit related inspection/measurement evidence only where a published TDP or explicit source association links it. Do not infer a part/lot/operation match from timestamp proximity or naming similarity.
5. Retain missing, stale, reordered, duplicate and unknown data as explicit evidence states.
6. Publish a small rights-cleared fixture and an independently authored expected mapping/association table.

The first success criterion is semantic and identity preservation—not prediction accuracy. A public data stream without an attributable part identity cannot support a product-conformity claim.

Primary source: https://www.nist.gov/laboratories/tools-instruments/smart-manufacturing-systems-sms-test-bed

### 3.2 NIST AM-Bench — physical-model validation anchor

Use AM-Bench as the external measured-physics programme. Prioritise the existing Symtropy scopes:
- AMB2025-06/07 laser-track and thermal/process challenges, including the exact TAM/SCR observables where relevant.
- AMB2025-08 phase-transformation measurements.
- Analytical Stefan verification and progressively richer phase-change models before claiming the measured challenge has been solved.

Every challenge has its own observable definitions, submission rules and comparison procedure. Do not substitute a convenient generic MAE or mean percentage-difference formula for a challenge-specific metric. For AMB2025-07 PCRTAM, retain per-pixel time-above-melt and same-spike solid-cooling-rate extraction, missing-value semantics and spatial/distribution comparisons separately.

AM-Bench challenge submissions are time-bounded; public measurements remain useful for reproducible offline validation after a challenge window closes. A leaderboard award or competition ranking is not inferred when exact disclosed model outputs and scoring data are unavailable.

Admission sources:
- https://www.nist.gov/ambench
- https://www.nist.gov/ambench/am-bench-data-and-challenge-problems-0
- https://www.nist.gov/ambench/direct-am-bench-data-links-and-referencing-guidance

### 3.3 Analytical and numerical references

For phase change and heat transfer:
- Exact one-phase and two-phase Stefan solutions with declared boundary conditions and property assumptions.
- Unequal-property Stefan oracles for the supported parameter regime.
- Independent finite-volume / enthalpy implementations at matched resolution.
- Mesh, time-step and nonlinear tolerance refinement.
- Energy/latent-heat balance and interface-location convergence.

For mechanical/structural domains, use applicable public NAFEMS and other rights-cleared benchmark cases rather than inventing a universal reference. Keep the structural solver reference separate from physical validation.

### 3.4 Industry software comparison

A vendor comparison requires licensed access and a controlled model. Public marketing material is not executable output data.

Minimum fair-protocol record:
- software product/module and exact version/build;
- licence/configuration and enabled solver/model fidelity;
- exact geometry, material, process, boundary and initial conditions;
- mesh/timestep and solver tolerances;
- input/output data digests;
- hardware/OS and wall-clock/resource consumption;
- calibration data, validation data and holdout split;
- operator tuning time and prior model knowledge;
- export precision and post-processing method.

When exact parity cannot be achieved, label the comparison *contextual / non-equivalent* and do not rank it as a head-to-head result.

## 4. Data-splitting and leakage prevention

Manufacturing observations are hierarchical and related. Random row-wise train/test splits are not acceptable as the principal generalisation result.

Use separately reported tracks:
1. Row-wise diagnostic split (not a generalisation claim).
2. Grouped split by physical part/serial and production lot.
3. Leave-one-run / build / machine cycle out.
4. Leave-one-tool / station / instrument / operator out where appropriate.
5. Leave-one-material batch / supplier lot out.
6. Leave-one-campaign / time period out.
7. External-site or independently measured holdout.

Build a provenance relationship graph over raw source artifacts, samples, replicates, derived records, calibration batches, features, labels, model training runs and predictions. Connected provenance families relevant to the claim must not cross train/calibration/test partitions. Unknown relationships remain unresolved; they are not proof of independence.

Calibration, model fitting, uncertainty calibration and final testing must be disjoint for every reported holdout. Any tuning after seeing the external test results invalidates that test as a final holdout and requires a newly held-out set.

## 5. Pre-registration and benchmark execution

Before running a result-bearing campaign, freeze:
- hypothesis and intended engineering claim;
- context of use and supported operating domain;
- primary and secondary metrics, their formulas, units and aggregation rules;
- data admission manifest, licences, checksums and evidence tier;
- split keys and exclusion/missingness policy;
- model families, configurations and hyperparameter budget;
- baselines, hardware, runtime limits and stopping rules;
- uncertainty and confidence-interval procedure;
- required negative controls and abstention behavior;
- analysis code/version and report template.

Retain the pre-registration and all deviations. Exploratory runs may guide the next preregistration but must not be relabelled as confirmatory.

## 6. Metric vector — do not collapse assurance into one score

### 6.1 Physical model / numerical solver

Report for every observable:
- MAE, RMSE, signed bias and normalized errors with a declared denominator;
- per-location / per-time residuals, not only global means;
- conservation residuals (energy, mass, momentum as applicable);
- analytical error and observed mesh/time convergence order;
- sensitivity to solver tolerance and initial/boundary conditions;
- runtime, memory, hardware and throughput at matched fidelity;
- numerical failure and invalid-domain rate.

### 6.2 Prediction and uncertainty

For interval predictions:
- empirical coverage at several declared nominal levels;
- interval width / sharpness;
- coverage and width by independent lot, machine, material, operator and regime;
- calibration curves and clustered confidence intervals.

For predictive distributions:
- proper scores such as CRPS or log score only where the predictive density is valid and normalized;
- quantile loss and tail exceedance calibration;
- separate accounting for residual/unlocated probability mass;
- dependence and shared-nuisance assumptions.

For OOD:
- detection recall at controlled shift severities;
- false abstention on in-domain cases;
- risk-versus-coverage curve;
- selective decision loss and worst-group behavior.

Never award a model a better calibration score merely because it emits uninformatively wide intervals. Plot calibration against sharpness and decision loss.

### 6.3 Measurement-system assurance

Report repeatability, reproducibility, bias, linearity, stability/drift, calibration/reference uncertainty, resolution and traceable uncertainty-budget reconstruction. Distinguish instrument measurement variation from actual process variation. Report MSA study design, crossed/nested structure, balance, sample counts and scope.

### 6.4 Conformity and decision risk

Report by characteristic, product and lot:
- ACCEPT / REJECT_NONCONFORMING / INDETERMINATE / EVIDENCE_INCOMPLETE;
- false-accept and false-reject rates against an independently known or adjudicated reference;
- consumer/producer risk and operating-characteristic curves under the declared population and measurement model;
- inspection and rework costs, expected decision loss and worst-group regret.

The guard-band classifier and posterior probability evaluator are separate tracks. Expanded uncertainty alone is not a PDF. A conformance decision is not a shipment release.

### 6.5 Interoperability / composition

Report:
- exact source identity and byte-digest admission coverage;
- semantic preservation by source element/type;
- association precision/recall against independently adjudicated links;
- false-link / missing-link / unresolved-link rates;
- unit and time-transform residuals;
- stale, duplicate, replayed, reordered and out-of-sequence detection;
- component and interface VVUQ scope match;
- deterministic replay and independent-verifier agreement;
- composition-level residual and uncertainty inflation.

Do not measure a link as "correct" merely because a mapper and its own test oracle share the same production implementation.

### 6.6 Intervention assurance and actual impact

First, measure safety/authority mechanics:
- rejection of expired, duplicate, replayed, unsigned, wrong-scope and out-of-envelope requests;
- zero completion without separately authorized execution and required postcondition receipt in the frozen failure corpus;
- audit completeness and recovery/rollback behavior.

Then, in a real pilot, assess effect:
- first-pass yield, rolled throughput yield, scrap/rework, cycle time, energy per conforming unit, inspection cost and delivery P50/P90;
- stratify or adjust for product mix, material lot, machine, shift, operator and demand;
- use a prospective randomized, stepped-wedge or matched-control design where feasible;
- preserve pre-intervention baselines and concurrent controls;
- report treatment effects with uncertainty, not before/after percentages alone.

A twin's predicted counterfactual is not evidence that the intervention caused the observed improvement.

## 7. Statistical acceptance and decision gates

Avoid arbitrary universal cut-offs. Set thresholds from the engineering risk, specification tolerance, baseline variation, measurement uncertainty, available sample size and decision utility before testing.

For each target, specify:
- the target parameter and minimum engineering-relevant effect;
- acceptable false-accept/false-reject risk;
- required confidence level and cluster structure;
- minimum independent lots/campaigns, justified by a power or precision analysis;
- missing-data and exclusion handling;
- multiplicity correction for multiple characteristics/models/metrics;
- sequential stopping policy if results are monitored during collection.

Use lot/campaign-clustered confidence intervals when lots/campaigns are the independent units. A large number of pixels/points from one build does not equal a large number of independent builds.

A result advances only when its pre-registered primary criteria pass and all required negative controls pass. Report uncertainty and adverse results; never let an average score compensate for a failed safety or evidence gate.

## 8. SOTA claim protocol

A claim of superiority must state:
1. Exact task and context of use.
2. Best disclosed baseline available for that task as of the evaluation date.
3. Whether the comparison is direct, reproduced, paper-reported, vendor-reported or contextual.
4. Exact software/model version and data split.
5. Matched fidelity, inputs, hardware and tuning budget.
6. Uncertainty and independent replication.
7. Failure modes, unsupported scope and abstention.
8. Whether the result is T0/T1/T2/T3/T4 evidence.

Use claim language proportional to evidence:
- "passes the frozen synthetic contract corpus";
- "matches the analytical reference within [pre-registered tolerance]";
- "is validated on NIST AM-Bench case X in domain Y";
- "outperforms baseline B on metric M under matched settings in domain Y";
- "improved pilot outcome by estimated effect Z under prospective comparison at site S."

Do not use an unqualified "better than SOTA", "industry-leading", "ISO-certified" or "factory-proven" description from a synthetic fixture or a single narrow benchmark.

## 9. Proposed first public benchmark release

The first release should be a small, reproducible **Manufacturing Assurance Challenge Pack** containing:

- one rights-cleared NIST SMS Test Bed MTConnect source package;
- one source-linked inspection/QIF example if an admitted public package supplies it (otherwise retain an explicit no-join / unresolved case);
- three ISO 23247-6 composition-mode manifests;
- a frozen synthetic lifecycle graph including calibration/MSA expiry and interface-schema failures;
- one independent structural verifier and separately authored expected records;
- one analytical thermal/Stefan benchmark with solver-convergence report;
- one NIST AM-Bench challenge-specific scorer and fixture;
- one guard-band plus separate posterior-based conformity reference;
- one intervention-authority state-machine negative-control suite;
- per-evidence-tier scorecards and exact source digests.

Release the fixtures and scripts with reproducible instructions and rights/attribution metadata. Keep raw large datasets external when licence terms or size do not permit redistribution.

## 10. Competitive interpretation and priority order

1. **First win:** prove interoperability and lifecycle evidence integrity against the NIST SMS Test Bed and source-pinned MTConnect/QIF fixtures.
2. **Second win:** establish physically credible results on narrow analytical and NIST AM-Bench cases with honest uncertainty.
3. **Third win:** demonstrate uncertainty-aware measurement and conformity decisions outperforming naive point-estimate / independence assumptions on frozen adversarial cases.
4. **Fourth win:** show independently verifiable, bounded interventions and postconditions in offline rehearsal.
5. **Fifth win:** establish measured production benefit in a prospective industrial pilot.

This sequence avoids a common mistake: optimising process predictions before knowing whether the inputs, material identity, measurement system and quality labels are trustworthy.

## References and current product pages

- NIST Smart Manufacturing Systems Test Bed: https://www.nist.gov/laboratories/tools-instruments/smart-manufacturing-systems-sms-test-bed
- NIST AM-Bench: https://www.nist.gov/ambench
- NIST AM-Bench data and challenge problems: https://www.nist.gov/ambench/am-bench-data-and-challenge-problems-0
- ISO 23247-6:2026: https://www.iso.org/standard/87426.html
- ISO/AWI 23247-8 (approved work item, under development): https://committee.iso.org/standard/94858.html
- NISTIR 8620: https://www.nist.gov/publications/digital-twins-workshops-summary-report
- Hexagon Q-DAS SPC: https://hexagon.com/products/product-groups/manufacturing-project-management/statistical-process-control-software
- Hexagon qs-STAT: https://hexagon.com/products/q-das-qs-stat
- Ansys Digital Twin 2026 R1: https://www.ansys.com/en-gb/webinars/ansys-2026-r1-ansys-digital-twin-whats-new
- Autodesk Fusion manufacturing inspection: https://help.autodesk.com/view/fusion360/ENU/?contextId=MFG-PROBING-OVERVIEW
- Autodesk Fusion Inspect Surface: https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-INSPECT-SURFACE-OVERVIEW.htm
