// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Structural evaluator for RRVR physical-command semantics.
//!
//! This crate checks that a command profile is complete and matches one exact
//! device capability profile. It does not inspect firmware, actuate hardware,
//! prove idempotency, classify physical outcomes, issue permits, or establish
//! functional safety. Acceptance means only that the supplied declaration
//! satisfies this reference contract.

pub const CONTRACT_VERSION: &str = "0.1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectClass {
    DesiredState,
    OneShot,
    ContinuousLease,
    Trajectory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionSemantics {
    Absolute,
    Relative,
    OneShot,
    Continuous,
    SegmentedTrajectory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SideEffectEnvelope {
    NoAdditionalEffectDeclared,
    BoundedAdditionalEffectDeclared,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WatchdogReaction {
    ControlledStop,
    DisableOutput,
    HoldLastValue,
    None,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceCapabilityProfile {
    pub device_id: String,
    pub device_model: String,
    pub firmware_revision: String,
    pub endpoint_revision: String,
    pub configuration_sha256: String,
    pub payload_schema_sha256: String,
    pub units: String,
    pub coordinate_frame: String,
    pub reference_frame: String,

    // Capabilities independently established for this exact device profile.
    pub supports_absolute_target: bool,
    pub absolute_target_replay_safe: bool,
    pub absolute_target_side_effect_free: bool,
    pub supports_durable_one_shot_token: bool,
    pub rejects_duplicate_one_shot_token: bool,
    pub maximum_effects_per_token: Option<u32>,
    pub supports_authority_epoch: bool,
    pub enforces_lease_expiry: bool,
    pub watchdog_reaction: WatchdogReaction,
    pub supports_stable_trajectory_segments: bool,
    pub supports_progress_checkpoints: bool,
    pub supports_measured_progress_resume: bool,
    pub supports_downstream_queue_reconciliation: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSemanticProfile {
    pub command_id: String,
    /// Exact device capability profile to which this command contract applies.
    pub device_id: String,
    pub device_model: String,
    pub firmware_revision: String,
    pub endpoint_revision: String,
    pub configuration_sha256: String,
    pub logical_effect_id_required: bool,
    pub attempt_id_required: bool,
    pub effect_class: EffectClass,
    pub payload_schema_sha256: String,
    pub units: String,
    pub coordinate_frame: String,
    pub reference_frame: String,
    pub motion_semantics: MotionSemantics,
    pub target_predicate: String,

    // Desired-state fields.
    pub idempotent_under_declared_contract: bool,
    pub same_payload_replay_safe: bool,
    pub tolerance_microunits: Option<u64>,
    pub side_effect_envelope: SideEffectEnvelope,

    // One-shot fields.
    pub maximum_physical_effects_per_attempt: Option<u32>,
    pub endpoint_token_required: bool,
    pub same_token_reactuates: bool,
    pub independent_outcome_observation_required: bool,

    // Continuous-lease fields.
    pub lease_expiry_ms: Option<u64>,
    pub endpoint_enforces_authority_epoch: bool,
    pub endpoint_enforces_expiry: bool,
    pub watchdog_supported: bool,
    pub watchdog_reaction: WatchdogReaction,
    pub clock_profile_id: String,

    // Trajectory fields.
    pub segment_count: Option<u32>,
    pub segment_ids_are_stable: bool,
    pub progress_checkpoints_required: bool,
    pub resume_from_measured_progress: bool,
    pub restart_from_segment_zero_allowed: bool,
    pub downstream_queue_reconciliation_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticProfileInput {
    pub device: DeviceCapabilityProfile,
    pub command: CommandSemanticProfile,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HoldReason {
    MissingDeviceIdentity,
    MissingCommandIdentity,
    MissingLogicalEffectLineage,
    MissingAttemptLineage,
    InvalidConfigurationDigest,
    InvalidDeviceSchemaDigest,
    InvalidCommandSchemaDigest,
    DeviceProfileMismatch,
    FirmwareProfileMismatch,
    EndpointProfileMissing,
    EndpointProfileMismatch,
    PayloadSchemaMismatch,
    UnitMismatch,
    CoordinateFrameMismatch,
    ReferenceFrameMismatch,
    MissingOutcomePredicate,
    DesiredStateRequiresAbsoluteMotion,
    DeviceAbsoluteTargetUnsupported,
    DeviceAbsoluteTargetNotReplaySafe,
    DeviceAbsoluteTargetHasSideEffects,
    DesiredStateNotDeclaredIdempotent,
    DesiredStateReplayNotSafe,
    DesiredStateToleranceMissing,
    DesiredStateSideEffectsUnknown,
    OneShotCardinalityNotOne,
    OneShotTokenMissing,
    DeviceOneShotTokenUnsupported,
    DeviceOneShotTokenReplayUnsafe,
    DeviceOneShotCardinalityNotOne,
    OneShotTokenCanReactuate,
    OneShotObserverMissing,
    ContinuousLeaseMissingExpiry,
    ContinuousLeaseEpochNotEnforced,
    DeviceLeaseEpochUnsupported,
    ContinuousLeaseExpiryNotEnforced,
    DeviceLeaseExpiryUnsupported,
    ContinuousLeaseWatchdogMissing,
    ContinuousLeaseUnsafeWatchdogReaction,
    DeviceWatchdogUnsafe,
    ContinuousLeaseClockMissing,
    TrajectorySegmentCountMissing,
    TrajectorySegmentIdentityUnstable,
    DeviceTrajectorySegmentsUnstable,
    TrajectoryCheckpointMissing,
    DeviceTrajectoryCheckpointMissing,
    TrajectoryResumeNotMeasured,
    DeviceTrajectoryResumeUnsupported,
    TrajectoryRestartFromZeroAllowed,
    TrajectoryQueueNotReconciled,
    DeviceTrajectoryQueueReconciliationUnsupported,
    EffectClassMotionSemanticsMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticDecision {
    AcceptProfile { effect_class: EffectClass },
    Hold(HoldReason),
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn common_profile_checks(input: &SemanticProfileInput) -> Result<(), HoldReason> {
    let device = &input.device;
    let command = &input.command;

    if !nonempty(&device.device_id) || !nonempty(&device.device_model) {
        return Err(HoldReason::MissingDeviceIdentity);
    }
    if !nonempty(&device.firmware_revision) {
        return Err(HoldReason::FirmwareProfileMismatch);
    }
    if !nonempty(&command.command_id) {
        return Err(HoldReason::MissingCommandIdentity);
    }
    if !command.logical_effect_id_required {
        return Err(HoldReason::MissingLogicalEffectLineage);
    }
    if !command.attempt_id_required {
        return Err(HoldReason::MissingAttemptLineage);
    }
    if !valid_sha256(&device.configuration_sha256)
        || !valid_sha256(&command.configuration_sha256)
    {
        return Err(HoldReason::InvalidConfigurationDigest);
    }
    if !valid_sha256(&device.payload_schema_sha256) {
        return Err(HoldReason::InvalidDeviceSchemaDigest);
    }
    if !valid_sha256(&command.payload_schema_sha256) {
        return Err(HoldReason::InvalidCommandSchemaDigest);
    }
    if !nonempty(&device.endpoint_revision) || !nonempty(&command.endpoint_revision) {
        return Err(HoldReason::EndpointProfileMissing);
    }
    if device.device_id != command.device_id
        || device.device_model != command.device_model
        || device.configuration_sha256 != command.configuration_sha256
    {
        return Err(HoldReason::DeviceProfileMismatch);
    }
    if device.firmware_revision != command.firmware_revision {
        return Err(HoldReason::FirmwareProfileMismatch);
    }
    if device.endpoint_revision != command.endpoint_revision {
        return Err(HoldReason::EndpointProfileMismatch);
    }
    if device.payload_schema_sha256 != command.payload_schema_sha256 {
        return Err(HoldReason::PayloadSchemaMismatch);
    }
    if device.units != command.units {
        return Err(HoldReason::UnitMismatch);
    }
    if device.coordinate_frame != command.coordinate_frame {
        return Err(HoldReason::CoordinateFrameMismatch);
    }
    if device.reference_frame != command.reference_frame {
        return Err(HoldReason::ReferenceFrameMismatch);
    }
    if !nonempty(&command.target_predicate) {
        return Err(HoldReason::MissingOutcomePredicate);
    }
    Ok(())
}

fn evaluate_class(
    device: &DeviceCapabilityProfile,
    command: &CommandSemanticProfile,
) -> Result<(), HoldReason> {
    match command.effect_class {
        EffectClass::DesiredState => {
            if command.motion_semantics != MotionSemantics::Absolute {
                return Err(HoldReason::DesiredStateRequiresAbsoluteMotion);
            }
            if !device.supports_absolute_target {
                return Err(HoldReason::DeviceAbsoluteTargetUnsupported);
            }
            if !device.absolute_target_replay_safe {
                return Err(HoldReason::DeviceAbsoluteTargetNotReplaySafe);
            }
            if !device.absolute_target_side_effect_free {
                return Err(HoldReason::DeviceAbsoluteTargetHasSideEffects);
            }
            if !command.idempotent_under_declared_contract {
                return Err(HoldReason::DesiredStateNotDeclaredIdempotent);
            }
            if !command.same_payload_replay_safe {
                return Err(HoldReason::DesiredStateReplayNotSafe);
            }
            if !matches!(command.tolerance_microunits, Some(value) if value > 0) {
                return Err(HoldReason::DesiredStateToleranceMissing);
            }
            if command.side_effect_envelope != SideEffectEnvelope::NoAdditionalEffectDeclared {
                return Err(HoldReason::DesiredStateSideEffectsUnknown);
            }
        }
        EffectClass::OneShot => {
            if command.motion_semantics != MotionSemantics::OneShot {
                return Err(HoldReason::EffectClassMotionSemanticsMismatch);
            }
            if command.maximum_physical_effects_per_attempt != Some(1) {
                return Err(HoldReason::OneShotCardinalityNotOne);
            }
            if device.maximum_effects_per_token != Some(1) {
                return Err(HoldReason::DeviceOneShotCardinalityNotOne);
            }
            if !device.supports_durable_one_shot_token {
                return Err(HoldReason::DeviceOneShotTokenUnsupported);
            }
            if !device.rejects_duplicate_one_shot_token {
                return Err(HoldReason::DeviceOneShotTokenReplayUnsafe);
            }
            if !command.endpoint_token_required {
                return Err(HoldReason::OneShotTokenMissing);
            }
            if command.same_token_reactuates {
                return Err(HoldReason::OneShotTokenCanReactuate);
            }
            if !command.independent_outcome_observation_required {
                return Err(HoldReason::OneShotObserverMissing);
            }
        }
        EffectClass::ContinuousLease => {
            if command.motion_semantics != MotionSemantics::Continuous {
                return Err(HoldReason::EffectClassMotionSemanticsMismatch);
            }
            if !matches!(command.lease_expiry_ms, Some(value) if value > 0) {
                return Err(HoldReason::ContinuousLeaseMissingExpiry);
            }
            if !command.endpoint_enforces_authority_epoch {
                return Err(HoldReason::ContinuousLeaseEpochNotEnforced);
            }
            if !device.supports_authority_epoch {
                return Err(HoldReason::DeviceLeaseEpochUnsupported);
            }
            if !command.endpoint_enforces_expiry {
                return Err(HoldReason::ContinuousLeaseExpiryNotEnforced);
            }
            if !device.enforces_lease_expiry {
                return Err(HoldReason::DeviceLeaseExpiryUnsupported);
            }
            if !command.watchdog_supported {
                return Err(HoldReason::ContinuousLeaseWatchdogMissing);
            }
            if !matches!(
                command.watchdog_reaction,
                WatchdogReaction::ControlledStop | WatchdogReaction::DisableOutput
            ) {
                return Err(HoldReason::ContinuousLeaseUnsafeWatchdogReaction);
            }
            if !matches!(
                device.watchdog_reaction,
                WatchdogReaction::ControlledStop | WatchdogReaction::DisableOutput
            ) {
                return Err(HoldReason::DeviceWatchdogUnsafe);
            }
            if !nonempty(&command.clock_profile_id) || command.clock_profile_id == "unknown" {
                return Err(HoldReason::ContinuousLeaseClockMissing);
            }
        }
        EffectClass::Trajectory => {
            if command.motion_semantics != MotionSemantics::SegmentedTrajectory {
                return Err(HoldReason::EffectClassMotionSemanticsMismatch);
            }
            if !matches!(command.segment_count, Some(value) if value > 0) {
                return Err(HoldReason::TrajectorySegmentCountMissing);
            }
            if !command.segment_ids_are_stable {
                return Err(HoldReason::TrajectorySegmentIdentityUnstable);
            }
            if !device.supports_stable_trajectory_segments {
                return Err(HoldReason::DeviceTrajectorySegmentsUnstable);
            }
            if !command.progress_checkpoints_required {
                return Err(HoldReason::TrajectoryCheckpointMissing);
            }
            if !device.supports_progress_checkpoints {
                return Err(HoldReason::DeviceTrajectoryCheckpointMissing);
            }
            if !command.resume_from_measured_progress {
                return Err(HoldReason::TrajectoryResumeNotMeasured);
            }
            if !device.supports_measured_progress_resume {
                return Err(HoldReason::DeviceTrajectoryResumeUnsupported);
            }
            if command.restart_from_segment_zero_allowed {
                return Err(HoldReason::TrajectoryRestartFromZeroAllowed);
            }
            if !command.downstream_queue_reconciliation_required {
                return Err(HoldReason::TrajectoryQueueNotReconciled);
            }
            if !device.supports_downstream_queue_reconciliation {
                return Err(HoldReason::DeviceTrajectoryQueueReconciliationUnsupported);
            }
        }
    }
    Ok(())
}

pub fn evaluate(input: &SemanticProfileInput) -> SemanticDecision {
    if let Err(reason) = common_profile_checks(input) {
        return SemanticDecision::Hold(reason);
    }
    match evaluate_class(&input.device, &input.command) {
        Ok(()) => SemanticDecision::AcceptProfile {
            effect_class: input.command.effect_class,
        },
        Err(reason) => SemanticDecision::Hold(reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_device() -> DeviceCapabilityProfile {
        DeviceCapabilityProfile {
            device_id: "device-01".into(),
            device_model: "reference-actuator".into(),
            firmware_revision: "fw-1.0".into(),
            endpoint_revision: "endpoint-1".into(),
            configuration_sha256: "a".repeat(64),
            payload_schema_sha256: "b".repeat(64),
            units: "radians".into(),
            coordinate_frame: "joint-space-v1".into(),
            reference_frame: "robot-base-v1".into(),
            supports_absolute_target: true,
            absolute_target_replay_safe: true,
            absolute_target_side_effect_free: true,
            supports_durable_one_shot_token: true,
            rejects_duplicate_one_shot_token: true,
            maximum_effects_per_token: Some(1),
            supports_authority_epoch: true,
            enforces_lease_expiry: true,
            watchdog_reaction: WatchdogReaction::ControlledStop,
            supports_stable_trajectory_segments: true,
            supports_progress_checkpoints: true,
            supports_measured_progress_resume: true,
            supports_downstream_queue_reconciliation: true,
        }
    }

    fn base_command(effect_class: EffectClass, motion_semantics: MotionSemantics) -> CommandSemanticProfile {
        CommandSemanticProfile {
            command_id: "command-01".into(),
            device_id: "device-01".into(),
            device_model: "reference-actuator".into(),
            firmware_revision: "fw-1.0".into(),
            endpoint_revision: "endpoint-1".into(),
            configuration_sha256: "a".repeat(64),
            logical_effect_id_required: true,
            attempt_id_required: true,
            effect_class,
            payload_schema_sha256: "b".repeat(64),
            units: "radians".into(),
            coordinate_frame: "joint-space-v1".into(),
            reference_frame: "robot-base-v1".into(),
            motion_semantics,
            target_predicate: "declared-predicate-v1".into(),
            idempotent_under_declared_contract: false,
            same_payload_replay_safe: false,
            tolerance_microunits: None,
            side_effect_envelope: SideEffectEnvelope::Unknown,
            maximum_physical_effects_per_attempt: None,
            endpoint_token_required: false,
            same_token_reactuates: true,
            independent_outcome_observation_required: false,
            lease_expiry_ms: None,
            endpoint_enforces_authority_epoch: false,
            endpoint_enforces_expiry: false,
            watchdog_supported: false,
            watchdog_reaction: WatchdogReaction::None,
            clock_profile_id: "unknown".into(),
            segment_count: None,
            segment_ids_are_stable: false,
            progress_checkpoints_required: false,
            resume_from_measured_progress: false,
            restart_from_segment_zero_allowed: true,
            downstream_queue_reconciliation_required: false,
        }
    }

    fn valid_desired_state() -> SemanticProfileInput {
        let mut command = base_command(EffectClass::DesiredState, MotionSemantics::Absolute);
        command.idempotent_under_declared_contract = true;
        command.same_payload_replay_safe = true;
        command.tolerance_microunits = Some(10);
        command.side_effect_envelope = SideEffectEnvelope::NoAdditionalEffectDeclared;
        SemanticProfileInput { device: base_device(), command }
    }

    fn valid_one_shot() -> SemanticProfileInput {
        let mut command = base_command(EffectClass::OneShot, MotionSemantics::OneShot);
        command.maximum_physical_effects_per_attempt = Some(1);
        command.endpoint_token_required = true;
        command.same_token_reactuates = false;
        command.independent_outcome_observation_required = true;
        SemanticProfileInput { device: base_device(), command }
    }

    fn valid_continuous_lease() -> SemanticProfileInput {
        let mut command = base_command(EffectClass::ContinuousLease, MotionSemantics::Continuous);
        command.lease_expiry_ms = Some(250);
        command.endpoint_enforces_authority_epoch = true;
        command.endpoint_enforces_expiry = true;
        command.watchdog_supported = true;
        command.watchdog_reaction = WatchdogReaction::ControlledStop;
        command.clock_profile_id = "monotonic-device-clock-v1".into();
        SemanticProfileInput { device: base_device(), command }
    }

    fn valid_trajectory() -> SemanticProfileInput {
        let mut command = base_command(EffectClass::Trajectory, MotionSemantics::SegmentedTrajectory);
        command.segment_count = Some(6);
        command.segment_ids_are_stable = true;
        command.progress_checkpoints_required = true;
        command.resume_from_measured_progress = true;
        command.restart_from_segment_zero_allowed = false;
        command.downstream_queue_reconciliation_required = true;
        SemanticProfileInput { device: base_device(), command }
    }

    #[test]
    fn accepts_all_four_well_formed_effect_classes() {
        for input in [valid_desired_state(), valid_one_shot(), valid_continuous_lease(), valid_trajectory()] {
            assert!(matches!(evaluate(&input), SemanticDecision::AcceptProfile { .. }));
        }
    }

    #[test]
    fn rejects_device_without_absolute_target_capability() {
        let mut input = valid_desired_state();
        input.device.supports_absolute_target = false;
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::DeviceAbsoluteTargetUnsupported)
        );
    }

    #[test]
    fn rejects_device_without_durable_one_shot_token() {
        let mut input = valid_one_shot();
        input.device.supports_durable_one_shot_token = false;
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::DeviceOneShotTokenUnsupported)
        );
    }

    #[test]
    fn rejects_device_that_reacts_to_duplicate_one_shot_tokens() {
        let mut input = valid_one_shot();
        input.device.rejects_duplicate_one_shot_token = false;
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::DeviceOneShotTokenReplayUnsafe)
        );
    }

    #[test]
    fn rejects_device_without_lease_expiry_enforcement() {
        let mut input = valid_continuous_lease();
        input.device.enforces_lease_expiry = false;
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::DeviceLeaseExpiryUnsupported)
        );
    }

    #[test]
    fn rejects_device_without_checkpointed_trajectory_resume() {
        let mut input = valid_trajectory();
        input.device.supports_measured_progress_resume = false;
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::DeviceTrajectoryResumeUnsupported)
        );
    }

    #[test]
    fn rejects_relative_motion_mislabeled_as_idempotent_desired_state() {
        let mut input = valid_desired_state();
        input.command.motion_semantics = MotionSemantics::Relative;
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::DesiredStateRequiresAbsoluteMotion)
        );
    }

    #[test]
    fn rejects_one_shot_without_endpoint_token() {
        let mut input = valid_one_shot();
        input.command.endpoint_token_required = false;
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::OneShotTokenMissing)
        );
    }

    #[test]
    fn rejects_one_shot_with_multiple_effects_per_attempt() {
        let mut input = valid_one_shot();
        input.command.maximum_physical_effects_per_attempt = Some(2);
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::OneShotCardinalityNotOne)
        );
    }

    #[test]
    fn rejects_continuous_lease_without_expiry_and_watchdog() {
        let mut input = valid_continuous_lease();
        input.command.lease_expiry_ms = None;
        input.command.watchdog_supported = false;
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::ContinuousLeaseMissingExpiry)
        );
    }

    #[test]
    fn rejects_continuous_lease_with_hold_last_value_watchdog() {
        let mut input = valid_continuous_lease();
        input.command.watchdog_reaction = WatchdogReaction::HoldLastValue;
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::ContinuousLeaseUnsafeWatchdogReaction)
        );
    }

    #[test]
    fn rejects_trajectory_restart_from_segment_zero() {
        let mut input = valid_trajectory();
        input.command.restart_from_segment_zero_allowed = true;
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::TrajectoryRestartFromZeroAllowed)
        );
    }

    #[test]
    fn rejects_device_firmware_mismatch() {
        let mut input = valid_desired_state();
        input.device.firmware_revision = "fw-new".into();
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::FirmwareProfileMismatch)
        );
    }

    #[test]
    fn rejects_configuration_profile_substitution() {
        let mut input = valid_desired_state();
        input.command.configuration_sha256 = "c".repeat(64);
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::DeviceProfileMismatch)
        );
    }

    #[test]
    fn rejects_endpoint_revision_substitution() {
        let mut input = valid_desired_state();
        input.device.endpoint_revision = "endpoint-new".into();
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::EndpointProfileMismatch)
        );
    }

    #[test]
    fn rejects_device_identity_substitution() {
        let mut input = valid_desired_state();
        input.command.device_id = "other-device".into();
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::DeviceProfileMismatch)
        );
    }

    #[test]
    fn rejects_coordinate_frame_substitution() {
        let mut input = valid_desired_state();
        input.command.coordinate_frame = "world-frame-substituted".into();
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::CoordinateFrameMismatch)
        );
    }

    #[test]
    fn rejects_payload_schema_substitution() {
        let mut input = valid_desired_state();
        input.command.payload_schema_sha256 = "c".repeat(64);
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::PayloadSchemaMismatch)
        );
    }

    #[test]
    fn rejects_missing_logical_effect_lineage() {
        let mut input = valid_desired_state();
        input.command.logical_effect_id_required = false;
        assert_eq!(
            evaluate(&input),
            SemanticDecision::Hold(HoldReason::MissingLogicalEffectLineage)
        );
    }
}
