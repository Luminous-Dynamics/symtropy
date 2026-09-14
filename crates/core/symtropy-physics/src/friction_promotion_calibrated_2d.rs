// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Calibrated physical promotion from token-bound stable 2D friction evidence.
//!
//! This is the qualified 2D counterpart to the historical generic calibrated
//! promotion path. Mechanical amount authority comes only from the exact
//! `AppliedFrictionTransaction<2>` plus #1046's stable factored transition
//! theorem; historical absolute pre/post energy fields are not consulted.

use crate::body::RigidBody;
use crate::dissipation::{FrictionHeatResult, HeatPartition};
use crate::energy::{
    EnergyForm, EnergyLedgerError, EnergyOwner, EnergyPort, EnergyTransferKind,
    EnergyTransferLedger,
};
use crate::friction_applied_transition_2d::{
    AppliedFrictionTransition2dError, classify_applied_friction_transition_2d_checked,
};
use crate::friction_evidence::{FrictionEvidenceRegime, classify_friction_evidence_regime};
use crate::friction_promotion::FrictionPromotionReceipt;
use crate::friction_transaction::{
    AppliedFrictionTransaction, FrictionTransactionJournal, FrictionTransactionPhase,
    FrictionTransactionTransitionError,
};
use crate::friction_transition_energy_2d::FrictionTransitionDelta2d;
use crate::mechanical_units::{MechanicalUnitCalibration, MechanicalUnitCalibrationError};
use crate::thermal::{ThermalBody, ThermalError};

const RELATIVE_TOLERANCE: f64 = 1.0e-12;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StableCalibratedFrictionPromotion2dError {
    UnknownTransaction,
    AlreadyTerminal,
    RequiresCenteredClosedDynamicPair,
    NonDissipativeTransition,
    InvalidHeatPartition,
    MissingThermalState,
    LedgerStateMismatch,
    Transition(AppliedFrictionTransition2dError),
    Calibration(MechanicalUnitCalibrationError),
    Thermal(ThermalError),
    Ledger(EnergyLedgerError),
}

impl From<AppliedFrictionTransition2dError> for StableCalibratedFrictionPromotion2dError {
    fn from(value: AppliedFrictionTransition2dError) -> Self {
        Self::Transition(value)
    }
}

impl From<MechanicalUnitCalibrationError> for StableCalibratedFrictionPromotion2dError {
    fn from(value: MechanicalUnitCalibrationError) -> Self {
        Self::Calibration(value)
    }
}

impl From<ThermalError> for StableCalibratedFrictionPromotion2dError {
    fn from(value: ThermalError) -> Self {
        Self::Thermal(value)
    }
}

impl From<EnergyLedgerError> for StableCalibratedFrictionPromotion2dError {
    fn from(value: EnergyLedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<FrictionTransactionTransitionError> for StableCalibratedFrictionPromotion2dError {
    fn from(value: FrictionTransactionTransitionError) -> Self {
        match value {
            FrictionTransactionTransitionError::UnknownTransaction => Self::UnknownTransaction,
            FrictionTransactionTransitionError::AlreadyTerminal => Self::AlreadyTerminal,
        }
    }
}

#[inline]
fn close_enough(a: f64, b: f64, scale: f64) -> bool {
    if !a.is_finite() || !b.is_finite() || !scale.is_finite() || scale < 0.0 {
        return false;
    }
    let tolerance = RELATIVE_TOLERANCE * scale;
    tolerance.is_finite() && (a - b).abs() <= tolerance
}

#[inline]
fn kinetic_port(body: &RigidBody<2>) -> EnergyPort {
    EnergyPort::new(EnergyOwner::Body(body.handle), EnergyForm::Kinetic)
}

#[inline]
fn thermal_port(body: &RigidBody<2>) -> EnergyPort {
    EnergyPort::new(
        EnergyOwner::Body(body.handle),
        EnergyForm::ThermalSensible,
    )
}

fn record_positive(
    ledger: &mut EnergyTransferLedger,
    source: EnergyPort,
    destination: EnergyPort,
    joules: f64,
) -> Result<(), StableCalibratedFrictionPromotion2dError> {
    if !joules.is_finite() {
        return Err(StableCalibratedFrictionPromotion2dError::Ledger(
            EnergyLedgerError::NonFiniteEnergy,
        ));
    }
    if joules > 0.0 {
        ledger.record(source, destination, joules, EnergyTransferKind::Friction)?;
    }
    Ok(())
}

/// Promote one exact 2D `Applied` friction transaction from token-bound stable
/// solver-energy evidence through an explicit solver-unit -> SI calibration.
///
/// This function never reads the historical observation's absolute kinetic-energy
/// fields or `FrictionMechanicalDelta`. Geometry/body-type regime remains a
/// separate admissibility gate; stable measurement does not make off-center or
/// external-boundary friction physically promotable.
pub fn promote_applied_friction_loss_to_heat_calibrated_2d_stable(
    body_a: &mut RigidBody<2>,
    body_b: &mut RigidBody<2>,
    applied: &AppliedFrictionTransaction<2>,
    partition: HeatPartition,
    calibration: MechanicalUnitCalibration,
    ledger: &mut EnergyTransferLedger,
    journal: &mut FrictionTransactionJournal,
) -> Result<FrictionPromotionReceipt, StableCalibratedFrictionPromotion2dError> {
    let transaction_id = applied.transaction_id();
    match journal.phase(transaction_id) {
        Some(FrictionTransactionPhase::Applied) => {}
        Some(FrictionTransactionPhase::Promoted)
        | Some(FrictionTransactionPhase::DiagnosticOnly(_)) => {
            return Err(StableCalibratedFrictionPromotion2dError::AlreadyTerminal);
        }
        None => return Err(StableCalibratedFrictionPromotion2dError::UnknownTransaction),
    }

    if !partition.fraction_to_a.is_finite()
        || !(0.0..=1.0).contains(&partition.fraction_to_a)
    {
        return Err(StableCalibratedFrictionPromotion2dError::InvalidHeatPartition);
    }

    // This call is the sole mechanical-amount authority. It also proves exact
    // #1041 post-state freshness before reconstructing the transition.
    let transition = classify_applied_friction_transition_2d_checked(body_a, body_b, applied)?;

    let observation = applied.observation();
    let current_regime = classify_friction_evidence_regime(
        body_a,
        body_b,
        &observation.contact_point,
    )
    .map_err(|_| StableCalibratedFrictionPromotion2dError::RequiresCenteredClosedDynamicPair)?;
    if observation.regime != FrictionEvidenceRegime::CenteredClosedDynamicPair
        || current_regime != FrictionEvidenceRegime::CenteredClosedDynamicPair
    {
        return Err(
            StableCalibratedFrictionPromotion2dError::RequiresCenteredClosedDynamicPair,
        );
    }

    let dissipated_solver = match transition.delta {
        FrictionTransitionDelta2d::DissipationCandidate { solver_energy }
            if solver_energy.is_finite() && solver_energy > 0.0 =>
        {
            solver_energy
        }
        FrictionTransitionDelta2d::DissipationCandidate { .. }
        | FrictionTransitionDelta2d::SolverInjection { .. }
        | FrictionTransitionDelta2d::Neutral => {
            return Err(StableCalibratedFrictionPromotion2dError::NonDissipativeTransition);
        }
    };

    let change_a_solver = transition.kinetic_change_a_solver;
    let change_b_solver = transition.kinetic_change_b_solver;
    if !close_enough(
        change_a_solver + change_b_solver,
        -dissipated_solver,
        dissipated_solver,
    ) {
        return Err(StableCalibratedFrictionPromotion2dError::LedgerStateMismatch);
    }

    // Physical-unit authority boundary. No thermal, ledger, or lifecycle state
    // is staged before all stable solver-energy quantities convert to SI.
    let dissipated_joules = calibration.energy_to_joules(dissipated_solver)?;
    let change_a_joules = calibration.signed_energy_to_joules(change_a_solver)?;
    let change_b_joules = calibration.signed_energy_to_joules(change_b_solver)?;
    if !close_enough(
        change_a_joules + change_b_joules,
        -dissipated_joules,
        dissipated_joules,
    ) {
        return Err(StableCalibratedFrictionPromotion2dError::LedgerStateMismatch);
    }

    let Some(initial_thermal_a) = body_a.thermal else {
        return Err(StableCalibratedFrictionPromotion2dError::MissingThermalState);
    };
    let Some(initial_thermal_b) = body_b.thermal else {
        return Err(StableCalibratedFrictionPromotion2dError::MissingThermalState);
    };
    initial_thermal_a.validate()?;
    initial_thermal_b.validate()?;

    let heat_a = dissipated_joules * partition.fraction_to_a;
    let heat_b = dissipated_joules - heat_a;
    if !heat_a.is_finite() || !heat_b.is_finite() {
        return Err(StableCalibratedFrictionPromotion2dError::LedgerStateMismatch);
    }

    let original_ledger = ledger.clone();
    let mut next_ledger = original_ledger.clone();
    let mut next_journal = journal.clone();
    let kinetic_a = kinetic_port(body_a);
    let kinetic_b = kinetic_port(body_b);
    let thermal_a = thermal_port(body_a);
    let thermal_b = thermal_port(body_b);

    // Decompose stable per-body kinetic changes into inter-body transfer. Only
    // the stable pair loss is converted to sensible heat.
    let loss_a = -change_a_joules;
    let loss_b = -change_b_joules;
    let mut residual_a = loss_a.max(0.0);
    let mut residual_b = loss_b.max(0.0);

    if loss_a > 0.0 && loss_b < 0.0 {
        let transfer = residual_a.min(-loss_b);
        record_positive(&mut next_ledger, kinetic_a, kinetic_b, transfer)?;
        residual_a -= transfer;
    } else if loss_b > 0.0 && loss_a < 0.0 {
        let transfer = residual_b.min(-loss_a);
        record_positive(&mut next_ledger, kinetic_b, kinetic_a, transfer)?;
        residual_b -= transfer;
    }

    let residual_total = residual_a + residual_b;
    if !residual_total.is_finite()
        || !close_enough(residual_total, dissipated_joules, dissipated_joules)
    {
        return Err(StableCalibratedFrictionPromotion2dError::LedgerStateMismatch);
    }

    for (source, residual) in [(kinetic_a, residual_a), (kinetic_b, residual_b)] {
        record_positive(
            &mut next_ledger,
            source,
            thermal_a,
            residual * partition.fraction_to_a,
        )?;
        record_positive(
            &mut next_ledger,
            source,
            thermal_b,
            residual * (1.0 - partition.fraction_to_a),
        )?;
    }

    let mut next_thermal_a: ThermalBody = initial_thermal_a;
    let mut next_thermal_b: ThermalBody = initial_thermal_b;
    next_thermal_a.add_heat_joules(heat_a)?;
    next_thermal_b.add_heat_joules(heat_b)?;

    // Reconcile staged physical ledger effects before any authoritative commit.
    let kinetic_residual_a = change_a_joules
        - (next_ledger.net_change_for(kinetic_a) - original_ledger.net_change_for(kinetic_a));
    let kinetic_residual_b = change_b_joules
        - (next_ledger.net_change_for(kinetic_b) - original_ledger.net_change_for(kinetic_b));
    let thermal_residual_a = heat_a
        - (next_ledger.net_change_for(thermal_a) - original_ledger.net_change_for(thermal_a));
    let thermal_residual_b = heat_b
        - (next_ledger.net_change_for(thermal_b) - original_ledger.net_change_for(thermal_b));
    let max_residual = kinetic_residual_a
        .abs()
        .max(kinetic_residual_b.abs())
        .max(thermal_residual_a.abs())
        .max(thermal_residual_b.abs());
    let allowed_residual = RELATIVE_TOLERANCE * dissipated_joules;
    if !max_residual.is_finite()
        || !allowed_residual.is_finite()
        || max_residual > allowed_residual
    {
        return Err(StableCalibratedFrictionPromotion2dError::LedgerStateMismatch);
    }

    next_journal.mark_promoted(transaction_id)?;

    body_a.thermal = Some(next_thermal_a);
    body_b.thermal = Some(next_thermal_b);
    *ledger = next_ledger;
    *journal = next_journal;

    Ok(FrictionPromotionReceipt {
        transaction_id,
        heat: FrictionHeatResult {
            kinetic_change_a_joules: change_a_joules,
            kinetic_change_b_joules: change_b_joules,
            dissipated_joules,
            heat_to_a_joules: heat_a,
            heat_to_b_joules: heat_b,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{BodyHandle, BodyType};
    use crate::friction_evidence::{FrictionMechanicalDelta, FrictionTransactionId};
    use crate::friction_transaction::{FrictionTransactionJournal, apply_friction_impulse_once};
    use crate::thermal::{ThermalMaterial, ThermalState};
    use nalgebra::SVector;
    use symtropy_math::{Point, Sphere, Transform};

    fn thermal_body(handle: usize, velocity_x: f64, angular_velocity: f64) -> RigidBody<2> {
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
    fn stable_promotion_uses_loss_erased_by_historical_observation() {
        let mut a = thermal_body(1, 1.0, 100_000_000.0);
        let mut b = thermal_body(2, 0.0, 0.0);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0]),
            FrictionTransactionId::new(20, 0, 0, 0),
            &mut journal,
        )
        .unwrap();
        assert_eq!(applied.observation().delta, FrictionMechanicalDelta::Neutral);

        let mut ledger = EnergyTransferLedger::new();
        let receipt = promote_applied_friction_loss_to_heat_calibrated_2d_stable(
            &mut a,
            &mut b,
            &applied,
            HeatPartition::equal(),
            MechanicalUnitCalibration::new(1.0, 1.0, 1.0).unwrap(),
            &mut ledger,
            &mut journal,
        )
        .unwrap();

        assert_eq!(receipt.heat.dissipated_joules, 0.25);
        assert_eq!(receipt.heat.kinetic_change_a_joules, -0.375);
        assert_eq!(receipt.heat.kinetic_change_b_joules, 0.125);
        assert_eq!(receipt.heat.heat_to_a_joules, 0.125);
        assert_eq!(receipt.heat.heat_to_b_joules, 0.125);
        assert_eq!(ledger.len(), 3);
        assert_eq!(
            journal.phase(applied.transaction_id()),
            Some(FrictionTransactionPhase::Promoted)
        );
    }

    #[test]
    fn pixel_like_calibration_scales_stable_loss_before_commit() {
        let mut a = thermal_body(1, 1.0, 2.0);
        let mut b = thermal_body(2, 0.0, -1.0);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0]),
            FrictionTransactionId::new(21, 0, 0, 0),
            &mut journal,
        )
        .unwrap();
        let mut ledger = EnergyTransferLedger::new();

        let receipt = promote_applied_friction_loss_to_heat_calibrated_2d_stable(
            &mut a,
            &mut b,
            &applied,
            HeatPartition::equal(),
            MechanicalUnitCalibration::new(1.0, 1.0 / 32.0, 1.0).unwrap(),
            &mut ledger,
            &mut journal,
        )
        .unwrap();

        assert_eq!(receipt.heat.dissipated_joules, 0.25 / 1024.0);
        assert_eq!(receipt.heat.heat_to_a_joules, 0.125 / 1024.0);
        assert_eq!(receipt.heat.heat_to_b_joules, 0.125 / 1024.0);
    }

    #[test]
    fn stale_mechanical_interpretation_fails_without_authoritative_commit() {
        let mut a = thermal_body(1, 1.0, 2.0);
        let mut b = thermal_body(2, 0.0, 0.0);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0]),
            FrictionTransactionId::new(22, 0, 0, 0),
            &mut journal,
        )
        .unwrap();
        a.inertia *= 2.0;
        a.inv_inertia *= 0.5;
        let before_thermal_a = a.thermal;
        let before_thermal_b = b.thermal;
        let mut ledger = EnergyTransferLedger::new();

        assert_eq!(
            promote_applied_friction_loss_to_heat_calibrated_2d_stable(
                &mut a,
                &mut b,
                &applied,
                HeatPartition::equal(),
                MechanicalUnitCalibration::new(1.0, 1.0, 1.0).unwrap(),
                &mut ledger,
                &mut journal,
            ),
            Err(StableCalibratedFrictionPromotion2dError::Transition(
                AppliedFrictionTransition2dError::StaleMechanicalState
            ))
        );
        assert_eq!(a.thermal, before_thermal_a);
        assert_eq!(b.thermal, before_thermal_b);
        assert!(ledger.is_empty());
        assert_eq!(
            journal.phase(applied.transaction_id()),
            Some(FrictionTransactionPhase::Applied)
        );
    }

    #[test]
    fn missing_thermal_state_fails_before_ledger_or_lifecycle_commit() {
        let mut a = thermal_body(1, 1.0, 0.0);
        let mut b = thermal_body(2, 0.0, 0.0);
        b.clear_thermal();
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0]),
            FrictionTransactionId::new(23, 0, 0, 0),
            &mut journal,
        )
        .unwrap();
        let mut ledger = EnergyTransferLedger::new();

        assert_eq!(
            promote_applied_friction_loss_to_heat_calibrated_2d_stable(
                &mut a,
                &mut b,
                &applied,
                HeatPartition::equal(),
                MechanicalUnitCalibration::new(1.0, 1.0, 1.0).unwrap(),
                &mut ledger,
                &mut journal,
            ),
            Err(StableCalibratedFrictionPromotion2dError::MissingThermalState)
        );
        assert!(ledger.is_empty());
        assert_eq!(
            journal.phase(applied.transaction_id()),
            Some(FrictionTransactionPhase::Applied)
        );
    }
}
