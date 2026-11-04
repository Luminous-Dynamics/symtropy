// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Derive a signed grid-import trajectory from paired, independently verified
//! workload schedules.
//!
//! This module bridges the workload schedule verifier and chronological
//! adequacy evaluator. It requires the baseline and flexible schedules to use
//! the exact same job contracts and slot/facility contracts. Each schedule is
//! independently verified before any flexibility delta is emitted.
//!
//! Positive net reduction means flexible grid import is below baseline import.
//! Negative net reduction means the flexible schedule increases grid import
//! in that slot (including rebound from previously deferred work).
//!
//! This is workload-only temporal shifting. It does not model storage state,
//! dynamic thermal inertia, nonlinear cooling, migration, network constraints,
//! job arrival uncertainty, or control/communications failures.

use core::fmt;

use serde::Serialize;

use crate::{
    WorkloadSchedule, WorkloadScheduleError, WorkloadScheduleTolerance,
    WorkloadScheduleVerificationReport, WorkloadSlotVerification,
    verify_workload_schedule,
};

/// Provenance identity supplied by the paired-schedule experiment owner.
///
/// Schedule digests are caller supplied. This module pins them into the receipt
/// but does not hash or authenticate schedule bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkloadFlexibilityTrajectoryIdentity {
    trajectory_id: String,
    site_id: String,
    pcc_measurement_boundary_id: String,
    baseline_schedule_digest: String,
    flexible_schedule_digest: String,
}

impl WorkloadFlexibilityTrajectoryIdentity {
    pub fn new(
        trajectory_id: impl Into<String>,
        site_id: impl Into<String>,
        pcc_measurement_boundary_id: impl Into<String>,
        baseline_schedule_digest: impl Into<String>,
        flexible_schedule_digest: impl Into<String>,
    ) -> Result<Self, WorkloadFlexibilityTrajectoryError> {
        let identity = Self {
            trajectory_id: trajectory_id.into(),
            site_id: site_id.into(),
            pcc_measurement_boundary_id: pcc_measurement_boundary_id.into(),
            baseline_schedule_digest: baseline_schedule_digest.into(),
            flexible_schedule_digest: flexible_schedule_digest.into(),
        };
        for (name, value) in [
            ("trajectory ID", identity.trajectory_id.as_str()),
            ("site ID", identity.site_id.as_str()),
            ("PCC measurement boundary ID", identity.pcc_measurement_boundary_id.as_str()),
            ("baseline schedule digest", identity.baseline_schedule_digest.as_str()),
            ("flexible schedule digest", identity.flexible_schedule_digest.as_str()),
        ] {
            require_id(name, value)?;
        }
        Ok(identity)
    }

    pub fn trajectory_id(&self) -> &str { &self.trajectory_id }
    pub fn site_id(&self) -> &str { &self.site_id }
    pub fn pcc_measurement_boundary_id(&self) -> &str {
        &self.pcc_measurement_boundary_id
    }
    pub fn baseline_schedule_digest(&self) -> &str { &self.baseline_schedule_digest }
    pub fn flexible_schedule_digest(&self) -> &str { &self.flexible_schedule_digest }
}

/// Signed import change and interval energy for one identical schedule slot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkloadFlexibilitySlotReceipt {
    pub slot_index: u32,
    pub start_elapsed_ns: u64,
    pub duration_ns: u64,
    pub baseline_grid_import_kw: f64,
    pub flexible_grid_import_kw: f64,
    /// Positive means grid import reduction; negative means rebound/increase.
    pub net_grid_import_reduction_kw: f64,
    /// Signed reduction integrated over this interval (kWh).
    pub signed_grid_import_reduction_energy_kwh: f64,
    pub gross_reduction_energy_kwh: f64,
    pub rebound_energy_kwh: f64,
    pub baseline_work_units: f64,
    pub flexible_work_units: f64,
    pub baseline_slot_receipt_id: String,
    pub flexible_slot_receipt_id: String,
}

/// Paired-schedule result. Every service obligation and slot contract matched.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkloadFlexibilityTrajectoryReport {
    pub identity: WorkloadFlexibilityTrajectoryIdentity,
    pub service_contract_id: String,
    pub work_unit_id: String,
    pub baseline_schedule_id: String,
    pub flexible_schedule_id: String,
    pub baseline_job_count: usize,
    pub flexible_job_count: usize,
    pub slot_count: usize,
    pub baseline_completed_work_units: f64,
    pub flexible_completed_work_units: f64,
    pub baseline_total_grid_import_energy_kwh: f64,
    pub flexible_total_grid_import_energy_kwh: f64,
    pub gross_reduction_energy_kwh: f64,
    pub rebound_energy_kwh: f64,
    /// Signed: positive is net energy reduction, negative is net added import.
    pub net_grid_import_reduction_energy_kwh: f64,
    pub maximum_instantaneous_reduction_kw: f64,
    pub maximum_instantaneous_rebound_kw: f64,
    pub service_equivalent: bool,
    pub schedule_verification_passed: bool,
    pub claim_ceiling: String,
    pub baseline_verification: WorkloadScheduleVerificationReport,
    pub flexible_verification: WorkloadScheduleVerificationReport,
    pub slot_receipts: Vec<WorkloadFlexibilitySlotReceipt>,
}

/// Errors for paired workload schedule comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkloadFlexibilityTrajectoryError {
    MissingIdentity { name: &'static str },
    ScheduleIdsMustDiffer,
    ServiceContractMismatch { baseline: String, flexible: String },
    WorkUnitMismatch { baseline: String, flexible: String },
    JobManifestMismatch,
    SlotContractMismatch,
    ScheduleVerificationFailed { schedule_id: String, reason: String },
    SlotReceiptCountMismatch { baseline: usize, flexible: usize },
    SlotIndexMismatch { index: usize, baseline: u32, flexible: u32 },
    WorkQuantityMismatch { baseline: f64, flexible: f64, tolerance: f64 },
    NumericalOverflow(&'static str),
}

impl fmt::Display for WorkloadFlexibilityTrajectoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { name } => write!(f, "{name} must not be empty"),
            Self::ScheduleIdsMustDiffer => {
                write!(f, "baseline and flexible schedule IDs must differ")
            }
            Self::ServiceContractMismatch { baseline, flexible } => write!(
                f, "service contract mismatch: baseline {baseline}, flexible {flexible}"
            ),
            Self::WorkUnitMismatch { baseline, flexible } => write!(
                f, "work unit mismatch: baseline {baseline}, flexible {flexible}"
            ),
            Self::JobManifestMismatch => {
                write!(f, "baseline and flexible schedules do not have identical job contracts")
            }
            Self::SlotContractMismatch => {
                write!(f, "baseline and flexible schedules do not have identical time/facility slot contracts")
            }
            Self::ScheduleVerificationFailed { schedule_id, reason } => {
                write!(f, "schedule {schedule_id} failed independent verification: {reason}")
            }
            Self::SlotReceiptCountMismatch { baseline, flexible } => write!(
                f, "paired reports have {baseline} and {flexible} slot receipts"
            ),
            Self::SlotIndexMismatch { index, baseline, flexible } => write!(
                f, "slot receipt {index} has baseline index {baseline} and flexible index {flexible}"
            ),
            Self::WorkQuantityMismatch { baseline, flexible, tolerance } => write!(
                f, "paired schedules complete {baseline} and {flexible} work units; tolerance {tolerance}"
            ),
            Self::NumericalOverflow(operation) => {
                write!(f, "non-finite arithmetic during {operation}")
            }
        }
    }
}

impl std::error::Error for WorkloadFlexibilityTrajectoryError {}

/// Independently verify the same work obligations under baseline and flexible
/// schedules, then derive the complete signed grid-import delta by slot.
///
/// The baseline and flexible schedules must share byte-for-byte-equivalent
/// job and slot contracts in their canonical supplied order. Allocations may
/// differ. Every job must still complete within its original release/deadline
/// window under the verifier. Slot contracts include the same background
/// facility load, cooling base, committed non-grid supply and power limits.
pub fn compare_workload_flexibility_schedules(
    identity: WorkloadFlexibilityTrajectoryIdentity,
    baseline_schedule: &WorkloadSchedule,
    flexible_schedule: &WorkloadSchedule,
    tolerance: WorkloadScheduleTolerance,
) -> Result<WorkloadFlexibilityTrajectoryReport, WorkloadFlexibilityTrajectoryError> {
    if baseline_schedule.schedule_id() == flexible_schedule.schedule_id() {
        return Err(WorkloadFlexibilityTrajectoryError::ScheduleIdsMustDiffer);
    }
    if baseline_schedule.service_contract_id() != flexible_schedule.service_contract_id() {
        return Err(WorkloadFlexibilityTrajectoryError::ServiceContractMismatch {
            baseline: baseline_schedule.service_contract_id().to_string(),
            flexible: flexible_schedule.service_contract_id().to_string(),
        });
    }
    if baseline_schedule.work_unit_id() != flexible_schedule.work_unit_id() {
        return Err(WorkloadFlexibilityTrajectoryError::WorkUnitMismatch {
            baseline: baseline_schedule.work_unit_id().to_string(),
            flexible: flexible_schedule.work_unit_id().to_string(),
        });
    }
    if baseline_schedule.jobs() != flexible_schedule.jobs() {
        return Err(WorkloadFlexibilityTrajectoryError::JobManifestMismatch);
    }
    if baseline_schedule.slots() != flexible_schedule.slots() {
        return Err(WorkloadFlexibilityTrajectoryError::SlotContractMismatch);
    }

    let baseline_verification = verify_workload_schedule(baseline_schedule, tolerance)
        .map_err(|error| verification_error(baseline_schedule.schedule_id(), error))?;
    let flexible_verification = verify_workload_schedule(flexible_schedule, tolerance)
        .map_err(|error| verification_error(flexible_schedule.schedule_id(), error))?;

    if !baseline_verification.passed() {
        return Err(WorkloadFlexibilityTrajectoryError::ScheduleVerificationFailed {
            schedule_id: baseline_schedule.schedule_id().to_string(),
            reason: "verification report did not pass".to_string(),
        });
    }
    if !flexible_verification.passed() {
        return Err(WorkloadFlexibilityTrajectoryError::ScheduleVerificationFailed {
            schedule_id: flexible_schedule.schedule_id().to_string(),
            reason: "verification report did not pass".to_string(),
        });
    }

    let completed_delta =
        (baseline_verification.completed_work_units() - flexible_verification.completed_work_units())
            .abs();
    if !completed_delta.is_finite() {
        return Err(WorkloadFlexibilityTrajectoryError::NumericalOverflow(
            "paired completed work comparison",
        ));
    }
    if completed_delta > tolerance.absolute_work_units() {
        return Err(WorkloadFlexibilityTrajectoryError::WorkQuantityMismatch {
            baseline: baseline_verification.completed_work_units(),
            flexible: flexible_verification.completed_work_units(),
            tolerance: tolerance.absolute_work_units(),
        });
    }

    let baseline_receipts = baseline_verification.slot_receipts();
    let flexible_receipts = flexible_verification.slot_receipts();
    if baseline_receipts.len() != flexible_receipts.len()
        || baseline_receipts.len() != baseline_schedule.slots().len()
    {
        return Err(WorkloadFlexibilityTrajectoryError::SlotReceiptCountMismatch {
            baseline: baseline_receipts.len(),
            flexible: flexible_receipts.len(),
        });
    }

    let mut gross_reduction_energy_kwh = 0.0_f64;
    let mut rebound_energy_kwh = 0.0_f64;
    let mut net_grid_import_reduction_energy_kwh = 0.0_f64;
    let mut maximum_instantaneous_reduction_kw = 0.0_f64;
    let mut maximum_instantaneous_rebound_kw = 0.0_f64;
    let mut slot_receipts = Vec::with_capacity(baseline_receipts.len());

    for (index, ((slot, baseline), flexible)) in baseline_schedule
        .slots()
        .iter()
        .zip(baseline_receipts)
        .zip(flexible_receipts)
        .enumerate()
    {
        if baseline.slot_index() != flexible.slot_index()
            || baseline.slot_index() != slot.slot_index()
        {
            return Err(WorkloadFlexibilityTrajectoryError::SlotIndexMismatch {
                index,
                baseline: baseline.slot_index(),
                flexible: flexible.slot_index(),
            });
        }
        let net_reduction_kw =
            baseline.grid_import_power_kw() - flexible.grid_import_power_kw();
        let duration_hours = slot.duration_ns() as f64 / 3_600_000_000_000.0;
        let signed_energy_kwh = net_reduction_kw * duration_hours;
        if ![
            net_reduction_kw,
            duration_hours,
            signed_energy_kwh,
        ]
        .iter()
        .all(|value| value.is_finite())
            || duration_hours <= 0.0
        {
            return Err(WorkloadFlexibilityTrajectoryError::NumericalOverflow(
                "per-slot signed grid-import reduction",
            ));
        }

        let gross_reduction_kwh = net_reduction_kw.max(0.0) * duration_hours;
        let rebound_kwh = (-net_reduction_kw).max(0.0) * duration_hours;
        gross_reduction_energy_kwh += gross_reduction_kwh;
        rebound_energy_kwh += rebound_kwh;
        net_grid_import_reduction_energy_kwh += signed_energy_kwh;
        maximum_instantaneous_reduction_kw =
            maximum_instantaneous_reduction_kw.max(net_reduction_kw.max(0.0));
        maximum_instantaneous_rebound_kw =
            maximum_instantaneous_rebound_kw.max((-net_reduction_kw).max(0.0));

        if ![
            gross_reduction_energy_kwh,
            rebound_energy_kwh,
            net_grid_import_reduction_energy_kwh,
            maximum_instantaneous_reduction_kw,
            maximum_instantaneous_rebound_kw,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err(WorkloadFlexibilityTrajectoryError::NumericalOverflow(
                "trajectory aggregation",
            ));
        }

        slot_receipts.push(WorkloadFlexibilitySlotReceipt {
            slot_index: slot.slot_index(),
            start_elapsed_ns: slot.start_elapsed_ns(),
            duration_ns: slot.duration_ns(),
            baseline_grid_import_kw: baseline.grid_import_power_kw(),
            flexible_grid_import_kw: flexible.grid_import_power_kw(),
            net_grid_import_reduction_kw: net_reduction_kw,
            signed_grid_import_reduction_energy_kwh: signed_energy_kwh,
            gross_reduction_energy_kwh: gross_reduction_kwh,
            rebound_energy_kwh: rebound_kwh,
            baseline_work_units: baseline.compute_work_units(),
            flexible_work_units: flexible.compute_work_units(),
            baseline_slot_receipt_id: format!(
                "{}:{}:baseline",
                identity.trajectory_id(),
                slot.slot_index()
            ),
            flexible_slot_receipt_id: format!(
                "{}:{}:flexible",
                identity.trajectory_id(),
                slot.slot_index()
            ),
        });
    }

    let service_equivalent = completed_delta <= tolerance.absolute_work_units();
    let claim_ceiling = "PAIRED_VERIFIED_WORKLOAD_SCHEDULE_IMPORT_TRAJECTORY_ONLY_NO_BATTERY_THERMAL_NETWORK_OR_MARKET_CAPACITY_CLAIM";

    Ok(WorkloadFlexibilityTrajectoryReport {
        identity,
        service_contract_id: baseline_schedule.service_contract_id().to_string(),
        work_unit_id: baseline_schedule.work_unit_id().to_string(),
        baseline_schedule_id: baseline_verification.schedule_id().to_string(),
        flexible_schedule_id: flexible_verification.schedule_id().to_string(),
        baseline_job_count: baseline_verification.job_count(),
        flexible_job_count: flexible_verification.job_count(),
        slot_count: baseline_receipts.len(),
        baseline_completed_work_units: baseline_verification.completed_work_units(),
        flexible_completed_work_units: flexible_verification.completed_work_units(),
        baseline_total_grid_import_energy_kwh:
            baseline_verification.total_grid_import_energy_kwh(),
        flexible_total_grid_import_energy_kwh:
            flexible_verification.total_grid_import_energy_kwh(),
        gross_reduction_energy_kwh,
        rebound_energy_kwh,
        net_grid_import_reduction_energy_kwh,
        maximum_instantaneous_reduction_kw,
        maximum_instantaneous_rebound_kw,
        service_equivalent,
        schedule_verification_passed: true,
        claim_ceiling: claim_ceiling.to_string(),
        baseline_verification,
        flexible_verification,
        slot_receipts,
    })
}

fn verification_error(
    schedule_id: &str,
    error: WorkloadScheduleError,
) -> WorkloadFlexibilityTrajectoryError {
    WorkloadFlexibilityTrajectoryError::ScheduleVerificationFailed {
        schedule_id: schedule_id.to_string(),
        reason: error.to_string(),
    }
}

fn require_id(
    name: &'static str,
    value: &str,
) -> Result<(), WorkloadFlexibilityTrajectoryError> {
    if value.trim().is_empty() {
        Err(WorkloadFlexibilityTrajectoryError::MissingIdentity { name })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        WorkloadAllocation, WorkloadJobContract, WorkloadPreemptionPolicy,
        WorkloadScheduleSlot,
    };

    const SLOT_NS: u64 = 3_600_000_000_000;

    fn identity() -> WorkloadFlexibilityTrajectoryIdentity {
        WorkloadFlexibilityTrajectoryIdentity::new(
            "trajectory-1",
            "site-1",
            "pcc-1",
            "baseline-digest",
            "flexible-digest",
        ).expect("identity")
    }

    fn tolerance() -> WorkloadScheduleTolerance {
        WorkloadScheduleTolerance::new(1e-9, 1e-9).expect("tolerance")
    }

    fn job(required: f64) -> WorkloadJobContract {
        WorkloadJobContract::new(
            "job-1",
            "useful-work-v1",
            0,
            2,
            required,
            10.0,
            0.5,
            0.1,
            WorkloadPreemptionPolicy::Deferrable,
            0.0,
        ).expect("job")
    }

    fn slots() -> Vec<WorkloadScheduleSlot> {
        (0..2).map(|index| {
            WorkloadScheduleSlot::new(
                index,
                u64::from(index) * SLOT_NS,
                SLOT_NS,
                100.0,
                0.0,
                0.0,
                1_000.0,
                1_000.0,
                100.0,
                100.0,
            ).expect("slot")
        }).collect()
    }

    fn schedule(
        schedule_id: &str,
        work_required: f64,
        allocation_slot: u32,
    ) -> WorkloadSchedule {
        WorkloadSchedule::new(
            schedule_id,
            "sla-v1",
            "useful-work-v1",
            vec![job(work_required)],
            slots(),
            vec![WorkloadAllocation::new("job-1", allocation_slot, work_required)
                .expect("allocation")],
        ).expect("schedule")
    }

    #[test]
    fn paired_schedules_emit_signed_reduction_and_rebound_without_losing_work() {
        let baseline = schedule("baseline", 10.0, 0);
        let flexible = schedule("flexible", 10.0, 1);
        let report = compare_workload_flexibility_schedules(
            identity(),
            &baseline,
            &flexible,
            tolerance(),
        ).expect("paired trajectory");

        assert!(report.service_equivalent);
        assert!(report.schedule_verification_passed);
        assert_eq!(report.baseline_completed_work_units, 10.0);
        assert_eq!(report.flexible_completed_work_units, 10.0);
        assert_eq!(report.slot_count, 2);
        assert_eq!(report.slot_receipts[0].net_grid_import_reduction_kw, 6.0);
        assert_eq!(report.slot_receipts[1].net_grid_import_reduction_kw, -6.0);
        assert_eq!(report.gross_reduction_energy_kwh, 6.0);
        assert_eq!(report.rebound_energy_kwh, 6.0);
        assert_eq!(report.net_grid_import_reduction_energy_kwh, 0.0);
        assert_eq!(report.maximum_instantaneous_reduction_kw, 6.0);
        assert_eq!(report.maximum_instantaneous_rebound_kw, 6.0);
    }

    #[test]
    fn differing_job_manifest_is_rejected_before_any_credit_is_emitted() {
        let baseline = schedule("baseline", 10.0, 0);
        let flexible = schedule("flexible", 9.0, 1);
        assert_eq!(
            compare_workload_flexibility_schedules(
                identity(),
                &baseline,
                &flexible,
                tolerance(),
            ),
            Err(WorkloadFlexibilityTrajectoryError::JobManifestMismatch)
        );
    }

    #[test]
    fn differing_slot_contract_is_rejected() {
        let baseline = schedule("baseline", 10.0, 0);
        let flexible = WorkloadSchedule::new(
            "flexible",
            "sla-v1",
            "useful-work-v1",
            vec![job(10.0)],
            vec![
                WorkloadScheduleSlot::new(0, 0, SLOT_NS, 101.0, 0.0, 0.0, 1_000.0, 1_000.0, 100.0, 100.0).expect("slot"),
                WorkloadScheduleSlot::new(1, SLOT_NS, SLOT_NS, 100.0, 0.0, 0.0, 1_000.0, 1_000.0, 100.0, 100.0).expect("slot"),
            ],
            vec![WorkloadAllocation::new("job-1", 1, 10.0).expect("allocation")],
        ).expect("schedule");
        assert_eq!(
            compare_workload_flexibility_schedules(identity(), &baseline, &flexible, tolerance()),
            Err(WorkloadFlexibilityTrajectoryError::SlotContractMismatch)
        );
    }

    #[test]
    fn invalid_baseline_schedule_fails_closed() {
        let baseline = WorkloadSchedule::new(
            "baseline",
            "sla-v1",
            "useful-work-v1",
            vec![job(10.0)],
            slots(),
            vec![WorkloadAllocation::new("job-1", 2, 10.0).expect("allocation")],
        ).expect("structurally valid schedule");
        let flexible = schedule("flexible", 10.0, 1);
        assert!(matches!(
            compare_workload_flexibility_schedules(identity(), &baseline, &flexible, tolerance()),
            Err(WorkloadFlexibilityTrajectoryError::ScheduleVerificationFailed { .. })
        ));
    }

    #[test]
    fn identical_schedule_ids_are_rejected() {
        let baseline = schedule("same-id", 10.0, 0);
        let flexible = schedule("same-id", 10.0, 1);
        assert_eq!(
            compare_workload_flexibility_schedules(identity(), &baseline, &flexible, tolerance()),
            Err(WorkloadFlexibilityTrajectoryError::ScheduleIdsMustDiffer)
        );
    }

    #[test]
    fn empty_identity_is_rejected_and_equal_content_digests_are_allowed() {
        assert!(matches!(
            WorkloadFlexibilityTrajectoryIdentity::new("", "site", "pcc", "base", "flex"),
            Err(WorkloadFlexibilityTrajectoryError::MissingIdentity { .. })
        ));
        assert!(WorkloadFlexibilityTrajectoryIdentity::new(
            "trajectory", "site", "pcc", "same-digest", "same-digest"
        ).is_ok());
    }

    #[test]
    fn service_contract_mismatch_is_rejected() {
        let baseline = schedule("baseline", 10.0, 0);
        let flexible = WorkloadSchedule::new(
            "flexible",
            "different-sla",
            "useful-work-v1",
            vec![job(10.0)],
            slots(),
            vec![WorkloadAllocation::new("job-1", 1, 10.0).expect("allocation")],
        ).expect("schedule");
        assert!(matches!(
            compare_workload_flexibility_schedules(identity(), &baseline, &flexible, tolerance()),
            Err(WorkloadFlexibilityTrajectoryError::ServiceContractMismatch { .. })
        ));
    }
}
