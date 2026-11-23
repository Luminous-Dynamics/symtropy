# Navigation integrity: 2026 SOTA landscape and matched-benchmark plan

**Date:** 2026-11-22  
**Status:** literature-grounded benchmark plan; no new physical scores claimed.

This review expands the navigation-integrity benchmark beyond NLOS classification and trajectory RMSE. It considers indoor localization standards, raw-CIR learning, dynamic uncertainty calibration, decoupled geometric correction, multi-robot UWB arrays, construction-site SLAM and secure ranging.

## 1. The field has several different leaders

These papers solve different tasks and use different hardware, datasets, labels and test protocols. Their reported numbers must not be ranked as if they came from one benchmark.

| Work | Published result / contribution | What we must compare |
|---|---|---|
| [ISO/IEC 21134:2026](https://www.iso.org/standard/86651.html) | International framework for integrated indoor localization/tracking with dead reckoning; master sets, environments, metrics, benchmarking process and conformance | Align accuracy, trajectory and practical-performance reporting before adding integrity claims |
| [Hilti–Trimble SLAM Challenge 2026](https://hilti-challenge.com/) / [SPARO result](https://sparolab.github.io/research/hilti_challenge/) | Official report: 0.109 m mean RMSE, 99.85% mean trajectory coverage, 25 sequences | Reproduce official trajectory metrics; add evidence-backed support, PL breaches, unsupported NORMAL time and recovery |
| [Zhao et al., IEEE T-RO 2025, “Robust and Scalable Multi-Robot Localization Using Stereo UWB Arrays”](https://doi.org/10.1109/TRO.2025.3587854) | Centimetre-level multi-robot localization; 100 Hz UWB-only update rate reported; distributed relative localization and inter-/intra-node cooperation | Match update rate, node count, packet loss, 3D pairwise geometry, accuracy, network airtime and failure-domain independence |
| [Yang et al., Measurement 2025, “A novel credibility evaluation and mitigation for ranging measurement in UWB localization”](https://doi.org/10.1016/j.measurement.2025.117721) | BERT-LSTM/CIR method; reported 80.94% NLOS and 37.9% LOS range-error reductions on its study-specific experiments | Reproduce credibility, correction and discard decisions; report LOS harm, selective risk and interval coverage |
| [Jiang et al., IEEE Communications Letters 2026, “Optimizing UWB CIR Truncation for NLOS Identification on MCU Deployment”](https://doi.org/10.1109/LCOMM.2026.3662481) | 63-sample CIR subset; 93.68% NLOS recall; quantized STM32 deployment | Reproduce on matched CIR representation/hardware; add false accepts, domain shift, energy and radio-rate cost |
| [Sun and Gao, Measurement Science and Technology 2026, “Probabilistic calibration and dynamic uncertainty evaluation for UWB ranging in NLOS environments”](https://doi.org/10.1088/1361-6501/ae7c20) | Physics-informed multi-task NLOS classification, correction and aleatoric/epistemic uncertainty; MAE below 0.12 m reported across its evaluated datasets | Reproduce on held-out experiment/material/anchor groups; compare both error and empirical uncertainty calibration |
| [Lu et al., Measurement 2026, “A decoupled backend optimizer for UWB localization in NLOS environments”](https://doi.org/10.1016/j.measurement.2026.121494) | Reusable posterior refinement layer that combines front-end estimates, ranges and anchor geometry, jointly estimating position and residual range errors | Integrate as a baseline behind multiple front ends; add bounded residuals, bias observability, failure cases and evidence provenance |
| [IUCA, IEEE Journal of Selected Areas in Sensors 2026](https://doi.org/10.1109/JSAS.2026.3731931) | Model-agnostic high-dimensional conformal calibration of joint/correlated state uncertainty, including pose ellipses | Reproduce exchangeability assumptions; measure joint coverage, sharpness, conditional/domain-shift failure and mission-level risk |
| 2026 CIR NLOS survey, Computers & Electrical Engineering 139B | Reviews 80 representative studies; distinguishes handcrafted CIR-feature and raw-CIR sequence methods; highlights label efficiency, domain generalization and physics-guided interpretability | Use as taxonomy and ablation guide; evaluate cross-scenario robustness and deployment feasibility, not just accuracy |
| [Bregar et al., Scientific Data 2023, UWB Positioning and Tracking Data Set v2.0](https://doi.org/10.5281/zenodo.8280736) | Four indoor environments, eight fixed DW1000 anchors + one mobile tag; raw per-channel CIR / first-path / RSS and LOS/NLOS metadata; CC BY 4.0 v2.0 record, 4.4 GB archive | Use as a smaller raw-CIR and indoor ranging-calibration candidate; verify payload hashes, timing, calibration and label lineage. Not a continuous UAV or motion-capture trajectory benchmark |
| [IEEE TII 2026 secure indoor ranging review](https://doi.org/10.1109/TII.2026.3680098) / [FiRa Core 4.1](https://www.firaconsortium.org/news/press-releases/2026/10/fira-consortium-launches-core-4-1-specifications-and-enhanced) | Overshadowing, early-path injection, relay and clock-related threats; FiRa Core 4.1 expands DS-TWR security certification, responder-specific sub-session keys and 256-bit STS key testing | Separate protocol authentication, physical channel integrity, clock integrity and navigation decision integrity |

## 2. Competitive paradigm: a verified front end and an independent posterior

The 2026 decoupled backend optimizer is an important comparator because it explicitly avoids being tied to one front-end algorithm. We should adopt that modularity—but make the posterior evidence-qualified.

Proposed architecture:

1. **Raw observation qualification:** preserve radio event identity, local clock, units, CIR/RSS provenance, calibration revision, STS/session evidence and acquisition time.
2. **Measurement hypothesis:** output a point estimate plus a hard range set and/or calibrated probabilistic uncertainty. Unknown-sign error must not be forced into a positive-NLOS-bias model.
3. **Front-end estimator:** EKF, factor graph, VIO/UWB, LS, or multi-robot cooperative estimator; preserve exact input observation IDs and dependency labels.
4. **Independent posterior audit:** examine geometric residuals, anchor/tag displacement, common-cause consistency and whether all predicted truth regions are actually supported.
5. **Protection-region qualification:** derive pose protection levels from the complete uncertainty/dependency model, not only the estimator covariance.
6. **Decision gate:** accept, acquire new evidence, degrade or abstain. Only the separately qualified contract may authorize NORMAL.

This lets us compare against the 2026 backend optimizer fairly while extending the question from “did residual correction improve RMSE?” to “did it improve accuracy without making the posterior falsely confident?”

## 3. Benchmark protocol: separate four leaderboards and one cross-cutting security campaign

### A. Estimation
- Median, RMSE, p95 and p99 position/orientation error.
- Relative drift, time-to-initialization, trajectory coverage and continuity.
- Official Hilti metrics and published MILUV baseline metrics reproduced exactly.
- Multi-robot update rate, relative-pose error and scaling by node count.

### B. Measurement integrity
- Range interval / set empirical coverage at 90%, 95%, 99%.
- p50/p95 interval width and a proper sharpness/score measure.
- NLOS/LOS detection false acceptance, precision/recall, selective risk and retained-measurement coverage.
- Bias-bound coverage and underestimation magnitude.
- Calibration stratified by obstacle material, anchor geometry, device, robot, distance and environment.
- Unknown-sign errors kept separate from positive excess-path bias.

### C. Service integrity
- Protection-level breaches and HMI per declared mission.
- One-sided exact binomial confidence bounds using independent mission/experiment units.
- Time-to-alert, unsupported-NORMAL exposure, degraded availability, abstention and recovery.
- Common-cause campaigns for shared obstacles, clock sources, calibration, power, anchors and passive receivers.
- Re-entry to NORMAL only after fresh, independently supported evidence.

### D. Efficiency and scaling
- p50/p95/p99 inference and update latency.
- Radio airtime, packet loss, throughput and synchronization overhead.
- CPU/MCU cycles, RAM, flash, energy per measurement and energy per metre.
- Performance at 1, 3, 6 and 12 nodes, subject to the available hardware/data.
- MCU and workstation results reported separately.

### E. Cross-cutting adversarial integrity
- Replay/stale session, identity and key-epoch binding failures.
- STS/session evidence loss, protocol downgrade and verifier unavailability.
- Clock reset/manipulation, timing substitution and common-mode drift.
- Controlled channel interference, corrupted CIR and source-provenance failures.
- Attack false acceptance, time-to-revoke authority, HMI and recovery.
- Compare only under explicit attacker capability and protocol profile.

Do not combine the five result families into a single weighted score.

## 4. Reproduction ladder

Run the following baselines under identical input availability and experiment-level splits:

1. Uncorrected DS-TWR + least squares.
2. Published MILUV UWB-IMU EKF.
3. Robust/adaptive EKF and factor-graph baselines.
4. BERT-LSTM / CIR credibility and correction.
5. 63-sample MCU CIR model.
6. Physics-informed evidential NLOS range calibration.
7. Decoupled geometric posterior optimizer.
8. Multi-robot cooperative UWB array / stereo-UWB baseline.
9. VIO/SLAM baseline where the sensor set and reference frame permit a matched comparison.
10. Our evidence-qualified measurement + posterior + mission-decision pipeline.

For every method, report feature availability, filtering, calibration split, train/test grouping, hardware and exact source revision.

## 5. Proposed stretch targets (not achieved results)

- 99% nominal range/pose set coverage on untouched experiment/material/anchor groups, with p50/p95 width reported.
- NLOS recall ≥95% while false-positive rate ≤5% on leave-one-material and leave-one-anchor splits; separately cap unsafe false acceptance.
- Multi-robot ≥100 Hz UWB-only or equivalent matched update rate where the device/network setup permits, without sacrificing qualification or hiding packet loss.
- HMI probability ≤10^-3 for a declared mission scope, supported by one-sided 95% confidence bounds and enough independent missions; do not use adjacent epochs as independent trials.
- One measurement-cycle fault detection for defined clock/session/calibration fault injections, followed by an evidence-qualified recovery process.
- Pareto improvement over the matched 63-sample MCU CIR baseline in false acceptance, NLOS detection, inference cost, energy and measurement-rate loss.
- Reproduce official Hilti accuracy/coverage scores first; then demonstrate higher evidence-backed coverage and lower unsupported-NORMAL exposure at comparable accuracy.

The targets are deliberately split. A model can have excellent NLOS recall but poor interval calibration; a backend optimizer can reduce RMSE but widen or miscalibrate residuals; a multi-robot system can run at high update rate while sharing common failure causes.

## 6. Statistical and data controls

- Split by full experiment/mission and environment; no random row splits.
- Hold out obstacle material, anchor placement, robot and device revision.
- Reserve a final sealed test set untouched by model selection.
- Use separate calibration/test cohorts for conformal or quantile calibration.
- Treat clusters/experiments as units for mission-risk claims.
- Report confidence intervals, missingness, exclusions and all failures.
- Keep controller/evaluator separation at field and row-selection levels; the MILUV truth-conditioned row filter remains blocked by [issue #6813](https://github.com/Luminous-Dynamics/symtropy/issues/6813).
- Do not admit any physical benchmark payload until rights, hashes, schema, calibration lineage and independent truth lineage are checked.

## 8. Active evidence acquisition and authority separation

The architecture now includes an active evidence-acquisition workstream ([issue #6853](https://github.com/Luminous-Dynamics/symtropy/issues/6853), [draft PR #6855](https://github.com/Luminous-Dynamics/symtropy/pull/6855)). Its initial deterministic policy uses a Pareto frontier over conditional protection-level reduction, worst-case latency and energy rather than arbitrary scalar weights. An action plan never authorizes NORMAL; success requires a new qualified observation, posterior re-audit and mission-contract evaluation.

The posterior auditor is implemented in [draft PR #6852](https://github.com/Luminous-Dynamics/symtropy/pull/6852), with exact-head CI run #211 passed (95 navigation tests, 12 MILUV ingest tests). The active-action planner in #6855 passed exact-head run #212 (95 navigation tests, 12 MILUV ingest tests). The secure-session evidence gate in [draft PR #6849](https://github.com/Luminous-Dynamics/symtropy/pull/6849) passed exact-head run #209 with the same configured test totals.

## 9. MILUV source-complete acquisition plan

The publisher links the source archive through [Figshare DOI 10.25452/figshare.plus.28386041.v1](https://doi.org/10.25452/figshare.plus.28386041.v1). The [NLM Dataset Catalog record](https://datasetcatalog.nlm.nih.gov/dataset?q=0002392468) lists CC BY 4.0 rights; the paper describes a 173 GB archive with 217 minutes across 36 experiments. The [authors' devkit](https://github.com/decargroup/miluv) loads original per-robot ROS bags under each experiment.

For an earlier, smaller raw-CIR step, [Bregar's UWB Positioning and Tracking Data Set v2.0](https://doi.org/10.5281/zenodo.8280736) is a 4.4 GB CC BY 4.0 candidate with four indoor environments, eight fixed anchors and one mobile tag. Its CSVs include raw CIR and first-path/RSS features. Use the v2.0 Zenodo record only; the earlier v1.0 record has different CC BY-NC-SA terms. It is suited to indoor range/CIR calibration and LOS/NLOS experiments, not multi-UAV trajectories. The registry entry remains candidate_not_admitted until every file is inventoried and SHA-256 hashed and the time, anchor-survey, calibration and label lineage are checked.

Do not download the entire archive simply to reproduce one range baseline. First inspect publisher metadata and the archive's experiment/file inventory, identify a small experiment whose original robot bags contain UWB raw messages, CIR and passive receptions, verify its licensing/attribution and freeze its exact file hashes. Process the source-complete messages without Vicon-conditioned row filtering; construct evaluator truth separately. The truth-conditioned processed CSV remains blocked by [issue #6813](https://github.com/Luminous-Dynamics/symtropy/issues/6813).

## 10. Current Symtropy work

- [#6793](https://github.com/Luminous-Dynamics/symtropy/pull/6793): interval-valued DS-TWR and positive NLOS bias kernel.
- [#6813](https://github.com/Luminous-Dynamics/symtropy/issues/6813): truth-conditioned MILUV row-selection leakage.
- [#6814](https://github.com/Luminous-Dynamics/symtropy/pull/6814): source audit of MILUV timestamp, clock and calibration semantics.
- [#6815](https://github.com/Luminous-Dynamics/symtropy/pull/6815): processed-CSV and bounded raw transaction adapters; explicit propagation speed.
- [#6816](https://github.com/Luminous-Dynamics/symtropy/issues/6816): source-clean benchmark charter.
- [#6843](https://github.com/Luminous-Dynamics/symtropy/pull/6843): evidence-bounded multi-wrap clock stream reconstruction.
- [#6844](https://github.com/Luminous-Dynamics/symtropy/issues/6844): adversarial UWB ranging integrity qualification.
- [#6849](https://github.com/Luminous-Dynamics/symtropy/pull/6849): external-verifier session evidence gate.
- [#6852](https://github.com/Luminous-Dynamics/symtropy/pull/6852): independent 3D range-set posterior audit.
- [#6853](https://github.com/Luminous-Dynamics/symtropy/issues/6853): risk-aware active evidence acquisition programme.
- [#6855](https://github.com/Luminous-Dynamics/symtropy/pull/6855): Pareto active evidence-acquisition planner (run #212 passed).
- [#6858](https://github.com/Luminous-Dynamics/symtropy/pull/6858): integrated clock, secure-session, posterior and active-acquisition research kernels (combined run #213 passed).
- [#6864](https://github.com/Luminous-Dynamics/symtropy/pull/6864): publisher-aligned Bregar CIR normalization/window and deterministic feature kernel (exact-head run #221 passed: 137 navigation, 12 ingest, 31 validator tests).
- [#6868](https://github.com/Luminous-Dynamics/symtropy/pull/6868): bounded Bregar CSV row parser with controller/evaluator field partition and raw-CIR connection; exact-head CI pending after parser hardening.
- [#6869](https://github.com/Luminous-Dynamics/symtropy/pull/6869): group-wise finite-sample positive NLOS-bias upper-bound calibration with one maximum residual per independent calibration unit; exact-head CI and physical calibration pending.
- [#6869](https://github.com/Luminous-Dynamics/symtropy/pull/6869): grouped finite-sample positive NLOS-bias calibration; exact-head CI pending.
- [#6779](https://github.com/Luminous-Dynamics/symtropy/pull/6779): candidate dataset registry and controller/evaluator boundary; now includes Bregar UWB v2.0 as an unadmitted raw-CIR candidate.

## 11. Claim boundary

This document is a literature review and reproduction plan. Published numbers are study-specific and are not directly comparable. No new physical benchmark result, reproduced SOTA result, cryptographic certification or navigation safety guarantee is claimed.


## 10. Updated research directions — 23 November 2026

### 10.1 Active sensing: observability surrogates are not task guarantees

A November 2026 paper, **Dual-path active configuration control for observability-aware UWB localization**, Internet of Things 40, DOI 10.1016/j.iot.2026.102143, compares centered/non-coincident fixed anchors, mobile heuristic configurations and mobile MPC. Its MPC maximizes a regularized Fisher-information geometry objective and reports p99 computation below the 100 ms sampling interval. The paper's own conclusion reports that stronger information scores did not consistently improve position/orientation errors over fixed configurations. Treat this as an important negative result, not an argument against geometry planning: it shows that a better surrogate observability metric is not enough to claim better navigation decisions.

Symtropy's active-sensing tournament must therefore measure separately:
- geometry rank/conditioning and Fisher scores;
- predicted versus realized observation utility;
- realized position/trajectory error at matched epochs;
- decision-incompatible fault pairs resolved after valid acquisition;
- false-safe exposure, coverage, availability and resource cost;
- a negative control where Fisher information improves but task error/decision does not.

### 10.2 Active perception evaluation: close the full observation loop

Li et al., **Active Visual Perception: Opportunities and Challenges**, Sensors 26(20), 6544 (15 October 2026), DOI 10.3390/s26206544, frames active perception as a loop from missing task evidence, through feasible sensing action and value estimation, to action execution and subsequent state update. It specifically argues that predicted value does not guarantee useful evidence can be physically acquired, and final task success alone cannot attribute improvement to active sensing. Its benchmark guidance includes passive/fixed, random, uniform, greedy and privileged oracle baselines; same initial belief/backbone/action space/budgets; before/after evidence metrics; execution delay/noise/packet loss; resource costs; and premature vs unnecessary stopping.

Apply that to navigation: static recorded data can test the perception/ranging model, but cannot establish an active policy's benefit unless the chosen action changes the next observation in a controlled closed-loop simulator or physical experiment.

### 10.3 Latency compensation is a different result type from an observation

Modaberi and Far, **Real-time delay-compensated UWB localization for dynamic agents via deep trajectory prediction**, Expert Systems with Applications 322, 132248 (1 August 2026), DOI 10.1016/j.eswa.2026.132248, combines CIR and recent motion history to predict a future position aligned to inference delay. It reports 0.26 m mean error and 85% within 0.5 m across four evaluated environments. This is a paper-reported tracking result; it is not a protection-level or mission integrity guarantee.

Benchmark with identical target epochs and sensor privileges against stale last position, constant velocity, kinematic/constant-acceleration, fixed-lag/out-of-sequence filtering, learned delay compensation and Symtropy time-validity + independent predictor-tube paths. Score error at the **declared target epoch**, not at the original capture epoch. Keep forecast error, clock/age envelope, statistical residual and deterministic fault offsets separate until a reviewed composition justifies their combination.

### 10.4 Standards and industry framing

ISO/IEC 21134:2026 is published (Edition 1, January 2026) and defines master sets/test environments, benchmark metrics, benchmarking process and conformance for integrated indoor localization/tracking using dead reckoning. Align its compatible process and metrics, then add application-level integrity, source/dependency and authority-state scoreboards. Do not call the standard a UWB integrity certification.

FiRa Core 4.1 and hardware-level product specifications should remain separate protocol/product-reference scoreboards. Interoperability, STS security-test scope and typical hardware accuracy do not establish application-level trajectory coverage or false-safe probability.

### 10.5 Symtropy implementation links and current evidence ceiling

- [#7098 — decision-equivalence projection](https://github.com/Luminous-Dynamics/symtropy/pull/7098), exact-head workflow #314: 293 navigation research tests, 31 transition-validator tests, 12 MILUV-ingest tests passed.
- [#7138 — consume-once acquisition attempt ledger](https://github.com/Luminous-Dynamics/symtropy/pull/7138), exact-head workflow #315: 307 navigation research tests, 31 validator tests, 12 MILUV-ingest tests passed.
- [#7140 — time-of-validity gate](https://github.com/Luminous-Dynamics/symtropy/pull/7140), exact-head workflow #322: 312 navigation research tests, 31 validator tests, 12 MILUV-ingest tests passed.
- [#7145 — decision-aware acquisition planner](https://github.com/Luminous-Dynamics/symtropy/pull/7145), corrected proof-carrying head, exact-head workflow #331: 302 navigation research tests, 31 validator tests, 12 MILUV-ingest tests passed.
- [#7151 — reliability-gated decision planning](https://github.com/Luminous-Dynamics/symtropy/pull/7151), exact-head workflow #332: 309 navigation research tests, 31 validator tests, 12 MILUV-ingest tests passed.

These are deterministic software tests on synthetic contracts. Bregar and MILUV payloads remain unadmitted/unscored; no physical accuracy, temporal prediction error, whole-episode coverage, acquisition yield, HMI/PHMI or safety-certification result is claimed. All listed PRs remain draft and unmerged.

### 10.6 Next confirmatory experiment

The next highest-value step is one frozen chronological synthetic episode composed across the current owners: decision-relevant action selection → #7082 reliability admission → #7138 attempt start/unknown/reconciliation/qualified receipt → #7140 time-validity → independent posterior/fault-envelope re-audit → profile-governed state result. Include same-decision nuisance ambiguity, cross-decision ambiguity, shared clock/calibration common cause, stale payload, future-data leakage, invalid acquisition and valid recovery positive controls. This is still a software-composition challenge; only after it passes should the benchmark progress to source-admitted Bregar/MILUV payload scoring and then real physical acquisition.
