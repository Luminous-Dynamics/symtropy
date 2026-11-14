# Mathematics Transfer Map for Symtropy

**Status:** research plan and source triage; no formal proof, Rust test, simulation, or benchmark PASS is claimed.  
**Date:** 2026-10-19  
**OpenAI source snapshot:** `openai/math@1581d4cbdd69a0faf34b2089614fe2a814316a6a`  
**Work to extend:** [proof-carrying physics RFC #1682](https://github.com/Luminous-Dynamics/symtropy/pull/1682), [energy ledger contract #1693](https://github.com/Luminous-Dynamics/symtropy/issues/1693), [max-min regeneration allocator #1747](https://github.com/Luminous-Dynamics/symtropy/issues/1747), [housing analytical reference proof-of-concept #1691](https://github.com/Luminous-Dynamics/symtropy/pull/1691).

## Principle

Use mathematics to make specific simulation claims testable, not to make the simulation sound more physically validated than it is. Each result needs a frozen model, explicit preconditions, a machine-checked theorem where tractable, known-answer/negative-control tests for the implementation, numerical-error analysis, and separate empirical validation where the claim is physical.

The upstream OpenAI math repository contains both Lean artifacts and manuscripts with differing verification status. A related result is a candidate until its exact statement, assumptions, revision and proof environment have been checked. Never infer that a result about one mathematical system proves behavior of a different production system.

## P0 — Repair the energy accounting model before proving conservation

Start with [THERMO-LEDGER-001 #1693](https://github.com/Luminous-Dynamics/symtropy/issues/1693). The current ledger's local residual omits storage changes and parts of the system boundary, while regeneration is recorded through a negative dissipation value. This is a modeling/typing defect before it is a theorem-proving problem.

Define a bounded system boundary and a signed accounting identity such as:

`Delta(E_stored) = E_external_in - E_external_out + W_boundary_in - W_boundary_out - Q_dissipated`

The exact signs and terms must follow the actual boundary convention. Split the ledger into explicit categories for external energy input, transfers between tracked stores, nonnegative dissipation, stored energy delta, and boundary work/outputs. Avoid double-counting internal transfers; validate finite/nonnegative values at typed API boundaries and define overflow behavior.

**Hostile controls:** negative/NaN/Inf dissipation; regeneration entered as negative heat; omitted stores; double-counted internal transfer; unregistered receiver; capacity-clamped credit; source debit without accepted credit; overflow; duplicated event; hidden boundary flow; sign mutation.

**Claim ceiling:** even a correct bookkeeping identity proves software accounting under its model, not that the current consciousness/physics coupling is an accurate thermodynamic law.

## P1 — Graph heat-flow entropy dissipation: a tractable formal pilot

The pinned OpenAI source [`OAI/Analysis/LpDimension/Entropy.lean`](https://github.com/openai/math/blob/1581d4cbdd69a0faf34b2089614fe2a814316a6a/lean/OAI/Analysis/LpDimension/Entropy.lean), blob `b2b32a01166f7990231a1d16240b1f80269d9114`, defines a weighted graph electrical-energy functional and proves its non-negativity under nonnegative edge weights and positive node values. It also derives an entropy derivative for its specific weighted graph heat equation.

For a finite graph with source `src`, destination `dst`, nonnegative conductances `c_e`, positive state `h_i`, and the exact graph heat-flow vector field in the upstream development, the relevant structure is:

`D(h) = sum_e c_e (h_src - h_dst) (ln h_src - ln h_dst) >= 0`

and the defined entropy derivative is `dH/dt = -D(h)`.

This works because logarithm monotonicity makes each edge contribution nonnegative. The sign result depends on the stated model and positivity assumptions.

### Proposed adaptation sequence

A versioned reference corpus with symbolic expected expressions, domain-rejection cases, permutation cases and seeded sign mutations is included at [`graph-entropy-reference-v1.json`](./data/graph-entropy-reference-v1.json). The corpus specifies source inputs and mathematical expectations; it is not yet consumed by a test harness and has not been run against Lean or Rust.

1. Re-express the finite graph and dynamics in the canonical, pinned Symtropy proof environment. Do not create a second Lake/Mathlib root.
2. Prove the abstract sign lemma with explicit nonnegative-conductance and positive-state premises.
3. Add small exact reference graphs: one node, two nodes, path, cycle, disconnected components, zero-weight edges, and permuted node/edge enumeration.
4. Check analytical reference outputs against a separate numerical implementation.
5. Only then investigate a production adapter to a concrete network-diffusion subsystem, with exact equations and identity bindings.

**Mutations:** negative edge weight admitted; zero/negative state admitted into `log`; reversed edge-difference sign; wrong boundary condition; altered conductance; incomplete/duplicate edges; theorem source/environment drift.

**Nonclaim:** this does not prove rigid-body dynamics, LTC stability, whole-system energy conservation, or biological validity. The entropy theorem must not be applied to a system whose vector field is different.

## P2 — Deterministic fair energy allocation as a constrained optimization contract

Continue [THERMO-ALLOC-001 #1747](https://github.com/Luminous-Dynamics/symtropy/issues/1747) and the existing max-min allocator work. Energy wells create a constrained batch-allocation problem with:
- total supply;
- recipient headroom;
- per-recipient offer limits;
- eligibility determined by the frozen spatial predicate;
- stable recipient identities;
- explicit behavior for overlapping wells.

For every accepted allocation `x_i`, prove/check:

`0 <= x_i <= min(headroom_i, offer_limit_i)`

`sum_i x_i <= supply`

and, after the allocation is committed, source debit equals actual accepted credit under the defined accounting precision. For a stated max-min policy, specify its objective and tie-breaking/allocation rule before claiming optimality or fairness.

**Benchmarks:** permutation invariance by stable ID; saturated low-headroom recipients; zero/empty demand; overlapping wells; unregistered recipients; receiver near capacity; non-finite values; 1k/10k/100k recipients; fairness/throughput/latency; conservation after commit. Preserve the existing eligibility geometry exactly until a separate behavior change is reviewed.

Potential math follow-up: formulate the policy as a finite constrained optimization problem and prove feasibility and the allocator's max-min properties. Do not claim an optimization theorem until the implemented algorithm and all tie/overlap semantics match that formulation.

## P3 — Structural engineering: dimensional homogeneity and scale invariance

Extend the existing [housing formalization PR #1691](https://github.com/Luminous-Dynamics/symtropy/pull/1691) and [housing reference work #1653](https://github.com/Luminous-Dynamics/symtropy/pull/1653). Its current direction maps analytical beam/section references to theorem IDs and dimensional contracts, but the PR itself explicitly leaves Lean execution, Rust execution and formal refinement pending.

High-value mathematics:
- unit-safe signatures and explicit SI exponent vectors;
- dimensionless groups and geometric-similarity relations;
- algebraic invariants for axial bars, cantilevers and simply supported beams;
- rational reference oracles for elementary formula families;
- perturbation/round-off budgets before comparing f64 outputs.

**Benchmarks:** all frozen reference fields mapped; malformed units/formulas rejected; seeded missing-length-power mutations detected; scale sweeps pass declared dimensionless invariants; exact-head Lean build and axiom audit; Rust outputs within declared tolerances; separately qualified floating-point/refinement argument. None of these alone establishes FEM validity, building-code compliance or physical safety.

## P4 — Numerical stability, bounds and robustness for the physics engine

Candidate mathematics to screen against actual kernel boundaries:

- **Perturbation and conditioning bounds:** quantify sensitivity of collision/contact results, matrix-based transforms, and constraint updates to small input/state changes.
- **Geometry inequalities:** explicit assumptions for distance, projection, support mappings and convexity; preserve conservative collision detection behavior under near-degenerate inputs.
- **Discrete dynamical stability:** bound integration error and invariant drift across step sizes; do not confuse a continuous-time theorem with PBD/impulse or floating-point behavior.
- **Constrained optimization:** feasibility and residual bounds for impulse/constraint batches where the solver's objective can be stated faithfully.
- **Probability/entropy bounds:** use only for a clearly defined stochastic network subsystem with normalized distributions and valid supports.

First catalogue production invariants and counterexamples, then choose a tractable theorem. These are opportunities for a Mathlib search, not assertions that the OpenAI papers already formalize these exact Symtropy algorithms.

## P5 — Cross-scale simulation and network stability

For suitable network models, explore graph Laplacian energy, spectral-gap bounds, and contraction properties. The question to test is whether a subsystem's state approaches a defined equilibrium or preserves a defined invariant under explicit conditions. A useful target might be a finite diffusion layer or a queue/resource flow model, not the entire physics engine.

For rigid-body/contact systems, seek a separate discrete-time result. Collision restitution, impulses, friction, sleeping, constraint iteration and discrete time steps create different obligations from graph heat flow.

**Required evidence:** exact vector field/transition rule, domain assumptions, invariant/equilibrium definition, boundary conditions, counterexamples near parameter boundaries, numerical trajectories across step sizes, and separate implementation correspondence.

## Second-pass scan: graph metrics and dynamical regimes

The pinned OpenAI formalization catalogue [`lean/formalization.yaml`](https://github.com/openai/math/blob/1581d4cbdd69a0faf34b2089614fe2a814316a6/lean/formalization.yaml), blob `275eee8cfab7ce49d3a5e88f02cbb8f28fe6fee6`, lists graph and dynamical results worth screening against actual Symtropy subsystems:

- **Planar graph metrics embedding into `L1` with constant distortion** and **`L1` embeddings for bounded-treewidth graphs** are candidates for *approximate route-distance analysis*. Before using either, determine if a target graph is planar or has the required bounded-treewidth structure, identify the exact theorem and distortion convention, then compare estimated pairwise distances and route costs against the exact graph metric. An embedding is not an exact shortest-path solver, and a bound for restricted graph families is not a bound for arbitrary Symtropy worlds.
- **A linear cycle-and-edge decomposition of every graph** may help structure graph-corpus analysis and independent graph invariants. Treat it as a tooling/analysis opportunity until a concrete subsystem and measurable improvement are identified.
- **Two limit cycles for quintic Liénard systems** is a useful warning that non-linear planar dynamics can have multiple periodic regimes. It can motivate multi-attractor and initial-condition testing for a specific reduced-order dynamical model, but it does not imply that the physics engine or LTC system shares that equation or its bifurcation structure.
- **Memory/entropy and sampling results** should only enter the physics repo where a subsystem genuinely has the stated probabilistic model; do not use information-theoretic language to dress up deterministic state without defining a distribution.

The first actionable follow-up is a route-metric applicability study with a strict scope: inventory graph instances, test assumptions, and benchmark distortion. Do not change live route selection based on an unqualified embedding.

## Math-transfer backlog and priority

| Priority | Mathematics | Candidate subsystem | First useful artifact |
|---|---|---|---|
| P0 | Typed conservation identity, source/sink/storage algebra | Thermodynamic ledger | Complete boundary model + hostile bookkeeping corpus |
| P1 | Log monotonicity, finite graph energy, entropy dissipation | Graph diffusion/transport | Lean sign theorem + exact small-graph fixtures |
| P1 | Finite constrained optimization and max-min fairness | Energy wells / regeneration | Formal policy spec + permutation and feasibility tests |
| P2 | Dimensional homogeneity and similarity | Housing/structural reference | Unit/exponent contract + scale metamorphic corpus |
| P2 | Norm/conditioning/perturbation bounds | Collision and matrix calculations | Error envelope + near-degenerate mutation set |
| P2 | Discrete-time Lyapunov and invariant sets | Specific integrator/contact model | Exact update-rule theorem, not a continuous-time proxy |
| P3 | Spectral gaps and contraction | Network dynamics | Proved convergence rate under stated graph assumptions |
| P3 | Probability divergence bounds | Explicit stochastic subsystems | Normalization/support contract and calibrated reference study |

## Common qualification protocol

For each item:
1. Freeze the mathematical subject and exact source blobs/revisions.
2. State preconditions, units, sign conventions and model boundary.
3. Use the canonical pinned Lean project; audit proof holes/axioms.
4. Test the production implementation independently using deterministic fixtures and hostile controls.
5. Quantify floating-point and discrete-solver error.
6. Add empirical validation and expert review when making physical claims.
7. Keep exact-head execution receipts and never treat queued CI as a PASS.

## Execution order

1. Fix/freeze #1693 accounting semantics; do not write a conservation theorem over an incomplete ledger.
2. Formalize the graph sign lemma in the approved project, with no claim about unrelated dynamics.
3. Extend #1747's allocation contract into objective/tie/overlap-specific proofs and tests.
4. Reconcile #1691's theorem and implementation mapping without claiming pending checks have passed.
5. Select one conditioning/stability problem from the production-source census and build the smallest counterexample-rich benchmark first.

## Source register

- OpenAI math immutable revision: [`1581d4cbdd69a0faf34b2089614fe2a814316a6a`](https://github.com/openai/math/tree/1581d4cbdd69a0faf34b2089614fe2a814316a6a).
- OpenAI graph entropy/energy source: [`Entropy.lean`](https://github.com/openai/math/blob/1581d4cbdd69a0faf34b2089614fe2a814316a6a/lean/OAI/Analysis/LpDimension/Entropy.lean).
- OpenAI relative entropy source: [finite-law entropy formalization](https://github.com/openai/math/blob/1581d4cbdd69a0faf34b2089614fe2a814316a6a/lean/OAI/Probability/SKRatio/Entropy/Entropy.lean).
- Symtropy contracts: [#1682](https://github.com/Luminous-Dynamics/symtropy/pull/1682), [#1693](https://github.com/Luminous-Dynamics/symtropy/issues/1693), [#1747](https://github.com/Luminous-Dynamics/symtropy/issues/1747), [#1691](https://github.com/Luminous-Dynamics/symtropy/pull/1691). Additional focused work: [graph entropy #2087](https://github.com/Luminous-Dynamics/symtropy/issues/2087), [graph metric #2088](https://github.com/Luminous-Dynamics/symtropy/issues/2088).

**Validation status:** this document is a source-grounded research map only. It does not execute Lean or Rust, does not run a simulation, and does not certify any physical model.
