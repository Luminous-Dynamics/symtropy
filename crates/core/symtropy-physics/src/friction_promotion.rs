// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Physical promotion of an already-applied friction transaction.
//!
//! This layer never applies mechanics. It accepts only an
//! [`AppliedFrictionTransaction`](crate::friction_transaction::AppliedFrictionTransaction)
//! produced by the canonical exactly-once application path, verifies that the
//! bound post-mechanical state is still current, then stages thermal + ledger +
//! lifecycle changes and commits them together.

use crate::body::RigidBody;
use crate::dissipation::{FrictionHeatResult, HeatPartition};
use crate::energy::{
    EnergyForm, EnergyLedgerError, EnergyOwner, EnergyPort, EnergyTransferKind,
    EnergyTransferLedger,
};
use crate::friction_evidence::{
    FrictionEvidenceRegime, FrictionMechanicalDelta, FrictionTransactionId,
    classify_friction_evidence_regime,
};
use crate::friction_transaction::{
    AppliedFrictionTransaction, FrictionTransactionJournal, FrictionTransactionPhase,
    FrictionTransactionTransitionError,
};
use crate::thermal::{ThermalBody, ThermalError};

const ENERGY_EPSILON_J: f64 = 1.0e-15;
const RELATIVE_TOLERANCE: f64 = 1.0e-12;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FrictionPromotionReceipt {
    pub transaction_id: FrictionTransactionId,
    pub heat: FrictionHeatResult,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrictionPromotionError {
    UnknownTransaction,
    AlreadyTerminal,
    StaleMechanicalState,
    RequiresCenteredClosedDynamicPair,
    NonDissipativeObservation,
    InvalidObservation,
    MissingThermalState,
    InvalidHeatPartition,
    LedgerStateMismatch,
    Thermal(ThermalError),
    Ledger(EnergyLedgerError),
}

impl From<ThermalError> for FrictionPromotionError {
    fn from(value: ThermalError) -> Self {
        Self::Thermal(value)
    }
}

impl From<EnergyLedgerError> for FrictionPromotionError {
    fn from(value: EnergyLedgerError) -> Self {
        Self::Ledger(value)
    }
}

impl From<FrictionTransactionTransitionError> for FrictionPromotionError {
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
) -> Result<(), FrictionPromotionError> {
    if !joules.is_finite() {
        return Err(FrictionPromotionError::Ledger(
            EnergyLedgerError::NonFiniteEnergy,
        ));
    }
    if joules > 0.0 {
        ledger.record(source, destination, joules, EnergyTransferKind::Friction)?;
    }
    Ok(())
}

/// Promote one canonical `Applied` friction transaction into physical heat.
///
/// The applied token is borrowed rather than consumed so a failed promotion may
/// be retried within the same still-open fixed tick after correcting a missing
/// thermal reservoir or other admissibility problem. A successful promotion
/// advances the canonical journal to `Promoted`; all later retries fail before
/// any physical state mutation.
pub fn promote_applied_friction_loss_to_heat<const D: usize>(
    body_a: &mut RigidBody<D>,
    body_b: &mut RigidBody<D>,
    applied: &AppliedFrictionTransaction<D>,
    partition: HeatPartition,
    ledger: &mut EnergyTransferLedger,
    journal: &mut FrictionTransactionJournal,
) -> Result<FrictionPromotionReceipt, FrictionPromotionError> {
    let transaction_id = applied.transaction_id();
    match journal.phase(transaction_id) {
        Some(FrictionTransactionPhase::Applied) => {}
        Some(FrictionTransactionPhase::Promoted)
        | Some(FrictionTransactionPhase::DiagnosticOnly(_)) => {
            return Err(FrictionPromotionError::AlreadyTerminal);
        }
        None => return Err(FrictionPromotionError::UnknownTransaction),
    }

    if !partition.fraction_to_a.is_finite()
        || !(0.0..=1.0).contains(&partition.fraction_to_a)
    {
        return Err(FrictionPromotionError::InvalidHeatPartition);
    }

    if !applied.matches_post_state(body_a, body_b) {
        return Err(FrictionPromotionError::StaleMechanicalState);
    }

    let observation = applied.observation();
    let current_regime = classify_friction_evidence_regime(
        body_a,
        body_b,
        &observation.contact_point,
    )
    .map_err(|_| FrictionPromotionError::StaleMechanicalState)?;
    if observation.regime != FrictionEvidenceRegime::CenteredClosedDynamicPair
        || current_regime != FrictionEvidenceRegime::CenteredClosedDynamicPair
    {
        return Err(FrictionPromotionError::RequiresCenteredClosedDynamicPair);
    }

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
        return Err(FrictionPromotionError::InvalidObservation);
    }

    let dissipated = match observation.delta {
        FrictionMechanicalDelta::DissipationCandidate { joules }
            if joules.is_finite() && joules > ENERGY_EPSILON_J =>
        {
            joules
        }
        FrictionMechanicalDelta::DissipationCandidate { .. }
        | FrictionMechanicalDelta::SolverInjection { .. }
        | FrictionMechanicalDelta::Neutral => {
            return Err(FrictionPromotionError::NonDissipativeObservation);
        }
    };

    let observed_pair_delta = (observation.kinetic_after_a_joules
        + observation.kinetic_after_b_joules)
        - (observation.kinetic_before_a_joules + observation.kinetic_before_b_joules);
    if !close_enough(
        observed_pair_delta,
        observation.pair_delta_joules,
        dissipated,
    ) || !close_enough(observation.pair_delta_joules, -dissipated, dissipated)
    {
        return Err(FrictionPromotionError::InvalidObservation);
    }

    // Recompute endpoint energy from the live bodies to catch changes in
    // energy-relevant mass/inertia state after the bound observation.
    let current_a = body_a.kinetic_energy();
    let current_b = body_b.kinetic_energy();
    if !current_a.is_finite()
        || !current_b.is_finite()
        || current_a < 0.0
        || current_b < 0.0
        || !close_enough(current_a, observation.kinetic_after_a_joules, dissipated)
        || !close_enough(current_b, observation.kinetic_after_b_joules, dissipated)
    {
        return Err(FrictionPromotionError::StaleMechanicalState);
    }

    let change_a = observation.kinetic_after_a_joules - observation.kinetic_before_a_joules;
    let change_b = observation.kinetic_after_b_joules - observation.kinetic_before_b_joules;
    if !close_enough(change_a + change_b, -dissipated, dissipated) {
        return Err(FrictionPromotionError::InvalidObservation);
    }

    let Some(initial_thermal_a) = body_a.thermal else {
        return Err(FrictionPromotionError::MissingThermalState);
    };
    let Some(initial_thermal_b) = body_b.thermal else {
        return Err(FrictionPromotionError::MissingThermalState);
    };
    initial_thermal_a.validate()?;
    initial_thermal_b.validate()?;

    let heat_a = dissipated * partition.fraction_to_a;
    let heat_b = dissipated - heat_a;
    if !heat_a.is_finite() || !heat_b.is_finite() {
        return Err(FrictionPromotionError::LedgerStateMismatch);
    }

    let original_ledger = ledger.clone();
    let mut next_ledger = original_ledger.clone();
    let mut next_journal = journal.clone();
    let kinetic_a = kinetic_port(body_a);
    let kinetic_b = kinetic_port(body_b);
    let thermal_a = thermal_port(body_a);
    let thermal_b = thermal_port(body_b);

    // Decompose per-body kinetic changes into pure inter-body kinetic transfer
    // first. Only the residual measured pair loss is eligible to become heat.
    let loss_a = -change_a;
    let loss_b = -change_b;
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
    if !residual_total.is_finite() || !close_enough(residual_total, dissipated, dissipated) {
        return Err(FrictionPromotionError::LedgerStateMismatch);
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

    // Reconcile staged accounting against the measured endpoint deltas before
    // committing thermal, ledger, or lifecycle state.
    let kinetic_residual_a = change_a
        - (next_ledger.net_change_for(kinetic_a) - original_ledger.net_change_for(kinetic_a));
    let kinetic_residual_b = change_b
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
        || max_residual > RELATIVE_TOLERANCE * dissipated.max(1.0)
    {
        return Err(FrictionPromotionError::LedgerStateMismatch);
    }

    next_journal.mark_promoted(transaction_id)?;

    body_a.thermal = Some(next_thermal_a);
    body_b.thermal = Some(next_thermal_b);
    *ledger = next_ledger;
    *journal = next_journal;

    Ok(FrictionPromotionReceipt {
        transaction_id,
        heat: FrictionHeatResult {
            kinetic_change_a_joules: change_a,
            kinetic_change_b_joules: change_b,
            dissipated_joules: dissipated,
            heat_to_a_joules: heat_a,
            heat_to_b_joules: heat_b,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;
    use crate::dissipation::apply_friction_impulse_with_heat;
    use crate::friction_evidence::FrictionTransactionId;
    use crate::friction_transaction::{
        FrictionDiagnosticFinalizeError, FrictionTransactionJournal,
        apply_friction_impulse_once, finalize_friction_diagnostic,
    };
    use crate::thermal::{ThermalMaterial, ThermalState};
    use nalgebra::SVector;
    use symtropy_math::Point;

    fn body(handle: usize, velocity_x: f64, with_thermal: bool) -> RigidBody<3> {
        let mut body = RigidBody::dynamic_sphere(
            BodyHandle(handle),
            Point::origin(),
            0.5,
            1.0,
        );
        body.linear_velocity[0] = velocity_x;
        if with_thermal {
            body.set_thermal(
                ThermalBody::new(
                    ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
                    ThermalState::new(300.0).unwrap(),
                    1.0,
                )
                .unwrap(),
            );
        }
        body
    }

    #[test]
    fn applied_promotion_matches_existing_centered_reference_exactly() {
        let id = FrictionTransactionId::new(30, 1, 2, 0);
        let mut staged_a = body(1, 1.0, true);
        let mut staged_b = body(2, 0.0, true);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut staged_a,
            &mut staged_b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            id,
            &mut journal,
        )
        .unwrap();
        let mut staged_ledger = EnergyTransferLedger::new();

        let receipt = promote_applied_friction_loss_to_heat(
            &mut staged_a,
            &mut staged_b,
            &applied,
            HeatPartition::equal(),
            &mut staged_ledger,
            &mut journal,
        )
        .unwrap();

        let mut reference_a = body(1, 1.0, true);
        let mut reference_b = body(2, 0.0, true);
        let mut reference_ledger = EnergyTransferLedger::new();
        let reference = apply_friction_impulse_with_heat(
            &mut reference_a,
            &mut reference_b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            HeatPartition::equal(),
            &mut reference_ledger,
        )
        .unwrap();

        assert_eq!(receipt.transaction_id, id);
        assert_eq!(receipt.heat, reference);
        assert_eq!(staged_a.linear_velocity, reference_a.linear_velocity);
        assert_eq!(staged_b.linear_velocity, reference_b.linear_velocity);
        assert_eq!(staged_a.angular_velocity, reference_a.angular_velocity);
        assert_eq!(staged_b.angular_velocity, reference_b.angular_velocity);
        assert_eq!(staged_a.thermal, reference_a.thermal);
        assert_eq!(staged_b.thermal, reference_b.thermal);
        assert_eq!(staged_ledger, reference_ledger);
        assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Promoted));
        assert!(journal.is_complete_for_finalize());
    }

    #[test]
    fn successful_promotion_is_exactly_once() {
        let id = FrictionTransactionId::new(31, 0, 0, 0);
        let mut a = body(1, 1.0, true);
        let mut b = body(2, 0.0, true);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            id,
            &mut journal,
        )
        .unwrap();
        let mut ledger = EnergyTransferLedger::new();
        promote_applied_friction_loss_to_heat(
            &mut a,
            &mut b,
            &applied,
            HeatPartition::equal(),
            &mut ledger,
            &mut journal,
        )
        .unwrap();

        let before_a = a.thermal;
        let before_b = b.thermal;
        let before_ledger = ledger.clone();
        let before_journal = journal.clone();
        assert_eq!(
            promote_applied_friction_loss_to_heat(
                &mut a,
                &mut b,
                &applied,
                HeatPartition::equal(),
                &mut ledger,
                &mut journal,
            ),
            Err(FrictionPromotionError::AlreadyTerminal)
        );
        assert_eq!(a.thermal, before_a);
        assert_eq!(b.thermal, before_b);
        assert_eq!(ledger, before_ledger);
        assert_eq!(journal, before_journal);
    }

    #[test]
    fn failed_missing_thermal_promotion_can_be_retried_without_reapplying_mechanics() {
        let id = FrictionTransactionId::new(32, 0, 0, 0);
        let mut a = body(1, 1.0, false);
        let mut b = body(2, 0.0, true);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            id,
            &mut journal,
        )
        .unwrap();
        let mechanical_a = (a.linear_velocity, a.angular_velocity);
        let mechanical_b = (b.linear_velocity, b.angular_velocity);
        let mut ledger = EnergyTransferLedger::new();

        assert_eq!(
            promote_applied_friction_loss_to_heat(
                &mut a,
                &mut b,
                &applied,
                HeatPartition::equal(),
                &mut ledger,
                &mut journal,
            ),
            Err(FrictionPromotionError::MissingThermalState)
        );
        assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Applied));
        assert!(!journal.is_complete_for_finalize());
        assert!(ledger.is_empty());

        a.set_thermal(
            ThermalBody::new(
                ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
                ThermalState::new(300.0).unwrap(),
                1.0,
            )
            .unwrap(),
        );
        promote_applied_friction_loss_to_heat(
            &mut a,
            &mut b,
            &applied,
            HeatPartition::equal(),
            &mut ledger,
            &mut journal,
        )
        .unwrap();

        assert_eq!((a.linear_velocity, a.angular_velocity), mechanical_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), mechanical_b);
        assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Promoted));
        assert!(journal.is_complete_for_finalize());
    }

    #[test]
    fn centered_loss_cannot_be_diagnostic_after_failed_physical_promotion() {
        let id = FrictionTransactionId::new(33, 0, 0, 0);
        let mut a = body(1, 1.0, false);
        let mut b = body(2, 0.0, true);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            id,
            &mut journal,
        )
        .unwrap();
        let mut ledger = EnergyTransferLedger::new();

        assert_eq!(
            promote_applied_friction_loss_to_heat(
                &mut a,
                &mut b,
                &applied,
                HeatPartition::equal(),
                &mut ledger,
                &mut journal,
            ),
            Err(FrictionPromotionError::MissingThermalState)
        );
        assert_eq!(
            finalize_friction_diagnostic(&a, &b, &applied, &mut journal),
            Err(FrictionDiagnosticFinalizeError::RequiresPhysicalPromotion)
        );
        assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Applied));
    }
}