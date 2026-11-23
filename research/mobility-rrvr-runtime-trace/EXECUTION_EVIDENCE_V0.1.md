# RRVR fault execution evidence bundle v0.1

**Issue:** [#7051 — bind injection requests to independent fault-delivery observations](https://github.com/Luminous-Dynamics/symtropy/issues/7051)  
**Validator:** src/execution_evidence.rs  
**CLI:** rrvr_execution_validate  
**Schema:** schemas/execution_evidence_bundle_v0_1.schema.json  
**Fixture:** fixtures/execution_evidence_positive_v0_1.json  
**Plan:** [CAMPAIGN_PLAN_V0.1.md](CAMPAIGN_PLAN_V0.1.md)

## Why this contract exists

A planned fault is not an injected fault. An injector API response is not proof that the target experienced the fault. A fault-injector self-report is not independent observation.

The v0.1 contract records four separate phases for every scheduled event:
1. target resolution to the exact platform component and revision;
2. dispatch request/response from the pinned native adapter;
3. execution receipt from the pinned fault injector;
4. delivery observation from a distinct observer with a distinct declared fault domain.

Each phase has a separate evidence ID, producer/revision, raw artifact ID/digest, run/world epoch, event cut point, target ID, clock ID, simulation step and simulation-time coordinate.

## Plan and event binding

The bundle carries the exact:
- plan SHA-256 and source-manifest SHA-256;
- campaign, pair, scenario, arm and seed;
- platform binding: simulator, ROS distribution, middleware, controller, driver, firmware, calibration, observer stack, fault injector ID/revision/binary digest/fault domain, native adapter ID/revision and runtime-monitor revision;
- run ID, process/run epoch and simulator world/reset epoch.

Each event embeds the full planned fault record from the plan: ordered sequence, scheduled simulation step, fault ID, cut point, target layer/component/revision/profile digest, target fault domain, fault kind, parameter digest and expected delivery-signature digest. The embedded planned fault must equal the exact event at that position in the compiled scenario schedule.

The target-resolution receipt must bind the logical component to the actual entity/plugin/endpoint, exact revision and target-profile digest. A later controller or firmware revision cannot inherit the plan silently.

## Four distinct verdicts

| Phase | States | What it proves in this bundle |
|---|---|---|
| Target resolution | resolved / not_resolved / unknown | Whether the requested logical target was mapped to the planned platform revision |
| Dispatch | accepted / rejected / unknown | Whether the pinned adapter records acceptance/rejection of the request |
| Injector execution | executed / not_executed / unknown | Whether the pinned injector records execution at the scheduled simulation step |
| Delivery observation | confirmed / refuted / unknown / not_observed | Whether a distinct observer artifact reports the exact expected fault signature |

A dispatch accepted without an execution receipt is not executed. An execution receipt without a matching delivery observation is not confirmed. A Confirmed delivery requires all of:
- resolved target ID, revision, profile digest and target fault domain match the compiled fault;
- dispatch is Accepted and dispatch receipt comes from the pinned native adapter ID/revision;
- execution is Executed at the exact scheduled simulation step and receipt comes from the pinned injector ID/revision;
- observation is Confirmed, receipt producer matches the declared observer ID/revision, its artifact is distinct, its declared observer fault domain differs from the injector fault domain, and its delivery-signature digest exactly matches the expected signature committed in the plan.

The declared domain distinction is a conservative structural requirement, not proof of universal physical independence. The observer's authenticity and physical fault model remain separately qualified.

## Unknown outcomes: reconcile, never blindly reinject

An Unknown dispatch/execution/delivery state sets reconcile_required. The CLI always reports retry_permitted=false: this validator is not a retry or actuation authority.

A production adapter must query its durable dispatch ledger by the same idempotency key, reconcile the run/world epoch and inspect execution receipts before it can propose any next event. A timeout or missing acknowledgement is not proof that the injector did not execute.

An explicit Rejected dispatch and NotExecuted receipt can establish a non-delivery record. An Unknown state cannot be coerced into NotExecuted. NotObserved is also distinct from Refuted: absence of an observation is not evidence that the event did not happen.

## Clock and identity discipline

Reset evidence and all event receipts use one simulation clock ID and epoch. For each event, receipt coordinates are monotonic in phase order: target resolution, dispatch, injector execution, delivery observation. Injector execution must be recorded at the plan's exact scheduled simulation step; an observation may occur later, but never earlier than execution.

Evidence IDs, raw artifact IDs and raw artifact digests are unique across the bundle. Dispatch IDs and idempotency keys are unique across events. A later retry/reconciliation is a new bundle/run linked to the same campaign/plan and original logical effect; it cannot mutate or overwrite this bundle.

## CLI and CI

Validate a bundle against its exact compiled plan and source manifest:

    cargo run --locked --manifest-path research/mobility-rrvr-runtime-trace/Cargo.toml --bin rrvr_execution_validate -- compiled-plan.json source-manifest.json expected-plan-sha256 execution-bundle.json report.json

The CLI checks:
- compiled-plan internal integrity, source-manifest bytes and external plan digest;
- exact scenario, pair, arm, seed, platform and full fault schedule;
- verified reset status and initial-state commitment;
- target revision/profile/fault-domain resolution;
- producer IDs/revisions for adapter, injector and observer phases;
- all receipt IDs/artifacts/digests, run/world epochs and comparable clock coordinates;
- exact delivery signature for Confirmed and mismatch for Refuted;
- reconciliation requirement and permanent non-authority claim ceiling.

CI uses a synthetic four-event known-answer bundle and verifies:
- all four deliveries are confirmed only with execution and separate observer evidence;
- accepted dispatch without execution/observation decreases the confirmed count and requires reconciliation;
- an unknown dispatch/execution requires reconciliation and never permits retry;
- a mismatched delivery signature is rejected.

## Research/industry positioning

The work composes rather than replaces the strongest existing benchmarks and tools:

- RoboRecover evaluates recovery policies from reconstructed execution-deviation states. RRVR should add an evidence/authority overlay to matched recovery episodes, not claim a superior policy Recovery Success Rate. [RoboRecover (2026)](https://arxiv.org/abs/2609.28952)
- SafeVLA-Bench reports native task success separately from safety, successful-but-unsafe episodes and violation severity. RRVR adds a distinct execution-evidence verdict and must keep it separate from both success and safety. [SafeVLA-Bench](https://safevla.org/leaderboard.html)
- The 2026 ROS fuzzing survey reviews 92 selected works and finds repeatable execution/reset and diagnosable failure analysis less mature than input modelling, feedback and oracle construction. This receipt bundle directly targets that reproducibility/diagnosis gap. [Yang et al. (20 July 2026)](https://doi.org/10.3390/app16147250)
- Gazebo's ROS 2 Simulation Interfaces expose simulator reset, pause/play, step, state query and world/entity operations, including a step action with feedback/cancellation. These are appropriate primitives for a later adapter; using them does not itself prove a fault was delivered. [Gazebo ROS 2 Simulation Interfaces](https://gazebosim.org/docs/latest/ros2_sim_interfaces/)
- ROS 2 REP 2014 recommends grey-box benchmarking across individual nodes and complete graphs using LTTng/ros2_tracing. RRVR should reuse that instrumentation baseline and add the semantic receipts linking the injector, target and independent observation. [REP 2014](https://reps.openrobotics.org/rep-2014/)

## Claim ceiling

A PASS establishes internal consistency and linkage between supplied artifacts and the exact compiled plan. It does not authenticate the artifact producers, prove simulator/robot state is truthful, prove universal physical observer independence, execute a fault injector, establish HIL/physical evidence, issue native action authority, grant retry permission, or establish robot safety.

The next integration is a Gazebo adapter that consumes the canonical plan, pauses/steps the simulator deterministically, emits a separate dispatch and execution receipt, captures independent target-state evidence, and never rewrites the plan. ROS runtime and controller/driver adapters follow the same interface; HIL is a distinct later qualification tier.


## Gazebo adapter boundary

The simulator adapter core uses the compiled plan's exact Gazebo build, Simulation Interfaces revision and SceneBroadcaster binary pin. Its transport orchestration advances only by discrete simulation steps, resolves each target in the current world epoch, and separates dispatch from execution and observer receipts. A timed-out dispatch becomes Unknown and is reconciled against the same idempotency key; it is never silently resent.

See [GAZEBO_ADAPTER_V0.1.md](GAZEBO_ADAPTER_V0.1.md). The current implementation is mock-tested and has no live ROS 2/Gazebo transport yet.


## Cross-event chronology

The validator now checks chronology across the ordered fault schedule, not only within each event. After target/dispatch/execution/observation receipts for event N, the first available receipt for event N+1 may not move backwards in simulation step or simulation time. All events retain the reset receipt's clock identity and run/world epochs. A locally ordered event cannot be replayed out of sequence in the larger campaign trace.


## Simulator-control versus world-state evidence

The step-service/action receipt proves only what the control endpoint returned. The simulator transport must additionally capture a receipt from the pinned Gazebo world-statistics topic, including source build, raw artifact digest, clock, iterations and sim_time. The adapter refuses to accept a cut point until the observed statistics agree with the requested step and the world-state snapshot. The current reference is a synthetic test contract, not a live Gazebo stream. See GAZEBO_ADAPTER_V0.1.md.


## World-statistics freshness evidence

Every fault event now includes a distinct WorldStatistics observation artifact. The reset receipt also commits the initial WorldStatistics artifact, topic/message type, collector-clock identity and monotonic receive time. Per-event samples must use the same collector clock, carry the exact run/world epoch and simulation clock, report the planned iterations/sim_time, and have a strictly increasing receive timestamp after reset and each preceding event. This rejects stale/replayed samples that merely match a target iteration numerically. The collector monotonic clock is separate from the simulator clock; both identities are preserved in the final bundle.
