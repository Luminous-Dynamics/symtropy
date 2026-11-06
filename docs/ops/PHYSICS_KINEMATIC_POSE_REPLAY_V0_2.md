# Scenario-bound Physics Replay Evidence v0.2 — full kinematic pose

**Status:** implementation candidate; exact-head CI pending  
**Crate:** `symtropy-physics`  
**API:** `WorldCommand::SetKinematicPose`, `execute_with_evidence`, `verify_replay_evidence`  
**Composition target:** ROAD-EGCI #3532 / #3886

## Purpose

v0.1 replay evidence bound an ordered command tape and per-tick snapshots but did not contain a single command for changing a kinematic actor's complete pose. v0.2 adds one atomic, versioned command for a declared kinematic body:

- translation;
- canonical `Rotor` orientation;
- linear velocity;
- angular velocity as a `Bivector`.

The command is applied at the frame boundary before the physics step. It is intentionally distinct from `SetLinearVelocity` and `SetAngularVelocity`, and it cannot be applied to a dynamic or static body.

## Validation

Before the first world mutation, the full replay tape is checked for:

- existing body handle;
- target body type exactly `Kinematic`;
- finite translation, rotation matrix and linear/angular velocity;
- proper rotation at tolerance `1e-9` (orthogonality and determinant +1);
- all pre-existing timestep, command-value and tape-budget conditions.

The command executor repeats the local body-type and pose checks before mutating the body. A valid pose is assigned as a complete transform and velocity tuple, then the body is woken. The physics world's normal step remains the only integrator.

## Commitment/versioning

- Replay evidence schema: `symtropy.physics.replay-evidence.v0.2`.
- Replay tape hash domain: `symtropy.physics.replay-tape.v2`.
- New command tag: `5`.
- The digest binds body handle, exact translation bits, every rotation-matrix element, linear velocity and every angular-velocity matrix element, in tape order.

This is a deliberate version boundary. A v0.1 receipt cannot be treated as a v0.2 full-pose receipt.

## Tests

The existing seven replay-evidence tests are retained. Five new tests qualify:

1. Full kinematic pose command round-trips through separately constructed worlds and retains the complete pose in bitwise snapshots.
2. Dynamic-body target is rejected before mutation.
3. Reflection / improper rotation is rejected before mutation.
4. Non-finite pose payload is rejected before mutation.
5. A changed pose changes both the input tape digest and receipt digest.

## Claim ceiling

This establishes a replayable, validated command interface for caller-declared kinematic poses. It does not validate road geometry, actor policy quality, human motion, sensor/actuator latency, vehicle dynamics, collision avoidance, physical fidelity, safety, or SAE L4/L5 capability. The caller remains responsible for authentic scenario, engine/build, initial-world and environment identities; the receipt digests are not signatures or engine attestation.
