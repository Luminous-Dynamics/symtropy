# Analytic Dynamics Qualification v0

The first fixture is a one-dimensional harmonic oscillator embedded in a
three-dimensional PhysicsWorld. This is a deliberately smooth, closed,
conservative system with an exact analytic trajectory.

## Frozen physical model

- Mass: 1 kg.
- Spring constant: 1 N/m.
- Angular frequency: `ω = sqrt(k/m) = 1 rad/s`.
- Initial position: `x(0)=1 m`.
- Initial velocity: `v(0)=0 m/s`.
- Applied force: `F(x)=-k*x` at each step, through the production body force
  accumulator and `PhysicsWorld::step`.
- Gravity: zero.
- Damping: zero.
- Final time: 1 s.

Exact solution:

```text
x(t) = cos(ω t)
v(t) = -ω sin(ω t)
```

The test reports position error in metres and velocity error in metres/second
separately. Do not combine them into a Euclidean norm without introducing and
declaring a characteristic time scale.

## Integrator stability claim

The production integrator is semi-implicit (symplectic) Euler:

```text
v[n+1] = v[n] - dt * ω² * x[n]
x[n+1] = x[n] + dt * v[n+1]
```

For the linear oscillator, the state-update matrix has determinant 1 and trace
`2 - (dt*ω)^2`. Its eigenvalues lie on the unit circle only for
`0 < dt*ω < 2`. At and beyond the boundary, the method is not unconditionally
stable; for `dt*ω > 2`, one eigenvalue has magnitude greater than 1.

Symplecticity preserves geometric structure within its regime, but it does not
guarantee arbitrary-step stability, exact trajectories, or low phase error.

## Refinement test

The current test executes dt values:

- 0.1 s
- 0.05 s
- 0.025 s
- 0.0125 s

Each divides the 1-second final time exactly. The production force and world-step
path is used. Position-error ratios are expected to be approximately 2 between
successive refinements, demonstrating first-order global convergence for this
specified initial phase and observable. Velocity error is reported separately
and must decrease monotonically; its observed order is not generalized beyond
this fixture.

The test's initial tolerance band is 1.8–2.2 for position refinement ratios.
That is a regression threshold for this frozen system, not a universal bound
for every force law or initial condition.

## What this establishes — and what it does not

It establishes:
- the production world force/integration path follows the expected
  semi-implicit Euler update for this smooth single-body case;
- error decreases under timestep refinement;
- the documented conditional stability claim agrees with the known linear
  oscillator analysis.

It does not establish:
- correct general nonlinear dynamics;
- contact, friction, restitution or joint accuracy;
- rotational rigid-body correctness;
- long-time energy conservation for arbitrary Hamiltonian systems;
- experimental physical validation;
- superiority to an external solver.

## Next analytic families

1. Free fall and constant acceleration, checking exact position/velocity.
2. Kepler two-body orbit, checking orbital phase, energy and angular momentum
   drift at fixed timestep.
3. Pendulum, checking period/phase convergence and energy envelope.
4. Damped oscillator, checking exponential envelope and phase.
5. Stiff oscillator, comparing explicit and implicit solver work-precision
   curves against SUNDIALS on matched equations.

For cross-engine comparisons, bind the exact model, units, integrator, step size,
tolerance, initial state and observable. SUNDIALS is an ODE/DAE reference for
matched smooth systems; MuJoCo/Rapier/Chrono serve other model classes where
semantics align.
