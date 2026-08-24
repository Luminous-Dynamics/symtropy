// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Measured mechanical dissipation and audited conversion into sensible heat.
//!
//! This module deliberately does not estimate dissipated energy from impulse
//! magnitude. It measures the pair's kinetic energy before and after a supplied
//! friction impulse, rejects impulses that inject net kinetic energy, and routes
//! only the measured loss into thermal reservoirs.

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
    SameBody,
    MissingThermalState,
    NonDissipativeImpulse,
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
) -> Result<(), EnergyLedgerError> {
    if joules > ENERGY_EPSILON_J {
        ledger.record(source, destination, joules, EnergyTransferKind::Friction)?;
    }
    Ok(())
}

/// Apply a supplied friction impulse and convert the measured pair kinetic-energy
/// loss into sensible heat.
///
/// `impulse_on_b` is applied to body B; body A receives the equal and opposite
/// impulse. Angular impulses are generated about `contact_point` using the same
/// wedge-product convention as the world contact solver.
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
    if !impulse_on_b.iter().all(|value| value.is_finite())
        || !contact_point.iter().all(|value| value.is_finite())
    {
        return Err(DissipationError::NonFiniteImpulse);
    }
    HeatPartition::new(partition.fraction_to_a)?;
    let Some(initial_thermal_a) = body_a.thermal else {
        return Err(DissipationError::MissingThermalState);
    };
    let Some(initial_thermal_b) = body_b.thermal else {
        return Err(DissipationError::MissingThermalState);
    };

    let snapshot_a = MechanicalSnapshot::capture(body_a);
    let snapshot_b = MechanicalSnapshot::capture(body_b);
    let original_ledger = ledger.clone();

    let pre_a = body_a.kinetic_energy();
    let pre_b = body_b.kinetic_energy();

    integrator::apply_impulse(body_a, &(-*impulse_on_b));
    integrator::apply_impulse(body_b, impulse_on_b);

    let r_a = *contact_point - body_a.position();
    let r_b = *contact_point - body_b.position();
    let torque_a = Bivector::from_wedge(&(-*impulse_on_b), &r_a);
    let torque_b = Bivector::from_wedge(impulse_on_b, &r_b);
    integrator::apply_angular_impulse(body_a, &torque_a);
    integrator::apply_angular_impulse(body_b, &torque_b);

    let post_a = body_a.kinetic_energy();
    let post_b = body_b.kinetic_energy();
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

    let kinetic_a = kinetic_port(body_a);
    let kinetic_b = kinetic_port(body_b);
    let thermal_a = thermal_port(body_a);
    let thermal_b = thermal_port(body_b);

    // First represent any pure kinetic-energy transfer between bodies. What
    // remains on the losing side(s) is the measured dissipation budget.
    let mut residual_a = loss_a.max(0.0);
    let mut residual_b = loss_b.max(0.0);
    let mut next_ledger = ledger.clone();
    let ledger_result: Result<(), EnergyLedgerError> = (|| {
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
        if (residual_total - dissipated).abs() > 1e-10 * dissipated.max(1.0) {
            return Err(EnergyLedgerError::NonFiniteAuditEnergy);
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
        return Err(error.into());
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
    use crate::thermal::{ThermalBody, ThermalMaterial, ThermalState};
    use symtropy_math::Point;

    fn body(handle: usize, velocity_x: f64) -> RigidBody<3> {
        let mut body = RigidBody::dynamic_sphere(
            crate::body::BodyHandle(handle),
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
        // 0.125 J is transferred A kinetic -> B kinetic; 0.25 J is converted
        // to heat. Throughput therefore exceeds dissipation but remains auditable.
        assert!((ledger.total_transferred_joules() - 0.375).abs() < 1e-12);
    }

    #[test]
    fn energy_injecting_impulse_is_rejected_and_rolled_back() {
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
        let before_b = (b.linear_velocity, b.angular_velocity, b.thermal);
        let mut ledger = EnergyTransferLedger::new();

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
        assert!(ledger.is_empty());
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
}
