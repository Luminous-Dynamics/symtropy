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
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const GAZEBO_ADAPTER_CONTRACT_VERSION: &str = "0.1";
pub const GAZEBO_DISPATCH_CANONICALIZATION_ID: &str = "serde-json-value-sorted-keys-v0.1";

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
    pub world_name: String,
    pub world_epoch: String,
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
    ObserverQuorumUnqualified,
    ObserverIdentityMismatch,
    ObserverCommonCauseOverlap,
    ObserverSignatureMismatch,
    ObserverEpochMismatch,
    ObserverStepMismatch,
    ScenarioNotFound,
    FaultNotFound,
    WorldChangedAtCutPoint,
    TargetBindingChanged,
    InvalidDispatchRequest,
    InvalidCompiledPlan,
    ResetNotVerified,
    ResetAlreadyVerified,
    ResetVerificationMismatch,
    ResetOutcomeUnknown,
    ResetOutcomeUnresolved,
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
        if self.simulation_interfaces_revision != plan.platform.simulation_interfaces_revision {
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

        if self
            .capabilities
            .contains(&GazeboCapability::StepSimulation)
        {
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
struct GazeboResetVerification {
    plan_sha256: String,
    profile: GazeboCapabilityProfile,
    stepping_method: SteppingMethod,
    world_epoch: String,
    clock_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GazeboResetAttempt {
    plan_sha256: String,
    profile: GazeboCapabilityProfile,
    stepping_method: SteppingMethod,
    prior_world_epoch: String,
    clock_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum GazeboResetLifecycle {
    NotAttempted,
    DispatchStarted(GazeboResetAttempt),
    OutcomeUnresolved(GazeboResetAttempt),
    Verified(GazeboResetVerification),
}

#[must_use = "retain and reconcile the session before any plan-bound simulator mutation"]
#[derive(Debug, Eq, PartialEq)]
pub struct GazeboAdapterSession {
    pub profile: GazeboCapabilityProfile,
    pub stepping_method: SteppingMethod,
    pub world_epoch: String,
    pub clock_id: String,
    // Private by design: callers cannot construct reset-qualified state from
    // descriptive fields, and an ambiguous reset remains non-retryable.
    reset_lifecycle: GazeboResetLifecycle,
}

/// Discover and pin the simulator before any reset, step or target mutation.
pub fn preflight_transport<T: GazeboSimulationTransport>(
    transport: &mut T,
    plan: &CompiledCampaignPlan,
) -> Result<GazeboAdapterSession, GazeboAdapterError> {
    plan.verify_integrity()
        .map_err(|_| GazeboAdapterError::InvalidCompiledPlan)?;
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
        reset_lifecycle: GazeboResetLifecycle::NotAttempted,
    })
}

/// Reset once and verify the exact initial snapshot before scheduling faults.
pub fn reset_and_verify<T: GazeboSimulationTransport>(
    transport: &mut T,
    plan: &CompiledCampaignPlan,
    session: &mut GazeboAdapterSession,
) -> Result<GazeboWorldSnapshot, GazeboAdapterError> {
    plan.verify_integrity()
        .map_err(|_| GazeboAdapterError::InvalidCompiledPlan)?;
    match &session.reset_lifecycle {
        GazeboResetLifecycle::NotAttempted => {}
        GazeboResetLifecycle::Verified(_) => {
            return Err(GazeboAdapterError::ResetAlreadyVerified);
        }
        GazeboResetLifecycle::DispatchStarted(_) | GazeboResetLifecycle::OutcomeUnresolved(_) => {
            return Err(GazeboAdapterError::ResetOutcomeUnresolved);
        }
    }
    let live_profile = transport
        .get_simulator_features()
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    if live_profile != session.profile {
        return Err(GazeboAdapterError::InvalidCapabilityProfile);
    }
    let selected_method = session.profile.validate_for_plan(plan)?;
    if selected_method != session.stepping_method {
        return Err(GazeboAdapterError::InvalidCapabilityProfile);
    }
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

    let attempt = GazeboResetAttempt {
        plan_sha256: plan.plan_sha256.clone(),
        profile: session.profile.clone(),
        stepping_method: session.stepping_method,
        prior_world_epoch: before.world_epoch.clone(),
        clock_id: before.clock_id.clone(),
    };
    session.reset_lifecycle = GazeboResetLifecycle::DispatchStarted(attempt.clone());
    if transport
        .reset_simulation(&session.profile.world_name)
        .is_err()
    {
        session.reset_lifecycle = GazeboResetLifecycle::OutcomeUnresolved(attempt);
        return Err(GazeboAdapterError::ResetOutcomeUnknown);
    }
    let after = match transport.get_world_state(&session.profile.world_name) {
        Ok(snapshot) => snapshot,
        Err(_) => {
            session.reset_lifecycle = GazeboResetLifecycle::OutcomeUnresolved(attempt);
            return Err(GazeboAdapterError::ResetOutcomeUnknown);
        }
    };
    if let Err(error) = after.validate_reset(&session.profile, plan, &before.world_epoch) {
        session.reset_lifecycle = GazeboResetLifecycle::OutcomeUnresolved(attempt);
        return Err(error);
    }
    if after.clock_id != before.clock_id {
        session.reset_lifecycle = GazeboResetLifecycle::OutcomeUnresolved(attempt);
        return Err(GazeboAdapterError::InvalidClock);
    }

    session.world_epoch = after.world_epoch.clone();
    session.clock_id = after.clock_id.clone();
    session.reset_lifecycle = GazeboResetLifecycle::Verified(GazeboResetVerification {
        plan_sha256: plan.plan_sha256.clone(),
        profile: session.profile.clone(),
        stepping_method: session.stepping_method,
        world_epoch: session.world_epoch.clone(),
        clock_id: session.clock_id.clone(),
    });
    Ok(after)
}

/// Reconcile an ambiguous reset using read-only state observation. This function
/// never calls ResetSimulation. It can qualify the reset only when the observed
/// world proves the planned new epoch and exact initial-state commitment.
pub fn reconcile_reset<T: GazeboSimulationTransport>(
    transport: &mut T,
    plan: &CompiledCampaignPlan,
    session: &mut GazeboAdapterSession,
) -> Result<GazeboWorldSnapshot, GazeboAdapterError> {
    plan.verify_integrity()
        .map_err(|_| GazeboAdapterError::InvalidCompiledPlan)?;
    let attempt = match &session.reset_lifecycle {
        GazeboResetLifecycle::DispatchStarted(attempt)
        | GazeboResetLifecycle::OutcomeUnresolved(attempt) => attempt.clone(),
        GazeboResetLifecycle::NotAttempted => {
            return Err(GazeboAdapterError::ResetNotVerified);
        }
        GazeboResetLifecycle::Verified(_) => {
            return Err(GazeboAdapterError::ResetAlreadyVerified);
        }
    };
    if session.profile != attempt.profile
        || session.stepping_method != attempt.stepping_method
        || plan.plan_sha256 != attempt.plan_sha256
        || session.world_epoch != attempt.prior_world_epoch
        || session.clock_id != attempt.clock_id
    {
        return Err(GazeboAdapterError::ResetVerificationMismatch);
    }
    let live_profile = transport
        .get_simulator_features()
        .map_err(|_| GazeboAdapterError::ResetOutcomeUnresolved)?;
    if live_profile != attempt.profile {
        return Err(GazeboAdapterError::ResetOutcomeUnresolved);
    }
    let after = transport
        .get_world_state(&attempt.profile.world_name)
        .map_err(|_| GazeboAdapterError::ResetOutcomeUnresolved)?;
    if after
        .validate_reset(&attempt.profile, plan, &attempt.prior_world_epoch)
        .is_err()
        || after.clock_id != attempt.clock_id
    {
        return Err(GazeboAdapterError::ResetOutcomeUnresolved);
    }

    session.world_epoch = after.world_epoch.clone();
    session.clock_id = after.clock_id.clone();
    session.reset_lifecycle = GazeboResetLifecycle::Verified(GazeboResetVerification {
        plan_sha256: attempt.plan_sha256,
        profile: attempt.profile,
        stepping_method: attempt.stepping_method,
        world_epoch: session.world_epoch.clone(),
        clock_id: session.clock_id.clone(),
    });
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
            .step_simulation(&session.profile.world_name, steps, session.stepping_method)
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
    let snapshot = transport
        .get_world_state(&session.profile.world_name)
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    snapshot.validate_against_profile(&session.profile)?;
    if snapshot.world_epoch != session.world_epoch {
        return Err(GazeboAdapterError::WrongWorldEpoch);
    }
    if snapshot.clock_id != session.clock_id {
        return Err(GazeboAdapterError::InvalidClock);
    }
    if snapshot.state != SimulationState::Paused {
        return Err(GazeboAdapterError::SimulatorNotPaused);
    }
    let binding = transport
        .resolve_target(&session.profile.world_name, &fault.target_component_id)
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    validate_target_binding(
        &binding,
        fault,
        &session.profile.world_name,
        &session.world_epoch,
    )?;
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GazeboDispatchRequest {
    pub plan_sha256: String,
    pub campaign_id: String,
    pub pair_id: String,
    pub scenario_id: String,
    pub fault_id: String,
    pub fault_sequence: u32,
    pub world_epoch: String,
    pub clock_id: String,
    pub target_id: String,
    pub fault_domain_id: String,
    pub scheduled_simulation_step: u64,
    pub parameters_sha256: String,
    /// Digest of the canonical request body excluding this field.
    pub payload_sha256: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GazeboDispatchOutcome {
    Accepted {
        dispatch_id: String,
    },
    Rejected {
        dispatch_id: String,
        reason_code: String,
    },
    Unknown {
        dispatch_id: Option<String>,
    },
    DuplicateSuppressed,
    PayloadConflict {
        idempotency_key: String,
    },
    Reconciled {
        decision: ReconciliationDecision,
    },
    InvalidDispatchRequest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DispatchAttemptEntry {
    payload_sha256: String,
    outcome: Option<GazeboDispatchOutcome>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DispatchAttemptRegistry {
    // Process-local fast guard only. The injector owner must persist the same
    // key-to-payload-to-outcome binding durably across coordinator restarts.
    attempted_idempotency_keys: BTreeMap<String, DispatchAttemptEntry>,
}

impl DispatchAttemptRegistry {
    pub fn attempted(&self, idempotency_key: &str) -> bool {
        self.attempted_idempotency_keys
            .contains_key(idempotency_key)
    }
}

/// Canonical request-body projection used by both the coordinator and injector.
/// The idempotency key is included; payload_sha256 itself is excluded.
#[derive(Serialize)]
struct GazeboDispatchPayload<'a> {
    canonicalization: &'static str,
    plan_sha256: &'a str,
    campaign_id: &'a str,
    pair_id: &'a str,
    scenario_id: &'a str,
    fault_id: &'a str,
    fault_sequence: u32,
    world_epoch: &'a str,
    clock_id: &'a str,
    target_id: &'a str,
    fault_domain_id: &'a str,
    scheduled_simulation_step: u64,
    parameters_sha256: &'a str,
    idempotency_key: &'a str,
}

/// Compute the stable payload commitment. The serde_json Value conversion
/// sorts object keys under this crate's pinned serde_json configuration.
pub fn dispatch_request_fingerprint(
    request: &GazeboDispatchRequest,
) -> Result<String, GazeboAdapterError> {
    if !valid_sha256(&request.plan_sha256)
        || !valid_sha256(&request.parameters_sha256)
        || !nonempty(&request.campaign_id)
        || !nonempty(&request.pair_id)
        || !nonempty(&request.scenario_id)
        || !nonempty(&request.fault_id)
        || !nonempty(&request.world_epoch)
        || !nonempty(&request.clock_id)
        || !nonempty(&request.target_id)
        || !nonempty(&request.fault_domain_id)
        || !nonempty(&request.idempotency_key)
    {
        return Err(GazeboAdapterError::InvalidDispatchRequest);
    }
    let payload = GazeboDispatchPayload {
        canonicalization: GAZEBO_DISPATCH_CANONICALIZATION_ID,
        plan_sha256: &request.plan_sha256,
        campaign_id: &request.campaign_id,
        pair_id: &request.pair_id,
        scenario_id: &request.scenario_id,
        fault_id: &request.fault_id,
        fault_sequence: request.fault_sequence,
        world_epoch: &request.world_epoch,
        clock_id: &request.clock_id,
        target_id: &request.target_id,
        fault_domain_id: &request.fault_domain_id,
        scheduled_simulation_step: request.scheduled_simulation_step,
        parameters_sha256: &request.parameters_sha256,
        idempotency_key: &request.idempotency_key,
    };
    let value =
        serde_json::to_value(payload).map_err(|_| GazeboAdapterError::InvalidDispatchRequest)?;
    let bytes =
        serde_json::to_vec(&value).map_err(|_| GazeboAdapterError::InvalidDispatchRequest)?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Owner-ledger lookup evidence. Production injectors should bind the returned
/// payload digest to the request and provide the exact owner-produced receipt bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GazeboDispatchLedgerReceipt {
    /// Digest of the payload used in this lookup request, echoed by the owner.
    pub payload_sha256: Option<String>,
    /// The digest already bound to this key at the owner. For PayloadConflict,
    /// this must be present and different from the requested payload digest.
    pub bound_payload_sha256: Option<String>,
    pub status: DispatchLedgerResult,
    pub dispatch_id: Option<String>,
    pub raw_artifact_bytes: Option<Vec<u8>>,
}

/// Injector contract: atomically and durably bind idempotency_key to
/// payload_sha256 before any fault effect can occur. Repeating the same key and
/// digest must return the prior operation result without repeating the effect;
/// reusing the key with a different digest must return PayloadConflict. The
/// ledger must survive coordinator/injector process restarts. This trait cannot
/// itself prove that a concrete remote injector satisfies those requirements.
/// A transport error maps to Unknown.
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

    /// Payload-bound owner lookup. The default preserves source compatibility
    /// but intentionally omits payload binding and owner artifact bytes. A
    /// journaled caller must treat that compatibility result as Unresolved.
    fn lookup_dispatch_bound(
        &mut self,
        request: &GazeboDispatchRequest,
    ) -> Result<GazeboDispatchLedgerReceipt, Self::Error> {
        let status = self.lookup_dispatch(&request.idempotency_key)?;
        Ok(GazeboDispatchLedgerReceipt {
            payload_sha256: None,
            bound_payload_sha256: None,
            status,
            dispatch_id: None,
            raw_artifact_bytes: None,
        })
    }
}

/// Issue one dispatch attempt. An error is not retried and maps to Unknown.
fn dispatch_fault_once<T: GazeboFaultInjectorTransport>(
    registry: &mut DispatchAttemptRegistry,
    injector: &mut T,
    request: &GazeboDispatchRequest,
) -> GazeboDispatchOutcome {
    if !nonempty(&request.idempotency_key) || !valid_sha256(&request.plan_sha256) {
        return GazeboDispatchOutcome::InvalidDispatchRequest;
    }
    let fingerprint = match dispatch_request_fingerprint(request) {
        Ok(fingerprint) if fingerprint == request.payload_sha256 => fingerprint,
        _ => return GazeboDispatchOutcome::InvalidDispatchRequest,
    };
    if let Some(previous) = registry
        .attempted_idempotency_keys
        .get(&request.idempotency_key)
    {
        if previous.payload_sha256 != fingerprint {
            return GazeboDispatchOutcome::PayloadConflict {
                idempotency_key: request.idempotency_key.clone(),
            };
        }
        return match &previous.outcome {
            Some(GazeboDispatchOutcome::Unknown { dispatch_id }) => {
                GazeboDispatchOutcome::Unknown {
                    dispatch_id: dispatch_id.clone(),
                }
            }
            Some(GazeboDispatchOutcome::PayloadConflict { idempotency_key }) => {
                GazeboDispatchOutcome::PayloadConflict {
                    idempotency_key: idempotency_key.clone(),
                }
            }
            Some(GazeboDispatchOutcome::InvalidDispatchRequest) => {
                GazeboDispatchOutcome::InvalidDispatchRequest
            }
            Some(GazeboDispatchOutcome::Reconciled { decision }) => {
                GazeboDispatchOutcome::Reconciled {
                    decision: *decision,
                }
            }
            Some(_) => GazeboDispatchOutcome::DuplicateSuppressed,
            None => GazeboDispatchOutcome::Unknown { dispatch_id: None },
        };
    }
    registry.attempted_idempotency_keys.insert(
        request.idempotency_key.clone(),
        DispatchAttemptEntry {
            payload_sha256: fingerprint,
            outcome: None,
        },
    );
    let outcome = injector
        .dispatch_once(request)
        .unwrap_or(GazeboDispatchOutcome::Unknown { dispatch_id: None });
    if let Some(entry) = registry
        .attempted_idempotency_keys
        .get_mut(&request.idempotency_key)
    {
        entry.outcome = Some(outcome.clone());
    }
    outcome
}

/// Reconcile by idempotency key only. This function never calls dispatch_once.
pub fn reconcile_fault_dispatch<T: GazeboFaultInjectorTransport>(
    registry: &mut DispatchAttemptRegistry,
    injector: &mut T,
    request: &GazeboDispatchRequest,
) -> ReconciliationDecision {
    let fingerprint = match dispatch_request_fingerprint(request) {
        Ok(fingerprint) if fingerprint == request.payload_sha256 => fingerprint,
        _ => return ReconciliationDecision::PayloadConflict,
    };
    let Some(entry) = registry
        .attempted_idempotency_keys
        .get_mut(&request.idempotency_key)
    else {
        // The journal/recovered registry entry must be restored before a
        // coordinator asks the remote ledger to reconcile this request.
        return ReconciliationDecision::Unresolved;
    };
    if entry.payload_sha256 != fingerprint {
        return ReconciliationDecision::PayloadConflict;
    }

    let decision = match injector.lookup_dispatch(&request.idempotency_key) {
        Ok(lookup) => reconcile_dispatch_lookup(lookup),
        Err(_) => ReconciliationDecision::Unresolved,
    };
    entry.outcome = Some(GazeboDispatchOutcome::Reconciled { decision });
    decision
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedFaultSignature {
    pub observer_id: String,
    pub observer_revision: String,
    pub observer_binary_sha256: String,
    pub common_cause_domains: Vec<String>,
    pub signature_sha256: String,
    pub world_epoch: String,
    pub simulation_step: u64,
}

/// The observer must be a separate subsystem from the injector. It receives
/// the event identity and observation coordinates, but never the expected
/// signature; that oracle value remains with the independent assessor.
pub trait GazeboIndependentObserver {
    type Error: std::fmt::Display;

    fn observe_fault(
        &mut self,
        target_id: &str,
        fault_id: &str,
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
    PayloadConflict,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconciliationDecision {
    ExistingDispatchAccepted,
    ExistingDispatchRejected,
    NoDispatchProven,
    Unresolved,
    PayloadConflict,
}

/// A lost dispatch response maps to Unknown. It never invokes dispatch again.
pub fn reconcile_dispatch_lookup(lookup: DispatchLedgerResult) -> ReconciliationDecision {
    match lookup {
        DispatchLedgerResult::Accepted => ReconciliationDecision::ExistingDispatchAccepted,
        DispatchLedgerResult::Rejected => ReconciliationDecision::ExistingDispatchRejected,
        DispatchLedgerResult::NoDispatchProven => ReconciliationDecision::NoDispatchProven,
        DispatchLedgerResult::Unknown => ReconciliationDecision::Unresolved,
        DispatchLedgerResult::PayloadConflict => ReconciliationDecision::PayloadConflict,
    }
}

/// Three-valued result of comparing a qualified observer quorum with the
/// hidden plan-committed delivery signature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeliveryEvidenceVerdict {
    Confirmed,
    Refuted,
    InsufficientEvidence,
}

/// Qualify a fault-delivery observer quorum against the exact plan profile.
///
/// Structural identity, epoch, step and common-cause defects are hard errors.
/// Once the quorum is structurally qualified, unanimous agreement on the
/// expected signature is Confirmed, unanimous agreement on a different valid
/// signature is Refuted, and disagreement is InsufficientEvidence.
pub fn validate_delivery_observer_quorum(
    observations: &[ObservedFaultSignature],
    plan: &CompiledCampaignPlan,
    fault_id: &str,
    expected_world_epoch: &str,
) -> Result<DeliveryEvidenceVerdict, GazeboAdapterError> {
    plan.verify_integrity()
        .map_err(|_| GazeboAdapterError::InvalidCompiledPlan)?;
    let fault = plan
        .scenarios
        .iter()
        .flat_map(|scenario| scenario.fault_events.iter())
        .find(|candidate| candidate.fault_id == fault_id)
        .ok_or(GazeboAdapterError::FaultNotFound)?;
    let required = plan.platform.required_delivery_observer_quorum;
    if observations.len() < required || observations.len() > 32 {
        return Err(GazeboAdapterError::ObserverQuorumUnqualified);
    }
    if !nonempty(expected_world_epoch) {
        return Err(GazeboAdapterError::ObserverEpochMismatch);
    }

    let mut observer_ids = BTreeSet::new();
    let mut common_cause_domains = BTreeSet::new();
    let mut observed_signatures = BTreeSet::new();
    let mut previous_observer: Option<&str> = None;
    for observation in observations {
        if !observer_ids.insert(observation.observer_id.as_str())
            || previous_observer
                .is_some_and(|previous| previous >= observation.observer_id.as_str())
        {
            return Err(GazeboAdapterError::ObserverIdentityMismatch);
        }
        previous_observer = Some(&observation.observer_id);

        if observation.common_cause_domains.is_empty()
            || observation.common_cause_domains.iter().any(|domain| {
                !nonempty(domain)
                    || domain == &plan.platform.fault_injector_fault_domain_id
                    || !common_cause_domains.insert(domain.as_str())
            })
        {
            return Err(GazeboAdapterError::ObserverCommonCauseOverlap);
        }

        let profile = plan
            .platform
            .observer_profiles
            .iter()
            .find(|profile| profile.observer_id == observation.observer_id)
            .ok_or(GazeboAdapterError::ObserverIdentityMismatch)?;
        if observation.observer_revision != profile.revision
            || observation.observer_binary_sha256 != profile.binary_sha256
            || observation.common_cause_domains != profile.common_cause_domains
            || observation.observer_id == plan.platform.fault_injector_id
        {
            return Err(GazeboAdapterError::ObserverIdentityMismatch);
        }
        if !valid_sha256(&observation.signature_sha256) {
            return Err(GazeboAdapterError::ObserverSignatureMismatch);
        }
        if observation.world_epoch != expected_world_epoch {
            return Err(GazeboAdapterError::ObserverEpochMismatch);
        }
        if observation.simulation_step != fault.scheduled_simulation_step {
            return Err(GazeboAdapterError::ObserverStepMismatch);
        }
        observed_signatures.insert(observation.signature_sha256.as_str());
    }

    if observed_signatures.len() != 1 {
        return Ok(DeliveryEvidenceVerdict::InsufficientEvidence);
    }
    let observed = observed_signatures
        .first()
        .ok_or(GazeboAdapterError::ObserverQuorumUnqualified)?;
    if *observed == fault.expected_delivery_signature_sha256.as_str() {
        Ok(DeliveryEvidenceVerdict::Confirmed)
    } else {
        Ok(DeliveryEvidenceVerdict::Refuted)
    }
}

#[derive(Serialize)]
struct GazeboIdempotencyIdentity<'a> {
    canonicalization: &'static str,
    plan_sha256: &'a str,
    campaign_id: &'a str,
    pair_id: &'a str,
    scenario_id: &'a str,
    fault_id: &'a str,
    fault_sequence: u32,
    world_epoch: &'a str,
    clock_id: &'a str,
    run_id: &'a str,
    run_epoch: &'a str,
}

/// A stable idempotency key binds one plan-committed event to one run epoch.
/// The canonical identity is hashed so delimiter characters in caller IDs
/// cannot create textual key collisions.
pub fn dispatch_idempotency_key(
    plan: &CompiledCampaignPlan,
    scenario: &CompiledScenario,
    fault: &FaultInjection,
    run_id: &str,
    run_epoch: &str,
    world_epoch: &str,
    clock_id: &str,
) -> Result<String, GazeboAdapterError> {
    plan.verify_integrity()
        .map_err(|_| GazeboAdapterError::InvalidCompiledPlan)?;
    if !nonempty(run_id)
        || !nonempty(run_epoch)
        || !nonempty(world_epoch)
        || !nonempty(clock_id)
        || !nonempty(&scenario.pair_id)
        || !nonempty(&scenario.scenario_id)
        || !nonempty(&fault.fault_id)
    {
        return Err(GazeboAdapterError::InvalidIdempotencyContext);
    }
    let committed_scenario = plan
        .scenarios
        .iter()
        .find(|candidate| candidate.scenario_id == scenario.scenario_id)
        .ok_or(GazeboAdapterError::InvalidIdempotencyContext)?;
    if committed_scenario != scenario
        || !committed_scenario
            .fault_events
            .iter()
            .any(|candidate| candidate == fault)
    {
        return Err(GazeboAdapterError::InvalidIdempotencyContext);
    }

    let identity = GazeboIdempotencyIdentity {
        canonicalization: GAZEBO_DISPATCH_CANONICALIZATION_ID,
        plan_sha256: &plan.plan_sha256,
        campaign_id: &plan.campaign_id,
        pair_id: &scenario.pair_id,
        scenario_id: &scenario.scenario_id,
        fault_id: &fault.fault_id,
        fault_sequence: fault.sequence,
        world_epoch,
        clock_id,
        run_id,
        run_epoch,
    };
    let value = serde_json::to_value(identity)
        .map_err(|_| GazeboAdapterError::InvalidIdempotencyContext)?;
    let bytes =
        serde_json::to_vec(&value).map_err(|_| GazeboAdapterError::InvalidIdempotencyContext)?;
    let digest = Sha256::digest(bytes);
    let digest: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!("rrvr/v0.1/{digest}"))
}

/// Validate a resolved platform target against the exact planned fault.
pub fn validate_target_binding(
    binding: &GazeboTargetBinding,
    fault: &FaultInjection,
    expected_world_name: &str,
    expected_world_epoch: &str,
) -> Result<(), GazeboAdapterError> {
    if binding.world_name != expected_world_name {
        return Err(GazeboAdapterError::WrongWorld);
    }
    if binding.world_epoch != expected_world_epoch {
        return Err(GazeboAdapterError::WrongWorldEpoch);
    }
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

/// Result of dispatching one fault selected from the verified compiled plan.
///
/// The record carries the exact cut-point snapshot and target binding that were
/// rechecked immediately before dispatch. An Unknown outcome remains a
/// reconciliation requirement; this type never grants retry or native authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GazeboPlannedDispatchResult {
    pub request: GazeboDispatchRequest,
    pub cut_snapshot: GazeboWorldSnapshot,
    pub target_binding: GazeboTargetBinding,
    pub outcome: GazeboDispatchOutcome,
}

/// The committed scenario/fault and coordinator run identity for one dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GazeboFaultRunIdentity<'a> {
    pub scenario_id: &'a str,
    pub fault_id: &'a str,
    pub run_id: &'a str,
    pub run_epoch: &'a str,
}

/// Execute the plan-to-dispatch boundary for one declared fault event.
///
/// Unlike calling the individual step/resolve/dispatch helpers, this function
/// selects the scenario and fault from the integrity-verified plan, advances to
/// the exact planned step, resolves the target, then re-reads the world and
/// target before dispatch. A changed epoch, clock, step, time, world-state digest,
/// or target binding prevents dispatch. The simulator must still be isolated
/// from external mutators: Gazebo's standard interfaces do not provide an
/// exclusive-world lease, so this final recheck narrows but cannot eliminate a
/// time-of-check/time-of-use race with an uncoordinated external client.
pub fn dispatch_planned_fault_once<S, I>(
    simulator: &mut S,
    injector: &mut I,
    registry: &mut DispatchAttemptRegistry,
    plan: &CompiledCampaignPlan,
    session: &GazeboAdapterSession,
    identity: GazeboFaultRunIdentity<'_>,
) -> Result<GazeboPlannedDispatchResult, GazeboAdapterError>
where
    S: GazeboSimulationTransport,
    I: GazeboFaultInjectorTransport,
{
    plan.verify_integrity()
        .map_err(|_| GazeboAdapterError::InvalidCompiledPlan)?;
    let reset_verification = match &session.reset_lifecycle {
        GazeboResetLifecycle::Verified(verification) => verification,
        GazeboResetLifecycle::NotAttempted => {
            return Err(GazeboAdapterError::ResetNotVerified);
        }
        GazeboResetLifecycle::DispatchStarted(_) | GazeboResetLifecycle::OutcomeUnresolved(_) => {
            return Err(GazeboAdapterError::ResetOutcomeUnresolved);
        }
    };
    if reset_verification.plan_sha256 != plan.plan_sha256
        || reset_verification.profile != session.profile
        || reset_verification.stepping_method != session.stepping_method
        || reset_verification.world_epoch != session.world_epoch
        || reset_verification.clock_id != session.clock_id
        || !nonempty(&session.world_epoch)
        || !nonempty(&session.clock_id)
    {
        return Err(GazeboAdapterError::ResetVerificationMismatch);
    }
    let live_profile = simulator
        .get_simulator_features()
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    if live_profile != reset_verification.profile {
        return Err(GazeboAdapterError::InvalidCapabilityProfile);
    }
    let selected_method = live_profile.validate_for_plan(plan)?;
    if selected_method != reset_verification.stepping_method {
        return Err(GazeboAdapterError::InvalidCapabilityProfile);
    }
    let scenario = plan
        .scenarios
        .iter()
        .find(|scenario| scenario.scenario_id == identity.scenario_id)
        .ok_or(GazeboAdapterError::ScenarioNotFound)?;
    let fault = scenario
        .fault_events
        .iter()
        .find(|fault| fault.fault_id == identity.fault_id)
        .ok_or(GazeboAdapterError::FaultNotFound)?;

    let cut_snapshot = advance_to_fault_cut(simulator, session, fault)?;
    let target_binding = resolve_fault_target(simulator, session, fault)?;

    // Close the gap between target resolution and dispatch as far as the
    // simulator interface permits. The unchanged digest is a state witness,
    // not a substitute for an enforced exclusive-world lease.
    let before_dispatch = simulator
        .get_world_state(&session.profile.world_name)
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    before_dispatch.validate_against_profile(&session.profile)?;
    if before_dispatch.world_epoch != session.world_epoch {
        return Err(GazeboAdapterError::WrongWorldEpoch);
    }
    if before_dispatch.clock_id != session.clock_id {
        return Err(GazeboAdapterError::InvalidClock);
    }
    if before_dispatch.state != SimulationState::Paused {
        return Err(GazeboAdapterError::SimulatorNotPaused);
    }
    if before_dispatch.simulation_step > fault.scheduled_simulation_step {
        return Err(GazeboAdapterError::SimulationStepOvershoot);
    }
    if before_dispatch.simulation_step != fault.scheduled_simulation_step {
        return Err(GazeboAdapterError::SimulationStepNotReached);
    }
    if before_dispatch.simulation_time_ns != cut_snapshot.simulation_time_ns
        || before_dispatch.state_sha256 != cut_snapshot.state_sha256
    {
        return Err(GazeboAdapterError::WorldChangedAtCutPoint);
    }

    let refreshed_target = simulator
        .resolve_target(&session.profile.world_name, &fault.target_component_id)
        .map_err(|_| GazeboAdapterError::TransportFailure)?;
    validate_target_binding(
        &refreshed_target,
        fault,
        &session.profile.world_name,
        &session.world_epoch,
    )?;
    if refreshed_target != target_binding {
        return Err(GazeboAdapterError::TargetBindingChanged);
    }

    let idempotency_key = dispatch_idempotency_key(
        plan,
        scenario,
        fault,
        identity.run_id,
        identity.run_epoch,
        &session.world_epoch,
        &session.clock_id,
    )?;
    let mut request = GazeboDispatchRequest {
        plan_sha256: plan.plan_sha256.clone(),
        campaign_id: plan.campaign_id.clone(),
        pair_id: scenario.pair_id.clone(),
        scenario_id: scenario.scenario_id.clone(),
        fault_id: fault.fault_id.clone(),
        fault_sequence: fault.sequence,
        world_epoch: session.world_epoch.clone(),
        clock_id: session.clock_id.clone(),
        target_id: target_binding.target_id.clone(),
        fault_domain_id: fault.fault_domain_id.clone(),
        scheduled_simulation_step: fault.scheduled_simulation_step,
        parameters_sha256: fault.parameters_sha256.clone(),
        payload_sha256: String::new(),
        idempotency_key,
    };
    request.payload_sha256 = dispatch_request_fingerprint(&request)?;
    let outcome = dispatch_fault_once(registry, injector, &request);

    Ok(GazeboPlannedDispatchResult {
        request,
        cut_snapshot,
        target_binding,
        outcome,
    })
}


/// Journal-aware simulator and injector execution. This module is nested in
/// gazebo_adapter so it can restore the private reset lifecycle from journaled
/// evidence without exposing that lifecycle to arbitrary callers.
pub mod journaled {
    use super::*;
    use crate::durable_journal::{
        DispatchIntentRecord, DispatchOutcomeRecord, DispatchOutcomeSource,
        DispatchOutcomeStatus, DispatchRecoveryDisposition, Journal, JournalError,
        ResetIntentRecord, ResetOutcomeRecord, ResetOutcomeSource, ResetOutcomeStatus,
        ResetRecoveryDisposition,
    };
    use serde::Serialize;
    use std::fmt::Display;

    /// Artifact storage must complete durably before returning the SHA-256.
    pub trait GazeboEvidenceArtifactStore {
        type Error: Display;

        fn persist(&mut self, artifact_name: &str, bytes: &[u8]) -> Result<String, Self::Error>;
    }

    #[derive(Debug)]
    pub enum JournaledGazeboError {
        Adapter(GazeboAdapterError),
        Journal(JournalError),
        ArtifactStore(String),
        ArtifactDigestMismatch,
        InvalidJournalIdentity,
        ExistingAttemptConflict,
    }

    impl From<GazeboAdapterError> for JournaledGazeboError {
        fn from(value: GazeboAdapterError) -> Self {
            Self::Adapter(value)
        }
    }

    impl From<JournalError> for JournaledGazeboError {
        fn from(value: JournalError) -> Self {
            Self::Journal(value)
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct GazeboResetRunIdentity<'a> {
        pub run_id: &'a str,
        pub run_epoch: &'a str,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct GazeboJournaledDispatchIdentity<'a> {
        pub reset_id: &'a str,
        pub scenario_id: &'a str,
        pub fault_id: &'a str,
        pub run_id: &'a str,
        pub run_epoch: &'a str,
        pub attempt_id: &'a str,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct GazeboJournaledResetResult {
        pub reset_id: String,
        pub snapshot: GazeboWorldSnapshot,
        pub reconciled_after_ambiguity: bool,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct GazeboJournaledDispatchResult {
        pub request: GazeboDispatchRequest,
        pub outcome: GazeboDispatchOutcome,
        pub cut_snapshot: Option<GazeboWorldSnapshot>,
        pub target_binding: Option<GazeboTargetBinding>,
        pub recovered_from_journal: bool,
    }

    fn digest_hex(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn persist_bytes<S: GazeboEvidenceArtifactStore>(
        store: &mut S,
        name: &str,
        bytes: &[u8],
    ) -> Result<String, JournaledGazeboError> {
        let expected = digest_hex(bytes);
        let returned = store
            .persist(name, bytes)
            .map_err(|error| JournaledGazeboError::ArtifactStore(error.to_string()))?;
        if returned != expected {
            return Err(JournaledGazeboError::ArtifactDigestMismatch);
        }
        Ok(expected)
    }

    fn persist_json<S: GazeboEvidenceArtifactStore, T: Serialize>(
        store: &mut S,
        name: &str,
        value: &T,
    ) -> Result<String, JournaledGazeboError> {
        let bytes = serde_json::to_vec(value)
            .map_err(|_| JournaledGazeboError::InvalidJournalIdentity)?;
        persist_bytes(store, name, &bytes)
    }

    #[derive(Serialize)]
    struct ResetIdentity<'a> {
        canonicalization: &'static str,
        plan_sha256: &'a str,
        simulator_profile_sha256: &'a str,
        world_id: &'a str,
        run_id: &'a str,
        run_epoch: &'a str,
    }

    /// Derive a collision-resistant reset identity from canonical structured
    /// fields. A new identity is required for every new reset attempt.
    pub fn reset_idempotency_key(
        plan: &CompiledCampaignPlan,
        session: &GazeboAdapterSession,
        identity: GazeboResetRunIdentity<'_>,
    ) -> Result<String, JournaledGazeboError> {
        plan.verify_integrity()
            .map_err(|_| GazeboAdapterError::InvalidCompiledPlan)?;
        if identity.run_id.trim().is_empty()
            || identity.run_epoch.trim().is_empty()
            || session.world_epoch.trim().is_empty()
            || session.clock_id.trim().is_empty()
            || !valid_sha256(&plan.platform.profile_sha256)
        {
            return Err(JournaledGazeboError::InvalidJournalIdentity);
        }
        let payload = ResetIdentity {
            canonicalization: GAZEBO_DISPATCH_CANONICALIZATION_ID,
            plan_sha256: &plan.plan_sha256,
            simulator_profile_sha256: &plan.platform.profile_sha256,
            world_id: &session.profile.world_name,
            run_id: identity.run_id,
            run_epoch: identity.run_epoch,
        };
        let value = serde_json::to_value(payload)
            .map_err(|_| JournaledGazeboError::InvalidJournalIdentity)?;
        let bytes = serde_json::to_vec(&value)
            .map_err(|_| JournaledGazeboError::InvalidJournalIdentity)?;
        Ok(format!("rrvr/reset/v0.1/{}", digest_hex(&bytes)))
    }

    /// Preflight without pausing or otherwise mutating the simulator. This is
    /// the correct entry point after restart, before journal reconciliation.
    pub fn preflight_transport_read_only<T: GazeboSimulationTransport>(
        transport: &mut T,
        plan: &CompiledCampaignPlan,
    ) -> Result<GazeboAdapterSession, GazeboAdapterError> {
        plan.verify_integrity()
            .map_err(|_| GazeboAdapterError::InvalidCompiledPlan)?;
        let profile = transport
            .get_simulator_features()
            .map_err(|_| GazeboAdapterError::TransportFailure)?;
        let stepping_method = profile.validate_for_plan(plan)?;
        let snapshot = transport
            .get_world_state(&profile.world_name)
            .map_err(|_| GazeboAdapterError::TransportFailure)?;
        snapshot.validate_against_profile(&profile)?;
        if snapshot.state != SimulationState::Paused {
            return Err(GazeboAdapterError::SimulatorNotPaused);
        }
        Ok(GazeboAdapterSession {
            profile,
            stepping_method,
            world_epoch: snapshot.world_epoch,
            clock_id: snapshot.clock_id,
            reset_lifecycle: GazeboResetLifecycle::NotAttempted,
        })
    }

    fn reset_intent_from_session(
        plan: &CompiledCampaignPlan,
        session: &GazeboAdapterSession,
        reset_id: &str,
        identity: GazeboResetRunIdentity<'_>,
    ) -> ResetIntentRecord {
        ResetIntentRecord {
            reset_id: reset_id.to_owned(),
            plan_sha256: plan.plan_sha256.clone(),
            simulator_profile_sha256: plan.platform.profile_sha256.clone(),
            world_id: session.profile.world_name.clone(),
            prior_world_epoch: session.world_epoch.clone(),
            clock_id: session.clock_id.clone(),
            expected_initial_state_sha256: plan.reset.expected_initial_state_sha256.clone(),
            run_id: identity.run_id.to_owned(),
            run_epoch: identity.run_epoch.to_owned(),
        }
    }

    fn reset_attempt_from_intent(
        intent: &ResetIntentRecord,
        profile: &GazeboCapabilityProfile,
        stepping_method: SteppingMethod,
    ) -> GazeboResetAttempt {
        GazeboResetAttempt {
            plan_sha256: intent.plan_sha256.clone(),
            profile: profile.clone(),
            stepping_method,
            prior_world_epoch: intent.prior_world_epoch.clone(),
            clock_id: intent.clock_id.clone(),
        }
    }

    /// Execute one reset with a synced journal intent before ResetSimulation.
    /// An existing reset identity is always reconciled read-only; it is never
    /// sent to ResetSimulation again.
    pub fn reset_and_verify_journaled<T, S>(
        transport: &mut T,
        store: &mut S,
        journal: &mut Journal,
        plan: &CompiledCampaignPlan,
        session: &mut GazeboAdapterSession,
        identity: GazeboResetRunIdentity<'_>,
    ) -> Result<GazeboJournaledResetResult, JournaledGazeboError>
    where
        T: GazeboSimulationTransport,
        S: GazeboEvidenceArtifactStore,
    {
        plan.verify_integrity()
            .map_err(|_| GazeboAdapterError::InvalidCompiledPlan)?;
        let reset_id = reset_idempotency_key(plan, session, identity)?;
        if journal
            .recovery_projection()
            .resets
            .iter()
            .any(|state| state.intent.reset_id == reset_id)
        {
            let snapshot = recover_reset_from_journal(
                transport,
                store,
                journal,
                plan,
                session,
                &reset_id,
                identity,
            )?;
            return Ok(GazeboJournaledResetResult {
                reset_id,
                snapshot,
                reconciled_after_ambiguity: true,
            });
        }

        if !matches!(session.reset_lifecycle, GazeboResetLifecycle::NotAttempted) {
            return Err(JournaledGazeboError::Adapter(
                GazeboAdapterError::ResetOutcomeUnresolved,
            ));
        }
        let live_profile = transport
            .get_simulator_features()
            .map_err(|_| GazeboAdapterError::TransportFailure)?;
        if live_profile != session.profile {
            return Err(JournaledGazeboError::Adapter(
                GazeboAdapterError::InvalidCapabilityProfile,
            ));
        }
        let method = session.profile.validate_for_plan(plan)?;
        if method != session.stepping_method {
            return Err(JournaledGazeboError::Adapter(
                GazeboAdapterError::InvalidCapabilityProfile,
            ));
        }
        let before = transport
            .get_world_state(&session.profile.world_name)
            .map_err(|_| GazeboAdapterError::TransportFailure)?;
        before.validate_against_profile(&session.profile)?;
        if before.world_epoch != session.world_epoch {
            return Err(JournaledGazeboError::Adapter(
                GazeboAdapterError::WrongWorldEpoch,
            ));
        }
        if before.clock_id != session.clock_id {
            return Err(JournaledGazeboError::Adapter(GazeboAdapterError::InvalidClock));
        }
        if before.state != SimulationState::Paused {
            return Err(JournaledGazeboError::Adapter(
                GazeboAdapterError::SimulatorNotPaused,
            ));
        }

        let intent = reset_intent_from_session(plan, session, &reset_id, identity);
        journal.append_reset_intent(intent.clone())?;
        let attempt = reset_attempt_from_intent(&intent, &session.profile, session.stepping_method);

        match super::reset_and_verify(transport, plan, session) {
            Ok(snapshot) => {
                let response = serde_json::json!({
                    "schema_version": "0.1",
                    "producer": "mobility-rrvr-gazebo-adapter",
                    "evidence_kind": "adapter_reset_return_envelope",
                    "operation": "ResetSimulation",
                    "result": "accepted",
                    "reset_id": reset_id,
                    "plan_sha256": plan.plan_sha256,
                    "world_id": session.profile.world_name,
                    "source_limit": "normalized adapter return; not a raw middleware packet",
                });
                let response_digest =
                    persist_json(store, "gazebo-reset-transport-response", &response);
                let response_status = if response_digest.is_ok() {
                    ResetOutcomeStatus::ResponseAccepted
                } else {
                    ResetOutcomeStatus::ResponseUnknown
                };
                journal.append_reset_outcome(ResetOutcomeRecord {
                    reset_id: reset_id.clone(),
                    source: ResetOutcomeSource::TransportResponse,
                    status: response_status,
                    observed_world_epoch: None,
                    observed_clock_id: None,
                    observed_initial_state_sha256: None,
                    observation_artifact_sha256: None,
                    transport_response_artifact_sha256: response_digest.ok(),
                })?;
                let observation_digest =
                    persist_json(store, "gazebo-reset-world-observation", &snapshot);
                match observation_digest {
                    Ok(observation_artifact_sha256) => {
                        journal.append_reset_outcome(ResetOutcomeRecord {
                            reset_id: reset_id.clone(),
                            source: ResetOutcomeSource::ReadOnlyObservation,
                            status: ResetOutcomeStatus::Verified,
                            observed_world_epoch: Some(snapshot.world_epoch.clone()),
                            observed_clock_id: Some(snapshot.clock_id.clone()),
                            observed_initial_state_sha256: Some(snapshot.state_sha256.clone()),
                            observation_artifact_sha256: Some(observation_artifact_sha256),
                            transport_response_artifact_sha256: None,
                        })?;
                        Ok(GazeboJournaledResetResult {
                            reset_id,
                            snapshot,
                            reconciled_after_ambiguity: false,
                        })
                    }
                    Err(error) => {
                        journal.append_reset_outcome(ResetOutcomeRecord {
                            reset_id: reset_id.clone(),
                            source: ResetOutcomeSource::ReadOnlyObservation,
                            status: ResetOutcomeStatus::Unresolved,
                            observed_world_epoch: Some(snapshot.world_epoch),
                            observed_clock_id: Some(snapshot.clock_id),
                            observed_initial_state_sha256: Some(snapshot.state_sha256),
                            observation_artifact_sha256: None,
                            transport_response_artifact_sha256: None,
                        })?;
                        Err(error)
                    }
                }
            }
            Err(error) => {
                // A transport error or post-call observation failure does not
                // establish NoEffect. Preserve Unknown and reconcile read-only.
                session.reset_lifecycle =
                    GazeboResetLifecycle::OutcomeUnresolved(attempt);
                let response = serde_json::json!({
                    "schema_version": "0.1",
                    "producer": "mobility-rrvr-gazebo-adapter",
                    "evidence_kind": "adapter_reset_return_envelope",
                    "operation": "ResetSimulation",
                    "result": "unknown",
                    "reset_id": reset_id,
                    "adapter_error": format!("{error:?}"),
                    "source_limit": "normalized adapter error; not raw middleware bytes",
                });
                let response_digest =
                    persist_json(store, "gazebo-reset-transport-response-unknown", &response)
                        .ok();
                journal.append_reset_outcome(ResetOutcomeRecord {
                    reset_id: reset_id.clone(),
                    source: ResetOutcomeSource::TransportResponse,
                    status: ResetOutcomeStatus::ResponseUnknown,
                    observed_world_epoch: None,
                    observed_clock_id: None,
                    observed_initial_state_sha256: None,
                    observation_artifact_sha256: None,
                    transport_response_artifact_sha256: response_digest,
                })?;
                match super::reconcile_reset(transport, plan, session) {
                    Ok(snapshot) => {
                        let digest = persist_json(
                            store,
                            "gazebo-reset-reconciliation-observation",
                            &snapshot,
                        )?;
                        journal.append_reset_outcome(ResetOutcomeRecord {
                            reset_id: reset_id.clone(),
                            source: ResetOutcomeSource::Reconciliation,
                            status: ResetOutcomeStatus::Verified,
                            observed_world_epoch: Some(snapshot.world_epoch.clone()),
                            observed_clock_id: Some(snapshot.clock_id.clone()),
                            observed_initial_state_sha256: Some(snapshot.state_sha256.clone()),
                            observation_artifact_sha256: Some(digest),
                            transport_response_artifact_sha256: None,
                        })?;
                        Ok(GazeboJournaledResetResult {
                            reset_id,
                            snapshot,
                            reconciled_after_ambiguity: true,
                        })
                    }
                    Err(reconcile_error) => {
                        let evidence = serde_json::json!({
                            "schema_version": "0.1",
                            "producer": "mobility-rrvr-gazebo-adapter",
                            "evidence_kind": "reset_reconciliation_unresolved",
                            "reset_id": reset_id,
                            "initial_error": format!("{error:?}"),
                            "reconciliation_error": format!("{reconcile_error:?}"),
                        });
                        let observation_artifact_sha256 = persist_json(
                            store,
                            "gazebo-reset-reconciliation-unresolved",
                            &evidence,
                        )
                        .ok();
                        journal.append_reset_outcome(ResetOutcomeRecord {
                            reset_id,
                            source: ResetOutcomeSource::Reconciliation,
                            status: ResetOutcomeStatus::Unresolved,
                            observed_world_epoch: None,
                            observed_clock_id: None,
                            observed_initial_state_sha256: None,
                            observation_artifact_sha256,
                            transport_response_artifact_sha256: None,
                        })?;
                        Err(JournaledGazeboError::Adapter(reconcile_error))
                    }
                }
            }
        }
    }

    /// Rehydrate reset qualification from verified journal history or reconcile
    /// an outstanding reset with read-only world observation. Never calls reset.
    pub fn recover_reset_from_journal<T, S>(
        transport: &mut T,
        store: &mut S,
        journal: &mut Journal,
        plan: &CompiledCampaignPlan,
        session: &mut GazeboAdapterSession,
        reset_id: &str,
        identity: GazeboResetRunIdentity<'_>,
    ) -> Result<GazeboWorldSnapshot, JournaledGazeboError>
    where
        T: GazeboSimulationTransport,
        S: GazeboEvidenceArtifactStore,
    {
        plan.verify_integrity()
            .map_err(|_| GazeboAdapterError::InvalidCompiledPlan)?;
        let state = journal
            .recovery_projection()
            .resets
            .into_iter()
            .find(|state| state.intent.reset_id == reset_id)
            .ok_or(JournaledGazeboError::InvalidJournalIdentity)?;
        let intent = state.intent;
        if intent.plan_sha256 != plan.plan_sha256
            || intent.simulator_profile_sha256 != plan.platform.profile_sha256
            || intent.world_id != session.profile.world_name
            || intent.expected_initial_state_sha256 != plan.reset.expected_initial_state_sha256
            || intent.run_id != identity.run_id
            || intent.run_epoch != identity.run_epoch
        {
            return Err(JournaledGazeboError::InvalidJournalIdentity);
        }
        let live_profile = transport
            .get_simulator_features()
            .map_err(|_| GazeboAdapterError::TransportFailure)?;
        if live_profile != session.profile {
            return Err(JournaledGazeboError::Adapter(
                GazeboAdapterError::InvalidCapabilityProfile,
            ));
        }
        let stepping_method = session.profile.validate_for_plan(plan)?;
        if let Some(outcome) = state.latest_outcome.as_ref().filter(|outcome| {
            outcome.status == ResetOutcomeStatus::Verified
        }) {
            let snapshot = transport
                .get_world_state(&session.profile.world_name)
                .map_err(|_| GazeboAdapterError::TransportFailure)?;
            snapshot.validate_reset(&session.profile, plan, &intent.prior_world_epoch)?;
            if snapshot.world_epoch != outcome.observed_world_epoch.as_deref().unwrap_or_default()
                || snapshot.clock_id != outcome.observed_clock_id.as_deref().unwrap_or_default()
                || snapshot.state_sha256
                    != outcome
                        .observed_initial_state_sha256
                        .as_deref()
                        .unwrap_or_default()
            {
                return Err(JournaledGazeboError::Adapter(
                    GazeboAdapterError::ResetVerificationMismatch,
                ));
            }
            session.world_epoch = snapshot.world_epoch.clone();
            session.clock_id = snapshot.clock_id.clone();
            session.reset_lifecycle = GazeboResetLifecycle::Verified(GazeboResetVerification {
                plan_sha256: intent.plan_sha256,
                profile: session.profile.clone(),
                stepping_method: stepping_method,
                world_epoch: session.world_epoch.clone(),
                clock_id: session.clock_id.clone(),
            });
            return Ok(snapshot);
        }

        session.world_epoch = intent.prior_world_epoch.clone();
        session.clock_id = intent.clock_id.clone();
        session.reset_lifecycle = GazeboResetLifecycle::OutcomeUnresolved(
            reset_attempt_from_intent(&intent, &session.profile, stepping_method),
        );
        match super::reconcile_reset(transport, plan, session) {
            Ok(snapshot) => {
                let digest =
                    persist_json(store, "gazebo-reset-restart-reconciliation", &snapshot)?;
                journal.append_reset_outcome(ResetOutcomeRecord {
                    reset_id: reset_id.to_owned(),
                    source: ResetOutcomeSource::Reconciliation,
                    status: ResetOutcomeStatus::Verified,
                    observed_world_epoch: Some(snapshot.world_epoch.clone()),
                    observed_clock_id: Some(snapshot.clock_id.clone()),
                    observed_initial_state_sha256: Some(snapshot.state_sha256.clone()),
                    observation_artifact_sha256: Some(digest),
                    transport_response_artifact_sha256: None,
                })?;
                Ok(snapshot)
            }
            Err(error) => {
                let evidence = serde_json::json!({
                    "schema_version": "0.1",
                    "producer": "mobility-rrvr-gazebo-adapter",
                    "evidence_kind": "reset_restart_reconciliation_unresolved",
                    "reset_id": reset_id,
                    "error": format!("{error:?}"),
                });
                let digest = persist_json(store, "gazebo-reset-restart-unresolved", &evidence).ok();
                journal.append_reset_outcome(ResetOutcomeRecord {
                    reset_id: reset_id.to_owned(),
                    source: ResetOutcomeSource::Reconciliation,
                    status: ResetOutcomeStatus::Unresolved,
                    observed_world_epoch: None,
                    observed_clock_id: None,
                    observed_initial_state_sha256: None,
                    observation_artifact_sha256: digest,
                    transport_response_artifact_sha256: None,
                })?;
                Err(JournaledGazeboError::Adapter(error))
            }
        }
    }

    fn request_from_intent(intent: &DispatchIntentRecord) -> GazeboDispatchRequest {
        GazeboDispatchRequest {
            plan_sha256: intent.plan_sha256.clone(),
            campaign_id: intent.campaign_id.clone(),
            pair_id: intent.pair_id.clone(),
            scenario_id: intent.scenario_id.clone(),
            fault_id: intent.fault_id.clone(),
            fault_sequence: intent.fault_sequence,
            world_epoch: intent.world_epoch.clone(),
            clock_id: intent.clock_id.clone(),
            target_id: intent.target_id.clone(),
            fault_domain_id: intent.fault_domain_id.clone(),
            scheduled_simulation_step: intent.scheduled_simulation_step,
            parameters_sha256: intent.parameters_sha256.clone(),
            payload_sha256: intent.payload_sha256.clone(),
            idempotency_key: intent.idempotency_key.clone(),
        }
    }

    fn prepare_dispatch_request<T: GazeboSimulationTransport>(
        simulator: &mut T,
        plan: &CompiledCampaignPlan,
        session: &GazeboAdapterSession,
        identity: GazeboJournaledDispatchIdentity<'_>,
    ) -> Result<(GazeboDispatchRequest, GazeboWorldSnapshot, GazeboTargetBinding), GazeboAdapterError> {
        plan.verify_integrity()
            .map_err(|_| GazeboAdapterError::InvalidCompiledPlan)?;
        let reset_verification = match &session.reset_lifecycle {
            GazeboResetLifecycle::Verified(verification) => verification,
            GazeboResetLifecycle::NotAttempted => {
                return Err(GazeboAdapterError::ResetNotVerified);
            }
            GazeboResetLifecycle::DispatchStarted(_) | GazeboResetLifecycle::OutcomeUnresolved(_) => {
                return Err(GazeboAdapterError::ResetOutcomeUnresolved);
            }
        };
        if reset_verification.plan_sha256 != plan.plan_sha256
            || reset_verification.profile != session.profile
            || reset_verification.stepping_method != session.stepping_method
            || reset_verification.world_epoch != session.world_epoch
            || reset_verification.clock_id != session.clock_id
        {
            return Err(GazeboAdapterError::ResetVerificationMismatch);
        }
        let scenario = plan
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == identity.scenario_id)
            .ok_or(GazeboAdapterError::ScenarioNotFound)?;
        if scenario.logical_effect_id.trim().is_empty() || identity.attempt_id.trim().is_empty() {
            return Err(GazeboAdapterError::InvalidIdempotencyContext);
        }
        let fault = scenario
            .fault_events
            .iter()
            .find(|fault| fault.fault_id == identity.fault_id)
            .ok_or(GazeboAdapterError::FaultNotFound)?;
        let live_profile = simulator
            .get_simulator_features()
            .map_err(|_| GazeboAdapterError::TransportFailure)?;
        if live_profile != session.profile {
            return Err(GazeboAdapterError::InvalidCapabilityProfile);
        }
        let stepping_method = live_profile.validate_for_plan(plan)?;
        if stepping_method != session.stepping_method {
            return Err(GazeboAdapterError::InvalidCapabilityProfile);
        }
        let cut_snapshot = super::advance_to_fault_cut(simulator, session, fault)?;
        let target_binding = super::resolve_fault_target(simulator, session, fault)?;
        let before_dispatch = simulator
            .get_world_state(&session.profile.world_name)
            .map_err(|_| GazeboAdapterError::TransportFailure)?;
        before_dispatch.validate_against_profile(&session.profile)?;
        if before_dispatch.world_epoch != session.world_epoch {
            return Err(GazeboAdapterError::WrongWorldEpoch);
        }
        if before_dispatch.clock_id != session.clock_id {
            return Err(GazeboAdapterError::InvalidClock);
        }
        if before_dispatch.state != SimulationState::Paused {
            return Err(GazeboAdapterError::SimulatorNotPaused);
        }
        if before_dispatch.simulation_step != fault.scheduled_simulation_step {
            return Err(GazeboAdapterError::SimulationStepNotReached);
        }
        if before_dispatch.simulation_time_ns != cut_snapshot.simulation_time_ns
            || before_dispatch.state_sha256 != cut_snapshot.state_sha256
        {
            return Err(GazeboAdapterError::WorldChangedAtCutPoint);
        }
        let refreshed_target = simulator
            .resolve_target(&session.profile.world_name, &fault.target_component_id)
            .map_err(|_| GazeboAdapterError::TransportFailure)?;
        super::validate_target_binding(
            &refreshed_target,
            fault,
            &session.profile.world_name,
            &session.world_epoch,
        )?;
        if refreshed_target != target_binding {
            return Err(GazeboAdapterError::TargetBindingChanged);
        }
        let idempotency_key = super::dispatch_idempotency_key(
            plan,
            scenario,
            fault,
            identity.run_id,
            identity.run_epoch,
            &session.world_epoch,
            &session.clock_id,
        )?;
        let mut request = GazeboDispatchRequest {
            plan_sha256: plan.plan_sha256.clone(),
            campaign_id: plan.campaign_id.clone(),
            pair_id: scenario.pair_id.clone(),
            scenario_id: scenario.scenario_id.clone(),
            fault_id: fault.fault_id.clone(),
            fault_sequence: fault.sequence,
            world_epoch: session.world_epoch.clone(),
            clock_id: session.clock_id.clone(),
            target_id: target_binding.target_id.clone(),
            fault_domain_id: fault.fault_domain_id.clone(),
            scheduled_simulation_step: fault.scheduled_simulation_step,
            parameters_sha256: fault.parameters_sha256.clone(),
            payload_sha256: String::new(),
            idempotency_key,
        };
        request.payload_sha256 = super::dispatch_request_fingerprint(&request)?;
        Ok((request, cut_snapshot, target_binding))
    }

    fn intent_from_request(
        request: &GazeboDispatchRequest,
        reset_id: &str,
        logical_effect_id: &str,
        attempt_id: &str,
        simulator_profile_sha256: &str,
        world_id: &str,
        run_id: &str,
        run_epoch: &str,
    ) -> DispatchIntentRecord {
        DispatchIntentRecord {
            idempotency_key: request.idempotency_key.clone(),
            payload_sha256: request.payload_sha256.clone(),
            reset_id: reset_id.to_owned(),
            plan_sha256: request.plan_sha256.clone(),
            simulator_profile_sha256: simulator_profile_sha256.to_owned(),
            world_id: world_id.to_owned(),
            logical_effect_id: logical_effect_id.to_owned(),
            attempt_id: attempt_id.to_owned(),
            campaign_id: request.campaign_id.clone(),
            pair_id: request.pair_id.clone(),
            scenario_id: request.scenario_id.clone(),
            fault_id: request.fault_id.clone(),
            fault_sequence: request.fault_sequence,
            run_id: run_id.to_owned(),
            run_epoch: run_epoch.to_owned(),
            world_epoch: request.world_epoch.clone(),
            clock_id: request.clock_id.clone(),
            target_id: request.target_id.clone(),
            fault_domain_id: request.fault_domain_id.clone(),
            scheduled_simulation_step: request.scheduled_simulation_step,
            parameters_sha256: request.parameters_sha256.clone(),
        }
    }

    fn transport_outcome_record<S: GazeboEvidenceArtifactStore>(
        store: &mut S,
        journal: &mut Journal,
        request: &GazeboDispatchRequest,
        outcome: &GazeboDispatchOutcome,
    ) -> Result<DispatchOutcomeStatus, JournaledGazeboError> {
        let (status, dispatch_id, label) = match outcome {
            GazeboDispatchOutcome::Accepted { dispatch_id } => (
                DispatchOutcomeStatus::ResponseAccepted,
                Some(dispatch_id.clone()),
                "accepted",
            ),
            GazeboDispatchOutcome::Rejected { dispatch_id, .. } => (
                DispatchOutcomeStatus::ResponseRejected,
                Some(dispatch_id.clone()),
                "rejected",
            ),
            GazeboDispatchOutcome::PayloadConflict { .. } => (
                DispatchOutcomeStatus::ResponseRejected,
                None,
                "payload_conflict",
            ),
            GazeboDispatchOutcome::Unknown { dispatch_id } => (
                DispatchOutcomeStatus::ResponseUnknown,
                dispatch_id.clone(),
                "unknown",
            ),
            GazeboDispatchOutcome::DuplicateSuppressed
            | GazeboDispatchOutcome::Reconciled { .. }
            | GazeboDispatchOutcome::InvalidDispatchRequest => (
                DispatchOutcomeStatus::ResponseUnknown,
                None,
                "ambiguous_adapter_outcome",
            ),
        };
        let artifact = serde_json::json!({
            "schema_version": "0.1",
            "producer": "mobility-rrvr-gazebo-adapter",
            "evidence_kind": "injector_transport_return_envelope",
            "source_limit": "normalized trait return; not raw network bytes or authenticated owner receipt",
            "idempotency_key": request.idempotency_key,
            "payload_sha256": request.payload_sha256,
            "result": label,
            "dispatch_id": dispatch_id,
        });
        let digest = persist_json(store, "gazebo-dispatch-transport-response", &artifact)?;
        journal.append_dispatch_outcome(DispatchOutcomeRecord {
            idempotency_key: request.idempotency_key.clone(),
            payload_sha256: request.payload_sha256.clone(),
            source: DispatchOutcomeSource::TransportResponse,
            status,
            dispatch_id,
            transport_response_artifact_sha256: Some(digest),
            ledger_artifact_sha256: None,
        })?;
        Ok(status)
    }

    fn lookup_and_record<I, S>(
        injector: &mut I,
        store: &mut S,
        journal: &mut Journal,
        request: &GazeboDispatchRequest,
    ) -> Result<GazeboDispatchOutcome, JournaledGazeboError>
    where
        I: GazeboFaultInjectorTransport,
        S: GazeboEvidenceArtifactStore,
    {
        let receipt = injector
            .lookup_dispatch_bound(request)
            .unwrap_or_else(|_| GazeboDispatchLedgerReceipt {
                payload_sha256: None,
                bound_payload_sha256: None,
                status: DispatchLedgerResult::Unknown,
                dispatch_id: None,
                raw_artifact_bytes: None,
            });
        let payload_bound =
            receipt.payload_sha256.as_deref() == Some(request.payload_sha256.as_str());
        let raw_owner_artifact = receipt.raw_artifact_bytes.is_some();
        let bytes = receipt.raw_artifact_bytes.clone().unwrap_or_else(|| {
            serde_json::to_vec(&serde_json::json!({
                "schema_version": "0.1",
                "producer": "mobility-rrvr-gazebo-adapter",
                "evidence_kind": "unqualified_owner_lookup_envelope",
                "source_limit": "legacy key-only lookup; not payload-bound owner evidence",
                "idempotency_key": request.idempotency_key,
                "expected_payload_sha256": request.payload_sha256,
                "returned_payload_sha256": receipt.payload_sha256,
                "owner_status": format!("{:?}", receipt.status),
                "raw_owner_artifact_present": false,
            }))
            .unwrap_or_default()
        });
        let ledger_artifact_sha256 =
            persist_bytes(store, "gazebo-dispatch-ledger-lookup", &bytes)?;

        // The compatibility lookup method is intentionally insufficient to
        // prove payload binding. Only an owner receipt carrying the exact
        // payload digest and its own artifact can produce a terminal verdict.
        let status = if !payload_bound || !raw_owner_artifact {
            DispatchOutcomeStatus::Unresolved
        } else {
            match receipt.status {
                DispatchLedgerResult::Accepted
                    if receipt.dispatch_id.as_deref().is_some_and(nonempty) =>
                {
                    DispatchOutcomeStatus::ReconciledAccepted
                }
                DispatchLedgerResult::Accepted => DispatchOutcomeStatus::Unresolved,
                DispatchLedgerResult::Rejected => DispatchOutcomeStatus::ReconciledRejected,
                DispatchLedgerResult::NoDispatchProven => DispatchOutcomeStatus::NoDispatchProven,
                DispatchLedgerResult::Unknown => DispatchOutcomeStatus::Unresolved,
                DispatchLedgerResult::PayloadConflict
                    if receipt
                        .bound_payload_sha256
                        .as_deref()
                        .is_some_and(|bound| valid_sha256(bound) && bound != request.payload_sha256) =>
                {
                    DispatchOutcomeStatus::PayloadConflict
                }
                DispatchLedgerResult::PayloadConflict => DispatchOutcomeStatus::Unresolved,
            }
        };
        journal.append_dispatch_outcome(DispatchOutcomeRecord {
            idempotency_key: request.idempotency_key.clone(),
            payload_sha256: request.payload_sha256.clone(),
            source: DispatchOutcomeSource::InjectorLedgerLookup,
            status,
            dispatch_id: if status == DispatchOutcomeStatus::ReconciledAccepted {
                receipt.dispatch_id.clone()
            } else {
                None
            },
            transport_response_artifact_sha256: None,
            ledger_artifact_sha256: Some(ledger_artifact_sha256),
        })?;
        Ok(match status {
            DispatchOutcomeStatus::ReconciledAccepted => GazeboDispatchOutcome::Reconciled {
                decision: ReconciliationDecision::ExistingDispatchAccepted,
            },
            DispatchOutcomeStatus::ReconciledRejected => GazeboDispatchOutcome::Reconciled {
                decision: ReconciliationDecision::ExistingDispatchRejected,
            },
            DispatchOutcomeStatus::NoDispatchProven => GazeboDispatchOutcome::Reconciled {
                decision: ReconciliationDecision::NoDispatchProven,
            },
            DispatchOutcomeStatus::PayloadConflict => GazeboDispatchOutcome::PayloadConflict {
                idempotency_key: request.idempotency_key.clone(),
            },
            _ => GazeboDispatchOutcome::Reconciled {
                decision: ReconciliationDecision::Unresolved,
            },
        })
    }

    fn outcome_from_existing(
        state: &crate::durable_journal::DispatchRecoveryState,
    ) -> Option<GazeboDispatchOutcome> {
        match state.disposition {
            DispatchRecoveryDisposition::AcceptedAwaitingExecutionEvidence => {
                state.latest_outcome.as_ref()?.dispatch_id.as_ref().map(|dispatch_id| {
                    GazeboDispatchOutcome::Accepted {
                        dispatch_id: dispatch_id.clone(),
                    }
                })
            }
            DispatchRecoveryDisposition::ReconciledAcceptedAwaitingExecutionEvidence => {
                Some(GazeboDispatchOutcome::Reconciled {
                    decision: ReconciliationDecision::ExistingDispatchAccepted,
                })
            }
            DispatchRecoveryDisposition::Rejected
            | DispatchRecoveryDisposition::ReconciledRejected => {
                Some(GazeboDispatchOutcome::Reconciled {
                    decision: ReconciliationDecision::ExistingDispatchRejected,
                })
            }
            DispatchRecoveryDisposition::NoDispatchProvenTerminal => {
                Some(GazeboDispatchOutcome::Reconciled {
                    decision: ReconciliationDecision::NoDispatchProven,
                })
            }
            DispatchRecoveryDisposition::PayloadConflictHold => {
                Some(GazeboDispatchOutcome::PayloadConflict {
                    idempotency_key: state.intent.idempotency_key.clone(),
                })
            }
            _ => None,
        }
    }

    /// Journal-first dispatch. For a new key, the simulator is advanced to the
    /// exact cut before the durable dispatch intent is appended; the injector
    /// call occurs only after append and sync. For an existing logical attempt,
    /// this function performs owner lookup only and never dispatches again.
    pub fn dispatch_planned_fault_once_journaled<T, I, S>(
        simulator: &mut T,
        injector: &mut I,
        store: &mut S,
        journal: &mut Journal,
        plan: &CompiledCampaignPlan,
        session: &GazeboAdapterSession,
        identity: GazeboJournaledDispatchIdentity<'_>,
    ) -> Result<GazeboJournaledDispatchResult, JournaledGazeboError>
    where
        T: GazeboSimulationTransport,
        I: GazeboFaultInjectorTransport,
        S: GazeboEvidenceArtifactStore,
    {
        plan.verify_integrity()
            .map_err(|_| GazeboAdapterError::InvalidCompiledPlan)?;
        let scenario = plan
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == identity.scenario_id)
            .ok_or(GazeboAdapterError::ScenarioNotFound)?;
        let fault = scenario
            .fault_events
            .iter()
            .find(|fault| fault.fault_id == identity.fault_id)
            .ok_or(GazeboAdapterError::FaultNotFound)?;
        let candidate_key = super::dispatch_idempotency_key(
            plan,
            scenario,
            fault,
            identity.run_id,
            identity.run_epoch,
            &session.world_epoch,
            &session.clock_id,
        )?;

        let projection = journal.recovery_projection();
        let same_attempt: Vec<_> = projection
            .dispatches
            .iter()
            .filter(|state| {
                state.intent.logical_effect_id == scenario.logical_effect_id
                    && state.intent.attempt_id == identity.attempt_id
                    && state.intent.plan_sha256 == plan.plan_sha256
                    && state.intent.run_id == identity.run_id
                    && state.intent.run_epoch == identity.run_epoch
                    && state.intent.scenario_id == identity.scenario_id
                    && state.intent.fault_id == identity.fault_id
            })
            .collect();
        if same_attempt.len() > 1 {
            return Err(JournaledGazeboError::ExistingAttemptConflict);
        }
        if let Some(existing) = same_attempt.first() {
            if existing.intent.idempotency_key != candidate_key
                || existing.intent.reset_id != identity.reset_id
                || existing.intent.world_epoch != session.world_epoch
                || existing.intent.clock_id != session.clock_id
                || existing.intent.payload_sha256.trim().is_empty()
            {
                return Err(JournaledGazeboError::ExistingAttemptConflict);
            }
            if let Some(outcome) = outcome_from_existing(existing) {
                return Ok(GazeboJournaledDispatchResult {
                    request: request_from_intent(&existing.intent),
                    outcome,
                    cut_snapshot: None,
                    target_binding: None,
                    recovered_from_journal: true,
                });
            }
            let request = request_from_intent(&existing.intent);
            let outcome = lookup_and_record(injector, store, journal, &request)?;
            return Ok(GazeboJournaledDispatchResult {
                request,
                outcome,
                cut_snapshot: None,
                target_binding: None,
                recovered_from_journal: true,
            });
        }

        // Reusing the same logical attempt in another world/reset generation is
        // not a fresh attempt. Only a new attempt identity may cross that boundary.
        if projection.dispatches.iter().any(|state| {
            state.intent.logical_effect_id == scenario.logical_effect_id
                && state.intent.attempt_id == identity.attempt_id
                && state.intent.run_id == identity.run_id
                && state.intent.run_epoch == identity.run_epoch
                && state.intent.plan_sha256 == plan.plan_sha256
        }) {
            return Err(JournaledGazeboError::ExistingAttemptConflict);
        }

        let reset = projection
            .resets
            .iter()
            .find(|state| state.intent.reset_id == identity.reset_id)
            .ok_or(JournaledGazeboError::InvalidJournalIdentity)?;
        if reset.disposition != ResetRecoveryDisposition::Verified
            || reset.intent.plan_sha256 != plan.plan_sha256
            || reset.intent.simulator_profile_sha256 != plan.platform.profile_sha256
            || reset.intent.world_id != session.profile.world_name
            || reset.intent.run_id != identity.run_id
            || reset.intent.run_epoch != identity.run_epoch
            || reset.intent.clock_id != session.clock_id
            || reset
                .latest_outcome
                .as_ref()
                .and_then(|outcome| outcome.observed_world_epoch.as_deref())
                != Some(session.world_epoch.as_str())
        {
            return Err(JournaledGazeboError::InvalidJournalIdentity);
        }

        let (request, cut_snapshot, target_binding) =
            prepare_dispatch_request(simulator, plan, session, identity)?;
        if request.idempotency_key != candidate_key {
            return Err(JournaledGazeboError::ExistingAttemptConflict);
        }
        let intent = intent_from_request(
            &request,
            identity.reset_id,
            &scenario.logical_effect_id,
            identity.attempt_id,
            &plan.platform.profile_sha256,
            &session.profile.world_name,
            identity.run_id,
            identity.run_epoch,
        );
        // Critical ordering boundary: a failed append or sync means the
        // injector method below is never invoked.
        journal.append_dispatch_intent(intent)?;

        match super::dispatch_planned_fault_once(
            simulator,
            injector,
            &mut DispatchAttemptRegistry::default(),
            plan,
            session,
            GazeboFaultRunIdentity {
                scenario_id: identity.scenario_id,
                fault_id: identity.fault_id,
                run_id: identity.run_id,
                run_epoch: identity.run_epoch,
            },
        ) {
            Ok(result) => {
                let response_status =
                    transport_outcome_record(store, journal, &result.request, &result.outcome)?;
                let final_outcome = if matches!(
                    response_status,
                    DispatchOutcomeStatus::ResponseUnknown | DispatchOutcomeStatus::ResponseRejected
                ) {
                    lookup_and_record(injector, store, journal, &result.request)?
                } else {
                    result.outcome.clone()
                };
                Ok(GazeboJournaledDispatchResult {
                    request: result.request,
                    outcome: final_outcome,
                    cut_snapshot: Some(result.cut_snapshot),
                    target_binding: Some(result.target_binding),
                    recovered_from_journal: false,
                })
            }
            Err(_adapter_error) => {
                // The planned adapter returns Err only before its injector
                // call. Query the owner read-only; never infer NoEffect.
                let outcome = lookup_and_record(injector, store, journal, &request)?;
                Ok(GazeboJournaledDispatchResult {
                    request,
                    outcome,
                    cut_snapshot: Some(cut_snapshot),
                    target_binding: Some(target_binding),
                    recovered_from_journal: false,
                })
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign_plan::CompiledCampaignPlan;
    use crate::fault_campaign::FaultCampaignManifest;

    fn plan() -> CompiledCampaignPlan {
        let manifest: FaultCampaignManifest =
            serde_json::from_str(include_str!("../fixtures/fault_campaign_paired_v0_1.json"))
                .expect("campaign fixture deserializes");
        let bytes = include_bytes!("../fixtures/fault_campaign_paired_v0_1.json");
        let digest = {
            use sha2::{Digest, Sha256};
            let value = Sha256::digest(bytes);
            let digest: String = value.iter().map(|byte| format!("{byte:02x}")).collect();
            digest
        };
        CompiledCampaignPlan::compile(&manifest, &digest).unwrap()
    }

    fn profile(plan: &CompiledCampaignPlan) -> GazeboCapabilityProfile {
        GazeboCapabilityProfile {
            version: GAZEBO_ADAPTER_CONTRACT_VERSION.into(),
            simulator_id: plan.platform.platform_id.clone(),
            simulator_revision: plan.platform.simulator_revision.clone(),
            ros_distribution: plan.platform.ros_distribution.clone(),
            simulation_interfaces_revision: plan.platform.simulation_interfaces_revision.clone(),
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

    struct MockGazebo {
        profile: GazeboCapabilityProfile,
        world: GazeboWorldSnapshot,
        initial_state_sha256: String,
        pause_calls: usize,
        reset_calls: usize,
        step_calls: Vec<(u64, SteppingMethod)>,
        target: GazeboTargetBinding,
        mutate_state_after_target_resolution: bool,
        reset_response_unknown: bool,
        reset_effect_applied: bool,
    }

    impl GazeboSimulationTransport for MockGazebo {
        type Error = &'static str;

        fn get_simulator_features(&mut self) -> Result<GazeboCapabilityProfile, Self::Error> {
            Ok(self.profile.clone())
        }

        fn get_world_state(
            &mut self,
            _world_name: &str,
        ) -> Result<GazeboWorldSnapshot, Self::Error> {
            Ok(self.world.clone())
        }

        fn pause_simulation(&mut self, _world_name: &str) -> Result<(), Self::Error> {
            self.pause_calls += 1;
            self.world.state = SimulationState::Paused;
            Ok(())
        }

        fn reset_simulation(&mut self, _world_name: &str) -> Result<(), Self::Error> {
            self.reset_calls += 1;
            if self.reset_effect_applied {
                self.world.world_epoch = format!("{}-reset", self.world.world_epoch);
                self.target.world_epoch = self.world.world_epoch.clone();
                self.world.simulation_step = 0;
                self.world.simulation_time_ns = 0;
                self.world.state = SimulationState::Paused;
                self.world.state_sha256 = self.initial_state_sha256.clone();
            }
            if self.reset_response_unknown {
                Err("reset may have completed before response loss")
            } else {
                Ok(())
            }
        }

        fn step_simulation(
            &mut self,
            _world_name: &str,
            steps: u64,
            method: SteppingMethod,
        ) -> Result<(), Self::Error> {
            self.step_calls.push((steps, method));
            self.world.simulation_step += steps;
            self.world.simulation_time_ns += steps * 10_000_000;
            Ok(())
        }

        fn resolve_target(
            &mut self,
            _world_name: &str,
            _logical_component_id: &str,
        ) -> Result<GazeboTargetBinding, Self::Error> {
            if self.mutate_state_after_target_resolution {
                self.world.state_sha256 = "b".repeat(64);
            }
            Ok(self.target.clone())
        }
    }

    fn mock_transport(plan: &CompiledCampaignPlan) -> MockGazebo {
        let profile = profile(plan);
        let fault = &plan.scenarios[0].fault_events[0];
        MockGazebo {
            profile: profile.clone(),
            world: GazeboWorldSnapshot {
                simulator_id: profile.simulator_id.clone(),
                simulator_revision: profile.simulator_revision.clone(),
                world_name: profile.world_name.clone(),
                world_epoch: "world-epoch-1".into(),
                clock_id: "sim-clock".into(),
                simulation_step: 50,
                simulation_time_ns: 500_000_000,
                state: SimulationState::Playing,
                state_sha256: "a".repeat(64),
            },
            initial_state_sha256: plan.reset.expected_initial_state_sha256.clone(),
            pause_calls: 0,
            reset_calls: 0,
            step_calls: Vec::new(),
            target: GazeboTargetBinding {
                world_name: profile.world_name.clone(),
                world_epoch: "world-epoch-1".into(),
                logical_component_id: fault.target_component_id.clone(),
                target_id: "gazebo-model-001".into(),
                target_revision: fault.target_revision.clone(),
                target_profile_sha256: fault.target_profile_sha256.clone(),
                fault_domain_id: fault.fault_domain_id.clone(),
            },
            mutate_state_after_target_resolution: false,
            reset_response_unknown: false,
            reset_effect_applied: true,
        }
    }

    struct MockInjector {
        dispatch_calls: usize,
        lookup: DispatchLedgerResult,
    }

    impl GazeboFaultInjectorTransport for MockInjector {
        type Error = &'static str;

        fn dispatch_once(
            &mut self,
            _request: &GazeboDispatchRequest,
        ) -> Result<GazeboDispatchOutcome, Self::Error> {
            self.dispatch_calls += 1;
            Err("connection dropped after injector may have accepted the command")
        }

        fn lookup_dispatch(
            &mut self,
            _idempotency_key: &str,
        ) -> Result<DispatchLedgerResult, Self::Error> {
            Ok(self.lookup)
        }
    }

    #[test]
    fn preflight_rejects_tampered_plan_before_simulator_discovery_or_pause() {
        let original = plan();
        let mut simulator = mock_transport(&original);
        let mut tampered = original.clone();
        tampered.plan_sha256 = "0".repeat(64);

        assert!(matches!(
            preflight_transport(&mut simulator, &tampered),
            Err(GazeboAdapterError::InvalidCompiledPlan)
        ));
        assert_eq!(simulator.pause_calls, 0);
        assert_eq!(simulator.reset_calls, 0);
    }

    #[test]
    fn planned_dispatch_rejects_preflight_only_session_before_transport_mutation() {
        let plan = plan();
        let mut simulator = mock_transport(&plan);
        let session = preflight_transport(&mut simulator, &plan).unwrap();
        let scenario = plan
            .scenarios
            .iter()
            .find(|scenario| !scenario.fault_events.is_empty())
            .unwrap();
        let fault = &scenario.fault_events[0];
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup: DispatchLedgerResult::Accepted,
        };
        let mut registry = DispatchAttemptRegistry::default();

        assert_eq!(
            dispatch_planned_fault_once(
                &mut simulator,
                &mut injector,
                &mut registry,
                &plan,
                &session,
                GazeboFaultRunIdentity {
                    scenario_id: &scenario.scenario_id,
                    fault_id: &fault.fault_id,
                    run_id: "run-no-reset",
                    run_epoch: "epoch-no-reset",
                }
            ),
            Err(GazeboAdapterError::ResetNotVerified)
        );
        assert_eq!(simulator.reset_calls, 0);
        assert!(simulator.step_calls.is_empty());
        assert_eq!(injector.dispatch_calls, 0);
    }

    #[test]
    fn planned_dispatch_rejects_a_different_valid_plan_after_reset() {
        let plan_a = plan();
        let mut simulator = mock_transport(&plan_a);
        let mut session = preflight_transport(&mut simulator, &plan_a).unwrap();
        reset_and_verify(&mut simulator, &plan_a, &mut session).unwrap();

        let mut manifest: FaultCampaignManifest =
            serde_json::from_str(include_str!("../fixtures/fault_campaign_paired_v0_1.json"))
                .unwrap();
        manifest.campaign_id.push_str("-alternate");
        let source_bytes = serde_json::to_vec(&manifest).unwrap();
        let source_digest = Sha256::digest(source_bytes);
        let source_digest: String = source_digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let plan_b = CompiledCampaignPlan::compile(&manifest, &source_digest).unwrap();
        assert_ne!(plan_a.plan_sha256, plan_b.plan_sha256);

        let scenario = plan_b
            .scenarios
            .iter()
            .find(|scenario| !scenario.fault_events.is_empty())
            .unwrap();
        let fault = &scenario.fault_events[0];
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup: DispatchLedgerResult::Accepted,
        };
        let mut registry = DispatchAttemptRegistry::default();

        assert_eq!(
            dispatch_planned_fault_once(
                &mut simulator,
                &mut injector,
                &mut registry,
                &plan_b,
                &session,
                GazeboFaultRunIdentity {
                    scenario_id: &scenario.scenario_id,
                    fault_id: &fault.fault_id,
                    run_id: "run-plan-substitution",
                    run_epoch: "epoch-plan-substitution",
                }
            ),
            Err(GazeboAdapterError::ResetVerificationMismatch)
        );
        assert!(simulator.step_calls.is_empty());
        assert_eq!(injector.dispatch_calls, 0);
    }

    #[test]
    fn planned_dispatch_rejects_session_epoch_mutation_after_reset() {
        let plan = plan();
        let mut simulator = mock_transport(&plan);
        let mut session = preflight_transport(&mut simulator, &plan).unwrap();
        reset_and_verify(&mut simulator, &plan, &mut session).unwrap();
        session.world_epoch.push_str("-tampered");
        let scenario = plan
            .scenarios
            .iter()
            .find(|scenario| !scenario.fault_events.is_empty())
            .unwrap();
        let fault = &scenario.fault_events[0];
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup: DispatchLedgerResult::Accepted,
        };
        let mut registry = DispatchAttemptRegistry::default();

        assert_eq!(
            dispatch_planned_fault_once(
                &mut simulator,
                &mut injector,
                &mut registry,
                &plan,
                &session,
                GazeboFaultRunIdentity {
                    scenario_id: &scenario.scenario_id,
                    fault_id: &fault.fault_id,
                    run_id: "run-mutated-session",
                    run_epoch: "epoch-mutated-session",
                }
            ),
            Err(GazeboAdapterError::ResetVerificationMismatch)
        );
        assert_eq!(injector.dispatch_calls, 0);
    }

    #[test]
    fn planned_dispatch_rechecks_live_simulator_profile_after_reset() {
        let plan = plan();
        let mut simulator = mock_transport(&plan);
        let mut session = preflight_transport(&mut simulator, &plan).unwrap();
        reset_and_verify(&mut simulator, &plan, &mut session).unwrap();
        simulator.profile.scene_broadcaster_sha256 = "d".repeat(64);
        let scenario = plan
            .scenarios
            .iter()
            .find(|scenario| !scenario.fault_events.is_empty())
            .unwrap();
        let fault = &scenario.fault_events[0];
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup: DispatchLedgerResult::Accepted,
        };
        let mut registry = DispatchAttemptRegistry::default();

        assert_eq!(
            dispatch_planned_fault_once(
                &mut simulator,
                &mut injector,
                &mut registry,
                &plan,
                &session,
                GazeboFaultRunIdentity {
                    scenario_id: &scenario.scenario_id,
                    fault_id: &fault.fault_id,
                    run_id: "run-profile-drift",
                    run_epoch: "epoch-profile-drift",
                }
            ),
            Err(GazeboAdapterError::InvalidCapabilityProfile)
        );
        assert_eq!(injector.dispatch_calls, 0);
    }

    #[test]
    fn reset_verification_is_single_use_and_not_repeatable() {
        let plan = plan();
        let mut simulator = mock_transport(&plan);
        let mut session = preflight_transport(&mut simulator, &plan).unwrap();
        reset_and_verify(&mut simulator, &plan, &mut session).unwrap();
        assert_eq!(simulator.reset_calls, 1);
        assert_eq!(
            reset_and_verify(&mut simulator, &plan, &mut session),
            Err(GazeboAdapterError::ResetAlreadyVerified)
        );
        assert_eq!(simulator.reset_calls, 1);
    }

    #[test]
    fn ambiguous_reset_response_suppresses_second_reset_and_reconciles_read_only() {
        let plan = plan();
        let mut simulator = mock_transport(&plan);
        simulator.reset_response_unknown = true;
        let mut session = preflight_transport(&mut simulator, &plan).unwrap();

        assert_eq!(
            reset_and_verify(&mut simulator, &plan, &mut session),
            Err(GazeboAdapterError::ResetOutcomeUnknown)
        );
        assert_eq!(simulator.reset_calls, 1);

        assert_eq!(
            reset_and_verify(&mut simulator, &plan, &mut session),
            Err(GazeboAdapterError::ResetOutcomeUnresolved)
        );
        assert_eq!(simulator.reset_calls, 1);

        let scenario = plan
            .scenarios
            .iter()
            .find(|scenario| !scenario.fault_events.is_empty())
            .unwrap();
        let fault = &scenario.fault_events[0];
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup: DispatchLedgerResult::Accepted,
        };
        let mut registry = DispatchAttemptRegistry::default();
        assert_eq!(
            dispatch_planned_fault_once(
                &mut simulator,
                &mut injector,
                &mut registry,
                &plan,
                &session,
                GazeboFaultRunIdentity {
                    scenario_id: &scenario.scenario_id,
                    fault_id: &fault.fault_id,
                    run_id: "run-reset-unknown",
                    run_epoch: "epoch-reset-unknown",
                }
            ),
            Err(GazeboAdapterError::ResetOutcomeUnresolved)
        );
        assert_eq!(injector.dispatch_calls, 0);
        assert!(simulator.step_calls.is_empty());

        // Read-only reconciliation observes that the reset actually happened.
        let reset_snapshot = reconcile_reset(&mut simulator, &plan, &mut session).unwrap();
        assert_eq!(simulator.reset_calls, 1);
        assert_eq!(session.world_epoch, reset_snapshot.world_epoch);
        assert_eq!(session.clock_id, reset_snapshot.clock_id);
        assert_eq!(
            reset_and_verify(&mut simulator, &plan, &mut session),
            Err(GazeboAdapterError::ResetAlreadyVerified)
        );
        assert_eq!(simulator.reset_calls, 1);
    }

    #[test]
    fn reset_reconciliation_does_not_infer_success_when_old_epoch_remains() {
        let plan = plan();
        let mut simulator = mock_transport(&plan);
        simulator.reset_response_unknown = true;
        simulator.reset_effect_applied = false;
        let mut session = preflight_transport(&mut simulator, &plan).unwrap();

        assert_eq!(
            reset_and_verify(&mut simulator, &plan, &mut session),
            Err(GazeboAdapterError::ResetOutcomeUnknown)
        );
        assert_eq!(simulator.reset_calls, 1);
        assert_eq!(
            reconcile_reset(&mut simulator, &plan, &mut session),
            Err(GazeboAdapterError::ResetOutcomeUnresolved)
        );
        assert_eq!(session.world_epoch, "world-epoch-1");
        assert_eq!(
            reset_and_verify(&mut simulator, &plan, &mut session),
            Err(GazeboAdapterError::ResetOutcomeUnresolved)
        );
        assert_eq!(simulator.reset_calls, 1);
    }

    #[test]
    fn planned_dispatch_rejects_tampered_compiled_plan_before_any_transport_call() {
        let original = plan();
        let mut simulator = mock_transport(&original);
        let session = preflight_transport(&mut simulator, &original).unwrap();
        let mut tampered = original.clone();
        tampered.plan_sha256 = "0".repeat(64);
        let scenario = original
            .scenarios
            .iter()
            .find(|s| !s.fault_events.is_empty())
            .unwrap();
        let fault = &scenario.fault_events[0];
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup: DispatchLedgerResult::Accepted,
        };
        let mut registry = DispatchAttemptRegistry::default();

        assert_eq!(
            dispatch_planned_fault_once(
                &mut simulator,
                &mut injector,
                &mut registry,
                &tampered,
                &session,
                GazeboFaultRunIdentity {
                    scenario_id: &scenario.scenario_id,
                    fault_id: &fault.fault_id,
                    run_id: "run-tampered",
                    run_epoch: "epoch-tampered",
                }
            ),
            Err(GazeboAdapterError::InvalidCompiledPlan)
        );
        assert_eq!(injector.dispatch_calls, 0);
    }

    #[test]
    fn planned_dispatch_is_bound_to_verified_plan_cut_and_target() {
        let plan = plan();
        let scenario = plan
            .scenarios
            .iter()
            .find(|s| !s.fault_events.is_empty())
            .unwrap();
        let fault = &scenario.fault_events[0];
        let mut simulator = mock_transport(&plan);
        let mut session = preflight_transport(&mut simulator, &plan).unwrap();
        reset_and_verify(&mut simulator, &plan, &mut session).unwrap();
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup: DispatchLedgerResult::Accepted,
        };
        let mut registry = DispatchAttemptRegistry::default();

        let result = dispatch_planned_fault_once(
            &mut simulator,
            &mut injector,
            &mut registry,
            &plan,
            &session,
            GazeboFaultRunIdentity {
                scenario_id: &scenario.scenario_id,
                fault_id: &fault.fault_id,
                run_id: "run-001",
                run_epoch: "run-epoch-001",
            },
        )
        .unwrap();

        assert_eq!(result.request.plan_sha256, plan.plan_sha256);
        assert_eq!(result.request.pair_id, scenario.pair_id);
        assert_eq!(result.request.scenario_id, scenario.scenario_id);
        assert_eq!(result.request.fault_id, fault.fault_id);
        assert_eq!(result.request.target_id, result.target_binding.target_id);
        assert_eq!(
            result.request.scheduled_simulation_step,
            fault.scheduled_simulation_step
        );
        assert_eq!(
            result.cut_snapshot.simulation_step,
            fault.scheduled_simulation_step
        );
        assert!(result.request.idempotency_key.starts_with("rrvr/v0.1/"));
        assert_eq!(
            result.request.idempotency_key.len(),
            "rrvr/v0.1/".len() + 64
        );
        assert_eq!(
            result.outcome,
            GazeboDispatchOutcome::Unknown { dispatch_id: None }
        );
        assert_eq!(injector.dispatch_calls, 1);
        assert_eq!(simulator.step_calls.len(), 1);
    }

    #[test]
    fn planned_dispatch_rejects_world_mutation_after_target_resolution() {
        let plan = plan();
        let scenario = plan
            .scenarios
            .iter()
            .find(|s| !s.fault_events.is_empty())
            .unwrap();
        let fault = &scenario.fault_events[0];
        let mut simulator = mock_transport(&plan);
        simulator.mutate_state_after_target_resolution = true;
        let mut session = preflight_transport(&mut simulator, &plan).unwrap();
        reset_and_verify(&mut simulator, &plan, &mut session).unwrap();
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup: DispatchLedgerResult::Accepted,
        };
        let mut registry = DispatchAttemptRegistry::default();

        assert_eq!(
            dispatch_planned_fault_once(
                &mut simulator,
                &mut injector,
                &mut registry,
                &plan,
                &session,
                GazeboFaultRunIdentity {
                    scenario_id: &scenario.scenario_id,
                    fault_id: &fault.fault_id,
                    run_id: "run-001",
                    run_epoch: "run-epoch-001",
                }
            ),
            Err(GazeboAdapterError::WorldChangedAtCutPoint)
        );
        assert_eq!(injector.dispatch_calls, 0);
    }

    #[test]
    fn planned_dispatch_rejects_scenario_or_fault_not_committed_by_plan() {
        let plan = plan();
        let mut simulator = mock_transport(&plan);
        let mut session = preflight_transport(&mut simulator, &plan).unwrap();
        reset_and_verify(&mut simulator, &plan, &mut session).unwrap();
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup: DispatchLedgerResult::Accepted,
        };
        let mut registry = DispatchAttemptRegistry::default();

        assert_eq!(
            dispatch_planned_fault_once(
                &mut simulator,
                &mut injector,
                &mut registry,
                &plan,
                &session,
                GazeboFaultRunIdentity {
                    scenario_id: "not-in-plan",
                    fault_id: "fault-process-crash",
                    run_id: "run-001",
                    run_epoch: "run-epoch-001",
                }
            ),
            Err(GazeboAdapterError::ScenarioNotFound)
        );
        let scenario = plan
            .scenarios
            .iter()
            .find(|s| !s.fault_events.is_empty())
            .unwrap();
        assert_eq!(
            dispatch_planned_fault_once(
                &mut simulator,
                &mut injector,
                &mut registry,
                &plan,
                &session,
                GazeboFaultRunIdentity {
                    scenario_id: &scenario.scenario_id,
                    fault_id: "not-in-plan",
                    run_id: "run-001",
                    run_epoch: "run-epoch-001",
                }
            ),
            Err(GazeboAdapterError::FaultNotFound)
        );
        assert_eq!(injector.dispatch_calls, 0);
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
        profile
            .capabilities
            .remove(&GazeboCapability::StepSimulation);
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
        profile
            .capabilities
            .remove(&GazeboCapability::StepSimulation);
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
            world_name: "rrvr-world".into(),
            world_epoch: "world-epoch-1".into(),
            logical_component_id: fault.target_component_id.clone(),
            target_id: "gazebo-model-001".into(),
            target_revision: fault.target_revision.clone(),
            target_profile_sha256: fault.target_profile_sha256.clone(),
            fault_domain_id: fault.fault_domain_id.clone(),
        };
        assert_eq!(
            validate_target_binding(&binding, fault, "rrvr-world", "world-epoch-1"),
            Ok(())
        );
        let mut wrong_target = binding;
        wrong_target.target_revision = "other-revision".into();
        assert_eq!(
            validate_target_binding(&wrong_target, fault, "rrvr-world", "world-epoch-1"),
            Err(GazeboAdapterError::TargetRevisionMismatch)
        );
    }

    #[test]
    fn capability_profile_rejects_scene_broadcaster_binary_drift() {
        let plan = plan();
        let mut wrong_plugin = profile(&plan);
        wrong_plugin.scene_broadcaster_sha256 = "f".repeat(64);
        assert_eq!(
            wrong_plugin.validate_for_plan(&plan),
            Err(GazeboAdapterError::SceneBroadcasterUnavailable)
        );

        let mut interfaces = profile(&plan);
        interfaces.simulation_interfaces_revision = "unreviewed-api".into();
        assert_eq!(
            interfaces.validate_for_plan(&plan),
            Err(GazeboAdapterError::PlatformRevisionMismatch)
        );
    }

    #[test]
    fn preflight_transport_pauses_before_returning_a_session() {
        let plan = plan();
        let mut transport = mock_transport(&plan);
        let session = preflight_transport(&mut transport, &plan).unwrap();
        assert_eq!(transport.pause_calls, 1);
        assert_eq!(session.world_epoch, "world-epoch-1");
        assert_eq!(transport.world.state, SimulationState::Paused);
    }

    #[test]
    fn reset_orchestration_verifies_epoch_and_initial_snapshot() {
        let plan = plan();
        let mut transport = mock_transport(&plan);
        let mut session = preflight_transport(&mut transport, &plan).unwrap();
        let reset = reset_and_verify(&mut transport, &plan, &mut session).unwrap();
        assert_eq!(transport.reset_calls, 1);
        assert_eq!(reset.world_epoch, "world-epoch-1-reset");
        assert_eq!(reset.simulation_step, 0);
        assert_eq!(reset.state_sha256, plan.reset.expected_initial_state_sha256);
        assert_eq!(session.world_epoch, reset.world_epoch);
    }

    #[test]
    fn step_orchestration_advances_to_the_exact_planned_iteration() {
        let plan = plan();
        let mut transport = mock_transport(&plan);
        let session = preflight_transport(&mut transport, &plan).unwrap();
        let fault = &plan.scenarios[0].fault_events[0];
        let result = advance_to_fault_cut(&mut transport, &session, fault).unwrap();
        assert_eq!(result.simulation_step, fault.scheduled_simulation_step);
        assert_eq!(
            transport.step_calls,
            vec![(
                fault.scheduled_simulation_step - 50,
                SteppingMethod::StepSimulationService
            )]
        );
    }

    #[test]
    fn target_resolution_uses_the_exact_planned_component_and_binding() {
        let plan = plan();
        let mut transport = mock_transport(&plan);
        let session = preflight_transport(&mut transport, &plan).unwrap();
        let fault = &plan.scenarios[0].fault_events[0];
        assert_eq!(
            resolve_fault_target(&mut transport, &session, fault),
            Ok(transport.target.clone())
        );
    }

    #[test]
    fn rejects_target_binding_from_a_stale_world_epoch() {
        let plan = plan();
        let fault = &plan.scenarios[0].fault_events[0];
        let binding = GazeboTargetBinding {
            world_name: "rrvr-world".into(),
            world_epoch: "old-world-epoch".into(),
            logical_component_id: fault.target_component_id.clone(),
            target_id: "gazebo-model-001".into(),
            target_revision: fault.target_revision.clone(),
            target_profile_sha256: fault.target_profile_sha256.clone(),
            fault_domain_id: fault.fault_domain_id.clone(),
        };
        assert_eq!(
            validate_target_binding(&binding, fault, "rrvr-world", "current-world-epoch"),
            Err(GazeboAdapterError::WrongWorldEpoch)
        );
    }

    fn observer_quorum(plan: &CompiledCampaignPlan) -> Vec<ObservedFaultSignature> {
        plan.platform
            .observer_profiles
            .iter()
            .take(plan.platform.required_delivery_observer_quorum)
            .map(|profile| ObservedFaultSignature {
                observer_id: profile.observer_id.clone(),
                observer_revision: profile.revision.clone(),
                observer_binary_sha256: profile.binary_sha256.clone(),
                common_cause_domains: profile.common_cause_domains.clone(),
                signature_sha256: plan.scenarios[0].fault_events[0]
                    .expected_delivery_signature_sha256
                    .clone(),
                world_epoch: "world-epoch-2".into(),
                simulation_step: plan.scenarios[0].fault_events[0].scheduled_simulation_step,
            })
            .collect()
    }

    #[test]
    fn qualifies_two_pinned_observers_with_disjoint_dependencies() {
        let plan = plan();
        let fault = &plan.scenarios[0].fault_events[0];
        let observations = observer_quorum(&plan);
        assert_eq!(
            validate_delivery_observer_quorum(
                &observations,
                &plan,
                &fault.fault_id,
                "world-epoch-2"
            ),
            Ok(DeliveryEvidenceVerdict::Confirmed)
        );
    }

    #[test]
    fn observer_quorum_refutes_unanimous_mismatch_and_abstains_on_disagreement() {
        let plan = plan();
        let fault = &plan.scenarios[0].fault_events[0];

        let mut unanimous_mismatch = observer_quorum(&plan);
        for observation in &mut unanimous_mismatch {
            observation.signature_sha256 = "f".repeat(64);
        }
        assert_eq!(
            validate_delivery_observer_quorum(
                &unanimous_mismatch,
                &plan,
                &fault.fault_id,
                "world-epoch-2"
            ),
            Ok(DeliveryEvidenceVerdict::Refuted)
        );

        let mut disagreement = observer_quorum(&plan);
        disagreement[0].signature_sha256 = "f".repeat(64);
        assert_eq!(
            validate_delivery_observer_quorum(
                &disagreement,
                &plan,
                &fault.fault_id,
                "world-epoch-2"
            ),
            Ok(DeliveryEvidenceVerdict::InsufficientEvidence)
        );
    }

    #[test]
    fn rejects_insufficient_or_duplicate_delivery_observers() {
        let plan = plan();
        let fault = &plan.scenarios[0].fault_events[0];
        let mut observations = observer_quorum(&plan);
        observations.pop();
        assert_eq!(
            validate_delivery_observer_quorum(
                &observations,
                &plan,
                &fault.fault_id,
                "world-epoch-2"
            ),
            Err(GazeboAdapterError::ObserverQuorumUnqualified)
        );

        let mut duplicate = observer_quorum(&plan);
        duplicate[1].observer_id = duplicate[0].observer_id.clone();
        assert_eq!(
            validate_delivery_observer_quorum(&duplicate, &plan, &fault.fault_id, "world-epoch-2"),
            Err(GazeboAdapterError::ObserverIdentityMismatch)
        );
    }

    #[test]
    fn rejects_shared_power_host_or_injector_failure_domains() {
        let plan = plan();
        let fault = &plan.scenarios[0].fault_events[0];
        let mut shared = observer_quorum(&plan);
        shared[1].common_cause_domains = shared[0].common_cause_domains.clone();
        assert_eq!(
            validate_delivery_observer_quorum(&shared, &plan, &fault.fault_id, "world-epoch-2"),
            Err(GazeboAdapterError::ObserverCommonCauseOverlap)
        );

        let mut injector = observer_quorum(&plan);
        injector[0].common_cause_domains =
            vec![plan.platform.fault_injector_fault_domain_id.clone()];
        assert_eq!(
            validate_delivery_observer_quorum(&injector, &plan, &fault.fault_id, "world-epoch-2"),
            Err(GazeboAdapterError::ObserverCommonCauseOverlap)
        );
    }

    #[test]
    fn rejects_unpinned_observer_binary_epoch_or_signature() {
        let plan = plan();
        let fault = &plan.scenarios[0].fault_events[0];
        let mut binary = observer_quorum(&plan);
        binary[0].observer_binary_sha256 = "c".repeat(64);
        assert_eq!(
            validate_delivery_observer_quorum(&binary, &plan, &fault.fault_id, "world-epoch-2"),
            Err(GazeboAdapterError::ObserverIdentityMismatch)
        );

        let mut late_step = observer_quorum(&plan);
        late_step[0].simulation_step += 1;
        assert_eq!(
            validate_delivery_observer_quorum(&late_step, &plan, &fault.fault_id, "world-epoch-2"),
            Err(GazeboAdapterError::ObserverStepMismatch)
        );

        let mut epoch = observer_quorum(&plan);
        epoch[0].world_epoch = "stale-world-epoch".into();
        assert_eq!(
            validate_delivery_observer_quorum(&epoch, &plan, &fault.fault_id, "world-epoch-2"),
            Err(GazeboAdapterError::ObserverEpochMismatch)
        );

        let mut signature = observer_quorum(&plan);
        signature[0].signature_sha256 = "f".repeat(64);
        assert_eq!(
            validate_delivery_observer_quorum(&signature, &plan, &fault.fault_id, "world-epoch-2"),
            Ok(DeliveryEvidenceVerdict::InsufficientEvidence)
        );
    }

    #[test]
    fn dispatch_request_fingerprint_matches_cross_language_known_answer() {
        let request = GazeboDispatchRequest {
            plan_sha256: "a".repeat(64),
            campaign_id: "campaign-1".into(),
            pair_id: "pair-1".into(),
            scenario_id: "scenario-1".into(),
            fault_id: "fault-1".into(),
            fault_sequence: 7,
            world_epoch: "world-epoch-2".into(),
            clock_id: "sim-clock-1".into(),
            target_id: "gazebo-model-001".into(),
            fault_domain_id: "injector-domain-1".into(),
            scheduled_simulation_step: 42,
            parameters_sha256: "b".repeat(64),
            payload_sha256: String::new(),
            idempotency_key: "rrvr/test-vector-001".into(),
        };

        let expected = dispatch_request_fingerprint(&request).unwrap();
        assert_eq!(
            expected,
            "4b03ee4bf55f363020a9d1df13c768942c9a5469428b5021653094eaaeac7175"
        );

        let mut changed_world_epoch = request.clone();
        changed_world_epoch.world_epoch = "world-epoch-3".into();
        assert_ne!(
            expected,
            dispatch_request_fingerprint(&changed_world_epoch).unwrap()
        );

        let mut changed_clock = request.clone();
        changed_clock.clock_id = "sim-clock-2".into();
        assert_ne!(
            expected,
            dispatch_request_fingerprint(&changed_clock).unwrap()
        );
    }

    #[test]
    fn idempotency_key_is_stable_and_bound_to_plan_run_and_fault() {
        let plan = plan();
        let scenario = &plan.scenarios[0];
        let fault = &scenario.fault_events[0];
        let first = dispatch_idempotency_key(
            &plan,
            scenario,
            fault,
            "run-1",
            "epoch-1",
            "world-epoch-2",
            "sim-clock-1",
        );
        let second = dispatch_idempotency_key(
            &plan,
            scenario,
            fault,
            "run-1",
            "epoch-1",
            "world-epoch-2",
            "sim-clock-1",
        );
        assert_eq!(first, second);
        assert_ne!(
            first,
            dispatch_idempotency_key(
                &plan,
                scenario,
                fault,
                "run-2",
                "epoch-1",
                "world-epoch-2",
                "sim-clock-1"
            )
        );
        let delimiter_left = dispatch_idempotency_key(
            &plan,
            scenario,
            fault,
            "run/a",
            "b",
            "world-epoch-2",
            "sim-clock-1",
        )
        .unwrap();
        let delimiter_right = dispatch_idempotency_key(
            &plan,
            scenario,
            fault,
            "run",
            "a/b",
            "world-epoch-2",
            "sim-clock-1",
        )
        .unwrap();
        assert_ne!(delimiter_left, delimiter_right);
        assert_ne!(
            first,
            dispatch_idempotency_key(
                &plan,
                scenario,
                fault,
                "run-1",
                "epoch-1",
                "world-epoch-3",
                "sim-clock-1"
            )
        );
        assert_ne!(
            first,
            dispatch_idempotency_key(
                &plan,
                scenario,
                fault,
                "run-1",
                "epoch-1",
                "world-epoch-2",
                "sim-clock-2"
            )
        );

        let mut uncommitted_scenario = scenario.clone();
        uncommitted_scenario.pair_id.push_str("-tampered");
        assert_eq!(
            dispatch_idempotency_key(
                &plan,
                &uncommitted_scenario,
                fault,
                "run-1",
                "epoch-1",
                "world-epoch-2",
                "sim-clock-1"
            ),
            Err(GazeboAdapterError::InvalidIdempotencyContext)
        );
    }

    #[test]
    fn lost_dispatch_response_remains_unknown_and_never_redispatches() {
        let plan = plan();
        let scenario = &plan.scenarios[0];
        let fault = &scenario.fault_events[0];
        let request = GazeboDispatchRequest {
            plan_sha256: plan.plan_sha256.clone(),
            campaign_id: plan.campaign_id.clone(),
            pair_id: scenario.pair_id.clone(),
            scenario_id: scenario.scenario_id.clone(),
            fault_id: fault.fault_id.clone(),
            fault_sequence: fault.sequence,
            world_epoch: "world-epoch-2".into(),
            clock_id: "sim-clock-1".into(),
            target_id: "gazebo-model-001".into(),
            fault_domain_id: fault.fault_domain_id.clone(),
            scheduled_simulation_step: fault.scheduled_simulation_step,
            parameters_sha256: fault.parameters_sha256.clone(),
            payload_sha256: String::new(),
            idempotency_key: dispatch_idempotency_key(
                &plan,
                scenario,
                fault,
                "run-1",
                "epoch-1",
                "world-epoch-2",
                "sim-clock-1",
            )
            .unwrap(),
        };
        let mut request = request;
        request.payload_sha256 = dispatch_request_fingerprint(&request).unwrap();
        let mut registry = DispatchAttemptRegistry::default();
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup: DispatchLedgerResult::Accepted,
        };
        assert_eq!(
            dispatch_fault_once(&mut registry, &mut injector, &request),
            GazeboDispatchOutcome::Unknown { dispatch_id: None }
        );
        assert_eq!(
            dispatch_fault_once(&mut registry, &mut injector, &request),
            GazeboDispatchOutcome::Unknown { dispatch_id: None }
        );
        assert_eq!(injector.dispatch_calls, 1);
        assert_eq!(
            reconcile_fault_dispatch(&mut registry, &mut injector, &request),
            ReconciliationDecision::ExistingDispatchAccepted
        );
        assert_eq!(injector.dispatch_calls, 1);
        assert_eq!(
            dispatch_fault_once(&mut registry, &mut injector, &request),
            GazeboDispatchOutcome::Reconciled {
                decision: ReconciliationDecision::ExistingDispatchAccepted
            }
        );
        assert_eq!(injector.dispatch_calls, 1);

        let mut changed_payload = request.clone();
        changed_payload.target_id = "different-target".into();
        changed_payload.payload_sha256 = dispatch_request_fingerprint(&changed_payload).unwrap();
        assert_eq!(
            dispatch_fault_once(&mut registry, &mut injector, &changed_payload),
            GazeboDispatchOutcome::PayloadConflict {
                idempotency_key: request.idempotency_key.clone()
            }
        );
        assert_eq!(injector.dispatch_calls, 1);
    }

    #[test]
    fn reconciliation_is_bound_to_registered_payload_and_unknown_stays_blocked() {
        let plan = plan();
        let scenario = &plan.scenarios[0];
        let fault = &scenario.fault_events[0];
        let mut request = GazeboDispatchRequest {
            plan_sha256: plan.plan_sha256.clone(),
            campaign_id: plan.campaign_id.clone(),
            pair_id: scenario.pair_id.clone(),
            scenario_id: scenario.scenario_id.clone(),
            fault_id: fault.fault_id.clone(),
            fault_sequence: fault.sequence,
            world_epoch: "world-epoch-2".into(),
            clock_id: "sim-clock-1".into(),
            target_id: "gazebo-model-001".into(),
            fault_domain_id: fault.fault_domain_id.clone(),
            scheduled_simulation_step: fault.scheduled_simulation_step,
            parameters_sha256: fault.parameters_sha256.clone(),
            payload_sha256: String::new(),
            idempotency_key: dispatch_idempotency_key(
                &plan,
                scenario,
                fault,
                "run-reconcile",
                "epoch-1",
                "world-epoch-2",
                "sim-clock-1",
            )
            .unwrap(),
        };
        request.payload_sha256 = dispatch_request_fingerprint(&request).unwrap();
        let mut registry = DispatchAttemptRegistry::default();
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup: DispatchLedgerResult::Unknown,
        };
        assert_eq!(
            dispatch_fault_once(&mut registry, &mut injector, &request),
            GazeboDispatchOutcome::Unknown { dispatch_id: None }
        );
        assert_eq!(
            reconcile_fault_dispatch(&mut registry, &mut injector, &request),
            ReconciliationDecision::Unresolved
        );
        assert_eq!(
            dispatch_fault_once(&mut registry, &mut injector, &request),
            GazeboDispatchOutcome::Reconciled {
                decision: ReconciliationDecision::Unresolved
            }
        );
        assert_eq!(injector.dispatch_calls, 1);

        injector.lookup = DispatchLedgerResult::NoDispatchProven;
        assert_eq!(
            reconcile_fault_dispatch(&mut registry, &mut injector, &request),
            ReconciliationDecision::NoDispatchProven
        );
        assert_eq!(
            dispatch_fault_once(&mut registry, &mut injector, &request),
            GazeboDispatchOutcome::Reconciled {
                decision: ReconciliationDecision::NoDispatchProven
            }
        );
        assert_eq!(injector.dispatch_calls, 1);

        let mut altered = request.clone();
        altered.target_id = "other-target".into();
        altered.payload_sha256 = dispatch_request_fingerprint(&altered).unwrap();
        assert_eq!(
            reconcile_fault_dispatch(&mut registry, &mut injector, &altered),
            ReconciliationDecision::PayloadConflict
        );
        assert_eq!(injector.dispatch_calls, 1);
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
        assert_eq!(
            reconcile_dispatch_lookup(DispatchLedgerResult::PayloadConflict),
            ReconciliationDecision::PayloadConflict
        );
    }

    struct DurableLedgerInjector {
        entries: BTreeMap<String, String>,
        effects: usize,
    }

    impl GazeboFaultInjectorTransport for DurableLedgerInjector {
        type Error = &'static str;

        fn dispatch_once(
            &mut self,
            request: &GazeboDispatchRequest,
        ) -> Result<GazeboDispatchOutcome, Self::Error> {
            let fingerprint =
                dispatch_request_fingerprint(request).map_err(|_| "invalid request")?;
            if fingerprint != request.payload_sha256 {
                return Ok(GazeboDispatchOutcome::InvalidDispatchRequest);
            }
            if let Some(previous) = self.entries.get(&request.idempotency_key) {
                return Ok(if previous == &fingerprint {
                    GazeboDispatchOutcome::DuplicateSuppressed
                } else {
                    GazeboDispatchOutcome::PayloadConflict {
                        idempotency_key: request.idempotency_key.clone(),
                    }
                });
            }
            self.entries
                .insert(request.idempotency_key.clone(), fingerprint);
            self.effects += 1;
            Ok(GazeboDispatchOutcome::Accepted {
                dispatch_id: format!("dispatch-{}", self.effects),
            })
        }

        fn lookup_dispatch(
            &mut self,
            idempotency_key: &str,
        ) -> Result<DispatchLedgerResult, Self::Error> {
            Ok(if self.entries.contains_key(idempotency_key) {
                DispatchLedgerResult::Accepted
            } else {
                DispatchLedgerResult::NoDispatchProven
            })
        }
    }

    #[test]
    fn durable_injector_ledger_rejects_payload_change_after_coordinator_restart() {
        let plan = plan();
        let scenario = &plan.scenarios[0];
        let fault = &scenario.fault_events[0];
        let mut request = GazeboDispatchRequest {
            plan_sha256: plan.plan_sha256.clone(),
            campaign_id: plan.campaign_id.clone(),
            pair_id: scenario.pair_id.clone(),
            scenario_id: scenario.scenario_id.clone(),
            fault_id: fault.fault_id.clone(),
            fault_sequence: fault.sequence,
            world_epoch: "world-epoch-2".into(),
            clock_id: "sim-clock-1".into(),
            target_id: "gazebo-model-001".into(),
            fault_domain_id: fault.fault_domain_id.clone(),
            scheduled_simulation_step: fault.scheduled_simulation_step,
            parameters_sha256: fault.parameters_sha256.clone(),
            payload_sha256: String::new(),
            idempotency_key: dispatch_idempotency_key(
                &plan,
                scenario,
                fault,
                "run-1",
                "epoch-1",
                "world-epoch-2",
                "sim-clock-1",
            )
            .unwrap(),
        };
        request.payload_sha256 = dispatch_request_fingerprint(&request).unwrap();
        let mut durable = DurableLedgerInjector {
            entries: BTreeMap::new(),
            effects: 0,
        };

        let mut first_process_registry = DispatchAttemptRegistry::default();
        assert!(matches!(
            dispatch_fault_once(&mut first_process_registry, &mut durable, &request),
            GazeboDispatchOutcome::Accepted { .. }
        ));
        assert_eq!(durable.effects, 1);
        assert_eq!(
            dispatch_fault_once(&mut first_process_registry, &mut durable, &request),
            GazeboDispatchOutcome::DuplicateSuppressed
        );
        assert_eq!(durable.effects, 1);

        // A new coordinator has an empty local registry, but the injector's
        // durable key-to-payload mapping survives and rejects the changed target.
        let mut restarted_process_registry = DispatchAttemptRegistry::default();
        request.target_id = "different-target".into();
        request.payload_sha256 = dispatch_request_fingerprint(&request).unwrap();
        let expected_conflict = GazeboDispatchOutcome::PayloadConflict {
            idempotency_key: request.idempotency_key.clone(),
        };
        assert_eq!(
            dispatch_fault_once(&mut restarted_process_registry, &mut durable, &request),
            expected_conflict
        );
        assert_eq!(
            dispatch_fault_once(&mut restarted_process_registry, &mut durable, &request),
            expected_conflict
        );
        assert_eq!(durable.effects, 1);
    }

    #[derive(Default)]
    struct MemoryGazeboArtifactStore {
        artifacts: BTreeMap<String, Vec<u8>>,
    }

    impl journaled::GazeboEvidenceArtifactStore for MemoryGazeboArtifactStore {
        type Error = &'static str;

        fn persist(&mut self, name: &str, bytes: &[u8]) -> Result<String, Self::Error> {
            use sha2::{Digest, Sha256};
            self.artifacts.insert(name.to_owned(), bytes.to_vec());
            let digest = Sha256::digest(bytes);
            Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
        }
    }

    fn journaled_test_path(label: &str) -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rrvr-journaled-{label}-{}-{nonce}.jsonl",
            std::process::id()
        ))
    }

    fn cleanup_journaled_test_path(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let mut lock = path.as_os_str().to_os_string();
        lock.push(".writer.lock");
        let _ = std::fs::remove_file(std::path::PathBuf::from(lock));
    }

    struct JournalObservingGazebo {
        inner: MockGazebo,
        journal_path: std::path::PathBuf,
    }

    impl GazeboSimulationTransport for JournalObservingGazebo {
        type Error = &'static str;

        fn get_simulator_features(&mut self) -> Result<GazeboCapabilityProfile, Self::Error> {
            self.inner.get_simulator_features()
        }

        fn get_world_state(
            &mut self,
            world_name: &str,
        ) -> Result<GazeboWorldSnapshot, Self::Error> {
            self.inner.get_world_state(world_name)
        }

        fn pause_simulation(&mut self, world_name: &str) -> Result<(), Self::Error> {
            self.inner.pause_simulation(world_name)
        }

        fn reset_simulation(&mut self, world_name: &str) -> Result<(), Self::Error> {
            let summary = crate::durable_journal::verify_file(&self.journal_path)
                .expect("reset intent must be readable and synced before ResetSimulation");
            assert_eq!(summary.reset_intent_count, 1);
            self.inner.reset_simulation(world_name)
        }

        fn step_simulation(
            &mut self,
            world_name: &str,
            steps: u64,
            method: SteppingMethod,
        ) -> Result<(), Self::Error> {
            self.inner.step_simulation(world_name, steps, method)
        }

        fn resolve_target(
            &mut self,
            world_name: &str,
            logical_component_id: &str,
        ) -> Result<GazeboTargetBinding, Self::Error> {
            self.inner.resolve_target(world_name, logical_component_id)
        }
    }

    #[test]
    fn journaled_reset_is_synced_before_the_simulator_mutator() {
        let plan = plan();
        let path = journaled_test_path("reset-order");
        let mut journal = crate::durable_journal::Journal::open(&path).unwrap();
        let mut store = MemoryGazeboArtifactStore::default();
        let mut simulator = JournalObservingGazebo {
            inner: mock_transport(&plan),
            journal_path: path.clone(),
        };
        let mut session = preflight_transport(&mut simulator, &plan).unwrap();
        let result = journaled::reset_and_verify_journaled(
            &mut simulator,
            &mut store,
            &mut journal,
            &plan,
            &mut session,
            journaled::GazeboResetRunIdentity {
                run_id: "run-journaled-reset",
                run_epoch: "epoch-1",
            },
        )
        .unwrap();
        assert_eq!(simulator.inner.reset_calls, 1);
        assert_eq!(journal.summary().reset_verified_count, 1);
        assert_eq!(result.snapshot.world_epoch, session.world_epoch);
        drop(journal);
        cleanup_journaled_test_path(&path);
    }

    #[test]
    fn reset_identity_is_stable_when_rehydrated_after_world_epoch_advances() {
        let plan = plan();
        let path = journaled_test_path("reset-rehydrate");
        let mut journal = crate::durable_journal::Journal::open(&path).unwrap();
        let mut store = MemoryGazeboArtifactStore::default();
        let mut simulator = mock_transport(&plan);
        let mut first_session = preflight_transport(&mut simulator, &plan).unwrap();
        let identity = journaled::GazeboResetRunIdentity {
            run_id: "run-reset-rehydrate",
            run_epoch: "epoch-1",
        };
        let first_key =
            journaled::reset_idempotency_key(&plan, &first_session, identity).unwrap();
        let reset = journaled::reset_and_verify_journaled(
            &mut simulator,
            &mut store,
            &mut journal,
            &plan,
            &mut first_session,
            identity,
        )
        .unwrap();
        let mut restarted_session =
            journaled::preflight_transport_read_only(&mut simulator, &plan).unwrap();
        let restarted_key =
            journaled::reset_idempotency_key(&plan, &restarted_session, identity).unwrap();
        assert_eq!(first_key, restarted_key);
        let snapshot = journaled::recover_reset_from_journal(
            &mut simulator,
            &mut store,
            &mut journal,
            &plan,
            &mut restarted_session,
            &reset.reset_id,
            identity,
        )
        .unwrap();
        assert_eq!(snapshot.world_epoch, reset.snapshot.world_epoch);
        assert_eq!(simulator.reset_calls, 1);
        drop(journal);
        cleanup_journaled_test_path(&path);
    }

    #[test]
    fn journaled_dispatch_is_not_redispatched_after_coordinator_restart() {
        let plan = plan();
        let path = journaled_test_path("dispatch-restart");
        let mut store = MemoryGazeboArtifactStore::default();
        let mut simulator = mock_transport(&plan);
        let mut session = preflight_transport(&mut simulator, &plan).unwrap();
        let mut journal = crate::durable_journal::Journal::open(&path).unwrap();
        let reset = journaled::reset_and_verify_journaled(
            &mut simulator,
            &mut store,
            &mut journal,
            &plan,
            &mut session,
            journaled::GazeboResetRunIdentity {
                run_id: "run-dispatch-restart",
                run_epoch: "epoch-1",
            },
        )
        .unwrap();
        let scenario = plan
            .scenarios
            .iter()
            .find(|scenario| !scenario.fault_events.is_empty())
            .unwrap();
        let fault = &scenario.fault_events[0];
        let identity = journaled::GazeboJournaledDispatchIdentity {
            reset_id: &reset.reset_id,
            scenario_id: &scenario.scenario_id,
            fault_id: &fault.fault_id,
            run_id: "run-dispatch-restart",
            run_epoch: "epoch-1",
            attempt_id: "attempt-1",
        };
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup: DispatchLedgerResult::Unknown,
        };
        let first = journaled::dispatch_planned_fault_once_journaled(
            &mut simulator,
            &mut injector,
            &mut store,
            &mut journal,
            &plan,
            &session,
            identity,
        )
        .unwrap();
        assert!(matches!(
            first.outcome,
            GazeboDispatchOutcome::Reconciled {
                decision: ReconciliationDecision::Unresolved
            }
        ));
        assert_eq!(injector.dispatch_calls, 1);
        drop(journal);

        let mut reopened = crate::durable_journal::Journal::open(&path).unwrap();
        let second = journaled::dispatch_planned_fault_once_journaled(
            &mut simulator,
            &mut injector,
            &mut store,
            &mut reopened,
            &plan,
            &session,
            identity,
        )
        .unwrap();
        assert!(second.recovered_from_journal);
        assert_eq!(injector.dispatch_calls, 1);
        assert_eq!(reopened.summary().dispatch_intent_count, 1);
        drop(reopened);
        cleanup_journaled_test_path(&path);
    }

}
