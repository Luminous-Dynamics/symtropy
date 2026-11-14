//! Fail-closed paired-profile metrology for synthetic inter-pass scan evidence.
//!
//! This v0 accepts synthetic contracts only. It does not parse BAM scans,
//! register physical point clouds, infer deposited metal volume, or qualify
//! a weld. Measurement uncertainty is conservatively combined by upper-bound
//! addition unless covariance evidence is separately qualified.

use serde::{Deserialize, Serialize};

pub const BENCHMARK_ID: &str = "SHIPYARD-SCAN-METROLOGY-001";
pub const CLAIM_CEILING: &str = "SURFACE_ENVELOPE_CHANGE_ONLY";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    SyntheticContractOnly,
    MeasuredAuthorized,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ScanIdentity {
    pub sha256: String,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub profile_station_id: String,
    pub coordinate_frame: String,
    pub instrument_id: String,
    pub calibration_revision: String,
    pub captured_at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CalibrationReceipt {
    pub sha256: String,
    pub instrument_id: String,
    pub calibration_revision: String,
    pub verified: bool,
    pub valid_from_ms: u64,
    pub valid_until_ms: u64,
    pub reference_artifact_error_um: u64,
    pub maximum_reference_artifact_error_um: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct RegistrationReceipt {
    pub sha256: String,
    pub before_scan_sha256: String,
    pub after_scan_sha256: String,
    pub transform_type: String,
    pub fit_region: String,
    pub stable_datum_verified: bool,
    pub handedness: String,
    pub check_point_count: u32,
    pub independent_check_point_error_um: u64,
    pub maximum_check_point_error_um: u64,
    pub scale_deviation_ppm: u64,
    pub maximum_scale_deviation_ppm: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct SurfaceCompatibilityReceipt {
    pub sha256: String,
    pub within_qualified_surface_range: bool,
    pub surface_condition: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct UncertaintyBudget {
    pub instrument_um: Option<u64>,
    pub calibration_um: Option<u64>,
    pub registration_um: Option<u64>,
    pub repeatability_um: Option<u64>,
    pub datum_um: Option<u64>,
    pub interpolation_um: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ProfileSample {
    pub x_um: u64,
    pub before_z_um: Option<i64>,
    pub after_z_um: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ProfilePairInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub evidence_class: EvidenceClass,
    pub pair_id: String,
    pub before_scan: ScanIdentity,
    pub after_scan: ScanIdentity,
    pub calibration_receipt: CalibrationReceipt,
    pub registration_receipt: RegistrationReceipt,
    pub surface_compatibility: SurfaceCompatibilityReceipt,
    pub uncertainty: UncertaintyBudget,
    pub maximum_accepted_uncertainty_um: u64,
    pub minimum_coverage_permille: u16,
    pub maximum_missing_run_samples: usize,
    pub profile_samples: Vec<ProfileSample>,
    pub claim_ceiling: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectReason {
    EmptyIdentity,
    WrongSchemaVersion,
    WrongBenchmarkId,
    EvidenceNotAdmitted,
    MalformedDigest,
    DifferentWorkpiece,
    DifferentJointRevision,
    DifferentLayer,
    DifferentProfileStation,
    DifferentCoordinateFrame,
    DifferentInstrument,
    DifferentCalibrationRevision,
    InvalidCaptureOrder,
    CalibrationNotVerified,
    CalibrationReceiptMismatch,
    CalibrationNotValidAtCapture,
    ReferenceArtifactOutOfTolerance,
    RegistrationReceiptMismatch,
    NonRigidTransform,
    UnstableDatum,
    RegistrationFitUsesChangingSurface,
    ImproperTransformHandedness,
    TooFewIndependentCheckPoints,
    CheckPointErrorTooLarge,
    ScaleDeviationTooLarge,
    SurfaceOutsideQualifiedRange,
    MissingUncertaintyComponent,
    UncertaintyOverflow,
    UncertaintyExceedsDecisionLimit,
    InvalidCoverageThreshold,
    TooFewProfileSamples,
    NonMonotonicProfileCoordinates,
    IncompletePairIdentity,
    CoverageBelowMinimum,
    MissingRunTooLong,
    NoValidMeasuredSpan,
    NumericOverflow,
    ClaimCeilingPromotion,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct MeasurementReport {
    pub pair_id: String,
    pub claim_ceiling: String,
    pub paired_sample_count: usize,
    pub expected_sample_count: usize,
    pub coverage_permille: u16,
    pub missing_sample_count: usize,
    pub maximum_missing_run_samples: usize,
    pub valid_span_count: usize,
    pub valid_span_length_um: u64,
    pub signed_area_change_um2: i64,
    pub absolute_area_change_um2: u64,
    pub mean_signed_displacement_um: i64,
    pub rmse_displacement_um: u64,
    pub maximum_absolute_displacement_um: u64,
    pub percentile_95_absolute_displacement_um: u64,
    pub combined_uncertainty_um: u64,
    pub mean_displacement_interval_um: [i64; 2],
    pub signed_area_interval_um2: [i64; 2],
}

fn is_empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn digest_is_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn checked_interval(center: i64, radius: u64) -> Result<[i64; 2], RejectReason> {
    let r = i64::try_from(radius).map_err(|_| RejectReason::NumericOverflow)?;
    let low = center.checked_sub(r).ok_or(RejectReason::NumericOverflow)?;
    let high = center.checked_add(r).ok_or(RejectReason::NumericOverflow)?;
    Ok([low, high])
}

fn checked_area_interval(center: i64, radius: u64) -> Result<[i64; 2], RejectReason> {
    checked_interval(center, radius)
}

fn integer_sqrt(value: u128) -> u128 {
    if value < 2 {
        return value;
    }
    let mut current = value;
    let mut next = (current + 1) / 2;
    while next < current {
        current = next;
        next = (current + value / current) / 2;
    }
    current
}

fn combine_uncertainty(budget: &UncertaintyBudget) -> Result<u64, RejectReason> {
    let values = [
        budget.instrument_um,
        budget.calibration_um,
        budget.registration_um,
        budget.repeatability_um,
        budget.datum_um,
        budget.interpolation_um,
    ];
    let mut total = 0_u64;
    for value in values {
        let component = value.ok_or(RejectReason::MissingUncertaintyComponent)?;
        total = total
            .checked_add(component)
            .ok_or(RejectReason::UncertaintyOverflow)?;
    }
    Ok(total)
}

fn validate_identity(input: &ProfilePairInput) -> Result<(), RejectReason> {
    if [
        input.pair_id.as_str(),
        input.before_scan.workpiece_id.as_str(),
        input.before_scan.joint_revision.as_str(),
        input.before_scan.layer_id.as_str(),
        input.before_scan.profile_station_id.as_str(),
        input.before_scan.coordinate_frame.as_str(),
        input.before_scan.instrument_id.as_str(),
        input.before_scan.calibration_revision.as_str(),
    ]
    .into_iter()
    .any(is_empty)
    {
        return Err(RejectReason::EmptyIdentity);
    }
    if input.schema_version != "0.1.0" {
        return Err(RejectReason::WrongSchemaVersion);
    }
    if input.benchmark_id != BENCHMARK_ID {
        return Err(RejectReason::WrongBenchmarkId);
    }
    if input.evidence_class != EvidenceClass::SyntheticContractOnly {
        return Err(RejectReason::EvidenceNotAdmitted);
    }
    if input.claim_ceiling != CLAIM_CEILING {
        return Err(RejectReason::ClaimCeilingPromotion);
    }

    for digest in [
        input.before_scan.sha256.as_str(),
        input.after_scan.sha256.as_str(),
        input.calibration_receipt.sha256.as_str(),
        input.registration_receipt.sha256.as_str(),
        input.surface_compatibility.sha256.as_str(),
    ] {
        if !digest_is_valid(digest) {
            return Err(RejectReason::MalformedDigest);
        }
    }

    if input.before_scan.workpiece_id != input.after_scan.workpiece_id {
        return Err(RejectReason::DifferentWorkpiece);
    }
    if input.before_scan.joint_revision != input.after_scan.joint_revision {
        return Err(RejectReason::DifferentJointRevision);
    }
    if input.before_scan.layer_id != input.after_scan.layer_id {
        return Err(RejectReason::DifferentLayer);
    }
    if input.before_scan.profile_station_id != input.after_scan.profile_station_id {
        return Err(RejectReason::DifferentProfileStation);
    }
    if input.before_scan.coordinate_frame != input.after_scan.coordinate_frame {
        return Err(RejectReason::DifferentCoordinateFrame);
    }
    if input.before_scan.instrument_id != input.after_scan.instrument_id {
        return Err(RejectReason::DifferentInstrument);
    }
    if input.before_scan.calibration_revision != input.after_scan.calibration_revision {
        return Err(RejectReason::DifferentCalibrationRevision);
    }
    if input.after_scan.captured_at_ms <= input.before_scan.captured_at_ms {
        return Err(RejectReason::InvalidCaptureOrder);
    }
    Ok(())
}

fn validate_calibration(input: &ProfilePairInput) -> Result<(), RejectReason> {
    let calibration = &input.calibration_receipt;
    if !calibration.verified {
        return Err(RejectReason::CalibrationNotVerified);
    }
    if calibration.instrument_id != input.before_scan.instrument_id
        || calibration.calibration_revision != input.before_scan.calibration_revision
    {
        return Err(RejectReason::CalibrationReceiptMismatch);
    }
    if calibration.valid_from_ms > input.before_scan.captured_at_ms
        || calibration.valid_until_ms < input.after_scan.captured_at_ms
    {
        return Err(RejectReason::CalibrationNotValidAtCapture);
    }
    if calibration.reference_artifact_error_um
        > calibration.maximum_reference_artifact_error_um
    {
        return Err(RejectReason::ReferenceArtifactOutOfTolerance);
    }
    Ok(())
}

fn validate_registration(input: &ProfilePairInput) -> Result<(), RejectReason> {
    let registration = &input.registration_receipt;
    if registration.before_scan_sha256 != input.before_scan.sha256
        || registration.after_scan_sha256 != input.after_scan.sha256
    {
        return Err(RejectReason::RegistrationReceiptMismatch);
    }
    if registration.transform_type != "rigid_2d" {
        return Err(RejectReason::NonRigidTransform);
    }
    if !registration.stable_datum_verified {
        return Err(RejectReason::UnstableDatum);
    }
    if registration.fit_region != "stable_datum"
        && registration.fit_region != "reference_artifact"
    {
        return Err(RejectReason::RegistrationFitUsesChangingSurface);
    }
    if registration.handedness != "proper" {
        return Err(RejectReason::ImproperTransformHandedness);
    }
    if registration.check_point_count < 3 {
        return Err(RejectReason::TooFewIndependentCheckPoints);
    }
    if registration.independent_check_point_error_um
        > registration.maximum_check_point_error_um
    {
        return Err(RejectReason::CheckPointErrorTooLarge);
    }
    if registration.scale_deviation_ppm > registration.maximum_scale_deviation_ppm {
        return Err(RejectReason::ScaleDeviationTooLarge);
    }
    Ok(())
}

fn validate_profile(input: &ProfilePairInput) -> Result<(), RejectReason> {
    if !input.surface_compatibility.within_qualified_surface_range {
        return Err(RejectReason::SurfaceOutsideQualifiedRange);
    }
    if input.minimum_coverage_permille == 0 || input.minimum_coverage_permille > 1_000 {
        return Err(RejectReason::InvalidCoverageThreshold);
    }
    if input.profile_samples.len() < 3 {
        return Err(RejectReason::TooFewProfileSamples);
    }
    for pair in input.profile_samples.windows(2) {
        if pair[1].x_um <= pair[0].x_um {
            return Err(RejectReason::NonMonotonicProfileCoordinates);
        }
    }
    Ok(())
}

pub fn evaluate_profile_pair(
    input: &ProfilePairInput,
) -> Result<MeasurementReport, RejectReason> {
    validate_identity(input)?;
    validate_calibration(input)?;
    validate_registration(input)?;
    validate_profile(input)?;

    let uncertainty = combine_uncertainty(&input.uncertainty)?;
    if input.maximum_accepted_uncertainty_um == 0 {
        return Err(RejectReason::UncertaintyExceedsDecisionLimit);
    }
    if uncertainty > input.maximum_accepted_uncertainty_um {
        return Err(RejectReason::UncertaintyExceedsDecisionLimit);
    }

    let expected = input.profile_samples.len();
    let mut valid_count = 0_usize;
    let mut missing_count = 0_usize;
    let mut current_missing_run = 0_usize;
    let mut maximum_missing_run = 0_usize;
    let mut valid_span_count = 0_usize;
    let mut valid_span_length_um = 0_u64;
    let mut signed_area_twice = 0_i128;
    let mut absolute_area_twice = 0_u128;
    let mut displacement_sum = 0_i128;
    let mut displacement_square_sum = 0_u128;
    let mut absolute_displacements = Vec::new();
    let mut previous_valid: Option<(u64, i64)> = None;

    for sample in &input.profile_samples {
        match (sample.before_z_um, sample.after_z_um) {
            (Some(before), Some(after)) => {
                let displacement = after
                    .checked_sub(before)
                    .ok_or(RejectReason::NumericOverflow)?;
                valid_count += 1;
                current_missing_run = 0;
                displacement_sum += i128::from(displacement);
                let absolute = displacement.unsigned_abs();
                displacement_square_sum = displacement_square_sum
                    .checked_add(u128::from(absolute) * u128::from(absolute))
                    .ok_or(RejectReason::NumericOverflow)?;
                absolute_displacements.push(absolute);

                if let Some((previous_x, previous_displacement)) = previous_valid {
                    let dx = sample
                        .x_um
                        .checked_sub(previous_x)
                        .ok_or(RejectReason::NumericOverflow)?;
                    let sum_dz = i128::from(previous_displacement)
                        .checked_add(i128::from(displacement))
                        .ok_or(RejectReason::NumericOverflow)?;
                    let segment_twice = sum_dz
                        .checked_mul(i128::from(dx))
                        .ok_or(RejectReason::NumericOverflow)?;
                    signed_area_twice = signed_area_twice
                        .checked_add(segment_twice)
                        .ok_or(RejectReason::NumericOverflow)?;
                    absolute_area_twice = absolute_area_twice
                        .checked_add(segment_twice.unsigned_abs())
                        .ok_or(RejectReason::NumericOverflow)?;
                    valid_span_length_um = valid_span_length_um
                        .checked_add(dx)
                        .ok_or(RejectReason::NumericOverflow)?;
                } else {
                    valid_span_count += 1;
                }
                previous_valid = Some((sample.x_um, displacement));
            }
            _ => {
                missing_count += 1;
                current_missing_run += 1;
                maximum_missing_run = maximum_missing_run.max(current_missing_run);
                previous_valid = None;
            }
        }
    }

    let coverage = valid_count
        .checked_mul(1_000)
        .ok_or(RejectReason::NumericOverflow)?
        / expected;
    let coverage_permille =
        u16::try_from(coverage).map_err(|_| RejectReason::NumericOverflow)?;
    if coverage_permille < input.minimum_coverage_permille {
        return Err(RejectReason::CoverageBelowMinimum);
    }
    if maximum_missing_run > input.maximum_missing_run_samples {
        return Err(RejectReason::MissingRunTooLong);
    }
    if valid_count < 2 || valid_span_length_um == 0 || valid_span_count == 0 {
        return Err(RejectReason::NoValidMeasuredSpan);
    }

    let signed_area = signed_area_twice / 2;
    let absolute_area = absolute_area_twice / 2;
    let signed_area_change_um2 =
        i64::try_from(signed_area).map_err(|_| RejectReason::NumericOverflow)?;
    let absolute_area_change_um2 =
        u64::try_from(absolute_area).map_err(|_| RejectReason::NumericOverflow)?;
    let mean_displacement_um = displacement_sum / (valid_count as i128);
    let mean_signed_displacement_um =
        i64::try_from(mean_displacement_um).map_err(|_| RejectReason::NumericOverflow)?;
    let mean_square = displacement_square_sum / (valid_count as u128);
    let rmse_displacement_um =
        u64::try_from(integer_sqrt(mean_square)).map_err(|_| RejectReason::NumericOverflow)?;

    absolute_displacements.sort_unstable();
    let percentile_rank = (95 * absolute_displacements.len()).div_ceil(100);
    let percentile_95_absolute_displacement_um = u64::try_from(
        absolute_displacements[percentile_rank.saturating_sub(1)],
    )
    .map_err(|_| RejectReason::NumericOverflow)?;
    let maximum_absolute_displacement_um = u64::try_from(
        *absolute_displacements.last().ok_or(RejectReason::NoValidMeasuredSpan)?,
    )
    .map_err(|_| RejectReason::NumericOverflow)?;

    let mean_displacement_interval_um =
        checked_interval(mean_signed_displacement_um, uncertainty)?;
    let area_uncertainty = uncertainty
        .checked_mul(valid_span_length_um)
        .ok_or(RejectReason::NumericOverflow)?;
    let signed_area_interval_um2 = checked_area_interval(
        signed_area_change_um2,
        area_uncertainty,
    )?;

    Ok(MeasurementReport {
        pair_id: input.pair_id.clone(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        paired_sample_count: valid_count,
        expected_sample_count: expected,
        coverage_permille,
        missing_sample_count: missing_count,
        maximum_missing_run_samples: maximum_missing_run,
        valid_span_count,
        valid_span_length_um,
        signed_area_change_um2,
        absolute_area_change_um2,
        mean_signed_displacement_um,
        rmse_displacement_um,
        maximum_absolute_displacement_um,
        percentile_95_absolute_displacement_um,
        combined_uncertainty_um: uncertainty,
        mean_displacement_interval_um,
        signed_area_interval_um2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn sample(x: u64, before: Option<i64>, after: Option<i64>) -> ProfileSample {
        ProfileSample {
            x_um: x,
            before_z_um: before,
            after_z_um: after,
        }
    }

    fn base_input() -> ProfilePairInput {
        let before_hash = digest('1');
        let after_hash = digest('2');
        ProfilePairInput {
            schema_version: "0.1.0".to_owned(),
            benchmark_id: BENCHMARK_ID.to_owned(),
            evidence_class: EvidenceClass::SyntheticContractOnly,
            pair_id: "PAIR-SYNTH-001".to_owned(),
            before_scan: ScanIdentity {
                sha256: before_hash.clone(),
                workpiece_id: "WID-SYNTH-01".to_owned(),
                joint_revision: "JOINT-R1".to_owned(),
                layer_id: "LAYER-02".to_owned(),
                profile_station_id: "STATION-001".to_owned(),
                coordinate_frame: "WELDING-FRAME-R1".to_owned(),
                instrument_id: "SCANNER-SYNTH-01".to_owned(),
                calibration_revision: "CAL-R1".to_owned(),
                captured_at_ms: 1_000,
            },
            after_scan: ScanIdentity {
                sha256: after_hash.clone(),
                workpiece_id: "WID-SYNTH-01".to_owned(),
                joint_revision: "JOINT-R1".to_owned(),
                layer_id: "LAYER-02".to_owned(),
                profile_station_id: "STATION-001".to_owned(),
                coordinate_frame: "WELDING-FRAME-R1".to_owned(),
                instrument_id: "SCANNER-SYNTH-01".to_owned(),
                calibration_revision: "CAL-R1".to_owned(),
                captured_at_ms: 2_000,
            },
            calibration_receipt: CalibrationReceipt {
                sha256: digest('3'),
                instrument_id: "SCANNER-SYNTH-01".to_owned(),
                calibration_revision: "CAL-R1".to_owned(),
                verified: true,
                valid_from_ms: 0,
                valid_until_ms: 10_000,
                reference_artifact_error_um: 2,
                maximum_reference_artifact_error_um: 5,
            },
            registration_receipt: RegistrationReceipt {
                sha256: digest('4'),
                before_scan_sha256: before_hash,
                after_scan_sha256: after_hash,
                transform_type: "rigid_2d".to_owned(),
                fit_region: "stable_datum".to_owned(),
                stable_datum_verified: true,
                handedness: "proper".to_owned(),
                check_point_count: 6,
                independent_check_point_error_um: 3,
                maximum_check_point_error_um: 10,
                scale_deviation_ppm: 2,
                maximum_scale_deviation_ppm: 10,
            },
            surface_compatibility: SurfaceCompatibilityReceipt {
                sha256: digest('5'),
                within_qualified_surface_range: true,
                surface_condition: "synthetic_matte_reference".to_owned(),
            },
            uncertainty: UncertaintyBudget {
                instrument_um: Some(5),
                calibration_um: Some(5),
                registration_um: Some(5),
                repeatability_um: Some(5),
                datum_um: Some(5),
                interpolation_um: Some(5),
            },
            maximum_accepted_uncertainty_um: 50,
            minimum_coverage_permille: 800,
            maximum_missing_run_samples: 1,
            profile_samples: vec![
                sample(0, Some(0), Some(100)),
                sample(1_000, Some(0), Some(100)),
                sample(2_000, Some(0), Some(100)),
                sample(3_000, Some(0), Some(100)),
            ],
            claim_ceiling: CLAIM_CEILING.to_owned(),
        }
    }

    #[test]
    fn identical_surface_pair_has_zero_surface_change() {
        let mut input = base_input();
        for point in &mut input.profile_samples {
            point.after_z_um = point.before_z_um;
        }
        let report = evaluate_profile_pair(&input).expect("identity pair should measure");
        assert_eq!(report.signed_area_change_um2, 0);
        assert_eq!(report.absolute_area_change_um2, 0);
        assert_eq!(report.maximum_absolute_displacement_um, 0);
    }

    #[test]
    fn uniform_translation_has_exact_known_area_and_displacement() {
        let report = evaluate_profile_pair(&base_input()).expect("known pair should measure");
        assert_eq!(report.signed_area_change_um2, 300_000);
        assert_eq!(report.absolute_area_change_um2, 300_000);
        assert_eq!(report.mean_signed_displacement_um, 100);
        assert_eq!(report.rmse_displacement_um, 100);
        assert_eq!(report.coverage_permille, 1_000);
        assert_eq!(report.combined_uncertainty_um, 30);
        assert_eq!(report.claim_ceiling, CLAIM_CEILING);
    }

    #[test]
    fn missing_samples_are_not_imputed_or_bridged() {
        let mut input = base_input();
        input.profile_samples = vec![
            sample(0, Some(0), Some(100)),
            sample(1_000, Some(0), Some(100)),
            sample(2_000, None, None),
            sample(3_000, Some(0), Some(100)),
            sample(4_000, Some(0), Some(100)),
        ];
        input.minimum_coverage_permille = 700;
        input.maximum_missing_run_samples = 1;
        let report = evaluate_profile_pair(&input).expect("two observed spans should measure");
        assert_eq!(report.paired_sample_count, 4);
        assert_eq!(report.missing_sample_count, 1);
        assert_eq!(report.valid_span_count, 2);
        assert_eq!(report.valid_span_length_um, 2_000);
        assert_eq!(report.signed_area_change_um2, 200_000);
    }

    #[test]
    fn low_coverage_abstains() {
        let mut input = base_input();
        input.profile_samples[1].before_z_um = None;
        input.profile_samples[1].after_z_um = None;
        input.minimum_coverage_permille = 900;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::CoverageBelowMinimum)
        );
    }

    #[test]
    fn long_missing_run_abstains() {
        let mut input = base_input();
        input.profile_samples[1].before_z_um = None;
        input.profile_samples[1].after_z_um = None;
        input.profile_samples[2].before_z_um = None;
        input.profile_samples[2].after_z_um = None;
        input.maximum_missing_run_samples = 1;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::MissingRunTooLong)
        );
    }

    #[test]
    fn missing_uncertainty_component_abstains() {
        let mut input = base_input();
        input.uncertainty.registration_um = None;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::MissingUncertaintyComponent)
        );
    }

    #[test]
    fn uncertainty_is_conservatively_summed_not_rss_combined() {
        let report = evaluate_profile_pair(&base_input()).expect("base pair should measure");
        assert_eq!(report.combined_uncertainty_um, 30);
    }

    #[test]
    fn excessive_uncertainty_abstains() {
        let mut input = base_input();
        input.maximum_accepted_uncertainty_um = 20;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::UncertaintyExceedsDecisionLimit)
        );
    }

    #[test]
    fn uncertainty_sum_overflow_abstains() {
        let mut input = base_input();
        input.uncertainty.instrument_um = Some(u64::MAX);
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::UncertaintyOverflow)
        );
    }

    #[test]
    fn rights_unadmitted_measured_evidence_is_rejected() {
        let mut input = base_input();
        input.evidence_class = EvidenceClass::MeasuredAuthorized;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::EvidenceNotAdmitted)
        );
    }

    #[test]
    fn malformed_source_digest_is_rejected() {
        let mut input = base_input();
        input.before_scan.sha256 = "not-a-digest".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::MalformedDigest)
        );
    }

    #[test]
    fn different_workpieces_are_rejected() {
        let mut input = base_input();
        input.after_scan.workpiece_id = "WID-SYNTH-02".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::DifferentWorkpiece)
        );
    }

    #[test]
    fn different_joint_revisions_are_rejected() {
        let mut input = base_input();
        input.after_scan.joint_revision = "JOINT-R2".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::DifferentJointRevision)
        );
    }

    #[test]
    fn different_layers_are_rejected() {
        let mut input = base_input();
        input.after_scan.layer_id = "LAYER-03".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::DifferentLayer)
        );
    }

    #[test]
    fn different_profile_stations_are_rejected() {
        let mut input = base_input();
        input.after_scan.profile_station_id = "STATION-002".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::DifferentProfileStation)
        );
    }

    #[test]
    fn different_coordinate_frames_are_rejected() {
        let mut input = base_input();
        input.after_scan.coordinate_frame = "OTHER-FRAME".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::DifferentCoordinateFrame)
        );
    }

    #[test]
    fn different_instruments_are_rejected() {
        let mut input = base_input();
        input.after_scan.instrument_id = "OTHER-SCANNER".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::DifferentInstrument)
        );
    }

    #[test]
    fn different_calibration_revisions_are_rejected() {
        let mut input = base_input();
        input.after_scan.calibration_revision = "CAL-R2".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::DifferentCalibrationRevision)
        );
    }

    #[test]
    fn reverse_capture_order_is_rejected() {
        let mut input = base_input();
        input.after_scan.captured_at_ms = input.before_scan.captured_at_ms;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::InvalidCaptureOrder)
        );
    }

    #[test]
    fn unverified_calibration_is_rejected() {
        let mut input = base_input();
        input.calibration_receipt.verified = false;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::CalibrationNotVerified)
        );
    }

    #[test]
    fn wrong_calibration_receipt_is_rejected() {
        let mut input = base_input();
        input.calibration_receipt.calibration_revision = "CAL-R0".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::CalibrationReceiptMismatch)
        );
    }

    #[test]
    fn expired_calibration_is_rejected() {
        let mut input = base_input();
        input.calibration_receipt.valid_until_ms = 1_500;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::CalibrationNotValidAtCapture)
        );
    }

    #[test]
    fn failed_reference_artifact_check_is_rejected() {
        let mut input = base_input();
        input.calibration_receipt.reference_artifact_error_um = 20;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::ReferenceArtifactOutOfTolerance)
        );
    }

    #[test]
    fn registration_must_bind_the_exact_scan_hashes() {
        let mut input = base_input();
        input.registration_receipt.after_scan_sha256 = digest('9');
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::RegistrationReceiptMismatch)
        );
    }

    #[test]
    fn non_rigid_registration_is_rejected() {
        let mut input = base_input();
        input.registration_receipt.transform_type = "affine_2d".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::NonRigidTransform)
        );
    }

    #[test]
    fn registration_without_stable_datum_is_rejected() {
        let mut input = base_input();
        input.registration_receipt.stable_datum_verified = false;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::UnstableDatum)
        );
    }

    #[test]
    fn fitting_registration_on_changing_bead_surface_is_rejected() {
        let mut input = base_input();
        input.registration_receipt.fit_region = "weld_bead_surface".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::RegistrationFitUsesChangingSurface)
        );
    }

    #[test]
    fn reflected_transform_is_rejected() {
        let mut input = base_input();
        input.registration_receipt.handedness = "reflected".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::ImproperTransformHandedness)
        );
    }

    #[test]
    fn too_few_independent_check_points_are_rejected() {
        let mut input = base_input();
        input.registration_receipt.check_point_count = 2;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::TooFewIndependentCheckPoints)
        );
    }

    #[test]
    fn independent_check_point_error_is_enforced() {
        let mut input = base_input();
        input.registration_receipt.independent_check_point_error_um = 11;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::CheckPointErrorTooLarge)
        );
    }

    #[test]
    fn scale_deviation_is_enforced() {
        let mut input = base_input();
        input.registration_receipt.scale_deviation_ppm = 11;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::ScaleDeviationTooLarge)
        );
    }

    #[test]
    fn noncooperative_surface_is_rejected() {
        let mut input = base_input();
        input.surface_compatibility.within_qualified_surface_range = false;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::SurfaceOutsideQualifiedRange)
        );
    }

    #[test]
    fn nonmonotonic_x_grid_is_rejected() {
        let mut input = base_input();
        input.profile_samples[2].x_um = input.profile_samples[1].x_um;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::NonMonotonicProfileCoordinates)
        );
    }

    #[test]
    fn zero_coverage_threshold_is_rejected() {
        let mut input = base_input();
        input.minimum_coverage_permille = 0;
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::InvalidCoverageThreshold)
        );
    }

    #[test]
    fn claim_promotion_to_deposited_volume_is_rejected() {
        let mut input = base_input();
        input.claim_ceiling = "DEPOSITED_METAL_VOLUME".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::ClaimCeilingPromotion)
        );
    }

    #[test]
    fn empty_pair_identity_is_rejected() {
        let mut input = base_input();
        input.pair_id.clear();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::EmptyIdentity)
        );
    }

    #[test]
    fn invalid_layer_metadata_schema_is_rejected() {
        let mut input = base_input();
        input.schema_version = "0.2.0".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::WrongSchemaVersion)
        );
    }

    #[test]
    fn wrong_benchmark_id_is_rejected() {
        let mut input = base_input();
        input.benchmark_id = "OTHER-BENCHMARK".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::WrongBenchmarkId)
        );
    }

    #[test]
    fn malformed_registration_receipt_digest_is_rejected() {
        let mut input = base_input();
        input.registration_receipt.sha256 = "bad".to_owned();
        assert_eq!(
            evaluate_profile_pair(&input),
            Err(RejectReason::MalformedDigest)
        );
    }

    #[test]
    fn missing_one_side_of_a_pair_is_not_counted_as_valid() {
        let mut input = base_input();
        input.profile_samples[1].after_z_um = None;
        input.minimum_coverage_permille = 700;
        let report = evaluate_profile_pair(&input).expect("remaining coverage is valid");
        assert_eq!(report.paired_sample_count, 3);
        assert_eq!(report.missing_sample_count, 1);
        assert_eq!(report.valid_span_count, 2);
    }
}
