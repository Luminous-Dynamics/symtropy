// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Evidence-bound recovery of previously modeled industrial flow capacity.
//!
//! Recovery is modeled as an orthogonal resilience coordinate. The nominal epoch
//! specification remains unchanged. A separately evidence-bound reserve state can
//! be consumed only by a contract qualified against that exact nominal spec. The
//! contract may restore only a previously modeled healthy flow ceiling.

use crate::industrial_ecology::{IndustrialDependencyState, IndustrialFlowKind, IndustrialGovernance, IndustrialShock};
use crate::industrial_epoch::{IndustrialEpochSpec, IndustrialEpochState};

const MAX_ID_LEN: usize = 256;
const MAX_BINDING_LEN: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialRecoveryReserveState {
    reserve_id: String,
    evidence_binding: String,
    available_units: u64,
}

impl IndustrialRecoveryReserveState {
    pub fn new(
        reserve_id: impl Into<String>,
        evidence_binding: impl Into<String>,
        available_units: u64,
    ) -> Result<Self, IndustrialRecoveryError> {
        let reserve_id = reserve_id.into();
        let evidence_binding = evidence_binding.into();
        validate_id(&reserve_id)?;
        validate_binding(&evidence_binding)?;
        Ok(Self {
            reserve_id,
            evidence_binding,
            available_units,
        })
    }

    pub fn reserve_id(&self) -> &str {
        &self.reserve_id
    }

    pub fn evidence_binding(&self) -> &str {
        &self.evidence_binding
    }

    pub const fn available_units(&self) -> u64 {
        self.available_units
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialQualifiedRecoveryContract {
    recovery_id: String,
    evidence_binding: String,
    epoch_id: String,
    epoch_evidence_binding: String,
    target_dependency_id: String,
    flow_kind: IndustrialFlowKind,
    qualified_units_per_tick: u64,
    reserve_id: String,
    reserve_evidence_binding: String,
    reserve_units_per_recovery: u64,
}

impl IndustrialQualifiedRecoveryContract {
    pub fn recovery_id(&self) -> &str { &self.recovery_id }
    pub fn evidence_binding(&self) -> &str { &self.evidence_binding }
    pub fn epoch_id(&self) -> &str { &self.epoch_id }
    pub fn epoch_evidence_binding(&self) -> &str { &self.epoch_evidence_binding }
    pub fn target_dependency_id(&self) -> &str { &self.target_dependency_id }
    pub const fn flow_kind(&self) -> IndustrialFlowKind { self.flow_kind }
    pub const fn qualified_units_per_tick(&self) -> u64 { self.qualified_units_per_tick }
    pub fn reserve_id(&self) -> &str { &self.reserve_id }
    pub fn reserve_evidence_binding(&self) -> &str { &self.reserve_evidence_binding }
    pub const fn reserve_units_per_recovery(&self) -> u64 { self.reserve_units_per_recovery }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialRecoveryReceipt {
    pub recovery_id: String,
    pub evidence_binding: String,
    pub epoch_id: String,
    pub epoch_evidence_binding: String,
    pub target_dependency_id: String,
    pub flow_kind: IndustrialFlowKind,
    pub prior_units_per_tick: u64,
    pub restored_units_per_tick: u64,
    pub reserve_id: String,
    pub reserve_evidence_binding: String,
    pub reserve_units_consumed: u64,
    pub reserve_units_before: u64,
    pub reserve_units_after: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndustrialRecoveryError {
    InvalidIdentifier,
    InvalidEvidenceBinding,
    EpochSpecInvalid,
    ZeroReserveCost,
    UnknownTargetDependency { dependency_id: String },
    SafeguardedTargetClaimsLocalRecovery { dependency_id: String },
    TargetFlowNotPositiveAtQualification { dependency_id: String, flow_kind: IndustrialFlowKind },
    EpochBindingMismatch,
    RecoveryReserveBindingMismatch,
    TargetFlowExceedsQualifiedCeiling {
        dependency_id: String,
        current_units_per_tick: u64,
        qualified_units_per_tick: u64,
    },
    TargetFlowAlreadyQualified { dependency_id: String },
    InsufficientRecoveryReserve {
        reserve_id: String,
        required_units: u64,
        available_units: u64,
    },
    RestoreRejected,
}

/// Qualify a future recovery path from an immutable nominal epoch specification
/// and a separately evidence-bound recovery reserve.
pub fn qualify_industrial_recovery_contract(
    spec: &IndustrialEpochSpec,
    reserve: &IndustrialRecoveryReserveState,
    recovery_id: impl Into<String>,
    evidence_binding: impl Into<String>,
    target_dependency_id: impl Into<String>,
    flow_kind: IndustrialFlowKind,
    reserve_units_per_recovery: u64,
) -> Result<IndustrialQualifiedRecoveryContract, IndustrialRecoveryError> {
    IndustrialEpochState::from_spec(spec.clone())
        .map_err(|_| IndustrialRecoveryError::EpochSpecInvalid)?;

    let recovery_id = recovery_id.into();
    let evidence_binding = evidence_binding.into();
    let target_dependency_id = target_dependency_id.into();
    validate_id(&recovery_id)?;
    validate_binding(&evidence_binding)?;
    validate_id(&target_dependency_id)?;
    if reserve_units_per_recovery == 0 {
        return Err(IndustrialRecoveryError::ZeroReserveCost);
    }
    if reserve.available_units < reserve_units_per_recovery {
        return Err(IndustrialRecoveryError::InsufficientRecoveryReserve {
            reserve_id: reserve.reserve_id.clone(),
            required_units: reserve_units_per_recovery,
            available_units: reserve.available_units,
        });
    }

    let target = spec
        .dependencies
        .iter()
        .find(|dependency| dependency.dependency_id == target_dependency_id)
        .ok_or_else(|| IndustrialRecoveryError::UnknownTargetDependency {
            dependency_id: target_dependency_id.clone(),
        })?;
    if target.governance == IndustrialGovernance::SafeguardedExternal {
        return Err(IndustrialRecoveryError::SafeguardedTargetClaimsLocalRecovery {
            dependency_id: target_dependency_id,
        });
    }
    let qualified_units_per_tick = flow_units(target, flow_kind);
    if qualified_units_per_tick == 0 {
        return Err(IndustrialRecoveryError::TargetFlowNotPositiveAtQualification {
            dependency_id: target_dependency_id,
            flow_kind,
        });
    }

    Ok(IndustrialQualifiedRecoveryContract {
        recovery_id,
        evidence_binding,
        epoch_id: spec.epoch_id.clone(),
        epoch_evidence_binding: spec.evidence_binding.clone(),
        target_dependency_id,
        flow_kind,
        qualified_units_per_tick,
        reserve_id: reserve.reserve_id.clone(),
        reserve_evidence_binding: reserve.evidence_binding.clone(),
        reserve_units_per_recovery,
    })
}

pub fn execute_industrial_recovery(
    state: &mut IndustrialEpochState,
    contract: &IndustrialQualifiedRecoveryContract,
    reserve: &mut IndustrialRecoveryReserveState,
) -> Result<IndustrialRecoveryReceipt, IndustrialRecoveryError> {
    if state.epoch_id() != contract.epoch_id
        || state.evidence_binding() != contract.epoch_evidence_binding
    {
        return Err(IndustrialRecoveryError::EpochBindingMismatch);
    }
    if reserve.reserve_id != contract.reserve_id
        || reserve.evidence_binding != contract.reserve_evidence_binding
    {
        return Err(IndustrialRecoveryError::RecoveryReserveBindingMismatch);
    }

    let target = state
        .dependency(&contract.target_dependency_id)
        .ok_or_else(|| IndustrialRecoveryError::UnknownTargetDependency {
            dependency_id: contract.target_dependency_id.clone(),
        })?;
    let prior_units_per_tick = flow_units(target, contract.flow_kind);
    if prior_units_per_tick > contract.qualified_units_per_tick {
        return Err(IndustrialRecoveryError::TargetFlowExceedsQualifiedCeiling {
            dependency_id: contract.target_dependency_id.clone(),
            current_units_per_tick: prior_units_per_tick,
            qualified_units_per_tick: contract.qualified_units_per_tick,
        });
    }
    if prior_units_per_tick == contract.qualified_units_per_tick {
        return Err(IndustrialRecoveryError::TargetFlowAlreadyQualified {
            dependency_id: contract.target_dependency_id.clone(),
        });
    }
    if reserve.available_units < contract.reserve_units_per_recovery {
        return Err(IndustrialRecoveryError::InsufficientRecoveryReserve {
            reserve_id: reserve.reserve_id.clone(),
            required_units: contract.reserve_units_per_recovery,
            available_units: reserve.available_units,
        });
    }

    let restore = match contract.flow_kind {
        IndustrialFlowKind::Production => IndustrialShock::SetLocalProduction {
            dependency_id: contract.target_dependency_id.clone(),
            units_per_tick: contract.qualified_units_per_tick,
        },
        IndustrialFlowKind::Recycling => IndustrialShock::SetRecycling {
            dependency_id: contract.target_dependency_id.clone(),
            units_per_tick: contract.qualified_units_per_tick,
        },
    };
    state
        .apply_shock(restore)
        .map_err(|_| IndustrialRecoveryError::RestoreRejected)?;

    let reserve_units_before = reserve.available_units;
    reserve.available_units -= contract.reserve_units_per_recovery;
    let reserve_units_after = reserve.available_units;

    Ok(IndustrialRecoveryReceipt {
        recovery_id: contract.recovery_id.clone(),
        evidence_binding: contract.evidence_binding.clone(),
        epoch_id: contract.epoch_id.clone(),
        epoch_evidence_binding: contract.epoch_evidence_binding.clone(),
        target_dependency_id: contract.target_dependency_id.clone(),
        flow_kind: contract.flow_kind,
        prior_units_per_tick,
        restored_units_per_tick: contract.qualified_units_per_tick,
        reserve_id: contract.reserve_id.clone(),
        reserve_evidence_binding: contract.reserve_evidence_binding.clone(),
        reserve_units_consumed: contract.reserve_units_per_recovery,
        reserve_units_before,
        reserve_units_after,
    })
}

fn flow_units(dependency: &IndustrialDependencyState, flow_kind: IndustrialFlowKind) -> u64 {
    match flow_kind {
        IndustrialFlowKind::Production => dependency.local_production_units_per_tick,
        IndustrialFlowKind::Recycling => dependency.recycling_units_per_tick,
    }
}

fn validate_id(value: &str) -> Result<(), IndustrialRecoveryError> {
    if value.is_empty()
        || value.len() > MAX_ID_LEN
        || value.trim() != value
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        Err(IndustrialRecoveryError::InvalidIdentifier)
    } else {
        Ok(())
    }
}

fn validate_binding(value: &str) -> Result<(), IndustrialRecoveryError> {
    if value.is_empty()
        || value.len() > MAX_BINDING_LEN
        || value.trim() != value
        || !value.contains(':')
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        Err(IndustrialRecoveryError::InvalidEvidenceBinding)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::industrial_ecology::{IndustrialCapability, IndustrialFlowPrerequisite};
    use crate::industrial_epoch::IndustrialEpochFlowModel;
    use std::collections::BTreeSet;

    fn dependency(id: &str, demand: u64, production: u64) -> IndustrialDependencyState {
        IndustrialDependencyState {
            dependency_id: id.into(),
            governance: IndustrialGovernance::Ordinary,
            demand_units_per_tick: demand,
            local_production_units_per_tick: production,
            recycling_units_per_tick: 0,
            inventory_units: 0,
        }
    }

    fn spec() -> IndustrialEpochSpec {
        IndustrialEpochSpec {
            epoch_id: "recovery-epoch-v1".into(),
            evidence_binding: "epoch:recovery-v1".into(),
            dependencies: vec![dependency("metrology", 1, 1), dependency("power", 1, 1)],
            capabilities: vec![IndustrialCapability {
                capability_id: "qualified-output".into(),
                essential: true,
                dependency_ids: BTreeSet::from(["metrology".into(), "power".into()]),
            }],
            flow_model: IndustrialEpochFlowModel::PrerequisiteGated(vec![
                IndustrialFlowPrerequisite {
                    dependency_id: "metrology".into(),
                    flow_kind: IndustrialFlowKind::Production,
                    prerequisite_dependency_ids: BTreeSet::from(["power".into()]),
                },
                IndustrialFlowPrerequisite {
                    dependency_id: "power".into(),
                    flow_kind: IndustrialFlowKind::Production,
                    prerequisite_dependency_ids: BTreeSet::from(["metrology".into()]),
                },
            ]),
        }
    }

    #[test]
    fn qualified_recovery_restores_only_specified_ceiling_and_spends_external_reserve() {
        let spec = spec();
        let mut reserve = IndustrialRecoveryReserveState::new(
            "repair-reserve-v1",
            "recovery-reserve:repair-v1",
            2,
        )
        .unwrap();
        let contract = qualify_industrial_recovery_contract(
            &spec,
            &reserve,
            "recover-metrology",
            "recovery:metrology:v1",
            "metrology",
            IndustrialFlowKind::Production,
            1,
        )
        .unwrap();
        let mut state = IndustrialEpochState::from_spec(spec).unwrap();
        state
            .apply_shock(IndustrialShock::SetLocalProduction {
                dependency_id: "metrology".into(),
                units_per_tick: 0,
            })
            .unwrap();
        let receipt = execute_industrial_recovery(&mut state, &contract, &mut reserve).unwrap();
        assert_eq!(receipt.restored_units_per_tick, 1);
        assert_eq!(receipt.reserve_units_before, 2);
        assert_eq!(receipt.reserve_units_after, 1);
        assert_eq!(reserve.available_units(), 1);
    }

    #[test]
    fn zero_flow_cannot_be_qualified_as_future_recovery_authority() {
        let mut spec = spec();
        spec.dependencies
            .iter_mut()
            .find(|dependency| dependency.dependency_id == "metrology")
            .unwrap()
            .local_production_units_per_tick = 0;
        let reserve = IndustrialRecoveryReserveState::new(
            "repair-reserve-v1",
            "recovery-reserve:repair-v1",
            1,
        )
        .unwrap();
        let error = qualify_industrial_recovery_contract(
            &spec,
            &reserve,
            "recover-metrology",
            "recovery:metrology:v1",
            "metrology",
            IndustrialFlowKind::Production,
            1,
        )
        .unwrap_err();
        assert!(matches!(error, IndustrialRecoveryError::EpochSpecInvalid));
    }

    #[test]
    fn depleted_external_reserve_cannot_be_reused() {
        let spec = spec();
        let mut reserve = IndustrialRecoveryReserveState::new(
            "repair-reserve-v1",
            "recovery-reserve:repair-v1",
            1,
        )
        .unwrap();
        let contract = qualify_industrial_recovery_contract(
            &spec,
            &reserve,
            "recover-metrology",
            "recovery:metrology:v1",
            "metrology",
            IndustrialFlowKind::Production,
            1,
        )
        .unwrap();
        let mut state = IndustrialEpochState::from_spec(spec).unwrap();
        state
            .apply_shock(IndustrialShock::SetLocalProduction {
                dependency_id: "metrology".into(),
                units_per_tick: 0,
            })
            .unwrap();
        execute_industrial_recovery(&mut state, &contract, &mut reserve).unwrap();
        state
            .apply_shock(IndustrialShock::SetLocalProduction {
                dependency_id: "metrology".into(),
                units_per_tick: 0,
            })
            .unwrap();
        assert!(matches!(
            execute_industrial_recovery(&mut state, &contract, &mut reserve),
            Err(IndustrialRecoveryError::InsufficientRecoveryReserve { .. })
        ));
    }
}
