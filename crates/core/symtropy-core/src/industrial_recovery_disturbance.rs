// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Evidence-bound disturbance observation for qualified industrial recovery.
//!
//! Recovery qualification and recovery execution are separate claims. This module
//! adds an intermediate observation that binds the exact degraded state before a
//! qualified recovery contract may execute. It does not infer cause or physical
//! repair feasibility; it only prevents an unrelated or stale degradation from
//! being silently reused as the dynamic subject of a recovery receipt.

use crate::industrial_ecology::{IndustrialDependencyState, IndustrialFlowKind};
use crate::industrial_epoch::IndustrialEpochState;
use crate::industrial_recovery::{
    execute_industrial_recovery, IndustrialQualifiedRecoveryContract, IndustrialRecoveryError,
    IndustrialRecoveryReceipt, IndustrialRecoveryReserveState,
};

const MAX_ID_LEN: usize = 256;
const MAX_BINDING_LEN: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialRecoveryDisturbanceObservation {
    pub disturbance_id: String,
    pub evidence_binding: String,
    pub epoch_id: String,
    pub epoch_evidence_binding: String,
    pub target_dependency_id: String,
    pub flow_kind: IndustrialFlowKind,
    pub observed_units_per_tick: u64,
    pub qualified_ceiling_units_per_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialEvidenceBoundRecoveryReceipt {
    pub disturbance_id: String,
    pub disturbance_evidence_binding: String,
    pub observed_degraded_units_per_tick: u64,
    pub recovery: IndustrialRecoveryReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndustrialRecoveryDisturbanceError {
    InvalidIdentifier,
    InvalidEvidenceBinding,
    EpochBindingMismatch,
    UnknownTargetDependency { dependency_id: String },
    DisturbanceDoesNotDegradeQualifiedFlow {
        dependency_id: String,
        observed_units_per_tick: u64,
        qualified_units_per_tick: u64,
    },
    DisturbanceSubjectMismatch,
    ObservedStateDrift {
        dependency_id: String,
        observed_units_per_tick: u64,
        current_units_per_tick: u64,
    },
    Recovery(IndustrialRecoveryError),
}

/// Observe the exact degraded state that a qualified contract is intended to
/// recover. The observation is valid only when the current modeled flow is below
/// the contract's already-qualified healthy ceiling.
pub fn observe_industrial_recovery_disturbance(
    state: &IndustrialEpochState,
    contract: &IndustrialQualifiedRecoveryContract,
    disturbance_id: impl Into<String>,
    evidence_binding: impl Into<String>,
) -> Result<IndustrialRecoveryDisturbanceObservation, IndustrialRecoveryDisturbanceError> {
    let disturbance_id = disturbance_id.into();
    let evidence_binding = evidence_binding.into();
    validate_id(&disturbance_id)?;
    validate_binding(&evidence_binding)?;

    if state.epoch_id() != contract.epoch_id()
        || state.evidence_binding() != contract.epoch_evidence_binding()
    {
        return Err(IndustrialRecoveryDisturbanceError::EpochBindingMismatch);
    }
    let target = state
        .dependency(contract.target_dependency_id())
        .ok_or_else(|| IndustrialRecoveryDisturbanceError::UnknownTargetDependency {
            dependency_id: contract.target_dependency_id().to_string(),
        })?;
    let observed_units_per_tick = flow_units(target, contract.flow_kind());
    if observed_units_per_tick >= contract.qualified_units_per_tick() {
        return Err(
            IndustrialRecoveryDisturbanceError::DisturbanceDoesNotDegradeQualifiedFlow {
                dependency_id: contract.target_dependency_id().to_string(),
                observed_units_per_tick,
                qualified_units_per_tick: contract.qualified_units_per_tick(),
            },
        );
    }

    Ok(IndustrialRecoveryDisturbanceObservation {
        disturbance_id,
        evidence_binding,
        epoch_id: state.epoch_id().to_string(),
        epoch_evidence_binding: state.evidence_binding().to_string(),
        target_dependency_id: contract.target_dependency_id().to_string(),
        flow_kind: contract.flow_kind(),
        observed_units_per_tick,
        qualified_ceiling_units_per_tick: contract.qualified_units_per_tick(),
    })
}

/// Execute recovery only if the current degraded state still matches the exact
/// observation. This prevents stale disturbance evidence from authorizing a later,
/// different state transition.
pub fn execute_evidence_bound_industrial_recovery(
    state: &mut IndustrialEpochState,
    contract: &IndustrialQualifiedRecoveryContract,
    reserve: &mut IndustrialRecoveryReserveState,
    disturbance: &IndustrialRecoveryDisturbanceObservation,
) -> Result<IndustrialEvidenceBoundRecoveryReceipt, IndustrialRecoveryDisturbanceError> {
    if disturbance.epoch_id != contract.epoch_id()
        || disturbance.epoch_evidence_binding != contract.epoch_evidence_binding()
        || disturbance.target_dependency_id != contract.target_dependency_id()
        || disturbance.flow_kind != contract.flow_kind()
        || disturbance.qualified_ceiling_units_per_tick != contract.qualified_units_per_tick()
    {
        return Err(IndustrialRecoveryDisturbanceError::DisturbanceSubjectMismatch);
    }
    if state.epoch_id() != disturbance.epoch_id
        || state.evidence_binding() != disturbance.epoch_evidence_binding
    {
        return Err(IndustrialRecoveryDisturbanceError::EpochBindingMismatch);
    }
    let target = state
        .dependency(&disturbance.target_dependency_id)
        .ok_or_else(|| IndustrialRecoveryDisturbanceError::UnknownTargetDependency {
            dependency_id: disturbance.target_dependency_id.clone(),
        })?;
    let current_units_per_tick = flow_units(target, disturbance.flow_kind);
    if current_units_per_tick != disturbance.observed_units_per_tick {
        return Err(IndustrialRecoveryDisturbanceError::ObservedStateDrift {
            dependency_id: disturbance.target_dependency_id.clone(),
            observed_units_per_tick: disturbance.observed_units_per_tick,
            current_units_per_tick,
        });
    }

    let recovery = execute_industrial_recovery(state, contract, reserve)
        .map_err(IndustrialRecoveryDisturbanceError::Recovery)?;
    Ok(IndustrialEvidenceBoundRecoveryReceipt {
        disturbance_id: disturbance.disturbance_id.clone(),
        disturbance_evidence_binding: disturbance.evidence_binding.clone(),
        observed_degraded_units_per_tick: disturbance.observed_units_per_tick,
        recovery,
    })
}

fn flow_units(dependency: &IndustrialDependencyState, flow_kind: IndustrialFlowKind) -> u64 {
    match flow_kind {
        IndustrialFlowKind::Production => dependency.local_production_units_per_tick,
        IndustrialFlowKind::Recycling => dependency.recycling_units_per_tick,
    }
}

fn validate_id(value: &str) -> Result<(), IndustrialRecoveryDisturbanceError> {
    if value.is_empty()
        || value.len() > MAX_ID_LEN
        || value.trim() != value
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        Err(IndustrialRecoveryDisturbanceError::InvalidIdentifier)
    } else {
        Ok(())
    }
}

fn validate_binding(value: &str) -> Result<(), IndustrialRecoveryDisturbanceError> {
    if value.is_empty()
        || value.len() > MAX_BINDING_LEN
        || value.trim() != value
        || !value.contains(':')
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        Err(IndustrialRecoveryDisturbanceError::InvalidEvidenceBinding)
    } else {
        Ok(())
    }
}
