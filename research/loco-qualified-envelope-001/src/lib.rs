//! Exact finite composition of empirical capability, robust viability,
//! adverse-outcome safety, energy/recovery reserve, and assurance evidence.
//!
//! The qualifier joins upstream products for one exact operating cell. It does
//! not calculate any upstream metric, authenticate receipts, or prove physics.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceTier {
    Simulation,
    Hardware,
    Biological,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceIdentity {
    pub scenario_digest: String,
    pub morphology_digest: String,
    pub evidence_tier: EvidenceTier,
    pub platform_physics_digest: String,
    pub controller_digest: String,
    pub task_digest: String,
    pub terrain_cell_id: String,
    pub terrain_axis_digest: String,
    pub sensor_health_digest: String,
    pub calibration_digest: String,
    pub actuator_digest: String,
    pub initial_state_digest: String,
    pub payload_digest: String,
    pub energy_boundary_id: String,
    pub heldout_manifest_digest: String,
    pub exposure_unit_id: String,
    pub exposure_value: u64,
}

impl EvidenceIdentity {
    fn validate(&self) -> Result<(), QualificationError> {
        let values = [
            self.scenario_digest.as_str(),
            self.morphology_digest.as_str(),
            self.platform_physics_digest.as_str(),
            self.controller_digest.as_str(),
            self.task_digest.as_str(),
            self.terrain_cell_id.as_str(),
            self.terrain_axis_digest.as_str(),
            self.sensor_health_digest.as_str(),
            self.calibration_digest.as_str(),
            self.actuator_digest.as_str(),
            self.initial_state_digest.as_str(),
            self.payload_digest.as_str(),
            self.energy_boundary_id.as_str(),
            self.heldout_manifest_digest.as_str(),
            self.exposure_unit_id.as_str(),
        ];
        if values.iter().any(|value| !nonempty(value)) || self.exposure_value == 0 {
            return Err(QualificationError::InvalidCellIdentity);
        }
        Ok(())
    }

    fn matches(&self, other: &Self) -> bool {
        self == other
    }

    fn same_scenario_except_controller(&self, other: &Self) -> bool {
        self.scenario_digest == other.scenario_digest
            && self.morphology_digest == other.morphology_digest
            && self.evidence_tier == other.evidence_tier
            && self.platform_physics_digest == other.platform_physics_digest
            && self.task_digest == other.task_digest
            && self.terrain_cell_id == other.terrain_cell_id
            && self.terrain_axis_digest == other.terrain_axis_digest
            && self.sensor_health_digest == other.sensor_health_digest
            && self.calibration_digest == other.calibration_digest
            && self.actuator_digest == other.actuator_digest
            && self.initial_state_digest == other.initial_state_digest
            && self.payload_digest == other.payload_digest
            && self.energy_boundary_id == other.energy_boundary_id
            && self.heldout_manifest_digest == other.heldout_manifest_digest
            && self.exposure_unit_id == other.exposure_unit_id
            && self.exposure_value == other.exposure_value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceHeader {
    pub receipt_id: String,
    pub identity: EvidenceIdentity,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViabilityStatus {
    Admitted {
        action_id: String,
        all_physical_successors_covered: bool,
        recovery_action_admitted: bool,
    },
    Rejected {
        witness_id: String,
    },
    Unknown {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViabilityEvidence {
    pub header: EvidenceHeader,
    pub status: ViabilityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PerformanceStatus {
    Measured {
        success_lower_bound_ppm: u32,
        cross_fitted_cvar_lower_bound: i64,
    },
    Unknown {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceEvidence {
    pub header: EvidenceHeader,
    pub status: PerformanceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyCategoryEvidence {
    pub category_id: String,
    pub required: bool,
    pub measurement_complete: bool,
    pub observed_exposure: u64,
    pub event_rate_upper_bound_ppm: u32,
    pub maximum_event_rate_ppm: u32,
    pub severity_upper_bound: u64,
    pub maximum_severity: u64,
    pub regressed_vs_matched_baseline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyEvidence {
    pub header: EvidenceHeader,
    pub categories: Vec<SafetyCategoryEvidence>,
    pub unknown_event_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergyEvidence {
    pub header: EvidenceHeader,
    pub remaining_recovery_reserve_mj: u64,
    pub recovery_energy_upper_bound_mj: u64,
    pub actuator_limits_valid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssuranceStatus {
    Valid {
        sensor_health_valid: bool,
        transition_receipts_valid: bool,
    },
    Invalid {
        reason: String,
    },
    Unknown {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssuranceEvidence {
    pub header: EvidenceHeader,
    pub status: AssuranceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualificationRequirements {
    pub minimum_success_lower_bound_ppm: u32,
    pub minimum_cross_fitted_cvar_lower_bound: i64,
    pub required_safety_category_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineComparisonIdentity {
    pub identity: EvidenceIdentity,
    pub measurement_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatingCellProblem {
    pub now_ns: u64,
    pub cell: EvidenceIdentity,
    pub requirements: QualificationRequirements,
    pub viability: Option<ViabilityEvidence>,
    pub performance: Option<PerformanceEvidence>,
    pub safety: Option<SafetyEvidence>,
    pub energy: Option<EnergyEvidence>,
    pub assurance: Option<AssuranceEvidence>,
    pub baseline: Option<BaselineComparisonIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViabilityClass {
    AdmittedAndRecoverable,
    Rejected,
    Unknown,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceClass {
    Pass,
    Fail,
    Unknown,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyClass {
    Pass,
    Fail,
    Incomplete,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceClass {
    Pass,
    RecoveryReserveInsufficient,
    ActuatorLimitInvalid,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssuranceClass {
    Valid,
    Invalid,
    Unknown,
    Stale,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryDisposition {
    Qualified,
    EmpiricallyCapableButNotRobustlyCertified,
    RobustlyAdmittedButPerformanceInsufficient,
    SafetyConstraintExceeded,
    RecoveryReserveInsufficient,
    AssuranceInvalidOrStale,
    EvidenceIncompleteOrUnknown,
    EvidenceIdentityMismatch,
    MultipleHardRequirementsFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Blocker {
    EvidenceIdentityMismatch,
    AssuranceInvalidOrStale,
    EvidenceIncompleteOrUnknown,
    ViabilityNotAdmitted,
    RecoveryNotAdmitted,
    PerformanceBelowThreshold,
    SafetyConstraintExceeded,
    SafetyMeasurementIncomplete,
    RecoveryReserveInsufficient,
    ActuatorLimitInvalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitingWitness {
    pub gate_id: String,
    pub receipt_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualificationReport {
    pub qualified: bool,
    pub primary_disposition: PrimaryDisposition,
    pub blockers: Vec<Blocker>,
    pub witnesses: Vec<LimitingWitness>,
    pub viability_class: ViabilityClass,
    pub performance_class: PerformanceClass,
    pub safety_class: SafetyClass,
    pub resource_class: ResourceClass,
    pub assurance_class: AssuranceClass,
    pub baseline_comparison_eligible: bool,
    pub baseline_comparison_reason: Option<String>,
    pub cell_scenario_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualificationError {
    InvalidCellIdentity,
    InvalidRequirements,
    DuplicateRequiredSafetyCategory,
    InvalidReceipt,
    DuplicateReceiptId,
    DuplicateSafetyCategory,
    UnknownSafetyCategory,
    SafetyExposureMismatch,
    InvalidSafetyBound,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn push_witness(
    witnesses: &mut Vec<LimitingWitness>,
    gate_id: &str,
    receipt_id: Option<&str>,
    detail: &str,
) {
    witnesses.push(LimitingWitness {
        gate_id: gate_id.to_owned(),
        receipt_id: receipt_id.map(str::to_owned),
        detail: detail.to_owned(),
    });
}

fn check_header(
    header: &EvidenceHeader,
    cell: &EvidenceIdentity,
    now_ns: u64,
    blockers: &mut BTreeSet<Blocker>,
    witnesses: &mut Vec<LimitingWitness>,
) -> bool {
    if !nonempty(&header.receipt_id) || header.valid_from_ns > header.valid_until_ns {
        blockers.insert(Blocker::EvidenceIncompleteOrUnknown);
        push_witness(
            witnesses,
            "evidence_header",
            Some(&header.receipt_id),
            "receipt identifier or validity interval is invalid",
        );
        return false;
    }
    if !header.identity.matches(cell) {
        blockers.insert(Blocker::EvidenceIdentityMismatch);
        push_witness(
            witnesses,
            "identity_join",
            Some(&header.receipt_id),
            "evidence identity does not exactly match the operating cell",
        );
        return false;
    }
    if now_ns < header.valid_from_ns || now_ns > header.valid_until_ns {
        blockers.insert(Blocker::AssuranceInvalidOrStale);
        push_witness(
            witnesses,
            "receipt_freshness",
            Some(&header.receipt_id),
            "evidence receipt is stale or not yet valid",
        );
        return false;
    }
    true
}

fn check_unique_receipts(problem: &OperatingCellProblem) -> Result<(), QualificationError> {
    let mut ids = BTreeSet::new();
    let headers = [
        problem.viability.as_ref().map(|item| &item.header),
        problem.performance.as_ref().map(|item| &item.header),
        problem.safety.as_ref().map(|item| &item.header),
        problem.energy.as_ref().map(|item| &item.header),
        problem.assurance.as_ref().map(|item| &item.header),
    ];
    for header in headers.into_iter().flatten() {
        if !ids.insert(header.receipt_id.as_str()) {
            return Err(QualificationError::DuplicateReceiptId);
        }
    }
    Ok(())
}

fn evaluate_viability(
    evidence: Option<&ViabilityEvidence>,
    cell: &EvidenceIdentity,
    now_ns: u64,
    blockers: &mut BTreeSet<Blocker>,
    witnesses: &mut Vec<LimitingWitness>,
) -> ViabilityClass {
    let Some(evidence) = evidence else {
        blockers.insert(Blocker::EvidenceIncompleteOrUnknown);
        push_witness(witnesses, "viability", None, "robust viability/recovery evidence is missing");
        return ViabilityClass::Missing;
    };
    if !check_header(&evidence.header, cell, now_ns, blockers, witnesses) {
        return if blockers.contains(&Blocker::EvidenceIdentityMismatch) {
            ViabilityClass::Unknown
        } else {
            ViabilityClass::Unknown
        };
    }

    match &evidence.status {
        ViabilityStatus::Admitted {
            action_id,
            all_physical_successors_covered,
            recovery_action_admitted,
        } => {
            if !nonempty(action_id) || !all_physical_successors_covered {
                blockers.insert(Blocker::ViabilityNotAdmitted);
                push_witness(
                    witnesses,
                    "viability",
                    Some(&evidence.header.receipt_id),
                    "admission does not cover every declared physical successor",
                );
                ViabilityClass::Rejected
            } else if !recovery_action_admitted {
                blockers.insert(Blocker::RecoveryNotAdmitted);
                push_witness(
                    witnesses,
                    "recovery",
                    Some(&evidence.header.receipt_id),
                    "no independently admitted recovery action is available",
                );
                ViabilityClass::Rejected
            } else {
                ViabilityClass::AdmittedAndRecoverable
            }
        }
        ViabilityStatus::Rejected { witness_id } => {
            blockers.insert(Blocker::ViabilityNotAdmitted);
            push_witness(
                witnesses,
                "viability",
                Some(&evidence.header.receipt_id),
                witness_id,
            );
            ViabilityClass::Rejected
        }
        ViabilityStatus::Unknown { reason } => {
            blockers.insert(Blocker::EvidenceIncompleteOrUnknown);
            push_witness(
                witnesses,
                "viability",
                Some(&evidence.header.receipt_id),
                reason,
            );
            ViabilityClass::Unknown
        }
    }
}

fn evaluate_performance(
    evidence: Option<&PerformanceEvidence>,
    cell: &EvidenceIdentity,
    requirements: &QualificationRequirements,
    now_ns: u64,
    blockers: &mut BTreeSet<Blocker>,
    witnesses: &mut Vec<LimitingWitness>,
) -> PerformanceClass {
    let Some(evidence) = evidence else {
        blockers.insert(Blocker::EvidenceIncompleteOrUnknown);
        push_witness(witnesses, "performance", None, "held-out performance evidence is missing");
        return PerformanceClass::Missing;
    };
    if !check_header(&evidence.header, cell, now_ns, blockers, witnesses) {
        return PerformanceClass::Unknown;
    }
    match &evidence.status {
        PerformanceStatus::Unknown { reason } => {
            blockers.insert(Blocker::EvidenceIncompleteOrUnknown);
            push_witness(
                witnesses,
                "performance",
                Some(&evidence.header.receipt_id),
                reason,
            );
            PerformanceClass::Unknown
        }
        PerformanceStatus::Measured {
            success_lower_bound_ppm,
            cross_fitted_cvar_lower_bound,
        } => {
            if *success_lower_bound_ppm > 1_000_000 {
                blockers.insert(Blocker::EvidenceIncompleteOrUnknown);
                push_witness(
                    witnesses,
                    "performance",
                    Some(&evidence.header.receipt_id),
                    "success lower bound exceeds one million ppm",
                );
                return PerformanceClass::Unknown;
            }
            if *success_lower_bound_ppm < requirements.minimum_success_lower_bound_ppm
                || *cross_fitted_cvar_lower_bound
                    < requirements.minimum_cross_fitted_cvar_lower_bound
            {
                blockers.insert(Blocker::PerformanceBelowThreshold);
                push_witness(
                    witnesses,
                    "performance",
                    Some(&evidence.header.receipt_id),
                    "held-out success lower bound or cross-fitted CVaR is below threshold",
                );
                PerformanceClass::Fail
            } else {
                PerformanceClass::Pass
            }
        }
    }
}

fn evaluate_safety(
    evidence: Option<&SafetyEvidence>,
    cell: &EvidenceIdentity,
    requirements: &QualificationRequirements,
    now_ns: u64,
    blockers: &mut BTreeSet<Blocker>,
    witnesses: &mut Vec<LimitingWitness>,
) -> Result<SafetyClass, QualificationError> {
    let Some(evidence) = evidence else {
        blockers.insert(Blocker::EvidenceIncompleteOrUnknown);
        blockers.insert(Blocker::SafetyMeasurementIncomplete);
        push_witness(witnesses, "safety", None, "adverse-outcome evidence is missing");
        return Ok(SafetyClass::Missing);
    };
    if !check_header(&evidence.header, cell, now_ns, blockers, witnesses) {
        return Ok(SafetyClass::Incomplete);
    }

    let mut categories = BTreeSet::new();
    let mut category_map = std::collections::BTreeMap::new();
    let mut failed = false;
    let mut incomplete = false;

    for category in &evidence.categories {
        if !nonempty(&category.category_id) {
            return Err(QualificationError::UnknownSafetyCategory);
        }
        if !categories.insert(category.category_id.as_str()) {
            return Err(QualificationError::DuplicateSafetyCategory);
        }
        if category.event_rate_upper_bound_ppm > 1_000_000
            || category.maximum_event_rate_ppm > 1_000_000
        {
            return Err(QualificationError::InvalidSafetyBound);
        }
        if category.observed_exposure > cell.exposure_value {
            return Err(QualificationError::SafetyExposureMismatch);
        }
        category_map.insert(category.category_id.as_str(), category);
    }

    let mut required = BTreeSet::new();
    for id in &requirements.required_safety_category_ids {
        if !nonempty(id) {
            return Err(QualificationError::UnknownSafetyCategory);
        }
        if !required.insert(id.as_str()) {
            return Err(QualificationError::DuplicateRequiredSafetyCategory);
        }
        let Some(category) = category_map.get(id.as_str()) else {
            incomplete = true;
            push_witness(
                witnesses,
                "safety_coverage",
                Some(&evidence.header.receipt_id),
                &format!("required adverse category '{id}' is missing"),
            );
            continue;
        };
        if !category.required
            || !category.measurement_complete
            || category.observed_exposure != cell.exposure_value
        {
            incomplete = true;
            push_witness(
                witnesses,
                "safety_coverage",
                Some(&evidence.header.receipt_id),
                &format!("required adverse category '{id}' lacks complete matched exposure"),
            );
        }
        if category.regressed_vs_matched_baseline
            || category.event_rate_upper_bound_ppm > category.maximum_event_rate_ppm
            || category.severity_upper_bound > category.maximum_severity
        {
            failed = true;
            push_witness(
                witnesses,
                "safety_limit",
                Some(&evidence.header.receipt_id),
                &format!("required adverse category '{id}' exceeded a limit or regressed"),
            );
        }
    }

    if evidence.unknown_event_count > 0 {
        incomplete = true;
        push_witness(
            witnesses,
            "safety_unknown",
            Some(&evidence.header.receipt_id),
            "unclassified adverse events remain in the evidence",
        );
    }
    if incomplete {
        blockers.insert(Blocker::EvidenceIncompleteOrUnknown);
        blockers.insert(Blocker::SafetyMeasurementIncomplete);
    }
    if failed {
        blockers.insert(Blocker::SafetyConstraintExceeded);
    }
    if failed {
        Ok(SafetyClass::Fail)
    } else if incomplete {
        Ok(SafetyClass::Incomplete)
    } else {
        Ok(SafetyClass::Pass)
    }
}

fn evaluate_energy(
    evidence: Option<&EnergyEvidence>,
    cell: &EvidenceIdentity,
    now_ns: u64,
    blockers: &mut BTreeSet<Blocker>,
    witnesses: &mut Vec<LimitingWitness>,
) -> ResourceClass {
    let Some(evidence) = evidence else {
        blockers.insert(Blocker::EvidenceIncompleteOrUnknown);
        push_witness(witnesses, "energy", None, "recovery reserve evidence is missing");
        return ResourceClass::Missing;
    };
    if !check_header(&evidence.header, cell, now_ns, blockers, witnesses) {
        return ResourceClass::Missing;
    }
    if !evidence.actuator_limits_valid {
        blockers.insert(Blocker::ActuatorLimitInvalid);
        push_witness(
            witnesses,
            "actuator",
            Some(&evidence.header.receipt_id),
            "actuator envelope is not valid for this operating cell",
        );
        return ResourceClass::ActuatorLimitInvalid;
    }
    if evidence.remaining_recovery_reserve_mj < evidence.recovery_energy_upper_bound_mj {
        blockers.insert(Blocker::RecoveryReserveInsufficient);
        push_witness(
            witnesses,
            "recovery_reserve",
            Some(&evidence.header.receipt_id),
            "remaining usable energy is below the conservative recovery-energy upper bound",
        );
        return ResourceClass::RecoveryReserveInsufficient;
    }
    ResourceClass::Pass
}

fn evaluate_assurance(
    evidence: Option<&AssuranceEvidence>,
    cell: &EvidenceIdentity,
    now_ns: u64,
    blockers: &mut BTreeSet<Blocker>,
    witnesses: &mut Vec<LimitingWitness>,
) -> AssuranceClass {
    let Some(evidence) = evidence else {
        blockers.insert(Blocker::EvidenceIncompleteOrUnknown);
        blockers.insert(Blocker::AssuranceInvalidOrStale);
        push_witness(witnesses, "assurance", None, "assurance receipt is missing");
        return AssuranceClass::Missing;
    };
    if !check_header(&evidence.header, cell, now_ns, blockers, witnesses) {
        return AssuranceClass::Stale;
    }
    match &evidence.status {
        AssuranceStatus::Valid {
            sensor_health_valid,
            transition_receipts_valid,
        } if *sensor_health_valid && *transition_receipts_valid => AssuranceClass::Valid,
        AssuranceStatus::Valid { .. } => {
            blockers.insert(Blocker::AssuranceInvalidOrStale);
            push_witness(
                witnesses,
                "assurance",
                Some(&evidence.header.receipt_id),
                "sensor health or transition receipt validity is false",
            );
            AssuranceClass::Invalid
        }
        AssuranceStatus::Invalid { reason } => {
            blockers.insert(Blocker::AssuranceInvalidOrStale);
            push_witness(
                witnesses,
                "assurance",
                Some(&evidence.header.receipt_id),
                reason,
            );
            AssuranceClass::Invalid
        }
        AssuranceStatus::Unknown { reason } => {
            blockers.insert(Blocker::EvidenceIncompleteOrUnknown);
            push_witness(
                witnesses,
                "assurance",
                Some(&evidence.header.receipt_id),
                reason,
            );
            AssuranceClass::Unknown
        }
    }
}

fn primary_disposition(
    blockers: &BTreeSet<Blocker>,
    viability: ViabilityClass,
    performance: PerformanceClass,
) -> PrimaryDisposition {
    if blockers.is_empty() {
        return PrimaryDisposition::Qualified;
    }
    if blockers.contains(&Blocker::EvidenceIdentityMismatch) {
        return PrimaryDisposition::EvidenceIdentityMismatch;
    }
    if blockers.contains(&Blocker::AssuranceInvalidOrStale) {
        return PrimaryDisposition::AssuranceInvalidOrStale;
    }
    if blockers.contains(&Blocker::SafetyConstraintExceeded) {
        return PrimaryDisposition::SafetyConstraintExceeded;
    }
    if blockers.contains(&Blocker::RecoveryReserveInsufficient)
        || blockers.contains(&Blocker::ActuatorLimitInvalid)
    {
        return PrimaryDisposition::RecoveryReserveInsufficient;
    }
    if blockers.contains(&Blocker::EvidenceIncompleteOrUnknown)
        || blockers.contains(&Blocker::SafetyMeasurementIncomplete)
    {
        return PrimaryDisposition::EvidenceIncompleteOrUnknown;
    }
    if viability == ViabilityClass::Rejected && performance == PerformanceClass::Pass {
        return PrimaryDisposition::EmpiricallyCapableButNotRobustlyCertified;
    }
    if viability == ViabilityClass::AdmittedAndRecoverable
        && performance == PerformanceClass::Fail
    {
        return PrimaryDisposition::RobustlyAdmittedButPerformanceInsufficient;
    }
    if viability != ViabilityClass::AdmittedAndRecoverable
        && performance != PerformanceClass::Pass
    {
        return PrimaryDisposition::MultipleHardRequirementsFailed;
    }
    if blockers.contains(&Blocker::ViabilityNotAdmitted) {
        return PrimaryDisposition::EmpiricallyCapableButNotRobustlyCertified;
    }
    if blockers.contains(&Blocker::PerformanceBelowThreshold) {
        return PrimaryDisposition::RobustlyAdmittedButPerformanceInsufficient;
    }
    PrimaryDisposition::MultipleHardRequirementsFailed
}

pub fn qualify_operating_cell(
    problem: &OperatingCellProblem,
) -> Result<QualificationReport, QualificationError> {
    problem.cell.validate()?;
    if problem.requirements.minimum_success_lower_bound_ppm > 1_000_000 {
        return Err(QualificationError::InvalidRequirements);
    }
    if problem.requirements.required_safety_category_ids.is_empty() {
        return Err(QualificationError::InvalidRequirements);
    }
    check_unique_receipts(problem)?;

    let mut blockers = BTreeSet::new();
    let mut witnesses = Vec::new();

    let viability = evaluate_viability(
        problem.viability.as_ref(),
        &problem.cell,
        problem.now_ns,
        &mut blockers,
        &mut witnesses,
    );
    let performance = evaluate_performance(
        problem.performance.as_ref(),
        &problem.cell,
        &problem.requirements,
        problem.now_ns,
        &mut blockers,
        &mut witnesses,
    );
    let safety = evaluate_safety(
        problem.safety.as_ref(),
        &problem.cell,
        &problem.requirements,
        problem.now_ns,
        &mut blockers,
        &mut witnesses,
    )?;
    let resource = evaluate_energy(
        problem.energy.as_ref(),
        &problem.cell,
        problem.now_ns,
        &mut blockers,
        &mut witnesses,
    );
    let assurance = evaluate_assurance(
        problem.assurance.as_ref(),
        &problem.cell,
        problem.now_ns,
        &mut blockers,
        &mut witnesses,
    );

    let baseline_comparison = problem.baseline.as_ref().map(|baseline| {
        baseline.measurement_complete
            && nonempty(&baseline.identity.controller_digest)
            && baseline.identity.same_scenario_except_controller(&problem.cell)
    });
    let baseline_comparison_eligible = baseline_comparison.unwrap_or(false);
    let baseline_comparison_reason = match &problem.baseline {
        None => Some("no matched baseline evidence supplied".to_owned()),
        Some(baseline) if !baseline.measurement_complete => {
            Some("baseline measurement is incomplete".to_owned())
        }
        Some(baseline)
            if !baseline
                .identity
                .same_scenario_except_controller(&problem.cell) =>
        {
            Some("baseline does not match the exact scenario, morphology, physics and exposure".to_owned())
        }
        Some(_) => None,
    };

    let disposition = primary_disposition(&blockers, viability, performance);
    let qualified = disposition == PrimaryDisposition::Qualified;

    Ok(QualificationReport {
        qualified,
        primary_disposition: disposition,
        blockers: blockers.into_iter().collect(),
        witnesses,
        viability_class: viability,
        performance_class: performance,
        safety_class: safety,
        resource_class: resource,
        assurance_class: assurance,
        baseline_comparison_eligible,
        baseline_comparison_reason,
        cell_scenario_digest: problem.cell.scenario_digest.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> EvidenceIdentity {
        EvidenceIdentity {
            scenario_digest: "scenario-v1".to_owned(),
            morphology_digest: "go2-morph-v1".to_owned(),
            evidence_tier: EvidenceTier::Hardware,
            platform_physics_digest: "go2-physics-v1".to_owned(),
            controller_digest: "symtropy-policy-v1".to_owned(),
            task_digest: "stairs-task-v1".to_owned(),
            terrain_cell_id: "stairs-20cm".to_owned(),
            terrain_axis_digest: "terrain-axes-v1".to_owned(),
            sensor_health_digest: "health-mode-v1".to_owned(),
            calibration_digest: "cal-v1".to_owned(),
            actuator_digest: "actuator-v1".to_owned(),
            initial_state_digest: "state-v1".to_owned(),
            payload_digest: "payload-v1".to_owned(),
            energy_boundary_id: "battery-terminal-v1".to_owned(),
            heldout_manifest_digest: "heldout-v1".to_owned(),
            exposure_unit_id: "crossings".to_owned(),
            exposure_value: 100,
        }
    }

    fn header(id: &str, identity: &EvidenceIdentity) -> EvidenceHeader {
        EvidenceHeader {
            receipt_id: id.to_owned(),
            identity: identity.clone(),
            valid_from_ns: 100,
            valid_until_ns: 1_000,
        }
    }

    fn base_problem() -> OperatingCellProblem {
        let cell = identity();
        OperatingCellProblem {
            now_ns: 500,
            cell: cell.clone(),
            requirements: QualificationRequirements {
                minimum_success_lower_bound_ppm: 900_000,
                minimum_cross_fitted_cvar_lower_bound: 50,
                required_safety_category_ids: vec![
                    "fall".to_owned(),
                    "foot_scuff".to_owned(),
                    "shank_collision".to_owned(),
                ],
            },
            viability: Some(ViabilityEvidence {
                header: header("viability-r1", &cell),
                status: ViabilityStatus::Admitted {
                    action_id: "advance".to_owned(),
                    all_physical_successors_covered: true,
                    recovery_action_admitted: true,
                },
            }),
            performance: Some(PerformanceEvidence {
                header: header("performance-r1", &cell),
                status: PerformanceStatus::Measured {
                    success_lower_bound_ppm: 950_000,
                    cross_fitted_cvar_lower_bound: 75,
                },
            }),
            safety: Some(SafetyEvidence {
                header: header("safety-r1", &cell),
                categories: vec![
                    SafetyCategoryEvidence {
                        category_id: "fall".to_owned(),
                        required: true,
                        measurement_complete: true,
                        observed_exposure: 100,
                        event_rate_upper_bound_ppm: 10_000,
                        maximum_event_rate_ppm: 20_000,
                        severity_upper_bound: 100,
                        maximum_severity: 200,
                        regressed_vs_matched_baseline: false,
                    },
                    SafetyCategoryEvidence {
                        category_id: "foot_scuff".to_owned(),
                        required: true,
                        measurement_complete: true,
                        observed_exposure: 100,
                        event_rate_upper_bound_ppm: 30_000,
                        maximum_event_rate_ppm: 50_000,
                        severity_upper_bound: 100,
                        maximum_severity: 150,
                        regressed_vs_matched_baseline: false,
                    },
                    SafetyCategoryEvidence {
                        category_id: "shank_collision".to_owned(),
                        required: true,
                        measurement_complete: true,
                        observed_exposure: 100,
                        event_rate_upper_bound_ppm: 5_000,
                        maximum_event_rate_ppm: 10_000,
                        severity_upper_bound: 50,
                        maximum_severity: 100,
                        regressed_vs_matched_baseline: false,
                    },
                ],
                unknown_event_count: 0,
            }),
            energy: Some(EnergyEvidence {
                header: header("energy-r1", &cell),
                remaining_recovery_reserve_mj: 150_000,
                recovery_energy_upper_bound_mj: 100_000,
                actuator_limits_valid: true,
            }),
            assurance: Some(AssuranceEvidence {
                header: header("assurance-r1", &cell),
                status: AssuranceStatus::Valid {
                    sensor_health_valid: true,
                    transition_receipts_valid: true,
                },
            }),
            baseline: Some(BaselineComparisonIdentity {
                identity: EvidenceIdentity {
                    controller_digest: "baseline-controller-v1".to_owned(),
                    ..cell.clone()
                },
                measurement_complete: true,
            }),
        }
    }

    #[test]
    fn all_five_gates_passing_qualifies_exact_cell() {
        let report = qualify_operating_cell(&base_problem()).expect("qualification");
        assert!(report.qualified);
        assert_eq!(report.primary_disposition, PrimaryDisposition::Qualified);
        assert!(report.baseline_comparison_eligible);
        assert!(report.blockers.is_empty());
    }

    #[test]
    fn empirical_capability_does_not_override_rejected_viability() {
        let mut problem = base_problem();
        problem.viability.as_mut().expect("viability").status =
            ViabilityStatus::Rejected {
                witness_id: "no-recoverable-stance".to_owned(),
            };
        let report = qualify_operating_cell(&problem).expect("not qualified");
        assert_eq!(
            report.primary_disposition,
            PrimaryDisposition::EmpiricallyCapableButNotRobustlyCertified
        );
        assert!(!report.qualified);
        assert!(report.blockers.contains(&Blocker::ViabilityNotAdmitted));
    }

    #[test]
    fn robust_admission_does_not_override_performance_failure() {
        let mut problem = base_problem();
        problem.performance.as_mut().expect("performance").status =
            PerformanceStatus::Measured {
                success_lower_bound_ppm: 700_000,
                cross_fitted_cvar_lower_bound: 10,
            };
        let report = qualify_operating_cell(&problem).expect("not qualified");
        assert_eq!(
            report.primary_disposition,
            PrimaryDisposition::RobustlyAdmittedButPerformanceInsufficient
        );
        assert!(report.blockers.contains(&Blocker::PerformanceBelowThreshold));
    }

    #[test]
    fn safety_category_regression_blocks_qualification() {
        let mut problem = base_problem();
        problem.safety.as_mut().expect("safety").categories[1]
            .regressed_vs_matched_baseline = true;
        let report = qualify_operating_cell(&problem).expect("safety blocked");
        assert_eq!(
            report.primary_disposition,
            PrimaryDisposition::SafetyConstraintExceeded
        );
        assert!(!report.qualified);
    }

    #[test]
    fn incomplete_adverse_category_exposure_blocks_qualification() {
        let mut problem = base_problem();
        problem.safety.as_mut().expect("safety").categories[1].observed_exposure = 90;
        let report = qualify_operating_cell(&problem).expect("coverage blocked");
        assert_eq!(
            report.primary_disposition,
            PrimaryDisposition::EvidenceIncompleteOrUnknown
        );
        assert!(report.blockers.contains(&Blocker::SafetyMeasurementIncomplete));
    }

    #[test]
    fn insufficient_recovery_reserve_blocks_qualification() {
        let mut problem = base_problem();
        problem.energy.as_mut().expect("energy").remaining_recovery_reserve_mj = 99_999;
        let report = qualify_operating_cell(&problem).expect("reserve blocked");
        assert_eq!(
            report.primary_disposition,
            PrimaryDisposition::RecoveryReserveInsufficient
        );
    }

    #[test]
    fn stale_assurance_blocks_otherwise_passing_cell() {
        let mut problem = base_problem();
        problem.assurance.as_mut().expect("assurance").header.valid_until_ns = 499;
        let report = qualify_operating_cell(&problem).expect("stale assurance");
        assert_eq!(
            report.primary_disposition,
            PrimaryDisposition::AssuranceInvalidOrStale
        );
    }

    #[test]
    fn simulator_and_hardware_evidence_cannot_be_joined() {
        let mut problem = base_problem();
        problem.performance.as_mut().expect("performance")
            .header.identity.evidence_tier = EvidenceTier::Simulation;
        let report = qualify_operating_cell(&problem).expect("identity mismatch");
        assert_eq!(
            report.primary_disposition,
            PrimaryDisposition::EvidenceIdentityMismatch
        );
        assert!(report.blockers.contains(&Blocker::EvidenceIdentityMismatch));
    }

    #[test]
    fn missing_viability_cell_is_unknown_not_interpolated() {
        let mut problem = base_problem();
        problem.viability = None;
        let report = qualify_operating_cell(&problem).expect("unknown frontier");
        assert_eq!(
            report.primary_disposition,
            PrimaryDisposition::EvidenceIncompleteOrUnknown
        );
        assert_eq!(report.viability_class, ViabilityClass::Missing);
    }

    #[test]
    fn point_estimate_is_not_used_when_lower_bound_misses() {
        let mut problem = base_problem();
        problem.performance.as_mut().expect("performance").status =
            PerformanceStatus::Measured {
                success_lower_bound_ppm: 899_999,
                cross_fitted_cvar_lower_bound: 75,
            };
        let report = qualify_operating_cell(&problem).expect("LCB fails");
        assert_eq!(
            report.primary_disposition,
            PrimaryDisposition::RobustlyAdmittedButPerformanceInsufficient
        );
    }

    #[test]
    fn zero_event_count_does_not_override_safety_upper_bound() {
        let mut problem = base_problem();
        let category = &mut problem.safety.as_mut().expect("safety").categories[0];
        category.event_rate_upper_bound_ppm = 25_000;
        assert_eq!(category.maximum_event_rate_ppm, 20_000);
        let report = qualify_operating_cell(&problem).expect("upper bound fails");
        assert_eq!(
            report.primary_disposition,
            PrimaryDisposition::SafetyConstraintExceeded
        );
    }

    #[test]
    fn actuator_limit_invalid_blocks_qualification() {
        let mut problem = base_problem();
        problem.energy.as_mut().expect("energy").actuator_limits_valid = false;
        let report = qualify_operating_cell(&problem).expect("actuator limit");
        assert_eq!(report.resource_class, ResourceClass::ActuatorLimitInvalid);
        assert!(report.blockers.contains(&Blocker::ActuatorLimitInvalid));
    }

    #[test]
    fn adjacent_cell_does_not_inherit_qualification() {
        let qualified = qualify_operating_cell(&base_problem()).expect("cell a");
        assert!(qualified.qualified);
        let mut neighbour = base_problem();
        neighbour.cell.terrain_cell_id = "stairs-30cm".to_owned();
        let report = qualify_operating_cell(&neighbour).expect("neighbour identity mismatch");
        assert!(!report.qualified);
        assert_eq!(
            report.primary_disposition,
            PrimaryDisposition::EvidenceIdentityMismatch
        );
    }

    #[test]
    fn baseline_mismatch_blocks_comparison_not_cell_qualification() {
        let mut problem = base_problem();
        problem.baseline.as_mut().expect("baseline").identity.exposure_value = 80;
        let report = qualify_operating_cell(&problem).expect("cell still qualifies");
        assert!(report.qualified);
        assert!(!report.baseline_comparison_eligible);
        assert!(report.baseline_comparison_reason.is_some());
    }

    #[test]
    fn duplicate_evidence_receipt_ids_are_rejected() {
        let mut problem = base_problem();
        problem.performance.as_mut().expect("performance").header.receipt_id =
            "viability-r1".to_owned();
        assert_eq!(
            qualify_operating_cell(&problem),
            Err(QualificationError::DuplicateReceiptId)
        );
    }
}
