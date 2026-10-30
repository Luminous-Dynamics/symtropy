# Symthaea–Symtropy Runtime Assurance Gate v0.1

**Status:** research prototype / stacked pull request. The gate is not a safety certification and has not been validated on physical robots.

## Hypothesis

Typed semantic intent plus an independent deterministic admission gate should reduce unsafe dispatches compared with forwarding cognitive requests directly to embodiment adapters, without making nominal accepted requests unusably restrictive. This is a falsifiable engineering hypothesis, not a claim that the current implementation has achieved the target.

## Proposed architecture

Symthaea cognition (beliefs, goals, planning, skill selection)
→ leased ControlRequest / EmbodiedIntent
→ RuntimeAssuranceGate
→ platform adapter and local controller
→ independent runtime safety authority
→ plant (Symtropy simulation or robot)
→ ControlOutcome + timestamped telemetry/evidence
→ Symthaea belief and policy update

The gate checks request freshness, supported intents, frame and target bounds, requested speed, snapshot and sensor freshness, emergency-stop state, actuator readiness, optional link health, and battery reserve.

The gate returns an admission decision only. It does not validate full trajectories, calculate collision-free paths, prove stability, monitor actuator behavior continuously, or control hardware. The local controller must recheck dynamic constraints throughout execution; the platform safety authority retains veto priority. A stop request is passed to the adapter during degraded health so that the adapter can select its platform-specific containment behavior. A gate permit is never an instruction to bypass a physical emergency stop.

## What exists in this prototype

The Rust module checks request freshness and basic shape, configured support for the semantic intent, exact reference-frame agreement, axis-aligned target bounds, navigation speed limits, safety-state validity, snapshot age, sensor age, emergency-stop state, actuator readiness, optional link health, and battery reserve. It emits stable machine-readable veto reasons. The associated unit tests are authored but require compilation and execution in CI before they are considered passing evidence.

Not covered yet: continuous path/workspace enforcement; collision checking; jerk/acceleration/force/torque and joint limits; human proximity; watchdog ownership; signed authorization; replay protection across process restarts; clock synchronization faults; hardware-in-the-loop; multi-robot deconfliction; certified safety mechanisms; platform-specific semantics for safe stop.

## Benchmark design

### Comparators

1. **Raw intent forwarding:** no generic admission checks; the adapter receives the semantic request. Use only in simulation/offline fixtures, never as a live-robot test.
2. **Request contract only:** validates request schema and expiry but does not evaluate platform capabilities or live health.
3. **Runtime-assurance gate:** the implementation in this branch.
4. **Gate + native controller/safety:** the intended deployment configuration, with platform-native limits and watchdogs active.

Comparators must use identical requests, environment seeds, observation traces, policy decisions, and adapter/controller settings wherever applicable. Publish the source SHA, scenario definitions, seed list, configuration, and all decisions. Do not compare independently sampled scenarios and attribute the difference to the gate.

### Fault-injection matrix

Each injection is paired with a nominal control case and an explicit expected outcome.

| Family | Injection | Required result |
| --- | --- | --- |
| Request lifecycle | expired or future-dated request | veto |
| Identity and schema | zero request ID, invalid confidence, non-finite pose, invalid quaternion | veto before adapter dispatch |
| Capability | unsupported semantic intent | veto with UnsupportedIntent |
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
| Replay/order | repeated request IDs, delayed outcomes, out-of-order telemetry | currently not covered by this gate; add to lifecycle evaluator before deployment |

The “identity and schema” entries not represented by the gate itself are validated by the upstream control contract; benchmark them end-to-end. The replay/order row is intentionally marked as a gap rather than implying replay protection exists.

### Scoreboard

Report every metric separately; do not collapse safety, task success and efficiency into one unqualified “autonomy score.”

- **Unsafe-dispatch rate:** injected requests that violate a declared hard constraint and nevertheless reach the adapter, divided by injected hard-constraint violations.
- **False-veto rate:** otherwise admissible nominal requests vetoed, divided by nominal admissible requests.
- **Fault detection coverage:** each injected failure class detected, divided by injected failure classes.
- **Task completion:** completed missions among trials declared admissible at the beginning.
- **Recovery quality:** success after a fault, time to safe state, and fraction of faults that lead to a controlled stop or assistance escalation.
- **Latency:** p50/p95/p99 admission time, measured on a named CPU/runtime and compiled build profile.
- **Determinism:** exact same decision and veto-code sequence when replaying the same fixed request/state trace.
- **Decision traceability:** fraction of vetoes with a valid machine-readable reason and evidence/telemetry reference.
- **Transfer:** repeat the exact protocol across drone, manipulator, mobile robot, AUV, quadruped and humanoid adapters; report per-platform numbers, not only pooled averages.

### Proposed acceptance targets (not measured results)

- **100% rejection** of all enumerated hard-contract violations in deterministic fixtures.
- **Zero observed unsafe dispatches** in at least 1,000,000 seeded/adversarial offline requests before any HIL phase.
- **At most 0.1% false vetoes** on a separately frozen nominal fixture suite.
- **Exact deterministic replay** for all frozen request/state traces.
- **p99 admission latency below 100 microseconds on a named desktop reference build**; set a separate embedded-platform budget only after measuring the intended target hardware.
- **No regressions** in task-completion, recovery, or safe-stop metrics when the gate is added to the native-controller baseline.

Zero failures in finite tests is not proof of safety. For independent Bernoulli trials, zero observed failures in one million trials still gives an approximate 95% upper bound of 3 per million; correlated or unrepresentative fault samples weaken that interpretation. Report confidence bounds and scenario coverage, and do not promote a synthetic result to a physical safety claim.

## Cross-embodiment progression

1. **Offline deterministic fixtures:** validate request gates and veto reasons.
2. **Simulation in Symtropy:** inject sensor, actuator, battery, link, timing, frame and workspace faults; score mission outcomes and recovery trajectories.
3. **Differential simulator tests:** run the same scenario through simplified and higher-fidelity plants; track model-mismatch and residual bounds.
4. **Software-in-the-loop / hardware-in-the-loop:** retain native autopilot or robot controllers, use geofenced and supervised test fixtures, and verify watchdog/failsafe behavior.
5. **Physical trials:** only after platform-specific hazard analysis, controller qualification and human-supervised incremental trials. This document is not authorization for autonomous physical operation.

The first useful claim is narrow: a deterministic gate rejects a frozen set of invalid requests and degraded-health motion proposals with a bounded false-veto rate. The stronger claim—that Symthaea improves general-purpose robotic autonomy across embodiments—requires task-level comparisons, ablations, repeated seeds, uncertainty intervals, and physical validation.

## Industry comparison and references

The comparison target is complementary integration, not a claim of replacing mature infrastructure:

- [ROS 2 Control architecture](https://control.ros.org/master/doc/getting_started/getting_started.html): controller manager, hardware/resource interfaces and control-loop lifecycle.
- [PX4 Offboard mode](https://docs.px4.io/main/en/flight_modes/offboard): external setpoint control, proof-of-life requirements and configured offboard-loss failsafe.
- [NVIDIA Isaac Lab](https://developer.nvidia.com/isaac/lab): scalable multi-embodiment learning and simulation-to-deployment workflows.
- [ISO 10218-2:2025](https://www.iso.org/standard/73934.html): industrial robot application and cell safety requirements.
- [ISO/FDIS 13482](https://www.iso.org/standard/83498.html): service robot safety requirements; at the time of writing this is a draft under approval, not a claim of completed certification.

A fair comparison records which capabilities are already provided by the platform controller or safety system, then tests whether the Symthaea–Symtropy layer improves cross-embodiment semantics, recovery, context use, measurable task success or evidence traceability without weakening those existing protections.
