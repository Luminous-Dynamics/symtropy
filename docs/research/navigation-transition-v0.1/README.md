# Navigation Observability Transition Benchmark v0.1

**Status:** draft contract and synthetic fixtures; not executed or qualified  
**Owner:** Symtropy scenario/transition benchmark  
**Cross-stack parent:** Symthaea EMSR-001 #10775  
**Transition benchmark:** Symtropy #6434  
**PNT degradation substrate:** Symtropy #3909

## Purpose

This benchmark evaluates whether a navigation system preserves a justified spatial belief while its observations, sensor modalities, reference frames, time bases, geometry and communication paths change.

It deliberately does not attempt to build another estimator. It composes:

- Mycelix Position #413 / #416 / #417 / #435 / #458 for spatial truth, frames, fusion, qualified time and profile-specific navigation health;
- Mycelix #5899 / #5904 for observation provenance and common-mode dependencies;
- Symthaea #10824 / #10826 for the mobile adapter and independent ground-truth/oracle qualification;
- Symtropy #3909 / #1612 / #3893 / #4115 for PNT degradation, deterministic replay, maritime modes and airborne pose observability;
- Sol-Atlas #584 for the PNT/timing/communications dependency twin.

No new global coordinate engine, time service, trust authority, runtime-assurance controller, evidence ledger or physical actuation authority is introduced here.

## Paradigm shift: measure continuity of justified belief

A static localization benchmark usually asks how close an estimated trajectory is to reference truth on one data sequence. This benchmark asks whether the system remains justified when the observation regime changes.

A transition episode has four separately represented layers:

1. **Controller-visible stream:** observations, source-health changes, integrity messages, transform availability, clock-correlation status and network status.
2. **Source registry:** modality, configuration, calibration reference, source clock domain and declared failure domains.
3. **Evaluator-only oracle:** independent/synthetic trajectory truth and fault adjudication. This object must be withheld from the navigation subject.
4. **Expected disposition:** the minimum defensible service state given the declared profile and currently admissible evidence.

The important outcome is not always a coordinate. It can be a qualified estimate, a bounded coast, a degraded state, an explicit abstention or an INDETERMINATE result.

## Initial synthetic transition coverage

The current fixture contains fourteen authored synthetic episodes:

| Case | Environment transition / challenge | Main failure mode |
|---|---|---|
| Land → urban → tunnel | GNSS degradation followed by GNSS loss and local geometric/inertial navigation | NLOS, multipath, sensor availability and frame transition |
| Surface water → underwater | GNSS loss, DVL/depth/sonar support and acoustic-network partition | Local navigation vs network reachability |
| Airborne GNSS integrity degradation | Plausible but integrity-uncertain GNSS followed by loss | False continuity and alternate-source qualification |
| Orbital absolute → optical relative | Ephemeris/inertial support, optical bearing, target occultation | Relative-only state, mission clock and observability collapse |
| Common-mode false independence | Three receiver identities with shared antenna/clock/power/correction dependencies | Peer count mistaken for independent evidence |
| Frame/time-basis mismatch | Deliberate frame and time association error | Silent coordinate or timestamp laundering |
| Lunar body-fixed → local surface | Moon body-fixed state to local terrain frame | Body identity and epoch-bound transform |
| Independent multimodal positive control | UWB + LiDAR + IMU with distinct declared failure domains | Correct acceptance of genuinely independent evidence |
| Valid land reacquisition positive control | Tunnel GNSS loss → fresh GNSS + transform + profile requalification | Recovery must not occur on source availability alone |
| Benign airborne camera loss | Camera obscuration while GNSS/INS and air data remain valid | Avoid false service loss from a noncritical modality fault |
| Space occultation and reacquisition | Optical target loss → relative reacquisition → absolute reference requalification | Separate relative recovery from absolute-state restoration |
| Indoor observability recovery | Geometry collapse → new viewpoint → repeated independent constraints | Condition-aware recovery rather than sensor-count heuristics |
| Protection-level cap saturation negative control | Persistent GNSS bias → PL ceiling reached while true error exceeds the cap | A bounded-looking protection level must not remain NORMAL when no longer a defensible overbound |
| Heavy-tailed residual integrity case | Non-Gaussian GNSS residual bursts during urban degradation | Compare Gaussian covariance and heavy-tail uncertainty claims on matched held-out episodes |

These are synthetic contract fixtures, not measured sensor traces or physical-world claims. The 14-case corpus is within the proposed 12–20 preregistration range and includes positive controls for independent sources, valid reacquisition, benign faults and observability recovery, plus negative controls for protection-level saturation and heavy-tailed residuals. Exact seeds, a full JSON Schema run, semantic-validator execution, independent oracle qualification, rights-admitted datasets and held-out execution remain open.

## External comparison anchors

The goal is to reproduce strong, task-matched baselines rather than compare unrelated headline numbers.

### GNSS / urban and interference resilience

- [UrbanNav](https://github.com/IPNL-POLYU/UrbanNavDataset) is a multisensor positioning dataset from Asian urban canyons. Its project page describes GNSS receiver, LiDAR, camera and IMU observations with SPAN-CPT reference data. Freeze the exact dataset release, sequence, license, calibration and reference-quality assumptions before admitting it.
- A 13 May 2026 cross-domain GNSS spoofing study on TEXBAT and OAKBAT reports average in-domain classification accuracy above 90% for its evaluated models, while its cross-testing accuracy falls to 66% or less. This motivates receiver/dataset/campaign holdouts; the result is not a general ceiling for GNSS integrity. [IEEE Access, DOI 10.1109/ACCESS.2026.3693081](https://doi.org/10.1109/ACCESS.2026.3693081).
- Use receiver-native integrity tests and statistical residual/RAIM-style baselines alongside matched ML methods. Anomaly classification is not proof of physical position validity, intentional interference or actor attribution.

### Indoor / terrestrial localization

[ISO/IEC 21134:2026](https://www.iso.org/standard/86651.html), published January 2026, specifies a reference framework for benchmarking integrated indoor localization and tracking methods using dead reckoning, including test environments, metrics, process and conformance. Align the indoor track to this framework. The standard is a benchmark-method reference, not proof that this v0.1 corpus conforms to it.

### Underground / subterranean robotics

[SubT-MRS (CVPR 2024)](https://openaccess.thecvf.com/content/CVPR2024/html/Zhao_SubT-MRS_Dataset_Pushing_SLAM_Towards_All-weather_Environments_CVPR_2024_paper.html) is a useful multi-platform reference for SLAM under subterranean and adverse visibility conditions. [CERBERUS DARPA SubT datasets](https://github.com/leggedrobotics/cerberus_darpa_subt_datasets) provide additional cave/tunnel sensor records. Exact release, redistribution and commercial-use terms must be reviewed independently. Neither benchmark establishes mine-wide positioning or personnel safety.

### Underwater navigation and SLAM

The [Tank dataset](https://senseroboticslab.github.io/underwater-tank-dataset/) includes stereo camera, IMU, DVL, pressure/depth and 6-DoF reference poses over eight tank sequences. Its published license is **CC BY-NC-SA 4.0**; do not use its data in a commercial-use path or redistribute it incompatibly without separate permission. The associated paper compares UVA, SVIN2, ORB-SLAM3 and VINS-Fusion under its stated setup. [Dataset paper](https://doi.org/10.1177/02783649251364904).

This is a strong controlled underwater SLAM baseline, not an open-ocean or long-duration multi-AUV navigation qualification. AQUALOC and Girona/Caves data remain candidate additions subject to exact source and rights review.

### Space / relative navigation

NASA's StarFOX is an actual on-orbit angles-only relative-navigation demonstration. NASA's current GNC overview reports approximately 0.5% relative-range error in the demonstrated configuration. This is a technology-demonstration reference, not a downloadable general leaderboard, and it must not be conflated with relative pose, rendezvous guidance, docking capture or servicing-arm performance. [NASA GNC state of the art](https://www.nasa.gov/smallsat-institute/sst-soa/guidance-navigation-and-control/).

NASA/JPL SPICE may supply ancillary ephemerides and observation geometry, but SPICE is not an orbit-determination or attitude-estimation algorithm. The exact NAIF kernels and time/frame conversions must remain bound to the evaluation record. [NAIF SPICE](https://naif.jpl.nasa.gov/naif/public.html).

## Baselines

For each domain/task, use the strongest exact-match baseline available:

- domain-appropriate ESKF/EKF and a factor-graph smoother where the data and measurement model support them;
- benchmark-native reference implementations;
- visual-inertial baselines such as ORB-SLAM3/VINS-Fusion for matching underwater/terrestrial visual tasks;
- underwater acoustic/visual/inertial systems such as UVA where the released sensor channels match;
- receiver-native GNSS integrity and statistical residual baselines;
- the frozen pre-change Mycelix/Symthaea subject;
- simple modality ablations and a cognition-disabled baseline when measuring Symthaea's decision value;
- an oracle trajectory only as an evaluator upper bound, never a fair navigation controller.

Never compare headline scores across different sensor privileges, reference truth, environments, splits, hardware or scoring protocols.



### Cooperative fusion / integrity comparators (2026)

Two recent papers should be included as direct method comparators in the cooperative-localization track:

- **DECLARE: conflict-aware evidence fusion for robust decentralized cooperative localization**, *Engineering Applications of Artificial Intelligence* 181, Part 7 (1 October 2026), [DOI 10.1016/j.engappai.2026.115684](https://doi.org/10.1016/j.engappai.2026.115684). Its abstract describes continuous evidence representations, conflict between incoming evidence and predicted beliefs, Bayesian source credibility, entropy-based information quality and a conservative covariance-intersection-compatible discount, evaluated in simulation and on a real multi-robot dataset. It is a direct comparator candidate; exact artifact, dataset, licensing, protocol and results must be frozen before claiming reproduction.
- **Resilient distributed localization for mobile sensor networks under malicious attacks**, *Information Sciences* 724 (January 2026), [DOI 10.1016/j.ins.2025.122708](https://doi.org/10.1016/j.ins.2025.122708). Its abstract describes innovation-based saturation for unreasonable relative-measurement innovations combined with covariance intersection, with consistency and bounded covariance analysis. Reproduce only the defensive software-level methodology and its stated threat assumptions.

The Mycelix PR #6389 is a correctness repair to the existing CI-labelled primitive, not a reproduction of either method. The matched benchmark should compare corrected two-source diagonal CI, the explicitly named sequential CI approximation, faithful DECLARE reproduction, innovation-saturation + CI, and task-appropriate classical distributed ESKF/factor-graph baselines. The historic trust-weighted average may be retained only as a defective historical baseline.

Extend the report beyond ATE/RPE to include NEES/NIS or an applicable consistency statistic, empirical confidence-region coverage, covariance boundedness under the declared fault model, conflict-detection quality, false accepted-position exposure, false rejection, recovery, communication and compute cost. Do not compare abstract headline claims unless dataset, sensors, split, evaluator, threat model and metric definitions match.

A further research direction is information-driven navigation: monitor observability (including Fisher-information rank/conditioning where the model supports it), and compare next-observation or next-best-view policies against passive fixed-rate sensing. Report information gain alongside measurement cost, travel/energy, time, safety envelope and downstream decision value. This should compose the existing geometry/observability owners, not create a second measurement planner.

## Metrics — keep them separate

Report by domain, transition, scenario and seed:

- ATE/RPE and position, velocity and attitude errors where independent truth supports them;
- p50/p95/p99 error, worst-tail error and maximum unqualified interval;
- covariance/protection-region empirical coverage and calibration;
- time-integrated invalid/unqualified-position exposure;
- false accepted-position exposure after integrity loss;
- time-to-detect, false alarms per operating hour and false exclusion;
- observability/conditioning and estimator consistency;
- time in NORMAL / DEGRADED / INTEGRITY_UNCERTAIN / UNAVAILABLE / RECOVERING / INDETERMINATE;
- requalification time and premature healthy-state re-entry;
- common-mode false-independence rate and correct unknown/abstention rate;
- energy, compute, memory, bandwidth and human handover burden;
- deterministic replay and independent-verifier reconstruction.

Do not publish one universal navigation score. Compare each operating profile's error/continuity/false-safe/recovery/resource Pareto frontier.

## Proposed research targets (not achieved)

- Zero accepted frame/body/time-basis mix-ups in the frozen negative-control corpus.
- Zero false independence decisions for known common-mode cases.
- Zero premature healthy-state re-entry in the frozen synthetic recovery corpus.
- Empirical uncertainty-region coverage consistent with the declared confidence level and model assumptions.
- At matched operating point, pursue at least 20% reduction in p95 trajectory error or unqualified-position exposure on a preregistered transition profile without worsening false alarms or false-safe exposure.
- Every decision-relevant output binds exact input, configuration, model/profile, frame/time transform, evidence snapshot and oracle-manifest digests—or returns INDETERMINATE.
- 100% byte-reproducible replay of the frozen synthetic corpus.

The 20% figure is a falsifiable research hypothesis, not a claimed result or a universal target across all domains.

## Current artifacts

- `nav_transition_case.schema.json`: JSON Schema Draft 2020-12 authoring contract for a single case.
- `fixtures/nav_transition_cases.v0.1.json`: fourteen synthetic episodes with an explicit controller/evaluator split.

The schema verifies structure only. A later semantic validator must enforce cross-reference integrity (case/source/frame/time IDs), monotonic event schedules, interval consistency, frame/body compatibility, source-clock-to-time-basis correlation, transformation validity, source common-mode graph correctness, oracle separation and expected-state logic. Schema syntax and fixture validity have not yet been independently checked or executed in CI.

## P0 work still required

1. Preserve the independent-source, valid-recovery and benign-loss controls; add known-answer frame/time transformations and additional domain-held-out episodes.
2. Add a semantic validator and mutation tests; a JSON Schema pass alone cannot establish cross-field correctness.
3. Freeze the current 14-case corpus, exact seed list, profiles and primary outcomes before held-out evaluation.
4. Bind the independent oracle manifest and source rights state using the existing EMSR receipt/ground-truth owners.
5. Audit each scenario against the exact Symtropy physics/model-capability census. Unsupported physical effects remain abstract or UNKNOWN; no convenient coefficients are invented.
6. Reproduce matched baselines on the exact data release and metric definitions before claiming any SOTA comparison.

## Explicit non-claims

This is not a navigation estimator, a real-time runtime-assurance controller, a flight/maritime/mine/space safety case, airworthiness/seaworthiness certification, an operational PNT service or physical navigation authority. No RF attack generation, emitter targeting, kinetic cueing or physical actuation is in scope. Synthetic success does not establish field performance.


### Frozen v0.1 case IDs

1. `nav.land.open-urban-tunnel.v0.1`
2. `nav.surface-to-underwater-acoustic-loss.v0.1`
3. `nav.air-gnss-integrity-degradation.v0.1`
4. `nav.space-orbit-optical-relative-reacquisition.v0.1`
5. `nav.common-mode-false-independence.negative.v0.1`
6. `nav.frame-time-basis-mismatch.negative.v0.1`
7. `nav.lunar-body-fixed-to-local-terrain.v0.1`
8. `nav.independent-multimodal-positive.v0.1`
9. `nav.land.valid-reacquisition-positive.v0.1`
10. `nav.air.benign-vision-degradation-control.v0.1`
11. `nav.space.occultation-reacquisition.v0.1`
12. `nav.indoor.observability-recovery.v0.1`
13. `nav.integrity.protection-level-cap-saturation.negative.v0.1`
14. `nav.land.heavy-tail-residual-integrity.v0.1`


## 2026 research, industry and dataset comparator register

See [INDUSTRY-AND-SOTA-COMPARATORS.md](./INDUSTRY-AND-SOTA-COMPARATORS.md) for the updated method comparison, commercial demonstration references, benchmark dataset candidates, dataset-rights caveats, matched-baseline protocol, required metrics and preregistered research targets.

The register deliberately separates:
- established 2026 SOTA methods from Luminous implementations;
- vendor-reported system demonstrations from public matched numerical results;
- dataset paper licence from the licence of every underlying data asset;
- synthetic/analytic prototype results from external benchmark and physical validation.

No dataset is considered admitted to a commercial or redistributable benchmark solely because its paper is open access.
