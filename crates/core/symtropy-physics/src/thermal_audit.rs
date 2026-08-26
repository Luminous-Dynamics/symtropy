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
use crate::thermal::{HeatExchange, ThermalBody, ThermalError, conductive_exchange_bodies};

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

/// Validity errors for the current constant-heat-capacity entropy diagnostic.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EntropyAuditError {
    /// One of the supplied endpoint reservoirs is outside the thermal model's
    /// validity or numerical representability contract.
    InvalidThermalState(ThermalError),
    /// Classical `C ln(T2/T1)` is undefined at absolute zero.
    UndefinedAtAbsoluteZero,
    /// Material or effective thermal mass changed across the interval, so the
    /// constant-parameter analytical expression is not valid.
    ChangedThermalParameters,
    NonFiniteEntropy,
}

/// Entropy change for a closed two-body sensible-heat system under the current
/// constant-`c_p`, lumped-capacitance validity model.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PairEntropyAudit {
    pub body_a_delta_j_per_k: f64,
    pub body_b_delta_j_per_k: f64,
    pub total_delta_j_per_k: f64,
}

impl PairEntropyAudit {
    /// Passive conduction should not reduce total entropy. A small absolute
    /// tolerance lets callers account for floating-point roundoff explicitly.
    pub fn is_second_law_consistent(self, tolerance_j_per_k: f64) -> bool {
        tolerance_j_per_k.is_finite()
            && tolerance_j_per_k >= 0.0
            && self.total_delta_j_per_k >= -tolerance_j_per_k
    }
}

/// Evaluate entropy change for two thermal bodies before and after an interval.
///
/// This diagnostic is intentionally narrow. It assumes constant specific heat,
/// unchanged effective thermal mass, no phase change, and strictly positive
/// absolute temperatures. Every endpoint is first revalidated against the core
/// thermal model so finite-but-unphysical or unrepresentable reservoirs cannot
/// produce second-law evidence. Within that domain,
/// `Delta S = C ln(T_after/T_before)`.
pub fn constant_cp_pair_entropy_audit(
    before_a: ThermalBody,
    after_a: ThermalBody,
    before_b: ThermalBody,
    after_b: ThermalBody,
) -> Result<PairEntropyAudit, EntropyAuditError> {
    for body in [before_a, after_a, before_b, after_b] {
        body.validate()
            .map_err(EntropyAuditError::InvalidThermalState)?;
    }

    if before_a.material != after_a.material
        || before_b.material != after_b.material
        || before_a.thermal_mass_kg != after_a.thermal_mass_kg
        || before_b.thermal_mass_kg != after_b.thermal_mass_kg
    {
        return Err(EntropyAuditError::ChangedThermalParameters);
    }

    let temperatures = [
        before_a.state.temperature_kelvin,
        after_a.state.temperature_kelvin,
        before_b.state.temperature_kelvin,
        after_b.state.temperature_kelvin,
    ];
    if temperatures.iter().any(|temperature| *temperature <= 0.0) {
        return Err(EntropyAuditError::UndefinedAtAbsoluteZero);
    }

    let capacity_a = before_a
        .material
        .heat_capacity(before_a.thermal_mass_kg)
        .map_err(EntropyAuditError::InvalidThermalState)?;
    let capacity_b = before_b
        .material
        .heat_capacity(before_b.thermal_mass_kg)
        .map_err(EntropyAuditError::InvalidThermalState)?;
    let body_a_delta_j_per_k = capacity_a
        * (after_a.state.temperature_kelvin / before_a.state.temperature_kelvin).ln();
    let body_b_delta_j_per_k = capacity_b
        * (after_b.state.temperature_kelvin / before_b.state.temperature_kelvin).ln();
    let total_delta_j_per_k = body_a_delta_j_per_k + body_b_delta_j_per_k;

    if !body_a_delta_j_per_k.is_finite()
        || !body_b_delta_j_per_k.is_finite()
        || !total_delta_j_per_k.is_finite()
    {
        return Err(EntropyAuditError::NonFiniteEntropy);
    }

    Ok(PairEntropyAudit {
        body_a_delta_j_per_k,
        body_b_delta_j_per_k,
        total_delta_j_per_k,
    })
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
        assert_eq!(
            ledger.entries()[0].source.owner,
            EnergyOwner::Body(BodyHandle(2))
        );
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

    #[test]
    fn passive_conduction_produces_non_negative_total_entropy() {
        let mut a = body(400.0);
        let mut b = body(300.0);
        let before_a = a;
        let before_b = b;
        let mut ledger = EnergyTransferLedger::new();

        conductive_exchange_bodies_audited(
            BodyHandle(1),
            &mut a,
            BodyHandle(2),
            &mut b,
            50.0,
            1.0,
            &mut ledger,
        )
        .unwrap();

        let audit = constant_cp_pair_entropy_audit(before_a, a, before_b, b).unwrap();
        assert!(audit.total_delta_j_per_k > 0.0);
        assert!(audit.is_second_law_consistent(1e-12));
    }

    #[test]
    fn entropy_audit_detects_energy_conserving_cold_to_hot_transfer() {
        let before_a = body(400.0);
        let before_b = body(300.0);
        let after_a = body(410.0);
        let after_b = body(290.0);

        let audit =
            constant_cp_pair_entropy_audit(before_a, after_a, before_b, after_b).unwrap();
        assert!(audit.total_delta_j_per_k < 0.0);
        assert!(!audit.is_second_law_consistent(1e-12));
    }

    #[test]
    fn entropy_audit_explicitly_excludes_absolute_zero() {
        let zero = body(0.0);
        let warm = body(300.0);
        assert_eq!(
            constant_cp_pair_entropy_audit(zero, zero, warm, warm),
            Err(EntropyAuditError::UndefinedAtAbsoluteZero)
        );
    }

    #[test]
    fn entropy_audit_rejects_post_construction_invalid_reservoir() {
        let before_a = body(400.0);
        let mut after_a = body(390.0);
        let before_b = body(300.0);
        let after_b = body(310.0);
        after_a.material.emissivity = 1.5;

        assert_eq!(
            constant_cp_pair_entropy_audit(before_a, after_a, before_b, after_b),
            Err(EntropyAuditError::InvalidThermalState(
                ThermalError::InvalidEmissivity
            ))
        );
    }

    #[test]
    fn entropy_audit_rejects_unrepresentable_heat_capacity() {
        let mut before_a = body(400.0);
        let mut after_a = body(390.0);
        let before_b = body(300.0);
        let after_b = body(310.0);
        before_a.material.specific_heat_capacity = f64::MAX;
        after_a.material.specific_heat_capacity = f64::MAX;
        before_a.thermal_mass_kg = 2.0;
        after_a.thermal_mass_kg = 2.0;

        assert_eq!(
            constant_cp_pair_entropy_audit(before_a, after_a, before_b, after_b),
            Err(EntropyAuditError::InvalidThermalState(
                ThermalError::InvalidHeatCapacity
            ))
        );
    }
}
