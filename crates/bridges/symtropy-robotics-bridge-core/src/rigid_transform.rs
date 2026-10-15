// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Rigid data-coordinate transforms with explicit source and target frames.

use std::fmt;

pub const QUATERNION_UNIT_NORM_TOLERANCE_V1: f64 = 1.0e-6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RigidTransformError {
    InvalidFrameIdentity,
    NonFiniteTranslation,
    NonFiniteQuaternion,
    QuaternionNormOutOfTolerance,
    NonFinitePoint,
    NonFiniteDirection,
    PointSourceFrameMismatch { expected_source: String, actual_frame: String },
    DirectionSourceFrameMismatch { expected_source: String, actual_frame: String },
    NonContiguousTransforms { left_source: String, right_target: String },
}

impl fmt::Display for RigidTransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrameIdentity => {
                f.write_str("frame identities must be non-empty and trimmed")
            }
            Self::NonFiniteTranslation => f.write_str("transform translation must be finite"),
            Self::NonFiniteQuaternion => f.write_str("transform quaternion must be finite"),
            Self::QuaternionNormOutOfTolerance => write!(
                f,
                "quaternion norm must be within {QUATERNION_UNIT_NORM_TOLERANCE_V1} of 1"
            ),
            Self::NonFinitePoint => f.write_str("point coordinates must be finite"),
            Self::NonFiniteDirection => f.write_str("direction components must be finite"),
            Self::PointSourceFrameMismatch { expected_source, actual_frame } => write!(
                f, "point frame {actual_frame:?} does not match source {expected_source:?}"
            ),
            Self::DirectionSourceFrameMismatch { expected_source, actual_frame } => write!(
                f, "direction frame {actual_frame:?} does not match source {expected_source:?}"
            ),
            Self::NonContiguousTransforms { left_source, right_target } => write!(
                f, "transform composition is not contiguous: {left_source:?} != {right_target:?}"
            ),
        }
    }
}

impl std::error::Error for RigidTransformError {}

#[derive(Debug, Clone, PartialEq)]
pub struct PointInFrameV1 {
    frame_id: String,
    coordinates_m: [f64; 3],
}

impl PointInFrameV1 {
    pub fn new(
        frame_id: impl Into<String>,
        coordinates_m: [f64; 3],
    ) -> Result<Self, RigidTransformError> {
        let frame_id = frame_id.into();
        validate_frame_identity(&frame_id)?;
        if !coordinates_m.iter().all(|value| value.is_finite()) {
            return Err(RigidTransformError::NonFinitePoint);
        }
        Ok(Self { frame_id, coordinates_m })
    }

    pub fn frame_id(&self) -> &str {
        &self.frame_id
    }

    pub fn coordinates_m(&self) -> [f64; 3] {
        self.coordinates_m
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DirectionInFrameV1 {
    frame_id: String,
    components: [f64; 3],
}

impl DirectionInFrameV1 {
    pub fn new(
        frame_id: impl Into<String>,
        components: [f64; 3],
    ) -> Result<Self, RigidTransformError> {
        let frame_id = frame_id.into();
        validate_frame_identity(&frame_id)?;
        if !components.iter().all(|value| value.is_finite()) {
            return Err(RigidTransformError::NonFiniteDirection);
        }
        Ok(Self { frame_id, components })
    }

    pub fn frame_id(&self) -> &str {
        &self.frame_id
    }

    pub fn components(&self) -> [f64; 3] {
        self.components
    }
}

/// Data transform from source to target:
/// p_target = R_target_source * p_source + t_target_source.
/// Translation is metres; rotations are active, right-handed XYZW quaternions.
/// Input quaternion norm must be within 1e-6 of one; accepted values are
/// normalized and their sign is canonicalized in W, X, Y, Z priority order.
#[derive(Debug, Clone, PartialEq)]
pub struct RigidTransformV1 {
    target_frame_id: String,
    source_frame_id: String,
    translation_target_m: [f64; 3],
    rotation_target_source_xyzw: [f64; 4],
}

impl RigidTransformV1 {
    pub fn new(
        target_frame_id: impl Into<String>,
        source_frame_id: impl Into<String>,
        translation_target_m: [f64; 3],
        rotation_target_source_xyzw: [f64; 4],
    ) -> Result<Self, RigidTransformError> {
        let target_frame_id = target_frame_id.into();
        let source_frame_id = source_frame_id.into();
        validate_frame_identity(&target_frame_id)?;
        validate_frame_identity(&source_frame_id)?;
        if !translation_target_m.iter().all(|value| value.is_finite()) {
            return Err(RigidTransformError::NonFiniteTranslation);
        }

        let mut rotation = rotation_target_source_xyzw;
        if !rotation.iter().all(|value| value.is_finite()) {
            return Err(RigidTransformError::NonFiniteQuaternion);
        }
        let norm = rotation.iter().map(|value| value * value).sum::<f64>().sqrt();
        if !norm.is_finite()
            || norm == 0.0
            || (norm - 1.0).abs() > QUATERNION_UNIT_NORM_TOLERANCE_V1
        {
            return Err(RigidTransformError::QuaternionNormOutOfTolerance);
        }
        for component in &mut rotation {
            *component /= norm;
        }
        canonicalize_quaternion_sign(&mut rotation);
        Ok(Self {
            target_frame_id,
            source_frame_id,
            translation_target_m,
            rotation_target_source_xyzw: rotation,
        })
    }

    pub fn identity(frame_id: impl Into<String>) -> Result<Self, RigidTransformError> {
        let frame_id = frame_id.into();
        Self::new(frame_id.clone(), frame_id, [0.0; 3], [0.0, 0.0, 0.0, 1.0])
    }

    pub fn target_frame_id(&self) -> &str {
        &self.target_frame_id
    }

    pub fn source_frame_id(&self) -> &str {
        &self.source_frame_id
    }

    pub fn translation_target_m(&self) -> [f64; 3] {
        self.translation_target_m
    }

    pub fn rotation_target_source_xyzw(&self) -> [f64; 4] {
        self.rotation_target_source_xyzw
    }

    /// Points receive rotation and translation.
    pub fn transform_point(
        &self,
        point: &PointInFrameV1,
    ) -> Result<PointInFrameV1, RigidTransformError> {
        if point.frame_id != self.source_frame_id {
            return Err(RigidTransformError::PointSourceFrameMismatch {
                expected_source: self.source_frame_id.clone(),
                actual_frame: point.frame_id.clone(),
            });
        }
        let rotated = rotate_xyzw(self.rotation_target_source_xyzw, point.coordinates_m);
        PointInFrameV1::new(
            self.target_frame_id.clone(),
            add(rotated, self.translation_target_m),
        )
    }

    /// Free vectors receive rotation only; the transform translation is omitted.
    pub fn transform_direction(
        &self,
        direction: &DirectionInFrameV1,
    ) -> Result<DirectionInFrameV1, RigidTransformError> {
        if direction.frame_id != self.source_frame_id {
            return Err(RigidTransformError::DirectionSourceFrameMismatch {
                expected_source: self.source_frame_id.clone(),
                actual_frame: direction.frame_id.clone(),
            });
        }
        DirectionInFrameV1::new(
            self.target_frame_id.clone(),
            rotate_xyzw(self.rotation_target_source_xyzw, direction.components),
        )
    }

    /// Compose target <- intermediate with intermediate <- source.
    /// Returns target <- source and rejects non-contiguous frame identifiers.
    pub fn compose(&self, next: &Self) -> Result<Self, RigidTransformError> {
        if self.source_frame_id != next.target_frame_id {
            return Err(RigidTransformError::NonContiguousTransforms {
                left_source: self.source_frame_id.clone(),
                right_target: next.target_frame_id.clone(),
            });
        }
        let translation = add(
            rotate_xyzw(self.rotation_target_source_xyzw, next.translation_target_m),
            self.translation_target_m,
        );
        let rotation = multiply_xyzw(
            self.rotation_target_source_xyzw,
            next.rotation_target_source_xyzw,
        );
        Self::new(
            self.target_frame_id.clone(),
            next.source_frame_id.clone(),
            translation,
            rotation,
        )
    }

    /// Return the inverse data-coordinate map, reversing source and target frames.
    pub fn inverse(&self) -> Result<Self, RigidTransformError> {
        let rotation = conjugate_xyzw(self.rotation_target_source_xyzw);
        let translation = scale(rotate_xyzw(rotation, self.translation_target_m), -1.0);
        Self::new(
            self.source_frame_id.clone(),
            self.target_frame_id.clone(),
            translation,
            rotation,
        )
    }
}

/// A frame-pose representation: the pose of frame_id expressed in reference_frame_id.
///
/// Its stored pose is the reference-from-frame transform. tf2-style frame-pose
/// representation and a data transform with the same frame labels are inverses;
/// call to_data_transform() at the integration boundary to obtain reference -> frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FramePoseV1 {
    frame_id: String,
    reference_frame_id: String,
    reference_from_frame: RigidTransformV1,
}

impl FramePoseV1 {
    pub fn new(
        frame_id: impl Into<String>,
        reference_frame_id: impl Into<String>,
        translation_reference_m: [f64; 3],
        rotation_reference_frame_xyzw: [f64; 4],
    ) -> Result<Self, RigidTransformError> {
        let frame_id = frame_id.into();
        let reference_frame_id = reference_frame_id.into();
        let reference_from_frame = RigidTransformV1::new(
            reference_frame_id.clone(),
            frame_id.clone(),
            translation_reference_m,
            rotation_reference_frame_xyzw,
        )?;
        Ok(Self {
            frame_id,
            reference_frame_id,
            reference_from_frame,
        })
    }

    pub fn frame_id(&self) -> &str {
        &self.frame_id
    }

    pub fn reference_frame_id(&self) -> &str {
        &self.reference_frame_id
    }

    pub fn translation_reference_m(&self) -> [f64; 3] {
        self.reference_from_frame.translation_target_m()
    }

    pub fn rotation_reference_frame_xyzw(&self) -> [f64; 4] {
        self.reference_from_frame.rotation_target_source_xyzw()
    }

    /// Convert the frame-pose representation to a reference -> frame data map.
    pub fn to_data_transform(&self) -> Result<RigidTransformV1, RigidTransformError> {
        self.reference_from_frame.inverse()
    }
}

fn validate_frame_identity(frame_id: &str) -> Result<(), RigidTransformError> {
    if frame_id.trim().is_empty() || frame_id.trim() != frame_id {
        return Err(RigidTransformError::InvalidFrameIdentity);
    }
    Ok(())
}

fn canonicalize_quaternion_sign(q: &mut [f64; 4]) {
    let first = [q[3], q[0], q[1], q[2]]
        .into_iter()
        .find(|value| *value != 0.0);
    if first.is_some_and(|value| value < 0.0) {
        for component in q {
            *component = -*component;
        }
    }
}

fn multiply_xyzw(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let [x1, y1, z1, w1] = a;
    let [x2, y2, z2, w2] = b;
    [
        w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
        w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
        w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
        w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
    ]
}

fn conjugate_xyzw(q: [f64; 4]) -> [f64; 4] {
    [-q[0], -q[1], -q[2], q[3]]
}

fn rotate_xyzw(q: [f64; 4], vector: [f64; 3]) -> [f64; 3] {
    let axis = [q[0], q[1], q[2]];
    let twice_cross = scale(cross(axis, vector), 2.0);
    add(vector, add(scale(twice_cross, q[3]), cross(axis, twice_cross)))
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(v: [f64; 3], factor: f64) -> [f64; 3] {
    [v[0] * factor, v[1] * factor, v[2] * factor]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close3(actual: [f64; 3], expected: [f64; 3]) {
        for index in 0..3 {
            assert!(
                (actual[index] - expected[index]).abs() < 1.0e-12,
                "actual {:?}, expected {:?}",
                actual,
                expected
            );
        }
    }

    fn close_transform(actual: &RigidTransformV1, expected: &RigidTransformV1) {
        assert_eq!(actual.target_frame_id(), expected.target_frame_id());
        assert_eq!(actual.source_frame_id(), expected.source_frame_id());
        close3(actual.translation_target_m(), expected.translation_target_m());
        for (left, right) in actual
            .rotation_target_source_xyzw()
            .into_iter()
            .zip(expected.rotation_target_source_xyzw())
        {
            assert!((left - right).abs() < 1.0e-12);
        }
    }

    fn fixtures() -> (RigidTransformV1, RigidTransformV1) {
        let s = (0.5_f64).sqrt();
        (
            RigidTransformV1::new("A", "B", [1.0, 0.0, 0.0], [0.0, 0.0, s, s]).unwrap(),
            RigidTransformV1::new("B", "C", [0.0, 2.0, 0.0], [s, 0.0, 0.0, s]).unwrap(),
        )
    }

    #[test]
    fn non_commuting_golden_composition_matches_xyzw_fixture() {
        let (a_b, b_c) = fixtures();
        let a_c = a_b.compose(&b_c).unwrap();
        let expected = RigidTransformV1::new(
            "A",
            "C",
            [-1.0, 0.0, 0.0],
            [0.5, 0.5, 0.5, 0.5],
        )
        .unwrap();
        close_transform(&a_c, &expected);

        let point = PointInFrameV1::new("C", [1.0, 0.0, 0.0]).unwrap();
        close3(a_c.transform_point(&point).unwrap().coordinates_m(), [-1.0, 1.0, 0.0]);
        let direction = DirectionInFrameV1::new("C", [1.0, 0.0, 0.0]).unwrap();
        close3(
            a_c.transform_direction(&direction).unwrap().components(),
            [0.0, 1.0, 0.0],
        );

        let wrong = multiply_xyzw(
            b_c.rotation_target_source_xyzw(),
            a_b.rotation_target_source_xyzw(),
        );
        close3([wrong[0], wrong[1], wrong[2]], [0.5, -0.5, 0.5]);
        assert!((wrong[3] - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn identity_composes_on_both_sides() {
        let (a_b, _) = fixtures();
        let left = RigidTransformV1::identity("A").unwrap().compose(&a_b).unwrap();
        let right = a_b.compose(&RigidTransformV1::identity("B").unwrap()).unwrap();
        close_transform(&left, &a_b);
        close_transform(&right, &a_b);
    }

    #[test]
    fn inverse_is_reverse_data_transform_and_round_trips() {
        let (a_b, _) = fixtures();
        let b_a = a_b.inverse().unwrap();
        let point_b = PointInFrameV1::new("B", [0.5, -1.0, 2.0]).unwrap();
        let point_a = a_b.transform_point(&point_b).unwrap();
        close3(b_a.transform_point(&point_a).unwrap().coordinates_m(), point_b.coordinates_m());
        let identity_a = RigidTransformV1::identity("A").unwrap();
        let identity_b = RigidTransformV1::identity("B").unwrap();
        close_transform(&a_b.compose(&b_a).unwrap(), &identity_a);
        close_transform(&b_a.compose(&a_b).unwrap(), &identity_b);
    }

    #[test]
    fn frame_pose_and_data_transform_are_inverse_formulations() {
        let s = (0.5_f64).sqrt();
        let pose_b_in_a =
            FramePoseV1::new("B", "A", [1.0, 0.0, 0.0], [0.0, 0.0, s, s]).unwrap();
        let a_from_b =
            RigidTransformV1::new("A", "B", [1.0, 0.0, 0.0], [0.0, 0.0, s, s]).unwrap();
        let b_from_a = pose_b_in_a.to_data_transform().unwrap();

        assert_eq!(pose_b_in_a.frame_id(), "B");
        assert_eq!(pose_b_in_a.reference_frame_id(), "A");
        assert_eq!(b_from_a.target_frame_id(), "B");
        assert_eq!(b_from_a.source_frame_id(), "A");
        close_transform(
            &b_from_a.compose(&a_from_b).unwrap(),
            &RigidTransformV1::identity("B").unwrap(),
        );
        close_transform(
            &a_from_b.compose(&b_from_a).unwrap(),
            &RigidTransformV1::identity("A").unwrap(),
        );
    }

    #[test]
    fn three_edge_composition_and_non_contiguous_rejection() {
        let (a_b, b_c) = fixtures();
        let c_d = RigidTransformV1::new("C", "D", [0.0, 0.0, 1.0], [0.0, 0.0, 0.0, 1.0])
            .unwrap();
        let a_d = a_b.compose(&b_c).unwrap().compose(&c_d).unwrap();
        assert_eq!(a_d.target_frame_id(), "A");
        assert_eq!(a_d.source_frame_id(), "D");
        close3(a_d.translation_target_m(), [0.0, 0.0, 0.0]);

        let x_c = RigidTransformV1::new("X", "C", [0.0; 3], [0.0, 0.0, 0.0, 1.0]).unwrap();
        assert!(matches!(
            a_b.compose(&x_c),
            Err(RigidTransformError::NonContiguousTransforms { .. })
        ));
    }

    #[test]
    fn source_frame_mismatch_is_rejected_for_points_and_directions() {
        let (a_b, _) = fixtures();
        let point = PointInFrameV1::new("C", [1.0, 2.0, 3.0]).unwrap();
        let vector = DirectionInFrameV1::new("C", [1.0, 2.0, 3.0]).unwrap();
        assert!(matches!(
            a_b.transform_point(&point),
            Err(RigidTransformError::PointSourceFrameMismatch { .. })
        ));
        assert!(matches!(
            a_b.transform_direction(&vector),
            Err(RigidTransformError::DirectionSourceFrameMismatch { .. })
        ));
    }

    #[test]
    fn quaternion_sign_is_canonicalized() {
        let positive = RigidTransformV1::new("A", "B", [0.0; 3], [0.0, 0.0, 0.0, 1.0])
            .unwrap();
        let negative = RigidTransformV1::new("A", "B", [0.0; 3], [-0.0, -0.0, -0.0, -1.0])
            .unwrap();
        assert_eq!(positive, negative);
    }

    #[test]
    fn non_finite_invalid_and_near_zero_inputs_are_rejected() {
        assert!(matches!(
            RigidTransformV1::new("A", "B", [f64::INFINITY, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]),
            Err(RigidTransformError::NonFiniteTranslation)
        ));
        assert!(matches!(
            RigidTransformV1::new("A", "B", [0.0; 3], [f64::NAN, 0.0, 0.0, 1.0]),
            Err(RigidTransformError::NonFiniteQuaternion)
        ));
        for q in [[0.0; 4], [1.0e-30, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.01]] {
            assert!(matches!(
                RigidTransformV1::new("A", "B", [0.0; 3], q),
                Err(RigidTransformError::QuaternionNormOutOfTolerance)
            ));
        }
        assert!(matches!(
            PointInFrameV1::new("A", [f64::NAN, 0.0, 0.0]),
            Err(RigidTransformError::NonFinitePoint)
        ));
        assert!(matches!(
            DirectionInFrameV1::new("A", [0.0, f64::INFINITY, 0.0]),
            Err(RigidTransformError::NonFiniteDirection)
        ));
        assert!(matches!(
            RigidTransformV1::new(" A", "B", [0.0; 3], [0.0, 0.0, 0.0, 1.0]),
            Err(RigidTransformError::InvalidFrameIdentity)
        ));
    }
}
