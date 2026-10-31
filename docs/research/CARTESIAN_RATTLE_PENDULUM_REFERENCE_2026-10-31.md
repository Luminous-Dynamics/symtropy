# Cartesian RATTLE pendulum reference

**Status:** experimental symplectic constrained-mechanics baseline; not the new port-Hamiltonian DAE method.  
**Date:** 2026-10-31  
**Prerequisites:** [#3113 intrinsic pendulum oracle](https://github.com/Luminous-Dynamics/symtropy/pull/3113), [#3108 PHYS-PORT-DAE-001](https://github.com/Luminous-Dynamics/symtropy/issues/3108), [#460 engine-wide physics qualification corpus](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Purpose

The intrinsic pendulum oracle provides a reduced-coordinate reference. This increment adds a distinct Cartesian constrained integrator baseline: RATTLE. RATTLE is a symplectic constrained method with position and momentum multiplier stages. It is not an energy-exact discrete-gradient DAE method. Keeping these methods separate gives the later #3108 DAE implementation something meaningful to beat or trade against.

## Model

For a mass m constrained to a circle of radius L in a uniform gravity vector g:

\[
q\in\mathbb R^2,\qquad
c(q)=\frac12(q^Tq-L^2)=0,
\]

\[
H(q,p)=\frac{p^Tp}{2m}-m g^Tq+m\lVert g\rVert L.
\]

The momentum must be tangent to the circle:

\[
q^Tp=0.
\]

The reference performs a RATTLE step:

1. Apply the first half gravity impulse and solve a scalar position multiplier so q1 lies on the circle.
2. Derive the half-step momentum from the constrained chord.
3. Apply the second half gravity impulse and solve the momentum multiplier so q1ᵀp1=0.

Both multipliers and both residuals are returned. Invalid initial position/tangent constraints are rejected; failure to converge the position multiplier is explicit.

## Claim boundary

RATTLE is symplectic for the constrained Hamiltonian setup, but it does not exactly conserve the nonlinear Hamiltonian at every step. Energy error is therefore measured as a separate diagnostic. The code does not label an energy change as physical dissipation, nor does it apply post-hoc energy correction.

The reference is frictionless, torque-free, planar, and uses constant gravity. It has no contact impact, friction, generalized multibody graph, damping, actuators, or PhysicsWorld integration.

## Frozen comparison campaign

| Test | Setup | Acceptance |
|---|---|---|
| Hanging equilibrium | q=(0,-L), p=0 | State remains stationary to roundoff |
| Position and tangent constraints | 5,000 steps, h=0.002 | Both relative residuals remain below 1e-10 |
| Energy behavior | Same 5,000-step campaign | Maximum relative energy deviation below 5e-4; exact conservation is not claimed |
| Temporal accuracy | h=0.04, 0.02, 0.01, 0.005; one-second terminal time | State-error ratios in [3.6, 4.4] against the independent reduced-coordinate RK4 solution |
| Metamorphic rotation | Rotate q, p and gravity by same planar rotation | Final state commutes with rotation within 2e-12 |
| Negative controls | Off-circle position, non-tangent momentum, invalid mass, zero timestep | Explicit rejection |

An independent Python implementation of the equations yielded maximum position and tangent residuals near 4.5e-16, maximum relative energy deviation 6.9e-5 for the frozen 5,000-step case, and convergence ratios 4.007, 4.002, 4.000. These are equation-level checks, not Rust CI evidence.

## Comparison with the intrinsic discrete-gradient oracle

The two methods have intentionally different strengths:

- **Intrinsic discrete-gradient oracle (#3113):** exact circle geometry by parameterization and an exact discrete energy/power identity; no Cartesian multiplier or DAE.
- **Cartesian RATTLE reference (this PR):** explicit Cartesian position and tangent constraints with two Lagrange-multiplier stages; symplectic constrained baseline, but bounded energy error rather than exact energy conservation.
- **Future port-Hamiltonian DAE (#3108):** must be evaluated independently for whether it can achieve both high-quality constraint satisfaction and a discrete power/energy balance without simply projecting states after an unconstrained step.

All three must use identical physical initial conditions, output times, units, and independently converted observables. An energy-exact intrinsic method is not automatically a fair DAE competitor because it eliminates the constraint through coordinates.

## Industry comparison

MuJoCo documents five integration choices. It recommends implicitfast for many general workloads, notes implicit can matter for coupled rotational dynamics such as multi-link pendula, and describes RK4 as useful for energy-conserving systems; its discrete option couples the integrator and constraint solve for stiff springs/position servos/strong damping. Therefore the comparison must freeze solver choice and settings for each declared workload rather than calling one method universally best. See the [MuJoCo computation documentation](https://mujoco.readthedocs.io/en/latest/computation/index.html).

Rapier documents tradeoffs between timestep, CCD subdivision, and constraint solver iterations: higher iteration counts can improve accuracy/stability at additional cost, with 8–12 suggested for demanding stacks/machinery. Its stabilization pass is explicitly used to prevent position-correction energy from remaining in the simulation. This is a useful contrast to measure, not proof that either implementation is universally better. See [Rapier integration parameters](https://rapier.rs/docs/user_guides/rust/integration_parameters/).

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/pendulum-rattle-reference/rattle_cartesian.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/pendulum-rattle-reference/rattle_cartesian.rs -o /tmp/rattle-tests
/tmp/rattle-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/pendulum-rattle-reference/rattle_cartesian.rs -o /tmp/rattle-ref
/tmp/rattle-ref > /tmp/rattle-a.csv
/tmp/rattle-ref > /tmp/rattle-b.csv
cmp /tmp/rattle-a.csv /tmp/rattle-b.csv
~~~
