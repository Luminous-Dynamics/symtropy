# Analytic numerical qualification v0

This tranche connects replay determinism to numerical verification. It adds
independent-oracle tests for continuous collision detection (CCD), distance
constraints and semi-implicit Euler integration.

## Executable cases

- **CCD:** a 1,000 m/s sphere crossing a thin static obstacle entirely within
  one 20 ms step; 31 swept-sphere chord offsets; exact grazing contact; and 20
  sphere/half-space speed points. Every returned time of impact is checked
  against the geometry at that instant.
- **Distance constraints:** residual decay is checked against the analytic
  geometric sequence for fixed stiffness; unequal-mass correction must preserve
  the isolated pair's center of mass.
- **Integrator:** free-fall is checked against the exact discrete recurrence of
  semi-implicit Euler; halving the timestep must halve the global position
  error for the smooth constant-acceleration test.

The frozen inputs, independent-oracle descriptions and predeclared tolerances
are in `analytic-validation-v0.json`.

## Claim ceiling

These are verification points, not engine-wide validation:

- CCD query correctness does not prove the full world stepper always clamps
  motion at time of impact or prevents tunnelling for every collider pair.
- The distance constraint case does not qualify coupled contacts, motors,
  rotational degrees of freedom, multibody joints or island convergence.
- Constant-acceleration integration does not qualify stiff/chaotic systems,
  contacts, variable timesteps or asymmetric rotational inertia.
- No performance or external-engine superiority claim follows from these tests.

## External comparator strategy

Match equations, units, collision/event semantics, solver settings and quality
thresholds before comparing:

- **MuJoCo 3.17.0:** rigid-body contact/CCD and native batch simulation. Its
  batch API documents bit-identical output to a serial loop at any thread count.
- **Rapier 0.35.3:** tagged Rust-engine reference. Keep Dimforge's Q3 2026
  parallel-architecture report as a separately pinned development lane.
- **Project Chrono 10.0.0:** multibody/joint/vehicle cases where models match.
- **SUNDIALS 7.9.0:** smooth ODE/DAE equations only, not a contact solver.
- **ASME V&V 20:** separate verification from validation and account for both
  numerical and experimental uncertainty at each validation point.

## Metrics

Report per scenario rather than hiding weaknesses in an aggregate score:
TOI error and contact residual; miss/false-positive and tunnelling counts;
constraint residual by iteration, overshoot and center-of-mass drift; integrator
error norm, observed convergence order and invariant drift under explicitly
declared boundary assumptions. Runtime and energy remain separate, measured
dimensions with uncertainty.

A 10% performance target is meaningful only after physical/numerical quality
passes against an equivalent baseline.
