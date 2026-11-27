#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

pub const RECEIPT_SCHEMA: &str = "luminous.spacecraft-se3-pose-region.receipt.v0.1";
pub const NOMINAL_LEVEL_BPS: u16 = 9_500;
pub const MIN_CALIBRATION_SAMPLES: u64 = 100;
pub const Q32_SCALE: f64 = 4_294_967_296.0;
pub const MAX_EXACT_Q32_INTEGER: u64 = 9_007_199_254_740_991;
pub const TANGENT_CONVENTION: &str = "BODY_LOCAL_TRANSLATION_XYZ_ROTATION_VECTOR_XYZ";
pub const PERTURBATION_CONVENTION: &str = "RIGHT_COMPOSED_BODY_LOCAL_SE3_EXP_V1";
pub const SCORE_DEFINITION: &str = "SQRT_MAHALANOBIS_BODY_LOCAL_SE3_V1";
const PI_UPPER: f64 = f64::from_bits(0x4009_21fb_5444_2d19);
const MICROMETRES_PER_METRE: f64 = 1_000_000.0;
const MICRORADIANS_PER_RADIAN: f64 = 1_000_000.0;
const I64_UPPER_EXCLUSIVE: f64 = 9_223_372_036_854_775_808.0;
const I64_LOWER_INCLUSIVE: f64 = -9_223_372_036_854_775_808.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    CandidateStatisticalPoseRegion,
    AbstainRuntimePoseNotCandidate,
    AbstainUncalibrated,
    OutOfScope,
    InvalidInput,
    InvalidCovariance,
    Overflow,
    DuplicateSample,
}

impl Disposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CandidateStatisticalPoseRegion => "CANDIDATE_STATISTICAL_POSE_REGION",
            Self::AbstainRuntimePoseNotCandidate => "ABSTAIN_RUNTIME_POSE_NOT_CANDIDATE",
            Self::AbstainUncalibrated => "ABSTAIN_UNCALIBRATED",
            Self::OutOfScope => "OUT_OF_SCOPE",
            Self::InvalidInput => "INVALID_INPUT",
            Self::InvalidCovariance => "INVALID_COVARIANCE",
            Self::Overflow => "OVERFLOW",
            Self::DuplicateSample => "DUPLICATE_SAMPLE",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConformalProfile {
    pub profile_id: String,
    pub profile_revision: String,
    pub profile_sha256: String,
    pub calibration_scores_sha256: String,
    pub profile_state: String,
    pub dataset_sha256: String,
    pub model_sha256: String,
    pub model_config_sha256: String,
    pub camera_calibration_sha256: String,
    pub target_population_sha256: String,
    pub calibration_split_sha256: String,
    pub heldout_split_sha256: String,
    pub calibration_method: String,
    pub score_definition: String,
    pub tangent_convention: String,
    pub perturbation_convention: String,
    pub nominal_level_bps: u16,
    pub sample_count: u64,
    pub order_statistic_rank: u64,
    /// Nonconformity radius in unsigned Q32.32, rounded upward by the profile producer.
    pub quantile_radius_q32_32: u64,
    pub heldout_coverage_lower_ci_ppm: u32,
    pub required_coverage_lower_ci_ppm: u32,
    pub source_bytes_authenticated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimePoseReceipt {
    pub case_id: String,
    pub sample_id: String,
    pub frame_id: String,
    pub receipt_sha256: String,
    pub disposition: String,
    pub candidate_only: bool,
    pub deterministic_safety_bound: bool,
    pub maneuver_authority_granted: bool,
    pub upstream_source_bytes_authenticated: bool,
    pub profile_id: String,
    pub profile_sha256: String,
    pub dataset_sha256: String,
    pub model_sha256: String,
    pub model_config_sha256: String,
    pub camera_calibration_sha256: String,
    pub target_population_sha256: String,
    pub frame_convention: String,
    pub quaternion_order: String,
    pub tangent_convention: String,
    pub translation_unit: String,
    pub rotation_unit: String,
    pub nominal_level_bps: u16,
    pub predicted_translation_m: [f64; 3],
    /// XYZW quaternion at the nominal pose.
    pub predicted_quaternion_xyzw: [f64; 4],
    /// Row-major covariance in the declared six-dimensional body-local tangent frame.
    pub covariance_row_major: [f64; 36],
    pub profile: ConformalProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeometricOuterRegion {
    /// World-axis interval endpoints in integer micrometres.
    pub translation_bounds_um: [[i64; 2]; 3],
    /// Geodesic orientation cap radius in integer microradians.
    pub orientation_cap_urad: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegionResult {
    pub case_id: String,
    pub sample_id: String,
    pub frame_id: String,
    pub runtime_receipt_sha256: String,
    pub profile_id: String,
    pub profile_sha256: String,
    pub calibration_scores_sha256: String,
    pub nominal_level_bps: u16,
    pub calibration_sample_count: u64,
    pub order_statistic_rank: u64,
    pub quantile_radius_q32_32: u64,
    pub disposition: Disposition,
    pub reason_code: String,
    pub region: Option<GeometricOuterRegion>,
    pub region_is_statistical: bool,
    pub outer_approximation_of_joint_tangent_region: bool,
    pub source_authentication_verified_here: bool,
    pub source_bytes_authenticated: bool,
    pub physical_calibration_validated: bool,
    pub deterministic_safety_bound: bool,
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

#[derive(Debug, Clone, Copy)]
struct Interval {
    lower: f64,
    upper: f64,
}

impl Interval {
    fn point(value: f64) -> Self {
        Self {
            lower: value,
            upper: value,
        }
    }

    fn finite(self) -> bool {
        self.lower.is_finite()
            && self.upper.is_finite()
            && self.lower <= self.upper
    }
}

fn interval_add(left: Interval, right: Interval) -> Option<Interval> {
    let result = Interval {
        lower: next_down(left.lower + right.lower),
        upper: next_up(left.upper + right.upper),
    };
    result.finite().then_some(result)
}

fn interval_sub(left: Interval, right: Interval) -> Option<Interval> {
    let result = Interval {
        lower: next_down(left.lower - right.upper),
        upper: next_up(left.upper - right.lower),
    };
    result.finite().then_some(result)
}

fn interval_mul(left: Interval, right: Interval) -> Option<Interval> {
    if (left.lower == 0.0 && left.upper == 0.0)
        || (right.lower == 0.0 && right.upper == 0.0)
    {
        return Some(Interval::point(0.0));
    }
    let products = [
        left.lower * right.lower,
        left.lower * right.upper,
        left.upper * right.lower,
        left.upper * right.upper,
    ];
    if !finite(&products) {
        return None;
    }
    let minimum = products.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = products
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let result = Interval {
        lower: next_down(minimum),
        upper: next_up(maximum),
    };
    result.finite().then_some(result)
}

fn interval_square(value: Interval) -> Option<Interval> {
    if value.lower <= 0.0 && value.upper >= 0.0 {
        let maximum = value.lower.abs().max(value.upper.abs());
        let upper = next_up(maximum * maximum);
        return upper.is_finite().then_some(Interval {
            lower: 0.0,
            upper,
        });
    }
    let minimum = value.lower.abs().min(value.upper.abs());
    let maximum = value.lower.abs().max(value.upper.abs());
    let lower = next_down(minimum * minimum).max(0.0);
    let upper = next_up(maximum * maximum);
    (lower.is_finite() && upper.is_finite()).then_some(Interval { lower, upper })
}

fn interval_div_positive(numerator: Interval, denominator: Interval) -> Option<Interval> {
    if denominator.lower <= 0.0 {
        return None;
    }
    let quotients = [
        numerator.lower / denominator.lower,
        numerator.lower / denominator.upper,
        numerator.upper / denominator.lower,
        numerator.upper / denominator.upper,
    ];
    if !finite(&quotients) {
        return None;
    }
    let minimum = quotients.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = quotients
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let result = Interval {
        lower: next_down(minimum),
        upper: next_up(maximum),
    };
    result.finite().then_some(result)
}

fn interval_sqrt_positive(value: Interval) -> Option<Interval> {
    if value.lower <= 0.0 || value.upper <= 0.0 {
        return None;
    }
    let result = Interval {
        lower: next_down(value.lower.sqrt()),
        upper: next_up(value.upper.sqrt()),
    };
    (result.lower > 0.0 && result.finite()).then_some(result)
}

/// Proves positive definiteness using outward-rounded interval Cholesky.
///
/// Every computed factor is enclosed by an interval. A pivot is accepted only when
/// its lower endpoint is strictly positive; inconclusive/overflowing factorizations
/// fail closed. This may reject some valid but excessively ill-conditioned matrices.
fn covariance_is_spd(matrix: &[f64; 36]) -> bool {
    if !finite(matrix) {
        return false;
    }
    for row in 0..6 {
        for column in 0..6 {
            if matrix[row * 6 + column] != matrix[column * 6 + row] {
                return false;
            }
        }
    }

    let zero = Interval::point(0.0);
    let mut lower = [[zero; 6]; 6];
    for row in 0..6 {
        for column in 0..=row {
            let mut residual = Interval::point(matrix[row * 6 + column]);
            for index in 0..column {
                let product = if row == column {
                    interval_square(lower[row][index])
                } else {
                    interval_mul(lower[row][index], lower[column][index])
                };
                let Some(product) = product else {
                    return false;
                };
                let Some(next_residual) = interval_sub(residual, product) else {
                    return false;
                };
                residual = next_residual;
            }

            if row == column {
                let Some(pivot) = interval_sqrt_positive(residual) else {
                    return false;
                };
                lower[row][column] = pivot;
            } else {
                let pivot = lower[column][column];
                let Some(factor) = interval_div_positive(residual, pivot) else {
                    return false;
                };
                lower[row][column] = factor;
            }
        }
    }
    true
}

fn profile_shape_valid(profile: &ConformalProfile) -> bool {
    [
        profile.profile_id.as_str(),
        profile.profile_revision.as_str(),
        profile.profile_state.as_str(),
        profile.calibration_method.as_str(),
        profile.score_definition.as_str(),
        profile.tangent_convention.as_str(),
        profile.perturbation_convention.as_str(),
    ]
    .iter()
    .all(|value| valid_text(value))
        && [
            profile.profile_sha256.as_str(),
            profile.calibration_scores_sha256.as_str(),
            profile.dataset_sha256.as_str(),
            profile.model_sha256.as_str(),
            profile.model_config_sha256.as_str(),
            profile.camera_calibration_sha256.as_str(),
            profile.target_population_sha256.as_str(),
            profile.calibration_split_sha256.as_str(),
            profile.heldout_split_sha256.as_str(),
        ]
        .iter()
        .all(|value| valid_sha256(value))
        && profile.calibration_split_sha256 != profile.heldout_split_sha256
        && profile.nominal_level_bps > 0
        && profile.nominal_level_bps <= 10_000
        && profile.heldout_coverage_lower_ci_ppm <= 1_000_000
        && profile.required_coverage_lower_ci_ppm <= 1_000_000
        && profile.quantile_radius_q32_32 <= MAX_EXACT_Q32_INTEGER
}

fn runtime_shape_valid(input: &RuntimePoseReceipt) -> bool {
    [
        input.case_id.as_str(),
        input.sample_id.as_str(),
        input.frame_id.as_str(),
        input.disposition.as_str(),
        input.profile_id.as_str(),
        input.frame_convention.as_str(),
        input.quaternion_order.as_str(),
        input.tangent_convention.as_str(),
        input.translation_unit.as_str(),
        input.rotation_unit.as_str(),
    ]
    .iter()
    .all(|value| valid_text(value))
        && [
            input.receipt_sha256.as_str(),
            input.profile_sha256.as_str(),
            input.dataset_sha256.as_str(),
            input.model_sha256.as_str(),
            input.model_config_sha256.as_str(),
            input.camera_calibration_sha256.as_str(),
            input.target_population_sha256.as_str(),
        ]
        .iter()
        .all(|value| valid_sha256(value))
        && input.nominal_level_bps > 0
        && input.nominal_level_bps <= 10_000
        && finite(&input.predicted_translation_m)
}

fn empty_result(
    input: &RuntimePoseReceipt,
    disposition: Disposition,
    reason_code: &str,
) -> RegionResult {
    RegionResult {
        case_id: input.case_id.clone(),
        sample_id: input.sample_id.clone(),
        frame_id: input.frame_id.clone(),
        runtime_receipt_sha256: input.receipt_sha256.clone(),
        profile_id: input.profile.profile_id.clone(),
        profile_sha256: input.profile.profile_sha256.clone(),
        calibration_scores_sha256: input.profile.calibration_scores_sha256.clone(),
        nominal_level_bps: input.profile.nominal_level_bps,
        calibration_sample_count: input.profile.sample_count,
        order_statistic_rank: input.profile.order_statistic_rank,
        quantile_radius_q32_32: input.profile.quantile_radius_q32_32,
        disposition,
        reason_code: reason_code.to_owned(),
        region: None,
        region_is_statistical: false,
        outer_approximation_of_joint_tangent_region: false,
        source_authentication_verified_here: false,
        source_bytes_authenticated: false,
        physical_calibration_validated: false,
        deterministic_safety_bound: false,
        maneuver_authority_granted: false,
    }
}

fn candidate_result(input: &RuntimePoseReceipt, region: GeometricOuterRegion) -> RegionResult {
    let mut result = empty_result(
        input,
        Disposition::CandidateStatisticalPoseRegion,
        "CALIBRATED_TANGENT_REGION_OUTER_APPROXIMATION_CONSTRUCTED",
    );
    result.region = Some(region);
    result.region_is_statistical = true;
    result.outer_approximation_of_joint_tangent_region = true;
    result
}

/// IEEE-754 successor/predecessor helpers used to direct every positive bound operation outward.
fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    if value > 0.0 {
        f64::from_bits(bits + 1)
    } else {
        f64::from_bits(bits - 1)
    }
}

fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = value.to_bits();
    if value > 0.0 {
        f64::from_bits(bits - 1)
    } else {
        f64::from_bits(bits + 1)
    }
}

fn upper_add(left: f64, right: f64) -> Option<f64> {
    let value = next_up(left + right);
    value.is_finite().then_some(value)
}

fn upper_mul(left: f64, right: f64) -> Option<f64> {
    let value = next_up(left * right);
    value.is_finite().then_some(value)
}

fn gershgorin_upper_3(matrix: &[f64; 36], offset: usize) -> Option<f64> {
    let mut maximum = 0.0_f64;
    for row in 0..3 {
        let mut sum = 0.0_f64;
        for column in 0..3 {
            sum = upper_add(sum, matrix[(offset + row) * 6 + offset + column].abs())?;
        }
        maximum = maximum.max(sum);
    }
    Some(next_up(maximum))
}

fn sqrt_upper(value: f64) -> Option<f64> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let result = next_up(next_up(value).sqrt());
    result.is_finite().then_some(result)
}

fn lower_um(center_m: f64, radius_m: f64) -> Option<i64> {
    let lower_m = next_down(center_m - radius_m);
    if !lower_m.is_finite() {
        return None;
    }
    let scaled = next_down(lower_m * MICROMETRES_PER_METRE);
    if !scaled.is_finite() {
        return None;
    }
    let integer = scaled.floor();
    if !(I64_LOWER_INCLUSIVE..I64_UPPER_EXCLUSIVE).contains(&integer) {
        return None;
    }
    Some(integer as i64)
}

fn upper_um(center_m: f64, radius_m: f64) -> Option<i64> {
    let upper_m = next_up(center_m + radius_m);
    if !upper_m.is_finite() {
        return None;
    }
    let scaled = next_up(upper_m * MICROMETRES_PER_METRE);
    if !scaled.is_finite() {
        return None;
    }
    let integer = scaled.ceil();
    if !(I64_LOWER_INCLUSIVE..I64_UPPER_EXCLUSIVE).contains(&integer) {
        return None;
    }
    Some(integer as i64)
}

fn orientation_cap_urad(radius_rad: f64) -> Option<u64> {
    if !radius_rad.is_finite() || radius_rad < 0.0 {
        return None;
    }
    let capped = radius_rad.min(PI_UPPER);
    let scaled = next_up(capped * MICRORADIANS_PER_RADIAN);
    if !scaled.is_finite() || !(0.0..18_446_744_073_709_551_616.0).contains(&scaled) {
        return None;
    }
    Some(scaled.ceil() as u64)
}

fn conformal_rank(sample_count: u64, nominal_level_bps: u16) -> Option<u64> {
    if nominal_level_bps == 0 || nominal_level_bps > 10_000 {
        return None;
    }
    let numerator = (u128::from(sample_count) + 1) * u128::from(nominal_level_bps);
    let rank = numerator.div_ceil(10_000);
    u64::try_from(rank).ok()
}

/// Builds an outer representation of a joint split-conformal SE(3) tangent ellipsoid.
///
/// This consumes a runtime receipt and an immutable externally produced quantile profile.
/// It does not calculate the profile quantile, inspect ground truth, authenticate source
/// bytes, or turn statistical uncertainty into deterministic physical clearance.
pub fn evaluate(input: &RuntimePoseReceipt) -> RegionResult {
    if !runtime_shape_valid(input) || !profile_shape_valid(&input.profile) {
        return empty_result(
            input,
            Disposition::InvalidInput,
            "RUNTIME_OR_PROFILE_SHAPE_INVALID",
        );
    }

    let profile = &input.profile;
    if input.disposition != "CANDIDATE_STATISTICAL_REGION" || !input.candidate_only {
        return empty_result(
            input,
            Disposition::AbstainRuntimePoseNotCandidate,
            "RUNTIME_POSE_RECEIPT_NOT_CANDIDATE_ONLY",
        );
    }
    if input.deterministic_safety_bound || input.maneuver_authority_granted {
        return empty_result(
            input,
            Disposition::InvalidInput,
            "RUNTIME_RECEIPT_ATTEMPTS_AUTHORITY_ESCALATION",
        );
    }

    if input.frame_convention != "WORLD_RIGHT_HANDED_METRES"
        || input.quaternion_order != "XYZW"
        || input.tangent_convention != TANGENT_CONVENTION
        || input.translation_unit != "metre"
        || input.rotation_unit != "radian"
        || profile.tangent_convention != TANGENT_CONVENTION
        || profile.perturbation_convention != PERTURBATION_CONVENTION
        || profile.score_definition != SCORE_DEFINITION
        || profile.calibration_method != "SPLIT_CONFORMAL_SE3-v1"
        || profile.nominal_level_bps != NOMINAL_LEVEL_BPS
        || input.nominal_level_bps != NOMINAL_LEVEL_BPS
    {
        return empty_result(
            input,
            Disposition::OutOfScope,
            "UNSUPPORTED_REGION_OR_TANGENT_CONVENTION",
        );
    }

    if input.profile_id != profile.profile_id
        || input.profile_sha256 != profile.profile_sha256
        || input.dataset_sha256 != profile.dataset_sha256
        || input.model_sha256 != profile.model_sha256
        || input.model_config_sha256 != profile.model_config_sha256
        || input.camera_calibration_sha256 != profile.camera_calibration_sha256
        || input.target_population_sha256 != profile.target_population_sha256
    {
        return empty_result(
            input,
            Disposition::OutOfScope,
            "RUNTIME_PROFILE_SCOPE_OR_DIGEST_MISMATCH",
        );
    }

    if !input.upstream_source_bytes_authenticated || !profile.source_bytes_authenticated {
        return empty_result(
            input,
            Disposition::AbstainUncalibrated,
            "UPSTREAM_SOURCE_AUTHENTICATION_CLAIM_MISSING",
        );
    }
    if profile.profile_state != "CURRENT"
        || profile.heldout_coverage_lower_ci_ppm < profile.required_coverage_lower_ci_ppm
    {
        return empty_result(
            input,
            Disposition::AbstainUncalibrated,
            "PROFILE_STALE_OR_HELDOUT_COVERAGE_TOO_WEAK",
        );
    }
    if profile.sample_count < MIN_CALIBRATION_SAMPLES {
        return empty_result(
            input,
            Disposition::AbstainUncalibrated,
            "INSUFFICIENT_CALIBRATION_SAMPLE_COUNT",
        );
    }

    let Some(expected_rank) = conformal_rank(profile.sample_count, profile.nominal_level_bps)
    else {
        return empty_result(
            input,
            Disposition::InvalidInput,
            "CONFORMAL_RANK_UNREPRESENTABLE",
        );
    };
    if expected_rank > profile.sample_count {
        return empty_result(
            input,
            Disposition::AbstainUncalibrated,
            "NO_FINITE_SPLIT_CONFORMAL_ORDER_STATISTIC",
        );
    }
    if profile.order_statistic_rank != expected_rank {
        return empty_result(
            input,
            Disposition::InvalidInput,
            "CONFORMAL_ORDER_STATISTIC_RANK_MISMATCH",
        );
    }
    if profile.quantile_radius_q32_32 > MAX_EXACT_Q32_INTEGER {
        return empty_result(
            input,
            Disposition::InvalidInput,
            "Q32_32_QUANTILE_EXCEEDS_EXACT_CONVERSION_RANGE",
        );
    }

    if !quaternion_is_unit_xyzw(input.predicted_quaternion_xyzw) {
        return empty_result(
            input,
            Disposition::InvalidInput,
            "PREDICTED_QUATERNION_NOT_UNIT_XYZW",
        );
    }
    if !covariance_is_spd(&input.covariance_row_major) {
        return empty_result(
            input,
            Disposition::InvalidCovariance,
            "COVARIANCE_NOT_FINITE_SYMMETRIC_POSITIVE_DEFINITE",
        );
    }

    let q = (profile.quantile_radius_q32_32 as f64) / Q32_SCALE;
    let translation_lambda = gershgorin_upper_3(&input.covariance_row_major, 0);
    let rotation_lambda = gershgorin_upper_3(&input.covariance_row_major, 3);
    let (Some(translation_lambda), Some(rotation_lambda)) = (translation_lambda, rotation_lambda)
    else {
        return empty_result(input, Disposition::Overflow, "COVARIANCE_ROW_SUM_OVERFLOW");
    };
    let translation_root = sqrt_upper(translation_lambda);
    let rotation_root = sqrt_upper(rotation_lambda);
    let (Some(translation_root), Some(rotation_root)) = (translation_root, rotation_root) else {
        return empty_result(
            input,
            Disposition::Overflow,
            "COVARIANCE_SQUARE_ROOT_OVERFLOW",
        );
    };
    let translation_radius = upper_mul(q, translation_root);
    let rotation_radius = upper_mul(q, rotation_root);
    let (Some(translation_radius), Some(rotation_radius)) = (translation_radius, rotation_radius)
    else {
        return empty_result(
            input,
            Disposition::Overflow,
            "CONFORMAL_REGION_RADIUS_OVERFLOW",
        );
    };

    let mut bounds = [[0_i64; 2]; 3];
    for (axis, bound) in bounds.iter_mut().enumerate() {
        let lower = lower_um(input.predicted_translation_m[axis], translation_radius);
        let upper = upper_um(input.predicted_translation_m[axis], translation_radius);
        let (Some(lower), Some(upper)) = (lower, upper) else {
            return empty_result(
                input,
                Disposition::Overflow,
                "TRANSLATION_BOUND_CONVERSION_OVERFLOW",
            );
        };
        if lower > upper {
            return empty_result(
                input,
                Disposition::InvalidInput,
                "TRANSLATION_INTERVAL_ORDER_INVALID",
            );
        }
        *bound = [lower, upper];
    }

    let Some(orientation_cap) = orientation_cap_urad(rotation_radius) else {
        return empty_result(
            input,
            Disposition::Overflow,
            "ORIENTATION_CAP_CONVERSION_OVERFLOW",
        );
    };

    candidate_result(
        input,
        GeometricOuterRegion {
            translation_bounds_um: bounds,
            orientation_cap_urad: orientation_cap,
        },
    )
}

/// Duplicate sample identities invalidate every occurrence; a manifest cannot select a winner.
pub fn evaluate_manifest(inputs: &[RuntimePoseReceipt]) -> Vec<RegionResult> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for input in inputs {
        *counts.entry(input.sample_id.as_str()).or_default() += 1;
    }
    let duplicates: BTreeSet<&str> = counts
        .into_iter()
        .filter_map(|(sample_id, count)| (count > 1).then_some(sample_id))
        .collect();

    inputs
        .iter()
        .map(|input| {
            if duplicates.contains(input.sample_id.as_str()) {
                empty_result(
                    input,
                    Disposition::DuplicateSample,
                    "SAMPLE_ID_DUPLICATED_IN_MANIFEST",
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
        ConformalProfile, Disposition, PERTURBATION_CONVENTION, RuntimePoseReceipt,
        SCORE_DEFINITION, TANGENT_CONVENTION, evaluate, evaluate_manifest,
    };

    fn profile() -> ConformalProfile {
        ConformalProfile {
            profile_id: "profile-1".to_owned(),
            profile_revision: "r1".to_owned(),
            profile_sha256: "a".repeat(64),
            calibration_scores_sha256: "b".repeat(64),
            profile_state: "CURRENT".to_owned(),
            dataset_sha256: "c".repeat(64),
            model_sha256: "d".repeat(64),
            model_config_sha256: "e".repeat(64),
            camera_calibration_sha256: "f".repeat(64),
            target_population_sha256: "1".repeat(64),
            calibration_split_sha256: "2".repeat(64),
            heldout_split_sha256: "3".repeat(64),
            calibration_method: "SPLIT_CONFORMAL_SE3-v1".to_owned(),
            score_definition: SCORE_DEFINITION.to_owned(),
            tangent_convention: TANGENT_CONVENTION.to_owned(),
            perturbation_convention: PERTURBATION_CONVENTION.to_owned(),
            nominal_level_bps: 9_500,
            sample_count: 1_000,
            order_statistic_rank: 951,
            quantile_radius_q32_32: 8_589_934_592,
            heldout_coverage_lower_ci_ppm: 925_000,
            required_coverage_lower_ci_ppm: 900_000,
            source_bytes_authenticated: true,
        }
    }

    fn input() -> RuntimePoseReceipt {
        let mut covariance = [0.0_f64; 36];
        for axis in 0..3 {
            covariance[axis * 6 + axis] = 1.0e-6;
            covariance[(axis + 3) * 6 + axis + 3] = 4.0e-6;
        }
        RuntimePoseReceipt {
            case_id: "REG-001".to_owned(),
            sample_id: "ACE/approach/0001".to_owned(),
            frame_id: "frame-0001".to_owned(),
            receipt_sha256: "4".repeat(64),
            disposition: "CANDIDATE_STATISTICAL_REGION".to_owned(),
            candidate_only: true,
            deterministic_safety_bound: false,
            maneuver_authority_granted: false,
            upstream_source_bytes_authenticated: true,
            profile_id: "profile-1".to_owned(),
            profile_sha256: "a".repeat(64),
            dataset_sha256: "c".repeat(64),
            model_sha256: "d".repeat(64),
            model_config_sha256: "e".repeat(64),
            camera_calibration_sha256: "f".repeat(64),
            target_population_sha256: "1".repeat(64),
            frame_convention: "WORLD_RIGHT_HANDED_METRES".to_owned(),
            quaternion_order: "XYZW".to_owned(),
            tangent_convention: TANGENT_CONVENTION.to_owned(),
            translation_unit: "metre".to_owned(),
            rotation_unit: "radian".to_owned(),
            nominal_level_bps: 9_500,
            predicted_translation_m: [1.0, -2.0, 0.5],
            predicted_quaternion_xyzw: [0.0, 0.0, 0.0, 1.0],
            covariance_row_major: covariance,
            profile: profile(),
        }
    }

    #[test]
    fn matched_profile_constructs_statistical_outer_region() {
        let result = evaluate(&input());
        assert_eq!(
            result.disposition,
            Disposition::CandidateStatisticalPoseRegion
        );
        assert!(result.region_is_statistical);
        assert!(result.outer_approximation_of_joint_tangent_region);
        assert!(!result.deterministic_safety_bound);
        assert!(!result.physical_calibration_validated);
        assert!(!result.source_authentication_verified_here);
        assert!(!result.source_bytes_authenticated);
        assert!(!result.maneuver_authority_granted);
        let region = result.region.expect("candidate must carry a region");
        assert_eq!(
            region.translation_bounds_um,
            [
                [997_999, 1_002_001],
                [-2_002_001, -1_997_999],
                [497_999, 502_001]
            ]
        );
        assert_eq!(region.orientation_cap_urad, 4_001);
    }

    #[test]
    fn finite_sample_rank_uses_ceiling_of_n_plus_one_times_level() {
        let mut value = input();
        value.profile.sample_count = 1_000;
        value.profile.order_statistic_rank = 951;
        assert_eq!(
            evaluate(&value).disposition,
            Disposition::CandidateStatisticalPoseRegion
        );
        value.profile.order_statistic_rank = 950;
        assert_eq!(evaluate(&value).disposition, Disposition::InvalidInput);
    }

    #[test]
    fn underpowered_profile_abstains_even_if_a_quantile_is_supplied() {
        let mut value = input();
        value.profile.sample_count = 99;
        value.profile.order_statistic_rank = 95;
        assert_eq!(
            evaluate(&value).disposition,
            Disposition::AbstainUncalibrated
        );
    }

    #[test]
    fn profile_scope_mismatch_is_out_of_scope() {
        let mut value = input();
        value.profile.model_sha256 = "9".repeat(64);
        assert_eq!(evaluate(&value).disposition, Disposition::OutOfScope);
    }

    #[test]
    fn heldout_coverage_below_profile_floor_abstains() {
        let mut value = input();
        value.profile.heldout_coverage_lower_ci_ppm = 899_999;
        assert_eq!(
            evaluate(&value).disposition,
            Disposition::AbstainUncalibrated
        );
    }

    #[test]
    fn stale_profile_abstains() {
        let mut value = input();
        value.profile.profile_state = "STALE".to_owned();
        assert_eq!(
            evaluate(&value).disposition,
            Disposition::AbstainUncalibrated
        );
    }

    #[test]
    fn wrong_score_or_perturbation_convention_is_out_of_scope() {
        let mut value = input();
        value.profile.score_definition = "GAUSSIAN_CHI_SQUARE".to_owned();
        assert_eq!(evaluate(&value).disposition, Disposition::OutOfScope);
        let mut value = input();
        value.profile.perturbation_convention = "LEFT_COMPOSED".to_owned();
        assert_eq!(evaluate(&value).disposition, Disposition::OutOfScope);
    }

    #[test]
    fn bad_or_overlapping_split_digests_are_invalid() {
        let mut value = input();
        value.profile.heldout_split_sha256 = value.profile.calibration_split_sha256.clone();
        assert_eq!(evaluate(&value).disposition, Disposition::InvalidInput);
    }

    #[test]
    fn absent_upstream_authentication_claim_abstains_without_upgrading_it() {
        let mut value = input();
        value.upstream_source_bytes_authenticated = false;
        let result = evaluate(&value);
        assert_eq!(result.disposition, Disposition::AbstainUncalibrated);
        assert!(!result.source_authentication_verified_here);
        assert!(!result.source_bytes_authenticated);
    }

    #[test]
    fn runtime_authority_escalation_is_invalid() {
        let mut value = input();
        value.deterministic_safety_bound = true;
        assert_eq!(evaluate(&value).disposition, Disposition::InvalidInput);
    }

    #[test]
    fn a_non_candidate_runtime_pose_cannot_be_promoted() {
        let mut value = input();
        value.disposition = "ABSTAIN_UNOBSERVABLE".to_owned();
        value.candidate_only = false;
        assert_eq!(
            evaluate(&value).disposition,
            Disposition::AbstainRuntimePoseNotCandidate
        );
    }

    #[test]
    fn correlated_covariance_is_preserved_and_conservatively_enclosed() {
        let mut value = input();
        value.covariance_row_major[3] = 1.0e-6;
        value.covariance_row_major[18] = 1.0e-6;
        let region = evaluate(&value)
            .region
            .expect("correlated SPD input remains admissible");
        assert!(region.translation_bounds_um[0][0] < 998_000);
        assert!(region.orientation_cap_urad > 4_000);
    }

    #[test]
    fn quaternion_sign_does_not_change_the_constructed_region() {
        let positive = input();
        let mut negative = input();
        negative.predicted_quaternion_xyzw = [0.0, 0.0, 0.0, -1.0];
        assert_eq!(evaluate(&positive).region, evaluate(&negative).region);
    }

    #[test]
    fn orientation_radius_is_capped_at_pi() {
        let mut value = input();
        for axis in 3..6 {
            value.covariance_row_major[axis * 6 + axis] = 9.0;
        }
        let region = evaluate(&value).region.expect("valid wide region");
        assert_eq!(region.orientation_cap_urad, 3_141_593);
    }

    #[test]
    fn negative_mean_translation_is_rounded_outward() {
        let mut value = input();
        value.predicted_translation_m = [-1.234_567, -0.000_001, -3.0];
        let region = evaluate(&value).region.expect("valid negative means");
        assert!(region.translation_bounds_um[0][0] <= -1_236_567);
        assert!(region.translation_bounds_um[0][1] >= -1_232_567);
    }

    #[test]
    fn asymmetric_or_non_spd_covariance_is_rejected() {
        let mut asymmetric = input();
        asymmetric.covariance_row_major[1] = 1.0e-4;
        assert_eq!(
            evaluate(&asymmetric).disposition,
            Disposition::InvalidCovariance
        );
        let mut negative = input();
        negative.covariance_row_major[0] = -1.0;
        assert_eq!(
            evaluate(&negative).disposition,
            Disposition::InvalidCovariance
        );
        let mut singular = input();
        singular.covariance_row_major[0] = 0.0;
        assert_eq!(
            evaluate(&singular).disposition,
            Disposition::InvalidCovariance
        );
    }

    #[test]
    fn even_small_covariance_asymmetry_is_rejected() {
        let mut value = input();
        value.covariance_row_major[1] = 1.0e-18;
        value.covariance_row_major[6] = 1.0e-18 * (1.0 + 5.0e-13);
        assert_eq!(evaluate(&value).disposition, Disposition::InvalidCovariance);
    }

    #[test]
    fn nonfinite_or_overflowing_inputs_never_produce_a_region() {
        let mut nan = input();
        nan.covariance_row_major[0] = f64::NAN;
        assert_eq!(evaluate(&nan).disposition, Disposition::InvalidCovariance);
        let mut huge = input();
        for axis in 0..3 {
            huge.covariance_row_major[axis * 6 + axis] = 1.0e308;
        }
        assert_eq!(evaluate(&huge).disposition, Disposition::Overflow);
    }

    #[test]
    fn duplicate_samples_invalidate_all_manifest_occurrences() {
        let first = input();
        let mut second = input();
        second.case_id = "REG-002".to_owned();
        let results = evaluate_manifest(&[first, second]);
        assert!(
            results
                .iter()
                .all(|result| result.disposition == Disposition::DuplicateSample)
        );
    }

    #[test]
    fn minimum_calibration_sample_floor_abstains_before_rank_evaluation() {
        let mut value = input();
        value.profile.sample_count = 18;
        value.profile.order_statistic_rank = 19;
        assert_eq!(
            evaluate(&value).disposition,
            Disposition::AbstainUncalibrated
        );
    }

    #[test]
    fn zero_quantile_is_a_degenerate_but_well_defined_region() {
        let mut value = input();
        value.profile.quantile_radius_q32_32 = 0;
        let result = evaluate(&value);
        assert_eq!(
            result.disposition,
            Disposition::CandidateStatisticalPoseRegion
        );
        assert!(result.region.is_some());
    }

    #[test]
    fn invalid_quaternion_and_bad_digest_are_rejected() {
        let mut value = input();
        value.predicted_quaternion_xyzw = [0.0; 4];
        assert_eq!(evaluate(&value).disposition, Disposition::InvalidInput);
        let mut value = input();
        value.receipt_sha256 = "bad".to_owned();
        assert_eq!(evaluate(&value).disposition, Disposition::InvalidInput);
    }

    #[test]
    fn profile_quantile_cannot_silently_overflow_float_conversion() {
        let mut value = input();
        value.profile.quantile_radius_q32_32 = u64::MAX;
        assert_eq!(evaluate(&value).disposition, Disposition::InvalidInput);
    }

    #[test]
    fn wrong_level_or_tangent_convention_is_out_of_scope() {
        let mut value = input();
        value.profile.nominal_level_bps = 9_000;
        assert_eq!(evaluate(&value).disposition, Disposition::OutOfScope);
        let mut value = input();
        value.tangent_convention = "WORLD_LOCAL_TANGENT".to_owned();
        assert_eq!(evaluate(&value).disposition, Disposition::OutOfScope);
    }
}
