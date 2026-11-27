#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

pub const RECEIPT_SCHEMA: &str = "luminous.spacecraft-runtime-pose-admission.receipt.v0.1";
pub const TANGENT_CONVENTION: &str = "BODY_LOCAL_TRANSLATION_XYZ_ROTATION_VECTOR_XYZ";
pub const COVARIANCE_UNITS: &str = "m,m,m,rad,rad,rad";
pub const ADMISSION_LEVEL_BPS: u16 = 9_500;
pub const MIN_PROFILE_SAMPLES: u64 = 100;
pub const MIN_VISIBLE_FRACTION: f64 = 0.05;
pub const MIN_CORRESPONDENCES: u32 = 6;
pub const MAX_GEOMETRY_CONDITION: f64 = 100_000_000.0;
const SYMMETRY_TOLERANCE: f64 = 1.0e-10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionDisposition {
    CandidateStatisticalRegion,
    AbstainUncalibrated,
    AbstainUnobservable,
    OutOfScope,
    InvalidInput,
    InvalidCovariance,
    DuplicateSample,
}

impl AdmissionDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CandidateStatisticalRegion => "CANDIDATE_STATISTICAL_REGION",
            Self::AbstainUncalibrated => "ABSTAIN_UNCALIBRATED",
            Self::AbstainUnobservable => "ABSTAIN_UNOBSERVABLE",
            Self::OutOfScope => "OUT_OF_SCOPE",
            Self::InvalidInput => "INVALID_INPUT",
            Self::InvalidCovariance => "INVALID_COVARIANCE",
            Self::DuplicateSample => "DUPLICATE_SAMPLE",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationProfile {
    pub profile_id: String,
    pub profile_revision: String,
    pub profile_sha256: String,
    pub profile_state: String,
    pub dataset_id: String,
    pub dataset_revision: String,
    pub model_revision: String,
    pub model_sha256: String,
    pub sensor_mode: String,
    pub frame_convention: String,
    pub camera_calibration_revision: String,
    pub camera_calibration_sha256: String,
    pub target_population_id: String,
    pub target_population_sha256: String,
    pub calibration_split_sha256: String,
    pub heldout_split_sha256: String,
    pub calibration_method: String,
    pub nominal_level_bps: u16,
    pub sample_count: u64,
    pub coverage_lower_ci_ppm: u32,
    pub required_coverage_lower_ci_ppm: u32,
    pub source_bytes_authenticated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimePosePrediction {
    pub case_id: String,
    pub sample_id: String,
    pub frame_id: String,
    pub target_id: String,
    pub trajectory_id: String,
    pub target_population_id: String,
    pub target_population_sha256: String,
    pub dataset_id: String,
    pub dataset_revision: String,
    pub dataset_sha256: String,
    pub sensor_mode: String,
    pub frame_convention: String,
    pub quaternion_order: String,
    pub tangent_convention: String,
    pub translation_unit: String,
    pub rotation_unit: String,
    pub model_revision: String,
    pub model_sha256: String,
    pub model_config_sha256: String,
    pub camera_calibration_revision: String,
    pub camera_calibration_sha256: String,
    pub frame_bundle_revision: String,
    pub frame_bundle_sha256: String,
    pub frame_bundle_state: String,
    pub frame_source_bytes_authenticated: bool,
    pub bundle_sample_id: String,
    pub bundle_frame_id: String,
    pub bundle_target_id: String,
    pub bundle_dataset_revision: String,
    pub bundle_camera_calibration_revision: String,
    pub nominal_level_bps: u16,
    pub support_mode: String,
    pub visible_fraction: f64,
    pub correspondence_count: u32,
    pub geometry_condition: f64,
    pub ood_flag: bool,
    pub symmetry_ambiguous: bool,
    pub predicted_translation_m: [f64; 3],
    /// Explicit XYZW source order.
    pub predicted_quaternion_xyzw: [f64; 4],
    /// Row-major 6x6 covariance in the declared body-local SE(3) tangent frame.
    pub covariance: [f64; 36],
    pub profile: CalibrationProfile,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdmissionResult {
    pub case_id: String,
    pub sample_id: String,
    pub frame_id: String,
    pub target_id: String,
    pub trajectory_id: String,
    pub dataset_id: String,
    pub dataset_revision: String,
    pub dataset_sha256: String,
    pub model_revision: String,
    pub model_sha256: String,
    pub model_config_sha256: String,
    pub camera_calibration_revision: String,
    pub camera_calibration_sha256: String,
    pub frame_bundle_revision: String,
    pub frame_bundle_sha256: String,
    pub frame_source_bytes_authenticated: bool,
    pub profile_id: String,
    pub profile_revision: String,
    pub profile_sha256: String,
    pub profile_source_bytes_authenticated: bool,
    pub sensor_mode: String,
    pub frame_convention: String,
    pub tangent_convention: String,
    pub covariance_units: String,
    pub nominal_level_bps: u16,
    pub profile_sample_count: u64,
    pub profile_coverage_lower_ci_ppm: u32,
    pub predicted_translation_m: [f64; 3],
    pub predicted_quaternion_xyzw: [f64; 4],
    pub covariance_row_major: [f64; 36],
    pub disposition: AdmissionDisposition,
    pub reason_codes: Vec<String>,
    pub candidate_only: bool,
    pub statistical_region_only: bool,
    pub deterministic_safety_bound: bool,
    pub physical_calibration_validated: bool,
    pub source_bytes_authenticated: bool,
    pub maneuver_authority_granted: bool,
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn finite(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn quaternion_is_unit_xyzw(quaternion: [f64; 4]) -> bool {
    if !finite(&quaternion) {
        return false;
    }
    let norm_squared = quaternion.iter().map(|value| value * value).sum::<f64>();
    norm_squared.is_finite()
        && norm_squared > 1.0e-24
        && (norm_squared.sqrt() - 1.0).abs() <= 1.0e-6
}

fn covariance_is_spd(matrix: &[f64; 36]) -> bool {
    if !finite(matrix) {
        return false;
    }

    let scale = matrix
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if !scale.is_finite() || scale <= 0.0 {
        return false;
    }

    for row in 0..6 {
        for column in 0..6 {
            if (matrix[row * 6 + column] - matrix[column * 6 + row]).abs()
                > SYMMETRY_TOLERANCE * scale
            {
                return false;
            }
        }
    }

    let mut lower = [[0.0_f64; 6]; 6];
    for row in 0..6 {
        for column in 0..=row {
            let mut sum = matrix[row * 6 + column];
            for (left, right) in lower[row][..column]
                .iter()
                .zip(lower[column][..column].iter())
            {
                sum -= left * right;
            }
            if row == column {
                if !sum.is_finite() || sum <= f64::EPSILON * scale * 16.0 {
                    return false;
                }
                lower[row][column] = sum.sqrt();
            } else {
                let pivot = lower[column][column];
                if pivot <= 0.0 || !pivot.is_finite() {
                    return false;
                }
                lower[row][column] = sum / pivot;
            }
        }
    }
    true
}

fn profile_shape_valid(profile: &CalibrationProfile) -> bool {
    [
        profile.profile_id.as_str(),
        profile.profile_revision.as_str(),
        profile.dataset_id.as_str(),
        profile.dataset_revision.as_str(),
        profile.model_revision.as_str(),
        profile.sensor_mode.as_str(),
        profile.frame_convention.as_str(),
        profile.camera_calibration_revision.as_str(),
        profile.target_population_id.as_str(),
        profile.calibration_method.as_str(),
    ]
    .iter()
    .all(|value| valid_text(value))
        && valid_sha256(&profile.profile_sha256)
        && valid_sha256(&profile.model_sha256)
        && valid_sha256(&profile.camera_calibration_sha256)
        && valid_sha256(&profile.target_population_sha256)
        && valid_sha256(&profile.calibration_split_sha256)
        && valid_sha256(&profile.heldout_split_sha256)
        && profile.coverage_lower_ci_ppm <= 1_000_000
        && profile.required_coverage_lower_ci_ppm <= 1_000_000
        && profile.nominal_level_bps <= 10_000
}

fn prediction_shape_valid(input: &RuntimePosePrediction) -> bool {
    [
        input.case_id.as_str(),
        input.sample_id.as_str(),
        input.frame_id.as_str(),
        input.target_id.as_str(),
        input.trajectory_id.as_str(),
        input.target_population_id.as_str(),
        input.dataset_id.as_str(),
        input.dataset_revision.as_str(),
        input.model_revision.as_str(),
        input.camera_calibration_revision.as_str(),
        input.frame_bundle_revision.as_str(),
        input.bundle_sample_id.as_str(),
        input.bundle_frame_id.as_str(),
        input.bundle_target_id.as_str(),
        input.bundle_dataset_revision.as_str(),
        input.bundle_camera_calibration_revision.as_str(),
        input.support_mode.as_str(),
    ]
    .iter()
    .all(|value| valid_text(value))
        && valid_sha256(&input.dataset_sha256)
        && valid_sha256(&input.target_population_sha256)
        && valid_sha256(&input.model_sha256)
        && valid_sha256(&input.model_config_sha256)
        && valid_sha256(&input.camera_calibration_sha256)
        && valid_sha256(&input.frame_bundle_sha256)
        && finite(&[
            input.predicted_translation_m[0],
            input.predicted_translation_m[1],
            input.predicted_translation_m[2],
            input.visible_fraction,
            input.geometry_condition,
        ])
        && (0.0..=1.0).contains(&input.visible_fraction)
        && input.geometry_condition >= 0.0
        && input.nominal_level_bps <= 10_000
}

fn base_result(
    input: &RuntimePosePrediction,
    disposition: AdmissionDisposition,
    reason: &str,
) -> AdmissionResult {
    AdmissionResult {
        case_id: input.case_id.clone(),
        sample_id: input.sample_id.clone(),
        frame_id: input.frame_id.clone(),
        target_id: input.target_id.clone(),
        trajectory_id: input.trajectory_id.clone(),
        dataset_id: input.dataset_id.clone(),
        dataset_revision: input.dataset_revision.clone(),
        dataset_sha256: input.dataset_sha256.clone(),
        model_revision: input.model_revision.clone(),
        model_sha256: input.model_sha256.clone(),
        model_config_sha256: input.model_config_sha256.clone(),
        camera_calibration_revision: input.camera_calibration_revision.clone(),
        camera_calibration_sha256: input.camera_calibration_sha256.clone(),
        frame_bundle_revision: input.frame_bundle_revision.clone(),
        frame_bundle_sha256: input.frame_bundle_sha256.clone(),
        frame_source_bytes_authenticated: input.frame_source_bytes_authenticated,
        profile_id: input.profile.profile_id.clone(),
        profile_revision: input.profile.profile_revision.clone(),
        profile_sha256: input.profile.profile_sha256.clone(),
        profile_source_bytes_authenticated: input.profile.source_bytes_authenticated,
        sensor_mode: input.sensor_mode.clone(),
        frame_convention: input.frame_convention.clone(),
        tangent_convention: input.tangent_convention.clone(),
        covariance_units: COVARIANCE_UNITS.to_owned(),
        nominal_level_bps: input.nominal_level_bps,
        profile_sample_count: input.profile.sample_count,
        profile_coverage_lower_ci_ppm: input.profile.coverage_lower_ci_ppm,
        predicted_translation_m: input.predicted_translation_m,
        predicted_quaternion_xyzw: input.predicted_quaternion_xyzw,
        covariance_row_major: input.covariance,
        disposition,
        reason_codes: if reason.is_empty() {
            Vec::new()
        } else {
            vec![reason.to_owned()]
        },
        candidate_only: disposition == AdmissionDisposition::CandidateStatisticalRegion,
        statistical_region_only: disposition == AdmissionDisposition::CandidateStatisticalRegion,
        deterministic_safety_bound: false,
        physical_calibration_validated: false,
        source_bytes_authenticated: input.frame_source_bytes_authenticated
            && input.profile.source_bytes_authenticated,
        maneuver_authority_granted: false,
    }
}

/// Performs inference-time scope/profile admission without accepting or using
/// ground truth. A positive result is only a statistical region candidate for
/// an upstream planner; it is not a deterministic safety bound.
pub fn evaluate(input: &RuntimePosePrediction) -> AdmissionResult {
    if !prediction_shape_valid(input) || !profile_shape_valid(&input.profile) {
        return base_result(
            input,
            AdmissionDisposition::InvalidInput,
            "PREDICTION_OR_PROFILE_SHAPE_INVALID",
        );
    }

    if input.sensor_mode != input.profile.sensor_mode
        || input.frame_convention != "WORLD_RIGHT_HANDED_METRES"
        || input.frame_convention != input.profile.frame_convention
        || input.quaternion_order != "XYZW"
        || input.tangent_convention != TANGENT_CONVENTION
        || input.translation_unit != "metre"
        || input.rotation_unit != "radian"
    {
        return base_result(
            input,
            AdmissionDisposition::OutOfScope,
            "FRAME_UNIT_OR_SENSOR_CONVENTION_MISMATCH",
        );
    }

    if input.bundle_sample_id != input.sample_id
        || input.bundle_frame_id != input.frame_id
        || input.bundle_target_id != input.target_id
        || input.bundle_dataset_revision != input.dataset_revision
        || input.bundle_camera_calibration_revision != input.camera_calibration_revision
    {
        return base_result(
            input,
            AdmissionDisposition::OutOfScope,
            "FRAME_BUNDLE_SCOPE_MISMATCH",
        );
    }

    if input.frame_bundle_state != "SYNCHRONIZED" {
        return base_result(
            input,
            AdmissionDisposition::AbstainUncalibrated,
            "FRAME_BUNDLE_NOT_SYNCHRONIZED",
        );
    }

    let profile = &input.profile;
    if profile.dataset_id != input.dataset_id
        || profile.dataset_revision != input.dataset_revision
        || profile.model_revision != input.model_revision
        || profile.model_sha256 != input.model_sha256
        || profile.sensor_mode != input.sensor_mode
        || profile.frame_convention != input.frame_convention
        || profile.camera_calibration_revision != input.camera_calibration_revision
        || profile.camera_calibration_sha256 != input.camera_calibration_sha256
        || profile.target_population_id != input.target_population_id
        || profile.target_population_sha256 != input.target_population_sha256
    {
        return base_result(
            input,
            AdmissionDisposition::OutOfScope,
            "CALIBRATION_PROFILE_SCOPE_MISMATCH",
        );
    }

    if profile.profile_state != "CURRENT"
        || !profile.source_bytes_authenticated
        || profile.calibration_method != "SPLIT_CONFORMAL_SE3-v1"
        || profile.nominal_level_bps != input.nominal_level_bps
        || profile.nominal_level_bps != ADMISSION_LEVEL_BPS
        || profile.sample_count < MIN_PROFILE_SAMPLES
        || profile.coverage_lower_ci_ppm < profile.required_coverage_lower_ci_ppm
    {
        return base_result(
            input,
            AdmissionDisposition::AbstainUncalibrated,
            "CALIBRATION_PROFILE_NOT_CURRENT_OR_SUFFICIENT",
        );
    }

    if !quaternion_is_unit_xyzw(input.predicted_quaternion_xyzw) {
        return base_result(
            input,
            AdmissionDisposition::InvalidInput,
            "PREDICTED_QUATERNION_INVALID",
        );
    }

    if !covariance_is_spd(&input.covariance) {
        return base_result(
            input,
            AdmissionDisposition::InvalidCovariance,
            "COVARIANCE_NOT_FINITE_SYMMETRIC_POSITIVE_DEFINITE",
        );
    }

    let direct = input.support_mode == "DIRECT_REGRESSION";
    if !matches!(
        input.support_mode.as_str(),
        "KEYPOINT_PNP" | "DENSE_PNP" | "EVENT_KEYPOINT_PNP" | "DIRECT_REGRESSION"
    ) {
        return base_result(
            input,
            AdmissionDisposition::OutOfScope,
            "POSE_SUPPORT_MODE_UNSUPPORTED",
        );
    }

    if input.ood_flag
        || input.symmetry_ambiguous
        || input.visible_fraction < MIN_VISIBLE_FRACTION
        || (!direct
            && (input.correspondence_count < MIN_CORRESPONDENCES
                || input.geometry_condition <= 0.0
                || input.geometry_condition > MAX_GEOMETRY_CONDITION))
    {
        return base_result(
            input,
            AdmissionDisposition::AbstainUnobservable,
            "POSE_OBSERVABILITY_OR_DOMAIN_GATE_FAILED",
        );
    }

    base_result(input, AdmissionDisposition::CandidateStatisticalRegion, "")
}

pub fn evaluate_manifest(inputs: &[RuntimePosePrediction]) -> Vec<AdmissionResult> {
    let mut seen_samples = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for input in inputs {
        if !seen_samples.insert(input.sample_id.as_str()) {
            duplicates.insert(input.sample_id.as_str());
        }
    }

    inputs
        .iter()
        .map(|input| {
            if duplicates.contains(input.sample_id.as_str()) {
                base_result(
                    input,
                    AdmissionDisposition::DuplicateSample,
                    "SAMPLE_ID_DUPLICATED",
                )
            } else {
                evaluate(input)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        AdmissionDisposition, CalibrationProfile, RuntimePosePrediction, evaluate,
        evaluate_manifest,
    };

    fn profile() -> CalibrationProfile {
        CalibrationProfile {
            profile_id: "cal-profile-1".to_owned(),
            profile_revision: "r1".to_owned(),
            profile_sha256: "d".repeat(64),
            profile_state: "CURRENT".to_owned(),
            dataset_id: "SpaceSense-Bench".to_owned(),
            dataset_revision: "f8c4ee9c7c0f9a4037f4977f3ddc4e0610a8fb8e".to_owned(),
            model_revision: "model-r1".to_owned(),
            model_sha256: "b".repeat(64),
            sensor_mode: "RGB".to_owned(),
            frame_convention: "WORLD_RIGHT_HANDED_METRES".to_owned(),
            camera_calibration_revision: "camera-r1".to_owned(),
            camera_calibration_sha256: "c".repeat(64),
            target_population_id: "spacecraft-family-v1".to_owned(),
            target_population_sha256: "e".repeat(64),
            calibration_split_sha256: "f".repeat(64),
            heldout_split_sha256: "1".repeat(64),
            calibration_method: "SPLIT_CONFORMAL_SE3-v1".to_owned(),
            nominal_level_bps: 9_500,
            sample_count: 1_000,
            coverage_lower_ci_ppm: 925_000,
            required_coverage_lower_ci_ppm: 900_000,
            source_bytes_authenticated: true,
        }
    }

    fn input() -> RuntimePosePrediction {
        let mut covariance = [0.0_f64; 36];
        for index in 0..6 {
            covariance[index * 6 + index] = 1.0;
        }
        RuntimePosePrediction {
            case_id: "RPA-001".to_owned(),
            sample_id: "ACE/approach/0001".to_owned(),
            frame_id: "frame-0001".to_owned(),
            target_id: "ACE".to_owned(),
            trajectory_id: "approach".to_owned(),
            target_population_id: "spacecraft-family-v1".to_owned(),
            target_population_sha256: "e".repeat(64),
            dataset_id: "SpaceSense-Bench".to_owned(),
            dataset_revision: "f8c4ee9c7c0f9a4037f4977f3ddc4e0610a8fb8e".to_owned(),
            dataset_sha256: "a".repeat(64),
            sensor_mode: "RGB".to_owned(),
            frame_convention: "WORLD_RIGHT_HANDED_METRES".to_owned(),
            quaternion_order: "XYZW".to_owned(),
            tangent_convention: super::TANGENT_CONVENTION.to_owned(),
            translation_unit: "metre".to_owned(),
            rotation_unit: "radian".to_owned(),
            model_revision: "model-r1".to_owned(),
            model_sha256: "b".repeat(64),
            model_config_sha256: "2".repeat(64),
            camera_calibration_revision: "camera-r1".to_owned(),
            camera_calibration_sha256: "c".repeat(64),
            frame_bundle_revision: "bundle-r1".to_owned(),
            frame_bundle_sha256: "3".repeat(64),
            frame_bundle_state: "SYNCHRONIZED".to_owned(),
            frame_source_bytes_authenticated: false,
            bundle_sample_id: "ACE/approach/0001".to_owned(),
            bundle_frame_id: "frame-0001".to_owned(),
            bundle_target_id: "ACE".to_owned(),
            bundle_dataset_revision: "f8c4ee9c7c0f9a4037f4977f3ddc4e0610a8fb8e".to_owned(),
            bundle_camera_calibration_revision: "camera-r1".to_owned(),
            nominal_level_bps: 9_500,
            support_mode: "KEYPOINT_PNP".to_owned(),
            visible_fraction: 0.75,
            correspondence_count: 16,
            geometry_condition: 20.0,
            ood_flag: false,
            symmetry_ambiguous: false,
            predicted_translation_m: [0.0; 3],
            predicted_quaternion_xyzw: [0.0, 0.0, 0.0, 1.0],
            covariance,
            profile: profile(),
        }
    }

    #[test]
    fn matched_scope_yields_candidate_only_statistical_region() {
        let result = evaluate(&input());
        assert_eq!(
            result.disposition,
            AdmissionDisposition::CandidateStatisticalRegion
        );
        assert!(result.candidate_only);
        assert!(result.statistical_region_only);
        assert!(!result.deterministic_safety_bound);
        assert!(!result.physical_calibration_validated);
        assert!(!result.source_bytes_authenticated);
        assert!(!result.maneuver_authority_granted);
    }

    #[test]
    fn runtime_prediction_type_has_no_ground_truth_fields() {
        let result = evaluate(&input());
        let receipt = format!("{result:?}");
        assert!(!receipt.contains("ground_truth"));
        assert!(!receipt.contains("mahalanobis_squared"));
        assert!(!receipt.contains("false_confident_95"));
    }

    #[test]
    fn profile_model_mismatch_is_out_of_scope() {
        let mut value = input();
        value.profile.model_revision = "other-model".to_owned();
        assert_eq!(
            evaluate(&value).disposition,
            AdmissionDisposition::OutOfScope
        );
    }

    #[test]
    fn profile_calibration_mismatch_is_out_of_scope() {
        let mut value = input();
        value.profile.camera_calibration_sha256 = "9".repeat(64);
        assert_eq!(
            evaluate(&value).disposition,
            AdmissionDisposition::OutOfScope
        );
    }

    #[test]
    fn stale_profile_abstains() {
        let mut value = input();
        value.profile.profile_state = "STALE".to_owned();
        assert_eq!(
            evaluate(&value).disposition,
            AdmissionDisposition::AbstainUncalibrated
        );
    }

    #[test]
    fn weak_profile_coverage_abstains() {
        let mut value = input();
        value.profile.coverage_lower_ci_ppm = 899_999;
        assert_eq!(
            evaluate(&value).disposition,
            AdmissionDisposition::AbstainUncalibrated
        );
    }

    #[test]
    fn insufficient_calibration_sample_count_abstains() {
        let mut value = input();
        value.profile.sample_count = 99;
        assert_eq!(
            evaluate(&value).disposition,
            AdmissionDisposition::AbstainUncalibrated
        );
    }

    #[test]
    fn unsynchronized_frame_bundle_abstains() {
        let mut value = input();
        value.frame_bundle_state = "UNSYNCHRONIZED".to_owned();
        assert_eq!(
            evaluate(&value).disposition,
            AdmissionDisposition::AbstainUncalibrated
        );
    }

    #[test]
    fn bundle_identity_mismatch_is_out_of_scope() {
        let mut value = input();
        value.bundle_frame_id = "frame-other".to_owned();
        assert_eq!(
            evaluate(&value).disposition,
            AdmissionDisposition::OutOfScope
        );
    }

    #[test]
    fn covariance_must_be_positive_definite() {
        let mut value = input();
        value.covariance[0] = -1.0;
        assert_eq!(
            evaluate(&value).disposition,
            AdmissionDisposition::InvalidCovariance
        );
    }

    #[test]
    fn ood_pose_abstains_even_with_a_current_profile() {
        let mut value = input();
        value.ood_flag = true;
        assert_eq!(
            evaluate(&value).disposition,
            AdmissionDisposition::AbstainUnobservable
        );
    }

    #[test]
    fn direct_regression_does_not_require_correspondences() {
        let mut value = input();
        value.support_mode = "DIRECT_REGRESSION".to_owned();
        value.correspondence_count = 0;
        value.geometry_condition = 0.0;
        assert_eq!(
            evaluate(&value).disposition,
            AdmissionDisposition::CandidateStatisticalRegion
        );
    }

    #[test]
    fn duplicate_runtime_sample_is_rejected() {
        let first = input();
        let mut second = input();
        second.case_id = "RPA-002".to_owned();
        let results = evaluate_manifest(&[first, second]);
        assert!(
            results
                .iter()
                .all(|value| { value.disposition == AdmissionDisposition::DuplicateSample })
        );
    }
}
