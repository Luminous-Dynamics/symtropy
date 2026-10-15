# Rigid Transform and Pose Composition V1

**Status:** Draft contract; exact-head Rust qualification required  
**Implementation:** symtropy-robotics-bridge-core::rigid_transform  
**Depends on:** qualified frame-convention contract in PR #1610

## One frozen data-transform convention

RigidTransformV1 stores a data-coordinate map from source into target:

    p_target = R_target_source * p_source + t_target_source

Translation is in metres. Rotation is an active, right-handed quaternion in XYZW order. Composition is ordered:

    T_A<-B.compose(T_B<-C) = T_A<-C
    R_A_C = R_A_B * R_B_C
    t_A_C = R_A_B * t_B_C + t_A_B

Only contiguous edges compose: the source frame of the left transform must equal the target frame of the next transform. Frame IDs are never inferred from convention labels or names.

## Data transforms versus frame-pose representations

This API stores data-coordinate transforms only. ROS 2 tf2 documents that its published frame formulation and the data-coordinate transformation formulation are inverses. The explicit `FramePoseV1` type represents the pose of a frame relative to a reference frame; `to_data_transform()` returns the inverse data map from that reference frame into the posed frame. Integrations must convert at this boundary rather than assuming both representations have the same direction. A regression composes the pose representation and data map in both orders and checks identity. See the official [tf2 transform semantics](https://github.com/ros2/ros2_documentation/blob/rolling/source/ROS-Framework/interfaces/About-Tf2/About-Tf2.rst).

The API is compatible with the base world-contract equation for static child-to-parent mappings. Coordinate axis conventions remain separate metadata as described in [ROS REP-103](https://reps.openrobotics.org/rep-0103/); this type does not guess conversions between ENU, NED, body, or camera frames.

## Points, directions, and composition

- PointInFrameV1 includes a frame label and metre coordinates. Transforming it applies rotation and translation.
- DirectionInFrameV1 includes a frame label and free-vector components. Transforming it applies rotation only.
- RigidTransformV1::compose composes rigid transforms/poses and rejects non-contiguous frame edges.
- QualifiedRigidTransformV1 binds each transform to the qualified-world content digest, rejects undeclared source/target frames, and refuses to compose edges bound to different qualified-world digests. A convention-registry mutation therefore prevents accidental mixing of edges from different qualified worlds.

Points and directions must be finite and match the transform source frame. Transformed non-finite results fail closed in their validated constructors. Binding a numerical transform to a world is a consistency check, not proof that its numeric values are physically correct.

## Quaternion acceptance policy

Input quaternion components must be finite, have nonzero norm, and satisfy abs(norm - 1) <= 1e-6. Accepted values are normalized once on construction. Sign is canonicalized by the first nonzero component in W, X, Y, Z order, so q and -q produce the same in-memory representation. Zero, near-zero, materially non-unit, and non-finite quaternions are rejected; arbitrary rescaling is not silently accepted.

## Pinned composition fixture

Let s = sqrt(1/2).

- T_A<-B: q XYZW = [0, 0, s, s], t = [1, 0, 0].
- T_B<-C: q XYZW = [s, 0, 0, s], t = [0, 2, 0].
- T_A<-C: q XYZW = [0.5, 0.5, 0.5, 0.5], t = [-1, 0, 0].
- Point [1, 0, 0] in C maps to [-1, 1, 0] in A.
- Direction [1, 0, 0] in C maps to [0, 1, 0] in A.
- Reversing quaternion multiplication gives [0.5, -0.5, 0.5, 0.5], a negative control.

Rust tests also cover identity on both sides, inverse/round trip, three-edge composition, mismatched frame rejection, quaternion sign equivalence, and invalid/non-finite inputs. A standard-library Python reference oracle independently calculates the fixture.

## Non-claims

The API establishes only algebra and validation invariants. It does not verify sensor calibration, externally supplied transforms, kinematic-provider outputs, solver fidelity, localization quality, or physical safety. A coordinate declaration cannot prove that a downstream solver consumed the transform.
