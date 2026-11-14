# Symtropy–Symthaea Embodiment Adapter: Fault-Injection Benchmark v1

**Date:** 2026-10-19  
**Status:** Preregistered benchmark proposal; measurements not yet executed  
**Implementation under test:** [Draft PR #2167](https://github.com/Luminous-Dynamics/symtropy/pull/2167)  
**Scope:** Software adapter validation and simulation-only testing. This is not a robot safety certification and does not authorize hardware control.

## Research question

Does the adapter turn malformed numeric inputs, failed bridge outcomes, or actuator-interface disagreements into explicit, testable rejection outcomes without emitting an apparently valid command vector?

The benchmark must assess both **defect detection** and **availability cost**. A system that rejects every input can score perfectly on false accepts, so valid controls and over-rejection are part of the benchmark.

## Result-status propagation

The generic robotics boundary now has a typed `MotorPlannerStatus` and `MotorPlanResult`. The preferred `RoboticAgent::tick_motor_plan_with` returns commands and status as one result, so callers do not accidentally lose a typed rejection while passing through the generic agent interface. The legacy vector-only method remains for compatibility and explicitly discards status; integrity-sensitive call sites should migrate to the typed method.

The adapter retains detailed `PlannerFault` values locally and maps them to portable categories: invalid configuration, gain, observation, encoding, actuator interface, bridge failure, or invalid output. The benchmark must test both the detailed adapter fault and the portable status visible to a generic caller.

## Generic agent-boundary gates

The benchmark also covers the generic Symtropy agent API, not just the concrete Symthaea adapter. The typed `MotorPlanner::plan_result` contract performs preflight input validation and postflight status/shape/finiteness checks; `RoboticAgent::tick_motor_plan_with` adds agent-state, danger-context, and `can_act()` gates, then independently checks the returned result again. The returned `MotorPlanResult` is serializable with stable snake_case fault names and marked `must_use` so callers receive a compiler warning if they discard it. Rust tests now round-trip ready/rejected outcomes and verify every serialized fault name is present in [motor-plan-result-v1.schema.json](../../crates/bridges/symtropy-robotics-bridge-core/schemas/motor-plan-result-v1.schema.json). That schema describes the JSON shape; it does not itself attest to source revision, event integrity, actuator units, or physical safety. The preferred typed path should enforce the following before a result can be treated as accepted:

- Non-finite observation values are rejected before `RoboticAgent::tick` runs, so corrupted sensor input cannot mutate agent state before rejection.
- Non-finite danger context is rejected before `tick`.
- A post-tick gain outside the finite `[0, 1]` contract is rejected before the planner runs.
- If `can_act()` is false, the planner must not be called and the typed result must report `AgentNotAuthorized`.
- A planner that returns the wrong number of commands or any non-finite command is reported as `InvalidOutput`, even when `plan_result` is called directly instead of through an agent.
- If a planner reports `Rejected`, candidate commands must not leak out as accepted output; the typed result retains the rejection category.
- The `MotorPlanResult` serializes with stable snake_case fault/status names so logs can record the exact contract outcome; serialization does not itself make the record a signed or independently verified attestation.
- The legacy vector-only method remains for compatibility and cannot preserve typed status. Safety-/integrity-sensitive call sites should use the typed result and implement the platform-specific fallback.

These are software API invariants, not proof that returning a zero vector is the correct physical safe state. That fallback decision belongs to platform-specific control and independent safety mechanisms.

## Fault corpus

Freeze a versioned case list and never remove a case merely because it causes a regression.

| Case family | Injected condition | Required result |
|---|---|---|
| Invalid timestep | NaN, +Inf, zero, negative | Typed rejection before bridge step |
| Invalid gain | NaN, ±Inf, below zero, above one | Typed rejection before bridge step |
| Invalid observation | NaN, +Inf, -Inf in any channel | Typed rejection before bridge step |
| Invalid encoder dimension | Zero HV dimension | Typed rejection before bridge step |
| Requested interface mismatch | Caller actuator count differs from bridge declaration | Typed rejection before bridge step |
| Result interface mismatch | Returned actuator count differs from bridge declaration | Typed rejection after bridge response |
| Bridge failure | `success == false` | Typed rejection |
| Invalid command magnitude | Non-finite control effort | Typed rejection |
| Invalid quality telemetry | Non-finite prediction error/confidence; confidence outside [0,1] | Typed rejection |
| Valid boundary controls | Gain = 0 or 1; zero actuators where the bridge also declares zero; valid finite observations | Accepted with contract-preserving output |

Each case records its ID, injected field/value, expected fault, whether the bridge should have been stepped, returned vector length, and expected command policy. Cases and results should be serialized as evidence records with version and source revision.

## Baselines and ablations

1. **Unprotected reference** — a test-only reference reproducing the pre-guard behavior. It must never be selected for production or hardware use.
2. **Guarded adapter** — current implementation with typed faults.
3. **Input guards only** — isolates preflight validation from returned-result validation.
4. **All guards except actuator-count checks** — isolates whether interface validation catches cases the numeric checks miss.

All arms receive identical inputs and mock-bridge responses. The unprotected reference is used to measure detection delta only, not as an acceptable operational policy.

## Metrics

- **Invalid acceptance count:** invalid cases that produce a nonzero or non-finite command vector without the expected fault. Target: **0** on the frozen software corpus.
- **Preflight leakage:** invalid call inputs that reach `bridge.step`. Target: **0**.
- **Interface mismatch leakage:** actuator-count disagreements that yield apparently valid commands. Target: **0**.
- **Fault observability:** proportion of injected faults with the exact expected typed fault. Target: **100%** on the frozen corpus.
- **Valid-case rejection rate:** valid control cases rejected by a guard. Target: **0** for the frozen valid controls.
- **Output shape:** returned vector length equals the caller-requested count in every case, including rejection paths.
- **Nominal overhead:** p50/p95/p99 execution time for valid paths, plus added validation overhead versus the unprotected test reference, on a pinned host and toolchain.
- **Fault recovery:** after an injected fault, a subsequent valid call clears the prior fault and produces the expected valid output.
- **Coverage:** publish case counts, counts by fault family, and every failed case. Do not report a single aggregate pass rate without category counts.

The initial software gate is a deterministic classification gate, not statistical proof of real-world safety. Timing targets must be fixed only after collecting baseline measurement variance on a pinned machine; no latency result is claimed yet.

## Execution protocol

1. Pin repository SHA, Rust toolchain, target triple, profile, and benchmark corpus digest.
2. Run unit tests and a separate test-only fault corpus on the exact head.
3. Run each valid and injected case in isolation, reset the mock bridge between cases, and verify step counts where applicable.
4. Repeat timed valid-path runs enough to report robust percentiles and variance; report warm-up and sample count.
5. Preserve raw results, test logs, configuration, and corpus digest.
6. Require an independent reviewer to reproduce classification results from the recorded SHA.
7. Do not add physical claims until a calibrated reference workcell or measurement protocol exists.

## Industry comparison boundary

- NIST's 2026 manufacturing digital-twin workshop report identifies interoperability, verification/validation/uncertainty quantification (VVUQ), cybersecurity, and trustworthiness as priorities. This benchmark addresses a narrow software-contract slice of that larger agenda, not an entire digital twin. [NISTIR 8620](https://doi.org/10.6028/NIST.IR.8620).
- ISO 10218-1:2025 addresses safety requirements for industrial robots, while ISO 10218-2:2025 addresses integration and robot-cell applications. The present tests are engineering checks that may support a broader assurance process; they **do not establish ISO conformity**. [ISO 10218-1:2025](https://www.iso.org/standard/73933.html) · [ISO 10218-2:2025](https://www.iso.org/standard/73934.html).
- The published implementation intentionally returns a zero-authority vector on adapter rejection and records a typed fault. For physical platforms, zero commands may not be the correct safe state. A platform-specific fallback or independent safety controller must decide whether to brake, hold, retract, descend, or de-energize. The adapter benchmark does not validate those physical behaviors.

## Claim discipline

A successful frozen software corpus supports this statement only: *the tested adapter rejected the enumerated malformed inputs and interface mismatches under the recorded configuration.*

It does not prove:
- that every possible invalid input or hardware fault is detected;
- that the simulator matches physical apparatus;
- that zero-authority output is safe on every platform;
- that latency or availability is competitive before comparative measurements;
- that a machine conforms to a safety standard; or
- that AtomFab or Symtropy outperforms state of the art.

The next expansion is a simulation workcell fault corpus with stale observations, bounded sensor drift, actuator saturation, obstruction/near-collision, and recovery traces. That corpus must distinguish *collision risk*, *constraint rejection*, and *recovery success* rather than collapse them into a single safety score.
