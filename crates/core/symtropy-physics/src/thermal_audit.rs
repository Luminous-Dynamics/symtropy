// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Audited thermodynamic couplings built on the core thermal kernel.
//!
//! The helpers in this module are transactional: a thermal state is committed
//! only if the corresponding energy-ledger entry is also accepted. This avoids
//! silent state mutation when accounting fails.

use crate::body::BodyHandle;
use crate::energy::{
    EnergyForm, EnergyLedgerError, EnergyOwner, EnergyPort, EnergyTransferKind,
    EnergyTransferLedger,
};
use crate::thermal::{
    HeatExchange, ThermalBody, ThermalError, conductive_exchange_bodies,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AuditedThermalError {
    Thermal(ThermalError),
    Ledger(EnergyLedgerError),
}

impl From<ThermalError> for AuditedThermalError {
    fn from(value: ThermalError) -> Self {
        Self::Thermal(value)
    }
}

impl From<EnergyLedgerError> for AuditedThermalError {
    fn from(value: EnergyLedgerError) -> Self {
        Self::Ledger(value)
    }
}

/// Exchange heat between two body-attached thermal states and record the exact
/// sensible-energy transfer in `ledger`.
///
/// The underlying exchange is first evaluated on copies of both thermal bodies.
/// Only after the ledger accepts the transfer are those next states committed,
/// so accounting and state mutation remain atomic from the caller's perspective.
/// A zero-transfer step produces no ledger entry.
pub fn conductive_exchange_bodies_audited(
    body_a: BodyHandle,
    thermal_a: &mut ThermalBody,
    body_b: BodyHandle,
    thermal_b: &mut ThermalBody,
    conductance_w_per_k: f64,
    dt_seconds: f64,
    ledger: &mut EnergyTransferLedger,
) -> Result<HeatExchange, AuditedThermalError> {
    let mut next_a = *thermal_a;
    let mut next_b = *thermal_b;
    let exchange = conductive_exchange_bodies(
        &mut next_a,
        &mut next_b,
        conductance_w_per_k,
        dt_seconds,
    )?;

    if exchange.joules_from_a_to_b != 0.0 {
        let a_port = EnergyPort::new(EnergyOwner::Body(body_a), EnergyForm::ThermalSensible);
        let b_port = EnergyPort::new(EnergyOwner::Body(body_b), EnergyForm::ThermalSensible);
        let (source, destination, joules) = if exchange.joules_from_a_to_b > 0.0 {
            (a_port, b_port, exchange.joules_from_a_to_b)
        } else {
            (b_port, a_port, -exchange.joules_from_a_to_b)
        };

        ledger.record(
            source,
            destination,
            joules,
            EnergyTransferKind::ConductiveHeat,
        )?;
    }

    *thermal_a = next_a;
    *thermal_b = next_b;
    Ok(exchange)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thermal::{ThermalMaterial, ThermalState};

    fn body(temp_k: f64) -> ThermalBody {
        ThermalBody::new(
            ThermalMaterial::new(1_000.0, 2.0, 0.5).unwrap(),
            ThermalState::new(temp_k).unwrap(),
            1.0,
        )
        .unwrap()
    }

    #[test]
    fn hotter_body_is_ledger_source() {
        let mut a = body(400.0);
        let mut b = body(300.0);
        let mut ledger = EnergyTransferLedger::new();

        let exchange = conductive_exchange_bodies_audited(
            BodyHandle(10),
            &mut a,
            BodyHandle(20),
            &mut b,
            20.0,
            1.0,
            &mut ledger,
        )
        .unwrap();

        assert!(exchange.joules_from_a_to_b > 0.0);
        assert_eq!(ledger.len(), 1);
        let entry = &ledger.entries()[0];
        assert_eq!(
            entry.source,
            EnergyPort::new(
                EnergyOwner::Body(BodyHandle(10)),
                EnergyForm::ThermalSensible
            )
        );
        assert_eq!(
            entry.destination,
            EnergyPort::new(
                EnergyOwner::Body(BodyHandle(20)),
                EnergyForm::ThermalSensible
            )
        );
        assert_eq!(entry.kind, EnergyTransferKind::ConductiveHeat);
        assert!((entry.joules - exchange.joules_from_a_to_b).abs() < 1e-12);
    }

    #[test]
    fn reverse_gradient_reverses_ledger_direction() {
        let mut a = body(250.0);
        let mut b = body(350.0);
        let mut ledger = EnergyTransferLedger::new();

        let exchange = conductive_exchange_bodies_audited(
            BodyHandle(1),
            &mut a,
            BodyHandle(2),
            &mut b,
            10.0,
            1.0,
            &mut ledger,
        )
        .unwrap();

        assert!(exchange.joules_from_a_to_b < 0.0);
        assert_eq!(ledger.entries()[0].source.owner, EnergyOwner::Body(BodyHandle(2)));
        assert_eq!(
            ledger.entries()[0].destination.owner,
            EnergyOwner::Body(BodyHandle(1))
        );
    }

    #[test]
    fn equal_temperatures_do_not_create_noise_entries() {
        let mut a = body(300.0);
        let mut b = body(300.0);
        let mut ledger = EnergyTransferLedger::new();

        let exchange = conductive_exchange_bodies_audited(
            BodyHandle(1),
            &mut a,
            BodyHandle(2),
            &mut b,
            10.0,
            1.0,
            &mut ledger,
        )
        .unwrap();

        assert_eq!(exchange.joules_from_a_to_b, 0.0);
        assert!(ledger.is_empty());
    }

    #[test]
    fn accounting_failure_does_not_commit_thermal_mutation() {
        let mut a = body(400.0);
        let mut b = body(300.0);
        let original_a = a;
        let original_b = b;
        let mut ledger = EnergyTransferLedger::new();

        let error = conductive_exchange_bodies_audited(
            BodyHandle(7),
            &mut a,
            BodyHandle(7),
            &mut b,
            20.0,
            1.0,
            &mut ledger,
        )
        .unwrap_err();

        assert_eq!(
            error,
            AuditedThermalError::Ledger(EnergyLedgerError::SelfTransfer)
        );
        assert_eq!(a, original_a);
        assert_eq!(b, original_b);
        assert!(ledger.is_empty());
    }
}
