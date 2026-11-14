# Three-dimensional Coulomb friction: fixed-normal-load disk oracle

**Status:** bounded reference subproblem; not a complete contact solver.  
**Date:** 2026-10-31  
**Related production programme:** [#3012 asymmetric inertia / contact verification](https://github.com/Luminous-Dynamics/symtropy/pull/3012), [#2998 Delassus reference](https://github.com/Luminous-Dynamics/symtropy/pull/2998), [#3142 SO(4) Moser-Veselov reference](https://github.com/Luminous-Dynamics/symtropy/pull/3142).

## Why this slice

The existing exact friction reference is planar single-contact. A 3D point contact has a two-dimensional tangent plane. The tangent impulse is therefore a vector constrained to a circular Coulomb disk, not a signed scalar interval. The tangential effective-mass matrix can be anisotropic and off-diagonal, even for an isotropic material, because of lever arms, body inertia and contact-frame orientation.

This PR isolates the local tangential maximum-dissipation subproblem after a normal impulse has been selected. It is a deliberately smaller claim than a coupled normal/tangent contact solve.

## Mathematical contract

Given:
- SPD tangential inverse-effective-mass/response matrix \(W_{tt}\);
- tangential velocity \(v_t\) after the fixed normal impulse, including \(W_{tn}\lambda_n\);
- friction coefficient \(\mu\ge0\);
- fixed non-negative normal impulse \(\lambda_n\);

solve:

\[
\min_{j_t}\quad \frac12 j_t^T W_{tt}j_t + v_t^Tj_t
\]

subject to:

\[
\|j_t\|_2\le r,\qquad r=\mu\lambda_n.
\]

If the unconstrained solution \(-W_{tt}^{-1}v_t\) lies inside the disk, it is the sticking solution and post-slip velocity is zero.

Otherwise, the KKT conditions give:

\[
(W_{tt}+\tau I)j_t=-v_t,\quad
\|j_t\|_2=r,\quad \tau>0.
\]

The impulse norm is monotone decreasing in \(\tau\). The reference brackets and bisects this one-dimensional secular equation. It does not use independent scalar clamps on tangent axes.

For the frozen-metric impulse response, signed kinetic-energy change is:

\[
\Delta K_t=j_t^Tv_t+\frac12j_t^TW_{tt}j_t.
\]

The accepted result must have non-positive \(\Delta K_t\), satisfy the disk constraint, and meet the normalized stationarity residual.

## Implemented

- New public module friction_disk with a fixed-load, two-tangent maximum-dissipation oracle.
- SPD/symmetry/finite-input validation and typed failures.
- Exact unconstrained sticking solve.
- Boundary sliding solve through deterministic bracketed bisection of the trust-region multiplier.
- Reports impulse, post-slip velocity, disk radius, multiplier, disk violation, KKT stationarity residual and signed work.
- Tests for sticking, sliding, anisotropic off-diagonal response, normal-to-tangent coupling folded into the post-normal free slip, contact-frame rotational covariance, frictionless/zero-load cases and invalid metrics.
- 128 deterministic anisotropic metric cases with KKT and dissipation checks.

## Explicit non-goals

This API does not solve the normal complementarity equation, and it does not iterate the normal impulse after the tangential impulse changes the normal velocity through W_nt. It is therefore not a complete Signorini-Coulomb point-contact solve, not a multi-contact solver, and not a contact-wrench/rolling-friction solver. The full coupled contact problem remains a follow-on.

No performance or SOTA claim is made. This reference is intended to become an independent oracle for a future coupled 3D/4D friction solver.
