# Symthaea–Symtropy Legacy Motor Authority Boundary v0.1

**Status:** research/draft. This change is an authority-boundary hardening step, not a certified safety mechanism or a physical-robot qualification.

## Finding

The generic Bevy robotics bridge currently takes \`CognitiveBrain.motor_output\` and converts each indexed value directly into joint torque using a fixed scalar gain and a single xy-plane hinge assumption. The bridge's own comments state that this is not a realistic actuator/controller model and that platform demos requiring PD/admittance control implement their own controllers.

A generic cognitive output vector is neither an embodiment-specific motion plan nor a validated actuator command. The default system must not quietly confer torque authority on every entity that has a CognitiveBrain and robotic-joint children.

## Change in this branch

- Add \`LegacyDirectMotorDrive\`, an explicit per-root-robot opt-in marker for the old direct-torque fallback.
- Require this marker in the query that reads \`CognitiveBrain.motor_output\` for direct actuation. Without it, the legacy system does not apply torque for that robot.
- Export the marker so prototypes that intentionally use the fallback must opt in explicitly.
- Reject NaN/infinite motor outputs and clamp finite values to the documented normalized interval [-1, 1] before the fixed torque conversion.
- Add unit tests for the explicit marker requirement and normalized-output fail-closed behavior.

This preserves the prototype path but makes the authority grant visible in the entity setup. Adding the marker is not a safety certification and should be limited to controlled simulation/research. Production robot embodiments should use their native local controller with independently enforced limits and the semantic-intent dispatch boundary.

## Paradigm shift

Move from *cognitive output is actuator authority* to *cognitive output is a proposal consumed by a platform-specific controller*. Shared cognition should select intents or skills. Robot-specific adapters own kinematics, dynamics, control gains and actuator constraints. An independent safety authority retains the ability to veto during execution. Every accepted action returns an explicit outcome so cognition can update its belief and re-plan.

## Falsifiable benchmark plan

### Baselines

1. **Legacy direct torque, opt-in:** current fixed-gain mapping, restricted to simulation fixtures.
2. **Direct torque with explicit marker and finite/range gate:** implementation in this branch.
3. **Platform-native local controller:** task-level controller without Symthaea motor output.
4. **Symthaea semantic intent → platform-native controller + safety authority:** target architecture when the real adapter is connected.

Use identical initial states, task targets, robot dynamics, perturbations, training data, and random seeds. Compare the exact same observations and disturbance schedules; record source SHAs and raw per-trial traces.

### Deterministic conformance fixtures

| Fixture | Expected result |
| --- | --- |
| Robot has no legacy opt-in marker | cognitive output causes zero direct torque applications |
| Robot has explicit prototype opt-in and finite output in [-1, 1] | normalized value is passed to legacy conversion |
| Output is NaN or ±infinity | no direct torque command is applied for that output |
| Finite output is outside [-1, 1] | value is clamped before torque conversion |
| Semantic request fails the runtime gate | once wired to the dispatch path, zero controller submissions |
| Sensor or actuator fault occurs during execution | local safety/controller response is evaluated continuously; pre-dispatch admission alone is insufficient |

### Metrics

Report separately:
- **Authority correctness:** direct-drive applications without the explicit opt-in marker; target zero.
- **Numeric validity:** non-finite outputs that reach torque conversion; target zero.
- **Bounds correctness:** normalized outputs outside [-1, 1] after validation; target zero.
- **Task outcome:** success, suboptimal result, failure, collision/contact and recovery state.
- **Control quality:** tracking RMSE, maximum error, settling time, constraint violations, and time to safe state after faults.
- **Efficiency:** energy per successful task (measured electrical energy if available; simulator thrust impulse is not electrical energy), compute latency p50/p95/p99, and deadline misses.
- **Transfer:** performance by robot and held-out environment/model parameters, never just a pooled average.

The deterministic code targets can be tested immediately. Claims about task performance require the native-controller and cognitive-policy experiments. Physical safety claims require robot-specific hazard analysis, qualified hardware safety functions, HIL, and supervised physical testing.

## Proposed acceptance targets (not results)

- 100% of deterministic fixtures have the expected direct-drive behavior.
- Zero torque applications to unmarked robot roots in simulation tests.
- Zero non-finite values passed to torque conversion.
- No claim of real-world advantage until paired task-level trials show an improvement and uncertainty intervals support it.
- The native controller and safety authority remain enabled in all HIL and physical trials; no benchmark disables platform failsafes.

## Industry comparison

ROS 2 Control organizes controllers, hardware interfaces, and control-loop execution; PX4 defines explicit offboard-control health and failsafe behavior. Symthaea–Symtropy should integrate above these mechanisms, not replace them with a generic cognitive vector. See:

- https://control.ros.org/master/doc/getting_started/getting_started.html
- https://docs.px4.io/main/en/flight_modes_fw/offboard

The strongest eventual comparison is not raw motor output versus a mature controller. It is the same platform-native controller and safety stack, with and without Symthaea's cognitive intent selection, active sensing and feedback-driven replanning, under identical fault-injected tasks.
