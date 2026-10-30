# Small-island normal-contact LCP reference implementation

**Date:** 2026-10-30  
**Status:** experimental mathematical reference; not wired into the production world solver  
**Research specification:** [PR #2966](https://github.com/Luminous-Dynamics/symtropy/pull/2966)

## Purpose

The current production contact path uses sequential impulses. Before replacing or hybridizing it, Symtropy needs an independent and transparent oracle for small contact islands. This increment implements the normal-contact linear complementarity problem (LCP) for up to eight rows by exhaustive active-set enumeration.

Given the Delassus matrix W and free normal velocity q, solve for normal impulses lambda:

```text
lambda >= 0
w = W lambda + q >= 0
lambda_i * w_i = 0 for every row i
```

The sign convention is impulse-positive: q < 0 means a row is closing, and W maps positive normal impulses to changes in separating velocity. A bias or target velocity is included in q by the caller.

## Implemented code

Module: `crates/core/symtropy-physics/src/discrete_contact.rs`

- `build_contact_jacobian<D>` assembles relative-normal rows from body indices, unit-normal directions and per-body COM offsets. Its generalized coordinate block is D translational values followed by lexicographically ordered bivector planes; angular coefficients use the physical -B*r point-velocity convention.
- `assemble_delassus` computes W = J M_eff^-1 J^T from that generalized row Jacobian and a symmetric PSD inverse effective-mass matrix. It validates the input metric and raw product before averaging only roundoff-sized asymmetry.
- `validate_delassus` checks nonempty square dimensions, finite entries, scale-relative symmetry and PSD within a caller-supplied tolerance. It does not silently project an invalid operator.
- `contact_lcp_kkt_residual` independently recomputes the minimum impulse, minimum slack and maximum complementarity product.
- `solve_normal_contact_lcp_reference` enumerates active masks in deterministic ascending order, solves each nonsingular active principal submatrix, and returns the first candidate meeting the unilateral KKT tolerance.
- Singular active submatrices are skipped, allowing a redundant PSD system to be solved by a smaller independent active subset where one exists.
- The eight-row limit bounds exhaustive enumeration to 256 active sets and keeps this code useful as a transparent reference.

## Regression coverage

The tests cover:

1. 3D contact-row translation and bivector sign/index layout;
2. all six 4D bivector plane coefficients;
3. 3D contact rows producing a symmetric PSD Delassus matrix;
4. a known generalized Jacobian/inverse-mass matrix assembling to the analytical Delassus matrix;
5. rejection of an indefinite inverse effective-mass metric;
6. one-row analytical impulse;
7. coupled two-row analytical solution;
8. separating rows receiving zero impulse;
9. redundant positive-semidefinite rows solved through a nonsingular subset;
10. asymmetric Delassus rejection;
11. indefinite Delassus rejection;
12. explicit rejection above eight rows;
13. all 256 active sets for an eight-row fully active system; and
14. no-feasible-solution handling for a closing row with zero effective mass.

## Explicit limits

This is a normal-only LCP oracle. It is not a complete Coulomb-friction contact solver, a maximum-dissipation cone solver, or a production island solver. It does not assemble W from Symtropy body Jacobians/inertia operators, choose CCD substeps, model restitution, or apply impulses to `RigidBody` objects.

The solver returns the first feasible active set in mask order. It does not promise a minimum-norm solution for non-unique LCPs. Ill-conditioned and singular active sets need additional independent treatment; skipping a singular submatrix is not a universal remedy for all degenerate LCPs.

## Next gates

- Independently assemble W from generalized translational + bivector response for D=2,3,4.
- Compare matrix entries against direct per-row impulse experiments and check W symmetry/PSD.
- Compare active-set results with an independent convex-QP/KKT oracle, not just this implementation's own residual routine.
- Extend to friction with a declared maximum-dissipation law; do not treat independent tangential scalar clamps as a cone solver.
- Build an island-level staged commit path using the phase-aware world failure receipts from [PR #2983](https://github.com/Luminous-Dynamics/symtropy/pull/2983).
- Benchmark residual/work-precision against sequential impulses, MuJoCo's discrete integrator, PhysX TGS and Rapier at matched physical error.

## Promotion thresholds

- Delassus symmetry error <= 1e-12 relative;
- no material negative eigenvalue beyond declared roundoff tolerance;
- normal impulse/effective-mass identity <= 1e-10 relative;
- complementarity and primal/dual feasibility residuals <= 1e-10 relative on the frozen reference suite;
- zero invalid-Delassus acceptance on asymmetric/indefinite negative controls;
- no unexplained increase in mechanical energy after the work ledger is integrated.

Passing these small-island checks will validate the oracle and assembly contract; it will not establish an industry advantage by itself.
