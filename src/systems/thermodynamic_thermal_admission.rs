// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pre-consequence thermal-readiness admission for the authoritative 2D physics path.
//!
//! This module does not choose materials or synthesize missing thermal state. It
//! only proves that every dynamic rigid body which may participate in a promoted
//! dynamic/dynamic friction contact already carries a valid core `ThermalBody`.
//! Static and kinematic bodies remain outside this closed-pair admission because
//! #809 still treats those contacts as external-boundary regimes rather than
//! silently assigning their work/heat to the dynamic participant.

use symtropy_physics::{BodyHandle, PhysicsWorld, ThermalError};

/// Why the current dynamic-body set is not ready for physical friction authority.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum DynamicThermalAdmissionError {
    /// Stable identity itself is ambiguous; no authority may be issued.
    DuplicateBodyHandle(BodyHandle),
    /// A dynamic body has no explicit physical thermal reservoir.
    MissingThermalState { body: BodyHandle },
    /// A present reservoir failed the core thermodynamic invariants.
    InvalidThermalState {
        body: BodyHandle,
        error: ThermalError,
    },
}

/// Deterministic receipt proving which dynamic body identities passed readiness.
///
/// The receipt intentionally contains identities only. It does not freeze
/// temperature or material values: friction promotion revalidates the exact live
/// participant reservoirs transactionally when heat is actually authored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DynamicThermalAdmission {
    handles: Vec<BodyHandle>,
}

impl DynamicThermalAdmission {
    pub(crate) fn handles(&self) -> &[BodyHandle] {
        &self.handles
    }
}

/// Admit the live dynamic-body set for the physical friction consequence path.
///
/// Validation order is canonical by `BodyHandle`, so both success receipts and
/// the identity of the first reported failure are independent of body-vector
/// storage order. Missing state is never defaulted here: the application layer
/// must explicitly choose and attach a thermal model before authority activation.
pub(crate) fn admit_dynamic_thermal_readiness(
    world: &PhysicsWorld<2>,
) -> Result<DynamicThermalAdmission, DynamicThermalAdmissionError> {
    let mut dynamic: Vec<_> = world.bodies.iter().filter(|body| body.is_dynamic()).collect();
    dynamic.sort_unstable_by_key(|body| body.handle);

    if let Some(pair) = dynamic
        .windows(2)
        .find(|pair| pair[0].handle == pair[1].handle)
    {
        return Err(DynamicThermalAdmissionError::DuplicateBodyHandle(
            pair[0].handle,
        ));
    }

    let mut handles = Vec::with_capacity(dynamic.len());
    for body in dynamic {
        let thermal = body
            .thermal
            .ok_or(DynamicThermalAdmissionError::MissingThermalState {
                body: body.handle,
            })?;
        thermal
            .validate()
            .map_err(|error| DynamicThermalAdmissionError::InvalidThermalState {
                body: body.handle,
                error,
            })?;
        handles.push(body.handle);
    }

    Ok(DynamicThermalAdmission { handles })
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::{Point, Sphere};
    use symtropy_physics::{RigidBody, ThermalBody, ThermalMaterial, ThermalState};

    fn valid_thermal() -> ThermalBody {
        ThermalBody::new(
            ThermalMaterial::new(1000.0, 1.0, 0.5).unwrap(),
            ThermalState::new(300.0).unwrap(),
            1.0,
        )
        .unwrap()
    }

    #[test]
    fn empty_world_is_ready_with_empty_dynamic_census() {
        let world = PhysicsWorld::<2>::default();
        let admission = admit_dynamic_thermal_readiness(&world).unwrap();
        assert!(admission.handles().is_empty());
    }

    #[test]
    fn static_body_without_thermal_is_outside_dynamic_admission() {
        let mut world = PhysicsWorld::<2>::default();
        world.add_body(RigidBody::static_body(
            BodyHandle(99),
            Point::origin(),
            Box::new(Sphere::unit()),
        ));

        let admission = admit_dynamic_thermal_readiness(&world).unwrap();
        assert!(admission.handles().is_empty());
    }

    #[test]
    fn dynamic_body_without_thermal_fails_closed_before_consequence() {
        let mut world = PhysicsWorld::<2>::default();
        let handle = world.add_sphere(Point::origin(), 1.0, 1.0);

        assert_eq!(
            admit_dynamic_thermal_readiness(&world),
            Err(DynamicThermalAdmissionError::MissingThermalState { body: handle })
        );
    }

    #[test]
    fn invalid_public_thermal_mutation_is_revalidated() {
        let mut world = PhysicsWorld::<2>::default();
        let handle = world.add_sphere(Point::origin(), 1.0, 1.0);
        world.body_mut(handle).unwrap().set_thermal(valid_thermal());
        world
            .body_mut(handle)
            .unwrap()
            .thermal
            .as_mut()
            .unwrap()
            .state
            .temperature_kelvin = -1.0;

        assert_eq!(
            admit_dynamic_thermal_readiness(&world),
            Err(DynamicThermalAdmissionError::InvalidThermalState {
                body: handle,
                error: ThermalError::InvalidTemperature,
            })
        );
    }

    #[test]
    fn valid_dynamic_bodies_produce_canonical_handle_receipt() {
        let mut world = PhysicsWorld::<2>::default();
        let a = world.add_sphere(Point::new([0.0, 0.0]), 1.0, 1.0);
        let b = world.add_sphere(Point::new([4.0, 0.0]), 1.0, 1.0);
        world.body_mut(a).unwrap().set_thermal(valid_thermal());
        world.body_mut(b).unwrap().set_thermal(valid_thermal());

        // The readiness theorem must not inherit body-vector storage order.
        world.bodies.swap(0, 1);
        let admission = admit_dynamic_thermal_readiness(&world).unwrap();
        assert_eq!(admission.handles(), &[a, b]);
    }

    #[test]
    fn duplicate_dynamic_identity_is_rejected_before_reservoir_validation() {
        let mut world = PhysicsWorld::<2>::default();
        let a = world.add_sphere(Point::origin(), 1.0, 1.0);
        let b = world.add_sphere(Point::new([4.0, 0.0]), 1.0, 1.0);
        world.body_mut(a).unwrap().set_thermal(valid_thermal());
        world.body_mut(b).unwrap().set_thermal(valid_thermal());

        // Adversarial corruption of the public body vector: readiness must not
        // issue authority over two physical records carrying one stable identity.
        world.bodies[1].handle = a;
        assert_eq!(
            admit_dynamic_thermal_readiness(&world),
            Err(DynamicThermalAdmissionError::DuplicateBodyHandle(a))
        );
    }
}
