# Navigation Integrity and Active-Sensing Benchmark Charter v0.1

**Owner:** [NAV-BENCHMARK-CHARTER-001 #6693](https://github.com/Luminous-Dynamics/symtropy/issues/6693)  
**Snapshot:** 22 November 2026  
**Status:** engineering and research charter; not a safety approval.

## 1. Executive direction

The goal is not to produce another position estimator or win a single average-RMSE leaderboard. The stronger opportunity is to treat navigation as an **evidence-qualified service** whose outputs remain bounded by source provenance, time/frame qualification, calibration, dependence, estimator uncertainty, integrity state, acquisition policy and independent truth.

The differentiator to test is the composition of:
1. conservative multi-sensor estimation and dependence-aware fusion;
2. observation selection by downstream decision value rather than information gain alone;
3. explicit model uncertainty and justified abstention;
4. integrity-aware transitions and qualified recovery;
5. leakage-resistant cross-domain evaluation;
6. controller/evaluator separation and reproducible evidence receipts.

A service must know when it cannot support a declared use, communicate degradation promptly, avoid false-safe claims, and recover only after evidence is requalified.

**No synthetic result here establishes SOTA, a physical protection level, airworthiness, maritime qualification or physical safety.**

## 2. Architecture and ownership

    Declared use profile / service envelope
                    |
                    v
    Source observation + provenance + calibration/configuration
                    |
                    v
    Clock/time basis + CRS/frame/body + transform lineage
                    |
                    v
    Source dependency / common-cause / correlation assessment
                    |
                    v
    Estimator and conservative fusion
                    |
                    v
    Uncertainty + integrity + observability + service disposition
                    |
                    v
    Active sensing / abstain / degraded operation / recovery
                    |
                    v
    Controller action and acquisition receipt
                    |
                    v
    Independent evaluator + truth provenance + reproducible report

### Mycelix Position

Mycelix owns domain-neutral navigation claim contracts, source/time/frame semantics, fusion, covariance handling, provenance, common-cause references and use-profile-specific health assessments. The v2 claim contract is not a sensor estimator or safety certificate.

### SOMA / Symthaea

SOMA owns mobile adapter and provider integration. Symthaea owns epistemic assessment and mobile service logic. Source observation time remains distinct from local receipt time. The adapter cannot mint calibration, physical independence, trust, health or authorization merely because it receives a measurement.

Production feature wiring now pins the Mycelix Position Git facade in [Symthaea PR #13052](https://github.com/Luminous-Dynamics/symthaea/pull/13052). Its isolated compatibility harness is not a complete Symthaea workspace build or Android/iOS provider qualification.

### Symtropy

Symtropy owns benchmark models, active-sensing research, transition replay, service consequence evaluation, risk–coverage, dataset split validation and independent evaluator design.

### Non-negotiable boundaries

- Availability is not integrity.
- A modality is not a failure-domain identity.
- Source count is not proof of independence.
- Covariance is not automatically a protection level.
- A new fix does not automatically restore NORMAL.
- A model prediction is not evaluator truth.
- A hash proves byte identity, not physical truth or independence.
- Unknown evidence stays unknown; it is never silently upgraded.

Recommended service states are NORMAL, DEGRADED, INTEGRITY_UNCERTAIN, UNAVAILABLE, RECOVERING and INDETERMINATE. Meanings and transition rules must be profile-specific.

## 3. Paradigm shifts

### Decision-qualified navigation

Measure whether a service can support a declared task, not only coordinate error. Outputs should carry the estimate, uncertainty, frame/body and transform lineage, source/time/calibration/provenance references, dependency assessment, integrity disposition, validity interval and requalification requirements.

### Decision value instead of entropy alone

[PR #6647](https://github.com/Luminous-Dynamics/symtropy/pull/6647) implements a finite one-step Bayesian decision oracle with asymmetric false-safe/false-degraded loss. It also models state-dependent acquisition success. Missingness is informative only when a separately justified acquisition model says it is; absence alone is not evidence.

### Contingent rather than myopic sensing

[PR #6690](https://github.com/Luminous-Dynamics/symtropy/pull/6690) implements a horizon-two oracle. Its synthetic test selects a balanced first observation and follows with a second measurement only on the branch where the posterior approaches the decision boundary. Expected terminal Bayes risk is 0.0486 versus 0.0728 for greedy-first under the same budget. This is a constructed binary-state result, not a physical sensing result.

### Model-robust value

[PR #6665](https://github.com/Luminous-Dynamics/symtropy/pull/6665) evaluates actions across a frozen finite scenario family, reporting mean, worst case, downside variation, risk premium and ignorance penalty separately. It can abstain despite positive nominal value.

### Genuine domain generalization

[PR #6686](https://github.com/Luminous-Dynamics/symtropy/pull/6686) adds grouped and leave-one-domain-out split validation so adjacent windows or repeated events cannot leak across partitions.

### Dependence-aware risk

[PR #6688](https://github.com/Luminous-Dynamics/symtropy/pull/6688) reports risk–coverage and false-safe/false-degraded exposure. [PR #6692](https://github.com/Luminous-Dynamics/symtropy/pull/6692) adds deterministic cluster bootstrap so repeated windows from one event are not treated as independent trials.

### Independent evaluation

[PR #6684](https://github.com/Luminous-Dynamics/symtropy/pull/6684) separates controller proposals and post-acquisition decisions from evaluator truth. Truth-swapping changes evaluator score without changing pre-acquisition proposal bytes. The current prototype is in-process; production needs process/capability isolation and independent receipts.

## 4. Direct industry and SOTA comparators

The following are published or vendor-reported figures, not Luminous reproductions. Different datasets, geometries and assumptions make numbers non-interchangeable.

### Indoor benchmark framework

ISO/IEC 21134:2026 (published January 2026) defines the framework for integrated indoor localization/tracking using dead reckoning: master sets, test environments, metrics, process and conformance. Its metric classes include position, trajectory/position series, orientation, relative relationships, integrated localization, practicability and data used.

Adopt it as the indoor/dead-reckoning conformance base, then add source provenance, common-cause dependency, integrity risk, false-safe exposure, abstention, transition continuity and qualified recovery.

- https://www.iso.org/standard/86651.html
- https://www.iso.org/obp/ui?_escaped_fragment_=iso%3Astd%3Aiso-iec%3A21134%3Aed-1%3Av1%3Aen

### UWB industry and estimator baselines

**Qorvo DWM3001C vendor reference.** Qorvo publishes less than 15 cm (2D) and less than 30 cm (3D) location-accuracy specifications, supports TWR and TDoA, and states hundreds of TWR tags / thousands of TDoA tags. These are vendor specifications, not a third-party p95/p99 field result or an end-to-end guarantee.

https://www.qorvo.com/products/p/DWM3001C

**HCCKF UWB/SINS robust estimator.** IEEE Sensors Journal early-access (6 November 2026), DOI 10.1109/JSEN.2026.3749285, proposes hybrid-correntropy CKF with adaptive norm mixing, bandwidth adaptation, Rao-test outlier handling and measurement-noise adaptation. It reports 0.051–0.165 m RMSE on four public fixed-anchor UAV TDoA trials, versus 0.060–0.173 m for its maximum-correntropy CKF comparator. Its dynamic-reference extension is evaluated separately using simulation and measured-residual-driven semi-synthetic experiments.

https://doi.org/10.1109/JSEN.2026.3749285

**Harsh indoor multiplicative-factor UWB.** Measurement 290 (15 November 2026), DOI 10.1016/j.measurement.2026.122957, reports errors within 10 cm for most tags in its tested underground-parking environment at around 30% NLOS. This is a specific hardware/geometry result, not a universal UWB guarantee.

https://doi.org/10.1016/j.measurement.2026.122957

### Active sensing

Li and Parlikad, *Value-Based Active Sensing for Improving Sampling Efficiency*, IEEE Transactions on Systems, Man, and Cybernetics: Systems, early access 30 October 2026, DOI 10.1109/TSMC.2026.3745855, is a direct methodology comparator: observe-then-plan VoI, opportunity loss relative to sensing cost, risk premium over belief-state variability and ignorance penalty for decaying predictive-model confidence. It reports temperature-alert and CO₂-aware office cases under a fixed one-step downstream decision setting.

https://doi.org/10.1109/TSMC.2026.3745855  
Open accepted manuscript: https://doi.org/10.17863/CAM.135881

Do not claim novelty for decision-theoretic active sensing, risk premiums or ignorance penalties. Test navigation-specific composition: time/frame transitions, dependence, acquisition failure, asymmetric integrity loss, contingent planning, abstention and independent truth.

### GNSS cross-domain generalization

An ION GNSS+ 2026 ST-GNN study reports 96.4% detection and 2.26% false alarm in mixed-split testing, falling to 58.6% detection when dynamic matched-power spoofing scenarios are held out. An IEEE Access 2026 paper reports strong in-domain classification but cross-dataset accuracy at or below 66% for its TEXBAT/OAKBAT experiments.

These figures use different protocols and are not directly comparable. Their shared lesson is that random/mixed splits can overstate robustness to unseen attack and environment classes.

- https://www.ion.org/publications/abstract.cfm?articleID=20700
- https://doi.org/10.1109/ACCESS.2026.3693081

## 5. Matched benchmark matrix

All contenders must receive the same observations, sensor privileges, initial beliefs, candidate actions, resource budgets, calibration data, truth manifest and compute budget.

### Estimator layer

| Comparator | Purpose | Minimum reporting |
|---|---|---|
| LS / WLS / NLS | geometric baseline | error, convergence, failure |
| EKF / UKF / CKF | conventional sequential fusion | error, consistency, latency |
| Robust M-estimator | heavy-tail/outlier baseline | tail error, rejection rate |
| Correntropy / HCCKF | NLOS/impulsive-noise comparator | matched RMSE and tails |
| Multiplicative-factor / DC UWB | NLOS cooperative localization | same geometry and NLOS split |
| Covariance Intersection | unknown correlation between estimates | conservative fusion and covariance validity |
| Proposed dependence-aware fusion | common-mode/correlation model | risk and calibration, not only RMS |

### Service/policy layer

| Comparator | Purpose |
|---|---|
| Periodic sensing | operational baseline |
| Fixed threshold / event-triggered | conventional adaptive baseline |
| Random feasible action | sanity baseline |
| Greedy mutual information | entropy-reduction baseline |
| Greedy FIM D/A/E | geometry/information baseline |
| Nominal one-step VoI | downstream utility baseline |
| Reproduced Li–Parlikad method | direct active-sensing comparator |
| Greedy-first horizon two | myopic planning comparator |
| Robust scenario VoI | model-risk-aware policy |
| Exact finite oracle | privileged small-instance comparator |

A privileged exact oracle is a finite synthetic upper-bound reference, not an implementable operating policy.

## 6. Evaluation dimensions

Do not collapse safety-critical dimensions into one weighted scalar before their hard limits are met.

**Position and motion:** median, p95, p99 and maximum position error; velocity/orientation error; trajectory validity; relative pose/distance; error accumulation per time, distance and angular change; performance by geometry and environment.

**Integrity and service:** false-safe count and exposure; false-degraded count and duration; detection at declared false-alert budget; time-to-detect and time-to-integrity-alert; availability, continuity, outage, missed-event probability, abstention coverage/correctness, recovery latency and misleading outputs during RECOVERING/INTEGRITY_UNCERTAIN.

Protection-level and alert-limit metrics are valid only where a use profile defines them and the statistical assumptions are met.

**Active sensing:** downstream decision value, energy and latency per acquisition, value per joule, acquisition yield/dropout, observation count, duty cycle, hard budget violations (must be zero), compute latency p50/p95/p99 and memory.

**Generalization/statistics:** grouped in-domain, leave-one-site/environment/device/firmware/dataset/event-class/failure-domain-out; per-domain and worst-domain performance; cluster-level intervals using a predeclared independence unit.

A one-way cluster bootstrap cannot solve every site/device/event/clock dependence. Use hierarchical or multiway inference when required.

## 7. Aspirational gates — targets, not achievements

1. Reproduce relevant published baselines on released datasets before claiming to beat them. Record artifact, rights and paper version.
2. On a matched indoor configuration, target at least the Qorvo published less-than-15-cm 2D / less-than-30-cm 3D envelope. Report p95/p99; do not assume the vendor statistic is a percentile.
3. Reproduce the published less-than-10-cm-for-most-tags result at around 30% NLOS on matched hardware/geometry before claiming improvement. Then test heavier NLOS, anchor loss, multipath bursts, clock drift and common-mode bias.
4. Require leave-one-scenario-class-out and leave-one-dataset/site-out for GNSS. A stretch target is at least 90% detection at a preregistered false-alert budget on held-out categories, with confidence intervals and separate availability/abstention. This is motivated by the 58.6% held-out ST-GNN result, not a current capability claim.
5. Set false-safe risk limits per use profile; do not invent one universal limit across asset tracking, indoor robotics, aviation and maritime.
6. Demonstrate lower energy/observation count without worsening false-safe exposure, missed-event probability, time-to-alert, continuity or requalification latency versus periodic and reproduced VoI baselines.
7. A restored sensor/new fix must not automatically restore NORMAL. Meet profile-specific requalification and measure false restoration/time-to-recover.
8. Bind every result to exact bytes, source/time/frame/provenance references, oracle assessment, immutable split manifest and independently reconstructable receipts.

## 8. Dataset, oracle and rights protocol

For each dataset, record immutable version/records; source/sensor/device/firmware/site/clock/anchor identities; source/oracle dependencies; time basis, uncertainty and event-onset adjudication; calibration/configuration and transform lineage; event class/severity/ambiguity; ground-truth method/adjudicator; payload digests/run receipt; licence, permitted use, redistribution, privacy and retention.

A recorded signal event does not automatically prove a navigation output invalid. A residual does not automatically prove a physical event. Controller inference cannot become evaluator truth. Digest integrity is not oracle correctness.

## 9. Exact-head evidence snapshot — 22 November 2026

| Workstream | Verified software evidence | Claim ceiling / open work |
|---|---|---|
| Mycelix Position v2 + Git facade [#6412](https://github.com/Luminous-Dynamics/mycelix/pull/6412) | 96/96 canonical and mirrored tests; facade format/Clippy/tests passed | contract only; no physical qualification |
| SOMA Position v2 adapter [#13052](https://github.com/Luminous-Dynamics/symthaea/pull/13052) | 6/6 adapter tests; feature-off check and feature-on Clippy/tests against production-pinned facade | full workspace build and mobile provider integration remain open |
| Correlation/sensitivity sensing [#6512](https://github.com/Luminous-Dynamics/symtropy/pull/6512) | 64 isolated tests at verified head | declared finite scenarios; not a protection level |
| One-step decision value [#6647](https://github.com/Luminous-Dynamics/symtropy/pull/6647) | 78 tests | binary, one-step synthetic model |
| Robust scenario VoI [#6665](https://github.com/Luminous-Dynamics/symtropy/pull/6665) | 85 tests | finite model family; confidence supplied externally |
| Controller/evaluator boundary [#6684](https://github.com/Luminous-Dynamics/symtropy/pull/6684) | 96 tests; truth-swap demo passed | in-process boundary, not OS isolation |
| Cross-domain split validator [#6686](https://github.com/Luminous-Dynamics/symtropy/pull/6686) | 29 validator + 12 other crate tests; Draft 2020-12 schema and split CLI passed | declared metadata only |
| Risk–coverage [#6688](https://github.com/Luminous-Dynamics/symtropy/pull/6688) | 93 tests | Wilson bound assumes independent records |
| Horizon-two sensing [#6690](https://github.com/Luminous-Dynamics/symtropy/pull/6690) | 101 tests; exact policy risk 0.0486 vs greedy 0.0728 in synthetic counterexample | binary static latent state, conditional independence, horizon two |
| Cluster risk bootstrap [#6692](https://github.com/Luminous-Dynamics/symtropy/pull/6692) | 100 tests; 2,000 deterministic demo replicates | one declared cluster axis; not multiway/hierarchical |

These are isolated software results. Workflows qualify only the exact listed heads; skipped/unrelated jobs are not passes. The transition corpus remains 14 synthetic cases. No independent field data or physical validation is claimed by this charter.

## 10. Promotion gates

1. source-authored tests;
2. exact-head isolated CI;
3. reproducible finite synthetic benchmark;
4. reproduced external public dataset;
5. matched on-site test with independently acquired ground truth;
6. independent replication across devices/sites/failure domains;
7. application-specific safety and regulatory review.

Never skip levels by changing terminology. A research benchmark pass cannot imply certification.

## 11. Immediate roadmap

**P0 — integration and receipts**
- full Symthaea SOMA package build and Android/iOS provider callbacks;
- GNSS/UWB/BLE/IMU source-time, frame, calibration and failure-domain evidence;
- independent controller/evaluator process separation and receipt binding;
- frozen train/validation/test and dataset/oracle manifests.

**P1 — scientific realism**
- extend horizon-two planner to correlated observations and common-cause scenarios;
- time-varying latent service state and transition/requalification costs;
- model-parameter uncertainty from separate calibration data;
- robust/sequential policy comparison on matched UWB/SINS and cross-domain GNSS datasets.

**P2 — physical comparative validation**
- select one indoor UWB/dead-reckoning target environment;
- reproduce Qorvo/standard/academic baselines under identical geometry;
- add NLOS, clock drift, anchor outage, moving obstruction, frame/body switch and reacquisition;
- measure energy, latency, availability, integrity and risk–coverage alongside position error.

**P3 — independent release**
- public protocol, pinned artifacts, rights-cleared data, independent evaluator, cluster-aware intervals, raw receipts and limitations;
- no SOTA statement unless a matched external baseline is reproduced and a statistically supported improvement appears on held-out domains.
