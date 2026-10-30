# One-dimensional finite-volume enthalpy reference

This standalone Rust utility is the first numerical candidate for the analytical one-phase Stefan reference in main.rs.

## Scope and formulation

- Cell-centred uniform 1D finite volumes.
- Volumetric enthalpy H is referenced to liquid at the melting temperature (H = 0).
- Fully solid at the melting temperature has H = -rho * L.
- The interval -rho*L <= H <= 0 represents an isothermal phase mixture at T_m, with solid fraction f_s = -H/(rho*L).
- Below -rho*L, the cell is fully solid and carries sensible cooling.
- Constant properties, no advection, no fluid flow, no mold contact, no volumetric heat source.
- Cold Dirichlet wall at the left face, insulated right boundary.
- Explicit finite-volume update, with dt <= CFL * dx^2 / alpha, and an admitted CFL <= 0.25.
- The front metric is the integral-equivalent solid length sum(f_s * dx), not an arbitrary temperature isosurface.

The solver reports global energy closure between cold-wall heat removed and the magnitude of integrated cell enthalpy change. It rejects runs where freezing reaches the far boundary.

## Build and run

Compile directly without adding the file to the Bevy workspace:

    rustc --edition=2024 -D warnings --test tools/casting-phase-reference/finite_volume.rs -o /tmp/stefan-fv-tests
    /tmp/stefan-fv-tests

    rustc --edition=2024 -D warnings tools/casting-phase-reference/finite_volume.rs -o /tmp/stefan-fv
    /tmp/stefan-fv --verify
    /tmp/stefan-fv --csv

## Benchmark design

The frozen 60-second reference uses the analytical front value 0.0023735493646876173 m, with 32, 64, 128 and 256 cells over a 0.01 m domain. The verifier requires decreasing front error under the 32/64/128/256 spatial ladder, energy residual below 1e-11 relative, and finest-grid relative front error below 5e-5. A separate 128-cell temporal ladder halves CFL from 0.2 to 0.025 and checks first-order contraction of successive front changes; this separates time-discretization behavior from the combined mesh/time refinement.

This first candidate is deliberately explicit and low-order. It is a transparent baseline, not a claim of state-of-the-art performance. Compare against implicit enthalpy methods and the 2025 flag-update enthalpy formulation at matched front/field error and complete compute cost.

## References

- Semi-analytical arbitrary-dimensional Stefan benchmark, DOI 10.1108/HFF-09-2021-0647.
- Peters et al. (2025), “A new implicit formulation of the enthalpy method using flag updates,” International Journal of Heat and Mass Transfer, 249, 127166, DOI 10.1016/j.ijheatmasstransfer.2025.127166.
- White (1982), “An Enthalpy Formulation of the Stefan Problem,” SIAM Journal on Numerical Analysis, DOI 10.1137/0719082.

## Claim ceiling

The numerical result is a 1D conduction-only algorithm-verification result against an idealized one-phase reference. It does not validate foundry casting, alloys, mold heat transfer, fluid filling, porosity, shrinkage, microstructure, defects or production improvements.
