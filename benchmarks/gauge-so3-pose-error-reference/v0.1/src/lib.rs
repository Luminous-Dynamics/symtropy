//! Frame-declared rigid-pose discrepancy on SE(3).
//!
//! Translation and rotation errors are reported separately. This reference
//! accepts right-handed, Hamilton, world-from-body poses and converts either
//! WXYZ or XYZW component storage into a canonical WXYZ representation.
//! It is an analytical metric reference, not a GAUGE dataset parser.

const UNIT_QUATERNION_TOLERANCE: f64 = 1.0e-6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuaternionOrder {
    Wxyz,
    Xyzw,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameHandedness {
    RightHanded,
    LeftHanded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoseDirection {
    WorldFromBody,
    BodyFromWorld,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuaternionAlgebra {
    Hamilton,
    Jpl,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameDeclaration {
    pub world_frame_id: String,
    pub body_frame_id: String,
    pub handedness: FrameHandedness,
    pub pose_direction: PoseDirection,
    pub quaternion_algebra: QuaternionAlgebra,
    pub quaternion_order: QuaternionOrder,
}

impl FrameDeclaration {
    fn validate(&self) -> Result<(), PoseError> {
        if self.world_frame_id.trim().is_empty() || self.body_frame_id.trim().is_empty() {
            return Err(PoseError::EmptyFrameId);
        }
        if self.handedness != FrameHandedness::RightHanded {
            return Err(PoseError::UnsupportedHandedness);
        }
        if self.pose_direction != PoseDirection::WorldFromBody {
            return Err(PoseError::UnsupportedPoseDirection);
        }
        if self.quaternion_algebra != QuaternionAlgebra::Hamilton {
            return Err(PoseError::UnsupportedQuaternionAlgebra);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct UnitQuaternionWxyz {
    w: f64,
    x: f64,
    y: f64,
    z: f64,
}

impl UnitQuaternionWxyz {
    fn from_components(components: [f64; 4], order: QuaternionOrder) -> Result<Self, PoseError> {
        let [w, x, y, z] = match order {
            QuaternionOrder::Wxyz => components,
            QuaternionOrder::Xyzw => [components[3], components[0], components[1], components[2]],
        };

        if ![w, x, y, z].iter().all(|value| value.is_finite()) {
            return Err(PoseError::InvalidQuaternion);
        }

        let norm = (w * w + x * x + y * y + z * z).sqrt();
        if !norm.is_finite()
            || norm <= f64::EPSILON
            || (norm - 1.0).abs() > UNIT_QUATERNION_TOLERANCE
        {
            return Err(PoseError::InvalidQuaternion);
        }

        Ok(Self {
            w: w / norm,
            x: x / norm,
            y: y / norm,
            z: z / norm,
        })
    }

    fn conjugate(self) -> Self {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    fn multiply(self, rhs: Self) -> Self {
        Self {
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        }
    }

    #[cfg(test)]
    fn rotate_vector(self, vector: [f64; 3]) -> [f64; 3] {
        let pure = Self {
            w: 0.0,
            x: vector[0],
            y: vector[1],
            z: vector[2],
        };
        let rotated = self.multiply(pure).multiply(self.conjugate());
        [rotated.x, rotated.y, rotated.z]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RigidPose {
    pub translation_world_m: [f64; 3],
    rotation_world_from_body: UnitQuaternionWxyz,
    pub frame: FrameDeclaration,
}

impl RigidPose {
    /// Construct a pose after explicitly declaring frame and quaternion semantics.
    pub fn try_new(
        translation_world_m: [f64; 3],
        rotation_components: [f64; 4],
        frame: FrameDeclaration,
    ) -> Result<Self, PoseError> {
        frame.validate()?;
        if !translation_world_m.iter().all(|value| value.is_finite()) {
            return Err(PoseError::InvalidTranslation);
        }
        let rotation_world_from_body =
            UnitQuaternionWxyz::from_components(rotation_components, frame.quaternion_order)?;

        Ok(Self {
            translation_world_m,
            rotation_world_from_body,
            frame,
        })
    }

    /// Return the normalized quaternion in canonical scalar-first WXYZ order.
    pub fn rotation_wxyz(&self) -> [f64; 4] {
        let q = self.rotation_world_from_body;
        [q.w, q.x, q.y, q.z]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseDifference {
    /// Component-wise simulated-minus-real translation, in metres.
    pub translation_delta_m: [f64; 3],
    /// Euclidean translation discrepancy, in metres.
    pub translation_error_m: f64,
    /// Geodesic rotation discrepancy on SO(3), in radians and in [0, pi].
    pub rotation_error_rad: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoseError {
    EmptyFrameId,
    UnsupportedHandedness,
    UnsupportedPoseDirection,
    UnsupportedQuaternionAlgebra,
    InvalidTranslation,
    InvalidQuaternion,
    FrameMismatch,
}

/// Compare two poses expressed in the same declared world/body frames.
///
/// Quaternion storage order may differ because each input is converted to
/// canonical WXYZ first. Frame IDs, handedness, transform direction and
/// quaternion algebra must remain compatible.
pub fn compare_poses(real: &RigidPose, simulated: &RigidPose) -> Result<PoseDifference, PoseError> {
    if real.frame.world_frame_id != simulated.frame.world_frame_id
        || real.frame.body_frame_id != simulated.frame.body_frame_id
        || real.frame.handedness != simulated.frame.handedness
        || real.frame.pose_direction != simulated.frame.pose_direction
        || real.frame.quaternion_algebra != simulated.frame.quaternion_algebra
    {
        return Err(PoseError::FrameMismatch);
    }

    let translation_delta_m = [
        simulated.translation_world_m[0] - real.translation_world_m[0],
        simulated.translation_world_m[1] - real.translation_world_m[1],
        simulated.translation_world_m[2] - real.translation_world_m[2],
    ];
    let translation_error_m = (translation_delta_m[0] * translation_delta_m[0]
        + translation_delta_m[1] * translation_delta_m[1]
        + translation_delta_m[2] * translation_delta_m[2])
        .sqrt();

    // q_delta = conjugate(q_real) * q_sim. The absolute scalar and vector
    // norm make q and -q represent the same physical orientation.
    let relative = real
        .rotation_world_from_body
        .conjugate()
        .multiply(simulated.rotation_world_from_body);
    let relative_vector_norm =
        (relative.x * relative.x + relative.y * relative.y + relative.z * relative.z).sqrt();
    let rotation_error_rad = 2.0 * relative_vector_norm.atan2(relative.w.abs());

    Ok(PoseDifference {
        translation_delta_m,
        translation_error_m,
        rotation_error_rad,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{PI, SQRT_2};

    fn frame(world: &str, body: &str, order: QuaternionOrder) -> FrameDeclaration {
        FrameDeclaration {
            world_frame_id: world.to_owned(),
            body_frame_id: body.to_owned(),
            handedness: FrameHandedness::RightHanded,
            pose_direction: PoseDirection::WorldFromBody,
            quaternion_algebra: QuaternionAlgebra::Hamilton,
            quaternion_order: order,
        }
    }

    fn pose(translation: [f64; 3], rotation: [f64; 4], order: QuaternionOrder) -> RigidPose {
        RigidPose::try_new(translation, rotation, frame("world", "body", order))
            .expect("valid test pose")
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:.16e} != expected {expected:.16e} within {tolerance:.3e}"
        );
    }

    #[test]
    fn identical_poses_have_zero_discrepancy() {
        let real = pose(
            [1.0, -2.0, 3.0],
            [1.0, 0.0, 0.0, 0.0],
            QuaternionOrder::Wxyz,
        );
        let simulated = real.clone();
        let difference = compare_poses(&real, &simulated).expect("same frames");
        close(difference.translation_error_m, 0.0, 0.0);
        close(difference.rotation_error_rad, 0.0, 0.0);
    }

    #[test]
    fn quaternion_sign_flip_is_the_same_rotation() {
        let real = pose([0.0; 3], [0.5, 0.5, 0.5, 0.5], QuaternionOrder::Wxyz);
        let simulated = pose([0.0; 3], [-0.5, -0.5, -0.5, -0.5], QuaternionOrder::Wxyz);
        let difference = compare_poses(&real, &simulated).expect("same frames");
        close(difference.rotation_error_rad, 0.0, 1.0e-12);
    }

    #[test]
    fn xyzw_storage_maps_to_the_same_canonical_rotation() {
        let real = pose(
            [0.0; 3],
            [SQRT_2 / 2.0, 0.0, 0.0, SQRT_2 / 2.0],
            QuaternionOrder::Wxyz,
        );
        let simulated = pose(
            [0.0; 3],
            [0.0, 0.0, SQRT_2 / 2.0, SQRT_2 / 2.0],
            QuaternionOrder::Xyzw,
        );
        let difference = compare_poses(&real, &simulated).expect("same frames");
        close(difference.rotation_error_rad, 0.0, 1.0e-12);
    }

    #[test]
    fn quarter_turn_has_known_geodesic_error() {
        let real = pose([0.0; 3], [1.0, 0.0, 0.0, 0.0], QuaternionOrder::Wxyz);
        let half = (PI / 4.0).cos();
        let simulated = pose([0.0; 3], [half, 0.0, 0.0, half], QuaternionOrder::Wxyz);
        let difference = compare_poses(&real, &simulated).expect("same frames");
        close(difference.rotation_error_rad, PI / 2.0, 1.0e-12);
    }

    #[test]
    fn pi_boundary_uses_the_shortest_rotation_not_euler_wraparound() {
        let half = (179.0_f64.to_radians() / 2.0).cos();
        let side = (179.0_f64.to_radians() / 2.0).sin();
        let real = pose([0.0; 3], [half, 0.0, 0.0, side], QuaternionOrder::Wxyz);
        let simulated = pose([0.0; 3], [half, 0.0, 0.0, -side], QuaternionOrder::Wxyz);
        let difference = compare_poses(&real, &simulated).expect("same frames");
        close(difference.rotation_error_rad, 2.0_f64.to_radians(), 1.0e-12);
    }

    #[test]
    fn exact_half_turn_is_finite_and_equals_pi() {
        let real = pose([0.0; 3], [1.0, 0.0, 0.0, 0.0], QuaternionOrder::Wxyz);
        let simulated = pose([0.0; 3], [0.0, 0.0, 1.0, 0.0], QuaternionOrder::Wxyz);
        let difference = compare_poses(&real, &simulated).expect("same frames");
        close(difference.rotation_error_rad, PI, 1.0e-12);
    }

    #[test]
    fn translation_error_is_euclidean_and_separate_from_rotation() {
        let real = pose([0.0; 3], [1.0, 0.0, 0.0, 0.0], QuaternionOrder::Wxyz);
        let simulated = pose([1.0, 2.0, 2.0], [1.0, 0.0, 0.0, 0.0], QuaternionOrder::Wxyz);
        let difference = compare_poses(&real, &simulated).expect("same frames");
        assert_eq!(difference.translation_delta_m, [1.0, 2.0, 2.0]);
        close(difference.translation_error_m, 3.0, 1.0e-12);
        close(difference.rotation_error_rad, 0.0, 0.0);
    }

    #[test]
    fn mismatched_world_or_body_frame_is_rejected() {
        let real = pose([0.0; 3], [1.0, 0.0, 0.0, 0.0], QuaternionOrder::Wxyz);
        let mut other_frame = frame("another_world", "body", QuaternionOrder::Wxyz);
        other_frame.world_frame_id = "another_world".to_owned();
        let simulated = RigidPose::try_new([0.0; 3], [1.0, 0.0, 0.0, 0.0], other_frame)
            .expect("valid but different frame");
        assert_eq!(
            compare_poses(&real, &simulated),
            Err(PoseError::FrameMismatch)
        );
    }

    #[test]
    fn non_unit_and_non_finite_quaternions_are_rejected() {
        let non_unit = RigidPose::try_new(
            [0.0; 3],
            [2.0, 0.0, 0.0, 0.0],
            frame("world", "body", QuaternionOrder::Wxyz),
        );
        assert_eq!(non_unit, Err(PoseError::InvalidQuaternion));

        let non_finite = RigidPose::try_new(
            [0.0; 3],
            [f64::NAN, 0.0, 0.0, 0.0],
            frame("world", "body", QuaternionOrder::Wxyz),
        );
        assert_eq!(non_finite, Err(PoseError::InvalidQuaternion));
    }

    #[test]
    fn unsupported_frame_conventions_fail_closed() {
        let mut left_handed = frame("world", "body", QuaternionOrder::Wxyz);
        left_handed.handedness = FrameHandedness::LeftHanded;
        assert_eq!(
            RigidPose::try_new([0.0; 3], [1.0, 0.0, 0.0, 0.0], left_handed),
            Err(PoseError::UnsupportedHandedness)
        );

        let mut jpl = frame("world", "body", QuaternionOrder::Wxyz);
        jpl.quaternion_algebra = QuaternionAlgebra::Jpl;
        assert_eq!(
            RigidPose::try_new([0.0; 3], [1.0, 0.0, 0.0, 0.0], jpl),
            Err(PoseError::UnsupportedQuaternionAlgebra)
        );
    }

    #[test]
    fn common_world_rigid_transform_preserves_pose_difference() {
        let real = pose(
            [0.3, -0.2, 1.1],
            [0.8, 0.2, -0.4, 0.4],
            QuaternionOrder::Wxyz,
        );
        let simulated = pose(
            [0.4, -0.1, 1.0],
            [0.7, 0.1, -0.5, 0.5],
            QuaternionOrder::Wxyz,
        );
        let original = compare_poses(&real, &simulated).expect("same frames");

        let global_half = (0.63_f64 / 2.0).cos();
        let global_side = (0.63_f64 / 2.0).sin();
        let global_rotation = UnitQuaternionWxyz::from_components(
            [global_half, 0.0, 0.0, global_side],
            QuaternionOrder::Wxyz,
        )
        .expect("unit global rotation");
        let global_translation = [-4.0, 2.0, 0.5];

        let transform = |source: &RigidPose| {
            let rotated = global_rotation.rotate_vector(source.translation_world_m);
            let translation = [
                global_translation[0] + rotated[0],
                global_translation[1] + rotated[1],
                global_translation[2] + rotated[2],
            ];
            let rotation = global_rotation.multiply(source.rotation_world_from_body);
            RigidPose::try_new(
                translation,
                [rotation.w, rotation.x, rotation.y, rotation.z],
                frame(
                    "world_after_common_transform",
                    "body",
                    QuaternionOrder::Wxyz,
                ),
            )
            .expect("transformed pose")
        };

        let transformed_real = transform(&real);
        let transformed_simulated = transform(&simulated);
        let transformed = compare_poses(&transformed_real, &transformed_simulated)
            .expect("transformed frames match");

        close(
            transformed.translation_error_m,
            original.translation_error_m,
            1.0e-12,
        );
        close(
            transformed.rotation_error_rad,
            original.rotation_error_rad,
            1.0e-12,
        );
    }

    #[test]
    fn frame_identifiers_must_not_be_blank() {
        let invalid = RigidPose::try_new(
            [0.0; 3],
            [1.0, 0.0, 0.0, 0.0],
            frame(" ", "body", QuaternionOrder::Wxyz),
        );
        assert_eq!(invalid, Err(PoseError::EmptyFrameId));
    }
}
