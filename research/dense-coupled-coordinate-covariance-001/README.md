# Dense coupled-chain coordinate covariance preflight v0.1

This is the first algebraic input contract for [PHYS-DENSE-COUPLED-COVARIANCE-001 #5805](https://github.com/Luminous-Dynamics/symtropy/issues/5805).

## Composition

The fixture reuses the existing spatial three-body / two-link work:
- [coupled pHDAE chain #3144](https://github.com/Luminous-Dynamics/symtropy/pull/3144);
- [geometric rank preflight #3157](https://github.com/Luminous-Dynamics/symtropy/pull/3157);
- [independent Cartesian SHAKE/RATTLE chain #3160](https://github.com/Luminous-Dynamics/symtropy/pull/3160).

It does not introduce a second mechanics solver.

## Frozen transformations

Two dense, non-orthogonal integer 9x9 basis transformations are frozen with exact inverse matrices and determinant +1.

| Basis | Reference condition number | Transformed mass-metric condition |
|---|---:|---:|
| dense_shear_1 | 185.995 | 33,712.94 |
| dense_shear_2 | 737.999 | 520,513.26 |

These are independent NumPy/SVD reference values, not Rust solver measurements.

## Contract

For q = S y, the fixture binds:
- mass-metric congruence M_y = S^T M_q S;
- position and velocity transformation by S^-1;
- force and momentum covector transformation by S^T;
- complete constraint Jacobian transformation J_y = J_q S;
- physical generator conjugation A_y = S^-1 A_q S;
- unchanged constraint multipliers when the original constraint functions are composed with S without rescaling.

The expected transformed state, force, constraint Jacobian, momentum covector, condition values and SO(3) metric-skew defects are recorded in the JSON fixture.

## Algebraic preflight scope

The warning-denied Rust tests verify exact inverse identities, the transformed mass metric, state/force/Jacobian/momentum laws, power duality, the canonical momentum one-form, multiplier virtual-work duality, SO(3) metric-skew generators and deliberately incorrect transformation laws.

**This is not a dense-coordinate pHDAE or RATTLE step-map qualification.** The next step must run the actual coupled discrete equations directly in the dense basis, lift the result, and compare the physical step, multipliers and per-edge residuals with both existing independent solver references. A dense path that reconstructs Cartesian state and calls the original solver is not acceptable.

No physical calibration, production readiness, performance improvement or SOTA claim is made.
