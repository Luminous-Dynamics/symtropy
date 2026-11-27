#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

pub const RECEIPT_SCHEMA: &str = "luminous.spacecraft-pose-annotation-contract.receipt.v0.1";
pub const PIXEL_SCALE: f64 = 1_000_000.0;
pub const QUATERNION_NORM_TOLERANCE: f64 = 1.0e-6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationDisposition {
    GeometricallyConsistent,
    AnnotationIncomplete,
    Inconsistent,
    OutOfScope,
    InvalidInput,
    DuplicateSample,
    DuplicateSplitSample,
    DuplicateKeypoint,
}

impl AnnotationDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GeometricallyConsistent => "GEOMETRICALLY_CONSISTENT",
            Self::AnnotationIncomplete => "ANNOTATION_INCOMPLETE",
            Self::Inconsistent => "INCONSISTENT",
            Self::OutOfScope => "OUT_OF_SCOPE",
            Self::InvalidInput => "INVALID_INPUT",
            Self::DuplicateSample => "DUPLICATE_SAMPLE",
            Self::DuplicateSplitSample => "DUPLICATE_SPLIT_SAMPLE",
            Self::DuplicateKeypoint => "DUPLICATE_KEYPOINT",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoseInput {
    pub case_id: String,
    pub sample_id: String,
    pub split: String,
    pub annotation_release: String,
    pub bbox3d_present: bool,
    pub keypoint_id: String,
    pub keypoint_count: u16,
    pub pose_semantics: String,
    pub quaternion_order: String,
    pub camera_axis_convention: String,
    pub projection_axis_convention: String,
    pub length_unit: String,
    pub image_width: u32,
    pub image_height: u32,
    pub fx: f64,
    pub fy: f64,
    pub cx: f64,
    pub cy: f64,
    pub quaternion: [f64; 4],
    /// Camera position in world coordinates, metres.
    pub camera_translation: [f64; 3],
    /// One spacecraft-template keypoint in world/model coordinates, metres.
    pub keypoint_world: [f64; 3],
    pub observed_pixel: [f64; 2],
    pub reprojection_tolerance_px: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoseResult {
    pub case_id: String,
    pub sample_id: String,
    pub split: String,
    pub disposition: AnnotationDisposition,
    pub keypoint_id: String,
    pub quaternion_norm_error_scaled: Option<i64>,
    pub projected_u_micro_px: Option<i64>,
    pub projected_v_micro_px: Option<i64>,
    pub reprojection_error_micro_px: Option<u64>,
    pub reason_codes: Vec<String>,
    pub pose_contract_version: String,
    pub source_annotation_release: String,
    pub source_bytes_authenticated: bool,
    pub pose_ground_truth_physically_validated: bool,
}

fn finite(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn rotate(q: [f64; 4], vector: [f64; 3]) -> [f64; 3] {
    let [x, y, z, w] = q;
    let u = [x, y, z];
    let cross_uv = [
        u[1] * vector[2] - u[2] * vector[1],
        u[2] * vector[0] - u[0] * vector[2],
        u[0] * vector[1] - u[1] * vector[0],
    ];
    let cross_uuv = [
        u[1] * cross_uv[2] - u[2] * cross_uv[1],
        u[2] * cross_uv[0] - u[0] * cross_uv[2],
        u[0] * cross_uv[1] - u[1] * cross_uv[0],
    ];
    [
        vector[0] + 2.0 * (w * cross_uv[0] + cross_uuv[0]),
        vector[1] + 2.0 * (w * cross_uv[1] + cross_uuv[1]),
        vector[2] + 2.0 * (w * cross_uv[2] + cross_uuv[2]),
    ]
}

fn canonical_xyzw(input: &PoseInput) -> Option<[f64; 4]> {
    match input.quaternion_order.as_str() {
        "XYZW" => Some(input.quaternion),
        "WXYZ" => Some([
            input.quaternion[1],
            input.quaternion[2],
            input.quaternion[3],
            input.quaternion[0],
        ]),
        _ => None,
    }
}

fn projected_pixel(input: &PoseInput, quaternion: [f64; 4]) -> Option<[f64; 2]> {
    let camera_to_world = quaternion;
    let world_to_camera = [
        -camera_to_world[0],
        -camera_to_world[1],
        -camera_to_world[2],
        camera_to_world[3],
    ];

    let relative_world = [
        input.keypoint_world[0] - input.camera_translation[0],
        input.keypoint_world[1] - input.camera_translation[1],
        input.keypoint_world[2] - input.camera_translation[2],
    ];
    let camera_blender = rotate(world_to_camera, relative_world);

    // Blender camera: +X right, +Y up, -Z forward.
    // OpenCV camera: +X right, +Y down, +Z forward.
    let camera_opencv = [camera_blender[0], -camera_blender[1], -camera_blender[2]];
    let depth = camera_opencv[2];
    if !depth.is_finite() || depth <= 1.0e-9 {
        return None;
    }

    let u = input.fx * camera_opencv[0] / depth + input.cx;
    let v = input.fy * camera_opencv[1] / depth + input.cy;
    if u.is_finite() && v.is_finite() {
        Some([u, v])
    } else {
        None
    }
}

fn result(
    input: &PoseInput,
    disposition: AnnotationDisposition,
    reason: &str,
    norm_error: Option<i64>,
    projection: Option<[f64; 2]>,
    reprojection: Option<u64>,
) -> PoseResult {
    PoseResult {
        case_id: input.case_id.clone(),
        sample_id: input.sample_id.clone(),
        split: input.split.clone(),
        disposition,
        keypoint_id: input.keypoint_id.clone(),
        quaternion_norm_error_scaled: norm_error,
        projected_u_micro_px: projection.and_then(|point| scaled_i64(point[0])),
        projected_v_micro_px: projection.and_then(|point| scaled_i64(point[1])),
        reprojection_error_micro_px: reprojection,
        reason_codes: if reason.is_empty() {
            Vec::new()
        } else {
            vec![reason.to_owned()]
        },
        pose_contract_version: "spacecraft-pose-annotation-contract.v0.1".to_owned(),
        source_annotation_release: input.annotation_release.clone(),
        source_bytes_authenticated: false,
        pose_ground_truth_physically_validated: false,
    }
}

fn scaled_i64(value: f64) -> Option<i64> {
    let scaled = (value * PIXEL_SCALE).round();
    // i64::MAX as f64 rounds up to 2^63, so the upper edge must be
    // exclusive. The exact lower edge -2^63 is representable and inclusive.
    let lower_inclusive = i64::MIN as f64;
    let upper_exclusive = -lower_inclusive;
    if scaled.is_finite() && (lower_inclusive..upper_exclusive).contains(&scaled) {
        Some(scaled as i64)
    } else {
        None
    }
}

pub fn evaluate(input: &PoseInput) -> PoseResult {
    if input.case_id.trim().is_empty()
        || input.sample_id.trim().is_empty()
        || input.keypoint_id.trim().is_empty()
        || input.keypoint_count == 0
    {
        return result(
            input,
            AnnotationDisposition::InvalidInput,
            "IDENTITY_OR_KEYPOINT_CARDINALITY_INVALID",
            None,
            None,
            None,
        );
    }

    if input.annotation_release != "CORRECTION" || !input.bbox3d_present {
        return result(
            input,
            AnnotationDisposition::AnnotationIncomplete,
            "CORRECTED_BBOX3D_ANNOTATION_REQUIRED",
            None,
            None,
            None,
        );
    }

    if input.split != "train" && input.split != "val" && input.split != "test" {
        return result(
            input,
            AnnotationDisposition::InvalidInput,
            "UNKNOWN_SPLIT",
            None,
            None,
            None,
        );
    }

    if input.pose_semantics != "CAMERA_TO_WORLD"
        || input.camera_axis_convention != "BLENDER_NEG_Z_FORWARD_POS_Y_UP"
        || input.projection_axis_convention != "OPENCV_POS_Z_FORWARD_Y_DOWN"
        || input.length_unit != "metre"
        || (input.quaternion_order != "XYZW" && input.quaternion_order != "WXYZ")
    {
        return result(
            input,
            AnnotationDisposition::OutOfScope,
            "UNSUPPORTED_POSE_OR_COORDINATE_CONVENTION",
            None,
            None,
            None,
        );
    }

    if input.image_width == 0
        || input.image_height == 0
        || !finite(&[
            input.fx,
            input.fy,
            input.cx,
            input.cy,
            input.reprojection_tolerance_px,
            input.observed_pixel[0],
            input.observed_pixel[1],
            input.camera_translation[0],
            input.camera_translation[1],
            input.camera_translation[2],
            input.keypoint_world[0],
            input.keypoint_world[1],
            input.keypoint_world[2],
            input.quaternion[0],
            input.quaternion[1],
            input.quaternion[2],
            input.quaternion[3],
        ])
        || input.fx <= 0.0
        || input.fy <= 0.0
        || input.reprojection_tolerance_px < 0.0
    {
        return result(
            input,
            AnnotationDisposition::InvalidInput,
            "CAMERA_OR_GEOMETRY_NUMERIC_INPUT_INVALID",
            None,
            None,
            None,
        );
    }

    let Some(mut quaternion) = canonical_xyzw(input) else {
        return result(
            input,
            AnnotationDisposition::OutOfScope,
            "UNSUPPORTED_QUATERNION_ORDER",
            None,
            None,
            None,
        );
    };
    let norm_squared = quaternion.iter().map(|value| value * value).sum::<f64>();
    let norm = norm_squared.sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 {
        return result(
            input,
            AnnotationDisposition::InvalidInput,
            "QUATERNION_NORM_INVALID",
            None,
            None,
            None,
        );
    }

    let norm_error = (norm - 1.0).abs();
    let norm_error_scaled = scaled_i64(norm_error * 1.0e9);
    if norm_error > QUATERNION_NORM_TOLERANCE {
        return result(
            input,
            AnnotationDisposition::InvalidInput,
            "QUATERNION_NOT_UNIT_WITHIN_TOLERANCE",
            norm_error_scaled,
            None,
            None,
        );
    }

    for value in &mut quaternion {
        *value /= norm;
    }

    let Some(projected) = projected_pixel(input, quaternion) else {
        return result(
            input,
            AnnotationDisposition::Inconsistent,
            "KEYPOINT_NOT_IN_FRONT_OF_CAMERA_OR_PROJECTION_INVALID",
            norm_error_scaled,
            None,
            None,
        );
    };
    if scaled_i64(projected[0]).is_none() || scaled_i64(projected[1]).is_none() {
        return result(
            input,
            AnnotationDisposition::InvalidInput,
            "PROJECTION_OUTPUT_OVERFLOW",
            norm_error_scaled,
            None,
            None,
        );
    }

    let du = projected[0] - input.observed_pixel[0];
    let dv = projected[1] - input.observed_pixel[1];
    let error = du.hypot(dv);
    let error_scaled = scaled_i64(error * PIXEL_SCALE).and_then(|value| u64::try_from(value).ok());
    if error_scaled.is_none() {
        return result(
            input,
            AnnotationDisposition::InvalidInput,
            "REPROJECTION_OUTPUT_OVERFLOW",
            norm_error_scaled,
            Some(projected),
            None,
        );
    }

    if !error.is_finite() || error > input.reprojection_tolerance_px {
        return result(
            input,
            AnnotationDisposition::Inconsistent,
            "ANNOTATED_KEYPOINT_EXCEEDS_REPROJECTION_TOLERANCE",
            norm_error_scaled,
            Some(projected),
            error_scaled,
        );
    }

    result(
        input,
        AnnotationDisposition::GeometricallyConsistent,
        "",
        norm_error_scaled,
        Some(projected),
        error_scaled,
    )
}

/// Enforces sample/keypoint uniqueness across an evaluation manifest. Repeated
/// sample IDs across different splits poison both affected records; duplicate
/// keypoint IDs within a sample also poison the sample's records.
pub fn evaluate_manifest(inputs: &[PoseInput]) -> Vec<PoseResult> {
    let mut sample_splits: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut sample_keypoints: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut duplicate_splits = BTreeSet::new();
    let mut duplicate_keypoints = BTreeSet::new();

    for input in inputs {
        let splits = sample_splits.entry(&input.sample_id).or_default();
        splits.insert(&input.split);
        let keypoints = sample_keypoints.entry(&input.sample_id).or_default();
        if !keypoints.insert(&input.keypoint_id) {
            duplicate_keypoints.insert(input.sample_id.as_str());
        }
    }

    for (sample_id, splits) in &sample_splits {
        if splits.len() > 1 {
            duplicate_splits.insert(*sample_id);
        }
    }

    inputs
        .iter()
        .map(|input| {
            if duplicate_splits.contains(input.sample_id.as_str()) {
                result(
                    input,
                    AnnotationDisposition::DuplicateSplitSample,
                    "SAMPLE_ID_PRESENT_IN_MULTIPLE_SPLITS",
                    None,
                    None,
                    None,
                )
            } else if duplicate_keypoints.contains(input.sample_id.as_str()) {
                result(
                    input,
                    AnnotationDisposition::DuplicateKeypoint,
                    "KEYPOINT_ID_DUPLICATED_WITHIN_SAMPLE",
                    None,
                    None,
                    None,
                )
            } else {
                evaluate(input)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{AnnotationDisposition, PoseInput, evaluate, evaluate_manifest};

    fn input() -> PoseInput {
        PoseInput {
            case_id: "pose-1".to_owned(),
            sample_id: "sample-1".to_owned(),
            split: "test".to_owned(),
            annotation_release: "CORRECTION".to_owned(),
            bbox3d_present: true,
            keypoint_id: "kp-0".to_owned(),
            keypoint_count: 16,
            pose_semantics: "CAMERA_TO_WORLD".to_owned(),
            quaternion_order: "XYZW".to_owned(),
            camera_axis_convention: "BLENDER_NEG_Z_FORWARD_POS_Y_UP".to_owned(),
            projection_axis_convention: "OPENCV_POS_Z_FORWARD_Y_DOWN".to_owned(),
            length_unit: "metre".to_owned(),
            image_width: 1920,
            image_height: 1200,
            fx: 1000.0,
            fy: 1000.0,
            cx: 960.0,
            cy: 600.0,
            quaternion: [0.0, 0.0, 0.0, 1.0],
            camera_translation: [0.0, 0.0, 2.0],
            keypoint_world: [0.0, 0.0, 0.0],
            observed_pixel: [960.0, 600.0],
            reprojection_tolerance_px: 0.01,
        }
    }

    #[test]
    fn identity_camera_pose_projects_center_keypoint() {
        let result = evaluate(&input());
        assert_eq!(
            result.disposition,
            AnnotationDisposition::GeometricallyConsistent
        );
        assert_eq!(result.projected_u_micro_px, Some(960_000_000));
        assert_eq!(result.projected_v_micro_px, Some(600_000_000));
    }

    #[test]
    fn camera_roll_quaternion_projects_with_inverse_camera_pose() {
        let mut value = input();
        let half = std::f64::consts::FRAC_1_SQRT_2;
        value.quaternion = [0.0, 0.0, half, half];
        value.keypoint_world = [0.1, 0.0, 0.0];
        value.observed_pixel = [960.0, 650.0];
        assert_eq!(
            evaluate(&value).disposition,
            AnnotationDisposition::GeometricallyConsistent
        );
    }

    #[test]
    fn q_and_negative_q_are_rotation_equivalent() {
        let positive = evaluate(&input());
        let mut negative_input = input();
        negative_input.quaternion = [0.0, 0.0, 0.0, -1.0];
        let negative = evaluate(&negative_input);
        assert_eq!(positive.projected_u_micro_px, negative.projected_u_micro_px);
        assert_eq!(positive.projected_v_micro_px, negative.projected_v_micro_px);
    }

    #[test]
    fn wxyz_quaternion_order_is_explicitly_supported() {
        let mut value = input();
        value.quaternion_order = "WXYZ".to_owned();
        value.quaternion = [1.0, 0.0, 0.0, 0.0];
        assert_eq!(
            evaluate(&value).disposition,
            AnnotationDisposition::GeometricallyConsistent
        );
    }

    #[test]
    fn initial_release_without_bbox3d_is_incomplete() {
        let mut value = input();
        value.annotation_release = "INITIAL".to_owned();
        value.bbox3d_present = false;
        assert_eq!(
            evaluate(&value).disposition,
            AnnotationDisposition::AnnotationIncomplete
        );
    }

    #[test]
    fn corrected_release_without_bbox3d_is_incomplete() {
        let mut value = input();
        value.bbox3d_present = false;
        assert_eq!(
            evaluate(&value).disposition,
            AnnotationDisposition::AnnotationIncomplete
        );
    }

    #[test]
    fn nonunit_or_zero_quaternion_is_invalid() {
        let mut value = input();
        value.quaternion = [0.0, 0.0, 0.0, 2.0];
        assert_eq!(
            evaluate(&value).disposition,
            AnnotationDisposition::InvalidInput
        );
        let mut zero = input();
        zero.quaternion = [0.0; 4];
        assert_eq!(
            evaluate(&zero).disposition,
            AnnotationDisposition::InvalidInput
        );
    }

    #[test]
    fn keypoint_behind_camera_is_inconsistent() {
        let mut value = input();
        value.keypoint_world = [0.0, 0.0, 3.0];
        assert_eq!(
            evaluate(&value).disposition,
            AnnotationDisposition::Inconsistent
        );
    }

    #[test]
    fn excessive_reprojection_error_is_inconsistent() {
        let mut value = input();
        value.observed_pixel = [100.0, 100.0];
        assert_eq!(
            evaluate(&value).disposition,
            AnnotationDisposition::Inconsistent
        );
    }

    #[test]
    fn unsupported_coordinate_semantics_are_out_of_scope() {
        let mut value = input();
        value.pose_semantics = "WORLD_TO_CAMERA".to_owned();
        assert_eq!(
            evaluate(&value).disposition,
            AnnotationDisposition::OutOfScope
        );
        let mut axes = input();
        axes.camera_axis_convention = "OPENCV".to_owned();
        assert_eq!(
            evaluate(&axes).disposition,
            AnnotationDisposition::OutOfScope
        );
    }

    #[test]
    fn invalid_intrinsics_and_cardinality_are_rejected() {
        let mut zero_fx = input();
        zero_fx.fx = 0.0;
        assert_eq!(
            evaluate(&zero_fx).disposition,
            AnnotationDisposition::InvalidInput
        );
        let mut zero_width = input();
        zero_width.image_width = 0;
        assert_eq!(
            evaluate(&zero_width).disposition,
            AnnotationDisposition::InvalidInput
        );
        let mut no_keypoint = input();
        no_keypoint.keypoint_count = 0;
        assert_eq!(
            evaluate(&no_keypoint).disposition,
            AnnotationDisposition::InvalidInput
        );
    }

    #[test]
    fn fixed_point_i64_edges_are_checked_without_saturation() {
        let lower = (i64::MIN as f64) / PIXEL_SCALE;
        let upper = -((i64::MIN as f64) / PIXEL_SCALE);
        assert_eq!(super::scaled_i64(lower), Some(i64::MIN));
        assert_eq!(super::scaled_i64(upper), None);
    }

    #[test]
    fn metre_millimetre_mismatch_is_out_of_scope() {
        let mut value = input();
        value.length_unit = "millimetre".to_owned();
        assert_eq!(
            evaluate(&value).disposition,
            AnnotationDisposition::OutOfScope
        );
    }

    #[test]
    fn manifest_rejects_sample_leakage_across_splits() {
        let first = input();
        let mut second = input();
        second.case_id = "pose-2".to_owned();
        second.split = "train".to_owned();
        let results = evaluate_manifest(&[first, second]);
        assert!(
            results.iter().all(|result| {
                result.disposition == AnnotationDisposition::DuplicateSplitSample
            })
        );
    }

    #[test]
    fn manifest_rejects_duplicate_keypoint_ids() {
        let first = input();
        let mut second = input();
        second.case_id = "pose-2".to_owned();
        let results = evaluate_manifest(&[first, second]);
        assert!(
            results
                .iter()
                .all(|result| { result.disposition == AnnotationDisposition::DuplicateKeypoint })
        );
    }
}
