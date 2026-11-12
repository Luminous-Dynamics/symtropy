//! Independent qualification of inter-pass scan receipts.
//!
//! V0.1 is synthetic-only. It validates a measurement receipt and supplies a
//! deterministic 2D rigid-transform known-answer oracle. It does not parse or
//! certify a physical laser-profiler point cloud, qualify welding procedures,
//! or authorize robotic execution.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticOnly,
    Measured,
    Simulated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptError {
    EmptyIdentity,
    EvidenceClassNotAdmitted,
    SourceDigestMalformed,
    TransformDigestMalformed,
    SourceDigestMismatch,
    IdentityMismatch,
    InvalidPolicy,
    ProfileCountOverflow,
    ResidualMetricsInconsistent,
    TransformDigestMismatch,
    SourceFrameMismatch,
    TargetFrameMismatch,
    CalibrationMissing,
    AlgorithmMissing,
    CaptureFromFuture,
    ReceiptStale,
    InvalidProfileCounts,
    CoverageOutOfRange,
    CoverageBelowMinimum,
    OcclusionOutOfRange,
    OcclusionAboveMaximum,
    ResidualRmsAboveLimit,
    ResidualP95AboveLimit,
    ResidualMaximumAboveLimit,
    UncertaintyMissing,
    UncertaintyAboveLimit,
    ConfidenceBelowMinimum,
    InvalidTransform,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitError {
    TooFewCorrespondences,
    NonFinitePoint,
    DegenerateGeometry,
    NumericFailure,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidTransform2 {
    /// Row-major homogeneous transform from source frame to target frame.
    pub matrix: [f64; 9],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Correspondence2 {
    pub source: [f64; 2],
    pub target: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitMetrics {
    pub translation_magnitude_units: f64,
    pub rotation_radians: f64,
    pub residual_rms_units: f64,
    pub residual_p95_units: f64,
    pub residual_max_units: f64,
    pub correspondence_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedContext {
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub predecessor_pass_id: String,
    pub scan_id: String,
    pub source_scan_sha256: String,
    pub registration_receipt_sha256: String,
    pub source_frame_id: String,
    pub target_frame_id: String,
    pub decision_tick: u64,
    pub maximum_age_ticks: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceiptPolicy {
    pub minimum_matched_profiles: u64,
    pub minimum_coverage_permille: u16,
    pub maximum_occlusion_permille: u16,
    pub maximum_residual_rms_microunits: u64,
    pub maximum_residual_p95_microunits: u64,
    pub maximum_residual_microunits: u64,
    pub maximum_uncertainty_microunits: u64,
    pub minimum_confidence_permille: u16,
    pub transform_tolerance_ppb: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScanReceipt {
    pub evidence_class: EvidenceClass,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub predecessor_pass_id: String,
    pub scan_id: String,
    pub source_scan_sha256: String,
    pub registration_receipt_sha256: String,
    pub source_frame_id: String,
    pub target_frame_id: String,
    pub calibration_id: String,
    pub calibration_revision: String,
    pub algorithm_id: String,
    pub algorithm_revision: String,
    pub capture_tick: u64,
    pub expected_profile_count: u64,
    pub acquired_profile_count: u64,
    pub matched_profile_count: u64,
    pub rejected_profile_count: u64,
    pub occluded_profile_count: u64,
    pub coverage_permille: u16,
    pub occlusion_permille: u16,
    pub residual_rms_microunits: u64,
    pub residual_p95_microunits: u64,
    pub residual_max_microunits: u64,
    pub uncertainty_95_microunits: u64,
    pub confidence_permille: u16,
    pub transform: RigidTransform2,
}

fn is_empty(value: &str) -> bool {
    value.trim().is_empty()
}

pub fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl RigidTransform2 {
    pub fn identity() -> Self {
        Self {
            matrix: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn apply(&self, point: [f64; 2]) -> [f64; 2] {
        [
            self.matrix[0] * point[0] + self.matrix[1] * point[1] + self.matrix[2],
            self.matrix[3] * point[0] + self.matrix[4] * point[1] + self.matrix[5],
        ]
    }

    pub fn is_proper_rigid(&self, tolerance: f64) -> bool {
        if !self.matrix.iter().all(|value| value.is_finite())
            || !tolerance.is_finite()
            || tolerance <= 0.0
        {
            return false;
        }
        let m = self.matrix;
        let row3_ok = m[6].abs() <= tolerance
            && m[7].abs() <= tolerance
            && (m[8] - 1.0).abs() <= tolerance;
        let col0_norm = m[0] * m[0] + m[3] * m[3];
        let col1_norm = m[1] * m[1] + m[4] * m[4];
        let col_dot = m[0] * m[1] + m[3] * m[4];
        let determinant = m[0] * m[4] - m[1] * m[3];
        row3_ok
            && (col0_norm - 1.0).abs() <= tolerance
            && (col1_norm - 1.0).abs() <= tolerance
            && col_dot.abs() <= tolerance
            && (determinant - 1.0).abs() <= tolerance
    }
}

pub fn fit_rigid_transform_2d(
    correspondences: &[Correspondence2],
) -> Result<(RigidTransform2, FitMetrics), FitError> {
    if correspondences.len() < 3 {
        return Err(FitError::TooFewCorrespondences);
    }
    if correspondences.iter().any(|pair| {
        pair.source
            .iter()
            .chain(pair.target.iter())
            .any(|value| !value.is_finite())
    }) {
        return Err(FitError::NonFinitePoint);
    }

    let count = correspondences.len() as f64;
    let source_centroid = correspondences.iter().fold([0.0, 0.0], |mut acc, pair| {
        acc[0] += pair.source[0] / count;
        acc[1] += pair.source[1] / count;
        acc
    });
    let target_centroid = correspondences.iter().fold([0.0, 0.0], |mut acc, pair| {
        acc[0] += pair.target[0] / count;
        acc[1] += pair.target[1] / count;
        acc
    });

    let mut sxx = 0.0;
    let mut syy = 0.0;
    let mut sxy = 0.0;
    let mut dot = 0.0;
    let mut cross = 0.0;
    for pair in correspondences {
        let sx = pair.source[0] - source_centroid[0];
        let sy = pair.source[1] - source_centroid[1];
        let tx = pair.target[0] - target_centroid[0];
        let ty = pair.target[1] - target_centroid[1];
        sxx += sx * sx;
        syy += sy * sy;
        sxy += sx * sy;
        dot += sx * tx + sy * ty;
        cross += sx * ty - sy * tx;
    }
    let source_covariance_det = sxx * syy - sxy * sxy;
    if !source_covariance_det.is_finite() || source_covariance_det <= 1.0e-12 {
        return Err(FitError::DegenerateGeometry);
    }

    let angle = cross.atan2(dot);
    if !angle.is_finite() {
        return Err(FitError::NumericFailure);
    }
    let cos = angle.cos();
    let sin = angle.sin();
    let tx = target_centroid[0] - (cos * source_centroid[0] - sin * source_centroid[1]);
    let ty = target_centroid[1] - (sin * source_centroid[0] + cos * source_centroid[1]);
    let transform = RigidTransform2 {
        matrix: [cos, -sin, tx, sin, cos, ty, 0.0, 0.0, 1.0],
    };
    if !transform.is_proper_rigid(1.0e-9) {
        return Err(FitError::NumericFailure);
    }

    let mut residuals: Vec<f64> = correspondences
        .iter()
        .map(|pair| {
            let estimate = transform.apply(pair.source);
            (estimate[0] - pair.target[0]).hypot(estimate[1] - pair.target[1])
        })
        .collect();
    if residuals.iter().any(|value| !value.is_finite()) {
        return Err(FitError::NumericFailure);
    }
    let sum_squares = residuals.iter().map(|value| value * value).sum::<f64>();
    let rms = (sum_squares / count).sqrt();
    residuals.sort_by(f64::total_cmp);
    let p95_index = ((residuals.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(residuals.len() - 1);
    let max = residuals.last().copied().unwrap_or(0.0);

    let metrics = FitMetrics {
        translation_magnitude_units: tx.hypot(ty),
        rotation_radians: angle,
        residual_rms_units: rms,
        residual_p95_units: residuals[p95_index],
        residual_max_units: max,
        correspondence_count: correspondences.len(),
    };
    Ok((transform, metrics))
}

pub fn validate_receipt(
    receipt: &ScanReceipt,
    expected: &ExpectedContext,
    policy: ReceiptPolicy,
) -> Result<(), ReceiptError> {
    if policy.minimum_coverage_permille > 1_000
        || policy.maximum_occlusion_permille > 1_000
        || policy.maximum_uncertainty_microunits == 0
        || policy.minimum_confidence_permille < 950
        || policy.minimum_confidence_permille > 1_000
        || policy.transform_tolerance_ppb == 0
    {
        return Err(ReceiptError::InvalidPolicy);
    }
    let identities = [
        receipt.workpiece_id.as_str(),
        receipt.joint_revision.as_str(),
        receipt.layer_id.as_str(),
        receipt.predecessor_pass_id.as_str(),
        receipt.scan_id.as_str(),
        receipt.source_frame_id.as_str(),
        receipt.target_frame_id.as_str(),
        receipt.calibration_id.as_str(),
        receipt.calibration_revision.as_str(),
        receipt.algorithm_id.as_str(),
        receipt.algorithm_revision.as_str(),
        expected.workpiece_id.as_str(),
        expected.joint_revision.as_str(),
        expected.layer_id.as_str(),
        expected.predecessor_pass_id.as_str(),
        expected.scan_id.as_str(),
        expected.source_frame_id.as_str(),
        expected.target_frame_id.as_str(),
    ];
    if identities.into_iter().any(is_empty) {
        return Err(ReceiptError::EmptyIdentity);
    }
    if receipt.evidence_class != EvidenceClass::SyntheticOnly {
        return Err(ReceiptError::EvidenceClassNotAdmitted);
    }
    if !is_lower_hex_sha256(&receipt.source_scan_sha256) {
        return Err(ReceiptError::SourceDigestMalformed);
    }
    if !is_lower_hex_sha256(&receipt.registration_receipt_sha256) {
        return Err(ReceiptError::TransformDigestMalformed);
    }
    if receipt.source_scan_sha256 != expected.source_scan_sha256 {
        return Err(ReceiptError::SourceDigestMismatch);
    }
    if receipt.registration_receipt_sha256 != expected.registration_receipt_sha256 {
        return Err(ReceiptError::TransformDigestMismatch);
    }
    if receipt.source_frame_id != expected.source_frame_id {
        return Err(ReceiptError::SourceFrameMismatch);
    }
    if receipt.target_frame_id != expected.target_frame_id {
        return Err(ReceiptError::TargetFrameMismatch);
    }
    if receipt.workpiece_id != expected.workpiece_id
        || receipt.joint_revision != expected.joint_revision
        || receipt.layer_id != expected.layer_id
        || receipt.predecessor_pass_id != expected.predecessor_pass_id
        || receipt.scan_id != expected.scan_id
    {
        return Err(ReceiptError::IdentityMismatch);
    }
    if receipt.capture_tick > expected.decision_tick {
        return Err(ReceiptError::CaptureFromFuture);
    }
    if expected.decision_tick.saturating_sub(receipt.capture_tick) > expected.maximum_age_ticks {
        return Err(ReceiptError::ReceiptStale);
    }
    let acquired_plus_occluded = receipt
        .acquired_profile_count
        .checked_add(receipt.occluded_profile_count)
        .ok_or(ReceiptError::ProfileCountOverflow)?;
    let matched_plus_rejected = receipt
        .matched_profile_count
        .checked_add(receipt.rejected_profile_count)
        .ok_or(ReceiptError::ProfileCountOverflow)?;
    if receipt.expected_profile_count == 0
        || receipt.acquired_profile_count > receipt.expected_profile_count
        || receipt.occluded_profile_count > receipt.expected_profile_count
        || acquired_plus_occluded != receipt.expected_profile_count
        || matched_plus_rejected != receipt.acquired_profile_count
        || receipt.matched_profile_count < policy.minimum_matched_profiles
    {
        return Err(ReceiptError::InvalidProfileCounts);
    }
    if receipt.coverage_permille > 1_000 {
        return Err(ReceiptError::CoverageOutOfRange);
    }
    if receipt.coverage_permille < policy.minimum_coverage_permille {
        return Err(ReceiptError::CoverageBelowMinimum);
    }
    if receipt.occlusion_permille > 1_000 {
        return Err(ReceiptError::OcclusionOutOfRange);
    }
    if receipt.occlusion_permille > policy.maximum_occlusion_permille {
        return Err(ReceiptError::OcclusionAboveMaximum);
    }
    if receipt.residual_rms_microunits > policy.maximum_residual_rms_microunits {
        return Err(ReceiptError::ResidualRmsAboveLimit);
    }
    if receipt.residual_p95_microunits > policy.maximum_residual_p95_microunits {
        return Err(ReceiptError::ResidualP95AboveLimit);
    }
    if receipt.residual_max_microunits > policy.maximum_residual_microunits {
        return Err(ReceiptError::ResidualMaximumAboveLimit);
    }
    if receipt.residual_max_microunits < receipt.residual_rms_microunits
        || receipt.residual_max_microunits < receipt.residual_p95_microunits
    {
        return Err(ReceiptError::ResidualMetricsInconsistent);
    }
    if !receipt.transform.is_proper_rigid(policy.transform_tolerance_ppb as f64 / 1.0e9) {
        return Err(ReceiptError::InvalidTransform);
    }
    if receipt.uncertainty_95_microunits == 0 {
        return Err(ReceiptError::UncertaintyMissing);
    }
    if receipt.uncertainty_95_microunits > policy.maximum_uncertainty_microunits {
        return Err(ReceiptError::UncertaintyAboveLimit);
    }
    if receipt.confidence_permille < policy.minimum_confidence_permille
        || receipt.confidence_permille > 1_000
    {
        return Err(ReceiptError::ConfidenceBelowMinimum);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_hex(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn policy() -> ReceiptPolicy {
        ReceiptPolicy {
            minimum_matched_profiles: 90,
            minimum_coverage_permille: 950,
            maximum_occlusion_permille: 50,
            maximum_residual_rms_microunits: 100,
            maximum_residual_p95_microunits: 200,
            maximum_residual_microunits: 300,
            maximum_uncertainty_microunits: 500,
            minimum_confidence_permille: 950,
            transform_tolerance_ppb: 100,
        }
    }

    fn context() -> ExpectedContext {
        ExpectedContext {
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            layer_id: "LAYER-02".to_owned(),
            predecessor_pass_id: "PASS-01".to_owned(),
            scan_id: "SCAN-POST-PASS-01".to_owned(),
            source_scan_sha256: seed_hex('1'),
            registration_receipt_sha256: seed_hex('2'),
            source_frame_id: "SCAN-FRAME-A".to_owned(),
            target_frame_id: "JOINT-FRAME-R1".to_owned(),
            decision_tick: 1_000,
            maximum_age_ticks: 25,
        }
    }

    fn valid_receipt() -> ScanReceipt {
        ScanReceipt {
            evidence_class: EvidenceClass::SyntheticOnly,
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            layer_id: "LAYER-02".to_owned(),
            predecessor_pass_id: "PASS-01".to_owned(),
            scan_id: "SCAN-POST-PASS-01".to_owned(),
            source_scan_sha256: seed_hex('1'),
            registration_receipt_sha256: seed_hex('2'),
            source_frame_id: "SCAN-FRAME-A".to_owned(),
            target_frame_id: "JOINT-FRAME-R1".to_owned(),
            calibration_id: "CAL-LASER-01".to_owned(),
            calibration_revision: "CAL-R3".to_owned(),
            algorithm_id: "RIGID-2D-KABSCH".to_owned(),
            algorithm_revision: "0.1.0".to_owned(),
            capture_tick: 990,
            expected_profile_count: 100,
            acquired_profile_count: 98,
            matched_profile_count: 95,
            rejected_profile_count: 3,
            occluded_profile_count: 2,
            coverage_permille: 970,
            occlusion_permille: 20,
            residual_rms_microunits: 50,
            residual_p95_microunits: 120,
            residual_max_microunits: 240,
            uncertainty_95_microunits: 250,
            confidence_permille: 950,
            transform: RigidTransform2::identity(),
        }
    }

    fn asymmetric_points() -> Vec<[f64; 2]> {
        vec![
            [0.0, 0.0],
            [2.0, 0.0],
            [0.0, 3.0],
            [4.0, 1.0],
            [1.0, 5.0],
            [6.0, 4.0],
        ]
    }

    fn transformed_pairs(angle: f64, translation: [f64; 2]) -> Vec<Correspondence2> {
        let cos = angle.cos();
        let sin = angle.sin();
        asymmetric_points()
            .into_iter()
            .map(|source| Correspondence2 {
                source,
                target: [
                    cos * source[0] - sin * source[1] + translation[0],
                    sin * source[0] + cos * source[1] + translation[1],
                ],
            })
            .collect()
    }

    fn assert_receipt_error(mutate: impl FnOnce(&mut ScanReceipt), expected: ReceiptError) {
        let mut receipt = valid_receipt();
        mutate(&mut receipt);
        assert_eq!(validate_receipt(&receipt, &context(), policy()), Err(expected));
    }

    #[test]
    fn valid_synthetic_receipt_is_admitted() {
        assert_eq!(validate_receipt(&valid_receipt(), &context(), policy()), Ok(()));
    }

    #[test]
    fn known_answer_identity_transform() {
        let pairs = transformed_pairs(0.0, [0.0, 0.0]);
        let (fit, metrics) = fit_rigid_transform_2d(&pairs).expect("fit");
        assert!((fit.matrix[0] - 1.0).abs() < 1.0e-10);
        assert!(metrics.residual_max_units < 1.0e-10);
    }

    #[test]
    fn known_answer_rotated_translated_transform() {
        let pairs = transformed_pairs(0.37, [12.5, -8.25]);
        let (fit, metrics) = fit_rigid_transform_2d(&pairs).expect("fit");
        assert!((fit.matrix[0] - 0.37_f64.cos()).abs() < 1.0e-10);
        assert!((fit.matrix[3] - 0.37_f64.sin()).abs() < 1.0e-10);
        assert!((fit.matrix[2] - 12.5).abs() < 1.0e-10);
        assert!((fit.matrix[5] + 8.25).abs() < 1.0e-10);
        assert!(metrics.residual_max_units < 1.0e-10);
    }

    #[test]
    fn known_answer_negative_rotation() {
        let pairs = transformed_pairs(-0.82, [-30.0, 4.75]);
        let (fit, metrics) = fit_rigid_transform_2d(&pairs).expect("fit");
        assert!((fit.matrix[0] - (-0.82_f64).cos()).abs() < 1.0e-10);
        assert!((fit.matrix[3] - (-0.82_f64).sin()).abs() < 1.0e-10);
        assert!(metrics.residual_max_units < 1.0e-10);
    }

    #[test]
    fn known_answer_large_translation() {
        let pairs = transformed_pairs(1.21, [1_000.0, -500.0]);
        let (fit, metrics) = fit_rigid_transform_2d(&pairs).expect("fit");
        assert!((fit.matrix[2] - 1_000.0).abs() < 1.0e-9);
        assert!((fit.matrix[5] + 500.0).abs() < 1.0e-9);
        assert!(metrics.residual_max_units < 1.0e-9);
    }

    #[test]
    fn too_few_correspondences_are_rejected() {
        assert_eq!(fit_rigid_transform_2d(&[]), Err(FitError::TooFewCorrespondences));
    }

    #[test]
    fn collinear_correspondences_are_rejected() {
        let pairs = vec![
            Correspondence2 { source: [0.0, 0.0], target: [1.0, 1.0] },
            Correspondence2 { source: [1.0, 1.0], target: [2.0, 2.0] },
            Correspondence2 { source: [2.0, 2.0], target: [3.0, 3.0] },
        ];
        assert_eq!(fit_rigid_transform_2d(&pairs), Err(FitError::DegenerateGeometry));
    }

    #[test]
    fn nonfinite_correspondence_is_rejected() {
        let mut pairs = transformed_pairs(0.1, [1.0, 2.0]);
        pairs[0].source[0] = f64::NAN;
        assert_eq!(fit_rigid_transform_2d(&pairs), Err(FitError::NonFinitePoint));
    }

    #[test]
    fn receipt_rejects_measured_data_in_synthetic_only_v0() {
        assert_receipt_error(
            |r| r.evidence_class = EvidenceClass::Measured,
            ReceiptError::EvidenceClassNotAdmitted,
        );
    }

    #[test]
    fn receipt_rejects_empty_calibration_identity() {
        assert_receipt_error(|r| r.calibration_id.clear(), ReceiptError::EmptyIdentity);
    }

    #[test]
    fn receipt_rejects_malformed_source_digest() {
        assert_receipt_error(|r| r.source_scan_sha256 = "abc".to_owned(), ReceiptError::SourceDigestMalformed);
    }

    #[test]
    fn receipt_rejects_source_digest_mismatch() {
        assert_receipt_error(|r| r.source_scan_sha256 = seed_hex('3'), ReceiptError::SourceDigestMismatch);
    }

    #[test]
    fn receipt_rejects_transform_digest_mismatch() {
        assert_receipt_error(|r| r.registration_receipt_sha256 = seed_hex('4'), ReceiptError::TransformDigestMismatch);
    }

    #[test]
    fn receipt_rejects_frame_mismatch() {
        assert_receipt_error(|r| r.source_frame_id = "OTHER-FRAME".to_owned(), ReceiptError::SourceFrameMismatch);
    }

    #[test]
    fn receipt_rejects_capture_from_future() {
        assert_receipt_error(|r| r.capture_tick = 1_001, ReceiptError::CaptureFromFuture);
    }

    #[test]
    fn receipt_rejects_stale_scan() {
        assert_receipt_error(|r| r.capture_tick = 900, ReceiptError::ReceiptStale);
    }

    #[test]
    fn receipt_rejects_profile_count_inconsistency() {
        assert_receipt_error(|r| r.matched_profile_count = 99, ReceiptError::InvalidProfileCounts);
    }

    #[test]
    fn receipt_rejects_coverage_below_limit() {
        assert_receipt_error(|r| r.coverage_permille = 900, ReceiptError::CoverageBelowMinimum);
    }

    #[test]
    fn receipt_rejects_occlusion_above_limit() {
        assert_receipt_error(|r| r.occlusion_permille = 100, ReceiptError::OcclusionAboveMaximum);
    }

    #[test]
    fn receipt_rejects_excessive_rms_residual() {
        assert_receipt_error(|r| r.residual_rms_microunits = 101, ReceiptError::ResidualRmsAboveLimit);
    }

    #[test]
    fn receipt_rejects_excessive_p95_residual() {
        assert_receipt_error(|r| r.residual_p95_microunits = 201, ReceiptError::ResidualP95AboveLimit);
    }

    #[test]
    fn receipt_rejects_excessive_max_residual() {
        assert_receipt_error(|r| r.residual_max_microunits = 301, ReceiptError::ResidualMaximumAboveLimit);
    }

    #[test]
    fn receipt_rejects_zero_uncertainty() {
        assert_receipt_error(|r| r.uncertainty_95_microunits = 0, ReceiptError::UncertaintyMissing);
    }

    #[test]
    fn receipt_rejects_uncertainty_above_policy() {
        assert_receipt_error(|r| r.uncertainty_95_microunits = 501, ReceiptError::UncertaintyAboveLimit);
    }

    #[test]
    fn receipt_rejects_confidence_below_policy() {
        assert_receipt_error(|r| r.confidence_permille = 949, ReceiptError::ConfidenceBelowMinimum);
    }

    #[test]
    fn receipt_rejects_reflection_transform() {
        assert_receipt_error(
            |r| r.transform.matrix = [-1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            ReceiptError::InvalidTransform,
        );
    }

    #[test]
    fn receipt_rejects_scale_transform() {
        assert_receipt_error(
            |r| r.transform.matrix = [1.01, 0.0, 0.0, 0.0, 1.01, 0.0, 0.0, 0.0, 1.0],
            ReceiptError::InvalidTransform,
        );
    }

    #[test]
    fn receipt_rejects_shear_transform() {
        assert_receipt_error(
            |r| r.transform.matrix = [1.0, 0.1, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            ReceiptError::InvalidTransform,
        );
    }

    #[test]
    fn receipt_rejects_wrong_homogeneous_row() {
        assert_receipt_error(
            |r| r.transform.matrix[6] = 0.1,
            ReceiptError::InvalidTransform,
        );
    }

    #[test]
    fn receipt_rejects_wrong_predecessor_identity() {
        let mut receipt = valid_receipt();
        receipt.predecessor_pass_id = "PASS-OLD".to_owned();
        assert_eq!(validate_receipt(&receipt, &context(), policy()), Err(ReceiptError::IdentityMismatch));
    }

    #[test]
    fn receipt_rejects_coverage_out_of_range() {
        assert_receipt_error(|r| r.coverage_permille = 1_001, ReceiptError::CoverageOutOfRange);
    }

    #[test]
    fn receipt_rejects_occlusion_out_of_range() {
        assert_receipt_error(|r| r.occlusion_permille = 1_001, ReceiptError::OcclusionOutOfRange);
    }

    #[test]
    fn receipt_rejects_target_frame_mismatch() {
        assert_receipt_error(|r| r.target_frame_id = "OTHER-TARGET".to_owned(), ReceiptError::TargetFrameMismatch);
    }

    #[test]
    fn receipt_rejects_malformed_transform_digest() {
        assert_receipt_error(|r| r.registration_receipt_sha256 = "xyz".to_owned(), ReceiptError::TransformDigestMalformed);
    }

    #[test]
    fn receipt_rejects_profile_count_overflow() {
        assert_receipt_error(
            |r| {
                r.expected_profile_count = u64::MAX;
                r.acquired_profile_count = u64::MAX;
                r.occluded_profile_count = 1;
            },
            ReceiptError::ProfileCountOverflow,
        );
    }

    #[test]
    fn receipt_rejects_zero_expected_profile_count() {
        assert_receipt_error(
            |r| {
                r.expected_profile_count = 0;
                r.acquired_profile_count = 0;
                r.matched_profile_count = 0;
                r.rejected_profile_count = 0;
                r.occluded_profile_count = 0;
            },
            ReceiptError::InvalidProfileCounts,
        );
    }

    #[test]
    fn receipt_rejects_insufficient_matched_profiles() {
        assert_receipt_error(
            |r| {
                r.acquired_profile_count = 90;
                r.matched_profile_count = 89;
                r.rejected_profile_count = 1;
                r.occluded_profile_count = 10;
            },
            ReceiptError::InvalidProfileCounts,
        );
    }

    #[test]
    fn receipt_rejects_nonfinite_transform_component() {
        assert_receipt_error(
            |r| r.transform.matrix[0] = f64::NAN,
            ReceiptError::InvalidTransform,
        );
    }

    #[test]
    fn receipt_rejects_inconsistent_residual_summary() {
        assert_receipt_error(
            |r| {
                r.residual_rms_microunits = 90;
                r.residual_max_microunits = 80;
            },
            ReceiptError::ResidualMetricsInconsistent,
        );
    }

    #[test]
    fn receipt_rejects_invalid_policy_bounds() {
        let mut policy = policy();
        policy.maximum_occlusion_permille = 1_001;
        assert_eq!(
            validate_receipt(&valid_receipt(), &context(), policy),
            Err(ReceiptError::InvalidPolicy)
        );
    }

    #[test]
    fn receipt_rejects_rejected_count_inconsistency() {
        assert_receipt_error(|r| r.rejected_profile_count = 2, ReceiptError::InvalidProfileCounts);
    }
}
