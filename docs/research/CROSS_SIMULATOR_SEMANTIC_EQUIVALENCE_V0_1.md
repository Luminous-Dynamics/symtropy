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
| Newton | 1.7.0 (2026-10-14) | GPU/multi-solver ecosystem; pin solver path, Warp/CUDA, model importer/options, damping mode, angular-unit compatibility flags, joint-reference conventions and MPM material scaling. |
| Project Chrono | 10.0.0 (2026-04-07) | Broad multibody, vehicle, FSI and peridynamics scope. Select a task-specific module and model path. |
| Rapier | 0.18.0 in the current Symtropy bridge manifest | Preserve old evidence under 0.18.0; compare newer versions in a separate lane. |

References:
- [MuJoCo 4.0.0 changelog](https://mujoco.readthedocs.io/en/4.0.0/changelog.html)
- [Isaac Lab releases](https://github.com/isaac-sim/IsaacLab/releases)
- [Newton 1.7.0 release](https://github.com/newton-physics/newton/releases/tag/v1.7.0)
- [Project Chrono 10.0.0 release](https://github.com/projectchrono/chrono/releases/tag/10.0.0)
- [FMI 3.0.2 specification](https://fmi-standard.org/docs/3.0.2/)

## 5.1 V0.2 proposal — effective imported-model identity

V0.1 is a pairwise decision receipt. It does not yet make importer/lowering identity a first-class field set. Do not change V0.1 in place. A future V0.2 should bind the *effective model actually presented to the solver*, not only the authored asset and backend release.

Proposed identity chain:

```text
authored source artifact(s)
  -> importer/parser + options
  -> normalized physical-problem representation
  -> backend lowering / selected model path
  -> effective imported-model digest
  -> solver profile + execution environment
  -> raw observations
```

Proposed fields (normative fields or digested subrecords):

| Field | Why it matters |
|---|---|
| `source_artifacts[]` | Preserve original MJCF/URDF/USD/native files and their individual digests |
| `importer_id`, `importer_version` | Parser and conversion implementation can change physical meaning |
| `importer_options_digest` | Bind unit conversion, damping interpretation, reference poses and compatibility flags |
| `normalized_problem_ir_digest` | Bind units, frames, entity topology and declared physical semantics before backend lowering |
| `effective_model_digest` | Identify the actual engine model after import/lowering, not just the authored source bytes |
| `lowering_profile_digest` | Bind all exact/approximate/unsupported model-path selections and conversion rules |
| `solver_path_id`, `solver_profile_digest` | Distinguish engines with multiple solver/integrator/contact paths |
| `execution_environment_digest` | Bind precision, hardware/backend, thread policy, relevant runtime and feature flags |

Newton 1.7.0 is a concrete motivating example: its release notes describe changed or clarified URDF damping interpretation, USD angular-velocity conversion, joint-coordinate reference handling and MPM material scaling. Two runs using the same authored asset bytes can therefore represent different effective models when importer version/options differ.

The IR must remain an interchange contract, not a new simulator or a duplicate of #1472's capability census. Pairwise admission remains #4366's responsibility; #1472 declares exact backend capabilities/model paths; #1172 owns scenario identity and replay; #460 owns benchmark scenarios and evidence; #443 owns Rapier execution/readback.

### V0.2 acceptance tests

- Source artifact digest unchanged but importer option changed: effective model identity must change or the adapter must prove the option is semantically irrelevant for that asset.
- Same source/importer/options but solver path changed: solver profile identity must change.
- Unit/frame conversion round trip: independently verified within an explicit bound.
- A lossy conversion (for example, full inertia tensor to isotropic approximation): reported as approximation with a domain-specific bound or excluded from matched scoring.
- Unsupported required feature: fail closed; no silent importer fallback.
- Post-import model digest reproduced independently from a clean checkout and pinned toolchain.

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
