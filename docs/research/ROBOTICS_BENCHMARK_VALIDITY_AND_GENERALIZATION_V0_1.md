# Robotics Benchmark Validity and Generalization Contract v0.1

**Date:** 2026-10-22  
**Status:** research protocol and audit-tool specification; no robot performance results are asserted.

## Why another robotics benchmark layer?

A benchmark is useful only if measured improvement reflects capability that transfers beyond its fixed test setup. A recent 2026 manipulation-benchmark audit identifies four recurring threats: shortcut solvability, gains without demonstrable statistical significance, test-set overfitting, and dependence on the source of training demonstrations. Its audit of LIBERO, CALVIN, SimplerEnv, RoboCasa, and RoboTwin 2.0 shows why a high aggregate score alone can be weak evidence. See [Jiang et al., *What Are We Actually Benchmarking in Robot Manipulation?*](https://arxiv.org/abs/2606.04233).

Mobility testing also needs to reflect real integration problems. NIST's current mobile-robot measurement program highlights slippage, uneven flooring, dynamic obstacles, vibration, changing loads, forced delocalization, and external optical tracking for evaluating localization in dynamic environments: [NIST Mobility Performance of Robotic Systems](https://www.nist.gov/programs-projects/mobility-performance-robotic-systems).

This contract complements, rather than substitutes for, those programs. ISO 18646-5:2026 defines locomotion performance testing for legged service robots, but explicitly does not verify or validate safety requirements: [ISO 18646-5:2026](https://committee.iso.org/standard/86850.html?browse=ics). Simulation credibility should be assessed separately under the risk-based lifecycle principles of [NASA-STD-7009B](https://standards.nasa.gov/standard/nasa/nasa-std-7009).

## Paradigm shift: evaluate a capability envelope, not a leaderboard score

Every evaluation records four separate questions:

1. **Utility:** Did the policy complete useful work, at what energy/cost, and with how much intervention?
2. **Disposition quality:** Did it run, retreat, reject, ask for a person, or remain unresolved when that was the declared warranted action?
3. **Robustness:** Did performance hold on independently specified terrain families, fault combinations, sensor degradation, load shifts, and held-out data sources?
4. **Evidence quality:** Was the trial simulated, hardware-in-the-loop, field-measured, or independently reviewed—and can its exact inputs, source versions, and outcomes be reproduced?

These dimensions must not be collapsed into one weighted scalar. A faster robot cannot trade away a hard safety failure; a cautious robot cannot claim high task capability simply because it abstains safely.

## Benchmark design contract

### 1. Four required policy classes

Evaluate each candidate against the *same* scenario instances, seeds, and evidence class using:

- **Conservative baseline:** a transparent, safety-oriented planner/controller.
- **Reactive baseline:** a simple reactive policy.
- **Metadata-only baseline:** receives scenario metadata but not the primary sensory inputs. This diagnoses shortcut solvability or label leakage.
- **Candidate:** the approach under evaluation.

Baseline implementation, version, resource budget, and available inputs must be recorded. These baselines diagnose different weaknesses; no single one is a universal industry baseline.

### 2. Freeze splits and training provenance

Each scenario manifest entry has one split: train, validation, in-distribution test, out-of-distribution test, or adversarial test. All runs for one scenario retain the same split. Test scenarios bind to a source identifier and are marked held out; a policy's declared training sources must not include those source identifiers. Development tuning may use train/validation only. The digest of the scenario manifest is the subject of the audit receipt.

Use at least three deliberately distinct test strata:

- **In-distribution:** unseen seeds/configurations within the declared operating envelope.
- **OOD:** held-out families or combinations such as wet surface + payload shift, terrain class changes, localization drift, or energy-model mismatch.
- **Adversarial:** fault injection, stale calibration/map, dropped sensors/communications, unknown voids, conflicting evidence, and unsafe actions that must be rejected.

Split by scenario family and source—not by randomly distributing nearly identical frames or seeds across train and test. Retain every failed trial and every negative control.

### 3. Pair everything that can reasonably be paired

For each scenario × seed × evidence-class group, the four policy classes must have one result each. Each trial binds scenario ID, family, seed, policy ID, evidence class, expected/observed disposition, task success, safe degradation, safety violations, predicted success probability, energy, useful work, and interventions.

The auditor checks pairing and provenance before computing comparisons. Missing trials are not silently dropped from the coverage report. Repeating the same scenario under many seeds improves precision for that scenario, but does not create the same amount of independent evidence as adding new scenario families.

### 4. Report metrics independently

Per policy × split × evidence class, report:

- task success and safe-degradation rates separately;
- correct-disposition rate and false-abstention rate;
- safety-violation count;
- Brier score for predicted task-success probabilities;
- useful work and measured energy/energy-per-useful-work only when both are known;
- interventions and relevant latency/downtime/cost measures when captured;
- trial count, scenario-family count, missing values, and all exclusions.

For candidate vs conservative baseline, report a paired task-success difference with a 95% family-cluster bootstrap interval. Resample scenario families, not individual repeated seeds, so repeated runs of one case do not masquerade as independent families. An interval overlapping zero is inconclusive; it is not evidence that the candidate is better. Preregister hypotheses and the bootstrap method before looking at candidate results.

The Brier score is mean `(predicted_success_probability - observed_success)^2`; lower is better. Report calibration separately for each split/evidence class. A policy that achieves the correct action for the wrong reason or with unjustified confidence still needs further investigation.

### 5. Hard gates are not leaderboard objectives

- Any observed safety violation fails the current qualification run; report the case and do not hide it in an aggregate.
- A run action for a scenario whose expected disposition is not run is a safety violation.
- Task success on a scenario that requires abort, rejection, human choice, or unresolved evidence is invalid.
- Missing energy or other essential evidence remains missing/unknown.
- Test-split tuning, held-out source leakage, missing paired baselines, duplicate trial IDs, or silently discarded failures fail the audit.
- Evidence classes are reported separately. A simulator pass never becomes a hardware or field result through hashing, a signature, or a report-format change.

A zero observed-violation sample is not a guarantee of safety. As a planning illustration only, zero events across 300 independent Bernoulli trials gives a one-sided 95% upper bound near 1%; dependence between episodes, shared scenario families, simulator bias, and limited fault coverage invalidate a naive interpretation. Critical strata need independent families and stronger validation evidence. This is a statistical screening target, not a certification criterion.

## Proposed benchmark targets (prospective, not achieved)

| Dimension | Initial target | Guardrail |
|---|---|---|
| Correct action under declared contracts | 100% on deterministic positive/negative controls | Any unsafe dispatch or control-contract violation fails |
| Pairing and provenance | 100% complete matched candidate/baseline cells; 0 test-source leakage | No success-only filtering |
| Held-out performance | Candidate improves macro task success by ≥5 percentage points over the best eligible baseline, with a positive family-bootstrap 95% lower bound | Report in-distribution, OOD, adversarial and evidence classes separately |
| Robust service | ≥90% correct completion-or-required-safe-degradation over the preregistered stress matrix | Completion and safe degradation remain separate rows |
| False abstention | <2% on held-out in-domain scenarios where the declared expected disposition is run | Does not relax hard safety gates |
| Calibration | Brier score improves ≥10% relative to the declared uncalibrated baseline, without worsening OOD calibration | Frozen calibration split; held-out evaluation only |
| Return-energy contract | 100% of under-reserve cases reject dispatch or take the declared safe recovery; unknown estimates stay unresolved | Scenario values are not hardware limits |
| Generalization | Evaluate at least 10 independently authored scenario families across 3+ test strata before making a cross-family claim | Family count, per-family outcomes and uncertainty are mandatory |
| Measurement traceability | Exact source, manifest/corpus digests, seed, config and evidence class on every trial | Hashes establish lineage, not physics validity |

These are aspirational acceptance gates for a research benchmark. They are not promises that a current policy meets them, nor direct comparable scores against proprietary products.

## Comparison with industry and state of the art

Do not claim superiority to a vendor from a score on Symtropy's synthetic fixtures. Start with reproducible public mobility tests (including ISO locomotion/navigation methods where in scope), published terrain datasets/benchmarks, and a declared transparent baseline. For manipulation or embodied reasoning, use the exact official protocol of the chosen public benchmark, rather than translating a synthetic mobility score into a RoboDojo-RC score. Publish deviations, compute/latency budget, failed runs, confidence intervals and license/data-rights constraints.

The current design differentiator should be **benchmark validity plus evidence lineage**: every claimed gain faces shortcut controls, source-leakage controls, paired evaluation, family-level OOD splits, uncertainty calibration, and independently checked dispositions. Whether this eventually beats state of the art is an empirical question.

## Repository ownership

- **Symtropy:** scenario contract, simulator interfaces, paired trial auditor, benchmark reports and model-credibility strata.
- **Symthaea:** candidate policies, decision confidence, active sensing, uncertainty-aware abort/retreat and recovery strategies.
- **Mycelix:** provenance and authorization for raw observations, calibration/configuration records, field evidence and repair history.
- **Sol-Atlas:** versioned map/terrain/hazard/access evidence and regional context.

Do not duplicate a source of truth: pass typed references and digests across these boundaries. Keep the evaluation auditor standalone so it can be run without waiting for the main workspace, GPU CI, or the full simulator.

## Implementation order

1. Freeze this evaluation contract and schema.
2. Run the auditor against contract-smoke tests, including deliberate leakage, pairing, confidence, and safety violations.
3. Export paired trial records from a transparent baseline and one candidate.
4. Add held-out scenario families and perturbations only after the split and hypotheses are frozen.
5. Validate simulator models independently and compare selected cases against measurements before making real-world claims.
6. Publish an evidence-class-separated report with exact-head receipt and every negative control.

**Claim ceiling:** this protocol and its smoke tests define how future robotics results should be audited. They do not establish policy performance, robot safety, simulator validity, regulatory compliance, or SOTA improvement.
