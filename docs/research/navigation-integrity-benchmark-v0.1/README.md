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


---

## 12. Addendum: dependency, time and dataset qualification — 22 November 2026

This addendum supersedes earlier “current work” snapshots where new PRs below exist. It does not change the charter's claim ceilings.

### 12.1 From one shared fault flag to overlapping source dependency domains

[Common-cause sequential sensing PR #6713](https://github.com/Luminous-Dynamics/symtropy/pull/6713) implements an exact four-state service-state × common-regime model. Its synthetic negative control has two modality-labelled sensors produce a safe reading during an unsafe service state with actual joint probability 0.082800 versus a naive independent approximation of 0.021600 (3.833×).

[Multi-cause dependency graph PR #6723](https://github.com/Luminous-Dynamics/symtropy/pull/6723) generalizes this to one service-state bit plus up to four binary failure-domain bits (maximum 32 joint states). Each source declares its failure-domain membership. The model rejects a source likelihood or acquisition vector that varies with an undeclared failure domain.

Exact-head workflow: 44996185330. Result: 117 tests passed, 0 failed, formatting, Clippy and all configured demos. Synthetic controls demonstrate:
- UWB/camera share a power cause: actual P(both safe | unsafe service) 0.082800 versus product of marginals 0.021600;
- UWB/IMU use distinct power and clock causes: joint probability equals product of marginals 0.022950 under the deliberately factorized cause prior;
- undeclared cause sensitivity fails closed.

These are constructed model checks. The dependency hypergraph must be populated from real wiring, power, clock, compute, firmware, mounting, map, calibration and environmental evidence. The algorithm cannot discover missing dependencies by itself.

### 12.2 From static-state sensing to time-aware evidence

[Temporal VoI PR #6732](https://github.com/Luminous-Dynamics/symtropy/pull/6732) propagates service-state belief through measurement/acquisition latency after sampling. It propagates the no-observation branch as well.

Exact-head workflow: 44996682429. Result: 125 tests passed, 0 failed, formatting, Clippy and configured demos. In the synthetic policy-reversal example:
- static planner: slow 95%-accurate sensor, 4-tick latency, expected terminal Bayes risk 0.050000;
- time-aware planner: fast 80%-accurate sensor, 1-tick latency, expected terminal Bayes risk 0.251000 under a constructed per-tick service degradation/recovery Markov model.

The higher risk for the time-aware policy is not a contradiction: the model shows that even a more accurate observation can be stale when the service state evolves before the decision. These transition rates are synthetic, not fitted vehicle hazard rates.

Next extension: estimate state-transition and evidence-freshness models on separate calibration routes; calibrate uncertainty and perform held-out tests before claims about a physical vehicle or service.

### 12.3 Rights-aware public data candidates

The public dataset registry now contains six candidates and admits none. Exact-head validation in [UrbanNav admission PR #6728](https://github.com/Luminous-Dynamics/symtropy/pull/6728) passed 31 dataset-admission tests and 12 additional research-crate tests, plus Draft 2020-12 schema, split and both use-scope admission CLIs. Both research-noncommercial and commercial CLI calls reported six candidates, zero admitted.

**UrbanNav-HK-Medium-Urban-1** is the recommended first physical-data target:
- upstream repository [IPNL-POLYU/UrbanNavDataset](https://github.com/IPNL-POLYU/UrbanNavDataset);
- pinned upstream commit `e53717a40c94803cd479a140433f6d803b1a0876`;
- upstream repository identifies MIT and the README explicitly says that dataset files distributed via its download links (ROS bags, GNSS RINEX, IMU and ground truth) are included under MIT, including commercial use/redistribution with notices;
- publisher reports 33.7 GB, 785 seconds, 3.64 km; GNSS, LiDAR, stereo camera and IMU; SPAN-CPT + Inertial Explorer reference; RINEX v3.02 and ROS TimeReference;
- important caveats: a known F9P malfunction interval, 1 Hz ground truth versus 400 Hz IMU, and separate sequence-specific time/frame/extrinsic/oracle review.

Rights are explicit but the local payload is not immutable: download object versions, sizes and SHA-256s are not yet frozen. The dataset remains candidate_not_admitted and no files are claimed downloaded or qualified.

**HK-UrbanSpoof** remains a second candidate, [upstream repo](https://github.com/Fangjingxiaotao/HK-UrbanSpoof) pinned at `95def2b6a42eee6299bbc1a5b880a9f3bfc070d6`. It is a strong receiver-level GPS L1 synchronous spoofing benchmark with S1–S3 real recordings and S4 simulated, 17 attacks, S2/S3 trajectory files, and ray-tracing labels described by the paper. Its GitHub API currently reports no licence and its raw IF payload is external Google Drive. Ground-truth file availability and README statements are inconsistent; rights and oracle status remain unknown. It stays candidate_not_admitted.

The first physical comparative benchmark should use UrbanNav's Medium-Urban route with entire loops/sessions as indivisible groups. First exclude or label the known F9P malfunction period, qualify SPAN-CPT/IE truth lineage and time/frame calibration, then reproduce standalone GNSS, GNSS/INS and available LiDAR/INS baselines before evaluating active-sensing or integrity policies.

### 12.4 New exact-head evidence snapshot

| Workstream | Exact-head evidence | Claim ceiling |
|---|---|---|
| Common-cause sequential sensing [#6713](https://github.com/Luminous-Dynamics/symtropy/pull/6713) | 110 tests | static binary shared-regime toy |
| Multi-cause dependency graph [#6723](https://github.com/Luminous-Dynamics/symtropy/pull/6723) | 117 tests | up to four static binary causes, declared dependencies |
| Temporal service-state VoI [#6732](https://github.com/Luminous-Dynamics/symtropy/pull/6732) | 125 tests | binary Markov service state, known synthetic transition rates |
| UrbanNav rights/metadata candidate [#6728](https://github.com/Luminous-Dynamics/symtropy/pull/6728) | 31 validator tests + 12 additional tests; all schema/split/admission CLIs pass | explicit rights but no frozen payload or physical/oracle qualification |
| HK-UrbanSpoof rights candidate [#6716](https://github.com/Luminous-Dynamics/symtropy/pull/6716) | 30 validator tests + 12 additional tests; admission CLIs pass | rights unknown; not admitted |

All results are software or metadata-admission evidence. No dataset payload has been admitted and no physical navigation trial has been executed.

### 12.5 Immediate next research gates

1. Freeze the UrbanNav Medium-Urban payload and compute exact file hashes, without mirroring data unless the license and distribution conditions are retained.
2. Qualify per-sensor timestamps, ROS/NMEA time relation, clock drift, camera/LiDAR/IMU extrinsics, antenna/IMU lever arm and SPAN-CPT/IE processing lineage.
3. Create whole-loop/site/device held-out splits. Keep overlapping windows and the same physical run together.
4. Reproduce standalone GNSS, GNSS/INS and LiDAR/INS baselines on identical segments before comparing new policies.
5. Estimate service-transition rates and confidence calibration on separate training/calibration routes.
6. Evaluate static vs temporal VoI, dependency-aware fusion, sequential acquisition, risk–coverage and cluster-bootstrap confidence on untouched held-out routes.
7. Run an independent evaluator with a separate truth capability; no controller-readable ground truth, shared truth cache or label-derived feature leakage.

No SOTA claim is available until published baselines are reproduced under matched conditions and an independently supported improvement appears on held-out environments.


---

## 13. Addendum: matched 2026 UrbanNav baselines — 22 November 2026

The four-method reproduction protocol is now public in [PR #6734](https://github.com/Luminous-Dynamics/symtropy/pull/6734), tracked by [NAV-URBANNAV-BASELINES-001 #6733](https://github.com/Luminous-Dynamics/symtropy/issues/6733). It is a protocol only; no baseline has been reproduced.

### 13.1 Adaptive protection-level control (direct integrity comparator)

Maharmeh, Resende and Nashashibi, *Integrity as a Control Problem: Smooth and Adaptive Protection Levels for Multi-Modal Localization*, Sensors 26(16), 5140 (14 August 2026), DOI 10.3390/s26165140.

The paper computes an along-track/cross-track error-growth signal from:
- base IMU drift;
- discrepancy between ES-EKF displacement and raw IMU dead-reckoning;
- LiDAR geometric registration residual weighted by a sensitivity term based on the minimum eigenvalue of the information/Hessian matrix.

It integrates a saturated ideal bound, then tracks it with an adaptive PID whose gains are scheduled by normalized Shannon entropy of LiDAR ICP residuals. The PL expands at the full command rate and contracts more slowly. It evaluates medium (3.64 km), low (1.21 km) and deep urban (4.51 km) UrbanNav routes.

The paper reports 0.0% pointwise PL exceedance in moderate routes, a deep-urban nonzero breach, and a sensitivity to maximum PL ceiling. Its primary reported configuration includes Smax=10m and a further 10/12/14m ceiling sweep. The full reproduction protocol captures the numeric PID/drift/entropy parameters and full/non-integral comparison.

**Critical interpretation:** this is a finite-route Stanford-diagram point fraction (true error > PL), not automatically an independently calibrated probability-of-integrity per hour or mission. The deep-urban breach and Smax sensitivity must be retained, not hidden. Our additional event-clustered and false-safe-exposure measures complement—but do not replace—the exact paper metric.

Paper:
- https://www.mdpi.com/1424-8220/26/16/5140
- https://doi.org/10.3390/s26165140

### 13.2 Adaptive temporal Kalman and solution separation

Luo, Sun and Fu, *ATM: Adaptive Temporal Kalman Filter-Based Solution Separation Method for Positioning and Integrity Monitoring*, IEEE Transactions on Instrumentation and Measurement 75 (2026), DOI 10.1109/TIM.2026.3670522.

The accessible abstract describes a first-order adaptive Markov representation for temporally colored measurement noise, measurement-noise modelling, combined state/measurement-vector formulation, weighted-least-squares update and position-domain solution separation for fault detection/exclusion. The available abstract/index reports UrbanNav horizontal error improvements of 4.80% / 4.57%, HPL reductions 13.32% / 18.32%, and VPL reductions 18.51% / 26.74% across its medium/deep urban tests.

**Reproduction status:** full paper/method and exact baseline configuration have not yet been obtained from accessible full-text sources. Percentages are paper-reported abstract values; do not infer an implementation or claim reproduction from them.

- https://doi.org/10.1109/TIM.2026.3670522
- https://www.researchgate.net/publication/401578449_ATM_Adaptive_Temporal_Kalman_Filter-Based_Solution_Separation_Method_for_Positioning_and_Integrity_Monitoring

### 13.3 Resilient GNSS–IMU–LiDAR–camera–loop fusion

Liu et al., *Robust urban SLAM via resilient GNSS–IMU–LiDAR–camera–loop fusion*, ISPRS Journal of Photogrammetry and Remote Sensing 238 (August 2026), pp. 794–815, DOI 10.1016/j.isprsjprs.2026.05.035.

Publisher description includes LVIO, GNSS and loop-closure factors, frame-by-frame covariance propagation on Lie groups, Hessian-eigenvalue degeneracy detection with per-DoF down-weighting, LiDAR-assisted depth-enriched visual features, and 3D object-detection-based dynamic point removal. The paper reports evaluation on UrbanNav, KITTI, GREAT-WHU and two self-collected datasets, with pose performance comparable to its SOTA comparisons and improved map construction.

Potential implementation repository: https://github.com/TK72/FAST-LIVOSAM

**Reproduction and code gate:** the candidate repository README says its current experiments are mainly GREAT-WHU and KITTI, so it does not prove exact paper UrbanNav configuration. It contains an MIT licence heading but the pinned root snapshot has no LICENSE file and GitHub does not recognize a repository licence. Do not reuse code until the licence is clarified and paper-version/config/data splits are matched.

- https://doi.org/10.1016/j.isprsjprs.2026.05.035
- https://github.com/TK72/FAST-LIVOSAM

### 13.4 Receiver-only Doppler-rate spoofing detector

Fang and Xu, *A Doppler frequency metric for GNSS spoofing detection in urban environments*, Aerospace Systems (6 July 2026), DOI 10.1007/s42401-026-00509-7.

The proposed receiver-only detector compares observed Doppler-frequency rate from tracking outputs with predicted Doppler-rate from the receiver navigation solution. It uses an adaptive threshold and Neyman–Pearson decision rule. The paper analyses 16 user-dynamics/spoofer-antenna cases and reports the method effective in 11; its TEXBAT experiment reports under-five-second detection delay and approximately 2% false alarm.

Use this as a low-compute receiver-only baseline. Reproduce both effective and ineffective attack cases. Do not report only the successful 11. Apply to HK-UrbanSpoof only after the payload rights, exact data, truth labels and release ambiguity are resolved.

- https://link.springer.com/article/10.1007/s42401-026-00509-7

### 13.5 Reproduction and promotion gates

All four baselines are recorded in [the machine-readable plan](https://github.com/Luminous-Dynamics/symtropy/blob/research/nav-urbannav-baselines-v0-2026-11-22/docs/research/nav-urbannav-v0.1/baseline_reproduction_plan.v0.1.json).

Before comparative scoring:
1. acquire exact paper/full-method/config/code versions;
2. freeze the same UrbanNav route payload and SHA-256 manifest for every method;
3. freeze coordinate/time transforms, sensor inputs, dropouts, calibration, truth lineage and route split;
4. reproduce each paper's metric separately, then add common service-level and cluster-aware metrics;
5. compare under the same compute/energy budget;
6. publish parameter receipts, raw logs, exclusions, seeds and input digests.

No baseline in this charter is currently reproduced by Symtropy. No SOTA superiority claim is permitted until a matched paper-faithful reproduction and statistically supported held-out-domain improvement exist.


### 13.6 Direct covariance baseline used by the PID paper

The adaptive-PID article's own Section 5.1 compares it against Al Hage et al. (2020), a heavy-tailed multivariate Student-t protection-level method (IEEE T-ITS 23, 2978–2990, DOI 10.1109/TITS.2020.2983681). The 2026 paper reports degrees-of-freedom sweeps {4,5,6,7,8,9,10,100} and derives overall/directional PLs from the 2D positional covariance eigenstructure and a 2D multivariate Student-t TIR multiplier.

The equation-level Rust implementation is in [PR #6737](https://github.com/Luminous-Dynamics/symtropy/pull/6737). It consumes a caller-supplied K_TIR(dof); exact 2D CDF computation / coverage convention and offline dof selection remain separate qualification steps. This is the direct statistical baseline to run beside the PID controller, not a generic Gaussian covariance bound.

- https://doi.org/10.1109/TITS.2020.2983681
- https://www.mdpi.com/1424-8220/26/16/5140 (Section 5.1)


### 13.7 Pointwise protection-level exceedance versus breach events

[PR #6757](https://github.com/Luminous-Dynamics/symtropy/pull/6757) carries the #6745 trace evaluator onto the current #6737 baseline head. It keeps the paper's pointwise Stanford-style exceedance fraction and reports distinct contiguous breach episodes and time exposed above PL separately.

Each record must carry an explicit support duration. The evaluator does not infer time support from row frequency, treats invalid truth as invalid rather than safe, splits events at support gaps/safe intervals/invalid truth/**cluster boundaries**, and reports maximum contiguous breach duration. Cluster boundaries prevent adjacent intervals from two independent route/session units being merged into one apparent physical breach.

The method assumes each sample's exceedance state is constant over its declared support interval. It is not continuous-time interpolation or a calibrated probability-of-integrity metric. Predecessor workflows #153/#154 failed at compile time; the identified field/getter errors were fixed in #6757. Exact-head workflow #161 passed on SHA `5cb348713cfa45788edd616b055d15a9f92c6e7a`: 83 tests across three isolated crates, Clippy, all four demos, generalization schema/split and dataset-admission CLIs. The dataset registry still admits zero payloads.


### 13.8 Equation fidelity and mathematical alternatives

The source audit found that the current #6737 code used the exponential first-order derivative coefficient while describing it as the paper's exact equation. Maharmeh et al. (2026), Eq. (34)-(35), prints the linear coefficient `alpha = min(1, 2*pi*f_c*dt)`, applied to the current finite-difference derivative. [PR #6757](https://github.com/Luminous-Dynamics/symtropy/pull/6757) makes this `PaperLinear` mode the default and keeps `ExactExponential` (`alpha = 1-exp(-2*pi*f_c*dt)`) as a separately named experimental alternative. Baseline result manifests must record the mode.

The Student-t baseline preserves the paper's printed directional equation but adds an independent covariance-projection comparison. The conventional directional variance is `e_i^T P e_i = sum_j lambda_j * v_(j,i)^2`, not generally the paper's printed `max(lambda_j * |v_(j,i)|)`. A constructed 45-degree covariance with eigenvalues 2 and 1 yields projected variance 1.5 versus a printed maximum term of sqrt(2); with the same degrees of freedom and radial multiplier, the printed PL is approximately 2.90% smaller. This is a formula-convention counterexample, not a claim about paper route results. Both methods remain separately reported pending author/source clarification.

The 2D radial multiplier `K_TIR = sqrt(TIR^(-2/nu)-1)` assumes the supplied matrix is covariance (not Student-t scale) and TIR means total 2D radial tail probability. A per-axis/two-sided marginal probability is a different convention. The paper's precise convention must be confirmed before route-result comparison. The article's LiDAR residual prose/formula ambiguity also remains unresolved in the frontend reproduction.

### 13.9 Zero-failure confidence gate

The isolated Rust evidence kernel in [PR #6757](https://github.com/Luminous-Dynamics/symtropy/pull/6757) implements the exact one-sided binomial upper bound for the special case of zero observed failures:

`p_upper = 1 - (1-confidence)^(1/n)`

For a target mission-failure probability of 1e-3 at 95% one-sided confidence, at least **2,995 independent, exchangeable, same-scope mission trials** with zero failures are required. 2,994 do not suffice. This is not 2,995 epochs, not 2,995 samples from a single route, and not a per-hour guarantee. A route loop is not automatically independent merely because it has a new ID; independence needs predeclared failure-domain and mission-scope justification. The helper deliberately rejects nonzero failures, which need a general binomial confidence method.

This bound is a statistical gate for a declared Bernoulli mission unit. It does not by itself prove ground-truth validity, independence, representativeness or protection-level calibration.

### 13.10 Competitive strategy and promotion targets

The competitive object is a **risk/accuracy/availability/continuity/latency/energy Pareto frontier**, not a single average-RMSE number. Every comparison must freeze the same sensor observations, privileges, initialization, calibration set, map, route split, compute/energy budget and independent truth manifest. Report the per-method Pareto points; do not collapse them into one weighted scalar before hard safety limits are satisfied.

| Dimension | Proposed promotion target (aspirational, not achieved) | Required comparator/report |
|---|---|---|
| Nominal position accuracy | p95 error no more than 5% worse than the strongest matched baseline | same sensor set, route, initialization, calibration and compute |
| Degraded-domain accuracy | at least 10% lower p95 error than the best reproduced matched baseline | predeclared GNSS multipath/LOS loss/LiDAR degeneracy/IMU fault slices |
| False-safe exposure | at least 30% lower time above PL than the strongest matched baseline, without more than 1 percentage point loss in service availability | breach episodes, maximum contiguous duration, event rate and uncertainty by independent cluster |
| Integrity claim | upper confidence bound at or below the declared use-profile target | independent mission units; no epoch-level independence substitution |
| Degradation response | p95 time-to-alert within the downstream hazard-analysis actuation budget | fault injection with onset adjudication and measured sensor/compute latency |
| Recovery | no false restoration to NORMAL; recovery latency inside the declared service budget | explicit requalification, reset, and common-cause recovery tests |
| Efficiency | at least 20% lower sensing/compute energy at matched integrity and accuracy, or a measured Pareto improvement | joules per kilometre/mission, sensing duty cycle, CPU/GPU time and p99 latency |
| Generalization | maintain coverage and false-safe limits on untouched site/device/firmware/weather/failure-domain holdouts | per-domain metrics and worst-domain confidence bounds |

These thresholds are research targets to make progress measurable; they are not universal safety limits. Application-specific alert limits, time-to-alert, availability and continuity must come from the declared ODD, hazard analysis and downstream response budget.

The standards comparison is complementary, not substitutive: ISO 26262 addresses hazards from malfunctioning safety-related E/E behaviour; ISO 21448 addresses SOTIF risks from functional insufficiencies and performance limits of complex sensing. Neither standard makes this research benchmark a safety approval or certifies a navigation estimator. Aviation-style HPL/VPL, alert-limit, availability, continuity and time-to-alert discipline are useful design references, but automotive claims require the relevant automotive safety case and applicable requirements.
