# Construction observation coverage benchmark v0.1

Issue: [#3558](https://github.com/Luminous-Dynamics/symtropy/issues/3558). The Rust reference gate is in `crates/apps/symtropy-construction-observation-gate`.

## Scope separation

- This package evaluates whether declared context observations are sufficient, fresh and qualified to say a work-zone cell is clear.
- The worksite epoch oracle (#3542) consumes changes, invalidates current permission, and requires timebase-bound requalification.
- The physical validation protocol (#3505/#3506) defines independent metrology, HIL and field trial methods.
- Symthaea #4918 remains the authority for claim-relative independence and common-mode dependency lineage.

No duplicate event log, epoch state machine or independence graph is introduced.

## Gate states

| State | Conditions | Required response |
|---|---|---|
| `verified_clear` | Every required source is present, authenticated upstream, fresh, on the expected timebase, qualified, within clock/measurement uncertainty, calibrated where required, independent where required, and every required cell is explicitly observed clear | May be used as one input to the higher-level epoch/authorization system; this state alone does not authorize motion |
| `occupied_or_changed` | At least one qualified observation reports a critical occupancy, obstruction or context change | Hold and invalidate the affected epoch/task permission |
| `unknown_or_unobserved` | Source/cell missing, stale, degraded, occluded, unauthenticated, uncalibrated, wrong timebase, or outside uncertainty limits | Hold; never promote unknown to clear |
| `conflicting` | Qualified sources disagree on a cell, or a source sequence rolls back / same sequence carries divergent content | Hold and require external reconciliation/re-baselining |

Only `verified_clear` is non-holding. The gate does not create an epoch or drive an actuator.

## Observation contract

Each source policy declares its source identifier, task-zone cells, maximum observation age, maximum clock uncertainty, maximum measurement uncertainty, calibration requirement and claim-relative independence requirement.

Each source policy pins a trusted baseline sequence/digest. An observation includes monotonic sequence, evidence digest and predecessor digest, task scope, timebase ID, observed monotonic time, clock and measurement uncertainty, sensor health, calibration state, upstream authentication assertion, independence assessment/reference, and explicit `clear_cells`, `changed_cells` and `occluded_cells`.

An unlisted cell is unobserved. Empty detections do not constitute a clear reading. Each source sequence is predecessor-linked to the trusted baseline and previous accepted observation. A sequence gap is latched unknown; rollback, predecessor mismatch or same-sequence content divergence is latched conflicting. A midstream restart is allowed only from an externally established baseline checkpoint and a new gate instance.

## Test corpus and current software verification targets

The Rust package has 25 unit tests covering complete clear coverage, missing source/cell, occlusion, stale/future timestamps, timebase mismatch, clock and measurement uncertainty, degraded sensor, calibration invalidity, missing independence, upstream authentication, positive changes despite partial coverage, cross-source conflict, duplicate idempotence, digest divergence, predecessor-chain mismatch, trusted midstream re-baseline, sequence rollback/gap, malformed digests and out-of-scope cells.

These tests exercise software-declared inputs only. A future physical benchmark must independently stage zone changes and occupancy/obstruction truth, capture source clocks independently, and include sensor blind spots, dust/rain/glare, network delays, frozen feeds, clock resets and overlapping trades.

## Metrics

- critical cell coverage rate: measured / required, with occluded, stale, unknown and excluded cells separately reported;
- missed critical changes / independently staged changes;
- false clear / independently occupied-or-changed events;
- detection-to-epoch-commit latency distribution and instrumentation uncertainty;
- source outage/frozen-feed detection delay and recovery-to-qualified time;
- false hold frequency/duration per robot-hour and per context transition;
- accepted output and requalification cost on matched task/design/ODD/acceptance criteria, reported separately from safety.

Use separate challenge and operational samples. One independently staged change is one event; sensor frames, heartbeats and repeated messages from the same event do not increase the denominator. Report per-stratum numerators/denominators and exact confidence bounds.

## Trust and privacy limits

The crate does not verify signatures, actor identity, ground truth, calibration certificates or independence lineage. `authenticated_by_upstream` and `IndependentUnderProfile` are inputs from their owning services; neither is proven by this crate. It does not identify workers or infer intent/affect; cells represent anonymous work-zone geometry only.

A software test pass is not a field detection rate, a functional-safety claim, certification or an operational permit. Physical use requires independent evidence and claim-specific admissibility.
