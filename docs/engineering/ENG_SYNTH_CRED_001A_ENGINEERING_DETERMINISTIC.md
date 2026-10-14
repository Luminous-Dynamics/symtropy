# ENG-SYNTH-CRED-001A — EngineeringDeterministic model/numerical-integrity reference

Issue: #1468  
Parent: #1173 / #1172  
Frozen base: `d56d6a6c1381dc131d493f0713b657f01c98cb80`  
Corpus SHA-256: `891aa10b0040f6681d1288cce1243e6f85204800bb121f264aa45e45d0f93a7b`  
Authority: **simulation-model and numerical-integrity semantics only; no physical-validity or engineering authority**.

## Purpose

Freeze the first execution-free target for `EngineeringDeterministic` before changing runtime physics or executing engineering digital twins.

```text
execution completed
!= numerically healthy
!= model applicable
!= model credible for intended use
!= engineering claim satisfied

deterministic
!= accurate
```

This tranche changes no ordinary game/runtime physics.

## Exact source audit

| Source | Git blob |
| --- | --- |
| `crates/core/symtropy-physics/src/world.rs` | `7dccf283f858c4874c96425fd7aa8aa3e176a557` |
| `crates/core/symtropy-physics/src/integrator.rs` | `c2e32f1bc14adf249f3b797e54209d501e491d9d` |
| `crates/core/symtropy-physics/src/diagnostics.rs` | `d4bab2ec7217471653315986eb77f0d13a895d44` |
| `crates/core/symtropy-physics/src/bin/replay_cli.rs` | `17673d1ab937ea3ccb282508a07b4b140add30e3` |
| `crates/core/symtropy-math/src/halfspace.rs` | `5ce17d01f793bc543037310bec737f463bb51e87` |

### Timestep

`PhysicsWorld::step(dt)` currently has no engineering precondition on `dt`. The future wrapper must reject zero, negative, and non-finite timesteps before ordinary engine execution.

### Non-finite values

The integrator sanitizes non-finite force and torque accumulators to zero before integration. Those **input sanitizations currently do not increment `NAN_ZEROED_COUNT`**. Post-integration non-finite linear/angular velocity is zeroed and counted; a non-finite position freezes linear velocity rather than teleporting the body.

For engineering use:

```text
sanitized input/state
!= healthy unmodified prediction
```

### Numerical guards

Current ordinary-engine guards include:

```text
linear speed           <= 1000
angular speed          <= 100
position-correction bias <= 10
```

A guard firing must be retained. This reference does not declare every guard event fatal in every future context of use.

### Rotational inertia

The current integration/contact angular response uses scalar mean inverse inertia. Source comments state this is exact for isotropic bodies such as spheres and approximate for asymmetric bodies.

```text
stable asymmetric-body run
!= validated asymmetric rotational dynamics
```

### Callbacks and providers

`PhysicsCallback` can modify force, collision impulse, and friction. `step()` uses `NoOpCallback`; `step_with_callback()` admits a supplied callback.

`EngineeringDeterministic` therefore rejects an undeclared callback. A declared non-default callback belongs to a distinct model/profile identity.

Mesh collision behavior can also be injected through a registered provider. Its identity is claim-relevant when used. Without that provider, source comments document a generic GJK fallback behaving as a convex-hull approximation for mesh pairs.

### HalfSpace audit correction

The historic one-HalfSpace GJK/EPA fallback concern is **not current `main` behavior**.

Current `world.rs` resolves exactly one `HalfSpace` against a bounded support-mapped collider through the analytical path and does not fall through after an analytical no-contact result. `HalfSpace::support()` still uses finite `HALFSPACE_EXTENT = 1e6` for generic GJK compatibility, but the current world avoids it for exactly-one-HalfSpace pairs.

`HalfSpace` vs `HalfSpace` remains unsupported.

The reference freezes current source truth rather than carrying forward the obsolete limitation.

### Deterministic-net

With `deterministic-net`, forces are Q16.16-quantized before integration and positions/linear velocities after integration.

```text
Q16DeterministicNet
!= NativeFloatExactEnvironment
```

No accuracy ordering is implied.

### Existing diagnostics

`InvariantSnapshot` / `InvariantDrift` already expose mass, center of mass, momentum, energy, speed, rotation errors, penetration, and non-finite body count.

`InvariantSnapshot::is_numerically_healthy()` checks only non-finite body state and rotation tolerances. It does not prove absence of clamps, input sanitization, unsupported physics, profile mismatch, callbacks/providers, or context-of-use problems.

### Existing replay

`SYMTAPE` v1 already records version/dimension, raw-bit world values, solver settings, stable `NetId`s, body definitions, frame commands, and per-tick state hashes for 2D/3D/4D demo tapes. It explicitly does not guarantee cross-platform bitwise equality.

Its FNV-1a state hash is a replay/debug diagnostic, not an engineering provenance commitment.

### Preserved negative result

Current source keeps an ignored ten-box stack test documenting a separate long-chain solver-scaling problem after the three-box regression was repaired.

```text
bounded small-stack result
!= arbitrary long-stack applicability
```

This negative finding is part of the engineering model boundary, not something to hide.

## Frozen semantic surface

### `EngineeringPhysicsModelProfileV1`

A future runtime type must bind exact source/features, dimension, integration/timestep profile, translational/rotational model, contact paths, joints/constraints, CCD coverage, damping/friction/restitution semantics, HalfSpace behavior, callback/provider policies, numerical guards, non-finite behavior, determinism profile, diagnostics, unsupported phenomena, approximations, and claim ceiling.

There is no universal physics-fidelity score.

### `NumericalIntegrityPolicyV1`

Frozen anomaly vocabulary:

```text
NonFiniteInput
NonFiniteState
StateSanitized
LinearVelocityClamped
AngularVelocityClamped
PenetrationBiasClamped
UnsupportedContactPair
ApproximateContactPath
RotationalInertiaApproximationActive
UndeclaredPhysicsCallback
DeclaredPhysicsCallbackActive
UndeclaredMeshProvider
DeclaredMeshProviderActive
QuantizationActive
ConstraintResidualExceeded
EnergyInvariantDrift
MomentumInvariantDrift
ReplayMismatch
InvalidTimeStep
OtherExplicit
```

Policy action is separately versioned; logging level is not policy.

### Determinism profiles

```text
NativeFloatExactEnvironment
Q16DeterministicNet
PortableSemanticTolerance
```

`PortableSemanticTolerance` compares declared engineering observables under typed units/frames/tolerances. It is not a bitwise-equality promise.

### `RunHealthReceiptV1`

A later executable receipt should bind exact run/model/policy/determinism identities, invariant snapshots, anomaly records, unsupported/approximate semantics, replay result, categorical health state, and claim ceiling.

Candidate categorical states:

```text
ExecutionRejected
ExecutionIncomplete
NumericallyInvalid
NumericallyHealthyUnderProfile
HealthyWithDeclaredLimitations
ReplayMismatch
ApplicabilityReviewRequired
```

No scalar run-health score.

## Reference corpus

`docs/release/evidence/eng-synth-cred-001a-engineering-deterministic-reference-v1.json` is canonical compact sorted JSON plus newline with SHA-256:

`891aa10b0040f6681d1288cce1243e6f85204800bb121f264aa45e45d0f93a7b`

It contains **26 cases / 22 dispositions**, including valid baseline; invalid timesteps; NaN/Inf authored inputs; non-finite post-state; all three numerical guards; callback/provider boundaries; isotropic vs asymmetric inertia; current HalfSpace analytical handling; unsupported HalfSpace pair; mesh fallback approximation; Q16 distinction; replay conformance/mismatch; invariant drift; unsupported phenomenon; model-profile mismatch; and trace-level behavior mutation.

All dispositions are exercised.

## Derivation precedence

Reference semantics are deterministic and fail-first:

1. invalid timestep;
2. non-finite authored input;
3. non-finite state;
4. undeclared callback/provider;
5. unsupported phenomenon;
6. model-profile mismatch;
7. trace behavior mutation;
8. replay mismatch;
9. numerical guards;
10. declared profile/approximation boundaries;
11. replay/invariant informational outcomes;
12. ordinary admissible-no-known-anomaly state.

This is a semantic oracle, not runtime implementation.

## Relationships

```text
RunHealthReceiptV1
!= Symthaea ModelCredibilityPassport
```

and:

```text
numerically healthy twin run
!= physical twin truth
```

Symtropy #1467 must eventually bind an exact run-health receipt for each claim-bearing synthetic twin trajectory. Symthaea owns intended-use credibility, physical comparison, evidence/currentness, design acceptance, and authority.

## Qualification target

An independent stdlib-only qualifier should establish only that this frozen artifact has the exact schema/authority/base/source blobs/vocabularies; 26 unique cases; all 22 dispositions; independent disposition derivation; current HalfSpace semantics; explicit unsupported HalfSpace pair; and no physical-validity, acceptance, or actuation authority.

A qualifier PASS does **not** establish runtime instrumentation, physical validity, or engineering suitability of Symtropy.
