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

## Dense-coordinate RATTLE step-map increment (v0.2)

The implementation now includes a separate force-free SHAKE/RATTLE step executed directly in the dense generalized coordinates. It forms the transformed mass metric and relative-constraint maps, solves the coupled position multipliers through the dense metric, then performs the endpoint momentum projection with the dense metric again. It never calls the Cartesian step function from the transformed solver.

The independent comparison test uses the same masses, initial physical positions/velocities, two link lengths and timestep in:
- the existing Cartesian `CartesianRattleChain::step`;
- the new dense-coordinate implementation using (M_y=S^T M_qS) and (J_y(y)=J_q(Sy)S).

It compares lifted positions/velocities, both multiplier vectors, squared-link residuals and endpoint tangency for both dense bases. The nonzero force covector in v0.1 remains a separate covector/work-duality control; the current Cartesian RATTLE baseline has no force port, so the v0.2 step case explicitly freezes zero applied force. Forced RATTLE and the pHDAE comparison remain future increments.

The new machine-readable step vector is `fixtures/dense_coupled_chain_rattle_step_case_v0.2.json`, with its own Draft 2020-12 schema. It pins the solver profile and per-observable tolerances.

**Qualification status:** the exact Rust 1.96 run on code head `f45a5e3` passed formatting, all 27 warning-denied tests, and the optimized deterministic replay. The dense-step comparison is therefore qualified for this frozen force-free open-chain fixture and the two tested basis changes. The later branch commits add only the v0.2 fixture/schema and README (no solver-code changes). This does not qualify forced RATTLE, pHDAE equivalence, contact/friction or long-horizon drift.

## Competitive benchmark ladder

1. Exact one-step covariance against the independent Cartesian SHAKE/RATTLE solver on fixed, moderate and severe dense-coordinate conditioning.
2. Timestep refinement and long-horizon invariant drift, while reporting conditioning and solver failures separately.
3. Coupled pHDAE agreement on the same frozen physical case, with independent solver formulations.
4. Only then compare matched models against MuJoCo, Chrono, Pinocchio and Ceres; align integrator, constraints, mass matrix, contact assumptions, frame conventions and tolerances before comparing performance.

The useful differentiator is not raw throughput. It is the fraction of admissible re-representations for which the complete physical step, multipliers and constraints remain independently qualified, with solver cost and certificate overhead reported.



## Bounded 32-step repeated-map qualification (v0.3)

The next increment runs 32 consecutive force-free SHAKE/RATTLE steps in both dense generalized-coordinate bases. The dense trajectory recursively advances from its own lifted state; the Cartesian oracle recursively advances from its own Cartesian state. They are compared after every step, rather than resetting the dense state to the reference state.

The test records the maximum over all steps of:
- lifted position and velocity errors;
- position and velocity multiplier differences;
- per-edge squared-length residual;
- endpoint tangency.

The frozen protocol is `fixtures/dense_coupled_chain_rattle_trajectory_case_v0.3.json` with its Draft 2020-12 schema. Thresholds are (2\times10^{-7}) for lifted state and multipliers, (2\times10^{-12}) for squared-link residual, and (2\times10^{-9}) for endpoint tangency.

**This is a bounded 32-step metamorphic regression, not a long-horizon stability claim.** It does not establish energy drift bounds over thousands of steps, forced RATTLE, pHDAE equivalence, contact/friction, or physical calibration. The exact-head Rust 1.96 workflow must pass before the 32-step extension is qualified.

## Algebraic preflight scope

The warning-denied Rust tests verify exact inverse identities, the transformed mass metric, state/force/Jacobian/momentum laws, power duality, the canonical momentum one-form, multiplier virtual-work duality, SO(3) metric-skew generators and deliberately incorrect transformation laws.

The v0.1 tests qualified algebraic transformation identities. The v0.2 direct dense-coordinate RATTLE implementation and its independent Cartesian comparison extend that scope, but remain unqualified until the exact-head Rust 1.96 workflow passes. A dense path that reconstructs Cartesian state and calls the original solver is not acceptable.

No physical calibration, production readiness, performance improvement or SOTA claim is made.
