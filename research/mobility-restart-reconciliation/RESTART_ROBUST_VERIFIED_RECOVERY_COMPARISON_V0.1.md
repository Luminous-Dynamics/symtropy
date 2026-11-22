# Restart-Robust Verified Recovery — comparative baselines and benchmark plan v0.1

**Date:** 22 November 2026  
**Status:** research comparison and preregistration proposal; no RRVR campaign results are claimed.

## 1. Executive position

There is already relevant work in black-box robot runtime assurance, VLA safety evaluation, specification-grounded manipulation benchmarks, NIST robotic measurement science and ROS 2 hardware lifecycle management. RRVR should not claim to invent cancellation safety, late-ACK isolation, black-box verification, or fault injection.

The proposed contribution is narrower and deeper:

> Does a robot preserve the identity, uncertainty, physical-effect truth and authority lineage of an action across process/host/controller restart—and can it prove that every old command-holding layer is unable to execute stale work before a new attempt is admitted?

This is a complementary assurance dimension, not a substitute for task-policy or physical-hazard benchmarks.

## 2. Relevant public baselines (checked 22 November 2026)

| Baseline | What it already measures | What RRVR should reuse | Gap RRVR targets |
|---|---|---|---|
| [Robot Runtime Assurance and System Acceptance](https://github.com/Quchaosheng/robot-runtime-assurance) | Black-box runtime acceptance; three-valued confirmed/refuted/insufficient-evidence verdicts; cancellation→STOP, dropped/late ACK, stale ACK isolation, process kill, incomplete evidence, raw-command bypass controls; mock transport and a ROS 2 action-server path | Independent oracle, negative controls that must be detected, re-checkable evidence bundles, subject claims separated from independent observations | Durable attempt/commit ambiguity, exact logical-effect/attempt linkage, boot/controller epochs, downstream queue quiescence, M1/M2 authority freshness, consume-once recovery permit, physical outcome continuity after a restart |
| [SafeVLA-Bench](https://safevla.org/leaderboard.html) | Separates native task success from safety, successful-but-unsafe episodes and violation severity across task suites | Never collapse success and safety into one score; preserve per-spec violations and severity | Does not establish durable restart/attempt reconciliation or end-to-end stale-command fencing |
| [ManiGuard](https://nu-ideas-lab.github.io/ManiGuard/) | Specification-grounded LTLf safety evaluation across 200 base tasks, 1,000 ID/OOD scenarios and more than 23,000 rollouts, including physical Franka evaluation | Freeze independent specifications, broad scenario families, ID/OOD separation and physical evaluation | Add process/host/controller crash cuts, commit ambiguity, exact old-attempt reconciliation and recovery-authority checks |
| [NIST Agility Performance of Robotic Systems](https://www.nist.gov/programs-projects/agility-performance-robotic-systems) | Robot-system metrics, test methods, information models, datasets and planning approaches; unexpected events, obstacles and failures; combined virtual/real validation | Metrology-oriented test methods, declared configurations and sim/real progression | Add evidence-qualified physical-effect continuity and restart recovery as first-class system metrics |
| [NIST Performance Assessment Framework](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems) | Composes perception, mobility, dexterity and safety measurements into system performance models | Treat recovery as an additional system dimension, not a task leaderboard replacement | Include effect continuity, stale-work fencing and recovery availability without allowing high task score to compensate for a safety violation |
| [ROS 2 Control hardware-component lifecycle](https://control.ros.org/rolling/doc/ros2_control/hardware_interface/doc/lifecycle_of_a_hardware_component.html) | Defines hardware component lifecycle states and transitions | Use explicit hardware lifecycle state and configuration evidence | The rolling documentation notes that command interfaces may still be available/written in INACTIVE, depending on implementation. A lifecycle transition or process restart must not be treated as proof of actuator quiescence |
 
### Concrete industry comparison

The public Robot Runtime Assurance project reports 20 shipped scenarios and 150 passing tests in its default no-ROS/no-workbench path, with four unavailable-target scenarios explicitly skipped; its ROS 2 action path exercises a stock action server. It also states that hardware replay is out of scope and kernel-level CAN/network fault injection is not available in its current environment. Its design is an important comparator for black-box verdicts and evidence custody.

SafeVLA-Bench shows why a task-only number is unsafe as a proxy: its RoboCasa-365 Atomic-Seen Xiaomi-Robotics-1 result reports 81.9% native success alongside 50.1% safety and 37.7% successful-but-unsafe episodes. These percentages are benchmark-specific and are not directly comparable with RRVR's recovery metrics.

ManiGuard reports more than 23,000 evaluation rollouts and finds that 6–21% of successful rollouts violate a monitored specification in its evaluated setting. RRVR should adopt specification-grounded negative controls, not claim that its own synthetic tests prove lower physical violation rates.

NIST's projects establish a strong measurement-science precedent for declaring metrics and test procedures and validating across virtual and real environments. RRVR should submit itself to that standard of evidence quality.

## 3. The critical stack-specific observation

ROS 2 hardware lifecycle states are not sufficient evidence of downstream command quiescence. The current rolling `ros2_control` hardware-component documentation says that, in INACTIVE, command interfaces can remain available and be written; whether the hardware component continues using commands or skips them is left to its implementation, with safety-critical interface work noted as a roadmap item.

Therefore RRVR's platform contract must enumerate every layer that can retain/execute a command and separately evidence:
1. controller queue epoch fence;
2. middleware/bridge queue disposition;
3. driver/PLC/fieldbus queue disposition;
4. device firmware/actuator scheduler disposition;
5. physical effect outcome for the exact old attempt.

A STOP request or cancel acknowledgment is not proof that all lower-level work has retired, nor proof that the physical effect did not occur. The downstream evidence contract is tracked in [MOBILITY-DOWNSTREAM-FENCE-001 #6817](https://github.com/Luminous-Dynamics/symtropy/issues/6817).

## 4. RRVR's proposed benchmark contribution

### 4.1 Three independent axes

Score separately:

**A. Physical-effect continuity**
- exact logical-effect and attempt identity survives restart;
- old effect is classified Applied, NoEffect, Partial, Unknown or Disputed using a declared observer;
- late evidence updates the correct old attempt and cannot contaminate a new task.

**B. Authority continuity**
- old boot/controller tickets are rejected;
- M1 mission and M2 applicability are checked at the native acceptance point;
- recovery permit is bound to the failed attempt and next attempt, authenticated by its owner and consumed exactly once across restart;
- any change in mission, mode, configuration, ODD, time basis or binding before acceptance causes a hold.

**C. Execution-path continuity**
- all command-holding layers are declared;
- every old downstream queue is quiescent/fenced or remains a hold;
- no hidden/unknown layer is silently counted as empty;
- a new command cannot overtake delayed old-epoch work.

A single score must never average these axes. Any hard safety gate violation fails the scenario regardless of task completion or latency.

### 4.2 Matched experimental arms

Freeze the same robot, firmware, controller, policy, scenario, sensor configuration, command trace and independent evaluator across arms:

A. No-crash baseline.  
B. Restart with no recovery evidence.  
C. Restart with persistence evidence only.  
D. Restart with effect observation but no current M1/M2 authority.  
E. Controller queue fenced but at least one downstream layer unresolved.  
F. Full owner composition: persistence classification + controller and downstream fences + independent effect observation + M1 resolution + fresh M1/M2 ticket + native one-shot permit consumption.  
G. Naive resubmission, permitted only in a non-actuating mock as a negative control.

### 4.3 Hard safety gates (research targets, not results)

- duplicate Applied physical effects per logical-effect identity: **0**;
- stale old-epoch command executions after accepted end-to-end quiescence: **0**;
- Unknown, Partial or Disputed outcomes released into a new action: **0**;
- false Applied/NoEffect classifications against the frozen independent evaluator: **0**;
- recovery-permit replay, double consumption or next-attempt substitution: **0**;
- admission with an unenumerated/unfenced downstream layer: **0**;
- declared lifecycle cut-point and command-holding-layer coverage: **100%**;
- positive-control admission: measure with a confidence interval; initial research target **≥99.9%** within the explicitly supported profile.

The 99.9% positive-control target is an availability/false-hold objective, not a safety substitute. For a zero-failure observation, 2,995 independent representative trials are approximately required to put the one-sided 95% upper bound below 0.1%. Correlated frames or repeated trials on the same unchanged setup do not create independent samples.

### 4.4 Performance and recovery metrics

Report p50/p95/p99, with task/robot/site clustering and all denominators:

- restart detection → application-path hold;
- restart detection → controller fence;
- restart detection → downstream quiescence;
- restart detection → independent effect classification;
- late evidence arrival → M1 reconciliation;
- qualified NoEffect → fresh-attempt candidate;
- candidate → native admission;
- false-hold duration and operator interventions;
- task makespan and verified throughput;
- energy/idle time and recovery overhead, where measured;
- stale work rejected at each queue layer.

The p99 fence deadline must be allocated from the declared robot's actual supervisory/control deadline. Do not invent one universal millisecond threshold for all robots.

### 4.5 Statistical and reproducibility rules

- Freeze the fault schedule, action trace, software/firmware/configuration revisions, owner-profile revisions, seeds, evaluator and exclusions before confirmatory testing.
- Use independent scenario schedules as experimental units; do not count video frames/control ticks as independent trials.
- Pair equivalent schedules across methods and cluster confidence intervals by robot, task and site where applicable.
- Keep subject assertions separate from independent observation.
- Preserve raw event streams and re-derivable evidence bundles; external evaluators must be able to replay verdict computation without trusting planner internals.
- Include expected-refutation and expected-insufficient-evidence controls in every CI run.
- Publish every failure and false hold; do not silently drop inconvenient cases.

## 5. How to exceed current baselines without making false comparisons

RRVR should not claim it beats SafeVLA, ManiGuard or NIST task-performance scores. Its comparison should be a new, orthogonal axis.

The most credible path is:

1. Reproduce the public black-box semantics from Robot Runtime Assurance: three-way verdicts, late/stale ACK isolation, cancellation and process-kill controls, expected-negative controls, re-checkable bundles.
2. Extend every applicable scenario with exact persisted operation identity, commit ambiguity, restart epoch changes, downstream queue layers and an independent physical-effect oracle.
3. Connect the scenario harness to the existing Symtropy #6749 native application seam and Mycelix #780/#803 persistence owners; avoid a second outbox, queue or authority service.
4. Check the lifecycle properties in TLA+ and cross-check the same frozen transition corpus against the Rust reference contract.
5. Run the same black-box harness over a declared ROS 2 adapter, then a declared HIL platform, then one physical robot/controller/firmware stack. Do not extrapolate between platforms.
6. Invite an independent lab or third-party robot runtime to supply its own SUT; the evaluator should not import the SUT's implementation or trust its self-reported physical outcome.

A strong first public claim would be: “RRVR detects and rejects the defined stale-command, duplicate-effect, ambiguous-commit and stale-authority restart failures in the published exact-stack test envelope.” It is substantially more defensible than “exactly-once robotics” or “safer than SOTA.”

## 6. Standards crosswalk — not certification

- [ISO 10218-1:2025](https://www.iso.org/standard/73933.html) is the current published Part 1 industrial-robot safety standard for the robot as a partly completed machine. A future integration crosswalk should also examine Part 2 system/application integration scope.
- [ISO/DIS 3691-4, Edition 3 (2026)](https://www.iso.org/standard/88615.html) is a draft under enquiry for driverless industrial trucks and their systems, including AMRs. It is not yet a published replacement for ISO 3691-4:2023.

Map RRVR evidence to applicable risk-reduction, system-integration and verification clauses only after selecting the robot class and intended use. A benchmark PASS is not conformity, certification, a risk assessment or authorization to operate.

## 7. Execution sequence

1. Bounded TLA+ transition model with safe invariants and a deliberately unsafe retry negative control.
2. TLA+ ↔ Rust decision-corpus differential checks.
3. Mycelix exact commit-classification / operation-identity adapter.
4. M1/M2 and authority-permit integration at the existing native application boundary.
5. Full queue-layer inventory and platform-specific downstream quiescence receipt (#6817).
6. Black-box simulator fault injection with independent effect oracle.
7. HIL fault injection on an exact, declared stack.
8. Reviewed physical campaign and independent replication.

## 8. Claim ceiling

Software tests establish software-contract behavior only. TLC establishes properties only for its bounded abstraction and assumptions. Simulator evidence establishes only the declared simulator model. HIL and physical evidence are configuration- and fault-envelope-scoped. No current artifact establishes durable production recovery, physical exactly-once effects, complete observation, functional-safety certification or regulatory conformity.
