// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Canonical bridge from embodied core thermal state into integration-field clarity.
//!
//! Physical temperature authority lives on `symtropy_physics::RigidBody::thermal`.
//! The legacy `EnergyBudget.temperature` field is retained only as migration/shadow
//! evidence and never selected as the temperature source by this module.

use symthaea_consciousness_equation::ConsciousnessInputs;
use symtropy_math::Point;
use symtropy_physics::{BodyHandle, PhysicsWorld, ThermalError};

use crate::coupling::ConsciousnessField;
use crate::safety::SafetyTier;
use crate::thermodynamics::smooth_temperature_penalty;

/// Shadow-state differences at or below this tolerance are considered numerically aligned.
pub const LEGACY_THERMAL_SHADOW_TOLERANCE_K: f64 = 1.0e-9;

/// Diagnostic relationship between the authoritative core temperature and the
/// compatibility `EnergyBudget.temperature` shadow.
///
/// This status is evidence only. It never changes which temperature is used for
/// clarity calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LegacyThermalShadowStatus {
    Consistent {
        legacy_temperature_kelvin: f64,
        absolute_delta_kelvin: f64,
    },
    Diverged {
        legacy_temperature_kelvin: f64,
        absolute_delta_kelvin: f64,
    },
    InvalidLegacy {
        legacy_temperature_kelvin: f64,
    },
    UnrepresentableDelta {
        legacy_temperature_kelvin: f64,
    },
}

/// Receipt proving which physical temperature was selected for one entity update.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermalAuthorityReceipt {
    pub body: BodyHandle,
    pub authoritative_temperature_kelvin: f64,
    pub legacy_shadow: LegacyThermalShadowStatus,
}

/// Fail-closed errors when physical thermal authority cannot be established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalAuthorityError {
    UnregisteredEntity,
    MissingBody,
    MissingThermalState,
    InvalidCoreThermalState(ThermalError),
}

/// Update one registered integration-field entity using exactly one validated
/// snapshot of its embodied core `ThermalBody` temperature.
///
/// The core thermal state is read once, validated, and then used for the entire
/// clarity update. The legacy `EnergyBudget.temperature` value is only compared
/// after selection to produce migration evidence; it cannot override or improve
/// authority.
///
/// Missing/invalid core thermal evidence fails closed by forcing the registered
/// entity to Red safety and zero motor precision. No favorable default temperature
/// is substituted.
pub fn update_entity_with_core_thermal_authority<const D: usize>(
    field: &mut ConsciousnessField<D>,
    world: &PhysicsWorld<D>,
    handle: BodyHandle,
    inputs: &ConsciousnessInputs,
    position: Point<D>,
) -> Result<ThermalAuthorityReceipt, ThermalAuthorityError> {
    if !field.entities.contains_key(&handle) {
        return Err(ThermalAuthorityError::UnregisteredEntity);
    }

    let authoritative_temperature_kelvin = match read_core_temperature_once(world, handle) {
        Ok(temperature) => temperature,
        Err(error) => {
            fail_closed_entity(field, handle);
            return Err(error);
        }
    };

    let legacy_temperature_kelvin = field
        .entities
        .get(&handle)
        .expect("registered entity checked above")
        .energy
        .temperature;
    let legacy_shadow = classify_legacy_shadow(
        authoritative_temperature_kelvin,
        legacy_temperature_kelvin,
    );

    let temp_penalty = smooth_temperature_penalty(authoritative_temperature_kelvin);
    let effective_inputs = ConsciousnessInputs {
        phi: inputs.phi * temp_penalty,
        broadcast: inputs.broadcast * temp_penalty,
        working_memory: inputs.working_memory * temp_penalty,
        attention: inputs.attention * temp_penalty,
        recurrence: inputs.recurrence,
        embodiment: inputs.embodiment,
        knowledge: inputs.knowledge,
        synchrony: inputs.synchrony * temp_penalty,
    };

    let entity = field
        .entities
        .get_mut(&handle)
        .expect("registered entity checked above");
    let phi_before = entity.phi();
    entity.compute(&effective_inputs);
    let phi_after = entity.phi();
    let delta_phi = (phi_after - phi_before).abs();
    if delta_phi > 1.0e-10 {
        field.ledger.record_phi_change(delta_phi);
    }

    if let Some(sanctuary) = field.sanctuaries.get_mut(&handle) {
        let conditions = entity.sanctuary_conditions();
        sanctuary.update(&conditions, position);
    }

    field.collective_phi = if field.entities.is_empty() {
        0.0
    } else {
        field.entities.values().map(|entity| entity.phi()).sum::<f64>()
            / field.entities.len() as f64
    };

    Ok(ThermalAuthorityReceipt {
        body: handle,
        authoritative_temperature_kelvin,
        legacy_shadow,
    })
}

fn read_core_temperature_once<const D: usize>(
    world: &PhysicsWorld<D>,
    handle: BodyHandle,
) -> Result<f64, ThermalAuthorityError> {
    let body = world
        .body(handle)
        .ok_or(ThermalAuthorityError::MissingBody)?;
    let thermal = body
        .thermal
        .ok_or(ThermalAuthorityError::MissingThermalState)?;
    thermal
        .validate()
        .map_err(ThermalAuthorityError::InvalidCoreThermalState)?;
    Ok(thermal.state.temperature_kelvin)
}

fn fail_closed_entity<const D: usize>(field: &mut ConsciousnessField<D>, handle: BodyHandle) {
    if let Some(entity) = field.entities.get_mut(&handle) {
        entity.safety_tier = SafetyTier::Red;
        entity.motor_precision = 0.0;
    }
}

fn classify_legacy_shadow(
    authoritative_temperature_kelvin: f64,
    legacy_temperature_kelvin: f64,
) -> LegacyThermalShadowStatus {
    if !legacy_temperature_kelvin.is_finite() || legacy_temperature_kelvin <= 0.0 {
        return LegacyThermalShadowStatus::InvalidLegacy {
            legacy_temperature_kelvin,
        };
    }

    let delta = (legacy_temperature_kelvin - authoritative_temperature_kelvin).abs();
    if !delta.is_finite() {
        LegacyThermalShadowStatus::UnrepresentableDelta {
            legacy_temperature_kelvin,
        }
    } else if delta <= LEGACY_THERMAL_SHADOW_TOLERANCE_K {
        LegacyThermalShadowStatus::Consistent {
            legacy_temperature_kelvin,
            absolute_delta_kelvin: delta,
        }
    } else {
        LegacyThermalShadowStatus::Diverged {
            legacy_temperature_kelvin,
            absolute_delta_kelvin: delta,
        }
    }
}

#[cfg(test)]
mod tests {
    use nalgebra::SVector;
    use symtropy_physics::{ThermalBody, ThermalMaterial, ThermalState};

    use super::*;

    fn test_inputs() -> ConsciousnessInputs {
        ConsciousnessInputs {
            phi: 0.9,
            broadcast: 0.8,
            working_memory: 0.7,
            attention: 0.6,
            recurrence: 0.5,
            embodiment: 0.7,
            knowledge: 0.6,
            synchrony: 0.8,
        }
    }

    fn world_with_temperature(
        temperature_kelvin: f64,
    ) -> (PhysicsWorld<2>, BodyHandle) {
        let mut world = PhysicsWorld::<2>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 1.0, 1.0);
        let thermal = ThermalBody::new(
            ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
            ThermalState::new(temperature_kelvin).unwrap(),
            1.0,
        )
        .unwrap();
        world.body_mut(handle).unwrap().set_thermal(thermal);
        (world, handle)
    }

    fn registered_field(handle: BodyHandle) -> ConsciousnessField<2> {
        let mut field = ConsciousnessField::<2>::new();
        field.register(handle, 100.0, 10.0);
        field
    }

    #[test]
    fn core_temperature_wins_over_divergent_legacy_shadow() {
        let (world, handle) = world_with_temperature(310.0);
        let mut divergent = registered_field(handle);
        let mut aligned = registered_field(handle);
        divergent
            .entities
            .get_mut(&handle)
            .unwrap()
            .energy
            .temperature = 1_000.0;
        aligned
            .entities
            .get_mut(&handle)
            .unwrap()
            .energy
            .temperature = 310.0;

        let divergent_receipt = update_entity_with_core_thermal_authority(
            &mut divergent,
            &world,
            handle,
            &test_inputs(),
            Point::origin(),
        )
        .unwrap();
        let aligned_receipt = update_entity_with_core_thermal_authority(
            &mut aligned,
            &world,
            handle,
            &test_inputs(),
            Point::origin(),
        )
        .unwrap();

        assert_eq!(divergent_receipt.authoritative_temperature_kelvin, 310.0);
        assert!(matches!(
            divergent_receipt.legacy_shadow,
            LegacyThermalShadowStatus::Diverged { .. }
        ));
        assert!(matches!(
            aligned_receipt.legacy_shadow,
            LegacyThermalShadowStatus::Consistent { .. }
        ));
        assert_eq!(divergent.phi(handle), aligned.phi(handle));
        assert_eq!(
            divergent.entities.get(&handle).unwrap().energy.temperature,
            1_000.0
        );
    }

    #[test]
    fn missing_core_thermal_state_fails_authority_closed() {
        let mut world = PhysicsWorld::<2>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 1.0, 1.0);
        let mut field = registered_field(handle);

        let result = update_entity_with_core_thermal_authority(
            &mut field,
            &world,
            handle,
            &test_inputs(),
            Point::origin(),
        );

        assert_eq!(result, Err(ThermalAuthorityError::MissingThermalState));
        let entity = field.entities.get(&handle).unwrap();
        assert_eq!(entity.safety_tier, SafetyTier::Red);
        assert_eq!(entity.motor_precision, 0.0);
    }

    #[test]
    fn missing_body_fails_registered_entity_closed() {
        let world = PhysicsWorld::<2>::new(SVector::zeros());
        let handle = BodyHandle(42);
        let mut field = registered_field(handle);

        let result = update_entity_with_core_thermal_authority(
            &mut field,
            &world,
            handle,
            &test_inputs(),
            Point::origin(),
        );

        assert_eq!(result, Err(ThermalAuthorityError::MissingBody));
        let entity = field.entities.get(&handle).unwrap();
        assert_eq!(entity.safety_tier, SafetyTier::Red);
        assert_eq!(entity.motor_precision, 0.0);
    }

    #[test]
    fn invalid_core_temperature_fails_authority_closed() {
        let (mut world, handle) = world_with_temperature(310.0);
        world
            .body_mut(handle)
            .unwrap()
            .thermal
            .as_mut()
            .unwrap()
            .state
            .temperature_kelvin = f64::NAN;
        let mut field = registered_field(handle);

        let result = update_entity_with_core_thermal_authority(
            &mut field,
            &world,
            handle,
            &test_inputs(),
            Point::origin(),
        );

        assert_eq!(
            result,
            Err(ThermalAuthorityError::InvalidCoreThermalState(
                ThermalError::InvalidTemperature
            ))
        );
        let entity = field.entities.get(&handle).unwrap();
        assert_eq!(entity.safety_tier, SafetyTier::Red);
        assert_eq!(entity.motor_precision, 0.0);
    }

    #[test]
    fn invalid_legacy_shadow_is_detected_but_cannot_override_core() {
        let (world, handle) = world_with_temperature(310.0);
        let mut field = registered_field(handle);
        field
            .entities
            .get_mut(&handle)
            .unwrap()
            .energy
            .temperature = f64::NAN;

        let receipt = update_entity_with_core_thermal_authority(
            &mut field,
            &world,
            handle,
            &test_inputs(),
            Point::origin(),
        )
        .unwrap();

        assert_eq!(receipt.authoritative_temperature_kelvin, 310.0);
        assert!(matches!(
            receipt.legacy_shadow,
            LegacyThermalShadowStatus::InvalidLegacy { .. }
        ));
        assert!(field.phi(handle).is_finite());
    }

    #[test]
    fn unregistered_entity_is_explicit_error() {
        let (world, handle) = world_with_temperature(310.0);
        let mut field = ConsciousnessField::<2>::new();
        assert_eq!(
            update_entity_with_core_thermal_authority(
                &mut field,
                &world,
                handle,
                &test_inputs(),
                Point::origin(),
            ),
            Err(ThermalAuthorityError::UnregisteredEntity)
        );
    }
}
