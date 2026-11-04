# Construction Robot End-to-End Assurance Chain v0.1

**Issue:** [#3566](https://github.com/Luminous-Dynamics/symtropy/issues/3566)  
**Status:** conformance scenario contract; synthetic only

## Purpose

This benchmark tests whether separately implemented construction-robot gates compose safely without duplicating their internal logic.

`approved model/export -> layout input admission -> observation coverage -> worksite epoch -> qualification lease -> command authorization -> physical outcome receipt -> claim-specific admissibility`

A task must not become executable merely because each component can produce a locally valid result. Exact outputs, artifact digests, context epoch, configuration, timebase, qualification generation and lease must survive each interface boundary.

## Component ownership

- #3561 / PR #3564: source model, export, frame/transform, inventory and survey checkpoint input admission.
- #3558 / PR #3558: source observation freshness/coverage and verified-clear vs unknown/changed/conflicting.
- #3542 / PR #3542: epoch sequencing, conflict/rollback, invalidation, lease, safe hold, requalification and command binding.
- #3531 / PR #3531: claim-specific evidence admission.
- #3522 and #3505/#3506: physical trial receipt and physical protocol/metrology.
- #3523 / PR #3524: task-matched layout robot comparison and common referee.
- Symthaea #4918: evidence dependency lineage and claim-relative independence.

This benchmark owns boundary conformance only. It must not grow a second coordinate engine, survey adjustment system, event ledger, observation fusion pipeline, epoch oracle, robot controller, physical metrology system or evidence-dependency graph.

## Frozen scenario contract

The Draft 2020-12 schema defines the challenge packet. Each stage has unique identity, monotonic sequence, explicit predecessor stage IDs, consumed/produced artifact digests, expected disposition and deterministic reason codes. Each qualification stage declares its generation.

Each command attempt explicitly references:
- its command-authorization stage;
- the latest prior observation stage;
- the latest prior worksite epoch stage;
- the latest prior qualification stage, or null where no qualification exists.

The command stage must be a descendant of those sources and directly consume their exact artifact IDs/digests. The validator computes causal blockers (for example unknown observation, qualification predating the epoch, lease invalid at command, or wrong timebase). Declared reason codes must exactly match the computed blockers in both the command attempt and command-stage receipt.

## Paradigm shift: per-command causal frontier

Do not validate every command against the final state at the end of a scenario. For each command at sequence *t*, evaluate the newest relevant observation, epoch and qualification that exist before *t* and are in that command's causal ancestry.

This permits a crucial positive control: command 1 can be valid under epoch 1, a site-change event can then advance to epoch 2, and command 2 must be rejected if it tries to reuse the pre-change qualification. The later epoch must not retroactively erase the validity of command 1, and the earlier qualification must not authorize command 2.

## Mandatory cross-stage properties

1. An inadmissible/indeterminate layout input cannot be promoted to task authorization.
2. Unknown/unobserved, occupied/changed or conflicting observation state blocks affected command authorization.
3. A newly accepted epoch revokes prior qualification for subsequent commands; command must reference the exact current epoch and qualification generation.
4. Each command's referenced observation, epoch and qualification are the latest prior records in that command's own causal timeline—not the scenario's eventual final records.
5. Each command stage must be a descendant of all referenced evidence stages and directly consume their exact produced artifact identities/digests.
6. Stale epoch, old configuration, wrong task, wrong timebase/boot identity or expired lease cannot authorize.
7. Reconnect, reboot, sensor recovery and operator acknowledgement never restore authority alone.
8. Evidence receipt must refer to exact scenario/task/configuration and independent measurement artifacts; field evidence cannot be inferred from software-stage success.
9. Field claims cannot be admitted from synthetic, simulated, bench or HIL evidence.
10. Productivity cannot repair an unsafe, failed, indeterminate or unevaluated safety claim.
11. No global PASS, confidence score or weighted total is emitted.
12. A failure blocks dependent commands/claims but cannot be silently discarded.

## Frozen adversarial scenarios

- Wrong design/export/frame despite nominal robot positioning accuracy.
- Previously admitted design or work-package revision becomes stale before execution.
- Required observation becomes occluded/unobserved.
- Source sequence gap, replay, rollback or same-sequence divergent digest.
- Trade enters or exclusion zone moves after qualification but before a later command.
- Later epoch arrives while a cached command carries old epoch/qualification.
- Payload, brake, tool, mode, sensor, calibration or software configuration changes after qualification.
- Communications loss, reconnect, reboot, timebase/boot change or stale ACTIVE-cache resurrection.
- Current source with clock uncertainty beyond policy.
- Lease expiry at exact command boundary; one tick before/at/after.
- Repeated frames/messages incorrectly counted as independent encounters.
- Bench result promoted to an unsupported field claim.
- One claim not evaluated while another supported claim remains available.

## Benchmark metrics

**Safety gates (non-compensable):** false-safe authorization after upstream failure/unknown; stale/old-epoch/old-configuration/expired-lease command leakage; unqualified restart; command-to-stage digest/sequence/ancestry mismatch. Initial software-contract target: zero in frozen corpus.

**End-to-end latency:** independently staged change to observation; detection to epoch/qualification invalidation; invalidation to command inhibit; hazard to safe-state completion; context reconciliation to requalification/restart. Report p50/p95/p99/max with clock and measurement uncertainty. Sample p99 is not a worst-case guarantee.

**Availability/productivity:** safe-hold frequency/duration by cause, requalification time, accepted output per robot-hour and crew-hour, and total workflow labour (BIM/VDC preparation, trade coordination, survey/control, setup, operator/spotter, print, QA, rework and recovery). Report separately; no trade-off score.

## Statistical protocol

Pre-register each endpoint's independent unit. One independently staged worksite change is one unit; repeated frames, heartbeats and controller cycles do not increase n. Preserve site/shift/task/zone/robot/configuration/source cluster IDs. Keep deterministic adversarial conformance cases separate from operational prevalence samples. Report exact confidence bounds and raw numerators/denominators per ODD stratum. A zero-failure synthetic corpus is not a field failure-probability estimate.

## Implementation sequence

1. Freeze schema and synthetic chain scenarios.
2. Validate schema, artifact lineage, per-command causal frontiers and multi-command transitions.
3. After upstream PRs merge, add direct dependencies on their public Rust evaluators and exercise real APIs from one scenario runner.
4. Add generated event-order, interleaving and fault-injection sequences.
5. Only after HIL, independent bench metrology and supervised field sampling, connect physical receipts and claim-specific physical evidence.
6. Replicate at held-out sites and robot configurations before generalization claims.

## Limits

Identifiers, digests and outcomes are invented. The contract is not a safety controller, field measurement, robot dispatch permit, certification or proof of physical truth. Authentication, metrology, actor competence, upstream independence and physical validation remain with their existing evidence owners and competent assessors.
