# SO(4) rigid-body dynamics: generalized discrete Moser-Veselov reference

**Status:** standalone research reference; not a production solver or SOTA claim.  
**Date:** 2026-10-31  
**Parent research:** [#3126, 3D Moser-Veselov asymmetric-top oracle](https://github.com/Luminous-Dynamics/symtropy/pull/3126)  
**Qualification authority:** [PHYS-VALID-06, issue #460](https://github.com/Luminous-Dynamics/symtropy/issues/460)

## Why this matters

SO(4) has six independent rotational coordinates. A generic rotation contains two simultaneous rotations in orthogonal planes; representing it as a single 3D-style axial vector loses information. Symtropy already represents rotations and angular momentum through bivectors, so a four-dimensional reference is a meaningful test of the architecture rather than a dimension-counting exercise.

The physical inertia structure is also unusually useful. Parker (2024) shows that the rigid-body rank-four inertia tensor in arbitrary Euclidean dimension has vanishing traceless Weyl component and is fully determined by its rank-two Ricci contraction. For a centered mass distribution, that contraction can be represented by the symmetric second-moment matrix used here. In a principal frame with second moment S=diag(s_i), the inertia eigenvalue for plane (i,j) is s_i+s_j.

Reference:
- E. Parker, “The Ricci decomposition of the inertia tensor for a rigid body in arbitrary spatial dimensions,” International Journal of Theoretical Physics 63, 3 (2024). https://doi.org/10.1007/s10773-023-05505-7
- B. Leimkuhler et al., “The symmetric representation of the rigid body equations and their discretization,” 2002, discussing SO(n) generalized rigid-body equations and Moser-Veselov discretization.
- “Solving the discrete Euler-Arnold equations for the generalized rigid body motion,” Journal of Computational and Applied Mathematics (2022), describing the generalized SO(n) Moser-Veselov equation and its numerical solution challenges.

This is not a new integrator claimed by Symtropy. The work is an independently implemented, falsifiable SO(4) qualification reference and a bridge to the production bivector-inertia representation.

## Physical inertia from second moments

For centered mass distribution, let S be the symmetric positive-definite second-moment matrix. If W in so(4) is the body angular-velocity bivector, the physical angular-momentum bivector is:

\[
M = S W + W S.
\]

When S is diagonal, each independent rotation plane has inertia:

\[
I_{ij}=s_i+s_j,\qquad 0\le i<j<4.
\]

There are six such planes: 01, 02, 03, 12, 13, 23. The implementation stores their coefficients in exactly this order.

The kinetic energy is:

\[
T=\frac12\langle M,W\rangle
=\frac12\sum_{i<j} M_{ij}W_{ij}.
\]

The model accepts a full symmetric positive-definite S, not only a diagonal matrix. That permits body-basis covariance tests and makes the reference useful for future adapters from mesh- or point-cloud-derived mass properties.

## Discrete Moser-Veselov map

For step size h, solve:

\[
O^T S - S O = h M_n,\qquad O\in SO(4),
\]

then update:

\[
M_{n+1}=O M_n O^T,\qquad Q_{n+1}=Q_n O^T.
\]

The orthogonal increment is parameterized using the Cayley transform:

\[
O=(I-A)^{-1}(I+A),\qquad A^T=-A.
\]

Thus every accepted increment is on SO(4) by construction, without a posteriori projection. The six Cayley parameters are found using a finite-difference Jacobian, pivoted dense solve and residual-decreasing backtracking. This is deliberately a modest reference solver: robust production solvers should compare analytic/Riemannian Jacobians, continuation and trust-region methods.

Generalized Moser-Veselov equations do not guarantee an SO(n) solution for arbitrary data. The implementation therefore fails closed on singular Cayley charts, singular linear systems, failed line searches and non-convergence. It does not convert failure into a zero angular response.

## Continuous-time independent reference

The reference trajectory uses the SO(4) Euler-Arnold equations:

\[
W = \mathcal I_S^{-1}(M),\qquad
\dot M = M W - W M,\qquad
\dot Q=QW.
\]

A separate RK4 implementation serves as a high-accuracy trajectory comparator. It is not treated as the mathematical truth for invariant preservation; the discrete group and momentum identities are checked separately.

## SO(4) invariants

The campaign reports these independently:

1. Kinetic energy.
2. Bivector/Frobenius quadratic Casimir.
3. Pfaffian, the second independent SO(4) quadratic invariant.
4. Spatial momentum map \(Q M Q^T\).
5. SO(4) orthogonality and determinant.
6. Discrete Moser-Veselov equation residual.
7. Time reversibility.
8. Body-basis covariance.
9. Observed temporal order against independent RK4.

The Pfaffian is:

\[
\operatorname{Pf}(M)=M_{01}M_{23}-M_{02}M_{13}+M_{03}M_{12}.
\]

It is preserved under SO(4) conjugation. Checking only the Frobenius norm would not fully distinguish the two orthogonal-plane angular-momentum sectors.

## Implemented scope

The dependency-free Rust source is tools/so4-moser-veselov/so4_reference.rs. It includes:

- Full symmetric SPD second-moment input.
- Six-component bivector coordinates and inertia action.
- SO(4) Cayley increments.
- Six-variable damped Newton solve and fail-closed typed errors.
- Independent Euler-Arnold RK4 reference.
- Three frozen cases: generic double rotation, near-degenerate principal values, and two independent plane spins.
- Tests for energy, both Casimirs, spatial momentum, SO(4) membership, time reversibility, second-order convergence, body-basis covariance, physical scaling, invalid second moments, invalid state and pairwise-sum inertia consistency.
- Deterministic CSV campaign over four step sizes and a pinned standalone Rust 1.96.0 workflow.

## Proposed promotion targets

These are frozen starting gates for a double-precision CPU reference, not achieved results.

| Observable | Target |
|---|---:|
| Relative kinetic-energy drift | ≤ 2e-10 |
| Relative bivector-norm Casimir drift | ≤ 2e-10 |
| Relative Pfaffian drift | ≤ 2e-10 |
| Spatial momentum-map drift | ≤ 2e-10 |
| SO(4) orthogonality and determinant defect | ≤ 2e-10 |
| Discrete equation residual | ≤ 1e-10 |
| Forward/backward state discrepancy | ≤ 2e-10 |
| Body-basis covariance defect | ≤ 2e-9 |
| Observed order under timestep halving | 1.8–2.2 |
| Invalid SPD or non-skew data silently accepted | 0 |

## What this does not prove

- It does not establish general SO(n) production dynamics.
- It does not handle contact, friction, joints, external torque, deformable matter or CCD.
- It does not yet compare the full SO(4) reference against production Symtropy BivectorInertia<4>.
- It does not provide a GPU or high-throughput solver.
- It does not prove that every Moser-Veselov nonlinear equation has a solution.
- It makes no physical validation, performance or SOTA superiority claim.

Promotion requires exact-head Rust workflow results, a production inertia adapter, cross-coordinate independent fixtures, and registration in the existing engine-wide qualification corpus #460.
