# Integrator Analytic Qualification V0

This tranche qualifies the production semi-implicit Euler update against its
exact discrete recurrence and an analytic smooth-system oracle.

## Corrected method claim

The integrator is velocity-first / position-second semi-implicit Euler. In the
undamped conservative separable Hamiltonian case, this update is symplectic.
Symplecticity does **not** imply unconditional stability.

For the linear harmonic oscillator with angular frequency omega, the linear
stability region of this update requires:

```
0 < omega * dt < 2
```

Outside that region the discrete map can become unstable even though the
method remains symplectic in its conservative form. Damping, contacts,
velocity clamps, external force splitting and finite precision also change the
map. The current rigid-body integrator's scalar-average angular inertia is
only an approximation for asymmetric bodies; the source retains an explicit
TODO for a full world-space inertia tensor.

## Exact free-fall recurrence

For constant acceleration g, timestep h, and N = T/h fixed steps:

```
v_(k+1) = v_k + g*h
x_(k+1) = x_k + h*v_(k+1)

v_N = v_0 + g*T
x_N = x_0 + v_0*T + g*h^2*N*(N+1)/2
    = x_0 + v_0*T + 0.5*g*T*(T+h)
```

The production test compares against this **discrete** result first. Comparing
only to the continuous analytic trajectory and applying a loose tolerance
would conflate integration error with implementation correctness.

Against the continuous solution,
`x(T)=x0+v0*T+0.5*g*T^2`, the method has a predictable first-order position
error of `0.5*g*T*h` for this fixture. Halving h should halve the global error;
the new test measures that ratio at 0.1, 0.05 and 0.025 seconds.

## Harmonic oscillator envelope

The undamped oscillator test uses a fixed stable step with `omega*dt < 2` and
tracks the mechanical-energy envelope over a long horizon. Semi-implicit Euler
does not conserve the original Hamiltonian exactly; bounded oscillatory energy
error is the appropriate expectation, not zero drift.

## Tests added

- Exact semi-implicit free-fall discrete recurrence for both velocity and
  position.
- First-order timestep-refinement of continuous free-fall position error.
- Bounded energy envelope for the linear harmonic oscillator inside the
  analytic stability region.

Existing integrator tests remain intact. No tolerance is weakened.

## Claim ceiling

These fixtures verify the translational update for the stated systems. They do
not qualify:
- asymmetric rigid-body rotational dynamics (the source explicitly averages
  inertia rather than using a world-space tensor);
- stiff/contact-constrained systems or nonsmooth impact;
- variable timesteps, force/drag operator splitting, or velocity-clamp
  saturation;
- solver convergence or whole-engine physical validation.

Those need separate scenario families and independent validation points.
