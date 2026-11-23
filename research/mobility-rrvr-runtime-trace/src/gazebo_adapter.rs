// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Gazebo ROS 2 Simulation Interfaces adapter core.
//!
//! This module contains transport-neutral orchestration and preflight rules.
//! A concrete ROS 2 client is intentionally separate: no network call, world
//! mutation, fault dispatch or actuation occurs in this crate module.

use crate::campaign_plan::{CompiledCampaignPlan, CompiledScenario};
use crate::fault_campaign::FaultInjection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const GAZEBO_ADAPTER_CONTRACT_VERSION: &str = "0.1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GazeboCapability {
    GetSimulatorFeatures,
    ResetSimulation,
    StepSimulation,
    SimulateSteps,
    GetSimulationState,
    SetSimulationState,
    GetEntities,
    GetEntityState,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SteppingMethod {
    StepSimulationService,
    SimulateStepsAction,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationState {
    Playing,
    Paused,
    Stopped,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GazeboCapabilityProfile {
    pub version: String,
    pub simulator_id: String,
    pub simulator_revision: String,
    pub ros_distribution: String,
    pub simulation_interfaces_revision: String,
    pub world_name: String,
    pub world_profile_sha256: String,
    pub reset_profile_sha256: String,
    pub scene_broadcaster_id: String,
    pub scene_broadcaster_revision: String,
    pub scene_broadcaster_sha256: String,
    pub scene_broadcaster_loaded: bool,
    pub capabilities: BTreeSet<GazeboCapability>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GazeboWorldSnapshot {
    pub simulator_id: String,
    pub simulator_revision: String,
    pub world_name: String,
    pub world_epoch: String,
    pub clock_id: String,
    pub simulation_step: u64,
    pub simulation_time_ns: u64,
    pub state: SimulationState,
    pub state_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GazeboTargetBinding {
    pub logical_component_id: String,
    pub target_id: String,
    pub target_revision: String,
    pub target_profile_sha256: String,
    pub fault_domain_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GazeboAdapterError {
    InvalidCapabilityProfile,
    PlatformRevisionMismatch,
    RosDistributionMismatch,
    MissingRequiredCapability,
    MissingSteppingMethod,
    SceneBroadcasterUnavailable,
    WrongWorld,
    WrongSimulator,
    WrongSimulatorRevision,
    WrongWorldEpoch,
    InvalidClock,
    WorldStateDigestMismatch,
    ResetProfileMismatch,
    UnsupportedResetKind,
    ResetDidNotAdvanceEpoch,
    ResetDidNotRestoreInitialState,
    SimulatorNotPaused,
    SimulatorStopped,
    SimulationStepOvershoot,
    SimulationStepNotReached,
    InvalidTargetBinding,
    TargetRevisionMismatch,
    TargetProfileMismatch,
    TargetFaultDomainMismatch,
    InvalidIdempotencyContext,
    TransportFailure,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl GazeboCapabilityProfile {
    /// Check the exact simulator build and required interface features.
    ///
    /// Gazebo's SceneBroadcaster is mandatory because Simulation Interfaces
    /// depend on it for synchronized world-state updates and reset detection.
    pub fn validate_for_plan(
        &self,
        plan: &CompiledCampaignPlan,
    ) -> Result<SteppingMethod, GazeboAdapterError> {
        plan.verify_integrity()
            .map_err(|_| GazeboAdapterError::InvalidCapabilityProfile)?;
        if self.version != GAZEBO_ADAPTER_CONTRACT_VERSION
            || !nonempty(&self.simulator_id)
            || !nonempty(&self.simulator_revision)
            || !nonempty(&self.ros_distribution)
            || !nonempty(&self.simulation_interfaces_revision)
            || !nonempty(&self.world_name)
            || !nonempty(&self.scene_broadcaster_id)
            || !nonempty(&self.scene_broadcaster_revision)
            || !valid_sha256(&self.scene_broadcaster_sha256)
            || !valid_sha256(&self.world_profile_sha256)
            || !valid_sha256(&self.reset_profile_sha256)
        {
            return Err(GazeboAdapterError::InvalidCapabilityProfile);
        }
        if self.simulator_id != plan.platform.platform_id {
            return Err(GazeboAdapterError::WrongSimulator);
        }
        if self.simulator_revision != plan.platform.simulator_revision {
            return Err(GazeboAdapterError::PlatformRevisionMismatch);
        }
        if self.ros_distribution != plan.platform.ros_distribution {
            return Err(GazeboAdapterError::RosDistributionMismatch);
        }
        if self.simulation_interfaces_revision
            != plan.platform.simulation_interfaces_revision
        {
            return Err(GazeboAdapterError::PlatformRevisionMismatch);
        }
        if self.world_profile_sha256 != plan.platform.environment_artifact_sha256 {
            return Err(GazeboAdapterError::InvalidCapabilityProfile);
        }
        if self.reset_profile_sha256 != plan.reset.profile_sha256 {
            return Err(GazeboAdapterError::ResetProfileMismatch);
        }
        if !self.scene_broadcaster_loaded
            || self.scene_broadcaster_id != plan.platform.scene_broadcaster_id
            || self.scene_broadcaster_revision != plan.platform.scene_broadcaster_revision
            || self.scene_broadcaster_sha256 != plan.platform.scene_broadcaster_sha256
        {
            return Err(GazeboAdapterError::SceneBroadcasterUnavailable);
        }

        for required in [
            GazeboCapability::GetSimulatorFeatures,
            GazeboCapability::ResetSimulation,
            GazeboCapability::GetSimulationState,
            GazeboCapability::SetSimulationState,
            GazeboCapability::GetEntities,
            GazeboCapability::GetEntityState,
        ] {
            if !self.capabilities.contains(&required) {
                return Err(GazeboAdapterError::MissingRequiredCapability);
            }
        }

        if self.capabilities.contains(&GazeboCapability::StepSimulation) {
            Ok(SteppingMethod::StepSimulationService)
        } else if self.capabilities.contains(&GazeboCapability::SimulateSteps) {
            Ok(SteppingMethod::SimulateStepsAction)
        } else {
            Err(GazeboAdapterError::MissingSteppingMethod)
        }
    }
}

impl GazeboWorldSnapshot {
    pub fn validate_against_profile(
        &self,
        profile: &GazeboCapabilityProfile,
    ) -> Result<(), GazeboAdapterError> {
        if self.simulator_id != profile.simulator_id {
            return Err(GazeboAdapterError::WrongSimulator);
        }
        if self.simulator_revision != profile.simulator_revision {
            return Err(GazeboAdapterError::WrongSimulatorRevision);
        }
        if self.world_name != profile.world_name {
            return Err(GazeboAdapterError::WrongWorld);
        }
        if !nonempty(&self.world_epoch) || !nonempty(&self.clock_id) {
            return Err(GazeboAdapterError::InvalidClock);
        }
        if !valid_sha256(&self.state_sha256) {
            return Err(GazeboAdapterError::WorldStateDigestMismatch);
        }
        Ok(())
    }

    pub fn pause_required(&self) -> Result<bool, GazeboAdapterError> {
        match self.state {
            SimulationState::Paused => Ok(false),
            SimulationState::Playing => Ok(true),
            SimulationState::Stopped => Err(GazeboAdapterError::SimulatorStopped),
            SimulationState::Unknown => Err(GazeboAdapterError::SimulatorNotPaused),
        }
    }

    /// Compute exact discrete stepping. Wall-clock sleeps are never a fallback.
    pub fn steps_to(&self, scheduled_step: u64) -> Result<u64, GazeboAdapterError> {
        if self.state != SimulationState::Paused {
            return Err(GazeboAdapterError::SimulatorNotPaused);
        }
        if self.simulation_step > scheduled_step {
            return Err(GazeboAdapterError::SimulationStepOvershoot);
        }
        Ok(scheduled_step - self.simulation_step)
    }

    /// Validate that a reset produced the expected pinned snapshot.
    pub fn validate_reset(
        &self,
        profile: &GazeboCapabilityProfile,
        plan: &CompiledCampaignPlan,
        prior_world_epoch: &str,
    ) -> Result<(), GazeboAdapterError> {
        self.validate_against_profile(profile)?;
        if plan.reset.kind != "simulator_snapshot_restore" {
            return Err(GazeboAdapterError::UnsupportedResetKind);
        }
        if self.world_epoch == prior_world_epoch {
            return Err(GazeboAdapterError::ResetDidNotAdvanceEpoch);
        }
        if self.simulation_step != 0
            || self.state_sha256 != plan.reset.expected_initial_state_sha256
            || self.state != SimulationState::Paused
        {
            return Err(GazeboAdapterError::ResetDidNotRestoreInitialState);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GazeboAdapterSession {
    pub profile: GazeboCapabilityProfile,
    pub stepping_method: SteppingMethod,
    pub world_epoch: String,
    pub clock_id: String,
}

/// Discover and pin the simulator before any reset, step or target mutation.
pub fn preflight_transport<T: GazeboSimulationTransport>(
    transport: &mut T,
    plan: &CompiledCampaignPlan,
) -> Result<GazeboAdapterSession, GazeboAdapterError> {
    let profile = transport
        .get_simulator_features()
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    let stepping_method = profile.validate_for_plan(plan)?;
    let mut snapshot = transport
        .get_world_state(&profile.world_name)
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    snapshot.validate_against_profile(&profile)?;

    if snapshot.pause_required()? {
        transport
            .pause_simulation(&profile.world_name)
            .map_err(|_| GazeboAdapterError::TransportFailure)?;
        snapshot = transport
            .get_world_state(&profile.world_name)
            .map_err(|_| GazeboAdapterError::TransportFailure)?;
        snapshot.validate_against_profile(&profile)?;
    }
    if snapshot.state != SimulationState::Paused {
        return Err(GazeboAdapterError::SimulatorNotPaused);
    }

    Ok(GazeboAdapterSession {
        profile,
        stepping_method,
        world_epoch: snapshot.world_epoch,
        clock_id: snapshot.clock_id,
    })
}

/// Reset once and verify the exact initial snapshot before scheduling faults.
pub fn reset_and_verify<T: GazeboSimulationTransport>(
    transport: &mut T,
    plan: &CompiledCampaignPlan,
    session: &mut GazeboAdapterSession,
) -> Result<GazeboWorldSnapshot, GazeboAdapterError> {
    let before = transport
        .get_world_state(&session.profile.world_name)
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    before.validate_against_profile(&session.profile)?;
    if before.world_epoch != session.world_epoch {
        return Err(GazeboAdapterError::WrongWorldEpoch);
    }
    if before.clock_id != session.clock_id {
        return Err(GazeboAdapterError::InvalidClock);
    }
    if before.state != SimulationState::Paused {
        return Err(GazeboAdapterError::SimulatorNotPaused);
    }

    transport
        .reset_simulation(&session.profile.world_name)
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    let after = transport
        .get_world_state(&session.profile.world_name)
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    after.validate_reset(&session.profile, plan, &before.world_epoch)?;
    if after.clock_id != before.clock_id {
        return Err(GazeboAdapterError::InvalidClock);
    }

    session.world_epoch = after.world_epoch.clone();
    session.clock_id = after.clock_id.clone();
    Ok(after)
}

/// Advance to the exact planned iteration using a pinned discrete-step API.
/// Any overshoot, epoch change or unexpected state is a hard failure.
pub fn advance_to_fault_cut<T: GazeboSimulationTransport>(
    transport: &mut T,
    session: &GazeboAdapterSession,
    fault: &FaultInjection,
) -> Result<GazeboWorldSnapshot, GazeboAdapterError> {
    let before = transport
        .get_world_state(&session.profile.world_name)
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    before.validate_against_profile(&session.profile)?;
    if before.world_epoch != session.world_epoch {
        return Err(GazeboAdapterError::WrongWorldEpoch);
    }
    if before.clock_id != session.clock_id {
        return Err(GazeboAdapterError::InvalidClock);
    }
    let steps = before.steps_to(fault.scheduled_simulation_step)?;
    if steps > 0 {
        transport
            .step_simulation(
                &session.profile.world_name,
                steps,
                session.stepping_method,
            )
            .map_err(|_| GazeboAdapterError::TransportFailure)?;
    }
    let after = transport
        .get_world_state(&session.profile.world_name)
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    after.validate_against_profile(&session.profile)?;
    if after.world_epoch != session.world_epoch {
        return Err(GazeboAdapterError::WrongWorldEpoch);
    }
    if after.clock_id != session.clock_id {
        return Err(GazeboAdapterError::InvalidClock);
    }
    if after.state != SimulationState::Paused {
        return Err(GazeboAdapterError::SimulatorNotPaused);
    }
    if after.simulation_step > fault.scheduled_simulation_step {
        return Err(GazeboAdapterError::SimulationStepOvershoot);
    }
    if after.simulation_step != fault.scheduled_simulation_step {
        return Err(GazeboAdapterError::SimulationStepNotReached);
    }
    Ok(after)
}

/// Resolve a logical target only through the pinned simulator/world profile.
pub fn resolve_fault_target<T: GazeboSimulationTransport>(
    transport: &mut T,
    session: &GazeboAdapterSession,
    fault: &FaultInjection,
) -> Result<GazeboTargetBinding, GazeboAdapterError> {
    let binding = transport
        .resolve_target(&session.profile.world_name, &fault.target_component_id)
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    validate_target_binding(&binding, fault)?;
    Ok(binding)
}

/// Transport for the standard Gazebo ROS 2 Simulation Interfaces.
///
/// The concrete implementation must map to the exact interface package and
/// simulator build recorded by the capability profile. This trait does not
/// choose a transport fallback or downgrade an unavailable interface.
pub trait GazeboSimulationTransport {
    type Error: std::fmt::Display;

    fn get_simulator_features(&mut self) -> Result<GazeboCapabilityProfile, Self::Error>;
    fn get_world_state(&mut self, world_name: &str) -> Result<GazeboWorldSnapshot, Self::Error>;
    fn pause_simulation(&mut self, world_name: &str) -> Result<(), Self::Error>;
    fn reset_simulation(&mut self, world_name: &str) -> Result<(), Self::Error>;
    fn step_simulation(
        &mut self,
        world_name: &str,
        steps: u64,
        method: SteppingMethod,
    ) -> Result<(), Self::Error>;
    fn resolve_target(
        &mut self,
        world_name: &str,
        logical_component_id: &str,
    ) -> Result<GazeboTargetBinding, Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GazeboDispatchRequest {
    pub plan_sha256: String,
    pub campaign_id: String,
    pub pair_id: String,
    pub scenario_id: String,
    pub fault_id: String,
    pub fault_sequence: u32,
    pub target_id: String,
    pub fault_domain_id: String,
    pub scheduled_simulation_step: u64,
    pub parameters_sha256: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GazeboDispatchOutcome {
    Accepted { dispatch_id: String },
    Rejected { dispatch_id: String, reason_code: String },
    Unknown { dispatch_id: Option<String> },
}

/// The injector transport has an exactly-once dispatch API plus a separate
/// idempotency-ledger lookup. The adapter must call dispatch_once at most once
/// for an idempotency key; a transport error maps to Unknown.
pub trait GazeboFaultInjectorTransport {
    type Error: std::fmt::Display;

    fn dispatch_once(
        &mut self,
        request: &GazeboDispatchRequest,
    ) -> Result<GazeboDispatchOutcome, Self::Error>;

    fn lookup_dispatch(
        &mut self,
        idempotency_key: &str,
    ) -> Result<DispatchLedgerResult, Self::Error>;
}

/// Issue one dispatch attempt. An error is not retried and maps to Unknown.
pub fn dispatch_fault_once<T: GazeboFaultInjectorTransport>(
    injector: &mut T,
    request: &GazeboDispatchRequest,
) -> GazeboDispatchOutcome {
    injector
        .dispatch_once(request)
        .unwrap_or(GazeboDispatchOutcome::Unknown { dispatch_id: None })
}

/// Reconcile by idempotency key only. This function never calls dispatch_once.
pub fn reconcile_fault_dispatch<T: GazeboFaultInjectorTransport>(
    injector: &mut T,
    idempotency_key: &str,
) -> ReconciliationDecision {
    match injector.lookup_dispatch(idempotency_key) {
        Ok(lookup) => reconcile_dispatch_lookup(lookup),
        Err(_) => ReconciliationDecision::Unresolved,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedFaultSignature {
    pub observer_id: String,
    pub observer_revision: String,
    pub observer_fault_domain_id: String,
    pub signature_sha256: String,
    pub world_epoch: String,
    pub simulation_step: u64,
}

/// The observer must be a separate subsystem from the injector. The concrete
/// implementation supplies raw evidence to the execution-evidence bundle.
pub trait GazeboIndependentObserver {
    type Error: std::fmt::Display;

    fn observe_fault(
        &mut self,
        target_id: &str,
        fault_id: &str,
        expected_signature_sha256: &str,
        world_epoch: &str,
        simulation_step: u64,
    ) -> Result<ObservedFaultSignature, Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchLedgerResult {
    Accepted,
    Rejected,
    NoDispatchProven,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconciliationDecision {
    ExistingDispatchAccepted,
    ExistingDispatchRejected,
    NoDispatchProven,
    Unresolved,
}

/// A lost dispatch response maps to Unknown. It never invokes dispatch again.
pub fn reconcile_dispatch_lookup(
    lookup: DispatchLedgerResult,
) -> ReconciliationDecision {
    match lookup {
        DispatchLedgerResult::Accepted => ReconciliationDecision::ExistingDispatchAccepted,
        DispatchLedgerResult::Rejected => ReconciliationDecision::ExistingDispatchRejected,
        DispatchLedgerResult::NoDispatchProven => ReconciliationDecision::NoDispatchProven,
        DispatchLedgerResult::Unknown => ReconciliationDecision::Unresolved,
    }
}

/// A stable idempotency key binds one dispatch to one plan/run/event.
pub fn dispatch_idempotency_key(
    plan: &CompiledCampaignPlan,
    scenario: &CompiledScenario,
    fault: &FaultInjection,
    run_id: &str,
    run_epoch: &str,
) -> Result<String, GazeboAdapterError> {
    if !nonempty(run_id)
        || !nonempty(run_epoch)
        || scenario.pair_id.is_empty()
        || scenario.scenario_id.is_empty()
        || fault.fault_id.is_empty()
        || !valid_sha256(&plan.plan_sha256)
    {
        return Err(GazeboAdapterError::InvalidIdempotencyContext);
    }
    Ok(format!(
        "rrvr/{}/{}/{}/{}/{}",
        plan.plan_sha256,
        scenario.pair_id,
        scenario.scenario_id,
        fault.fault_id,
        run_epoch
    ))
}

/// Validate a resolved platform target against the exact planned fault.
pub fn validate_target_binding(
    binding: &GazeboTargetBinding,
    fault: &FaultInjection,
) -> Result<(), GazeboAdapterError> {
    if !nonempty(&binding.logical_component_id)
        || !nonempty(&binding.target_id)
        || !nonempty(&binding.target_revision)
        || !nonempty(&binding.fault_domain_id)
        || !valid_sha256(&binding.target_profile_sha256)
        || binding.logical_component_id != fault.target_component_id
    {
        return Err(GazeboAdapterError::InvalidTargetBinding);
    }
    if binding.target_revision != fault.target_revision {
        return Err(GazeboAdapterError::TargetRevisionMismatch);
    }
    if binding.target_profile_sha256 != fault.target_profile_sha256 {
        return Err(GazeboAdapterError::TargetProfileMismatch);
    }
    if binding.fault_domain_id != fault.fault_domain_id {
        return Err(GazeboAdapterError::TargetFaultDomainMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign_plan::CompiledCampaignPlan;
    use crate::fault_campaign::FaultCampaignManifest;

    fn plan() -> CompiledCampaignPlan {
        let manifest: FaultCampaignManifest = serde_json::from_str(include_str!(
            "../fixtures/fault_campaign_paired_v0_1.json"
        ))
        .expect("campaign fixture deserializes");
        let bytes = include_bytes!("../fixtures/fault_campaign_paired_v0_1.json");
        let digest = {
            use sha2::{Digest, Sha256};
            let value = Sha256::digest(bytes);
            value.iter().map(|byte| format!("{byte:02x}")).collect()
        };
        CompiledCampaignPlan::compile(&manifest, &digest).unwrap()
    }

    fn profile(plan: &CompiledCampaignPlan) -> GazeboCapabilityProfile {
        GazeboCapabilityProfile {
            version: GAZEBO_ADAPTER_CONTRACT_VERSION.into(),
            simulator_id: plan.platform.platform_id.clone(),
            simulator_revision: plan.platform.simulator_revision.clone(),
            ros_distribution: plan.platform.ros_distribution.clone(),
            simulation_interfaces_revision: "simulation_interfaces-fixture-v0.1".into(),
            world_name: "rrvr-world".into(),
            world_profile_sha256: plan.platform.environment_artifact_sha256.clone(),
            reset_profile_sha256: plan.reset.profile_sha256.clone(),
            scene_broadcaster_id: plan.platform.scene_broadcaster_id.clone(),
            scene_broadcaster_revision: plan.platform.scene_broadcaster_revision.clone(),
            scene_broadcaster_sha256: plan.platform.scene_broadcaster_sha256.clone(),
            scene_broadcaster_loaded: true,
            capabilities: [
                GazeboCapability::GetSimulatorFeatures,
                GazeboCapability::ResetSimulation,
                GazeboCapability::StepSimulation,
                GazeboCapability::GetSimulationState,
                GazeboCapability::SetSimulationState,
                GazeboCapability::GetEntities,
                GazeboCapability::GetEntityState,
            ]
            .into_iter()
            .collect(),
        }
    }

    #[test]
    fn accepts_pinned_capability_profile_with_scene_broadcaster_and_step_service() {
        let plan = plan();
        let profile = profile(&plan);
        assert_eq!(
            profile.validate_for_plan(&plan),
            Ok(SteppingMethod::StepSimulationService)
        );
    }

    #[test]
    fn accepts_action_based_stepping_when_step_service_is_unavailable() {
        let plan = plan();
        let mut profile = profile(&plan);
        profile.capabilities.remove(&GazeboCapability::StepSimulation);
        profile.capabilities.insert(GazeboCapability::SimulateSteps);
        assert_eq!(
            profile.validate_for_plan(&plan),
            Ok(SteppingMethod::SimulateStepsAction)
        );
    }

    #[test]
    fn rejects_missing_scene_broadcaster_even_when_services_are_advertised() {
        let plan = plan();
        let mut profile = profile(&plan);
        profile.scene_broadcaster_loaded = false;
        assert_eq!(
            profile.validate_for_plan(&plan),
            Err(GazeboAdapterError::SceneBroadcasterUnavailable)
        );
    }

    #[test]
    fn rejects_profile_with_no_deterministic_stepping_interface() {
        let plan = plan();
        let mut profile = profile(&plan);
        profile.capabilities.remove(&GazeboCapability::StepSimulation);
        assert_eq!(
            profile.validate_for_plan(&plan),
            Err(GazeboAdapterError::MissingSteppingMethod)
        );
    }

    #[test]
    fn rejects_wrong_simulator_build_or_ros_distribution() {
        let plan = plan();
        let mut wrong_build = profile(&plan);
        wrong_build.simulator_revision = "unreviewed-gazebo-build".into();
        assert_eq!(
            wrong_build.validate_for_plan(&plan),
            Err(GazeboAdapterError::PlatformRevisionMismatch)
        );

        let mut wrong_ros = profile(&plan);
        wrong_ros.ros_distribution = "rolling".into();
        assert_eq!(
            wrong_ros.validate_for_plan(&plan),
            Err(GazeboAdapterError::RosDistributionMismatch)
        );
    }

    #[test]
    fn rejects_unresolved_world_identity_and_invalid_snapshot_digest() {
        let plan = plan();
        let profile = profile(&plan);
        let mut world = GazeboWorldSnapshot {
            simulator_id: plan.platform.platform_id.clone(),
            simulator_revision: plan.platform.simulator_revision.clone(),
            world_name: profile.world_name.clone(),
            world_epoch: "world-epoch-1".into(),
            clock_id: "sim-clock".into(),
            simulation_step: 10,
            simulation_time_ns: 100,
            state: SimulationState::Paused,
            state_sha256: "a".repeat(64),
        };
        world.world_name = "another-world".into();
        assert_eq!(
            world.validate_against_profile(&profile),
            Err(GazeboAdapterError::WrongWorld)
        );
        world.world_name = profile.world_name.clone();
        world.state_sha256 = "invalid".into();
        assert_eq!(
            world.validate_against_profile(&profile),
            Err(GazeboAdapterError::WorldStateDigestMismatch)
        );
    }

    #[test]
    fn requires_pause_and_exact_monotonic_step_alignment() {
        let plan = plan();
        let profile = profile(&plan);
        let mut world = GazeboWorldSnapshot {
            simulator_id: plan.platform.platform_id.clone(),
            simulator_revision: plan.platform.simulator_revision.clone(),
            world_name: profile.world_name.clone(),
            world_epoch: "world-epoch-1".into(),
            clock_id: "sim-clock".into(),
            simulation_step: 90,
            simulation_time_ns: 900,
            state: SimulationState::Paused,
            state_sha256: "a".repeat(64),
        };
        assert_eq!(world.steps_to(100), Ok(10));
        assert_eq!(
            world.steps_to(80),
            Err(GazeboAdapterError::SimulationStepOvershoot)
        );
        world.state = SimulationState::Playing;
        assert_eq!(
            world.steps_to(100),
            Err(GazeboAdapterError::SimulatorNotPaused)
        );
    }

    #[test]
    fn reset_requires_new_world_epoch_paused_at_exact_initial_digest() {
        let plan = plan();
        let profile = profile(&plan);
        let world = GazeboWorldSnapshot {
            simulator_id: plan.platform.platform_id.clone(),
            simulator_revision: plan.platform.simulator_revision.clone(),
            world_name: profile.world_name.clone(),
            world_epoch: "world-epoch-2".into(),
            clock_id: "sim-clock".into(),
            simulation_step: 0,
            simulation_time_ns: 0,
            state: SimulationState::Paused,
            state_sha256: plan.reset.expected_initial_state_sha256.clone(),
        };
        assert_eq!(
            world.validate_reset(&profile, &plan, "world-epoch-1"),
            Ok(())
        );
    }

    #[test]
    fn does_not_accept_a_reset_that_kept_epoch_or_drifted_initial_state() {
        let plan = plan();
        let profile = profile(&plan);
        let mut world = GazeboWorldSnapshot {
            simulator_id: plan.platform.platform_id.clone(),
            simulator_revision: plan.platform.simulator_revision.clone(),
            world_name: profile.world_name.clone(),
            world_epoch: "world-epoch-1".into(),
            clock_id: "sim-clock".into(),
            simulation_step: 0,
            simulation_time_ns: 0,
            state: SimulationState::Paused,
            state_sha256: plan.reset.expected_initial_state_sha256.clone(),
        };
        assert_eq!(
            world.validate_reset(&profile, &plan, "world-epoch-1"),
            Err(GazeboAdapterError::ResetDidNotAdvanceEpoch)
        );
        world.world_epoch = "world-epoch-2".into();
        world.state_sha256 = "d".repeat(64);
        assert_eq!(
            world.validate_reset(&profile, &plan, "world-epoch-1"),
            Err(GazeboAdapterError::ResetDidNotRestoreInitialState)
        );
    }

    #[test]
    fn validates_target_entity_revision_profile_and_fault_domain() {
        let plan = plan();
        let fault = &plan.scenarios[0].fault_events[0];
        let binding = GazeboTargetBinding {
            logical_component_id: fault.target_component_id.clone(),
            target_id: "gazebo-model-001".into(),
            target_revision: fault.target_revision.clone(),
            target_profile_sha256: fault.target_profile_sha256.clone(),
            fault_domain_id: fault.fault_domain_id.clone(),
        };
        assert_eq!(validate_target_binding(&binding, fault), Ok(()));
        let mut wrong_target = binding;
        wrong_target.target_revision = "other-revision".into();
        assert_eq!(
            validate_target_binding(&wrong_target, fault),
            Err(GazeboAdapterError::TargetRevisionMismatch)
        );
    }

    #[test]
    fn idempotency_key_is_stable_and_bound_to_plan_run_and_fault() {
        let plan = plan();
        let scenario = &plan.scenarios[0];
        let fault = &scenario.fault_events[0];
        let first = dispatch_idempotency_key(&plan, scenario, fault, "run-1", "epoch-1");
        let second = dispatch_idempotency_key(&plan, scenario, fault, "run-1", "epoch-1");
        assert_eq!(first, second);
        assert_ne!(
            first,
            dispatch_idempotency_key(&plan, scenario, fault, "run-2", "epoch-1")
        );
    }

    #[test]
    fn unknown_dispatch_is_reconciled_without_redispatch_semantics() {
        assert_eq!(
            reconcile_dispatch_lookup(DispatchLedgerResult::Accepted),
            ReconciliationDecision::ExistingDispatchAccepted
        );
        assert_eq!(
            reconcile_dispatch_lookup(DispatchLedgerResult::Rejected),
            ReconciliationDecision::ExistingDispatchRejected
        );
        assert_eq!(
            reconcile_dispatch_lookup(DispatchLedgerResult::NoDispatchProven),
            ReconciliationDecision::NoDispatchProven
        );
        assert_eq!(
            reconcile_dispatch_lookup(DispatchLedgerResult::Unknown),
            ReconciliationDecision::Unresolved
        );
    }
}
