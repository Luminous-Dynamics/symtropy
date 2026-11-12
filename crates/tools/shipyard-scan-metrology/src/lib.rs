//! Evidence-gated inter-pass optical scan qualification.
//!
//! V0 accepts synthetic contract evidence only. It qualifies the measurement
//! chain before geometry is consumed by a planner. It does not certify a real
//! scanner, implement ISO acceptance tests, or authorize welding execution.

use std::collections::HashSet;

const MAX_UNCERTAINTY_COMPONENTS: usize = 16;
const SHA256_HEX_LENGTH: usize = 64;
const DETERMINANT_IDENTITY_MILLI: i64 = 1_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticContractOnly,
    MeasuredObservation,
    Simulation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfacePreparation {
    DocumentedClean,
    Unknown,
    SlagOrSpatterPresent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrelationPolicy {
    IndependenceDeclared,
    CorrelatedKnown,
    CorrelationUnknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibrationEvidence {
    pub sensor_id: String,
    pub calibration_receipt_sha256: String,
    pub reference_artifact_id: String,
    pub reference_certificate_id: String,
    pub calibration_config_revision: String,
    pub acquisition_config_revision: String,
    pub valid_from_tick: u64,
    pub expires_at_tick: u64,
    pub environment_in_calibration_envelope: bool,
    pub reference_length_error_units: u64,
    pub maximum_reference_length_error_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistrationEvidence {
    pub source_scan_sha256: String,
    pub registration_receipt_sha256: String,
    pub source_frame: String,
    pub target_frame: String,
    pub icp_converged: bool,
    pub rotation_determinant_milli: i64,
    pub orthogonality_error_units: u64,
    pub maximum_orthogonality_error_units: u64,
    pub independent_reference_target_count: u32,
    pub required_reference_target_count: u32,
    pub reference_target_rms_residual_units: u64,
    pub maximum_reference_target_rms_residual_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceObservation {
    pub coordinate_frame: String,
    pub coordinate_unit: String,
    pub preparation: SurfacePreparation,
    pub cleaning_receipt_sha256: String,
    pub occlusion_assessment_receipt_sha256: String,
    pub expected_profile_count: u64,
    pub received_profile_count: u64,
    pub dropped_profile_count: u64,
    pub declared_profile_coverage_permille: u16,
    pub minimum_profile_coverage_permille: u16,
    pub maximum_dropped_profile_permille: u16,
    pub total_point_count: u64,
    pub valid_point_count: u64,
    pub invalid_point_count: u64,
    pub clipped_point_count: u64,
    pub minimum_valid_point_count: u64,
    pub maximum_invalid_point_permille: u16,
    pub occlusion_permille: u16,
    pub maximum_occlusion_permille: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncertaintyComponent {
    pub component_id: String,
    pub standard_uncertainty_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncertaintyBudget {
    pub components: Vec<UncertaintyComponent>,
    pub correlation_policy: CorrelationPolicy,
    pub coverage_factor_permille: u32,
    pub maximum_expanded_uncertainty_units: u64,
    pub decision_tolerance_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionPolicy {
    pub policy_id: String,
    pub revision: String,
    pub frozen_before_acquisition: bool,
    pub future_information_used: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanEvidence {
    pub evidence_class: EvidenceClass,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub predecessor_pass_id: String,
    pub scan_id: String,
    pub source_scan_sha256: String,
    pub acquisition_config_revision: String,
    pub acquisition_tick: u64,
    pub decision_tick: u64,
    pub maximum_observation_age_ticks: u64,
    pub calibration: CalibrationEvidence,
    pub registration: RegistrationEvidence,
    pub observation: SurfaceObservation,
    pub uncertainty: UncertaintyBudget,
    pub decision_policy: DecisionPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbstainReason {
    EmptyIdentity,
    NonSyntheticEvidenceNotAdmitted,
    InvalidSourceDigest,
    InvalidCalibrationReceipt,
    MissingReferenceArtifactOrCertificate,
    CalibrationExpiredOrNotYetValid,
    CalibrationConfigurationMismatch,
    EnvironmentOutsideCalibrationEnvelope,
    ReferenceLengthCheckFailed,
    InvalidRegistrationReceipt,
    RegistrationSourceDigestMismatch,
    CoordinateFrameMismatch,
    IcpDidNotConverge,
    InvalidRigidTransform,
    RegistrationOrthogonalityFailed,
    IndependentTargetCountMismatch,
    IndependentTargetResidualFailed,
    MissingCleaningReceipt,
    SurfacePreparationNotQualified,
    MissingOcclusionAssessmentReceipt,
    InvalidCoordinateUnit,
    InvalidProfileCounts,
    ProfileCoverageMismatch,
    ProfileCoverageBelowMinimum,
    DroppedProfilesAboveLimit,
    InvalidPointCounts,
    ClippedPointCountInvalid,
    InvalidPointFractionAboveLimit,
    ValidPointCountBelowMinimum,
    OcclusionAboveLimit,
    ObservationFromFuture,
    ObservationTooOld,
    FutureInformationLeakage,
    DecisionPolicyNotFrozen,
    MissingOrExcessiveUncertaintyComponents,
    DuplicateUncertaintyComponent,
    CorrelationPolicyNotAdmitted,
    InvalidCoverageFactor,
    InvalidDecisionTolerance,
    ExpandedUncertaintyAboveLimit,
    NumericOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualificationReceipt {
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub predecessor_pass_id: String,
    pub scan_id: String,
    pub source_scan_sha256: String,
    pub calibration_receipt_sha256: String,
    pub registration_receipt_sha256: String,
    pub decision_policy_id: String,
    pub decision_policy_revision: String,
    pub profile_coverage_permille: u16,
    pub occlusion_permille: u16,
    pub expanded_uncertainty_units: u64,
    pub decision_tolerance_units: u64,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanDecision {
    Qualified(QualificationReceipt),
    Abstain(Vec<AbstainReason>),
}

fn empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == SHA256_HEX_LENGTH
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        && value.bytes().any(|byte| byte != b'0')
}

fn push_reason(reasons: &mut Vec<AbstainReason>, reason: AbstainReason) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

fn ceil_sqrt(value: u128) -> u128 {
    if value == 0 {
        return 0;
    }
    let mut low = 0u128;
    let mut high = u64::MAX as u128 + 1;
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if middle <= value / middle {
            low = middle;
        } else {
            high = middle;
        }
    }
    if low * low == value { low } else { low + 1 }
}

fn expanded_uncertainty(budget: &UncertaintyBudget) -> Result<u64, AbstainReason> {
    if budget.components.is_empty() || budget.components.len() > MAX_UNCERTAINTY_COMPONENTS {
        return Err(AbstainReason::MissingOrExcessiveUncertaintyComponents);
    }
    if budget.correlation_policy != CorrelationPolicy::IndependenceDeclared {
        return Err(AbstainReason::CorrelationPolicyNotAdmitted);
    }
    if budget.coverage_factor_permille == 0 {
        return Err(AbstainReason::InvalidCoverageFactor);
    }
    let mut ids = HashSet::new();
    let mut sum_squares = 0u128;
    for component in &budget.components {
        if empty(&component.component_id) || !ids.insert(component.component_id.as_str()) {
            return Err(AbstainReason::DuplicateUncertaintyComponent);
        }
        let value = component.standard_uncertainty_units as u128;
        let square = value
            .checked_mul(value)
            .ok_or(AbstainReason::NumericOverflow)?;
        sum_squares = sum_squares
            .checked_add(square)
            .ok_or(AbstainReason::NumericOverflow)?;
    }
    let standard_uncertainty = ceil_sqrt(sum_squares);
    let scaled = standard_uncertainty
        .checked_mul(budget.coverage_factor_permille as u128)
        .ok_or(AbstainReason::NumericOverflow)?;
    let expanded = scaled
        .checked_add(999)
        .ok_or(AbstainReason::NumericOverflow)?
        / 1_000;
    u64::try_from(expanded).map_err(|_| AbstainReason::NumericOverflow)
}

fn validate_profile_and_point_counts(
    observation: &SurfaceObservation,
    reasons: &mut Vec<AbstainReason>,
) {
    if observation.expected_profile_count == 0
        || observation.received_profile_count > observation.expected_profile_count
        || observation.dropped_profile_count > observation.expected_profile_count
        || observation
            .received_profile_count
            .checked_add(observation.dropped_profile_count)
            != Some(observation.expected_profile_count)
        || observation.declared_profile_coverage_permille > 1_000
        || observation.minimum_profile_coverage_permille > 1_000
        || observation.maximum_dropped_profile_permille > 1_000
    {
        push_reason(reasons, AbstainReason::InvalidProfileCounts);
        return;
    }
    let Some(computed_coverage) = observation
        .received_profile_count
        .checked_mul(1_000)
        .map(|value| value / observation.expected_profile_count)
    else {
        push_reason(reasons, AbstainReason::NumericOverflow);
        return;
    };
    if computed_coverage != observation.declared_profile_coverage_permille as u64 {
        push_reason(reasons, AbstainReason::ProfileCoverageMismatch);
    }
    if computed_coverage < observation.minimum_profile_coverage_permille as u64 {
        push_reason(reasons, AbstainReason::ProfileCoverageBelowMinimum);
    }
    let Some(dropped_permille) = observation
        .dropped_profile_count
        .checked_mul(1_000)
        .map(|value| value / observation.expected_profile_count)
    else {
        push_reason(reasons, AbstainReason::NumericOverflow);
        return;
    };
    if dropped_permille > observation.maximum_dropped_profile_permille as u64 {
        push_reason(reasons, AbstainReason::DroppedProfilesAboveLimit);
    }

    if observation.total_point_count == 0
        || observation
            .valid_point_count
            .checked_add(observation.invalid_point_count)
            != Some(observation.total_point_count)
        || observation.maximum_invalid_point_permille > 1_000
    {
        push_reason(reasons, AbstainReason::InvalidPointCounts);
        return;
    }
    if observation.clipped_point_count > observation.invalid_point_count {
        push_reason(reasons, AbstainReason::ClippedPointCountInvalid);
    }
    let Some(invalid_permille) = observation
        .invalid_point_count
        .checked_mul(1_000)
        .map(|value| value / observation.total_point_count)
    else {
        push_reason(reasons, AbstainReason::NumericOverflow);
        return;
    };
    if invalid_permille > observation.maximum_invalid_point_permille as u64 {
        push_reason(reasons, AbstainReason::InvalidPointFractionAboveLimit);
    }
    if observation.valid_point_count < observation.minimum_valid_point_count {
        push_reason(reasons, AbstainReason::ValidPointCountBelowMinimum);
    }
    if observation.occlusion_permille > 1_000
        || observation.maximum_occlusion_permille > 1_000
        || observation.occlusion_permille > observation.maximum_occlusion_permille
    {
        push_reason(reasons, AbstainReason::OcclusionAboveLimit);
    }
}

pub fn evaluate_scan(evidence: &ScanEvidence) -> ScanDecision {
    let mut reasons = Vec::new();
    let identity_values = [
        evidence.workpiece_id.as_str(),
        evidence.joint_revision.as_str(),
        evidence.layer_id.as_str(),
        evidence.predecessor_pass_id.as_str(),
        evidence.scan_id.as_str(),
        evidence.acquisition_config_revision.as_str(),
        evidence.calibration.sensor_id.as_str(),
        evidence.calibration.calibration_config_revision.as_str(),
        evidence.calibration.acquisition_config_revision.as_str(),
        evidence.calibration.reference_artifact_id.as_str(),
        evidence.calibration.reference_certificate_id.as_str(),
        evidence.registration.source_frame.as_str(),
        evidence.registration.target_frame.as_str(),
        evidence.observation.coordinate_frame.as_str(),
        evidence.observation.coordinate_unit.as_str(),
        evidence.decision_policy.policy_id.as_str(),
        evidence.decision_policy.revision.as_str(),
    ];
    if identity_values.into_iter().any(empty) {
        push_reason(&mut reasons, AbstainReason::EmptyIdentity);
    }
    if evidence.evidence_class != EvidenceClass::SyntheticContractOnly {
        push_reason(&mut reasons, AbstainReason::NonSyntheticEvidenceNotAdmitted);
    }
    if !valid_sha256(&evidence.source_scan_sha256) {
        push_reason(&mut reasons, AbstainReason::InvalidSourceDigest);
    }
    if !valid_sha256(&evidence.calibration.calibration_receipt_sha256) {
        push_reason(&mut reasons, AbstainReason::InvalidCalibrationReceipt);
    }
    if empty(&evidence.calibration.reference_artifact_id)
        || empty(&evidence.calibration.reference_certificate_id)
    {
        push_reason(
            &mut reasons,
            AbstainReason::MissingReferenceArtifactOrCertificate,
        );
    }
    if evidence.acquisition_tick < evidence.calibration.valid_from_tick
        || evidence.acquisition_tick > evidence.calibration.expires_at_tick
    {
        push_reason(&mut reasons, AbstainReason::CalibrationExpiredOrNotYetValid);
    }
    if evidence.acquisition_config_revision != evidence.calibration.calibration_config_revision
        || evidence.acquisition_config_revision != evidence.calibration.acquisition_config_revision
    {
        push_reason(
            &mut reasons,
            AbstainReason::CalibrationConfigurationMismatch,
        );
    }
    if !evidence.calibration.environment_in_calibration_envelope {
        push_reason(
            &mut reasons,
            AbstainReason::EnvironmentOutsideCalibrationEnvelope,
        );
    }
    if evidence.calibration.reference_length_error_units
        > evidence.calibration.maximum_reference_length_error_units
    {
        push_reason(&mut reasons, AbstainReason::ReferenceLengthCheckFailed);
    }

    if !valid_sha256(&evidence.registration.registration_receipt_sha256) {
        push_reason(&mut reasons, AbstainReason::InvalidRegistrationReceipt);
    }
    if evidence.registration.source_scan_sha256 != evidence.source_scan_sha256 {
        push_reason(
            &mut reasons,
            AbstainReason::RegistrationSourceDigestMismatch,
        );
    }
    if evidence.registration.target_frame != evidence.observation.coordinate_frame {
        push_reason(&mut reasons, AbstainReason::CoordinateFrameMismatch);
    }
    if !evidence.registration.icp_converged {
        push_reason(&mut reasons, AbstainReason::IcpDidNotConverge);
    }
    if evidence
        .registration
        .rotation_determinant_milli
        .abs_diff(DETERMINANT_IDENTITY_MILLI)
        > 10
    {
        push_reason(&mut reasons, AbstainReason::InvalidRigidTransform);
    }
    if evidence.registration.orthogonality_error_units
        > evidence.registration.maximum_orthogonality_error_units
    {
        push_reason(&mut reasons, AbstainReason::RegistrationOrthogonalityFailed);
    }
    if evidence.registration.required_reference_target_count == 0
        || evidence.registration.independent_reference_target_count
            != evidence.registration.required_reference_target_count
    {
        push_reason(&mut reasons, AbstainReason::IndependentTargetCountMismatch);
    }
    if evidence.registration.reference_target_rms_residual_units
        > evidence
            .registration
            .maximum_reference_target_rms_residual_units
    {
        push_reason(&mut reasons, AbstainReason::IndependentTargetResidualFailed);
    }

    if !valid_sha256(&evidence.observation.cleaning_receipt_sha256) {
        push_reason(&mut reasons, AbstainReason::MissingCleaningReceipt);
    }
    if evidence.observation.preparation != SurfacePreparation::DocumentedClean {
        push_reason(&mut reasons, AbstainReason::SurfacePreparationNotQualified);
    }
    if !valid_sha256(&evidence.observation.occlusion_assessment_receipt_sha256) {
        push_reason(
            &mut reasons,
            AbstainReason::MissingOcclusionAssessmentReceipt,
        );
    }
    if evidence.observation.coordinate_unit != "mm" {
        push_reason(&mut reasons, AbstainReason::InvalidCoordinateUnit);
    }
    validate_profile_and_point_counts(&evidence.observation, &mut reasons);

    if evidence.acquisition_tick > evidence.decision_tick {
        push_reason(&mut reasons, AbstainReason::ObservationFromFuture);
    } else if evidence.decision_tick - evidence.acquisition_tick
        > evidence.maximum_observation_age_ticks
    {
        push_reason(&mut reasons, AbstainReason::ObservationTooOld);
    }
    if evidence.decision_policy.future_information_used {
        push_reason(&mut reasons, AbstainReason::FutureInformationLeakage);
    }
    if !evidence.decision_policy.frozen_before_acquisition {
        push_reason(&mut reasons, AbstainReason::DecisionPolicyNotFrozen);
    }
    if evidence.uncertainty.decision_tolerance_units == 0 {
        push_reason(&mut reasons, AbstainReason::InvalidDecisionTolerance);
    }

    let expanded = match expanded_uncertainty(&evidence.uncertainty) {
        Ok(value) => Some(value),
        Err(reason) => {
            push_reason(&mut reasons, reason);
            None
        }
    };
    if let Some(value) = expanded {
        if value > evidence.uncertainty.maximum_expanded_uncertainty_units
            || value > evidence.uncertainty.decision_tolerance_units
        {
            push_reason(&mut reasons, AbstainReason::ExpandedUncertaintyAboveLimit);
        }
    }

    if !reasons.is_empty() {
        return ScanDecision::Abstain(reasons);
    }
    let expanded_uncertainty_units = expanded.unwrap_or(0);
    ScanDecision::Qualified(QualificationReceipt {
        workpiece_id: evidence.workpiece_id.clone(),
        joint_revision: evidence.joint_revision.clone(),
        layer_id: evidence.layer_id.clone(),
        predecessor_pass_id: evidence.predecessor_pass_id.clone(),
        scan_id: evidence.scan_id.clone(),
        source_scan_sha256: evidence.source_scan_sha256.clone(),
        calibration_receipt_sha256: evidence.calibration.calibration_receipt_sha256.clone(),
        registration_receipt_sha256: evidence.registration.registration_receipt_sha256.clone(),
        decision_policy_id: evidence.decision_policy.policy_id.clone(),
        decision_policy_revision: evidence.decision_policy.revision.clone(),
        profile_coverage_permille: evidence.observation.declared_profile_coverage_permille,
        occlusion_permille: evidence.observation.occlusion_permille,
        expanded_uncertainty_units,
        decision_tolerance_units: evidence.uncertainty.decision_tolerance_units,
        claim_ceiling: "SYNTHETIC_SCAN_MEASUREMENT_CONTRACT_ONLY",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn base_evidence() -> ScanEvidence {
        let source_digest = digest('a');
        ScanEvidence {
            evidence_class: EvidenceClass::SyntheticContractOnly,
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            layer_id: "LAYER-03".to_owned(),
            predecessor_pass_id: "PASS-02".to_owned(),
            scan_id: "SCAN-AFTER-PASS-02".to_owned(),
            source_scan_sha256: source_digest.clone(),
            acquisition_config_revision: "CFG-R3".to_owned(),
            acquisition_tick: 100,
            decision_tick: 100,
            maximum_observation_age_ticks: 10,
            calibration: CalibrationEvidence {
                sensor_id: "SYNTH-LASER-01".to_owned(),
                calibration_receipt_sha256: digest('b'),
                reference_artifact_id: "REF-LENGTH-01".to_owned(),
                reference_certificate_id: "CERT-SYNTH-01".to_owned(),
                calibration_config_revision: "CFG-R3".to_owned(),
                acquisition_config_revision: "CFG-R3".to_owned(),
                valid_from_tick: 50,
                expires_at_tick: 150,
                environment_in_calibration_envelope: true,
                reference_length_error_units: 2,
                maximum_reference_length_error_units: 5,
            },
            registration: RegistrationEvidence {
                source_scan_sha256: source_digest,
                registration_receipt_sha256: digest('c'),
                source_frame: "SENSOR-FRAME".to_owned(),
                target_frame: "JOINT-FRAME".to_owned(),
                icp_converged: true,
                rotation_determinant_milli: 1_000,
                orthogonality_error_units: 1,
                maximum_orthogonality_error_units: 3,
                independent_reference_target_count: 4,
                required_reference_target_count: 4,
                reference_target_rms_residual_units: 2,
                maximum_reference_target_rms_residual_units: 5,
            },
            observation: SurfaceObservation {
                coordinate_frame: "JOINT-FRAME".to_owned(),
                coordinate_unit: "mm".to_owned(),
                preparation: SurfacePreparation::DocumentedClean,
                cleaning_receipt_sha256: digest('d'),
                occlusion_assessment_receipt_sha256: digest('e'),
                expected_profile_count: 100,
                received_profile_count: 99,
                dropped_profile_count: 1,
                declared_profile_coverage_permille: 990,
                minimum_profile_coverage_permille: 980,
                maximum_dropped_profile_permille: 20,
                total_point_count: 10_000,
                valid_point_count: 9_950,
                invalid_point_count: 50,
                clipped_point_count: 5,
                minimum_valid_point_count: 9_000,
                maximum_invalid_point_permille: 10,
                occlusion_permille: 20,
                maximum_occlusion_permille: 50,
            },
            uncertainty: UncertaintyBudget {
                components: vec![
                    UncertaintyComponent {
                        component_id: "sensor-repeatability".to_owned(),
                        standard_uncertainty_units: 3,
                    },
                    UncertaintyComponent {
                        component_id: "reference-length".to_owned(),
                        standard_uncertainty_units: 4,
                    },
                    UncertaintyComponent {
                        component_id: "registration".to_owned(),
                        standard_uncertainty_units: 2,
                    },
                ],
                correlation_policy: CorrelationPolicy::IndependenceDeclared,
                coverage_factor_permille: 2_000,
                maximum_expanded_uncertainty_units: 15,
                decision_tolerance_units: 20,
            },
            decision_policy: DecisionPolicy {
                policy_id: "SYNTHETIC-SCAN-POLICY".to_owned(),
                revision: "R1".to_owned(),
                frozen_before_acquisition: true,
                future_information_used: false,
            },
        }
    }

    fn abstains_with(evidence: &ScanEvidence, reason: AbstainReason) {
        let ScanDecision::Abstain(reasons) = evaluate_scan(evidence) else {
            panic!("mutated scan evidence should abstain");
        };
        assert!(
            reasons.contains(&reason),
            "missing reason {reason:?}: {reasons:?}"
        );
    }

    #[test]
    fn valid_synthetic_scan_qualifies_with_a_bound_receipt() {
        let ScanDecision::Qualified(receipt) = evaluate_scan(&base_evidence()) else {
            panic!("valid synthetic scan should qualify");
        };
        assert_eq!(receipt.expanded_uncertainty_units, 12);
        assert_eq!(receipt.profile_coverage_permille, 990);
        assert_eq!(
            receipt.claim_ceiling,
            "SYNTHETIC_SCAN_MEASUREMENT_CONTRACT_ONLY"
        );
    }

    #[test]
    fn malformed_source_digest_abstains() {
        let mut evidence = base_evidence();
        evidence.source_scan_sha256 = "bad".to_owned();
        abstains_with(&evidence, AbstainReason::InvalidSourceDigest);
    }

    #[test]
    fn measured_scan_is_not_admitted_by_synthetic_v0() {
        let mut evidence = base_evidence();
        evidence.evidence_class = EvidenceClass::MeasuredObservation;
        abstains_with(&evidence, AbstainReason::NonSyntheticEvidenceNotAdmitted);
    }

    #[test]
    fn calibration_receipt_must_be_present_and_digest_bound() {
        let mut evidence = base_evidence();
        evidence.calibration.calibration_receipt_sha256.clear();
        abstains_with(&evidence, AbstainReason::InvalidCalibrationReceipt);
    }

    #[test]
    fn stale_calibration_abstains() {
        let mut evidence = base_evidence();
        evidence.acquisition_tick = 151;
        abstains_with(&evidence, AbstainReason::CalibrationExpiredOrNotYetValid);
    }

    #[test]
    fn calibration_configuration_mismatch_abstains() {
        let mut evidence = base_evidence();
        evidence.acquisition_config_revision = "CFG-R4".to_owned();
        abstains_with(&evidence, AbstainReason::CalibrationConfigurationMismatch);
    }

    #[test]
    fn environment_outside_calibration_envelope_abstains() {
        let mut evidence = base_evidence();
        evidence.calibration.environment_in_calibration_envelope = false;
        abstains_with(
            &evidence,
            AbstainReason::EnvironmentOutsideCalibrationEnvelope,
        );
    }

    #[test]
    fn reference_length_error_above_limit_abstains() {
        let mut evidence = base_evidence();
        evidence.calibration.reference_length_error_units = 6;
        abstains_with(&evidence, AbstainReason::ReferenceLengthCheckFailed);
    }

    #[test]
    fn registration_must_bind_exact_scan_bytes() {
        let mut evidence = base_evidence();
        evidence.registration.source_scan_sha256 = digest('f');
        abstains_with(&evidence, AbstainReason::RegistrationSourceDigestMismatch);
    }

    #[test]
    fn icp_convergence_does_not_replace_independent_target_check() {
        let mut evidence = base_evidence();
        evidence.registration.reference_target_rms_residual_units = 6;
        abstains_with(&evidence, AbstainReason::IndependentTargetResidualFailed);
    }

    #[test]
    fn reflection_transform_is_rejected() {
        let mut evidence = base_evidence();
        evidence.registration.rotation_determinant_milli = -1_000;
        abstains_with(&evidence, AbstainReason::InvalidRigidTransform);
    }

    #[test]
    fn independent_reference_target_count_is_required() {
        let mut evidence = base_evidence();
        evidence.registration.independent_reference_target_count = 3;
        abstains_with(&evidence, AbstainReason::IndependentTargetCountMismatch);
    }

    #[test]
    fn registration_frame_must_match_observation_frame() {
        let mut evidence = base_evidence();
        evidence.observation.coordinate_frame = "OTHER-FRAME".to_owned();
        abstains_with(&evidence, AbstainReason::CoordinateFrameMismatch);
    }

    #[test]
    fn profile_coverage_below_minimum_abstains() {
        let mut evidence = base_evidence();
        evidence.observation.received_profile_count = 95;
        evidence.observation.dropped_profile_count = 5;
        evidence.observation.declared_profile_coverage_permille = 950;
        abstains_with(&evidence, AbstainReason::ProfileCoverageBelowMinimum);
    }

    #[test]
    fn false_profile_coverage_claim_abstains() {
        let mut evidence = base_evidence();
        evidence.observation.declared_profile_coverage_permille = 1_000;
        abstains_with(&evidence, AbstainReason::ProfileCoverageMismatch);
    }

    #[test]
    fn excess_dropped_profiles_abstain() {
        let mut evidence = base_evidence();
        evidence.observation.received_profile_count = 95;
        evidence.observation.dropped_profile_count = 5;
        evidence.observation.declared_profile_coverage_permille = 950;
        evidence.observation.maximum_dropped_profile_permille = 20;
        abstains_with(&evidence, AbstainReason::DroppedProfilesAboveLimit);
    }

    #[test]
    fn invalid_point_accounting_abstains() {
        let mut evidence = base_evidence();
        evidence.observation.valid_point_count = 9_900;
        abstains_with(&evidence, AbstainReason::InvalidPointCounts);
    }

    #[test]
    fn clipped_points_must_be_a_subset_of_invalid_points() {
        let mut evidence = base_evidence();
        evidence.observation.clipped_point_count = 51;
        abstains_with(&evidence, AbstainReason::ClippedPointCountInvalid);
    }

    #[test]
    fn excessive_invalid_point_fraction_abstains() {
        let mut evidence = base_evidence();
        evidence.observation.invalid_point_count = 200;
        evidence.observation.valid_point_count = 9_800;
        evidence.observation.clipped_point_count = 20;
        abstains_with(&evidence, AbstainReason::InvalidPointFractionAboveLimit);
    }

    #[test]
    fn excessive_occlusion_abstains() {
        let mut evidence = base_evidence();
        evidence.observation.occlusion_permille = 80;
        abstains_with(&evidence, AbstainReason::OcclusionAboveLimit);
    }

    #[test]
    fn unknown_surface_preparation_abstains() {
        let mut evidence = base_evidence();
        evidence.observation.preparation = SurfacePreparation::Unknown;
        abstains_with(&evidence, AbstainReason::SurfacePreparationNotQualified);
    }

    #[test]
    fn cleaning_and_occlusion_assessment_receipts_are_required() {
        let mut evidence = base_evidence();
        evidence.observation.cleaning_receipt_sha256.clear();
        evidence
            .observation
            .occlusion_assessment_receipt_sha256
            .clear();
        let ScanDecision::Abstain(reasons) = evaluate_scan(&evidence) else {
            panic!("missing receipts should abstain");
        };
        assert!(reasons.contains(&AbstainReason::MissingCleaningReceipt));
        assert!(reasons.contains(&AbstainReason::MissingOcclusionAssessmentReceipt));
    }

    #[test]
    fn stale_observation_abstains() {
        let mut evidence = base_evidence();
        evidence.acquisition_tick = 50;
        evidence.decision_tick = 100;
        evidence.maximum_observation_age_ticks = 10;
        abstains_with(&evidence, AbstainReason::ObservationTooOld);
    }

    #[test]
    fn future_observation_abstains() {
        let mut evidence = base_evidence();
        evidence.acquisition_tick = 101;
        evidence.decision_tick = 100;
        abstains_with(&evidence, AbstainReason::ObservationFromFuture);
    }

    #[test]
    fn future_information_leakage_abstains() {
        let mut evidence = base_evidence();
        evidence.decision_policy.future_information_used = true;
        abstains_with(&evidence, AbstainReason::FutureInformationLeakage);
    }

    #[test]
    fn decision_policy_must_be_frozen_before_acquisition() {
        let mut evidence = base_evidence();
        evidence.decision_policy.frozen_before_acquisition = false;
        abstains_with(&evidence, AbstainReason::DecisionPolicyNotFrozen);
    }

    #[test]
    fn unknown_correlation_is_not_treated_as_independent() {
        let mut evidence = base_evidence();
        evidence.uncertainty.correlation_policy = CorrelationPolicy::CorrelationUnknown;
        abstains_with(&evidence, AbstainReason::CorrelationPolicyNotAdmitted);
    }

    #[test]
    fn duplicate_uncertainty_component_is_rejected() {
        let mut evidence = base_evidence();
        evidence.uncertainty.components[1].component_id =
            evidence.uncertainty.components[0].component_id.clone();
        abstains_with(&evidence, AbstainReason::DuplicateUncertaintyComponent);
    }

    #[test]
    fn expanded_uncertainty_above_decision_limit_abstains() {
        let mut evidence = base_evidence();
        evidence.uncertainty.components[0].standard_uncertainty_units = 8;
        abstains_with(&evidence, AbstainReason::ExpandedUncertaintyAboveLimit);
    }

    #[test]
    fn zero_uncertainty_components_abstains() {
        let mut evidence = base_evidence();
        evidence.uncertainty.components.clear();
        abstains_with(
            &evidence,
            AbstainReason::MissingOrExcessiveUncertaintyComponents,
        );
    }

    #[test]
    fn excessive_uncertainty_component_count_abstains() {
        let mut evidence = base_evidence();
        evidence.uncertainty.components = (0..17)
            .map(|index| UncertaintyComponent {
                component_id: format!("COMP-{index}"),
                standard_uncertainty_units: 1,
            })
            .collect();
        abstains_with(
            &evidence,
            AbstainReason::MissingOrExcessiveUncertaintyComponents,
        );
    }

    #[test]
    fn decision_tolerance_must_be_positive() {
        let mut evidence = base_evidence();
        evidence.uncertainty.decision_tolerance_units = 0;
        abstains_with(&evidence, AbstainReason::InvalidDecisionTolerance);
    }

    #[test]
    fn profile_count_arithmetic_overflow_abstains() {
        let mut evidence = base_evidence();
        evidence.observation.expected_profile_count = u64::MAX;
        evidence.observation.received_profile_count = u64::MAX;
        evidence.observation.dropped_profile_count = 0;
        evidence.observation.declared_profile_coverage_permille = 990;
        let ScanDecision::Abstain(reasons) = evaluate_scan(&evidence) else {
            panic!("profile arithmetic overflow must abstain");
        };
        assert!(reasons.contains(&AbstainReason::NumericOverflow));
    }

    #[test]
    fn determinant_minimum_signed_integer_is_rejected_without_panicking() {
        let mut evidence = base_evidence();
        evidence.registration.rotation_determinant_milli = i64::MIN;
        abstains_with(&evidence, AbstainReason::InvalidRigidTransform);
    }

    #[test]
    fn uncertainty_overflow_abstains_without_wrapping() {
        let mut evidence = base_evidence();
        evidence.uncertainty.components = vec![
            UncertaintyComponent {
                component_id: "HUGE-A".to_owned(),
                standard_uncertainty_units: u64::MAX,
            },
            UncertaintyComponent {
                component_id: "HUGE-B".to_owned(),
                standard_uncertainty_units: u64::MAX,
            },
        ];
        abstains_with(&evidence, AbstainReason::NumericOverflow);
    }
}
