// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Calibrated physical promotion of an already-applied friction transaction.
//!
//! The older `friction_promotion` theorem is retained as the SI-native reference
//! path: its mechanical-energy numbers are interpreted directly as Joules. This
//! module is the explicit unit-boundary variant for embeddings whose solver mass,
//! length or time units are not already kilograms, metres and seconds.
//!
//! All freshness/regime checks happen in solver-native state. Conversion to SI is
//! performed only after the exact measured mechanical transition is validated and
//! before any `ThermalBody` or `EnergyTransferLedger` mutation is staged.

use crate::body::RigidBody;
use crate::dissipation::{FrictionHeatResult, HeatPartition};
use crate::energy::{
    EnergyForm, EnergyLedgerError, EnergyOwner, EnergyPort, EnergyTransferKind,
    EnergyTransferLedger,
};
use crate::friction_evidence::{
    FrictionEvidenceRegime, FrictionMechanicalDelta, classify_friction_evidence_regime,
};
use crate::friction_promotion::FrictionPromotionReceipt;
use crate::friction_transaction::{
    AppliedFrictionTransaction, FrictionTransactionJournal, FrictionTransactionPhase,
    FrictionTransactionTransitionError,
};
use crate::mechanical_units::{MechanicalUnitCalibration, MechanicalUnitCalibrationError};
use crate::thermal::{ThermalBody, ThermalError};

const SOLVER_ENERGY_EPSILON: f64 = 1.0e-15;
const RELATIVE_TOLERANCE: f64 = 1.0e-12;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CalibratedFrictionPromotionError {
    UnknownTransaction,
    AlreadyTerminal,
    StaleMechanicalState,
    RequiresCenteredClosedDynamicPair,
    NonDissipativeObservation,
    InvalidObservation,
    MissingThermalState,
    InvalidHeatPartition,
    LedgerStateMismatch,
    Calibration(MechanicalUnitCalibrationError),
    Thermal(ThermalError),
    Ledger(EnergyLedgerError),
}

impl From<MechanicalUnitCalibrationError> for CalibratedFrictionPromotionError {
    fn from(value: MechanicalUnitCalibrationError) -> Self {
        Self::Calibration(value)
    }
}

impl From<ThermalError> for CalibratedFrictionPromotionError {
    fn from(value: ThermalError) -> Self {
        Self::Thermal(value)
    }
}

impl From<EnergyLedgerError> for CalibratedFrictionPromotionError {
    fn from(value: EnergyLedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<FrictionTransactionTransitionError> for CalibratedFrictionPromotionError {
    fn from(value: FrictionTransactionTransitionError) -> Self {
        match value {
            FrictionTransactionTransitionError::UnknownTransaction => Self::UnknownTransaction,
            FrictionTransactionTransitionError::AlreadyTerminal => Self::AlreadyTerminal,
        }
    }
}

#[inline]
fn close_enough(a: f64, b: f64, scale: f64) -> bool {
    a.is_finite()
        && b.is_finite()
        && (a - b).abs() <= RELATIVE_TOLERANCE * scale.abs().max(1.0)
}

#[inline]
fn kinetic_port<const D: usize>(body: &RigidBody<D>) -> EnergyPort {
    EnergyPort::new(EnergyOwner::Body(body.handle), EnergyForm::Kinetic)
}

#[inline]
fn thermal_port<const D: usize>(body: &RigidBody<D>) -> EnergyPort {
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
) -> Result<(), CalibratedFrictionPromotionError> {
    if !joules.is_finite() {
        return Err(CalibratedFrictionPromotionError::Ledger(
            EnergyLedgerError::NonFiniteEnergy,
        ));
    }
    if joules > 0.0 {
        ledger.record(source, destination, joules, EnergyTransferKind::Friction)?;
    }
    Ok(())
}

/// Promote one exact `Applied` friction transaction using an explicit solver->SI
/// mechanical unit calibration.
///
/// The bound observation's historical `*_joules` field names are intentionally
/// not trusted as an SI claim here. Their numeric values are treated as
/// solver-native energy coordinates, validated against the live post-state, and
/// converted through `calibration` before thermal or ledger mutation.
pub fn promote_applied_friction_loss_to_heat_calibrated<const D: usize>(
    body_a: &mut RigidBody<D>,
    body_b: &mut RigidBody<D>,
    applied: &AppliedFrictionTransaction<D>,
    partition: HeatPartition,
    calibration: MechanicalUnitCalibration,
    ledger: &mut EnergyTransferLedger,
    journal: &mut FrictionTransactionJournal,
) -> Result<FrictionPromotionReceipt, CalibratedFrictionPromotionError> {
    let transaction_id = applied.transaction_id();
    match journal.phase(transaction_id) {
        Some(FrictionTransactionPhase::Applied) => {}
        Some(FrictionTransactionPhase::Promoted)
        | Some(FrictionTransactionPhase::DiagnosticOnly(_)) => {
            return Err(CalibratedFrictionPromotionError::AlreadyTerminal);
        }
        None => return Err(CalibratedFrictionPromotionError::UnknownTransaction),
    }

    if !partition.fraction_to_a.is_finite()
        || !(0.0..=1.0).contains(&partition.fraction_to_a)
    {
        return Err(CalibratedFrictionPromotionError::InvalidHeatPartition);
    }

    if !applied.matches_post_state(body_a, body_b) {
        return Err(CalibratedFrictionPromotionError::StaleMechanicalState);
    }

    let observation = applied.observation();
    let current_regime = classify_friction_evidence_regime(
        body_a,
        body_b,
        &observation.contact_point,
    )
    .map_err(|_| CalibratedFrictionPromotionError::StaleMechanicalState)?;
    if observation.regime != FrictionEvidenceRegime::CenteredClosedDynamicPair
        || current_regime != FrictionEvidenceRegime::CenteredClosedDynamicPair
    {
        return Err(CalibratedFrictionPromotionError::RequiresCenteredClosedDynamicPair);
    }

    // Transitional naming note: these fields predate explicit solver-unit
    // calibration. Validate them as solver-native mechanical-energy quantities.
    let values = [
        observation.kinetic_before_a_joules,
        observation.kinetic_before_b_joules,
        observation.kinetic_after_a_joules,
        observation.kinetic_after_b_joules,
        observation.pair_delta_joules,
    ];
    if !values.iter().all(|value| value.is_finite())
        || observation.kinetic_before_a_joules < 0.0
        || observation.kinetic_before_b_joules < 0.0
        || observation.kinetic_after_a_joules < 0.0
        || observation.kinetic_after_b_joules < 0.0
    {
        return Err(CalibratedFrictionPromotionError::InvalidObservation);
    }

    let dissipated_solver = match observation.delta {
        FrictionMechanicalDelta::DissipationCandidate { joules }
            if joules.is_finite() && joules > SOLVER_ENERGY_EPSILON =>
        {
            joules
        }
        FrictionMechanicalDelta::DissipationCandidate { .. }
        | FrictionMechanicalDelta::SolverInjection { .. }
        | FrictionMechanicalDelta::Neutral => {
            return Err(CalibratedFrictionPromotionError::NonDissipativeObservation);
        }
    };

    let observed_pair_delta_solver = (observation.kinetic_after_a_joules
        + observation.kinetic_after_b_joules)
        - (observation.kinetic_before_a_joules + observation.kinetic_before_b_joules);
    if !close_enough(
        observed_pair_delta_solver,
        observation.pair_delta_joules,
        dissipated_solver,
    ) || !close_enough(
        observation.pair_delta_joules,
        -dissipated_solver,
        dissipated_solver,
    ) {
        return Err(CalibratedFrictionPromotionError::InvalidObservation);
    }

    // Freshness stays in solver-native state and solver-native energy. Unit
    // conversion must not weaken the exact-state binding of the applied token.
    let current_a_solver = body_a.kinetic_energy();
    let current_b_solver = body_b.kinetic_energy();
    if !current_a_solver.is_finite()
        || !current_b_solver.is_finite()
        || current_a_solver < 0.0
        || current_b_solver < 0.0
        || !close_enough(
            current_a_solver,
            observation.kinetic_after_a_joules,
            dissipated_solver,
        )
        || !close_enough(
            current_b_solver,
            observation.kinetic_after_b_joules,
            dissipated_solver,
        )
    {
        return Err(CalibratedFrictionPromotionError::StaleMechanicalState);
    }

    let change_a_solver =
        observation.kinetic_after_a_joules - observation.kinetic_before_a_joules;
    let change_b_solver =
        observation.kinetic_after_b_joules - observation.kinetic_before_b_joules;
    if !close_enough(
        change_a_solver + change_b_solver,
        -dissipated_solver,
        dissipated_solver,
    ) {
        return Err(CalibratedFrictionPromotionError::InvalidObservation);
    }

    // This is the physical-unit authority boundary. No thermal or ledger state
    // has been staged before all required mechanical quantities convert to SI.
    let dissipated_joules = calibration.energy_to_joules(dissipated_solver)?;
    let change_a_joules = calibration.signed_energy_to_joules(change_a_solver)?;
    let change_b_joules = calibration.signed_energy_to_joules(change_b_solver)?;
    if !close_enough(
        change_a_joules + change_b_joules,
        -dissipated_joules,
        dissipated_joules,
    ) {
        return Err(CalibratedFrictionPromotionError::InvalidObservation);
    }

    let Some(initial_thermal_a) = body_a.thermal else {
        return Err(CalibratedFrictionPromotionError::MissingThermalState);
    };
    let Some(initial_thermal_b) = body_b.thermal else {
        return Err(CalibratedFrictionPromotionError::MissingThermalState);
    };
    initial_thermal_a.validate()?;
    initial_thermal_b.validate()?;

    let heat_a = dissipated_joules * partition.fraction_to_a;
    let heat_b = dissipated_joules - heat_a;
    if !heat_a.is_finite() || !heat_b.is_finite() {
        return Err(CalibratedFrictionPromotionError::LedgerStateMismatch);
    }

    let original_ledger = ledger.clone();
    let mut next_ledger = original_ledger.clone();
    let mut next_journal = journal.clone();
    let kinetic_a = kinetic_port(body_a);
    let kinetic_b = kinetic_port(body_b);
    let thermal_a = thermal_port(body_a);
    let thermal_b = thermal_port(body_b);

    // Decompose calibrated per-body kinetic changes into inter-body transfer;
    // only the calibrated pair loss is eligible to become sensible heat.
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
        return Err(CalibratedFrictionPromotionError::LedgerStateMismatch);
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

    // Reconcile the staged ledger in physical Joules before committing any
    // thermal, ledger, or lifecycle state.
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
    if !max_residual.is_finite()
        || max_residual > RELATIVE_TOLERANCE * dissipated_joules.max(1.0)
    {
        return Err(CalibratedFrictionPromotionError::LedgerStateMismatch);
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
    use crate::body::BodyHandle;
    use crate::friction_promotion::promote_applied_friction_loss_to_heat;
    use crate::friction_transaction::{FrictionTransactionJournal, apply_friction_impulse_once};
    use crate::thermal::{ThermalMaterial, ThermalState};
    use nalgebra::SVector;
    use symtropy_math::Point;

    fn body(handle: usize, velocity_x: f64) -> RigidBody<3> {
        let mut body = RigidBody::dynamic_sphere(
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

    fn applied_pair(
        fixed_tick: u64,
    ) -> (
        RigidBody<3>,
        RigidBody<3>,
        AppliedFrictionTransaction<3>,
        FrictionTransactionJournal,
    ) {
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            crate::FrictionTransactionId::new(fixed_tick, 0, 0, 0),
            &mut journal,
        )
        .unwrap();
        (a, b, applied, journal)
    }

    #[test]
    fn si_calibration_reproduces_existing_reference_exactly() {
        let (mut calibrated_a, mut calibrated_b, calibrated_applied, mut calibrated_journal) =
            applied_pair(10);
        let mut calibrated_ledger = EnergyTransferLedger::new();
        let calibrated = promote_applied_friction_loss_to_heat_calibrated(
            &mut calibrated_a,
            &mut calibrated_b,
            &calibrated_applied,
            HeatPartition::equal(),
            MechanicalUnitCalibration::new(1.0, 1.0, 1.0).unwrap(),
            &mut calibrated_ledger,
            &mut calibrated_journal,
        )
        .unwrap();

        let (mut reference_a, mut reference_b, reference_applied, mut reference_journal) =
            applied_pair(10);
        let mut reference_ledger = EnergyTransferLedger::new();
        let reference = promote_applied_friction_loss_to_heat(
            &mut reference_a,
            &mut reference_b,
            &reference_applied,
            HeatPartition::equal(),
            &mut reference_ledger,
            &mut reference_journal,
        )
        .unwrap();

        assert_eq!(calibrated, reference);
        assert_eq!(calibrated_ledger, reference_ledger);
        assert_eq!(calibrated_a.thermal, reference_a.thermal);
        assert_eq!(calibrated_b.thermal, reference_b.thermal);
    }

    #[test]
    fn pixel_like_length_calibration_scales_heat_before_physical_mutation() {
        let (mut a, mut b, applied, mut journal) = applied_pair(11);
        let initial_a = a.thermal.unwrap();
        let initial_b = b.thermal.unwrap();
        let mut ledger = EnergyTransferLedger::new();

        let receipt = promote_applied_friction_loss_to_heat_calibrated(
            &mut a,
            &mut b,
            &applied,
            HeatPartition::equal(),
            MechanicalUnitCalibration::new(1.0, 1.0 / 32.0, 1.0).unwrap(),
            &mut ledger,
            &mut journal,
        )
        .unwrap();

        // Raw solver loss for this pair is 0.25; the M-L-T energy factor is 1/1024.
        assert_eq!(receipt.heat.dissipated_joules, 0.25 / 1024.0);
        assert_eq!(receipt.heat.heat_to_a_joules, 0.125 / 1024.0);
        assert_eq!(receipt.heat.heat_to_b_joules, 0.125 / 1024.0);
        assert_eq!(
            a.thermal.unwrap().sensible_energy_joules(0.0).unwrap()
                - initial_a.sensible_energy_joules(0.0).unwrap(),
            receipt.heat.heat_to_a_joules
        );
        assert_eq!(
            b.thermal.unwrap().sensible_energy_joules(0.0).unwrap()
                - initial_b.sensible_energy_joules(0.0).unwrap(),
            receipt.heat.heat_to_b_joules
        );
        assert_eq!(journal.phase(receipt.transaction_id), Some(FrictionTransactionPhase::Promoted));
    }

    #[test]
    fn missing_thermal_state_fails_without_ledger_or_lifecycle_commit() {
        let (mut a, mut b, applied, mut journal) = applied_pair(12);
        b.clear_thermal();
        let before_ledger = EnergyTransferLedger::new();
        let mut ledger = before_ledger.clone();

        assert_eq!(
            promote_applied_friction_loss_to_heat_calibrated(
                &mut a,
                &mut b,
                &applied,
                HeatPartition::equal(),
                MechanicalUnitCalibration::new(1.0, 1.0, 1.0).unwrap(),
                &mut ledger,
                &mut journal,
            ),
            Err(CalibratedFrictionPromotionError::MissingThermalState)
        );
        assert_eq!(ledger, before_ledger);
        assert_eq!(
            journal.phase(applied.transaction_id()),
            Some(FrictionTransactionPhase::Applied)
        );
    }
}
