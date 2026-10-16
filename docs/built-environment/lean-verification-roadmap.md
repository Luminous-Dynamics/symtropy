# Lean-backed verification roadmap for Symtropy

- **Status:** proof-of-concept proposal; the Lean proof source must pass the exact-head CI gate before it is treated as verified.
- **Scope:** public repository only; initial focus on elementary analytical housing reference equations.
- **Non-goal:** claiming that Lean verifies the Rust implementation, FEM solvers, physical assumptions, consciousness-physics hypotheses, code compliance, or building safety.

## Paradigm shift: from test evidence to proof-carrying engineering evidence

A numerical benchmark supplies sampled evidence. A Lean theorem supplies a machine-checked implication from stated assumptions. A Rust test supplies evidence about the implementation. A solver study supplies convergence evidence. Physical measurements supply validation evidence. These are complementary and non-substitutable.

For every assurance claim, build a typed evidence record with at least:

1. **Mathematical contract:** named definitions, units/dimensions, assumptions, theorem identifiers, Lean version, Mathlib revision, source digest, and build result.
2. **Implementation link:** exact Rust function/crate revision, mapping from Rust inputs/outputs to the formal contract, generated known-answer vectors, and differential/property test results.
3. **Numerical budget:** floating-point format, rounding assumptions, input conditioning, expected error bound, observed residual, and solver tolerance—never reuse a real-arithmetic theorem as a floating-point guarantee.
4. **Physical/model boundary:** applicable geometry, material/constitutive model, loads, boundary conditions, validation data, uncertainty and exclusions.
5. **Decision record:** exact intended use, permitted scope, reviewers, unresolved hazards, and explicit block reasons.

A hash establishes identity, not correctness or execution. A proof establishes a consequence of assumptions, not the truth of those assumptions. A passing analytical comparison does not qualify an arbitrary FEM model.

## Initial Lean slice

The package under formalization/lean introduces a separately pinned Lean 4 + Mathlib project for elementary closed-form references:

- rectangular second moment of area, I = b h^3 / 12;
- uniform axial bar extension, delta = F L / (E A);
- cantilever tip deflection, delta = F L^3 / (3 E I);
- simply supported midpoint-load deflection, delta = F L^3 / (48 E I);
- simply supported uniform-load deflection, delta = 5 q L^4 / (384 E I).

The proof obligations cover positivity under explicit physical sign assumptions and exact force/length scaling laws. These identities form a seed contract set, not a complete engineering semantics. They do not yet prove any existing Rust function implements the equations.

## Program of work

### F0 — Reproducible proof environment
- Pin the Lean release and immutable Mathlib commit.
- Build in CI on every relevant pull request.
- Audit the Symtropy theorem namespace for unapproved axioms and proof placeholders.
- Preserve exact-head CI evidence; queued or skipped is not passed.

### F1 — Formal analytical benchmark contracts
- Extend coverage to reactions, bending moments, stress, geometry-derived section properties, sign conventions, and dimensional analysis.
- Give every theorem an explicit list of assumptions and linked benchmark IDs.
- Add counterexamples proving that key assumptions are necessary where appropriate (for example, zero stiffness is outside the formula's physical domain).

### F2 — Rust refinement/conformance
- Create a stable machine-readable contract catalogue; generate test vectors from it and test the actual Rust implementations.
- Add differential, metamorphic, property-based, and mutation tests for each declared contract.
- Require exact mapping from formal arguments to Rust fields and units; missing or unknown quantities block conformance.
- Keep the proof, implementation test, analytical fixture, and execution receipt as distinct artifacts in the evidence chain.

### F3 — Floating-point and algorithmic bounds
- Review formal floating-point libraries such as [FloatSpec](https://reservoir.lean-lang.org/%40Beneficial-AI-Foundation/FloatSpec) before adding a dependency. Establish bounds for rounding, cancellation, overflow, underflow, and ill-conditioned inputs; define which values must be rejected or reported as inconclusive.
- Move from identities over real numbers to a proved or bounded implementation relation only when the arithmetic model and compiler/runtime assumptions are explicit.
- For iterative solvers, prove properties of the algorithm separately from empirical convergence tests and mesh/time-step refinement studies.

### F4 — Expanded domains, only after evidence gates
- Geometry and units; rigid-body and constraint invariants; energy bookkeeping; then selected nonlinear/thermal/structural cases with independent oracle data.
- Keep model assumptions and empirical evidence separately versioned. Do not infer empirical or regulatory validity from theorem count.
- Formalize consciousness/thermodynamic hypotheses as conditional mathematics only where definitions are precise; do not treat formalization as experimental confirmation.

## Proposed benchmark scorecard

Report exact numerators/denominators and revision identifiers; never use a single composite score to conceal a blocked gate.

| Gate | Initial acceptance target | Evidence |
|---|---|---|
| Proof build | 100% of committed Lean modules build on the pinned toolchain | exact commit + CI run URL/conclusion |
| Proof hygiene | zero sorry/admit and no unapproved axioms in the Symtropy namespace | axiom-audit report |
| Contract coverage | 100% of the explicitly declared initial equation/scaling/positivity claims are linked to stable theorem names | theorem ↔ contract manifest |
| Rust conformance | 100% of approved known-answer cases pass and all seeded mutations are rejected or blocked | per-case results and exact Rust head |
| Property checks | all declared linear/cubic/quartic scaling metamorphisms pass over generated valid-domain inputs | reproducible seeds, counts and tolerances |
| Dimensional validity | zero unit-inconsistent contract bindings in the supported subset | dimension-analysis proof/tests |
| Numeric honesty | every real-arithmetic contract used by floating-point code has an explicit implementation-error budget or is labelled MATHEMATICAL_ONLY | error budget + conditioning analysis |
| Solver credibility | residual, equilibrium, refinement, solver-specific tolerances and uncertainty declared per benchmark | solver-specific report; no universal tolerance |
| Physical validation | validation data are independent of calibration where feasible and include measurement uncertainty | traceable validation dossier |
| Failure semantics | zero false passes in the published negative-control corpus; incomplete evidence is BLOCKED/UNSUPPORTED | mutation corpus with numerator/denominator |
| Reproducibility | exact Lean/Mathlib/Rust revisions, fixtures and result receipts are pinned | content digests and clean rerun |

## Research pointers

- [OpenAI math release](https://github.com/openai/math): a research corpus whose README distinguishes formalized from unformalized results. Import only a specific relevant result after checking its Lean proof, dependencies, provenance and applicability; do not treat the release as a certified engineering library.
- [Mathlib](https://github.com/leanprover-community/mathlib4): reusable mathematical foundations; keep the dependency revision pinned.
- [Lean documentation](https://lean-lang.org/lean4/doc/): language and proof-development reference.
- [FloatSpec](https://reservoir.lean-lang.org/%40Beneficial-AI-Foundation/FloatSpec): candidate research dependency for formal floating-point semantics, subject to compatibility and assurance review.

## Definition of done for this prototype

Do not call these theorems verified until the exact proposed head has a successful Lean build and axiom audit. Do not call the Rust implementation verified until a separately reviewed refinement/conformance bridge and implementation-side tests exist. Do not call the solver or building physically validated without solver-appropriate numerical evidence, empirical evidence, uncertainty treatment, and qualified review.
