# Careframe Static Collision-Proxy Adapter v0

**Evidence level:** code/interface proposal awaiting exact-head CI. It uses synthetic model/state fixtures. It is not a dynamic articulated robot, validated digital twin, or medical-device qualification.

## Entry point

The new module in `symtropy-robotics-bridge-core::careframe` exposes:

```rust
spawn_static_collision_assembly(
    &mut physics_world,
    morphology_json,
    joint_state_json,
    base_position,
)
```

It validates the full morphology and joint-state map, computes all link transforms, then adds each link as a `BodyType::Static` rigid body with an oriented `HyperBox<3>` collision shape. The result maps morphology link IDs to real Symtropy physics body handles.

## Why this is a useful intermediate step

The standalone Careframe contract already computes geometry-only forward kinematics. This adapter proves the next narrow interface: each FK link pose and bounding-box definition can become a body/shape in the actual `PhysicsWorld<3>`. A test also places an independent dynamic sphere into the synthetic scene and checks that collision detection creates a contact.

It gives collision queries a shared geometry representation without claiming the whole robot is dynamically simulated.

## Fail-closed behavior

- Every morphology object rejects unknown JSON fields via `serde(deny_unknown_fields)`.
- Schema/evidence/units and synthetic geometry source are checked.
- Link/joint identifiers, parent/child references, root/parent counts and connectivity are checked.
- Fixed/revolute constraints, axes, joint limits, positive dimensions and mass placeholder bounds are checked.
- The controller must be kinematics-only; the joint command flags must be false; the authority flags for physical actuation, patient contact and clinical use must all be false.
- Joint-state input must contain every revolute joint in bounds and may not contain unknown or fixed-joint keys.
- Morphology and state validation and all FK computation occur before the world is changed. Tests assert invalid input leaves the body count unchanged.

## What is actually imported

Only static geometry. Even when a revolute joint value changes, the adapter creates a new frozen pose for every link; it does not install constraints, run a joint solver, expose actuators or update the shape hierarchy over time. Each collider is an oriented box centered on the link frame. All dimensions and masses in the source JSON are synthetic placeholders.

## Acceptance tests

- Nominal model imports as 16 static box proxies and adds zero constraints.
- Root and torso positions match known reference coordinates.
- One-radian torso yaw moves the left arm origin to the expected analytic position and produces a proper rotation.
- A dynamic test sphere overlapping the chassis proxy produces a nonempty physics contact set.
- Unknown nested fields, unsupported shape, physical-authority escalation, and out-of-range joint state reject.
- Every rejected request leaves the physics body's count unchanged.
- Focused exact-head CI runs rustfmt, `cargo test --locked -p symtropy-robotics-bridge-core`, and Clippy with warnings denied.

These are software integration checks only. A green result would establish that this code path executes against the current Symtropy physics crate and synthetic fixture, not that the real robot is buildable, dynamically accurate, safe around people, or clinically suitable.

## Next stages after CI

1. Run the exact-head focused Rust tests and resolve failures rather than inferring success from static review.
2. Compare link transforms between the standalone FK oracle and physics-body transforms over randomized valid joint states, including boundaries and mirrored poses.
3. Validate broadphase/narrowphase coverage and measure oriented-box contact errors against analytic fixtures and independently computed references.
4. Design a true articulated dynamic adapter using appropriate engine constraints only after validating joint-origin conventions, orientation constraints, inertias, limits and solver stability. Do not use the current static importer as motor-control infrastructure.
5. Keep HIL, non-clinical guarded bench, human-contact and clinical stages independently gated.

## Claim ceiling

If all focused software checks pass, an acceptable claim is: “The synthetic morphology can be imported as static oriented-box proxies and participates in the existing Symtropy collision pipeline under the recorded revision and tests.” This does not establish an articulated simulation or any physical/clinical safety property.
