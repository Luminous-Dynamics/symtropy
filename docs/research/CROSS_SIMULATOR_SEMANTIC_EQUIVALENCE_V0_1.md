# Cross-Simulator Semantic Equivalence Contract V0.1

**Status:** proposed benchmark admission contract  
**Tracking issue:** [PHYS-SEMANTIC-EQUIVALENCE-001 / #4366](https://github.com/Luminous-Dynamics/symtropy/issues/4366)  
**Scope:** pairwise simulator-to-simulator mapping for a frozen scenario. Not a physics solver or a new benchmark corpus.

## 1. The rule

A simulator comparison is valid only when the compared runs represent the same declared physical problem, or when every difference is explicitly mapped with a bounded and approved approximation.

```text
same asset file       != same physical model
same timestep         != same integration semantics
same contact geometry != same contact law
same observed output  != same physical truth
```

The contract separates four sources of discrepancy:

1. **Semantic mismatch:** the two configurations solve different declared problems.
2. **Numerical divergence:** they implement the same declared problem but produce different numerical results.
3. **Model discrepancy:** the mathematical model omits or simplifies physical behaviour.
4. **Measurement discrepancy:** physical observations differ from predictions, with instrumentation and sampling uncertainty.

Do not attribute a semantic mismatch to solver accuracy.

## 2. Mapping disposition

Every backend-pair/scenario receipt must assign one disposition per decision-critical semantic dimension:

| Status | Admission |
|---|---|
| `StrictlyEquivalent` | May enter matched differential comparison |
| `MappedEquivalentWithBound` | May enter only with approved transformation and propagated error budget |
| `NonEquivalent` | Exploratory side-by-side only; excluded from matched score |
| `Unsupported` | No comparison for this dimension |
| `UnknownUnassessed` | Fail closed; not admitted |

No aggregate status may hide an unknown or unsupported decision-critical field.

## 3. Minimum semantic dimensions

| Dimension | Required declaration |
|---|---|
| Identity and geometry | Model/configuration digest, topology, shape identity, scale, units, transforms, coordinate frames |
| Mass properties | Mass, COM, full inertia tensor, frame, approximation path, gyroscopic treatment |
| Constraints | Joint type/axis/frame, limits, compliance, stabilization, residual definition |
| Contacts | Contact law, friction cone/dimensions, static/kinetic law, restitution, regularization, warm start, manifold policy |
| Integration | Integrator, order, fixed/adaptive step, substeps, damping and event timing |
| CCD | Supported pairs, translational/rotational sweeps, speculative contacts, TOI bounds, iteration/early exit |
| Actuators | Control mode, transmission, torque/force/speed limits, dynamics, delay, saturation, loss/thermal boundary |
| Observations | Pose/quaternion convention, velocity and force frames, sign convention, timestamps, interpolation, absent observables |
| Numerical profile | Solver/version, precision, tolerance, iterations, backend, deterministic policy and hardware |

A field may be `NotApplicable`, `NotProvided` or `Unknown`; absence is never encoded as numeric zero.

## 4. Evidence workflow

1. Freeze a backend-neutral scenario and requirement profile.
2. Pin engine version, adapter commit, model revision, solver and feature set.
3. Generate backend-specific capability/path declarations from exact code/configuration.
4. Produce a pairwise mapping receipt with dimension status, source, transformation, error bound and reviewer.
5. Check unit/frame conversions with analytical and metamorphic controls.
6. Admit only when all decision-critical dimensions are equivalent or bounded-equivalent.
7. Preserve raw traces; emit a normalized observation trace as a separate derived artifact.
8. Report semantic mismatch, numerical divergence and uncertainty independently.

The mapping must not be self-certified by the backend that produced the result. High-consequence mappings require an independent reviewer, analytical oracle or second implementation.

## 5. Version-pinned external reference baselines

As checked 2026-11-09:

| System | Baseline | Important comparison implications |
|---|---|---|
| MuJoCo | 4.0.0 (2026-11-06) | Speculative convex contacts for tunnelling resistance; stateful ideal and continuum elasto-plasticity. Record exact contact mode and plastic state. |
| Isaac Lab | 3.0.0 GA (2026-10-20) | Task API spans multiple physics/render backends. Pin the selected backend independently of the Isaac Lab release. |
| Newton | 1.6.0 | GPU/multi-solver ecosystem; pin solver path, Warp/CUDA, model importer and feature flags. |
| Project Chrono | 10.0.0 (2026-03-27) | Broad multibody, vehicle, FSI and peridynamics scope. Select a task-specific module and model path. |
| Rapier | 0.18.0 in the current Symtropy bridge manifest | Preserve old evidence under 0.18.0; compare newer versions in a separate lane. |

References:
- [MuJoCo 4.0.0 changelog](https://mujoco.readthedocs.io/en/4.0.0/changelog.html)
- [Isaac Lab releases](https://github.com/isaac-sim/IsaacLab/releases)
- [Newton releases](https://github.com/newton-physics/newton/releases)
- [Project Chrono 10.0.0 announcement](https://projectchrono.org/news/)
- [FMI 3.0.2 specification](https://fmi-standard.org/docs/3.0.2/)

## 6. First negative controls

1. **Friction mismatch:** nominally same friction coefficient but one engine uses a different friction-cone/contact formulation. Expected: `NonEquivalent` until mapped and bounded.
2. **CCD mismatch:** one engine uses swept time-of-impact for the tested convex shape while the other uses discrete or unsupported CCD. Expected: `Unsupported`/ `NonEquivalent`, never a matched score.
3. **Frame mismatch:** equivalent physical pose represented with different quaternion order or angular-velocity frame. Expected: reject normalization until the conversion is explicit and metamorphically verified.
4. **Mass-property mismatch:** one backend uses a full inertia tensor and the other uses a scalar/mean approximation. Expected: `MappedEquivalentWithBound` only for a scenario where the approximation error is bounded; otherwise `NonEquivalent`.
5. **Integrator/damping mismatch:** continuous-time damping is compared with a discrete integrator whose stiffness/damping treatment changes the step map. Expected: separate profile or bounded mapping.

## 7. Benchmark reporting

A paired report should include:

- scenario and configuration digest;
- engine, adapter, solver and build identities;
- semantic status per dimension;
- every conversion/approximation and its bound;
- exact raw and normalized trace digests;
- trajectory, force/impulse, contact timing, penetration, constraint residual and energy diagnostics where comparable;
- numerical tolerance and test result;
- unsupported dimensions and inconclusive outcomes;
- hardware, precision, thread/GPU settings and latency distribution.

A result with a non-equivalent semantic receipt may be useful exploratory evidence, but cannot be called a matched solver-accuracy comparison.

## 8. Ownership boundaries

- #4366 owns the semantic-equivalence contract and pairwise admission.
- #443 owns the Rapier execution adapter.
- #460 owns the shared physics qualification corpus and evidence record integration.
- #1472 owns Symtropy's exact capability/model-path census.
- #1172 owns scenario/replay identity and orchestration.
- #3425 owns hard-contact gradient benchmark semantics.
- #3879/#3970 own checked/general-shape CCD correctness and TOI evidence.
- #3357 owns locomotion task/controller comparison.
- #4337 owns the physical humanoid joint work package.

This document does not replace any of those authorities.
