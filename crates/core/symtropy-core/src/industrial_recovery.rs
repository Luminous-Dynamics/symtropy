// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Evidence-bound recovery of previously modeled industrial flow capacity.
//!
//! A recovery contract is qualified while the target flow is still present and
//! captures that exact positive flow as its immutable recovery ceiling. Later
//! execution may restore a degraded flow only up to that captured ceiling and
//! consumes an explicit reserve dependency. It cannot create a flow that was not
//! already modeled by the bound epoch. This is deterministic simulation/evidence
//! plumbing only; it grants no repair, manufacturing, procurement or physical
//! control authority.

use crate::industrial_ecology::{IndustrialFlowKind, IndustrialGovernance, IndustrialShock};
use crate::industrial_epoch::IndustrialEpochState;

const MAX_ID_LEN: usize = 256;
const MAX_BINDING_LEN: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialQualifiedRecoveryContract {
    recovery_id: String,
    evidence_binding: String,
    epoch_id: String,
    epoch_evidence_binding: String,
    target_dependency_id: String,
    flow_kind: IndustrialFlowKind,
    qualified_units_per_tick: u64,
    reserve_dependency_id: String,
    reserve_units_per_recovery: u64,
}

impl IndustrialQualifiedRecoveryContract {
    pub fn recovery_id(&self) -> &str {
        &self.recovery_id
    }

    pub fn evidence_binding(&self) -> &str {
        &self.evidence_binding
    }

    pub fn epoch_id(&self) -> &str {
        &self.epoch_id
    }

    pub fn epoch_evidence_binding(&self) -> &str {
        &self.epoch_evidence_binding
    }

    pub fn target_dependency_id(&self) -> &str {
        &self.target_dependency_id
    }

    pub const fn flow_kind(&self) -> IndustrialFlowKind {
        self.flow_kind
    }

    pub const fn qualified_units_per_tick(&self) -> u64 {
        self.qualified_units_per_tick
    }

    pub fn reserve_dependency_id(&self) -> &str {
        &self.reserve_dependency_id
    }

    pub const fn reserve_units_per_recovery(&self) -> u64 {
        self.reserve_units_per_recovery
    }
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
    pub reserve_dependency_id: String,
    pub reserve_units_consumed: u64,
    pub reserve_units_before: u64,
    pub reserve_units_after: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndustrialRecoveryError {
    InvalidIdentifier,
    InvalidEvidenceBinding,
    ZeroReserveCost,
    UnknownTargetDependency { dependency_id: String },
    UnknownReserveDependency { dependency_id: String },
    TargetAndReserveNotDistinct,
    SafeguardedTargetClaimsLocalRecovery { dependency_id: String },
    SafeguardedReserveCannotAuthorizeLocalRecovery { dependency_id: String },
    TargetFlowNotPositiveAtQualification { dependency_id: String, flow_kind: IndustrialFlowKind },
    EpochBindingMismatch,
    TargetFlowExceedsQualifiedCeiling {
        dependency_id: String,
        current_units_per_tick: u64,
        qualified_units_per_tick: u64,
    },
    TargetFlowAlreadyQualified { dependency_id: String },
    InsufficientRecoveryReserve {
        dependency_id: String,
        required_units: u64,
        available_units: u64,
    },
    RestoreRejected,
    ReserveConsumptionInvariantViolation,
}

/// Qualify a future recovery path against an exact epoch while the modeled flow
/// is still present. The observed positive flow becomes the immutable ceiling.
pub fn qualify_industrial_recovery_contract(
    state: &IndustrialEpochState,
    recovery_id: impl Into<String>,
    evidence_binding: impl Into<String>,
    target_dependency_id: impl Into<String>,
    flow_kind: IndustrialFlowKind,
    reserve_dependency_id: impl Into<String>,
    reserve_units_per_recovery: u64,
) -> Result<IndustrialQualifiedRecoveryContract, IndustrialRecoveryError> {
    let recovery_id = recovery_id.into();
    let evidence_binding = evidence_binding.into();
    let target_dependency_id = target_dependency_id.into();
    let reserve_dependency_id = reserve_dependency_id.into();
    validate_id(&recovery_id)?;
    validate_binding(&evidence_binding)?;
    validate_id(&target_dependency_id)?;
    validate_id(&reserve_dependency_id)?;
    if reserve_units_per_recovery == 0 {
        return Err(IndustrialRecoveryError::ZeroReserveCost);
    }
    if target_dependency_id == reserve_dependency_id {
        return Err(IndustrialRecoveryError::TargetAndReserveNotDistinct);
    }

    let target = state
        .dependency(&target_dependency_id)
        .ok_or_else(|| IndustrialRecoveryError::UnknownTargetDependency {
            dependency_id: target_dependency_id.clone(),
        })?;
    let reserve = state
        .dependency(&reserve_dependency_id)
        .ok_or_else(|| IndustrialRecoveryError::UnknownReserveDependency {
            dependency_id: reserve_dependency_id.clone(),
        })?;
    if target.governance == IndustrialGovernance::SafeguardedExternal {
        return Err(IndustrialRecoveryError::SafeguardedTargetClaimsLocalRecovery {
            dependency_id: target_dependency_id,
        });
    }
    if reserve.governance == IndustrialGovernance::SafeguardedExternal {
        return Err(
            IndustrialRecoveryError::SafeguardedReserveCannotAuthorizeLocalRecovery {
                dependency_id: reserve_dependency_id,
            },
        );
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
        epoch_id: state.epoch_id().to_string(),
        epoch_evidence_binding: state.evidence_binding().to_string(),
        target_dependency_id,
        flow_kind,
        qualified_units_per_tick,
        reserve_dependency_id,
        reserve_units_per_recovery,
    })
}

/// Execute one qualified recovery atomically with respect to reserve spending.
///
/// The target restore is attempted first. Only after that succeeds is reserve
/// inventory consumed. Given successful pre-validation, `LoseInventory` on the
/// known reserve cannot invalidate flow-model coverage; if that invariant is ever
/// violated, the target flow is rolled back before returning an error.
pub fn execute_industrial_recovery(
    state: &mut IndustrialEpochState,
    contract: &IndustrialQualifiedRecoveryContract,
) -> Result<IndustrialRecoveryReceipt, IndustrialRecoveryError> {
    if state.epoch_id() != contract.epoch_id
        || state.evidence_binding() != contract.epoch_evidence_binding
    {
        return Err(IndustrialRecoveryError::EpochBindingMismatch);
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
    let reserve_units_before = state
        .dependency(&contract.reserve_dependency_id)
        .ok_or_else(|| IndustrialRecoveryError::UnknownReserveDependency {
            dependency_id: contract.reserve_dependency_id.clone(),
        })?
        .inventory_units;
    if reserve_units_before < contract.reserve_units_per_recovery {
        return Err(IndustrialRecoveryError::InsufficientRecoveryReserve {
            dependency_id: contract.reserve_dependency_id.clone(),
            required_units: contract.reserve_units_per_recovery,
            available_units: reserve_units_before,
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

    if state
        .apply_shock(IndustrialShock::LoseInventory {
            dependency_id: contract.reserve_dependency_id.clone(),
            units: contract.reserve_units_per_recovery,
        })
        .is_err()
    {
        let rollback = match contract.flow_kind {
            IndustrialFlowKind::Production => IndustrialShock::SetLocalProduction {
                dependency_id: contract.target_dependency_id.clone(),
                units_per_tick: prior_units_per_tick,
            },
            IndustrialFlowKind::Recycling => IndustrialShock::SetRecycling {
                dependency_id: contract.target_dependency_id.clone(),
                units_per_tick: prior_units_per_tick,
            },
        };
        let _ = state.apply_shock(rollback);
        return Err(IndustrialRecoveryError::ReserveConsumptionInvariantViolation);
    }

    let reserve_units_after = state
        .dependency(&contract.reserve_dependency_id)
        .ok_or_else(|| IndustrialRecoveryError::UnknownReserveDependency {
            dependency_id: contract.reserve_dependency_id.clone(),
        })?
        .inventory_units;
    Ok(IndustrialRecoveryReceipt {
        recovery_id: contract.recovery_id.clone(),
        evidence_binding: contract.evidence_binding.clone(),
        epoch_id: contract.epoch_id.clone(),
        epoch_evidence_binding: contract.epoch_evidence_binding.clone(),
        target_dependency_id: contract.target_dependency_id.clone(),
        flow_kind: contract.flow_kind,
        prior_units_per_tick,
        restored_units_per_tick: contract.qualified_units_per_tick,
        reserve_dependency_id: contract.reserve_dependency_id.clone(),
        reserve_units_consumed: contract.reserve_units_per_recovery,
        reserve_units_before,
        reserve_units_after,
    })
}

fn flow_units(
    dependency: &crate::industrial_ecology::IndustrialDependencyState,
    flow_kind: IndustrialFlowKind,
) -> u64 {
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
    use crate::industrial_ecology::{
        IndustrialCapability, IndustrialDependencyState, IndustrialFlowPrerequisite,
    };
    use crate::industrial_epoch::{IndustrialEpochFlowModel, IndustrialEpochSpec};
    use std::collections::BTreeSet;

    fn dependency(
        id: &str,
        demand: u64,
        production: u64,
        inventory: u64,
    ) -> IndustrialDependencyState {
        IndustrialDependencyState {
            dependency_id: id.into(),
            governance: IndustrialGovernance::Ordinary,
            demand_units_per_tick: demand,
            local_production_units_per_tick: production,
            recycling_units_per_tick: 0,
            inventory_units: inventory,
        }
    }

    fn state() -> IndustrialEpochState {
        IndustrialEpochState::from_spec(IndustrialEpochSpec {
            epoch_id: "recovery-epoch-v1".into(),
            evidence_binding: "epoch:recovery-v1".into(),
            dependencies: vec![
                dependency("metrology", 1, 1, 0),
                dependency("power", 1, 1, 0),
                dependency("repair-reserve", 1, 0, 3),
            ],
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
        })
        .unwrap()
    }

    #[test]
    fn qualified_recovery_restores_only_previously_modeled_ceiling_and_spends_reserve() {
        let mut state = state();
        let contract = qualify_industrial_recovery_contract(
            &state,
            "recover-metrology",
            "recovery:metrology:v1",
            "metrology",
            IndustrialFlowKind::Production,
            "repair-reserve",
            2,
        )
        .unwrap();
        assert_eq!(contract.qualified_units_per_tick(), 1);
        state
            .apply_shock(IndustrialShock::SetLocalProduction {
                dependency_id: "metrology".into(),
                units_per_tick: 0,
            })
            .unwrap();
        let receipt = execute_industrial_recovery(&mut state, &contract).unwrap();
        assert_eq!(receipt.prior_units_per_tick, 0);
        assert_eq!(receipt.restored_units_per_tick, 1);
        assert_eq!(receipt.reserve_units_before, 3);
        assert_eq!(receipt.reserve_units_after, 1);
        assert_eq!(state.dependency("metrology").unwrap().local_production_units_per_tick, 1);
    }

    #[test]
    fn zero_flow_cannot_be_qualified_as_future_recovery_authority() {
        let state = state();
        let error = qualify_industrial_recovery_contract(
            &state,
            "recover-reserve",
            "recovery:reserve:v1",
            "repair-reserve",
            IndustrialFlowKind::Production,
            "metrology",
            1,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            IndustrialRecoveryError::TargetFlowNotPositiveAtQualification { .. }
        ));
    }

    #[test]
    fn insufficient_reserve_does_not_restore_target() {
        let mut state = state();
        let contract = qualify_industrial_recovery_contract(
            &state,
            "recover-metrology",
            "recovery:metrology:v1",
            "metrology",
            IndustrialFlowKind::Production,
            "repair-reserve",
            4,
        )
        .unwrap();
        state
            .apply_shock(IndustrialShock::SetLocalProduction {
                dependency_id: "metrology".into(),
                units_per_tick: 0,
            })
            .unwrap();
        assert!(matches!(
            execute_industrial_recovery(&mut state, &contract),
            Err(IndustrialRecoveryError::InsufficientRecoveryReserve { .. })
        ));
        assert_eq!(state.dependency("metrology").unwrap().local_production_units_per_tick, 0);
        assert_eq!(state.dependency("repair-reserve").unwrap().inventory_units, 3);
    }
}
