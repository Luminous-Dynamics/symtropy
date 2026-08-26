// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Measured mechanical dissipation and audited conversion into sensible heat.
//!
//! This module deliberately does not estimate dissipated energy from impulse
//! magnitude. It measures the pair's modeled kinetic energy before and after a
//! supplied friction impulse, rejects impulses that inject net kinetic energy,
//! and routes only the measured loss into thermal reservoirs.
//!
//! The currently validated coupling is intentionally centered. Off-center
//! impulses remain blocked until the engine's angular-velocity convention and
//! full anisotropic 3D inertia migration are complete. This prevents a known
//! rotational-model limitation from being laundered into apparently physical heat.

use nalgebra::SVector;
use symtropy_math::Bivector;

use crate::body::RigidBody;
use crate::energy::{
    EnergyForm, EnergyLedgerError, EnergyOwner, EnergyPort, EnergyTransferKind,
    EnergyTransferLedger,
};
use crate::integrator;
use crate::thermal::{ThermalBody, ThermalError};

const ENERGY_EPSILON_J: f64 = 1e-15;
const CENTERED_OFFSET_EPSILON_SQUARED: f64 = 1e-24;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HeatPartition {
    pub fraction_to_a: f64,
}

impl HeatPartition {
    pub fn new(fraction_to_a: f64) -> Result<Self, DissipationError> {
        if !fraction_to_a.is_finite() || !(0.0..=1.0).contains(&fraction_to_a) {
            return Err(DissipationError::InvalidHeatPartition);
        }
        Ok(Self { fraction_to_a })
    }

    pub fn equal() -> Self {
        Self { fraction_to_a: 0.5 }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FrictionHeatResult {
    pub kinetic_change_a_joules: f64,
    pub kinetic_change_b_joules: f64,
    pub dissipated_joules: f64,
    pub heat_to_a_joules: f64,
    pub heat_to_b_joules: f64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DissipationError {
    InvalidHeatPartition,
    NonFiniteImpulse,
    NonFiniteContactPoint,
    NonFiniteMechanicalState,
    NonFiniteKineticEnergy,
    SameBody,
    RequiresDynamicBodies,
    /// The current certified primitive excludes angular lever arms until the
    /// Rotor/Bivector convention and anisotropic inertia path are migrated.
    OffCenterImpulseNotValidated,
    MissingThermalState,
    NonDissipativeImpulse,
    /// The measured per-body kinetic changes could not be decomposed into the
    /// ledger's kinetic-transfer plus dissipation reservoirs within tolerance.
    LedgerStateMismatch,
    Thermal(ThermalError),
    Ledger(EnergyLedgerError),
}

impl From<ThermalError> for DissipationError {
    fn from(value: ThermalError) -> Self {
        Self::Thermal(value)
    }
}

impl From<EnergyLedgerError> for DissipationError {
    fn from(value: EnergyLedgerError) -> Self {
        Self::Ledger(value)
    }
}

#[derive(Copy, Clone)]
struct MechanicalSnapshot<const D: usize> {
    linear_velocity: SVector<f64, D>,
    angular_velocity: Bivector<D>,
    thermal: Option<ThermalBody>,
}

impl<const D: usize> MechanicalSnapshot<D> {
    fn capture(body: &RigidBody<D>) -> Self {
        Self {
            linear_velocity: body.linear_velocity,
            angular_velocity: body.angular_velocity,
            thermal: body.thermal,
        }
    }

    fn restore(self, body: &mut RigidBody<D>) {
        body.linear_velocity = self.linear_velocity;
        body.angular_velocity = self.angular_velocity;
        body.thermal = self.thermal;
    }
}

fn rollback<const D: usize>(
    snapshot_a: MechanicalSnapshot<D>,
    body_a: &mut RigidBody<D>,
    snapshot_b: MechanicalSnapshot<D>,
    body_b: &mut RigidBody<D>,
    original_ledger: &EnergyTransferLedger,
    ledger: &mut EnergyTransferLedger,
) {
    snapshot_a.restore(body_a);
    snapshot_b.restore(body_b);
    *ledger = original_ledger.clone();
}

fn kinetic_port<const D: usize>(body: &RigidBody<D>) -> EnergyPort {
    EnergyPort::new(EnergyOwner::Body(body.handle), EnergyForm::Kinetic)
}

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
) -> Result<(), DissipationError> {
    if !joules.is_finite() {
        return Err(DissipationError::Ledger(EnergyLedgerError::NonFiniteEnergy));
    }
    // Zero is the only quiet case. Any positive state transfer, however small,
    // must have a matching ledger entry so state and accounting cannot diverge.
    if joules > 0.0 {
        ledger.record(source, destination, joules, EnergyTransferKind::Friction)?;
    }
    Ok(())
}

fn mechanical_state_is_finite<const D: usize>(body: &RigidBody<D>) -> bool {
    body.position().iter().all(|value| value.is_finite())
        && body.linear_velocity.iter().all(|value| value.is_finite())
        && body.angular_velocity.is_finite()
        && body.mass.is_finite()
        && body.inv_mass.is_finite()
        && body.inertia.iter().all(|value| value.is_finite())
        && body.inv_inertia.iter().all(|value| value.is_finite())
}

/// Apply a supplied centered friction impulse and convert the measured pair
/// kinetic-energy loss into sensible heat.
///
/// `impulse_on_b` is applied to body B; body A receives the equal and opposite
/// impulse. For the current validated primitive, `contact_point` must coincide
/// with both body centers within a tiny numerical tolerance. This deliberately
/// excludes angular impulse until the engine's P0 angular convention/inertia
/// migration is complete.
///
/// `RigidBody::kinetic_energy()` is the authoritative modeled energy used here.
/// Its current rotational term uses the engine's scalar-mean inertia model; the
/// centered validation cases do not alter angular velocity, so they do not claim
/// certified asymmetric-body rotational dissipation.
///
/// The operation is transactional across velocities, thermal state, and ledger.
pub fn apply_friction_impulse_with_heat<const D: usize>(
    body_a: &mut RigidBody<D>,
    body_b: &mut RigidBody<D>,
    contact_point: &SVector<f64, D>,
    impulse_on_b: &SVector<f64, D>,
    partition: HeatPartition,
    ledger: &mut EnergyTransferLedger,
) -> Result<FrictionHeatResult, DissipationError> {
    if body_a.handle == body_b.handle {
        return Err(DissipationError::SameBody);
    }
    if !impulse_on_b.iter().all(|value| value.is_finite()) {
        return Err(DissipationError::NonFiniteImpulse);
    }
    if !contact_point.iter().all(|value| value.is_finite()) {
        return Err(DissipationError::NonFiniteContactPoint);
    }
    HeatPartition::new(partition.fraction_to_a)?;

    if !body_a.is_dynamic() || !body_b.is_dynamic() {
        // Static/kinematic contacts exchange momentum/work with reservoirs that
        // are outside this closed two-dynamic-body accounting model.
        return Err(DissipationError::RequiresDynamicBodies);
    }
    if !mechanical_state_is_finite(body_a) || !mechanical_state_is_finite(body_b) {
        return Err(DissipationError::NonFiniteMechanicalState);
    }

    let Some(initial_thermal_a) = body_a.thermal else {
        return Err(DissipationError::MissingThermalState);
    };
    let Some(initial_thermal_b) = body_b.thermal else {
        return Err(DissipationError::MissingThermalState);
    };
    // Public thermal fields can be mutated after construction. Revalidate both
    // reservoirs before any mechanical state is touched.
    initial_thermal_a.validate()?;
    initial_thermal_b.validate()?;

    let pre_a = body_a.kinetic_energy();
    let pre_b = body_b.kinetic_energy();
    if !pre_a.is_finite() || !pre_b.is_finite() || pre_a < 0.0 || pre_b < 0.0 {
        return Err(DissipationError::NonFiniteKineticEnergy);
    }

    let r_a = *contact_point - body_a.position();
    let r_b = *contact_point - body_b.position();
    if r_a.norm_squared() > CENTERED_OFFSET_EPSILON_SQUARED
        || r_b.norm_squared() > CENTERED_OFFSET_EPSILON_SQUARED
    {
        return Err(DissipationError::OffCenterImpulseNotValidated);
    }

    let snapshot_a = MechanicalSnapshot::capture(body_a);
    let snapshot_b = MechanicalSnapshot::capture(body_b);
    let original_ledger = ledger.clone();

    integrator::apply_impulse(body_a, &(-*impulse_on_b));
    integrator::apply_impulse(body_b, impulse_on_b);

    let post_a = body_a.kinetic_energy();
    let post_b = body_b.kinetic_energy();
    if !post_a.is_finite() || !post_b.is_finite() || post_a < 0.0 || post_b < 0.0 {
        rollback(
            snapshot_a,
            body_a,
            snapshot_b,
            body_b,
            &original_ledger,
            ledger,
        );
        return Err(DissipationError::NonFiniteKineticEnergy);
    }

    let change_a = post_a - pre_a;
    let change_b = post_b - pre_b;
    let loss_a = -change_a;
    let loss_b = -change_b;
    let dissipated = loss_a + loss_b;

    if !dissipated.is_finite() || dissipated <= ENERGY_EPSILON_J {
        rollback(
            snapshot_a,
            body_a,
            snapshot_b,
            body_b,
            &original_ledger,
            ledger,
        );
        return Err(DissipationError::NonDissipativeImpulse);
    }

    let heat_a = dissipated * partition.fraction_to_a;
    let heat_b = dissipated - heat_a;
    if !heat_a.is_finite() || !heat_b.is_finite() {
        rollback(
            snapshot_a,
            body_a,
            snapshot_b,
            body_b,
            &original_ledger,
            ledger,
        );
        return Err(DissipationError::LedgerStateMismatch);
    }

    let kinetic_a = kinetic_port(body_a);
    let kinetic_b = kinetic_port(body_b);
    let thermal_a = thermal_port(body_a);
    let thermal_b = thermal_port(body_b);

    // First represent any pure kinetic-energy transfer between bodies. What
    // remains on the losing side(s) is the measured dissipation budget.
    let mut residual_a = loss_a.max(0.0);
    let mut residual_b = loss_b.max(0.0);
    let mut next_ledger = ledger.clone();
    let ledger_result: Result<(), DissipationError> = (|| {
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
            || (residual_total - dissipated).abs() > 1e-12 * dissipated.max(1.0)
        {
            return Err(DissipationError::LedgerStateMismatch);
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
        Ok(())
    })();

    if let Err(error) = ledger_result {
        rollback(
            snapshot_a,
            body_a,
            snapshot_b,
            body_b,
            &original_ledger,
            ledger,
        );
        return Err(error);
    }

    let mut next_thermal_a = initial_thermal_a;
    let mut next_thermal_b = initial_thermal_b;
    if let Err(error) = next_thermal_a.add_heat_joules(heat_a) {
        rollback(
            snapshot_a,
            body_a,
            snapshot_b,
            body_b,
            &original_ledger,
            ledger,
        );
        return Err(error.into());
    }
    if let Err(error) = next_thermal_b.add_heat_joules(heat_b) {
        rollback(
            snapshot_a,
            body_a,
            snapshot_b,
            body_b,
            &original_ledger,
            ledger,
        );
        return Err(error.into());
    }

    // Reconcile the staged ledger against the exact modeled state deltas before
    // committing either side. This protects against decomposition drift and tiny
    // omitted transfers.
    let kinetic_residual_a = change_a - (next_ledger.net_change_for(kinetic_a)
        - original_ledger.net_change_for(kinetic_a));
    let kinetic_residual_b = change_b - (next_ledger.net_change_for(kinetic_b)
        - original_ledger.net_change_for(kinetic_b));
    let thermal_residual_a = heat_a - (next_ledger.net_change_for(thermal_a)
        - original_ledger.net_change_for(thermal_a));
    let thermal_residual_b = heat_b - (next_ledger.net_change_for(thermal_b)
        - original_ledger.net_change_for(thermal_b));
    let max_residual = kinetic_residual_a
        .abs()
        .max(kinetic_residual_b.abs())
        .max(thermal_residual_a.abs())
        .max(thermal_residual_b.abs());
    if !max_residual.is_finite() || max_residual > 1e-12 * dissipated.max(1.0) {
        rollback(
            snapshot_a,
            body_a,
            snapshot_b,
            body_b,
            &original_ledger,
            ledger,
        );
        return Err(DissipationError::LedgerStateMismatch);
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
    use crate::thermal::{ThermalBody, ThermalMaterial, ThermalState};
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

    fn seed_ledger() -> EnergyTransferLedger {
        let mut ledger = EnergyTransferLedger::new();
        ledger
            .record(
                EnergyPort::new(EnergyOwner::External(99), EnergyForm::ThermalSensible),
                EnergyPort::new(EnergyOwner::Body(BodyHandle(50)), EnergyForm::ThermalSensible),
                3.0,
                EnergyTransferKind::ExternalHeat,
            )
            .unwrap();
        ledger
    }

    #[test]
    fn measured_loss_becomes_heat_and_closes_energy_budget() {
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        let mut ledger = EnergyTransferLedger::new();
        let initial = a.kinetic_energy()
            + b.kinetic_energy()
            + a.thermal_energy_joules(0.0).unwrap()
            + b.thermal_energy_joules(0.0).unwrap();

        let result = apply_friction_impulse_with_heat(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            HeatPartition::equal(),
            &mut ledger,
        )
        .unwrap();

        assert!((result.dissipated_joules - 0.25).abs() < 1e-12);
        assert!((result.heat_to_a_joules - 0.125).abs() < 1e-12);
        assert!((result.heat_to_b_joules - 0.125).abs() < 1e-12);

        let final_energy = a.kinetic_energy()
            + b.kinetic_energy()
            + a.thermal_energy_joules(0.0).unwrap()
            + b.thermal_energy_joules(0.0).unwrap();
        assert!((final_energy - initial).abs() < 1e-9);
        assert_eq!(ledger.net_external_joules(), 0.0);
        assert!((ledger.total_transferred_joules() - 0.375).abs() < 1e-12);
    }

    #[test]
    fn energy_injecting_impulse_rolls_back_over_existing_history() {
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
        let before_b = (b.linear_velocity, b.angular_velocity, b.thermal);
        let mut ledger = seed_ledger();
        let before_ledger = ledger.clone();

        let error = apply_friction_impulse_with_heat(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([2.0, 0.0, 0.0]),
            HeatPartition::equal(),
            &mut ledger,
        )
        .unwrap_err();

        assert_eq!(error, DissipationError::NonDissipativeImpulse);
        assert_eq!((a.linear_velocity, a.angular_velocity, a.thermal), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity, b.thermal), before_b);
        assert_eq!(ledger, before_ledger);
    }

    #[test]
    fn invalid_thermal_state_is_rejected_before_mechanical_mutation() {
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        b.thermal.as_mut().unwrap().material.emissivity = 1.5;
        let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
        let before_b = (b.linear_velocity, b.angular_velocity, b.thermal);
        let mut ledger = seed_ledger();
        let before_ledger = ledger.clone();

        let error = apply_friction_impulse_with_heat(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            HeatPartition::equal(),
            &mut ledger,
        )
        .unwrap_err();

        assert_eq!(
            error,
            DissipationError::Thermal(ThermalError::InvalidEmissivity)
        );
        assert_eq!((a.linear_velocity, a.angular_velocity, a.thermal), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity, b.thermal), before_b);
        assert_eq!(ledger, before_ledger);
    }

    #[test]
    fn missing_thermal_state_is_rejected_before_mutation() {
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        b.clear_thermal();
        let before_a = a.linear_velocity;
        let before_b = b.linear_velocity;
        let mut ledger = EnergyTransferLedger::new();

        let error = apply_friction_impulse_with_heat(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            HeatPartition::equal(),
            &mut ledger,
        )
        .unwrap_err();

        assert_eq!(error, DissipationError::MissingThermalState);
        assert_eq!(a.linear_velocity, before_a);
        assert_eq!(b.linear_velocity, before_b);
        assert!(ledger.is_empty());
    }

    #[test]
    fn off_center_impulse_is_explicitly_outside_current_validity_envelope() {
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
        let before_b = (b.linear_velocity, b.angular_velocity, b.thermal);
        let mut ledger = seed_ledger();
        let before_ledger = ledger.clone();

        assert_eq!(
            apply_friction_impulse_with_heat(
                &mut a,
                &mut b,
                &SVector::from([0.0, 0.1, 0.0]),
                &SVector::from([0.5, 0.0, 0.0]),
                HeatPartition::equal(),
                &mut ledger,
            ),
            Err(DissipationError::OffCenterImpulseNotValidated)
        );
        assert_eq!((a.linear_velocity, a.angular_velocity, a.thermal), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity, b.thermal), before_b);
        assert_eq!(ledger, before_ledger);
    }

    #[test]
    fn non_finite_kinetic_state_is_rejected_before_mutation() {
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        a.mass = f64::MAX;
        a.linear_velocity[0] = f64::MAX;
        let before_a = a.linear_velocity;
        let before_b = b.linear_velocity;
        let mut ledger = EnergyTransferLedger::new();

        assert_eq!(
            apply_friction_impulse_with_heat(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::from([0.5, 0.0, 0.0]),
                HeatPartition::equal(),
                &mut ledger,
            ),
            Err(DissipationError::NonFiniteKineticEnergy)
        );
        assert_eq!(a.linear_velocity, before_a);
        assert_eq!(b.linear_velocity, before_b);
        assert!(ledger.is_empty());
    }
}
