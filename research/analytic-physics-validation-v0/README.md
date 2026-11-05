# Analytic Physics Validation v0

This is the first mathematical-oracle tranche for Symtropy physics. It extends
the deterministic batch/replay foundation, but establishes a separate evidence
class: agreement with an analytic solution and the known discrete numerical map.

## Integrator claim

The current translational integrator is semi-implicit (symplectic) Euler:
velocity is updated using acceleration, then position is updated using the new
velocity. The method is symplectic under the appropriate separable-Hamiltonian
assumptions; it is not unconditionally stable for every Hamiltonian system.
Accuracy and stability depend on the problem and timestep.

## Frozen ballistic fixture

See `manifests/ballistic-constant-acceleration-v0.json`.

Initial state:

- Position: `[1.25, 3.0, -2.0] m`
- Velocity: `[2.5, 1.25, -0.5] m/s`
- Constant acceleration: `[0.0, -9.81, 0.0] m/s^2`
- No damping, no collisions, no force clamp, no speed clamp.
- Duration: exactly one second.

For N fixed steps of width h, the exact discrete map is:

```text
v_N = v_0 + N a h
x_N = x_0 + N v_0 h + a h^2 N(N+1)/2
```

The continuous ballistic reference at T = Nh is:

```text
v(T) = v_0 + a T
x(T) = x_0 + v_0 T + 0.5 a T^2
```

The semi-implicit Euler velocity is exact for this constant-acceleration case
(absent damping/clamps); position has first-order global error. At fixed T, the
position error magnitude is `|a| T h / 2`, so halving h should halve the error.
The unit test asserts both the exact discrete map and this convergence ratio.

## Evidence status

This is **code verification against a mathematical oracle**, not experimental
validation and not a comparison against MuJoCo/Rapier/PhysX. It checks only
translation under constant acceleration. It does not validate rotational
inertia, contact, friction, CCD, joints, or open-system energy accounting.

The next fixtures should add harmonic oscillators, Kepler orbits, isolated
collisions, external-wall impulse accounting, resting contact, frictional
sliding, CCD thin-obstacle cases, and constraint residuals. Asymmetric rotation
accuracy remains limited by the scalar-mean inertia approximation documented in
`integrator.rs`; it must not be promoted as physically validated until that
model is corrected.
