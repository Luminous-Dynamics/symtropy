# EV-BMS event-onset censoring and warning benchmark v0.1

**Programme:** Luminous Dynamics EV engineering  
**Status:** research protocol; no event-onset truth, model score, physical validation, or safety certification is claimed  
**Date:** 28 November 2026  
**Dependency:** EV-BMS-EVENT-CENSORING-001 (#7871), extending—but not replacing—the point-time event evaluator in #7853

## 1. Why the benchmark needs a second clock model

An EV battery fault rarely comes with a directly observed physical onset timestamp. Available records can instead be:

- the first BMS fault bit or diagnostic trouble code;
- the first sample crossing a threshold;
- the first workshop inspection;
- a pack teardown finding;
- the maintenance confirmation timestamp;
- an independently adjudicated physical onset estimate.

These are different events. A timestamp can be perfectly accurate as a *record timestamp* while remaining a poor estimate of physical onset. Treating the first available record as exact onset can manufacture apparent warning lead time or incorrectly label a late alarm as an early warning.

This is not a minor bookkeeping problem. The 2026 multi-fidelity SOH/RUL review (176 classified studies) argues that deployment credibility depends on observability, domain transfer, calibration, pack-level evidence, warning risk and staged validation—not a single cell-level error score. A 2026 unsupervised thermal-runaway paper explicitly evaluates vehicle-level false-positive behaviour and warning lead time, but those metrics still need to be interpreted against the available event-time evidence.

**Sources**
- Rizanov, Stoynova & Mihov (2026), *Batteries* 12(7), 255, DOI: https://doi.org/10.3390/batteries12070255
- Li, Li & Wen (2026), *Energy*, DOI: https://doi.org/10.1016/j.energy.2026.142573
- Liu et al. (ICLR 2026), CH-BatteryGen: https://proceedings.iclr.cc/paper_files/paper/2026/hash/0d781fa5f639bf2caf728a68e9678362-Abstract-Conference.html

## 2. Event time is a bounded claim, not just a timestamp

For each event, store a time-bound object:

| Field | Meaning |
|---|---|
| onset_lower_ms | Earliest time at which the event could have begun, given the admitted evidence |
| onset_upper_ms | Latest time by which the event is known to have begun |
| lower_bound_semantics | Why the lower bound is credible (for example, last independently verified normal sample) |
| upper_bound_semantics | Why the upper bound is credible (for example, first verified abnormal sample or teardown confirmation) |
| time_evidence_refs | Immutable references to the actual observations and their evidence class |
| onset_adjudication_revision | Version of the rule/authority that constructed the interval |

The evaluator must not silently infer these bounds from a service note. If one side is unavailable, the event is censored; do not fabricate a point timestamp.

### 2.1 Shared time basis and clock uncertainty

Alarm times, event bounds, and monitoring exposures must be normalized to the same explicit time basis (for example, UTC Unix milliseconds). The evaluation manifest binds a time-basis ID, a clock-alignment artifact ID, the maximum relative clock error, and an explicit assertion that the uncertainty has already been applied.

The onset interval must be widened conservatively to include source timestamp resolution and relative clock-alignment uncertainty. Monitoring exposure windows must be guard-banded so that only continuously valid, time-aligned coverage is represented. The evaluator fails closed when the time basis or alignment evidence is missing, or when the manifest does not assert that uncertainty was applied. A clock-alignment assumption is not itself proof: the alignment artifact and its validation method must be retained and independently reviewed.

For a source with a verified one-millisecond clock resolution, if the last verified-normal sample is at t, the earliest possible onset bound is no earlier than t+1 ms; do not store the last-normal timestamp itself as an inclusive possible-onset lower bound. When resolution or clock uncertainty is larger, expand the interval accordingly.

### 2.1 Four states

- **Exact reported time:** lower = upper, but retain what the timestamp actually represents. It may be an alarm, first positive sample, maintenance confirmation, or an independently adjudicated onset.
- **Interval-censored:** both bounds are known and lower < upper.
- **Left-censored:** the event may have started before reliable monitoring began; only an upper bound may be available.
- **Right-censored follow-up:** no event is observed through a recorded follow-up end. This is finite at-risk exposure, not an eternal negative label.

For an exact reported time, the record's exactness does not upgrade its semantic authority. In particular, a BMS alert time is not physical thermal-runaway onset.

## 3. Definite versus possible warning

Let the physical onset time be unknown but lie in the closed interval:

\[
T \in [L,U]
\]

Let an alarm episode occur at time \(A\), and let the declared warning horizon be \(H>0\). The alert is a warning for a particular onset realization when:

\[
0 < T-A \le H
\]

The *possible* warning window—there exists at least one onset consistent with the evidence for which the alert is timely—is:

\[
\boxed{A \in [L-H, U)}
\]

The *guaranteed* warning window—every onset consistent with the evidence makes the alert timely—is:

\[
\boxed{A \in [U-H, L)}
\]

The second interval is empty whenever the onset uncertainty width is greater than the warning horizon. That is a meaningful label-quality limitation, not a reason to choose the midpoint.

For alarms outside these windows:

- \(A<L\): definitely before onset, but possibly too early for the declared horizon;
- \(L\le A<U\): onset ordering is ambiguous;
- \(A\ge U\): definitely not an early warning for this event.

For an alarm definitely before onset, the guaranteed-to-possible lead-time interval is:

\[
\boxed{\text{lead}(A) \in [L-A, U-A]}
\]

Do not replace this interval with its midpoint or report its upper endpoint as a guaranteed lead time.

### 3.1 Minimum actionable lead

If a reaction needs at least \(M\) time to act, a guaranteed actionable alert must satisfy:

\[
A \le L-M
\]

Combining the minimum actionable lead with the warning horizon gives the guaranteed actionable window:

\[
\boxed{A \in [U-H, L-M]}
\]

It may be empty even when the event is definitely detected. Report separately:
1. definitely pre-onset;
2. definitely within the warning horizon;
3. guaranteed to satisfy minimum action lead;
4. only possibly timely;
5. ambiguous or definitely late.

### 3.2 Hand-calculated example

Take \(L=100\) h, \(U=102\) h, \(H=24\) h, \(M=2\) h.

| Alert time | Classification | Lead evidence |
|---:|---|---|
| 96 h | Guaranteed actionable warning | lead in [4 h, 6 h] |
| 99 h | Definitely pre-onset and within H, but not guaranteed to provide M | lead in [1 h, 3 h] |
| 101 h | Ambiguous ordering; event may already have begun | no positive lead guaranteed |
| 102 h | Definitely not an early warning | onset has occurred by U |

The possible warning window is [76 h, 102 h); the guaranteed warning window is [78 h, 100 h); the guaranteed actionable window is [78 h, 98 h]. The interval width is 2 h. If the onset interval were 30 h wide while H remained 24 h, the guaranteed warning window would be empty.

## 4. Censoring and valid exposure

Censoring must affect denominators and alert labels, not just a footnote.

### 4.1 Left-censored events

If a vehicle is first observed already abnormal, an alert within the observation record cannot be called a pre-onset warning. Keep the event in event counts and report it as left-censored. Do not calculate finite physical lead time or silently drop it from the cohort.

### 4.2 Right-censored follow-up

If no event is observed by follow-up time \(C\), retain the asset as right-censored. For a *future-event-within-H* alarm task, an alarm at time \(A\) can be scored as a fully observed non-event only when its entire future horizon was monitored:

\[
A+H \le C
\]

Thus the final H of follow-up cannot contribute fully observed event-free forecast opportunities. With gaps, compute only from contiguous verified monitoring. Keep right-censored at-risk exposure visible and separate from explicitly adjudicated negative-control exposure.

This avoids turning short follow-up into false negatives and prevents a future incident beyond the data cut from retroactively becoming a false alarm.

### 4.3 Event-associated unmatched alarms

An alarm on an asset that later has an event but cannot be matched to a possible/guaranteed warning window is not automatically false. It may be:
- definitely pre-onset but too early;
- ambiguous within the onset interval;
- definitely post-onset;
- outside the monitoring interval;
- related to a different event or fault family.

Report these categories separately. Only classify false alarms against a verified event-free opportunity with a complete future horizon or a separately adjudicated negative-control interval.

### 4.4 Monitoring gaps

To claim a miss, require continuous valid monitoring over the full *possible* warning window [L-H,U), not merely around U. Any telemetry gap, sensor invalidity, BMS-offline period, or unverified clock reset that intersects that window makes warning opportunity incomplete. A missing warning opportunity is not a model miss.

## 5. Alarm episode matching

Raw threshold-crossing windows are not equivalent to independent driver/operator alarms. Before scoring:

1. Freeze the threshold policy and alarm episode grouping rule on calibration data.
2. Convert consecutive or within-timeout threshold exceedances into one alarm episode.
3. Bind each episode to the exact vehicle, pack/configuration, BMS firmware/rules, source clock and decision time.
4. Preserve repeats; do not let repeated windows inflate event detections.
5. Use a deterministic one-to-one event/episode association. An episode must not detect two events. For overlapping possible windows, either use a frozen maximum-cardinality matching rule with stable tie-breaks or mark the cohort as requiring adjudication; never choose matches after inspecting model performance.
6. Keep alarm policy, model digest, threshold, calibration data digest and score timestamp fixed before test evaluation.

The matching contract itself is benchmarked with permutation tests and adversarial overlap fixtures.

## 6. Required benchmark outputs

All primary metrics must be reported at the event/asset level, with full denominators. Stratify by fault family, chemistry, source domain, manufacturer/data owner, vehicle/pack, firmware/rule version, operating-temperature band and evidence class where sample support permits.

| Family | Required outputs |
|---|---|
| Event detection | definite coverage, possible-only coverage, ambiguous ordering, misses, left-censored count |
| Lead | interval-valued lead, minimum guaranteed lead, P05/P50/P95 only for eligible events, onset-interval width |
| False alarm burden | episodes / 1,000 adjudicated negative-control hours; plus complete-horizon event-free forecast opportunities |
| Exposure | asset-hours, valid monitored hours, gaps, right-censored hours, excluded H-edge time |
| Alarm quality | repeats, unmatched alarms, too-early alarms, ambiguous alarms, definitely late alarms |
| Calibration | reliability, Brier/log loss where scores and labels permit; risk-coverage and abstention |
| Transfer | whole-vehicle, whole-pack, whole-station and whole-owner holdouts |
| Compute | latency per decision, peak memory, energy/CPU budget, missing-input behaviour |
| Safety decision | false-feasible count on frozen adversarial inputs; INDETERMINATE/abstention rate and rationale |

Do not collapse definite and possible warning into one headline score. Do not present overall calibration as evidence of subgroup calibration. If the event or asset denominator is too small for meaningful inference, state that explicitly and report intervals rather than rank models.

## 7. SOTA and industry comparison: use matched lanes

A fair tournament needs distinct tasks and source tiers.

### Lane A — fault classification on CH-BatteryGen

Reproduce the paper's tasks/protocols first. The ICLR 2026 abstract reports a CNN F1 of 0.9280 for LFP discharging and 0.8813 for fault grading; the registered benchmark also records the paper's NCM charging result and source-specific split details. These are paper-reported reference points, not field-warning results.

Compare random forest, SVM, LSTM and CNN paper baselines, plus boosted trees, TCN/GRU and Transformer/Mamba candidates under the *same* feature access and split. Add generated-pack-grouped holdout, topology holdout, temperature holdout and chemistry/domain transfer. Report macro-F1, per-class recall, confusion matrices, calibration, selective risk/coverage, memory and latency. Preserve the paper protocol as a reproduction lane; a stricter pack-held-out lane is a separate result.

CH-BatteryGen is mechanism-constrained/generated evidence, not 1,000 independently measured fleet vehicles. The dataset's licence and redistribution/commercial-use restrictions must be enforced in the registry and artifact handling. This lane does not establish field TR warning or physical fault-onset lead time.

### Lane B — field fault/event warning

Use Cao onboard-BMS and Shenzhen charger-side datasets as distinct source domains. The first has onboard BMS measurement semantics; the latter is station/charger-side evidence and cannot justify cell-level localization. Before any score:
- admit raw snapshots and hash every source asset;
- audit missingness, sampling, class prevalence, label semantics and asset identity;
- generate pseudonymous whole-vehicle/pack, whole-station and whole-owner split manifests;
- reserve calibration and test groups before tuning;
- report exact paper-protocol reproduction separately from zero-shot cross-domain transfer and few-shot adaptation;
- apply this interval-censoring protocol to event labels wherever actual onset is not observed.

Report definite/possible event coverage, alarm episodes per 1,000 valid hours, lead intervals, event/asset counts and abstention. Do not compare those directly with CH-BatteryGen window/segment F1.

### Lane C — SOH/RUL and transferable capability

The 2026 review finds that the target itself must be task-specific: capacity retention, resistance growth, power capability and a composite index are not interchangeable. Benchmark capacity and power capability separately, then produce an operational envelope conditional on SOC, temperature, load and future duty. Compare equivalent-circuit/Kalman, incremental-capacity/differential-voltage, boosted-tree, GRU/TCN/Mamba/Transformer and hybrid physics-informed baselines.

Require chronological prediction, whole-vehicle/pack holdout, chemistry/temperature/manufacturer holdout, early-data budgets (for example 5%, 10%, 20% of admitted history), calibrated intervals and worst-domain error. Include pack heterogeneity and sensor/clock drift. RUL must state the future duty and policy assumptions; it is not a context-free scalar.

### Lane D — battery-aware charging and fleet service

Compare immediate/FCFS, EDF, delayed/valley-fill, TOU, deterministic MILP/LP, robust optimization and receding-horizon MPC under identical arrivals, departures, pack states, EVSE limits, site import capacity and tariff. If using RL, freeze a training budget and compare against optimization oracles on held-out workloads.

Score completed duties, guaranteed departure energy, unmet kWh, queue/driver time, battery throughput/degradation proxy, peak import, demand charge, ramp, charger occupancy and compute. A battery degradation cost is admissible only when a declared model is calibrated for the relevant chemistry/temperature/SOC regime. ACN workplace sessions and NLR heavy-duty aggregate profiles remain distinct evidence domains.

## 8. How to beat SOTA without gaming the benchmark

The strongest opportunity is not merely to add a larger model. It is to improve the *decision under incomplete evidence*:

1. **State as an operating envelope, not a scalar SOH.** Estimate bounded capacity, power acceptance, resistance, thermal margin and fault hypotheses; map them into allowed/unknown operating regions by SOC, temperature and duty.
2. **Predict and abstain jointly.** A useful system knows when the evidence cannot support a high-consequence claim. Compare risk at fixed coverage and coverage at fixed risk.
3. **Active diagnostic sensing / value of information.** When ambiguous, select the next lowest-cost measurement (rest interval, controlled pulse, additional temperature/voltage sample, independent inspection) expected to resolve the decision. Record the measurement cost, delay and whether it actually changed the disposition.
4. **Cross-fidelity transfer with explicit promotion.** Synthetic → lab cell → module/pack → vehicle shadow → fleet operations. Each promotion requires a new held-out evidence cut and calibration; a generated-data win does not promote itself to field evidence.
5. **Decision receipts and independent qualification.** Bind the exact subject/configuration, evidence snapshot, trust state, model/calibration, constraints, policy, recommendation and replay digest. Separate model output from authorized action and post-action verification.
6. **Measure usefulness per cost.** Report earlier guaranteed warning, reduced false-alarm burden, fewer missed duties, lower grid peak and better service—while also reporting sensing cost, compute, downtime and abstention. Do not scalarize away hard safety constraints.

## 9. Prospective targets and release gates

These are engineering objectives, not achieved results or a universal industry standard.

- **Zero false-feasible outcomes** in the frozen adversarial reserve/authority/provenance suite; any occurrence blocks promotion.
- **Calibration:** nominal 90% intervals should demonstrate coverage consistent with 90% on held-out test groups, with confidence intervals and subgroup results; no overall average may hide material high-risk undercoverage.
- **Transfer:** outperform the strongest matched baseline on at least two predeclared task-relevant axes while no worse on critical-event misses, false-alarm burden, calibration, or abstention. Publish the matched budget and paired uncertainty intervals.
- **Classification:** reproduce paper numbers on its paper split before claiming an improvement; then beat them on stricter group-held-out data without class leakage.
- **Event warning:** report the full definite/possible coverage-versus-false-alarm frontier at preregistered operating points. No single “best F1” is sufficient for safety claims.
- **Replay:** deterministic input, split, model, threshold, report and artifact digests; independent challenger recomputation.
- **Physical promotion:** shadow-mode replay and independent adjudication first; HIL and physical pack/vehicle testing only under a separately approved safety plan. Software CI is never physical qualification.

A target such as “less than one false alarm per 1,000 hours” is not universal. It must be selected by the vehicle/operator hazard analysis and compared at matched operating points. Public papers do not provide a single standardized cross-OEM fleet benchmark from which to declare universal industry SOTA.

## 10. Implementation sequence

1. Keep #7853 v0.1 point-time evaluation as the reproduction baseline.
2. Admit and hash source snapshots through #7840; then generate real parser-derived group splits through #7845. Synthetic manifest fixtures do not count as real split evidence.
3. Implement this event-censoring v0.2 contract as a separate Rust module/CLI, using integer milliseconds, checked arithmetic and typed exact/interval/left/right-censor states.
4. Add known-answer fixtures and negative controls for interval boundaries, wide intervals, clock resets, gaps, overlaps, event matching and right-censor horizon edges.
5. Build an independent challenger evaluator; freeze test manifests and thresholds before model runs.
6. Run paper baseline reproductions; then strict vehicle/pack/station/owner holdouts and cross-fidelity transfer.
7. Only after field data is admitted, run Symthaea models and compare evidence-qualified operating decisions. No generated benchmark result is promoted to physical warning performance.

## 11. Claim ceiling

This protocol defines a more honest event-warning benchmark. It does not establish true physical onset for existing datasets, certify a BMS, qualify a safety reaction, prove a model is superior to commercial OEM systems, or authorize vehicle actuation. The initial implementation should remain advisory/shadow-only and fail closed when provenance, event interval, exposure or model applicability is indeterminate.
