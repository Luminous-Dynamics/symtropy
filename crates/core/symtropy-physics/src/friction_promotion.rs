// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Physical promotion of an already-applied friction observation.
//!
//! Mechanical state transition and physical heat authority are deliberately
//! separate. [`crate::friction_evidence::apply_friction_impulse_measured`]
//! applies the solver impulse once and returns signed evidence. This module may
//! then promote a fresh, centered closed-pair loss into authoritative thermal
//! state + double-entry ledger entries without applying mechanics a second time.

use crate::body::RigidBody;
use crate::dissipation::{FrictionHeatResult, HeatPartition};
use crate::energy::{
    EnergyForm, EnergyLedgerError, EnergyOwner, EnergyPort, EnergyTransferKind,
    EnergyTransferLedger,
};
use crate::friction_evidence::{
    FrictionEvidenceRegime, FrictionMechanicalDelta, FrictionMechanicalObservation,
};
use crate::thermal::{ThermalBody, ThermalError};

const ENERGY_EPSILON_J: f64 = 1.0e-15;
const RELATIVE_TOLERANCE: f64 = 1.0e-12;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrictionPromotionError {
    SameBody,
    ObservationIdentityMismatch,
    RequiresCenteredClosedDynamicPair,
    NonDissipativeObservation,
    InvalidObservation,
    StaleMechanicalState,
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

fn validate_observation<const D: usize>(
    body_a: &RigidBody<D>,
    body_b: &RigidBody<D>,
    observation: &FrictionMechanicalObservation<D>,
) -> Result<(f64, f64, f64), FrictionPromotionError> {
    if body_a.handle == body_b.handle {
        return Err(FrictionPromotionError::SameBody);
    }
    if observation.body_a != body_a.handle || observation.body_b != body_b.handle {
        return Err(FrictionPromotionError::ObservationIdentityMismatch);
    }
    if observation.regime != FrictionEvidenceRegime::CenteredClosedDynamicPair
        || !body_a.is_dynamic()
        || !body_b.is_dynamic()
    {
        return Err(FrictionPromotionError::RequiresCenteredClosedDynamicPair);
    }

    if !observation
        .contact_point
        .iter()
        .all(|value| value.is_finite())
        || !observation
            .impulse_on_b
            .iter()
            .all(|value| value.is_finite())
    {
        return Err(FrictionPromotionError::InvalidObservation);
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

    // Promotion is only valid for the immediate post-mechanical state. This
    // catches stale observations and accidental replay after later solver work.
    let current_a = body_a.kinetic_energy();
    let current_b = body_b.kinetic_energy();
    if !current_a.is_finite()
        || !current_b.is_finite()
        || current_a < 0.0
        || current_b < 0.0
        || !close_enough(
            current_a,
            observation.kinetic_after_a_joules,
            dissipated,
        )
        || !close_enough(
            current_b,
            observation.kinetic_after_b_joules,
            dissipated,
        )
    {
        return Err(FrictionPromotionError::StaleMechanicalState);
    }

    let change_a = observation.kinetic_after_a_joules - observation.kinetic_before_a_joules;
    let change_b = observation.kinetic_after_b_joules - observation.kinetic_before_b_joules;
    if !close_enough(change_a + change_b, -dissipated, dissipated) {
        return Err(FrictionPromotionError::InvalidObservation);
    }

    Ok((change_a, change_b, dissipated))
}

/// Promote a fresh, already-applied centered friction loss into sensible heat.
///
/// This function does **not** modify linear/angular mechanical state. Thermal
/// state and ledger changes are staged and committed together; any error leaves
/// both thermal reservoirs and the supplied ledger unchanged.
pub fn promote_measured_friction_loss_to_heat<const D: usize>(
    body_a: &mut RigidBody<D>,
    body_b: &mut RigidBody<D>,
    observation: &FrictionMechanicalObservation<D>,
    partition: HeatPartition,
    ledger: &mut EnergyTransferLedger,
) -> Result<FrictionHeatResult, FrictionPromotionError> {
    if !partition.fraction_to_a.is_finite()
        || !(0.0..=1.0).contains(&partition.fraction_to_a)
    {
        return Err(FrictionPromotionError::InvalidHeatPartition);
    }

    let (change_a, change_b, dissipated) = validate_observation(body_a, body_b, observation)?;

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
    let kinetic_a = kinetic_port(body_a);
    let kinetic_b = kinetic_port(body_b);
    let thermal_a = thermal_port(body_a);
    let thermal_b = thermal_port(body_b);

    // Decompose the observed per-body kinetic changes into pure kinetic
    // transfer first, then route the remaining measured loss into heat.
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
    if !residual_total.is_finite()
        || !close_enough(residual_total, dissipated, dissipated)
    {
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

    // Reconcile staged accounting to the measured state changes before commit.
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

    body_a.thermal = Some(next_thermal_a);
    body_b.thermal = Some(next_thermal_b);
    *ledger = next_ledger;

    Ok(FrictionHeatResult {
        kinetic_change_a_joules: change_a,
        kinetic_change_b_joules: change_b,
        dissipated_joules: dissipated,
        heat_to_a_joules: heat_a,
        heat_to_b_joules: heat_b,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;
    use crate::dissipation::apply_friction_impulse_with_heat;
    use crate::friction_evidence::apply_friction_impulse_measured;
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

    #[test]
    fn already_applied_promotion_matches_existing_audited_reference() {
        let contact = SVector::zeros();
        let impulse = SVector::from([0.5, 0.0, 0.0]);

        let mut staged_a = body(1, 1.0);
        let mut staged_b = body(2, 0.0);
        let observation = apply_friction_impulse_measured(
            &mut staged_a,
            &mut staged_b,
            &contact,
            &impulse,
        )
        .unwrap();
        let mut staged_ledger = EnergyTransferLedger::new();
        let staged = promote_measured_friction_loss_to_heat(
            &mut staged_a,
            &mut staged_b,
            &observation,
            HeatPartition::equal(),
            &mut staged_ledger,
        )
        .unwrap();

        let mut reference_a = body(1, 1.0);
        let mut reference_b = body(2, 0.0);
        let mut reference_ledger = EnergyTransferLedger::new();
        let reference = apply_friction_impulse_with_heat(
            &mut reference_a,
            &mut reference_b,
            &contact,
            &impulse,
            HeatPartition::equal(),
            &mut reference_ledger,
        )
        .unwrap();

        assert_eq!(staged, reference);
        assert_eq!(staged_a.linear_velocity, reference_a.linear_velocity);
        assert_eq!(staged_b.linear_velocity, reference_b.linear_velocity);
        assert_eq!(staged_a.angular_velocity, reference_a.angular_velocity);
        assert_eq!(staged_b.angular_velocity, reference_b.angular_velocity);
        assert_eq!(staged_a.thermal, reference_a.thermal);
        assert_eq!(staged_b.thermal, reference_b.thermal);
        assert_eq!(staged_ledger, reference_ledger);
    }

    #[test]
    fn stale_observation_cannot_be_replayed_after_more_mechanical_work() {
        let contact = SVector::zeros();
        let impulse = SVector::from([0.5, 0.0, 0.0]);
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        let observation =
            apply_friction_impulse_measured(&mut a, &mut b, &contact, &impulse).unwrap();

        a.linear_velocity[0] += 0.25;
        let thermal_a = a.thermal;
        let thermal_b = b.thermal;
        let mut ledger = EnergyTransferLedger::new();
        let before_ledger = ledger.clone();

        assert_eq!(
            promote_measured_friction_loss_to_heat(
                &mut a,
                &mut b,
                &observation,
                HeatPartition::equal(),
                &mut ledger,
            ),
            Err(FrictionPromotionError::StaleMechanicalState)
        );
        assert_eq!(a.thermal, thermal_a);
        assert_eq!(b.thermal, thermal_b);
        assert_eq!(ledger, before_ledger);
    }

    #[test]
    fn injecting_observation_cannot_be_promoted() {
        let contact = SVector::zeros();
        let impulse = SVector::from([2.0, 0.0, 0.0]);
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        let observation =
            apply_friction_impulse_measured(&mut a, &mut b, &contact, &impulse).unwrap();
        let thermal_a = a.thermal;
        let thermal_b = b.thermal;
        let mut ledger = EnergyTransferLedger::new();

        assert_eq!(
            promote_measured_friction_loss_to_heat(
                &mut a,
                &mut b,
                &observation,
                HeatPartition::equal(),
                &mut ledger,
            ),
            Err(FrictionPromotionError::NonDissipativeObservation)
        );
        assert_eq!(a.thermal, thermal_a);
        assert_eq!(b.thermal, thermal_b);
        assert!(ledger.is_empty());
    }

    #[test]
    fn mismatched_identity_is_rejected_before_thermal_or_ledger_mutation() {
        let contact = SVector::zeros();
        let impulse = SVector::from([0.5, 0.0, 0.0]);
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        let observation =
            apply_friction_impulse_measured(&mut a, &mut b, &contact, &impulse).unwrap();

        let mut wrong_b = body(3, b.linear_velocity[0]);
        let mut ledger = EnergyTransferLedger::new();
        assert_eq!(
            promote_measured_friction_loss_to_heat(
                &mut a,
                &mut wrong_b,
                &observation,
                HeatPartition::equal(),
                &mut ledger,
            ),
            Err(FrictionPromotionError::ObservationIdentityMismatch)
        );
        assert!(ledger.is_empty());
    }

    #[test]
    fn off_center_observation_remains_ineligible_for_physical_promotion() {
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        a.transform.translation = Point::new([-1.0, 0.0, 0.0]);
        b.transform.translation = Point::new([1.0, 0.0, 0.0]);
        let contact = SVector::from([0.0, 0.5, 0.0]);
        let impulse = SVector::from([0.0, -0.1, 0.0]);
        let observation =
            apply_friction_impulse_measured(&mut a, &mut b, &contact, &impulse).unwrap();
        assert_eq!(
            observation.regime,
            FrictionEvidenceRegime::OffCenterUnqualified
        );

        let mut ledger = EnergyTransferLedger::new();
        assert_eq!(
            promote_measured_friction_loss_to_heat(
                &mut a,
                &mut b,
                &observation,
                HeatPartition::equal(),
                &mut ledger,
            ),
            Err(FrictionPromotionError::RequiresCenteredClosedDynamicPair)
        );
        assert!(ledger.is_empty());
    }
}
