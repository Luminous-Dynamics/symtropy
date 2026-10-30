# Small-island Delassus reference oracle

**Status:** experimental mathematical reference; not a production solver.
**Date:** 2026-10-30
**Research campaign:** [unified discrete contact/effective-mass solve](DISCRETE_CONTACT_EFFECTIVE_METRIC_CAMPAIGN_2026-10-30.md)

## Purpose

The project needs an independent standard against which to test its production sequential-impulse/TGS contact solver and the future unified discrete solver. Visual stack stability is insufficient: a solver can appear stable while assembling the wrong effective mass, violating complementarity, or injecting energy.

This increment implements a transparent small-island reference for frictionless unilateral contacts. It is intentionally bounded to 1–8 rows so exhaustive active-set enumeration is practical and inspectable.

## Mathematical contract

For generalized velocity (u=(v,B)), where (v\in\mathbb R^D) and (B\in\Lambda^2(\mathbb R^D)), each row is a relative point-velocity constraint with unit normal (n_i), two body handles, and center-of-mass lever arms (r_{i,a},r_{i,b}).

The response of row j's unit impulse is evaluated at row i's contact points. The matrix is assembled as:

\[
W = J M^{-1}J^T,\qquad W_{ij}=J_i M^{-1}J_j^T.
\]

The generalized response uses:
- the body's translational inverse mass;
- the same body-space `BivectorInertia` rotated into world coordinates used by the integrator;
- the same stored-bivector sign conversion and physical point-velocity helper as contact resolution;
- the explicitly identified legacy isotropic fallback only when no authoritative inertia operator is present.

The reduced frictionless contact problem is:

\[
\lambda\ge 0,\qquad w=W\lambda+q\ge0,\qquad
\lambda^T w=0.
\]

Here q is the pre-solve relative normal velocity minus the authored target/bias. This module assembles W; it does not invent restitution, stabilization, or compliance semantics.

## Reference algorithm

1. Validate row handles, finite lever arms, and unit normals.
2. Assemble the full (not forcibly symmetrized) Delassus matrix by applying each column impulse through the generalized response and observing each row's relative point velocity.
3. Validate symmetry and positive semidefiniteness when solving.
4. Enumerate all 2^n active sets for n<=8.
5. Solve each active block with SVD least-norm semantics. Rank-deficient blocks are not silently regularized; feasibility is checked against the original unmodified matrix.
6. Return impulses, residual, active rows, primal violation and complementarity residual. If no feasible active set is found, return an explicit failure.

## Regression coverage

The module includes:
- one-row analytic impulse;
- coupled two-row analytic active solution;
- inactive row with exactly zero impulse;
- rejection of asymmetric and indefinite matrices;
- redundant rank-deficient contact rows without diagonal regularization;
- common coefficient scaling and an explicit SI-to-g/cm/ms dimensional conversion;
- assembled two-body rows with asymmetric rotational inertia, checking Delassus symmetry, PSD and a non-zero off-diagonal coupling;
- an end-to-end solve in which the assembled asymmetric Delassus matrix is fed to the LCP oracle;
- isolated sphere contact with analytic translational effective mass.

These tests are source-level fixtures only until the exact branch head passes Rust formatting, compilation, Clippy and tests.

## Microbenchmarks

The package now has a Criterion target:

```sh
cargo bench -p symtropy-physics --bench delassus_reference
```

It measures Delassus assembly and exhaustive LCP solve independently at 1, 2, 4 and 8 rows for a fixed asymmetric 3D two-body fixture. The timing output must be accompanied by host/compiler metadata and the exact source SHA. These are microbenchmarks for the reference layer, not engine-level throughput comparisons. No measurements are claimed until the benchmark is actually run.

## Mechanical evidence increment

The runtime contact solver now records frictional dissipation using the kinetic-energy change implied by the actual tangential impulse and the same effective-mass denominator used to apply it:

\[
\Delta K_t=j_t\left(v_t+\frac12d_tj_t\right),\qquad
E_{\mathrm{diss},t}=\max(0,-\Delta K_t).
\]

A runtime callback regression reconstructs the applied impulse from the actual post-step contact-point velocity and checks the reported joules. See [Mechanical Work Ledger](MECHANICAL_WORK_LEDGER_2026-10-30.md). This is only friction work; a complete contact/step energy ledger remains future work.

## Planar friction reference

The cumulative implementation also adds a separate planar, single-contact Signorini/maximum-dissipation reference in `friction_cone.rs`. It exhaustively checks separation, sticking, and both sliding directions under the scalar Coulomb bound. Its input effective-mass matrix must be SPD; it reports no-admissible-solution rather than silently regularizing a degenerate contact. Criterion now measures this reference separately.

The reference is deliberately 2D and one-contact only. Its randomized 256-seed suite checks Signorini complementarity, Coulomb-cone feasibility, sliding maximum-dissipation and non-positive tangential work over SPD effective metrics. It does not yet solve a 3D circular friction cone, simultaneous multi-contact friction, or 6D torsional/rolling friction.

## Scope and caveats

- Frictionless normal rows only. This is not a Coulomb friction cone solver.
- At most eight rows. It is an oracle, not a scalable production implementation.
- It does not include joints, damping/stiffness effective-metric terms, CCD, position-level constraints, or state integration.
- It does not assemble q; callers must use the same signed relative-velocity/target convention consistently.
- It must not replace the current runtime solver until matched-work tests demonstrate correctness, stability and performance.

## Promotion gates

Before the reference is trusted as a promotion oracle:
- symmetry error <= 1e-12 relative on well-conditioned analytic cases;
- minimum eigenvalue >= -1e-12 relative for valid unconstrained mass operators;
- effective-mass impulse identity <= 1e-10;
- primal and complementarity residuals <= 1e-10 on analytic and randomized small cases;
- dimensional conversion invariance for impulse solutions across SI and g/cm/ms representations;
- explicit singular-inertia and redundant-row negative controls;
- oracle-versus-production-solver residual comparisons across randomized seeds and D=2,3,4.

The next extension is not a faster LCP implementation. It is a test campaign comparing production TGS impulses and post-step velocities with this independent oracle, then adding friction maximum-dissipation cases only after the frictionless model is trusted.
