# Symtropy Physics VVUQ Reference Seed v0.1

**Status:** focused regression seed. Not full-engine validation, physical qualification, or external certification.

**Tracking issue:** [#7621](https://github.com/Luminous-Dynamics/symtropy/issues/7621).

## Purpose

Turn two requirements already stated in `RESEARCH_VALIDATION.md` into exact-head, CI-runnable references:

1. Numerical solution verification by comparing free fall to a closed-form solution and the exact discrete update of the integrator.
2. Geometry verification by comparing transformed 2D OBB GJK classification with an independent separating-axis reference.

The test code is `crates/core/symtropy-physics/tests/vvuq_reference.rs`.

## Track A — Semi-implicit Euler convergence

For constant acceleration (g), semi-implicit Euler uses:

`v_{n+1}=v_n+g\,\Delta t`, `x_{n+1}=x_n+v_{n+1}\,\Delta t`.

At fixed duration (T=N\Delta t), the velocity matches the continuous constant-acceleration solution in exact arithmetic. Its position difference from the closed form is:

`x_N-x(T)=\tfrac12 gT\Delta t`.

This gives an exact scheme-specific reference error, not a fitted threshold. The seed tests (\Delta t\in\{1/30,1/60,1/120,1/240\}), checks the discrete error to (10^{-9}), velocity error to (10^{-10}), and first-order convergence ratios to (10^{-8}).

The test verifies the current semi-implicit Euler translational path only. It does not validate contacts, friction, rotational dynamics, long-term energy behavior, or other integrators.

## Track B — Independent transformed 2D OBB oracle

The reference is a standalone 2D separating-axis test over the four local face axes (two axes per box). For a rectangle with half-extents ((h_x,h_y)), rotated local unit axes ((u,v)), and test axis (a), its projection radius is:

`r(a)=h_x|u\cdot a|+h_y|v\cdot a|`.

The pair overlaps exactly when centre separation on every candidate axis is no greater than the sum of the two projection radii, within the declared (10^{-12}) reference tolerance.

The Rust test generates 4,096 deterministic cases from seed `0x4551435656553031`, with fixed half-extents and varied rotations/translations. It compares this reference to the public transformed-GJK API and records false-positive/false-negative counts as hard test failures. A translation-only GJK path is evaluated as a negative control; the corpus must produce more than 10% disagreements, proving that the test can detect ignored rotations.

The test deliberately avoids calling Symtropy support maps or GJK from the reference implementation. It is still a synthetic mathematical reference, not a comparison against a physical experiment or an external engine.

## Reproduction

```bash
cargo test --locked -p symtropy-physics --test vvuq_reference
```

The focused workflow pins the repository Rust toolchain, runs formatting and this integration test on the exact triggering revision, and preserves the test log plus source revision as a CI artifact.

## Industry comparison and next gates

- Rapier, Jolt or MuJoCo comparisons should be added only after matching shape definitions, transforms, tolerance rules, solver/contact semantics and versions.
- Next geometry tiers: 3D 15-axis OBB SAT, convex polygon/hull pairs, degeneracy and grazing corpus, EPA depth and witness error, then public analytic TOI/CCD datasets.
- Next VVUQ tiers: oscillator/pendulum reference, energy/momentum budgets for closed systems, constrained-system residuals, independent inertia/asymmetric-top solutions, and finally blind experimental points with measurement uncertainty.

ASME V&V 20 requires simulation accuracy to be assessed at specified validation points while accounting for both numerical and experimental uncertainty. These two synthetic seed tests cover only the analytical/code-verification part of that larger process; they cannot establish physical validation or a global model-validity claim.

## Evidence to publish

For each exact-head run, preserve the source commit, toolchain, seed, case count, reference algorithm version, tolerances, false-positive/negative counts, translation-only negative-control count, and raw test log. If the corpus changes, increment its version and retain the prior seed for regression comparison.

## References

- [Symtropy research validation envelope](../../crates/core/symtropy-physics/RESEARCH_VALIDATION.md).
- [Orientation-aware collision validation limits](../../crates/core/symtropy-physics/ORIENTED_COLLISION_VALIDATION.md).
- [ASME V&V 20: Verification and Validation in CFD and Heat Transfer](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-fluid-dynamics-and-heat-transfer).
- [NASA CFD verification assessment](https://www.grc.nasa.gov/www/wind/valid/tutorial/verassess.html).
