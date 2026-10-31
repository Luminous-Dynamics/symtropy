# Intrinsic pendulum discrete-gradient reference

**Status:** experimental nonlinear smooth-system oracle; not a Cartesian constrained DAE solver.  
**Date:** 2026-10-31  
**Related work:** [#3108 PHYS-PORT-DAE-001](https://github.com/Luminous-Dynamics/symtropy/issues/3108), [#3106 nonlinear quartic reference](https://github.com/Luminous-Dynamics/symtropy/pull/3106), [#460 engine-wide physics qualification corpus](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Purpose

The next constrained-mechanics experiment needs an independent reference that does not share the Cartesian multiplier algebra being tested. This implementation represents the planar pendulum in intrinsic canonical coordinates. Its circle geometry is exact by construction, the gravitational potential is nonlinear, and an exact discrete gradient gives a per-step energy/power identity.

This makes it useful as a reference oracle for a later Cartesian DAE, RATTLE/SHAKE, or multibody implementation. It does **not** solve for Cartesian constraint forces or claim to qualify a DAE solver.

## Model

Use the unwrapped angle θ and canonical angular momentum ℓ:

\[
H(\theta,\ell)=\frac{\ell^2}{2I}+mgL(1-\cos\theta),
\qquad I=mL^2.
\]

The equations are

\[
\dot\theta=\frac{\ell}{I},
\qquad
\dot\ell=-mgL\sin\theta-\gamma\dot\theta+\tau.
\]

The conjugate output of the applied torque is y=θ̇. The continuous energy balance is

\[
\dot H=\tau y-\gamma y^2.
\]

The corresponding Cartesian point is defined by

\[
(x,y_c)=(L\sin\theta,-L\cos\theta).
\]

Therefore x²+y_c²=L² to floating-point evaluation accuracy. This is a geometric parameterization, not a numerical constraint enforcement algorithm.

## Exact discrete gradient

For endpoints θ0, θ1, the gravitational potential V(θ)=mgL(1-cos θ) has the exact discrete gradient

\[
\overline{\nabla}V(\theta_0,\theta_1)
=mgL\sin\left(\frac{\theta_0+\theta_1}{2}\right)
\operatorname{sinc}\left(\frac{\theta_1-\theta_0}{2}\right),
\]

where sinc(z)=sin(z)/z with sinc(0)=1. The implementation uses a Taylor series near zero to avoid cancellation and also implements the analytic derivative needed by Newton's method.

This identity gives

\[
(\theta_1-\theta_0)\overline{\nabla}V=V(\theta_1)-V(\theta_0).
\]

The implicit step is

\[
\theta_1-\theta_0=h\frac{\ell_0+\ell_1}{2I},
\]

\[
\ell_1-\ell_0
=-h\overline{\nabla}V
-h\gamma\frac{\ell_0+\ell_1}{2I}+h\tau.
\]

The expected exact discrete power balance is

\[
H_1-H_0
=h\tau\bar\omega-h\gamma\bar\omega^2,
\qquad \bar\omega=\frac{\ell_0+\ell_1}{2I}.
\]

There is no post-hoc energy correction. Any finite nonlinear-solve defect remains visible in the receipt.

## Implementation and verification

The standalone dependency-free Rust source is:

tools/pendulum-intrinsic-reference/pendulum_discrete_gradient.rs

It includes:

- Validated positive mass/length and nonnegative gravity/damping.
- Stable sinc and sinc-derivative evaluations.
- An implicit two-state Newton solver with analytic Jacobian and residual-decreasing backtracking.
- Fail-closed input, singular solve, and non-convergence handling.
- Separate energy, applied-torque work, damping dissipation, raw nonlinear residual, scaled nonlinear residual, and Newton iteration receipts.
- A deterministic 500-step CSV replay.
- An independently coded RK4 accuracy reference in the test module.

### Frozen tests

| Test | Setup | Acceptance |
|---|---|---|
| Discrete-gradient identity | 81 × 81 deterministic endpoint grid | Potential finite difference matches gradient product within roundoff-scaled tolerance |
| Intrinsic geometry | 10,000 steps | Cartesian radius residual < 8e-16 for L=1 |
| Conservative energy | 5,000 steps, h=0.002 | Maximum relative per-step balance residual < 1e-11; accumulated relative energy drift < 3e-11 |
| Driven + damped | γ=0.3, τ=0.7 | Work and dissipation remain separately attributable; balance closes within 1e-11 |
| Temporal order | h=0.04, 0.02, 0.01, 0.005 against 80,000-step RK4 | Successive final-state error ratios in [3.6, 4.4] |
| Negative controls | Invalid model, timestep, solver options, NaN state | Explicit error; no accepted receipt |

An independent numerical prototype gave a worst relative per-step energy residual of approximately 5.9e-12 over the 5,000-step conservative fixture and accumulated energy drift of approximately 8.3e-14. The temporal-convergence ratios were approximately 3.996, 3.999, and 4.000. These are mathematical prototype checks; they are not Rust workflow results or physical validation.

## Comparison plan

The intrinsic pendulum is the common smooth-system oracle. Later Cartesian implementations must be compared against it after converting states and observables through the same angle convention and initial conditions.

The next comparison should include:

1. This intrinsic discrete-gradient reference.
2. A Cartesian RATTLE/SHAKE reference, reporting position and tangent residuals separately.
3. The proposed Cartesian port-Hamiltonian DAE solver from #3108.
4. Symtropy's real-time TGS profile where an equivalent model is supported.
5. External constrained-mechanics solvers only after matching equation, tolerances, output cadence, and compute budgets.

Report trajectory error, phase error, energy drift, constraint residual, reaction work, nonlinear iterations, rejected steps, and wall-clock time. The target is a Pareto frontier, not a single score.

## Claim ceiling

This is a reduced one-degree-of-freedom pendulum reference. It does not implement a Cartesian Lagrange multiplier, RATTLE, SHAKE, general DAE, multibody chain, contact/impact, friction, adaptive timestep, sparse solver, or PhysicsWorld integration. It does not establish SOTA performance or physical-model validation. It is one independent reference lane for the broader #3108 and #460 programmes.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/pendulum-intrinsic-reference/pendulum_discrete_gradient.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/pendulum-intrinsic-reference/pendulum_discrete_gradient.rs -o /tmp/pendulum-dg-tests
/tmp/pendulum-dg-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/pendulum-intrinsic-reference/pendulum_discrete_gradient.rs -o /tmp/pendulum-dg
/tmp/pendulum-dg > /tmp/pendulum-dg-a.csv
/tmp/pendulum-dg > /tmp/pendulum-dg-b.csv
cmp /tmp/pendulum-dg-a.csv /tmp/pendulum-dg-b.csv
~~~
