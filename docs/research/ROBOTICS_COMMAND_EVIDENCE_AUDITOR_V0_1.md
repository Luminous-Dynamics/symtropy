# Command Evidence Auditor v0.1

**Status:** pure Rust trace-consistency verifier with adversarial unit tests authored; no production event collector, authenticated log, ROS 2 adapter, or physical robot integration is included.

## Paradigm shift: verify the event history, not the controller's summary

The differential-drive adapter defines what a native controller must enforce. This module checks whether the trace actually supplied is consistent with that contract, using independently supplied expected request identities. A success-shaped receipt alone cannot establish application: the trace must contain the explicit `Applied` event from the platform boundary that owns that evidence.

## Input model

Each expected identity freezes request ID, session ID, sequence, controller epoch, monotonic deadline, and clock-domain ID. Every event repeats that identity and records its monotonic observation time. This lets the auditor reject identity drift across stages, clock-domain substitution, time regression, missing traces, and unexpected requests.

Allowed positive path:

`Proposed → RuntimeAdmitted → NativeAccepted(epoch match, before deadline) → Applied(epoch match, before deadline) → OutcomeRecorded(Succeeded|Failed|TimedOut)`

Safe alternatives terminate with an explicit stage-specific rejection or cancellation followed by a compatible terminal outcome. Rejections after a deadline can be safe; acceptances/applications at or after the deadline cannot.

## Fail-closed rules

- Native acceptance or application under the wrong epoch fails independently.
- Runtime admission, native acceptance, or application at/after the deadline fails.
- Every expected command must have a trace and exactly one terminal outcome.
- Missing, extra, identity-mutated, out-of-order, or terminal-after-terminal evidence fails auditing.
- Timestamp order is checked within each request. All timestamps and the deadline must share the identity's declared monotonic clock domain.

## What this does not prove

This is a **consistency auditor**, not yet a cryptographic proof of provenance. It cannot establish that logs are truthful, correctly signed, durably retained, or emitted from the actual hardware boundary. Production qualification must define canonical payload/configuration digest generation, add authenticated event capture, append-only/digest-bound storage, an independent native application witness, restart/rollback fault injection, and hardware-in-the-loop trials. Native controller admission is not physical application evidence; missing physical application evidence must remain explicitly unobserved.

## Industry comparison and benchmark plan

ROS 2 Control exposes controller/hardware lifecycle and error/fallback behavior and warns against non-real-time-safe lifecycle queries in its control loop. The auditor therefore runs out-of-band and should consume captured evidence rather than insert blocking work into the real-time path. NIST's response-robot program emphasizes repeatable elemental test methods and task-specific quantitative thresholds. This verifier complements those capability tests; it does not replace robot task benchmarks or safety qualification.

- ROS 2 Control controller manager: https://control.ros.org/kilted/doc/ros2_control/controller_manager/doc/userdoc.html
- NIST Performance of Emergency Response Robots: https://www.nist.gov/programs-projects/performance-emergency-response-robots

## Evaluation protocol

Freeze an adversarial mutation matrix before scoring:

- Valid complete success, native rejection, application rejection, cancellation, and safe deadline rejection traces.
- Request/session/sequence/controller identity substitution.
- Controller epoch, accepted epoch, applied epoch, deadline, clock-domain, command-payload digest, and controller-configuration digest mutation.
- Timestamp regression, reordered stages, duplicate proposal, duplicate terminal, outcome/disposition contradiction, missing event, missing terminal, and unexpected request.
- Empty expected set and missing entire expected request identities.

Target 100% detection of seeded hard-gate mutations and 0 false rejection of valid complete traces. Report denominators by mutation class, not just a single aggregate. Scale the verifier over 1,000; 10,000; 100,000; and 1,000,000 events, reporting events/second, p50/p95 audit latency, peak memory, and bytes per event. The auditor is out-of-band: its added latency to the real-time controller loop must be exactly zero by architecture, not a best-effort target.

## Industry benchmark positioning

RoboDojo reports 42 simulation tasks and 18 real-world tasks across three embodiments, with generalization, memory, precision, long-horizon, and open-vocabulary capability dimensions. Its reproducible, seed-controlled evaluation is a useful model for task-level benchmarking; its manipulation success scores are not directly comparable to this mobile-base command-integrity auditor. Freeze benchmark assets, evaluator versions, and seeds when integrating cross-benchmark task evaluation.

NIST's response-robot program develops repeatable elemental capability tests and leaves mission-specific pass thresholds to the deploying community. Pair those capability measurements with this command-lifecycle audit, rather than collapsing both into one reward.

## Acceptance target

On a frozen conformance corpus, require 100% detection of injected identity, epoch, deadline, order, missing-trace, missing-terminal, and contradictory-outcome faults, with zero false rejection of valid complete traces. This is a test target only; it is not yet a measured result. The next step is to bind the auditor to the campaign runner and emit a digest-bound receipt for each audited run.
