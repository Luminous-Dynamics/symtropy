// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Evidence contract between a compiled RRVR plan and fault-delivery records.
//!
//! Dispatch acceptance, injector execution and independently observed delivery
//! are separate facts. This module validates their linkage; it does not
//! authenticate producers, execute injectors, or grant retry/actuation authority.

use crate::campaign_plan::{CompiledCampaignPlan, CompiledPlatformBinding};
use crate::fault_campaign::{BenchmarkArm, FaultCutPoint, FaultInjection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const EXECUTION_EVIDENCE_VERSION: &str = "0.1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetStatus {
    Confirmed,
    Rejected,
    Unknown,
}

impl ResetStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Rejected => "rejected",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetResolutionStatus {
    Resolved,
    NotResolved,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchStatus {
    Accepted,
    Rejected,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Executed,
    NotExecuted,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryVerdict {
    Confirmed,
    Refuted,
    Unknown,
    NotObserved,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidencePhase {
    TargetResolution,
    Dispatch,
    Execution,
    DeliveryObservation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResetEvidenceReceipt {
    pub evidence_id: String,
    pub producer_id: String,
    pub producer_revision: String,
    pub raw_artifact_id: String,
    pub raw_artifact_sha256: String,
    pub run_id: String,
    pub run_epoch: String,
    pub world_epoch: String,
    pub reset_profile_sha256: String,
    pub expected_initial_state_sha256: String,
    pub observed_initial_state_sha256: String,
    pub simulation_step: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResetObservation {
    pub status: ResetStatus,
    pub receipt: Option<ResetEvidenceReceipt>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaultEvidenceReceipt {
    pub phase: EvidencePhase,
    pub evidence_id: String,
    pub producer_id: String,
    pub producer_revision: String,
    pub raw_artifact_id: String,
    pub raw_artifact_sha256: String,
    pub run_id: String,
    pub run_epoch: String,
    pub world_epoch: String,
    pub fault_id: String,
    pub fault_sequence: u32,
    pub cut_point: FaultCutPoint,
    pub target_id: String,
    pub clock_id: String,
    pub simulation_step: u64,
    pub simulation_time_ns: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetResolution {
    pub status: TargetResolutionStatus,
    pub logical_component_id: String,
    pub resolved_target_id: Option<String>,
    pub resolved_target_revision: Option<String>,
    pub resolved_target_profile_sha256: Option<String>,
    pub resolved_fault_domain_id: Option<String>,
    pub receipt: Option<FaultEvidenceReceipt>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InjectionDispatch {
    pub status: DispatchStatus,
    pub dispatch_id: String,
    pub idempotency_key: String,
    pub receipt: Option<FaultEvidenceReceipt>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InjectorExecution {
    pub status: ExecutionStatus,
    pub receipt: Option<FaultEvidenceReceipt>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaultDeliveryObservation {
    pub verdict: DeliveryVerdict,
    pub observer_id: Option<String>,
    pub observer_revision: Option<String>,
    pub observer_fault_domain_id: Option<String>,
    pub observed_fault_signature_sha256: Option<String>,
    pub receipt: Option<FaultEvidenceReceipt>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaultExecutionEvent {
    pub planned_fault: FaultInjection,
    pub target_resolution: TargetResolution,
    pub dispatch: InjectionDispatch,
    pub execution: InjectorExecution,
    pub delivery_observation: FaultDeliveryObservation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionEvidenceBundle {
    pub version: String,
    pub campaign_id: String,
    pub plan_sha256: String,
    pub source_manifest_sha256: String,
    pub pair_id: String,
    pub scenario_id: String,
    pub arm: BenchmarkArm,
    pub run_id: String,
    pub run_epoch: String,
    pub world_epoch: String,
    pub seed: u64,
    pub platform: CompiledPlatformBinding,
    pub reset: ResetObservation,
    pub events: Vec<FaultExecutionEvent>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionEvidenceError {
    InvalidHeader,
    PlanBindingMismatch,
    ScenarioNotFound,
    InvalidResetReceipt,
    ResetNotConfirmed,
    EventCountMismatch,
    PlannedFaultMismatch,
    InvalidTargetResolution,
    TargetRevisionMismatch,
    InvalidDispatch,
    InvalidExecution,
    InvalidDeliveryObservation,
    DeliverySignatureMismatch,
    ObserverNotDistinct,
    InvalidReceipt,
    DuplicateEvidenceIdentity,
    DuplicateDispatchIdentity,
    ExecutionCoordinateMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionSummary {
    pub scenario_id: String,
    pub reset_status: ResetStatus,
    pub planned_fault_count: usize,
    pub dispatch_accepted_count: usize,
    pub dispatch_rejected_count: usize,
    pub dispatch_unknown_count: usize,
    pub executed_count: usize,
    pub not_executed_count: usize,
    pub execution_unknown_count: usize,
    pub delivery_confirmed_count: usize,
    pub delivery_refuted_count: usize,
    pub delivery_unknown_count: usize,
    pub delivery_not_observed_count: usize,
    pub reconcile_required: bool,
    /// This validator never issues native retry or actuation authority.
    pub retry_permitted: bool,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_receipt(
    receipt: &FaultEvidenceReceipt,
    phase: EvidencePhase,
    bundle: &ExecutionEvidenceBundle,
    fault: &FaultInjection,
    target_id: &str,
) -> Result<(), ExecutionEvidenceError> {
    if receipt.phase != phase
        || !nonempty(&receipt.evidence_id)
        || !nonempty(&receipt.producer_id)
        || !nonempty(&receipt.producer_revision)
        || !nonempty(&receipt.raw_artifact_id)
        || !valid_sha256(&receipt.raw_artifact_sha256)
        || receipt.run_id != bundle.run_id
        || receipt.run_epoch != bundle.run_epoch
        || receipt.world_epoch != bundle.world_epoch
        || receipt.fault_id != fault.fault_id
        || receipt.fault_sequence != fault.sequence
        || receipt.cut_point != fault.cut_point
        || receipt.target_id != target_id
        || !nonempty(&receipt.clock_id)
    {
        return Err(ExecutionEvidenceError::InvalidReceipt);
    }
    Ok(())
}

fn validate_reset(
    bundle: &ExecutionEvidenceBundle,
    plan: &CompiledCampaignPlan,
) -> Result<(), ExecutionEvidenceError> {
    match bundle.reset.status {
        ResetStatus::Unknown => {
            if bundle.reset.receipt.is_some() || !bundle.events.is_empty() {
                return Err(ExecutionEvidenceError::ResetNotConfirmed);
            }
            return Ok(());
        }
        ResetStatus::Rejected | ResetStatus::Confirmed => {}
    }

    let receipt = bundle
        .reset
        .receipt
        .as_ref()
        .ok_or(ExecutionEvidenceError::InvalidResetReceipt)?;
    if !nonempty(&receipt.evidence_id)
        || !nonempty(&receipt.producer_id)
        || !nonempty(&receipt.producer_revision)
        || !nonempty(&receipt.raw_artifact_id)
        || !valid_sha256(&receipt.raw_artifact_sha256)
        || receipt.run_id != bundle.run_id
        || receipt.run_epoch != bundle.run_epoch
        || receipt.world_epoch != bundle.world_epoch
        || receipt.reset_profile_sha256 != plan.reset.profile_sha256
        || receipt.expected_initial_state_sha256 != plan.reset.expected_initial_state_sha256
        || receipt.producer_revision != plan.platform.simulator_revision
        || (bundle.reset.status == ResetStatus::Confirmed
            && receipt.observed_initial_state_sha256
                != plan.reset.expected_initial_state_sha256)
    {
        return Err(ExecutionEvidenceError::InvalidResetReceipt);
    }
    if bundle.reset.status == ResetStatus::Rejected && !bundle.events.is_empty() {
        return Err(ExecutionEvidenceError::ResetNotConfirmed);
    }
    Ok(())
}

impl ExecutionEvidenceBundle {
    /// Validate this bundle against the exact compiled plan.
    ///
    /// A Confirmed delivery requires target resolution, accepted dispatch,
    /// injector execution evidence and a distinct observer artifact/domain
    /// reporting the exact expected delivery signature. Declared domain
    /// separation is not a proof of universal physical independence.
    pub fn validate_against_plan(
        &self,
        plan: &CompiledCampaignPlan,
    ) -> Result<ExecutionSummary, ExecutionEvidenceError> {
        plan.verify_integrity()
            .map_err(|_| ExecutionEvidenceError::PlanBindingMismatch)?;

        if self.version != EXECUTION_EVIDENCE_VERSION
            || !nonempty(&self.campaign_id)
            || !nonempty(&self.run_id)
            || !nonempty(&self.run_epoch)
            || !nonempty(&self.world_epoch)
            || !valid_sha256(&self.plan_sha256)
            || !valid_sha256(&self.source_manifest_sha256)
        {
            return Err(ExecutionEvidenceError::InvalidHeader);
        }

        if self.campaign_id != plan.campaign_id
            || self.plan_sha256 != plan.plan_sha256
            || self.source_manifest_sha256 != plan.source_manifest_sha256
            || self.platform != plan.platform
        {
            return Err(ExecutionEvidenceError::PlanBindingMismatch);
        }

        let scenario = plan
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == self.scenario_id)
            .ok_or(ExecutionEvidenceError::ScenarioNotFound)?;
        if self.pair_id != scenario.pair_id
            || self.arm != scenario.arm
            || self.seed != scenario.seed
        {
            return Err(ExecutionEvidenceError::PlanBindingMismatch);
        }

        validate_reset(self, plan)?;
        if self.reset.status != ResetStatus::Confirmed {
            return Ok(ExecutionSummary {
                scenario_id: self.scenario_id.clone(),
                reset_status: self.reset.status,
                planned_fault_count: scenario.fault_events.len(),
                dispatch_accepted_count: 0,
                dispatch_rejected_count: 0,
                dispatch_unknown_count: 0,
                executed_count: 0,
                not_executed_count: 0,
                execution_unknown_count: 0,
                delivery_confirmed_count: 0,
                delivery_refuted_count: 0,
                delivery_unknown_count: 0,
                delivery_not_observed_count: 0,
                reconcile_required: self.reset.status == ResetStatus::Unknown,
                retry_permitted: false,
            });
        }

        if self.events.len() != scenario.fault_events.len() {
            return Err(ExecutionEvidenceError::EventCountMismatch);
        }

        let mut evidence_ids = BTreeSet::new();
        let mut artifact_ids = BTreeSet::new();
        let mut artifact_hashes = BTreeSet::new();
        let mut dispatch_ids = BTreeSet::new();
        let mut idempotency_keys = BTreeSet::new();
        let mut dispatch_accepted_count = 0usize;
        let mut dispatch_rejected_count = 0usize;
        let mut dispatch_unknown_count = 0usize;
        let mut executed_count = 0usize;
        let mut not_executed_count = 0usize;
        let mut execution_unknown_count = 0usize;
        let mut delivery_confirmed_count = 0usize;
        let mut delivery_refuted_count = 0usize;
        let mut delivery_unknown_count = 0usize;
        let mut delivery_not_observed_count = 0usize;
        let mut reconcile_required = false;

        let reset_receipt = self
            .reset
            .receipt
            .as_ref()
            .ok_or(ExecutionEvidenceError::InvalidResetReceipt)?;
        if !evidence_ids.insert(reset_receipt.evidence_id.clone())
            || !artifact_ids.insert(reset_receipt.raw_artifact_id.clone())
            || !artifact_hashes.insert(reset_receipt.raw_artifact_sha256.clone())
        {
            return Err(ExecutionEvidenceError::DuplicateEvidenceIdentity);
        }

        for (index, event) in self.events.iter().enumerate() {
            let expected = scenario
                .fault_events
                .get(index)
                .ok_or(ExecutionEvidenceError::EventCountMismatch)?;
            if event.planned_fault != *expected {
                return Err(ExecutionEvidenceError::PlannedFaultMismatch);
            }

            if !nonempty(&event.target_resolution.logical_component_id)
                || event.target_resolution.logical_component_id != expected.target_component_id
            {
                return Err(ExecutionEvidenceError::InvalidTargetResolution);
            }

            let resolved_target = match event.target_resolution.status {
                TargetResolutionStatus::Resolved => {
                    let target_id = event
                        .target_resolution
                        .resolved_target_id
                        .as_deref()
                        .filter(|value| nonempty(value))
                        .ok_or(ExecutionEvidenceError::InvalidTargetResolution)?;
                    let revision = event
                        .target_resolution
                        .resolved_target_revision
                        .as_deref()
                        .filter(|value| nonempty(value))
                        .ok_or(ExecutionEvidenceError::TargetRevisionMismatch)?;
                    let profile_sha = event
                        .target_resolution
                        .resolved_target_profile_sha256
                        .as_deref()
                        .ok_or(ExecutionEvidenceError::TargetRevisionMismatch)?;
                    let fault_domain = event
                        .target_resolution
                        .resolved_fault_domain_id
                        .as_deref()
                        .filter(|value| nonempty(value))
                        .ok_or(ExecutionEvidenceError::InvalidTargetResolution)?;
                    if revision != expected.target_revision
                        || profile_sha != expected.target_profile_sha256
                        || fault_domain != expected.fault_domain_id
                    {
                        return Err(ExecutionEvidenceError::TargetRevisionMismatch);
                    }
                    let receipt = event
                        .target_resolution
                        .receipt
                        .as_ref()
                        .ok_or(ExecutionEvidenceError::InvalidTargetResolution)?;
                    validate_receipt(
                        receipt,
                        EvidencePhase::TargetResolution,
                        self,
                        expected,
                        target_id,
                    )?;
                    if receipt.simulation_step > expected.scheduled_simulation_step {
                        return Err(ExecutionEvidenceError::ExecutionCoordinateMismatch);
                    }
                    if receipt.producer_id != self.platform.native_adapter_id
                        || receipt.producer_revision != self.platform.native_adapter_revision
                    {
                        return Err(ExecutionEvidenceError::InvalidReceipt);
                    }
                    Some(target_id)
                }
                TargetResolutionStatus::NotResolved => {
                    if event.target_resolution.resolved_target_id.is_some()
                        || event.target_resolution.resolved_target_revision.is_some()
                        || event.target_resolution.resolved_target_profile_sha256.is_some()
                        || event.target_resolution.resolved_fault_domain_id.is_some()
                        || event.target_resolution.receipt.is_some()
                    {
                        return Err(ExecutionEvidenceError::InvalidTargetResolution);
                    }
                    None
                }
                TargetResolutionStatus::Unknown => {
                    if event.target_resolution.resolved_target_id.is_some()
                        || event.target_resolution.resolved_target_revision.is_some()
                        || event.target_resolution.resolved_target_profile_sha256.is_some()
                        || event.target_resolution.resolved_fault_domain_id.is_some()
                    {
                        return Err(ExecutionEvidenceError::InvalidTargetResolution);
                    }
                    reconcile_required = true;
                    None
                }
            };

            if let Some(receipt) = event.target_resolution.receipt.as_ref() {
                validate_receipt(
                    receipt,
                    EvidencePhase::TargetResolution,
                    self,
                    expected,
                    resolved_target.unwrap_or(&expected.target_component_id),
                )?;
                if receipt.simulation_step > expected.scheduled_simulation_step {
                    return Err(ExecutionEvidenceError::ExecutionCoordinateMismatch);
                }
                if receipt.producer_id != self.platform.native_adapter_id
                    || receipt.producer_revision != self.platform.native_adapter_revision
                {
                    return Err(ExecutionEvidenceError::InvalidReceipt);
                }
                register_receipt(
                    receipt,
                    &mut evidence_ids,
                    &mut artifact_ids,
                    &mut artifact_hashes,
                )?;
            }

            if !nonempty(&event.dispatch.dispatch_id)
                || !nonempty(&event.dispatch.idempotency_key)
                || !dispatch_ids.insert(event.dispatch.dispatch_id.clone())
                || !idempotency_keys.insert(event.dispatch.idempotency_key.clone())
            {
                return Err(ExecutionEvidenceError::DuplicateDispatchIdentity);
            }

            match event.dispatch.status {
                DispatchStatus::Accepted => {
                    dispatch_accepted_count += 1;
                    if resolved_target.is_none() {
                        return Err(ExecutionEvidenceError::InvalidDispatch);
                    }
                    let receipt = event
                        .dispatch
                        .receipt
                        .as_ref()
                        .ok_or(ExecutionEvidenceError::InvalidDispatch)?;
                    validate_receipt(
                        receipt,
                        EvidencePhase::Dispatch,
                        self,
                        expected,
                        resolved_target.expect("resolved target checked"),
                    )?;
                    if receipt.producer_id != self.platform.native_adapter_id
                        || receipt.producer_revision != self.platform.native_adapter_revision
                    {
                        return Err(ExecutionEvidenceError::InvalidReceipt);
                    }
                    register_receipt(
                        receipt,
                        &mut evidence_ids,
                        &mut artifact_ids,
                        &mut artifact_hashes,
                    )?;
                    if receipt.simulation_step > expected.scheduled_simulation_step {
                        return Err(ExecutionEvidenceError::ExecutionCoordinateMismatch);
                    }
                }
                DispatchStatus::Rejected => {
                    dispatch_rejected_count += 1;
                    let receipt = event
                        .dispatch
                        .receipt
                        .as_ref()
                        .ok_or(ExecutionEvidenceError::InvalidDispatch)?;
                    let target_id = resolved_target.unwrap_or(&expected.target_component_id);
                    validate_receipt(
                        receipt,
                        EvidencePhase::Dispatch,
                        self,
                        expected,
                        target_id,
                    )?;
                    if receipt.producer_id != self.platform.native_adapter_id
                        || receipt.producer_revision != self.platform.native_adapter_revision
                    {
                        return Err(ExecutionEvidenceError::InvalidReceipt);
                    }
                    register_receipt(
                        receipt,
                        &mut evidence_ids,
                        &mut artifact_ids,
                        &mut artifact_hashes,
                    )?;
                }
                DispatchStatus::Unknown => {
                    dispatch_unknown_count += 1;
                    reconcile_required = true;
                    if let Some(receipt) = event.dispatch.receipt.as_ref() {
                        let target_id = resolved_target.unwrap_or(&expected.target_component_id);
                        validate_receipt(
                            receipt,
                            EvidencePhase::Dispatch,
                            self,
                            expected,
                            target_id,
                        )?;
                        if receipt.producer_id != self.platform.native_adapter_id
                            || receipt.producer_revision != self.platform.native_adapter_revision
                        {
                            return Err(ExecutionEvidenceError::InvalidReceipt);
                        }
                        register_receipt(
                            receipt,
                            &mut evidence_ids,
                            &mut artifact_ids,
                            &mut artifact_hashes,
                        )?;
                    }
                }
            }

            match event.execution.status {
                ExecutionStatus::Executed => {
                    executed_count += 1;
                    if event.dispatch.status != DispatchStatus::Accepted {
                        return Err(ExecutionEvidenceError::InvalidExecution);
                    }
                    let receipt = event
                        .execution
                        .receipt
                        .as_ref()
                        .ok_or(ExecutionEvidenceError::InvalidExecution)?;
                    let target_id = resolved_target.ok_or(ExecutionEvidenceError::InvalidExecution)?;
                    validate_receipt(
                        receipt,
                        EvidencePhase::Execution,
                        self,
                        expected,
                        target_id,
                    )?;
                    if receipt.simulation_step != expected.scheduled_simulation_step {
                        return Err(ExecutionEvidenceError::ExecutionCoordinateMismatch);
                    }
                    if receipt.producer_id != self.platform.fault_injector_id
                        || receipt.producer_revision != self.platform.fault_injector_revision
                    {
                        return Err(ExecutionEvidenceError::InvalidReceipt);
                    }
                    if receipt.simulation_step != expected.scheduled_simulation_step {
                        return Err(ExecutionEvidenceError::ExecutionCoordinateMismatch);
                    }
                    register_receipt(
                        receipt,
                        &mut evidence_ids,
                        &mut artifact_ids,
                        &mut artifact_hashes,
                    )?;
                }
                ExecutionStatus::NotExecuted => {
                    not_executed_count += 1;
                    let receipt = event
                        .execution
                        .receipt
                        .as_ref()
                        .ok_or(ExecutionEvidenceError::InvalidExecution)?;
                    let target_id = resolved_target.unwrap_or(&expected.target_component_id);
                    validate_receipt(
                        receipt,
                        EvidencePhase::Execution,
                        self,
                        expected,
                        target_id,
                    )?;
                    if receipt.producer_id != self.platform.fault_injector_id
                        || receipt.producer_revision != self.platform.fault_injector_revision
                    {
                        return Err(ExecutionEvidenceError::InvalidReceipt);
                    }
                    register_receipt(
                        receipt,
                        &mut evidence_ids,
                        &mut artifact_ids,
                        &mut artifact_hashes,
                    )?;
                }
                ExecutionStatus::Unknown => {
                    execution_unknown_count += 1;
                    reconcile_required = true;
                    if let Some(receipt) = event.execution.receipt.as_ref() {
                        let target_id = resolved_target.unwrap_or(&expected.target_component_id);
                        validate_receipt(
                            receipt,
                            EvidencePhase::Execution,
                            self,
                            expected,
                            target_id,
                        )?;
                        if receipt.producer_id != self.platform.fault_injector_id
                            || receipt.producer_revision != self.platform.fault_injector_revision
                        {
                            return Err(ExecutionEvidenceError::InvalidReceipt);
                        }
                        register_receipt(
                            receipt,
                            &mut evidence_ids,
                            &mut artifact_ids,
                            &mut artifact_hashes,
                        )?;
                    }
                }
            }

            let observation = &event.delivery_observation;
            match observation.verdict {
                DeliveryVerdict::Confirmed => {
                    delivery_confirmed_count += 1;
                    if event.execution.status != ExecutionStatus::Executed {
                        return Err(ExecutionEvidenceError::InvalidDeliveryObservation);
                    }
                    let target_id = resolved_target
                        .ok_or(ExecutionEvidenceError::InvalidDeliveryObservation)?;
                    let observer_id = observation
                        .observer_id
                        .as_deref()
                        .filter(|value| nonempty(value))
                        .ok_or(ExecutionEvidenceError::ObserverNotDistinct)?;
                    let observer_revision = observation
                        .observer_revision
                        .as_deref()
                        .filter(|value| nonempty(value))
                        .ok_or(ExecutionEvidenceError::ObserverNotDistinct)?;
                    let observer_fault_domain_id = observation
                        .observer_fault_domain_id
                        .as_deref()
                        .filter(|value| nonempty(value))
                        .ok_or(ExecutionEvidenceError::ObserverNotDistinct)?;
                    if observer_id == self.platform.fault_injector_id
                        || observer_fault_domain_id
                            == self.platform.fault_injector_fault_domain_id
                    {
                        return Err(ExecutionEvidenceError::ObserverNotDistinct);
                    }
                    let signature = observation
                        .observed_fault_signature_sha256
                        .as_deref()
                        .ok_or(ExecutionEvidenceError::DeliverySignatureMismatch)?;
                    if !valid_sha256(signature)
                        || signature != expected.expected_delivery_signature_sha256
                    {
                        return Err(ExecutionEvidenceError::DeliverySignatureMismatch);
                    }
                    let receipt = observation
                        .receipt
                        .as_ref()
                        .ok_or(ExecutionEvidenceError::InvalidDeliveryObservation)?;
                    validate_receipt(
                        receipt,
                        EvidencePhase::DeliveryObservation,
                        self,
                        expected,
                        target_id,
                    )?;
                    if receipt.producer_id != observer_id
                        || receipt.producer_revision != observer_revision
                    {
                        return Err(ExecutionEvidenceError::ObserverNotDistinct);
                    }
                    let execution_receipt = event
                        .execution
                        .receipt
                        .as_ref()
                        .ok_or(ExecutionEvidenceError::InvalidExecution)?;
                    if receipt.simulation_step < execution_receipt.simulation_step {
                        return Err(ExecutionEvidenceError::ExecutionCoordinateMismatch);
                    }
                    register_receipt(
                        receipt,
                        &mut evidence_ids,
                        &mut artifact_ids,
                        &mut artifact_hashes,
                    )?;
                }
                DeliveryVerdict::Refuted => {
                    delivery_refuted_count += 1;
                    let observer_id = observation
                        .observer_id
                        .as_deref()
                        .filter(|value| nonempty(value))
                        .ok_or(ExecutionEvidenceError::ObserverNotDistinct)?;
                    let observer_revision = observation
                        .observer_revision
                        .as_deref()
                        .filter(|value| nonempty(value))
                        .ok_or(ExecutionEvidenceError::ObserverNotDistinct)?;
                    let observer_fault_domain_id = observation
                        .observer_fault_domain_id
                        .as_deref()
                        .filter(|value| nonempty(value))
                        .ok_or(ExecutionEvidenceError::ObserverNotDistinct)?;
                    if observer_id == self.platform.fault_injector_revision
                        || observer_fault_domain_id
                            == self.platform.fault_injector_fault_domain_id
                    {
                        return Err(ExecutionEvidenceError::ObserverNotDistinct);
                    }
                    let signature = observation
                        .observed_fault_signature_sha256
                        .as_deref()
                        .ok_or(ExecutionEvidenceError::DeliverySignatureMismatch)?;
                    if !valid_sha256(signature)
                        || signature == expected.expected_delivery_signature_sha256
                    {
                        return Err(ExecutionEvidenceError::DeliverySignatureMismatch);
                    }
                    let receipt = observation
                        .receipt
                        .as_ref()
                        .ok_or(ExecutionEvidenceError::InvalidDeliveryObservation)?;
                    let target_id = resolved_target.unwrap_or(&expected.target_component_id);
                    validate_receipt(
                        receipt,
                        EvidencePhase::DeliveryObservation,
                        self,
                        expected,
                        target_id,
                    )?;
                    if receipt.producer_id != observer_id
                        || receipt.producer_revision != observer_revision
                    {
                        return Err(ExecutionEvidenceError::ObserverNotDistinct);
                    }
                    register_receipt(
                        receipt,
                        &mut evidence_ids,
                        &mut artifact_ids,
                        &mut artifact_hashes,
                    )?;
                }
                DeliveryVerdict::Unknown => {
                    delivery_unknown_count += 1;
                    if event.execution.status == ExecutionStatus::Executed
                        || event.dispatch.status == DispatchStatus::Unknown
                        || event.execution.status == ExecutionStatus::Unknown
                    {
                        reconcile_required = true;
                    }
                    if let Some(receipt) = observation.receipt.as_ref() {
                        let observer_id = observation
                            .observer_id
                            .as_deref()
                            .filter(|value| nonempty(value))
                            .ok_or(ExecutionEvidenceError::ObserverNotDistinct)?;
                        let observer_revision = observation
                            .observer_revision
                            .as_deref()
                            .filter(|value| nonempty(value))
                            .ok_or(ExecutionEvidenceError::ObserverNotDistinct)?;
                        let observer_fault_domain_id = observation
                            .observer_fault_domain_id
                            .as_deref()
                            .filter(|value| nonempty(value))
                            .ok_or(ExecutionEvidenceError::ObserverNotDistinct)?;
                        if observer_id == self.platform.fault_injector_id
                            || observer_fault_domain_id
                                == self.platform.fault_injector_fault_domain_id
                        {
                            return Err(ExecutionEvidenceError::ObserverNotDistinct);
                        }
                        if let Some(signature) = observation.observed_fault_signature_sha256.as_deref() {
                            if !valid_sha256(signature) {
                                return Err(ExecutionEvidenceError::DeliverySignatureMismatch);
                            }
                        }
                        validate_receipt(
                            receipt,
                            EvidencePhase::DeliveryObservation,
                            self,
                            expected,
                            resolved_target.unwrap_or(&expected.target_component_id),
                        )?;
                        if receipt.producer_id != observer_id
                            || receipt.producer_revision != observer_revision
                        {
                            return Err(ExecutionEvidenceError::ObserverNotDistinct);
                        }
                        register_receipt(
                            receipt,
                            &mut evidence_ids,
                            &mut artifact_ids,
                            &mut artifact_hashes,
                        )?;
                    } else if observation.observer_id.is_some()
                        || observation.observer_revision.is_some()
                        || observation.observer_fault_domain_id.is_some()
                        || observation.observed_fault_signature_sha256.is_some()
                    {
                        return Err(ExecutionEvidenceError::InvalidDeliveryObservation);
                    }
                }
                DeliveryVerdict::NotObserved => {
                    delivery_not_observed_count += 1;
                    if observation.receipt.is_some()
                        || observation.observer_id.is_some()
                        || observation.observer_revision.is_some()
                        || observation.observer_fault_domain_id.is_some()
                        || observation.observed_fault_signature_sha256.is_some()
                    {
                        return Err(ExecutionEvidenceError::InvalidDeliveryObservation);
                    }
                    if event.execution.status == ExecutionStatus::Executed
                        || event.dispatch.status == DispatchStatus::Unknown
                        || event.execution.status == ExecutionStatus::Unknown
                    {
                        reconcile_required = true;
                    }
                }
            }

            if event.target_resolution.status == TargetResolutionStatus::Unknown {
                reconcile_required = true;
            }
        }

        Ok(ExecutionSummary {
            scenario_id: self.scenario_id.clone(),
            reset_status: self.reset.status,
            planned_fault_count: scenario.fault_events.len(),
            dispatch_accepted_count,
            dispatch_rejected_count,
            dispatch_unknown_count,
            executed_count,
            not_executed_count,
            execution_unknown_count,
            delivery_confirmed_count,
            delivery_refuted_count,
            delivery_unknown_count,
            delivery_not_observed_count,
            reconcile_required,
            retry_permitted: false,
        })
    }
}

fn register_receipt(
    receipt: &FaultEvidenceReceipt,
    evidence_ids: &mut BTreeSet<String>,
    artifact_ids: &mut BTreeSet<String>,
    artifact_hashes: &mut BTreeSet<String>,
) -> Result<(), ExecutionEvidenceError> {
    if !evidence_ids.insert(receipt.evidence_id.clone())
        || !artifact_ids.insert(receipt.raw_artifact_id.clone())
        || !artifact_hashes.insert(receipt.raw_artifact_sha256.clone())
    {
        return Err(ExecutionEvidenceError::DuplicateEvidenceIdentity);
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
        let source_digest = {
            use sha2::{Digest, Sha256};
            let bytes = include_bytes!("../fixtures/fault_campaign_paired_v0_1.json");
            let digest = Sha256::digest(bytes);
            let mut value = String::new();
            for byte in digest {
                value.push_str(&format!("{byte:02x}"));
            }
            value
        };
        CompiledCampaignPlan::compile(&manifest, &source_digest).unwrap()
    }

    fn bundle() -> ExecutionEvidenceBundle {
        let mut bundle: ExecutionEvidenceBundle = serde_json::from_str(include_str!(
            "../fixtures/execution_evidence_positive_v0_1.json"
        ))
        .expect("execution evidence fixture deserializes");
        let plan = plan();
        bundle.plan_sha256 = plan.plan_sha256;
        bundle.source_manifest_sha256 = plan.source_manifest_sha256;
        bundle
    }

    #[test]
    fn confirms_delivery_only_with_execution_and_distinct_observation() {
        let plan = plan();
        let summary = bundle().validate_against_plan(&plan).unwrap();
        assert_eq!(summary.planned_fault_count, 4);
        assert_eq!(summary.dispatch_accepted_count, 4);
        assert_eq!(summary.executed_count, 4);
        assert_eq!(summary.delivery_confirmed_count, 4);
        assert!(!summary.reconcile_required);
        assert!(!summary.retry_permitted);
    }

    #[test]
    fn dispatch_acceptance_alone_cannot_confirm_delivery() {
        let plan = plan();
        let mut evidence = bundle();
        evidence.events[0].execution.status = ExecutionStatus::Unknown;
        evidence.events[0].execution.receipt = None;
        evidence.events[0].delivery_observation.verdict = DeliveryVerdict::Unknown;
        evidence.events[0].delivery_observation.observer_id = None;
        evidence.events[0].delivery_observation.observer_revision = None;
        evidence.events[0].delivery_observation.observer_fault_domain_id = None;
        evidence.events[0].delivery_observation.observed_fault_signature_sha256 = None;
        evidence.events[0].delivery_observation.receipt = None;
        let summary = evidence.validate_against_plan(&plan).unwrap();
        assert_eq!(summary.delivery_confirmed_count, 3);
        assert!(summary.reconcile_required);
    }

    #[test]
    fn rejects_wrong_target_revision_or_profile_digest() {
        let plan = plan();
        let mut evidence = bundle();
        evidence.events[0]
            .target_resolution
            .resolved_target_revision = Some("other-revision".into());
        assert_eq!(
            evidence.validate_against_plan(&plan),
            Err(ExecutionEvidenceError::TargetRevisionMismatch)
        );
    }

    #[test]
    fn rejects_execution_receipt_at_wrong_simulation_step() {
        let plan = plan();
        let mut evidence = bundle();
        evidence.events[0]
            .execution
            .receipt
            .as_mut()
            .unwrap()
            .simulation_step += 1;
        assert_eq!(
            evidence.validate_against_plan(&plan),
            Err(ExecutionEvidenceError::ExecutionCoordinateMismatch)
        );
    }

    #[test]
    fn rejects_observer_reuse_of_injector_fault_domain() {
        let plan = plan();
        let mut evidence = bundle();
        evidence.events[0]
            .delivery_observation
            .observer_fault_domain_id = Some(
            plan.platform.fault_injector_fault_domain_id.clone(),
        );
        assert_eq!(
            evidence.validate_against_plan(&plan),
            Err(ExecutionEvidenceError::ObserverNotDistinct)
        );
    }

    #[test]
    fn rejects_execution_receipt_from_a_non_pinned_injector() {
        let plan = plan();
        let mut evidence = bundle();
        evidence.events[0]
            .execution
            .receipt
            .as_mut()
            .unwrap()
            .producer_id = "unlisted-injector".into();
        assert_eq!(
            evidence.validate_against_plan(&plan),
            Err(ExecutionEvidenceError::InvalidReceipt)
        );
    }

    #[test]
    fn rejects_observed_signature_that_does_not_match_planned_delivery() {
        let plan = plan();
        let mut evidence = bundle();
        evidence.events[0]
            .delivery_observation
            .observed_fault_signature_sha256 = Some("f".repeat(64));
        assert_eq!(
            evidence.validate_against_plan(&plan),
            Err(ExecutionEvidenceError::DeliverySignatureMismatch)
        );
    }

    #[test]
    fn unknown_dispatch_requires_reconciliation_and_never_permits_retry() {
        let plan = plan();
        let mut evidence = bundle();
        evidence.events[0].dispatch.status = DispatchStatus::Unknown;
        evidence.events[0].dispatch.receipt = None;
        evidence.events[0].execution.status = ExecutionStatus::Unknown;
        evidence.events[0].execution.receipt = None;
        evidence.events[0].delivery_observation.verdict = DeliveryVerdict::Unknown;
        evidence.events[0].delivery_observation.observer_id = None;
        evidence.events[0].delivery_observation.observer_revision = None;
        evidence.events[0].delivery_observation.observer_fault_domain_id = None;
        evidence.events[0].delivery_observation.observed_fault_signature_sha256 = None;
        evidence.events[0].delivery_observation.receipt = None;
        let summary = evidence.validate_against_plan(&plan).unwrap();
        assert!(summary.reconcile_required);
        assert!(!summary.retry_permitted);
    }

    #[test]
    fn rejects_receipt_replay_across_events() {
        let plan = plan();
        let mut evidence = bundle();
        let mut replay = evidence.events[0].dispatch.receipt.clone().unwrap();
        let second_fault = &evidence.events[1].planned_fault;
        replay.fault_id = second_fault.fault_id.clone();
        replay.fault_sequence = second_fault.sequence;
        replay.cut_point = second_fault.cut_point;
        replay.target_id = evidence.events[1]
            .target_resolution
            .resolved_target_id
            .clone()
            .unwrap();
        evidence.events[1].dispatch.receipt = Some(replay);
        assert_eq!(
            evidence.validate_against_plan(&plan),
            Err(ExecutionEvidenceError::DuplicateEvidenceIdentity)
        );
    }

    #[test]
    fn unknown_reset_prevents_any_fault_event_and_requires_reconciliation() {
        let plan = plan();
        let mut evidence = bundle();
        evidence.reset.status = ResetStatus::Unknown;
        evidence.reset.receipt = None;
        evidence.events.clear();
        let summary = evidence.validate_against_plan(&plan).unwrap();
        assert!(summary.reconcile_required);
        assert_eq!(summary.executed_count, 0);
        assert!(!summary.retry_permitted);
    }

    #[test]
    fn rejects_wrong_plan_digest() {
        let plan = plan();
        let mut evidence = bundle();
        evidence.plan_sha256 = "0".repeat(64);
        assert_eq!(
            evidence.validate_against_plan(&plan),
            Err(ExecutionEvidenceError::PlanBindingMismatch)
        );
    }
}
