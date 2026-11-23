//! Time-of-validity gate for delayed and future-epoch navigation results.
//!
//! A spatial estimate is not a complete navigation claim without the epoch at
//! which it is intended to be valid. This module keeps physical measurement
//! time, completed-payload receipt time, processing time, decision cutoff and
//! target/valid time separate. It uses closed integer-nanosecond intervals and
//! refuses to infer ordering from interval midpoints.
//!
//! This is a temporal-applicability gate only. It does not calculate spatial
//! error, predictor error, covariance, protection levels, physical clock
//! truth, PHMI or navigation authority. Time digests are caller-supplied
//! commitments. Streaming/incremental payloads and durable clock correlation
//! are outside this first batch-receipt contract.

use std::fmt;

const SHA256_BYTES: usize = 32;

/// Closed interval on one normalized, explicitly identified time basis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationTimeInterval {
    pub lower_ns: i64,
    pub upper_ns: i64,
    pub time_basis_sha256: [u8; SHA256_BYTES],
}

/// Meaning of the result's target-valid epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationTimeClaimKind {
    /// A state estimate at the historical physical measurement epoch.
    /// It is usable for replay, not as the current control state.
    EventTimeEstimate,
    /// A state claim intended to describe the decision epoch.
    DecisionEpochNowcast,
    /// A forecast intended to describe a strictly future target epoch.
    FutureEpochPrediction,
}

/// Frozen applicability policy for one deployment/profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationTimeValidityPolicy {
    pub policy_id: u64,
    pub policy_sha256: [u8; SHA256_BYTES],
    pub expected_time_basis_sha256: [u8; SHA256_BYTES],
    pub maximum_observation_age_ns: u64,
    pub maximum_acquisition_to_receipt_delay_ns: u64,
    pub maximum_receipt_to_processing_complete_ns: u64,
    pub maximum_prediction_horizon_ns: u64,
}

/// Controller-visible timing and provenance for one navigation result.
///
/// Receipt time is when the complete payload becomes available to the
/// estimator. This first contract assumes batch payload completion; streaming
/// partial-payload semantics require a separate event-level contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationTimeValidityInput {
    pub result_id: u64,
    pub episode_id: u64,
    pub claim_kind: NavigationTimeClaimKind,
    pub measurement_event_time: NavigationTimeInterval,
    pub payload_receipt_time: NavigationTimeInterval,
    pub processing_start_time: NavigationTimeInterval,
    pub processing_completion_time: NavigationTimeInterval,
    pub decision_cutoff_time: NavigationTimeInterval,
    pub target_valid_time: NavigationTimeInterval,
    pub clock_correlation_sha256: [u8; SHA256_BYTES],
    pub source_manifest_sha256: [u8; SHA256_BYTES],
    pub estimator_or_predictor_revision_sha256: [u8; SHA256_BYTES],
    pub model_domain_support_sha256: [u8; SHA256_BYTES],
}

/// Applicability result. None of these statuses grants navigation authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationTimeDisposition {
    /// A valid historical event-time result. Not a current/future position.
    HistoricalEventTimeOnly,
    /// The target interval covers the decision-cutoff interval.
    DecisionEpochNowcastApplicable,
    /// The target interval is provably in the future and within horizon.
    FutureEpochPredictionApplicable,
    /// The observation is too old at the decision cutoff.
    StaleObservation,
    /// The completed payload arrived too long after its physical event.
    AcquisitionDelayExceeded,
    /// Receipt-to-processing-completion latency exceeds the policy.
    ProcessingDelayExceeded,
    /// The processing result became available after the decision cutoff.
    FutureDataLeakage,
    /// Required chronological order cannot be established from the intervals.
    IndeterminateChronology,
    /// A future prediction's target interval is partly or wholly non-future.
    IndeterminateTargetEpoch,
    /// A historical event-time estimate was assigned another target epoch.
    TargetEpochMismatch,
    /// The target epoch is provably before the decision cutoff.
    TargetEpochBeforeDecision,
    FutureTargetForNowcast,
    /// The future prediction horizon exceeds the pinned policy.
    PredictionHorizonExceeded,
}

/// Deterministic time-validity report; metrics are populated only after the
/// required event/receipt/decision chronology is established.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationTimeValidityReport {
    pub result_id: u64,
    pub episode_id: u64,
    pub policy_id: u64,
    pub policy_sha256: [u8; SHA256_BYTES],
    pub time_basis_sha256: [u8; SHA256_BYTES],
    pub clock_correlation_sha256: [u8; SHA256_BYTES],
    pub source_manifest_sha256: [u8; SHA256_BYTES],
    pub estimator_or_predictor_revision_sha256: [u8; SHA256_BYTES],
    pub model_domain_support_sha256: [u8; SHA256_BYTES],
    pub disposition: NavigationTimeDisposition,
    pub observation_age_upper_bound_ns: Option<u64>,
    pub acquisition_to_receipt_upper_bound_ns: Option<u64>,
    pub receipt_to_processing_complete_upper_bound_ns: Option<u64>,
    pub processing_runtime_upper_bound_ns: Option<u64>,
    pub prediction_horizon_upper_bound_ns: Option<u64>,
    /// Always false. Time applicability is not integrity qualification.
    pub integrity_qualified: bool,
    /// Always false. This module has no authority-transition capability.
    pub navigation_authorized: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationTimeValidityError {
    InvalidPolicy,
    InvalidResultIdentity,
    InvalidTimeInterval,
    MissingEvidenceCommitment,
    UnsupportedTimeBasis,
    InvalidTimeChronology,
    TimeArithmeticOverflow,
}

impl fmt::Display for NavigationTimeValidityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy => write!(
                f,
                "time-validity policy needs nonzero identities/commitments and a finite duration contract"
            ),
            Self::InvalidResultIdentity => {
                write!(f, "navigation result and episode identities must be nonzero")
            }
            Self::InvalidTimeInterval => {
                write!(f, "time intervals need finite ordered integer bounds and nonzero time-basis identities")
            }
            Self::MissingEvidenceCommitment => {
                write!(f, "clock, source, estimator/predictor and domain commitments must be nonzero")
            }
            Self::UnsupportedTimeBasis => {
                write!(f, "all result time intervals must match the policy's normalized time basis")
            }
            Self::InvalidTimeChronology => {
                write!(f, "time intervals establish a contradictory event/receipt/processing chronology")
            }
            Self::TimeArithmeticOverflow => {
                write!(f, "time interval arithmetic exceeded the supported duration range")
            }
        }
    }
}

impl std::error::Error for NavigationTimeValidityError {}

impl NavigationTimeValidityPolicy {
    pub fn validate(self) -> Result<(), NavigationTimeValidityError> {
        if self.policy_id == 0
            || is_zero_digest(&self.policy_sha256)
            || is_zero_digest(&self.expected_time_basis_sha256)
        {
            return Err(NavigationTimeValidityError::InvalidPolicy);
        }
        Ok(())
    }
}

impl NavigationTimeInterval {
    fn validate(self) -> Result<(), NavigationTimeValidityError> {
        if self.lower_ns > self.upper_ns || is_zero_digest(&self.time_basis_sha256) {
            return Err(NavigationTimeValidityError::InvalidTimeInterval);
        }
        Ok(())
    }

    fn matches_basis(self, basis: &[u8; SHA256_BYTES]) -> bool {
        self.time_basis_sha256 == *basis
    }
}

enum ChronologyCheck {
    Ordered,
    DefinitelyReversed,
    Indeterminate,
}

fn prove_non_decreasing(
    earlier: NavigationTimeInterval,
    later: NavigationTimeInterval,
) -> ChronologyCheck {
    if earlier.upper_ns <= later.lower_ns {
        ChronologyCheck::Ordered
    } else if earlier.lower_ns > later.upper_ns {
        ChronologyCheck::DefinitelyReversed
    } else {
        ChronologyCheck::Indeterminate
    }
}

fn upper_difference(
    later_upper_ns: i64,
    earlier_lower_ns: i64,
) -> Result<u64, NavigationTimeValidityError> {
    let difference = i128::from(later_upper_ns) - i128::from(earlier_lower_ns);
    if difference < 0 {
        return Err(NavigationTimeValidityError::InvalidTimeChronology);
    }
    u64::try_from(difference).map_err(|_| NavigationTimeValidityError::TimeArithmeticOverflow)
}

/// Evaluate time applicability of one state result against a frozen decision
/// cutoff and target epoch.
///
/// Closed interval ordering is deliberately conservative:
/// - if A.upper <= B.lower, A is provably no later than B;
/// - if A.lower > B.upper, chronology is definitely reversed;
/// - otherwise the intervals overlap and order is INDETERMINATE.
///
/// The function does not compare midpoints or assume independent clock errors.
pub fn evaluate_navigation_time_validity(
    policy: NavigationTimeValidityPolicy,
    input: NavigationTimeValidityInput,
) -> Result<NavigationTimeValidityReport, NavigationTimeValidityError> {
    policy.validate()?;
    if input.result_id == 0 || input.episode_id == 0 {
        return Err(NavigationTimeValidityError::InvalidResultIdentity);
    }

    let intervals = [
        input.measurement_event_time,
        input.payload_receipt_time,
        input.processing_start_time,
        input.processing_completion_time,
        input.decision_cutoff_time,
        input.target_valid_time,
    ];
    for interval in intervals {
        interval.validate()?;
        if !interval.matches_basis(&policy.expected_time_basis_sha256) {
            return Err(NavigationTimeValidityError::UnsupportedTimeBasis);
        }
    }
    if is_zero_digest(&input.clock_correlation_sha256)
        || is_zero_digest(&input.source_manifest_sha256)
        || is_zero_digest(&input.estimator_or_predictor_revision_sha256)
        || is_zero_digest(&input.model_domain_support_sha256)
    {
        return Err(NavigationTimeValidityError::MissingEvidenceCommitment);
    }

    let report = |disposition,
                  observation_age_upper_bound_ns,
                  acquisition_to_receipt_upper_bound_ns,
                  receipt_to_processing_complete_upper_bound_ns,
                  processing_runtime_upper_bound_ns,
                  prediction_horizon_upper_bound_ns| {
        NavigationTimeValidityReport {
            result_id: input.result_id,
            episode_id: input.episode_id,
            policy_id: policy.policy_id,
            policy_sha256: policy.policy_sha256,
            time_basis_sha256: policy.expected_time_basis_sha256,
            clock_correlation_sha256: input.clock_correlation_sha256,
            source_manifest_sha256: input.source_manifest_sha256,
            estimator_or_predictor_revision_sha256: input.estimator_or_predictor_revision_sha256,
            model_domain_support_sha256: input.model_domain_support_sha256,
            disposition,
            observation_age_upper_bound_ns,
            acquisition_to_receipt_upper_bound_ns,
            receipt_to_processing_complete_upper_bound_ns,
            processing_runtime_upper_bound_ns,
            prediction_horizon_upper_bound_ns,
            integrity_qualified: false,
            navigation_authorized: false,
        }
    };

    match prove_non_decreasing(
        input.measurement_event_time,
        input.payload_receipt_time,
    ) {
        ChronologyCheck::Ordered => {}
        ChronologyCheck::DefinitelyReversed => {
            return Ok(report(
                NavigationTimeDisposition::FutureDataLeakage,
                None, None, None, None, None,
            ));
        }
        ChronologyCheck::Indeterminate => {
            return Ok(report(
                NavigationTimeDisposition::IndeterminateChronology,
                None, None, None, None, None,
            ));
        }
    }

    match prove_non_decreasing(
        input.payload_receipt_time,
        input.processing_start_time,
    ) {
        ChronologyCheck::Ordered => {}
        ChronologyCheck::DefinitelyReversed => {
            return Err(NavigationTimeValidityError::InvalidTimeChronology);
        }
        ChronologyCheck::Indeterminate => {
            return Ok(report(
                NavigationTimeDisposition::IndeterminateChronology,
                None, None, None, None, None,
            ));
        }
    }

    match prove_non_decreasing(
        input.processing_start_time,
        input.processing_completion_time,
    ) {
        ChronologyCheck::Ordered => {}
        ChronologyCheck::DefinitelyReversed => {
            return Err(NavigationTimeValidityError::InvalidTimeChronology);
        }
        ChronologyCheck::Indeterminate => {
            return Ok(report(
                NavigationTimeDisposition::IndeterminateChronology,
                None, None, None, None, None,
            ));
        }
    }

    match prove_non_decreasing(
        input.processing_completion_time,
        input.decision_cutoff_time,
    ) {
        ChronologyCheck::Ordered => {}
        ChronologyCheck::DefinitelyReversed => {
            return Ok(report(
                NavigationTimeDisposition::FutureDataLeakage,
                None, None, None, None, None,
            ));
        }
        ChronologyCheck::Indeterminate => {
            return Ok(report(
                NavigationTimeDisposition::IndeterminateChronology,
                None, None, None, None, None,
            ));
        }
    }

    let acquisition_to_receipt = upper_difference(
        input.payload_receipt_time.upper_ns,
        input.measurement_event_time.lower_ns,
    )?;
    let receipt_to_processing_complete = upper_difference(
        input.processing_completion_time.upper_ns,
        input.payload_receipt_time.lower_ns,
    )?;
    let processing_runtime = upper_difference(
        input.processing_completion_time.upper_ns,
        input.processing_start_time.lower_ns,
    )?;
    let observation_age = upper_difference(
        input.decision_cutoff_time.upper_ns,
        input.measurement_event_time.lower_ns,
    )?;

    let metrics = |disposition, prediction_horizon| {
        report(
            disposition,
            Some(observation_age),
            Some(acquisition_to_receipt),
            Some(receipt_to_processing_complete),
            Some(processing_runtime),
            prediction_horizon,
        )
    };

    if observation_age > policy.maximum_observation_age_ns {
        return Ok(metrics(NavigationTimeDisposition::StaleObservation, None));
    }
    if acquisition_to_receipt > policy.maximum_acquisition_to_receipt_delay_ns {
        return Ok(metrics(
            NavigationTimeDisposition::AcquisitionDelayExceeded,
            None,
        ));
    }
    if receipt_to_processing_complete
        > policy.maximum_receipt_to_processing_complete_ns
    {
        return Ok(metrics(
            NavigationTimeDisposition::ProcessingDelayExceeded,
            None,
        ));
    }

    match input.claim_kind {
        NavigationTimeClaimKind::EventTimeEstimate => {
            if input.target_valid_time != input.measurement_event_time {
                return Ok(metrics(
                    NavigationTimeDisposition::TargetEpochMismatch,
                    None,
                ));
            }
            Ok(metrics(
                NavigationTimeDisposition::HistoricalEventTimeOnly,
                None,
            ))
        }
        NavigationTimeClaimKind::DecisionEpochNowcast => {
            let decision = input.decision_cutoff_time;
            let target = input.target_valid_time;
            if target.upper_ns < decision.lower_ns {
                return Ok(metrics(
                    NavigationTimeDisposition::TargetEpochBeforeDecision,
                    None,
                ));
            }
            if target.lower_ns > decision.upper_ns {
                return Ok(metrics(
                    NavigationTimeDisposition::FutureTargetForNowcast,
                    None,
                ));
            }
            if target.lower_ns > decision.lower_ns
                || target.upper_ns < decision.upper_ns
            {
                return Ok(metrics(
                    NavigationTimeDisposition::IndeterminateTargetEpoch,
                    None,
                ));
            }
            Ok(metrics(
                NavigationTimeDisposition::DecisionEpochNowcastApplicable,
                Some(0),
            ))
        }
        NavigationTimeClaimKind::FutureEpochPrediction => {
            let decision = input.decision_cutoff_time;
            let target = input.target_valid_time;
            if target.upper_ns < decision.lower_ns {
                return Ok(metrics(
                    NavigationTimeDisposition::TargetEpochBeforeDecision,
                    None,
                ));
            }
            if target.lower_ns <= decision.upper_ns {
                return Ok(metrics(
                    NavigationTimeDisposition::IndeterminateTargetEpoch,
                    None,
                ));
            }
            let horizon = upper_difference(target.upper_ns, decision.lower_ns)?;
            if horizon > policy.maximum_prediction_horizon_ns {
                return Ok(metrics(
                    NavigationTimeDisposition::PredictionHorizonExceeded,
                    Some(horizon),
                ));
            }
            Ok(metrics(
                NavigationTimeDisposition::FutureEpochPredictionApplicable,
                Some(horizon),
            ))
        }
    }
}

fn is_zero_digest(digest: &[u8; SHA256_BYTES]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(lower_ns: i64, upper_ns: i64) -> NavigationTimeInterval {
        NavigationTimeInterval {
            lower_ns,
            upper_ns,
            time_basis_sha256: [0x11; SHA256_BYTES],
        }
    }

    fn policy() -> NavigationTimeValidityPolicy {
        NavigationTimeValidityPolicy {
            policy_id: 3,
            policy_sha256: [0x33; SHA256_BYTES],
            expected_time_basis_sha256: [0x11; SHA256_BYTES],
            maximum_observation_age_ns: 100,
            maximum_acquisition_to_receipt_delay_ns: 30,
            maximum_receipt_to_processing_complete_ns: 20,
            maximum_prediction_horizon_ns: 50,
        }
    }

    fn input(kind: NavigationTimeClaimKind) -> NavigationTimeValidityInput {
        NavigationTimeValidityInput {
            result_id: 7,
            episode_id: 9,
            claim_kind: kind,
            measurement_event_time: interval(10, 10),
            payload_receipt_time: interval(20, 20),
            processing_start_time: interval(21, 21),
            processing_completion_time: interval(25, 25),
            decision_cutoff_time: interval(30, 30),
            target_valid_time: interval(30, 30),
            clock_correlation_sha256: [0x44; SHA256_BYTES],
            source_manifest_sha256: [0x55; SHA256_BYTES],
            estimator_or_predictor_revision_sha256: [0x66; SHA256_BYTES],
            model_domain_support_sha256: [0x77; SHA256_BYTES],
        }
    }

    #[test]
    fn valid_nowcast_is_bound_to_the_decision_epoch() {
        let result = evaluate_navigation_time_validity(
            policy(),
            input(NavigationTimeClaimKind::DecisionEpochNowcast),
        )
        .unwrap();
        assert_eq!(
            result.disposition,
            NavigationTimeDisposition::DecisionEpochNowcastApplicable
        );
        assert_eq!(result.observation_age_upper_bound_ns, Some(20));
        assert_eq!(result.acquisition_to_receipt_upper_bound_ns, Some(10));
        assert_eq!(result.receipt_to_processing_complete_upper_bound_ns, Some(5));
        assert_eq!(result.processing_runtime_upper_bound_ns, Some(4));
        assert_eq!(result.prediction_horizon_upper_bound_ns, Some(0));
        assert!(!result.integrity_qualified);
        assert!(!result.navigation_authorized);
    }

    #[test]
    fn valid_future_prediction_has_an_explicit_prediction_horizon() {
        let mut candidate = input(NavigationTimeClaimKind::FutureEpochPrediction);
        candidate.target_valid_time = interval(40, 45);
        let result = evaluate_navigation_time_validity(policy(), candidate).unwrap();
        assert_eq!(
            result.disposition,
            NavigationTimeDisposition::FutureEpochPredictionApplicable
        );
        assert_eq!(result.prediction_horizon_upper_bound_ns, Some(15));
    }

    #[test]
    fn event_time_estimate_is_historical_not_a_current_position() {
        let mut candidate = input(NavigationTimeClaimKind::EventTimeEstimate);
        candidate.target_valid_time = candidate.measurement_event_time;
        let result = evaluate_navigation_time_validity(policy(), candidate).unwrap();
        assert_eq!(
            result.disposition,
            NavigationTimeDisposition::HistoricalEventTimeOnly
        );
        assert!(!result.navigation_authorized);
    }

    #[test]
    fn stale_observation_is_not_rescued_by_recent_payload_receipt() {
        let mut candidate = input(NavigationTimeClaimKind::DecisionEpochNowcast);
        candidate.measurement_event_time = interval(0, 0);
        candidate.payload_receipt_time = interval(25, 25);
        candidate.processing_start_time = interval(26, 26);
        candidate.processing_completion_time = interval(27, 27);
        candidate.decision_cutoff_time = interval(150, 150);
        candidate.target_valid_time = interval(150, 150);
        let result = evaluate_navigation_time_validity(policy(), candidate).unwrap();
        assert_eq!(result.disposition, NavigationTimeDisposition::StaleObservation);
        assert_eq!(result.observation_age_upper_bound_ns, Some(150));
    }

    #[test]
    fn future_receipt_or_processing_completion_is_rejected_as_data_leakage() {
        let mut future_receipt = input(NavigationTimeClaimKind::DecisionEpochNowcast);
        future_receipt.payload_receipt_time = interval(31, 31);
        future_receipt.processing_start_time = interval(32, 32);
        future_receipt.processing_completion_time = interval(33, 33);
        future_receipt.decision_cutoff_time = interval(30, 30);
        future_receipt.target_valid_time = interval(30, 30);
        let result = evaluate_navigation_time_validity(policy(), future_receipt).unwrap();
        assert_eq!(result.disposition, NavigationTimeDisposition::FutureDataLeakage);

        let mut late_completion = input(NavigationTimeClaimKind::DecisionEpochNowcast);
        late_completion.processing_completion_time = interval(31, 31);
        let result = evaluate_navigation_time_validity(policy(), late_completion).unwrap();
        assert_eq!(result.disposition, NavigationTimeDisposition::FutureDataLeakage);
    }

    #[test]
    fn overlapping_event_and_receipt_intervals_are_indeterminate() {
        let mut candidate = input(NavigationTimeClaimKind::DecisionEpochNowcast);
        candidate.measurement_event_time = interval(10, 22);
        candidate.payload_receipt_time = interval(20, 24);
        let result = evaluate_navigation_time_validity(policy(), candidate).unwrap();
        assert_eq!(
            result.disposition,
            NavigationTimeDisposition::IndeterminateChronology
        );
        assert_eq!(result.observation_age_upper_bound_ns, None);
    }

    #[test]
    fn overlapping_receipt_and_decision_intervals_are_indeterminate() {
        let mut candidate = input(NavigationTimeClaimKind::DecisionEpochNowcast);
        candidate.decision_cutoff_time = interval(19, 21);
        candidate.target_valid_time = interval(19, 21);
        let result = evaluate_navigation_time_validity(policy(), candidate).unwrap();
        assert_eq!(
            result.disposition,
            NavigationTimeDisposition::IndeterminateChronology
        );
    }

    #[test]
    fn interval_boundaries_are_inclusive_and_use_worst_case_differences() {
        let mut cfg = policy();
        cfg.maximum_observation_age_ns = 20;
        cfg.maximum_acquisition_to_receipt_delay_ns = 10;
        cfg.maximum_receipt_to_processing_complete_ns = 5;
        let result = evaluate_navigation_time_validity(
            cfg,
            input(NavigationTimeClaimKind::DecisionEpochNowcast),
        )
        .unwrap();
        assert_eq!(
            result.disposition,
            NavigationTimeDisposition::DecisionEpochNowcastApplicable
        );
    }

    #[test]
    fn acquisition_delay_and_processing_delay_have_separate_failures() {
        let mut slow_acquisition = input(NavigationTimeClaimKind::DecisionEpochNowcast);
        slow_acquisition.payload_receipt_time = interval(41, 41);
        slow_acquisition.processing_start_time = interval(42, 42);
        slow_acquisition.processing_completion_time = interval(43, 43);
        slow_acquisition.decision_cutoff_time = interval(50, 50);
        slow_acquisition.target_valid_time = interval(50, 50);
        let result = evaluate_navigation_time_validity(policy(), slow_acquisition).unwrap();
        assert_eq!(
            result.disposition,
            NavigationTimeDisposition::AcquisitionDelayExceeded
        );

        let mut slow_processing = input(NavigationTimeClaimKind::DecisionEpochNowcast);
        slow_processing.processing_completion_time = interval(45, 45);
        let result = evaluate_navigation_time_validity(policy(), slow_processing).unwrap();
        assert_eq!(
            result.disposition,
            NavigationTimeDisposition::ProcessingDelayExceeded
        );
    }

    #[test]
    fn future_prediction_must_be_separated_from_decision_interval() {
        let mut candidate = input(NavigationTimeClaimKind::FutureEpochPrediction);
        candidate.decision_cutoff_time = interval(30, 35);
        candidate.target_valid_time = interval(35, 40);
        let result = evaluate_navigation_time_validity(policy(), candidate).unwrap();
        assert_eq!(
            result.disposition,
            NavigationTimeDisposition::IndeterminateTargetEpoch
        );
    }

    #[test]
    fn prediction_horizon_uses_the_farthest_target_endpoint() {
        let mut candidate = input(NavigationTimeClaimKind::FutureEpochPrediction);
        candidate.target_valid_time = interval(40, 100);
        let result = evaluate_navigation_time_validity(policy(), candidate).unwrap();
        assert_eq!(
            result.disposition,
            NavigationTimeDisposition::PredictionHorizonExceeded
        );
        assert_eq!(result.prediction_horizon_upper_bound_ns, Some(70));
    }

    #[test]
    fn a_target_strictly_after_decision_is_not_a_nowcast() {
        let mut candidate = input(NavigationTimeClaimKind::DecisionEpochNowcast);
        candidate.target_valid_time = interval(31, 35);
        let result = evaluate_navigation_time_validity(policy(), candidate).unwrap();
        assert_eq!(
            result.disposition,
            NavigationTimeDisposition::FutureTargetForNowcast
        );
    }

    #[test]
    fn nowcast_must_cover_the_full_decision_interval() {
        let mut candidate = input(NavigationTimeClaimKind::DecisionEpochNowcast);
        candidate.decision_cutoff_time = interval(30, 35);
        candidate.target_valid_time = interval(30, 34);
        let result = evaluate_navigation_time_validity(policy(), candidate).unwrap();
        assert_eq!(
            result.disposition,
            NavigationTimeDisposition::IndeterminateTargetEpoch
        );
    }

    #[test]
    fn event_time_estimate_must_use_the_measurement_epoch_as_its_target() {
        let candidate = input(NavigationTimeClaimKind::EventTimeEstimate);
        let result = evaluate_navigation_time_validity(policy(), candidate).unwrap();
        assert_eq!(
            result.disposition,
            NavigationTimeDisposition::TargetEpochMismatch
        );
    }

    #[test]
    fn time_basis_mismatch_and_missing_commitments_reject() {
        let mut candidate = input(NavigationTimeClaimKind::DecisionEpochNowcast);
        candidate.target_valid_time.time_basis_sha256 = [0x12; SHA256_BYTES];
        assert_eq!(
            evaluate_navigation_time_validity(policy(), candidate),
            Err(NavigationTimeValidityError::UnsupportedTimeBasis)
        );

        candidate = input(NavigationTimeClaimKind::DecisionEpochNowcast);
        candidate.clock_correlation_sha256 = [0; SHA256_BYTES];
        assert_eq!(
            evaluate_navigation_time_validity(policy(), candidate),
            Err(NavigationTimeValidityError::MissingEvidenceCommitment)
        );
    }

    #[test]
    fn malformed_intervals_and_policy_fail_closed() {
        let mut candidate = input(NavigationTimeClaimKind::DecisionEpochNowcast);
        candidate.payload_receipt_time = interval(25, 20);
        assert_eq!(
            evaluate_navigation_time_validity(policy(), candidate),
            Err(NavigationTimeValidityError::InvalidTimeInterval)
        );

        let mut cfg = policy();
        cfg.policy_id = 0;
        assert_eq!(
            evaluate_navigation_time_validity(
                cfg,
                input(NavigationTimeClaimKind::DecisionEpochNowcast),
            ),
            Err(NavigationTimeValidityError::InvalidPolicy)
        );
    }

    #[test]
    fn definitely_reversed_processing_intervals_reject_as_invalid_chronology() {
        let mut candidate = input(NavigationTimeClaimKind::DecisionEpochNowcast);
        candidate.processing_start_time = interval(30, 30);
        candidate.processing_completion_time = interval(25, 25);
        assert_eq!(
            evaluate_navigation_time_validity(policy(), candidate),
            Err(NavigationTimeValidityError::InvalidTimeChronology)
        );
    }
}
