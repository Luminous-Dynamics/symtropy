# Careframe Forward-Kinematics Contract v0

**Status:** geometry-only transforms for a synthetic kinematic tree. Not a dynamics engine, a motor controller, validated CAD, a physical robot, or clinical qualification.

## Command

```text
medical-robot-tool-gate --evaluate-kinematics MORPHOLOGY_JSON JOINT_STATE_JSON
```

The validator first checks the morphology graph/envelope contract. It then checks the joint-state object and returns link transforms only. This mode has no actuator, velocity, force, controller or physics side effects.

## State input contract

`careframe-nominal-joint-state-v0.json` provides an example containing exactly one numeric radian value for each of the nine revolute joints in the synthetic model.

- Every revolute joint must be represented exactly once.
- Values must be finite JSON numbers within the joint's declared lower/upper limits.
- Unknown joint IDs reject; supplying a state for a fixed joint rejects; missing revolute states reject.
- These are geometric pose values, not actuator commands, controller targets, or permission to move anything.
- The current fixture has revolute axes and joint origins, but no joint-origin orientation (RPY/quaternion). That is a known representational boundary. Adding joint-origin orientation requires an explicit schema revision and new reference fixtures—not an undocumented interpretation.

## Transform convention

Each link pose is expressed relative to the root link. The root link starts at identity rotation and zero translation. For each joint from a parent link to a child link:

1. Rotate the joint-origin translation by the parent's world rotation and add the parent's world translation.
2. For a revolute joint, compute an axis-angle rotation in the declared joint frame from the normalized axis and supplied radian position; fixed joints use identity rotation.
3. Compose the parent's rotation with the joint rotation to obtain the child rotation.
4. Continue along the validated tree until every link has a world pose.

Receipts emit each link's `translation_m` and `rotation_matrix_row_major`, plus the exact morphology, joint-state, source and tested-revision digests. This output is geometry-only.

## Reference fixtures and conformance

For the current synthetic model with all revolute joints at zero:
- root `base_chassis` has translation `[0, 0, 0]`;
- `torso_shell` has translation `[0, 0, 0.32]`;
- `left_tool_mount` has translation `[0, 0.19, 0.28]`;
- `right_tool_mount` has translation `[0, -0.19, 0.28]`.

With `torso_yaw = q` and all other revolute joints zero, the left arm origin should be `[-0.19 sin(q), 0.19 cos(q), 0.56]`, and the right arm origin should be `[0.19 sin(q), -0.19 cos(q), 0.56]`. The current test uses `q = 1.0 rad`, safely inside the illustrative synthetic bounds.

Negative fixtures reject missing states, unknown IDs, a state assigned to a fixed joint, an out-of-range joint position, invalid morphology, or disconnected/cyclic geometry. The CI step also checks the nominal transform coordinates and an out-of-range negative control.

## URDF 1.0 interchange

The Rust gate can also run:

```text
medical-robot-tool-gate --export-urdf MORPHOLOGY_JSON OUTPUT_URDF
```

It exports named links, centered box visual/collision geometry, parent/child joints, origins, revolute axes and limits. Each box is centered on its link frame because v0 has no per-geometry origin field; that is an explicit modeling assumption, not measured CAD. A digest-bound receipt labels the output `KINEMATIC_INTERCHANGE_ONLY`. The exporter rejects non-token names and unsupported geometry instead of silently changing the model. The current v0 schema omits joint-origin orientation, so the exporter writes `rpy="0 0 0"` for all joint origins.

The CI workflow independently parses the generated XML and checks link/joint counts and element structure. It does not run ROS or MuJoCo as a runtime, and the URDF intentionally omits inertial parameters, transmissions and controller tags. **This is a kinematic/collision interchange artifact, not a dynamics-ready or physics-validated robot model.**

## Claim ceiling

A passing result means only that **the declared synthetic tree produced transforms under the specified geometric convention at the recorded revision and joint-state digest**.

It does not establish:
- correct import or rendering by Symtropy's physics/Bevy engine;
- collision accuracy, inertias, stability, actuator dynamics, friction or contact forces;
- reachability in a real chassis, cable routing, interference, human factors or payload safety;
- physical buildability, clinical suitability or regulatory approval.

## Industry comparison and next adapter gate

ROS `robot_state_publisher` consumes a URDF kinematic tree and joint-state values to publish link transforms; MuJoCo supports articulated models in MJCF and loads URDF models. Careframe's present contract is deliberately smaller and Rust-first. Before claiming parity with either toolchain, build explicit import/export adapters and differential tests against a known reference model, including transform, joint limit and geometry checks. Do not assert compatibility solely because the internal schema has links and joints.

References:
- ROS robot_state_publisher: https://github.com/ros/robot_state_publisher
- MuJoCo model formats: https://mujoco.readthedocs.io/en/stable/modeling.html
- NIST Mobility Performance of Robotic Systems: https://www.nist.gov/programs-projects/mobility-performance-robotic-systems
- H2RBench (CoRL 2026): https://h2rbench.github.io/
