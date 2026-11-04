# CivBench Cyber-Physical Event Evaluation v0.1

**Status:** research contract proposal; not an externally registered confirmatory protocol  
**Owner:** CivBench / Symtropy qualification track  
**Parent contract:** `CIVBENCH-JHB-WATER-EXPERIMENT-001`, protocol v0.1  
**Related:** [Symtropy #3510](https://github.com/Luminous-Dynamics/symtropy/issues/3510), [PR #3395](https://github.com/Luminous-Dynamics/symtropy/pull/3395), [PR #3412](https://github.com/Luminous-Dynamics/symtropy/pull/3412)

## 1. Purpose and claim boundary

This contract evaluates whether an event-monitoring and response pipeline can detect consequential cyber-physical incidents early enough to support safe, feasible response, while controlling false alarm burden and preserving essential service.

The primary evaluation unit is an **incident/event**, not a telemetry sample. Ten thousand positive samples from one prolonged incident are one incident, not ten thousand successful detections.

This document does not introduce a new water simulator, hydraulic solver, OT platform, asset registry, operator authority model or service-workflow owner. It extends the separate CivBench C1–C3 track:

- **C1 — event discrimination:** distinguish supported leak, sensor, communication, process-drift and cyber event classes.
- **C2 — cyber-to-physical propagation:** measure whether a cyber event produces a material process deviation in a controlled testbed/model.
- **C3 — containment and recovery:** compare safe-state, isolation and recovery policies under matched scenarios.

Synthetic, testbed, SCADA benchmark and utility evidence remain separate. A testbed result is not a municipal deployment result. A detector score is not a service outcome. A hash or signed receipt proves only the integrity/provenance of the artifact it binds, not the truth of the underlying physical claim.

## 2. Why event-level evaluation

A 2026 systematic review of 32 peer-reviewed studies on AI-based detection in water distribution networks reports a concentration on synthetic and testbed datasets, little cross-dataset validation in the reviewed corpus, and limited linkage between detection and operational resilience outcomes. The review recommends broader dataset diversity and evaluation of impacts and recovery, while noting the evidence-base limitations of the included studies: https://doi.org/10.3390/w18040519.

The existing CivBench protocol also records a 2026 chronological water-testbed result in which cumulative alarm policies increased sample-level recall while sustaining alarms and detecting fewer event onsets than a fixed-threshold policy in that paper's five primary runs. This is a paper-specific finding, not a universal model ranking: https://doi.org/10.3390/eng7100564.

Therefore sample accuracy, pointwise recall and ROC-AUC may be reported as diagnostics, but **must not be the primary event-performance claim**.

## 3. Event taxonomy and truth provenance

The evaluator uses the following mutually distinguishable classes:

| Class ID | Meaning | Qualification requirement |
|---|---|---|
| `leak` | Physical leakage / hydraulic event | Dataset or scenario truth supports a leak event |
| `sensor_fault` | Sensor, instrument or calibration fault | Source supports instrument fault or injection |
| `communication_fault` | Link, gateway, timestamp, transport or telemetry failure | Source supports communication impairment |
| `process_drift` | Ordinary drift, maintenance transition or non-malicious process change | Declared process/event evidence |
| `cyber_compromise` | Unauthorized access, command or compromise | Independent source/testbed attack ground truth |
| `unknown` | Cause or event status unresolved | Explicitly unknown; never silently converted to a negative |

The taxonomy separates **observed signal**, **hypothesized cause**, **confirmed event class**, and **physical consequence**. A hydraulic residual, outlier or model disagreement alone cannot establish malicious intent. A cyber event without process deviation remains a cyber event without demonstrated physical impact. A process deviation without a confirmed cyber cause must not be attributed to an attacker.

Truth records belong to the evaluator. Candidate policies receive only the released observation stream and permitted metadata. This contract does not itself provide operating-system or process-level isolation; the runner must enforce that boundary.

## 4. Canonical time and record contract

The evaluator must declare one time basis per dataset lane. Normalize timestamps to non-negative millisecond offsets from a declared UTC window origin. Retain each source timestamp, timezone/clock basis and conversion provenance. A monotonic device clock must not be silently treated as UTC wall time. Clock uncertainty and timestamp correction must be reported; the origin and conversion are part of the frozen dataset-lane manifest.

An incident truth record contains:

- immutable `event_id`, `scenario_id`, `source_id` and dataset identity;
- class and truth status (`confirmed`, `unknown`, or `not_applicable`);
- onset and end when actually supported;
- evidence reference/digest and source time basis;
- optional declared severity/physical-consequence reference.

An alarm record contains:

- immutable `alarm_id`, `scenario_id`, source identity and emitting detector/version;
- alarm start and `clear_state` (`closed` or `still_open`) plus a clear time when closed; a still-open alarm has no clear time and remains active through the evaluation horizon.
- asserted class or `unclassified`;
- evidence/provenance reference and observation time basis.

Reject duplicate IDs, impossible intervals, out-of-order source sequences where the source promises order, scenario-boundary crossings and missing required provenance. Do not repair malformed input by sorting away evidence of clock or sequence failure. Preserve a diagnostic status for invalid records.

Unknown event intervals and unlabelled periods are not verified negative periods. Alarms overlapping unresolved truth are reported as **unscorable/overlapping-unknown**, not forced into the false-positive denominator.

## 5. Alarm episode construction and event matching

### 5.1 Alarm episodes

A pre-registered `merge_gap_ms` defines when consecutive alarms from the same scenario and source belong to one alarm episode. Overlapping alerts are merged. Alerts separated by a gap no larger than the declared value are merged; longer gaps begin a new episode.

The merge gap is selected using a calibration partition and frozen before evaluation. It may not be tuned on the final test. Episodes never merge across scenario, source or dataset boundaries. Retain every constituent alarm ID so fragmentation and repeated alerts remain auditable.

### 5.2 Event association versus timely detection

Keep these separate:

- **Association window:** an episode can be associated with a confirmed event when it has the same scenario and starts in `[event_onset - allowed_lead_ms, event_end + late_association_grace_ms]`. Association is class-agnostic; class correctness is evaluated separately. Association is not proof of correct classification or timely detection.
- **Timely detection window:** the first alarm-episode onset falls in `[event_onset - allowed_lead_ms, event_onset + maximum_detection_delay_ms]`.

First perform deterministic **maximum-cardinality one-to-one timely matching** with an augmenting-path algorithm. Order events by eligible-candidate count ascending, then event ID; within each event, visit candidate episodes by absolute onset difference, then episode ID. This yields a maximum-cardinality matching with a reproducible preference order; it does not claim to minimize aggregate delay across all possible matchings. On the remaining unmatched events and episodes, perform one-to-one late association using the same ordering. An event receives either a timely match or a late association, never both; an episode can be assigned to at most one event. Any alternate matching rule requires a protocol version change.

Event-onset matching is **class-agnostic**: a timely alarm can detect an event even when its asserted class is wrong. Score classification afterward from the matched episode's asserted class using the published confusion matrix. An `unclassified` alarm can contribute to detection but not to correct class attribution. If every constituent alarm asserts the same class, the episode retains that class; if constituent labels conflict, the episode is `conflicting`; if any constituent is unclassified, the episode is `unclassified`. These states do not change event matching, but `unclassified` and `conflicting` episodes receive no correct-class credit. This prevents an event detector from hiding a detection merely because its diagnosis was wrong.

Additional episodes associated with an already timely matched event are **fragmentation/re-alert burden**, not extra true positives. An episode associated with a confirmed event but outside the timely window is a **late associated alarm**. A wholly unmatched episode outside unresolved-truth intervals is a false alarm episode. An episode overlapping unknown truth is unscorable unless an independent truth source resolves the interval.

## 6. Required metrics

Report by dataset lane, event class, scenario stratum and detector version. Include denominators and confidence intervals; do not pool unlike datasets into a single score.

| Metric | Definition / reporting requirement |
|---|---|
| Event-onset recall | Timely one-to-one detected confirmed events / confirmed eligible events |
| Event precision | Timely matched episodes / episodes eligible for event scoring; show late, false and unscorable episodes separately |
| False alarm episode rate | Unmatched confirmed-false episodes / monitored system-days; declare monitored exposure time |
| Onset delay | First qualifying episode onset minus truth onset; report lead-time and delay separately, p50 and p95 |
| Event fragmentation | Additional episodes associated with an event after its first matched episode |
| Alarm duty cycle | Union duration of alarm-active intervals / monitored duration |
| False-alarm duration | Time in confirmed-negative exposure covered by false alarm episodes |
| Class confusion | Event-level confusion matrix; unknown and unclassified remain explicit |
| Calibration | Brier/log score or suitable classwise calibration measure on the declared event unit; calibration plots and sample counts |
| Abstention | Coverage, abstained event fraction and error-versus-coverage curve |
| Detection-to-containment | Onset-to-detection and detection-to-authorized-isolation intervals, separately |
| Safe-state / recovery | Time to safe state, restoration time and recurrence after independently defined verification |
| Physical service impact | Service-deficit area, worst-group service floor, quality excursions and other supported physical outcomes |

A sample-level confusion matrix may be included as a diagnostic, but it cannot replace the event-level results. Do not treat adjacent samples, windows or timesteps as independent incidents for confidence intervals.

**Implementation boundary:** the first dependency-free Rust scoring core accepts alarm time and asserted class, not calibrated class probabilities or operator/physical-response records. It therefore computes episode construction, one-to-one event matching, event precision/recall, class-confusion counts, delay/lead-time quantiles, false-alarm burden and alarm duty cycle. Calibration, abstention/coverage, containment, safe-state, service-deficit and restoration metrics are specified by this protocol but remain **not measured** until separate probability-output and response/outcome interfaces are supplied. A binary alarm must never be reverse-engineered into a confidence score.

## 7. Dataset lanes and transfer discipline

The following are distinct evidence lanes:

1. **WDSEventDB:** real-time measurements from a sensor-instrumented research water testbed, with leak, cyberattack and sensor-failure events. Preserve its event definitions and source archive digest. Dataset record: https://zenodo.org/records/17547955.
2. **ACG-Water-Testbed:** controlled IT/OT-to-physical propagation and defensive-control experiments, including remote access, segmentation, PLC/controller, sensors, pumps and storage dynamics. Research-only; no production systems: https://zenodo.org/records/21814947.
3. **BATADAL/C-Town:** historical simulated SCADA with one-year normal training data, partially labelled attack data and a three-month unlabelled test period. Preserve the official challenge rules and label limitations: https://www.batadal.net/data.html.
4. **SWaT/WaDi and other ICS corpora:** adjacent industrial-control transfer lanes, not water-distribution equivalents. Report them separately.
5. **Utility-derived observations:** require independent data governance, a declared source/assurance class, and a claim ceiling. Public weak-label utility data are not automatically a blind holdout.

For each defensible pair of datasets, report train-on-A/test-on-B and train-on-B/test-on-A, with all transforms and thresholds fitted only on training/calibration partitions. Freeze event groups and chronological boundaries before evaluation; purge lookback, onset and recovery windows at split boundaries to prevent leakage. Report unavailable raw data and un-reconstructable preprocessing explicitly.

## 8. Baseline ladder and matched information budgets

Compare at least:

- fixed threshold;
- hysteresis/persistence;
- CUSUM and EWMA;
- transparent classical ML;
- temporal or graph-aware detection;
- hydraulic/process residual plus OT-context fusion.

Declare each method's input modalities, update cadence, compute budget, training data and threshold-calibration procedure. A network-traffic detector and a hydraulic-only detector do not have matched information access; compare within modality lanes before comparing system-level compositions.

For cyber-to-physical and recovery experiments, compare nominal/reference controls with documented secure remote-access, least-privilege, segmentation, monitoring, safe-state and recovery configurations under identical controlled event schedules. The evaluation measures the effect of the declared controls; it does not authorize live interaction.

## 9. Negative controls and acceptance tests

The evaluator implementation must reject or correctly classify all of these:

1. One long positive sample run from one incident scores one event, not hundreds of true positives.
2. Repeated alarms within the frozen merge gap form one episode and preserve constituent IDs.
3. One episode cannot score two nearby incidents; one incident cannot score two timely detections.
4. An alarm immediately outside the onset window is late/unmatched according to the declared temporal association rule, never silently timely.
5. A timely alarm with the wrong asserted class still counts as event detection and appears as a class-confusion error.
6. Persistent false alarms expose duty-cycle and false-duration costs even when sample-level precision is high.
7. Missing/unknown onset does not become a false negative or verified true negative.
8. Leak, sensor fault, communication fault, process drift and cyber compromise remain separate event classes.
9. Duplicate IDs, replayed IDs, invalid intervals, source-boundary crossings and malformed chronology fail closed.
10. A generic anomaly without independent cyber evidence receives no confirmed-attack attribution.
11. A cyber event without a process deviation is not reported as physical impact.
12. A physical consequence with no timely alarm remains a missed event.
13. An alarm without successful containment or restoration is not a recovered-service outcome.
14. Input-order permutations produce the same normalized output where order is not semantically meaningful.
15. Repeated runs with identical frozen inputs, code and runtime produce byte-identical receipts.

## 10. Aspirational competitive targets

First reconstruct baselines under a common frozen episode and event-matching policy. Then pursue either:

- at least **+5 percentage points event-onset recall** at the same pre-declared false-alarm-episode budget; or
- at least **30% fewer false-alarm episodes** at matched event-onset recall;

without worsening p95 onset delay, event-class confusion, calibration, worst-stratum outcomes or protected service floors.

These are research targets, not literature-derived universal operating limits. Alarm budgets and onset windows must be selected with operators and registered before confirmatory evaluation. For controlled hardening fixtures, target zero unauthorized control actions accepted by the hardened reference configuration, and publish every test case; this is testbed conformance, not a production-security guarantee.

## 11. Safety, authority and ownership

- Use offline replay and isolated, purpose-built testbeds only. No production PLC/SCADA, utility credentials, public OT endpoint or operational network.
- Human authorization, independent safety interlocks, safe-state transition, rollback and recovery remain mandatory in controlled actuation experiments.
- **Sol-Atlas** owns physical/hydraulic modelling and physical consequence context. **Symtropy** owns benchmark execution and qualification. **Mycelix** owns governance, authority, contestability and appeal pathways.
- A detection or security receipt does not grant authority, prove intent, prove repair, prove safe water quality or establish causal improvement in household service.

## 12. Required result package

Each result package binds the protocol/contract version, dataset lane and archive digest, scenario/event split digest, detector and threshold digests, input-modality declaration, runtime/toolchain, episode parameters, event-matching policy, complete metric vector, exclusions/unscorable intervals, logs and output digest.

A result is marked **design**, **synthetic**, **testbed**, **historical SCADA**, or **utility-observed**. It cannot promote itself to a higher evidence class. Confirmatory status requires an externally timestamped pre-registration and independently verifiable source snapshots; this document alone does not provide either.
