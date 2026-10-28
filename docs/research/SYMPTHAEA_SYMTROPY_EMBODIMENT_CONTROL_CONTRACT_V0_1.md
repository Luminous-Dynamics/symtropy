# Symthaea × Symtropy: cross-embodiment control boundary v0.1

**Status:** Foundational interface slice. It defines typed request and outcome contracts only. It does not run a controller, drive actuators, validate a physical robot, or certify safety.

## The intended division of responsibility

Symthaea should be the cognitive decision system that predicts, forms beliefs, selects goals, chooses skills, responds to outcome feedback and decides when to seek information or assistance. Symtropy should provide the physical world, simulation, embodiment adapters, instrumentation and scenario-level evaluation. Platform-specific controllers retain the fast motor loop and domain-specific control authority.

This is not “one brain emits the same motor vector for every body.” A quadrotor needs a flight-control interface; a wheeled robot needs navigation and velocity control; a manipulator needs kinematics and joint/impedance control; a quadruped/humanoid needs gait or whole-body coordination; an AUV needs buoyancy/depth/attitude/velocity handling. The common interface is the **semantic intent and request/outcome lifecycle**, not the actuator mathematics.

## Current repository seams

- The multirotor integration uses symthaea_multirotor::FlightController and QuadrotorCommand against SymtropyFlightSimulator in the failure-aware rollout research branch. It demonstrates the controller-to-physics seam, but the simulator is explicitly a simplified spherical rigid-body/mean-inertia model, not a calibrated Crazyflie/MuJoCo-equivalent plant.
- symtropy-robotics-bridge-core has RoboticAgent, MotorPlanner, UniformGainPlanner, and JointSafetyAuthority. The scalar gain is a cap, not a meaningful general motion command.
- symthaea-bevy-brain carries generic CognitiveBrain.motor_output, and the generic Bevy robot path maps indexed outputs to torque. Treat this as a prototype/baseline, not a production control strategy for arbitrary platforms.
- Existing issue #1183 already proposes typed embodiment and controller feedback. This PR lands the first dependency-light request/outcome contract without replacing the existing APIs.

## New contract in symtropy-robotics-bridge-core

### Control request

ControlRequest contains:
- stable non-zero request ID;
- monotonic issue and expiry timestamps from one declared clock;
- belief confidence, explicitly not a motor-authority multiplier;
- typed EmbodiedIntent.

The v0.1 vocabulary is deliberately small: navigate to a pose, hold, inspect, reach, manipulate, stabilize, stop, and request assistance. Pose3 carries an explicit reference-frame ID, translation, and normalized quaternion. Validation rejects non-finite targets, unknown/empty frame labels, invalid speeds, malformed quaternions, invalid confidence, future-dated requests, and expired requests.

This generic validation does **not** prove the target exists, the robot supports an intent, the frame transform is correct, collision avoidance succeeds, or the action is safe. An adapter must validate capability, transform convention, freshness of underlying sensor evidence, feasibility, local limits, and any mode-specific preconditions.

### Control outcome

ControlOutcome reports the same request ID, lifecycle status, observation time/tick, applied fraction, machine-readable constraint causes and telemetry references. The lifecycle validator rejects request-ID changes, backwards time/ticks, missing constraint causes, and transitions after a terminal status.

Execution feedback matters: if a local controller constrains/vetoes an intent, Symthaea must receive that outcome and use it as new evidence for replanning, rather than repeatedly assuming the original intent succeeded.

### Authority rules

1. A request is a proposal, not an actuator command.
2. Local controllers own platform-specific closed-loop control.
3. Safety authority validates constraints independently and may veto.
4. Invalid or expired proposals fail closed.
5. Constraints/vetoes are explicit outcomes; they are not silently treated as success.
6. All commands and outcome evidence retain request identity.
7. This shared contract does not require symtropy-robotics-bridge-core to depend on symthaea-core; platform products can write the adapter.
8. Simulation oracle state must not leak into the cognitive observation unless the registered observation contract permits it.

## Architecture target

    qualified sensors / telemetry
              ↓
    typed embodied observation + provenance
              ↓
    Symthaea belief / self model / task cognition
              ↓
    ControlRequest (semantic intent + expiry)
              ↓
    embodiment adapter: capability, frame, freshness and feasibility checks
              ↓
    platform controller (fast local loop)
              ↓
    independent safety authority / hard constraints
              ↓
    Symtropy physics or hardware adapter
              ↓
    ControlOutcome + telemetry references
              ↺ Symthaea updates belief and replans

A full observation model, intent arbitration, capability map, and concrete per-platform actuator drivers are subsequent slices. This PR does not yet wire the contract into a drone run or other live robot.

## Research and industry comparisons

ROS 2 Control separates controller plugins from hardware interfaces and manages controller lifecycle and its control loop. See the [ROS 2 Control architecture documentation](https://control.ros.org/master/doc/getting_started/getting_started.html).

PX4's current offboard documentation warns that external setpoints require explicit ownership, correct frame semantics and continuing proof-of-life messages; conflicting or stale setpoints can produce dangerous behavior. See [PX4 Offboard Mode](https://docs.px4.io/main/en/flight_modes/offboard).

The design here is compatible with those principles but is not a replacement for ROS 2 Control or PX4 safety logic. A future adapter must map the typed request to supported PX4 setpoints/modes, respect offboard-loss behavior, and keep the autopilot/local safety mechanisms authoritative. Similar platform-specific constraints apply to manipulators and mobile robots.

## Benchmark plan

Maintain separate scoreboards instead of a unit-mixed score:
- intent feasibility and task completion;
- controller tracking, settling/recovery and action-completion rate;
- safety constraint activations/veto correctness and hard-boundary violations;
- robustness to stale/missing sensors, actuator degradation, delay, dropped commands and regime shift;
- p50/p95/p99/max controller and end-to-end intent-to-action latency, memory and deadline misses;
- transfer across declared simulator configurations, then HIL, then physical hardware as separate evidence tiers.

Minimum contract gates:
- all malformed, future-dated and expired request fixtures rejected;
- every accepted request has an explicit terminal outcome or timeout record;
- every constraint/veto has a machine-readable cause and feeds back to cognition;
- no silent request-ID change, missing outcome, or time/tick reversal;
- matched-task and matched-compute comparisons against the platform's existing controller before claiming a gain;
- no physical-flight, hardware-generalization or safety claim from synthetic tests.

Research targets to add after adapters exist (not standards or achieved results): no hard-boundary regression against the local-controller baseline; zero silently omitted terminal outcomes; 100% rejection of invalid envelope fixtures; and p99 latency within a preregistered per-platform control budget.

## Next implementation slices

1. Wire request validation and outcome lifecycle to the Symtropy Bevy adapter, retaining old APIs during migration.
2. Create a drone adapter that maps approved requests to the existing symthaea_multirotor::FlightController and QuadrotorCommand pipeline; measure the full request-to-action latency, not only the HDC/LTC forward pass.
3. Add one non-drone vertical slice (prefer mobile navigation or manipulator reach) that routes intents through that platform's actual controller and returns outcomes. Do not route arbitrary CognitiveBrain.motor_output directly into platform actuators as the generic production path.
4. Add adversarial tests for stale sensors, unsupported intent, frame mismatch, veto/replan feedback, dropped/duplicated request, controller timeout, actuator saturation and reload/interruption.
5. Only later evaluate HIL/real hardware through the relevant platform protocols.

Related work: [#2828](https://github.com/Luminous-Dynamics/symtropy/issues/2828), [#1183](https://github.com/Luminous-Dynamics/symtropy/issues/1183), [#1176](https://github.com/Luminous-Dynamics/symtropy/issues/1176), [#2783](https://github.com/Luminous-Dynamics/symtropy/issues/2783), [#2796](https://github.com/Luminous-Dynamics/symtropy/pull/2796).
