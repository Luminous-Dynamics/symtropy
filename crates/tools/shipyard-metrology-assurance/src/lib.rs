//! Conservative uncertainty propagation and guarded geometry decisions.
//!
//! V0.1 uses symbolic integer units and deterministic absolute-error bounds.
//! It does not qualify a sensor, parse point clouds, accept welds or authorize
//! production release. RSS is intentionally not used without a covariance
//! model and a declared statistical interpretation.

use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationStatus {
    Verified,
    Unverified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UncertaintyKind {
    SensorCalibration,
    SurfaceResponse,
    RobotPose,
    Registration,
    Datum,
    ThermalCorrection,
    FeatureExtraction,
    EnvironmentalCorrection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncertaintyContribution {
    pub source_id: String,
    pub kind: UncertaintyKind,
    pub scope_id: String,
    pub unit: String,
    pub absolute_bound_units: u64,
    pub valid_from_tick: u64,
    pub valid_until_tick: u64,
    pub verification: VerificationStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncertaintyBudget {
    pub unit: String,
    pub contributions: Vec<UncertaintyContribution>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Measurement {
    pub measurement_id: String,
    pub scope_id: String,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub coordinate_frame: String,
    pub unit: String,
    pub scan_id: String,
    pub scan_sha256: [u8; 32],
    pub registration_receipt_sha256: [u8; 32],
    pub observed_at_tick: u64,
    pub value_units: i64,
    pub budget: UncertaintyBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedBudget {
    pub unit: String,
    pub absolute_bound_units: u64,
    pub source_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedMeasurement {
    pub measurement_id: String,
    pub scope_id: String,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub coordinate_frame: String,
    pub unit: String,
    pub scan_id: String,
    pub scan_sha256: [u8; 32],
    pub registration_receipt_sha256: [u8; 32],
    pub observed_at_tick: u64,
    pub value_units: i64,
    pub uncertainty: ValidatedBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DifferenceRequest {
    pub before: Measurement,
    pub after: Measurement,
    pub difference_scope_id: String,
    pub transform_budget: UncertaintyBudget,
    pub datum_budget: UncertaintyBudget,
    pub decision_at_tick: u64,
    pub maximum_observation_age_ticks: u64,
    pub specification_lower_units: i64,
    pub specification_upper_units: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConformityStatus {
    Conforming,
    NonconformingBelow,
    NonconformingAbove,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    EmptyIdentity,
    EmptyBudget,
    EmptySourceIdentity,
    DuplicateSourceIdentity,
    UnitMismatch,
    UncertaintyKindMismatch,
    ScopeMismatch,
    UnverifiedContribution,
    ContributionNotYetValid,
    ContributionExpired,
    InvalidContributionValidity,
    ArithmeticOverflow,
    MissingScanDigest,
    MissingRegistrationReceipt,
    FutureObservation,
    ObservationTooOld,
    WorkpieceMismatch,
    JointRevisionMismatch,
    CoordinateFrameMismatch,
    NonMonotonicObservations,
    InvalidSpecificationInterval,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeometryDecisionReceipt {
    pub before_measurement_id: String,
    pub after_measurement_id: String,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub coordinate_frame: String,
    pub before_scan_id: String,
    pub after_scan_id: String,
    pub before_scan_sha256: [u8; 32],
    pub after_scan_sha256: [u8; 32],
    pub before_registration_receipt_sha256: [u8; 32],
    pub after_registration_receipt_sha256: [u8; 32],
    pub difference_units: i64,
    pub uncertainty_bound_units: u64,
    pub interval_lower_units: i128,
    pub interval_upper_units: i128,
    pub specification_lower_units: i64,
    pub specification_upper_units: i64,
    pub conformity: ConformityStatus,
    pub uncertainty_source_ids: Vec<String>,
    pub automatic_acceptance: bool,
    pub production_release_authorized: bool,
    pub claim_ceiling: &'static str,
}

fn is_empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn valid_digest(digest: &[u8; 32]) -> bool {
    digest.iter().any(|byte| *byte != 0)
}

fn validate_budget(
    budget: &UncertaintyBudget,
    expected_unit: &str,
    expected_scope: &str,
    measurement_tick: u64,
    expected_kind: Option<UncertaintyKind>,
) -> Result<ValidatedBudget, AdmissionError> {
    if budget.contributions.is_empty() {
        return Err(AdmissionError::EmptyBudget);
    }
    if is_empty(&budget.unit) || is_empty(expected_unit) || budget.unit != expected_unit {
        return Err(AdmissionError::UnitMismatch);
    }
    let mut seen = HashSet::new();
    let mut total = 0u64;
    let mut source_ids = Vec::with_capacity(budget.contributions.len());

    for contribution in &budget.contributions {
        if is_empty(&contribution.source_id) {
            return Err(AdmissionError::EmptySourceIdentity);
        }
        if !seen.insert(contribution.source_id.as_str()) {
            return Err(AdmissionError::DuplicateSourceIdentity);
        }
        if contribution.scope_id != expected_scope {
            return Err(AdmissionError::ScopeMismatch);
        }
        if contribution.unit != expected_unit {
            return Err(AdmissionError::UnitMismatch);
        }
        if expected_kind.is_some_and(|kind| contribution.kind != kind) {
            return Err(AdmissionError::UncertaintyKindMismatch);
        }
        if contribution.valid_from_tick > contribution.valid_until_tick {
            return Err(AdmissionError::InvalidContributionValidity);
        }
        if measurement_tick < contribution.valid_from_tick {
            return Err(AdmissionError::ContributionNotYetValid);
        }
        if measurement_tick > contribution.valid_until_tick {
            return Err(AdmissionError::ContributionExpired);
        }
        if contribution.verification != VerificationStatus::Verified {
            return Err(AdmissionError::UnverifiedContribution);
        }
        total = total
            .checked_add(contribution.absolute_bound_units)
            .ok_or(AdmissionError::ArithmeticOverflow)?;
        source_ids.push(contribution.source_id.clone());
    }
    source_ids.sort();

    Ok(ValidatedBudget {
        unit: expected_unit.to_owned(),
        absolute_bound_units: total,
        source_ids,
    })
}

fn validate_measurement(
    measurement: &Measurement,
    decision_at_tick: u64,
    maximum_observation_age_ticks: u64,
) -> Result<ValidatedMeasurement, AdmissionError> {
    let identities = [
        measurement.measurement_id.as_str(),
        measurement.scope_id.as_str(),
        measurement.workpiece_id.as_str(),
        measurement.joint_revision.as_str(),
        measurement.coordinate_frame.as_str(),
        measurement.scan_id.as_str(),
        measurement.budget.unit.as_str(),
    ];
    if identities.into_iter().any(is_empty) {
        return Err(AdmissionError::EmptyIdentity);
    }
    if !valid_digest(&measurement.scan_sha256) {
        return Err(AdmissionError::MissingScanDigest);
    }
    if !valid_digest(&measurement.registration_receipt_sha256) {
        return Err(AdmissionError::MissingRegistrationReceipt);
    }
    if measurement.observed_at_tick > decision_at_tick {
        return Err(AdmissionError::FutureObservation);
    }
    if decision_at_tick - measurement.observed_at_tick > maximum_observation_age_ticks {
        return Err(AdmissionError::ObservationTooOld);
    }
    let uncertainty = validate_budget(
        &measurement.budget,
        &measurement.unit,
        &measurement.scope_id,
        measurement.observed_at_tick,
        None,
    )?;

    Ok(ValidatedMeasurement {
        measurement_id: measurement.measurement_id.clone(),
        scope_id: measurement.scope_id.clone(),
        workpiece_id: measurement.workpiece_id.clone(),
        joint_revision: measurement.joint_revision.clone(),
        coordinate_frame: measurement.coordinate_frame.clone(),
        unit: measurement.unit.clone(),
        scan_id: measurement.scan_id.clone(),
        scan_sha256: measurement.scan_sha256,
        registration_receipt_sha256: measurement.registration_receipt_sha256,
        observed_at_tick: measurement.observed_at_tick,
        value_units: measurement.value_units,
        uncertainty,
    })
}

fn combine_budgets(
    budgets: &[&ValidatedBudget],
) -> Result<(u64, Vec<String>), AdmissionError> {
    let mut total = 0u64;
    let mut all_ids = HashSet::new();
    let mut source_ids = Vec::new();
    let mut expected_unit: Option<&str> = None;

    for budget in budgets {
        if let Some(unit) = expected_unit {
            if budget.unit != unit {
                return Err(AdmissionError::UnitMismatch);
            }
        } else {
            expected_unit = Some(&budget.unit);
        }
        total = total
            .checked_add(budget.absolute_bound_units)
            .ok_or(AdmissionError::ArithmeticOverflow)?;
        for source_id in &budget.source_ids {
            if !all_ids.insert(source_id.as_str()) {
                return Err(AdmissionError::DuplicateSourceIdentity);
            }
            source_ids.push(source_id.clone());
        }
    }
    source_ids.sort();
    Ok((total, source_ids))
}

fn classify_interval(
    lower: i128,
    upper: i128,
    specification_lower: i64,
    specification_upper: i64,
) -> Result<ConformityStatus, AdmissionError> {
    if specification_lower > specification_upper {
        return Err(AdmissionError::InvalidSpecificationInterval);
    }
    let lower_spec = i128::from(specification_lower);
    let upper_spec = i128::from(specification_upper);

    if lower >= lower_spec && upper <= upper_spec {
        Ok(ConformityStatus::Conforming)
    } else if upper < lower_spec {
        Ok(ConformityStatus::NonconformingBelow)
    } else if lower > upper_spec {
        Ok(ConformityStatus::NonconformingAbove)
    } else {
        Ok(ConformityStatus::Indeterminate)
    }
}

/// Validate two scans and return a conservative before/after geometry receipt.
///
/// Uncertainty bounds are deterministic absolute-error envelopes. This method
/// sums them by the triangle inequality; it does not assume independence or
/// use RSS. A receipt is never an automatic weld-acceptance decision.
pub fn decide_difference(
    request: &DifferenceRequest,
) -> Result<GeometryDecisionReceipt, AdmissionError> {
    if is_empty(&request.difference_scope_id) {
        return Err(AdmissionError::EmptyIdentity);
    }

    let before = validate_measurement(
        &request.before,
        request.decision_at_tick,
        request.maximum_observation_age_ticks,
    )?;
    let after = validate_measurement(
        &request.after,
        request.decision_at_tick,
        request.maximum_observation_age_ticks,
    )?;

    if before.workpiece_id != after.workpiece_id {
        return Err(AdmissionError::WorkpieceMismatch);
    }
    if before.joint_revision != after.joint_revision {
        return Err(AdmissionError::JointRevisionMismatch);
    }
    if before.coordinate_frame != after.coordinate_frame {
        return Err(AdmissionError::CoordinateFrameMismatch);
    }
    if after.observed_at_tick <= before.observed_at_tick {
        return Err(AdmissionError::NonMonotonicObservations);
    }
    if before.unit != after.unit
        || before.uncertainty.unit != before.unit
        || after.uncertainty.unit != after.unit
    {
        return Err(AdmissionError::UnitMismatch);
    }

    let transform = validate_budget(
        &request.transform_budget,
        &before.unit,
        &request.difference_scope_id,
        request.decision_at_tick,
        Some(UncertaintyKind::Registration),
    )?;
    let datum = validate_budget(
        &request.datum_budget,
        &before.unit,
        &request.difference_scope_id,
        request.decision_at_tick,
        Some(UncertaintyKind::Datum),
    )?;

    let difference_units = after
        .value_units
        .checked_sub(before.value_units)
        .ok_or(AdmissionError::ArithmeticOverflow)?;
    let (uncertainty_bound_units, source_ids) = combine_budgets(&[
        &before.uncertainty,
        &after.uncertainty,
        &transform,
        &datum,
    ])?;
    let interval_lower_units =
        i128::from(difference_units) - i128::from(uncertainty_bound_units);
    let interval_upper_units =
        i128::from(difference_units) + i128::from(uncertainty_bound_units);
    let conformity = classify_interval(
        interval_lower_units,
        interval_upper_units,
        request.specification_lower_units,
        request.specification_upper_units,
    )?;

    Ok(GeometryDecisionReceipt {
        before_measurement_id: before.measurement_id,
        after_measurement_id: after.measurement_id,
        workpiece_id: before.workpiece_id,
        joint_revision: before.joint_revision,
        coordinate_frame: before.coordinate_frame,
        before_scan_id: before.scan_id,
        after_scan_id: after.scan_id,
        before_scan_sha256: before.scan_sha256,
        after_scan_sha256: after.scan_sha256,
        before_registration_receipt_sha256: before.registration_receipt_sha256,
        after_registration_receipt_sha256: after.registration_receipt_sha256,
        difference_units,
        uncertainty_bound_units,
        interval_lower_units,
        interval_upper_units,
        specification_lower_units: request.specification_lower_units,
        specification_upper_units: request.specification_upper_units,
        conformity,
        uncertainty_source_ids: source_ids,
        automatic_acceptance: false,
        production_release_authorized: false,
        claim_ceiling: "SYNTHETIC_MEASUREMENT_ASSURANCE_CONTRACT_ONLY",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contribution(
        source_id: &str,
        scope_id: &str,
        unit: &str,
        bound: u64,
        valid_from_tick: u64,
        valid_until_tick: u64,
    ) -> UncertaintyContribution {
        UncertaintyContribution {
            source_id: source_id.to_owned(),
            kind: UncertaintyKind::SensorCalibration,
            scope_id: scope_id.to_owned(),
            unit: unit.to_owned(),
            absolute_bound_units: bound,
            valid_from_tick,
            valid_until_tick,
            verification: VerificationStatus::Verified,
        }
    }

    fn measurement(
        id: &str,
        scan_id: &str,
        scope_id: &str,
        value: i64,
        observed_at_tick: u64,
        bound: u64,
        digest: u8,
    ) -> Measurement {
        Measurement {
            measurement_id: id.to_owned(),
            scope_id: scope_id.to_owned(),
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            coordinate_frame: "SYNTHETIC-JOINT-FRAME".to_owned(),
            unit: "SYMBOLIC_AREA".to_owned(),
            scan_id: scan_id.to_owned(),
            scan_sha256: [digest; 32],
            registration_receipt_sha256: [digest.wrapping_add(1); 32],
            observed_at_tick,
            value_units: value,
            budget: UncertaintyBudget {
                unit: "SYMBOLIC_AREA".to_owned(),
                contributions: vec![contribution(
                    &format!("SRC-{id}"),
                    scope_id,
                    "SYMBOLIC_AREA",
                    bound,
                    0,
                    1_000,
                )],
            },
        }
    }

    fn base_request(before_value: i64, after_value: i64) -> DifferenceRequest {
        let before_scope = "MEAS-BEFORE";
        let after_scope = "MEAS-AFTER";
        let delta_scope = "DELTA-TRANSFORM";
        DifferenceRequest {
            before: measurement(
                "M-BEFORE", "SCAN-BEFORE", before_scope, before_value, 100, 2, 0x11
            ),
            after: measurement(
                "M-AFTER", "SCAN-AFTER", after_scope, after_value, 110, 3, 0x33
            ),
            difference_scope_id: delta_scope.to_owned(),
            transform_budget: UncertaintyBudget {
                unit: "SYMBOLIC_AREA".to_owned(),
                contributions: vec![contribution(
                    "SRC-REGISTRATION",
                    delta_scope,
                    "SYMBOLIC_AREA",
                    1,
                    0,
                    200,
                )],
            },
            datum_budget: UncertaintyBudget {
                unit: "SYMBOLIC_AREA".to_owned(),
                contributions: vec![contribution(
                    "SRC-DATUM",
                    delta_scope,
                    "SYMBOLIC_AREA",
                    2,
                    0,
                    200,
                )],
            },
            decision_at_tick: 120,
            maximum_observation_age_ticks: 30,
            specification_lower_units: 50,
            specification_upper_units: 70,
        }
    }

    #[test]
    fn exact_budget_and_guard_band_known_answer() {
        let receipt = decide_difference(&base_request(200, 260)).expect("known case");
        assert_eq!(receipt.difference_units, 60);
        assert_eq!(receipt.uncertainty_bound_units, 8);
        assert_eq!(receipt.interval_lower_units, 52);
        assert_eq!(receipt.interval_upper_units, 68);
        assert_eq!(receipt.conformity, ConformityStatus::Conforming);
        assert_eq!(receipt.uncertainty_source_ids.len(), 4);
        assert!(!receipt.automatic_acceptance);
        assert!(!receipt.production_release_authorized);
    }

    #[test]
    fn uncertainty_interval_touching_specification_boundary_is_conforming() {
        let mut request = base_request(200, 258);
        request.specification_lower_units = 50;
        request.specification_upper_units = 70;
        let receipt = decide_difference(&request).expect("overlap case");
        assert_eq!(receipt.difference_units, 58);
        assert_eq!(receipt.interval_lower_units, 50);
        assert_eq!(receipt.interval_upper_units, 66);
        assert_eq!(receipt.conformity, ConformityStatus::Conforming);
    }

    #[test]
    fn uncertainty_crossing_specification_boundary_is_indeterminate() {
        let mut request = base_request(200, 255);
        let receipt = decide_difference(&request).expect("boundary overlap");
        assert_eq!(receipt.difference_units, 55);
        assert_eq!(receipt.interval_lower_units, 47);
        assert_eq!(receipt.interval_upper_units, 63);
        assert_eq!(receipt.conformity, ConformityStatus::Indeterminate);
    }

    #[test]
    fn wholly_below_specification_is_nonconforming_below() {
        let receipt = decide_difference(&base_request(200, 230)).expect("below case");
        assert_eq!(receipt.interval_lower_units, 22);
        assert_eq!(receipt.interval_upper_units, 38);
        assert_eq!(receipt.conformity, ConformityStatus::NonconformingBelow);
    }

    #[test]
    fn wholly_above_specification_is_nonconforming_above() {
        let receipt = decide_difference(&base_request(200, 290)).expect("above case");
        assert_eq!(receipt.interval_lower_units, 82);
        assert_eq!(receipt.interval_upper_units, 98);
        assert_eq!(receipt.conformity, ConformityStatus::NonconformingAbove);
    }

    #[test]
    fn invalid_specification_interval_is_rejected() {
        let mut request = base_request(200, 260);
        request.specification_lower_units = 71;
        request.specification_upper_units = 70;
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::InvalidSpecificationInterval)
        );
    }

    #[test]
    fn stale_observation_is_rejected() {
        let mut request = base_request(200, 260);
        request.before.observed_at_tick = 70;
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::ObservationTooOld)
        );
    }

    #[test]
    fn future_observation_is_rejected() {
        let mut request = base_request(200, 260);
        request.after.observed_at_tick = 121;
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::FutureObservation)
        );
    }

    #[test]
    fn expired_calibration_contribution_is_rejected() {
        let mut request = base_request(200, 260);
        request.before.budget.contributions[0].valid_until_tick = 90;
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::ContributionExpired)
        );
    }

    #[test]
    fn unverified_contribution_is_rejected() {
        let mut request = base_request(200, 260);
        request.after.budget.contributions[0].verification = VerificationStatus::Unverified;
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::UnverifiedContribution)
        );
    }

    #[test]
    fn wrong_unit_is_rejected_without_implicit_conversion() {
        let mut request = base_request(200, 260);
        request.after.budget.contributions[0].unit = "MILLIMETRE".to_owned();
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::UnitMismatch)
        );
    }

    #[test]
    fn contribution_from_another_measurement_scope_is_rejected() {
        let mut request = base_request(200, 260);
        request.before.budget.contributions[0].scope_id = "OTHER-MEASUREMENT".to_owned();
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::ScopeMismatch)
        );
    }

    #[test]
    fn duplicate_source_ids_across_budgets_are_rejected() {
        let mut request = base_request(200, 260);
        request.datum_budget.contributions[0].source_id = "SRC-REGISTRATION".to_owned();
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::DuplicateSourceIdentity)
        );
    }

    #[test]
    fn measurement_unit_must_match_its_uncertainty_budget() {
        let mut request = base_request(200, 260);
        request.after.unit = "MILLIMETRE".to_owned();
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::UnitMismatch)
        );
    }

    #[test]
    fn transform_budget_accepts_only_registration_contributions() {
        let mut request = base_request(200, 260);
        request.transform_budget.contributions[0].kind =
            UncertaintyKind::SensorCalibration;
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::UncertaintyKindMismatch)
        );
    }

    #[test]
    fn datum_budget_accepts_only_datum_contributions() {
        let mut request = base_request(200, 260);
        request.datum_budget.contributions[0].kind =
            UncertaintyKind::Registration;
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::UncertaintyKindMismatch)
        );
    }

    #[test]
    fn missing_scan_digest_is_rejected() {
        let mut request = base_request(200, 260);
        request.before.scan_sha256 = [0; 32];
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::MissingScanDigest)
        );
    }

    #[test]
    fn missing_registration_receipt_is_rejected() {
        let mut request = base_request(200, 260);
        request.after.registration_receipt_sha256 = [0; 32];
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::MissingRegistrationReceipt)
        );
    }

    #[test]
    fn workpiece_mismatch_is_rejected() {
        let mut request = base_request(200, 260);
        request.after.workpiece_id = "WID-SYNTH-02".to_owned();
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::WorkpieceMismatch)
        );
    }

    #[test]
    fn joint_revision_mismatch_is_rejected() {
        let mut request = base_request(200, 260);
        request.after.joint_revision = "JOINT-R2".to_owned();
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::JointRevisionMismatch)
        );
    }

    #[test]
    fn coordinate_frame_mismatch_is_rejected() {
        let mut request = base_request(200, 260);
        request.after.coordinate_frame = "OTHER-FRAME".to_owned();
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::CoordinateFrameMismatch)
        );
    }

    #[test]
    fn non_monotonic_scan_order_is_rejected() {
        let mut request = base_request(200, 260);
        request.after.observed_at_tick = request.before.observed_at_tick;
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::NonMonotonicObservations)
        );
    }

    #[test]
    fn empty_budget_is_rejected() {
        let mut request = base_request(200, 260);
        request.after.budget.contributions.clear();
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::EmptyBudget)
        );
    }

    #[test]
    fn arithmetic_overflow_in_uncertainty_budget_is_rejected() {
        let mut request = base_request(200, 260);
        request.before.budget.contributions[0].absolute_bound_units = u64::MAX;
        request.after.budget.contributions[0].absolute_bound_units = 1;
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::ArithmeticOverflow)
        );
    }

    #[test]
    fn signed_difference_overflow_is_rejected() {
        let request = base_request(i64::MIN, i64::MAX);
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::ArithmeticOverflow)
        );
    }

    #[test]
    fn contribution_not_yet_valid_is_rejected() {
        let mut request = base_request(200, 260);
        request.before.budget.contributions[0].valid_from_tick = 105;
        assert_eq!(
            decide_difference(&request),
            Err(AdmissionError::ContributionNotYetValid)
        );
    }
}
