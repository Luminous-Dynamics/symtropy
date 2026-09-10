// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symtropy_powered_frame_core::{
    PoweredSubsystem, ResourceArbiter, ResourceMode, ResourceRequest,
};
use symtropy_protection_core::{
    Effect, EffectKind,
    active_field::{ActiveField, ActiveFieldConfig},
};

#[test]
fn hot_field_recovers_only_from_its_grant_and_remains_thermally_derated() {
    let mut field = ActiveField::new(ActiveFieldConfig {
        max_charge: 20.0,
        heat_per_absorbed: 3.5,
        ..ActiveFieldConfig::default()
    })
    .unwrap();

    let hit = field
        .intercept(Effect {
            magnitude: 80.0,
            kind: EffectKind::Impact,
        })
        .unwrap();
    assert_eq!(hit.absorbed, 20.0);
    assert_eq!(field.charge(), 0.0);
    assert_eq!(field.heat(), 70.0);
    assert!(field.thermal_scale() > 0.0 && field.thermal_scale() < 1.0);

    let receipt = ResourceArbiter::new(ResourceMode::MobilityPriority)
        .allocate(
            10.0,
            &[
                ResourceRequest {
                    subsystem: PoweredSubsystem::Cooling,
                    requested: 3.0,
                    minimum: 1.0,
                },
                ResourceRequest {
                    subsystem: PoweredSubsystem::Mobility,
                    requested: 8.0,
                    minimum: 2.0,
                },
                ResourceRequest {
                    subsystem: PoweredSubsystem::Protection,
                    requested: 8.0,
                    minimum: 0.0,
                },
            ],
        )
        .unwrap();

    let protection_grant = receipt
        .grant(PoweredSubsystem::Protection)
        .unwrap()
        .granted;
    let mobility_grant = receipt
        .grant(PoweredSubsystem::Mobility)
        .unwrap()
        .granted;
    assert!(mobility_grant > protection_grant);

    let service = field.service(protection_grant, 1.0).unwrap();
    assert!(service.accepted_resource <= protection_grant + 1e-12);
    assert!(service.stored_charge > 0.0);
    assert!(service.stored_charge < protection_grant);
    assert!(service.heat_after < 70.0);
    assert!(field.thermal_scale() < 1.0);
}
