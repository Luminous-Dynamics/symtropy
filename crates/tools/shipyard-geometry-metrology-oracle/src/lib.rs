//! Synthetic-only cross-sectional geometry metrology oracle.
//!
//! This crate qualifies and compares two discretized profiles. It is not a
//! physical scan adapter, weld-quality predictor, NDT acceptance system, or
//! robot controller. No interpolation is performed across unsupported gaps.

use std::convert::TryFrom;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticContractOnly,
    MeasuredPhysical,
    Simulation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileSample {
    pub x_um: i64,
    pub z_um: Option<i64>,
    pub uncertainty_bound_um: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceProfile {
    pub evidence_class: EvidenceClass,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub transition_pass_id: String,
    pub scan_id: String,
    pub source_sha256: [u8; 32],
    pub coordinate_frame: String,
    pub x_unit: String,
    pub z_unit: String,
    pub observed_at_tick: u64,
    pub samples: Vec<ProfileSample>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparisonRequest {
    pub reference: SurfaceProfile,
    pub observed: SurfaceProfile,
    pub registration_receipt_sha256: [u8; 32],
    pub registration_residual_bound_um: u64,
    pub maximum_registration_residual_um: u64,
    pub decision_at_tick: u64,
    pub maximum_observation_age_ticks: u64,
    pub roi_start_x_um: i64,
    pub roi_end_x_um: i64,
    pub minimum_coverage_permille: u16,
    pub maximum_unsupported_gap_um: u64,
    pub maximum_area_uncertainty2_um2: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbstainReason {
    EmptyIdentity,
    PhysicalEvidenceNotAdmitted,
    MissingScanDigest,
    MissingRegistrationReceipt,
    SameScanIdentity,
    WorkpieceMismatch,
    JointRevisionMismatch,
    LayerMismatch,
    TransitionPassMismatch,
    CoordinateFrameMismatch,
    UnitMismatch,
    EmptyProfile,
    TooFewSamples,
    NonMonotonicCoordinates,
    PartialMissingSample,
    XGridMismatch,
    InvalidRegionOfInterest,
    RegionNotOnSampleGrid,
    ObservationFromFuture,
    ObservationTooOld,
    RegistrationResidualTooLarge,
    CoverageThresholdInvalid,
    NoSupportedIntervals,
    CoverageBelowMinimum,
    UnsupportedGapTooLarge,
    AreaUncertaintyTooLarge,
    NumericOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeometryComparison {
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub transition_pass_id: String,
    pub reference_scan_id: String,
    pub observed_scan_id: String,
    pub reference_scan_sha256: [u8; 32],
    pub observed_scan_sha256: [u8; 32],
    pub registration_receipt_sha256: [u8; 32],
    pub roi_start_x_um: i64,
    pub roi_end_x_um: i64,
    pub coverage_permille: u16,
    pub supported_length_um: u64,
    pub maximum_unsupported_gap_um: u64,
    /// Twice the signed area in µm², avoiding half-unit rounding.
    pub net_area2_um2: i128,
    /// Twice the positive (added-surface) area in µm², rounded upward.
    pub added_area2_um2: u128,
    /// Twice the negative (removed-surface) area in µm², rounded upward.
    pub removed_area2_um2: u128,
    /// Twice the conservative integrated vertical uncertainty in µm².
    pub uncertainty_area2_um2: u128,
    pub net_area_interval_lower2_um2: i128,
    pub net_area_interval_upper2_um2: i128,
}

fn is_empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn nonzero_digest(digest: &[u8; 32]) -> bool {
    digest.iter().any(|byte| *byte != 0)
}

fn validate_profile(profile: &SurfaceProfile) -> Result<(), AbstainReason> {
    if is_empty(&profile.workpiece_id)
        || is_empty(&profile.joint_revision)
        || is_empty(&profile.layer_id)
        || is_empty(&profile.transition_pass_id)
        || is_empty(&profile.scan_id)
        || is_empty(&profile.coordinate_frame)
    {
        return Err(AbstainReason::EmptyIdentity);
    }
    if profile.evidence_class != EvidenceClass::SyntheticContractOnly {
        return Err(AbstainReason::PhysicalEvidenceNotAdmitted);
    }
    if !nonzero_digest(&profile.source_sha256) {
        return Err(AbstainReason::MissingScanDigest);
    }
    if profile.x_unit != "um" || profile.z_unit != "um" {
        return Err(AbstainReason::UnitMismatch);
    }
    if profile.samples.is_empty() {
        return Err(AbstainReason::EmptyProfile);
    }
    if profile.samples.len() < 2 {
        return Err(AbstainReason::TooFewSamples);
    }

    for pair in profile.samples.windows(2) {
        if pair[1].x_um <= pair[0].x_um {
            return Err(AbstainReason::NonMonotonicCoordinates);
        }
    }
    for sample in &profile.samples {
        if sample.z_um.is_some() != sample.uncertainty_bound_um.is_some() {
            return Err(AbstainReason::PartialMissingSample);
        }
    }
    Ok(())
}

fn ceil_div(numerator: u128, denominator: u128) -> Result<u128, AbstainReason> {
    if denominator == 0 {
        return Err(AbstainReason::NumericOverflow);
    }
    let adjusted = numerator
        .checked_add(denominator - 1)
        .ok_or(AbstainReason::NumericOverflow)?;
    Ok(adjusted / denominator)
}

fn add_u128(left: u128, right: u128) -> Result<u128, AbstainReason> {
    left.checked_add(right)
        .ok_or(AbstainReason::NumericOverflow)
}

fn checked_mul_u128(left: u128, right: u128) -> Result<u128, AbstainReason> {
    left.checked_mul(right)
        .ok_or(AbstainReason::NumericOverflow)
}

fn segment_areas2(
    dx_um: u128,
    delta_start_um: i128,
    delta_end_um: i128,
) -> Result<(i128, u128, u128), AbstainReason> {
    let endpoint_sum = delta_start_um
        .checked_add(delta_end_um)
        .ok_or(AbstainReason::NumericOverflow)?;
    let signed_net = i128::try_from(dx_um)
        .map_err(|_| AbstainReason::NumericOverflow)?
        .checked_mul(endpoint_sum)
        .ok_or(AbstainReason::NumericOverflow)?;

    let positive;
    let negative;
    if delta_start_um >= 0 && delta_end_um >= 0 {
        let sum = u128::try_from(delta_start_um)
            .map_err(|_| AbstainReason::NumericOverflow)?
            .checked_add(u128::try_from(delta_end_um).map_err(|_| AbstainReason::NumericOverflow)?)
            .ok_or(AbstainReason::NumericOverflow)?;
        positive = checked_mul_u128(dx_um, sum)?;
        negative = 0;
    } else if delta_start_um <= 0 && delta_end_um <= 0 {
        let sum = delta_start_um
            .unsigned_abs()
            .checked_add(delta_end_um.unsigned_abs())
            .ok_or(AbstainReason::NumericOverflow)?;
        positive = 0;
        negative = checked_mul_u128(dx_um, sum)?;
    } else {
        let start_abs = delta_start_um.unsigned_abs();
        let end_abs = delta_end_um.unsigned_abs();
        let denominator = start_abs
            .checked_add(end_abs)
            .ok_or(AbstainReason::NumericOverflow)?;
        let start_square = checked_mul_u128(start_abs, start_abs)?;
        let end_square = checked_mul_u128(end_abs, end_abs)?;
        let start_area = ceil_div(checked_mul_u128(dx_um, start_square)?, denominator)?;
        let end_area = ceil_div(checked_mul_u128(dx_um, end_square)?, denominator)?;
        if delta_start_um > 0 {
            positive = start_area;
            negative = end_area;
        } else {
            positive = end_area;
            negative = start_area;
        }
    }
    Ok((signed_net, positive, negative))
}

fn point_delta(
    reference: &ProfileSample,
    observed: &ProfileSample,
) -> Result<Option<(i128, u128)>, AbstainReason> {
    match (
        reference.z_um,
        reference.uncertainty_bound_um,
        observed.z_um,
        observed.uncertainty_bound_um,
    ) {
        (Some(reference_z), Some(reference_u), Some(observed_z), Some(observed_u)) => {
            let delta = i128::from(observed_z) - i128::from(reference_z);
            let uncertainty = u128::from(reference_u)
                .checked_add(u128::from(observed_u))
                .ok_or(AbstainReason::NumericOverflow)?;
            Ok(Some((delta, uncertainty)))
        }
        (None, None, None, None) => Ok(None),
        _ => Err(AbstainReason::PartialMissingSample),
    }
}

/// Compares two synthetic-only profiles using gap-aware, piecewise-linear integration.
///
/// The result is a measurement-contract output, not a physical weld prediction.
/// It never interpolates across unsupported intervals or authorizes execution.
pub fn compare_profiles(request: &ComparisonRequest) -> Result<GeometryComparison, AbstainReason> {
    validate_profile(&request.reference)?;
    validate_profile(&request.observed)?;

    if !nonzero_digest(&request.registration_receipt_sha256) {
        return Err(AbstainReason::MissingRegistrationReceipt);
    }
    if request.reference.scan_id == request.observed.scan_id {
        return Err(AbstainReason::SameScanIdentity);
    }
    if request.reference.workpiece_id != request.observed.workpiece_id {
        return Err(AbstainReason::WorkpieceMismatch);
    }
    if request.reference.joint_revision != request.observed.joint_revision {
        return Err(AbstainReason::JointRevisionMismatch);
    }
    if request.reference.layer_id != request.observed.layer_id {
        return Err(AbstainReason::LayerMismatch);
    }
    if request.reference.transition_pass_id != request.observed.transition_pass_id {
        return Err(AbstainReason::TransitionPassMismatch);
    }
    if request.reference.coordinate_frame != request.observed.coordinate_frame {
        return Err(AbstainReason::CoordinateFrameMismatch);
    }
    if request.reference.samples.len() != request.observed.samples.len()
        || request
            .reference
            .samples
            .iter()
            .zip(&request.observed.samples)
            .any(|(left, right)| left.x_um != right.x_um)
    {
        return Err(AbstainReason::XGridMismatch);
    }
    if request.minimum_coverage_permille > 1_000 {
        return Err(AbstainReason::CoverageThresholdInvalid);
    }
    if request.roi_start_x_um >= request.roi_end_x_um {
        return Err(AbstainReason::InvalidRegionOfInterest);
    }
    if request.registration_residual_bound_um > request.maximum_registration_residual_um {
        return Err(AbstainReason::RegistrationResidualTooLarge);
    }
    if request.reference.observed_at_tick > request.decision_at_tick
        || request.observed.observed_at_tick > request.decision_at_tick
    {
        return Err(AbstainReason::ObservationFromFuture);
    }
    if request
        .decision_at_tick
        .saturating_sub(request.reference.observed_at_tick)
        > request.maximum_observation_age_ticks
        || request
            .decision_at_tick
            .saturating_sub(request.observed.observed_at_tick)
            > request.maximum_observation_age_ticks
    {
        return Err(AbstainReason::ObservationTooOld);
    }

    let xs: Vec<i64> = request
        .reference
        .samples
        .iter()
        .map(|sample| sample.x_um)
        .collect();
    let start_index = xs
        .iter()
        .position(|x| *x == request.roi_start_x_um)
        .ok_or(AbstainReason::RegionNotOnSampleGrid)?;
    let end_index = xs
        .iter()
        .position(|x| *x == request.roi_end_x_um)
        .ok_or(AbstainReason::RegionNotOnSampleGrid)?;
    if start_index >= end_index {
        return Err(AbstainReason::InvalidRegionOfInterest);
    }

    let total_length_um =
        u128::try_from(i128::from(request.roi_end_x_um) - i128::from(request.roi_start_x_um))
            .map_err(|_| AbstainReason::NumericOverflow)?;
    let mut supported_length_um = 0u128;
    let mut current_unsupported_run_um = 0u128;
    let mut maximum_unsupported_gap_um = 0u128;
    let mut net_area2_um2 = 0i128;
    let mut added_area2_um2 = 0u128;
    let mut removed_area2_um2 = 0u128;
    let mut uncertainty_area2_um2 = 0u128;
    let registration_uncertainty = u128::from(request.registration_residual_bound_um);

    for index in start_index..end_index {
        let reference_start = &request.reference.samples[index];
        let reference_end = &request.reference.samples[index + 1];
        let observed_start = &request.observed.samples[index];
        let observed_end = &request.observed.samples[index + 1];
        let dx_um =
            u128::try_from(i128::from(reference_end.x_um) - i128::from(reference_start.x_um))
                .map_err(|_| AbstainReason::NumericOverflow)?;

        let start_delta = point_delta(reference_start, observed_start)?;
        let end_delta = point_delta(reference_end, observed_end)?;
        let supported = start_delta.is_some()
            && end_delta.is_some()
            && dx_um <= u128::from(request.maximum_unsupported_gap_um);

        if !supported {
            current_unsupported_run_um = add_u128(current_unsupported_run_um, dx_um)?;
            maximum_unsupported_gap_um = maximum_unsupported_gap_um.max(current_unsupported_run_um);
            continue;
        }

        current_unsupported_run_um = 0;
        supported_length_um = add_u128(supported_length_um, dx_um)?;

        let (delta_start, uncertainty_start) =
            start_delta.ok_or(AbstainReason::NoSupportedIntervals)?;
        let (delta_end, uncertainty_end) = end_delta.ok_or(AbstainReason::NoSupportedIntervals)?;

        let (net, added, removed) = segment_areas2(dx_um, delta_start, delta_end)?;
        let net = i128::try_from(net).map_err(|_| AbstainReason::NumericOverflow)?;
        net_area2_um2 = net_area2_um2
            .checked_add(net)
            .ok_or(AbstainReason::NumericOverflow)?;
        added_area2_um2 = add_u128(added_area2_um2, added)?;
        removed_area2_um2 = add_u128(removed_area2_um2, removed)?;

        let total_uncertainty_start = add_u128(uncertainty_start, registration_uncertainty)?;
        let total_uncertainty_end = add_u128(uncertainty_end, registration_uncertainty)?;
        let uncertainty_sum = add_u128(total_uncertainty_start, total_uncertainty_end)?;
        let segment_uncertainty = checked_mul_u128(dx_um, uncertainty_sum)?;
        uncertainty_area2_um2 = add_u128(uncertainty_area2_um2, segment_uncertainty)?;
    }

    if supported_length_um == 0 {
        return Err(AbstainReason::NoSupportedIntervals);
    }
    let coverage = checked_mul_u128(supported_length_um, 1_000)? / total_length_um;
    let coverage_permille = u16::try_from(coverage).map_err(|_| AbstainReason::NumericOverflow)?;
    if coverage_permille < request.minimum_coverage_permille {
        return Err(AbstainReason::CoverageBelowMinimum);
    }
    if maximum_unsupported_gap_um > u128::from(request.maximum_unsupported_gap_um) {
        return Err(AbstainReason::UnsupportedGapTooLarge);
    }
    if uncertainty_area2_um2 > request.maximum_area_uncertainty2_um2 {
        return Err(AbstainReason::AreaUncertaintyTooLarge);
    }

    let uncertainty_i128 =
        i128::try_from(uncertainty_area2_um2).map_err(|_| AbstainReason::NumericOverflow)?;
    let net_area_interval_lower2_um2 = net_area2_um2
        .checked_sub(uncertainty_i128)
        .ok_or(AbstainReason::NumericOverflow)?;
    let net_area_interval_upper2_um2 = net_area2_um2
        .checked_add(uncertainty_i128)
        .ok_or(AbstainReason::NumericOverflow)?;

    Ok(GeometryComparison {
        workpiece_id: request.reference.workpiece_id.clone(),
        joint_revision: request.reference.joint_revision.clone(),
        layer_id: request.reference.layer_id.clone(),
        transition_pass_id: request.reference.transition_pass_id.clone(),
        reference_scan_id: request.reference.scan_id.clone(),
        observed_scan_id: request.observed.scan_id.clone(),
        reference_scan_sha256: request.reference.source_sha256,
        observed_scan_sha256: request.observed.source_sha256,
        registration_receipt_sha256: request.registration_receipt_sha256,
        roi_start_x_um: request.roi_start_x_um,
        roi_end_x_um: request.roi_end_x_um,
        coverage_permille,
        supported_length_um: u64::try_from(supported_length_um)
            .map_err(|_| AbstainReason::NumericOverflow)?,
        maximum_unsupported_gap_um: u64::try_from(maximum_unsupported_gap_um)
            .map_err(|_| AbstainReason::NumericOverflow)?,
        net_area2_um2,
        added_area2_um2,
        removed_area2_um2,
        uncertainty_area2_um2,
        net_area_interval_lower2_um2,
        net_area_interval_upper2_um2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(
        scan_id: &str,
        digest: u8,
        z_values: &[Option<i64>],
        uncertainties: &[Option<u64>],
    ) -> SurfaceProfile {
        let xs = [0, 10, 20];
        let samples = xs
            .into_iter()
            .zip(z_values.iter().copied())
            .zip(uncertainties.iter().copied())
            .map(|((x_um, z_um), uncertainty_bound_um)| ProfileSample {
                x_um,
                z_um,
                uncertainty_bound_um,
            })
            .collect();
        SurfaceProfile {
            evidence_class: EvidenceClass::SyntheticContractOnly,
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            layer_id: "LAYER-02".to_owned(),
            transition_pass_id: "PASS-02".to_owned(),
            scan_id: scan_id.to_owned(),
            source_sha256: [digest; 32],
            coordinate_frame: "SYNTHETIC-JOINT-FRAME".to_owned(),
            x_unit: "um".to_owned(),
            z_unit: "um".to_owned(),
            observed_at_tick: 100,
            samples,
        }
    }

    fn request(reference: SurfaceProfile, observed: SurfaceProfile) -> ComparisonRequest {
        ComparisonRequest {
            reference,
            observed,
            registration_receipt_sha256: [0x55; 32],
            registration_residual_bound_um: 0,
            maximum_registration_residual_um: 2,
            decision_at_tick: 100,
            maximum_observation_age_ticks: 10,
            roi_start_x_um: 0,
            roi_end_x_um: 20,
            minimum_coverage_permille: 1_000,
            maximum_unsupported_gap_um: 10,
            maximum_area_uncertainty2_um2: 1_000,
        }
    }

    fn flat_reference() -> SurfaceProfile {
        profile(
            "SCAN-BEFORE",
            0x11,
            &[Some(0), Some(0), Some(0)],
            &[Some(0); 3],
        )
    }

    #[test]
    fn positive_only_surface_change_has_exact_area() {
        let result = compare_profiles(&request(
            flat_reference(),
            profile(
                "SCAN-AFTER",
                0x22,
                &[Some(0), Some(4), Some(0)],
                &[Some(0); 3],
            ),
        ))
        .expect("positive geometry change should be admitted");
        assert_eq!(result.net_area2_um2, 80);
        assert_eq!(result.added_area2_um2, 80);
        assert_eq!(result.removed_area2_um2, 0);
        assert_eq!(result.coverage_permille, 1_000);
    }

    #[test]
    fn negative_only_surface_change_has_exact_removed_area() {
        let result = compare_profiles(&request(
            flat_reference(),
            profile(
                "SCAN-AFTER",
                0x22,
                &[Some(0), Some(-4), Some(0)],
                &[Some(0); 3],
            ),
        ))
        .expect("negative geometry change should be admitted");
        assert_eq!(result.net_area2_um2, -80);
        assert_eq!(result.added_area2_um2, 0);
        assert_eq!(result.removed_area2_um2, 80);
    }

    #[test]
    fn mixed_surface_change_splits_zero_crossing_exactly() {
        let result = compare_profiles(&request(
            flat_reference(),
            profile(
                "SCAN-AFTER",
                0x22,
                &[Some(0), Some(-2), Some(2)],
                &[Some(0); 3],
            ),
        ))
        .expect("mixed geometry change should be admitted");
        assert_eq!(result.net_area2_um2, -20);
        assert_eq!(result.added_area2_um2, 10);
        assert_eq!(result.removed_area2_um2, 30);
    }

    #[test]
    fn unchanged_surface_has_zero_net_added_and_removed_area() {
        let result = compare_profiles(&request(
            flat_reference(),
            profile("SCAN-AFTER", 0x22, &[Some(0); 3], &[Some(0); 3]),
        ))
        .expect("unchanged surface should be admitted");
        assert_eq!(result.net_area2_um2, 0);
        assert_eq!(result.added_area2_um2, 0);
        assert_eq!(result.removed_area2_um2, 0);
    }

    #[test]
    fn uncertainty_integrates_reference_observed_and_registration_bounds() {
        let mut input = request(
            profile("SCAN-BEFORE", 0x11, &[Some(0); 3], &[Some(1); 3]),
            profile("SCAN-AFTER", 0x22, &[Some(4); 3], &[Some(2); 3]),
        );
        input.registration_residual_bound_um = 1;
        let result = compare_profiles(&input).expect("bounded uncertainty should pass");
        assert_eq!(result.uncertainty_area2_um2, 160);
        assert_eq!(result.net_area_interval_lower2_um2, 0);
        assert_eq!(result.net_area_interval_upper2_um2, 320);
    }

    #[test]
    fn area_uncertainty_above_threshold_abstains() {
        let mut input = request(
            profile("SCAN-BEFORE", 0x11, &[Some(0); 3], &[Some(1); 3]),
            profile("SCAN-AFTER", 0x22, &[Some(4); 3], &[Some(2); 3]),
        );
        input.registration_residual_bound_um = 1;
        input.maximum_area_uncertainty2_um2 = 119;
        assert_eq!(
            compare_profiles(&input),
            Err(AbstainReason::AreaUncertaintyTooLarge)
        );
    }

    #[test]
    fn workpiece_identity_mismatch_abstains() {
        let mut observed = profile("SCAN-AFTER", 0x22, &[Some(1); 3], &[Some(0); 3]);
        observed.workpiece_id = "WID-SYNTH-02".to_owned();
        assert_eq!(
            compare_profiles(&request(flat_reference(), observed)),
            Err(AbstainReason::WorkpieceMismatch)
        );
    }

    #[test]
    fn revision_mismatch_abstains() {
        let mut observed = profile("SCAN-AFTER", 0x22, &[Some(1); 3], &[Some(0); 3]);
        observed.joint_revision = "JOINT-R2".to_owned();
        assert_eq!(
            compare_profiles(&request(flat_reference(), observed)),
            Err(AbstainReason::JointRevisionMismatch)
        );
    }

    #[test]
    fn frame_mismatch_abstains() {
        let mut observed = profile("SCAN-AFTER", 0x22, &[Some(1); 3], &[Some(0); 3]);
        observed.coordinate_frame = "OTHER-FRAME".to_owned();
        assert_eq!(
            compare_profiles(&request(flat_reference(), observed)),
            Err(AbstainReason::CoordinateFrameMismatch)
        );
    }

    #[test]
    fn missing_scan_digest_abstains() {
        let mut reference = flat_reference();
        reference.source_sha256 = [0; 32];
        assert_eq!(
            compare_profiles(&request(
                reference,
                profile("SCAN-AFTER", 0x22, &[Some(1); 3], &[Some(0); 3])
            )),
            Err(AbstainReason::MissingScanDigest)
        );
    }

    #[test]
    fn missing_registration_receipt_abstains() {
        let mut input = request(
            flat_reference(),
            profile("SCAN-AFTER", 0x22, &[Some(1); 3], &[Some(0); 3]),
        );
        input.registration_receipt_sha256 = [0; 32];
        assert_eq!(
            compare_profiles(&input),
            Err(AbstainReason::MissingRegistrationReceipt)
        );
    }

    #[test]
    fn x_grid_mismatch_abstains_without_interpolation() {
        let mut observed = profile("SCAN-AFTER", 0x22, &[Some(1); 3], &[Some(0); 3]);
        observed.samples[1].x_um = 11;
        assert_eq!(
            compare_profiles(&request(flat_reference(), observed)),
            Err(AbstainReason::XGridMismatch)
        );
    }

    #[test]
    fn nonmonotonic_profile_coordinates_abstain() {
        let mut observed = profile("SCAN-AFTER", 0x22, &[Some(1); 3], &[Some(0); 3]);
        observed.samples[2].x_um = 10;
        assert_eq!(
            compare_profiles(&request(flat_reference(), observed)),
            Err(AbstainReason::NonMonotonicCoordinates)
        );
    }

    #[test]
    fn missing_uncertainty_is_not_imputed_to_zero() {
        let observed = profile(
            "SCAN-AFTER",
            0x22,
            &[Some(1), Some(1), Some(1)],
            &[Some(0), None, Some(0)],
        );
        assert_eq!(
            compare_profiles(&request(flat_reference(), observed)),
            Err(AbstainReason::PartialMissingSample)
        );
    }

    #[test]
    fn unsupported_gap_is_never_interpolated() {
        let reference = profile(
            "SCAN-BEFORE",
            0x11,
            &[Some(0), None, Some(0)],
            &[Some(0), None, Some(0)],
        );
        let observed = profile(
            "SCAN-AFTER",
            0x22,
            &[Some(1), None, Some(1)],
            &[Some(0), None, Some(0)],
        );
        let mut input = request(reference, observed);
        input.minimum_coverage_permille = 0;
        input.maximum_unsupported_gap_um = 10;
        assert_eq!(
            compare_profiles(&input),
            Err(AbstainReason::NoSupportedIntervals)
        );
    }

    #[test]
    fn minimum_coverage_is_enforced() {
        let mut input = request(
            flat_reference(),
            profile("SCAN-AFTER", 0x22, &[Some(1); 3], &[Some(0); 3]),
        );
        input.minimum_coverage_permille = 1_001;
        assert_eq!(
            compare_profiles(&input),
            Err(AbstainReason::CoverageThresholdInvalid)
        );
    }

    #[test]
    fn future_observation_abstains() {
        let mut observed = profile("SCAN-AFTER", 0x22, &[Some(1); 3], &[Some(0); 3]);
        observed.observed_at_tick = 101;
        assert_eq!(
            compare_profiles(&request(flat_reference(), observed)),
            Err(AbstainReason::ObservationFromFuture)
        );
    }

    #[test]
    fn stale_observation_abstains() {
        let mut observed = profile("SCAN-AFTER", 0x22, &[Some(1); 3], &[Some(0); 3]);
        observed.observed_at_tick = 50;
        assert_eq!(
            compare_profiles(&request(flat_reference(), observed)),
            Err(AbstainReason::ObservationTooOld)
        );
    }

    #[test]
    fn excessive_registration_residual_abstains() {
        let mut input = request(
            flat_reference(),
            profile("SCAN-AFTER", 0x22, &[Some(1); 3], &[Some(0); 3]),
        );
        input.registration_residual_bound_um = 3;
        assert_eq!(
            compare_profiles(&input),
            Err(AbstainReason::RegistrationResidualTooLarge)
        );
    }

    #[test]
    fn same_scan_identity_abstains() {
        let reference = flat_reference();
        let observed = profile("SCAN-BEFORE", 0x22, &[Some(1); 3], &[Some(0); 3]);
        assert_eq!(
            compare_profiles(&request(reference, observed)),
            Err(AbstainReason::SameScanIdentity)
        );
    }

    #[test]
    fn physical_observations_are_not_admitted_in_v01() {
        let mut observed = profile("SCAN-AFTER", 0x22, &[Some(1); 3], &[Some(0); 3]);
        observed.evidence_class = EvidenceClass::MeasuredPhysical;
        assert_eq!(
            compare_profiles(&request(flat_reference(), observed)),
            Err(AbstainReason::PhysicalEvidenceNotAdmitted)
        );
    }
}
