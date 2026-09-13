// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Launcher-local 2D friction execution bridge.
//!
//! This adapter composes the private fixed-tick reservation authority with the
//! checked solver-consistent 2D pair-energy measurement path. It deliberately
//! does not decide heat promotion or diagnostic terminalization: the returned
//! `AppliedFrictionTransaction<2>` remains the continuation authority for those
//! later policy decisions.

use nalgebra::SVector;
use symtropy_physics::{
    AppliedFrictionTransaction, FrictionPairEnergy2dError, FrictionPairEnergyChange2d,
    FrictionSolverCoordinates, RigidBody, capture_friction_pair_energy_2d_checked,
    classify_friction_pair_energy_change_2d_checked,
};

use super::thermodynamic_runtime::{RuntimeFrictionError, ThermodynamicTransactionRuntime};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CheckedFrictionEnergy2d {
    Qualified(FrictionPairEnergyChange2d),
    Unavailable(FrictionPairEnergy2dError),
}

/// One successfully applied runtime friction transaction plus independent checked
/// 2D mechanical-energy evidence when that measurement was available.
///
/// The applied token is intentionally non-cloneable through its core type. A
/// caller must still terminalize it through physical promotion or the admissible
/// diagnostic path before the enclosing fixed tick may close.
#[derive(Debug, PartialEq)]
pub struct RuntimeFrictionExecution2d {
    pub applied: AppliedFrictionTransaction<2>,
    pub checked_energy: CheckedFrictionEnergy2d,
}

/// Reserve and apply one exact 2D friction impulse under the open fixed tick,
/// while measuring solver-consistent A/B pair energy independently.
///
/// Checked measurement is observational: if pre-measurement is unavailable but
/// the runtime/core mechanics are otherwise valid, the impulse still executes and
/// the result explicitly carries `CheckedFrictionEnergy2d::Unavailable`. This
/// prevents a measurement limitation from silently becoming a second mechanics
/// authority.
pub fn execute_runtime_friction_impulse_2d(
    runtime: &mut ThermodynamicTransactionRuntime,
    body_a: &mut RigidBody<2>,
    body_b: &mut RigidBody<2>,
    contact_point: &SVector<f64, 2>,
    impulse_on_b: &SVector<f64, 2>,
    coordinates: FrictionSolverCoordinates,
) -> Result<RuntimeFrictionExecution2d, RuntimeFrictionError> {
    let before = capture_friction_pair_energy_2d_checked(body_a, body_b);

    let reservation = runtime.reserve_friction_impulse_at(
        body_a.handle,
        body_b.handle,
        contact_point,
        impulse_on_b,
        coordinates,
    )?;
    let applied = runtime.apply_reserved_friction_impulse(body_a, body_b, reservation)?;

    let checked_energy = match before {
        Err(error) => CheckedFrictionEnergy2d::Unavailable(error),
        Ok(before) => match capture_friction_pair_energy_2d_checked(body_a, body_b) {
            Err(error) => CheckedFrictionEnergy2d::Unavailable(error),
            Ok(after) => match classify_friction_pair_energy_change_2d_checked(before, after) {
                Ok(change) => CheckedFrictionEnergy2d::Qualified(change),
                Err(error) => CheckedFrictionEnergy2d::Unavailable(error),
            },
        },
    };

    Ok(RuntimeFrictionExecution2d {
        applied,
        checked_energy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::{Point, Sphere, Transform};
    use symtropy_physics::{
        BodyHandle, BodyType, FrictionDiagnosticReason, FrictionPairEnergyDelta2d,
        RigidBodyEnergy2dError,
    };

    fn sphere(handle: usize, velocity_x: f64) -> RigidBody<2> {
        let mut body = RigidBody::<2>::dynamic_sphere(
            BodyHandle(handle),
            Point::origin(),
            0.5,
            1.0,
        );
        body.linear_velocity[0] = velocity_x;
        body
    }

    fn anisotropic(handle: usize, position: [f64; 2]) -> RigidBody<2> {
        RigidBody::new(
            BodyHandle(handle),
            BodyType::Dynamic,
            Transform::from_translation(Point::new(position)),
            Box::new(Sphere::<2>::unit()),
            2.0,
            SVector::from([2.0, 8.0]),
        )
    }

    #[test]
    fn centered_runtime_application_returns_checked_loss_and_pending_token() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = sphere(1, 1.0);
        let mut b = sphere(2, 0.0);

        let execution = execute_runtime_friction_impulse_2d(
            &mut runtime,
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0]),
            FrictionSolverCoordinates::new(0, 0, 0),
        )
        .unwrap();

        assert_eq!(execution.applied.transaction_id().fixed_tick, 0);
        match execution.checked_energy {
            CheckedFrictionEnergy2d::Qualified(change) => {
                assert_eq!(
                    change.delta,
                    FrictionPairEnergyDelta2d::DissipationCandidate { joules: 0.25 }
                );
                assert_eq!(change.pair_delta_joules, -0.25);
            }
            CheckedFrictionEnergy2d::Unavailable(error) => {
                panic!("checked centered evidence unexpectedly unavailable: {error:?}")
            }
        }
        assert_eq!(runtime.friction_journal().pending_application_count(), 1);
    }

    #[test]
    fn measurement_unavailability_does_not_suppress_valid_runtime_mechanics() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = sphere(1, 1.0);
        let mut b = sphere(2, 0.0);
        a.inv_mass = 0.5; // intentionally inconsistent with stored mass = 1.0
        let a_before = a.linear_velocity;
        let b_before = b.linear_velocity;

        let execution = execute_runtime_friction_impulse_2d(
            &mut runtime,
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.25, 0.0]),
            FrictionSolverCoordinates::new(0, 0, 0),
        )
        .unwrap();

        assert_eq!(
            execution.checked_energy,
            CheckedFrictionEnergy2d::Unavailable(
                FrictionPairEnergy2dError::BodyA(
                    RigidBodyEnergy2dError::InconsistentInverseMass
                )
            )
        );
        assert_ne!(a.linear_velocity, a_before);
        assert_ne!(b.linear_velocity, b_before);
        assert_eq!(runtime.friction_journal().pending_application_count(), 1);
    }

    #[test]
    fn off_center_checked_delta_is_independent_of_legacy_generic_energy() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = anisotropic(1, [-1.0, 0.0]);
        let mut b = anisotropic(2, [1.0, 0.0]);

        let execution = execute_runtime_friction_impulse_2d(
            &mut runtime,
            &mut a,
            &mut b,
            &SVector::from([0.0, 0.5]),
            &SVector::from([0.0, -0.1]),
            FrictionSolverCoordinates::new(0, 1, 0),
        )
        .unwrap();

        let checked_delta = match execution.checked_energy {
            CheckedFrictionEnergy2d::Qualified(change) => change.pair_delta_joules,
            CheckedFrictionEnergy2d::Unavailable(error) => {
                panic!("checked off-center evidence unexpectedly unavailable: {error:?}")
            }
        };
        let legacy_delta = execution.applied.observation().pair_delta_joules;
        assert!((checked_delta - legacy_delta).abs() > 1.0e-6);

        assert_eq!(
            runtime
                .finalize_friction_diagnostic(&a, &b, &execution.applied)
                .unwrap(),
            FrictionDiagnosticReason::OffCenterUnqualified
        );
        assert!(runtime.friction_journal().is_complete_for_finalize());
    }
}
