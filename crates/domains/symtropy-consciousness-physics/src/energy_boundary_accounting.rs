// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Boundary-aware energy accounting as an additive companion to the legacy
//! thermodynamics module.
//!
//! The legacy ThermodynamicLedger computes a flow-only residual from its action-cost
//! and dissipation counters. This module provides a separate, explicit boundary
//! contract without changing that legacy API or its output semantics.
//!
//! For a declared boundary, one common energy unit, and a complete inventory of
//! boundary-crossing flows, the reference identity is:
//!
//!     R = E_initial + boundary_inflows - boundary_outflows - E_final
//!
//! Internal transfers between tracked stores wholly inside the boundary are
//! validated but do not enter the boundary-flow sums. A zero residual establishes
//! only arithmetic consistency for the declared inputs. It does not prove that the
//! boundary is complete, the event inventory is truthful, the units are physically
//! compatible, or the model obeys a physical law.

use std::collections::BTreeSet;

/// Direction of a transfer that crosses the declared system boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryDirection {
    Inflow,
    Outflow,
}

/// One non-negative energy magnitude that crosses the declared boundary exactly once.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundaryFlow {
    pub event_id: String,
    pub direction: BoundaryDirection,
    pub amount: f64,
}

/// An energy transfer between two tracked stores entirely inside the boundary.
///
/// The transfer is validated and reported as evidence of internal movement, but is
/// not added to either boundary inflows or outflows.
#[derive(Debug, Clone, PartialEq)]
pub struct InternalEnergyTransfer {
    pub event_id: String,
    pub from_store: String,
    pub to_store: String,
    pub amount: f64,
}

/// A complete declared input snapshot for one energy boundary and one energy unit.
///
/// Relative tolerance must be finite and in (0, 1]. The arithmetic tolerance is
/// relative_tolerance * max(1, E_initial + inflows + outflows + E_final).
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyBoundarySnapshot {
    pub boundary_id: String,
    pub unit: String,
    pub tracked_store_ids: Vec<String>,
    pub initial_stored_energy: f64,
    pub final_stored_energy: f64,
    pub boundary_flows: Vec<BoundaryFlow>,
    pub internal_transfers: Vec<InternalEnergyTransfer>,
    pub relative_tolerance: f64,
}

/// Result of applying the declared-boundary arithmetic identity.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyBoundaryAssessment {
    pub boundary_id: String,
    pub unit: String,
    pub initial_stored_energy: f64,
    pub boundary_inflows: f64,
    pub boundary_outflows: f64,
    pub final_stored_energy: f64,
    pub internal_transfer_count: usize,
    /// Signed residual: initial + inflows - outflows - final.
    pub residual: f64,
    pub tolerance: f64,
    pub balanced: bool,
}

/// Fail-closed validation or arithmetic errors for the declared-boundary contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyBoundaryError {
    EmptyBoundaryId,
    EmptyEnergyUnit,
    InvalidRelativeTolerance,
    InvalidStoredEnergy,
    EmptyTrackedStoreId,
    DuplicateTrackedStoreId,
    EmptyEventId,
    DuplicateEventId,
    InvalidFlowAmount,
    EmptyTransferStoreId,
    UnknownTransferStore,
    SameStoreTransfer,
    InvalidTransferAmount,
    NonFiniteArithmetic,
}

fn checked_sum(total: f64, amount: f64) -> Result<f64, EnergyBoundaryError> {
    let next = total + amount;
    if next.is_finite() {
        Ok(next)
    } else {
        Err(EnergyBoundaryError::NonFiniteArithmetic)
    }
}

/// Validate and assess one declared energy boundary without mutating production state.
///
/// Every magnitude must be finite and non-negative. Event IDs are unique across
/// both boundary flows and internal transfers. Internal-transfer endpoints must be
/// distinct tracked stores. The function does not assert that the declared boundary
/// or event inventory is physically complete.
pub fn assess_energy_boundary(
    snapshot: &EnergyBoundarySnapshot,
) -> Result<EnergyBoundaryAssessment, EnergyBoundaryError> {
    if snapshot.boundary_id.trim().is_empty() {
        return Err(EnergyBoundaryError::EmptyBoundaryId);
    }
    if snapshot.unit.trim().is_empty() {
        return Err(EnergyBoundaryError::EmptyEnergyUnit);
    }
    if !snapshot.relative_tolerance.is_finite()
        || snapshot.relative_tolerance <= 0.0
        || snapshot.relative_tolerance > 1.0
    {
        return Err(EnergyBoundaryError::InvalidRelativeTolerance);
    }
    if !snapshot.initial_stored_energy.is_finite()
        || snapshot.initial_stored_energy < 0.0
        || !snapshot.final_stored_energy.is_finite()
        || snapshot.final_stored_energy < 0.0
    {
        return Err(EnergyBoundaryError::InvalidStoredEnergy);
    }

    let mut stores = BTreeSet::new();
    for store_id in &snapshot.tracked_store_ids {
        if store_id.trim().is_empty() {
            return Err(EnergyBoundaryError::EmptyTrackedStoreId);
        }
        if !stores.insert(store_id.as_str()) {
            return Err(EnergyBoundaryError::DuplicateTrackedStoreId);
        }
    }

    let mut event_ids = BTreeSet::new();
    let mut inflows = 0.0;
    let mut outflows = 0.0;
    for flow in &snapshot.boundary_flows {
        if flow.event_id.trim().is_empty() {
            return Err(EnergyBoundaryError::EmptyEventId);
        }
        if !event_ids.insert(flow.event_id.as_str()) {
            return Err(EnergyBoundaryError::DuplicateEventId);
        }
        if !flow.amount.is_finite() || flow.amount < 0.0 {
            return Err(EnergyBoundaryError::InvalidFlowAmount);
        }
        match flow.direction {
            BoundaryDirection::Inflow => inflows = checked_sum(inflows, flow.amount)?,
            BoundaryDirection::Outflow => outflows = checked_sum(outflows, flow.amount)?,
        }
    }

    for transfer in &snapshot.internal_transfers {
        if transfer.event_id.trim().is_empty() {
            return Err(EnergyBoundaryError::EmptyEventId);
        }
        if !event_ids.insert(transfer.event_id.as_str()) {
            return Err(EnergyBoundaryError::DuplicateEventId);
        }
        if transfer.from_store.trim().is_empty() || transfer.to_store.trim().is_empty() {
            return Err(EnergyBoundaryError::EmptyTransferStoreId);
        }
        if !stores.contains(transfer.from_store.as_str())
            || !stores.contains(transfer.to_store.as_str())
        {
            return Err(EnergyBoundaryError::UnknownTransferStore);
        }
        if transfer.from_store == transfer.to_store {
            return Err(EnergyBoundaryError::SameStoreTransfer);
        }
        if !transfer.amount.is_finite() || transfer.amount < 0.0 {
            return Err(EnergyBoundaryError::InvalidTransferAmount);
        }
    }

    let scale = checked_sum(
        checked_sum(
            checked_sum(snapshot.initial_stored_energy, inflows)?,
            outflows,
        )?,
        snapshot.final_stored_energy,
    )?;
    let residual = checked_sum(
        checked_sum(
            checked_sum(snapshot.initial_stored_energy, inflows)?,
            -outflows,
        )?,
        -snapshot.final_stored_energy,
    )?;
    let tolerance = snapshot.relative_tolerance * scale.max(1.0);
    if !tolerance.is_finite() {
        return Err(EnergyBoundaryError::NonFiniteArithmetic);
    }

    Ok(EnergyBoundaryAssessment {
        boundary_id: snapshot.boundary_id.clone(),
        unit: snapshot.unit.clone(),
        initial_stored_energy: snapshot.initial_stored_energy,
        boundary_inflows: inflows,
        boundary_outflows: outflows,
        final_stored_energy: snapshot.final_stored_energy,
        internal_transfer_count: snapshot.internal_transfers.len(),
        residual,
        tolerance,
        balanced: residual.abs() <= tolerance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flow(id: &str, direction: BoundaryDirection, amount: f64) -> BoundaryFlow {
        BoundaryFlow {
            event_id: id.to_owned(),
            direction,
            amount,
        }
    }

    fn transfer(id: &str, from: &str, to: &str, amount: f64) -> InternalEnergyTransfer {
        InternalEnergyTransfer {
            event_id: id.to_owned(),
            from_store: from.to_owned(),
            to_store: to.to_owned(),
            amount,
        }
    }

    fn balanced_snapshot() -> EnergyBoundarySnapshot {
        EnergyBoundarySnapshot {
            boundary_id: "cell-A".to_owned(),
            unit: "J".to_owned(),
            tracked_store_ids: vec!["battery".to_owned(), "thermal-store".to_owned()],
            initial_stored_energy: 10.0,
            final_stored_energy: 12.0,
            boundary_flows: vec![
                flow("in-1", BoundaryDirection::Inflow, 5.0),
                flow("out-1", BoundaryDirection::Outflow, 3.0),
            ],
            internal_transfers: vec![transfer("internal-1", "battery", "thermal-store", 2.0)],
            relative_tolerance: 1.0e-12,
        }
    }

    #[test]
    fn balanced_boundary_matches_declared_energy_identity() {
        let result = assess_energy_boundary(&balanced_snapshot()).unwrap();
        assert_eq!(result.boundary_inflows, 5.0);
        assert_eq!(result.boundary_outflows, 3.0);
        assert_eq!(result.internal_transfer_count, 1);
        assert_eq!(result.residual, 0.0);
        assert!(result.balanced);
    }

    #[test]
    fn stored_energy_drift_is_not_hidden_by_equal_inflow_and_outflow() {
        let mut input = balanced_snapshot();
        input.boundary_flows = vec![
            flow("in", BoundaryDirection::Inflow, 5.0),
            flow("out", BoundaryDirection::Outflow, 5.0),
        ];
        input.initial_stored_energy = 10.0;
        input.final_stored_energy = 9.0;

        let result = assess_energy_boundary(&input).unwrap();
        assert_eq!(result.boundary_inflows, result.boundary_outflows);
        assert_eq!(result.residual, 1.0);
        assert!(!result.balanced);
    }

    #[test]
    fn internal_transfers_do_not_change_boundary_residual() {
        let mut with_transfer = balanced_snapshot();
        let without_transfer = EnergyBoundarySnapshot {
            internal_transfers: Vec::new(),
            ..with_transfer.clone()
        };
        with_transfer.internal_transfers.push(transfer(
            "internal-2",
            "thermal-store",
            "battery",
            1.5,
        ));

        let before = assess_energy_boundary(&without_transfer).unwrap();
        let after = assess_energy_boundary(&with_transfer).unwrap();
        assert_eq!(before.residual, after.residual);
        assert_eq!(after.internal_transfer_count, 2);
    }

    #[test]
    fn boundary_flow_order_does_not_change_result() {
        let mut first = balanced_snapshot();
        let mut second = first.clone();
        second.boundary_flows.reverse();
        second.internal_transfers.reverse();

        assert_eq!(
            assess_energy_boundary(&first).unwrap(),
            assess_energy_boundary(&second).unwrap()
        );
        first.boundary_flows.reverse();
        assert_eq!(
            assess_energy_boundary(&first).unwrap(),
            assess_energy_boundary(&second).unwrap()
        );
    }

    #[test]
    fn duplicate_event_ids_are_rejected_across_event_classes() {
        let mut input = balanced_snapshot();
        input.internal_transfers[0].event_id = "in-1".to_owned();
        assert_eq!(
            assess_energy_boundary(&input),
            Err(EnergyBoundaryError::DuplicateEventId)
        );
    }

    #[test]
    fn invalid_internal_transfer_endpoints_reject() {
        let mut unknown_store = balanced_snapshot();
        unknown_store.internal_transfers[0].to_store = "untracked".to_owned();
        assert_eq!(
            assess_energy_boundary(&unknown_store),
            Err(EnergyBoundaryError::UnknownTransferStore)
        );

        let mut self_transfer = balanced_snapshot();
        self_transfer.internal_transfers[0].to_store = "battery".to_owned();
        assert_eq!(
            assess_energy_boundary(&self_transfer),
            Err(EnergyBoundaryError::SameStoreTransfer)
        );
    }

    #[test]
    fn invalid_energy_magnitudes_and_tolerance_reject() {
        let mut negative_flow = balanced_snapshot();
        negative_flow.boundary_flows[0].amount = -1.0;
        assert_eq!(
            assess_energy_boundary(&negative_flow),
            Err(EnergyBoundaryError::InvalidFlowAmount)
        );

        let mut non_finite_flow = balanced_snapshot();
        non_finite_flow.boundary_flows[0].amount = f64::NAN;
        assert_eq!(
            assess_energy_boundary(&non_finite_flow),
            Err(EnergyBoundaryError::InvalidFlowAmount)
        );

        let mut negative_store = balanced_snapshot();
        negative_store.initial_stored_energy = -1.0;
        assert_eq!(
            assess_energy_boundary(&negative_store),
            Err(EnergyBoundaryError::InvalidStoredEnergy)
        );

        let mut loose = balanced_snapshot();
        loose.relative_tolerance = 1.1;
        assert_eq!(
            assess_energy_boundary(&loose),
            Err(EnergyBoundaryError::InvalidRelativeTolerance)
        );
    }

    #[test]
    fn overflow_during_flow_accumulation_fails_closed() {
        let mut input = balanced_snapshot();
        input.boundary_flows = vec![
            flow("large-1", BoundaryDirection::Inflow, 1.0e308),
            flow("large-2", BoundaryDirection::Inflow, 1.0e308),
        ];
        assert_eq!(
            assess_energy_boundary(&input),
            Err(EnergyBoundaryError::NonFiniteArithmetic)
        );
    }
}
