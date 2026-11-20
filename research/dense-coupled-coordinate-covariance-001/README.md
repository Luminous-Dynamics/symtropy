# Dense coupled-chain coordinate covariance preflight v0.1

This is the first algebraic input contract for [PHYS-DENSE-COUPLED-COVARIANCE-001 #5805](https://github.com/Luminous-Dynamics/symtropy/issues/5805).

## Composition

The fixture reuses the existing spatial three-body / two-link work:
- [coupled pHDAE chain #3144](https://github.com/Luminous-Dynamics/symtropy/pull/3144);
- [geometric rank preflight #3157](https://github.com/Luminous-Dynamics/symtropy/pull/3157);
- [independent Cartesian SHAKE/RATTLE chain #3160](https://github.com/Luminous-Dynamics/symtropy/pull/3160).

It does not introduce a second mechanics solver.

## Frozen transformations

Three dense, non-orthogonal integer 9x9 basis transformations are now frozen with exact inverse matrices and determinant +1; the third is a separate one-step conditioning stress case.

| Basis | Reference condition number | Transformed mass-metric condition |
|---|---:|---:|
| dense_shear_1 | 185.995 | 33,712.94 |
| dense_shear_2 | 737.999 | 520,513.26 |
| dense_shear_3 | 1,658.0 | 2,611,081.18 |

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

It compares lifted positions/velocities, both multiplier vectors, squared-link residuals and endpoint tangency for the two trajectory bases. A separate third-basis one-step stress uses condition number 1,658 and transformed mass-metric condition about 2.61 million, with looser state tolerances but unchanged link-residual bound. The nonzero force covector in v0.1 remains a separate covector/work-duality control; the current Cartesian RATTLE baseline has no force port, so the v0.2 step case explicitly freezes zero applied force. Forced RATTLE and the pHDAE comparison remain future increments.

The new machine-readable step vector is `fixtures/dense_coupled_chain_rattle_step_case_v0.2.json`, with its own Draft 2020-12 schema. It pins the solver profile and per-observable tolerances.

**Qualification status (exact-head):**

- At code head `f45a5e3`, pinned Rust 1.96 passed formatting, 27 warning-denied tests and optimized deterministic replay for the original two-basis one-step slice.
- At code head `c5ce8ba393e81e54d4d68125e06583ed75f2fa50`, pinned Rust 1.96 passed formatting, **30 warning-denied tests**, optimized deterministic RATTLE replay, and the rank-diagnostics workflow's **15 tests plus optimized replay**. This included the third-basis conditioning stress, 32-step recursive trajectory, 64-step dense-vs-Cartesian trajectory and timestep-refinement ladder.
- A further regression now adds position- and velocity-multiplier comparisons at every step of the 64-step trajectory (commit `e962fd5`). That incremental strengthening needs its own exact-head run; no pass is claimed until it completes.

The qualified scope at `c5ce8ba` is still force-free, three-body/two-link open-chain mechanics over a 0.32 s trajectory and the declared basis ladder. The v0.4 constant-force extension is newly authored and awaits exact-head qualification. Neither result is a general long-horizon stability theorem. Neither qualifies pHDAE equivalence, contact/friction, arbitrary charts, physical calibration or production performance.

## Competitive benchmark ladder

1. Exact one-step covariance against the independent Cartesian SHAKE/RATTLE solver on fixed, moderate and severe dense-coordinate conditioning.
2. Timestep refinement and long-horizon invariant drift, while reporting conditioning and solver failures separately.
3. Coupled pHDAE agreement on the same frozen physical case, with independent solver formulations.
4. Only then compare matched models against MuJoCo, Chrono, Pinocchio and Ceres; align integrator, constraints, mass matrix, contact assumptions, frame conventions and tolerances before comparing performance.

The useful differentiator is not raw throughput. It is the fraction of admissible re-representations for which the complete physical step, multipliers and constraints remain independently qualified, with solver cost and certificate overhead reported.



## Bounded 64-step repeated-map qualification (v0.3)

The trajectory increment runs 64 consecutive force-free SHAKE/RATTLE steps in both dense generalized-coordinate bases over 0.32 s (64 × 0.005 s). The dense trajectory recursively advances from its own lifted state; the Cartesian oracle recursively advances from its own Cartesian state. They are compared after every step, rather than resetting the dense state to the reference state.

The test records the maximum over all steps of:
- lifted position and velocity errors;
- position and velocity multiplier differences;
- per-edge squared-length residual;
- endpoint tangency.

The frozen protocol is `fixtures/dense_coupled_chain_rattle_trajectory_case_v0.3.json` with its Draft 2020-12 schema. Thresholds are (2\times10^{-7}) for lifted state and multipliers, (2\times10^{-12}) for squared-link residual, and (2\times10^{-9}) for endpoint tangency.

**This is a bounded 64-step metamorphic regression, not a long-horizon stability claim.** It does not establish energy drift bounds over thousands of steps, forced RATTLE, pHDAE equivalence, contact/friction, or physical calibration. The 64-step trajectory and third-basis stress are newly authored and remain unqualified until the latest exact-head Rust 1.96 workflow passes.



## Constant-force dense-coordinate RATTLE step-map increment (v0.4)

The force-free reference is now extended with a matched constant-force SHAKE/RATTLE step. The independent Cartesian baseline exposes `CartesianRattleChain::step_with_force`; the dense solver advances entirely in generalized coordinates and uses the force covector law

`Q_y = S^T Q_q`.

For constant applied force, the reference scheme applies:
- a first half-force kick and the initial constraint impulse;
- the force-aware SHAKE free drift and coupled position correction;
- a second half-force kick followed by the endpoint RATTLE momentum projection.

The dense solver forms `Q_y`, solves `M_y a_y = Q_y` directly in the dense metric, and applies both force half-kicks in the dense momentum equations. It does not reconstruct Cartesian state to call the original solver.

The frozen load acts only on body 0, deliberately producing nonzero position and velocity multipliers. The test compares:
- lifted positions and velocities;
- both multiplier vectors;
- per-edge squared-length residual and endpoint tangency;
- the constant-force work and kinetic-energy work-balance residual;
- canonical and transformed force power.

The negative control treats force as a vector using `S^-1 Q_q`; its power pairing must differ from the physical covector pairing. The new fixture is `fixtures/dense_coupled_chain_forced_rattle_case_v0.4.json` with its Draft 2020-12 schema. It pins the force, solver profile, two dense transformations, expected Cartesian output, multiplier vectors and comparison tolerances.

**Qualification status:** the forced step and its golden regression are newly authored. The existing force-free and 64-step results remain qualified only at their earlier recorded code heads. The forced path is not qualified until the current exact-head Rust 1.96 workflow passes formatting, the warning-denied tests and optimized replay.

The force model here is constant over one timestep. Time-varying force, position-dependent force Jacobians, external potential-energy accounting, forced long-horizon trajectories and pHDAE force coupling are not yet covered.

## Algebraic preflight scope

The warning-denied Rust tests verify exact inverse identities, the transformed mass metric, state/force/Jacobian/momentum laws, power duality, the canonical momentum one-form, multiplier virtual-work duality, SO(3) metric-skew generators and deliberately incorrect transformation laws.

The v0.1 tests qualified algebraic transformation identities. The v0.2 direct dense-coordinate RATTLE implementation and its independent Cartesian comparison extend that scope, but remain unqualified until the exact-head Rust 1.96 workflow passes. A dense path that reconstructs Cartesian state and calls the original solver is not acceptable.

No physical calibration, production readiness, performance improvement or SOTA claim is made.


## Forced dense-coordinate RATTLE (v0.3, authored; exact-head test pending)

The Cartesian reference now exposes a constant-force Velocity-Verlet/RATTLE step. The dense-coordinate path accepts the original force covector (Q_q), transforms it by (Q_y=S^TQ_q), and solves the SHAKE drift, first and second force half-kicks, and endpoint momentum projection directly in the dense (M_y) metric.

The new v0.3 fixture freezes:
- one-step force case: (Q_q=[0.18,-0.07,0.11,-0.12,0.09,0.05,0.04,-0.15,0.08]), (dt=0.01);
- 24-step replay case: (Q_q=[0.08,-0.04,0.06,-0.05,0.03,0.02,0.02,-0.07,0.04]), (dt=0.005), across both dense basis changes;
- lifted position/velocity, both multiplier vectors, link residuals, external work and work-energy residual tolerances.

The new warning-denied tests compare forced one-step and multi-step results against `CartesianRattleChain::step_with_force`. This tests the dual force-covector law and the actual forced discrete map—not only tensor transformations. A small independent numerical transcription of the frozen equations gave one-step lifted-state differences below (5\times10^{-13}) for the two dense bases; these are preflight calculations, not Rust or physical test results.

**Qualification status:** v0.1 algebraic checks and the force-free v0.2 one-step RATTLE comparison have passed the exact Rust 1.96 workflow at code head `f45a5e3` (27 tests, 0 failures, plus optimized replay). The forced v0.3 tests are newly authored and remain pending their own exact-head format/test/Clippy/replay run.

