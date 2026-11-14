//! Synthetic, fail-closed reference-artifact drift surveillance.
//!
//! The sentinel monitors a frozen check-standard policy. It never recalibrates
//! an instrument, edits a baseline, or authorizes a physical welding action.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const BENCHMARK_ID: &str = "SHIPYARD-REFERENCE-DRIFT-001";
pub const CLAIM_CEILING: &str = "SYNTHETIC_REFERENCE_ARTIFACT_DRIFT_ONLY";
pub const EWMA_SCALE: i128 = 1_000_000;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    SyntheticContractOnly,
    MeasuredAuthorized,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct DriftPolicy {
    pub policy_id: String,
    pub revision: String,
    pub sha256: String,
    pub frozen_baseline: bool,
    pub instrument_id: String,
    pub calibration_revision: String,
    pub artifact_id: String,
    pub characteristic_id: String,
    pub certified_value_um: i64,
    pub maximum_absolute_error_um: u64,
    pub ewma_lambda_ppm: u32,
    pub initial_ewma_residual_um: i64,
    pub ewma_limit_um: u64,
    pub minimum_observations: usize,
    pub maximum_gap_ms: u64,
    pub minimum_environment_milli_c: i32,
    pub maximum_environment_milli_c: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CalibrationCertificate {
    pub sha256: String,
    pub artifact_id: String,
    pub characteristic_id: String,
    pub instrument_id: String,
    pub calibration_revision: String,
    pub verified: bool,
    pub valid_from_ms: u64,
    pub valid_until_ms: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CheckObservation {
    pub sha256: String,
    pub artifact_id: String,
    pub characteristic_id: String,
    pub instrument_id: String,
    pub calibration_revision: String,
    pub captured_at_ms: u64,
    pub measured_value_um: i64,
    pub expanded_uncertainty_um: Option<u64>,
    pub environment_milli_c: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct DriftSequenceInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub evidence_class: EvidenceClass,
    pub sequence_id: String,
    pub policy: DriftPolicy,
    pub certificate: CalibrationCertificate,
    pub observations: Vec<CheckObservation>,
    pub claim_ceiling: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    ContinueWithinDeclaredCheckLimits,
    HoldForMetrologyReview,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldReason {
    EmptyIdentity,
    WrongSchemaVersion,
    WrongBenchmarkId,
    EvidenceNotAdmitted,
    ClaimCeilingPromotion,
    MalformedDigest,
    PolicyNotFrozen,
    InvalidPolicy,
    PolicyCertificateMismatch,
    CertificateNotVerified,
    CertificateNotValidForSequence,
    TooFewObservations,
    DuplicateObservationDigest,
    NonMonotonicTimestamp,
    ObservationGapTooLarge,
    ObservationIdentityMismatch,
    ObservationUncertaintyMissingOrZero,
    EnvironmentOutsideQualifiedRange,
    NumericOverflow,
    PointwiseGuardExceeded,
    EwmaGuardExceeded,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct DriftReport {
    pub sequence_id: String,
    pub decision: Decision,
    pub hold_reason: Option<HoldReason>,
    pub observation_count: usize,
    pub maximum_pointwise_guard_um: u64,
    pub maximum_ewma_guard_um: u64,
    pub final_ewma_scaled_um_ppm: i64,
    pub ewma_limit_um: u64,
    pub maximum_absolute_error_um: u64,
    pub policy_sha256: String,
    pub certificate_sha256: String,
    pub observation_sha256: Vec<String>,
    pub baseline_mutated: bool,
    pub automatic_recalibration_authorized: bool,
    pub claim_ceiling: String,
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

fn round_div_nearest_away(numerator: i128, denominator: i128) -> Result<i128, HoldReason> {
    if denominator <= 0 {
        return Err(HoldReason::InvalidPolicy);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let round_away = remainder
        .unsigned_abs()
        .checked_mul(2)
        .ok_or(HoldReason::NumericOverflow)?
        >= denominator as u128;
    if !round_away {
        return Ok(quotient);
    }
    if numerator >= 0 {
        quotient.checked_add(1).ok_or(HoldReason::NumericOverflow)
    } else {
        quotient.checked_sub(1).ok_or(HoldReason::NumericOverflow)
    }
}

fn ceil_div_nonnegative(numerator: u128, denominator: u128) -> Result<u128, HoldReason> {
    if denominator == 0 {
        return Err(HoldReason::InvalidPolicy);
    }
    let quotient = numerator / denominator;
    if numerator % denominator == 0 {
        Ok(quotient)
    } else {
        quotient.checked_add(1).ok_or(HoldReason::NumericOverflow)
    }
}

fn checked_abs_i128(value: i128) -> Result<u128, HoldReason> {
    Ok(value.unsigned_abs())
}

fn validate_static(input: &DriftSequenceInput) -> Result<(), HoldReason> {
    let policy = &input.policy;
    let certificate = &input.certificate;
    if [
        input.sequence_id.as_str(),
        policy.policy_id.as_str(),
        policy.revision.as_str(),
        policy.instrument_id.as_str(),
        policy.calibration_revision.as_str(),
        policy.artifact_id.as_str(),
        policy.characteristic_id.as_str(),
    ]
    .into_iter()
    .any(is_empty)
    {
        return Err(HoldReason::EmptyIdentity);
    }
    if input.schema_version != "0.1.0" {
        return Err(HoldReason::WrongSchemaVersion);
    }
    if input.benchmark_id != BENCHMARK_ID {
        return Err(HoldReason::WrongBenchmarkId);
    }
    if input.evidence_class != EvidenceClass::SyntheticContractOnly {
        return Err(HoldReason::EvidenceNotAdmitted);
    }
    if input.claim_ceiling != CLAIM_CEILING {
        return Err(HoldReason::ClaimCeilingPromotion);
    }
    if !policy.frozen_baseline {
        return Err(HoldReason::PolicyNotFrozen);
    }
    if !digest_is_valid(&policy.sha256)
        || !digest_is_valid(&certificate.sha256)
        || input
            .observations
            .iter()
            .any(|observation| !digest_is_valid(&observation.sha256))
    {
        return Err(HoldReason::MalformedDigest);
    }
    if policy.maximum_absolute_error_um == 0
        || policy.ewma_limit_um == 0
        || !(1..=1_000_000).contains(&policy.ewma_lambda_ppm)
        || policy.minimum_observations < 2
        || policy.maximum_gap_ms == 0
        || policy.minimum_environment_milli_c > policy.maximum_environment_milli_c
    {
        return Err(HoldReason::InvalidPolicy);
    }
    if certificate.artifact_id != policy.artifact_id
        || certificate.characteristic_id != policy.characteristic_id
        || certificate.instrument_id != policy.instrument_id
        || certificate.calibration_revision != policy.calibration_revision
    {
        return Err(HoldReason::PolicyCertificateMismatch);
    }
    if !certificate.verified {
        return Err(HoldReason::CertificateNotVerified);
    }
    if input.observations.len() < policy.minimum_observations {
        return Err(HoldReason::TooFewObservations);
    }
    let first = input
        .observations
        .first()
        .ok_or(HoldReason::TooFewObservations)?;
    let last = input
        .observations
        .last()
        .ok_or(HoldReason::TooFewObservations)?;
    if certificate.valid_from_ms > first.captured_at_ms
        || certificate.valid_until_ms < last.captured_at_ms
    {
        return Err(HoldReason::CertificateNotValidForSequence);
    }
    Ok(())
}

pub fn evaluate_drift_sequence(
    input: &DriftSequenceInput,
) -> Result<DriftReport, HoldReason> {
    validate_static(input)?;

    let policy = &input.policy;
    let certificate = &input.certificate;
    let mut seen_digests = HashSet::new();
    let mut previous_timestamp: Option<u64> = None;
    let mut previous_ewma_scaled = i128::from(policy.initial_ewma_residual_um)
        .checked_mul(EWMA_SCALE)
        .ok_or(HoldReason::NumericOverflow)?;
    let mut maximum_pointwise_guard = 0_u64;
    let mut maximum_ewma_guard = 0_u64;
    let mut observation_digests = Vec::with_capacity(input.observations.len());
    let lambda = i128::from(policy.ewma_lambda_ppm);
    let complement = EWMA_SCALE
        .checked_sub(lambda)
        .ok_or(HoldReason::InvalidPolicy)?;

    for observation in &input.observations {
        if !seen_digests.insert(observation.sha256.as_str()) {
            return Err(HoldReason::DuplicateObservationDigest);
        }
        if observation.artifact_id != certificate.artifact_id
            || observation.characteristic_id != certificate.characteristic_id
            || observation.instrument_id != certificate.instrument_id
            || observation.calibration_revision != certificate.calibration_revision
        {
            return Err(HoldReason::ObservationIdentityMismatch);
        }
        let uncertainty = observation
            .expanded_uncertainty_um
            .filter(|value| *value > 0)
            .ok_or(HoldReason::ObservationUncertaintyMissingOrZero)?;
        if observation.environment_milli_c < policy.minimum_environment_milli_c
            || observation.environment_milli_c > policy.maximum_environment_milli_c
        {
            return Err(HoldReason::EnvironmentOutsideQualifiedRange);
        }
        if let Some(previous) = previous_timestamp {
            if observation.captured_at_ms <= previous {
                return Err(HoldReason::NonMonotonicTimestamp);
            }
            if observation.captured_at_ms - previous > policy.maximum_gap_ms {
                return Err(HoldReason::ObservationGapTooLarge);
            }
        }
        previous_timestamp = Some(observation.captured_at_ms);

        let residual = observation
            .measured_value_um
            .checked_sub(policy.certified_value_um)
            .ok_or(HoldReason::NumericOverflow)?;
        let pointwise_guard = residual
            .unsigned_abs()
            .checked_add(uncertainty)
            .ok_or(HoldReason::NumericOverflow)?;
        maximum_pointwise_guard = maximum_pointwise_guard.max(pointwise_guard);
        observation_digests.push(observation.sha256.clone());

        if pointwise_guard > policy.maximum_absolute_error_um {
            return Ok(DriftReport {
                sequence_id: input.sequence_id.clone(),
                decision: Decision::HoldForMetrologyReview,
                hold_reason: Some(HoldReason::PointwiseGuardExceeded),
                observation_count: observation_digests.len(),
                maximum_pointwise_guard_um: maximum_pointwise_guard,
                maximum_ewma_guard_um: maximum_ewma_guard,
                final_ewma_scaled_um_ppm: i64::try_from(previous_ewma_scaled)
                    .map_err(|_| HoldReason::NumericOverflow)?,
                ewma_limit_um: policy.ewma_limit_um,
                maximum_absolute_error_um: policy.maximum_absolute_error_um,
                policy_sha256: policy.sha256.clone(),
                certificate_sha256: certificate.sha256.clone(),
                observation_sha256: observation_digests,
                baseline_mutated: false,
                automatic_recalibration_authorized: false,
                claim_ceiling: CLAIM_CEILING.to_owned(),
            });
        }

        let weighted_current = lambda
            .checked_mul(i128::from(residual))
            .and_then(|value| value.checked_mul(EWMA_SCALE))
            .ok_or(HoldReason::NumericOverflow)?;
        let weighted_prior = complement
            .checked_mul(previous_ewma_scaled)
            .ok_or(HoldReason::NumericOverflow)?;
        let weighted_sum = weighted_current
            .checked_add(weighted_prior)
            .ok_or(HoldReason::NumericOverflow)?;
        let ewma_scaled = round_div_nearest_away(weighted_sum, EWMA_SCALE)?;
        previous_ewma_scaled = ewma_scaled;

        let ewma_magnitude = checked_abs_i128(ewma_scaled)?;
        let ewma_um = ceil_div_nonnegative(ewma_magnitude, EWMA_SCALE as u128)?;
        let ewma_guard = u64::try_from(ewma_um)
            .map_err(|_| HoldReason::NumericOverflow)?
            .checked_add(uncertainty)
            .ok_or(HoldReason::NumericOverflow)?;
        maximum_ewma_guard = maximum_ewma_guard.max(ewma_guard);

        if ewma_guard > policy.ewma_limit_um {
            return Ok(DriftReport {
                sequence_id: input.sequence_id.clone(),
                decision: Decision::HoldForMetrologyReview,
                hold_reason: Some(HoldReason::EwmaGuardExceeded),
                observation_count: observation_digests.len(),
                maximum_pointwise_guard_um: maximum_pointwise_guard,
                maximum_ewma_guard_um: maximum_ewma_guard,
                final_ewma_scaled_um_ppm: i64::try_from(previous_ewma_scaled)
                    .map_err(|_| HoldReason::NumericOverflow)?,
                ewma_limit_um: policy.ewma_limit_um,
                maximum_absolute_error_um: policy.maximum_absolute_error_um,
                policy_sha256: policy.sha256.clone(),
                certificate_sha256: certificate.sha256.clone(),
                observation_sha256: observation_digests,
                baseline_mutated: false,
                automatic_recalibration_authorized: false,
                claim_ceiling: CLAIM_CEILING.to_owned(),
            });
        }
    }

    Ok(DriftReport {
        sequence_id: input.sequence_id.clone(),
        decision: Decision::ContinueWithinDeclaredCheckLimits,
        hold_reason: None,
        observation_count: input.observations.len(),
        maximum_pointwise_guard_um: maximum_pointwise_guard,
        maximum_ewma_guard_um: maximum_ewma_guard,
        final_ewma_scaled_um_ppm: i64::try_from(previous_ewma_scaled)
            .map_err(|_| HoldReason::NumericOverflow)?,
        ewma_limit_um: policy.ewma_limit_um,
        maximum_absolute_error_um: policy.maximum_absolute_error_um,
        policy_sha256: policy.sha256.clone(),
        certificate_sha256: certificate.sha256.clone(),
        observation_sha256: observation_digests,
        baseline_mutated: false,
        automatic_recalibration_authorized: false,
        claim_ceiling: CLAIM_CEILING.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn base_input(residuals: &[i64]) -> DriftSequenceInput {
        let observations = residuals
            .iter()
            .enumerate()
            .map(|(index, residual)| CheckObservation {
                sha256: digest(char::from(b'a' + index as u8)),
                artifact_id: "ARTIFACT-SYNTH-01".to_owned(),
                characteristic_id: "LENGTH-X-100MM".to_owned(),
                instrument_id: "SCANNER-SYNTH-01".to_owned(),
                calibration_revision: "CAL-R1".to_owned(),
                captured_at_ms: 1_000 + index as u64 * 1_000,
                measured_value_um: 100_000 + residual,
                expanded_uncertainty_um: Some(10),
                environment_milli_c: 20_000,
            })
            .collect();
        DriftSequenceInput {
            schema_version: "0.1.0".to_owned(),
            benchmark_id: BENCHMARK_ID.to_owned(),
            evidence_class: EvidenceClass::SyntheticContractOnly,
            sequence_id: "SEQ-SYNTH-01".to_owned(),
            policy: DriftPolicy {
                policy_id: "POLICY-SYNTH-01".to_owned(),
                revision: "R1".to_owned(),
                sha256: digest('1'),
                frozen_baseline: true,
                instrument_id: "SCANNER-SYNTH-01".to_owned(),
                calibration_revision: "CAL-R1".to_owned(),
                artifact_id: "ARTIFACT-SYNTH-01".to_owned(),
                characteristic_id: "LENGTH-X-100MM".to_owned(),
                certified_value_um: 100_000,
                maximum_absolute_error_um: 100,
                ewma_lambda_ppm: 500_000,
                initial_ewma_residual_um: 0,
                ewma_limit_um: 45,
                minimum_observations: 3,
                maximum_gap_ms: 2_000,
                minimum_environment_milli_c: 15_000,
                maximum_environment_milli_c: 30_000,
            },
            certificate: CalibrationCertificate {
                sha256: digest('2'),
                artifact_id: "ARTIFACT-SYNTH-01".to_owned(),
                characteristic_id: "LENGTH-X-100MM".to_owned(),
                instrument_id: "SCANNER-SYNTH-01".to_owned(),
                calibration_revision: "CAL-R1".to_owned(),
                verified: true,
                valid_from_ms: 0,
                valid_until_ms: 10_000,
            },
            observations,
            claim_ceiling: CLAIM_CEILING.to_owned(),
        }
    }

    #[test]
    fn stable_sequence_continues() {
        let input = base_input(&[2, -1, 3, 0]);
        let report = evaluate_drift_sequence(&input).expect("stable sequence should evaluate");
        assert_eq!(report.decision, Decision::ContinueWithinDeclaredCheckLimits);
        assert_eq!(report.maximum_pointwise_guard_um, 13);
        assert!(!report.baseline_mutated);
        assert!(!report.automatic_recalibration_authorized);
    }

    #[test]
    fn ewma_detects_small_persistent_drift_inside_each_pointwise_limit() {
        let input = base_input(&[20, 20, 30, 40, 50, 60]);
        let report = evaluate_drift_sequence(&input).expect("sequence should evaluate");
        assert_eq!(report.decision, Decision::HoldForMetrologyReview);
        assert_eq!(report.hold_reason, Some(HoldReason::EwmaGuardExceeded));
        assert!(report.maximum_pointwise_guard_um < input.policy.maximum_absolute_error_um);
        assert!(report.maximum_ewma_guard_um > input.policy.ewma_limit_um);
    }

    #[test]
    fn abrupt_pointwise_excursion_holds_immediately() {
        let input = base_input(&[0, 0, 150]);
        let report = evaluate_drift_sequence(&input).expect("sequence should evaluate");
        assert_eq!(report.hold_reason, Some(HoldReason::PointwiseGuardExceeded));
        assert_eq!(report.observation_count, 3);
    }

    #[test]
    fn negative_persistent_drift_is_detected() {
        let input = base_input(&[-20, -20, -30, -40, -50, -60]);
        let report = evaluate_drift_sequence(&input).expect("sequence should evaluate");
        assert_eq!(report.hold_reason, Some(HoldReason::EwmaGuardExceeded));
    }

    #[test]
    fn uncertainty_consuming_guard_band_holds() {
        let mut input = base_input(&[80, 80, 80]);
        input.observations[0].expanded_uncertainty_um = Some(25);
        let report = evaluate_drift_sequence(&input).expect("sequence should evaluate");
        assert_eq!(report.hold_reason, Some(HoldReason::PointwiseGuardExceeded));
    }

    #[test]
    fn wrong_artifact_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.observations[1].artifact_id = "OTHER-ARTIFACT".to_owned();
        assert_eq!(
            evaluate_drift_sequence(&input),
            Err(HoldReason::ObservationIdentityMismatch)
        );
    }

    #[test]
    fn calibration_revision_mismatch_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.observations[1].calibration_revision = "CAL-R2".to_owned();
        assert_eq!(
            evaluate_drift_sequence(&input),
            Err(HoldReason::ObservationIdentityMismatch)
        );
    }

    #[test]
    fn expired_certificate_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.certificate.valid_until_ms = 1_500;
        assert_eq!(
            evaluate_drift_sequence(&input),
            Err(HoldReason::CertificateNotValidForSequence)
        );
    }

    #[test]
    fn unfrozen_baseline_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.policy.frozen_baseline = false;
        assert_eq!(evaluate_drift_sequence(&input), Err(HoldReason::PolicyNotFrozen));
    }

    #[test]
    fn invalid_ewma_lambda_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.policy.ewma_lambda_ppm = 0;
        assert_eq!(evaluate_drift_sequence(&input), Err(HoldReason::InvalidPolicy));
    }

    #[test]
    fn too_few_observations_are_rejected() {
        let mut input = base_input(&[0, 0]);
        assert_eq!(evaluate_drift_sequence(&input), Err(HoldReason::TooFewObservations));
    }

    #[test]
    fn missing_uncertainty_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.observations[1].expanded_uncertainty_um = None;
        assert_eq!(
            evaluate_drift_sequence(&input),
            Err(HoldReason::ObservationUncertaintyMissingOrZero)
        );
    }

    #[test]
    fn zero_uncertainty_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.observations[1].expanded_uncertainty_um = Some(0);
        assert_eq!(
            evaluate_drift_sequence(&input),
            Err(HoldReason::ObservationUncertaintyMissingOrZero)
        );
    }

    #[test]
    fn nonmonotonic_time_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.observations[2].captured_at_ms = input.observations[1].captured_at_ms;
        assert_eq!(
            evaluate_drift_sequence(&input),
            Err(HoldReason::NonMonotonicTimestamp)
        );
    }

    #[test]
    fn excessive_gap_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.observations[2].captured_at_ms = 5_000;
        assert_eq!(
            evaluate_drift_sequence(&input),
            Err(HoldReason::ObservationGapTooLarge)
        );
    }

    #[test]
    fn out_of_range_environment_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.observations[1].environment_milli_c = 40_000;
        assert_eq!(
            evaluate_drift_sequence(&input),
            Err(HoldReason::EnvironmentOutsideQualifiedRange)
        );
    }

    #[test]
    fn duplicate_observation_digest_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.observations[1].sha256 = input.observations[0].sha256.clone();
        assert_eq!(
            evaluate_drift_sequence(&input),
            Err(HoldReason::DuplicateObservationDigest)
        );
    }

    #[test]
    fn malformed_policy_digest_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.policy.sha256 = "invalid".to_owned();
        assert_eq!(evaluate_drift_sequence(&input), Err(HoldReason::MalformedDigest));
    }

    #[test]
    fn baseline_and_auto_calibration_are_never_mutated() {
        let input = base_input(&[10, 10, 10]);
        let report = evaluate_drift_sequence(&input).expect("stable sequence should evaluate");
        assert!(!report.baseline_mutated);
        assert!(!report.automatic_recalibration_authorized);
        assert_eq!(input.policy.initial_ewma_residual_um, 0);
    }

    #[test]
    fn fixed_point_ewma_overflow_holds_instead_of_wrapping() {
        let mut input = base_input(&[0, 0, 0]);
        input.observations[0].measured_value_um = i64::MAX;
        input.policy.certified_value_um = i64::MIN;
        input.policy.maximum_absolute_error_um = u64::MAX;
        input.policy.ewma_limit_um = u64::MAX;
        assert_eq!(evaluate_drift_sequence(&input), Err(HoldReason::NumericOverflow));
    }

    #[test]
    fn calibration_not_verified_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.certificate.verified = false;
        assert_eq!(evaluate_drift_sequence(&input), Err(HoldReason::CertificateNotVerified));
    }

    #[test]
    fn invalid_claim_ceiling_is_rejected() {
        let mut input = base_input(&[0, 0, 0]);
        input.claim_ceiling = "ISO_CONFORMANT".to_owned();
        assert_eq!(evaluate_drift_sequence(&input), Err(HoldReason::ClaimCeilingPromotion));
    }
}
