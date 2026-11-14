# Real Symthaea-in-Symtropy Closed-Loop Evaluation v0.1

**Status:** integration-smoke contract and test harness. No task-performance, reliability, safety, or SOTA claim is made.

## The correction

Symtropy's public standalone build intentionally substitutes two dependencies:
- `stubs/symthaea` returns zeroed cognitive outputs/telemetry.
- `stubs/symthaea-orbital` is an `EmbodimentBridge` stand-in and explicitly is not a physics simulator.

Therefore, a green standalone test that uses those stubs establishes API/wiring compatibility only. It must not be described as evidence that the full Symthaea cognitive system controls a Symtropy world. Separately, `symtropy-symthaea-adapter` calls an `EmbodimentBridge` whose `step` advances the platform-specific implementation; that adapter is not, on its own, evidence that Symtropy's `PhysicsWorld` is the physics backend for the resulting behavior.

Sources in this repository:
- `stubs/symthaea/src/lib.rs`
- `stubs/symthaea-orbital/src/lib.rs`
- `crates/bridges/symtropy-symthaea-adapter/src/lib.rs`
- `crates/bridges/symtropy-robotics-bridge/src/agent.rs`

## Tranche 0: real-source closed-loop smoke

The excluded package `integration/symthaea-physics-loop` is run by a dual-checkout workflow. The job checks out an exact Symthaea commit, temporarily replaces standalone stubs with paths into that checkout, and runs a headless Bevy test. Test phases are explicit and ordered:

```text
Symtropy PhysicsWorld state
  -> robotic sensor/HDC encoding
  -> real Symthaea cognitive cycle
  -> motor output
  -> actuator torque on Symtropy PhysicsWorld
  -> next physics state / next observation
```

The one-body torsional probe intentionally aliases the actuator handle to the sensed body handle. This is solely an integration probe that proves the signal loop closes; it is not an articulated robot model, task benchmark, or claim about physical fidelity. The test fails if real Symthaea yields only zero motor outputs, the commands do not change Symtropy angular velocity, or the changed physical state is not reflected in the subsequent observation.

The standalone/stub build remains useful for quick API checks. The real-source integration lane is separate because it compiles a materially different dependency graph. A passing smoke test proves only that a real cognitive output reached Symtropy physics and that physics state returned to the observation path.

## Causal scheduling contract

Each fixed tick must obey this order:
1. Refresh sensor/proximity state and encode the current physics observation.
2. Run the Symthaea cognitive cycle on that observation.
3. Apply the resulting motor output.
4. Advance Symtropy physics (the integration test does this explicitly after the fixed schedule).

The shared `SymthaeaBrainSet::CognitiveCycle` set and `RoboticBrainSet` ordering make steps 1–3 explicit instead of leaving the two plugin systems unordered in Bevy's schedule.

## Tranche 1: task-level paired benchmarks

After the smoke qualifies, add task-level, matched-seed comparisons in Symtropy:
- **Tasks:** target tracking/hover, point-to-point movement, obstacle approach-and-retreat, one-joint reach, then domain-specific platforms.
- **Policies:** a conservative classical controller (PD/PID where appropriate), Symthaea as shipped, and predeclared ablations (disable memory, predictive state, or selected observation channels). Never tune against the final test split.
- **Disturbances:** sensor dropout/staleness, observation bias, force/wind pulses, actuator saturation, and resource depletion; use identical events for every policy in a matched pair.
- **Metrics:** task success, tracking RMSE, collision/severity events, recovery time, useful work, energy, p50/p95 control latency, operator intervention, and worst-stratum performance. Report separate metrics; hard safety failures cannot be offset by a weighted aggregate.
- **Statistics:** preregistered seeds and held-out scenario families; report paired differences, uncertainty intervals clustered by scenario family, and every missing/failed trial.

Initial target is not a fabricated “SOTA score”. It is: zero unexplained state transitions in the harness; no non-finite outputs; no uncommanded actions after hard stop; and a measurable task-level improvement over matched baselines that survives held-out scenarios. Improvements only count after the real-source test lane has an exact-head pass.

## Evidence ladder and claim limits

1. Stub/API test — interface compatibility only.
2. Real-source closed-loop smoke — cognition-to-Symtropy-physics signal path only.
3. Deterministic task simulation — benchmarked policy/task results in a declared simulator.
4. Hardware-in-loop — measured compute, sensor/actuator timing and known platform integration.
5. Physical robot — separately approved trials with hardware, operator, safety and ground-truth instrumentation.

No stage automatically qualifies the next. A smoke pass does not prove useful control; simulation does not establish real-world safety; a result on one task does not establish general robotics SOTA.
