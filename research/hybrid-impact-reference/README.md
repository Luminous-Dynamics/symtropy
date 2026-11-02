# Hybrid impact reference (V0)

This isolated Rust crate implements an analytical reference for the first impact
of a one-dimensional point mass falling toward a fixed plane under constant
gravity with Newton restitution.

It supports the Symtropy research issue
[PHYS-HYBRID-STRUCTURE-001](https://github.com/Luminous-Dynamics/symtropy/issues/3393).

## Model

With the plane at y = 0, positive y directed away from the plane, initial
height h > 0, initial velocity v0, and gravity magnitude g > 0,

    y(t) = h + v0*t - 0.5*g*t^2
    v(t) = v0 - g*t

Let s = sqrt(v0^2 + 2*g*h). The first positive impact time is
t* = (v0 + s)/g. A cancellation-resistant equivalent is used when v0 < 0.
The incoming velocity is v_minus = -s; Newton restitution gives
v_plus = -e*v_minus, with e in [0, 1].

The impact dissipation is D = 0.5*m*(1-e^2)*v_minus^2.

The ledger reports initial mechanical energy, pre/post-impact kinetic energies,
physical impact dissipation, restitution residual, and energy-balance residual.
Physical dissipation is kept distinct from numerical residual.

## Scope and evidence boundary

- Analytical, one-dimensional, first-impact reference only.
- Constant gravity, fixed plane, point mass, Newton restitution.
- No time-stepping integration, friction, coupled bodies, repeated impacts,
  event-capturing method, or physical experiment.
- This crate is not a production Symtropy solver and does not validate the
  general engine.
- Passing these tests would qualify only the analytic reference implementation
  against its stated equations and hand-checkable fixtures.

## Run

    cargo fmt --check
    cargo test --locked
    cargo clippy --locked --all-targets -- -D warnings


## Repeated impacts and Zeno classification

The optional repeated-bounce reference classifies the ideal one-dimensional
sequence without numerically stepping increasingly short flight intervals.

Let s0 be the incoming speed at the first impact and e the constant Newton
restitution coefficient. For impact k >= 1:

    incoming_speed(k) = s0 * e^(k - 1)
    outgoing_speed(k) = s0 * e^k

The time of impact k is

    t_k = t_1 + (2*s0/g) * sum(j=1..k-1, e^j)

The infinite-sequence regimes are distinct:

- e = 0: the first impact settles the body at the plane; there is no second
  distinct impact.
- 0 < e < 1: infinitely many impacts have finite accumulation time
  t_Z = t_1 + (2*s0/g) * e/(1-e).
- e = 1: impacts continue with constant period 2*s0/g and no finite-time
  accumulation.

For impact k, cumulative impact dissipation is
0.5*m*s0^2*(1 - e^(2*k)); the remaining post-impact kinetic energy is
0.5*m*s0^2*e^(2*k). Stable expm1-based forms are used near e = 1. If distinct
impact times become unrepresentable in f64, the state query returns the typed
TimeResolutionExhausted result rather than report coincident events as distinct.

This analytic Zeno classification does not choose a physically appropriate
post-Zeno continuation. Real systems may require compliance, deformation,
surface roughness, velocity-dependent restitution or another richer model;
that continuation is a separate modeling decision.
