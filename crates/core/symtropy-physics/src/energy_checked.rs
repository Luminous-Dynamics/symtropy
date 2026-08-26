// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked aggregate queries for [`EnergyTransferLedger`].
//!
//! Individual ledger entries are finite positive joule transfers, but a long or
//! adversarial accounting interval can still overflow an `f64` while aggregating
//! otherwise-valid entries. These helpers make aggregate representability an
//! explicit validation result instead of exposing `inf` as if it were evidence.

use crate::energy::{EnergyPort, EnergyTransferLedger};

/// Failures produced while reducing valid ledger entries into an aggregate.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EnergyAggregateError {
    /// A mathematically valid sequence of finite transfers could not be reduced
    /// into a finite `f64` aggregate in deterministic insertion order.
    NonFiniteAggregate,
}

/// Checked aggregate queries for the physical energy-transfer journal.
///
/// Existing convenience methods on [`EnergyTransferLedger`] remain available for
/// source compatibility. Strict validation/evidence paths should prefer this
/// trait so overflow cannot silently become a non-finite measurement.
pub trait EnergyTransferLedgerCheckedExt {
    /// Checked net energy change for one modeled reservoir.
    fn net_change_for_checked(&self, port: EnergyPort) -> Result<f64, EnergyAggregateError>;

    /// Checked net energy crossing the modeled accounting boundary.
    fn net_external_joules_checked(&self) -> Result<f64, EnergyAggregateError>;

    /// Checked sum of all transfer throughput in the interval.
    fn total_transferred_joules_checked(&self) -> Result<f64, EnergyAggregateError>;
}

impl EnergyTransferLedgerCheckedExt for EnergyTransferLedger {
    fn net_change_for_checked(&self, port: EnergyPort) -> Result<f64, EnergyAggregateError> {
        let mut net = 0.0;
        for entry in self.entries() {
            let delta = match (entry.source == port, entry.destination == port) {
                (true, false) => -entry.joules,
                (false, true) => entry.joules,
                (false, false) | (true, true) => 0.0,
            };
            net = checked_add(net, delta)?;
        }
        Ok(net)
    }

    fn net_external_joules_checked(&self) -> Result<f64, EnergyAggregateError> {
        let mut net = 0.0;
        for entry in self.entries() {
            let delta = match (
                entry.source.owner.is_external(),
                entry.destination.owner.is_external(),
            ) {
                (true, false) => entry.joules,
                (false, true) => -entry.joules,
                (false, false) | (true, true) => 0.0,
            };
            net = checked_add(net, delta)?;
        }
        Ok(net)
    }

    fn total_transferred_joules_checked(&self) -> Result<f64, EnergyAggregateError> {
        let mut total = 0.0;
        for entry in self.entries() {
            total = checked_add(total, entry.joules)?;
        }
        Ok(total)
    }
}

fn checked_add(lhs: f64, rhs: f64) -> Result<f64, EnergyAggregateError> {
    let result = lhs + rhs;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(EnergyAggregateError::NonFiniteAggregate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;
    use crate::energy::{EnergyForm, EnergyOwner, EnergyTransferKind};

    fn body_port(body: usize, form: EnergyForm) -> EnergyPort {
        EnergyPort::new(EnergyOwner::Body(BodyHandle(body)), form)
    }

    #[test]
    fn checked_aggregates_match_normal_finite_interval() {
        let external = EnergyPort::new(EnergyOwner::External(1), EnergyForm::Electrical);
        let kinetic = body_port(0, EnergyForm::Kinetic);
        let thermal = body_port(0, EnergyForm::ThermalSensible);
        let mut ledger = EnergyTransferLedger::new();
        ledger
            .record(external, kinetic, 100.0, EnergyTransferKind::ExternalWork)
            .unwrap();
        ledger
            .record(kinetic, thermal, 25.0, EnergyTransferKind::Friction)
            .unwrap();

        assert_eq!(ledger.net_external_joules_checked().unwrap(), 100.0);
        assert_eq!(ledger.net_change_for_checked(kinetic).unwrap(), 75.0);
        assert_eq!(ledger.net_change_for_checked(thermal).unwrap(), 25.0);
        assert_eq!(ledger.total_transferred_joules_checked().unwrap(), 125.0);
    }

    #[test]
    fn throughput_overflow_is_explicit_failure() {
        let a = body_port(0, EnergyForm::Kinetic);
        let b = body_port(1, EnergyForm::ThermalSensible);
        let mut ledger = EnergyTransferLedger::new();
        ledger
            .record(a, b, f64::MAX, EnergyTransferKind::Friction)
            .unwrap();
        ledger
            .record(a, b, f64::MAX, EnergyTransferKind::Friction)
            .unwrap();

        assert_eq!(
            ledger.total_transferred_joules_checked(),
            Err(EnergyAggregateError::NonFiniteAggregate)
        );
    }

    #[test]
    fn reservoir_net_change_overflow_is_explicit_failure() {
        let external = EnergyPort::new(EnergyOwner::External(9), EnergyForm::Electrical);
        let thermal = body_port(0, EnergyForm::ThermalSensible);
        let mut ledger = EnergyTransferLedger::new();
        for _ in 0..2 {
            ledger
                .record(
                    external,
                    thermal,
                    f64::MAX,
                    EnergyTransferKind::ExternalWork,
                )
                .unwrap();
        }

        assert_eq!(
            ledger.net_change_for_checked(thermal),
            Err(EnergyAggregateError::NonFiniteAggregate)
        );
        assert_eq!(
            ledger.net_external_joules_checked(),
            Err(EnergyAggregateError::NonFiniteAggregate)
        );
    }

    #[test]
    fn wholly_external_transfers_do_not_affect_boundary_net() {
        let a = EnergyPort::new(EnergyOwner::External(1), EnergyForm::Electrical);
        let b = EnergyPort::new(EnergyOwner::External(2), EnergyForm::ThermalSensible);
        let mut ledger = EnergyTransferLedger::new();
        ledger
            .record(a, b, f64::MAX, EnergyTransferKind::ExternalWork)
            .unwrap();
        ledger
            .record(a, b, f64::MAX, EnergyTransferKind::ExternalWork)
            .unwrap();

        assert_eq!(ledger.net_external_joules_checked().unwrap(), 0.0);
        assert_eq!(
            ledger.total_transferred_joules_checked(),
            Err(EnergyAggregateError::NonFiniteAggregate)
        );
    }

    #[test]
    fn deterministic_intermediate_overflow_fails_even_if_later_entry_would_cancel() {
        let external = EnergyPort::new(EnergyOwner::External(1), EnergyForm::Electrical);
        let thermal = body_port(0, EnergyForm::ThermalSensible);
        let mut ledger = EnergyTransferLedger::new();
        ledger
            .record(
                external,
                thermal,
                f64::MAX,
                EnergyTransferKind::ExternalWork,
            )
            .unwrap();
        ledger
            .record(
                external,
                thermal,
                f64::MAX,
                EnergyTransferKind::ExternalWork,
            )
            .unwrap();
        ledger
            .record(
                thermal,
                external,
                f64::MAX,
                EnergyTransferKind::ExternalWork,
            )
            .unwrap();

        assert_eq!(
            ledger.net_external_joules_checked(),
            Err(EnergyAggregateError::NonFiniteAggregate)
        );
    }
}
