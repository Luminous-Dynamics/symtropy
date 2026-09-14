// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Stable token-bound calibrated 2D friction terminal routing.
//!
//! Centered 2D loss/injection/neutral classification comes from the stable
//! applied-transaction theorem. Historical absolute-energy classification may
//! only be reused for diagnostic mutation after it is proven to agree with the
//! stable class. Non-promotable geometry retains the existing diagnostic path.

use nalgebra::SVector;
use symtropy_physics::{
    FrictionEvidenceRegime, FrictionMechanicalDelta, FrictionPromotionReceipt,
    FrictionSolverCoordinates, FrictionTransitionDelta2d, HeatPartition, MechanicalUnitCalibration,
    RigidBody, classify_applied_friction_transition_2d_checked,
    promote_applied_friction_loss_to_heat_calibrated_2d_stable,
    rollback_applied_friction_impulse,
};

use super::CalibratedRuntimeFrictionError;
use super::super::{
    TerminalFrictionOutcome, TerminalFrictionReceipt, ThermodynamicTransactionRuntime,
};

impl ThermodynamicTransactionRuntime {
    pub(crate) fn promote_friction_loss_owned_calibrated_2d_stable(
        &mut self,
        body_a: &mut RigidBody<2>,
        body_b: &mut RigidBody<2>,
        applied: &symtropy_physics::AppliedFrictionTransaction<2>,
        partition: HeatPartition,
        calibration: MechanicalUnitCalibration,
    ) -> Result<FrictionPromotionReceipt, CalibratedRuntimeFrictionError> {
        self.require_applied_tick(applied)
            .map_err(CalibratedRuntimeFrictionError::Runtime)?;
        promote_applied_friction_loss_to_heat_calibrated_2d_stable(
            body_a,
            body_b,
            applied,
            partition,
            calibration,
            &mut self.physical_ledger,
            &mut self.friction_journal,
        )
        .map_err(CalibratedRuntimeFrictionError::StablePromotion)
    }

    /// Atomically execute one 2D friction request and choose its terminal path
    /// from stable token-bound transition evidence when geometry is centered.
    ///
    /// Off-center/external-boundary geometry remains diagnostic and does not gain
    /// a stable-energy measurement veto. For centered geometry, stable loss goes
    /// to stable calibrated promotion. Stable injection/neutral may enter the
    /// historical diagnostic mutator only when its historical class already
    /// agrees; otherwise this method fails before terminalization and rolls the
    /// exact applied transition back.
    pub(crate) fn execute_terminal_friction_impulse_at_calibrated_2d_stable(
        &mut self,
        body_a: &mut RigidBody<2>,
        body_b: &mut RigidBody<2>,
        contact_point: &SVector<f64, 2>,
        impulse_on_b: &SVector<f64, 2>,
        coordinates: FrictionSolverCoordinates,
        partition: HeatPartition,
        calibration: MechanicalUnitCalibration,
    ) -> Result<TerminalFrictionReceipt, CalibratedRuntimeFrictionError> {
        let applied = self
            .apply_friction_impulse_at(
                body_a,
                body_b,
                contact_point,
                impulse_on_b,
                coordinates,
            )
            .map_err(CalibratedRuntimeFrictionError::Runtime)?;
        let transaction_id = applied.transaction_id();

        let terminalization = match applied.observation().regime {
            FrictionEvidenceRegime::OffCenterUnqualified
            | FrictionEvidenceRegime::ExternalBoundaryUnqualified => self
                .finalize_friction_diagnostic(body_a, body_b, &applied)
                .map(TerminalFrictionOutcome::Diagnostic)
                .map_err(CalibratedRuntimeFrictionError::Runtime),
            FrictionEvidenceRegime::CenteredClosedDynamicPair => {
                match classify_applied_friction_transition_2d_checked(body_a, body_b, &applied) {
                    Err(error) => Err(CalibratedRuntimeFrictionError::StableTransition(error)),
                    Ok(stable) => match stable.delta {
                        FrictionTransitionDelta2d::DissipationCandidate { .. } => self
                            .promote_friction_loss_owned_calibrated_2d_stable(
                                body_a,
                                body_b,
                                &applied,
                                partition,
                                calibration,
                            )
                            .map(TerminalFrictionOutcome::Promoted),
                        FrictionTransitionDelta2d::SolverInjection { .. } => {
                            if !matches!(
                                applied.observation().delta,
                                FrictionMechanicalDelta::SolverInjection { .. }
                            ) {
                                Err(
                                    CalibratedRuntimeFrictionError::CenteredStableInjectionHistoricalMismatch,
                                )
                            } else {
                                self.finalize_friction_diagnostic(body_a, body_b, &applied)
                                    .map(TerminalFrictionOutcome::Diagnostic)
                                    .map_err(CalibratedRuntimeFrictionError::Runtime)
                            }
                        }
                        FrictionTransitionDelta2d::Neutral => {
                            if applied.observation().delta != FrictionMechanicalDelta::Neutral {
                                Err(
                                    CalibratedRuntimeFrictionError::CenteredStableNeutralHistoricalMismatch,
                                )
                            } else {
                                self.finalize_friction_diagnostic(body_a, body_b, &applied)
                                    .map(TerminalFrictionOutcome::Diagnostic)
                                    .map_err(CalibratedRuntimeFrictionError::Runtime)
                            }
                        }
                    },
                }
            }
        };

        match terminalization {
            Ok(outcome) => Ok(TerminalFrictionReceipt {
                transaction_id,
                outcome,
            }),
            Err(terminalization) => match rollback_applied_friction_impulse(
                body_a,
                body_b,
                applied,
                &mut self.friction_journal,
            ) {
                Ok(()) => Err(terminalization),
                Err(rollback) => Err(CalibratedRuntimeFrictionError::Rollback {
                    terminalization: Box::new(terminalization),
                    rollback,
                }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::{Point, Sphere, Transform};
    use symtropy_physics::{
        BodyHandle, BodyType, FrictionDiagnosticReason, FrictionTransactionId,
        FrictionTransactionPhase, StableCalibratedFrictionPromotion2dError, ThermalBody,
        ThermalMaterial, ThermalState,
    };

    fn thermal() -> ThermalBody {
        ThermalBody::new(
            ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
            ThermalState::new(300.0).unwrap(),
            1.0,
        )
        .unwrap()
    }

    fn anisotropic_body(handle: usize, velocity_x: f64, angular_velocity: f64) -> RigidBody<2> {
        let mut body = RigidBody::new(
            BodyHandle(handle),
            BodyType::Dynamic,
            Transform::from_translation(Point::origin()),
            Box::new(Sphere::<2>::unit()),
            1.0,
            SVector::from([2.0, 8.0]),
        );
        body.linear_velocity[0] = velocity_x;
        body.angular_velocity.set(0, 1, angular_velocity);
        body.set_thermal(thermal());
        body
    }

    #[test]
    fn stable_loss_promotes_even_when_historical_observation_is_neutral() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = anisotropic_body(1, 1.0, 100_000_000.0);
        let mut b = anisotropic_body(2, 0.0, 0.0);

        let terminal = runtime
            .execute_terminal_friction_impulse_at_calibrated_2d_stable(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::from([0.5, 0.0]),
                FrictionSolverCoordinates::new(2, 3, 4),
                HeatPartition::equal(),
                MechanicalUnitCalibration::new(1.0, 1.0, 1.0).unwrap(),
            )
            .unwrap();

        let receipt = match terminal.outcome {
            TerminalFrictionOutcome::Promoted(receipt) => receipt,
            TerminalFrictionOutcome::Diagnostic(reason) => {
                panic!("stable loss incorrectly terminalized as diagnostic {reason:?}")
            }
        };
        assert_eq!(receipt.heat.dissipated_joules, 0.25);
        assert_eq!(receipt.heat.kinetic_change_a_joules, -0.375);
        assert_eq!(receipt.heat.kinetic_change_b_joules, 0.125);
        assert_eq!(
            runtime.friction_journal().phase(FrictionTransactionId::new(0, 2, 3, 4)),
            Some(FrictionTransactionPhase::Promoted)
        );
        assert_eq!(runtime.physical_energy_ledger().len(), 3);
    }

    #[test]
    fn stable_injection_that_historical_rounds_to_neutral_rolls_back() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = anisotropic_body(1, 0.0, 100_000_000.0);
        let mut b = anisotropic_body(2, 0.0, 0.0);
        let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
        let before_b = (b.linear_velocity, b.angular_velocity, b.thermal);

        let error = runtime
            .execute_terminal_friction_impulse_at_calibrated_2d_stable(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::from([0.5, 0.0]),
                FrictionSolverCoordinates::new(0, 0, 0),
                HeatPartition::equal(),
                MechanicalUnitCalibration::new(1.0, 1.0, 1.0).unwrap(),
            )
            .unwrap_err();

        assert!(matches!(
            error,
            CalibratedRuntimeFrictionError::CenteredStableInjectionHistoricalMismatch
        ));
        assert_eq!((a.linear_velocity, a.angular_velocity, a.thermal), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity, b.thermal), before_b);
        assert!(runtime.friction_journal().is_empty());
        assert!(runtime.physical_energy_ledger().is_empty());
    }

    #[test]
    fn normal_scale_centered_injection_remains_diagnostic() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = anisotropic_body(1, 0.0, 0.0);
        let mut b = anisotropic_body(2, 0.0, 0.0);

        let terminal = runtime
            .execute_terminal_friction_impulse_at_calibrated_2d_stable(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::from([0.5, 0.0]),
                FrictionSolverCoordinates::new(0, 1, 0),
                HeatPartition::equal(),
                MechanicalUnitCalibration::new(1.0, 1.0, 1.0).unwrap(),
            )
            .unwrap();

        assert_eq!(
            terminal.outcome,
            TerminalFrictionOutcome::Diagnostic(FrictionDiagnosticReason::SolverInjection)
        );
        assert!(runtime.physical_energy_ledger().is_empty());
    }

    #[test]
    fn zero_impulse_remains_neutral_diagnostic() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = anisotropic_body(1, 1.0, 2.0);
        let mut b = anisotropic_body(2, 0.0, -1.0);

        let terminal = runtime
            .execute_terminal_friction_impulse_at_calibrated_2d_stable(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::zeros(),
                FrictionSolverCoordinates::new(0, 2, 0),
                HeatPartition::equal(),
                MechanicalUnitCalibration::new(2.0, 3.0, 0.5).unwrap(),
            )
            .unwrap();

        assert_eq!(
            terminal.outcome,
            TerminalFrictionOutcome::Diagnostic(FrictionDiagnosticReason::Neutral)
        );
        assert!(runtime.physical_energy_ledger().is_empty());
    }

    #[test]
    fn missing_thermal_stable_loss_rolls_back_exact_mechanics() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = anisotropic_body(1, 1.0, 0.0);
        let mut b = anisotropic_body(2, 0.0, 0.0);
        b.clear_thermal();
        let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
        let before_b = (b.linear_velocity, b.angular_velocity, b.thermal);

        let error = runtime
            .execute_terminal_friction_impulse_at_calibrated_2d_stable(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::from([0.5, 0.0]),
                FrictionSolverCoordinates::new(0, 3, 0),
                HeatPartition::equal(),
                MechanicalUnitCalibration::new(1.0, 1.0, 1.0).unwrap(),
            )
            .unwrap_err();

        assert!(matches!(
            error,
            CalibratedRuntimeFrictionError::StablePromotion(
                StableCalibratedFrictionPromotion2dError::MissingThermalState
            )
        ));
        assert_eq!((a.linear_velocity, a.angular_velocity, a.thermal), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity, b.thermal), before_b);
        assert!(runtime.friction_journal().is_empty());
        assert!(runtime.physical_energy_ledger().is_empty());
    }
}
