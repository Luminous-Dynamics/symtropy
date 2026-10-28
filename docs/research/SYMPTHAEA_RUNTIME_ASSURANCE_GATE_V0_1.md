# Symthaea–Symtropy Runtime Assurance Gate v0.1

**Status:** research prototype / stacked pull requests. Neither the gate nor the sequence guard is a safety certification, and neither has been validated on physical robots.

## Hypothesis

Typed semantic intent plus an independent deterministic admission gate should reduce unsafe dispatches compared with forwarding cognitive requests directly to embodiment adapters, without making nominal accepted requests unusably restrictive. This is a falsifiable engineering hypothesis, not a claim that the current implementation has achieved the target.

## Proposed architecture

Symthaea cognition (beliefs, goals, planning, skill selection)
→ leased ControlRequest / EmbodiedIntent
→ RuntimeAssuranceGate
→ per-session RequestSequenceGuard
→ platform adapter and local controller
→ independent runtime safety authority
→ plant (Symtropy simulation or robot)
→ ControlOutcome + timestamped telemetry/evidence
→ Symthaea belief and policy update

The runtime gate checks request freshness, supported intents, frame and target bounds, requested speed, snapshot and sensor freshness, emergency-stop state, actuator readiness, optional link health, and battery reserve. The sequence guard rejects duplicate or out-of-order sequence numbers within one live authority session.

## Safety and claim boundaries

The gate returns an admission decision only. It does not validate full trajectories, calculate collision-free paths, prove stability, monitor actuator behavior continuously, or control hardware. The local controller must recheck dynamic constraints throughout execution; the platform safety authority retains veto priority. A stop request is passed to the adapter during degraded health so that the adapter can select its platform-specific containment behavior. A gate permit is never an instruction to bypass a physical emergency stop.

The sequence guard is a small in-memory ordering primitive. It is not transport authentication, cryptographic anti-replay, persistent storage, multi-process atomicity, or protection against a sender re-encoding the same semantic action with a fresh sequence. The adapter must serialize sequence commitment with dispatch, authenticate the session identity, establish a non-reused session ID after restart, and persist integrity-protected state if anti-replay must survive restart. The helper is not yet wired into the runtime gate or an actual adapter dispatch path.

## What exists in this prototype

The runtime gate checks request freshness and basic shape, configured support for the semantic intent, exact reference-frame agreement, axis-aligned target bounds, navigation speed limits, safety-state validity, snapshot age, sensor age, emergency-stop state, actuator readiness, optional link health, and battery reserve. It emits stable machine-readable veto reasons.

The sequence guard tracks a high-water sequence per live session, allows gaps, rejects duplicate/lower sequences, and fails closed when its fixed session capacity is exhausted rather than evicting a session. Unit tests for both modules are authored but require compilation and execution in CI before they count as passing evidence.

Not covered yet: continuous path/workspace enforcement; collision checking; jerk/acceleration/force/torque and joint limits; human proximity; watchdog ownership; authenticated session establishment; durable replay protection across process restarts; prevention of semantic re-encoding under a fresh sequence; clock synchronization faults; hardware-in-the-loop; multi-robot deconfliction; certified safety mechanisms; platform-specific safe-stop semantics; adapter-level proof that a vetoed request never reaches the controller.

## Benchmark design

### Comparators

1. **Raw intent forwarding:** no generic admission checks; the adapter receives the semantic request. Use only in simulation/offline fixtures, never as a live-robot test.
2. **Request contract only:** validates request schema and expiry but does not evaluate platform capabilities or live health.
3. **Runtime-assurance gate:** the admission checks implemented in this branch.
4. **Gate + sequence guard:** gate plus per-session sequence-order rejection.
5. **Gate + sequence guard + native controller/safety:** intended deployment configuration, with platform-native limits and watchdogs active.

Comparators must use identical requests, environment seeds, observation traces, policy decisions, and adapter/controller settings wherever applicable. Publish the source SHA, scenario definitions, seed list, configuration, and all decisions. Do not compare independently sampled scenarios and attribute the difference to the gate.

### Fault-injection matrix

Each injection is paired with a nominal control case and an explicit expected outcome.

| Family | Injection | Required result |
| --- | --- | --- |
| Request lifecycle | expired or future-dated request | veto |
| Identity and schema | zero request ID, invalid confidence, non-finite pose, invalid quaternion | veto before adapter dispatch |
| Capability | unsupported semantic intent | veto with a machine-readable reason |
| Coordinate semantics | target frame differs from declared workspace frame | veto; never guess a transform |
| Workspace | target outside configured bounds | veto |
| Kinematics request | requested navigation speed exceeds envelope | veto |
| Sensor health | sensor age above configured threshold | veto motion |
| State health | state snapshot older than configured threshold | veto motion |
| Actuator | actuator not ready | veto motion |
| Emergency stop | emergency-stop state active | veto motion |
| Communication | lost link while policy requires it | veto motion |
| Energy | battery below configured reserve | veto motion |
| Degraded containment | supported stop or assistance request with stale sensors / low battery / lost link | forward to adapter for local containment or escalation; never direct-actuate |
| Policy/configuration | invalid thresholds, empty capability set, inverted/non-finite bounds | fail closed |
| Sequence | duplicate or lower sequence for an existing live session | reject without changing high-water mark |
| Sequence capacity | a new session after the fixed session cap is reached | reject without evicting tracked sessions |
| Restart / authentication | reused session ID or forged identity after restart | not solved by the in-memory guard; test at the authenticated durable adapter boundary |
| Semantic replay | same action re-encoded under a new valid sequence | not detectable from sequence ordering alone; requires authority/intent policy and expiry context |
| Dispatch conformance | a vetoed request reaches controller input | zero tolerated; requires adapter integration tests |
| Outcome ordering | delayed or out-of-order controller outcomes | reject or reconcile in lifecycle manager; not implemented by this sequence guard |

The identity/schema checks are validated by the upstream control contract and should be tested end-to-end. The sequence rows test the helper in isolation until adapter wiring exists; do not claim end-to-end replay resistance yet.

### Scoreboard

Report every metric separately; do not collapse safety, task success and efficiency into one unqualified autonomy score.

- **Unsafe-dispatch rate:** injected requests that violate a declared hard constraint and nevertheless reach the adapter, divided by injected hard-constraint violations.
- **False-veto rate:** otherwise admissible nominal requests vetoed, divided by nominal admissible requests.
- **Sequence rejection coverage:** all duplicate/lower sequence injections rejected before dispatch, divided by all such injected attempts.
- **Fault detection coverage:** each injected failure class detected, divided by injected failure classes.
- **Task completion:** completed missions among trials declared admissible at the beginning.
- **Recovery quality:** success after a fault, time to safe state, and fraction of faults that lead to a controlled stop or assistance escalation.
- **Latency:** p50/p95/p99 admission and sequence-check time, measured on a named CPU/runtime and compiled build profile.
- **Determinism:** exact same decision and veto-code sequence when replaying the same fixed request/state trace.
- **Decision traceability:** fraction of vetoes with a valid machine-readable reason and evidence/telemetry reference.
- **Transfer:** repeat the exact protocol across drone, manipulator, mobile robot, AUV, quadruped and humanoid adapters; report per-platform numbers, not only pooled averages.

### Proposed acceptance targets (not measured results)

- **100% rejection** of all enumerated hard-contract and sequence-order violations in deterministic fixtures.
- **Zero observed unsafe dispatches** in at least 1,000,000 seeded/adversarial offline requests before any HIL phase.
- **At most 0.1% false vetoes** on a separately frozen nominal fixture suite.
- **Exact deterministic replay** for all frozen request/state traces.
- **p99 combined admission and sequence-check latency below 100 microseconds** on a named desktop reference build; set a separate embedded-platform budget only after measuring the intended target hardware.
- **No regressions** in task completion, recovery, or safe-stop metrics when layered with native controller/safety.
- **Zero capacity evictions:** when sequence tracking is full, a new session must be rejected, not admitted by evicting another session.

Zero failures in finite tests is not proof of safety. For independent Bernoulli trials, zero observed failures in one million trials still gives an approximate 95% upper bound of 3 per million; correlated or unrepresentative fault samples weaken that interpretation. Report confidence bounds and scenario coverage, and do not promote a synthetic result to a physical safety claim.

## Cross-embodiment progression

1. **Offline deterministic fixtures:** validate request gates, sequence ordering and veto reasons.
2. **Simulation in Symtropy:** inject sensor, actuator, battery, link, timing, frame and workspace faults; score mission outcomes and recovery trajectories.
3. **Differential simulator tests:** run the same scenario through simplified and higher-fidelity plants; track model-mismatch and residual bounds.
4. **Software-in-the-loop / hardware-in-the-loop:** retain native autopilot or robot controllers, use geofenced and supervised test fixtures, verify watchdog/failsafe behavior, and test session authentication and restart persistence.
5. **Physical trials:** only after platform-specific hazard analysis, controller qualification and human-supervised incremental trials. This document is not authorization for autonomous physical operation.

The first useful claim is narrow: deterministic helpers reject a frozen set of invalid intents and out-of-order session sequences in tests. The stronger claim—that Symthaea improves general-purpose robotic autonomy across embodiments—requires adapter conformance, task-level comparisons, ablations, repeated seeds, uncertainty intervals, and physical validation.

## Industry comparison and references

The comparison target is complementary integration, not a claim of replacing mature infrastructure:

- [ROS 2 Control architecture](https://control.ros.org/master/doc/getting_started/getting_started.html): controller manager, hardware/resource interfaces, control-loop lifecycle and different update rates.
- [PX4 Offboard mode](https://docs.px4.io/main/en/flight_modes/offboard): external setpoint control, proof-of-life requirements and configured offboard-loss failsafe.
- [NVIDIA Isaac Lab](https://developer.nvidia.com/isaac/lab): scalable multi-embodiment learning and simulation-to-deployment workflows.
- [RoboDojo benchmark](https://robodojo-benchmark.com/leaderboard?bench=sim): reports distinct generalization, precision, long-horizon, memory and open-task dimensions rather than one headline score.
- [AI Index 2026, robotics section](https://hai.stanford.edu/assets/files/ai_index_report_2026.pdf): notes strong manipulation-benchmark performance while also identifying the limitations of short-horizon controlled simulation.

A fair comparison records which capabilities are already provided by the platform controller or safety system, then tests whether the Symthaea–Symtropy layer improves cross-embodiment semantics, recovery, context use, measurable task success or evidence traceability without weakening those existing protections.
