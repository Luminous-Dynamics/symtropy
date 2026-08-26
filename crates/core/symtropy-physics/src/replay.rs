// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Determinism helpers: record/replay command streams and bitwise snapshots.
//!
//! This module intentionally keeps the command vocabulary small. Commands that
//! cross the modeled energy boundary require the audited executor so replay
//! cannot silently mutate thermal state without a matching energy record.

use std::collections::BTreeMap;

use nalgebra::SVector;
use symtropy_math::Bivector;

use crate::body::{BodyHandle, BodyType, RigidBody};
use crate::energy::EnergyTransferLedger;
use crate::external_heat::{
    ExternalHeatError, exchange_external_heat_audited, exchange_external_heat_thermal_audited,
};
use crate::integrator;
use crate::thermal::ThermalBody;
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

fn command_body<const D: usize>(command: &WorldCommand<D>) -> BodyHandle {
    match command {
        WorldCommand::ApplyForce { body, .. }
        | WorldCommand::ApplyImpulse { body, .. }
        | WorldCommand::SetLinearVelocity { body, .. }
        | WorldCommand::SetAngularVelocity { body, .. }
        | WorldCommand::ApplyExternalHeat { body, .. }
        | WorldCommand::Wake { body } => *body,
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

fn preflight_untracked_commands<const D: usize>(
    world: &PhysicsWorld<D>,
    commands: &[WorldCommand<D>],
) -> Result<(), ApplyCommandError> {
    for command in commands {
        let body = command_body(command);
        if world.body(body).is_none() {
            return Err(ApplyCommandError::MissingBody(body));
        }
        if matches!(command, WorldCommand::ApplyExternalHeat { .. }) {
            return Err(ApplyCommandError::EnergyLedgerRequired);
        }
    }
    Ok(())
}

/// Preflight the full audited command slice without mutating the authoritative
/// world or ledger.
///
/// Current non-boundary commands are infallible after body-existence validation.
/// Boundary heat commands are evaluated sequentially against copied thermal
/// reservoirs and a cloned ledger, so repeated heat commands for one body observe
/// the staged result of prior commands in the same batch. If the command vocabulary
/// gains another fallible mutation, its validation must be added here before that
/// command may be committed by `apply_commands_audited`.
fn preflight_audited_commands<const D: usize>(
    world: &PhysicsWorld<D>,
    commands: &[WorldCommand<D>],
    ledger: &EnergyTransferLedger,
) -> Result<(), ApplyCommandError> {
    let mut staged_thermal = BTreeMap::<BodyHandle, ThermalBody>::new();
    let mut staged_ledger = ledger.clone();

    for command in commands {
        let body_handle = command_body(command);
        let Some(body) = world.body(body_handle) else {
            return Err(ApplyCommandError::MissingBody(body_handle));
        };

        let WorldCommand::ApplyExternalHeat {
            signed_joules,
            external_source_id,
            ..
        } = command
        else {
            continue;
        };

        if !staged_thermal.contains_key(&body_handle) {
            let thermal = body
                .thermal
                .ok_or(ExternalHeatError::MissingThermalState)?;
            staged_thermal.insert(body_handle, thermal);
        }

        let thermal = staged_thermal
            .get_mut(&body_handle)
            .expect("staged thermal state was inserted above");
        exchange_external_heat_thermal_audited(
            body_handle,
            thermal,
            *signed_joules,
            *external_source_id,
            &mut staged_ledger,
        )?;
    }

    Ok(())
}

/// Apply commands that do not cross an audited energy boundary.
///
/// The complete slice is preflighted before the first mutation. `ApplyExternalHeat`
/// is deliberately rejected here; callers must use `apply_commands_audited` so the
/// matching boundary transfer is recorded.
pub fn apply_commands<const D: usize>(
    world: &mut PhysicsWorld<D>,
    commands: &[WorldCommand<D>],
) -> Result<(), ApplyCommandError> {
    preflight_untracked_commands(world, commands)?;
    for command in commands {
        let applied = apply_non_boundary_command(world, command)?;
        debug_assert!(applied, "untracked preflight rejected all boundary commands");
    }
    Ok(())
}

/// Apply all replay commands while recording energy-boundary interventions.
///
/// The full batch is preflighted against copied thermal state and a cloned ledger
/// before any authoritative mutation. With the current command vocabulary, a
/// rejected batch therefore leaves both world and ledger unchanged.
pub fn apply_commands_audited<const D: usize>(
    world: &mut PhysicsWorld<D>,
    commands: &[WorldCommand<D>],
    ledger: &mut EnergyTransferLedger,
) -> Result<(), ApplyCommandError> {
    preflight_audited_commands(world, commands, ledger)?;

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
    fn external_heat_requires_audited_executor_without_partial_mutation() {
        let (mut world, handle) = thermal_world();
        let before = WorldSnapshot::capture(&world);
        let commands = [
            WorldCommand::SetLinearVelocity {
                body: handle,
                velocity: Box::new(SVector::from([1.0, 0.0, 0.0])),
            },
            WorldCommand::ApplyExternalHeat {
                body: handle,
                signed_joules: 1_000.0,
                external_source_id: 5,
            },
        ];

        assert_eq!(
            apply_commands(&mut world, &commands),
            Err(ApplyCommandError::EnergyLedgerRequired)
        );
        assert_eq!(WorldSnapshot::capture(&world), before);
    }

    #[test]
    fn audited_batch_rejects_late_missing_body_without_partial_commit() {
        let (mut world, handle) = thermal_world();
        let mut ledger = EnergyTransferLedger::new();
        apply_commands_audited(
            &mut world,
            &[WorldCommand::ApplyExternalHeat {
                body: handle,
                signed_joules: 500.0,
                external_source_id: 8,
            }],
            &mut ledger,
        )
        .unwrap();
        let before_world = WorldSnapshot::capture(&world);
        let before_ledger = ledger.clone();

        let commands = [
            WorldCommand::ApplyExternalHeat {
                body: handle,
                signed_joules: 1_000.0,
                external_source_id: 8,
            },
            WorldCommand::SetLinearVelocity {
                body: BodyHandle(usize::MAX),
                velocity: Box::new(SVector::from([1.0, 0.0, 0.0])),
            },
        ];

        assert_eq!(
            apply_commands_audited(&mut world, &commands, &mut ledger),
            Err(ApplyCommandError::MissingBody(BodyHandle(usize::MAX)))
        );
        assert_eq!(WorldSnapshot::capture(&world), before_world);
        assert_eq!(ledger, before_ledger);
    }

    #[test]
    fn audited_batch_stages_repeated_heat_before_commit() {
        let (mut world, handle) = thermal_world();
        let mut ledger = EnergyTransferLedger::new();
        let before_world = WorldSnapshot::capture(&world);
        let before_ledger = ledger.clone();

        let commands = [
            WorldCommand::ApplyExternalHeat {
                body: handle,
                signed_joules: 1_000.0,
                external_source_id: 8,
            },
            WorldCommand::ApplyExternalHeat {
                body: handle,
                signed_joules: -400_000.0,
                external_source_id: 8,
            },
        ];

        assert_eq!(
            apply_commands_audited(&mut world, &commands, &mut ledger),
            Err(ApplyCommandError::ExternalHeat(ExternalHeatError::Thermal(
                crate::thermal::ThermalError::InvalidTemperature
            )))
        );
        assert_eq!(WorldSnapshot::capture(&world), before_world);
        assert_eq!(ledger, before_ledger);
    }

    #[test]
    fn audited_external_heat_is_bitwise_replayable() {
        fn run() -> (WorldSnapshot<3>, EnergyTransferLedger) {
            let (mut world, handle) = thermal_world();
            let mut ledger = EnergyTransferLedger::new();
            let commands = [
                WorldCommand::ApplyExternalHeat {
                    body: handle,
                    signed_joules: 2_000.0,
                    external_source_id: 8,
                },
                WorldCommand::ApplyExternalHeat {
                    body: handle,
                    signed_joules: -500.0,
                    external_source_id: 9,
                },
            ];
            apply_commands_audited(&mut world, &commands, &mut ledger).unwrap();
            (WorldSnapshot::capture(&world), ledger)
        }

        let (snapshot_a, ledger_a) = run();
        let (snapshot_b, ledger_b) = run();
        assert_eq!(snapshot_a, snapshot_b);
        assert_eq!(ledger_a, ledger_b);
    }
}
