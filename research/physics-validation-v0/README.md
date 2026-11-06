# Physics Analytic Verification v0

This tranche strengthens mathematical correctness of two narrow numerical
components: continuous collision detection (CCD) for sphere primitives and the
semi-implicit Euler translational update. It does not validate the complete
rigid-body engine or establish agreement with experiments.

## Why this exists

The audit found three claim/implementation mismatches:

1. `CcdHit::toi` was documented as a normalized fraction in [0, 1], but both
   CCD functions return physical time in seconds in [0, dt]. This matters when a
   timestep exceeds one second and when composing multi-rate substeps.
2. Sphere-halfspace CCD compared signed distance directly with a radius, which
   assumes a unit plane normal. A geometrically identical plane represented by
   a scaled normal could produce the wrong contact time and point.
3. Sphere-sphere CCD used the direct quadratic discriminant and root formula.
   For far-apart, high-speed pairs, `b*b - 4*a*c` and direct subtraction can
   lose the small impact interval through floating-point cancellation.

The integrator documentation also called semi-implicit Euler "unconditionally
stable for Hamiltonian systems." Symplecticity is not unconditional stability:
for the linear harmonic oscillator, the semi-implicit Euler map is linearly
stable only when 0 < omega * dt < 2. The method can still have phase error,
and an overly large timestep can diverge.

## Implemented analytic checks

### Sphere-sphere CCD

The implementation projects relative motion to the closest-approach time and
computes the half-width of the intersection interval. This avoids the
cancellation-prone quadratic discriminant/root subtraction for the tested
far-distance/high-speed case. Inputs with non-finite positions/velocities,
negative or non-finite radii, or invalid timestep return no hit through the
existing Option-based API.

### Sphere-halfspace CCD

The normal and plane offset are normalized together. The plane is represented
by n·x = offset; scaling both by the same positive constant leaves the hit
time, point and unit normal unchanged. Zero/non-finite normals and invalid
inputs fail closed.

### Time-of-impact semantics

`toi` is measured in seconds from the beginning of the query, in [0, dt].
It is not a normalized [0, 1] fraction. An explicit test uses a 10-second
sweep whose impact occurs at 9.5 seconds.

### Semi-implicit Euler

An analytic constant-gravity free-fall fixture checks the exact velocity and
the expected first-order position error. Halving the timestep halves the
position error. A long harmonic oscillator run checks bounded energy behavior
inside the linear stability region; it does not claim exact energy conservation
or zero phase error.

## External comparison and validation framing

- MuJoCo 3.17.0's native batch API is a deterministic multi-simulation
  comparator, not a substitute for analytic validation of these equations.
- Rapier 0.35.3 and Chrono 10.0.0 are relevant rigid-body reference engines
  only after matching contact, solver, material, timestep and integration
  semantics.
- SUNDIALS 7.9.0 is a reference for matched ODE/DAE numerical problems, not
  rigid-body contact equivalence.
- ASME V&V 20 recommends pointwise comparison of specified validation
  variables and explicit treatment of solution and experimental-data
  uncertainty. This tranche is analytical verification, not experimental
  validation.

## Next benchmark gates

1. **CCD sweep corpus:** head-on, grazing, near-tangent, initially touching,
   initially overlapping, moving apart, extreme distance/speed, scaled plane
   normal/offset, and invalid input cases. Record time-of-impact error in
   seconds, contact-point error, unit-normal error and false-positive/negative
   counts against high-precision or analytic roots.
2. **Integrator convergence:** free fall, constant force, harmonic oscillator,
   and coupled central-force orbit. Measure global position/velocity error,
   phase error, energy envelope, momentum and order of convergence over at
   least four timestep refinements.
3. **Contact validation:** elastic/inelastic two-body impact, static-plane
   impact, stacked bodies, mixed mass ratios and frictional incline. Account
   for momentum/energy crossing the selected boundary; a static wall is an
   external reservoir for dynamic-body momentum.
4. **Solver comparison:** match model, initial states, collision geometry,
   material coefficients, integrator, solver iterations and contact conventions
   across Symtropy, MuJoCo, Rapier and Chrono. Compare work-precision curves,
   not just runtime.
5. **Reproducibility:** bind exact source/config/scenario SHA, compiler, target,
   deterministic profile, and raw outputs into the result receipt.

## Claim ceiling

The added tests establish selected analytic subproblems and numerical behavior.
They do not show that every CCD collision shape is correct, that angular
inertia is physically exact for asymmetric bodies, that the contact solver
conserves energy, or that Symtropy beats SOTA. Those need separate fixtures,
matched baselines and uncertainty-qualified evidence.
