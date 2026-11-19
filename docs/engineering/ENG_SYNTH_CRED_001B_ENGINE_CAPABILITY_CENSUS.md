# ENG-SYNTH-CRED-001B — Exact engine capability and model-path census reference

**Status:** frozen source reference v1; independent qualification is pending.  
**Parent:** [#1472 — exact read-only engine capability and model-path census](https://github.com/Luminous-Dynamics/symtropy/issues/1472)  
**Predecessor:** [#1468](https://github.com/Luminous-Dynamics/symtropy/issues/1468) / source reference [PR #1469](https://github.com/Luminous-Dynamics/symtropy/pull/1469), independently qualified by [PR #1471](https://github.com/Luminous-Dynamics/symtropy/pull/1471).  
**Fresh source baseline:** `main@540a6097be6bc31b85f0e43e5f31947fa078c6f4` (19 November 2026).

## Why this reference exists

The census answers a narrow question:

> What can this exact Symtropy engine revision and feature profile represent, and through which exact, numerical, approximate, provider-dependent, unsupported or unknown path?

It does not answer whether a model is credible for a context of use.

```text
capability advertised
  != capability exercised
  != run numerically healthy
  != model credible for an intended use
  != physical truth
  != requirement satisfied
  != authorized action
```

This is a refreshed source reference, not the production descriptor/API. The previous #1469/#1471 reference was pinned to an older source commit. #1471's exact-head hosted reference-qualification workflow completed successfully, but that result qualifies its pinned older reference—not this newly refreshed baseline. This tranche therefore starts the source-refresh and independent-qualification sequence again.

## Frozen source identity

- Repository: `Luminous-Dynamics/symtropy`
- Main commit: `540a6097be6bc31b85f0e43e5f31947fa078c6f4`
- Engine package: `symtropy-physics 0.2.1`, Rust edition 2024
- Geometry package: `symtropy-math 0.2.1`
- Source lock: 25 exact Git blob IDs in [the machine-readable reference](../../release/evidence/eng-synth-cred-001b-engine-capability-reference-v1.json)
- Canonical semantic identity: SHA-256 over UTF-8 canonical JSON (sorted object keys, compact separators, excluding only `content_identity.digest`). The exact digest is in the reference.

The source lock spans engine manifests, public exports, world/contact/CCD/integrator/body/joint/replay/diagnostic implementations and the geometric shape contract. A source file changing without a corresponding refreshed reference fails the qualifier.

## Capability path findings

### Geometry and contact

The public geometric layer exposes Sphere, HyperBox, Capsule, HalfSpace, ConvexHull and CompoundShape support-map shapes. A trait or generic type is not a pairwise collision guarantee.

| Requested path | Current route | Census disposition |
|---|---|---|
| Exactly one HalfSpace plus one bounded convex shape | Dedicated analytical HalfSpace path | Supported under that declared path |
| HalfSpace versus HalfSpace | No analytical path; intentionally excluded from generic GJK/EPA | Unsupported |
| HyperBox pair, identity rotations | Axis-aligned SAT | Supported under implemented SAT path |
| Rotated HyperBox pair, 2D/3D | Oriented-box SAT | Supported under implemented SAT path |
| Rotated HyperBox pair, 4D | Transform-aware GJK/EPA | Numerical model, not the 2D/3D SAT path |
| Other bounded convex support-map pairs | Transform-aware GJK/EPA | Numerical model |
| Non-convex mesh semantics | Optional downstream mesh provider | Provider-dependent; caller must bind provider identity |
| Generic convex fallback for a mesh-like collider | Support-map path where applicable | Approximation, not native non-convex mesh contact |

### CCD and ray queries

World CCD is not generic continuous collision detection. The current world pass considers dynamic, awake, non-sensor Sphere casters above speed 5.0 and tests Sphere/HalfSpace targets through analytic sweep routines. Fast boxes, capsules and generic convex bodies are not covered by that world CCD path.

Transformed ray queries have analytic paths for Sphere, HyperBox, Capsule and HalfSpace. Other shapes use a transformed bounding-sphere fallback and can produce false-positive geometry hits. Neither path is a calibrated sensor model or an observation-evidence source.

### Joints, motor and constraints

Fixed, Ball, Hinge and Prismatic joint types are exported; a public type export does not qualify all parameter regimes.

The Hinge implementation is particularly important for robotics and exoskeleton use:

- it uses a velocity-only motor path;
- the configured `target_pos` is not consumed by that path;
- `angle_limits` are stored by the builder but are not enforced by the inspected hinge solve;
- motor effort is converted with `torque * dt` and a fixed half-split angular-velocity update, not the endpoint inverse-inertia mapping;
- the recorded work expression must not be mistaken for proof that the applied generalized impulse is physically correct.

[PHYS-HINGE-MOTOR-001 / #4516](https://github.com/Luminous-Dynamics/symtropy/issues/4516) owns the focused correction and analytic tests. Until then, hinge position targeting, hard limits and unequal-inertia dynamics are not admissible assumptions merely because the fields/types exist.

### Determinism, sleep and diagnostics

The `deterministic-net` feature does not quantize every dynamical variable. The inspected world path applies Q16.16 quantization to:
- force accumulator components before integration;
- translation components after integration;
- linear velocity components after integration.

It does not equivalently quantize torque, angular velocity, rotation, contact cache/lambda, constraint state, callback outputs or provider outputs. The feature comment must not be promoted into an all-state cross-machine bitwise theorem.

Sleep is a state transformation: after the configured low-speed tick count, the body is marked sleeping and both linear and angular velocity are set to zero. Default world settings are solver iterations 4, sleep threshold 0.5 and sleep ticks 60. These are model/profile identity, not a hidden optimization detail.

`InvariantSnapshot` observes mass, center of mass, momentum, kinetic/potential/mechanical energy, speed, rotation errors, penetration and non-finite body count. However, `is_numerically_healthy` checks only:
1. no non-finite body count;
2. rotation orthogonality error within the supplied tolerance;
3. rotation determinant error within the supplied tolerance.

It does not check the other measured fields, input sanitization, clamp activation, callback/provider activity, sleep zeroing, Q16 error or model-path applicability. A true result is not comprehensive numerical qualification.

### Replay and mass properties

Low-level `ReplayTape`, `WorldCommand` and `WorldSnapshot` use transient `BodyHandle` identity and raw floating-point state bits. SYMTAPE v1 is a CLI proof of concept that reconstructs bodies from sorted `NetId` values for dimensions 2, 3 and 4. Its per-tick FNV-1a 64 digest is non-cryptographic; it is a divergence diagnostic, not an evidence commitment. A generalized reusable SYMTAPE library and a portable engineering identity envelope are not established by this source.

The integrator uses a mean inverse-inertia scalar for angular acceleration, and kinetic-energy diagnostics use a mean inertia scalar. These are exact only for isotropic inertia profiles such as spheres; asymmetric rigid bodies retain a known approximation. `merge_toolhead` additionally estimates tool inertia with a fixed (0.4 m r^2)-style term at a hard-coded length scale and collapses the combined result to an isotropic scalar.

A ten-box stack regression remains ignored with a source comment describing failure to settle cleanly even after trying 30 solver iterations. It must remain a negative finding; the separate three-box regression does not erase it.

### Capacity and overflow

The contact module declares `MAX_CONTACTS = 8`; the current manifold generator selects at most four points. That is a per-manifold detail, not a global world contact-pair limit. The inspected source does not establish a single maximum body count, global constraint count, broadphase capacity or factorization-workspace limit. Those remain `UnknownUncensused` rather than guessed.

## Reference corpus

The machine-readable reference freezes 30 mutations/controls. They cover:
- stable identity and feature-set changes;
- HalfSpace, HyperBox and CCD path selection;
- provider/callback semantics;
- asymmetric inertia and long-stack limitations;
- diagnostic-health boundaries;
- replay identity and hashing;
- sensor/raycast claim boundaries;
- joint/limit/motor capability;
- Q16 coverage and sleep behavior;
- mass-property approximations;
- unknown resource capacity and authority non-promotion.

The corpus is a static reference suite. It does not execute the physics engine or create physical observations.

## Independent qualifier

`scripts/validate_eng_synth_cred_001b.py` is a standard-library-only reference oracle. It:
1. recomputes the canonical SHA-256 identity;
2. verifies every locked source path resolves to its exact Git blob at the frozen commit;
3. checks high-consequence source anchors (Q16 coverage, sleep zeroing, CCD path, ray fallback, narrow health predicate, replay hashing, hinge behavior and ignored stack finding);
4. checks the complete 30-case reference corpus and required status boundaries.

The dedicated GitHub Actions workflow runs that oracle against the exact repository history. This is deliberately a research/qualification script, not production runtime code and not a substitute for the future Rust descriptor.

## Required next stages

1. Review and independently qualify this fresh static reference.
2. Derive a read-only Rust descriptor from actual production types/constants—do not add self-test execution or mutate a PhysicsWorld during census generation.
3. Differential-test the Rust descriptor against this frozen reference, including feature/profile changes.
4. Prove descriptor construction is read-only and has no physics side effects.
5. Only then make the descriptor an input to EngineeringDeterministic admission and run-health policy.

Compose the ownership boundaries:
- #1472 owns this engine capability/model-path census.
- #4366/#4369 own pairwise cross-simulator semantic equivalence.
- #460 owns the single physical qualification corpus and run-result evidence.
- #1468/#1469/#1471 own the upstream EngineeringDeterministic reference and its independent qualifier.
- #4516 owns hinge motor/limit repair.

## Claim ceiling

A future qualified PASS may establish only that Symtropy reports the capabilities and known limitations of one exact source/profile revision correctly. It does not establish physical validity, accuracy for any engineering context of use, model credibility, requirement satisfaction, safety, certification, authorization or physical-control authority.
