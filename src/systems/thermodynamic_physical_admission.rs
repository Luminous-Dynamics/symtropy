// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unified pre-consequence readiness for authoritative physical friction heat.
//!
//! This layer chooses no material constants and invents no unit scale. It only
//! proves that an explicit checked mechanical calibration has been supplied and
//! that every dynamic body eligible for the closed-pair friction theorem already
//! carries a valid thermal reservoir.

use symtropy_physics::{BodyHandle, MechanicalUnitCalibration, PhysicsWorld};

use super::thermodynamic_thermal_admission::{
    DynamicThermalAdmissionError, admit_dynamic_thermal_readiness,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum PhysicalFrictionReadinessError {
    MissingMechanicalUnitCalibration,
    Thermal(DynamicThermalAdmissionError),
}

impl From<DynamicThermalAdmissionError> for PhysicalFrictionReadinessError {
    fn from(value: DynamicThermalAdmissionError) -> Self {
        Self::Thermal(value)
    }
}

/// Non-cloneable proof that the current world is eligible to enter the physical
/// friction path for one consequential step.
///
/// The calibration is copied into the receipt so the exact admitted unit system
/// can be passed to the friction authority without consulting mutable global
/// configuration after the pre-consequence gate. Dynamic body identities are
/// canonical and sorted by `BodyHandle` by the thermal-readiness theorem.
///
/// The receipt is intentionally non-cloneable: the production solver authority
/// consumes it, so ordinary safe code cannot fan one pre-consequence admission
/// out into multiple independent physical-friction authorities.
#[derive(Debug, PartialEq)]
pub(crate) struct PhysicalFrictionReadiness {
    calibration: MechanicalUnitCalibration,
    dynamic_handles: Vec<BodyHandle>,
}

impl PhysicalFrictionReadiness {
    pub(crate) const fn calibration(&self) -> MechanicalUnitCalibration {
        self.calibration
    }

    pub(crate) fn dynamic_handles(&self) -> &[BodyHandle] {
        &self.dynamic_handles
    }
}

/// Admit one live 2D world for calibrated physical friction authority.
///
/// Missing calibration is rejected before the thermal census. This makes an
/// unconfigured unit system visible even for an otherwise empty world instead of
/// letting physical authority become conditionally configured only after bodies
/// happen to spawn.
pub(crate) fn admit_physical_friction_readiness(
    world: &PhysicsWorld<2>,
    calibration: Option<MechanicalUnitCalibration>,
) -> Result<PhysicalFrictionReadiness, PhysicalFrictionReadinessError> {
    let calibration =
        calibration.ok_or(PhysicalFrictionReadinessError::MissingMechanicalUnitCalibration)?;
    let thermal = admit_dynamic_thermal_readiness(world)?;

    Ok(PhysicalFrictionReadiness {
        calibration,
        dynamic_handles: thermal.handles().to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Point;
    use symtropy_physics::{ThermalBody, ThermalMaterial, ThermalState};

    fn calibration() -> MechanicalUnitCalibration {
        MechanicalUnitCalibration::new(1.0, 1.0 / 32.0, 1.0).unwrap()
    }

    fn thermal() -> ThermalBody {
        ThermalBody::new(
            ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
            ThermalState::new(300.0).unwrap(),
            1.0,
        )
        .unwrap()
    }

    #[test]
    fn missing_calibration_fails_even_for_empty_world() {
        let world = PhysicsWorld::<2>::default();
        assert_eq!(
            admit_physical_friction_readiness(&world, None),
            Err(PhysicalFrictionReadinessError::MissingMechanicalUnitCalibration)
        );
    }

    #[test]
    fn calibration_does_not_excuse_missing_dynamic_thermal_state() {
        let mut world = PhysicsWorld::<2>::default();
        let handle = world.add_sphere(Point::origin(), 1.0, 1.0);

        assert_eq!(
            admit_physical_friction_readiness(&world, Some(calibration())),
            Err(PhysicalFrictionReadinessError::Thermal(
                DynamicThermalAdmissionError::MissingThermalState { body: handle }
            ))
        );
    }

    #[test]
    fn valid_world_binds_exact_calibration_and_canonical_dynamic_identities() {
        let mut world = PhysicsWorld::<2>::default();
        let a = world.add_sphere(Point::origin(), 1.0, 1.0);
        let b = world.add_sphere(Point::new([4.0, 0.0]), 1.0, 1.0);
        world.body_mut(a).unwrap().set_thermal(thermal());
        world.body_mut(b).unwrap().set_thermal(thermal());
        world.bodies.swap(0, 1);

        let expected_calibration = calibration();
        let readiness =
            admit_physical_friction_readiness(&world, Some(expected_calibration)).unwrap();

        assert_eq!(readiness.calibration(), expected_calibration);
        assert_eq!(readiness.dynamic_handles(), &[a, b]);
    }
}
