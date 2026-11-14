# CEA/cavern experiment campaign adapter v0.1

**Parent:** [CAVERN-EXPERIMENT-VOI-001 #4041](https://github.com/Luminous-Dynamics/symtropy/issues/4041)  
**Composes:** [ACTIVE-DIAGNOSIS-001 #4027](https://github.com/Luminous-Dynamics/symtropy/issues/4027), [ROBUST-EXPERIMENT-001 #4057](https://github.com/Luminous-Dynamics/symtropy/issues/4057), [CAVERN-ANALOG-001 #4002](https://github.com/Luminous-Dynamics/symtropy/issues/4002), [CEA-FULL-BOUNDARY-001 #3957](https://github.com/Luminous-Dynamics/symtropy/issues/3957), [PHOTON-DELIVERY-001 #3985](https://github.com/Luminous-Dynamics/symtropy/issues/3985).

**Status:** documentation/schema/fixture proposal. It is not a qualified experimental protocol, physical authorization or validated decision policy.

## 1. Purpose and boundaries

This adapter describes the decision contract for selecting the next safe, decision-relevant measurement in the uncrewed subsurface analogue. It does not implement a second sequential optimizer.

It consumes measurement definitions and system boundaries from the existing domain owners. It may route an admissible campaign to:
- #4027 when probability distributions, likelihoods, dependence assumptions, utilities and costs are supported in-domain;
- #4057 when a defensible finite set of competing models exists but a single calibrated probability model is not justified;
- HUMAN_REVIEW / NEEDS_EVIDENCE when neither route is justified.

It must not invent a prior, use a uniform likelihood as a missing-data default, or interpret unknown likelihood as zero.

The v0.1 fixture in this directory is synthetic and demonstrates admissibility/routing only. It intentionally selects no physical experiment.

## 2. Pre-register the decision before selecting measurements

Each campaign must declare:
- the decision owner and candidate terminal decisions;
- a baseline/reference architecture and version;
- one functional unit and its food/service definition;
- the complete energy, water, thermal, nutrient and harvest boundary;
- the minimum effect worth resolving (for example, a 10% electricity-intensity reduction);
- hard safety, contamination, preserve, food-quality and service constraints;
- the campaign window and all available resource budgets.

A claim such as "10% lower energy" is not testable until crop, cultivar, seed lot, plant density, crop stage, time window, edible acceptance, wet/dry basis, facility boundary and metering uncertainty are matched or explicitly modelled.

The functional unit is decision-specific. Fresh edible kg, kcal, protein, crop-cycle, protected production availability and service during disruption are not interchangeable denominators.

## 3. Resource costs stay dimensionally separate

Before calling a policy engine, apply hard campaign admissibility against a vector of resources:
- electrical energy (Wh);
- instrument-hours;
- operator-hours and specialist-hours;
- assay/consumable counts;
- crop-cycle or replicate slots;
- monetary cost with currency and time basis;
- prohibited or unapproved actions.

A candidate that exceeds any hard limit is rejected; it cannot trade extra operator-hours for a better energy score.

Preserve the full original resource vector in all outputs. #4027's current interface uses a scalar decision-utility cost. Supply one only if a named decision owner has approved a dimensionally defensible utility mapping and its versioned evidence receipt. Otherwise do not pass the candidate into that scalar-cost engine; use #4057 or human review as appropriate. The engine's search objective must never erase the original resource ledger.

## 4. Measurement admissibility gates

A candidate is eligible only when all required gates pass:

1. It is a PASSIVE_OBSERVATION for v0.1. Any intervention, actuation, pressure change, hazardous challenge, biological release or interlock modification is outside this benchmark.
2. Method, instrument identity, measurement domain and physical sample/batch are declared.
3. Calibration status is current and in-range for the actual operating interval.
4. Time basis, synchronization, sampling interval and raw-data retention are specified.
5. Common-cause, calibration-chain, sample-lineage and dependence groups are declared.
6. Resource cost is known in every applicable dimension.
7. A qualified measurement likelihood exists, or the candidate is routed to bounded-model analysis / human review without fabricating a probability.
8. Preserve, safety, food-quality and containment rules permit the observation.

Statuses are ACCEPT, REJECT or NEEDS_EVIDENCE. Unknown is not equivalent to ACCEPT. A sensor's channel count does not establish independent measurement principles.

## 5. CEA reporting alignment

The data dictionary should map to external reporting vocabulary without claiming certification:

- ISO/AWI 34713-1 is an approved work item under development, not a final published standard. It covers general CEA concepts, control framework, functional requirements and data, including greenhouses and vertical farms: https://www.iso.org/standard/93560.html
- ICCEG / NCERA-101 provides existing guidance for measuring and reporting environmental parameters in controlled-environment plant experiments: https://www.controlledenvironments.org/reporting-guidelines/
- The July 2026 LBNL review assesses 80 technologies and ten region-specific pathways; reported energy and water savings are case-specific, not universal guarantees: https://doi.org/10.1016/j.seta.2026.105136

At minimum retain crop/cultivar, seed lot, stage, canopy position/layer, PPFD time series, DLI, spectrum, temperature, RH, CO2, airflow, water quality, nutrient assay, metered electricity, thermal observations, harvest dispositions, calibration, uncertainty and missingness. Do not silently convert broadband irradiance to PAR/PPFD without a declared spectral model.

## 6. Frozen benchmark portfolio

Before comparing policies, create at least 30 deterministic cases with hidden truth and an independently authored evaluator. Include:
- a real 10% improvement resolvable by qualified meters;
- the same effect obscured by drift or high uncertainty;
- no-effect and negative-effect controls;
- a crop-cycle confounder that reverses a naive result;
- optical transmission degradation versus source variability;
- canopy non-uniformity versus mean-DLI reporting;
- daily-energy pass but peak-power failure;
- thermal load shifted outside the declared boundary;
- water recovery with increasing contaminant concentration;
- unmetered storage change or purge;
- harvest wet/dry basis mismatch;
- correlation between two nominally independent channels;
- missing likelihood / out-of-domain model;
- unsafe or unauthorized intervention candidate;
- one candidate that is high-information but cannot change the terminal decision;
- one adaptive two-stage case where the best second measurement depends on the first outcome;
- a case where STOP is optimal;
- a finite alternative-model case where robust policy differs from Bayesian EVSI;
- a case where expert review is the only admissible disposition.

All synthetic values must be obviously marked synthetic and must not be confused with a measured facility baseline.

## 7. Baselines and equal-budget comparison

Compare:
1. fixed pre-registered checklist;
2. round-robin measurements;
3. maximum-entropy / information-gain selection (diagnostic baseline only);
4. one-step EVSI (#3917);
5. exact finite-horizon Bayesian policy (#4027);
6. finite-model robust selection (#4057);
7. an independent exhaustive oracle on bounded cases;
8. a blinded expert campaign plan when available.

Charge every method for observation, setup, calibration, failed/aborted measurement, analysis, repeat work and decision-review overhead. Use identical evidence, admissible actions and resource ceilings. Keep resource dimensions separate.

## 8. Metrics

Report separately:
- false-superiority and decision-regret outcomes on hidden truth;
- lower-tail / worst-model regret;
- rate of measurements that change the admissible decision;
- cost per decision-changing observation;
- Wh, instrument-hours, operator-hours, specialist-hours, assays and crop-cycle slots;
- calibration/uncertainty coverage and out-of-domain abstention;
- dependence-model violations and rejected candidates;
- time and measurements to STOP / HUMAN_REVIEW;
- facility functional-unit results and Pareto alternatives.

No pooled average may hide a false-clear, preserve breach, unsafe proposal or false-superiority result.

## 9. Proposed progression targets (not achieved)

Freeze thresholds and the evaluation corpus before policy comparison. Initial targets:
- zero false-superiority outcomes on the frozen negative-control set;
- zero unsafe / unqualified / out-of-domain measurement selections;
- exact agreement with brute-force policies on bounded golden cases;
- non-inferior decision quality to the best simple baseline at identical total budget;
- stretch: at least 20% lower held-out decision regret or campaign cost without degrading any hard constraint;
- measured effect claims have uncertainty small enough to resolve the pre-registered minimum effect;
- STOP / HUMAN_REVIEW is counted as correct when evidence does not support a physical conclusion.

If the adaptive policy does not outperform the simple checklist on a case family, preserve that result and prefer the simpler method there.

## 10. Physical evidence progression

A passing schema, fixture or synthetic policy benchmark does not authorize experiments. Physical work requires site/geotechnical review, hazard analysis, permissions, qualified professionals, instrument calibration and an approved uncrewed procedure. Human occupancy, pressure-habitat qualification, pathogens, hazardous chemicals and planetary analogues remain separate authorization tracks.

Raw observations, corrected measurements, model outputs and adjudications stay distinct. Corrections create a new version and retain the original bytes, actor, reason and time. Mycelix provenance can preserve the record but cannot prove that a sensor reading is true.

## 11. Claim ceiling

v0.1 establishes only a proposed data/decision interface and synthetic admissibility semantics. It does not qualify the sequential engine, prove a real likelihood model, approve a physical experiment, establish crop yield, certify food or water quality, close life support, or qualify a cavern or planetary habitat.


## Addendum — representative measurement under distribution shift

The adapter now has a deterministic synthetic counterexample package:
- [CAVERN-REPRESENTATIVENESS-SHIFT-001 protocol](./CAVERN_REPRESENTATIVENESS_SHIFT_BENCHMARK_V0_1.md)
- [JSON Schema](./cavern_representativeness_shift_case.schema.json)
- [Known-answer fixture](./fixtures/cavern-representativeness-shift-001.v0_1.json)

It separates training and deployment strata and freezes a case where a maximum-mutual-information measurement wins on the nominal/training mix but loses on a shifted decision-loss objective. All values are SYNTHETIC_ONLY. This composes with #3171; it does not implement R-IDeA or a new acquisition engine.
