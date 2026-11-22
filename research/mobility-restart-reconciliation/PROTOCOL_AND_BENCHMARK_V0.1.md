# Mobility Restart/Reconciliation Contract and Benchmark v0.1

**Status:** bounded reference contract and research proposal  
**Date:** 22 November 2026  
**License:** AGPL-3.0-or-later

## 1. Purpose and claim boundary

A restarted process is not a restored physical world. A robot arm may have moved, an AMR may have crossed a zone boundary, a gripper may have released an object, or an external provider may have accepted a command even when the process crashed before recording the acknowledgment.

This package defines a small cross-owner consistency check for recovery evidence after a new process/boot incarnation. It deliberately does not implement a durable journal, outbox, provider query protocol, authority evaluator, observer, controller or native sink.

It consumes claims from existing owners and checks identity, epoch and outcome agreement. The production integration must still authenticate those claims and call the existing native application boundary for any new action.

## 2. Paradigm shift: four facts, four owners

Keep four different truths separate:

| Plane | Question | Owner |
|---|---|---|
| Persistence | Did the local durable attempt/receipt record commit? | Mycelix integration-runtime durability and effect owners |
| Physical effect | Did this exact logical effect and attempt happen? | Device/provider and independent effect observer |
| Mission resolution | Is the exact M1 effect predicate satisfied, absent, partial, unknown or disputed? | M1 + qualified independent observer/effect owners |
| Current authority | Is a new action valid now under current mission, platform, mode, ODD, time/boot and controller epoch? | M1/M2, authority boundary and controller queue |

These implications are forbidden:

- local Committed does not prove the device/provider performed the effect;
- local DefinitelyNotCommitted does not prove the device/provider did not perform it;
- native controller acceptance does not prove physical completion;
- a recovery receipt does not reconstruct authority;
- restart, timeout, lost acknowledgment or process memory loss does not authorize retry;
- a no-effect observation does not mint new M1 authority or a new M2-valid ticket.

The package only checks cross-owner consistency. It does not verify signatures, source truth, completeness of observation, or the declared independence of failure domains. Distinct failure-domain identifiers are a necessary shape check, not proof of statistical or physical independence.

## 3. Recovery protocol

The intended sequence is:

```text
old process / boot epoch
  -> exact logical effect + attempt identity
  -> durable attempt owner records its own lifecycle
  -> native application boundary consumes the exact ticket
  -> controller/device may or may not produce the physical effect
  -> process/host/controller restarts
  -> old queue epoch is fenced; old queued work is discarded
  -> current owners re-observe persistence and physical effect independently
  -> M1 resolves the exact old effect attempt
  -> only then may a new ticket/new attempt be assessed
  -> ordinary native application boundary rechecks it again
```

The cross-owner evidence must bind:

- stable logical-effect identity, separate from attempt identity;
- exact old attempt identity;
- old and current boot-epoch identifiers;
- strictly increasing controller epoch;
- queue-fence receipt binding the old/new boot and controller epochs;
- persistence verdict from the durable store owner;
- effect evidence for the exact logical effect and exact old attempt;
- observer profile and separately declared observer/effect failure domains;
- matching M1 resolution for that exact old attempt;
- when the prior attempt is authoritatively NoEffect, a one-shot recovery permit bound to the failed attempt, logical effect, new attempt, prior authority epoch and next binding commitment;
- a new ticket bound to the current boot/controller epoch, with current M1 authority, M2 applicability, and equal non-zero ticket/live commitments.

### Decision semantics

| Exact old-attempt evidence | Decision | What it does not mean |
|---|---|---|
| Missing or Unknown effect | Hold / reconcile only | No retry, no inferred no-effect |
| Applied + matching M1 effect-predicate resolution | Effect predicate observed | Not task success, not permission for another effect |
| NoEffect + matching M1 NoEffect resolution + recovery permit + fresh ticket + new epochs + current owner claims | Fresh-attempt candidate | Not a dispatch capability; native gate must revalidate |
| Partial | Component quarantine / hold | No blind replay or whole-task rollback |
| Disputed / contradictory | Integrity hold | No automatic repair or winner selection |
| Any effect with IndeterminateCommit persistence | Hold | Do not bypass uncertain local durable state |

The bounded Rust API returns only Hold, EffectPredicateObserved or FreshAttemptCandidate. In particular, it has no native-sink function. FreshAttemptCandidate is a precondition assessment, not authorization. The normal application adapter must revalidate M1/M2 and the native queue fence at the real application seam.

### Why local commit classification is a separate axis

Mycelix's shared commit vocabulary remains Committed, DefinitelyNotCommitted and IndeterminateCommit. This package does not reproduce its store transaction classifier or re-read procedure. It uses that verdict only as an orthogonal condition: IndeterminateCommit blocks; Committed and DefinitelyNotCommitted do not resolve the physical effect. The independent observer/effect owner must still classify the exact action attempt.

### Composition, not duplication

- Symtropy #6498/#6503 own mission/action semantics and ticket applicability.
- Symtropy #6669 owns the in-process authority-linearized one-shot mutation boundary.
- Symtropy #2883 owns the controller epoch/queue fence reference.
- Symtropy #6743 owns fail-closed dependency-component quarantine.
- Symtropy #6749 composes those existing references at one synchronous callback.
- Mycelix #1535/#780 provide durable effect/outbox and commit-classification ancestry; AGENT-009 #803 tracks the exact integration with current agent authority and provider observations.
- This package owns only the v0.1 cross-owner restart-evidence shape check.

## 4. Crash-cut and adversarial schedule

The conformance campaign needs one schedule for each actual boundary cut, not merely many controller ticks:

1. crash before durable attempt reservation;
2. crash after durable reservation, before dispatch-start persistence;
3. crash after dispatch-start persistence, before native acceptance;
4. crash after native acceptance, before receipt/acknowledgment persistence;
5. possible physical effect followed by lost acknowledgment;
6. crash during effect-observation persistence;
7. restart with old queue work still present;
8. new queue epoch established, but effect outcome still Unknown;
9. late independent effect evidence arrives after restart;
10. observer reports Partial or Disputed;
11. duplicated worker/observer delivery presents the same receipt twice;
12. recovery permit is replayed, changed or bound to a different new attempt;
13. old ticket is replayed under the new boot ID;
14. live authority, ODD, mode, configuration or navigation context changes after the recovery assessment but before the native callback;
15. downstream driver/firmware has a second queue that the adapter cannot fence.

The last two are native-integration obligations, not solvable by this pure reference crate. The full application adapter must revalidate at the actual sink; it must not forward into an unfenced downstream queue.

## 5. Restart-Robust Verified Recovery (RRVR) benchmark

### Matched arms

Use the same robot, policy, controller, action traces, resources, sensors, environment, fault schedule and evaluator:

A. No-crash baseline, to establish task and effect-observation behavior.
B. Restart with no recovery evidence, expected to hold.
C. Restart with local persistence verdict only, expected to hold if physical effect is unresolved.
D. Restart with physical observation but no current M1/M2 authority, expected not to admit a new action.
E. Full owner composition: queue fence + persistence classification + independent effect observation + M1 resolution + fresh M1/M2 ticket + native boundary.
F. Naive restart/resubmit, diagnostic only in a non-actuating mock; never an operational or HIL control.

### Non-compensatory hard metrics

- duplicate physical effects per logical-effect identity: **0**;
- old-boot/old-epoch queue replay reaching the native sink: **0**;
- Unknown, Partial or Disputed outcomes released into a new mutation: **0**;
- false Applied/NoEffect classifications against the independent evaluator: **0**;
- recovery-permit replay or next-attempt substitution: **0**;
- declared crash-cut coverage: **100%**;
- valid positive-control admission: report with confidence bounds, initially target **≥99.9%** for the explicitly supported profile;
- every reported disposition linked to the exact source/owner/evaluator revisions and evidence references.

Do not combine a safety violation with speed or success in a weighted score.

### Latency and work-conservation metrics

Report p50/p95/p99 for:

- restart detection to old-queue fence;
- restart detection to application-path hold;
- restart to durable-attempt re-observation;
- restart to independent effect classification;
- late evidence arrival to M1 resolution;
- qualified NoEffect to FreshAttemptCandidate;
- false-hold duration and operator intervention;
- queued work cancelled by the epoch change;
- policy/task time, controller time, observer time and recovery overhead separately.

The p99 fence deadline must be preregistered against the actual platform's supervisory/control deadline. Do not invent a universal millisecond limit across AMRs, manipulators and other robots.

### Statistics and reproducibility

Use independent scenario schedules as experimental units, not video frames or control ticks. Pair schedules across arms, cluster uncertainty by task/robot/site, report all denominators and exclusions, and preserve raw event streams. Include evaluator-held crash cutpoints and mutation cases. Freeze code SHA, owner profile revisions, fault injector, runtime, firmware, scenario, seeds and evaluator before confirmatory runs.

A first software-only campaign may target 100,000 deterministic injected cut schedules, with zero invalid transitions. That is a research target, not an executed result or a certification threshold. Physical claims require a separately reviewed simulator, HIL and hardware campaign.

## 6. Industry and SOTA positioning

This benchmark is complementary to task-policy leaderboards rather than a directly comparable score.

- **NIST Agility Performance of Robotic Systems** is a strong measurement-science comparator: it targets performance under unexpected events, obstacles and failures and includes metrics, information models, datasets, a combined virtual/real testbed and a sim-to-real pipeline. RRVR adds the missing authority/effect continuity question at restart boundaries.
- **NIST Mobility Performance** studies dynamic, unstructured environments, localization and integrated learning capabilities, including forced delocalization. Use its test-method mindset for environment and pose disturbance, then add crash-cut and post-restart evidence checks.
- **NIST Measurement Science for Robotics and Autonomous Systems** emphasizes black-box decision-quality assessment from observable behavior. RRVR should likewise publish an observable event/evidence schema so an independent evaluator can score recovery without reading the planner's private internal state.
- **SafeVLA-Bench** separates task success, safety rate and successful-but-unsafe episodes. Reuse the same fixed policy traces and safety predicates, then add restart injection; do not claim a higher SafeVLA score from software-only gate tests.
- **ManiGuard-Bench** contributes specification-grounded, ID/OOD scenarios and physical-robot evaluation. Reuse its safety predicates as an independent effect/hazard layer; the restart contract does not replace physical hazard validation.
- A recent 2026 survey of long-horizon embodied manipulation emphasizes that retry/replanning requires observed state-transition evidence; textual reflection or plan backtracking cannot restore an irreversible physical state. RRVR turns that observation into crash/restart-specific conformance cases.

References:

- NIST Agility Performance: https://www.nist.gov/programs-projects/agility-performance-robotic-systems
- NIST Mobility Performance: https://www.nist.gov/programs-projects/mobility-performance-robotic-systems
- NIST measurement science for robotics and autonomous systems: https://www.nist.gov/programs-projects/measurement-science-robotics-and-autonomous-systems-program
- Long-horizon embodied manipulation survey (2026): https://link.springer.com/article/10.1007/s44336-026-00048-x
- SafeVLA-Bench: https://safevla.org/leaderboard.html
- ManiGuard: https://nu-ideas-lab.github.io/ManiGuard/

## 7. Qualification ladder and claim ceiling

1. v0.1 Rust reference contract, public schema and decision matrix.
2. Exact Mycelix/M1 owner adapter using their existing store/effect observation semantics.
3. Restart-capable native adapter with durable pending-attempt rehydration and old-queue invalidation.
4. Simulator crash-cut campaign with fixed policy traces and independent effect oracle.
5. HIL campaign on a declared robot/controller/firmware stack.
6. Physical fault-injection campaign with safety professional review and independent observers.
7. Domain-specific standard-to-test crosswalk and conformity assessment by qualified parties.

A v0.1 PASS establishes only bounded consistency checking over supplied owner claims. It does not establish durable crash consistency, cryptographic authenticity, complete observation, exactly-once physical effect, provider honesty, simulator fidelity, HIL, real-time bounds, physical safety, certification or regulatory conformity.
