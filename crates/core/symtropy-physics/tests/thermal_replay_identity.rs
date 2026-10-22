// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::replay::WorldSnapshot;
use symtropy_physics::{PhysicsWorld, ThermalBody, ThermalMaterial, ThermalState};

fn one_body_world() -> (PhysicsWorld<3>, symtropy_physics::BodyHandle) {
    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
    (world, handle)
}

fn thermal_body(temperature_kelvin: f64) -> ThermalBody {
    ThermalBody::new(
        ThermalMaterial::new(500.0, 2.0, 0.5).unwrap(),
        ThermalState::new(temperature_kelvin).unwrap(),
        1.0,
    )
    .unwrap()
}

#[test]
fn temperature_only_difference_changes_bitwise_world_snapshot() {
    let (mut cold, cold_handle) = one_body_world();
    let (mut hot, hot_handle) = one_body_world();

    cold.body_mut(cold_handle)
        .unwrap()
        .set_thermal(thermal_body(300.0));
    hot.body_mut(hot_handle)
        .unwrap()
        .set_thermal(thermal_body(301.0));

    let cold_snapshot = WorldSnapshot::capture(&cold);
    let hot_snapshot = WorldSnapshot::capture(&hot);

    assert_ne!(cold_snapshot, hot_snapshot);
    assert_eq!(
        cold_snapshot.bodies[0].thermal_temperature_kelvin,
        Some(300.0_f64.to_bits())
    );
    assert_eq!(
        hot_snapshot.bodies[0].thermal_temperature_kelvin,
        Some(301.0_f64.to_bits())
    );
}

#[test]
fn absent_thermal_state_differs_from_attached_zero_kelvin_reservoir() {
    let (without_thermal, _) = one_body_world();
    let (mut at_zero_kelvin, handle) = one_body_world();

    at_zero_kelvin
        .body_mut(handle)
        .unwrap()
        .set_thermal(thermal_body(0.0));

    let absent_snapshot = WorldSnapshot::capture(&without_thermal);
    let zero_kelvin_snapshot = WorldSnapshot::capture(&at_zero_kelvin);

    assert_ne!(absent_snapshot, zero_kelvin_snapshot);
    assert_eq!(absent_snapshot.bodies[0].thermal_temperature_kelvin, None);
    assert_eq!(
        zero_kelvin_snapshot.bodies[0].thermal_temperature_kelvin,
        Some(0.0_f64.to_bits())
    );
}
