# TWR-CLOUD-Bern v6 — candidate admission and cooperative-integrity assessment

**Status:** candidate only. No payload downloaded, no file SHA-256 commitments, no rights clearance, no physical evaluation, no benchmark result.

**Registry record:** `twr-cloud-bern-v6` in `datasets.v0.1.json`.

**Tracking:** [#6931 — TWR-CLOUD-Bern v6 candidate qualification](https://github.com/Luminous-Dynamics/symtropy/issues/6931).

## 1. Exact source version

- Dataset: TWR-CLOUD-Bern, University of Bern.
- Current reviewed release: Zenodo Version 6, published 17 October 2026.
- Version-specific DOI: https://doi.org/10.5281/zenodo.23434390
- Record page: https://zenodo.org/records/23434390
- Technical website: https://twr-cloud.inf.unibe.ch/
- Methodology: https://twr-cloud.inf.unibe.ch/methodology.html
- Dataset dictionary: https://twr-cloud.inf.unibe.ch/dataset.html
- Evaluation framework: https://twr-cloud.inf.unibe.ch/evaluation_framework.html
- Dataset descriptor DOI: https://doi.org/10.1109/IEEEDATA.2026.3701019

Version 6 supersedes Version 5 for new research. It corrects three device-to-surveyed-position assignments in training scans:
- scan 5: device 31, position 134 instead of 137;
- scan 6: device 24, position 98 instead of 108;
- scan 7: device 23, position 110 instead of 100.

Across the four measurement/statistics tables, the corrections change 44,280 rows and 191,514 values. The publisher states that the evaluation rows (386,033 across the four tables), 148,260 evaluation scenarios, 40-agent example, and official scorer are unchanged. Existing training results remain associated with their original versions. The release notes state no measurement was added, removed, filtered, resampled or calibrated in this correction.

Never combine older training labels with Version 6 results without explicitly treating them as distinct snapshots.

## 2. Rights status: not cleared

The Zenodo Version 6 record currently renders the dataset as Open, but its Rights/License field is blank in the reviewed page. The technical website does not state an explicit licence for the payload.

Do not infer rights from public download availability, the word Open, or the article's publication status. Zenodo's help page says record reuse depends on the licence stated in the Rights section: https://help.zenodo.org/docs/deposit/describe-records/licenses/

Before downloading for use beyond metadata inspection, obtain written author/publisher confirmation of:
- research use;
- commercial use;
- derivative works and model training;
- redistribution or mirroring of original/derived data;
- attribution/citation requirements.

Until then the registry records rights as unknown, candidate_not_admitted, admission_scope=none. No source payload should be redistributed or mirrored.

## 3. Data and protocol profile

The documented experiment comprises:
- 40 Raspberry Pi Zero 2W nodes and Qorvo DWM3001CDK development kits with DWM3001C UWB modules;
- IEEE 802.15.4z compliant hardware;
- approximately 500 m² on the first floor of the University of Bern's Engehalde building;
- 170 surveyed deployment positions;
- Leica BLK360 imaging laser scanner ground-truth coordinates;
- 12 scans: 11 training scans and a distinct held-out evaluation scan using positions not used in the training scans.

Documented radio profile:
- channel 5;
- preamble length 128;
- PAC 8;
- preamble code 9;
- 6.8 Mbit/s;
- 4z SFD;
- STS OFF;
- phase difference of arrival OFF.

The ranging protocol uses four messages. The responder computes an enhanced estimate after the third message; the poller dataset contains only exchanges whose fourth message successfully returns that estimate. Consequently:
- responder-side measurements are a superset of poller-side measurements;
- the difference represents fourth-message delivery failures and other missing completion outcomes;
- poller/responder appearances are directional and should be matched by timestamp and swapped node IDs rather than assumed duplicate rows.

The nodes run unsynchronized and without time-division coordination/channel access scheduling. Listening-period choices cause real contention and collisions.

## 4. Dataset components and first acquisition scope

The Version 6 record includes:
- responder-side ranging table (~2.1M measurements, publisher-described);
- poller-side ranging table (~1.9M measurements);
- responder statistics at one-second granularity (~330K records);
- poller statistics at one-second granularity (~330K records);
- 40-agent JSON example;
- 148,260 standardized evaluation scenarios in a 3.6 GB ZIP;
- parser code, release-generation script and official scorer;
- a 25.5 GB LAS point cloud and optional viewer/documentation.

Initial work should avoid the 25.5 GB LAS point cloud unless needed to validate coordinates/environment geometry. The initial parser/scorer lane should first determine whether the measurement tables, statistics, 40-agent example and one selected evaluation-scenario subset suffice. The 3.6 GB scenario ZIP need not be extracted wholesale to verify a single scenario.

The record's publisher MD5 values are useful secondary integrity metadata only. Our own admission requires per-file byte counts and SHA-256 commitments.

## 5. What this benchmark can answer

### A. Cooperative positioning
The official framework varies:
- 0, 4, 8, 12, 16 or 20 anchors;
- 4, 7, 10, 13, 16 or 19 agents;
- single-sample and 10-sample measurement configurations;
- origin and random initial positions.

The official modes are anchor-free cooperative, anchor-aided cooperative and anchor-only non-cooperative. Use the official scenario format and scorer first, then add an integrity extension.

Matched baselines:
1. official reference implementation;
2. unweighted least squares / WLS;
3. robust M-estimation;
4. factor-graph or batch nonlinear least-squares method;
5. cooperative graph optimization with explicit topology/dependency treatment;
6. our evidence-qualified fault-hypothesis envelope and posterior audit.

All methods must use identical scenario, ToF samples, initialization, anchor/agent assignment and compute budget.

### B. Network/communication-conditioned integrity
Use both measurement tables and one-second statistics to evaluate:
- incomplete four-message exchanges;
- poller/responder asymmetry;
- node/link dropout and sparse connectivity;
- collision-heavy listening-period configurations;
- common endpoint/node failures;
- stale protocol statistics and missing evidence;
- low-connectivity geometry collapse.

Report positioning error conditional on successful ranging separately from availability, end-to-end measurement completion, and qualified pose availability. A missing fourth message is not a zero range and must not silently become a safe observation.

### C. Dependency-aware fault hypotheses
Compose with [#6900](https://github.com/Luminous-Dynamics/symtropy/pull/6900):
- leave-one-node-out;
- leave-one-link-out;
- shared-endpoint/common-node;
- shared listening-role/protocol acquisition group;
- bounded per-link or node-associated bias where the data support it;
- multi-node fault combinations under a declared cardinality budget.

Node identity or a shared endpoint is a candidate dependency relationship, not automatic proof of physical common-cause independence. Record the evidential basis and retain unknown dependencies as unknown.

### D. Active evidence acquisition
Compose with [#6855](https://github.com/Luminous-Dynamics/symtropy/pull/6855) and [#6438](https://github.com/Luminous-Dynamics/symtropy/issues/6438). Test whether the planner selects a peer/anchor that adds a genuinely new dependency/geometry constraint rather than another measurement sharing the same failure domain.

## 6. What it cannot establish

This dataset does not include raw CIR/waveform samples. It cannot directly benchmark:
- raw-CIR NLOS classification;
- CIR feature extraction or MCU CIR inference;
- STS cryptographic/session security (the documented STS setting is OFF);
- physical early-path injection/relay/overshadowing resistance;
- continuous mobile/UAV trajectories;
- dynamic tracking, time-to-alert under a continuous mission or full mission-level integrity risk;
- hardware power/energy beyond documented node statistics unless external power instrumentation exists.

Use Bregar for raw-CIR development and MILUV for multi-UAV trajectory/passive-observer studies after source-complete admission. Do not transfer scores across these distinct task profiles.

## 7. Statistical design and test leakage

The official evaluation scan is a single held-out deployment with unseen positions. It is valuable for spatial generalization but must remain entirely untouched until final evaluation.

The 148,260 scenarios are derived from that single held-out scan. They are not 148,260 independent missions. They can support controlled comparisons across node/anchor/agent configurations, but must not inflate rare-event confidence denominators.

Training and calibration splits must be grouped by scan and position identity. No random row split across measurements from the same scan or deployment position.

Anchor-free localization has a gauge freedom: positions may only be identifiable up to a rigid transform without fixed anchors. Use the official scoring/alignment convention. Report both the gauge-aligned relative-position result and any absolute-reference result separately. Do not attribute an alignment-free coordinate error to the estimator when the problem itself is unobservable in global translation/rotation.

Keep ground-truth agent positions evaluator-only. Controller event availability, row count, order and digest must remain invariant under evaluator coordinate/truth mutations.

## 8. Metrics

Report separate axes:
- median, p95, p99 and RMSE 3D position error;
- convergence/failure and unobservable scenario rates;
- error versus anchor count, agent count, graph degree and mean connected-anchor distance;
- CDFs by positioning mode and sample count;
- range completion and four-message completion rates;
- measurement availability, directional poller/responder success and per-stage message failure;
- protection envelope width and independent truth containment where a qualified truth comparison is possible;
- false-safe/unsupported-NORMAL exposure and correct abstention;
- time to detect node/link faults and qualified recovery where the scenario has a time axis;
- computation time, worst-case hypothesis count, RAM and energy if directly measured.

Do not combine these into one weighted score. A single static held-out scan does not provide independent mission trials for a rare-event claim.

## 9. Admission gates

1. Written payload rights confirmation.
2. Exact Version 6 file inventory, byte counts and SHA-256s.
3. Reconciliation against the Version 6 release manifest and reproduction script.
4. Independent decoded-data comparison, including column types, row identities, ordering and the three corrected training device/position assignments.
5. Poller/responder matching and directionality audit.
6. Timestamp unit/order/clock semantics and one-second stats window audit.
7. Anchor/agent coordinate frame, LiDAR survey and position-reference lineage review.
8. Controller/evaluator separation and truth-mutation negative controls.
9. Frozen scan/position split and exact official scenario/scorer version.
10. Independent parser and scorer review.

Only after all gates pass may the payload become an admitted physical benchmark. Rights clearance, payload admission and physical-performance qualification remain distinct decisions.

## Claim ceiling

This candidate provides a route to a reproducible cooperative UWB ranging benchmark with realistic contention and unseen-position testing. No Luminous data result, SOTA comparison, physical fault detection, false-safe rate, cryptographic security result or navigation safety claim is currently established.
