# Nonlinear port-Hamiltonian quartic discrete-gradient reference

**Status:** experimental, standalone numerical reference; not a production PhysicsWorld integrator.  
**Date:** 2026-10-31  
**Parent:** [quadratic port-Hamiltonian reference PR #3102](https://github.com/Luminous-Dynamics/symtropy/pull/3102)

## Why this is the next useful increment

The quadratic prototype in port_hamiltonian.rs demonstrates that a structure-preserving implicit step can report stored energy, external port work, dissipation, and algebraic residual separately. Its exact discrete gradient is linear, however. Nearly every useful mechanical potential is nonlinear.

This child moves one step beyond that limitation without pretending to solve the whole multibody problem: it admits a nonlinear quartic potential and tests the algorithm against an independent high-accuracy ODE integrator. The purpose is to establish a reusable nonlinear energy-balance contract before adding rotations, constraints, contacts, and differential-algebraic equations.

## Mathematical contract

The state is x = (q,p), with Hamiltonian

\[
H(q,p)=\frac{p^2}{2m}+\frac{kq^2}{2}+\frac{\beta q^4}{4},
\qquad m>0,\; k,\beta,\gamma\geq0.
\]

The canonical interconnection is skew-symmetric, the damping operator is positive semidefinite, and the external input is a power port:

\[
\dot q=\frac{p}{m},\qquad
\dot p=-kq-\beta q^3-\gamma\frac{p}{m}+u,
\qquad y=\frac{p}{m}.
\]

The continuous balance is

\[
\dot H=-\gamma y^2+yu.
\]

For endpoints q0 and q1, the potential discrete gradient is

\[
\overline{\nabla}V(q_0,q_1)
=\frac{k}{2}(q_0+q_1)
+\frac{\beta}{4}
(q_1^3+q_1^2q_0+q_1q_0^2+q_0^3).
\]

At q0=q1, its continuous-gradient limit is kq+βq³. More importantly, it satisfies the exact finite-difference identity

\[
(q_1-q_0)\overline{\nabla}V(q_0,q_1)
=V(q_1)-V(q_0).
\]

The kinetic discrete gradient is (p0+p1)/(2m). With constant input u over one step of size h, the method solves

\[
q_1-q_0=h\frac{p_0+p_1}{2m},
\]

\[
p_1-p_0=-h\overline{\nabla}V(q_0,q_1)
-h\gamma\frac{p_0+p_1}{2m}+hu.
\]

Consequently, an exactly solved step obeys

\[
H_1-H_0
=\underbrace{huy_{1/2}}_{\text{supplied work}}
-\underbrace{h\gamma y_{1/2}^2}_{\text{dissipated energy}},
\qquad y_{1/2}=\frac{p_0+p_1}{2m}.
\]

The implementation does not enforce this by modifying energy after the update. It follows from the discrete-gradient identity and the implicit equations. Finite nonlinear-solve error is measured separately and appears in the reported balance residual.

## Implemented reference

The new standalone Rust source at tools/port-hamiltonian-nonlinear/quartic_discrete_gradient.rs is dependency-free and contains:

- A validated one-degree-of-freedom nonlinear port-Hamiltonian model.
- The exact quartic discrete gradient, including the coincident-endpoint limit.
- An implicit average-vector-field/discrete-gradient update.
- A two-variable Newton solve with an analytic Jacobian and residual-decreasing backtracking.
- Fail-closed rejection for invalid parameters, non-finite state/input, invalid solver controls, singular Newton systems, and non-convergence.
- A step receipt containing raw and scaled nonlinear residuals, Newton iterations, stored energy, supplied work, dissipated energy, energy-balance residual, and midpoint port output.
- A deterministic 100-step CSV fixture for replay.

No energy projection, velocity clipping, post-hoc normalization, or silent fallback to an explicit step is used.

## Frozen verification campaign

The standalone test module contains these gates:

| Gate | Fixture | Required result |
|---|---|---|
| Discrete-gradient identity | 41 × 41 endpoint grid, plus q0=q1 | Potential finite difference matches the discrete-gradient product to roundoff-scaled tolerance |
| Conservative invariant | 2,000 steps at h=0.01, β=0.75 | Per-step balance residual below 2e-12; accumulated relative energy drift below 2e-11 |
| Dissipative passivity | γ=0.4 | Nonnegative recorded dissipation, lower stored energy for zero input, closed balance |
| External power port | γ=0.2, u=0.7 | Input work and dissipation remain separately attributed; closed balance |
| Temporal accuracy | h=0.1, 0.05, 0.025, 0.0125 against 40,000-step RK4 reference | Successive state-error ratios in [3.6, 4.4], consistent with second order on this fixture |
| Negative controls | Invalid mass, timestep, solver configuration, NaN state | Explicit error; no accepted receipt |

The RK4 fixture is an independent accuracy reference, not an energy-conserving truth oracle. The algebraic discrete-gradient identity and the per-step energy ledger are the independent structure checks.

## Competitive evaluation: what to measure next

Do not rank integrators by raw steps per second. Compare at matched accuracy and the same initial conditions, force law, timestep policy, output cadence, and total compute budget.

1. **Accuracy and structure:** trajectory error against a highly refined reference; energy drift; power-balance residual; momentum/angular-momentum drift where applicable; constraint residual; phase error.
2. **Robustness:** accepted/rejected steps, Newton iterations, line-search backtracks, failure rate, and behavior near stiff or nearly singular states.
3. **Efficiency:** wall-clock time, allocations, peak memory, factorization/iteration cost, and verified simulated time per CPU-second.
4. **Reproducibility:** exact source/config/toolchain digests, deterministic CSV replay, fixed seeds for randomized extensions, and an independently implemented verifier.

Candidate baselines for the next comparative campaign:

- Explicit RK4 (high-order accuracy reference; not structure preserving).
- Semi-implicit/symplectic Euler (cheap long-horizon baseline for separable mechanics).
- Implicit midpoint (second-order, symplectic for canonical Hamiltonian systems, but not generally exact-energy preserving for nonlinear H).
- A higher-order or adaptive discrete-gradient method.
- The current Symtropy real-time TGS / semi-implicit path, once the same model is represented in the production engine.

The fair objective is a Pareto frontier over verified error, physical-balance defects, robustness, and total compute cost—not an isolated speed or energy number.

## Industry comparison and claim ceiling

This one-degree-of-freedom research fixture is **not directly comparable** with MuJoCo, Rapier, PhysX, Bullet, Chrono, or OpenFOAM. Those systems solve different workloads and expose different integration/contact pipelines. The next fair comparison ladder should be:

1. A common smooth nonlinear oscillator and driven/damped fixtures.
2. A 3D asymmetric rigid-body torque-free top, checking energy and world angular momentum.
3. A pendulum / double-pendulum system with holonomic constraints.
4. A frictionless impact and then a frictional contact fixture, with an explicit contact work/impact ledger.
5. A coupled rigid-body + SPH or continuum subsystem only after each subsystem has its own verified reference.

Compare against current industry methods on their own intended profiles: realtime contact throughput and stability; high-accuracy offline integration; constrained multibody robustness; reproducibility and energy/constraint residuals. Never use the scalar oscillator result to claim engine-wide superiority.

The present implementation establishes only a nonlinear, smooth, one-degree-of-freedom discrete-gradient reference. It does **not** implement rigid-body rotation, joints, constraints, impact, friction, DAE solvers, adaptive time stepping, sparse linear algebra, or PhysicsWorld integration. No SOTA, production, or performance claim is made.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/port-hamiltonian-nonlinear/quartic_discrete_gradient.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/port-hamiltonian-nonlinear/quartic_discrete_gradient.rs -o /tmp/quartic-dg-tests
/tmp/quartic-dg-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/port-hamiltonian-nonlinear/quartic_discrete_gradient.rs -o /tmp/quartic-dg
/tmp/quartic-dg > /tmp/quartic-dg-a.csv
/tmp/quartic-dg > /tmp/quartic-dg-b.csv
cmp /tmp/quartic-dg-a.csv /tmp/quartic-dg-b.csv
~~~
