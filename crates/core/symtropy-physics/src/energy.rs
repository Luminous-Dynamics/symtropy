// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Deterministic, double-entry accounting for energy transfers.
//!
//! The ledger does not decide *how* a solver computes a transfer. It records
//! where modeled energy came from, where it went, in which form, and by which
//! mechanism. Every entry has one source and one destination, so internal
//! transfers are exactly balanced by construction.
//!
//! Boundary closure is deliberately distinct from accounting completeness. A
//! numerically tiny closure error is not evidence of first-law conservation if
//! one of the measured endpoint states omitted an invalid or unresolved modeled
//! reservoir. Use [`EnergyTransferLedger::audit_internal_energy_complete`] for
//! validation gates that require this stronger contract.

use serde::{Deserialize, Serialize};

use crate::body::BodyHandle;

/// Modeled forms of energy that can participate in a transfer.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnergyForm {
    Kinetic,
    GravitationalPotential,
    ThermalSensible,
    ThermalLatent,
    Elastic,
    FractureSurface,
    Chemical,
    Electrical,
    Radiant,
    /// Stable extension point for domain-specific forms without stringly typed
    /// identifiers in deterministic replay data.
    Other(u16),
}

/// Owner of an energy reservoir.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnergyOwner {
    Body(BodyHandle),
    /// A modeled world reservoir that remains inside the accounting boundary.
    World,
    /// A modeled environment reservoir that remains inside the boundary.
    Environment,
    /// Energy outside the modeled accounting boundary. The numeric id lets a
    /// caller distinguish multiple external sources/sinks deterministically.
    External(u64),
}

impl EnergyOwner {
    pub fn is_external(self) -> bool {
        matches!(self, Self::External(_))
    }
}

/// A concrete reservoir: an owner plus an energy form.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EnergyPort {
    pub owner: EnergyOwner,
    pub form: EnergyForm,
}

impl EnergyPort {
    pub const fn new(owner: EnergyOwner, form: EnergyForm) -> Self {
        Self { owner, form }
    }
}

/// Physical mechanism responsible for a transfer.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnergyTransferKind {
    ConductiveHeat,
    /// Prescribed sensible heat crossing the modeled accounting boundary.
    ExternalHeat,
    Friction,
    InelasticCollision,
    ViscousDissipation,
    PlasticWork,
    Fracture,
    PhaseChange,
    Radiation,
    ElectricalResistance,
    ChemicalReaction,
    ExternalWork,
    /// Stable extension point for domain-specific mechanisms.
    Other(u16),
}

/// One double-entry energy movement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnergyTransfer {
    /// Monotonic insertion order. This makes event ordering explicit in replay
    /// and evidence artifacts rather than depending on allocation order.
    pub sequence: u64,
    pub source: EnergyPort,
    pub destination: EnergyPort,
    /// Strictly positive transferred energy in joules.
    pub joules: f64,
    pub kind: EnergyTransferKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EnergyLedgerError {
    NonFiniteEnergy,
    NonPositiveEnergy,
    SelfTransfer,
    SequenceOverflow,
    NonFiniteAuditEnergy,
    /// A strict first-law audit was requested for an interval whose declared
    /// modeled reservoirs were not completely accounted for at both endpoints.
    IncompleteAccounting,
}

/// First-law boundary arithmetic for an accounting interval.
///
/// `closure_error_joules` only compares the supplied measured endpoint totals
/// against the ledger's net external flow. The audit does not by itself prove
/// that those measured totals contain every declared modeled reservoir. Research
/// gates should obtain this value through `audit_internal_energy_complete`.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnergyAudit {
    pub initial_internal_joules: f64,
    pub final_internal_joules: f64,
    /// Positive means net energy entered from `External` owners.
    pub net_external_joules: f64,
    /// `(final - initial) - net_external`.
    pub closure_error_joules: f64,
}

impl EnergyAudit {
    pub fn within_absolute_tolerance(self, tolerance_joules: f64) -> bool {
        tolerance_joules.is_finite()
            && tolerance_joules >= 0.0
            && self.closure_error_joules.abs() <= tolerance_joules
    }
}

/// Ordered energy-transfer journal for one run or accounting interval.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EnergyTransferLedger {
    entries: Vec<EnergyTransfer>,
    next_sequence: u64,
}

impl EnergyTransferLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entries(&self) -> &[EnergyTransfer] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_sequence = 0;
    }

    /// Record one physical transfer and return its deterministic sequence id.
    pub fn record(
        &mut self,
        source: EnergyPort,
        destination: EnergyPort,
        joules: f64,
        kind: EnergyTransferKind,
    ) -> Result<u64, EnergyLedgerError> {
        if !joules.is_finite() {
            return Err(EnergyLedgerError::NonFiniteEnergy);
        }
        if joules <= 0.0 {
            return Err(EnergyLedgerError::NonPositiveEnergy);
        }
        if source == destination {
            return Err(EnergyLedgerError::SelfTransfer);
        }

        let sequence = self.next_sequence;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(EnergyLedgerError::SequenceOverflow)?;
        self.entries.push(EnergyTransfer {
            sequence,
            source,
            destination,
            joules,
            kind,
        });
        Ok(sequence)
    }

    /// Net energy change represented by ledger entries for one reservoir.
    /// Incoming transfers are positive and outgoing transfers are negative.
    pub fn net_change_for(&self, port: EnergyPort) -> f64 {
        self.entries.iter().fold(0.0, |net, entry| {
            let incoming = if entry.destination == port {
                entry.joules
            } else {
                0.0
            };
            let outgoing = if entry.source == port {
                entry.joules
            } else {
                0.0
            };
            net + incoming - outgoing
        })
    }

    /// Net energy crossing the accounting boundary.
    ///
    /// External -> internal is positive; internal -> external is negative.
    /// Transfers wholly inside or wholly outside the boundary contribute zero.
    pub fn net_external_joules(&self) -> f64 {
        self.entries.iter().fold(0.0, |net, entry| {
            match (
                entry.source.owner.is_external(),
                entry.destination.owner.is_external(),
            ) {
                (true, false) => net + entry.joules,
                (false, true) => net - entry.joules,
                (false, false) | (true, true) => net,
            }
        })
    }

    /// Total transfer throughput. This is not a conserved-state quantity: an
    /// internal conversion contributes to throughput even though total modeled
    /// energy is unchanged.
    pub fn total_transferred_joules(&self) -> f64 {
        self.entries.iter().map(|entry| entry.joules).sum()
    }

    /// Compare a measured internal-energy change with external ledger flows.
    ///
    /// This is the low-level arithmetic operation. It verifies finite arithmetic
    /// but does **not** establish that the supplied endpoint totals account for
    /// every declared modeled reservoir. Use `audit_internal_energy_complete`
    /// for a first-law validation gate.
    pub fn audit_internal_energy(
        &self,
        initial_internal_joules: f64,
        final_internal_joules: f64,
    ) -> Result<EnergyAudit, EnergyLedgerError> {
        if !initial_internal_joules.is_finite() || !final_internal_joules.is_finite() {
            return Err(EnergyLedgerError::NonFiniteAuditEnergy);
        }

        let net_external_joules = self.net_external_joules();
        let observed_delta = final_internal_joules - initial_internal_joules;
        let closure_error_joules = observed_delta - net_external_joules;
        if !net_external_joules.is_finite()
            || !observed_delta.is_finite()
            || !closure_error_joules.is_finite()
        {
            return Err(EnergyLedgerError::NonFiniteAuditEnergy);
        }

        Ok(EnergyAudit {
            initial_internal_joules,
            final_internal_joules,
            net_external_joules,
            closure_error_joules,
        })
    }

    /// Strict first-law audit for a declared accounting interval.
    ///
    /// `initial_accounting_complete` and `final_accounting_complete` must mean
    /// that every reservoir declared by the caller's modeled validity contract
    /// was represented in the corresponding endpoint total. For example,
    /// `InvariantSnapshot::has_complete_modeled_energy_accounting()` supplies
    /// this evidence for the current rigid-body + sensible-thermal snapshot.
    ///
    /// This method deliberately refuses to turn a numerically small closure
    /// residual into a pass when either endpoint total is known to be partial.
    pub fn audit_internal_energy_complete(
        &self,
        initial_internal_joules: f64,
        final_internal_joules: f64,
        initial_accounting_complete: bool,
        final_accounting_complete: bool,
    ) -> Result<EnergyAudit, EnergyLedgerError> {
        if !initial_accounting_complete || !final_accounting_complete {
            return Err(EnergyLedgerError::IncompleteAccounting);
        }
        self.audit_internal_energy(initial_internal_joules, final_internal_joules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn port(body: usize, form: EnergyForm) -> EnergyPort {
        EnergyPort::new(EnergyOwner::Body(BodyHandle(body)), form)
    }

    #[test]
    fn internal_conversion_is_double_entry_balanced() {
        let kinetic = port(0, EnergyForm::Kinetic);
        let thermal = port(0, EnergyForm::ThermalSensible);
        let mut ledger = EnergyTransferLedger::new();
        ledger
            .record(kinetic, thermal, 125.0, EnergyTransferKind::Friction)
            .unwrap();

        assert_eq!(ledger.net_change_for(kinetic), -125.0);
        assert_eq!(ledger.net_change_for(thermal), 125.0);
        assert_eq!(ledger.net_external_joules(), 0.0);
        assert_eq!(ledger.total_transferred_joules(), 125.0);
    }

    #[test]
    fn external_flow_closes_complete_first_law_audit() {
        let external = EnergyPort::new(EnergyOwner::External(7), EnergyForm::Electrical);
        let thermal = port(2, EnergyForm::ThermalSensible);
        let mut ledger = EnergyTransferLedger::new();
        ledger
            .record(
                external,
                thermal,
                500.0,
                EnergyTransferKind::ElectricalResistance,
            )
            .unwrap();

        let audit = ledger
            .audit_internal_energy_complete(1_000.0, 1_500.0, true, true)
            .unwrap();
        assert_eq!(audit.net_external_joules, 500.0);
        assert_eq!(audit.closure_error_joules, 0.0);
        assert!(audit.within_absolute_tolerance(0.0));
    }

    #[test]
    fn strict_audit_rejects_incomplete_accounting_even_when_arithmetic_closes() {
        let external = EnergyPort::new(EnergyOwner::External(7), EnergyForm::Electrical);
        let thermal = port(2, EnergyForm::ThermalSensible);
        let mut ledger = EnergyTransferLedger::new();
        ledger
            .record(
                external,
                thermal,
                500.0,
                EnergyTransferKind::ElectricalResistance,
            )
            .unwrap();

        let arithmetic = ledger.audit_internal_energy(1_000.0, 1_500.0).unwrap();
        assert_eq!(arithmetic.closure_error_joules, 0.0);
        assert_eq!(
            ledger.audit_internal_energy_complete(1_000.0, 1_500.0, false, true),
            Err(EnergyLedgerError::IncompleteAccounting)
        );
        assert_eq!(
            ledger.audit_internal_energy_complete(1_000.0, 1_500.0, true, false),
            Err(EnergyLedgerError::IncompleteAccounting)
        );
    }

    #[test]
    fn audit_rejects_overflow_from_finite_endpoint_values() {
        let ledger = EnergyTransferLedger::new();
        assert_eq!(
            ledger.audit_internal_energy(f64::MAX, -f64::MAX),
            Err(EnergyLedgerError::NonFiniteAuditEnergy)
        );
    }

    #[test]
    fn deterministic_sequence_is_explicit() {
        let a = port(0, EnergyForm::Kinetic);
        let b = port(1, EnergyForm::ThermalSensible);
        let mut ledger = EnergyTransferLedger::new();
        assert_eq!(
            ledger
                .record(a, b, 1.0, EnergyTransferKind::InelasticCollision)
                .unwrap(),
            0
        );
        assert_eq!(
            ledger
                .record(b, a, 0.5, EnergyTransferKind::Other(9))
                .unwrap(),
            1
        );
        assert_eq!(ledger.entries()[0].sequence, 0);
        assert_eq!(ledger.entries()[1].sequence, 1);
    }

    #[test]
    fn invalid_entries_are_rejected() {
        let a = port(0, EnergyForm::Kinetic);
        let b = port(1, EnergyForm::ThermalSensible);
        let mut ledger = EnergyTransferLedger::new();

        assert_eq!(
            ledger.record(a, b, f64::NAN, EnergyTransferKind::Friction),
            Err(EnergyLedgerError::NonFiniteEnergy)
        );
        assert_eq!(
            ledger.record(a, b, 0.0, EnergyTransferKind::Friction),
            Err(EnergyLedgerError::NonPositiveEnergy)
        );
        assert_eq!(
            ledger.record(a, a, 1.0, EnergyTransferKind::Friction),
            Err(EnergyLedgerError::SelfTransfer)
        );
        assert!(ledger.is_empty());
    }
}
