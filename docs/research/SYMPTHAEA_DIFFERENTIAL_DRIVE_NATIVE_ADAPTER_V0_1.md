# Symthaea–Symtropy Differential-Drive Native Adapter v0.1

**Status:** research reference adapter and mock tests. This is not a ROS 2 plugin, hardware driver, collision checker, real-time qualification, or robot-safety certification. No exact-head Rust test result is claimed until CI runs on this PR head.

## Concrete change

This slice adds a non-drone adapter from the shared EmbodiedIntent contract to a typed differential-drive local-controller interface. It maps only NavigateTo, HoldPosition, Stabilize, Stop, and RequestAssistance; manipulator-only Reach, Inspect, and Manipulate requests fail closed. It carries session ID, monotonic sequence, request ID, and expiry provenance through to the controller trait. It never emits wheel speeds, motor power, or torque.

The adapter validates configuration at construction, rechecks the request lease against its injected monotonic clock immediately before invoking the native-controller interface, validates target frame and speed capability, and returns a specific failure category when the local adapter cannot accept the request.

## Authority model

1. Symthaea proposes a typed task intent, not actuator values.
2. The serialized DispatchBoundary authenticates the caller, commits/checks its sequence, and evaluates the current generic runtime gate.
3. The differential-drive adapter rechecks expiry and embodiment-specific frame/speed/intent capability.
4. A platform-owned local controller/planner receives a semantic goal and request provenance.
5. The native controller must check expiry again at its actual enqueue/application point and retain collision checking, watchdogs, actuator limits, and independent emergency-stop behavior.
6. Controller ticket means only accepted for execution. It is not task completion. Execution outcomes still need a separate lifecycle/telemetry connection to Symthaea.

The clock is injected intentionally: the dispatcher and downstream controller must agree on one monotonic time domain. Do not substitute UTC wall time. A pre-submit check cannot by itself close the scheduling race that occurs after adapter validation.

## Immediate conformance suite

The new unit tests check:
- valid navigation becomes a native semantic goal with default speed and provenance;
- an expired request is rejected at the adapter boundary with zero native-controller calls;
- frame mismatch, speed-limit violation, and unsupported manipulation are rejected with zero native-controller calls;
- invalid configuration fails at construction;
- the composed DispatchBoundary reports a sink-side expired-lease rejection as Rejected, not ControllerFailed; and
- a controlled-stop intent becomes an explicit Stop command, not a motor vector.

These are test assertions authored in source. They become evidence only after the exact commit's test command passes.

## Proposed benchmark gates (targets, not results)

| Metric | Initial target |
|---|---:|
| Deterministic invalid/expired/unsupported fixtures reaching native controller | 0 |
| Admitted request with missing provenance fields | 0 |
| Platform-specific frame/speed mismatch reaching native controller | 0 |
| Fresh accepted request submitted exactly once | 100% of registered fixtures |
| Exact-head unit-test fixtures passed | 100% |
| p99 admission + adapter overhead | report first; target <100 μs on a named, reproducible host profile |
| False veto on frozen, capability-valid nominal intents | ≤0.1% target, with confidence interval and denominator |
| Deadline miss under declared load profile | 0 critical misses in the registered test suite |

The latency and false-veto goals are engineering targets, not standards. A zero observed event count is not proof of zero physical risk. The current mock suite does not measure end-to-end ROS scheduling, local planner time, actuator latency, obstacle avoidance, or task completion.

## Experiments required before comparing to industry

Run a matched benchmark with identical start/goal pairs, obstacle maps, base kinematics, sensor traces, disturbances, and random seeds:

- **B0:** platform-native planner/controller, no Symthaea task choice.
- **B1:** Symthaea intent selection feeding the same native planner/controller.
- **B2:** B1 plus runtime gate and adapter-side revalidation.
- **B3:** matched ablation without outcome feedback/replanning.

Report success/suboptimal/failure labels, collision/contact events, time-to-goal, path length, near-collision clearance, time-to-safe-state after injected faults, rejected/false-veto counts, request-to-controller latency p50/p95/p99/max, deadline misses, CPU/memory, and energy only if measured by a valid electrical/energy model. Use paired starts and crossed seeds; retain failed and missing trials in the primary analysis. Keep simulation, hardware-in-loop (HIL), and physical robot evidence as separate tiers.

This adapter is the reusable non-drone interface and conformance slice, not yet a runnable real robot implementation. The next integration requires one concrete local controller (for example, a ROS 2 Control base controller), a consistent monotonic clock, current safety telemetry, downstream expiry recheck, round-trip ControlOutcome feedback, and fault-injected simulator/HIL validation.

## Industry comparison

Use the same platform-native controller instead of comparing generic cognitive outputs to controller leaderboards. The implementation is intended to integrate above a platform-native navigation/control stack, preserving established control and safety layers; it does not replace their guarantees.

Relevant upstream references:
- ROS 2 Control: https://control.ros.org/master/doc/getting_started/getting_started.html
- PX4 Offboard mode and failsafe requirements (relevant to the separate drone integration): https://docs.px4.io/main/en/flight_modes/offboard
- NVIDIA Isaac Lab: https://github.com/isaac-sim/IsaacLab
- RoboDojo benchmark: https://github.com/robodojo/robodojo

## Claim ceiling

This PR provides typed semantic mapping and mock-sink conformance tests. It does not yet provide a ROS 2 node, actual local-controller binding, calibrated dynamics, physical-robot measurements, real-time guarantees, safe navigation proof, or an empirical SOTA improvement.
