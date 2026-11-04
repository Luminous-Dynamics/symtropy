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

HP publishes ±2 mm under its specified high-accuracy layout setup; Dusty FieldPrinter 2 publishes ±1.6 mm. Vendor specifications are not matched independent dynamic-ODD safety tests. Compare only identical task, revision, network, surface, configuration, density and acceptance criteria.
- https://www.hp.com/us-en/printers/site-print/layout-robot.html
- https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs

Targets are hypotheses, not achieved results or certification limits: zero false-safe/epoch leakage in frozen corpus, 100% traceability and requalification, no missed task-specific stopping deadlines, independent site/configuration replication before generalization.