# Thermal Model Validation Contract v0.1

**Tracking issue:** [THERMAL-MODEL-VALIDATION-001 #6412](https://github.com/Luminous-Dynamics/symtropy/issues/6412)  
**Thermal experiment contract:** [ACTUATOR-THERMAL-MISSION-001 #6155](https://github.com/Luminous-Dynamics/symtropy/issues/6155)  
**Thermal mission record:** [PR #6157](https://github.com/Luminous-Dynamics/symtropy/pull/6157)  
**Analytic one-node reference:** [PR #6394](https://github.com/Luminous-Dynamics/symtropy/pull/6394)

**Status:** M1 schema and semantic validator with an illustrative synthetic fixture. No physical thermal model has been fitted or qualified.

## Why this layer is needed

The 2026 reproducible actuator-characterization framework defines eleven tests across no-load, locked-rotor and dynamic-load conditions, and proposes descriptive metrics for load robustness, vibration rejection, dynamic stability and usable bandwidth under load. It also states that these indicators complement standard control metrics and system identification rather than replacing them.

HYPERDYNE (ICRA 2026) contributes a reconfigurable testbed with no-load, fixed-load and interaction configurations, demonstrating extraction of backlash, friction, inertia and frequency-response characteristics.

Symtropy's existing thermal mission record captures test conditions, cooling boundary, initial thermal state, temperature locations, curve points, censoring and raw trace availability. The remaining gap is how to qualify the predictive model on new mission data without leakage or overclaim.

## Paradigm shift: validate mission prediction, not the fitted curve

A model that fits portions of a thermal trace can still fail when initial temperature, load order, cooldown, mounting or cooling changes. The M1-to-M2 progression should therefore hold out entire missions and hardware articles before any windowing, feature scaling, parameter fitting, threshold selection or interval calibration.

A held-out result means a whole mission/profile group not used to fit or tune the model. For stronger generalization, article groups must also be disjoint. A result based on different time windows cut from the same mission is not a valid held-out mission.

## Record contract

The thermal_model_validation.schema.json file (JSON Schema Draft 2020-12) records:

- Exact model class, governing equations, parameter count, values, units and uncertainty status.
- Immutable trace catalogue with article group, mission group, configuration digest and channel calibration provenance.
- Locked training, validation and held-out partitions.
- Fit algorithm, evaluation budget, stopping status, training data digest and identifiability diagnostics.
- Held-out per-trace/per-channel RMSE, MAE, signed bias, maximum error, normalized error, 90%/95% predictive interval coverage and residual autocorrelation.
- Matched baseline/model comparisons over the exact same held-out IDs, split digest and evaluation method.
- Validity domain and extrapolation policy.
- Independent review/reproduction status and claim ceiling.

Unknown identifiability, sensor calibration or limit provenance remains explicit; it does not become a zero error or a physical validation pass.

## Leakage-resistant split contract

The semantic verifier requires each trace ID to appear exactly once in training, validation or held-out sets. It verifies pairwise disjoint group values for each declared grouping key: article_group_id, mission_group_id and/or configuration_digest. The split digest binds the ordered partition IDs. The training-data digest binds the exact training trace IDs and trace digests.

This is intentionally stricter than randomly splitting samples from traces. A model cannot see the first 80% of a thermal mission in training and call the final 20% independent validation.

## Model comparison

Model classes may include:
1. Ambient-only baseline.
2. Exact-step one-node linear RC reference (#6394).
3. Two-node winding/stator/case network where channels and excitation identify its parameters.
4. Multi-node or temperature-dependent models where needed inputs exist.
5. Physics model plus learned residual, or data-driven surrogate, with explicit out-of-domain handling.

All comparisons must use the same held-out trace IDs, split digest, evaluation method and declared fitting budget policy. Report performance vectors; do not invent a universal thermal score. A more complex model should justify its additional parameters with held-out predictive improvement and identifiable parameters.

## Suggested benchmark gates

These are proposed research acceptance gates, not published universal standards:

- No trace, mission, article or configuration leakage across declared holdout group keys.
- Parameter identifiability assessed with sensitivities/singular values, condition number, parameter correlation and (where data permit) bootstrap or profile likelihood.
- Report residuals by sensor location and by mission, not just one pooled RMSE.
- Report signed bias and residual autocorrelation alongside absolute error.
- Calibrate nominal 90% and 95% prediction intervals on held-out missions; show empirical coverage and interval width by thermal channel.
- Threshold-crossing comparisons retain the source sensor/location and censored-time semantics from #6157.
- A model must be rejected or marked out-of-domain beyond its declared ambient, initial-state, cooling, speed, load and configuration envelope.
- Any physical qualification status requires measured traces, calibration certificates, multiple initial states and cooling boundaries, plus independent review.

Avoid hard thresholds on RMSE until sensor accuracy, engineering tolerance and task-level consequences are declared. A 0.5°C error may be material for a winding-limit claim and irrelevant for a case-temperature trend.

## Reproduction and validation

The example is deliberately synthetic and compares a one-node reference with an ambient-only baseline on one synthetic held-out mission. Its numeric metrics exist only to test the evidence contract. It is not a real fitted model or a reported physical result.

Run from the repository root:

    python -m pip install jsonschema==4.24.0
    python tools/validate_thermal_model_validation.py

The validator checks Draft 2020-12 conformance and semantic rules for unique IDs, canonical digests, disjoint mission/article groups, frozen holdout trace sets, model-comparison fairness and evidence references. Negative controls intentionally attempt leakage, changed split manifests, mismatched comparison splits, missing held-out results and false physical claims.

A passing schema and semantic validator proves only that the record is internally consistent. It does not prove the model predicts a physical actuator.

## Claim and safety boundary

This contract does not approve thermal derating, motor protection, safe torque limits, deliberate thermal-limit trips or energized testing. Physical work requires a separately approved guarded rig, calibrated sensors, verified manufacturer limits, competent operators, interlocks and hazard analysis.
