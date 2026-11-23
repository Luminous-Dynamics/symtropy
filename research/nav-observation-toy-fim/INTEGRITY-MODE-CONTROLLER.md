# Evidence-Gated Integrity Mode Controller

## Purpose

The controller converts the existing integrity evidence stack into an explicit operating mode. It is designed to sit downstream of the estimator, fault-hypothesis monitor, independent posterior audit, conformal calibration and protection-level calculation.

Its central policy is asymmetric:

- **Downgrade immediately** when any required integrity gate fails.
- **Recover slowly** only after a configured number of consecutive fresh evidence epochs with distinct observation-set sequences, distinct receipt sequences/digests and distinct dependency groups.
- Never let a smooth numerical protection-level controller override an incomplete fault inventory, an unbounded hard-fault envelope, a failed independent posterior audit, stale calibration or a changed predictor/score contract.

This is not a PID controller, a PHMI theorem, a motion-planning policy or an actuator authorization mechanism. It is a deterministic qualification state machine.

## Relationship to the 2026 control-theoretic protection-level literature

Maharmeh, Resende and Nashashibi, *Integrity as a Control Problem: Smooth and Adaptive Protection Levels for Multi-Modal Localization*, Sensors 2026, 26(16), 5140, DOI: https://doi.org/10.3390/s26165140, reformulates protection-level generation as a closed-loop control problem. Its instantaneous error rate combines inertial noise, disagreement between filter-based and dead-reckoned displacement, and LiDAR scan-map registration quality weighted by geometric sensitivity. A saturation-controlled setpoint and entropy-scheduled PID controller produce a smooth protection level. Its update law expands a bound faster than it contracts it.

The paper reports 0.0% empirical integrity risk on its medium-urban and low-urban UrbanNav sequences and 2.3% in deep-urban degradation. Its benchmark uses a Student-t protection-level comparator and reports a target integrity risk of 10^-3. The paper also explicitly says that availability and continuity were not addressed, and its data-availability statement says no new data were created or analyzed.

This is a valuable research direction, but a smooth numerical bound and an evidence/authority state are different objects. A protection-level controller can react to a growing error-rate proxy, but it should not be the only mechanism deciding whether a source inventory is complete, whether a hard-fault set is bounded, whether an independent posterior audit passed, or whether a calibration still matches the predictor.

The Symtropy controller therefore treats any smooth/adaptive PL as an upstream input. It adds evidence-gated modes and recovery quorum rather than replacing the published controller with an unvalidated PID variant.

## Modes and exact meaning

| Mode | Meaning | Normal operation qualified? |
|---|---|---:|
| UNKNOWN | Source, fault, posterior, calibration, model binding or freshness evidence is incomplete, invalid or contradictory | No |
| UNAVAILABLE | No usable localization solution is available | No |
| DEGRADED | Evidence is supported but the protection level exceeds the alert limit, or recovery evidence has not yet met its quorum | No |
| NORMAL | All evidence gates pass, the protection-level upper bound is within the configured alert limit, and recovery requirements are satisfied | Yes, as a qualification state only |

The output field `qualified_for_normal_operation` means only that the localization evidence stack met this mode policy. It does not authorize motion, payment, safety actions or actuators.

## Evidence gates

A NORMAL-eligible frame must satisfy all of the following:

1. Localization solution is available.
2. Source evidence is complete.
3. Fault-hypothesis inventory is complete.
4. The deterministic hard-fault envelope is bounded.
5. An independent posterior audit passed.
6. The evidence is fresh.
7. Conformal/statistical calibration is current.
8. Predictor digest matches calibration.
9. Score-definition digest matches calibration.
10. Protection-level upper bound is finite, nonnegative and no greater than the frozen alert limit.
11. Observation-set sequence, evidence-receipt sequence and receipt digest are present and fresh.

Failure to meet a support gate immediately moves the controller to UNKNOWN. Missing the solution moves it to UNAVAILABLE. A supported estimate whose protection level exceeds the alert limit moves it to DEGRADED.

## Downgrade and recovery semantics

The mode is initialized as UNKNOWN.

Any failed gate clears the recovery streak and leaves NORMAL immediately. A new low protection level alone is not sufficient for recovery.

From any non-NORMAL state, the controller requires a configured sequence of qualifying recovery frames. Every frame in the streak must have:

- a strictly increasing evidence epoch;
- a strictly increasing observation-set sequence;
- a strictly increasing evidence-receipt sequence;
- a nonzero receipt digest that binds the epoch and receipt sequence;
- a dependency-group identity not already used in the current recovery streak;
- a fresh, source-complete evidence frame passing all required audits.

A repeated dependency group resets the recovery streak. A repeated or reordered epoch, observation sequence or receipt sequence is rejected and leaves the controller UNKNOWN. A fresh evidence receipt is required even when all numerical PL values look safe.

The default test policy uses three consecutive independent recovery epochs. Production policy values must be frozen from the application hazard analysis and evaluated for recovery latency and availability trade-offs; three is not a universal safety standard.

## Canonical receipt contract

The caller must compute the evidence receipt digest over a canonical, source-complete record containing at least:

- evidence epoch ID;
- observation-set sequence ID;
- receipt sequence ID;
- exact source manifest and observation identities;
- fault-inventory commitment and enumeration-completeness result;
- hard-fault envelope result and configuration commitment;
- independent posterior audit receipt;
- predictor and score-definition commitments;
- protection-level value, alert limit and units;
- freshness/clock domain and evidence timestamps;
- policy/configuration version.

The module verifies nonzero digests and monotonic source-ledger sequence IDs. It does not hash or authenticate the source bytes itself, verify signatures, validate a clock authority, or prove that a dependency group is statistically independent. Those must be supplied by separately qualified evidence producers.

## Falsification and metamorphic test requirements

- Startup never enters NORMAL from one safe-looking frame.
- Any missing source, incomplete inventory, unbounded hard-fault envelope, failed independent audit, stale evidence, stale calibration, predictor mismatch or score-definition mismatch immediately exits NORMAL.
- A missing solution produces UNAVAILABLE rather than silently retaining the last NORMAL state.
- A protection level just above the alert limit immediately enters DEGRADED.
- A lower PL alone never restores NORMAL.
- Repeated dependency groups cannot advance a recovery streak.
- Replayed observation-set and evidence-receipt sequences cannot restore NORMAL.
- Out-of-order epochs cannot roll back the replay watermark.
- Invalid policy, non-finite PL and missing commitments fail closed.
- A numerical PL smoother or PID may change the PL input but cannot override these gates.

## Benchmark against current practice

Compare at least four variants with identical source streams and fault injections:

1. covariance/Student-t protection-level baseline;
2. smooth adaptive PL controller inspired by the 2026 Sensors paper, with its source-specific input features clearly matched;
3. Symtropy's existing adaptive PL calculation with ordinary threshold-based mode selection;
4. Symtropy adaptive PL plus this evidence-gated mode controller.

Report separate metrics:

**Integrity:** empirical error-over-PL risk; false-safe transitions; missed known faults; time to downgrade; replay acceptance; unsupported NORMAL exposure.

**Bound quality:** PL coverage, mean/median/P95/P99 PL, excess width over true error, saturation time and bound contraction delay.

**Availability/continuity:** fraction of time NORMAL, DEGRADED, UNKNOWN and UNAVAILABLE; transition count; false degradation; recovery time; continuity loss over fixed mission windows.

**Robustness:** source truncation, missing/late packets, common clock fault, shared-anchor calibration fault, stale predictor/calibration, incorrect posterior, incomplete hypothesis inventory, unbounded set, observation replay, receipt replay, sequence rollback and domain shift.

**Resources:** CPU p50/p95/p99, peak memory, state size and update time on named hardware.

No single weighted score. Compare accuracy, integrity, bound quality, availability and resources on a Pareto frontier. Report the exact alert limit, target risk, decision interval, source privileges, data split, fault prevalence and truth qualification.

The 2026 paper's reported UrbanNav risk results are literature figures, not results from this code. A UWB controller must be evaluated on UWB source-complete episodes; LiDAR/IMU/GNSS benchmark numbers are not directly transferable.

## Current implementation

Implemented in `integrity_mode_controller.rs`:

- explicit UNKNOWN / UNAVAILABLE / DEGRADED / NORMAL modes;
- immediate downgrade on a failed evidence gate;
- configured multi-epoch recovery quorum;
- distinct dependency groups during recovery;
- monotonic evidence epoch, observation-set sequence and receipt sequence checks;
- source receipt digest requirement;
- protection-level alert limit;
- predictor, score-definition, calibration and posterior-audit gates;
- replay, sequence rollback, stale-evidence and common-cause fault controls;
- tests for startup, downgrade, recovery, duplicate evidence, replay and invalid values.

Not implemented or demonstrated:

- cryptographic receipt calculation/signature verification;
- source producer or trusted-clock qualification;
- proof of statistical independence between dependency groups;
- integration with a real UWB runtime or motion planner;
- formal verification of the complete transition system;
- benchmark results, physical availability or PHMI qualification.

The module is research-only until the evidence producers and policy parameters are independently qualified.
