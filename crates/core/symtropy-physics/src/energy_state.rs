// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Reconcile measured world energy state against the explicit transfer ledger.
//!
//! Total-energy closure can hide compensating mistakes. This module therefore
//! compares each tracked internal reservoir's measured change with the ledger's
//! claimed net change and reports any internal ledger ports that are not yet
//! represented by the snapshot model.

use serde::{Deserialize, Serialize};

use crate::energy::{EnergyForm, EnergyOwner, EnergyPort, EnergyTransferLedger};
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

impl EnergyStateSnapshot {
    /// Capture the energy forms currently represented directly in the core world:
    /// body kinetic energy, body gravitational potential energy, and attached
    /// body sensible thermal energy.
    pub fn capture<const D: usize>(
        world: &PhysicsWorld<D>,
        thermal_reference_temperature_kelvin: f64,
    ) -> Result<Self, ThermalError> {
        // Reuse the thermal state's validation contract even when the world has
        // no thermal bodies, so an invalid reference cannot enter evidence data.
        ThermalState::new(thermal_reference_temperature_kelvin)?;

        let mut bodies: Vec<_> = world.bodies.iter().collect();
        bodies.sort_by_key(|body| body.handle);

        let mut reservoirs = Vec::with_capacity(bodies.len() * 3);
        for body in bodies {
            if body.is_dynamic() {
                reservoirs.push(ReservoirEnergy {
                    port: EnergyPort::new(EnergyOwner::Body(body.handle), EnergyForm::Kinetic),
                    joules: body.kinetic_energy(),
                });
                reservoirs.push(ReservoirEnergy {
                    port: EnergyPort::new(
                        EnergyOwner::Body(body.handle),
                        EnergyForm::GravitationalPotential,
                    ),
                    joules: -body.mass * world.gravity.dot(&body.position()),
                });
            }

            if let Some(thermal) = body.thermal {
                reservoirs.push(ReservoirEnergy {
                    port: EnergyPort::new(
                        EnergyOwner::Body(body.handle),
                        EnergyForm::ThermalSensible,
                    ),
                    joules: thermal
                        .sensible_energy_joules(thermal_reference_temperature_kelvin)?,
                });
            }
        }

        Ok(Self {
            thermal_reference_temperature_kelvin,
            reservoirs,
        })
    }

    pub fn energy_for(&self, port: EnergyPort) -> Option<f64> {
        self.reservoirs
            .iter()
            .find(|entry| entry.port == port)
            .map(|entry| entry.joules)
    }

    pub fn total_tracked_internal_joules(&self) -> f64 {
        self.reservoirs.iter().map(|entry| entry.joules).sum()
    }

    pub fn reconcile(
        &self,
        later: &Self,
        ledger: &EnergyTransferLedger,
    ) -> Result<EnergyReconciliationAudit, EnergyStateAuditError> {
        if self.thermal_reference_temperature_kelvin.to_bits()
            != later.thermal_reference_temperature_kelvin.to_bits()
        {
            return Err(EnergyStateAuditError::MismatchedThermalReference);
        }

        let mut ports: Vec<EnergyPort> = self.reservoirs.iter().map(|entry| entry.port).collect();
        for entry in &later.reservoirs {
            if !ports.contains(&entry.port) {
                ports.push(entry.port);
            }
        }

        let mut entries = Vec::with_capacity(ports.len());
        for port in &ports {
            let initial = self.energy_for(*port).unwrap_or(0.0);
            let final_energy = later.energy_for(*port).unwrap_or(0.0);
            let measured_delta = final_energy - initial;
            let ledger_delta = ledger.net_change_for(*port);
            entries.push(ReservoirReconciliation {
                port: *port,
                measured_delta_joules: measured_delta,
                ledger_delta_joules: ledger_delta,
                residual_joules: measured_delta - ledger_delta,
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

        let initial_total = self.total_tracked_internal_joules();
        let final_total = later.total_tracked_internal_joules();
        let boundary = ledger
            .audit_internal_energy(initial_total, final_total)
            .map_err(|_| EnergyStateAuditError::NonFiniteBoundaryAudit)?;

        Ok(EnergyReconciliationAudit {
            entries,
            untracked_ledger_ports,
            initial_total_joules: initial_total,
            final_total_joules: final_total,
            net_external_joules: boundary.net_external_joules,
            total_closure_error_joules: boundary.closure_error_joules,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReservoirReconciliation {
    pub port: EnergyPort,
    pub measured_delta_joules: f64,
    pub ledger_delta_joules: f64,
    pub residual_joules: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnergyReconciliationAudit {
    pub entries: Vec<ReservoirReconciliation>,
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
            .map(|entry| entry.residual_joules.abs())
            .fold(0.0_f64, f64::max)
    }

    pub fn unexplained_reservoir_count(&self, tolerance_joules: f64) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.residual_joules.abs() > tolerance_joules)
            .count()
    }

    pub fn fully_reconciled(&self, tolerance_joules: f64) -> bool {
        tolerance_joules.is_finite()
            && tolerance_joules >= 0.0
            && self.untracked_ledger_ports.is_empty()
            && self.max_abs_residual_joules() <= tolerance_joules
            && self.total_closure_error_joules.abs() <= tolerance_joules
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EnergyStateAuditError {
    MismatchedThermalReference,
    NonFiniteBoundaryAudit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;
    use crate::energy::{EnergyTransferKind, EnergyTransferLedger};
    use crate::external_heat::exchange_external_heat_audited;
    use crate::thermal::{ThermalBody, ThermalMaterial, ThermalState};
    use nalgebra::SVector;
    use symtropy_math::Point;

    fn world() -> (PhysicsWorld<3>, BodyHandle) {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        world.body_mut(handle).unwrap().set_thermal(
            ThermalBody::new(
                ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
                ThermalState::new(300.0).unwrap(),
                1.0,
            )
            .unwrap(),
        );
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
}
