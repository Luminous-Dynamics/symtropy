# Production SO(4) torque-free trajectory qualification

**Status:** integration regression and convergence campaign; not a variational integrator or SOTA claim.  
**Date:** 2026-10-31  
**Build basis:** [#3112 explicit integration policy](https://github.com/Luminous-Dynamics/symtropy/pull/3112), [#3012 asymmetric inertia](https://github.com/Luminous-Dynamics/symtropy/pull/3012), independent SO(4) reference [#3142](https://github.com/Luminous-Dynamics/symtropy/pull/3142), adapter contract [#3143](https://github.com/Luminous-Dynamics/symtropy/pull/3143).

## Why this campaign exists

The SO(4) variational reference and algebraic production adapter are not sufficient by themselves. The production integrator is semi-implicit Euler with an explicit unbounded scientific integration policy; the Moser-Veselov reference is a different discrete map. Finite-step states should not be required to match exactly.

The correct test is that production trajectories converge toward the same continuous torque-free mechanics as the timestep is refined, while reporting the physical invariants separately. The current scheme is expected to show first-order global invariant-defect contraction, whereas the Moser-Veselov reference has its own variational invariants.

## Frozen physical fixture

- Dimension: D=4.
- Mass: 1 kg, with mass-aware inertia metadata.
- Full symmetric positive-definite, non-diagonal second-moment matrix.
- All six stored bivector coordinates initially nonzero.
- Zero force, zero torque, zero linear damping, zero angular damping, zero gravity.
- Explicit unbounded integration limits so the reference run has no hidden speed caps.
- Horizon 0.5 s; dt = 0.002, 0.001, 0.0005 s, all dividing the horizon.

Stored B is the negative of physical angular velocity. Production stored momentum L=BS+SB is the negative of physical momentum. The test uses M=-L for physical momentum-map comparison.

## Observables and expected order

Measure independently:
- kinetic energy;
- bivector-norm Casimir;
- Pfaffian Casimir;
- spatial angular-momentum bivector;
- SO(4) orthogonality and determinant;
- world-frame rotational covariance;
- invalid-state rejection and transactional non-mutation.

Because this production path is semi-implicit Euler, the global invariant defects should contract approximately linearly with dt. The test checks ratios near 2 when dt is halved. It does not compare the production state bit-for-bit with the Moser-Veselov state at finite dt.

## Limits

This is one full-rank smooth free-top fixture. It does not test degenerate/partial-rank distributions, contact, friction, joints, external torques, or GPU/parallel backends. It does not establish that semi-implicit Euler is the best integrator; it supplies an executable baseline that can be compared fairly with a variational solver on matched horizon and physical data.

Promotion requires exact-head cargo format/build/tests, the standard workspace matrix, and registration in the engine-wide corpus #460. No local Rust pass is claimed by creating this test.
