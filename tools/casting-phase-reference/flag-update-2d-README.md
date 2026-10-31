# Complete 2D implicit flag-update enthalpy reference

This standalone solver composes the matrix-free 2D active-phase operator and Jacobi-PCG kernel from PR #2986 with a cell-centred finite-volume phase-flag iteration.

## Physical scope

- Equal solid/liquid density, heat capacity and conductivity.
- Isothermal latent heat and sharp solid/liquid interface.
- Fixed cold west wall; other exterior faces insulated.
- Initially superheated liquid.
- Conduction only: no flow, shrinkage, mold thermal resistance, porosity or microstructure.

It is a genuine 2D grid, but the first acceptance fixture is a **planar extrusion**: all rows have identical initial/boundary conditions, so the expected solution must remain invariant in y. That is an integration check of 2D indexing and boundary handling, not curved-front validation.

## Discrete step

For each fixed flag map, active solid/liquid nodes use the shared matrix-free SPD operator. Mixed nodes set Kirchhoff potential v=0. The active RHS includes the cold-wall half-cell Dirichlet contribution. Mixed latent enthalpy is recovered from its finite-volume energy row. Flags iterate using the solid/mixed/liquid consistency conditions. Cycles, iteration exhaustion, non-finite state, block residual or energy residual failure abort the step.

Per-step energy closure compares:
1. change in cell-integrated enthalpy;
2. independently accumulated west-wall flux from the half-cell gradient.

## Validation

- 2D matrix-free operator and PCG reuse the implementation from PR #2986.
- Every time step checks the finite-volume row residual.
- Every step closes wall heat against integrated enthalpy change.
- Planar extrusion checks row-to-row field and flag invariance.
- Exact equal-property two-phase Stefan temperatures/front are sampled at cell centres and compared to the computed fields.
- Report temperature MAE/RMSE, mean/final front error, block residual, wall heat, enthalpy drop, flag iterations, PCG iterations, operator applications and failed-run status.

## Build and run

    rustc --edition=2024 -D warnings --test tools/casting-phase-reference/flag_update_2d.rs -o /tmp/flag-update-2d-tests
    /tmp/flag-update-2d-tests

    rustc --edition=2024 -D warnings tools/casting-phase-reference/flag_update_2d.rs -o /tmp/flag-update-2d
    /tmp/flag-update-2d --verify
    /tmp/flag-update-2d --csv

## Claim ceiling

This is a conduction-only 2D phase-change numerical experiment. The planar extrusion does not validate curved fronts or full 3D. It does not validate alloy property variation, mushy zones, melt flow, mold filling, shrinkage, porosity, microstructure, defect predictions, production quality or foundry release.
