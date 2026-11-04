# Construction Robot End-to-End Assurance Chain v0.1

**Issue:** [#3566](https://github.com/Luminous-Dynamics/symtropy/issues/3566)  
**Status:** conformance scenario contract; synthetic only

## Purpose

This benchmark tests whether the separately implemented construction-robot gates compose safely. It does not duplicate their internal logic.

`approved model/export -> layout input admission -> observation coverage -> worksite epoch -> qualification lease -> command authorization -> physical outcome receipt -> claim-specific admissibility`

A task must not become executable merely because each component can produce a locally valid result. The exact outputs, artifact digests, context epoch, configuration, timebase, qualification generation and lease must survive every interface boundary.

## Component ownership

- #3561 / PR #3564: source model, export, frame/transform, inventory and survey checkpoint input admission.
- #3558 / PR #3558: source observation freshness/coverage and verified-clear vs unknown/changed/conflicting.
- #3542 / PR #3542: epoch sequencing, conflict/rollback, invalidation, lease, safe hold, requalification and command binding.
- #3531 / PR #3531: claim-specific evidence admission.
- #3522 and #3505/#3506: physical trial receipt and physical protocol/metrology.
- #3523 / PR #3524: task-matched layout robot comparison and common referee.
- Symthaea #4918: evidence dependency lineage and claim-relative independence.

This benchmark is the boundary conformance owner. It must not grow a second coordinate engine, survey adjustment system, event ledger, observation fusion pipeline, epoch oracle, robot controller, physical metrology system or evidence-dependency graph.

## Frozen scenario contract

`assurance-chain-scenario-v0.1.schema.json` defines the challenge packet. Each stage has:
- unique stage identity, sequence and explicit predecessor stage IDs;
- consumed artifact identities and digests;
- produced artifact identities/digests and evidence scope;
- expected claim-relative disposition and deterministic reason codes.

Each attempted command binds the current epoch, robot configuration, task scope, timebase, qualification generation and lease validity. The expected outcome is explicit. Claim expectations bind evidence origin to intended scope. Statistical units identify site/shift/robot clusters and distinguish independently staged events from repeated sensor frames or controller ticks.

## Mandatory cross-stage properties

1. An inadmissible/indeterminate layout input cannot be promoted to a task authorization.
2. `unknown_or_unobserved`, `occupied_or_changed`, or `conflicting` observation state blocks command authorization and invalidates the affected epoch/qualification.
3. A newly accepted epoch revokes the prior qualification generation before the next command.
4. A stale epoch, old configuration digest, wrong task scope, wrong timebase/boot identity, stale clock, or expired lease cannot authorize.
5. Reconnect, reboot, sensor recovery and operator acknowledgment never restore authority by themselves.
6. An evidence receipt must refer to the exact scenario/task/configuration and independent measurement artifacts; physical evidence scope cannot be inferred from a software stage.
7. A field claim cannot be admitted from synthetic, simulated, bench or HIL evidence, irrespective of later-stage success.
8. A passing productivity claim cannot alter an unsafe, failed, indeterminate or unevaluated safety claim.
9. No global PASS, confidence score or weighted total is emitted. Stage decisions and claim decisions remain separate.
10. A failure blocks only dependent tasks/claims, but cannot be silently discarded from the chain.

## Required adversarial scenario families

- Design/export/frame admission is wrong although the robot's localization/printing accuracy is nominal.
- A previously admitted design or site/work-package revision becomes stale before execution.
- An observation region becomes occluded or unobserved while no positive hazard detection is emitted.
- A source sequence gap, replay, rollback or same-sequence divergent digest occurs.
- A trade enters or an exclusion zone moves after qualification but before command.
- The epoch is advanced while a cached command still carries the previous epoch/qualification generation.
- Payload, brake, tool, mode, sensor, calibration, software or firmware configuration changes after qualification.
- Communication loss, reconnect, reboot, timebase/boot change or stale ACTIVE-cache resurrection.
- The source is current but clock uncertainty exceeds policy.
- The lease expires exactly at the command boundary; test one tick before, at and after expiry.
- Repeated messages/frames from one staged event are incorrectly counted as independent encounters.
- One genuine bench result is used to support an unsupported field claim.
- A claim is not evaluated while an independent claim in the same packet is supported; unsupported claims remain unpromoted.

## Metrics

**Safety gates (non-compensable):** false-safe authorizations after upstream failure/unknown; stale/old-epoch/old-configuration/expired-lease command leakage; unqualified restart; stage-to-stage digest/sequence mismatches. Initial software-contract target: zero in the frozen challenge corpus.

**End-to-end latency:** independently staged change to observation detection; detection to epoch/qualification invalidation; invalidation to command inhibit; hazard to safe-state completion; context reconciliation to requalification/restart. Report p50/p95/p99/max, clock uncertainty and physical measurement uncertainty. Sample p99 is not a worst-case guarantee.

**Availability/productivity:** safe-hold frequency/duration by cause, requalification time, accepted output per robot-hour and crew-hour, and total workflow labor (BIM/VDC preparation, trade coordination, survey/control, setup, operator/spotter, print, QA, rework and recovery). Report separately; no trade-off score.

## Statistical protocol

Pre-register the independent unit for each endpoint. One independently staged worksite change is one unit; repeated frames, heartbeats, and controller cycles from the same event do not increase (n). Preserve site/shift/task/zone/robot/configuration/source cluster IDs. Keep adversarial conformance cases separate from field-prevalence samples. Report exact confidence bounds and raw numerators/denominators per ODD stratum. A zero-failure synthetic corpus is not a field failure-probability estimate.

## Implementation sequence

1. Freeze schema and synthetic chain scenarios.
2. Validate fixture schema and internal references.
3. After upstream PRs merge, add direct dependencies on their public Rust evaluators and exercise the real APIs from one test runner.
4. Add generated event-order/fault-injection sequences across stage boundaries.
5. Only after HIL, independent bench metrology and supervised field sampling: connect physical receipts and task-specific claim profiles.
6. Replicate at a held-out site and robot configuration before generalization claims.

## Limits

The v0.1 artifacts describe expected software behavior. Identifiers, hashes and outcomes are invented. The contract is not a safety controller, field measurement, robot dispatch permit, certification or proof of physical truth. Authentication, metrology, actor competence, upstream independence and physical validation remain with their existing evidence owners and competent assessors.
