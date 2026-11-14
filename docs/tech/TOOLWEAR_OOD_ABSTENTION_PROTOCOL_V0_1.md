# MFG-DATA-003 — OOD-aware selective prediction and abstention protocol v0.1

## Status and claim ceiling

**Protocol proposal only.** This document defines a next benchmark after the Symtropy MFG-DATA-002 leave-one-tool-out (LOTO) diagnostic proposed in [PR #2178](https://github.com/Luminous-Dynamics/symtropy/pull/2178). It does not implement an OOD detector, validate a calibrated prediction interval, or report an observed selective-prediction result.

Keep the MFG-DATA-001 and MFG-DATA-002 report contracts and outputs unchanged. A future MFG-DATA-003 report must have its own schema/version and digest. The current pinned Vicomtech feature table has 13 tool IDs and source-native target `Vb`; the upstream README does not specify physical units or the necessary instrument/calibration history. Therefore it is **external-dataset evaluation, unqualified for process control**.

## Paradigm shift

Do not force every model prediction to become a recommendation. Make **abstention a first-class, auditable model output**.

A useful industrial system should expose three separate questions:

1. **Prediction:** What does the fixed model estimate?
2. **Applicability:** Does the input resemble the data/domain where that model was evaluated?
3. **Decision:** Is evidence sufficient for the proposed use, or must the system abstain and request a measurement, review, or qualified fallback?

These states must not be collapsed into a single confidence score. An OOD score is not a probability of correctness; a narrow interval does not prove validity under shift; a human review does not erase an OOD flag; and an abstention is not a failed prediction.

This direction matches NIST's use of controlled physical measurements to validate manufacturing models in [AM-Bench](https://www.nist.gov/ambench) and the NIST AI RMF's emphasis on measurement, monitoring, uncertainty, and documented evaluation ([AI RMF Core](https://airc.nist.gov/airmf-resources/airmf/5-sec-core/), [Measure playbook](https://airc.nist.gov/airmf-resources/playbook/measure/)). It also responds to industrial condition-monitoring research on machine-to-machine domain shift, including Goodarzi, Schütze and Schneider (2025), [*Domain shifts in industrial condition monitoring: a comparative analysis of automated machine learning models*](https://doi.org/10.5194/jsss-14-119-2025).

## Benchmark question

Can a predeclared, target-label-blind applicability detector reduce prediction risk on unseen tool groups at useful coverage, while avoiding excessive abstention on in-domain examples?

Compare all methods at matched task, data access, training budget, feature inputs, and evaluation groups. Do not compare only overall MAE or an OOD AUROC: a detector that abstains on everything can appear safe while having no utility.

## Data separation and leakage rules

- Freeze the data bytes, parser, feature names, target, tool-group identities, executable revision, and report schema by digest.
- Use tool-group-disjoint outer evaluation. No row from the held-out tool may be used to fit a predictor, normalize features, select an OOD threshold, tune abstention, select features, or choose a model.
- A deployment-style detector may inspect **unlabeled** input features from a target batch only if that access is explicitly declared in advance. It must not inspect held-out `Vb` labels when fitting its score or threshold.
- Threshold selection and method selection belong inside the training/development groups. The held-out tool's labels are for evaluation only.
- Do not use synthetic shifted features as if they were physical observations. Report synthetic stress tests in a separate evidence class.
- Retain the fixed constant and predeclared single-feature models from MFG-DATA-002 as transparent prediction baselines. Add no opaque or adaptive comparator without a preregistered compute/data budget.
- An invalid schema, changed source bytes, missing required feature, non-finite value, or failed integrity check must reject before model inference; it is not merely an OOD case to be accepted with a warning.

## Required output

A versioned machine-readable report must include:

- source and executable digests, schema version, split identity, target/features, feature/target units where defined (otherwise explicitly `unspecified`), evidence class, allowed target-domain access, and threshold-selection method;
- for each outer tool fold: training/test tool IDs, row denominators, training-fitted preprocessing, OOD score definition, fixed threshold, accepted/abstained counts, and per-tool error metrics;
- unconditional MAE/RMSE and accepted-only (selective) MAE/RMSE, both pooled and macro-tool;
- coverage (accepted fraction), abstention fraction, false-accept rate at predeclared high-error criteria, and false-abstain rate on predeclared in-domain controls;
- risk-versus-coverage points and area under the risk-coverage curve, with the exact convention documented;
- OOD detection AUROC/AUPRC only when ground-truth domain labels exist for the retrospective evaluation; clearly label these as diagnostic metrics, not evidence of calibrated probabilities;
- per-tool and worst-tool risk, confusion counts, all thresholds, and exclusions; never emit only one aggregate score;
- deterministic replay identity and machine-readable rejection reasons.

The held-out tool labels may score the report after the decision is made; they must never change that decision.

## Protocol and controls

### P0 — Integrity and leakage

1. Re-run unchanged MFG-DATA-001 and MFG-DATA-002 checks.
2. Prove held-out groups are disjoint from all model fitting and threshold selection.
3. Add a test that deliberately mutates only held-out target labels and verifies that predictions, OOD scores, thresholds, and accept/abstain decisions are byte-identical; only outcome-scoring fields may change.
4. Verify deterministic replay, input-order invariance where semantics are row-order independent, and rejection of same-length source mutation.

### P1 — Predeclared applicability baselines

Compare only methods that can be fit without target labels: simple robust feature-range checks, standardized feature-distance baselines where covariance/scale estimators are specified, and an explicit always-accept comparator. Define missing-value behavior before fitting. Keep this benchmark small and interpretable before considering neural or adaptive OOD models.

### P2 — Stress and falsification

Use labeled synthetic fixtures for feature scale shifts, correlated-feature changes, heavy-tailed values, feature missingness, corrupted units/schema, and distribution-preserving row shuffles. Keep each stressor's seed and transform identity in the report. Distinguish invalid/corrupt input (must reject) from a syntactically valid but unfamiliar domain (may abstain). Ensure that adding an OOD warning cannot silently bypass an existing safety/quality gate.

### P3 — External replication

Require additional independent tool/site datasets with documented target units, measurement definitions, instrument/calibration status, group identities, and licensing. Keep final external test groups sealed until the methods and thresholds are frozen. Match published comparators' target, input modality, split, and metric aggregation before claiming parity with a paper or benchmark.

### P4 — Physical utility

Only a separately approved, controlled trial may establish utility. Compare a locked prediction-plus-abstention policy against the existing maintenance / measurement / review process. Record false accepts, unnecessary abstentions, inspection burden, downtime, conforming output, escaped defects, and energy per conforming unit. Safety and quality constraints remain non-compensatory; higher throughput cannot compensate for a critical safety failure or escaped defect.

## Metrics and provisional goals

Report every item below; do not optimize one metric at the expense of hiding another.

| Metric | Required interpretation |
|---|---|
| Unconditional MAE/RMSE | Baseline prediction quality without abstention |
| Selective MAE/RMSE | Prediction error among accepted cases, always paired with coverage |
| Coverage / abstention | Fraction accepted / deferred for more evidence |
| Risk-coverage curve | Error as coverage varies under thresholds chosen without test labels |
| False-accept rate | Fraction of predeclared high-error cases accepted; report denominator |
| False-abstain rate | Fraction of in-domain control cases unnecessarily abstained; report denominator |
| Worst-tool error | Exposes the domain hidden by the aggregate |
| OOD AUROC/AUPRC | Ranking ability only, not probability calibration or operational value |
| Decision invariance under target-label mutation | A hard anti-leakage test, not a model-quality score |

**Stretch target to investigate, not a result or safety guarantee:** on held-out shifted groups, seek at least 30% lower selective MAE at at least 80% coverage than the unconditional baseline, while keeping false-abstain rate at or below 5% on predeclared in-domain controls. These numbers are hypotheses for a future benchmark round; ratify them only after inspecting baseline variability, group count, and measurement quality. If the target is not met, publish the failure and per-tool breakdown rather than changing the threshold after seeing test labels.

The current 13-tool dataset is too small to justify broad population guarantees. Report group counts and uncertainty; do not treat rows within a tool as independent samples or imply the tools represent independent factories. A passing synthetic suite proves protocol behavior only.

## Integration boundaries

- **Symtropy:** owns the tool-disjoint feature-evaluation diagnostics.
- **Symthaea:** owns paired production-outcome deltas and the cluster-level descriptive interval in [PR #8129](https://github.com/Luminous-Dynamics/symthaea/pull/8129); a prediction interval or OOD score must not be mistaken for that outcome interval.
- **Mycelix:** [process-run receipt PR #5228](https://github.com/Luminous-Dynamics/mycelix/pull/5228) can preserve the declared evidence origin, source/config/report digests, OOD status, and review references. A receipt's digest syntax does not authenticate bytes or signer identity.
- **Sol-Atlas:** spatial/site context can supply provenance-bearing context, but geography, facility categories, or utility proximity cannot establish a facility's qualified process capability.

Do not create a second custody ledger, process-run receipt schema, or site ontology. Extend the owners' existing contracts by versioned references after compatibility is demonstrated.

## Exit criteria for implementation

- [ ] Frozen benchmark protocol and machine-readable schema version.
- [ ] Existing MFG-DATA-001/002 reports remain unchanged.
- [ ] Outer tool groups, feature/label access, and threshold fitting are auditable.
- [ ] Target-label mutation cannot affect decisions.
- [ ] Invalid inputs reject; valid-but-unfamiliar inputs can abstain.
- [ ] Risk/coverage, false-accept, false-abstain, macro, pooled, and worst-tool metrics reconcile from the same per-tool outputs.
- [ ] Deterministic report with source/code/split digests.
- [ ] Independent external-domain test completed before any SOTA claim.
- [ ] Physical utility only claimed after a controlled instrument-observed trial.
