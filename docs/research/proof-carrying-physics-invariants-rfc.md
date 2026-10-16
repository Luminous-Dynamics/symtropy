# Proof-Carrying Physics: Invariant Contracts and Benchmarks

**Status:** Research RFC; proposed, not implemented or qualified.  
**Date:** 2026-10-16  
**Repository:** `Luminous-Dynamics/symtropy`

## Thesis

Treat mathematical invariants as explicit, testable contracts of the simulation substrate. The goal is not to claim that physics is “formally verified” because some equations are written down. The goal is to connect each contract to its exact assumptions, its Rust implementation, hostile tests, and reproducible evidence.

OpenAI's [mathematical research repository](https://github.com/openai/math) is a source of formalized techniques for real analysis, entropy, inequalities and related subjects. Its authors state that results have mixed verification status and that not every manuscript has a Lean formalization. Use narrowly selected, compatible lemmas as references; do not treat the full library as a drop-in dependency or import a proof without verifying its statement and assumptions.

## Initial source census

A first inspection of the current public source surfaces concrete tests and cautions:

- [`safety.rs`](https://github.com/Luminous-Dynamics/symtropy/blob/main/crates/domains/symtropy-consciousness-physics/src/safety.rs) already documents that the hardcoded `SafetyTier::from_phi` mapping is not used by platform demos; the demos use `sprint_floor_gain`, while the four-tier mapping remains used within the crate's coupling path. Existing tests cover sampled monotonicity and representative thresholds, but not exact threshold neighbors or invalid-domain values. [Draft PR #1683](https://github.com/Luminous-Dynamics/symtropy/pull/1683) adds checked-domain classification and exact-boundary hostile tests. This is a bounded fix to the legacy mapping, not a claim that all motor-authority paths are verified or calibrated.
- [`energy.rs`](https://github.com/Luminous-Dynamics/symtropy/blob/main/crates/domains/symtropy-consciousness-physics/src/energy.rs) exposes public `f64` methods for consumption, regeneration and heat. The previous code did not explicitly reject all negative/non-finite inputs, and the regeneration counter recorded requested flow even when the capacity cap accepted less. [Draft PR #1684](https://github.com/Luminous-Dynamics/symtropy/pull/1684) adds fail-closed input guards, candidate-state validation, accepted-flow telemetry, and deterministic regression tests. Its tests have not yet been run in this environment.
- [`thermodynamics.rs`](https://github.com/Luminous-Dynamics/symtropy/blob/main/crates/domains/symtropy-consciousness-physics/src/thermodynamics.rs) documents a sub-1% conservation-error target. That numerical threshold is not itself a proof of ledger conservation. ST-INV-004 must first define a complete one-step accounting identity and its external inputs/sinks; long-horizon floating-point drift is a separate measurement.

These findings strengthen the RFC's principle: formalization should follow the real update paths and known counterexamples, not merely translate prose equations into Lean.

## Paradigm shift: from equations in prose to contracts with evidence

For each invariant, maintain a contract record:

- stable contract ID and exact mathematical statement;
- explicit premises, units, state bounds and boundary semantics;
- exact Rust functions/types that implement the contract;
- classification: algebraic proof, formal theorem, property test, numerical experiment, or empirical/scientific hypothesis;
- proof and test artifacts, seed/corpus, exact source head and dependency/environment identity;
- failure and non-goal statements.

The contract record must separate three claims:

1. **Mathematical validity:** the abstract statement follows from the stated premises.
2. **Implementation fidelity:** the specific Rust code refines or approximates that statement within a declared error/domain contract.
3. **Model validity:** the equations adequately describe the physical or cognitive phenomenon being modeled.

Evidence for one level cannot silently promote either of the others.

## Ledger source audit: accounting semantics are not yet a conservation theorem

The current implementation in [`thermodynamics.rs`](https://github.com/Luminous-Dynamics/symtropy/blob/main/crates/domains/symtropy-consciousness-physics/src/thermodynamics.rs) defines `energy_in` as the sum of action costs and `energy_out` as dissipated heat, then computes `abs(energy_in - energy_out) / energy_in`. This is a local action-cost/dissipation residual. It is not, by itself, a whole-system conservation error: the current record does not include change in reservoir storage, kinetic/potential energy, external well input, or all boundaries.

There is also a concrete sign-semantic collision in [`coupling.rs`](https://github.com/Luminous-Dynamics/symtropy/blob/main/crates/domains/symtropy-consciousness-physics/src/coupling.rs): the field-energy path calls `record_dissipation(-effect)` when an entity regenerates. That makes `energy_out` a signed net-flow accumulator despite its documented meaning as dissipated heat. The public ledger record methods also accept negative and non-finite values without explicit rejection, allowing invalid inputs to alter or poison totals.

This has consequences beyond naming:
- `TickBalance.conservation_error` is not constrained to the documented [0, 1] range when the signed accumulator or invalid inputs occur.
- A finite, positive dissipation and a negative regeneration share a counter whose intended physical meaning is not homogeneous.
- The current tests prove the arithmetic behavior for synthetic `action_cost == dissipation` examples, not conservation of the coupled simulation.

### Required redesign before promoting ST-INV-004

Do not “fix” the residual by guessing the right sign. First introduce typed, finite, non-negative flow categories (action cost/consumption, dissipation, regeneration/external input) and migrate the negative-`record_dissipation` call site. Then define the exact accounted system boundary and track the relevant state delta. Only after all sources, sinks, storage changes, and units are explicit should a conservation identity be stated.

Keep legacy residual metrics clearly named/documented as local diagnostics until a full ledger contract is implemented. Add hostile tests for negative/non-finite flows, overflow, double recording, regeneration sign, missing flow categories, and balance mutations. A test suite that only checks “100 consumed and 100 dissipated gives zero” is insufficient evidence for whole-system conservation.

## P0 contract candidates

These are proposed candidates to be checked against the actual current implementation. This RFC does not assert they are already proved.

### ST-INV-001 — motor gain is bounded and monotone

For the piecewise motor gain in `docs/tech/FORMAL_SPECIFICATION.md`, establish `0 ≤ g(Φ) ≤ 1` and monotonicity on the declared `Φ ∈ [0,1]` domain. Encode exact threshold behavior at `0.1`, `0.3`, and `0.6` as hostile boundary tests. The theorem must preserve the current strict/non-strict inequalities rather than smoothing them.

### ST-INV-002 — precision transform is bounded

For `π = 1 / (1 + σ)`, prove `0 < π ≤ 1` and monotonic non-increase in `σ` under `σ ≥ 0`. The Rust contract must define finite-value handling and reject or quarantine NaN/infinity before they enter this invariant's domain.

### ST-INV-003 — collision/habituation update preserves admissible error state

Specify the exact update order and premises. For example, `σ' = σ(1 - λ) + min(0.01 |J|, 2)` preserves `σ' ≥ 0` when `σ ≥ 0`, `0 ≤ λ ≤ 1`, and `J` is finite. This formula is a candidate abstract contract: confirm that it matches the real update code before formalizing it. Test large impulses, zero impulses, zero state, and parameter boundaries.

### ST-INV-004 — energy ledger transition is balanced

Before proving a conservation theorem, resolve the exact meanings and units of input, output, body reservoirs, dissipated energy, and remaining well capacity. Every transfer must have a named source and sink; dissipation is tracked rather than silently discarded. A valid theorem must quantify one simulation transition and list all permitted external work/input.

Do **not** infer energy conservation from a target tolerance such as “less than 1% error.” First prove the bookkeeping identity at the abstract transition level; then measure floating-point drift in the implementation.

### ST-INV-005 — harmony similarity has explicit zero-norm semantics

Cosine similarity requires a defined policy when either vector has zero norm. Identify the production helper and its current convention. Prove bounds only under the correct preconditions, and test zero vectors, nearly-zero vectors, finite ranges, symmetry and clamping if applicable. Do not assume undefined inputs are harmless.

## Integration with mathematical proof tools

- Prefer the smallest sufficient theorem or Mathlib lemma; avoid importing the OpenAI repository wholesale.
- Do not introduce a second unapproved Lean/Lake environment. First identify whether Symtropy has an existing formal verification substrate and obtain an explicit decision on the canonical toolchain/dependency manifest.
- A future Lean/Rust bridge must bind theorem digest, source head, toolchain/manifest, assumptions and the Rust function/contract it is meant to constrain.
- If no qualified formal substrate exists yet, start with pure Rust contract tests plus a written theorem statement and proof plan. Label this as design/test evidence, not formal proof.
- Use bounded, deterministic checks and save exact test inputs, seeds, tool versions and exit status. A queued runner remains unverified, never PASS.

## Benchmark targets

| Dimension | Target / gate |
|---|---|
| Contract traceability | 100% of promoted invariants map to exact implementation symbols and tests |
| Mathematical assumptions | Every theorem states its domains, units, bounds and update order explicitly |
| Boundary coverage | Every piecewise threshold tested at `t-ε`, `t`, and `t+ε` where representable |
| Hostile testing | Include NaN/infinity, overflow, zero norm, invalid parameters, reversed signs and omitted-premise mutations where relevant |
| Ledger correctness | Exact abstract transition balances for every modeled transfer case before numeric-drift claims |
| Numerical behavior | Publish long-horizon drift distributions across seeded workloads; use tolerances justified by the arithmetic and scenario, not selected post hoc |
| Reproducibility | Every result binds exact source head, command/tool versions, fixtures/seeds and captured output |
| Claim discipline | Zero instances where theorem validity, code fidelity, or physical validity are presented as interchangeable |

Targets are proposed acceptance criteria, not measured Symtropy performance.

## Benchmark design

Build a small deterministic suite that executes locally before CI:

1. Pure piecewise-function and numeric-boundary fixtures.
2. Property tests over bounded valid domains with deterministic seeds.
3. Mutation tests: flip a sign, remove a premise, change one threshold, reverse one transfer, or make the zero-vector policy ambiguous. The suite must detect each mutation relevant to a contract.
4. Exact abstract ledger-transition fixtures with all energy flows enumerated.
5. Long-horizon numerical simulations reported separately from exact mathematical properties.
6. Reproducibility check: rerun the same fixture and compare canonical evidence digests. Do not compare only renderer output or frame timing.

Record baseline results before optimizing. Any tolerance change must explain the arithmetic/error model and must not mask a failing invariant.

## Proposed rollout

- **P0 — implementation census:** identify the actual Rust function/type behind each candidate contract; remove any claim for which the implementation cannot yet be identified.
- **P1 — pure tests:** implement the smallest deterministic tests and mutation controls for ST-INV-001 through ST-INV-003.
- **P2 — ledger semantics:** resolve input/output/dissipation/well semantics and add exact transition fixtures for ST-INV-004.
- **P3 — proof substrate decision:** inspect existing repository tooling before selecting a single pinned formal project. Do not introduce a parallel project by convenience.
- **P4 — formalization:** prove the smallest stable contracts and connect them to a separate implementation-fidelity obligation.
- **P5 — benchmark publication:** report pass/fail/blocked separately, with denominators, exact head, command, environment identity and artifact digests.

## Explicit non-goals

This RFC does not establish that integrated information is a physical force, that cooperation is thermodynamically necessary in nature, or that the current energy coefficients are empirically calibrated. It does not authorize arbitrary runtime actions. Those are distinct scientific and authority claims requiring independent evidence.

## Definition of done

A contract graduates only when (a) the implementation mapping is reviewed, (b) premises are explicit, (c) its proof or test evidence is reproducible, (d) relevant hostile controls fail closed, and (e) numerical and scientific claim ceilings are documented. No result is promoted based solely on a green aggregate CI badge.

## References

- [Symtropy formal specification](https://github.com/Luminous-Dynamics/symtropy/blob/main/docs/tech/FORMAL_SPECIFICATION.md)
- [OpenAI math repository](https://github.com/openai/math)
- [OpenAI Lean formalizations](https://github.com/openai/math/tree/main/lean)
