# Semi-explicit port-Hamiltonian DAE discrete-gradient pendulum reference

**Status:** research reference implementing a specialized semi-explicit pHDAE discrete-gradient step; not a general multibody engine.  
**Date:** 2026-10-31  
**Related work:** [#3108 PHYS-PORT-DAE-001](https://github.com/Luminous-Dynamics/symtropy/issues/3108), [#3113 intrinsic pendulum oracle](https://github.com/Luminous-Dynamics/symtropy/pull/3113), [#3114 Cartesian RATTLE baseline](https://github.com/Luminous-Dynamics/symtropy/pull/3114), [#3115 matched solver campaign](https://github.com/Luminous-Dynamics/symtropy/pull/3115), [#460 qualification corpus](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Literature grounding

This implementation specializes the multibody construction in:

Kinon, Philipp L.; Morandin, Riccardo; Schulze, Philipp. “Discrete gradient methods for port-Hamiltonian differential-algebraic equations.” *Applied Numerical Mathematics* 223 (2026), 45–75. DOI [10.1016/j.apnum.2025.12.006](https://doi.org/10.1016/j.apnum.2025.12.006). Open full text: [KIT institutional copy](https://publikationen.bibliothek.kit.edu/1000189990/173848887).

The paper models constrained mechanics as a semi-explicit pHDAE with state (q,v,λ), Hamiltonian H(q,v)=1/2 vᵀMv+V(q), and hidden velocity constraint Dg(q)v=0. Its discrete method uses a discrete gradient for H and a discrete Jacobian for the holonomic constraint. The paper proves position-level drift-off is eliminated when the initial position constraint holds and the discrete Jacobian identity is satisfied.

Important qualification: the paper's Remark 12 says the velocity-level constraint is only approximately satisfied in general for its discrete-Jacobian scheme. We preserve and measure this distinction. We do not claim exact endpoint tangency for this method.

## Model

For a planar pendulum:

\[
q\in\mathbb R^2,\quad v=\dot q,\quad
g(q)=\tfrac12(q^Tq-L^2)=0,
\]

\[
H(q,v)=\tfrac12m v^Tv -m g_{\mathrm{vec}}^Tq + m\lVert g_{\mathrm{vec}}\rVert L.
\]

The semi-explicit pHDAE is

\[
\dot q=v,
\qquad
m\dot v=m g_{\mathrm{vec}}-Dg(q)^T\lambda+u,
\qquad
0=Dg(q)v.
\]

There is no viscous dissipation in this first slice; the input force u is a constant power port. A later extension can add a positive-semidefinite Rayleigh dissipation operator without changing the evidence boundary.

## Discrete-gradient step

For the circle constraint, the exact discrete Jacobian is

\[
Dg(q_0,q_1)=\bar q=\frac{q_0+q_1}{2},
\]

and it satisfies

\[
Dg(q_0,q_1)(q_1-q_0)=g(q_1)-g(q_0).
\]

The discrete equations are

\[
q_1-q_0=h\bar v,
\]

\[
m(v_1-v_0)=hm g_{\mathrm{vec}}+hu-h\bar q\lambda,
\]

\[
\bar q^T\bar v=0,
\qquad
\bar v=\frac{v_0+v_1}{2}.
\]

The constraint equation and kinematic update give

\[
g(q_1)-g(q_0)=Dg(q_0,q_1)(q_1-q_0)
=h\bar q^T\bar v=0.
\]

Therefore a valid initial position constraint is preserved at each accepted step, to the nonlinear solve and floating-point envelope.

The discrete energy identity is

\[
H_1-H_0
=h u^T\bar v-h\lambda\bar q^T\bar v
=h u^T\bar v.
\]

Thus the reaction has zero discrete work and external work is accounted for separately. No post-hoc energy correction is applied.

## Specialized scalar multiplier solve

Eliminating q1 and v1 gives a scalar nonlinear equation for λ. Define

\[
N(\lambda)=2m v_0+hm g_{\mathrm{vec}}+hu-h\lambda q_0,
\quad
d(\lambda)=2m+\frac{h^2\lambda}{2}.
\]

Then

\[
\bar v(\lambda)=\frac{N(\lambda)}{d(\lambda)},
\qquad
F(\lambda)=q_0^T\bar v+\frac h2\bar v^T\bar v=0.
\]

The source computes the analytic derivative F′(λ), solves with Newton and residual-decreasing backtracking, and rejects singular or non-convergent solves. The continuous acceleration-level multiplier is only the initial guess; the final multiplier comes from the discrete constraint equation.

## Three different constraint observables

These are intentionally not collapsed into one “constraint residual”:

1. **Position-level:** |q1ᵀq1−L²|. The discrete Jacobian identity makes this zero to solve/roundoff tolerance.
2. **Midpoint algebraic velocity:** |Dg(q0,q1) vbar| = |qbarᵀvbar|. This is the actual algebraic equation enforced by the semi-explicit pHDAE step.
3. **Endpoint hidden velocity:** |q1ᵀv1|. It is measured and reported, but is not constrained to zero exactly by this method. For the frozen pendulum fixture it converges at approximately second order as h is refined.

The receipt also records the reaction work, supplied input work, Hamiltonian change, energy-balance residual, scalar multiplier residual, and Newton iteration count.

## Frozen tests and independent prototype checks

The dependency-free Rust source is:

tools/pendulum-phdae-reference/semi_explicit_phdae_dg.rs

The test contract includes:
- position and midpoint algebraic constraints over 5,000 steps;
- long-horizon energy drift and per-step discrete power balance without post-correction;
- separate external-force work and constraint-reaction work;
- endpoint hidden-velocity residual reported and shown to converge quadratically under h-halving;
- second-order final-state convergence against an independent intrinsic-coordinate RK4 reference;
- rotation metamorphism and fail-closed initial-manifold/input/solver controls.

An independent equation-level numerical prototype (not Rust CI output) gave:

| h (s) | Max endpoint tangent residual (absolute) | Final state error vs 80k-step RK4 | Relative energy drift |
|---:|---:|---:|---:|
| 0.040 | 6.8142e-3 | 1.4793e-2 | 1.32e-15 |
| 0.020 | 1.6997e-3 | 3.6703e-3 | 1.65e-15 |
| 0.010 | 4.2469e-4 | 9.1776e-4 | 8.27e-16 |
| 0.005 | 1.0619e-4 | 2.2945e-4 | 1.82e-15 |

The endpoint tangent residual drops by approximately 4× per timestep halving, while energy drift remains at floating-point scale. Position and midpoint algebraic velocity residuals were also at roundoff scale. These are independent equation-level checks, not Rust implementation results or CI passes.

## Comparison against the other reference methods

This creates a useful method portfolio:

| Method | Position constraint | Endpoint tangent | Energy/power balance | Character |
|---|---|---|---|---|
| Intrinsic discrete gradient (#3113) | Exact by coordinates | Exact by coordinates | Exact discrete identity | Reduced-coordinate oracle; not a Cartesian DAE |
| Cartesian RATTLE (#3114) | Exact to solve tolerance | Exact to solve tolerance | Bounded energy error | Symplectic constrained baseline |
| Semi-explicit pHDAE DG (this reference) | Exact to solve tolerance | Approximate; measured and convergent | Exact discrete balance to solve tolerance | Cartesian pHDAE discrete-gradient candidate |

A strong follow-on method should try to improve the Pareto frontier: preserve position and energy while reducing endpoint hidden-velocity error, ideally without state projection and without forfeiting the discrete power identity. G-equivariant discrete gradients are a relevant next research direction for preserving momentum maps such as angular momentum, as discussed by the paper.

## Claim ceiling

This is a specialized one-degree-of-freedom Cartesian semi-explicit pHDAE reference with constant mass and gravity. It does not implement general nonlinear multibody systems, multiple constraints, Rayleigh damping, joints, flexible bodies, impact, friction, a general DAE descriptor solver, adaptive timestepping, sparse solver infrastructure, or PhysicsWorld integration. It is based on a published method and does not claim novelty of that mathematical scheme. No SOTA, production, or physical-validation claim is made.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/pendulum-phdae-reference/semi_explicit_phdae_dg.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/pendulum-phdae-reference/semi_explicit_phdae_dg.rs -o /tmp/phdae-pendulum-tests
/tmp/phdae-pendulum-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/pendulum-phdae-reference/semi_explicit_phdae_dg.rs -o /tmp/phdae-pendulum
/tmp/phdae-pendulum > /tmp/phdae-pendulum-a.csv
/tmp/phdae-pendulum > /tmp/phdae-pendulum-b.csv
cmp /tmp/phdae-pendulum-a.csv /tmp/phdae-pendulum-b.csv
~~~
