//! Independent, synthetic-only oracle for registered 2D profile comparisons.
//!
//! This crate checks measurement identity, frame and registration bindings,
//! freshness, station support, gaps and uncertainty before integrating profile
//! differences. It does not perform 3D point-cloud registration, calibrate a
//! laser scanner, infer weld quality, or authorize production decisions.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticContractOnly,
    Measured,
    Simulation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfilePoint {
    pub x_um: i64,
    pub z_um: Option<i64>,
    pub uncertainty_um: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub evidence_class: EvidenceClass,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub predecessor_pass_id: String,
    pub transition_id: String,
    pub scan_id: String,
    pub scan_sha256: String,
    pub coordinate_frame: String,
    /// Producer-supplied digest of canonical ordered x stations + unit.
    pub x_grid_sha256: String,
    pub captured_at_tick: u64,
    pub points: Vec<ProfilePoint>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistrationReceipt {
    pub receipt_sha256: String,
    pub reference_scan_sha256: String,
    pub observed_scan_sha256: String,
    pub source_frame: String,
    pub target_frame: String,
    pub calibration_id: String,
    pub algorithm_revision: String,
    pub maximum_residual_um: u64,
    pub declared_residual_limit_um: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparisonLimits {
    pub region_start_um: i64,
    pub region_end_um: i64,
    pub minimum_coverage_permille: u16,
    pub maximum_unsupported_gap_um: u64,
    pub maximum_point_uncertainty_um: u64,
    pub maximum_registration_residual_um: u64,
    pub maximum_integrated_uncertainty_area2_um2: u128,
    pub decision_at_tick: u64,
    pub maximum_observation_age_ticks: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparisonRequest {
    pub reference: Profile,
    pub observed: Profile,
    pub registration: RegistrationReceipt,
    pub limits: ComparisonLimits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbstainReason {
    EmptyIdentity,
    InvalidDigest,
    PhysicalEvidenceNotAdmitted,
    WorkpieceMismatch,
    JointRevisionMismatch,
    LayerMismatch,
    PredecessorPassMismatch,
    TransitionMismatch,
    ScanIdentityMismatch,
    CoordinateFrameMismatch,
    XGridDigestMismatch,
    RegistrationReceiptMismatch,
    RegistrationResidualTooLarge,
    InvalidRegion,
    InvalidCoverageLimit,
    InvalidGapLimit,
    InvalidUncertaintyLimit,
    FutureObservation,
    ObservationTooOld,
    ObservationOrderMismatch,
    ProfileTooShort,
    RegionNotExactlyCovered,
    NonMonotonicStations,
    StationGridMismatch,
    MissingPointUncertainty,
    UncertaintyWithoutMeasurement,
    PointUncertaintyTooLarge,
    UnsupportedGapTooLarge,
    CoverageBelowMinimum,
    IntegratedUncertaintyTooLarge,
    NumericOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeometryMetrics {
    /// All areas use doubled integer micrometre-squared units.
    /// Divide by two to obtain µm² without floating-point loss.
    pub signed_net_area2_um2: i128,
    pub added_area2_um2: u128,
    pub removed_area2_um2: u128,
    pub absolute_change_area2_um2: u128,
    pub integrated_uncertainty_bound_area2_um2: u128,
    pub maximum_absolute_deviation_um: u64,
    pub mean_absolute_deviation_um: u64,
    pub coverage_permille: u16,
    pub covered_length_um: u64,
    pub unsupported_length_um: u64,
    pub maximum_unsupported_gap_um: u64,
    pub maximum_sample_spacing_um: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualifiedComparison {
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub predecessor_pass_id: String,
    pub transition_id: String,
    pub reference_scan_sha256: String,
    pub observed_scan_sha256: String,
    pub registration_receipt_sha256: String,
    pub target_frame: String,
    pub reference_scan_id: String,
    pub observed_scan_id: String,
    pub x_grid_sha256: String,
    pub length_unit: &'static str,
    pub maximum_unsupported_gap_units: u64,
    pub largest_unsupported_gap_units: u64,
    pub minimum_coverage_permille: u16,
    pub maximum_area_uncertainty_units: u128,
    pub metrics: GeometryMetrics,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComparisonDecision {
    Qualified(QualifiedComparison),
    Abstain(AbstainReason),
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn observation_age(captured: u64, decision: u64, maximum: u64) -> Result<(), AbstainReason> {
    if captured > decision {
        return Err(AbstainReason::FutureObservation);
    }
    if decision - captured > maximum {
        return Err(AbstainReason::ObservationTooOld);
    }
    Ok(())
}

fn checked_add_u128(left: u128, right: u128) -> Result<u128, AbstainReason> {
    left.checked_add(right)
        .ok_or(AbstainReason::NumericOverflow)
}

fn checked_mul_u128(left: u128, right: u128) -> Result<u128, AbstainReason> {
    left.checked_mul(right)
        .ok_or(AbstainReason::NumericOverflow)
}

fn round_div_nearest(numerator: u128, denominator: u128) -> Result<u128, AbstainReason> {
    if denominator == 0 {
        return Err(AbstainReason::NumericOverflow);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let round_up = remainder >= denominator - remainder;
    quotient
        .checked_add(if round_up { 1 } else { 0 })
        .ok_or(AbstainReason::NumericOverflow)
}

/// Return twice-area contributions for positive and negative lobes of one
/// piecewise-linear segment. Opposite-sign segments are split at their exact
/// linear zero crossing; each rational triangle is rounded to the nearest
/// integer µm²×2 unit, with ties upward.
fn segment_lobes_area2(
    width_um: u64,
    left_difference_um: i128,
    right_difference_um: i128,
) -> Result<(u128, u128), AbstainReason> {
    let width = u128::from(width_um);
    let left_abs = left_difference_um.unsigned_abs();
    let right_abs = right_difference_um.unsigned_abs();

    if left_difference_um >= 0 && right_difference_um >= 0 {
        let sum = checked_add_u128(left_abs, right_abs)?;
        return Ok((checked_mul_u128(width, sum)?, 0));
    }
    if left_difference_um <= 0 && right_difference_um <= 0 {
        let sum = checked_add_u128(left_abs, right_abs)?;
        return Ok((0, checked_mul_u128(width, sum)?));
    }

    let denominator = checked_add_u128(left_abs, right_abs)?;
    if denominator == 0 {
        return Ok((0, 0));
    }
    let left_square = checked_mul_u128(left_abs, left_abs)?;
    let right_square = checked_mul_u128(right_abs, right_abs)?;
    let left_numerator = checked_mul_u128(width, left_square)?;
    let right_numerator = checked_mul_u128(width, right_square)?;
    let left_lobe = round_div_nearest(left_numerator, denominator)?;
    let right_lobe = round_div_nearest(right_numerator, denominator)?;

    if left_difference_um > 0 {
        Ok((left_lobe, right_lobe))
    } else {
        Ok((right_lobe, left_lobe))
    }
}

fn validate_metadata(request: &ComparisonRequest) -> Result<(), AbstainReason> {
    let reference = &request.reference;
    let observed = &request.observed;
    let registration = &request.registration;
    let limits = &request.limits;

    let identities = [
        reference.workpiece_id.as_str(),
        reference.joint_revision.as_str(),
        reference.layer_id.as_str(),
        reference.predecessor_pass_id.as_str(),
        reference.transition_id.as_str(),
        reference.scan_id.as_str(),
        reference.coordinate_frame.as_str(),
        reference.x_grid_sha256.as_str(),
        observed.x_grid_sha256.as_str(),
        observed.workpiece_id.as_str(),
        observed.joint_revision.as_str(),
        observed.layer_id.as_str(),
        observed.predecessor_pass_id.as_str(),
        observed.transition_id.as_str(),
        observed.scan_id.as_str(),
        observed.coordinate_frame.as_str(),
        registration.source_frame.as_str(),
        registration.target_frame.as_str(),
        registration.calibration_id.as_str(),
        registration.algorithm_revision.as_str(),
    ];
    if identities.into_iter().any(|value| !nonempty(value)) {
        return Err(AbstainReason::EmptyIdentity);
    }

    if reference.evidence_class != EvidenceClass::SyntheticContractOnly
        || observed.evidence_class != EvidenceClass::SyntheticContractOnly
    {
        return Err(AbstainReason::PhysicalEvidenceNotAdmitted);
    }
    if !valid_sha256(&reference.scan_sha256)
        || !valid_sha256(&observed.scan_sha256)
        || !valid_sha256(&reference.x_grid_sha256)
        || !valid_sha256(&observed.x_grid_sha256)
        || !valid_sha256(&registration.receipt_sha256)
        || !valid_sha256(&registration.reference_scan_sha256)
        || !valid_sha256(&registration.observed_scan_sha256)
    {
        return Err(AbstainReason::InvalidDigest);
    }
    if reference.workpiece_id != observed.workpiece_id {
        return Err(AbstainReason::WorkpieceMismatch);
    }
    if reference.joint_revision != observed.joint_revision {
        return Err(AbstainReason::JointRevisionMismatch);
    }
    if reference.layer_id != observed.layer_id {
        return Err(AbstainReason::LayerMismatch);
    }
    if reference.predecessor_pass_id != observed.predecessor_pass_id {
        return Err(AbstainReason::PredecessorPassMismatch);
    }
    if reference.transition_id != observed.transition_id {
        return Err(AbstainReason::TransitionMismatch);
    }
    if reference.scan_id == observed.scan_id
        || reference.scan_sha256 == observed.scan_sha256
        || registration.reference_scan_sha256 != reference.scan_sha256
        || registration.observed_scan_sha256 != observed.scan_sha256
    {
        return Err(AbstainReason::ScanIdentityMismatch);
    }
    if reference.coordinate_frame != observed.coordinate_frame
        || reference.coordinate_frame != registration.target_frame
    {
        return Err(AbstainReason::CoordinateFrameMismatch);
    }
    if reference.x_grid_sha256 != observed.x_grid_sha256 {
        return Err(AbstainReason::XGridDigestMismatch);
    }
    if registration.maximum_residual_um > registration.declared_residual_limit_um
        || registration.maximum_residual_um > limits.maximum_registration_residual_um
    {
        return Err(AbstainReason::RegistrationResidualTooLarge);
    }

    if limits.region_start_um >= limits.region_end_um {
        return Err(AbstainReason::InvalidRegion);
    }
    if limits.minimum_coverage_permille == 0 || limits.minimum_coverage_permille > 1_000 {
        return Err(AbstainReason::InvalidCoverageLimit);
    }
    if limits.maximum_unsupported_gap_um == 0 {
        return Err(AbstainReason::InvalidGapLimit);
    }
    if limits.maximum_point_uncertainty_um == 0
        || limits.maximum_integrated_uncertainty_area2_um2 == 0
    {
        return Err(AbstainReason::InvalidUncertaintyLimit);
    }

    if observed.captured_at_tick <= reference.captured_at_tick {
        return Err(AbstainReason::ObservationOrderMismatch);
    }
    observation_age(
        reference.captured_at_tick,
        limits.decision_at_tick,
        limits.maximum_observation_age_ticks,
    )?;
    observation_age(
        observed.captured_at_tick,
        limits.decision_at_tick,
        limits.maximum_observation_age_ticks,
    )?;
    Ok(())
}

fn validate_point_measurement(
    point: &ProfilePoint,
    maximum_uncertainty_um: u64,
) -> Result<bool, AbstainReason> {
    match (point.z_um, point.uncertainty_um) {
        (Some(_), Some(uncertainty)) if uncertainty > 0 => {
            if uncertainty > maximum_uncertainty_um {
                return Err(AbstainReason::PointUncertaintyTooLarge);
            }
            Ok(true)
        }
        (Some(_), _) => Err(AbstainReason::MissingPointUncertainty),
        (None, None) => Ok(false),
        (None, Some(_)) => Err(AbstainReason::UncertaintyWithoutMeasurement),
    }
}

fn station_is_measured(
    reference: &ProfilePoint,
    observed: &ProfilePoint,
    maximum_uncertainty_um: u64,
) -> Result<bool, AbstainReason> {
    let reference_measured = validate_point_measurement(reference, maximum_uncertainty_um)?;
    let observed_measured = validate_point_measurement(observed, maximum_uncertainty_um)?;
    Ok(reference_measured && observed_measured)
}

fn width_between(left_um: i64, right_um: i64) -> Result<u64, AbstainReason> {
    let width = i128::from(right_um) - i128::from(left_um);
    if width <= 0 {
        return Err(AbstainReason::NonMonotonicStations);
    }
    u64::try_from(width).map_err(|_| AbstainReason::NumericOverflow)
}

pub fn compare_profiles(request: &ComparisonRequest) -> ComparisonDecision {
    match compare_profiles_inner(request) {
        Ok(comparison) => ComparisonDecision::Qualified(comparison),
        Err(reason) => ComparisonDecision::Abstain(reason),
    }
}

fn compare_profiles_inner(
    request: &ComparisonRequest,
) -> Result<QualifiedComparison, AbstainReason> {
    validate_metadata(request)?;

    let reference = &request.reference;
    let observed = &request.observed;
    let limits = &request.limits;
    if reference.points.len() < 2 || observed.points.len() < 2 {
        return Err(AbstainReason::ProfileTooShort);
    }
    if reference.points.len() != observed.points.len() {
        return Err(AbstainReason::StationGridMismatch);
    }
    if reference.points.first().map(|point| point.x_um) != Some(limits.region_start_um)
        || reference.points.last().map(|point| point.x_um) != Some(limits.region_end_um)
        || observed.points.first().map(|point| point.x_um) != Some(limits.region_start_um)
        || observed.points.last().map(|point| point.x_um) != Some(limits.region_end_um)
    {
        return Err(AbstainReason::RegionNotExactlyCovered);
    }

    for (reference_point, observed_point) in reference.points.iter().zip(observed.points.iter()) {
        if reference_point.x_um != observed_point.x_um {
            return Err(AbstainReason::StationGridMismatch);
        }
    }

    let mut station_valid = Vec::with_capacity(reference.points.len());
    for (reference_point, observed_point) in reference.points.iter().zip(observed.points.iter()) {
        station_valid.push(station_is_measured(
            reference_point,
            observed_point,
            limits.maximum_point_uncertainty_um,
        )?);
    }

    let mut added_area2 = 0u128;
    let mut removed_area2 = 0u128;
    let mut uncertainty_area2 = 0u128;
    let mut maximum_absolute_deviation = 0u128;
    let mut covered_length = 0u128;
    let mut unsupported_length = 0u128;
    let mut maximum_unsupported_gap = 0u64;
    let mut current_unsupported_gap = 0u64;
    let mut maximum_sample_spacing = 0u64;

    for index in 0..reference.points.len() - 1 {
        let left_reference = &reference.points[index];
        let right_reference = &reference.points[index + 1];
        let left_observed = &observed.points[index];
        let right_observed = &observed.points[index + 1];
        let width = width_between(left_reference.x_um, right_reference.x_um)?;
        maximum_sample_spacing = maximum_sample_spacing.max(width);

        if !station_valid[index]
            || !station_valid[index + 1]
            || width > limits.maximum_unsupported_gap_um
        {
            current_unsupported_gap = current_unsupported_gap
                .checked_add(width)
                .ok_or(AbstainReason::NumericOverflow)?;
            unsupported_length = checked_add_u128(unsupported_length, u128::from(width))?;
            maximum_unsupported_gap = maximum_unsupported_gap.max(current_unsupported_gap);
            continue;
        }

        current_unsupported_gap = 0;
        covered_length = checked_add_u128(covered_length, u128::from(width))?;

        let left_difference = i128::from(
            left_observed
                .z_um
                .ok_or(AbstainReason::MissingPointUncertainty)?,
        ) - i128::from(
            left_reference
                .z_um
                .ok_or(AbstainReason::MissingPointUncertainty)?,
        );
        let right_difference = i128::from(
            right_observed
                .z_um
                .ok_or(AbstainReason::MissingPointUncertainty)?,
        ) - i128::from(
            right_reference
                .z_um
                .ok_or(AbstainReason::MissingPointUncertainty)?,
        );

        maximum_absolute_deviation = maximum_absolute_deviation
            .max(left_difference.unsigned_abs())
            .max(right_difference.unsigned_abs());

        let (segment_added, segment_removed) =
            segment_lobes_area2(width, left_difference, right_difference)?;
        added_area2 = checked_add_u128(added_area2, segment_added)?;
        removed_area2 = checked_add_u128(removed_area2, segment_removed)?;

        let left_uncertainty = checked_add_u128(
            checked_add_u128(
                u128::from(
                    left_reference
                        .uncertainty_um
                        .ok_or(AbstainReason::MissingPointUncertainty)?,
                ),
                u128::from(
                    left_observed
                        .uncertainty_um
                        .ok_or(AbstainReason::MissingPointUncertainty)?,
                ),
            )?,
            u128::from(request.registration.maximum_residual_um),
        )?;
        let right_uncertainty = checked_add_u128(
            checked_add_u128(
                u128::from(
                    right_reference
                        .uncertainty_um
                        .ok_or(AbstainReason::MissingPointUncertainty)?,
                ),
                u128::from(
                    right_observed
                        .uncertainty_um
                        .ok_or(AbstainReason::MissingPointUncertainty)?,
                ),
            )?,
            u128::from(request.registration.maximum_residual_um),
        )?;
        let segment_uncertainty = checked_mul_u128(
            u128::from(width),
            checked_add_u128(left_uncertainty, right_uncertainty)?,
        )?;
        uncertainty_area2 = checked_add_u128(uncertainty_area2, segment_uncertainty)?;
    }

    if maximum_unsupported_gap > limits.maximum_unsupported_gap_um {
        return Err(AbstainReason::UnsupportedGapTooLarge);
    }
    let region_length =
        u128::try_from(i128::from(limits.region_end_um) - i128::from(limits.region_start_um))
            .map_err(|_| AbstainReason::NumericOverflow)?;
    let coverage_numerator = checked_mul_u128(covered_length, 1_000)?;
    let coverage_permille = round_div_nearest(coverage_numerator, region_length)?;
    let coverage_permille =
        u16::try_from(coverage_permille).map_err(|_| AbstainReason::NumericOverflow)?;
    if coverage_permille < limits.minimum_coverage_permille {
        return Err(AbstainReason::CoverageBelowMinimum);
    }
    if uncertainty_area2 > limits.maximum_integrated_uncertainty_area2_um2 {
        return Err(AbstainReason::IntegratedUncertaintyTooLarge);
    }
    if covered_length == 0 {
        return Err(AbstainReason::CoverageBelowMinimum);
    }

    let signed_net_area2 = i128::try_from(added_area2)
        .map_err(|_| AbstainReason::NumericOverflow)?
        - i128::try_from(removed_area2).map_err(|_| AbstainReason::NumericOverflow)?;
    let absolute_change_area2 = checked_add_u128(added_area2, removed_area2)?;
    let mean_deviation =
        round_div_nearest(absolute_change_area2, checked_mul_u128(covered_length, 2)?)?;
    let maximum_absolute_deviation_um =
        u64::try_from(maximum_absolute_deviation).map_err(|_| AbstainReason::NumericOverflow)?;
    let mean_absolute_deviation_um =
        u64::try_from(mean_deviation).map_err(|_| AbstainReason::NumericOverflow)?;
    let covered_length_um =
        u64::try_from(covered_length).map_err(|_| AbstainReason::NumericOverflow)?;
    let unsupported_length_um =
        u64::try_from(unsupported_length).map_err(|_| AbstainReason::NumericOverflow)?;

    Ok(QualifiedComparison {
        workpiece_id: reference.workpiece_id.clone(),
        joint_revision: reference.joint_revision.clone(),
        layer_id: reference.layer_id.clone(),
        predecessor_pass_id: reference.predecessor_pass_id.clone(),
        transition_id: reference.transition_id.clone(),
        reference_scan_sha256: reference.scan_sha256.clone(),
        observed_scan_sha256: observed.scan_sha256.clone(),
        registration_receipt_sha256: request.registration.receipt_sha256.clone(),
        target_frame: request.registration.target_frame.clone(),
        reference_scan_id: reference.scan_id.clone(),
        observed_scan_id: observed.scan_id.clone(),
        x_grid_sha256: reference.x_grid_sha256.clone(),
        length_unit: "mm",
        maximum_unsupported_gap_units: limits.maximum_unsupported_gap_um,
        largest_unsupported_gap_units: maximum_unsupported_gap,
        minimum_coverage_permille: limits.minimum_coverage_permille,
        maximum_area_uncertainty_units: limits.maximum_integrated_uncertainty_area2_um2,
        metrics: GeometryMetrics {
            signed_net_area2_um2: signed_net_area2,
            added_area2_um2: added_area2,
            removed_area2_um2: removed_area2,
            absolute_change_area2_um2: absolute_change_area2,
            integrated_uncertainty_bound_area2_um2: uncertainty_area2,
            maximum_absolute_deviation_um,
            mean_absolute_deviation_um,
            coverage_permille,
            covered_length_um,
            unsupported_length_um,
            maximum_unsupported_gap_um: maximum_unsupported_gap,
            maximum_sample_spacing_um: maximum_sample_spacing,
        },
        claim_ceiling: "SYNTHETIC_REGISTERED_2D_PROFILE_COMPARISON_ONLY",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn points(xs: &[i64], zs: &[Option<i64>], uncertainty: Option<u64>) -> Vec<ProfilePoint> {
        xs.iter()
            .zip(zs.iter())
            .map(|(x_um, z_um)| ProfilePoint {
                x_um: *x_um,
                z_um: *z_um,
                uncertainty_um: if z_um.is_some() { uncertainty } else { None },
            })
            .collect()
    }

    fn base_request(reference_z: &[Option<i64>], observed_z: &[Option<i64>]) -> ComparisonRequest {
        let xs = [0, 1_000, 2_000];
        let reference_sha = digest('a');
        let observed_sha = digest('b');
        let reference = Profile {
            evidence_class: EvidenceClass::SyntheticContractOnly,
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            layer_id: "LAYER-02".to_owned(),
            predecessor_pass_id: "PASS-03".to_owned(),
            transition_id: "LAYER-02-AFTER-PASS-03".to_owned(),
            scan_id: "SCAN-BEFORE".to_owned(),
            scan_sha256: reference_sha.clone(),
            coordinate_frame: "WELD-FRAME-R1".to_owned(),
            x_grid_sha256: digest('d'),
            captured_at_tick: 90,
            points: points(&xs, reference_z, Some(2)),
        };
        let observed = Profile {
            evidence_class: EvidenceClass::SyntheticContractOnly,
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            layer_id: "LAYER-02".to_owned(),
            predecessor_pass_id: "PASS-03".to_owned(),
            transition_id: "LAYER-02-AFTER-PASS-03".to_owned(),
            scan_id: "SCAN-AFTER".to_owned(),
            scan_sha256: observed_sha.clone(),
            coordinate_frame: "WELD-FRAME-R1".to_owned(),
            x_grid_sha256: digest('d'),
            captured_at_tick: 95,
            points: points(&xs, observed_z, Some(3)),
        };
        let registration = RegistrationReceipt {
            receipt_sha256: digest('c'),
            reference_scan_sha256: reference_sha,
            observed_scan_sha256: observed_sha,
            source_frame: "SENSOR-FRAME-R1".to_owned(),
            target_frame: "WELD-FRAME-R1".to_owned(),
            calibration_id: "CAL-R1".to_owned(),
            algorithm_revision: "REG-ALG-R1".to_owned(),
            maximum_residual_um: 1,
            declared_residual_limit_um: 2,
        };
        ComparisonRequest {
            reference,
            observed,
            registration,
            limits: ComparisonLimits {
                region_start_um: 0,
                region_end_um: 2_000,
                minimum_coverage_permille: 1_000,
                maximum_unsupported_gap_um: 1_000,
                maximum_point_uncertainty_um: 10,
                maximum_registration_residual_um: 2,
                maximum_integrated_uncertainty_area2_um2: 30_000,
                decision_at_tick: 100,
                maximum_observation_age_ticks: 20,
            },
        }
    }

    fn qualified(request: &ComparisonRequest) -> QualifiedComparison {
        match compare_profiles(request) {
            ComparisonDecision::Qualified(result) => result,
            ComparisonDecision::Abstain(reason) => panic!("expected qualification, got {reason:?}"),
        }
    }

    fn abstains(request: &ComparisonRequest, expected: AbstainReason) {
        assert_eq!(
            compare_profiles(request),
            ComparisonDecision::Abstain(expected)
        );
    }

    #[test]
    fn identical_profiles_have_zero_signed_and_absolute_area() {
        let request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        let result = qualified(&request);
        assert_eq!(result.metrics.signed_net_area2_um2, 0);
        assert_eq!(result.metrics.absolute_change_area2_um2, 0);
        assert_eq!(result.metrics.coverage_permille, 1_000);
        assert_eq!(
            result.metrics.integrated_uncertainty_bound_area2_um2,
            24_000
        );
    }

    #[test]
    fn constant_positive_offset_has_exact_area_and_mean_deviation() {
        let request = base_request(
            &[Some(0), Some(0), Some(0)],
            &[Some(1_000), Some(1_000), Some(1_000)],
        );
        let result = qualified(&request);
        assert_eq!(result.metrics.signed_net_area2_um2, 4_000_000);
        assert_eq!(result.metrics.added_area2_um2, 4_000_000);
        assert_eq!(result.metrics.removed_area2_um2, 0);
        assert_eq!(result.metrics.maximum_absolute_deviation_um, 1_000);
        assert_eq!(result.metrics.mean_absolute_deviation_um, 1_000);
    }

    #[test]
    fn sign_crossing_is_split_into_exact_positive_and_negative_lobes() {
        let xs = [0, 1_000];
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.reference.points = points(&xs, &[Some(0), Some(0)], Some(2));
        request.observed.points = points(&xs, &[Some(1_000), Some(-1_000)], Some(3));
        request.limits.region_end_um = 1_000;
        let result = qualified(&request);
        assert_eq!(result.metrics.signed_net_area2_um2, 0);
        assert_eq!(result.metrics.added_area2_um2, 500_000);
        assert_eq!(result.metrics.removed_area2_um2, 500_000);
        assert_eq!(result.metrics.absolute_change_area2_um2, 1_000_000);
        assert_eq!(result.metrics.mean_absolute_deviation_um, 500);
    }

    #[test]
    fn constant_negative_offset_is_reported_as_removed_area() {
        let request = base_request(
            &[Some(0), Some(0), Some(0)],
            &[Some(-500), Some(-500), Some(-500)],
        );
        let result = qualified(&request);
        assert_eq!(result.metrics.added_area2_um2, 0);
        assert_eq!(result.metrics.removed_area2_um2, 2_000_000);
        assert_eq!(result.metrics.signed_net_area2_um2, -2_000_000);
    }

    #[test]
    fn long_internal_missing_region_is_rejected_even_with_valid_endpoints() {
        let xs = [0, 1_000, 2_000, 8_000, 9_000, 10_000];
        let zs = [Some(0), Some(0), None, None, Some(0), Some(0)];
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.reference.points = points(&xs, &zs, Some(2));
        request.observed.points = points(&xs, &zs, Some(3));
        request.limits.region_end_um = 10_000;
        request.limits.minimum_coverage_permille = 500;
        request.limits.maximum_unsupported_gap_um = 2_000;
        abstains(&request, AbstainReason::UnsupportedGapTooLarge);
    }

    #[test]
    fn incomplete_uncertainty_is_not_treated_as_zero() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.points[1].uncertainty_um = None;
        abstains(&request, AbstainReason::MissingPointUncertainty);
    }

    #[test]
    fn present_observation_without_uncertainty_is_rejected_when_reference_is_missing() {
        let mut request = base_request(&[Some(0), None, Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.points[1].uncertainty_um = None;
        abstains(&request, AbstainReason::MissingPointUncertainty);
    }

    #[test]
    fn missing_measurement_cannot_carry_an_uncertainty_value() {
        let mut request = base_request(&[Some(0), None, Some(0)], &[Some(0), None, Some(0)]);
        request.reference.points[1].uncertainty_um = Some(1);
        abstains(&request, AbstainReason::UncertaintyWithoutMeasurement);
    }

    #[test]
    fn point_uncertainty_above_limit_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.points[1].uncertainty_um = Some(11);
        abstains(&request, AbstainReason::PointUncertaintyTooLarge);
    }

    #[test]
    fn mismatched_reference_and_observed_station_grids_are_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.points[1].x_um = 999;
        abstains(&request, AbstainReason::StationGridMismatch);
    }

    #[test]
    fn duplicate_or_reversed_x_stations_are_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.reference.points[1].x_um = 0;
        request.observed.points[1].x_um = 0;
        abstains(&request, AbstainReason::NonMonotonicStations);
    }

    #[test]
    fn frame_mismatch_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.coordinate_frame = "OTHER-FRAME".to_owned();
        abstains(&request, AbstainReason::CoordinateFrameMismatch);
    }

    #[test]
    fn changed_scan_digest_breaks_registration_binding() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.scan_sha256 = digest('d');
        abstains(&request, AbstainReason::ScanIdentityMismatch);
    }

    #[test]
    fn mismatched_x_grid_digest_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.x_grid_sha256 = digest('e');
        abstains(&request, AbstainReason::XGridDigestMismatch);
    }

    #[test]
    fn malformed_scan_digest_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.reference.scan_sha256 = "not-a-sha256".to_owned();
        abstains(&request, AbstainReason::InvalidDigest);
    }

    #[test]
    fn excessive_registration_residual_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.registration.maximum_residual_um = 3;
        abstains(&request, AbstainReason::RegistrationResidualTooLarge);
    }

    #[test]
    fn stale_profile_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.reference.captured_at_tick = 70;
        abstains(&request, AbstainReason::ObservationTooOld);
    }

    #[test]
    fn future_profile_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.captured_at_tick = 101;
        abstains(&request, AbstainReason::FutureObservation);
    }

    #[test]
    fn observed_profile_must_follow_reference_profile_in_time() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.captured_at_tick = request.reference.captured_at_tick;
        abstains(&request, AbstainReason::ObservationOrderMismatch);
    }

    #[test]
    fn workpiece_mismatch_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.workpiece_id = "WID-SYNTH-02".to_owned();
        abstains(&request, AbstainReason::WorkpieceMismatch);
    }

    #[test]
    fn layer_identity_mismatch_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.layer_id = "LAYER-03".to_owned();
        abstains(&request, AbstainReason::LayerMismatch);
    }

    #[test]
    fn predecessor_pass_mismatch_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.predecessor_pass_id = "PASS-02".to_owned();
        abstains(&request, AbstainReason::PredecessorPassMismatch);
    }

    #[test]
    fn transition_mismatch_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.transition_id = "OTHER-TRANSITION".to_owned();
        abstains(&request, AbstainReason::TransitionMismatch);
    }

    #[test]
    fn simulated_or_measured_evidence_is_not_admitted_in_v0() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.evidence_class = EvidenceClass::Measured;
        abstains(&request, AbstainReason::PhysicalEvidenceNotAdmitted);
    }

    #[test]
    fn integrated_uncertainty_above_limit_abstains() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.limits.maximum_integrated_uncertainty_area2_um2 = 23_999;
        abstains(&request, AbstainReason::IntegratedUncertaintyTooLarge);
    }

    #[test]
    fn registration_receipt_cannot_bind_a_different_target_frame() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.registration.target_frame = "OTHER-FRAME".to_owned();
        abstains(&request, AbstainReason::CoordinateFrameMismatch);
    }

    #[test]
    fn joint_revision_mismatch_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.joint_revision = "JOINT-R2".to_owned();
        abstains(&request, AbstainReason::JointRevisionMismatch);
    }

    #[test]
    fn malformed_registration_receipt_digest_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.registration.receipt_sha256 = "bad".to_owned();
        abstains(&request, AbstainReason::InvalidDigest);
    }

    #[test]
    fn registration_receipt_reference_scan_mismatch_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.registration.reference_scan_sha256 = digest('d');
        abstains(&request, AbstainReason::ScanIdentityMismatch);
    }

    #[test]
    fn identical_scan_ids_are_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.observed.scan_id = request.reference.scan_id.clone();
        abstains(&request, AbstainReason::ScanIdentityMismatch);
    }

    #[test]
    fn profile_must_cover_the_declared_region_exactly() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.reference.points[0].x_um = 1;
        request.observed.points[0].x_um = 1;
        abstains(&request, AbstainReason::RegionNotExactlyCovered);
    }

    #[test]
    fn coverage_below_minimum_is_rejected_without_gap_bridging() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.reference.points[1].z_um = None;
        request.observed.points[1].z_um = None;
        request.limits.maximum_unsupported_gap_um = 2_000;
        request.limits.minimum_coverage_permille = 750;
        abstains(&request, AbstainReason::CoverageBelowMinimum);
    }

    #[test]
    fn zero_minimum_coverage_is_an_invalid_limit() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.limits.minimum_coverage_permille = 0;
        abstains(&request, AbstainReason::InvalidCoverageLimit);
    }

    #[test]
    fn invalid_region_and_zero_gap_limits_abstain() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.limits.region_start_um = request.limits.region_end_um;
        abstains(&request, AbstainReason::InvalidRegion);

        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.limits.maximum_unsupported_gap_um = 0;
        abstains(&request, AbstainReason::InvalidGapLimit);
    }

    #[test]
    fn empty_scan_identity_is_rejected() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.reference.scan_id.clear();
        abstains(&request, AbstainReason::EmptyIdentity);
    }

    #[test]
    fn a_single_profile_station_is_not_a_comparison() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        request.reference.points.truncate(1);
        request.observed.points.truncate(1);
        abstains(&request, AbstainReason::ProfileTooShort);
    }

    #[test]
    fn arithmetic_overflow_abstains_instead_of_wrapping() {
        let mut request = base_request(&[Some(0), Some(0), Some(0)], &[Some(0), Some(0), Some(0)]);
        let xs = [i64::MIN, i64::MAX];
        request.reference.points = points(&xs, &[Some(0), Some(0)], Some(1));
        request.observed.points = points(&xs, &[Some(i64::MAX), Some(i64::MIN)], Some(1));
        request.limits.region_start_um = i64::MIN;
        request.limits.region_end_um = i64::MAX;
        request.limits.maximum_unsupported_gap_um = u64::MAX;
        request.limits.maximum_integrated_uncertainty_area2_um2 = u128::MAX;
        abstains(&request, AbstainReason::NumericOverflow);
    }
}
