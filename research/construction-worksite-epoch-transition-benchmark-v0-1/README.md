# Worksite epoch / ODD transition benchmark v0.1

Issue: #3542. Protocol proposal only; no physical trials.

## Rationale

A valid snapshot becomes obsolete when a construction site changes: zones move, trades enter, routes close, sensors degrade, payload or software changes. Existing snapshot and temporal systems do not yet define exactly when a change invalidates an active task, what safe state follows, and what evidence permits restart.

NIOSH (2026-10-07) identifies dynamic work areas, multiple trades, unexpected entry, sensor degradation, comms failures, software changes and recovery as risks. NIST's Mobility Performance of Robotic Systems develops measurement science for dynamic mobility, adaptability, task efficiency, quality and assurance.
- https://www.cdc.gov/niosh/bulletin/2026/safety-in-construction-robotics.html
- https://www.nist.gov/programs-projects/mobility-performance-robotic-systems

## Epoch contract

Each task segment binds to an immutable, monotonic, content-addressed worksite context epoch. It references exclusion-zone geometry, routes/closures, site/design phase, active trade/plant activity and briefings, robot configuration/payload/speed/mode/brakes, software/firmware, sensor health/calibration, comms, clock uncertainty, freshness and task ODD.

Each epoch carries sequence, canonical digest, predecessor digest, effective/expiry policy, source references and affected task scope. Same sequence/different digest is conflict; lower sequence is rollback; identical sequence/digest is idempotent. Epochs are declarations, not proof that site facts are true.

Privacy: represent work-zone occupancy/trade presence/clearance, never worker identity, intent, emotion or biometrics.

## States and mandatory invariants

- ACTIVE(epoch): only commands bound to the exact epoch/configuration.
- SAFE_HOLD(reason,last_valid_epoch): inhibit work commands and reach declared safe condition.
- CONTEXT_STALE(epoch,age): no continuation outside declared freshness/clock bounds.
- CONTEXT_CONFLICT(epoch_set): fail closed; arrival order cannot choose a winner.
- REQUALIFICATION_REQUIRED(epoch): hold until evidence-backed requalification.
- ABORTED(reason): latch abort if safe continuation cannot be established.

Reject rollback, same-sequence/different-digest, unauthenticated critical updates and unknown critical fields. Critical changes invalidate affected qualification before the next work command. Delayed/dropped/duplicated/reordered updates cannot revive old permission. Reconnect, reboot, sensor recovery or operator acknowledgement never restores ACTIVE alone. Restart requires requalification against the newest epoch and current machine/sensor/brake state. No universal timeout is assumed; if the task-specific stopping envelope cannot be justified, do not proceed. Productivity cannot offset a false-safe transition.

The reference oracle binds each qualification to the current invalidation generation and treats it as a short-lived lease: it carries a monotonic issue/expiry interval and a timebase/boot identity. A proof from before a context-loss, clock-fault, epoch or configuration invalidation is rejected. A lease cannot exceed the task policy's maximum duration; command authorization checks the live monotonic timebase, lease expiry, measured context age and clock uncertainty. Observed staleness/uncertainty latches `CONTEXT_STALE` and clears qualification. The freshness/lease limits are task-policy inputs and must be derived from the task risk assessment, not chosen to make tests pass.

## Challenge corpus

Freeze zone expansion/relocation/removal during motion; trade/plant entry and handover; route closure/obstructions; stale, dropped, duplicate, reordered, replayed and rolled-back updates; same-revision digest conflict; clock drift/network partition; dust/rain/glare/occlusion/contamination; sensor loss; payload/speed/brake/tool/mode/calibration and software changes; reboot, watchdog and partial recovery. Include negative controls, boundaries, timing races, property-based/metamorphic variants, adversarial scheduling and held-out combinations.

## Separate metrics

**Safety gates:** false-safe transitions = 0 on frozen deterministic corpus; no commands against stale/conflicting/rolled-back/unqualified epochs; no old-epoch restart; 100% command-to-epoch/configuration traceability.

**Timeliness/metrology:** context-change-to-inhibit and hazard-to-safe-state p50/p95/p99/max; epoch age, reconciliation delay, clock uncertainty, remaining clearance/stopping margin; false holds per robot-hour/context change.

**Availability/productivity:** hold duration by cause, requalification time, lost accepted output, accepted output per crew-hour/robot-hour and matched manual/vendor baseline on same task/design/ODD/acceptance/functional unit.

Task-specific condition: `available_clearance > v × (detection + communication + control latency) + measured stopping distance + U95 + declared reserve`. Use independent stopping-distance metrology at actual payload, speed, surface, slope, brake and mode. If this cannot be supported, continuation is inadmissible. Mean latency is not a safe bound.

## Verification and composition

Deterministic transition contract → property/metamorphic and adversarial fault tests → HIL with independent clocks → bench stop-distance metrology → supervised field trials with independent work-zone ground truth → held-out second-site and second-configuration replication. Synthetic tests qualify only software contract; HIL is not field validation.

Reuse #3317 snapshot, #3499 temporal, #1595 scenario identity, #1172 replay, #3522 trial receipts, #3505/#3506 physical protocol and Symthaea #4918 dependency lineage. #3542 owns changing-site invalidation/requalification; do not fork existing owners.

HP publishes ±2 mm layout tolerance only under its specified high-accuracy setup (SMR prism, high-accuracy print mode and 1-inch total station, tested at 5–30 m); its up-to-30% productivity claim compares with its previous software version and is based on internal tests. Dusty publishes ±1.6 mm with accurate surveyed control; its FieldPrinter 2 specifies three forward ToF obstacle sensors with about 0.9 m range, four cliff sensors, IP51-equivalent protection and no rain operation. These are vendor specifications, not matched independent dynamic-ODD safety tests. Compare only identical task, revision, network, surface, configuration, density and acceptance criteria.

For earth-moving machinery within the scope of ISO 19014, Part 2 addresses safety-control systems that control machine motion or mitigate hazards, while excluding awareness-only systems that do not affect motion; Part 4 addresses software/data transmission but excludes cybersecurity.

Additional adjacent standards references:
- **AMRA-201:2026** is a general mobile-robot requirements and test-method reference for robots on solid travel surfaces (including wheels, legs and tracks); use it for baseline performance test design, not as proof of construction-site functional safety.
- **ISO 3691-4:2023** applies to driverless industrial trucks and their systems, and its defined application assumes people in the area have specific training, instruction or awareness. A third edition, **ISO/DIS 3691-4**, is under development; the draft includes external movement-stop interfaces and loss-of-communication provisions, while listing protection against corruption and self-evolving AI behavior among exclusions. Treat this as adjacent input only after product classification; it is not automatically applicable to every layout robot or mixed-trade construction site.
- **ISO 10218-1:2025** covers industrial robots as incomplete machines; ISO 10218-2 addresses integration and application safety. It is not a substitute for mobile-machine or worksite coordination analysis.
- **ISO 19014** remains scoped to earth-moving machinery, not all construction robots.

No generic standards-compliance claim is made. Applicability must be determined product-by-product and use-case-by-use-case:
- AMRA-201:2026: https://www.amr-alliance.org/amra-201/
- ISO 3691-4:2023: https://www.iso.org/standard/83545.html
- ISO/DIS 3691-4 (third edition in development): https://www.iso.org/standard/88615.html
- ISO 10218-1:2025: https://committee.iso.org/standard/73933.html
- ISO 19014-2:2026: https://www.iso.org/standard/87718.html
- ISO 19014-4:2026: https://www.iso.org/standard/87720.html
- https://www.hp.com/us-en/printers/site-print/layout-robot.html
- https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs

Targets are hypotheses, not achieved results or certification limits: zero false-safe/epoch leakage in frozen corpus, 100% traceability and requalification, no missed task-specific stopping deadlines, independent site/configuration replication before generalization.

## Statistical validation

See [evaluation-plan.md](evaluation-plan.md) for preregistered independent-unit definitions, ODD stratification, confidence bounds, challenge-versus-operational sampling, latency/metrology rules, matched baselines and stop conditions.


## Observation-coverage follow-on

The epoch oracle reacts to context updates; it cannot prove that the upstream observation pipeline detected every material change. Complementary issue [#3558 — observation coverage and missed worksite-change challenge benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/3558) defines `verified_clear` versus `unknown_or_unobserved`, source freshness/coverage and missed-change testing. It owns observation coverage and event-to-epoch timeliness; this PR continues to own epoch transition and authorization invalidation.
## Executable transition validation

The Rust reference oracle contains 23 unit and frozen-scenario tests. It validates the four scenario fixtures against a closed Draft 2020-12 schema and replays their event-by-event expected outcomes. It also runs a bounded exhaustive invariant sweep over 11 event classes at depth four: 14,641 complete event sequences and 16,105 total prefixes. This is bounded software-model regression testing, not a formal proof or physical safety evidence.

The `scenarios/` directory contains ordered epoch change; replay/rollback rejection; communications recovery requiring requalification; and qualification-lease expiry. The versioned schema is `scenarios/worksite-epoch-scenario-v0.1.schema.json`. Strict Rust deserialization rejects unknown fields. Physical HIL, metrology and supervised field trials remain separate evidence gates.
