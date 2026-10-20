# Symtropy Shallow Water Reference Kernel

A small, deterministic first-order finite-volume solver for the 1D inviscid Saint-Venant equations over a fixed piecewise-constant bed.

## Implemented

- Conservative state variables: water depth h and unit-width discharge q = h*u.
- Local Lax-Friedrichs (Rusanov) interface flux.
- Hydrostatic reconstruction with side-specific pressure corrections for a well-balanced still-water state over non-flat bed.
- Reflective and transmissive ghost-cell boundaries.
- SSP-RK2 time stepping with a declared CFL upper bound and fail-closed state checks.
- Water storage per unit channel width.
- Exact idealized Ritter dry-bed dam-break rarefaction profile.
- Exact wet-bed Stoker dam-break Riemann profile, including its left rarefaction, constant star state and right-moving shock.
- Exact symmetric double-rarefaction profile, including either a lower-depth intermediate state or a dry/vacuum gap when the outward velocity reaches the analytical threshold.

## Current numerical claim ceiling

The code includes synthetic numerical tests for still water, dry state, storage conservation with reflective boundaries, analytical landmarks, and depth RMSE across three grid resolutions for the idealized Ritter dry-bed rarefaction, wet-bed Stoker shock/rarefaction, and symmetric double-rarefaction solution, including an explicit dry-gap negative/edge test. Passing these tests qualifies only the listed formulation and settings.

It is not a 2D solver and does not model bed friction, rainfall/infiltration, sediment, structures, moving beds, turbulent dynamics or real terrain. The Ritter solution assumes an inviscid semi-infinite reservoir discharging onto a flat dry bed; it is not a real-event dataset. No historical flood predictive skill, operational warning ability or superiority to a specialist model is claimed.

## Test

Run the focused gates:

    cargo test --locked -p symtropy-shallow-water
    cargo clippy --locked -p symtropy-shallow-water --all-targets -- -D warnings

The next qualification layer is to expand the analytical suite against independently curated SWASHES / published hydraulic benchmarks, with case assumptions, license, equations and output support explicitly recorded. The 2D path stays separate in https://github.com/Luminous-Dynamics/symtropy/issues/483.
