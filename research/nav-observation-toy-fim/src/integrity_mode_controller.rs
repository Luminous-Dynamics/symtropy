//! Evidence-gated operating modes for localization integrity.
//!
//! The controller deliberately separates:
//! - immediate downgrade when support or integrity evidence fails;
//! - slow recovery requiring several fresh, distinct evidence epochs;
//! - a protection-level alert-limit decision when all integrity gates pass.
//!
//! This is a deterministic mode controller, not a probabilistic integrity
//! proof, a PID controller, or an actuator authorization mechanism. Smooth PL
//! dynamics may be used upstream, but they must never suppress these gates.

use std::fmt;

const SHA256_BYTES: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrityMode {
    /// Evidence is missing, unsupported, contradictory or stale.
    Unknown,
    /// No usable localization solution is currently available.
    Unavailable,
    /// Evidence is supported, but the protection level exceeds the alert limit
    /// or the system is still accumulating independent recovery evidence.
    Degraded,
    /// All required evidence gates pass and the recovery policy is satisfied.
    Normal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrityModeReason {
    AwaitingInitialQualification,
    NormalEvidenceMaintained,
    RecoveryEvidenceAccumulating,
    IndependentRecoveryQualified,
    SolutionUnavailable,
    SourceEvidenceIncomplete,
    FaultInventoryIncomplete,
    HardFaultEnvelopeUnbounded,
    IndependentPosteriorAuditFailed,
    EvidenceStale,
    CalibrationStale,
    PredictorBindingMismatch,
    ScoreDefinitionMismatch,
    InvalidProtectionLevel,
    ProtectionLevelExceedsAlertLimit,
    InvalidEpochIdentity,
    EpochReplayOrReordering,
    InvalidObservationSetIdentity,
    InvalidDependencyGroupIdentity,
    MissingEvidenceReceipt,
    EvidenceReceiptReplay,
    ObservationSetReplay,
    RecoveryDependencyGroupRepeated,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntegrityModePolicy {
    /// Frozen application alert limit in metres.
    pub alert_limit_m: f64,
    /// Number of consecutive, independently sourced qualifying epochs required
    /// to transition from any non-NORMAL state to NORMAL.
    pub consecutive_independent_recovery_epochs: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntegrityEvidenceFrame {
    /// Strictly increasing caller-defined evidence epoch.
    pub epoch_id: u64,
    /// Strictly increasing source-ledger sequence for the complete observation set.
    pub observation_set_sequence_id: u64,
    /// Independence/dependency group used to qualify recovery evidence.
    ///
    /// A recovery streak must use a distinct group for every qualifying epoch.
    pub dependency_group_id: u64,
    /// Strictly increasing sequence of the source-complete evidence receipt.
    pub evidence_receipt_sequence_id: u64,
    /// Digest of the source-complete evidence receipt; the canonical receipt
    /// must include epoch_id and evidence_receipt_sequence_id.
    pub evidence_receipt_sha256: [u8; SHA256_BYTES],
    pub solution_available: bool,
    pub source_complete: bool,
    pub fault_inventory_complete: bool,
    pub hard_fault_envelope_bounded: bool,
    pub independent_posterior_audit_passed: bool,
    pub evidence_fresh: bool,
    pub calibration_current: bool,
    pub predictor_binding_matches: bool,
    pub score_definition_binding_matches: bool,
    /// Already-computed conservative protection-level upper bound.
    pub protection_level_upper_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntegrityModeTransition {
    pub previous_mode: IntegrityMode,
    pub current_mode: IntegrityMode,
    pub reason: IntegrityModeReason,
    pub recovery_streak: usize,
    pub recovery_required: usize,
    /// This flag describes the qualification state only. It does not authorize
    /// a motion, payment, safety or actuator transition.
    pub qualified_for_normal_operation: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrityModeError {
    InvalidPolicy,
}

impl fmt::Display for IntegrityModeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy => write!(
                f,
                "alert limit must be finite and positive and recovery epochs must be at least two"
            ),
        }
    }
}

impl std::error::Error for IntegrityModeError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RecoveryIdentity {
    observation_set_sequence_id: u64,
    dependency_group_id: u64,
    evidence_receipt_sha256: [u8; SHA256_BYTES],
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntegrityModeController {
    policy: IntegrityModePolicy,
    mode: IntegrityMode,
    last_epoch_id: Option<u64>,
    last_observation_set_sequence_id: Option<u64>,
    last_evidence_receipt_sequence_id: Option<u64>,
    last_evidence_receipt_sha256: Option<[u8; SHA256_BYTES]>,
    recovery_identities: Vec<RecoveryIdentity>,
}

impl IntegrityModeController {
    pub fn new(policy: IntegrityModePolicy) -> Result<Self, IntegrityModeError> {
        if !policy.alert_limit_m.is_finite()
            || policy.alert_limit_m <= 0.0
            || policy.consecutive_independent_recovery_epochs < 2
        {
            return Err(IntegrityModeError::InvalidPolicy);
        }

        Ok(Self {
            policy,
            mode: IntegrityMode::Unknown,
            last_epoch_id: None,
            last_observation_set_sequence_id: None,
            last_evidence_receipt_sequence_id: None,
            last_evidence_receipt_sha256: None,
            recovery_identities: Vec::new(),
        })
    }

    pub fn mode(&self) -> IntegrityMode {
        self.mode
    }

    pub fn policy(&self) -> IntegrityModePolicy {
        self.policy
    }

    pub fn recovery_streak(&self) -> usize {
        self.recovery_identities.len()
    }

    /// Consume one evidence epoch and compute the next operating mode.
    ///
    /// Any failed evidence gate immediately leaves NORMAL. Returning to NORMAL
    /// requires a run of fresh evidence with unique observation-set IDs,
    /// unique receipt digests, and distinct dependency groups.
    pub fn observe(&mut self, frame: IntegrityEvidenceFrame) -> IntegrityModeTransition {
        let previous_mode = self.mode;

        if frame.epoch_id == 0 {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::InvalidEpochIdentity,
                true,
            );
        }

        if self
            .last_epoch_id
            .is_some_and(|last_epoch| frame.epoch_id <= last_epoch)
        {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::EpochReplayOrReordering,
                true,
            );
        }

        self.last_epoch_id = Some(frame.epoch_id);
        let previous_observation_set_sequence_id = self.last_observation_set_sequence_id;
        let previous_evidence_receipt_sequence_id = self.last_evidence_receipt_sequence_id;
        let previous_evidence_receipt_sha256 = self.last_evidence_receipt_sha256;

        if frame.observation_set_sequence_id != 0
            && previous_observation_set_sequence_id
                .is_none_or(|previous| frame.observation_set_sequence_id > previous)
        {
            self.last_observation_set_sequence_id = Some(frame.observation_set_sequence_id);
        }
        if frame.evidence_receipt_sequence_id != 0
            && previous_evidence_receipt_sequence_id
                .is_none_or(|previous| frame.evidence_receipt_sequence_id > previous)
        {
            self.last_evidence_receipt_sequence_id = Some(frame.evidence_receipt_sequence_id);
            if frame.evidence_receipt_sha256.iter().any(|byte| *byte != 0) {
                self.last_evidence_receipt_sha256 = Some(frame.evidence_receipt_sha256);
            }
        }

        if !frame.solution_available {
            return self.transition(
                previous_mode,
                IntegrityMode::Unavailable,
                IntegrityModeReason::SolutionUnavailable,
                true,
            );
        }

        if !frame.source_complete {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::SourceEvidenceIncomplete,
                true,
            );
        }

        if !frame.fault_inventory_complete {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::FaultInventoryIncomplete,
                true,
            );
        }

        if !frame.hard_fault_envelope_bounded {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::HardFaultEnvelopeUnbounded,
                true,
            );
        }

        if !frame.independent_posterior_audit_passed {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::IndependentPosteriorAuditFailed,
                true,
            );
        }

        if !frame.evidence_fresh {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::EvidenceStale,
                true,
            );
        }

        if !frame.calibration_current {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::CalibrationStale,
                true,
            );
        }

        if !frame.predictor_binding_matches {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::PredictorBindingMismatch,
                true,
            );
        }

        if !frame.score_definition_binding_matches {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::ScoreDefinitionMismatch,
                true,
            );
        }

        if !frame.protection_level_upper_m.is_finite()
            || frame.protection_level_upper_m < 0.0
        {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::InvalidProtectionLevel,
                true,
            );
        }

        if frame.observation_set_sequence_id == 0 {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::InvalidObservationSetIdentity,
                true,
            );
        }

        if frame.dependency_group_id == 0 {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::InvalidDependencyGroupIdentity,
                true,
            );
        }

        if frame.evidence_receipt_sequence_id == 0
            || frame.evidence_receipt_sha256.iter().all(|byte| *byte == 0)
        {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::MissingEvidenceReceipt,
                true,
            );
        }

        if previous_observation_set_sequence_id
            .is_some_and(|previous| frame.observation_set_sequence_id <= previous)
        {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::ObservationSetReplay,
                true,
            );
        }

        if previous_evidence_receipt_sequence_id
            .is_some_and(|previous| frame.evidence_receipt_sequence_id <= previous)
            || previous_evidence_receipt_sha256 == Some(frame.evidence_receipt_sha256)
        {
            return self.transition(
                previous_mode,
                IntegrityMode::Unknown,
                IntegrityModeReason::EvidenceReceiptReplay,
                true,
            );
        }

        if frame.protection_level_upper_m > self.policy.alert_limit_m {
            return self.transition(
                previous_mode,
                IntegrityMode::Degraded,
                IntegrityModeReason::ProtectionLevelExceedsAlertLimit,
                true,
            );
        }

        if previous_mode == IntegrityMode::Normal {
            self.mode = IntegrityMode::Normal;
            self.recovery_identities.clear();
            return self.transition(
                previous_mode,
                self.mode,
                IntegrityModeReason::NormalEvidenceMaintained,
                false,
            );
        }

        let recovery_identity = RecoveryIdentity {
            observation_set_sequence_id: frame.observation_set_sequence_id,
            dependency_group_id: frame.dependency_group_id,
            evidence_receipt_sha256: frame.evidence_receipt_sha256,
        };

        if self.recovery_identities.iter().any(|identity| {
            identity.observation_set_sequence_id == recovery_identity.observation_set_sequence_id
                || identity.dependency_group_id == recovery_identity.dependency_group_id
                || identity.evidence_receipt_sha256
                    == recovery_identity.evidence_receipt_sha256
        }) {
            return self.transition(
                previous_mode,
                IntegrityMode::Degraded,
                IntegrityModeReason::RecoveryDependencyGroupRepeated,
                true,
            );
        }

        self.recovery_identities.push(recovery_identity);

        if self.recovery_identities.len()
            >= self.policy.consecutive_independent_recovery_epochs
        {
            self.mode = IntegrityMode::Normal;
            return self.transition(
                previous_mode,
                self.mode,
                IntegrityModeReason::IndependentRecoveryQualified,
                false,
            );
        }

        self.mode = IntegrityMode::Degraded;
        self.transition(
            previous_mode,
            self.mode,
            IntegrityModeReason::RecoveryEvidenceAccumulating,
            false,
        )
    }

    fn transition(
        &mut self,
        previous_mode: IntegrityMode,
        next_mode: IntegrityMode,
        reason: IntegrityModeReason,
        reset_recovery: bool,
    ) -> IntegrityModeTransition {
        if reset_recovery {
            self.recovery_identities.clear();
        }
        self.mode = next_mode;
        IntegrityModeTransition {
            previous_mode,
            current_mode: self.mode,
            reason,
            recovery_streak: self.recovery_identities.len(),
            recovery_required: self.policy.consecutive_independent_recovery_epochs,
            qualified_for_normal_operation: self.mode == IntegrityMode::Normal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> IntegrityModePolicy {
        IntegrityModePolicy {
            alert_limit_m: 0.5,
            consecutive_independent_recovery_epochs: 3,
        }
    }

    fn controller() -> IntegrityModeController {
        IntegrityModeController::new(policy()).unwrap()
    }

    fn frame(epoch: u64, group: u64, pl: f64) -> IntegrityEvidenceFrame {
        IntegrityEvidenceFrame {
            epoch_id: epoch,
            observation_set_sequence_id: 1_000 + epoch,
            dependency_group_id: group,
            evidence_receipt_sequence_id: epoch,
            evidence_receipt_sha256: [epoch as u8; SHA256_BYTES],
            solution_available: true,
            source_complete: true,
            fault_inventory_complete: true,
            hard_fault_envelope_bounded: true,
            independent_posterior_audit_passed: true,
            evidence_fresh: true,
            calibration_current: true,
            predictor_binding_matches: true,
            score_definition_binding_matches: true,
            protection_level_upper_m: pl,
        }
    }

    fn qualify_normal(controller: &mut IntegrityModeController, start: u64) {
        for offset in 0..3 {
            let result = controller.observe(frame(start + offset, 100 + offset, 0.2));
            if offset < 2 {
                assert_eq!(result.current_mode, IntegrityMode::Degraded);
                assert!(!result.qualified_for_normal_operation);
            } else {
                assert_eq!(result.current_mode, IntegrityMode::Normal);
                assert!(result.qualified_for_normal_operation);
            }
        }
    }

    #[test]
    fn startup_requires_three_independent_evidence_epochs() {
        let mut controller = controller();
        assert_eq!(controller.mode(), IntegrityMode::Unknown);
        qualify_normal(&mut controller, 1);
    }

    #[test]
    fn any_source_inventory_or_hard_fault_failure_immediately_exits_normal() {
        let mut controller = controller();
        qualify_normal(&mut controller, 1);

        let mut bad_source = frame(4, 104, 0.1);
        bad_source.source_complete = false;
        let result = controller.observe(bad_source);
        assert_eq!(result.current_mode, IntegrityMode::Unknown);
        assert_eq!(result.reason, IntegrityModeReason::SourceEvidenceIncomplete);
        assert_eq!(result.recovery_streak, 0);

        let mut incomplete_inventory = frame(5, 105, 0.1);
        incomplete_inventory.fault_inventory_complete = false;
        assert_eq!(
            controller.observe(incomplete_inventory).current_mode,
            IntegrityMode::Unknown
        );

        let mut unbounded = frame(6, 106, 0.1);
        unbounded.hard_fault_envelope_bounded = false;
        assert_eq!(
            controller.observe(unbounded).reason,
            IntegrityModeReason::HardFaultEnvelopeUnbounded
        );
    }

    #[test]
    fn unavailable_solution_is_distinct_from_unknown_integrity() {
        let mut controller = controller();
        qualify_normal(&mut controller, 1);
        let mut no_solution = frame(4, 104, 0.1);
        no_solution.solution_available = false;
        let result = controller.observe(no_solution);
        assert_eq!(result.current_mode, IntegrityMode::Unavailable);
        assert_eq!(result.reason, IntegrityModeReason::SolutionUnavailable);
    }

    #[test]
    fn protection_level_above_alert_limit_immediately_degrades() {
        let mut controller = controller();
        qualify_normal(&mut controller, 1);
        let result = controller.observe(frame(4, 104, 0.500_001));
        assert_eq!(result.current_mode, IntegrityMode::Degraded);
        assert_eq!(
            result.reason,
            IntegrityModeReason::ProtectionLevelExceedsAlertLimit
        );
        assert_eq!(result.recovery_streak, 0);
    }

    #[test]
    fn stale_calibration_or_changed_predictor_or_score_contract_downgrades() {
        let mut controller = controller();
        qualify_normal(&mut controller, 1);

        let mut stale_calibration = frame(4, 104, 0.1);
        stale_calibration.calibration_current = false;
        assert_eq!(
            controller.observe(stale_calibration).reason,
            IntegrityModeReason::CalibrationStale
        );

        let mut changed_predictor = frame(5, 105, 0.1);
        changed_predictor.predictor_binding_matches = false;
        assert_eq!(
            controller.observe(changed_predictor).reason,
            IntegrityModeReason::PredictorBindingMismatch
        );

        let mut changed_score = frame(6, 106, 0.1);
        changed_score.score_definition_binding_matches = false;
        assert_eq!(
            controller.observe(changed_score).reason,
            IntegrityModeReason::ScoreDefinitionMismatch
        );
    }

    #[test]
    fn lower_protection_level_alone_does_not_restore_normal() {
        let mut controller = controller();
        qualify_normal(&mut controller, 1);

        let first = controller.observe(frame(4, 104, 0.1));
        assert_eq!(first.current_mode, IntegrityMode::Degraded);
        assert_eq!(first.recovery_streak, 1);
        assert!(!first.qualified_for_normal_operation);

        let second = controller.observe(frame(5, 105, 0.05));
        assert_eq!(second.current_mode, IntegrityMode::Degraded);
        assert_eq!(second.recovery_streak, 2);

        let third = controller.observe(frame(6, 106, 0.04));
        assert_eq!(third.current_mode, IntegrityMode::Normal);
        assert_eq!(
            third.reason,
            IntegrityModeReason::IndependentRecoveryQualified
        );
    }

    #[test]
    fn repeated_recovery_dependency_group_resets_the_recovery_streak() {
        let mut controller = controller();

        assert_eq!(
            controller.observe(frame(1, 10, 0.1)).recovery_streak,
            1
        );
        assert_eq!(
            controller.observe(frame(2, 10, 0.1)).reason,
            IntegrityModeReason::RecoveryDependencyGroupRepeated
        );
        assert_eq!(controller.mode(), IntegrityMode::Degraded);
        assert_eq!(controller.recovery_streak(), 0);

        assert_eq!(controller.observe(frame(3, 11, 0.1)).recovery_streak, 1);
        assert_eq!(controller.observe(frame(4, 12, 0.1)).recovery_streak, 2);
        assert_eq!(controller.observe(frame(5, 13, 0.1)).current_mode, IntegrityMode::Normal);
    }

    #[test]
    fn replayed_normal_evidence_and_out_of_order_epochs_force_unknown() {
        let mut controller = controller();
        qualify_normal(&mut controller, 1);

        let mut replay = frame(4, 104, 0.1);
        let first = controller.observe(replay);
        assert_eq!(first.current_mode, IntegrityMode::Normal);

        replay.epoch_id = 5;
        replay.observation_set_sequence_id += 1;
        let replayed_receipt = controller.observe(replay);
        assert_eq!(replayed_receipt.current_mode, IntegrityMode::Unknown);
        assert_eq!(
            replayed_receipt.reason,
            IntegrityModeReason::EvidenceReceiptReplay
        );

        let out_of_order = controller.observe(frame(4, 106, 0.1));
        assert_eq!(out_of_order.current_mode, IntegrityMode::Unknown);
        assert_eq!(
            out_of_order.reason,
            IntegrityModeReason::EpochReplayOrReordering
        );
    }

    #[test]
    fn reordered_receipt_sequence_cannot_roll_back_replay_watermark() {
        let mut controller = controller();
        qualify_normal(&mut controller, 1);
        let first = controller.observe(frame(4, 104, 0.1));
        assert_eq!(first.current_mode, IntegrityMode::Normal);

        let mut reordered = frame(5, 105, 0.1);
        reordered.evidence_receipt_sequence_id = 3;
        assert_eq!(
            controller.observe(reordered).reason,
            IntegrityModeReason::EvidenceReceiptReplay
        );

        let mut replay_old_watermark = frame(6, 106, 0.1);
        replay_old_watermark.evidence_receipt_sequence_id = 4;
        assert_eq!(
            controller.observe(replay_old_watermark).reason,
            IntegrityModeReason::EvidenceReceiptReplay
        );
    }

    #[test]
    fn missing_ids_receipts_invalid_pl_and_stale_evidence_fail_closed() {
        let mut controller = controller();

        let mut missing_receipt = frame(1, 1, 0.1);
        missing_receipt.evidence_receipt_sha256 = [0; SHA256_BYTES];
        assert_eq!(
            controller.observe(missing_receipt).reason,
            IntegrityModeReason::MissingEvidenceReceipt
        );

        let mut missing_observation = frame(2, 2, 0.1);
        missing_observation.observation_set_sequence_id = 0;
        assert_eq!(
            controller.observe(missing_observation).reason,
            IntegrityModeReason::InvalidObservationSetIdentity
        );

        let mut stale = frame(3, 3, 0.1);
        stale.evidence_fresh = false;
        assert_eq!(
            controller.observe(stale).reason,
            IntegrityModeReason::EvidenceStale
        );

        let invalid_pl = controller.observe(frame(4, 4, f64::NAN));
        assert_eq!(invalid_pl.current_mode, IntegrityMode::Unknown);
        assert_eq!(invalid_pl.reason, IntegrityModeReason::InvalidProtectionLevel);
    }

    #[test]
    fn duplicate_observation_set_in_normal_mode_forces_unknown() {
        let mut controller = controller();
        qualify_normal(&mut controller, 1);

        let mut next = frame(4, 104, 0.1);
        next.observation_set_sequence_id = 1_003;
        let result = controller.observe(next);
        assert_eq!(result.current_mode, IntegrityMode::Unknown);
        assert_eq!(
            result.reason,
            IntegrityModeReason::ObservationSetReplay
        );
    }

    #[test]
    fn invalid_policy_is_rejected() {
        assert_eq!(
            IntegrityModeController::new(IntegrityModePolicy {
                alert_limit_m: f64::NAN,
                consecutive_independent_recovery_epochs: 3,
            }),
            Err(IntegrityModeError::InvalidPolicy)
        );
        assert_eq!(
            IntegrityModeController::new(IntegrityModePolicy {
                alert_limit_m: 0.5,
                consecutive_independent_recovery_epochs: 1,
            }),
            Err(IntegrityModeError::InvalidPolicy)
        );
    }
}
