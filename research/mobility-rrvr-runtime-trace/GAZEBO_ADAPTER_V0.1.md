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


## Plan-derived dispatch composition extension

The new dispatch_planned_fault_once function is the composed plan-to-injector boundary. It does not accept a free-form fault event: it first verifies the compiled plan, selects the named scenario and fault from that plan, and revalidates the capability profile and stepping method carried by the session. It then:

1. Advances to the exact planned simulation iteration.
2. Resolves the logical component to its pinned world/entity/revision/profile/fault-domain binding.
3. Re-reads the world immediately before dispatch and requires the same world epoch, clock identity, paused state, exact iteration, simulation time and world-state digest as the cut-point snapshot.
4. Resolves the target a second time and requires an identical binding.
5. Derives the idempotency key from plan, pair, scenario, fault, run and run epoch, constructs the request from the committed fault definition, and makes at most one dispatch call through the registry.

A changed world digest or target binding is not retried; it becomes a failed precondition for dispatch. A lost injector response remains Unknown and must be reconciled by looking up the same idempotency key. The result includes the exact cut snapshot, target binding, plan-derived request and dispatch outcome. It does not emit authenticated execution receipts or grant retry/actuation authority.

### Important residual: Gazebo does not provide an exclusive-world lease

The final re-read narrows the time-of-check/time-of-use window; it does not make a read followed by a dispatch atomic. Gazebo's standard Simulation Interfaces do not, by themselves, prevent another ROS/Gazebo client from mutating the same world between the final snapshot and injector acceptance.

A live campaign must therefore demonstrate exclusive mutation ownership. The preferred deployment is a dedicated simulator process/ROS domain in which only the campaign coordinator can invoke world-mutating services, with all reset/spawn/delete/set-state clients routed through that coordinator. A lease/lock that is not enforced by every mutator is not evidence of exclusivity. If this property cannot be established, report the campaign as external_mutation_uncontrolled and do not promote it to a race-free live qualification.

Gazebo's current documentation adds concrete reasons for this gate:
- ResetSimulation can be detected through SceneBroadcaster's /scene/info; spawning an entity concurrently with reset can trigger reset detection early.
- The documented GetEntities/GetEntitiesStates interfaces enumerate top-level models; nested models are not included, and GetEntityState expects a model entity. Target resolution must not silently treat a nested link or a partial entity inventory as complete.
- Simulation Interfaces service calls synchronize with SceneBroadcaster state updates. The default throttles are nominally 60 Hz while running and 30 Hz while paused, while step completion feedback is tracked from the separate /stats stream at 10 Hz. The documented one-second synchronization timeout is a wall-clock timeout, not a simulation-time bound.

Consequently, the live adapter must record separately: (a) requested and observed simulation iterations/time, (b) wall-clock request/response latency, and (c) SceneBroadcaster state synchronization latency. Do not infer simulation-step completion from wall-clock sleep or infer world freshness from the 10 Hz stats stream alone.

References: [Gazebo Simulation Interfaces: dependencies, timing and known limitations](https://gazebosim.org/docs/rotary/ros2_sim_interfaces/); [Gazebo latest Simulation Interfaces](https://gazebosim.org/docs/latest/ros2_sim_interfaces/).


## Payload-bound durable dispatch idempotency

A stable idempotency key must not be allowed to alias two different fault requests. The dispatch request now carries a SHA-256 commitment over a canonical JSON projection of the plan, campaign, pair, scenario, fault ID/sequence, target ID, fault domain, scheduled simulation step, fault-parameter digest and idempotency key. The payload digest excludes only its own digest field. The coordinator verifies the digest before calling the injector.

The in-process registry is only a fast duplicate/conflict guard. It is intentionally not treated as restart-safe storage. The concrete injector owner must atomically and durably store the mapping:

idempotency_key -> payload_sha256 -> operation outcome

before any fault effect can occur. Required semantics:
- same key + same payload digest: return the previously recorded operation result without executing the effect again;
- same key + different payload digest: reject with PayloadConflict and do not execute;
- unknown ledger result or unavailable lookup: remain unresolved; never blind-resubmit;
- ledger survives coordinator and injector process restarts, and its durability/linearizability are tested against the actual injector store.

The Rust reference tests include an in-memory durable-ledger test double showing payload conflict rejection after a coordinator-registry restart. This is not evidence that a real fault injector already implements a durable ledger. Exactly-once physical effects are not claimed; the target is at-most-once effect admission under a verified, durable injector contract plus independent outcome observation.


## Blind observer evaluation and three-valued delivery verdict

The observer API receives the target, fault identity, world epoch and simulation step, but it is not passed the plan's expected delivery-signature digest. The expected signature remains with the independent assessor. This avoids making a supposedly independent observer echo or tune its measurement to the expected answer.

Once observer identities, binaries, epochs, steps and disjoint common-cause domains have passed structural qualification, the assessor returns one of three results:
- Confirmed: every qualified observer reports the same signature, and it equals the plan-committed expected signature;
- Refuted: every qualified observer reports the same valid signature, but it differs from the plan-committed expected signature;
- InsufficientEvidence: qualified observers disagree on the signature.

Missing members, duplicate/unpinned observers, stale epochs, invalid digests and common-cause overlap remain hard qualification errors rather than being converted into a verdict. The observer verdict is evidence classification only; it does not authorize an action, retry or physical mutation.


The request commitment includes the explicit canonicalization identifier serde-json-value-sorted-keys-v0.1. This lets an injector implementation reproduce the digest outside Rust without guessing a serialization profile. The standard industry pattern of storing the original request parameters with a client idempotency token and rejecting parameter changes is also described in [AWS Builder's Library: Making retries safe with idempotent APIs](https://aws.amazon.com/builders-library/making-retries-safe-with-idempotent-APIs/). RRVR applies that principle to a physical fault command, where a payload mismatch must be a hard conflict rather than a second target mutation.

This implementation advances part of [#7148 — crash-safe dispatch journaling and blinded observation](https://github.com/Luminous-Dynamics/symtropy/issues/7148). The append-only write-ahead journal, durable external injector implementation and authenticated raw observation artifacts remain separate acceptance items under that issue.


## Verified-reset prerequisite for plan-derived dispatch

The composed dispatch function now refuses a session created only by capability preflight. A GazeboAdapterSession carries a private reset-verification marker, which is populated only after reset_and_verify has:
- revalidated the exact compiled plan and live simulator capability profile;
- confirmed the current world epoch, clock and paused state against the session;
- executed the campaign-declared reset;
- verified a changed world epoch, paused state, step zero and the exact initial-state digest; and
- retained the resulting epoch/clock and exact capability profile in the session marker.

The dispatch path checks that the public session fields still equal the private marker, re-queries the live simulator capability profile and requires exact equality with the profile captured at reset. A preflight-only session, a caller-mutated epoch/profile/clock, a live capability drift, or a repeated reset attempt is rejected before injector dispatch. The preflight and reset functions also verify the compiled plan before issuing simulator operations, so a tampered plan cannot cause a reset.

This marker is an in-process API invariant, not a durable reset receipt. It cannot establish that a concrete transport's reset record survived coordinator restart; the actual live adapter must persist reset receipts in the append-only ledger specified by #7148. A fresh coordinator after restart must not reconstruct a verified session merely from a remembered epoch string; it must reconcile the durable reset/evidence record and re-read live simulator state before resuming.


### Ambiguous reset outcomes are non-retryable

Reset lifecycle is now explicit: NotAttempted -> DispatchStarted -> Verified, with OutcomeUnresolved entered whenever the reset call or its immediate state observation is ambiguous or the returned state fails the reset contract. Once ResetSimulation might have been sent, reset_and_verify refuses a second reset request. It returns ResetOutcomeUnknown on a lost/ambiguous response and ResetOutcomeUnresolved on any subsequent attempt to reset without reconciliation.

The read-only reconcile_reset path never calls ResetSimulation. It re-reads live capabilities and world state, then promotes the session to Verified only if the observed world proves a new epoch, the campaign-pinned initial state digest, paused state, step zero and matching clock. Otherwise the session remains unresolved. This is deliberately conservative: a timeout is not proof that reset did not happen.

The regression test simulates the difficult cut: the mock applies the reset, then returns a transport error as though the response were lost. A second reset request is blocked; dispatch is also blocked while unresolved; read-only reconciliation observes the exact committed initial state and marks the session verified without a second reset.

The state machine is still in-memory in this contract-only layer. A coordinator crash can erase DispatchStarted/OutcomeUnresolved; the concrete adapter must therefore persist reset intent before the request and reconcile from the #7148 append-only journal after restart.
