# Symtropy road actor path adapter v0.1

**Status:** bounded synthetic route-to-pose reference, stacked on the physics-independent actor-policy core #3489 and owned by #3502. This crate is an adapter boundary, not a pedestrian model or simulator.

## Contract

The actor policy exposes a bounded forward-speed setpoint. The physics engine requires a world-space kinematic pose. V0.1 bridges only a scenario-authored straight directed segment:

`ScenarioActorId + policy profile/state + route artifact + tick clock + measured speed/progress`
`-> acceleration-limited speed + arc-length progress + x/y pose + heading + receipt`

The route explicitly binds:
- stable route ID;
- named Cartesian coordinate frame;
- start/end points in metres;
- directed travel orientation;
- canonical route digest.

The adapter derives heading from the route vector, never from current body velocity. A zero measured speed therefore does not create an undefined heading.

## Motion semantics

- Time is derived from integer `elapsed_ticks / ticks_per_second`, not wall-clock time.
- Acceleration and deceleration limits are separate and explicit.
- The policy speed remains a requested setpoint; the adapter records both requested and applied speed.
- Displacement uses a declared trapezoidal integration approximation over the interval.
- Reverse movement is rejected in v0.1.
- Missing route profiles and policy states without a delivered decision tick are typed errors.
- A step that would reach or cross the route endpoint is rejected as `PathEndWouldBeReached`; it is never silently clipped or extrapolated.
- Only one straight segment is representable. Curved, multi-segment and discontinuous routes are out of scope until a separately versioned geometry profile exists.

## Replay and evidence boundary

Each `PathStepReceipt` binds the actor ID, policy profile, policy mode and decision tick, route/frame IDs and digest, tick basis, motion limits, prior progress and measured speed, requested/applied setpoints, requested/applied distance and target pose. Verification re-runs the projection from the exact inputs. The BLAKE3 digest is an unkeyed content commitment, not authentication or proof that route/policy evidence is true.

This reference does not advance `PhysicsWorld`, call a simulator, infer human intent, choose an actor action, or authorize actuation. A downstream physics adapter must map `KinematicPoseTarget` to #3539's scenario-bound `SetKinematicPose` command and retain the path receipt alongside the physics receipt. The two receipts are complementary and must not be collapsed into one untyped hash.

## Commands

```sh
cargo test -p symtropy-road-actor-adapter
cargo clippy -p symtropy-road-actor-adapter --all-targets -- -D warnings
```

## Claim ceiling

Passing establishes route-to-pose bookkeeping and receipt replay for the declared straight-line synthetic profile. It does not validate actor policy quality, real pedestrian trajectories, physics integration, collision probability, roadway safety or SAE L4/L5 capability.
