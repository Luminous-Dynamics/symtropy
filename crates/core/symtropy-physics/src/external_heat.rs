// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Audited heat exchange across the modeled accounting boundary.
//!
//! Positive signed energy enters a body from an external reservoir; negative
//! signed energy leaves the body. State mutation is transactional with ledger
//! insertion, so a failed accounting operation cannot silently change temperature.

use crate::body::{BodyHandle, RigidBody};
use crate::energy::{
    EnergyForm, EnergyLedgerError, EnergyOwner, EnergyPort, EnergyTransferKind,
    EnergyTransferLedger,
};
use crate::thermal::{ThermalBody, ThermalError};

/// Stable reserved mechanism id for direct thermal exchange across the accounting
/// boundary. The core ledger deliberately provides `Other(u16)` as a deterministic
/// extension point; this id is owned by the thermal boundary layer.
pub const EXTERNAL_HEAT_TRANSFER_KIND_ID: u16 = 1;

pub const EXTERNAL_HEAT_TRANSFER_KIND: EnergyTransferKind =
    EnergyTransferKind::Other(EXTERNAL_HEAT_TRANSFER_KIND_ID);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExternalHeatError {
    Thermal(ThermalError),
    Ledger(EnergyLedgerError),
    MissingThermalState,
    NonFiniteEnergy,
}

impl From<ThermalError> for ExternalHeatError {
    fn from(value: ThermalError) -> Self {
        Self::Thermal(value)
    }
}

impl From<EnergyLedgerError> for ExternalHeatError {
    fn from(value: EnergyLedgerError) -> Self {
        Self::Ledger(value)
    }
}

/// Apply signed heat to a standalone thermal reservoir while recording the exact
/// boundary transfer.
///
/// This is crate-visible so replay can stage a whole command batch without
/// cloning a `RigidBody` and its collider. It has the same transactional contract
/// as `exchange_external_heat_audited`: the thermal state is committed only after
/// the corresponding ledger entry succeeds.
pub(crate) fn exchange_external_heat_thermal_audited(
    body_handle: BodyHandle,
    thermal: &mut ThermalBody,
    signed_joules: f64,
    external_source_id: u64,
    ledger: &mut EnergyTransferLedger,
) -> Result<f64, ExternalHeatError> {
    if !signed_joules.is_finite() {
        return Err(ExternalHeatError::NonFiniteEnergy);
    }

    // A zero transfer is quiet, but it is not a loophole around the thermal
    // validity contract. Callers still receive an error for corrupted state.
    thermal.validate()?;
    if signed_joules == 0.0 {
        return Ok(thermal.state.temperature_kelvin);
    }

    let mut next = *thermal;
    let next_temperature = next.add_heat_joules(signed_joules)?;

    let body_port = EnergyPort::new(
        EnergyOwner::Body(body_handle),
        EnergyForm::ThermalSensible,
    );
    let external_port = EnergyPort::new(
        EnergyOwner::External(external_source_id),
        EnergyForm::ThermalSensible,
    );

    let (source, destination, joules) = if signed_joules > 0.0 {
        (external_port, body_port, signed_joules)
    } else {
        (body_port, external_port, -signed_joules)
    };

    ledger.record(source, destination, joules, EXTERNAL_HEAT_TRANSFER_KIND)?;
    *thermal = next;
    Ok(next_temperature)
}

/// Apply signed heat across the accounting boundary.
///
/// `signed_joules > 0` means external -> body. `signed_joules < 0` means body ->
/// external. A valid zero transfer is a no-op and produces no ledger entry.
pub fn exchange_external_heat_audited<const D: usize>(
    body_handle: BodyHandle,
    body: &mut RigidBody<D>,
    signed_joules: f64,
    external_source_id: u64,
    ledger: &mut EnergyTransferLedger,
) -> Result<f64, ExternalHeatError> {
    let thermal = body
        .thermal
        .as_mut()
        .ok_or(ExternalHeatError::MissingThermalState)?;
    exchange_external_heat_thermal_audited(
        body_handle,
        thermal,
        signed_joules,
        external_source_id,
        ledger,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thermal::{ThermalMaterial, ThermalState};
    use symtropy_math::Point;

    fn body(temp_k: f64) -> RigidBody<3> {
        let mut body = RigidBody::dynamic_sphere(BodyHandle(4), Point::origin(), 0.5, 1.0);
        body.set_thermal(
            ThermalBody::new(
                ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
                ThermalState::new(temp_k).unwrap(),
                2.0,
            )
            .unwrap(),
        );
        body
    }

    #[test]
    fn positive_heat_enters_from_external_reservoir() {
        let mut body = body(300.0);
        let mut ledger = EnergyTransferLedger::new();
        let next = exchange_external_heat_audited(
            BodyHandle(4),
            &mut body,
            2_000.0,
            9,
            &mut ledger,
        )
        .unwrap();

        assert!((next - 301.0).abs() < 1e-12);
        assert_eq!(ledger.len(), 1);
        let entry = &ledger.entries()[0];
        assert_eq!(entry.source.owner, EnergyOwner::External(9));
        assert_eq!(entry.destination.owner, EnergyOwner::Body(BodyHandle(4)));
        assert_eq!(entry.kind, EXTERNAL_HEAT_TRANSFER_KIND);
        assert_eq!(entry.joules, 2_000.0);
    }

    #[test]
    fn negative_heat_is_an_external_sink() {
        let mut body = body(300.0);
        let mut ledger = EnergyTransferLedger::new();
        exchange_external_heat_audited(
            BodyHandle(4),
            &mut body,
            -1_000.0,
            3,
            &mut ledger,
        )
        .unwrap();

        let entry = &ledger.entries()[0];
        assert_eq!(entry.source.owner, EnergyOwner::Body(BodyHandle(4)));
        assert_eq!(entry.destination.owner, EnergyOwner::External(3));
        assert_eq!(ledger.net_external_joules(), -1_000.0);
    }

    #[test]
    fn failed_cooling_does_not_mutate_body_or_ledger() {
        let mut body = body(1.0);
        let before = body.thermal.unwrap();
        let mut ledger = EnergyTransferLedger::new();

        let error = exchange_external_heat_audited(
            BodyHandle(4),
            &mut body,
            -10_000.0,
            3,
            &mut ledger,
        )
        .unwrap_err();

        assert_eq!(error, ExternalHeatError::Thermal(ThermalError::InvalidTemperature));
        assert_eq!(body.thermal.unwrap(), before);
        assert!(ledger.is_empty());
    }

    #[test]
    fn zero_transfer_is_quiet_but_still_revalidates_state() {
        let mut body = body(300.0);
        let mut ledger = EnergyTransferLedger::new();
        let next = exchange_external_heat_audited(
            BodyHandle(4),
            &mut body,
            0.0,
            3,
            &mut ledger,
        )
        .unwrap();
        assert_eq!(next, 300.0);
        assert!(ledger.is_empty());

        body.thermal.as_mut().unwrap().material.emissivity = 1.5;
        assert_eq!(
            exchange_external_heat_audited(BodyHandle(4), &mut body, 0.0, 3, &mut ledger),
            Err(ExternalHeatError::Thermal(ThermalError::InvalidEmissivity))
        );
        assert!(ledger.is_empty());
    }
}
