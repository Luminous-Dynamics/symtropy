# Symtropy Shallow Water Reference Kernel

A small, deterministic first-order finite-volume solver for the 1D inviscid Saint-Venant equations over a fixed piecewise-constant bed.

## Implemented

- Conservative state variables: water depth h and unit-width discharge q = h*u.
- Local Lax-Friedrichs (Rusanov) interface flux.
- Hydrostatic reconstruction with side-specific pressure corrections for a well-balanced still-water state over non-flat bed.
- Reflective and transmissive ghost-cell boundaries.
- SSP-RK2 time stepping with a declared CFL upper bound and fail-closed state checks.
- Water storage per unit channel width.
- Exact idealized Ritter dam-break reference profile.

## Current numerical claim ceiling

The code includes synthetic numerical tests for still water, dry state, storage conservation with reflective boundaries and dam-break depth RMSE against the idealized Ritter profile. Passing these tests qualifies only the listed formulation and settings.

It is not a 2D solver and does not model bed friction, rainfall/infiltration, sediment, structures, moving beds, turbulent dynamics or real terrain. The Ritter solution assumes an inviscid semi-infinite reservoir discharging onto a flat dry bed; it is not a real-event dataset. No historical flood predictive skill, operational warning ability or superiority to a specialist model is claimed.

## Test

Run the focused gates:

    cargo test --locked -p symtropy-shallow-water
    cargo clippy --locked -p symtropy-shallow-water --all-targets -- -D warnings

The next qualification layer is convergence across grid refinements and comparison against curated SWASHES / published hydraulic benchmarks with license and observation support checked. The 2D path stays separate in https://github.com/Luminous-Dynamics/symtropy/issues/483.
