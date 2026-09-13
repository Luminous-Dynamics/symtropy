// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Launcher adapter binding the core solver friction-authority contract to the
//! fixed-tick thermodynamic transaction runtime.

use nalgebra::SVector;
use symtropy_physics::{
    FrictionImpulseAuthority, FrictionSolverCoordinates, HeatPartition, RigidBody,
};

use super::thermodynamic_runtime::{RuntimeFrictionError, ThermodynamicTransactionRuntime};

/// Borrowed production friction authority for one physics consequence.
///
/// The heat partition is explicit at construction rather than hidden inside the
/// core physics solver. The runtime remains the owner of fixed-tick identity,
/// friction lifecycle, thermal promotion, and the canonical physical ledger.
pub struct ThermodynamicFrictionAuthority<'a> {
    runtime: &'a mut ThermodynamicTransactionRuntime,
    partition: HeatPartition,
}

impl<'a> ThermodynamicFrictionAuthority<'a> {
    pub fn new(
        runtime: &'a mut ThermodynamicTransactionRuntime,
        partition: HeatPartition,
    ) -> Self {
        Self { runtime, partition }
    }

    /// Current v0.1 production policy: split certified centered friction heat
    /// equally between the two dynamic participants.
    pub fn equal(runtime: &'a mut ThermodynamicTransactionRuntime) -> Self {
        Self::new(runtime, HeatPartition::equal())
    }
}

impl<const D: usize> FrictionImpulseAuthority<D> for ThermodynamicFrictionAuthority<'_> {
    type Error = RuntimeFrictionError;

    fn execute_friction_impulse(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        contact_point: &SVector<f64, D>,
        impulse_on_b: &SVector<f64, D>,
        coordinates: FrictionSolverCoordinates,
    ) -> Result<(), Self::Error> {
        self.runtime
            .execute_terminal_friction_impulse_at(
                body_a,
                body_b,
                contact_point,
                impulse_on_b,
                coordinates,
                self.partition,
            )
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Point;
    use symtropy_physics::{
        BodyHandle, FrictionTransactionId, FrictionTransactionPhase, ThermalBody,
        ThermalMaterial, ThermalState,
    };

    fn thermal_body(handle: usize, velocity_x: f64) -> RigidBody<3> {
        let mut body = RigidBody::<3>::dynamic_sphere(
            BodyHandle(handle),
            Point::origin(),
            0.5,
            1.0,
        );
        body.linear_velocity[0] = velocity_x;
        body.set_thermal(
            ThermalBody::new(
                ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
                ThermalState::new(300.0).unwrap(),
                1.0,
            )
            .unwrap(),
        );
        body
    }

    #[test]
    fn adapter_routes_core_authority_request_into_runtime_terminalization() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = thermal_body(1, 1.0);
        let mut b = thermal_body(2, 0.0);

        {
            let mut authority = ThermodynamicFrictionAuthority::equal(&mut runtime);
            authority
                .execute_friction_impulse(
                    &mut a,
                    &mut b,
                    &SVector::zeros(),
                    &SVector::from([0.5, 0.0, 0.0]),
                    FrictionSolverCoordinates::new(2, 3, 4),
                )
                .unwrap();
        }

        let id = FrictionTransactionId::new(0, 2, 3, 4);
        assert_eq!(
            runtime.friction_journal().phase(id),
            Some(FrictionTransactionPhase::Promoted)
        );
        assert_eq!(runtime.physical_energy_ledger().len(), 3);
        assert_eq!(runtime.pending_friction_reservation_count(), 0);
    }
}
