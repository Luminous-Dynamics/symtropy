// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Determinism helpers: record/replay command streams and bitwise snapshots.
//!
//! This module intentionally keeps the command vocabulary small. Commands that
//! cross the modeled energy boundary require the audited executor so replay
//! cannot silently mutate thermal state without a matching energy record.

use nalgebra::SVector;
use symtropy_math::Bivector;

use crate::body::{BodyHandle, BodyType, RigidBody};
use crate::energy::EnergyTransferLedger;
use crate::external_heat::{ExternalHeatError, exchange_external_heat_audited};
use crate::integrator;
use crate::world::PhysicsWorld;

/// Commands that mutate a physics world at a tick boundary.
#[derive(Clone, Debug)]
pub enum WorldCommand<const D: usize> {
    ApplyForce {
        body: BodyHandle,
        force: Box<SVector<f64, D>>,
    },
    ApplyImpulse {
        body: BodyHandle,
        impulse: Box<SVector<f64, D>>,
    },
    SetLinearVelocity {
        body: BodyHandle,
        velocity: Box<SVector<f64, D>>,
    },
    SetAngularVelocity {
        body: BodyHandle,
        velocity: Box<Bivector<D>>,
    },
    /// Signed sensible heat across the accounting boundary. Positive enters the
    /// body, negative leaves it. This command requires `apply_commands_audited`.
    ApplyExternalHeat {
        body: BodyHandle,
        signed_joules: f64,
        external_source_id: u64,
    },
    Wake {
        body: BodyHandle,
    },
}

/// A single replay frame: `dt` + ordered list of commands to apply before stepping.
#[derive(Clone, Debug)]
pub struct ReplayFrame<const D: usize> {
    pub dt: f64,
    pub commands: Vec<WorldCommand<D>>,
}

/// A full replay tape: ordered frames.
#[derive(Clone, Debug, Default)]
pub struct ReplayTape<const D: usize> {
    pub frames: Vec<ReplayFrame<D>>,
}

impl<const D: usize> ReplayTape<D> {
    pub fn push_frame(&mut self, dt: f64, commands: Vec<WorldCommand<D>>) {
        self.frames.push(ReplayFrame { dt, commands });
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyCommandError {
    MissingBody(BodyHandle),
    /// An energy-boundary command was passed to the legacy untracked executor.
    EnergyLedgerRequired,
    ExternalHeat(ExternalHeatError),
}

impl From<ExternalHeatError> for ApplyCommandError {
    fn from(value: ExternalHeatError) -> Self {
        Self::ExternalHeat(value)
    }
}

fn apply_non_boundary_command<const D: usize>(
    world: &mut PhysicsWorld<D>,
    command: &WorldCommand<D>,
) -> Result<bool, ApplyCommandError> {
    match command {
        WorldCommand::ApplyForce { body, force } => {
            let Some(b) = world.body_mut(*body) else {
                return Err(ApplyCommandError::MissingBody(*body));
            };
            b.apply_force(**force);
        }
        WorldCommand::ApplyImpulse { body, impulse } => {
            let Some(b) = world.body_mut(*body) else {
                return Err(ApplyCommandError::MissingBody(*body));
            };
            integrator::apply_impulse(b, &**impulse);
        }
        WorldCommand::SetLinearVelocity { body, velocity } => {
            let Some(b) = world.body_mut(*body) else {
                return Err(ApplyCommandError::MissingBody(*body));
            };
            b.linear_velocity = **velocity;
        }
        WorldCommand::SetAngularVelocity { body, velocity } => {
            let Some(b) = world.body_mut(*body) else {
                return Err(ApplyCommandError::MissingBody(*body));
            };
            b.angular_velocity = **velocity;
        }
        WorldCommand::Wake { body } => {
            let Some(b) = world.body_mut(*body) else {
                return Err(ApplyCommandError::MissingBody(*body));
            };
            b.wake();
        }
        WorldCommand::ApplyExternalHeat { .. } => return Ok(false),
    }
    Ok(true)
}

/// Apply commands that do not cross an audited energy boundary.
///
/// `ApplyExternalHeat` is deliberately rejected here; callers must use
/// `apply_commands_audited` so the matching boundary transfer is recorded.
pub fn apply_commands<const D: usize>(
    world: &mut PhysicsWorld<D>,
    commands: &[WorldCommand<D>],
) -> Result<(), ApplyCommandError> {
    for command in commands {
        if !apply_non_boundary_command(world, command)? {
            return Err(ApplyCommandError::EnergyLedgerRequired);
        }
    }
    Ok(())
}

/// Apply all replay commands while recording energy-boundary interventions.
pub fn apply_commands_audited<const D: usize>(
    world: &mut PhysicsWorld<D>,
    commands: &[WorldCommand<D>],
    ledger: &mut EnergyTransferLedger,
) -> Result<(), ApplyCommandError> {
    for command in commands {
        if apply_non_boundary_command(world, command)? {
            continue;
        }

        let WorldCommand::ApplyExternalHeat {
            body,
            signed_joules,
            external_source_id,
        } = command
        else {
            unreachable!("only boundary command returns false");
        };

        let Some(target) = world.body_mut(*body) else {
            return Err(ApplyCommandError::MissingBody(*body));
        };
        exchange_external_heat_audited(
            *body,
            target,
            *signed_joules,
            *external_source_id,
            ledger,
        )?;
    }
    Ok(())
}

/// Bitwise snapshot of a rigid body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodySnapshot<const D: usize> {
    pub handle: BodyHandle,
    pub body_type: BodyType,
    pub translation: [u64; D],
    pub rotation: [[u64; D]; D],
    pub linear_velocity: [u64; D],
    pub angular_velocity: [[u64; D]; D],
    pub sleeping: bool,
    pub sleep_counter: u32,
    pub thermal_temperature_kelvin: Option<u64>,
    pub thermal_mass_kg: Option<u64>,
    pub thermal_specific_heat_capacity: Option<u64>,
    pub thermal_conductivity: Option<u64>,
    pub thermal_emissivity: Option<u64>,
}

impl<const D: usize> BodySnapshot<D> {
    pub fn from_body(body: &RigidBody<D>) -> Self {
        let translation = std::array::from_fn(|i| body.transform.translation.0[i].to_bits());

        let rot = body.transform.rotation.to_matrix();
        let rotation = std::array::from_fn(|r| std::array::from_fn(|c| rot[(r, c)].to_bits()));

        let linear_velocity = std::array::from_fn(|i| body.linear_velocity[i].to_bits());

        let ang = body.angular_velocity.to_matrix();
        let angular_velocity =
            std::array::from_fn(|r| std::array::from_fn(|c| ang[(r, c)].to_bits()));

        let (
            thermal_temperature_kelvin,
            thermal_mass_kg,
            thermal_specific_heat_capacity,
            thermal_conductivity,
            thermal_emissivity,
        ) = if let Some(thermal) = body.thermal {
            (
                Some(thermal.state.temperature_kelvin.to_bits()),
                Some(thermal.thermal_mass_kg.to_bits()),
                Some(thermal.material.specific_heat_capacity.to_bits()),
                Some(thermal.material.thermal_conductivity.to_bits()),
                Some(thermal.material.emissivity.to_bits()),
            )
        } else {
            (None, None, None, None, None)
        };

        Self {
            handle: body.handle,
            body_type: body.body_type,
            translation,
            rotation,
            linear_velocity,
            angular_velocity,
            sleeping: body.sleeping,
            sleep_counter: body.sleep_counter,
            thermal_temperature_kelvin,
            thermal_mass_kg,
            thermal_specific_heat_capacity,
            thermal_conductivity,
            thermal_emissivity,
        }
    }
}

/// Bitwise snapshot of a collision event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollisionEventSnapshot<const D: usize> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub impulse: u64,
    pub normal: [u64; D],
    pub depth: u64,
}

impl<const D: usize> CollisionEventSnapshot<D> {
    pub fn from_event(event: &crate::contact::CollisionEvent<D>) -> Self {
        Self {
            body_a: event.body_a,
            body_b: event.body_b,
            impulse: event.impulse.to_bits(),
            normal: std::array::from_fn(|i| event.normal[i].to_bits()),
            depth: event.depth.to_bits(),
        }
    }
}

/// Bitwise snapshot of a physics world (bodies + last-step collision events).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldSnapshot<const D: usize> {
    pub bodies: Vec<BodySnapshot<D>>,
    pub collision_events: Vec<CollisionEventSnapshot<D>>,
}

impl<const D: usize> WorldSnapshot<D> {
    pub fn capture(world: &PhysicsWorld<D>) -> Self {
        let mut bodies: Vec<_> = world.bodies.iter().map(BodySnapshot::from_body).collect();
        bodies.sort_by_key(|b| b.handle);

        let mut collision_events: Vec<_> = world
            .collision_events
            .iter()
            .map(CollisionEventSnapshot::from_event)
            .collect();
        collision_events.sort_by_key(|e| (e.body_a, e.body_b, e.impulse, e.depth));

        Self {
            bodies,
            collision_events,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thermal::{ThermalBody, ThermalMaterial, ThermalState};
    use symtropy_math::Point;

    fn thermal_world() -> (PhysicsWorld<3>, BodyHandle) {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        world.body_mut(handle).unwrap().set_thermal(
            ThermalBody::new(
                ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
                ThermalState::new(300.0).unwrap(),
                1.0,
            )
            .unwrap(),
        );
        (world, handle)
    }

    #[test]
    fn external_heat_requires_audited_executor() {
        let (mut world, handle) = thermal_world();
        let command = WorldCommand::ApplyExternalHeat {
            body: handle,
            signed_joules: 1_000.0,
            external_source_id: 5,
        };
        assert_eq!(
            apply_commands(&mut world, &[command]),
            Err(ApplyCommandError::EnergyLedgerRequired)
        );
        assert_eq!(
            world.body(handle).unwrap().thermal.unwrap().state.temperature_kelvin,
            300.0
        );
    }

    #[test]
    fn audited_external_heat_is_bitwise_replayable() {
        fn run() -> (WorldSnapshot<3>, EnergyTransferLedger) {
            let (mut world, handle) = thermal_world();
            let mut ledger = EnergyTransferLedger::new();
            let command = WorldCommand::ApplyExternalHeat {
                body: handle,
                signed_joules: 2_000.0,
                external_source_id: 8,
            };
            apply_commands_audited(&mut world, &[command], &mut ledger).unwrap();
            (WorldSnapshot::capture(&world), ledger)
        }

        let (snapshot_a, ledger_a) = run();
        let (snapshot_b, ledger_b) = run();
        assert_eq!(snapshot_a, snapshot_b);
        assert_eq!(ledger_a, ledger_b);
    }
}
