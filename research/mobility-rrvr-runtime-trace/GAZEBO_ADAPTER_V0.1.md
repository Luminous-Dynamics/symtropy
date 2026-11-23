# RRVR Gazebo adapter core v0.1

**Issue:** [#7067 — execute frozen campaigns with plan-bound simulator receipts](https://github.com/Luminous-Dynamics/symtropy/issues/7067)  
**Core:** src/gazebo_adapter.rs  
**Preflight CLI:** rrvr_gazebo_preflight  
**Capability schema:** schemas/gazebo_capability_profile_v0_1.schema.json  
**Synthetic profile:** fixtures/gazebo_capabilities_positive_v0_1.json  
**Execution receipts:** [EXECUTION_EVIDENCE_V0.1.md](EXECUTION_EVIDENCE_V0.1.md)

## Scope of this increment

This is the transport-neutral core and mock-tested orchestration contract. It is not yet a concrete ROS 2 client and has not contacted or mutated a running Gazebo instance. The captured capability-profile preflight verifies the declared simulator/API/plugin identity against the compiled campaign plan; it does not establish that a live server advertises those features.

The adapter is a plan-in/receipts-out translator. It does not rewrite the compiled plan, issue native authority, or provide a physical actuation path.

## Pinned capability profile

The preflight binds:
- simulator ID and exact simulator build revision;
- ROS distribution and Simulation Interfaces package revision;
- world name and world-profile digest;
- reset-profile digest;
- SceneBroadcaster canonical system ID, revision, binary SHA-256, and loaded status;
- exact interface feature set reported by the simulator.

The required feature set includes simulator feature discovery, reset, get/set simulation state, world entity enumeration and entity state query. It requires one deterministic stepping interface: StepSimulation service or SimulateSteps action. It does not substitute a wall-clock delay or arbitrary topic silence if a required operation is unavailable.

SceneBroadcaster is mandatory because Gazebo's Simulation Interfaces depend on its synchronized world-state feed and reset detection. The official Gazebo docs explain that state-dependent operations can fail when the SceneBroadcaster is missing and describe its throttled updates (nominally 60 Hz while running, 30 Hz while paused) and the one-second synchronization timeout. The adapter therefore pins this plugin and treats missing/stale state as a hard failure, not as permission to continue. See [Gazebo ROS 2 Simulation Interfaces](https://gazebosim.org/docs/rotary/ros2_sim_interfaces/).

## Transport orchestration

The transport interface exposes:
- GetSimulatorFeatures / capability discovery;
- GetSimulationState and SetSimulationState (pause);
- ResetSimulation;
- StepSimulation or SimulateSteps;
- GetEntities and target resolution.

The core performs these stages:

1. **Preflight:** discover profile, compare exact simulator/ROS/API/SceneBroadcaster pins, query world state, and pause if necessary. A stopped or unknown simulation state is rejected.
2. **Reset:** execute the declared simulator snapshot reset exactly once, then verify a changed world epoch, step zero, paused state and exact initial-state digest. The first implementation accepts only the pinned simulator_snapshot_restore reset type.
3. **Schedule:** query current iteration and step exactly to the scheduled simulation iteration using the selected advertised stepping method. Overshoot, unexpected state, clock mismatch or world-epoch change aborts the event.
4. **Resolve target:** query the world again, then map logical component ID to world/entity, exact revision, target-profile digest and fault-domain ID. The binding carries world name and world epoch to prevent stale reuse across reset.
5. **Dispatch boundary:** derive a deterministic idempotency key from plan SHA, pair, scenario, fault and run epoch. A per-process registry suppresses duplicate calls; a real injector must also implement a durable, linearizable ledger so a process restart cannot erase idempotency.
6. **Reconciliation:** a lost dispatch response maps to Unknown. The adapter queries the same idempotency key; it never blindly redispatches. The ledger can report accepted, rejected, proven-not-dispatched, or unknown.
7. **Observation:** the observer contract is separate from the injector contract. A concrete implementation must return an observation from a separately qualified observer/fault domain and bind the same plan/event/world epoch/clock/step/signature into the execution-evidence bundle.

The current core contains the transport traits and deterministic orchestration. It includes no network client or Gazebo plugin implementation yet. Unit tests use a mock transport to exercise pause, reset/epoch verification, exact stepping, target binding, and dispatch idempotency.

## CLI

    cargo run --locked --manifest-path research/mobility-rrvr-runtime-trace/Cargo.toml --bin rrvr_gazebo_preflight -- compiled-plan.json source-manifest.json expected-plan-sha256 gazebo-capabilities.json report.json

The preflight CLI checks the compiled plan against exact source bytes and the external expected plan digest, then validates the captured capability profile. Its report explicitly sets:
- live_simulator_discovery_performed=false;
- simulator_mutation_performed=false;
- fault_injection_executed=false;
- execution_receipts_emitted=false;
- native_authority_created=false;
- physical_safety_established=false.

It is a static capability preflight, not a live Gazebo test.

## Why the standard Simulation Interfaces matter

Gazebo publishes a simulator-agnostic ROS 2 Simulation Interfaces set. The current docs describe:
- ResetSimulation;
- StepSimulation (service);
- SimulateSteps (action with feedback and cancellation);
- GetSimulationState and SetSimulationState;
- GetSimulatorFeatures;
- GetEntities and GetEntityState.

The feature-discovery call is important: the interface set is not assumed to be universally implemented by every simulator build. Gazebo's ros_gz_bridge documentation also notes that the bridge only supports certain message types; it should not be treated as a generic substitute for the standard simulation control/state interfaces.

References:
- [Gazebo: Use ROS 2 Simulation Interfaces to Interact with Gazebo](https://gazebosim.org/docs/latest/ros2_sim_interfaces/)
- [Gazebo Rotary Simulation Interfaces dependencies and limitations](https://gazebosim.org/docs/rotary/ros2_sim_interfaces/)
- [Gazebo ROS 2 integration and ros_gz_bridge limits](https://gazebosim.org/docs/latest/ros2_integration/)

## Qualification metrics for the live transport

When a concrete ROS 2 transport is added, publish:
- capability-discovery completeness and unsupported-feature failures;
- reset reproducibility and exact initial-state digest match rate;
- scheduled-versus-observed iteration skew (zero tolerance for discrete step service semantics where promised);
- world-epoch/clock mismatch kill rate;
- target-resolution success and stale-target rejection;
- injector dispatch/ledger reconciliation and blind-redispatch count;
- independently confirmed delivery rate (separate from dispatch acceptance);
- receipt completeness and replay-complete rate;
- p50/p95/p99 capability, pause, reset, step, target-resolution, dispatch, lookup and observer latency;
- SceneBroadcaster state-update latency, timeout rate and tracing overhead.

Do not collapse these into one score. Safety/evidence correctness, recovery utility and performance remain separate dimensions. Zero unconfirmed events silently treated as delivered and zero blind redispatch are hard gates, not weighted objectives.

## Claim ceiling

Passing the mock tests and capability-profile preflight validates only the transport-neutral orchestration logic against a synthetic profile. It does not prove a live Gazebo build implements the declared feature set, execute a reset/step, inject a fault, authenticate injector or observer artifacts, qualify a physical fault domain, establish HIL evidence, or prove robot safety.

The next integration is a version-pinned ROS 2 service/action client implementing GazeboSimulationTransport, followed by real simulator receipts and the execution-evidence validator. Hardware-in-the-loop and physical qualification remain separate stages.


## Cross-event trace ordering

The execution-evidence contract checks one continuous simulation clock from reset through every fault event. It requires both phase-local order and cross-event nondecreasing step/time coordinates. This catches a replay where each fault's local receipts appear valid but the sequence of fault events is globally reordered.


## Supported ROS/Gazebo release pairing and profile provenance

The campaign manifest and compiled plan now carry an explicit Gazebo release label as well as the exact simulator revision and Simulation Interfaces package revision. The v0.1 supported LTS pairings are:

- ROS 2 Jazzy + Gazebo Harmonic on Ubuntu Noble 24.04;
- ROS 2 Lyrical + Gazebo Jetty on Ubuntu Resolute 26.04.

The adapter rejects unreviewed cross-pairings, including Jazzy + Jetty. This follows Gazebo's current installation/compatibility guidance rather than assuming that any ROS distribution can be combined with any Gazebo release. See the Gazebo ROS installation matrix: https://gazebosim.org/docs/latest/ros_installation/.

The checked-in profile is explicitly a **synthetic contract fixture**: it carries Jazzy/Harmonic labels so it conforms to a supported pair, but its simulator, plugin, interface, world and reset identifiers are fixture placeholders, not hashes captured from a live Gazebo installation. Preflight PASS only checks those captured strings against the plan. A real qualification must capture the actual package versions, package/binary digests, server configuration, SceneBroadcaster build and world profile from the target container.

## Independent world iteration witness

A successful StepSimulation service/action response is not treated as proof that the requested iteration occurred. The transport must also supply a pinned Gazebo world-statistics receipt from the exact world stats topic. The adapter compares the message type, producer/build, topic, clock, iterations and sim_time with the world snapshot. A mismatch between service-control result and published world statistics fails closed. The topic path, raw artifact ID and digest are carried as measurement evidence; this closes the control/measurement distinction but does not authenticate the telemetry producer.


## Fresh world-statistics samples and reset epochs

A WorldStatistics record is not just an iteration counter. Each captured sample is tagged by the adapter with the current world epoch, simulation clock ID, collector monotonic-clock ID and receive timestamp. After reset, the first stats sample must be a new artifact in the new world epoch and later on the same collector clock. After a discrete step, the adapter must observe a newer stats artifact whose iterations and sim_time match the requested step. A stale sample from a previous epoch or an old message cached by the transport is rejected even when its numeric iteration count happens to match.

The WorldStats stream is documented as throttled to 5 Hz on the server API, so a live transport must wait for the next matching sample using a bounded telemetry timeout. The timeout is only for observing telemetry; physics advances only through the explicit step interface. No wall-clock sleep or stale cached message can count as a step receipt. The synthetic test transport advances its collector clock deterministically; it is not a live Gazebo proof.
