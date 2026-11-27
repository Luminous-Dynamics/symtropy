#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

pub const RECEIPT_SCHEMA: &str = "luminous.se3-pose-trustworthiness.receipt.v0.1";
pub const COVARIANCE_ORDER: &str = "BODY_LOCAL_TRANSLATION_XYZ_ROTATION_VECTOR_XYZ";
pub const COVARIANCE_UNIT: &str = "m,m,m,rad,rad,rad";
pub const MIN_VISIBLE_FRACTION: f64 = 0.05;
pub const MIN_CORRESPONDENCES: u32 = 6;
pub const MAX_GEOMETRY_CONDITION: f64 = 100_000_000.0;
pub const ADMISSION_LEVEL_BPS: u16 = 9_500;
const SYMMETRY_TOLERANCE: f64 = 1.0e-10;

pub const CHI_SQUARE_6DOF: [(u16, f64); 5] = [
    (5_000, 5.348_120_627_447_118),
    (8_000, 8.558_059_720_250_295),
    (9_000, 10.644_640_675_668_422),
    (9_500, 12.591_587_243_743_977),
    (9_900, 16.811_893_829_770_927),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoseDecision {
    Accept,
    Abstain,
}

impl PoseDecision {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ACCEPT" => Some(Self::Accept),
            "ABSTAIN" => Some(Self::Abstain),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "ACCEPT",
            Self::Abstain => "ABSTAIN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeState {
    Measured,
    NotMeasured,
}

impl ComputeState {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "MEASURED" => Some(Self::Measured),
            "NOT_MEASURED" => Some(Self::NotMeasured),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationDisposition {
    Evaluated,
    Abstained,
    InvalidInput,
    OutOfScope,
    InvalidCovariance,
    DuplicateSample,
    SplitLeakage,
}

impl EvaluationDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Evaluated => "EVALUATED",
            Self::Abstained => "ABSTAINED",
            Self::InvalidInput => "INVALID_INPUT",
            Self::OutOfScope => "OUT_OF_SCOPE",
            Self::InvalidCovariance => "INVALID_COVARIANCE",
            Self::DuplicateSample => "DUPLICATE_SAMPLE",
            Self::SplitLeakage => "SPLIT_LEAKAGE",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoseInput {
    pub case_id: String,
    pub sample_id: String,
    pub target_id: String,
    pub split: String,
    pub dataset_id: String,
    pub dataset_revision: String,
    pub dataset_sha256: String,
    pub model_revision: String,
    pub model_sha256: String,
    pub calibration_revision: String,
    pub calibration_sha256: String,
    pub sensor_mode: String,
    pub support_mode: String,
    pub frame_convention: String,
    pub quaternion_order: String,
    pub tangent_convention: String,
    pub translation_unit: String,
    pub rotation_unit: String,
    pub decision: PoseDecision,
    pub abstain_reason: String,
    pub admission_confidence_bps: u16,
    pub confidence_score_ppm: u32,
    pub visible_fraction: f64,
    pub correspondence_count: u32,
    pub geometry_condition: f64,
    pub ood_flag: bool,
    pub symmetry_ambiguous: bool,
    pub compute_state: ComputeState,
    pub latency_us: u64,
    pub energy_uj: u64,
    pub predicted_translation_m: [f64; 3],
    /// Explicit XYZW source order.
    pub predicted_quaternion_xyzw: [f64; 4],
    pub ground_truth_translation_m: [f64; 3],
    /// Explicit XYZW source order.
    pub ground_truth_quaternion_xyzw: [f64; 4],
    /// Row-major 6x6 covariance in COVARIANCE_ORDER and COVARIANCE_UNIT.
    pub covariance: [f64; 36],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoseResult {
    pub case_id: String,
    pub sample_id: String,
    pub target_id: String,
    pub split: String,
    pub dataset_id: String,
    pub dataset_revision: String,
    pub dataset_sha256: String,
    pub model_revision: String,
    pub model_sha256: String,
    pub calibration_revision: String,
    pub calibration_sha256: String,
    pub sensor_mode: String,
    pub support_mode: String,
    pub frame_convention: String,
    pub tangent_convention: String,
    pub decision_reason: String,
    pub disposition: EvaluationDisposition,
    pub decision: PoseDecision,
    pub translation_error_m: Option<f64>,
    pub rotation_error_rad: Option<f64>,
    pub mahalanobis_squared: Option<f64>,
    pub covariance_log_determinant: Option<f64>,
    pub prediction_region_log_volume_95: Option<f64>,
    pub coverage_50: Option<bool>,
    pub coverage_80: Option<bool>,
    pub coverage_90: Option<bool>,
    pub coverage_95: Option<bool>,
    pub coverage_99: Option<bool>,
    pub confidence_score_ppm: u32,
    pub accepted_by_model: bool,
    pub observability_policy_violation: bool,
    pub false_confident_95: bool,
    pub benchmark_admissible: bool,
    pub visible_fraction: f64,
    pub correspondence_count: u32,
    pub geometry_condition: f64,
    pub ood_flag: bool,
    pub symmetry_ambiguous: bool,
    pub latency_us: Option<u64>,
    pub energy_uj: Option<u64>,
    pub reason_codes: Vec<String>,
    pub source_bytes_authenticated: bool,
    pub physical_calibration_validated: bool,
    pub maneuver_authority_granted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RiskCoveragePoint {
    pub confidence_threshold_ppm: u32,
    pub selected_count: usize,
    pub coverage_fraction: f64,
    pub false_confident_count: usize,
    pub selective_risk: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationSummary {
    pub valid_count: usize,
    pub accepted_count: usize,
    pub accepted_pose_coverage_fraction: Option<f64>,
    pub abstained_count: usize,
    pub invalid_count: usize,
    pub accepted_false_confident_95_count: usize,
    pub accepted_coverage_50: Option<f64>,
    pub accepted_coverage_80: Option<f64>,
    pub accepted_coverage_90: Option<f64>,
    pub accepted_coverage_95: Option<f64>,
    pub accepted_coverage_99: Option<f64>,
    pub accepted_translation_p95_m: Option<f64>,
    pub accepted_rotation_p95_rad: Option<f64>,
    pub accepted_covariance_logdet_median: Option<f64>,
    pub accepted_95_region_log_volume_median: Option<f64>,
    pub risk_coverage: Vec<RiskCoveragePoint>,
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

fn quat_norm(q: [f64; 4]) -> f64 {
    q.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn quat_conjugate(q: [f64; 4]) -> [f64; 4] {
    [-q[0], -q[1], -q[2], q[3]]
}

fn quat_multiply(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

fn rotate(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let u = [q[0], q[1], q[2]];
    let uv = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let uuv = [
        u[1] * uv[2] - u[2] * uv[1],
        u[2] * uv[0] - u[0] * uv[2],
        u[0] * uv[1] - u[1] * uv[0],
    ];
    [
        v[0] + 2.0 * (q[3] * uv[0] + uuv[0]),
        v[1] + 2.0 * (q[3] * uv[1] + uuv[1]),
        v[2] + 2.0 * (q[3] * uv[2] + uuv[2]),
    ]
}

fn normalized_quaternion(q: [f64; 4]) -> Option<[f64; 4]> {
    if !finite(&q) {
        return None;
    }
    let norm = quat_norm(q);
    if !norm.is_finite() || norm <= 1.0e-12 || (norm - 1.0).abs() > 1.0e-6 {
        return None;
    }
    Some([q[0] / norm, q[1] / norm, q[2] / norm, q[3] / norm])
}

fn rotation_log(q: [f64; 4]) -> [f64; 3] {
    let mut canonical = q;
    let negate = if canonical[3] != 0.0 {
        canonical[3] < 0.0
    } else {
        canonical[..3]
            .iter()
            .find(|value| value.abs() > 1.0e-15)
            .is_some_and(|value| *value < 0.0)
    };
    if negate {
        for value in &mut canonical {
            *value = -*value;
        }
    }
    let vector_norm = (canonical[0] * canonical[0]
        + canonical[1] * canonical[1]
        + canonical[2] * canonical[2])
        .sqrt();
    if vector_norm <= 1.0e-12 {
        return [
            2.0 * canonical[0],
            2.0 * canonical[1],
            2.0 * canonical[2],
        ];
    }
    let angle = 2.0 * vector_norm.atan2(canonical[3].clamp(-1.0, 1.0));
    let factor = angle / vector_norm;
    [
        canonical[0] * factor,
        canonical[1] * factor,
        canonical[2] * factor,
    ]
}

fn cholesky(matrix: &[f64; 36]) -> Option<[[f64; 6]; 6]> {
    if !finite(matrix) {
        return None;
    }
    let scale = matrix.iter().map(|value| value.abs()).fold(0.0_f64, f64::max);
    if scale <= 0.0 || !scale.is_finite() {
        return None;
    }

    for row in 0..6 {
        for column in 0..6 {
            let a = matrix[row * 6 + column];
            let b = matrix[column * 6 + row];
            if (a - b).abs() > SYMMETRY_TOLERANCE * scale {
                return None;
            }
        }
    }

    let mut lower = [[0.0_f64; 6]; 6];
    for row in 0..6 {
        for column in 0..=row {
            let mut sum = matrix[row * 6 + column];
            for k in 0..column {
                sum -= lower[row][k] * lower[column][k];
            }
            if row == column {
                if !sum.is_finite() || sum <= f64::EPSILON * scale * 16.0 {
                    return None;
                }
                lower[row][column] = sum.sqrt();
            } else {
                let pivot = lower[column][column];
                if pivot <= 0.0 || !pivot.is_finite() {
                    return None;
                }
                lower[row][column] = sum / pivot;
            }
        }
    }
    Some(lower)
}

fn mahalanobis_squared(lower: &[[f64; 6]; 6], error: [f64; 6]) -> Option<f64> {
    let mut solved = [0.0_f64; 6];
    for row in 0..6 {
        let mut value = error[row];
        for column in 0..row {
            value -= lower[row][column] * solved[column];
        }
        value /= lower[row][row];
        if !value.is_finite() {
            return None;
        }
        solved[row] = value;
    }
    let result = solved.iter().map(|value| value * value).sum::<f64>();
    result.is_finite().then_some(result)
}

fn quantile(level_bps: u16) -> Option<f64> {
    CHI_SQUARE_6DOF
        .iter()
        .find_map(|(level, value)| (*level == level_bps).then_some(*value))
}

fn covariance_coverage(value: f64) -> [bool; 5] {
    [
        value <= CHI_SQUARE_6DOF[0].1,
        value <= CHI_SQUARE_6DOF[1].1,
        value <= CHI_SQUARE_6DOF[2].1,
        value <= CHI_SQUARE_6DOF[3].1,
        value <= CHI_SQUARE_6DOF[4].1,
    ]
}

fn base_result(
    input: &PoseInput,
    disposition: EvaluationDisposition,
    reason: &str,
) -> PoseResult {
    let accepted = input.decision == PoseDecision::Accept;
    PoseResult {
        case_id: input.case_id.clone(),
        sample_id: input.sample_id.clone(),
        target_id: input.target_id.clone(),
        split: input.split.clone(),
        dataset_id: input.dataset_id.clone(),
        dataset_revision: input.dataset_revision.clone(),
        dataset_sha256: input.dataset_sha256.clone(),
        model_revision: input.model_revision.clone(),
        model_sha256: input.model_sha256.clone(),
        calibration_revision: input.calibration_revision.clone(),
        calibration_sha256: input.calibration_sha256.clone(),
        sensor_mode: input.sensor_mode.clone(),
        support_mode: input.support_mode.clone(),
        frame_convention: input.frame_convention.clone(),
        tangent_convention: input.tangent_convention.clone(),
        decision_reason: input.abstain_reason.clone(),
        disposition,
        decision: input.decision,
        translation_error_m: None,
        rotation_error_rad: None,
        mahalanobis_squared: None,
        covariance_log_determinant: None,
        prediction_region_log_volume_95: None,
        coverage_50: None,
        coverage_80: None,
        coverage_90: None,
        coverage_95: None,
        coverage_99: None,
        confidence_score_ppm: input.confidence_score_ppm,
        accepted_by_model: accepted,
        observability_policy_violation: false,
        false_confident_95: false,
        benchmark_admissible: false,
        visible_fraction: input.visible_fraction,
        correspondence_count: input.correspondence_count,
        geometry_condition: input.geometry_condition,
        ood_flag: input.ood_flag,
        symmetry_ambiguous: input.symmetry_ambiguous,
        latency_us: (input.compute_state == ComputeState::Measured).then_some(input.latency_us),
        energy_uj: (input.compute_state == ComputeState::Measured).then_some(input.energy_uj),
        reason_codes: if reason.is_empty() {
            Vec::new()
        } else {
            vec![reason.to_owned()]
        },
        source_bytes_authenticated: false,
        physical_calibration_validated: false,
        maneuver_authority_granted: false,
    }
}

pub fn evaluate(input: &PoseInput) -> PoseResult {
    if [
        input.case_id.as_str(),
        input.sample_id.as_str(),
        input.target_id.as_str(),
        input.dataset_id.as_str(),
        input.dataset_revision.as_str(),
        input.model_revision.as_str(),
        input.calibration_revision.as_str(),
    ]
    .iter()
    .any(|value| !valid_text(value))
    {
        return base_result(
            input,
            EvaluationDisposition::InvalidInput,
            "IDENTITY_OR_REASON_INVALID",
        );
    }

    if !valid_sha256(&input.dataset_sha256)
        || !valid_sha256(&input.model_sha256)
        || !valid_sha256(&input.calibration_sha256)
        || !matches!(input.split.as_str(), "train" | "validation" | "test")
        || input.confidence_score_ppm > 1_000_000
        || input.admission_confidence_bps != ADMISSION_LEVEL_BPS
        || (input.decision == PoseDecision::Abstain && !valid_text(&input.abstain_reason))
        || (input.decision == PoseDecision::Accept && !input.abstain_reason.is_empty())
    {
        return base_result(
            input,
            EvaluationDisposition::InvalidInput,
            "IDENTITY_CONFIDENCE_OR_DECISION_INVALID",
        );
    }

    if input.dataset_id != "SYNTHETIC-SE3-TRUST-v0.1"
        || input.sensor_mode != "RGB"
        || !matches!(
            input.support_mode.as_str(),
            "KEYPOINT_PNP" | "DENSE_PNP" | "EVENT_KEYPOINT_PNP" | "DIRECT_REGRESSION"
        )
        || input.frame_convention != "WORLD_RIGHT_HANDED_METRES"
        || input.quaternion_order != "XYZW"
        || input.tangent_convention != COVARIANCE_ORDER
        || input.translation_unit != "metre"
        || input.rotation_unit != "radian"
        || !finite(&[
            input.visible_fraction,
            input.geometry_condition,
            input.predicted_translation_m[0],
            input.predicted_translation_m[1],
            input.predicted_translation_m[2],
            input.ground_truth_translation_m[0],
            input.ground_truth_translation_m[1],
            input.ground_truth_translation_m[2],
        ])
        || !(0.0..=1.0).contains(&input.visible_fraction)
        || (input.support_mode != "DIRECT_REGRESSION"
            && input.geometry_condition <= 0.0)
        || (input.support_mode == "DIRECT_REGRESSION"
            && input.geometry_condition < 0.0)
    {
        return base_result(
            input,
            EvaluationDisposition::OutOfScope,
            "FRAME_UNIT_OR_SENSOR_CONVENTION_UNSUPPORTED",
        );
    }

    if input.compute_state == ComputeState::Measured
        && (input.latency_us == 0 || input.energy_uj == 0)
    {
        return base_result(
            input,
            EvaluationDisposition::InvalidInput,
            "MEASURED_COMPUTE_REQUIRES_POSITIVE_LATENCY_AND_ENERGY",
        );
    }
    if input.compute_state == ComputeState::NotMeasured
        && (input.latency_us != 0 || input.energy_uj != 0)
    {
        return base_result(
            input,
            EvaluationDisposition::InvalidInput,
            "UNMEASURED_COMPUTE_MUST_NOT_CARRY_NUMERIC_CLAIMS",
        );
    }

    let Some(predicted_q) = normalized_quaternion(input.predicted_quaternion_xyzw) else {
        return base_result(
            input,
            EvaluationDisposition::InvalidInput,
            "PREDICTED_QUATERNION_INVALID",
        );
    };
    let Some(ground_truth_q) = normalized_quaternion(input.ground_truth_quaternion_xyzw) else {
        return base_result(
            input,
            EvaluationDisposition::InvalidInput,
            "GROUND_TRUTH_QUATERNION_INVALID",
        );
    };

    let Some(cholesky) = cholesky(&input.covariance) else {
        return base_result(
            input,
            EvaluationDisposition::InvalidCovariance,
            "COVARIANCE_NOT_FINITE_SYMMETRIC_POSITIVE_DEFINITE",
        );
    };

    let world_delta = [
        input.ground_truth_translation_m[0] - input.predicted_translation_m[0],
        input.ground_truth_translation_m[1] - input.predicted_translation_m[1],
        input.ground_truth_translation_m[2] - input.predicted_translation_m[2],
    ];
    let body_translation = rotate(quat_conjugate(predicted_q), world_delta);
    let relative_rotation = quat_multiply(quat_conjugate(predicted_q), ground_truth_q);
    let rotation_vector = rotation_log(relative_rotation);
    let error = [
        body_translation[0],
        body_translation[1],
        body_translation[2],
        rotation_vector[0],
        rotation_vector[1],
        rotation_vector[2],
    ];

    if !finite(&error) {
        return base_result(
            input,
            EvaluationDisposition::InvalidInput,
            "SE3_TANGENT_RESIDUAL_NONFINITE",
        );
    }

    let Some(mahalanobis) = mahalanobis_squared(&cholesky, error) else {
        return base_result(
            input,
            EvaluationDisposition::InvalidCovariance,
            "MAHALANOBIS_SOLVE_FAILED",
        );
    };
    let translation_error = world_delta.iter().map(|value| value * value).sum::<f64>().sqrt();
    let rotation_error = rotation_vector.iter().map(|value| value * value).sum::<f64>().sqrt();
    let [cov50, cov80, cov90, cov95, cov99] = covariance_coverage(mahalanobis);
    let covariance_log_determinant =
        2.0 * (0..6).map(|index| cholesky[index][index].ln()).sum::<f64>();
    let prediction_region_log_volume_95 = (std::f64::consts::PI.powi(3) / 6.0).ln()
        + 0.5 * covariance_log_determinant
        + 3.0 * quantile(ADMISSION_LEVEL_BPS)
            .expect("the fixed 95% chi-square quantile is present")
            .ln();

    let requires_correspondence_geometry = matches!(
        input.support_mode.as_str(),
        "KEYPOINT_PNP" | "DENSE_PNP" | "EVENT_KEYPOINT_PNP"
    );
    let observability_violation = input.ood_flag
        || input.symmetry_ambiguous
        || input.visible_fraction < MIN_VISIBLE_FRACTION
        || (requires_correspondence_geometry
            && (input.correspondence_count < MIN_CORRESPONDENCES
                || input.geometry_condition > MAX_GEOMETRY_CONDITION));
    let false_confident = input.decision == PoseDecision::Accept
        && (!cov95 || observability_violation);
    let admissible = input.decision == PoseDecision::Accept && !false_confident;

    let mut result = base_result(
        input,
        if input.decision == PoseDecision::Abstain {
            EvaluationDisposition::Abstained
        } else {
            EvaluationDisposition::Evaluated
        },
        "",
    );
    result.translation_error_m = Some(translation_error);
    result.rotation_error_rad = Some(rotation_error);
    result.mahalanobis_squared = Some(mahalanobis);
    result.covariance_log_determinant = Some(covariance_log_determinant);
    result.prediction_region_log_volume_95 = Some(prediction_region_log_volume_95);
    result.coverage_50 = Some(cov50);
    result.coverage_80 = Some(cov80);
    result.coverage_90 = Some(cov90);
    result.coverage_95 = Some(cov95);
    result.coverage_99 = Some(cov99);
    result.observability_policy_violation = observability_violation;
    result.false_confident_95 = false_confident;
    result.benchmark_admissible = admissible;
    if false_confident {
        result.reason_codes.push(if !cov95 {
            "ACCEPT_OUTSIDE_DECLARED_95_PERCENT_REGION".to_owned()
        } else {
            "ACCEPT_WITH_OOD_OR_UNOBSERVABLE_GEOMETRY".to_owned()
        });
    }
    result
}

pub fn evaluate_manifest(inputs: &[PoseInput]) -> Vec<PoseResult> {
    let mut seen_samples = BTreeSet::new();
    let mut duplicate_samples = BTreeSet::new();
    let mut target_splits: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for input in inputs {
        if !seen_samples.insert(input.sample_id.as_str()) {
            duplicate_samples.insert(input.sample_id.as_str());
        }
        target_splits
            .entry(input.target_id.as_str())
            .or_default()
            .insert(input.split.as_str());
    }
    let leaked_targets: BTreeSet<&str> = target_splits
        .iter()
        .filter_map(|(target, splits)| (splits.len() > 1).then_some(*target))
        .collect();

    inputs
        .iter()
        .map(|input| {
            if leaked_targets.contains(input.target_id.as_str()) {
                base_result(
                    input,
                    EvaluationDisposition::SplitLeakage,
                    "TARGET_ID_PRESENT_IN_MULTIPLE_SPLITS",
                )
            } else if duplicate_samples.contains(input.sample_id.as_str()) {
                base_result(
                    input,
                    EvaluationDisposition::DuplicateSample,
                    "SAMPLE_ID_DUPLICATED",
                )
            } else {
                evaluate(input)
            }
        })
        .collect()
}

fn percentile(values: &mut [f64], percentile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let index = ((values.len() - 1) as f64 * percentile).ceil() as usize;
    values.get(index).copied()
}

pub fn summarize(results: &[PoseResult]) -> EvaluationSummary {
    let valid: Vec<&PoseResult> = results
        .iter()
        .filter(|result| {
            matches!(
                result.disposition,
                EvaluationDisposition::Evaluated | EvaluationDisposition::Abstained
            )
        })
        .collect();
    let accepted: Vec<&PoseResult> = valid
        .iter()
        .copied()
        .filter(|result| result.accepted_by_model)
        .collect();
    let false_confident_count = accepted
        .iter()
        .filter(|result| result.false_confident_95)
        .count();

    let coverage = |select: fn(&PoseResult) -> Option<bool>| -> Option<f64> {
        if accepted.is_empty() {
            None
        } else {
            Some(
                accepted
                    .iter()
                    .filter(|result| select(result) == Some(true))
                    .count() as f64
                    / accepted.len() as f64,
            )
        }
    };

    let mut translation_errors: Vec<f64> = accepted
        .iter()
        .filter_map(|result| result.translation_error_m)
        .collect();
    let mut rotation_errors: Vec<f64> = accepted
        .iter()
        .filter_map(|result| result.rotation_error_rad)
        .collect();
    let mut covariance_logd: Vec<f64> = accepted
        .iter()
        .filter_map(|result| result.covariance_log_determinant)
        .collect();
    let mut region_log_volume: Vec<f64> = accepted
        .iter()
        .filter_map(|result| result.prediction_region_log_volume_95)
        .collect();

    let thresholds = [0, 250_000, 500_000, 750_000, 900_000, 950_000, 990_000, 1_000_000];
    let risk_coverage = thresholds
        .iter()
        .map(|threshold| {
            let selected: Vec<&PoseResult> = valid
                .iter()
                .copied()
                .filter(|result| {
                    result.accepted_by_model && result.confidence_score_ppm >= *threshold
                })
                .collect();
            let false_confident = selected
                .iter()
                .filter(|result| result.false_confident_95)
                .count();
            RiskCoveragePoint {
                confidence_threshold_ppm: *threshold,
                selected_count: selected.len(),
                coverage_fraction: if valid.is_empty() {
                    0.0
                } else {
                    selected.len() as f64 / valid.len() as f64
                },
                false_confident_count: false_confident,
                selective_risk: if selected.is_empty() {
                    None
                } else {
                    Some(false_confident as f64 / selected.len() as f64)
                },
            }
        })
        .collect();

    EvaluationSummary {
        valid_count: valid.len(),
        accepted_count: accepted.len(),
        accepted_pose_coverage_fraction: if valid.is_empty() {
            None
        } else {
            Some(accepted.len() as f64 / valid.len() as f64)
        },
        abstained_count: valid.len().saturating_sub(accepted.len()),
        invalid_count: results.len().saturating_sub(valid.len()),
        accepted_false_confident_95_count: false_confident_count,
        accepted_coverage_50: coverage(|result| result.coverage_50),
        accepted_coverage_80: coverage(|result| result.coverage_80),
        accepted_coverage_90: coverage(|result| result.coverage_90),
        accepted_coverage_95: coverage(|result| result.coverage_95),
        accepted_coverage_99: coverage(|result| result.coverage_99),
        accepted_translation_p95_m: percentile(&mut translation_errors, 0.95),
        accepted_rotation_p95_rad: percentile(&mut rotation_errors, 0.95),
        accepted_covariance_logdet_median: percentile(&mut covariance_logd, 0.5),
        accepted_95_region_log_volume_median: percentile(&mut region_log_volume, 0.5),
        risk_coverage,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EvaluationDisposition, PoseDecision, PoseInput, ComputeState, evaluate, evaluate_manifest,
        summarize,
    };

    fn input() -> PoseInput {
        let mut covariance = [0.0_f64; 36];
        for index in 0..6 {
            covariance[index * 6 + index] = 1.0;
        }
        PoseInput {
            case_id: "POSE-001".to_owned(),
            sample_id: "sample-001".to_owned(),
            target_id: "target-001".to_owned(),
            split: "test".to_owned(),
            dataset_id: "SYNTHETIC-SE3-TRUST-v0.1".to_owned(),
            dataset_revision: "r1".to_owned(),
            dataset_sha256: "a".repeat(64),
            model_revision: "model-r1".to_owned(),
            model_sha256: "b".repeat(64),
            calibration_revision: "cal-r1".to_owned(),
            calibration_sha256: "c".repeat(64),
            sensor_mode: "RGB".to_owned(),
            support_mode: "KEYPOINT_PNP".to_owned(),
            frame_convention: "WORLD_RIGHT_HANDED_METRES".to_owned(),
            quaternion_order: "XYZW".to_owned(),
            tangent_convention: super::COVARIANCE_ORDER.to_owned(),
            translation_unit: "metre".to_owned(),
            rotation_unit: "radian".to_owned(),
            decision: PoseDecision::Accept,
            abstain_reason: String::new(),
            admission_confidence_bps: super::ADMISSION_LEVEL_BPS,
            confidence_score_ppm: 900_000,
            visible_fraction: 1.0,
            correspondence_count: 16,
            geometry_condition: 10.0,
            ood_flag: false,
            symmetry_ambiguous: false,
            compute_state: ComputeState::NotMeasured,
            latency_us: 0,
            energy_uj: 0,
            predicted_translation_m: [0.0; 3],
            predicted_quaternion_xyzw: [0.0, 0.0, 0.0, 1.0],
            ground_truth_translation_m: [0.0; 3],
            ground_truth_quaternion_xyzw: [0.0, 0.0, 0.0, 1.0],
            covariance,
        }
    }

    #[test]
    fn exact_pose_is_inside_every_covariance_region() {
        let result = evaluate(&input());
        assert_eq!(result.disposition, EvaluationDisposition::Evaluated);
        assert_eq!(result.mahalanobis_squared, Some(0.0));
        assert_eq!(result.coverage_95, Some(true));
        assert!(result.benchmark_admissible);
    }

    #[test]
    fn quaternion_sign_does_not_change_pose_error() {
        let mut value = input();
        value.ground_truth_quaternion_xyzw = [0.0, 0.0, 0.0, -1.0];
        let result = evaluate(&value);
        assert_eq!(result.rotation_error_rad, Some(0.0));
        assert!(result.benchmark_admissible);
    }

    #[test]
    fn one_metre_translation_with_unit_covariance_has_mahalanobis_one() {
        let mut value = input();
        value.ground_truth_translation_m[0] = 1.0;
        let result = evaluate(&value);
        assert_eq!(result.mahalanobis_squared, Some(1.0));
    }

    #[test]
    fn invalid_covariance_is_not_admissible() {
        let mut value = input();
        value.covariance[0] = -1.0;
        assert_eq!(
            evaluate(&value).disposition,
            EvaluationDisposition::InvalidCovariance
        );
    }

    #[test]
    fn asymmetric_covariance_is_not_admissible() {
        let mut value = input();
        value.covariance[1] = 0.1;
        assert_eq!(
            evaluate(&value).disposition,
            EvaluationDisposition::InvalidCovariance
        );
    }

    #[test]
    fn nonunit_quaternion_is_invalid() {
        let mut value = input();
        value.predicted_quaternion_xyzw = [0.0, 0.0, 0.0, 2.0];
        assert_eq!(
            evaluate(&value).disposition,
            EvaluationDisposition::InvalidInput
        );
    }

    #[test]
    fn confidence_region_is_not_mistaken_for_a_deterministic_enclosure() {
        let mut value = input();
        value.ground_truth_translation_m[0] = 4.0;
        let result = evaluate(&value);
        assert_eq!(result.coverage_95, Some(false));
        assert!(result.false_confident_95);
        assert!(!result.benchmark_admissible);
        assert!(!result.maneuver_authority_granted);
    }

    #[test]
    fn direct_regression_does_not_require_pnp_correspondences() {
        let mut value = input();
        value.support_mode = "DIRECT_REGRESSION".to_owned();
        value.correspondence_count = 0;
        value.geometry_condition = 0.0;
        let result = evaluate(&value);
        assert!(result.benchmark_admissible);
        assert!(!result.observability_policy_violation);
    }

    #[test]
    fn dense_and_keypoint_pnp_require_observable_correspondence_support() {
        let mut value = input();
        value.support_mode = "DENSE_PNP".to_owned();
        value.correspondence_count = 5;
        assert!(evaluate(&value).observability_policy_violation);
    }

    #[test]
    fn pi_rotation_log_is_sign_canonical_for_q_and_negative_q() {
        let positive = super::rotation_log([1.0, 0.0, 0.0, 0.0]);
        let negative = super::rotation_log([-1.0, 0.0, 0.0, -0.0]);
        assert_eq!(positive, negative);
    }

    #[test]
    fn ood_acceptance_is_a_policy_violation_even_inside_covariance_region() {
        let mut value = input();
        value.ood_flag = true;
        let result = evaluate(&value);
        assert!(result.false_confident_95);
        assert!(result.observability_policy_violation);
    }

    #[test]
    fn abstention_is_counted_without_becoming_a_false_confident_acceptance() {
        let mut value = input();
        value.decision = PoseDecision::Abstain;
        value.abstain_reason = "LOW_VISIBLE_SUPPORT".to_owned();
        value.visible_fraction = 0.02;
        let result = evaluate(&value);
        assert_eq!(result.disposition, EvaluationDisposition::Abstained);
        assert!(!result.false_confident_95);
    }

    #[test]
    fn manifest_rejects_target_leakage_across_splits() {
        let first = input();
        let mut second = input();
        second.case_id = "POSE-002".to_owned();
        second.sample_id = "sample-002".to_owned();
        second.split = "train".to_owned();
        let results = evaluate_manifest(&[first, second]);
        assert!(results.iter().all(|result| {
            result.disposition == EvaluationDisposition::SplitLeakage
        }));
    }

    #[test]
    fn risk_coverage_does_not_penalize_explicit_abstentions_as_accepts() {
        let mut accepted = input();
        let mut abstained = input();
        accepted.target_id = "target-accepted".to_owned();
        accepted.confidence_score_ppm = 900_000;
        abstained.case_id = "POSE-ABSTAIN".to_owned();
        abstained.sample_id = "sample-abstain".to_owned();
        abstained.target_id = "target-abstain".to_owned();
        abstained.decision = PoseDecision::Abstain;
        abstained.abstain_reason = "LOW_VISIBLE_SUPPORT".to_owned();
        abstained.visible_fraction = 0.01;
        abstained.confidence_score_ppm = 1_000_000;

        let results = vec![evaluate(&accepted), evaluate(&abstained)];
        let summary = summarize(&results);
        assert_eq!(summary.valid_count, 2);
        assert_eq!(summary.accepted_count, 1);
        assert_eq!(summary.abstained_count, 1);
        assert_eq!(summary.risk_coverage[0].selected_count, 1);
        assert_eq!(summary.risk_coverage[0].false_confident_count, 0);
    }

    #[test]
    fn summary_reports_selective_risk_and_coverage() {    #[test]
    fn summary_reports_selective_risk_and_coverage() {
        let first = input();
        let mut second = input();
        second.case_id = "POSE-002".to_owned();
        second.sample_id = "sample-002".to_owned();
        second.target_id = "target-002".to_owned();
        second.ground_truth_translation_m[0] = 4.0;
        let results = vec![evaluate(&first), evaluate(&second)];
        let summary = summarize(&results);
        assert_eq!(summary.valid_count, 2);
        assert_eq!(summary.accepted_count, 2);
        assert_eq!(summary.accepted_false_confident_95_count, 1);
        assert_eq!(summary.accepted_coverage_95, Some(0.5));
        assert_eq!(summary.risk_coverage.len(), 8);
    }
}
