// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use symtropy_physics::{
    AuditedThermalError, BodyHandle, EnergyForm, EnergyLedgerError, EnergyOwner, EnergyPort,
    EnergyTransferKind, EnergyTransferLedger, ThermalBody, ThermalError, ThermalMaterial,
    ThermalState, conductive_exchange_bodies_audited,
};

fn body(temp_k: f64) -> ThermalBody {
    ThermalBody::new(
        ThermalMaterial::new(1_000.0, 2.0, 0.5).unwrap(),
        ThermalState::new(temp_k).unwrap(),
        1.0,
    )
    .unwrap()
}

fn seed_ledger(ledger: &mut EnergyTransferLedger) {
    ledger
        .record(
            EnergyPort::new(EnergyOwner::External(99), EnergyForm::Electrical),
            EnergyPort::new(EnergyOwner::Body(BodyHandle(99)), EnergyForm::ThermalSensible),
            1.0,
            EnergyTransferKind::ElectricalResistance,
        )
        .unwrap();
}

#[test]
fn thermal_failure_preserves_state_history_and_next_sequence() {
    let mut a = body(400.0);
    let mut b = body(300.0);
    let mut ledger = EnergyTransferLedger::new();
    seed_ledger(&mut ledger);

    a.state.temperature_kelvin = -1.0;
    let original_a = a;
    let original_b = b;
    let original_entry = ledger.entries()[0].clone();

    let error = conductive_exchange_bodies_audited(
        BodyHandle(1),
        &mut a,
        BodyHandle(2),
        &mut b,
        20.0,
        1.0,
        &mut ledger,
    )
    .unwrap_err();

    assert_eq!(
        error,
        AuditedThermalError::Thermal(ThermalError::InvalidTemperature)
    );
    assert_eq!(a, original_a);
    assert_eq!(b, original_b);
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger.entries()[0], original_entry);

    // Repair the invalid input and prove the failed attempt did not consume a
    // deterministic ledger sequence number.
    a.state.temperature_kelvin = 400.0;
    conductive_exchange_bodies_audited(
        BodyHandle(1),
        &mut a,
        BodyHandle(2),
        &mut b,
        20.0,
        1.0,
        &mut ledger,
    )
    .unwrap();

    assert_eq!(ledger.len(), 2);
    assert_eq!(ledger.entries()[0].sequence, 0);
    assert_eq!(ledger.entries()[1].sequence, 1);
}

#[test]
fn ledger_failure_preserves_state_and_existing_history() {
    let mut a = body(400.0);
    let mut b = body(300.0);
    let original_a = a;
    let original_b = b;
    let mut ledger = EnergyTransferLedger::new();
    seed_ledger(&mut ledger);
    let original_entry = ledger.entries()[0].clone();

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
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger.entries()[0], original_entry);
}
