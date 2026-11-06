# Scenario-bound physics replay evidence v0.2

**Status:** implementation candidate; exact-head qualification pending.

This is the v0.2 extension of the scenario-bound replay evidence API. See [the protocol note](../../docs/ops/PHYSICS_KINEMATIC_POSE_REPLAY_V0_2.md).

v0.2 adds `WorldCommand::SetKinematicPose`, binding a kinematic body's translation, proper Rotor orientation, linear velocity and angular velocity into one ordered command. The command is preflighted against body type and finite/proper pose requirements, applied before the physics frame, and included in the v2 replay-tape digest. The evidence receipt schema is versioned to `symtropy.physics.replay-evidence.v0.2`.

The focused workflow executes the full `replay_harness` integration test, not only library unit tests. A pass would establish deterministic replay of the declared physics tape and pose command under the named caller-supplied profiles. It would not prove actor-policy validity, human behaviour, road realism or safety.
