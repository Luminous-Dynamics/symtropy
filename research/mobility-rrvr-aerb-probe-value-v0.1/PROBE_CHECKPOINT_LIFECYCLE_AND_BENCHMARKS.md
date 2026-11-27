# AERB probe checkpoint lifecycle v0.1

**Status:** implementation candidate; exact-head software qualification pending. The reducer is an observer-side contract adapter, not a robot controller or safety authority.

## Research shift

Move from *choosing a probe before it starts* to *controlling a preemptible evidence-acquisition lifecycle*. A probe is a time-extended physical trajectory with checkpoints, not a zero-cost query that freezes the plant.

At each checkpoint, four questions are evaluated separately:

1. What evidence prefix has actually been acquired, for which source, calibration, clock and time interval?
2. Does the prefix support the declared proposition over its exact scope, or only a narrower sub-scope?
3. Does the existing complete sequential decision oracle say that further sensing can still improve the worst-case terminal decision?
4. Which exact remaining continue, safe-stop or abort trajectory has an independently supplied, checkpoint-bound physical qualification and a sufficient conservative horizon?

The reducer composes upstream receipts. It does not create a second robust minimax oracle, dynamics model, viability solver, sensor scheduler, command-closure engine, action-ticket owner or actuator controller.

## Implemented contract

Source: `evaluator/src/probe_checkpoint_v0_1.rs`.

- Checkpoint sequence must advance by exactly one and chain to the exact previous checkpoint digest. Replays and reordered checkpoints fail closed.
- Checkpoint identity is bound to attempt, probe, checkpoint sequence and a physical-plan digest.
- Evidence prefixes are bound to proposition, source/profile, calibration, clock mapping and acquired time interval. A prefix must begin at the requested interval start, end no later than its requested end, and carry an explicit gap-free coverage assertion for that prefix. Endpoint equality alone is not proof that frames or intervals were not dropped.
- An admitted positive/general measurement prefix can be admitted only for its observed sub-scope. A partial `absence_over_interval` claim is explicitly insufficient for the full declared interval; it cannot narrow possible-world support.
- Support narrowing requires a complete, payload-committed derivation receipt bound to exact before/after support sets, the admitted artifact digest, and a digest of the complete typed evidence-prefix record (source/profile, calibration, claim semantics, clock profile, interval and gap-free coverage assertion). Support widening is delegated to the existing evidence-invalidation owner.
- Removing `open_world_residual` additionally requires an explicit scope-closure receipt and an admitted complete declared scope. A partial observation can never close the residual.
- Decision sufficiency is accepted only from a complete, payload-committed receipt bound to the exact checkpoint and current support, asserting that all supported worlds share a terminal recommendation and no remaining probe strictly reduces worst-case terminal loss.
- Continue, safe-stop/hold and abort are distinct path kinds. A path must bind the exact attempt, probe, checkpoint and physical plan; cover the full trajectory; be qualified; and have a known safe horizon no shorter than its required duration. Unknown horizon is not infinity.
- A `completed` acquisition is not automatically a qualified physical hold: the completed state still needs a checkpoint-bound safe-stop/hold path, or the reducer returns `NO_SAFE_CONTINUATION`. When acquisition cannot continue and no decision-sufficiency oracle has required abort, a qualified hold is preferred to abort.
- `STOP_SENSING_HOLD`, `ABORT_PROBE` and `NO_SAFE_CONTINUATION` never grant a new physical effect. New effects still require #7788/#7814 pre-effect revalidation and #6503 independent action-ticket commit authority.

### Dispositions

| Disposition | Meaning |
|---|---|
| `CONTINUE_PROBE` | Only the remaining acquisition trajectory has a complete, checkpoint-bound qualified path and enough known safe horizon. |
| `STOP_SENSING_HOLD` | Further sensing is decision-sufficient to stop, or continuation is unavailable but a qualified safe-stop/hold path exists; unresolved effect finality remains unresolved. |
| `ABORT_PROBE` | A qualified abort path is available and selected because continuation is no longer appropriate or qualified. This is not proof of cancellation or command closure. |
| `NO_SAFE_CONTINUATION` | No complete qualified continue, stop/hold or abort path is available. Preserve uncertainty and defer to the independently prequalified fallback. |

Partial-evidence disposition is orthogonal to the lifecycle action: `ADMITTED_FOR_DECLARED_SCOPE`, `ADMITTED_FOR_SUBSCOPE`, `INSUFFICIENT_FOR_DECLARED_SCOPE`, `UNQUALIFIED`, or `NO_EVIDENCE`.

## Trust boundary

The Rust reducer verifies identity, sequence, digest shape, deterministic payload commitments, exact scope binding and cross-receipt consistency. The gap-free coverage bit is an upstream assertion, not independently established by this reducer. A hash/commitment detects payload changes; it does **not** authenticate an issuer. It does not prove that an upstream sensor measured reality, that an upstream dependency inventory is complete, that the physical supervisor's horizon is sound, or that an abort actually stopped motion. Those remain upstream and independently qualified responsibilities.

The module is a pure process-model reducer. No simulator, HIL, physical robot or regulatory qualification is implied.

## Benchmark protocol

Freeze scenario worlds, action budgets, probe costs, acquisition/transport/inference delays, clock uncertainty and physical transition branches before evaluating candidate observers. Candidate processes receive only the allowed checkpoint prefix and admitted receipts. Evaluator truth, true unobserved branch, and optimal stopping/abort policy remain evaluator-only. Keep the public deterministic corpus labelled as conformance, not blind. A performance claim requires the separate #7732 custody, isolation and leakage audit.

### Matched comparison arms

1. Fixed-duration probe with no early stopping.
2. Existing sequential robust policy (#13571) without checkpoint preemption.
3. Entropy/EIG stopping only where a calibrated joint observation model is justified.
4. Expected decision-value stopping (#9821) only where priors/likelihoods are justified.
5. Prior-free robust decision-sufficiency stopping with checkpoint preemption.
6. Logical-deadline-only cancellation.
7. Full-trajectory viability-bounded continuation and independently qualified stop/abort paths (#7738).
8. Evaluator-only optimal stopping/abort oracle as a non-deployable upper bound.

All arms pay for elapsed sensing, stop/settle, abort, communication, inference, actuator energy and recovery. Preserve native task/recovery success as a separate host metric. Use paired schedules: late effect evidence before/during/after a probe; authority revocation; shrinking hazard horizon; missing abort route; sensor/calibration/clock invalidation; partial measurement; gapful intervals and sensor dropouts; evidence-prefix metadata tampering; delayed/duplicated abort; candidate-visible metadata leakage; and checkpoint replay/permutation.

### Metrics — report separately

- False safe-to-continue, false safe-to-stop and false safe-to-abort.
- Hard physical constraint violations during full continuation and abort trajectories.
- False partial-to-full coverage promotion and false absence/NoEffect claims.
- Stale, replayed or wrong-scope checkpoint acceptance.
- Open-world residual removal without complete scope closure.
- Unsafe next-effect recommendations after checkpoint evidence changes.
- Decision-value lost/gained from preemption; unnecessary full-probe continuation.
- Wasted sensor/actuator energy, compute, bytes and physical exposure.
- Time from evidence arrival to policy update; time to qualified stop/abort.
- Time to independent effect finality and pre-effect commit revalidation.
- Positive-control acceptance / unnecessary Hold.
- p50/p95/p99 checkpoint, inference, stop and abort latency.
- CPU, memory, communication and storage overhead.
- Native task success and recovery success (not blended into safety).

Hard-safety and false-finality errors are disqualifying and must never be averaged against task success or efficiency.

### Prospective gates

- 100% agreement with an independently authored finite checkpoint/branch oracle.
- Zero false safe-to-continue/stop/abort in frozen negative controls.
- Zero partial-prefix promotion beyond measured scope; zero gapful-prefix admission for support narrowing; zero partial absence promotion to full-window absence.
- Zero open-world residual deletion without the required complete scope-closure receipt.
- Zero acceptance of replayed, stale, wrong-plan or payload-tampered receipts.
- Zero new-effect authorization from this reducer (invariant, not a benchmark score).
- Deterministic replay under non-semantic input permutations.
- Rust 1.96.0 formatting, locked tests and strict Clippy on the exact commit before software qualification.
- Simulator, HIL and physical qualification each require separately frozen protocols and evidence.

These are research targets, not achieved performance or certification thresholds.

## Industry comparison and benchmark composition

Do not replace or claim to beat the native tasks of existing benchmarks. Add the AERB checkpoint assurance overlay to the same physical traces and report both native metrics and assurance-specific metrics.

- **DiMuRe / LIBERO-Disturb:** diagnosis-guided program reuse/recovery with local post-recovery verification. Its reported 77.65% complete-task success for Local DiMuRe versus 58.68% original program is under the same physical-step allowance; the 81.90% Full DiMuRe result adds terminal replanning and extra step allowance. Preserve that budget distinction when comparing.
  - https://arxiv.org/abs/2611.04226
- **RoboRecover and LIBERO-Recovery:** native execution-deviation and recovery baselines; preserve their task and recovery scoring, then overlay checkpoint/stop/abort assurance.
- **SafeVLA-Bench:** separates native success from safety, unsafe-success share and worst-violation severity. Its September 2026 public snapshot reports 22,500 episodes, 24 policies and 8 safety specifications; it is an appropriate safety-frequency/severity comparison, not a substitute for effect-finality tests.
  - https://safevla.org/
  - https://arxiv.org/abs/2606.00773
- **SafeManip:** temporal-property monitoring over finite traces; use its temporal safety properties as host monitors for the probe/abort trace.
  - https://arxiv.org/abs/2605.12386
- **ActiveArena:** 35 active-perception tasks across five categories, with ID/OOD scene protocols; use it to evaluate information acquisition and memory/generalization, while AERB adds execution ambiguity, physical preemption and evidence-scope correctness.
  - https://arxiv.org/abs/2609.24124
- **EmbodiedGovBench:** governance tests for capability boundaries, runtime drift, recovery, portability, upgrade safety, human override and audit completeness. Use audit/override/governance checks as a complementary dimension.
  - https://arxiv.org/abs/2604.11174

For industrial deployment, map hazards and risk reduction into applicable machinery/robot standards and the integrator's risk assessment. ISO 10218-1:2025 addresses industrial robot requirements and ISO 10218-2:2025 integration; this software is not a substitute for either standard, a safety-rated controller, or the integrator's functional-safety obligations.

## Upstream composition

- #7738: full-trajectory physical safety-to-observe and qualified physical horizons.
- #7694 / Symthaea #13571: robust sequential decision value and the independent finite oracle.
- #7728 / #7729: typed evidence projection and materialization.
- #7433 / #7435: command-path temporal closure and late arrivals.
- #7788 / PR #7814 and #6503: revocable pre-effect support and independent action-ticket commit.
- #7732 / PR #7815: blind assignment/commitment primitive; separate process custody and leakage audit still required.

## Explicit non-claims

No claim of source authentication, complete evidence/change-stream coverage, physical no-effect, safe actuator cancellation, atomic abort, physical viability proof, sensor truth, blind performance, SOTA superiority, simulator/HIL/physical qualification, regulatory compliance or robot safety certification.
