// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Calibrated friction terminalization inside the fixed-tick runtime.
//!
//! This child module deliberately reuses the parent runtime's private journal and
//! physical ledger. It adds no second fixed-tick authority and exposes no mutable
//! accounting state. The existing SI-assumed runtime path remains unchanged as a
//! compatibility/reference lineage.

use nalgebra::SVector;
use symtropy_physics::{
    AppliedFrictionTransaction, CalibratedFrictionPromotionError,
    FrictionApplicationRollbackError, FrictionPromotionReceipt, FrictionSolverCoordinates,
    HeatPartition, MechanicalUnitCalibration, RigidBody,
    promote_applied_friction_loss_to_heat_calibrated, rollback_applied_friction_impulse,
};

use super::{
    RuntimeFrictionError, TerminalFrictionOutcome, TerminalFrictionReceipt,
    ThermodynamicTransactionRuntime,
};

/// Typed failure surface for calibrated terminal friction execution.
///
/// Existing runtime failures remain distinguishable from explicit calibrated
/// promotion failures. If terminalization rejects and exact mechanical rollback
/// also refuses, both causes are retained.
#[derive(Debug)]
pub enum CalibratedRuntimeFrictionError {
    Runtime(RuntimeFrictionError),
    Promotion(CalibratedFrictionPromotionError),
    Rollback {
        terminalization: Box<CalibratedRuntimeFrictionError>,
        rollback: FrictionApplicationRollbackError,
    },
}

impl From<RuntimeFrictionError> for CalibratedRuntimeFrictionError {
    fn from(value: RuntimeFrictionError) -> Self {
        Self::Runtime(value)
    }
}

impl From<CalibratedFrictionPromotionError> for CalibratedRuntimeFrictionError {
    fn from(value: CalibratedFrictionPromotionError) -> Self {
        Self::Promotion(value)
    }
}

impl ThermodynamicTransactionRuntime {
    /// Promote one already-applied friction transaction through an explicit
    /// solver-mechanical-unit -> SI Joule calibration while retaining private
    /// ownership of the canonical physical ledger and lifecycle journal.
    pub fn promote_friction_loss_owned_calibrated<const D: usize>(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        applied: &AppliedFrictionTransaction<D>,
        partition: HeatPartition,
        calibration: MechanicalUnitCalibration,
    ) -> Result<FrictionPromotionReceipt, CalibratedRuntimeFrictionError> {
        self.require_applied_tick(applied)
            .map_err(CalibratedRuntimeFrictionError::Runtime)?;
        promote_applied_friction_loss_to_heat_calibrated(
            body_a,
            body_b,
            applied,
            partition,
            calibration,
            &mut self.physical_ledger,
            &mut self.friction_journal,
        )
        .map_err(CalibratedRuntimeFrictionError::Promotion)
    }

    /// Atomically execute one friction request through a calibrated terminal
    /// lifecycle state before later solver mechanics may proceed.
    ///
    /// The mechanical transition is applied exactly once. Centered measured loss
    /// is promoted through the admitted unit calibration; diagnostic-only regimes
    /// retain the existing runtime diagnostic theorem. Any terminalization error
    /// immediately consumes the unique applied token to restore the exact pre-step
    /// velocities and remove only this transaction's still-`Applied` journal entry.
    pub fn execute_terminal_friction_impulse_at_calibrated<const D: usize>(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        contact_point: &SVector<f64, D>,
        impulse_on_b: &SVector<f64, D>,
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

        let terminalization = if applied
            .observation()
            .centered_promotable_loss_candidate_joules()
            .is_some()
        {
            self.promote_friction_loss_owned_calibrated(
                body_a,
                body_b,
                &applied,
                partition,
                calibration,
            )
            .map(TerminalFrictionOutcome::Promoted)
        } else {
            self.finalize_friction_diagnostic(body_a, body_b, &applied)
                .map(TerminalFrictionOutcome::Diagnostic)
                .map_err(CalibratedRuntimeFrictionError::Runtime)
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
    use symtropy_math::Point;
    use symtropy_physics::{
        BodyHandle, FrictionTransactionId, FrictionTransactionPhase, ThermalBody,
        ThermalMaterial, ThermalState,
    };

    fn thermal_body(handle: usize, velocity_x: f64) -> RigidBody<2> {
        let mut body = RigidBody::<2>::dynamic_sphere(
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
    fn calibrated_terminal_execution_keeps_private_runtime_ledger_authority() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = thermal_body(1, 1.0);
        let mut b = thermal_body(2, 0.0);
        let calibration = MechanicalUnitCalibration::new(1.0, 1.0 / 32.0, 1.0).unwrap();

        let terminal = runtime
            .execute_terminal_friction_impulse_at_calibrated(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::from([0.5, 0.0]),
                FrictionSolverCoordinates::new(2, 3, 4),
                HeatPartition::equal(),
                calibration,
            )
            .unwrap();

        let receipt = match terminal.outcome {
            TerminalFrictionOutcome::Promoted(receipt) => receipt,
            TerminalFrictionOutcome::Diagnostic(reason) => {
                panic!("expected calibrated promotion, got diagnostic {reason:?}")
            }
        };
        assert_eq!(receipt.transaction_id, FrictionTransactionId::new(0, 2, 3, 4));
        assert_eq!(receipt.heat.dissipated_joules, 0.25 / 1024.0);
        assert_eq!(runtime.physical_energy_ledger().len(), 3);
        assert_eq!(
            runtime.friction_journal().phase(receipt.transaction_id),
            Some(FrictionTransactionPhase::Promoted)
        );
        assert_eq!(runtime.pending_friction_reservation_count(), 0);
    }

    #[test]
    fn calibrated_promotion_failure_rolls_back_mechanics_and_applied_journal_entry() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = thermal_body(1, 1.0);
        let mut b = thermal_body(2, 0.0);
        b.clear_thermal();
        let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
        let before_b = (b.linear_velocity, b.angular_velocity, b.thermal);

        let error = runtime
            .execute_terminal_friction_impulse_at_calibrated(
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
            CalibratedRuntimeFrictionError::Promotion(
                CalibratedFrictionPromotionError::MissingThermalState
            )
        ));
        assert_eq!((a.linear_velocity, a.angular_velocity, a.thermal), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity, b.thermal), before_b);
        assert!(runtime.friction_journal().is_empty());
        assert!(runtime.physical_energy_ledger().is_empty());
        assert_eq!(runtime.pending_friction_reservation_count(), 0);
    }

    #[test]
    fn off_center_path_remains_diagnostic_without_physical_ledger_mutation() {
        let mut runtime = ThermodynamicTransactionRuntime::new();
        runtime.begin_next_tick().unwrap();
        let mut a = thermal_body(1, 0.0);
        let mut b = thermal_body(2, 0.0);
        a.transform.translation = Point::new([-1.0, 0.0]);
        b.transform.translation = Point::new([1.0, 0.0]);

        let terminal = runtime
            .execute_terminal_friction_impulse_at_calibrated(
                &mut a,
                &mut b,
                &SVector::from([0.0, 1.0]),
                &SVector::from([0.25, 0.0]),
                FrictionSolverCoordinates::new(1, 2, 3),
                HeatPartition::equal(),
                MechanicalUnitCalibration::new(1.0, 1.0 / 32.0, 1.0).unwrap(),
            )
            .unwrap();

        assert!(matches!(terminal.outcome, TerminalFrictionOutcome::Diagnostic(_)));
        assert!(runtime.physical_energy_ledger().is_empty());
        assert_eq!(
            runtime.friction_journal().phase(terminal.transaction_id),
            Some(FrictionTransactionPhase::DiagnosticOnly(_))
        );
    }
}
