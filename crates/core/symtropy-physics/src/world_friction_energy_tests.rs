// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Independent accounting checks for the kinetic-energy change caused by
//! the production contact-friction impulse path.

use super::*;
use crate::body::BodyType;
use symtropy_math::{Point, Sphere};

#[derive(Default)]
struct RecordingCallback {
    contact_energy_change: Option<f64>,
    dissipated_joules: f64,
}

impl PhysicsCallback<3> for RecordingCallback {
    fn modulate_force(&self, _body: BodyHandle, force: &SVector<f64, 3>) -> SVector<f64, 3> {
        *force
    }

    fn modulate_impulse(&self, impulse: f64, _point: &SVector<f64, 3>) -> f64 {
        impulse
    }

    fn friction_multiplier(&self, _point: &SVector<f64, 3>, _body: BodyHandle) -> f64 {
        1.0
    }

    fn on_collision(&mut self, _event: &CollisionEvent<3>) {}

    fn record_dissipation(&mut self, energy: f64) {
        self.dissipated_joules += energy;
    }

    fn record_work(&mut self, _body: BodyHandle, _work_joules: f64) {}

    fn record_contact_energy_change(
        &mut self,
        _body_a: BodyHandle,
        _body_b: BodyHandle,
        delta_kinetic_energy: f64,
    ) {
        self.contact_energy_change = Some(delta_kinetic_energy);
        if delta_kinetic_energy.is_finite() && delta_kinetic_energy < 0.0 {
            self.record_dissipation(-delta_kinetic_energy);
        }
    }

    fn apply_trauma(&mut self, _event: &CollisionEvent<3>) {}
}

#[test]
fn friction_callback_receives_measured_kinetic_energy_change() {
    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    world.baumgarte = 0.0;
    world.slop = 0.0;

    let mut fixed = RigidBody::<3>::static_body(
        BodyHandle(0),
        Point::origin(),
        Box::new(Sphere::<3>::unit()),
    );
    fixed.friction = 1.0;

    let mut moving =
        RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([0.0, 1.0, 0.0]), 0.5, 1.0);
    moving.linear_velocity = SVector::from([1.0, -1.0, 0.0]);
    moving.friction = 1.0;

    let fixed_handle = world.add_body(fixed);
    let moving_handle = world.add_body(moving);
    let contact_point = SVector::from([0.0, 1.0, 0.0]);
    let mut manifold = ContactManifold::single(
        fixed_handle,
        moving_handle,
        SVector::from([0.0, 1.0, 0.0]),
        contact_point,
        0.0,
    );
    manifold.points[0].restitution_bias = 0.0;

    // The normal impulse first removes the -1 m/s normal component. The
    // remaining 1 m/s tangential component has 0.5 J of kinetic energy.
    let mut callback = RecordingCallback::default();
    let solved = world.resolve_contact(manifold, 1.0 / 60.0, &mut callback);

    assert!((solved.points[0].lambda - 1.0).abs() < 1.0e-12);
    let delta = callback
        .contact_energy_change
        .expect("friction energy balance should be reported");
    assert!(
        (delta - (-0.5)).abs() < 1.0e-12,
        "one joule of tangential kinetic energy should lose 0.5 J under the applied friction impulse; got {delta}"
    );
    assert!(
        (callback.dissipated_joules - 0.5).abs() < 1.0e-12,
        "legacy dissipation callback should receive the measured positive loss"
    );
    assert!(
        world.bodies[1].linear_velocity.norm() < 1.0e-12,
        "normal and tangential velocities should both be resolved"
    );
}
