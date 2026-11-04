# SO(4) production inertia adapter: sign and frame contract

**Status:** production-adapter regression tests; stacked on the mass-property implementation branch.  
**Date:** 2026-10-31  
**Related:** [#3012 asymmetric inertia and mass consistency](https://github.com/Luminous-Dynamics/symtropy/pull/3012), [#3126 3D Moser-Veselov reference](https://github.com/Luminous-Dynamics/symtropy/pull/3126), [#3142 generalized SO(4) reference](https://github.com/Luminous-Dynamics/symtropy/pull/3142).

## Critical convention mapping

The independent Moser-Veselov references use physical angular velocity W and physical angular momentum M:

- physical point velocity is W*r;
- physical inertia map is M = S W + W S.

Production Symtropy stores a bivector B whose rotor update is exp(-B). Therefore:

- physical angular velocity W = -B;
- production stored angular momentum L_store = B S + S B;
- physical angular momentum M = -L_store.

Both sides change sign. Consequently:

\[
M = -L_{\mathrm{store}}
  = S(-B)+(-B)S
  = S W + W S.
\]

This mapping is essential. Comparing the reference physical momentum directly to the production stored momentum would produce a false sign failure; comparing stored angular velocity directly to physical W would produce the same error in attitude increments.

## Executable adapter tests

The integration test uses a full, non-diagonal, positive-definite 4×4 second-moment matrix and all six SO(4) bivector coordinates. It verifies:

1. Production angular momentum converts to the reference physical momentum with the declared sign.
2. Production inverse inertia recovers the stored bivector, and therefore the negative physical angular velocity.
3. Production kinetic energy equals the physical reference pairing.
4. Production gyroscopic angular acceleration agrees with the physical Euler-Arnold equation after the sign conversion.
5. The inertia map, energy and inverse are covariant under a generic body-basis rotation in SO(4).
6. Rotor::from_bivector uses exp(-B), consistent with physical exp(+W).

These tests validate the algebraic adapter and gyroscopic right-hand side. They do **not** yet demonstrate that a full production PhysicsWorld trajectory matches the Moser-Veselov discrete trajectory. That is a separate integration campaign.

## Promotion

Run the exact-head Rust test with the full symtropy-physics feature/build contract and standard workspace CI. Then connect a frozen, torque-free SO(4) trajectory to production integration and compare attitude, bivector momentum, kinetic energy and both SO(4) Casimirs against the independent reference.
