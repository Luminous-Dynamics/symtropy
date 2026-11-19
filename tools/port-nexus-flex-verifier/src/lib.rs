//! Standalone, deterministic admission verifier for port-compute workload shifts.
//!
//! This module is intentionally a verifier, not a scheduler. It checks that a
//! candidate placement preserves the exact job set and useful-work manifest,
//! reconstructs full-horizon PCC and feeder trajectories, and rejects
//! new-peak, import-envelope, feeder-envelope, and ramp violations.
//!
//! The verifier accepts caller-supplied base net-import traces and limits. It
//! does not authenticate those inputs, validate a network model, infer a
//! physical grid state, prove voltage/frequency stability, model storage or
//! cooling dynamics, or authorize dispatch.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct WorkloadJob {
    pub job_id: String,
    pub feeder_id: String,
    pub power_kw: f64,
    pub duration_slots: usize,
    pub release_slot: usize,
    /// Exclusive upper bound: the job must finish no later than this slot.
    pub deadline_exclusive: usize,
    pub useful_work_units: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobPlacement {
    pub job_id: String,
    pub start_slot: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeederEnvelope {
    pub feeder_id: String,
    /// Net feeder import before the flexible jobs in this manifest.
    pub base_net_import_kw: Vec<f64>,
    /// Optional upper import limit for each interval.
    pub max_import_kw: Vec<Option<f64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdmissionContract {
    pub study_id: String,
    pub pcc_boundary_id: String,
    pub interval_hours: f64,
    /// PCC net import before the flexible jobs in this manifest.
    /// Negative values represent net export.
    pub base_pcc_net_import_kw: Vec<f64>,
    /// Optional upper import limit for each interval.
    pub pcc_max_import_kw: Vec<Option<f64>>,
    /// Optional maximum absolute PCC change between adjacent intervals.
    /// Length must be slot_count - 1.
    pub pcc_max_abs_ramp_kw: Vec<Option<f64>>,
    pub feeders: Vec<FeederEnvelope>,
    pub require_no_new_peak: bool,
    pub limit_tolerance_kw: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationCode {
    NewPeak,
    PccImportLimit,
    PccRampLimit,
    FeederImportLimit,
}

impl fmt::Display for ViolationCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NewPeak => write!(f, "NEW_PEAK"),
            Self::PccImportLimit => write!(f, "PCC_IMPORT_LIMIT"),
            Self::PccRampLimit => write!(f, "PCC_RAMP_LIMIT"),
            Self::FeederImportLimit => write!(f, "FEEDER_IMPORT_LIMIT"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdmissionViolation {
    pub code: ViolationCode,
    pub asset_id: String,
    /// Zero-based interval index; for a ramp this is the interval reached.
    pub slot: usize,
    pub observed_kw: f64,
    pub allowed_kw: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdmissionReport {
    pub study_id: String,
    pub pcc_boundary_id: String,
    pub admitted: bool,
    pub baseline_pcc_net_import_kw: Vec<f64>,
    pub candidate_pcc_net_import_kw: Vec<f64>,
    pub baseline_peak_import_kw: f64,
    pub candidate_peak_import_kw: f64,
    pub maximum_absolute_pcc_ramp_kw: f64,
    pub candidate_pcc_net_energy_kwh: f64,
    pub candidate_pcc_import_energy_kwh: f64,
    pub flexible_work_energy_kwh: f64,
    pub useful_work_units: f64,
    pub violations: Vec<AdmissionViolation>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerificationError {
    MissingIdentity(&'static str),
    InvalidQuantity { field: String, value: f64 },
    InvalidContract(String),
    WrongLength { field: String, expected: usize, actual: usize },
    DuplicateId(String),
    UnknownFeeder { job_id: String, feeder_id: String },
    InvalidJob { job_id: String, reason: String },
    InvalidSchedule(String),
    BaselineInfeasible(Vec<AdmissionViolation>),
    NumericalOverflow(&'static str),
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity(name) => write!(f, "{name} must not be empty"),
            Self::InvalidQuantity { field, value } => {
                write!(f, "{field} must be finite and in range; got {value}")
            }
            Self::InvalidContract(reason) => write!(f, "invalid contract: {reason}"),
            Self::WrongLength { field, expected, actual } => {
                write!(f, "{field} has length {actual}; expected {expected}")
            }
            Self::DuplicateId(id) => write!(f, "duplicate ID: {id}"),
            Self::UnknownFeeder { job_id, feeder_id } => {
                write!(f, "job {job_id} references unknown feeder {feeder_id}")
            }
            Self::InvalidJob { job_id, reason } => {
                write!(f, "invalid job {job_id}: {reason}")
            }
            Self::InvalidSchedule(reason) => write!(f, "invalid schedule: {reason}"),
            Self::BaselineInfeasible(violations) => {
                write!(f, "baseline violates hard constraints: {violations:?}")
            }
            Self::NumericalOverflow(operation) => {
                write!(f, "non-finite arithmetic during {operation}")
            }
        }
    }
}

impl Error for VerificationError {}

#[derive(Debug, Clone, PartialEq)]
struct Trajectory {
    pcc: Vec<f64>,
    feeders: BTreeMap<String, Vec<f64>>,
}

fn require_id(name: &'static str, value: &str) -> Result<(), VerificationError> {
    if value.trim().is_empty() {
        return Err(VerificationError::MissingIdentity(name));
    }
    Ok(())
}

fn require_finite(
    field: impl Into<String>,
    value: f64,
) -> Result<(), VerificationError> {
    if !value.is_finite() {
        return Err(VerificationError::InvalidQuantity {
            field: field.into(),
            value,
        });
    }
    Ok(())
}

fn require_non_negative(
    field: impl Into<String>,
    value: f64,
) -> Result<(), VerificationError> {
    let field = field.into();
    require_finite(field.clone(), value)?;
    if value < 0.0 {
        return Err(VerificationError::InvalidQuantity { field, value });
    }
    Ok(())
}

fn add_finite(
    current: f64,
    increment: f64,
    operation: &'static str,
) -> Result<f64, VerificationError> {
    let next = current + increment;
    if !next.is_finite() {
        return Err(VerificationError::NumericalOverflow(operation));
    }
    Ok(next)
}

fn validate_optional_limits(
    field: &str,
    limits: &[Option<f64>],
    expected: usize,
) -> Result<(), VerificationError> {
    if limits.len() != expected {
        return Err(VerificationError::WrongLength {
            field: field.to_owned(),
            expected,
            actual: limits.len(),
        });
    }
    for (index, value) in limits.iter().enumerate() {
        if let Some(limit) = value {
            require_finite(format!("{field}[{index}]"), *limit)?;
        }
    }
    Ok(())
}

fn validate_contract_and_jobs(
    contract: &AdmissionContract,
    jobs: &[WorkloadJob],
) -> Result<(usize, BTreeMap<String, WorkloadJob>), VerificationError> {
    require_id("study ID", &contract.study_id)?;
    require_id("PCC boundary ID", &contract.pcc_boundary_id)?;
    require_non_negative("limit tolerance kW", contract.limit_tolerance_kw)?;

    if !contract.interval_hours.is_finite() || contract.interval_hours <= 0.0 {
        return Err(VerificationError::InvalidQuantity {
            field: "interval duration hours".to_owned(),
            value: contract.interval_hours,
        });
    }

    let slot_count = contract.base_pcc_net_import_kw.len();
    if slot_count == 0 {
        return Err(VerificationError::InvalidContract(
            "at least one chronological interval is required".to_owned(),
        ));
    }

    if contract.pcc_max_import_kw.len() != slot_count {
        return Err(VerificationError::WrongLength {
            field: "PCC import-limit profile".to_owned(),
            expected: slot_count,
            actual: contract.pcc_max_import_kw.len(),
        });
    }
    if contract.pcc_max_abs_ramp_kw.len() != slot_count.saturating_sub(1) {
        return Err(VerificationError::WrongLength {
            field: "PCC ramp-limit profile".to_owned(),
            expected: slot_count.saturating_sub(1),
            actual: contract.pcc_max_abs_ramp_kw.len(),
        });
    }

    for (slot, value) in contract.base_pcc_net_import_kw.iter().enumerate() {
        require_finite(format!("base PCC net import[{slot}]"), *value)?;
    }
    validate_optional_limits(
        "PCC import limits",
        &contract.pcc_max_import_kw,
        slot_count,
    )?;
    validate_optional_limits(
        "PCC absolute ramp limits",
        &contract.pcc_max_abs_ramp_kw,
        slot_count.saturating_sub(1),
    )?;

    if contract.feeders.is_empty() {
        return Err(VerificationError::InvalidContract(
            "at least one feeder envelope is required".to_owned(),
        ));
    }

    let mut feeder_ids = BTreeSet::new();
    for feeder in &contract.feeders {
        require_id("feeder ID", &feeder.feeder_id)?;
        if !feeder_ids.insert(feeder.feeder_id.clone()) {
            return Err(VerificationError::DuplicateId(feeder.feeder_id.clone()));
        }
        if feeder.base_net_import_kw.len() != slot_count {
            return Err(VerificationError::WrongLength {
                field: format!("feeder {} base profile", feeder.feeder_id),
                expected: slot_count,
                actual: feeder.base_net_import_kw.len(),
            });
        }
        for (slot, value) in feeder.base_net_import_kw.iter().enumerate() {
            require_finite(
                format!("feeder {} base net import[{slot}]", feeder.feeder_id),
                *value,
            )?;
        }
        validate_optional_limits(
            &format!("feeder {} import limits", feeder.feeder_id),
            &feeder.max_import_kw,
            slot_count,
        )?;
    }

    if jobs.is_empty() {
        return Err(VerificationError::InvalidContract(
            "at least one flexible workload is required".to_owned(),
        ));
    }

    let mut job_map = BTreeMap::new();
    for job in jobs {
        require_id("job ID", &job.job_id)?;
        require_id("job feeder ID", &job.feeder_id)?;
        if job_map.contains_key(&job.job_id) {
            return Err(VerificationError::DuplicateId(job.job_id.clone()));
        }
        if !feeder_ids.contains(&job.feeder_id) {
            return Err(VerificationError::UnknownFeeder {
                job_id: job.job_id.clone(),
                feeder_id: job.feeder_id.clone(),
            });
        }
        if !job.power_kw.is_finite() || job.power_kw <= 0.0 {
            return Err(VerificationError::InvalidJob {
                job_id: job.job_id.clone(),
                reason: "power must be finite and positive".to_owned(),
            });
        }
        if !job.useful_work_units.is_finite() || job.useful_work_units <= 0.0 {
            return Err(VerificationError::InvalidJob {
                job_id: job.job_id.clone(),
                reason: "useful work must be finite and positive".to_owned(),
            });
        }
        if job.duration_slots == 0
            || job.release_slot >= job.deadline_exclusive
            || job.deadline_exclusive > slot_count
            || job.duration_slots > job.deadline_exclusive - job.release_slot
        {
            return Err(VerificationError::InvalidJob {
                job_id: job.job_id.clone(),
                reason: "invalid duration or release/deadline window".to_owned(),
            });
        }
        let work_energy = job.power_kw * job.duration_slots as f64 * contract.interval_hours;
        if !work_energy.is_finite() || work_energy <= 0.0 {
            return Err(VerificationError::InvalidJob {
                job_id: job.job_id.clone(),
                reason: "derived workload energy is non-finite or non-positive".to_owned(),
            });
        }
        job_map.insert(job.job_id.clone(), job.clone());
    }

    Ok((slot_count, job_map))
}

fn build_trajectory(
    contract: &AdmissionContract,
    slot_count: usize,
    jobs: &BTreeMap<String, WorkloadJob>,
    placements: &[JobPlacement],
) -> Result<Trajectory, VerificationError> {
    if placements.len() != jobs.len() {
        return Err(VerificationError::InvalidSchedule(format!(
            "contains {} placements for {} required jobs",
            placements.len(),
            jobs.len()
        )));
    }

    let mut seen = BTreeSet::new();
    let mut pcc = contract.base_pcc_net_import_kw.clone();
    let mut feeders: BTreeMap<String, Vec<f64>> = contract
        .feeders
        .iter()
        .map(|feeder| (feeder.feeder_id.clone(), feeder.base_net_import_kw.clone()))
        .collect();

    for placement in placements {
        if !seen.insert(placement.job_id.clone()) {
            return Err(VerificationError::InvalidSchedule(format!(
                "job {} appears more than once",
                placement.job_id
            )));
        }
        let job = jobs.get(&placement.job_id).ok_or_else(|| {
            VerificationError::InvalidSchedule(format!(
                "unknown job {}",
                placement.job_id
            ))
        })?;

        let end_slot = placement
            .start_slot
            .checked_add(job.duration_slots)
            .ok_or(VerificationError::NumericalOverflow("job end slot"))?;
        if placement.start_slot < job.release_slot
            || end_slot > job.deadline_exclusive
            || end_slot > slot_count
        {
            return Err(VerificationError::InvalidSchedule(format!(
                "job {} placement [{}, {}) violates release/deadline window [{}, {})",
                job.job_id,
                placement.start_slot,
                end_slot,
                job.release_slot,
                job.deadline_exclusive
            )));
        }

        let feeder_profile = feeders.get_mut(&job.feeder_id).ok_or_else(|| {
            VerificationError::UnknownFeeder {
                job_id: job.job_id.clone(),
                feeder_id: job.feeder_id.clone(),
            }
        })?;

        for slot in placement.start_slot..end_slot {
            pcc[slot] = add_finite(pcc[slot], job.power_kw, "PCC trajectory")?;
            feeder_profile[slot] =
                add_finite(feeder_profile[slot], job.power_kw, "feeder trajectory")?;
        }
    }

    if seen.len() != jobs.len() {
        let missing: Vec<_> = jobs
            .keys()
            .filter(|job_id| !seen.contains(*job_id))
            .cloned()
            .collect();
        return Err(VerificationError::InvalidSchedule(format!(
            "missing workload placements: {missing:?}"
        )));
    }

    Ok(Trajectory { pcc, feeders })
}

fn collect_violations(
    contract: &AdmissionContract,
    trajectory: &Trajectory,
    baseline_peak: f64,
    check_new_peak: bool,
) -> Vec<AdmissionViolation> {
    let tolerance = contract.limit_tolerance_kw;
    let mut violations = Vec::new();

    for (slot, (observed, limit)) in trajectory
        .pcc
        .iter()
        .zip(contract.pcc_max_import_kw.iter())
        .enumerate()
    {
        if let Some(allowed) = limit {
            if *observed > *allowed + tolerance {
                violations.push(AdmissionViolation {
                    code: ViolationCode::PccImportLimit,
                    asset_id: contract.pcc_boundary_id.clone(),
                    slot,
                    observed_kw: *observed,
                    allowed_kw: *allowed,
                });
            }
        }
    }

    for (transition, limit) in contract.pcc_max_abs_ramp_kw.iter().enumerate() {
        if let Some(allowed) = limit {
            let observed = (trajectory.pcc[transition + 1] - trajectory.pcc[transition]).abs();
            if observed > *allowed + tolerance {
                violations.push(AdmissionViolation {
                    code: ViolationCode::PccRampLimit,
                    asset_id: contract.pcc_boundary_id.clone(),
                    slot: transition + 1,
                    observed_kw: observed,
                    allowed_kw: *allowed,
                });
            }
        }
    }

    for feeder in &contract.feeders {
        let Some(profile) = trajectory.feeders.get(&feeder.feeder_id) else {
            continue;
        };
        for (slot, (observed, limit)) in profile
            .iter()
            .zip(feeder.max_import_kw.iter())
            .enumerate()
        {
            if let Some(allowed) = limit {
                if *observed > *allowed + tolerance {
                    violations.push(AdmissionViolation {
                        code: ViolationCode::FeederImportLimit,
                        asset_id: feeder.feeder_id.clone(),
                        slot,
                        observed_kw: *observed,
                        allowed_kw: *allowed,
                    });
                }
            }
        }
    }

    if check_new_peak && contract.require_no_new_peak {
        let candidate_peak = trajectory
            .pcc
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        if candidate_peak > baseline_peak + tolerance {
            let slot = trajectory
                .pcc
                .iter()
                .position(|value| (*value - candidate_peak).abs() <= f64::EPSILON)
                .unwrap_or(0);
            violations.push(AdmissionViolation {
                code: ViolationCode::NewPeak,
                asset_id: contract.pcc_boundary_id.clone(),
                slot,
                observed_kw: candidate_peak,
                allowed_kw: baseline_peak,
            });
        }
    }

    violations
}

fn sum_profile_energy(
    profile: &[f64],
    interval_hours: f64,
    positive_only: bool,
) -> Result<f64, VerificationError> {
    let mut energy = 0.0;
    for power in profile {
        let power = if positive_only { power.max(0.0) } else { *power };
        energy = add_finite(energy, power * interval_hours, "PCC energy integration")?;
    }
    Ok(energy)
}

/// Reconstruct and qualify a candidate workload schedule.
///
/// The baseline and candidate use the same immutable job manifest. A rejected
/// candidate returns a report with explicit violations; malformed manifests or
/// infeasible baselines return an error and cannot be treated as an admitted
/// result.
pub fn verify_admission(
    contract: &AdmissionContract,
    jobs: &[WorkloadJob],
    baseline_schedule: &[JobPlacement],
    candidate_schedule: &[JobPlacement],
) -> Result<AdmissionReport, VerificationError> {
    let (slot_count, job_map) = validate_contract_and_jobs(contract, jobs)?;

    let baseline = build_trajectory(contract, slot_count, &job_map, baseline_schedule)?;
    let baseline_peak = baseline
        .pcc
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);

    // A baseline that already breaches the declared physical envelope makes
    // the counterfactual admission question non-identifiable for this contract.
    let baseline_violations =
        collect_violations(contract, &baseline, baseline_peak, false);
    if !baseline_violations.is_empty() {
        return Err(VerificationError::BaselineInfeasible(baseline_violations));
    }

    let candidate = build_trajectory(contract, slot_count, &job_map, candidate_schedule)?;
    let candidate_violations =
        collect_violations(contract, &candidate, baseline_peak, true);

    let candidate_peak = candidate
        .pcc
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let maximum_absolute_pcc_ramp_kw = candidate
        .pcc
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .fold(0.0, f64::max);

    let mut flexible_work_energy_kwh = 0.0;
    let mut useful_work_units = 0.0;
    for job in job_map.values() {
        flexible_work_energy_kwh = add_finite(
            flexible_work_energy_kwh,
            job.power_kw * job.duration_slots as f64 * contract.interval_hours,
            "flexible workload energy",
        )?;
        useful_work_units = add_finite(
            useful_work_units,
            job.useful_work_units,
            "useful work aggregation",
        )?;
    }

    Ok(AdmissionReport {
        study_id: contract.study_id.clone(),
        pcc_boundary_id: contract.pcc_boundary_id.clone(),
        admitted: candidate_violations.is_empty(),
        baseline_pcc_net_import_kw: baseline.pcc,
        candidate_pcc_net_import_kw: candidate.pcc.clone(),
        baseline_peak_import_kw: baseline_peak,
        candidate_peak_import_kw: candidate_peak,
        maximum_absolute_pcc_ramp_kw,
        candidate_pcc_net_energy_kwh: sum_profile_energy(
            &candidate.pcc,
            contract.interval_hours,
            false,
        )?,
        candidate_pcc_import_energy_kwh: sum_profile_energy(
            &candidate.pcc,
            contract.interval_hours,
            true,
        )?,
        flexible_work_energy_kwh,
        useful_work_units,
        violations: candidate_violations,
        claim_ceiling: "Deterministic admission under declared one-zone PCC/feeder profiles and fixed job contracts only. Not a network power-flow, dynamic stability, storage/thermal, resource-adequacy, field, regulatory, or dispatch qualification.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (AdmissionContract, Vec<WorkloadJob>, Vec<JobPlacement>) {
        let base = vec![14.0, 15.0, 18.0, 17.0, 10.0, 8.0, 12.0, 13.0];
        let feeder = FeederEnvelope {
            feeder_id: "port-feeder-a".to_owned(),
            base_net_import_kw: base.clone(),
            max_import_kw: vec![None; 8],
        };
        let contract = AdmissionContract {
            study_id: "synthetic-port-nexus-v0-1".to_owned(),
            pcc_boundary_id: "synthetic-pcc-01".to_owned(),
            interval_hours: 1.0,
            base_pcc_net_import_kw: base,
            pcc_max_import_kw: vec![None; 8],
            pcc_max_abs_ramp_kw: vec![None; 7],
            feeders: vec![feeder],
            require_no_new_peak: true,
            limit_tolerance_kw: 1e-9,
        };
        let jobs = (1..=4)
            .map(|index| WorkloadJob {
                job_id: format!("job-{index}"),
                feeder_id: "port-feeder-a".to_owned(),
                power_kw: 2.0,
                duration_slots: 1,
                release_slot: 0,
                deadline_exclusive: 8,
                useful_work_units: 1.0,
            })
            .collect();
        let baseline = (1..=4)
            .map(|index| JobPlacement {
                job_id: format!("job-{index}"),
                start_slot: 4,
            })
            .collect();
        (contract, jobs, baseline)
    }

    fn placements(slots: [usize; 4]) -> Vec<JobPlacement> {
        slots
            .into_iter()
            .enumerate()
            .map(|(index, slot)| JobPlacement {
                job_id: format!("job-{}", index + 1),
                start_slot: slot,
            })
            .collect()
    }

    #[test]
    fn no_new_peak_tou_fixture_is_admitted() {
        let (contract, jobs, baseline) = fixture();
        let report = verify_admission(&contract, &jobs, &baseline, &placements([0, 0, 1, 5]))
            .expect("valid paired schedules");
        assert!(report.admitted, "{:?}", report.violations);
        assert_eq!(report.baseline_peak_import_kw, 18.0);
        assert_eq!(report.candidate_peak_import_kw, 18.0);
        assert_eq!(report.candidate_pcc_net_import_kw, vec![
            18.0, 17.0, 18.0, 17.0, 10.0, 10.0, 12.0, 13.0
        ]);
        assert_eq!(report.flexible_work_energy_kwh, 8.0);
        assert_eq!(report.useful_work_units, 4.0);
    }

    #[test]
    fn unconstrained_tou_peak_is_rejected() {
        let (contract, jobs, baseline) = fixture();
        let report = verify_admission(&contract, &jobs, &baseline, &placements([0, 0, 0, 0]))
            .expect("valid paired schedules");
        assert!(!report.admitted);
        assert_eq!(report.candidate_peak_import_kw, 22.0);
        assert!(report.violations.iter().any(|v| v.code == ViolationCode::NewPeak));
    }

    #[test]
    fn unconstrained_carbon_peak_is_rejected() {
        let (contract, jobs, baseline) = fixture();
        let report = verify_admission(&contract, &jobs, &baseline, &placements([1, 1, 1, 1]))
            .expect("valid paired schedules");
        assert!(!report.admitted);
        assert_eq!(report.candidate_peak_import_kw, 23.0);
        assert!(report.violations.iter().any(|v| v.code == ViolationCode::NewPeak));
    }

    #[test]
    fn full_horizon_catches_rebound_after_dispatch_window() {
        let (contract, jobs, baseline) = fixture();
        // The dispatch window could end at slot 5, but deferred work is run
        // together at slot 7. The verifier evaluates all eight slots.
        let report = verify_admission(&contract, &jobs, &baseline, &placements([7, 7, 7, 7]))
            .expect("valid paired schedules");
        assert!(!report.admitted);
        assert_eq!(report.candidate_pcc_net_import_kw[7], 21.0);
        assert!(report.violations.iter().any(|v| v.code == ViolationCode::NewPeak));
    }

    #[test]
    fn pcc_time_varying_limit_is_independent_of_historical_peak() {
        let (mut contract, jobs, baseline) = fixture();
        contract.pcc_max_import_kw[1] = Some(16.0);
        let report = verify_admission(&contract, &jobs, &baseline, &placements([1, 5, 5, 5]))
            .expect("baseline remains feasible");
        assert!(!report.admitted);
        assert!(report.violations.iter().any(|v| v.code == ViolationCode::PccImportLimit));
        assert!(!report.violations.iter().any(|v| v.code == ViolationCode::NewPeak));
    }

    #[test]
    fn feeder_limit_catches_local_congestion_when_pcc_peak_passes() {
        let (mut contract, jobs, baseline) = fixture();
        contract.feeders[0].max_import_kw[1] = Some(16.0);
        let report = verify_admission(&contract, &jobs, &baseline, &placements([1, 5, 5, 5]))
            .expect("PCC trajectory does not create a new peak");
        assert!(!report.admitted);
        assert!(report.violations.iter().any(|v| v.code == ViolationCode::FeederImportLimit));
        assert!(!report.violations.iter().any(|v| v.code == ViolationCode::NewPeak));
    }

    #[test]
    fn ramp_limit_is_checked_even_when_peak_is_unchanged() {
        let mut contract = AdmissionContract {
            study_id: "ramp-study".to_owned(),
            pcc_boundary_id: "ramp-pcc".to_owned(),
            interval_hours: 1.0,
            base_pcc_net_import_kw: vec![10.0, 13.0, 13.0, 13.0, 10.0, 10.0],
            pcc_max_import_kw: vec![None; 6],
            pcc_max_abs_ramp_kw: vec![Some(3.0); 5],
            feeders: vec![FeederEnvelope {
                feeder_id: "f1".to_owned(),
                base_net_import_kw: vec![10.0, 13.0, 13.0, 13.0, 10.0, 10.0],
                max_import_kw: vec![None; 6],
            }],
            require_no_new_peak: true,
            limit_tolerance_kw: 1e-9,
        };
        let jobs = vec![WorkloadJob {
            job_id: "j1".to_owned(),
            feeder_id: "f1".to_owned(),
            power_kw: 4.0,
            duration_slots: 1,
            release_slot: 0,
            deadline_exclusive: 6,
            useful_work_units: 1.0,
        }];
        let baseline = vec![JobPlacement {
            job_id: "j1".to_owned(),
            start_slot: 0,
        }];
        let candidate = vec![JobPlacement {
            job_id: "j1".to_owned(),
            start_slot: 4,
        }];
        // Keep the contract mutable binding explicit: only the test's
        // ramp-envelope values differ from the default fixture shape.
        contract.require_no_new_peak = true;
        let report = verify_admission(&contract, &jobs, &baseline, &candidate)
            .expect("baseline ramp is within the declared limit");
        assert_eq!(report.baseline_peak_import_kw, 14.0);
        assert_eq!(report.candidate_peak_import_kw, 14.0);
        assert!(!report.admitted);
        assert!(report.violations.iter().any(|v| v.code == ViolationCode::PccRampLimit));
    }

    #[test]
    fn duplicate_or_missing_jobs_fail_closed() {
        let (contract, jobs, baseline) = fixture();
        let duplicate = placements([0, 0, 1, 5]);
        let mut duplicate = duplicate;
        duplicate[3].job_id = "job-1".to_owned();
        assert!(matches!(
            verify_admission(&contract, &jobs, &baseline, &duplicate),
            Err(VerificationError::InvalidSchedule(_))
        ));
    }

    #[test]
    fn invalid_deadline_fails_closed() {
        let (contract, mut jobs, baseline) = fixture();
        jobs[0].deadline_exclusive = 0;
        assert!(matches!(
            verify_admission(&contract, &jobs, &baseline, &placements([0, 0, 1, 5])),
            Err(VerificationError::InvalidJob { .. })
        ));
    }

    #[test]
    fn non_finite_power_fails_closed() {
        let (contract, mut jobs, baseline) = fixture();
        jobs[0].power_kw = f64::NAN;
        assert!(matches!(
            verify_admission(&contract, &jobs, &baseline, &placements([0, 0, 1, 5])),
            Err(VerificationError::InvalidJob { .. })
        ));
    }

    #[test]
    fn infeasible_baseline_is_not_used_as_a_pass() {
        let (mut contract, jobs, baseline) = fixture();
        contract.pcc_max_import_kw[2] = Some(17.0);
        assert!(matches!(
            verify_admission(&contract, &jobs, &baseline, &placements([0, 0, 1, 5])),
            Err(VerificationError::BaselineInfeasible(_))
        ));
    }
}
