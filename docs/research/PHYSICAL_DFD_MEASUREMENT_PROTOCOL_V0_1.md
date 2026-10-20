# Physical Disassembly Measurement Protocol v0.1

**Tracking:** [PHYS-DISASSEMBLY-OBS-001 #2282](https://github.com/Luminous-Dynamics/symtropy/issues/2282)  
**Related DfD implementation:** [draft PR #2271](https://github.com/Luminous-Dynamics/symtropy/pull/2271)  
**Status:** proposed protocol; no measured outcomes are claimed.

## 1. Research question

For connector/component families within a defined test envelope, does Symtropy's pre-task DfD metadata vector predict actual disassembly effort, required tools, damage risk, connector reuse, and retained component condition better than simple baselines?

The model is not to be scored against outcomes it has already seen. Freeze predictions before the task, preserve raw observations and calibration provenance, and reserve whole specimen/joint families for blind holdout.

## 2. Scope and controls

Start with purpose-built, non-load-bearing joint coupons or bench assemblies. Do not use occupied buildings, load-bearing structures, or real demolition tasks. Exclude thermal cutting, hazardous dust, heavy suspended parts or other higher-risk operations unless they have a separately reviewed method, qualified facility and approved controls. Safety controls are not an optimization target and must not be weakened to increase throughput.

A task is a defined disassembly operation on one specimen/connection set under a recorded protocol. Specify when timing starts and stops, whether setup/cleanup counts, whether it is wall-clock or active labor time, and how pauses, rework, unsafe conditions and incomplete tasks are handled. Crew size and summed person-minutes must not be conflated.

## 3. Freeze the prediction

Before task start, record a versioned prediction linked to:
- specimen/component/connection IDs and the model-input digest;
- connector kind and source of connector semantics;
- declared disassembly effort and its unit;
- required tool class, tool-access state, inspection-access state;
- predicted damage risk and connector reuse class;
- predictor version/configuration and the exact source revision;
- any unknown/unresolved field.

Do not edit a prediction after seeing the result; issue a new version with a reason if a factual pre-task input was corrected.

## 4. Observation record

Record at minimum:

| Observation | Required detail |
|---|---|
| Identity | specimen, component, connection, repetition ID |
| People/time | anonymized operator IDs, crew size, start/stop, pauses, active time, wall-clock time, summed person-minutes |
| Tools | actual tool class, tools changed/added, special equipment, deviations |
| Access | actual tool/inspection access, obstruction and access changes |
| Outcome | task complete/incomplete, number of steps/fasteners, rework, safety stop |
| Damage | component damage category, measured condition change where feasible, connector damage/reuse outcome |
| Materials | input/output weights only where relevant, scale ID/calibration ref/resolution/uncertainty |
| Provenance | observer, timestamps, source record and adjudication note |

Keep task video optional. Collect the minimum worker data required, use consent and privacy safeguards, anonymize identifiers before sharing, and do not publish personally identifiable recordings without appropriate consent.

## 5. Quality and uncertainty

- Calibrate scales and timers where used, and preserve calibration references, resolution, and declared uncertainty.
- Record units explicitly. Empty strings, unbound identifiers, duplicate observation IDs or impossible timestamps fail validation.
- A task stopped for safety, abandoned, damaged beyond recovery or unable to expose the connector is an outcome, not a silently discarded outlier.
- Distinguish measured, derived, estimated, synthetic and unknown fields. Never upgrade an estimate because it agrees with the prediction.
- Preserve observer disagreements and adjudication; do not overwrite original ratings.

## 6. Split policy

Group repeated measurements by specimen/joint design and family. Keep whole families in a blind holdout; do not split near-identical replicas across calibration and holdout unless a separate evaluation explicitly tests repeatability within the same family. Freeze the split before calibration/tuning, publish the split manifest and its digest, and do not retune based on holdout output.

Where feasible, randomize the order of connector tasks and use repeated independent trials to reveal operator learning, fatigue and observer effects. Report operator and specimen variance separately rather than presenting all repetitions as independent samples without justification.

## 7. Metrics and baselines

Report metrics separately:

- Effort prediction: median absolute error, MAE, P95 absolute error, relative error where denominator is meaningful, calibration of prediction intervals, and abstention rate.
- Tool prediction: confusion matrix, class-specific recall and false-confidence rate for unsupported/unknown tool classes.
- Outcome prediction: damage/reuse confusion matrix and precision/recall for reusable parts/connectors.
- Process feasibility: completion rate, prerequisite violations, rework rate, incomplete/censored task rate and safety-stop rate.
- Robustness: results by joint family, operator, access condition, tool availability and specimen condition.
- Value recovery: retained component count/condition and material quantities only where measured/provenance-bound.

Use at least two naive baselines:
1. constant median effort from the calibration set;
2. connector-kind-only predictor.

Compare against DEI/DCI only after pinning the published calculation, exact input tables, units, interpretation and attribution. A more complex vector is useful only if it produces better held-out prediction or clearer honest abstention than the simple baselines.

## 8. Candidate acceptance gates

These are proposed project gates, not achieved results or universal standards:

1. 100% of benchmark records validate or are explicitly rejected as incomplete.
2. No test case can be silently removed after the outcome is known.
3. Holdout is grouped by specimen/joint family and frozen before tuning.
4. Publish all observed cases, missingness, safety stops, uncertainty and model abstentions.
5. Require baseline deltas and a second independent replication before a superiority claim.
6. Limit claims to the measured connector families, tools, operators and conditions.

Set numerical effort-error thresholds only after a pilot establishes measurement repeatability and the operational scale of practical error. Pre-register them before the blind evaluation; never relax them after seeing the holdout.

## 9. Rights, ethics, and claim ceiling

The protocol itself contains no measured dataset. Any newly collected records require rights, consent, safety review and an explicit sharing policy before publication. Existing DEI/DCI/DeCon publications are methodological baselines, not a substitute for ground truth.

Passing this protocol validates only the reported outcomes within the tested envelope. It is not a building-collapse benchmark, structural safety determination, demolition method statement, building-code verdict or machinery-control authorization.
