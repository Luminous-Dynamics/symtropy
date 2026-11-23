// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Frozen, paired RRVR fault-campaign manifests.
//!
//! This module validates experiment design and comparability. It does not
//! inject faults, certify platform inventory, authenticate physical evidence,
//! or claim that the declared injector actually delivered a fault.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const CAMPAIGN_MANIFEST_VERSION: &str = "0.1";
pub const MAX_SCENARIOS: usize = 256;
pub const MAX_FAULTS_PER_SCENARIO: usize = 64;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkArm {
    LastAttemptOnly,
    FlatHistory,
    OwnerReceipts,
    FullRrvr,
}

impl BenchmarkArm {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LastAttemptOnly => "last_attempt_only",
            Self::FlatHistory => "flat_history",
            Self::OwnerReceipts => "owner_receipts",
            Self::FullRrvr => "full_rrvr",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlClass {
    Positive,
    Negative,
    Mutation,
}

impl ControlClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Mutation => "mutation",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedDisposition {
    Admit,
    Hold,
    Block,
    Measure,
}

impl ExpectedDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admit => "admit",
            Self::Hold => "hold",
            Self::Block => "block",
            Self::Measure => "measure",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetKind {
    ColdStart,
    ProcessRestart,
    ControllerReset,
    SimulatorSnapshotRestore,
    HardwarePowerCycle,
}

impl ResetKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ColdStart => "cold_start",
            Self::ProcessRestart => "process_restart",
            Self::ControllerReset => "controller_reset",
            Self::SimulatorSnapshotRestore => "simulator_snapshot_restore",
            Self::HardwarePowerCycle => "hardware_power_cycle",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultLayer {
    Process,
    RosRuntime,
    Middleware,
    Controller,
    Driver,
    Firmware,
    Actuator,
    Observer,
    Persistence,
    Mission,
    NativeAuthority,
}

impl FaultLayer {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Process => "process",
            Self::RosRuntime => "ros_runtime",
            Self::Middleware => "middleware",
            Self::Controller => "controller",
            Self::Driver => "driver",
            Self::Firmware => "firmware",
            Self::Actuator => "actuator",
            Self::Observer => "observer",
            Self::Persistence => "persistence",
            Self::Mission => "mission",
            Self::NativeAuthority => "native_authority",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultCutPoint {
    AfterEffectReservation,
    AfterNativeAcceptanceBeforeEffect,
    AfterPossibleEffectBeforePersistenceAck,
    AfterPersistenceAckBeforeObservation,
    AfterObservationBeforeControllerFence,
    AfterControllerFenceBeforeDownstreamFence,
    AfterPermitConsumptionBeforeAck,
    DuringRestartRecovery,
    AfterSecondAttemptPossibleEffect,
}

impl FaultCutPoint {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AfterEffectReservation => "after_effect_reservation",
            Self::AfterNativeAcceptanceBeforeEffect => "after_native_acceptance_before_effect",
            Self::AfterPossibleEffectBeforePersistenceAck => {
                "after_possible_effect_before_persistence_ack"
            }
            Self::AfterPersistenceAckBeforeObservation => {
                "after_persistence_ack_before_observation"
            }
            Self::AfterObservationBeforeControllerFence => {
                "after_observation_before_controller_fence"
            }
            Self::AfterControllerFenceBeforeDownstreamFence => {
                "after_controller_fence_before_downstream_fence"
            }
            Self::AfterPermitConsumptionBeforeAck => "after_permit_consumption_before_ack",
            Self::DuringRestartRecovery => "during_restart_recovery",
            Self::AfterSecondAttemptPossibleEffect => "after_second_attempt_possible_effect",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultKind {
    ProcessCrash,
    ControllerRestart,
    MessageDrop,
    MessageDelay,
    MessageDuplicate,
    MessageReorder,
    QueueRetention,
    StaleEpoch,
    PowerLoss,
    ClockReset,
    ObserverLoss,
    CommonCauseFailure,
    ArtifactReplay,
    PersistenceAmbiguity,
    PartialEffect,
    PermitReplay,
}

impl FaultKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProcessCrash => "process_crash",
            Self::ControllerRestart => "controller_restart",
            Self::MessageDrop => "message_drop",
            Self::MessageDelay => "message_delay",
            Self::MessageDuplicate => "message_duplicate",
            Self::MessageReorder => "message_reorder",
            Self::QueueRetention => "queue_retention",
            Self::StaleEpoch => "stale_epoch",
            Self::PowerLoss => "power_loss",
            Self::ClockReset => "clock_reset",
            Self::ObserverLoss => "observer_loss",
            Self::CommonCauseFailure => "common_cause_failure",
            Self::ArtifactReplay => "artifact_replay",
            Self::PersistenceAmbiguity => "persistence_ambiguity",
            Self::PartialEffect => "partial_effect",
            Self::PermitReplay => "permit_replay",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformProfile {
    pub platform_id: String,
    pub profile_sha256: String,
    pub environment_artifact_sha256: String,
    pub simulator_revision: String,
    pub fault_injector_id: String,
    pub fault_injector_revision: String,
    pub fault_injector_sha256: String,
    pub fault_injector_fault_domain_id: String,
    pub native_adapter_id: String,
    pub native_adapter_revision: String,
    pub runtime_monitor_revision: String,
    pub ros_distribution: String,
    pub middleware_revision: String,
    pub controller_revision: String,
    pub driver_revision: String,
    pub firmware_revision: String,
    pub calibration_revision: String,
    pub observer_stack_revision: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResetProfile {
    pub kind: ResetKind,
    pub revision: String,
    pub profile_sha256: String,
    pub expected_initial_state_sha256: String,
    pub post_reset_observation_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaultInjection {
    pub sequence: u32,
    pub scheduled_simulation_step: u64,
    pub fault_id: String,
    pub cut_point: FaultCutPoint,
    pub target_layer: FaultLayer,
    pub target_component_id: String,
    pub target_revision: String,
    pub target_profile_sha256: String,
    pub fault_domain_id: String,
    pub kind: FaultKind,
    pub parameters_sha256: String,
    pub expected_delivery_signature_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignScenario {
    pub scenario_id: String,
    pub pair_id: String,
    pub arm: BenchmarkArm,
    pub control_class: ControlClass,
    pub task_id: String,
    pub policy_id: String,
    pub policy_sha256: String,
    pub scene_sha256: String,
    pub action_prefix_sha256: String,
    pub seed: u64,
    pub logical_effect_id: String,
    pub faults: Vec<FaultInjection>,
    pub expected_disposition: ExpectedDisposition,
    pub expected_reason_code: String,
    pub invariant_id: String,
    /// Required for mutation/negative controls; identifies the exact
    /// property, violation class and witness event that replay must preserve.
    pub failure_signature_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaultCampaignManifest {
    pub version: String,
    pub campaign_id: String,
    pub fault_model_revision: String,
    pub fault_model_sha256: String,
    pub platform: PlatformProfile,
    pub reset: ResetProfile,
    pub required_layers: Vec<FaultLayer>,
    pub required_cut_points: Vec<FaultCutPoint>,
    pub scenarios: Vec<CampaignScenario>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampaignError {
    InvalidHeader,
    InvalidPlatformProfile,
    InvalidResetProfile,
    EmptyRequiredCoverage,
    DuplicateRequiredCoverage,
    EmptyCampaign,
    TooManyScenarios,
    InvalidScenario,
    DuplicateScenarioIdentity,
    DuplicateFaultIdentity,
    FaultSequenceDiscontinuity,
    TooManyFaults,
    InvalidFaultInjection,
    PositiveControlHasFaults,
    NegativeControlMissingFaults,
    InvalidExpectedDisposition,
    MissingFailureSignature,
    InvalidFailureSignature,
    InvalidDigest,
    InvalidPair,
    IncomparablePair,
    MissingFullRrvrPositiveControl,
    MissingFullRrvrNegativeControl,
    MissingRequiredLayer,
    MissingRequiredCutPoint,
    DuplicateOperationalIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignSummary {
    pub scenario_count: usize,
    pub pair_count: usize,
    pub positive_control_count: usize,
    pub negative_control_count: usize,
    pub mutation_control_count: usize,
    pub fault_injection_count: usize,
    pub covered_layers: BTreeSet<FaultLayer>,
    pub covered_cut_points: BTreeSet<FaultCutPoint>,
    pub full_rrvr_positive_count: usize,
    pub full_rrvr_negative_count: usize,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn require_sha256(value: &str) -> Result<(), CampaignError> {
    if valid_sha256(value) {
        Ok(())
    } else {
        Err(CampaignError::InvalidDigest)
    }
}

fn validate_platform(platform: &PlatformProfile) -> Result<(), CampaignError> {
    for value in [
        platform.platform_id.as_str(),
        platform.simulator_revision.as_str(),
        platform.fault_injector_id.as_str(),
        platform.fault_injector_revision.as_str(),
        platform.fault_injector_fault_domain_id.as_str(),
        platform.native_adapter_id.as_str(),
        platform.native_adapter_revision.as_str(),
        platform.runtime_monitor_revision.as_str(),
        platform.ros_distribution.as_str(),
        platform.simulation_interfaces_revision.as_str(),
        platform.middleware_revision.as_str(),
        platform.controller_revision.as_str(),
        platform.driver_revision.as_str(),
        platform.firmware_revision.as_str(),
        platform.calibration_revision.as_str(),
        platform.observer_stack_revision.as_str(),
    ] {
        if !nonempty(value) {
            return Err(CampaignError::InvalidPlatformProfile);
        }
    }
    require_sha256(&platform.profile_sha256)?;
    require_sha256(&platform.environment_artifact_sha256)?;
    require_sha256(&platform.scene_broadcaster_sha256)?;
    require_sha256(&platform.fault_injector_sha256)?;
    Ok(())
}

fn validate_reset(reset: &ResetProfile) -> Result<(), CampaignError> {
    if !nonempty(&reset.revision) || !nonempty(&reset.post_reset_observation_id) {
        return Err(CampaignError::InvalidResetProfile);
    }
    require_sha256(&reset.profile_sha256)?;
    require_sha256(&reset.expected_initial_state_sha256)?;
    Ok(())
}

fn validate_scenario(scenario: &CampaignScenario) -> Result<(), CampaignError> {
    for value in [
        scenario.scenario_id.as_str(),
        scenario.pair_id.as_str(),
        scenario.task_id.as_str(),
        scenario.policy_id.as_str(),
        scenario.logical_effect_id.as_str(),
        scenario.expected_reason_code.as_str(),
        scenario.invariant_id.as_str(),
    ] {
        if !nonempty(value) {
            return Err(CampaignError::InvalidScenario);
        }
    }

    for digest in [
        scenario.policy_sha256.as_str(),
        scenario.scene_sha256.as_str(),
        scenario.action_prefix_sha256.as_str(),
    ] {
        require_sha256(digest)?;
    }

    match scenario.control_class {
        ControlClass::Positive => {
            if !scenario.faults.is_empty() {
                return Err(CampaignError::PositiveControlHasFaults);
            }
            if scenario.arm == BenchmarkArm::FullRrvr
                && scenario.expected_disposition != ExpectedDisposition::Admit
            {
                return Err(CampaignError::InvalidExpectedDisposition);
            }
            if scenario.failure_signature_sha256.is_some() {
                return Err(CampaignError::InvalidScenario);
            }
        }
        ControlClass::Negative | ControlClass::Mutation => {
            if scenario.faults.is_empty() {
                return Err(CampaignError::NegativeControlMissingFaults);
            }
            if scenario.arm == BenchmarkArm::FullRrvr
                && !matches!(
                    scenario.expected_disposition,
                    ExpectedDisposition::Hold | ExpectedDisposition::Block
                )
            {
                return Err(CampaignError::InvalidExpectedDisposition);
            }
            let signature = scenario
                .failure_signature_sha256
                .as_deref()
                .ok_or(CampaignError::MissingFailureSignature)?;
            require_sha256(signature).map_err(|_| CampaignError::InvalidFailureSignature)?;
        }
    }

    if scenario.arm != BenchmarkArm::FullRrvr
        && scenario.expected_disposition != ExpectedDisposition::Measure
    {
        return Err(CampaignError::InvalidExpectedDisposition);
    }

    if scenario.faults.len() > MAX_FAULTS_PER_SCENARIO {
        return Err(CampaignError::TooManyFaults);
    }
    let mut fault_ids = BTreeSet::new();
    let mut covered_sequence = 0u32;
    let mut previous_scheduled_step = None;
    for fault in &scenario.faults {
        if !nonempty(&fault.fault_id)
            || !nonempty(&fault.target_component_id)
            || !nonempty(&fault.target_revision)
            || !nonempty(&fault.fault_domain_id)
        {
            return Err(CampaignError::InvalidFaultInjection);
        }
        if !fault_ids.insert(fault.fault_id.clone()) {
            return Err(CampaignError::DuplicateFaultIdentity);
        }
        covered_sequence += 1;
        if fault.sequence != covered_sequence
            || previous_scheduled_step
                .is_some_and(|previous| fault.scheduled_simulation_step < previous)
        {
            return Err(CampaignError::FaultSequenceDiscontinuity);
        }
        previous_scheduled_step = Some(fault.scheduled_simulation_step);
        require_sha256(&fault.parameters_sha256)?;
        require_sha256(&fault.target_profile_sha256)?;
        require_sha256(&fault.expected_delivery_signature_sha256)?;
    }
    Ok(())
}

fn pair_equivalent(left: &CampaignScenario, right: &CampaignScenario) -> bool {
    left.control_class == right.control_class
        && left.task_id == right.task_id
        && left.policy_id == right.policy_id
        && left.policy_sha256 == right.policy_sha256
        && left.scene_sha256 == right.scene_sha256
        && left.action_prefix_sha256 == right.action_prefix_sha256
        && left.seed == right.seed
        && left.logical_effect_id == right.logical_effect_id
        && left.faults == right.faults
}

impl FaultCampaignManifest {
    /// Validate a frozen campaign and summarize its declared coverage.
    ///
    /// This is a manifest/experiment-plan check only. It never executes the
    /// injector or upgrades simulator evidence into HIL/physical evidence.
    pub fn validate(&self) -> Result<CampaignSummary, CampaignError> {
        if self.version != CAMPAIGN_MANIFEST_VERSION
            || !nonempty(&self.campaign_id)
            || !nonempty(&self.fault_model_revision)
        {
            return Err(CampaignError::InvalidHeader);
        }
        require_sha256(&self.fault_model_sha256)?;
        validate_platform(&self.platform)?;
        validate_reset(&self.reset)?;

        if self.required_layers.is_empty() || self.required_cut_points.is_empty() {
            return Err(CampaignError::EmptyRequiredCoverage);
        }
        let required_layers: BTreeSet<FaultLayer> = self.required_layers.iter().copied().collect();
        let required_cuts: BTreeSet<FaultCutPoint> =
            self.required_cut_points.iter().copied().collect();
        if required_layers.len() != self.required_layers.len()
            || required_cuts.len() != self.required_cut_points.len()
        {
            return Err(CampaignError::DuplicateRequiredCoverage);
        }

        if self.scenarios.len() < 2 {
            return Err(CampaignError::EmptyCampaign);
        }
        if self.scenarios.len() > MAX_SCENARIOS {
            return Err(CampaignError::TooManyScenarios);
        }

        let mut scenario_ids = BTreeSet::new();
        let mut pairs: BTreeMap<&str, Vec<&CampaignScenario>> = BTreeMap::new();
        let mut covered_layers = BTreeSet::new();
        let mut covered_cut_points = BTreeSet::new();
        let mut fault_injection_count = 0usize;
        let mut positive_control_count = 0usize;
        let mut negative_control_count = 0usize;
        let mut mutation_control_count = 0usize;
        let mut full_rrvr_positive_count = 0usize;
        let mut full_rrvr_negative_count = 0usize;

        for scenario in &self.scenarios {
            validate_scenario(scenario)?;
            if !scenario_ids.insert(scenario.scenario_id.as_str()) {
                return Err(CampaignError::DuplicateScenarioIdentity);
            }
            pairs
                .entry(scenario.pair_id.as_str())
                .or_default()
                .push(scenario);

            match scenario.control_class {
                ControlClass::Positive => positive_control_count += 1,
                ControlClass::Negative => negative_control_count += 1,
                ControlClass::Mutation => mutation_control_count += 1,
            }
            if scenario.arm == BenchmarkArm::FullRrvr {
                match scenario.control_class {
                    ControlClass::Positive => full_rrvr_positive_count += 1,
                    ControlClass::Negative | ControlClass::Mutation => {
                        full_rrvr_negative_count += 1;
                    }
                }
            }

            fault_injection_count += scenario.faults.len();
            for fault in &scenario.faults {
                covered_layers.insert(fault.target_layer);
                covered_cut_points.insert(fault.cut_point);
            }
        }

        for members in pairs.values() {
            if members.len() < 2 {
                return Err(CampaignError::InvalidPair);
            }
            let Some(reference) = members.first() else {
                return Err(CampaignError::InvalidPair);
            };
            let mut arms = BTreeSet::new();
            for scenario in members {
                if !pair_equivalent(reference, scenario) || !arms.insert(scenario.arm) {
                    return Err(CampaignError::IncomparablePair);
                }
            }
            if !arms.contains(&BenchmarkArm::FullRrvr)
                || !arms.iter().any(|arm| *arm != BenchmarkArm::FullRrvr)
            {
                return Err(CampaignError::InvalidPair);
            }
        }

        if full_rrvr_positive_count == 0 {
            return Err(CampaignError::MissingFullRrvrPositiveControl);
        }
        if full_rrvr_negative_count == 0 {
            return Err(CampaignError::MissingFullRrvrNegativeControl);
        }
        if !required_layers.is_subset(&covered_layers) {
            return Err(CampaignError::MissingRequiredLayer);
        }
        if !required_cuts.is_subset(&covered_cut_points) {
            return Err(CampaignError::MissingRequiredCutPoint);
        }

        Ok(CampaignSummary {
            scenario_count: self.scenarios.len(),
            pair_count: pairs.len(),
            positive_control_count,
            negative_control_count,
            mutation_control_count,
            fault_injection_count,
            covered_layers,
            covered_cut_points,
            full_rrvr_positive_count,
            full_rrvr_negative_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn valid_manifest() -> FaultCampaignManifest {
        serde_json::from_str(include_str!("../fixtures/fault_campaign_paired_v0_1.json"))
            .expect("known-answer campaign fixture deserializes")
    }

    #[test]
    fn accepts_paired_positive_and_negative_campaign_arms() {
        let summary = valid_manifest().validate().unwrap();
        assert_eq!(summary.scenario_count, 4);
        assert_eq!(summary.pair_count, 2);
        assert_eq!(summary.positive_control_count, 2);
        assert_eq!(summary.negative_control_count, 2);
        assert_eq!(summary.full_rrvr_positive_count, 1);
        assert_eq!(summary.full_rrvr_negative_count, 1);
        assert_eq!(summary.fault_injection_count, 8);
        assert_eq!(summary.covered_layers.len(), 4);
        assert_eq!(summary.covered_cut_points.len(), 4);
    }

    #[test]
    fn rejects_a_pair_that_changes_the_seed_or_fault_schedule() {
        let mut manifest = valid_manifest();
        manifest.scenarios[1].seed += 1;
        assert_eq!(manifest.validate(), Err(CampaignError::IncomparablePair));

        let mut changed_fault = valid_manifest();
        changed_fault.scenarios[3].faults[0].kind = FaultKind::PowerLoss;
        assert_eq!(
            changed_fault.validate(),
            Err(CampaignError::IncomparablePair)
        );
    }

    #[test]
    fn rejects_full_rrvr_positive_control_that_is_not_admitted() {
        let mut manifest = valid_manifest();
        manifest.scenarios[0].expected_disposition = ExpectedDisposition::Hold;
        assert_eq!(
            manifest.validate(),
            Err(CampaignError::InvalidExpectedDisposition)
        );
    }

    #[test]
    fn rejects_full_rrvr_negative_control_that_is_admitted() {
        let mut manifest = valid_manifest();
        manifest.scenarios[2].expected_disposition = ExpectedDisposition::Admit;
        assert_eq!(
            manifest.validate(),
            Err(CampaignError::InvalidExpectedDisposition)
        );
    }

    #[test]
    fn rejects_missing_fault_cut_coverage() {
        let mut manifest = valid_manifest();
        manifest
            .required_cut_points
            .push(FaultCutPoint::AfterEffectReservation);
        assert_eq!(
            manifest.validate(),
            Err(CampaignError::MissingRequiredCutPoint)
        );
    }

    #[test]
    fn rejects_missing_fault_layer_coverage() {
        let mut manifest = valid_manifest();
        manifest.required_layers.push(FaultLayer::NativeAuthority);
        assert_eq!(
            manifest.validate(),
            Err(CampaignError::MissingRequiredLayer)
        );
    }

    #[test]
    fn rejects_invalid_reset_or_unpinned_platform_profile() {
        let mut reset = valid_manifest();
        reset.reset.post_reset_observation_id.clear();
        assert_eq!(reset.validate(), Err(CampaignError::InvalidResetProfile));

        let mut platform = valid_manifest();
        platform.platform.environment_artifact_sha256 = "latest".into();
        assert_eq!(platform.validate(), Err(CampaignError::InvalidDigest));
    }

    #[test]
    fn rejects_unpinned_fault_injector_or_native_adapter_identity() {
        let mut injector = valid_manifest();
        injector.platform.fault_injector_sha256 = "unknown".into();
        assert_eq!(injector.validate(), Err(CampaignError::InvalidDigest));

        let mut native = valid_manifest();
        native.platform.native_adapter_revision.clear();
        assert_eq!(
            native.validate(),
            Err(CampaignError::InvalidPlatformProfile)
        );
    }

    #[test]
    fn rejects_non_monotonic_scheduled_simulation_steps() {
        let mut manifest = valid_manifest();
        manifest.scenarios[2].faults[1].scheduled_simulation_step = 50;
        assert_eq!(
            manifest.validate(),
            Err(CampaignError::FaultSequenceDiscontinuity)
        );
    }

    #[test]
    fn rejects_duplicate_scenario_and_fault_identities() {
        let mut scenario = valid_manifest();
        let duplicate_scenario_id = scenario.scenarios[0].scenario_id.clone();
        scenario.scenarios[1].scenario_id = duplicate_scenario_id;
        assert_eq!(
            scenario.validate(),
            Err(CampaignError::DuplicateScenarioIdentity)
        );

        let mut fault = valid_manifest();
        let duplicate_fault_id = fault.scenarios[2].faults[0].fault_id.clone();
        fault.scenarios[2].faults[1].fault_id = duplicate_fault_id;
        assert_eq!(fault.validate(), Err(CampaignError::DuplicateFaultIdentity));
    }

    #[test]
    fn rejects_unordered_fault_schedule_and_bad_failure_signature() {
        let mut order = valid_manifest();
        order.scenarios[2].faults[1].sequence = 1;
        assert_eq!(
            order.validate(),
            Err(CampaignError::FaultSequenceDiscontinuity)
        );

        let mut signature = valid_manifest();
        signature.scenarios[2].failure_signature_sha256 = Some("not-a-digest".into());
        assert_eq!(
            signature.validate(),
            Err(CampaignError::InvalidFailureSignature)
        );
    }

    #[test]
    fn strict_deserializer_rejects_unknown_fields() {
        let mut value: Value =
            serde_json::from_str(include_str!("../fixtures/fault_campaign_paired_v0_1.json"))
                .unwrap();
        value["unverified_platform_override"] = serde_json::json!(true);
        assert!(serde_json::from_value::<FaultCampaignManifest>(value).is_err());
    }
}
