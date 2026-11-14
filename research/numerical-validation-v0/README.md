# Numerical Verification v0

This lane separates a method's structural properties, analytic accuracy,
stability and physical validation.

## Semi-implicit Euler ballistic fixture

The production integrator uses kick-drift semi-implicit Euler for linear
translation:

```text
v[n+1] = v[n] + a * dt
x[n+1] = x[n] + v[n+1] * dt
```

For constant acceleration and fixed final time (T=N,dt), the velocity is
exact up to floating-point accumulation:

```text
v(T) = v0 + a*T
```

The position is:

```text
x_num(T) = x0 + v0*T + a*T^2/2 + a*T*dt/2
```

Therefore this fixture has a global position error of (O(dt)). Halving the
timestep halves the position error. This result is specific to the constant
acceleration fixture and the stated update order; it is not a proof of method
stability for arbitrary nonlinear or stiff systems.

The test uses:
- (T=1) second;
- (dt = 0.1, 0.05, 0.025) seconds;
- fixed initial position, velocity and gravity;
- zero linear/angular damping;
- exact ballistic reference position and velocity.

The expected position-error ratios are approximately 2.0 for both refinements.
Velocity error should remain near floating-point roundoff.

## Claim separation

- **Structure preservation:** symplectic Euler is symplectic for suitable
  separable Hamiltonian systems; this is not a universal stability guarantee.
- **Numerical verification:** compare the discrete update to an analytic
  solution and quantify error versus timestep.
- **Stability:** requires separate tests for the equation family, stiffness,
  timestep range and perturbation behavior.
- **Physical validation:** requires measured validation points and uncertainty
  accounting, not merely an analytic unit test.
- **Performance:** requires controlled runtime/energy campaigns at matched
  numerical error.

Do not claim second-order position convergence for this integrator or
"unconditionally stable" behavior. Damping and safety clamps alter the
conservative Hamiltonian update and should be treated as separate model terms.
