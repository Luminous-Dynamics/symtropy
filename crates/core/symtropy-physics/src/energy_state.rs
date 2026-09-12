// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Reconcile measured world energy state against the explicit transfer ledger.
//!
//! Total-energy closure can hide compensating mistakes. This module therefore
//! compares each tracked internal reservoir's measured change with the ledger's
//! claimed net change, preserves whether a reservoir exists at each endpoint,
//! and reports internal ledger ports that are not yet represented by the state
//! snapshot model.
//!
//! Reservoir presence is part of the evidence contract. A missing reservoir is
//! not numerically equivalent to a represented reservoir containing `0 J`.
//! Ledger reductions used by reconciliation are overflow-aware: individually
//! valid finite transfers do not become valid evidence if their deterministic
//! aggregate cannot be represented as a finite `f64`.

use serde::{Deserialize, Serialize};

use crate::energy::{EnergyForm, EnergyOwner, EnergyPort, EnergyTransferLedger};
use crate::energy_checked::EnergyTransferLedgerCheckedExt;
use crate::thermal::{ThermalError, ThermalState};
use crate::world::PhysicsWorld;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReservoirEnergy {
    pub port: EnergyPort,
    pub joules: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnergyStateSnapshot {
    pub thermal_reference_temperature_kelvin: f64,
    pub reservoirs: Vec<ReservoirEnergy>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReservoirPresenceChangeKind {
    Appeared,
    Disappeared,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReservoirPresenceChange {
    pub port: EnergyPort,
    pub kind: ReservoirPresenceChangeKind,
}

impl EnergyStateSnapshot {
    /// Capture the energy forms currently represented directly in the core world:
    /// body kinetic energy, body gravitational potential energy, and attached
    /// body sensible thermal energy.
    pub fn capture<const D: usize>(
        world: &PhysicsWorld<D>,
        thermal_reference_temperature_kelvin: f64,
    ) -> Result<Self, EnergyStateAuditError> {
        ThermalState::new(thermal_reference_temperature_kelvin)?;

        let mut bodies: Vec<_> = world.bodies.iter().collect();
        bodies.sort_by_key(|body| body.handle);

        let mut reservoirs = Vec::with_capacity(bodies.len() * 3);
        for body in bodies {
            if body.is_dynamic() {
                let kinetic_port =
                    EnergyPort::new(EnergyOwner::Body(body.handle), EnergyForm::Kinetic);
                push_reservoir(&mut reservoirs, kinetic_port, body.kinetic_energy())?;

                let potential_port = EnergyPort::new(
                    EnergyOwner::Body(body.handle),
                    EnergyForm::GravitationalPotential,
                );
                let potential_joules = -body.mass * world.gravity.dot(&body.position());
                push_reservoir(&mut reservoirs, potential_port, potential_joules)?;
            }

            if let Some(thermal) = body.thermal {
                let thermal_port = EnergyPort::new(
                    EnergyOwner::Body(body.handle),
                    EnergyForm::ThermalSensible,
                );
                let sensible_joules =
                    thermal.sensible_energy_joules(thermal_reference_temperature_kelvin)?;
                push_reservoir(&mut reservoirs, thermal_port, sensible_joules)?;
            }
        }

        let snapshot = Self {
            thermal_reference_temperature_kelvin,
            reservoirs,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    /// Revalidate evidence loaded from storage or mutated after capture.
    pub fn validate(&self) -> Result<(), EnergyStateAuditError> {
        ThermalState::new(self.thermal_reference_temperature_kelvin)?;

        for (index, entry) in self.reservoirs.iter().enumerate() {
            if !entry.joules.is_finite() {
                return Err(EnergyStateAuditError::NonFiniteReservoir(entry.port));
            }
            if self.reservoirs[..index]
                .iter()
                .any(|previous| previous.port == entry.port)
            {
                return Err(EnergyStateAuditError::DuplicateReservoir(entry.port));
            }
        }

        self.total_tracked_internal_joules()?;
        Ok(())
    }

    pub fn energy_for(&self, port: EnergyPort) -> Option<f64> {
        self.reservoirs
            .iter()
            .find(|entry| entry.port == port)
            .map(|entry| entry.joules)
    }

    pub fn total_tracked_internal_joules(&self) -> Result<f64, EnergyStateAuditError> {
        let mut total = 0.0;
        for entry in &self.reservoirs {
            if !entry.joules.is_finite() {
                return Err(EnergyStateAuditError::NonFiniteReservoir(entry.port));
            }
            total += entry.joules;
            if !total.is_finite() {
                return Err(EnergyStateAuditError::NonFiniteTrackedTotal);
            }
        }
        Ok(total)
    }

    pub fn reconcile(
        &self,
        later: &Self,
        ledger: &EnergyTransferLedger,
    ) -> Result<EnergyReconciliationAudit, EnergyStateAuditError> {
        self.validate()?;
        later.validate()?;

        if self.thermal_reference_temperature_kelvin != later.thermal_reference_temperature_kelvin {
            return Err(EnergyStateAuditError::MismatchedThermalReference);
        }

        let mut ports: Vec<EnergyPort> = self.reservoirs.iter().map(|entry| entry.port).collect();
        for entry in &later.reservoirs {
            if !ports.contains(&entry.port) {
                ports.push(entry.port);
            }
        }

        let mut entries = Vec::with_capacity(ports.len());
        let mut reservoir_presence_changes = Vec::new();
        for port in &ports {
            let initial = self.energy_for(*port);
            let final_energy = later.energy_for(*port);
            let ledger_delta = ledger
                .net_change_for_checked(*port)
                .map_err(|_| EnergyStateAuditError::NonFiniteLedgerDelta(*port))?;

            let (measured_delta, residual) = match (initial, final_energy) {
                (Some(initial), Some(final_energy)) => {
                    let measured_delta = final_energy - initial;
                    let residual = measured_delta - ledger_delta;
                    if !measured_delta.is_finite() || !residual.is_finite() {
                        return Err(EnergyStateAuditError::NonFiniteResidual(*port));
                    }
                    (Some(measured_delta), Some(residual))
                }
                (None, Some(_)) => {
                    reservoir_presence_changes.push(ReservoirPresenceChange {
                        port: *port,
                        kind: ReservoirPresenceChangeKind::Appeared,
                    });
                    (None, None)
                }
                (Some(_), None) => {
                    reservoir_presence_changes.push(ReservoirPresenceChange {
                        port: *port,
                        kind: ReservoirPresenceChangeKind::Disappeared,
                    });
                    (None, None)
                }
                (None, None) => unreachable!("ports are the union of both snapshots"),
            };

            entries.push(ReservoirReconciliation {
                port: *port,
                initial_joules: initial,
                final_joules: final_energy,
                measured_delta_joules: measured_delta,
                ledger_delta_joules: ledger_delta,
                residual_joules: residual,
            });
        }

        let mut untracked_ledger_ports = Vec::new();
        for transfer in ledger.entries() {
            for port in [transfer.source, transfer.destination] {
                if port.owner.is_external() {
                    continue;
                }
                if !ports.contains(&port) && !untracked_ledger_ports.contains(&port) {
                    untracked_ledger_ports.push(port);
                }
            }
        }

        let initial_total = self.total_tracked_internal_joules()?;
        let final_total = later.total_tracked_internal_joules()?;
        let net_external_joules = ledger
            .net_external_joules_checked()
            .map_err(|_| EnergyStateAuditError::NonFiniteBoundaryAudit)?;
        let observed_delta = final_total - initial_total;
        let total_closure_error_joules = observed_delta - net_external_joules;
        if !observed_delta.is_finite() || !total_closure_error_joules.is_finite() {
            return Err(EnergyStateAuditError::NonFiniteBoundaryAudit);
        }

        Ok(EnergyReconciliationAudit {
            entries,
            reservoir_presence_changes,
            untracked_ledger_ports,
            initial_total_joules: initial_total,
            final_total_joules: final_total,
            net_external_joules,
            total_closure_error_joules,
        })
    }
}

fn push_reservoir(
    reservoirs: &mut Vec<ReservoirEnergy>,
    port: EnergyPort,
    joules: f64,
) -> Result<(), EnergyStateAuditError> {
    if !joules.is_finite() {
        return Err(EnergyStateAuditError::NonFiniteReservoir(port));
    }
    if reservoirs.iter().any(|entry| entry.port == port) {
        return Err(EnergyStateAuditError::DuplicateReservoir(port));
    }
    reservoirs.push(ReservoirEnergy { port, joules });
    Ok(())
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReservoirReconciliation {
    pub port: EnergyPort,
    pub initial_joules: Option<f64>,
    pub final_joules: Option<f64>,
    /// `None` means reservoir presence changed, so a numeric state delta would
    /// be semantically misleading until lifecycle provenance is modeled.
    pub measured_delta_joules: Option<f64>,
    pub ledger_delta_joules: f64,
    /// `None` means the reservoir appeared or disappeared during the interval.
    pub residual_joules: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnergyReconciliationAudit {
    pub entries: Vec<ReservoirReconciliation>,
    pub reservoir_presence_changes: Vec<ReservoirPresenceChange>,
    pub untracked_ledger_ports: Vec<EnergyPort>,
    pub initial_total_joules: f64,
    pub final_total_joules: f64,
    pub net_external_joules: f64,
    pub total_closure_error_joules: f64,
}

impl EnergyReconciliationAudit {
    pub fn max_abs_residual_joules(&self) -> f64 {
        self.entries
            .iter()
            .filter_map(|entry| entry.residual_joules)
            .map(f64::abs)
            .fold(0.0_f64, f64::max)
    }

    pub fn unexplained_reservoir_count(&self, tolerance_joules: f64) -> usize {
        if !tolerance_joules.is_finite() || tolerance_joules < 0.0 {
            return self.entries.len();
        }
        self.entries
            .iter()
            .filter(|entry| {
                entry
                    .residual_joules
                    .is_none_or(|residual| residual.abs() > tolerance_joules)
            })
            .count()
    }

    pub fn has_stable_reservoir_set(&self) -> bool {
        self.reservoir_presence_changes.is_empty()
    }

    pub fn has_complete_state_representation(&self) -> bool {
        self.has_stable_reservoir_set() && self.untracked_ledger_ports.is_empty()
    }

    pub fn fully_reconciled(&self, tolerance_joules: f64) -> bool {
        tolerance_joules.is_finite()
            && tolerance_joules >= 0.0
            && self.has_complete_state_representation()
            && self
                .entries
                .iter()
                .all(|entry| entry.residual_joules.is_some_and(|r| r.abs() <= tolerance_joules))
            && self.total_closure_error_joules.abs() <= tolerance_joules
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EnergyStateAuditError {
    Thermal(ThermalError),
    NonFiniteReservoir(EnergyPort),
    DuplicateReservoir(EnergyPort),
    NonFiniteTrackedTotal,
    MismatchedThermalReference,
    NonFiniteLedgerDelta(EnergyPort),
    NonFiniteResidual(EnergyPort),
    NonFiniteBoundaryAudit,
}

impl From<ThermalError> for EnergyStateAuditError {
    fn from(value: ThermalError) -> Self {
        Self::Thermal(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{BodyHandle, RigidBody};
    use crate::energy::{EnergyTransferKind, EnergyTransferLedger};
    use crate::external_heat::exchange_external_heat_audited;
    use crate::thermal::{ThermalBody, ThermalMaterial, ThermalState};
    use nalgebra::SVector;
    use symtropy_math::Point;

    fn thermal_body(temp_k: f64) -> ThermalBody {
        ThermalBody::new(
            ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
            ThermalState::new(temp_k).unwrap(),
            1.0,
        )
        .unwrap()
    }

    fn world() -> (PhysicsWorld<3>, BodyHandle) {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        world
            .body_mut(handle)
            .unwrap()
            .set_thermal(thermal_body(300.0));
        (world, handle)
    }

    #[test]
    fn audited_external_heat_reconciles_per_reservoir() {
        let (mut world, handle) = world();
        let before = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
        let mut ledger = EnergyTransferLedger::new();
        exchange_external_heat_audited(
            handle,
            world.body_mut(handle).unwrap(),
            1_000.0,
            7,
            &mut ledger,
        )
        .unwrap();
        let after = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
        let audit = before.reconcile(&after, &ledger).unwrap();

        assert!(audit.fully_reconciled(1e-10));
        assert!(audit.reservoir_presence_changes.is_empty());
        assert!(audit.untracked_ledger_ports.is_empty());
        assert!(audit.max_abs_residual_joules() <= 1e-10);
    }

    #[test]
    fn direct_untracked_heat_is_exposed_as_residual() {
        let (mut world, handle) = world();
        let before = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
        world.body_mut(handle).unwrap().add_heat_joules(500.0).unwrap();
        let after = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
        let audit = before
            .reconcile(&after, &EnergyTransferLedger::new())
            .unwrap();

        assert!((audit.max_abs_residual_joules() - 500.0).abs() < 1e-10);
        assert!(!audit.fully_reconciled(1e-10));
    }

    #[test]
    fn zero_energy_reservoir_appearance_is_not_numeric_zero() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        let before = EnergyStateSnapshot::capture(&world, 0.0).unwrap();

        world.body_mut(handle).unwrap().set_thermal(thermal_body(0.0));
        let after = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
        let audit = before
            .reconcile(&after, &EnergyTransferLedger::new())
            .unwrap();
        let thermal_port = EnergyPort::new(
            EnergyOwner::Body(handle),
            EnergyForm::ThermalSensible,
        );

        assert_eq!(audit.total_closure_error_joules, 0.0);
        assert_eq!(
            audit.reservoir_presence_changes,
            vec![ReservoirPresenceChange {
                port: thermal_port,
                kind: ReservoirPresenceChangeKind::Appeared,
            }]
        );
        let entry = audit
            .entries
            .iter()
            .find(|entry| entry.port == thermal_port)
            .unwrap();
        assert_eq!(entry.initial_joules, None);
        assert_eq!(entry.final_joules, Some(0.0));
        assert_eq!(entry.measured_delta_joules, None);
        assert_eq!(entry.residual_joules, None);
        assert!(!audit.fully_reconciled(0.0));
    }

    #[test]
    fn stable_zero_energy_reservoir_reconciles() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        world.body_mut(handle).unwrap().set_thermal(thermal_body(0.0));
        let before = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
        let after = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
        let audit = before
            .reconcile(&after, &EnergyTransferLedger::new())
            .unwrap();

        assert!(audit.reservoir_presence_changes.is_empty());
        assert!(audit.fully_reconciled(0.0));
    }

    #[test]
    fn reservoir_disappearance_is_explicit() {
        let (mut world, handle) = world();
        let before = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
        world.body_mut(handle).unwrap().clear_thermal();
        let after = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
        let audit = before
            .reconcile(&after, &EnergyTransferLedger::new())
            .unwrap();

        assert!(audit.reservoir_presence_changes.iter().any(|change| {
            change.port
                == EnergyPort::new(
                    EnergyOwner::Body(handle),
                    EnergyForm::ThermalSensible,
                )
                && change.kind == ReservoirPresenceChangeKind::Disappeared
        }));
        assert!(!audit.fully_reconciled(1e-12));
    }

    #[test]
    fn capture_rejects_non_finite_mechanical_reservoir() {
        let (mut world, handle) = world();
        world.body_mut(handle).unwrap().linear_velocity[0] = f64::INFINITY;
        let kinetic_port = EnergyPort::new(EnergyOwner::Body(handle), EnergyForm::Kinetic);

        assert_eq!(
            EnergyStateSnapshot::capture(&world, 0.0),
            Err(EnergyStateAuditError::NonFiniteReservoir(kinetic_port))
        );
    }

    #[test]
    fn capture_rejects_duplicate_reservoir_identity() {
        let (mut world, handle) = world();
        world.bodies.push(RigidBody::dynamic_sphere(
            handle,
            Point::new([1.0, 0.0, 0.0]),
            0.5,
            1.0,
        ));
        let kinetic_port = EnergyPort::new(EnergyOwner::Body(handle), EnergyForm::Kinetic);

        assert_eq!(
            EnergyStateSnapshot::capture(&world, 0.0),
            Err(EnergyStateAuditError::DuplicateReservoir(kinetic_port))
        );
    }

    #[test]
    fn reconcile_revalidates_mutated_snapshot() {
        let (world, handle) = world();
        let mut before = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
        let later = before.clone();
        let kinetic_port = EnergyPort::new(EnergyOwner::Body(handle), EnergyForm::Kinetic);
        before
            .reservoirs
            .iter_mut()
            .find(|entry| entry.port == kinetic_port)
            .unwrap()
            .joules = f64::NAN;

        assert_eq!(
            before.reconcile(&later, &EnergyTransferLedger::new()),
            Err(EnergyStateAuditError::NonFiniteReservoir(kinetic_port))
        );
    }

    #[test]
    fn ledger_ports_without_state_representation_are_reported() {
        let (world, handle) = world();
        let before = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
        let mut ledger = EnergyTransferLedger::new();
        ledger
            .record(
                EnergyPort::new(EnergyOwner::Body(handle), EnergyForm::Chemical),
                EnergyPort::new(EnergyOwner::Body(handle), EnergyForm::ThermalSensible),
                10.0,
                EnergyTransferKind::ChemicalReaction,
            )
            .unwrap();
        let audit = before.reconcile(&before, &ledger).unwrap();

        assert_eq!(audit.untracked_ledger_ports.len(), 1);
        assert_eq!(
            audit.untracked_ledger_ports[0],
            EnergyPort::new(EnergyOwner::Body(handle), EnergyForm::Chemical)
        );
        assert!(!audit.fully_reconciled(1e-12));
    }

    #[test]
    fn reconcile_rejects_tracked_reservoir_aggregate_overflow() {
        let (world, handle) = world();
        let snapshot = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
        let thermal = EnergyPort::new(
            EnergyOwner::Body(handle),
            EnergyForm::ThermalSensible,
        );
        let external = EnergyPort::new(EnergyOwner::External(99), EnergyForm::Electrical);
        let mut ledger = EnergyTransferLedger::new();
        for _ in 0..2 {
            ledger
                .record(
                    external,
                    thermal,
                    f64::MAX,
                    EnergyTransferKind::ExternalWork,
                )
                .unwrap();
        }

        assert_eq!(
            snapshot.reconcile(&snapshot, &ledger),
            Err(EnergyStateAuditError::NonFiniteLedgerDelta(thermal))
        );
    }

    #[test]
    fn reconcile_rejects_boundary_aggregate_overflow_from_untracked_internal_port() {
        let (world, handle) = world();
        let snapshot = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
        let chemical = EnergyPort::new(EnergyOwner::Body(handle), EnergyForm::Chemical);
        let external = EnergyPort::new(EnergyOwner::External(100), EnergyForm::Chemical);
        let mut ledger = EnergyTransferLedger::new();
        for _ in 0..2 {
            ledger
                .record(
                    external,
                    chemical,
                    f64::MAX,
                    EnergyTransferKind::ChemicalReaction,
                )
                .unwrap();
        }

        assert_eq!(
            snapshot.reconcile(&snapshot, &ledger),
            Err(EnergyStateAuditError::NonFiniteBoundaryAudit)
        );
    }
}
