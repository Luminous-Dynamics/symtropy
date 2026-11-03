// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent feasibility verification for grid-interactive compute schedules.
//!
//! Schedulers may use EDF, MILP, MPC, heuristics or learned policies. This
//! module does not select a schedule. It independently reconstructs compute,
//! facility input, grid import, cooling and checkpoint overhead from the
//! frozen job contracts, slot budgets and proposed allocations.
//!
//! A successful receipt establishes schedule/accounting feasibility against
//! these declared constraints. It does not control equipment, predict real
//! workload power or establish site/utility qualification.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

/// Whether a job may be interrupted and how a pause/resume is accounted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum WorkloadPreemptionPolicy {
    /// Work must occupy consecutive slots once the job begins.
    NonPreemptible,
    /// Gaps are allowed, but each resume incurs declared energy overhead.
    Checkpointable,
    /// Work may be deferred or spread across eligible slots without a
    /// checkpoint overhead in this first deterministic model.
    Deferrable,
}

/// Frozen workload/service contract for one job.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkloadJobContract {
    job_id: String,
    work_unit_id: String,
    release_slot: u32,
    deadline_slot_exclusive: u32,
    required_work_units: f64,
    maximum_work_units_per_slot: f64,
    it_power_kw_per_work_unit: f64,
    cooling_power_kw_per_work_unit: f64,
    preemption_policy: WorkloadPreemptionPolicy,
    checkpoint_resume_overhead_kwh: f64,
}

impl WorkloadJobContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        job_id: impl Into<String>,
        work_unit_id: impl Into<String>,
        release_slot: u32,
        deadline_slot_exclusive: u32,
        required_work_units: f64,
        maximum_work_units_per_slot: f64,
        it_power_kw_per_work_unit: f64,
        cooling_power_kw_per_work_unit: f64,
        preemption_policy: WorkloadPreemptionPolicy,
        checkpoint_resume_overhead_kwh: f64,
    ) -> Result<Self, WorkloadScheduleError> {
        let job_id = job_id.into();
        let work_unit_id = work_unit_id.into();

        require_non_empty("job ID", &job_id)?;
        require_non_empty("work unit ID", &work_unit_id)?;

        if release_slot >= deadline_slot_exclusive {
            return Err(WorkloadScheduleError::InvalidJobWindow {
                job_id,
                release_slot,
                deadline_slot_exclusive,
            });
        }

        for (name, value) in [
            ("required work", required_work_units),
            ("maximum work per slot", maximum_work_units_per_slot),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(WorkloadScheduleError::InvalidJobQuantity { name, value });
            }
        }

        for (name, value) in [
            ("IT power per work unit", it_power_kw_per_work_unit),
            ("cooling power per work unit", cooling_power_kw_per_work_unit),
            ("checkpoint resume overhead", checkpoint_resume_overhead_kwh),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(WorkloadScheduleError::InvalidJobQuantity { name, value });
            }
        }

        if preemption_policy != WorkloadPreemptionPolicy::Checkpointable
            && checkpoint_resume_overhead_kwh > 0.0
        {
            return Err(WorkloadScheduleError::UnexpectedCheckpointOverhead {
                job_id,
                policy: preemption_policy,
            });
        }

        Ok(Self {
            job_id,
            work_unit_id,
            release_slot,
            deadline_slot_exclusive,
            required_work_units,
            maximum_work_units_per_slot,
            it_power_kw_per_work_unit,
            cooling_power_kw_per_work_unit,
            preemption_policy,
            checkpoint_resume_overhead_kwh,
        })
    }

    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub fn work_unit_id(&self) -> &str {
        &self.work_unit_id
    }

    pub const fn release_slot(&self) -> u32 {
        self.release_slot
    }

    pub const fn deadline_slot_exclusive(&self) -> u32 {
        self.deadline_slot_exclusive
    }

    pub const fn required_work_units(&self) -> f64 {
        self.required_work_units
    }

    pub const fn maximum_work_units_per_slot(&self) -> f64 {
        self.maximum_work_units_per_slot
    }

    pub const fn it_power_kw_per_work_unit(&self) -> f64 {
        self.it_power_kw_per_work_unit
    }

    pub const fn cooling_power_kw_per_work_unit(&self) -> f64 {
        self.cooling_power_kw_per_work_unit
    }

    pub const fn checkpoint_resume_overhead_kwh(&self) -> f64 {
        self.checkpoint_resume_overhead_kwh
    }

    pub const fn preemption_policy(&self) -> WorkloadPreemptionPolicy {
        self.preemption_policy
    }
}

/// Physical and operational budget for one scheduling interval.
///
/// `base_non_compute_power_kw` excludes both IT work in this schedule and
/// the separately declared base cooling load. `committed_non_grid_supply_kw`
/// is supply already committed at the same facility-input boundary (e.g.
/// on-site generation and storage discharge); it is not added to facility
/// demand and is used only to estimate the residual grid import.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkloadScheduleSlot {
    slot_index: u32,
    start_elapsed_ns: u64,
    duration_ns: u64,
    base_non_compute_power_kw: f64,
    base_cooling_power_kw: f64,
    committed_non_grid_supply_kw: f64,
    maximum_facility_input_kw: f64,
    maximum_grid_import_kw: f64,
    maximum_cooling_power_kw: f64,
    maximum_compute_work_units: f64,
}

impl WorkloadScheduleSlot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        slot_index: u32,
        start_elapsed_ns: u64,
        duration_ns: u64,
        base_non_compute_power_kw: f64,
        base_cooling_power_kw: f64,
        committed_non_grid_supply_kw: f64,
        maximum_facility_input_kw: f64,
        maximum_grid_import_kw: f64,
        maximum_cooling_power_kw: f64,
        maximum_compute_work_units: f64,
    ) -> Result<Self, WorkloadScheduleError> {
        if duration_ns == 0 {
            return Err(WorkloadScheduleError::InvalidSlotDuration {
                slot_index,
                duration_ns,
            });
        }

        for (name, value) in [
            ("base non-compute power", base_non_compute_power_kw),
            ("base cooling power", base_cooling_power_kw),
            ("committed non-grid supply", committed_non_grid_supply_kw),
            ("maximum facility input", maximum_facility_input_kw),
            ("maximum grid import", maximum_grid_import_kw),
            ("maximum cooling power", maximum_cooling_power_kw),
            ("maximum compute work", maximum_compute_work_units),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(WorkloadScheduleError::InvalidSlotQuantity {
                    slot_index,
                    name,
                    value,
                });
            }
        }

        if maximum_facility_input_kw <= 0.0 {
            return Err(WorkloadScheduleError::InvalidSlotBudget { slot_index });
        }

        Ok(Self {
            slot_index,
            start_elapsed_ns,
            duration_ns,
            base_non_compute_power_kw,
            base_cooling_power_kw,
            committed_non_grid_supply_kw,
            maximum_facility_input_kw,
            maximum_grid_import_kw,
            maximum_cooling_power_kw,
            maximum_compute_work_units,
        })
    }

    pub const fn slot_index(&self) -> u32 {
        self.slot_index
    }

    pub const fn start_elapsed_ns(&self) -> u64 {
        self.start_elapsed_ns
    }

    pub const fn duration_ns(&self) -> u64 {
        self.duration_ns
    }

    pub const fn base_non_compute_power_kw(&self) -> f64 {
        self.base_non_compute_power_kw
    }

    pub const fn base_cooling_power_kw(&self) -> f64 {
        self.base_cooling_power_kw
    }

    pub const fn committed_non_grid_supply_kw(&self) -> f64 {
        self.committed_non_grid_supply_kw
    }

    pub const fn maximum_facility_input_kw(&self) -> f64 {
        self.maximum_facility_input_kw
    }

    pub const fn maximum_grid_import_kw(&self) -> f64 {
        self.maximum_grid_import_kw
    }

    pub const fn maximum_cooling_power_kw(&self) -> f64 {
        self.maximum_cooling_power_kw
    }

    pub const fn maximum_compute_work_units(&self) -> f64 {
        self.maximum_compute_work_units
    }
}

/// One proposed allocation of useful work to a job and slot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkloadAllocation {
    job_id: String,
    slot_index: u32,
    work_units: f64,
}

impl WorkloadAllocation {
    pub fn new(
        job_id: impl Into<String>,
        slot_index: u32,
        work_units: f64,
    ) -> Result<Self, WorkloadScheduleError> {
        let job_id = job_id.into();
        require_non_empty("allocation job ID", &job_id)?;
        if !work_units.is_finite() || work_units <= 0.0 {
            return Err(WorkloadScheduleError::InvalidAllocation {
                job_id,
                slot_index,
                work_units,
            });
        }

        Ok(Self {
            job_id,
            slot_index,
            work_units,
        })
    }

    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub const fn slot_index(&self) -> u32 {
        self.slot_index
    }

    pub const fn work_units(&self) -> f64 {
        self.work_units
    }
}

/// Frozen schedule proposal and its service/site contracts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkloadSchedule {
    schedule_id: String,
    service_contract_id: String,
    work_unit_id: String,
    jobs: Vec<WorkloadJobContract>,
    slots: Vec<WorkloadScheduleSlot>,
    allocations: Vec<WorkloadAllocation>,
}

impl WorkloadSchedule {
    pub fn new(
        schedule_id: impl Into<String>,
        service_contract_id: impl Into<String>,
        work_unit_id: impl Into<String>,
        jobs: Vec<WorkloadJobContract>,
        slots: Vec<WorkloadScheduleSlot>,
        allocations: Vec<WorkloadAllocation>,
    ) -> Result<Self, WorkloadScheduleError> {
        let schedule_id = schedule_id.into();
        let service_contract_id = service_contract_id.into();
        let work_unit_id = work_unit_id.into();

        require_non_empty("schedule ID", &schedule_id)?;
        require_non_empty("service contract ID", &service_contract_id)?;
        require_non_empty("schedule work unit ID", &work_unit_id)?;

        if jobs.is_empty() || slots.is_empty() || allocations.is_empty() {
            return Err(WorkloadScheduleError::EmptySchedule);
        }

        let schedule = Self {
            schedule_id,
            service_contract_id,
            work_unit_id,
            jobs,
            slots,
            allocations,
        };
        schedule.validate_structure()?;
        Ok(schedule)
    }

    pub fn schedule_id(&self) -> &str {
        &self.schedule_id
    }

    pub fn service_contract_id(&self) -> &str {
        &self.service_contract_id
    }

    pub fn work_unit_id(&self) -> &str {
        &self.work_unit_id
    }

    pub fn jobs(&self) -> &[WorkloadJobContract] {
        &self.jobs
    }

    pub fn slots(&self) -> &[WorkloadScheduleSlot] {
        &self.slots
    }

    pub fn allocations(&self) -> &[WorkloadAllocation] {
        &self.allocations
    }

    fn validate_structure(&self) -> Result<(), WorkloadScheduleError> {
        let mut job_ids = BTreeSet::new();
        for job in &self.jobs {
            if !job_ids.insert(job.job_id().to_string()) {
                return Err(WorkloadScheduleError::DuplicateJob {
                    job_id: job.job_id().to_string(),
                });
            }
            if job.work_unit_id() != self.work_unit_id() {
                return Err(WorkloadScheduleError::WorkUnitMismatch {
                    job_id: job.job_id().to_string(),
                    expected: self.work_unit_id().to_string(),
                    observed: job.work_unit_id().to_string(),
                });
            }
        }

        let mut slot_indices = BTreeSet::new();
        let mut previous_end_ns = None;
        let mut previous_slot_index = None;
        for slot in &self.slots {
            if !slot_indices.insert(slot.slot_index()) {
                return Err(WorkloadScheduleError::DuplicateSlot {
                    slot_index: slot.slot_index(),
                });
            }

            if let Some(previous_index) = previous_slot_index {
                if previous_index.checked_add(1) != Some(slot.slot_index()) {
                    return Err(WorkloadScheduleError::NonContiguousSlots {
                        previous_slot_index: previous_index,
                        next_slot_index: slot.slot_index(),
                    });
                }
            }
            if let Some(previous_end) = previous_end_ns {
                if slot.start_elapsed_ns() != previous_end {
                    return Err(WorkloadScheduleError::NonContiguousSlotTime {
                        expected_start_ns: previous_end,
                        observed_start_ns: slot.start_elapsed_ns(),
                    });
                }
            }

            previous_slot_index = Some(slot.slot_index());
            previous_end_ns = Some(
                slot.start_elapsed_ns()
                    .checked_add(slot.duration_ns())
                    .ok_or(WorkloadScheduleError::SlotTimeOverflow {
                        slot_index: slot.slot_index(),
                    })?,
            );
        }

        let horizon_start = self.slots[0].slot_index();
        let horizon_end = self.slots[self.slots.len() - 1]
            .slot_index()
            .checked_add(1)
            .ok_or(WorkloadScheduleError::SlotIndexOverflow)?;
        if self.jobs.iter().any(|job| {
            job.release_slot() >= horizon_end
                || job.deadline_slot_exclusive() > horizon_end
                || job.deadline_slot_exclusive() <= horizon_start
        }) {
            return Err(WorkloadScheduleError::JobWindowOutsideHorizon);
        }

        Ok(())
    }
}

/// Numerical tolerance for work-unit and power-budget comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct WorkloadScheduleTolerance {
    absolute_work_units: f64,
    absolute_power_kw: f64,
}

impl WorkloadScheduleTolerance {
    pub fn new(
        absolute_work_units: f64,
        absolute_power_kw: f64,
    ) -> Result<Self, WorkloadScheduleError> {
        for (name, value) in [
            ("absolute work tolerance", absolute_work_units),
            ("absolute power tolerance", absolute_power_kw),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(WorkloadScheduleError::InvalidTolerance { name, value });
            }
        }
        Ok(Self {
            absolute_work_units,
            absolute_power_kw,
        })
    }
}

/// Independently reconstructed slot load and feasibility receipt.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkloadSlotVerification {
    slot_index: u32,
    compute_work_units: f64,
    it_power_kw: f64,
    cooling_power_kw: f64,
    checkpoint_overhead_kwh: f64,
    checkpoint_power_kw: f64,
    facility_input_power_kw: f64,
    grid_import_power_kw: f64,
    facility_energy_kwh: f64,
    grid_import_energy_kwh: f64,
    passed: bool,
}

impl WorkloadSlotVerification {
    pub const fn slot_index(&self) -> u32 {
        self.slot_index
    }

    pub const fn compute_work_units(&self) -> f64 {
        self.compute_work_units
    }

    pub const fn it_power_kw(&self) -> f64 {
        self.it_power_kw
    }

    pub const fn cooling_power_kw(&self) -> f64 {
        self.cooling_power_kw
    }

    pub const fn checkpoint_overhead_kwh(&self) -> f64 {
        self.checkpoint_overhead_kwh
    }

    pub const fn checkpoint_power_kw(&self) -> f64 {
        self.checkpoint_power_kw
    }

    pub const fn facility_input_power_kw(&self) -> f64 {
        self.facility_input_power_kw
    }

    pub const fn grid_import_power_kw(&self) -> f64 {
        self.grid_import_power_kw
    }

    pub const fn passed(&self) -> bool {
        self.passed
    }
}

/// Complete schedule verification receipt.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkloadScheduleVerificationReport {
    schedule_id: String,
    service_contract_id: String,
    work_unit_id: String,
    job_count: usize,
    slot_count: usize,
    allocation_count: usize,
    completed_work_units: f64,
    total_facility_energy_kwh: f64,
    total_grid_import_energy_kwh: f64,
    total_cooling_energy_kwh: f64,
    checkpoint_overhead_energy_kwh: f64,
    peak_facility_input_kw: f64,
    peak_grid_import_kw: f64,
    peak_cooling_power_kw: f64,
    slot_receipts: Vec<WorkloadSlotVerification>,
    passed: bool,
}

impl WorkloadScheduleVerificationReport {
    pub fn schedule_id(&self) -> &str {
        &self.schedule_id
    }

    pub fn service_contract_id(&self) -> &str {
        &self.service_contract_id
    }

    pub fn work_unit_id(&self) -> &str {
        &self.work_unit_id
    }

    pub const fn job_count(&self) -> usize {
        self.job_count
    }

    pub const fn slot_count(&self) -> usize {
        self.slot_count
    }

    pub const fn allocation_count(&self) -> usize {
        self.allocation_count
    }

    pub const fn completed_work_units(&self) -> f64 {
        self.completed_work_units
    }

    pub const fn total_facility_energy_kwh(&self) -> f64 {
        self.total_facility_energy_kwh
    }

    pub const fn total_grid_import_energy_kwh(&self) -> f64 {
        self.total_grid_import_energy_kwh
    }

    pub const fn total_cooling_energy_kwh(&self) -> f64 {
        self.total_cooling_energy_kwh
    }

    pub const fn checkpoint_overhead_energy_kwh(&self) -> f64 {
        self.checkpoint_overhead_energy_kwh
    }

    pub const fn peak_facility_input_kw(&self) -> f64 {
        self.peak_facility_input_kw
    }

    pub const fn peak_grid_import_kw(&self) -> f64 {
        self.peak_grid_import_kw
    }

    pub const fn peak_cooling_power_kw(&self) -> f64 {
        self.peak_cooling_power_kw
    }

    pub fn slot_receipts(&self) -> &[WorkloadSlotVerification] {
        &self.slot_receipts
    }

    pub const fn passed(&self) -> bool {
        self.passed
    }
}

/// Reconstruct and verify a proposed workload schedule.
///
/// This is deliberately an independent feasibility checker, not a planner.
/// It verifies every frozen job and slot contract, including release/deadline
/// windows, preemption policy, compute capacity, facility input, residual grid
/// import, cooling and checkpoint energy.
pub fn verify_workload_schedule(
    schedule: &WorkloadSchedule,
    tolerance: WorkloadScheduleTolerance,
) -> Result<WorkloadScheduleVerificationReport, WorkloadScheduleError> {
    let jobs = schedule
        .jobs()
        .iter()
        .map(|job| (job.job_id(), job))
        .collect::<BTreeMap<_, _>>();
    let slots = schedule
        .slots()
        .iter()
        .map(|slot| (slot.slot_index(), slot))
        .collect::<BTreeMap<_, _>>();

    let mut allocations_by_job = BTreeMap::<String, Vec<&WorkloadAllocation>>::new();
    let mut allocations_by_slot = BTreeMap::<u32, Vec<&WorkloadAllocation>>::new();
    let mut unique_job_slot_pairs = BTreeSet::new();

    for allocation in schedule.allocations() {
        let job = jobs.get(allocation.job_id()).ok_or_else(|| {
            WorkloadScheduleError::UnknownJob {
                job_id: allocation.job_id().to_string(),
            }
        })?;
        if !slots.contains_key(&allocation.slot_index()) {
            return Err(WorkloadScheduleError::UnknownSlot {
                slot_index: allocation.slot_index(),
                job_id: allocation.job_id().to_string(),
            });
        }
        if allocation.slot_index() < job.release_slot()
            || allocation.slot_index() >= job.deadline_slot_exclusive()
        {
            return Err(WorkloadScheduleError::AllocationOutsideJobWindow {
                job_id: allocation.job_id().to_string(),
                slot_index: allocation.slot_index(),
                release_slot: job.release_slot(),
                deadline_slot_exclusive: job.deadline_slot_exclusive(),
            });
        }
        if allocation.work_units() > job.maximum_work_units_per_slot() + tolerance.absolute_work_units
        {
            return Err(WorkloadScheduleError::JobSlotWorkExceeded {
                job_id: allocation.job_id().to_string(),
                slot_index: allocation.slot_index(),
                observed_work_units: allocation.work_units(),
                maximum_work_units: job.maximum_work_units_per_slot(),
            });
        }
        if !unique_job_slot_pairs.insert((allocation.job_id().to_string(), allocation.slot_index()))
        {
            return Err(WorkloadScheduleError::DuplicateJobSlotAllocation {
                job_id: allocation.job_id().to_string(),
                slot_index: allocation.slot_index(),
            });
        }

        allocations_by_job
            .entry(allocation.job_id().to_string())
            .or_default()
            .push(allocation);
        allocations_by_slot
            .entry(allocation.slot_index())
            .or_default()
            .push(allocation);
    }

    // Verify completion and preemption policy independently for each job.
    let mut checkpoint_overhead_by_slot = BTreeMap::<u32, f64>::new();
    let mut completed_work_units = 0.0;

    for job in schedule.jobs() {
        let mut allocations = allocations_by_job.remove(job.job_id()).unwrap_or_default();
        allocations.sort_by_key(|allocation| allocation.slot_index());

        let allocated_work_units = allocations
            .iter()
            .map(|allocation| allocation.work_units())
            .sum::<f64>();

        if !allocated_work_units.is_finite() {
            return Err(WorkloadScheduleError::WorkOverflow {
                job_id: job.job_id().to_string(),
            });
        }
        if (allocated_work_units - job.required_work_units()).abs()
            > tolerance.absolute_work_units
        {
            return Err(WorkloadScheduleError::JobWorkMismatch {
                job_id: job.job_id().to_string(),
                required_work_units: job.required_work_units(),
                allocated_work_units,
            });
        }

        match job.preemption_policy() {
            WorkloadPreemptionPolicy::NonPreemptible => {
                for pair in allocations.windows(2) {
                    if pair[1].slot_index() != pair[0].slot_index() + 1 {
                        return Err(WorkloadScheduleError::NonPreemptibleJobFragmented {
                            job_id: job.job_id().to_string(),
                            previous_slot: pair[0].slot_index(),
                            next_slot: pair[1].slot_index(),
                        });
                    }
                }
            }
            WorkloadPreemptionPolicy::Checkpointable => {
                for pair in allocations.windows(2) {
                    if pair[1].slot_index() > pair[0].slot_index() + 1 {
                        let entry = checkpoint_overhead_by_slot
                            .entry(pair[1].slot_index())
                            .or_default();
                        *entry += job.checkpoint_resume_overhead_kwh();
                    }
                }
            }
            WorkloadPreemptionPolicy::Deferrable => {}
        }

        completed_work_units += allocated_work_units;
        if !completed_work_units.is_finite() {
            return Err(WorkloadScheduleError::WorkOverflow {
                job_id: job.job_id().to_string(),
            });
        }
    }

    // Independently reconstruct every slot's compute, cooling and facility loads.
    let mut slot_receipts = Vec::with_capacity(schedule.slots().len());
    let mut total_facility_energy_kwh = 0.0;
    let mut total_grid_import_energy_kwh = 0.0;
    let mut total_cooling_energy_kwh = 0.0;
    let mut checkpoint_overhead_energy_kwh = 0.0;
    let mut peak_facility_input_kw: f64 = 0.0;
    let mut peak_grid_import_kw: f64 = 0.0;
    let mut peak_cooling_power_kw: f64 = 0.0;

    for slot in schedule.slots() {
        let allocations = allocations_by_slot
            .get(&slot.slot_index())
            .cloned()
            .unwrap_or_default();

        let mut compute_work_units = 0.0;
        let mut it_power_kw = 0.0;
        let mut incremental_cooling_kw = 0.0;

        for allocation in allocations {
            let job = jobs[allocation.job_id()];
            compute_work_units += allocation.work_units();
            it_power_kw +=
                allocation.work_units() * job.it_power_kw_per_work_unit();
            incremental_cooling_kw +=
                allocation.work_units() * job.cooling_power_kw_per_work_unit();
        }

        let checkpoint_overhead_kwh = checkpoint_overhead_by_slot
            .get(&slot.slot_index())
            .copied()
            .unwrap_or(0.0);
        let duration_hours = slot.duration_ns() as f64 / 3_600_000_000_000.0;
        let checkpoint_power_kw = checkpoint_overhead_kwh / duration_hours;
        let cooling_power_kw = slot.base_cooling_power_kw() + incremental_cooling_kw;
        let facility_input_power_kw = slot.base_non_compute_power_kw()
            + cooling_power_kw
            + it_power_kw
            + checkpoint_power_kw;
        let grid_import_power_kw =
            (facility_input_power_kw - slot.committed_non_grid_supply_kw()).max(0.0);

        for (name, value) in [
            ("slot compute work", compute_work_units),
            ("slot IT power", it_power_kw),
            ("slot cooling power", cooling_power_kw),
            ("slot checkpoint power", checkpoint_power_kw),
            ("slot facility input power", facility_input_power_kw),
            ("slot grid import power", grid_import_power_kw),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(WorkloadScheduleError::InvalidDerivedSlotValue {
                    slot_index: slot.slot_index(),
                    name,
                    value,
                });
            }
        }

        if compute_work_units
            > slot.maximum_compute_work_units() + tolerance.absolute_work_units
        {
            return Err(WorkloadScheduleError::ComputeCapacityExceeded {
                slot_index: slot.slot_index(),
                observed_work_units: compute_work_units,
                maximum_work_units: slot.maximum_compute_work_units(),
            });
        }
        if facility_input_power_kw
            > slot.maximum_facility_input_kw() + tolerance.absolute_power_kw
        {
            return Err(WorkloadScheduleError::FacilityInputLimitExceeded {
                slot_index: slot.slot_index(),
                observed_kw: facility_input_power_kw,
                maximum_kw: slot.maximum_facility_input_kw(),
            });
        }
        if grid_import_power_kw > slot.maximum_grid_import_kw() + tolerance.absolute_power_kw {
            return Err(WorkloadScheduleError::GridImportLimitExceeded {
                slot_index: slot.slot_index(),
                observed_kw: grid_import_power_kw,
                maximum_kw: slot.maximum_grid_import_kw(),
            });
        }
        if cooling_power_kw > slot.maximum_cooling_power_kw() + tolerance.absolute_power_kw {
            return Err(WorkloadScheduleError::CoolingLimitExceeded {
                slot_index: slot.slot_index(),
                observed_kw: cooling_power_kw,
                maximum_kw: slot.maximum_cooling_power_kw(),
            });
        }

        let facility_energy_kwh = facility_input_power_kw * duration_hours;
        let grid_import_energy_kwh = grid_import_power_kw * duration_hours;
        let cooling_energy_kwh = cooling_power_kw * duration_hours;
        if !facility_energy_kwh.is_finite()
            || !grid_import_energy_kwh.is_finite()
            || !cooling_energy_kwh.is_finite()
        {
            return Err(WorkloadScheduleError::EnergyOverflow {
                slot_index: slot.slot_index(),
            });
        }

        total_facility_energy_kwh += facility_energy_kwh;
        total_grid_import_energy_kwh += grid_import_energy_kwh;
        total_cooling_energy_kwh += cooling_energy_kwh;
        checkpoint_overhead_energy_kwh += checkpoint_overhead_kwh;

        peak_facility_input_kw = peak_facility_input_kw.max(facility_input_power_kw);
        peak_grid_import_kw = peak_grid_import_kw.max(grid_import_power_kw);
        peak_cooling_power_kw = peak_cooling_power_kw.max(cooling_power_kw);

        slot_receipts.push(WorkloadSlotVerification {
            slot_index: slot.slot_index(),
            compute_work_units,
            it_power_kw,
            cooling_power_kw,
            checkpoint_overhead_kwh,
            checkpoint_power_kw,
            facility_input_power_kw,
            grid_import_power_kw,
            facility_energy_kwh,
            grid_import_energy_kwh,
            passed: true,
        });
    }

    for (name, value) in [
        ("total facility energy", total_facility_energy_kwh),
        ("total grid import energy", total_grid_import_energy_kwh),
        ("total cooling energy", total_cooling_energy_kwh),
        ("total checkpoint overhead", checkpoint_overhead_energy_kwh),
        ("peak facility input", peak_facility_input_kw),
        ("peak grid import", peak_grid_import_kw),
        ("peak cooling", peak_cooling_power_kw),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(WorkloadScheduleError::InvalidDerivedTotal { name, value });
        }
    }

    Ok(WorkloadScheduleVerificationReport {
        schedule_id: schedule.schedule_id().to_string(),
        service_contract_id: schedule.service_contract_id().to_string(),
        work_unit_id: schedule.work_unit_id().to_string(),
        job_count: schedule.jobs().len(),
        slot_count: schedule.slots().len(),
        allocation_count: schedule.allocations().len(),
        completed_work_units,
        total_facility_energy_kwh,
        total_grid_import_energy_kwh,
        total_cooling_energy_kwh,
        checkpoint_overhead_energy_kwh,
        peak_facility_input_kw,
        peak_grid_import_kw,
        peak_cooling_power_kw,
        slot_receipts,
        passed: true,
    })
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), WorkloadScheduleError> {
    if value.trim().is_empty() {
        return Err(WorkloadScheduleError::MissingIdentity { name });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkloadScheduleError {
    MissingIdentity {
        name: &'static str,
    },
    EmptySchedule,
    InvalidJobWindow {
        job_id: String,
        release_slot: u32,
        deadline_slot_exclusive: u32,
    },
    InvalidJobQuantity {
        name: &'static str,
        value: f64,
    },
    UnexpectedCheckpointOverhead {
        job_id: String,
        policy: WorkloadPreemptionPolicy,
    },
    InvalidSlotDuration {
        slot_index: u32,
        duration_ns: u64,
    },
    InvalidSlotQuantity {
        slot_index: u32,
        name: &'static str,
        value: f64,
    },
    InvalidSlotBudget {
        slot_index: u32,
    },
    InvalidAllocation {
        job_id: String,
        slot_index: u32,
        work_units: f64,
    },
    DuplicateJob {
        job_id: String,
    },
    WorkUnitMismatch {
        job_id: String,
        expected: String,
        observed: String,
    },
    DuplicateSlot {
        slot_index: u32,
    },
    NonContiguousSlots {
        previous_slot_index: u32,
        next_slot_index: u32,
    },
    SlotIndexOverflow,
    NonContiguousSlotTime {
        expected_start_ns: u64,
        observed_start_ns: u64,
    },
    SlotTimeOverflow {
        slot_index: u32,
    },
    JobWindowOutsideHorizon,
    InvalidTolerance {
        name: &'static str,
        value: f64,
    },
    UnknownJob {
        job_id: String,
    },
    UnknownSlot {
        slot_index: u32,
        job_id: String,
    },
    AllocationOutsideJobWindow {
        job_id: String,
        slot_index: u32,
        release_slot: u32,
        deadline_slot_exclusive: u32,
    },
    JobSlotWorkExceeded {
        job_id: String,
        slot_index: u32,
        observed_work_units: f64,
        maximum_work_units: f64,
    },
    DuplicateJobSlotAllocation {
        job_id: String,
        slot_index: u32,
    },
    WorkOverflow {
        job_id: String,
    },
    JobWorkMismatch {
        job_id: String,
        required_work_units: f64,
        allocated_work_units: f64,
    },
    NonPreemptibleJobFragmented {
        job_id: String,
        previous_slot: u32,
        next_slot: u32,
    },
    InvalidDerivedSlotValue {
        slot_index: u32,
        name: &'static str,
        value: f64,
    },
    ComputeCapacityExceeded {
        slot_index: u32,
        observed_work_units: f64,
        maximum_work_units: f64,
    },
    FacilityInputLimitExceeded {
        slot_index: u32,
        observed_kw: f64,
        maximum_kw: f64,
    },
    GridImportLimitExceeded {
        slot_index: u32,
        observed_kw: f64,
        maximum_kw: f64,
    },
    CoolingLimitExceeded {
        slot_index: u32,
        observed_kw: f64,
        maximum_kw: f64,
    },
    EnergyOverflow {
        slot_index: u32,
    },
    InvalidDerivedTotal {
        name: &'static str,
        value: f64,
    },
}

impl fmt::Display for WorkloadScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { name } => write!(f, "{name} must not be empty"),
            Self::EmptySchedule => {
                write!(f, "schedule requires jobs, slots and allocations")
            }
            Self::InvalidJobWindow {
                job_id,
                release_slot,
                deadline_slot_exclusive,
            } => write!(
                f,
                "job {job_id} has invalid window [{release_slot}, {deadline_slot_exclusive})"
            ),
            Self::InvalidJobQuantity { name, value } => {
                write!(f, "job {name} must be finite and in range; got {value}")
            }
            Self::UnexpectedCheckpointOverhead { job_id, policy } => write!(
                f,
                "job {job_id} has checkpoint overhead but policy is {policy:?}"
            ),
            Self::InvalidSlotDuration {
                slot_index,
                duration_ns,
            } => write!(
                f,
                "slot {slot_index} duration must be positive; got {duration_ns} ns"
            ),
            Self::InvalidSlotQuantity {
                slot_index,
                name,
                value,
            } => write!(
                f,
                "slot {slot_index} {name} must be finite and non-negative; got {value}"
            ),
            Self::InvalidSlotBudget { slot_index } => {
                write!(f, "slot {slot_index} facility and compute capacity must be positive")
            }
            Self::InvalidAllocation {
                job_id,
                slot_index,
                work_units,
            } => write!(
                f,
                "allocation for {job_id} in slot {slot_index} has invalid work {work_units}"
            ),
            Self::DuplicateJob { job_id } => write!(f, "duplicate job ID {job_id}"),
            Self::WorkUnitMismatch {
                job_id,
                expected,
                observed,
            } => write!(
                f,
                "job {job_id} uses work unit {observed}; schedule requires {expected}"
            ),
            Self::DuplicateSlot { slot_index } => {
                write!(f, "duplicate scheduling slot {slot_index}")
            }
            Self::NonContiguousSlots {
                previous_slot_index,
                next_slot_index,
            } => write!(
                f,
                "slot index gap: previous {previous_slot_index}, next {next_slot_index}"
            ),
            Self::SlotIndexOverflow => {
                write!(f, "last scheduling slot index cannot be incremented")
            }
            Self::NonContiguousSlotTime {
                expected_start_ns,
                observed_start_ns,
            } => write!(
                f,
                "slot time gap: expected start {expected_start_ns}, observed {observed_start_ns}"
            ),
            Self::SlotTimeOverflow { slot_index } => {
                write!(f, "slot {slot_index} end timestamp overflowed")
            }
            Self::JobWindowOutsideHorizon => {
                write!(f, "job release/deadline window falls outside schedule horizon")
            }
            Self::InvalidTolerance { name, value } => {
                write!(f, "{name} must be finite and non-negative; got {value}")
            }
            Self::UnknownJob { job_id } => write!(f, "allocation references unknown job {job_id}"),
            Self::UnknownSlot {
                slot_index,
                job_id,
            } => write!(
                f,
                "allocation for {job_id} references unknown slot {slot_index}"
            ),
            Self::AllocationOutsideJobWindow {
                job_id,
                slot_index,
                release_slot,
                deadline_slot_exclusive,
            } => write!(
                f,
                "job {job_id} assigned slot {slot_index} outside [{release_slot}, {deadline_slot_exclusive})"
            ),
            Self::JobSlotWorkExceeded {
                job_id,
                slot_index,
                observed_work_units,
                maximum_work_units,
            } => write!(
                f,
                "job {job_id} in slot {slot_index} allocated {observed_work_units}; max {maximum_work_units}"
            ),
            Self::DuplicateJobSlotAllocation { job_id, slot_index } => {
                write!(f, "job {job_id} has multiple allocations in slot {slot_index}")
            }
            Self::WorkOverflow { job_id } => {
                write!(f, "work total overflowed for job {job_id}")
            }
            Self::JobWorkMismatch {
                job_id,
                required_work_units,
                allocated_work_units,
            } => write!(
                f,
                "job {job_id} requires {required_work_units} work units; schedule allocates {allocated_work_units}"
            ),
            Self::NonPreemptibleJobFragmented {
                job_id,
                previous_slot,
                next_slot,
            } => write!(
                f,
                "non-preemptible job {job_id} is fragmented between slots {previous_slot} and {next_slot}"
            ),
            Self::InvalidDerivedSlotValue {
                slot_index,
                name,
                value,
            } => write!(
                f,
                "slot {slot_index} derived {name} is invalid: {value}"
            ),
            Self::ComputeCapacityExceeded {
                slot_index,
                observed_work_units,
                maximum_work_units,
            } => write!(
                f,
                "slot {slot_index} has {observed_work_units} work units; max {maximum_work_units}"
            ),
            Self::FacilityInputLimitExceeded {
                slot_index,
                observed_kw,
                maximum_kw,
            } => write!(
                f,
                "slot {slot_index} facility input {observed_kw} kW exceeds {maximum_kw} kW"
            ),
            Self::GridImportLimitExceeded {
                slot_index,
                observed_kw,
                maximum_kw,
            } => write!(
                f,
                "slot {slot_index} grid import {observed_kw} kW exceeds {maximum_kw} kW"
            ),
            Self::CoolingLimitExceeded {
                slot_index,
                observed_kw,
                maximum_kw,
            } => write!(
                f,
                "slot {slot_index} cooling {observed_kw} kW exceeds {maximum_kw} kW"
            ),
            Self::EnergyOverflow { slot_index } => {
                write!(f, "energy integration overflowed in slot {slot_index}")
            }
            Self::InvalidDerivedTotal { name, value } => {
                write!(f, "derived schedule total {name} is invalid: {value}")
            }
        }
    }
}

impl std::error::Error for WorkloadScheduleError {}

#[cfg(test)]
mod tests {
    use super::*;

    const SLOT_NS: u64 = 3_600_000_000_000;

    fn slot(index: u32, facility_cap: f64, grid_cap: f64, cooling_cap: f64) -> WorkloadScheduleSlot {
        WorkloadScheduleSlot::new(
            index,
            index as u64 * SLOT_NS,
            SLOT_NS,
            200.0,
            50.0,
            100.0,
            facility_cap,
            grid_cap,
            cooling_cap,
            100.0,
        )
        .unwrap()
    }

    fn job(
        id: &str,
        required: f64,
        policy: WorkloadPreemptionPolicy,
        checkpoint_overhead_kwh: f64,
    ) -> WorkloadJobContract {
        WorkloadJobContract::new(
            id,
            "training-steps-v1",
            0,
            4,
            required,
            20.0,
            10.0,
            2.0,
            policy,
            checkpoint_overhead_kwh,
        )
        .unwrap()
    }

    fn allocation(job_id: &str, slot_index: u32, work: f64) -> WorkloadAllocation {
        WorkloadAllocation::new(job_id, slot_index, work).unwrap()
    }

    fn tolerance() -> WorkloadScheduleTolerance {
        WorkloadScheduleTolerance::new(1e-9, 1e-9).unwrap()
    }

    fn valid_schedule() -> WorkloadSchedule {
        WorkloadSchedule::new(
            "SCHEDULE-VALID",
            "SLA-BASELINE-V1",
            "training-steps-v1",
            vec![
                job("JOB-A", 20.0, WorkloadPreemptionPolicy::Deferrable, 0.0),
                job("JOB-B", 20.0, WorkloadPreemptionPolicy::NonPreemptible, 0.0),
            ],
            vec![
                slot(0, 600.0, 500.0, 100.0),
                slot(1, 600.0, 500.0, 100.0),
                slot(2, 600.0, 500.0, 100.0),
                slot(3, 600.0, 500.0, 100.0),
            ],
            vec![
                allocation("JOB-A", 0, 10.0),
                allocation("JOB-A", 2, 10.0),
                allocation("JOB-B", 1, 20.0),
            ],
        )
        .unwrap()
    }

    #[test]
    fn verifies_schedule_and_reconstructs_facility_grid_and_cooling_energy() {
        let report = verify_workload_schedule(&valid_schedule(), tolerance()).unwrap();

        assert!(report.passed());
        assert_eq!(report.job_count(), 2);
        assert_eq!(report.slot_count(), 4);
        assert_eq!(report.allocation_count(), 3);
        assert_eq!(report.completed_work_units(), 40.0);
        assert_eq!(report.peak_facility_input_kw(), 490.0);
        assert_eq!(report.peak_grid_import_kw(), 390.0);
        assert_eq!(report.peak_cooling_power_kw(), 90.0);
        assert!(report.total_facility_energy_kwh() > report.total_grid_import_energy_kwh());
    }

    #[test]
    fn rejects_allocations_outside_deadline_and_missing_work() {
        let late = WorkloadSchedule::new(
            "SCHEDULE-LATE",
            "SLA-BASELINE-V1",
            "training-steps-v1",
            vec![WorkloadJobContract::new(
                "JOB-LATE",
                "training-steps-v1",
                0,
                2,
                10.0,
                10.0,
                10.0,
                2.0,
                WorkloadPreemptionPolicy::Deferrable,
                0.0,
            )
            .unwrap()],
            vec![
                slot(0, 600.0, 500.0, 100.0),
                slot(1, 600.0, 500.0, 100.0),
                slot(2, 600.0, 500.0, 100.0),
                slot(3, 600.0, 500.0, 100.0),
            ],
            vec![allocation("JOB-LATE", 2, 10.0)],
        )
        .unwrap();
        assert!(matches!(
            verify_workload_schedule(&late, tolerance()),
            Err(WorkloadScheduleError::AllocationOutsideJobWindow { .. })
        ));

        let missing = WorkloadSchedule::new(
            "SCHEDULE-MISSING",
            "SLA-BASELINE-V1",
            "training-steps-v1",
            vec![job("JOB-A", 20.0, WorkloadPreemptionPolicy::Deferrable, 0.0)],
            vec![slot(0, 600.0, 500.0, 100.0), slot(1, 600.0, 500.0, 100.0)],
            vec![allocation("JOB-A", 0, 10.0)],
        )
        .unwrap();
        assert!(matches!(
            verify_workload_schedule(&missing, tolerance()),
            Err(WorkloadScheduleError::JobWorkMismatch { .. })
        ));
    }

    #[test]
    fn rejects_non_preemptible_job_fragmentation() {
        let schedule = WorkloadSchedule::new(
            "SCHEDULE-FRAGMENTED",
            "SLA-BASELINE-V1",
            "training-steps-v1",
            vec![job("JOB-A", 20.0, WorkloadPreemptionPolicy::NonPreemptible, 0.0)],
            vec![
                slot(0, 600.0, 500.0, 100.0),
                slot(1, 600.0, 500.0, 100.0),
                slot(2, 600.0, 500.0, 100.0),
            ],
            vec![allocation("JOB-A", 0, 10.0), allocation("JOB-A", 2, 10.0)],
        )
        .unwrap();

        assert!(matches!(
            verify_workload_schedule(&schedule, tolerance()),
            Err(WorkloadScheduleError::NonPreemptibleJobFragmented { .. })
        ));
    }

    #[test]
    fn checkpointable_pause_adds_explicit_resume_energy() {
        let schedule = WorkloadSchedule::new(
            "SCHEDULE-CHECKPOINT",
            "SLA-BASELINE-V1",
            "training-steps-v1",
            vec![job("JOB-A", 20.0, WorkloadPreemptionPolicy::Checkpointable, 5.0)],
            vec![
                slot(0, 600.0, 500.0, 100.0),
                slot(1, 600.0, 500.0, 100.0),
                slot(2, 600.0, 500.0, 100.0),
            ],
            vec![allocation("JOB-A", 0, 10.0), allocation("JOB-A", 2, 10.0)],
        )
        .unwrap();

        let report = verify_workload_schedule(&schedule, tolerance()).unwrap();
        assert_eq!(report.checkpoint_overhead_energy_kwh(), 5.0);
        assert!((report.slot_receipts()[2].checkpoint_power_kw() - 5.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_grid_facility_and_cooling_budget_violations() {
        let facility_limited = WorkloadSchedule::new(
            "SCHEDULE-FACILITY-LIMIT",
            "SLA-BASELINE-V1",
            "training-steps-v1",
            vec![job("JOB-A", 20.0, WorkloadPreemptionPolicy::Deferrable, 0.0)],
            vec![slot(0, 400.0, 500.0, 100.0), slot(1, 600.0, 500.0, 100.0)],
            vec![allocation("JOB-A", 0, 20.0)],
        )
        .unwrap();
        assert!(matches!(
            verify_workload_schedule(&facility_limited, tolerance()),
            Err(WorkloadScheduleError::FacilityInputLimitExceeded { .. })
        ));

        let grid_limited = WorkloadSchedule::new(
            "SCHEDULE-GRID-LIMIT",
            "SLA-BASELINE-V1",
            "training-steps-v1",
            vec![job("JOB-A", 20.0, WorkloadPreemptionPolicy::Deferrable, 0.0)],
            vec![slot(0, 600.0, 350.0, 100.0), slot(1, 600.0, 500.0, 100.0)],
            vec![allocation("JOB-A", 0, 20.0)],
        )
        .unwrap();
        assert!(matches!(
            verify_workload_schedule(&grid_limited, tolerance()),
            Err(WorkloadScheduleError::GridImportLimitExceeded { .. })
        ));

        let cooling_limited = WorkloadSchedule::new(
            "SCHEDULE-COOLING-LIMIT",
            "SLA-BASELINE-V1",
            "training-steps-v1",
            vec![job("JOB-A", 20.0, WorkloadPreemptionPolicy::Deferrable, 0.0)],
            vec![slot(0, 600.0, 500.0, 80.0), slot(1, 600.0, 500.0, 100.0)],
            vec![allocation("JOB-A", 0, 20.0)],
        )
        .unwrap();
        assert!(matches!(
            verify_workload_schedule(&cooling_limited, tolerance()),
            Err(WorkloadScheduleError::CoolingLimitExceeded { .. })
        ));
    }

    #[test]
    fn allows_zero_compute_capacity_slots_and_jobs_released_before_horizon() {
        let zero_compute_slot = WorkloadScheduleSlot::new(
            0,
            0,
            SLOT_NS,
            200.0,
            50.0,
            100.0,
            600.0,
            500.0,
            100.0,
            0.0,
        )
        .unwrap();
        let active_slot = slot(1, 600.0, 500.0, 100.0);
        let job = WorkloadJobContract::new(
            "JOB-ALREADY-RELEASED",
            "training-steps-v1",
            0,
            2,
            10.0,
            10.0,
            10.0,
            2.0,
            WorkloadPreemptionPolicy::Deferrable,
            0.0,
        )
        .unwrap();
        let schedule = WorkloadSchedule::new(
            "SCHEDULE-IDLE-SLOT",
            "SLA-BASELINE-V1",
            "training-steps-v1",
            vec![job],
            vec![zero_compute_slot, active_slot],
            vec![allocation("JOB-ALREADY-RELEASED", 1, 10.0)],
        )
        .unwrap();

        let report = verify_workload_schedule(&schedule, tolerance()).unwrap();
        assert!(report.passed());
        assert_eq!(report.slot_receipts()[0].compute_work_units(), 0.0);
        assert_eq!(report.completed_work_units(), 10.0);
    }

    #[test]
    fn rejects_duplicate_allocations_slot_identity_and_wrong_work_unit() {
        let duplicate = WorkloadSchedule::new(
            "SCHEDULE-DUP-ALLOC",
            "SLA-BASELINE-V1",
            "training-steps-v1",
            vec![job("JOB-A", 20.0, WorkloadPreemptionPolicy::Deferrable, 0.0)],
            vec![slot(0, 600.0, 500.0, 100.0), slot(1, 600.0, 500.0, 100.0)],
            vec![allocation("JOB-A", 0, 10.0), allocation("JOB-A", 0, 10.0)],
        )
        .unwrap();
        assert!(matches!(
            verify_workload_schedule(&duplicate, tolerance()),
            Err(WorkloadScheduleError::DuplicateJobSlotAllocation { .. })
        ));

        let wrong_unit = WorkloadSchedule::new(
            "SCHEDULE-WRONG-UNIT",
            "SLA-BASELINE-V1",
            "training-steps-v1",
            vec![WorkloadJobContract::new(
                "JOB-BAD-UNIT",
                "tokens",
                0,
                2,
                10.0,
                10.0,
                1.0,
                1.0,
                WorkloadPreemptionPolicy::Deferrable,
                0.0,
            )
            .unwrap()],
            vec![slot(0, 600.0, 500.0, 100.0), slot(1, 600.0, 500.0, 100.0)],
            vec![allocation("JOB-BAD-UNIT", 0, 10.0)],
        );
        assert!(matches!(
            wrong_unit,
            Err(WorkloadScheduleError::WorkUnitMismatch { .. })
        ));
    }

    #[test]
    fn rejects_invalid_contracts_and_checkpoint_policy_mismatch() {
        assert!(WorkloadScheduleTolerance::new(-1.0, 0.0).is_err());
        assert!(WorkloadJobContract::new(
            "JOB-A",
            "training-steps-v1",
            2,
            2,
            10.0,
            10.0,
            1.0,
            1.0,
            WorkloadPreemptionPolicy::Deferrable,
            1.0
        )
        .is_err());
        assert!(WorkloadScheduleSlot::new(
            0, 0, 0, 0.0, 0.0, 0.0, 10.0, 10.0, 10.0, 10.0
        )
        .is_err());
    }
}
