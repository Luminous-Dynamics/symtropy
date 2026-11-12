//! Evidence-bound 2D cross-section profile comparison.
//!
//! This oracle compares a registered measured profile with a reference profile
//! on an identical station grid. It returns a conservative upper bound on the
//! absolute deviation area under a piecewise-linear-between-samples assumption.
//! It is not a 3D weld-volume, NDT, weld-acceptance or free-state distortion
//! validator.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfilePoint {
    pub station_units: u64,
    pub height_units: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasuredPoint {
    pub station_units: u64,
    pub height_units: i64,
    pub vertical_uncertainty_bound_units: u64,
    pub valid: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceProfile {
    pub workpiece_id: String,
    pub joint_revision: String,
    pub coordinate_frame: String,
    pub profile_id: String,
    pub sha256: [u8; 32],
    pub points: Vec<ProfilePoint>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasuredProfile {
    pub workpiece_id: String,
    pub joint_revision: String,
    pub coordinate_frame: String,
    pub profile_id: String,
    pub sha256: [u8; 32],
    pub points: Vec<MeasuredPoint>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistrationReceipt {
    pub sha256: [u8; 32],
    pub source_scan_sha256: [u8; 32],
    pub reference_profile_sha256: [u8; 32],
    pub transform_revision: String,
    pub registration_vertical_error_bound_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparisonInput {
    pub reference: ReferenceProfile,
    pub measured: MeasuredProfile,
    pub registration: RegistrationReceipt,
    pub minimum_coverage_permille: u16,
    pub maximum_station_gap_units: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    EmptyIdentity,
    WorkpieceMismatch,
    JointRevisionMismatch,
    CoordinateFrameMismatch,
    MissingReferenceDigest,
    MissingScanDigest,
    MissingRegistrationDigest,
    RegistrationReceiptSourceMismatch,
    EmptyTransformRevision,
    InvalidCoverageThreshold,
    ZeroMaximumStationGap,
    TooFewProfilePoints,
    ProfileLengthMismatch,
    StationGridMismatch,
    NonMonotonicStationGrid,
    StationGapExceedsLimit,
    InsufficientCoverage,
    NoCoveredIntervals,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparisonResult {
    pub reference_profile_id: String,
    pub measured_profile_id: String,
    pub reference_profile_sha256: [u8; 32],
    pub measured_profile_sha256: [u8; 32],
    pub registration_receipt_sha256: [u8; 32],
    pub covered_length_units: u64,
    pub total_length_units: u64,
    pub coverage_permille: u16,
    pub valid_interval_count: usize,
    /// Twice the area bound in coordinate_units squared.
    ///
    /// Keeping twice-area integral avoids introducing floating-point rounding
    /// into the exact trapezoidal accumulation.
    pub absolute_deviation_area_upper_bound_twice_units: u128,
    pub registration_vertical_error_bound_units: u64,
    pub interpolation_assumption: &'static str,
    pub claim_ceiling: &'static str,
}

fn is_empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn digest_is_zero(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

fn validate_identity(input: &ComparisonInput) -> Result<(), Rejection> {
    let reference = &input.reference;
    let measured = &input.measured;
    let registration = &input.registration;

    if [
        reference.workpiece_id.as_str(),
        reference.joint_revision.as_str(),
        reference.coordinate_frame.as_str(),
        reference.profile_id.as_str(),
        measured.workpiece_id.as_str(),
        measured.joint_revision.as_str(),
        measured.coordinate_frame.as_str(),
        measured.profile_id.as_str(),
    ]
    .into_iter()
    .any(is_empty)
    {
        return Err(Rejection::EmptyIdentity);
    }
    if reference.workpiece_id != measured.workpiece_id {
        return Err(Rejection::WorkpieceMismatch);
    }
    if reference.joint_revision != measured.joint_revision {
        return Err(Rejection::JointRevisionMismatch);
    }
    if reference.coordinate_frame != measured.coordinate_frame {
        return Err(Rejection::CoordinateFrameMismatch);
    }
    if digest_is_zero(&reference.sha256) {
        return Err(Rejection::MissingReferenceDigest);
    }
    if digest_is_zero(&measured.sha256) {
        return Err(Rejection::MissingScanDigest);
    }
    if digest_is_zero(&registration.sha256) {
        return Err(Rejection::MissingRegistrationDigest);
    }
    if registration.source_scan_sha256 != measured.sha256
        || registration.reference_profile_sha256 != reference.sha256
    {
        return Err(Rejection::RegistrationReceiptSourceMismatch);
    }
    if is_empty(&registration.transform_revision) {
        return Err(Rejection::EmptyTransformRevision);
    }
    if !(1..=1_000).contains(&input.minimum_coverage_permille) {
        return Err(Rejection::InvalidCoverageThreshold);
    }
    if input.maximum_station_gap_units == 0 {
        return Err(Rejection::ZeroMaximumStationGap);
    }
    Ok(())
}

fn validate_station_grids(input: &ComparisonInput) -> Result<u64, Rejection> {
    let reference = &input.reference.points;
    let measured = &input.measured.points;

    if reference.len() < 2 || measured.len() < 2 {
        return Err(Rejection::TooFewProfilePoints);
    }
    if reference.len() != measured.len() {
        return Err(Rejection::ProfileLengthMismatch);
    }

    for index in 0..reference.len() {
        if reference[index].station_units != measured[index].station_units {
            return Err(Rejection::StationGridMismatch);
        }
        if index > 0 {
            let prior = reference[index - 1].station_units;
            let current = reference[index].station_units;
            if current <= prior {
                return Err(Rejection::NonMonotonicStationGrid);
            }
            if current - prior > input.maximum_station_gap_units {
                return Err(Rejection::StationGapExceedsLimit);
            }
        }
    }
    let first = reference[0].station_units;
    let last = reference[reference.len() - 1].station_units;
    last.checked_sub(first)
        .filter(|length| *length > 0)
        .ok_or(Rejection::NonMonotonicStationGrid)
}

fn point_error_upper_bound(
    reference_height: i64,
    measured_height: i64,
    measured_uncertainty: u64,
    registration_uncertainty: u64,
) -> Result<u64, Rejection> {
    reference_height
        .abs_diff(measured_height)
        .checked_add(measured_uncertainty)
        .and_then(|error| error.checked_add(registration_uncertainty))
        .ok_or(Rejection::ArithmeticOverflow)
}

/// Compare two registered profiles with explicit missingness and uncertainty.
///
/// For each valid interval, the endpoint error envelope is
/// |reference - measured| + measurement_uncertainty + registration_bound.
/// Under the declared piecewise-linear-between-samples assumption, the
/// trapezoid formed by these endpoint bounds is an upper bound on the
/// integrated absolute profile deviation for that interval.
///
/// Intervals touching an invalid measured point are not bridged or imputed.
/// Coverage is measured by the total length of valid adjacent intervals over
/// the full station range. The function never extrapolates into missing spans.
pub fn compare_profiles(input: &ComparisonInput) -> Result<ComparisonResult, Rejection> {
    validate_identity(input)?;
    let total_length = validate_station_grids(input)?;

    let mut covered_length = 0u64;
    let mut interval_count = 0usize;
    let mut area_twice = 0u128;

    for index in 0..input.reference.points.len() - 1 {
        let measured_left = &input.measured.points[index];
        let measured_right = &input.measured.points[index + 1];
        if !measured_left.valid || !measured_right.valid {
            continue;
        }

        let reference_left = &input.reference.points[index];
        let reference_right = &input.reference.points[index + 1];
        let width = reference_right
            .station_units
            .checked_sub(reference_left.station_units)
            .ok_or(Rejection::NonMonotonicStationGrid)?;

        let left_error = point_error_upper_bound(
            reference_left.height_units,
            measured_left.height_units,
            measured_left.vertical_uncertainty_bound_units,
            input.registration.registration_vertical_error_bound_units,
        )?;
        let right_error = point_error_upper_bound(
            reference_right.height_units,
            measured_right.height_units,
            measured_right.vertical_uncertainty_bound_units,
            input.registration.registration_vertical_error_bound_units,
        )?;

        let endpoint_sum = u128::from(left_error)
            .checked_add(u128::from(right_error))
            .ok_or(Rejection::ArithmeticOverflow)?;
        let interval_area_twice = u128::from(width)
            .checked_mul(endpoint_sum)
            .ok_or(Rejection::ArithmeticOverflow)?;

        area_twice = area_twice
            .checked_add(interval_area_twice)
            .ok_or(Rejection::ArithmeticOverflow)?;
        covered_length = covered_length
            .checked_add(width)
            .ok_or(Rejection::ArithmeticOverflow)?;
        interval_count += 1;
    }

    if interval_count == 0 {
        return Err(Rejection::NoCoveredIntervals);
    }

    let coverage = u128::from(covered_length)
        .checked_mul(1_000)
        .ok_or(Rejection::ArithmeticOverflow)?
        / u128::from(total_length);
    let coverage_permille = u16::try_from(coverage).map_err(|_| Rejection::ArithmeticOverflow)?;
    if coverage_permille < input.minimum_coverage_permille {
        return Err(Rejection::InsufficientCoverage);
    }

    Ok(ComparisonResult {
        reference_profile_id: input.reference.profile_id.clone(),
        measured_profile_id: input.measured.profile_id.clone(),
        reference_profile_sha256: input.reference.sha256,
        measured_profile_sha256: input.measured.sha256,
        registration_receipt_sha256: input.registration.sha256,
        covered_length_units: covered_length,
        total_length_units: total_length,
        coverage_permille,
        valid_interval_count: interval_count,
        absolute_deviation_area_upper_bound_twice_units: area_twice,
        registration_vertical_error_bound_units: input
            .registration
            .registration_vertical_error_bound_units,
        interpolation_assumption: "piecewise_linear_between_valid_adjacent_samples_only",
        claim_ceiling: "REGISTERED_2D_CROSS_SECTION_PROFILE_DEVIATION_ONLY",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile_input() -> ComparisonInput {
        let reference = ReferenceProfile {
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            coordinate_frame: "WELD-SECTION-FRAME".to_owned(),
            profile_id: "REFERENCE-01".to_owned(),
            sha256: [0x11; 32],
            points: vec![
                ProfilePoint {
                    station_units: 0,
                    height_units: 0,
                },
                ProfilePoint {
                    station_units: 100,
                    height_units: 100,
                },
                ProfilePoint {
                    station_units: 200,
                    height_units: 0,
                },
            ],
        };
        let measured = MeasuredProfile {
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            coordinate_frame: "WELD-SECTION-FRAME".to_owned(),
            profile_id: "SCAN-01".to_owned(),
            sha256: [0x22; 32],
            points: vec![
                MeasuredPoint {
                    station_units: 0,
                    height_units: 0,
                    vertical_uncertainty_bound_units: 1,
                    valid: true,
                },
                MeasuredPoint {
                    station_units: 100,
                    height_units: 90,
                    vertical_uncertainty_bound_units: 1,
                    valid: true,
                },
                MeasuredPoint {
                    station_units: 200,
                    height_units: 0,
                    vertical_uncertainty_bound_units: 1,
                    valid: true,
                },
            ],
        };
        let registration = RegistrationReceipt {
            sha256: [0x33; 32],
            source_scan_sha256: [0x22; 32],
            reference_profile_sha256: [0x11; 32],
            transform_revision: "SYNTH-REG-R1".to_owned(),
            registration_vertical_error_bound_units: 1,
        };
        ComparisonInput {
            reference,
            measured,
            registration,
            minimum_coverage_permille: 1_000,
            maximum_station_gap_units: 100,
        }
    }

    #[test]
    fn known_answer_integrates_profile_error_and_uncertainty() {
        let result = compare_profiles(&profile_input()).expect("valid profile pair");
        assert_eq!(result.coverage_permille, 1_000);
        assert_eq!(result.valid_interval_count, 2);
        assert_eq!(result.covered_length_units, 200);
        assert_eq!(result.total_length_units, 200);
        assert_eq!(
            result.absolute_deviation_area_upper_bound_twice_units,
            2_800
        );
        assert_eq!(
            result.claim_ceiling,
            "REGISTERED_2D_CROSS_SECTION_PROFILE_DEVIATION_ONLY"
        );
    }

    #[test]
    fn identical_profiles_with_zero_uncertainty_have_zero_deviation_bound() {
        let mut input = profile_input();
        for point in &mut input.measured.points {
            point.height_units = input
                .reference
                .points
                .iter()
                .find(|reference| reference.station_units == point.station_units)
                .expect("matching reference station")
                .height_units;
            point.vertical_uncertainty_bound_units = 0;
        }
        input.registration.registration_vertical_error_bound_units = 0;
        let result = compare_profiles(&input).expect("identical profiles");
        assert_eq!(result.absolute_deviation_area_upper_bound_twice_units, 0);
    }

    #[test]
    fn measurement_and_registration_uncertainty_are_composed() {
        let result = compare_profiles(&profile_input()).expect("valid profile pair");
        assert_eq!(
            result.absolute_deviation_area_upper_bound_twice_units,
            2_800
        );
        assert_eq!(result.registration_vertical_error_bound_units, 1);
    }

    #[test]
    fn wrong_coordinate_frame_is_rejected() {
        let mut input = profile_input();
        input.measured.coordinate_frame = "OTHER-FRAME".to_owned();
        assert_eq!(
            compare_profiles(&input),
            Err(Rejection::CoordinateFrameMismatch)
        );
    }

    #[test]
    fn wrong_joint_revision_is_rejected() {
        let mut input = profile_input();
        input.measured.joint_revision = "JOINT-R2".to_owned();
        assert_eq!(
            compare_profiles(&input),
            Err(Rejection::JointRevisionMismatch)
        );
    }

    #[test]
    fn registration_receipt_must_bind_both_profile_digests() {
        let mut input = profile_input();
        input.registration.source_scan_sha256 = [0x44; 32];
        assert_eq!(
            compare_profiles(&input),
            Err(Rejection::RegistrationReceiptSourceMismatch)
        );
    }

    #[test]
    fn missing_scan_digest_is_rejected() {
        let mut input = profile_input();
        input.measured.sha256 = [0; 32];
        assert_eq!(compare_profiles(&input), Err(Rejection::MissingScanDigest));
    }

    #[test]
    fn missing_reference_digest_is_rejected() {
        let mut input = profile_input();
        input.reference.sha256 = [0; 32];
        assert_eq!(
            compare_profiles(&input),
            Err(Rejection::MissingReferenceDigest)
        );
    }

    #[test]
    fn missing_registration_digest_is_rejected() {
        let mut input = profile_input();
        input.registration.sha256 = [0; 32];
        assert_eq!(
            compare_profiles(&input),
            Err(Rejection::MissingRegistrationDigest)
        );
    }

    #[test]
    fn mismatched_station_grid_is_rejected() {
        let mut input = profile_input();
        input.measured.points[1].station_units = 101;
        assert_eq!(
            compare_profiles(&input),
            Err(Rejection::StationGridMismatch)
        );
    }

    #[test]
    fn non_monotonic_station_grid_is_rejected() {
        let mut input = profile_input();
        input.reference.points[2].station_units = 90;
        input.measured.points[2].station_units = 90;
        assert_eq!(
            compare_profiles(&input),
            Err(Rejection::NonMonotonicStationGrid)
        );
    }

    #[test]
    fn excessive_station_gap_is_rejected() {
        let mut input = profile_input();
        input.maximum_station_gap_units = 99;
        assert_eq!(
            compare_profiles(&input),
            Err(Rejection::StationGapExceedsLimit)
        );
    }

    #[test]
    fn missing_adjacent_intervals_are_not_bridged_or_imputed() {
        let mut input = profile_input();
        input.measured.points[1].valid = false;
        input.minimum_coverage_permille = 1;
        assert_eq!(compare_profiles(&input), Err(Rejection::NoCoveredIntervals));
    }

    #[test]
    fn partial_coverage_is_measured_only_on_valid_adjacent_spans() {
        let mut input = profile_input();
        input.reference.points = (0..=4)
            .map(|i| ProfilePoint {
                station_units: i * 25,
                height_units: 10,
            })
            .collect();
        input.measured.points = (0..=4)
            .map(|i| MeasuredPoint {
                station_units: i * 25,
                height_units: 10,
                vertical_uncertainty_bound_units: 1,
                valid: i != 2,
            })
            .collect();
        input.maximum_station_gap_units = 25;
        input.minimum_coverage_permille = 500;
        let result = compare_profiles(&input).expect("two edge intervals meet 50% coverage");
        assert_eq!(result.covered_length_units, 50);
        assert_eq!(result.total_length_units, 100);
        assert_eq!(result.coverage_permille, 500);
        assert_eq!(result.valid_interval_count, 2);
    }

    #[test]
    fn coverage_below_threshold_is_rejected() {
        let mut input = profile_input();
        input.reference.points = (0..=4)
            .map(|i| ProfilePoint {
                station_units: i * 25,
                height_units: 10,
            })
            .collect();
        input.measured.points = (0..=4)
            .map(|i| MeasuredPoint {
                station_units: i * 25,
                height_units: 10,
                vertical_uncertainty_bound_units: 1,
                valid: i != 2,
            })
            .collect();
        input.maximum_station_gap_units = 25;
        input.minimum_coverage_permille = 501;
        assert_eq!(
            compare_profiles(&input),
            Err(Rejection::InsufficientCoverage)
        );
    }

    #[test]
    fn overflowing_uncertainty_composition_is_rejected() {
        let mut input = profile_input();
        input.reference.points[0].height_units = i64::MIN;
        input.measured.points[0].height_units = i64::MAX;
        input.measured.points[0].vertical_uncertainty_bound_units = 1;
        assert_eq!(compare_profiles(&input), Err(Rejection::ArithmeticOverflow));
    }

    #[test]
    fn empty_transform_revision_is_rejected() {
        let mut input = profile_input();
        input.registration.transform_revision.clear();
        assert_eq!(
            compare_profiles(&input),
            Err(Rejection::EmptyTransformRevision)
        );
    }

    #[test]
    fn invalid_coverage_threshold_is_rejected() {
        let mut input = profile_input();
        input.minimum_coverage_permille = 0;
        assert_eq!(
            compare_profiles(&input),
            Err(Rejection::InvalidCoverageThreshold)
        );
    }

    #[test]
    fn arithmetic_overflow_in_integral_is_rejected() {
        let mut input = profile_input();
        input.reference.points = vec![
            ProfilePoint {
                station_units: 0,
                height_units: i64::MIN,
            },
            ProfilePoint {
                station_units: u64::MAX,
                height_units: i64::MIN,
            },
        ];
        input.measured.points = vec![
            MeasuredPoint {
                station_units: 0,
                height_units: i64::MAX,
                vertical_uncertainty_bound_units: 0,
                valid: true,
            },
            MeasuredPoint {
                station_units: u64::MAX,
                height_units: i64::MAX,
                vertical_uncertainty_bound_units: 0,
                valid: true,
            },
        ];
        input.maximum_station_gap_units = u64::MAX;
        assert_eq!(compare_profiles(&input), Err(Rejection::ArithmeticOverflow));
    }
}
