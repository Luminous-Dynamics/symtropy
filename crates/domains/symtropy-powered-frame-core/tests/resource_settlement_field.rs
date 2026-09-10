// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symtropy_powered_frame_core::{
    PoweredSubsystem, ResourceArbiter, ResourceMode, ResourceRequest,
    settlement::{ConsumptionDisposition, ResourceEpoch, ResourceSettlement},
};
use symtropy_protection_core::{
    Effect, EffectKind,
    active_field::{ActiveField, ActiveFieldConfig},
};

#[test]
fn repeated_consumption_identity_does_not_reapply_field_recharge() {
    let mut field = ActiveField::new(ActiveFieldConfig {
        max_charge: 10.0,
        ..ActiveFieldConfig::default()
    })
    .unwrap();
    field
        .intercept(Effect {
            magnitude: 20.0,
            kind: EffectKind::Impact,
        })
        .unwrap();
    assert_eq!(field.charge(), 0.0);

    let allocation = ResourceArbiter::new(ResourceMode::ProtectionPriority)
        .allocate(
            4.0,
            &[ResourceRequest {
                subsystem: PoweredSubsystem::Protection,
                requested: 4.0,
                minimum: 0.0,
            }],
        )
        .unwrap();
    let mut settlement = ResourceSettlement::new(
        ResourceEpoch::new("pack:arin", 50, 0).unwrap(),
        allocation,
    );

    let first = settlement
        .consume("field-service:50", PoweredSubsystem::Protection, 4.0)
        .unwrap();
    assert_eq!(first.disposition, ConsumptionDisposition::New);
    if first.is_new() {
        field.service(first.receipt.amount, 1.0).unwrap();
    }
    let charge_after_first_application = field.charge();
    assert!(charge_after_first_application > 0.0);

    // A transport/network retry returns the same receipt, but its Replay disposition
    // tells the integration not to execute the field mutation again.
    let retry = settlement
        .consume("field-service:50", PoweredSubsystem::Protection, 4.0)
        .unwrap();
    assert_eq!(retry.disposition, ConsumptionDisposition::Replay);
    assert_eq!(retry.receipt, first.receipt);
    if retry.is_new() {
        field.service(retry.receipt.amount, 1.0).unwrap();
    }

    assert_eq!(field.charge(), charge_after_first_application);
    assert_eq!(settlement.remaining(PoweredSubsystem::Protection), 0.0);
}
