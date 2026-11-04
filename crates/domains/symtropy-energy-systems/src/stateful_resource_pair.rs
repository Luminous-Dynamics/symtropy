// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Paired counterfactual comparison for stateful storage/thermal site trajectories.
//!
//! Both trajectories are independently verified by stateful_resource.rs. The
//! pair must share the same initial storage/thermal contracts, scenario, site,
//! PCC boundary, time windows, background facility demand, non-storage supply
//! and ambient-temperature trace. Operational IT, cooling and storage actions
//! may differ. A paired trajectory is only emitted when terminal storage energy
//! and thermal temperature reconcile within explicit caller tolerances.
//!
//! This prevents capacity/response claims from borrowing battery energy or
//! thermal headroom from beyond the measured horizon.

use core::fmt;

use serde::Serialize;

use crate::{
    ResourceTrajectoryError, StatefulResourceTrajectory,
    StatefulResourceVerificationReport, verify_stateful_resource_trajectory,
};

/// Numerical tolerance for matching end-of-horizon physical states.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct StatefulResourcePairTolerance {
    terminal_storage_energy_kwh: f64,
    terminal_temperature_c: f64,
}

impl StatefulResourcePairTolerance {
    pub fn new(
        terminal_storage_energy_kwh: f64,
        terminal_temperature_c: f64,
    ) -> Result<Self, StatefulResourcePairError> {
        for (name, value) in [
            ("terminal storage energy tolerance kWh", terminal_storage_energy_kwh),
            ("terminal temperature tolerance C", terminal_temperature_c),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(StatefulResourcePairError::InvalidTolerance { name, value });
            }
        }
        Ok(Self {
            terminal_storage_energy_kwh,
            terminal_temperature_c,
        })
    }

    pub const fn terminal_storage_energy_kwh(&self) -> f64 {
        self.terminal_storage_energy_kwh
    }

    pub const fn terminal_temperature_c(&self) -> f64 {
        self.terminal_temperature_c
    }
}

/// Pair identity supplied by the experiment owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatefulResourcePairIdentity {
    pair_id: String,
    baseline_service_receipt_id: Option<String>,
    flexible_service_receipt_id: Option<String>,
}

impl StatefulResourcePairIdentity {
    pub fn new(
        pair_id: impl Into<String>,
        baseline_service_receipt_id: Option<String>,
        flexible_service_receipt_id: Option<String>,
    ) -> Result<Self, StatefulResourcePairError> {
        let pair_id = pair_id.into();
        require_id("pair ID", &pair_id)?;
        if baseline_service_receipt_id.as_deref().is_some_and(|x| x.trim().is_empty()) {
            return Err(StatefulResourcePairError::MissingIdentity {
                name: "baseline service receipt ID",
            });
        }
        if flexible_service_receipt_id.as_deref().is_some_and(|x| x.trim().is_empty()) {
            return Err(StatefulResourcePairError::MissingIdentity {
                name: "flexible service receipt ID",
            });
        }
        Ok(Self {
            pair_id,
            baseline_service_receipt_id,
            flexible_service_receipt_id,
        })
    }

    pub fn pair_id(&self) -> &str { &self.pair_id }
    pub fn baseline_service_receipt_id(&self) -> Option<&str> {
        self.baseline_service_receipt_id.as_deref()
    }
    pub fn flexible_service_receipt_id(&self) -> Option<&str> {
        self.flexible_service_receipt_id.as_deref()
    }
}

/// Signed PCC import difference and physical state receipts for one interval.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatefulResourcePairSlotReceipt {
    pub slot_index: usize,
    pub sample_id_baseline: String,
    pub sample_id_flexible: String,
    pub start_unix_seconds: i64,
    pub end_unix_seconds: i64,
    pub baseline_grid_import_kw: f64,
    pub flexible_grid_import_kw: f64,
    /// Positive means lower import in the flexible case; negative is rebound.
    pub net_grid_import_reduction_kw: f64,
    pub signed_grid_import_reduction_energy_kwh: f64,
    pub gross_reduction_energy_kwh: f64,
    pub rebound_energy_kwh: f64,
    pub baseline_storage_energy_end_kwh: f64,
    pub flexible_storage_energy_end_kwh: f64,
    pub baseline_zone_temperature_end_c: f64,
    pub flexible_zone_temperature_end_c: f64,
    pub baseline_power_evidence_id: String,
    pub flexible_power_evidence_id: String,
    pub baseline_storage_evidence_id: String,
    pub flexible_storage_evidence_id: String,
    pub baseline_thermal_evidence_id: String,
    pub flexible_thermal_evidence_id: String,
}

/// Paired physical flexibility trajectory with terminal-state closure.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatefulResourcePairReport {
    pub identity: StatefulResourcePairIdentity,
    pub scenario_id: String,
    pub site_id: String,
    pub boundary_id: String,
    pub baseline_configuration_digest: String,
    pub flexible_configuration_digest: String,
    pub slot_count: usize,
    pub baseline_total_grid_import_energy_kwh: f64,
    pub flexible_total_grid_import_energy_kwh: f64,
    pub gross_reduction_energy_kwh: f64,
    pub rebound_energy_kwh: f64,
    pub net_grid_import_reduction_energy_kwh: f64,
    pub maximum_instantaneous_reduction_kw: f64,
    pub maximum_instantaneous_rebound_kw: f64,
    pub baseline_terminal_storage_energy_kwh: f64,
    pub flexible_terminal_storage_energy_kwh: f64,
    pub terminal_storage_energy_delta_kwh: f64,
    pub baseline_terminal_zone_temperature_c: f64,
    pub flexible_terminal_zone_temperature_c: f64,
    pub terminal_zone_temperature_delta_c: f64,
    pub terminal_storage_state_reconciled: bool,
    pub terminal_thermal_state_reconciled: bool,
    pub service_receipts_pinned: bool,
    pub both_physical_trajectories_passed: bool,
    pub claim_ceiling: String,
    pub baseline_verification: StatefulResourceVerificationReport,
    pub flexible_verification: StatefulResourceVerificationReport,
    pub slot_receipts: Vec<StatefulResourcePairSlotReceipt>,
}

/// Fail-closed paired-trajectory comparison errors.
#[derive(Debug, Clone, PartialEq)]
pub enum StatefulResourcePairError {
    MissingIdentity { name: &'static str },
    InvalidTolerance { name: &'static str, value: f64 },
    ScenarioMismatch { baseline: String, flexible: String },
    SiteMismatch,
    BoundaryMismatch,
    StorageContractMismatch,
    ThermalContractMismatch,
    FacilityLimitMismatch,
    TimeWindowCountMismatch { baseline: usize, flexible: usize },
    TimeWindowMismatch { slot_index: usize },
    BackgroundContextMismatch { slot_index: usize, field: &'static str },
    ResourceVerificationFailed { scenario_id: String, reason: String },
    TrajectoryDidNotPass { scenario_id: String, failing_slot_ids: Vec<String> },
    TerminalStorageStateMismatch { baseline_kwh: f64, flexible_kwh: f64, tolerance_kwh: f64 },
    TerminalThermalStateMismatch { baseline_c: f64, flexible_c: f64, tolerance_c: f64 },
    SlotReceiptCountMismatch { baseline: usize, flexible: usize },
    NumericalOverflow(&'static str),
}

impl fmt::Display for StatefulResourcePairError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { name } => write!(f, "{name} must not be empty"),
            Self::InvalidTolerance { name, value } => {
                write!(f, "{name} must be finite and non-negative; got {value}")
            }
            Self::ScenarioMismatch { baseline, flexible } => {
                write!(f, "scenario mismatch: baseline {baseline}, flexible {flexible}")
            }
            Self::SiteMismatch => write!(f, "paired trajectories use different sites"),
            Self::BoundaryMismatch => write!(f, "paired trajectories use different measurement boundaries"),
            Self::StorageContractMismatch => {
                write!(f, "paired trajectories do not share an identical storage contract and initial state")
            }
            Self::ThermalContractMismatch => {
                write!(f, "paired trajectories do not share an identical thermal contract and initial state")
            }
            Self::FacilityLimitMismatch => {
                write!(f, "paired trajectories use different facility or grid-import limits")
            }
            Self::TimeWindowCountMismatch { baseline, flexible } => {
                write!(f, "paired trajectories contain {baseline} and {flexible} time windows")
            }
            Self::TimeWindowMismatch { slot_index } => {
                write!(f, "paired trajectories have different time windows at slot {slot_index}")
            }
            Self::BackgroundContextMismatch { slot_index, field } => {
                write!(f, "paired trajectories differ in {field} at slot {slot_index}")
            }
            Self::ResourceVerificationFailed { scenario_id, reason } => {
                write!(f, "scenario {scenario_id} stateful verification failed: {reason}")
            }
            Self::TrajectoryDidNotPass { scenario_id, failing_slot_ids } => write!(
                f, "scenario {scenario_id} has physical gate failures in slots {failing_slot_ids:?}"
            ),
            Self::TerminalStorageStateMismatch { baseline_kwh, flexible_kwh, tolerance_kwh } => write!(
                f, "terminal storage states differ: baseline {baseline_kwh} kWh, flexible {flexible_kwh} kWh, tolerance {tolerance_kwh} kWh"
            ),
            Self::TerminalThermalStateMismatch { baseline_c, flexible_c, tolerance_c } => write!(
                f, "terminal thermal states differ: baseline {baseline_c} C, flexible {flexible_c} C, tolerance {tolerance_c} C"
            ),
            Self::SlotReceiptCountMismatch { baseline, flexible } => {
                write!(f, "verified slot receipts differ in count: {baseline} vs {flexible}")
            }
            Self::NumericalOverflow(operation) => write!(f, "non-finite arithmetic during {operation}"),
        }
    }
}

impl std::error::Error for StatefulResourcePairError {}

/// Independently verify and pair a baseline and flexible stateful resource path.
///
/// The pair must use identical initial storage/thermal contracts, power
/// envelopes, scenario, location, time axis, background facility load, other
/// facility load, non-storage onsite supply and ambient temperature. IT load,
/// cooling actions and battery charge/discharge may differ. Both paths must
/// pass all stateful resource gates and end at equivalent storage/thermal
/// states within explicit tolerances.
pub fn compare_stateful_resource_trajectories(
    identity: StatefulResourcePairIdentity,
    baseline: &StatefulResourceTrajectory,
    flexible: &StatefulResourceTrajectory,
    tolerance: StatefulResourcePairTolerance,
) -> Result<StatefulResourcePairReport, StatefulResourcePairError> {
    let base_contract = baseline.contract();
    let flex_contract = flexible.contract();

    if base_contract.scenario_id() != flex_contract.scenario_id() {
        return Err(StatefulResourcePairError::ScenarioMismatch {
            baseline: base_contract.scenario_id().to_string(),
            flexible: flex_contract.scenario_id().to_string(),
        });
    }
    if base_contract.site_id() != flex_contract.site_id() {
        return Err(StatefulResourcePairError::SiteMismatch);
    }
    if base_contract.boundary_id() != flex_contract.boundary_id() {
        return Err(StatefulResourcePairError::BoundaryMismatch);
    }
    if base_contract.storage() != flex_contract.storage() {
        return Err(StatefulResourcePairError::StorageContractMismatch);
    }
    if base_contract.thermal_zone() != flex_contract.thermal_zone() {
        return Err(StatefulResourcePairError::ThermalContractMismatch);
    }
    if base_contract.maximum_facility_input_power_kw()
        != flex_contract.maximum_facility_input_power_kw()
        || base_contract.maximum_grid_import_power_kw()
            != flex_contract.maximum_grid_import_power_kw()
    {
        return Err(StatefulResourcePairError::FacilityLimitMismatch);
    }

    let base_slots = baseline.slots();
    let flex_slots = flexible.slots();
    if base_slots.len() != flex_slots.len() {
        return Err(StatefulResourcePairError::TimeWindowCountMismatch {
            baseline: base_slots.len(),
            flexible: flex_slots.len(),
        });
    }

    for (index, (base_slot, flex_slot)) in base_slots.iter().zip(flex_slots).enumerate() {
        if base_slot.start_unix_seconds() != flex_slot.start_unix_seconds()
            || base_slot.end_unix_seconds() != flex_slot.end_unix_seconds()
        {
            return Err(StatefulResourcePairError::TimeWindowMismatch { slot_index: index });
        }
        for (field, base_value, flex_value) in [
            (
                "site ID",
                base_slot.site_id(),
                flex_slot.site_id(),
            ),
            (
                "measurement boundary ID",
                base_slot.boundary_id(),
                flex_slot.boundary_id(),
            ),
        ] {
            if base_value != flex_value {
                return Err(StatefulResourcePairError::BackgroundContextMismatch {
                    slot_index: index,
                    field,
                });
            }
        }
        for (field, base_value, flex_value) in [
            (
                "base facility power",
                base_slot.base_facility_power_kw(),
                flex_slot.base_facility_power_kw(),
            ),
            (
                "other facility power",
                base_slot.other_facility_power_kw(),
                flex_slot.other_facility_power_kw(),
            ),
            (
                "non-storage onsite supply",
                base_slot.onsite_non_storage_supply_kw(),
                flex_slot.onsite_non_storage_supply_kw(),
            ),
            (
                "ambient temperature",
                base_slot.ambient_temperature_c(),
                flex_slot.ambient_temperature_c(),
            ),
        ] {
            if base_value != flex_value {
                return Err(StatefulResourcePairError::BackgroundContextMismatch {
                    slot_index: index,
                    field,
                });
            }
        }
    }

    let baseline_verification = verify_stateful_resource_trajectory(baseline)
        .map_err(|error| resource_verification_error(base_contract.scenario_id(), error))?;
    let flexible_verification = verify_stateful_resource_trajectory(flexible)
        .map_err(|error| resource_verification_error(flex_contract.scenario_id(), error))?;

    if !baseline_verification.passed || !flexible_verification.passed {
        let failing_slot_ids = baseline_verification.slot_receipts.iter()
            .chain(flexible_verification.slot_receipts.iter())
            .filter(|receipt| !receipt.passed)
            .map(|receipt| receipt.sample_id.clone())
            .collect::<Vec<_>>();
        return Err(StatefulResourcePairError::TrajectoryDidNotPass {
            scenario_id: base_contract.scenario_id().to_string(),
            failing_slot_ids,
        });
    }

    let baseline_receipts = &baseline_verification.slot_receipts;
    let flexible_receipts = &flexible_verification.slot_receipts;
    if baseline_receipts.len() != flexible_receipts.len()
        || baseline_receipts.len() != base_slots.len()
    {
        return Err(StatefulResourcePairError::SlotReceiptCountMismatch {
            baseline: baseline_receipts.len(),
            flexible: flexible_receipts.len(),
        });
    }

    let baseline_terminal = baseline_receipts.last()
        .ok_or(StatefulResourcePairError::SlotReceiptCountMismatch {
            baseline: baseline_receipts.len(),
            flexible: flexible_receipts.len(),
        })?;
    let flexible_terminal = flexible_receipts.last()
        .ok_or(StatefulResourcePairError::SlotReceiptCountMismatch {
            baseline: baseline_receipts.len(),
            flexible: flexible_receipts.len(),
        })?;
    let storage_delta = flexible_terminal.storage_energy_end_kwh
        - baseline_terminal.storage_energy_end_kwh;
    let thermal_delta = flexible_terminal.zone_temperature_end_c
        - baseline_terminal.zone_temperature_end_c;
    if !storage_delta.is_finite() || !thermal_delta.is_finite() {
        return Err(StatefulResourcePairError::NumericalOverflow(
            "terminal-state comparison",
        ));
    }
    if storage_delta.abs() > tolerance.terminal_storage_energy_kwh {
        return Err(StatefulResourcePairError::TerminalStorageStateMismatch {
            baseline_kwh: baseline_terminal.storage_energy_end_kwh,
            flexible_kwh: flexible_terminal.storage_energy_end_kwh,
            tolerance_kwh: tolerance.terminal_storage_energy_kwh,
        });
    }
    if thermal_delta.abs() > tolerance.terminal_temperature_c {
        return Err(StatefulResourcePairError::TerminalThermalStateMismatch {
            baseline_c: baseline_terminal.zone_temperature_end_c,
            flexible_c: flexible_terminal.zone_temperature_end_c,
            tolerance_c: tolerance.terminal_temperature_c,
        });
    }

    let mut gross_reduction_energy_kwh = 0.0_f64;
    let mut rebound_energy_kwh = 0.0_f64;
    let mut net_grid_import_reduction_energy_kwh = 0.0_f64;
    let mut maximum_instantaneous_reduction_kw = 0.0_f64;
    let mut maximum_instantaneous_rebound_kw = 0.0_f64;
    let mut slot_receipts = Vec::with_capacity(base_slots.len());

    for (index, ((base_slot, base), flex)) in base_slots
        .iter()
        .zip(baseline_receipts)
        .zip(flexible_receipts)
        .enumerate()
    {
        if base.start_unix_seconds != flex.start_unix_seconds
            || base.end_unix_seconds != flex.end_unix_seconds
        {
            return Err(StatefulResourcePairError::TimeWindowMismatch {
                slot_index: index,
            });
        }
        let duration_hours = base.duration_hours;
        let reduction_kw = base.grid_import_power_kw - flex.grid_import_power_kw;
        let signed_energy_kwh = reduction_kw * duration_hours;
        if ![
            duration_hours,
            reduction_kw,
            signed_energy_kwh,
        ].iter().all(|value| value.is_finite()) {
            return Err(StatefulResourcePairError::NumericalOverflow(
                "paired PCC import delta",
            ));
        }
        let gross_kwh = reduction_kw.max(0.0) * duration_hours;
        let rebound_kwh = (-reduction_kw).max(0.0) * duration_hours;
        gross_reduction_energy_kwh += gross_kwh;
        rebound_energy_kwh += rebound_kwh;
        net_grid_import_reduction_energy_kwh += signed_energy_kwh;
        maximum_instantaneous_reduction_kw =
            maximum_instantaneous_reduction_kw.max(reduction_kw.max(0.0));
        maximum_instantaneous_rebound_kw =
            maximum_instantaneous_rebound_kw.max((-reduction_kw).max(0.0));

        if ![
            gross_reduction_energy_kwh,
            rebound_energy_kwh,
            net_grid_import_reduction_energy_kwh,
            maximum_instantaneous_reduction_kw,
            maximum_instantaneous_rebound_kw,
        ].iter().all(|value| value.is_finite()) {
            return Err(StatefulResourcePairError::NumericalOverflow(
                "paired trajectory aggregation",
            ));
        }

        slot_receipts.push(StatefulResourcePairSlotReceipt {
            slot_index: index,
            sample_id_baseline: base.sample_id.clone(),
            sample_id_flexible: flex.sample_id.clone(),
            start_unix_seconds: base.start_unix_seconds,
            end_unix_seconds: base.end_unix_seconds,
            baseline_grid_import_kw: base.grid_import_power_kw,
            flexible_grid_import_kw: flex.grid_import_power_kw,
            net_grid_import_reduction_kw: reduction_kw,
            signed_grid_import_reduction_energy_kwh: signed_energy_kwh,
            gross_reduction_energy_kwh: gross_kwh,
            rebound_energy_kwh: rebound_kwh,
            baseline_storage_energy_end_kwh: base.storage_energy_end_kwh,
            flexible_storage_energy_end_kwh: flex.storage_energy_end_kwh,
            baseline_zone_temperature_end_c: base.zone_temperature_end_c,
            flexible_zone_temperature_end_c: flex.zone_temperature_end_c,
            baseline_power_evidence_id: base.power_evidence_id.clone(),
            flexible_power_evidence_id: flex.power_evidence_id.clone(),
            baseline_storage_evidence_id: base.storage_evidence_id.clone(),
            flexible_storage_evidence_id: flex.storage_evidence_id.clone(),
            baseline_thermal_evidence_id: base.thermal_evidence_id.clone(),
            flexible_thermal_evidence_id: flex.thermal_evidence_id.clone(),
        });
        debug_assert_eq!(base_slot.start_unix_seconds(), base.start_unix_seconds);
    }

    let service_receipts_pinned = identity.baseline_service_receipt_id.is_some()
        && identity.flexible_service_receipt_id.is_some();

    Ok(StatefulResourcePairReport {
        identity,
        scenario_id: base_contract.scenario_id().to_string(),
        site_id: base_contract.site_id().to_string(),
        boundary_id: base_contract.boundary_id().to_string(),
        baseline_configuration_digest: baseline_verification.configuration_digest.clone(),
        flexible_configuration_digest: flexible_verification.configuration_digest.clone(),
        slot_count: slot_receipts.len(),
        baseline_total_grid_import_energy_kwh:
            baseline_verification.total_grid_import_energy_kwh,
        flexible_total_grid_import_energy_kwh:
            flexible_verification.total_grid_import_energy_kwh,
        gross_reduction_energy_kwh,
        rebound_energy_kwh,
        net_grid_import_reduction_energy_kwh,
        maximum_instantaneous_reduction_kw,
        maximum_instantaneous_rebound_kw,
        baseline_terminal_storage_energy_kwh: baseline_terminal.storage_energy_end_kwh,
        flexible_terminal_storage_energy_kwh: flexible_terminal.storage_energy_end_kwh,
        terminal_storage_energy_delta_kwh: storage_delta,
        baseline_terminal_zone_temperature_c: baseline_terminal.zone_temperature_end_c,
        flexible_terminal_zone_temperature_c: flexible_terminal.zone_temperature_end_c,
        terminal_zone_temperature_delta_c: thermal_delta,
        terminal_storage_state_reconciled: true,
        terminal_thermal_state_reconciled: true,
        service_receipts_pinned,
        both_physical_trajectories_passed: true,
        claim_ceiling: "PAIRED_STATEFUL_SITE_TRAJECTORIES_WITH_TERMINAL_SOC_AND_THERMAL_STATE_RECONCILED_NOT_WORKLOAD_SERVICE_EQUIVALENCE_OR_GRID_CAPACITY_ACCREDITATION".to_string(),
        baseline_verification,
        flexible_verification,
        slot_receipts,
    })
}

fn resource_verification_error(
    scenario_id: &str,
    error: ResourceTrajectoryError,
) -> StatefulResourcePairError {
    StatefulResourcePairError::ResourceVerificationFailed {
        scenario_id: scenario_id.to_string(),
        reason: error.to_string(),
    }
}

fn require_id(
    name: &'static str,
    value: &str,
) -> Result<(), StatefulResourcePairError> {
    if value.trim().is_empty() {
        Err(StatefulResourcePairError::MissingIdentity { name })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        StatefulResourceContract, StatefulResourceSlot, StorageReserveContract,
        ThermalZoneContract,
    };

    fn tolerance() -> StatefulResourcePairTolerance {
        StatefulResourcePairTolerance::new(1e-8, 1e-8).expect("tolerance")
    }

    fn identity() -> StatefulResourcePairIdentity {
        StatefulResourcePairIdentity::new(
            "paired-site-trajectory",
            Some("workload-baseline-receipt".to_string()),
            Some("workload-flexible-receipt".to_string()),
        ).expect("identity")
    }

    fn storage() -> StorageReserveContract {
        StorageReserveContract::new(
            100.0, 90.0, 20.0, 50.0, 50.0, 0.95, 0.95,
            10.0, 3_600, 0.20,
        ).expect("storage")
    }

    fn thermal(initial_c: f64, maximum_c: f64) -> ThermalZoneContract {
        ThermalZoneContract::new(
            "zone-a", initial_c, maximum_c, 10.0, 1.0,
            80.0, 30.0, 2.0, 8.0,
        ).expect("thermal")
    }

    fn contract(config: &str) -> StatefulResourceContract {
        StatefulResourceContract::new(
            "stress-scenario-1", "site-1", "site-pcc", config,
            storage(), thermal(25.0, 32.0), 200.0, 150.0,
        ).expect("contract")
    }

    #[allow(clippy::too_many_arguments)]
    fn slot(
        id: &str,
        index: i64,
        base_kw: f64,
        it_kw: f64,
        cooling_electric_kw: f64,
        onsite_kw: f64,
        charge_kw: f64,
        discharge_kw: f64,
        it_heat_kw: f64,
        ambient_c: f64,
        heat_removed_kw: f64,
    ) -> StatefulResourceSlot {
        let start = 100_000 + index * 3_600;
        StatefulResourceSlot::new(
            id, "site-1", "site-pcc", start, start + 3_600,
            base_kw, it_kw, cooling_electric_kw, 2.0, onsite_kw,
            charge_kw, discharge_kw, it_heat_kw, ambient_c, heat_removed_kw,
            format!("power-{id}"), format!("storage-{id}"), format!("thermal-{id}"),
        ).expect("slot")
    }

    fn trajectory(
        config: &str,
        actions: &[(f64, f64, f64, f64)],
    ) -> StatefulResourceTrajectory {
        StatefulResourceTrajectory::new(
            contract(config),
            actions.iter().enumerate().map(|(index, action)| {
                slot(
                    &format!("{config}-slot-{index}"),
                    index as i64,
                    10.0,
                    20.0,
                    action.0,
                    5.0,
                    action.1,
                    action.2,
                    20.0,
                    25.0,
                    action.3,
                )
            }).collect(),
        ).expect("trajectory")
    }

    #[test]
    fn paired_battery_shift_reconciles_soc_and_counts_round_trip_losses() {
        let baseline = trajectory("baseline", &[(5.0, 0.0, 0.0, 25.0), (5.0, 0.0, 0.0, 25.0, 0.0)]);
        let flexible = trajectory("flexible", &[(5.0, 0.0, 5.0, 25.0), (5.0, 5.54016620498615, 0.0, 25.0)]);
        let report = compare_stateful_resource_trajectories(
            identity(), &baseline, &flexible, tolerance()
        ).expect("paired resource path");
        assert!(report.both_physical_trajectories_passed);
        assert!(report.terminal_storage_state_reconciled);
        assert!(report.terminal_thermal_state_reconciled);
        assert!((report.terminal_storage_energy_delta_kwh).abs() < 1e-8);
        assert!((report.gross_reduction_energy_kwh - 5.0).abs() < 1e-8);
        assert!((report.rebound_energy_kwh - 5.54016620498615).abs() < 1e-8);
        assert!(report.net_grid_import_reduction_energy_kwh < 0.0);
        assert!(report.service_receipts_pinned);
    }

    #[test]
    fn terminal_soc_debt_is_rejected_even_when_each_path_individually_passes() {
        let baseline = trajectory("baseline", &[(5.0, 0.0, 0.0, 25.0, 0.0)]);
        let flexible = trajectory("flexible", &[(5.0, 0.0, 5.0, 25.0, 0.0)]);
        assert!(verify_stateful_resource_trajectory(&baseline).expect("base").passed);
        assert!(verify_stateful_resource_trajectory(&flexible).expect("flex").passed);
        assert!(matches!(
            compare_stateful_resource_trajectories(identity(), &baseline, &flexible, tolerance()),
            Err(StatefulResourcePairError::TerminalStorageStateMismatch { .. })
        ));
    }

    #[test]
    fn terminal_thermal_debt_is_rejected() {
        let baseline = trajectory("baseline", &[(5.0, 0.0, 0.0, 25.0)]);
        let flexible = trajectory("flexible", &[(0.0, 0.0, 0.0, 0.0)]);
        assert!(matches!(
            compare_stateful_resource_trajectories(identity(), &baseline, &flexible, tolerance()),
            Err(StatefulResourcePairError::TerminalThermalStateMismatch { .. })
        ));
    }

    #[test]
    fn different_weather_context_is_rejected() {
        let baseline = trajectory("baseline", &[(5.0, 0.0, 0.0, 25.0, 25.0)]);
        let flex = StatefulResourceTrajectory::new(
            contract("flexible"),
            vec![slot("flex-slot-0", 0, 10.0, 20.0, 5.0, 5.0, 0.0, 0.0, 20.0, 26.0, 25.0)],
        ).expect("trajectory");
        assert!(matches!(
            compare_stateful_resource_trajectories(identity(), &baseline, &flex, tolerance()),
            Err(StatefulResourcePairError::BackgroundContextMismatch { field: "ambient temperature", .. })
        ));
    }

    #[test]
    fn different_initial_storage_contract_is_rejected() {
        let baseline = trajectory("baseline", &[(5.0, 0.0, 0.0, 25.0, 0.0)]);
        let flex_contract = StatefulResourceContract::new(
            "stress-scenario-1", "site-1", "site-pcc", "flexible",
            StorageReserveContract::new(100.0, 89.0, 20.0, 50.0, 50.0, 0.95, 0.95, 10.0, 3_600, 0.20).expect("storage"),
            thermal(25.0, 32.0), 200.0, 150.0,
        ).expect("contract");
        let flexible = StatefulResourceTrajectory::new(
            flex_contract,
            vec![slot("flex-slot-0", 0, 10.0, 20.0, 5.0, 5.0, 0.0, 0.0, 20.0, 25.0, 0.0)],
        ).expect("trajectory");
        assert!(matches!(
            compare_stateful_resource_trajectories(identity(), &baseline, &flexible, tolerance()),
            Err(StatefulResourcePairError::StorageContractMismatch)
        ));
    }

    #[test]
    fn different_time_windows_are_rejected() {
        let baseline = trajectory("baseline", &[(5.0, 0.0, 0.0, 25.0, 0.0)]);
        let flex_contract = contract("flexible");
        let shifted = StatefulResourceSlot::new(
            "flex-shifted", "site-1", "site-pcc", 100_001, 103_601,
            10.0, 20.0, 5.0, 2.0, 5.0, 0.0, 0.0,
            20.0, 25.0, 0.0, "p", "s", "t",
        ).expect("slot");
        let flexible = StatefulResourceTrajectory::new(flex_contract, vec![shifted]).expect("trajectory");
        assert!(matches!(
            compare_stateful_resource_trajectories(identity(), &baseline, &flexible, tolerance()),
            Err(StatefulResourcePairError::TimeWindowMismatch { .. })
        ));
    }

    #[test]
    fn failed_physical_gate_emits_no_paired_trajectory() {
        let baseline = trajectory("baseline", &[(5.0, 0.0, 0.0, 25.0, 0.0)]);
        let flexible = trajectory("flexible", &[(5.0, 0.0, 60.0, 25.0)]);
        assert!(matches!(
            compare_stateful_resource_trajectories(identity(), &baseline, &flexible, tolerance()),
            Err(StatefulResourcePairError::TrajectoryDidNotPass { .. })
        ));
    }

    #[test]
    fn missing_identity_and_invalid_tolerance_are_rejected() {
        assert!(matches!(
            StatefulResourcePairIdentity::new("", None, None),
            Err(StatefulResourcePairError::MissingIdentity { .. })
        ));
        assert!(matches!(
            StatefulResourcePairTolerance::new(-1.0, 0.1),
            Err(StatefulResourcePairError::InvalidTolerance { .. })
        ));
    }
}
