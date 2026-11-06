//! Evidence-bounded full-cycle rock-extraction trial validation.
//!
//! This crate validates comparison boundaries and computes descriptive pooled
//! rates from already-measured records. It is not a physical model, safety
//! authority, scientific protocol reviewer, MSE implementation, cost model,
//! statistical inference engine, or machine-control interface.
//!
//! Primary metrics use independently measured accepted solid volume and a
//! declared common energy boundary over full-cycle elapsed time. Loose spoil,
//! cutting-only time, tool-interface energy, model estimates and simulated
//! outputs cannot silently substitute for those measurands.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TrialArm {
    MechanicalBaseline,
    Candidate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum StudyDesign {
    RandomizedBlocked,
    RandomizedMatchedPair,
    ObservationalMatched,
    ObservationalAdjacentInterval,
    SingleArm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum EvidenceRole {
    IndependentMeasurement,
    DerivedFromIndependentMeasurements,
    ProcessSensor,
    OperatorReported,
    ModelEstimate,
    Simulation,
}

impl EvidenceRole {
    fn is_independent_measurement(self) -> bool {
        matches!(
            self,
            Self::IndependentMeasurement | Self::DerivedFromIndependentMeasurements
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum EnergyBasis {
    FacilityElectricalInput,
    MachineElectricalInput,
    DeclaredSystemEnergy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum VolumeBasis {
    AcceptedSolidVolume,
    RemovedSolidVolume,
    LooseSpoilVolume,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TimeBasis {
    FullCycle,
    ActiveCuttingOnly,
    ScheduledShift,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComparisonProfile {
    pub profile_id: String,
    pub protocol_ref: String,
    pub task_profile_ref: String,
    pub material_cohort_ref: String,
    pub material_pairing_ref: String,
    pub geometry_profile_ref: String,
    pub acceptance_profile_ref: String,
    pub site_context_ref: String,
    pub metrology_protocol_ref: String,
    pub energy_boundary_ref: String,
    pub energy_basis: EnergyBasis,
    /// Exact inventory of energy-consuming subsystems inside the comparison boundary.
    pub included_subsystems: Vec<String>,
    pub time_basis: TimeBasis,
    pub study_design: StudyDesign,
    pub minimum_replicates_per_arm: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnergyObservation {
    pub total_energy_kwh: f64,
    pub standard_uncertainty_kwh: f64,
    pub basis: EnergyBasis,
    pub boundary_ref: String,
    /// Declares how energy inputs and conversion losses are accounted for.
    pub energy_accounting_ref: String,
    /// Must exactly match the comparison profile's declared inventory.
    pub included_subsystems: Vec<String>,
    pub evidence_role: EvidenceRole,
    pub calibration_ref: String,
    pub lineage_ref: Option<String>,
    pub source_ref: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AcceptedOutputObservation {
    pub accepted_solid_volume_m3: f64,
    pub standard_uncertainty_m3: f64,
    pub basis: VolumeBasis,
    pub evidence_role: EvidenceRole,
    pub calibration_ref: String,
    pub lineage_ref: Option<String>,
    pub source_ref: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimeObservation {
    pub elapsed_hours: f64,
    pub standard_uncertainty_hours: f64,
    pub basis: TimeBasis,
    pub evidence_role: EvidenceRole,
    pub calibration_ref: String,
    pub lineage_ref: Option<String>,
    pub source_ref: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrialRecord {
    pub trial_id: String,
    pub profile_ref: String,
    pub protocol_ref: String,
    pub task_profile_ref: String,
    pub material_cohort_ref: String,
    pub material_pairing_ref: String,
    /// Unique physical specimen/block identity; cannot be reused across trials.
    pub specimen_ref: String,
    /// Shared pair identifier for matched baseline/candidate specimens.
    pub matched_pair_ref: String,
    pub geometry_profile_ref: String,
    pub acceptance_profile_ref: String,
    pub site_context_ref: String,
    pub metrology_protocol_ref: String,
    pub method_profile_ref: String,
    pub machine_profile_ref: String,
    pub tool_profile_ref: String,
    pub arm: TrialArm,
    pub energy: EnergyObservation,
    pub accepted_output: AcceptedOutputObservation,
    pub time: TimeObservation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationCode {
    EmptyProfileReference,
    EmptyRequiredSubsystem,
    DuplicateRequiredSubsystem,
    InvalidMinimumReplicates,
    EmptyTrialReference,
    DuplicateTrialId,
    DuplicateSpecimenReference,
    MissingMatchedPairReference,
    MatchedPairMismatch,
    MethodConfigurationMismatch,
    ProfileMismatch,
    InvalidNumericValue,
    InvalidUncertainty,
    AggregateMetricOutOfRange,
    NonIndependentEnergyEvidence,
    NonIndependentOutputEvidence,
    NonIndependentTimeEvidence,
    MissingEvidenceLineage,
    WrongOutputBasis,
    WrongTimeBasis,
    EnergyBasisMismatch,
    EnergyBoundaryMismatch,
    SubsystemInventoryMismatch,
    DuplicateIncludedSubsystem,
    MissingBaselineArm,
    MissingCandidateArm,
    InsufficientBaselineReplicates,
    InsufficientCandidateReplicates,
    UnsupportedSingleArmComparison,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationIssue {
    pub trial_id: Option<String>,
    pub code: ValidationCode,
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.trial_id {
            Some(id) => write!(f, "{:?} for trial {id}", self.code),
            None => write!(f, "{:?}", self.code),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationErrors(pub Vec<ValidationIssue>);

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, issue) in self.0.iter().enumerate() {
            if index > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{issue}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

#[derive(Clone, Debug, PartialEq)]
pub struct ArmMetrics {
    pub trial_count: usize,
    pub total_accepted_solid_volume_m3: f64,
    pub total_full_cycle_energy_kwh: f64,
    pub total_full_cycle_hours: f64,
    pub accepted_output_rate_m3_per_hour: f64,
    pub full_system_energy_kwh_per_accepted_m3: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComparisonSummary {
    pub profile_id: String,
    pub study_design: StudyDesign,
    pub baseline: ArmMetrics,
    pub candidate: ArmMetrics,
    /// Positive means the candidate arm's accepted output rate is higher.
    pub candidate_output_rate_change_fraction: f64,
    /// Negative means the candidate arm uses less declared-boundary energy per accepted m3.
    pub candidate_energy_intensity_change_fraction: f64,
    /// This crate only emits descriptive comparisons; a separate preregistered analysis is required.
    pub causal_effect_claim_allowed: bool,
}

/// Validate a profile and every trial, then compute descriptive pooled arm metrics.
///
/// The comparison is rejected if any record violates the common profile.
/// A "candidate" is not a winner; no inferential claim is made.
pub fn compare_trials(
    profile: &ComparisonProfile,
    trials: &[TrialRecord],
) -> Result<ComparisonSummary, ValidationErrors> {
    let mut issues = validate_profile(profile);

    if trials.is_empty() {
        issues.push(ValidationIssue {
            trial_id: None,
            code: ValidationCode::MissingBaselineArm,
        });
        issues.push(ValidationIssue {
            trial_id: None,
            code: ValidationCode::MissingCandidateArm,
        });
        return Err(ValidationErrors(issues));
    }

    let mut ids = BTreeSet::new();
    let mut specimen_ids = BTreeSet::new();
    let mut baseline = Vec::new();
    let mut candidate = Vec::new();

    for trial in trials {
        if trial.trial_id.trim().is_empty() {
            push_issue(&mut issues, None, ValidationCode::EmptyTrialReference);
        } else if !ids.insert(trial.trial_id.as_str()) {
            push_issue(
                &mut issues,
                Some(trial.trial_id.clone()),
                ValidationCode::DuplicateTrialId,
            );
        }

        validate_trial(profile, trial, &mut issues);
        if !trial.specimen_ref.trim().is_empty() && !specimen_ids.insert(trial.specimen_ref.as_str()) {
            push_issue(&mut issues, Some(trial.trial_id.clone()), ValidationCode::DuplicateSpecimenReference);
        }

        match trial.arm {
            TrialArm::MechanicalBaseline => baseline.push(trial),
            TrialArm::Candidate => candidate.push(trial),
        }
    }

    if profile.study_design == StudyDesign::SingleArm {
        push_issue(&mut issues, None, ValidationCode::UnsupportedSingleArmComparison);
    }
    validate_matched_pairs(profile, &baseline, &candidate, &mut issues);
    if !arm_configuration_is_consistent(&baseline) || !arm_configuration_is_consistent(&candidate) {
        push_issue(&mut issues, None, ValidationCode::MethodConfigurationMismatch);
    }
    if baseline.is_empty() {
        push_issue(&mut issues, None, ValidationCode::MissingBaselineArm);
    }
    if candidate.is_empty() {
        push_issue(&mut issues, None, ValidationCode::MissingCandidateArm);
    }
    if baseline.len() < profile.minimum_replicates_per_arm {
        push_issue(
            &mut issues,
            None,
            ValidationCode::InsufficientBaselineReplicates,
        );
    }
    if candidate.len() < profile.minimum_replicates_per_arm {
        push_issue(
            &mut issues,
            None,
            ValidationCode::InsufficientCandidateReplicates,
        );
    }

    if !issues.is_empty() {
        return Err(ValidationErrors(issues));
    }

    baseline.sort_by(|left, right| left.trial_id.cmp(&right.trial_id));
    candidate.sort_by(|left, right| left.trial_id.cmp(&right.trial_id));

    let baseline_metrics = aggregate_arm(&baseline);
    let candidate_metrics = aggregate_arm(&candidate);

    if !arm_metrics_are_valid(&baseline_metrics) || !arm_metrics_are_valid(&candidate_metrics) {
        push_issue(&mut issues, None, ValidationCode::AggregateMetricOutOfRange);
        return Err(ValidationErrors(issues));
    }

    let candidate_output_rate_change_fraction =
        candidate_metrics.accepted_output_rate_m3_per_hour
            / baseline_metrics.accepted_output_rate_m3_per_hour
            - 1.0;

    let candidate_energy_intensity_change_fraction =
        candidate_metrics.full_system_energy_kwh_per_accepted_m3
            / baseline_metrics.full_system_energy_kwh_per_accepted_m3
            - 1.0;

    if !candidate_output_rate_change_fraction.is_finite()
        || !candidate_energy_intensity_change_fraction.is_finite()
    {
        push_issue(&mut issues, None, ValidationCode::AggregateMetricOutOfRange);
        return Err(ValidationErrors(issues));
    }

    Ok(ComparisonSummary {
        profile_id: profile.profile_id.clone(),
        study_design: profile.study_design,
        baseline: baseline_metrics,
        candidate: candidate_metrics,
        candidate_output_rate_change_fraction,
        candidate_energy_intensity_change_fraction,
        causal_effect_claim_allowed: false,
    })
}

fn validate_profile(profile: &ComparisonProfile) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let references = [
        &profile.profile_id,
        &profile.protocol_ref,
        &profile.task_profile_ref,
        &profile.material_cohort_ref,
        &profile.material_pairing_ref,
        &profile.geometry_profile_ref,
        &profile.acceptance_profile_ref,
        &profile.site_context_ref,
        &profile.metrology_protocol_ref,
        &profile.energy_boundary_ref,
    ];

    if references.iter().any(|reference| reference.trim().is_empty()) {
        push_issue(&mut issues, None, ValidationCode::EmptyProfileReference);
    }

    if profile.included_subsystems.is_empty()
        || profile.included_subsystems.iter().any(|item| item.trim().is_empty())
    {
        push_issue(&mut issues, None, ValidationCode::EmptyRequiredSubsystem);
    }

    let mut subsystems = BTreeSet::new();
    for subsystem in &profile.included_subsystems {
        if !subsystems.insert(subsystem.as_str()) {
            push_issue(&mut issues, None, ValidationCode::DuplicateRequiredSubsystem);
        }
    }

    if profile.minimum_replicates_per_arm == 0 {
        push_issue(&mut issues, None, ValidationCode::InvalidMinimumReplicates);
    }
    if profile.time_basis != TimeBasis::FullCycle {
        push_issue(&mut issues, None, ValidationCode::WrongTimeBasis);
    }

    issues
}

fn validate_trial(
    profile: &ComparisonProfile,
    trial: &TrialRecord,
    issues: &mut Vec<ValidationIssue>,
) {
    let id = Some(trial.trial_id.clone());
    let references = [
        &trial.profile_ref,
        &trial.protocol_ref,
        &trial.task_profile_ref,
        &trial.material_cohort_ref,
        &trial.material_pairing_ref,
        &trial.specimen_ref,
        &trial.geometry_profile_ref,
        &trial.acceptance_profile_ref,
        &trial.site_context_ref,
        &trial.metrology_protocol_ref,
        &trial.method_profile_ref,
        &trial.machine_profile_ref,
        &trial.tool_profile_ref,
        &trial.energy.boundary_ref,
        &trial.energy.energy_accounting_ref,
        &trial.energy.calibration_ref,
        &trial.energy.source_ref,
        &trial.accepted_output.calibration_ref,
        &trial.accepted_output.source_ref,
        &trial.time.calibration_ref,
        &trial.time.source_ref,
    ];

    if references.iter().any(|reference| reference.trim().is_empty()) {
        push_issue(issues, id.clone(), ValidationCode::EmptyTrialReference);
    }

    if matches!(profile.study_design, StudyDesign::RandomizedMatchedPair | StudyDesign::ObservationalMatched)
        && trial.matched_pair_ref.trim().is_empty()
    {
        push_issue(issues, id.clone(), ValidationCode::MissingMatchedPairReference);
    }

    if trial.profile_ref != profile.profile_id
        || trial.protocol_ref != profile.protocol_ref
        || trial.task_profile_ref != profile.task_profile_ref
        || trial.material_cohort_ref != profile.material_cohort_ref
        || trial.material_pairing_ref != profile.material_pairing_ref
        || trial.geometry_profile_ref != profile.geometry_profile_ref
        || trial.acceptance_profile_ref != profile.acceptance_profile_ref
        || trial.site_context_ref != profile.site_context_ref
        || trial.metrology_protocol_ref != profile.metrology_protocol_ref
    {
        push_issue(issues, id.clone(), ValidationCode::ProfileMismatch);
    }

    if trial.energy.basis != profile.energy_basis {
        push_issue(issues, id.clone(), ValidationCode::EnergyBasisMismatch);
    }
    if trial.energy.boundary_ref != profile.energy_boundary_ref {
        push_issue(issues, id.clone(), ValidationCode::EnergyBoundaryMismatch);
    }

    if has_duplicates(&trial.energy.included_subsystems)
        || trial.energy.included_subsystems.iter().any(|item| item.trim().is_empty())
    {
        push_issue(issues, id.clone(), ValidationCode::DuplicateIncludedSubsystem);
    }

    if set_of(&trial.energy.included_subsystems) != set_of(&profile.included_subsystems) {
        push_issue(issues, id.clone(), ValidationCode::SubsystemInventoryMismatch);
    }

    if !is_positive_finite(trial.energy.total_energy_kwh) {
        push_issue(issues, id.clone(), ValidationCode::InvalidNumericValue);
    }
    if !is_nonnegative_finite(trial.energy.standard_uncertainty_kwh) {
        push_issue(issues, id.clone(), ValidationCode::InvalidUncertainty);
    }
    if !trial.energy.evidence_role.is_independent_measurement() {
        push_issue(issues, id.clone(), ValidationCode::NonIndependentEnergyEvidence);
    }
    if !valid_lineage(trial.energy.evidence_role, trial.energy.lineage_ref.as_ref()) {
        push_issue(issues, id.clone(), ValidationCode::MissingEvidenceLineage);
    }

    if !is_positive_finite(trial.accepted_output.accepted_solid_volume_m3) {
        push_issue(issues, id.clone(), ValidationCode::InvalidNumericValue);
    }
    if !is_nonnegative_finite(trial.accepted_output.standard_uncertainty_m3) {
        push_issue(issues, id.clone(), ValidationCode::InvalidUncertainty);
    }
    if trial.accepted_output.basis != VolumeBasis::AcceptedSolidVolume {
        push_issue(issues, id.clone(), ValidationCode::WrongOutputBasis);
    }
    if !trial.accepted_output.evidence_role.is_independent_measurement() {
        push_issue(issues, id.clone(), ValidationCode::NonIndependentOutputEvidence);
    }
    if !valid_lineage(trial.accepted_output.evidence_role, trial.accepted_output.lineage_ref.as_ref()) {
        push_issue(issues, id.clone(), ValidationCode::MissingEvidenceLineage);
    }

    if !is_positive_finite(trial.time.elapsed_hours) {
        push_issue(issues, id.clone(), ValidationCode::InvalidNumericValue);
    }
    if !is_nonnegative_finite(trial.time.standard_uncertainty_hours) {
        push_issue(issues, id.clone(), ValidationCode::InvalidUncertainty);
    }
    if trial.time.basis != TimeBasis::FullCycle {
        push_issue(issues, id.clone(), ValidationCode::WrongTimeBasis);
    }
    if !trial.time.evidence_role.is_independent_measurement() {
        push_issue(issues, id.clone(), ValidationCode::NonIndependentTimeEvidence);
    }
    if !valid_lineage(trial.time.evidence_role, trial.time.lineage_ref.as_ref()) {
        push_issue(issues, id, ValidationCode::MissingEvidenceLineage);
    }
}

fn validate_matched_pairs(
    profile: &ComparisonProfile,
    baseline: &[&TrialRecord],
    candidate: &[&TrialRecord],
    issues: &mut Vec<ValidationIssue>,
) {
    if !matches!(profile.study_design, StudyDesign::RandomizedMatchedPair | StudyDesign::ObservationalMatched) {
        return;
    }

    let mut pairs: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for trial in baseline {
        if !trial.matched_pair_ref.trim().is_empty() {
            pairs.entry(trial.matched_pair_ref.as_str()).or_default().0 += 1;
        }
    }
    for trial in candidate {
        if !trial.matched_pair_ref.trim().is_empty() {
            pairs.entry(trial.matched_pair_ref.as_str()).or_default().1 += 1;
        }
    }
    if pairs.values().any(|counts| *counts != (1, 1)) {
        push_issue(issues, None, ValidationCode::MatchedPairMismatch);
    }
}

fn arm_configuration_is_consistent(trials: &[&TrialRecord]) -> bool {
    let Some(first) = trials.first() else { return true; };
    trials.iter().all(|trial| {
        trial.method_profile_ref == first.method_profile_ref
            && trial.machine_profile_ref == first.machine_profile_ref
            && trial.tool_profile_ref == first.tool_profile_ref
    })
}

fn arm_metrics_are_valid(metrics: &ArmMetrics) -> bool {
    is_positive_finite(metrics.total_accepted_solid_volume_m3)
        && is_positive_finite(metrics.total_full_cycle_energy_kwh)
        && is_positive_finite(metrics.total_full_cycle_hours)
        && is_positive_finite(metrics.accepted_output_rate_m3_per_hour)
        && is_positive_finite(metrics.full_system_energy_kwh_per_accepted_m3)
}

fn aggregate_arm(trials: &[&TrialRecord]) -> ArmMetrics {
    let total_volume: f64 = trials
        .iter()
        .map(|trial| trial.accepted_output.accepted_solid_volume_m3)
        .sum();
    let total_energy: f64 = trials
        .iter()
        .map(|trial| trial.energy.total_energy_kwh)
        .sum();
    let total_hours: f64 = trials
        .iter()
        .map(|trial| trial.time.elapsed_hours)
        .sum();

    ArmMetrics {
        trial_count: trials.len(),
        total_accepted_solid_volume_m3: total_volume,
        total_full_cycle_energy_kwh: total_energy,
        total_full_cycle_hours: total_hours,
        accepted_output_rate_m3_per_hour: total_volume / total_hours,
        full_system_energy_kwh_per_accepted_m3: total_energy / total_volume,
    }
}

fn valid_lineage(role: EvidenceRole, lineage_ref: Option<&String>) -> bool {
    match role {
        EvidenceRole::IndependentMeasurement => lineage_ref
            .map(|reference| !reference.trim().is_empty())
            .unwrap_or(true),
        EvidenceRole::DerivedFromIndependentMeasurements => lineage_ref
            .map(|reference| !reference.trim().is_empty())
            .unwrap_or(false),
        EvidenceRole::ProcessSensor
        | EvidenceRole::OperatorReported
        | EvidenceRole::ModelEstimate
        | EvidenceRole::Simulation => true,
    }
}

fn is_positive_finite(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn is_nonnegative_finite(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn has_duplicates(items: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    items.iter().any(|item| !seen.insert(item.as_str()))
}

fn set_of(items: &[String]) -> BTreeSet<&str> {
    items.iter().map(String::as_str).collect()
}

fn push_issue(
    issues: &mut Vec<ValidationIssue>,
    trial_id: Option<String>,
    code: ValidationCode,
) {
    issues.push(ValidationIssue { trial_id, code });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> ComparisonProfile {
        ComparisonProfile {
            profile_id: "comparison-profile:v1".to_owned(),
            protocol_ref: "protocol:paired-granite:v1".to_owned(),
            task_profile_ref: "task:slot-cut:100x50:v1".to_owned(),
            material_cohort_ref: "cohort:granite-a:v1".to_owned(),
            material_pairing_ref: "pairing:block-17:v1".to_owned(),
            geometry_profile_ref: "geometry:slot-100x50:v1".to_owned(),
            acceptance_profile_ref: "acceptance:solid-volume-and-wall-quality:v1".to_owned(),
            site_context_ref: "site:lab-rig-02:v1".to_owned(),
            metrology_protocol_ref: "metrology:independent-scan-weigh:v1".to_owned(),
            energy_boundary_ref: "energy:facility-meter-plus-listed-aux:v1".to_owned(),
            energy_basis: EnergyBasis::FacilityElectricalInput,
            included_subsystems: vec![
                "primary_tool".to_owned(),
                "cooling".to_owned(),
                "dust_extraction".to_owned(),
            ],
            time_basis: TimeBasis::FullCycle,
            study_design: StudyDesign::RandomizedMatchedPair,
            minimum_replicates_per_arm: 2,
        }
    }

    fn trial(id: &str, arm: TrialArm, volume: f64, energy: f64, hours: f64) -> TrialRecord {
        let profile = profile();
        TrialRecord {
            trial_id: id.to_owned(),
            profile_ref: profile.profile_id,
            protocol_ref: profile.protocol_ref,
            task_profile_ref: profile.task_profile_ref,
            material_cohort_ref: profile.material_cohort_ref,
            material_pairing_ref: profile.material_pairing_ref,
            specimen_ref: format!("specimen:{id}"),
            matched_pair_ref: if id.ends_with('1') { "matched-pair:01".to_owned() } else { "matched-pair:02".to_owned() },
            geometry_profile_ref: profile.geometry_profile_ref,
            acceptance_profile_ref: profile.acceptance_profile_ref,
            site_context_ref: profile.site_context_ref,
            metrology_protocol_ref: profile.metrology_protocol_ref,
            method_profile_ref: match arm {
                TrialArm::MechanicalBaseline => "method:disc-cutter:v1".to_owned(),
                TrialArm::Candidate => "method:sparc-plus-pdc:v1".to_owned(),
            },
            machine_profile_ref: match arm {
                TrialArm::MechanicalBaseline => "machine:mechanical-rig:v1".to_owned(),
                TrialArm::Candidate => "machine:hybrid-rig:v1".to_owned(),
            },
            tool_profile_ref: match arm {
                TrialArm::MechanicalBaseline => "tool:disc-cutter:v1".to_owned(),
                TrialArm::Candidate => "tool:sparc-pdc:v1".to_owned(),
            },
            arm,
            energy: EnergyObservation {
                total_energy_kwh: energy,
                standard_uncertainty_kwh: energy * 0.01,
                basis: EnergyBasis::FacilityElectricalInput,
                boundary_ref: profile.energy_boundary_ref,
                energy_accounting_ref: "energy-accounting:site-input-v1".to_owned(),
                included_subsystems: profile.included_subsystems,
                evidence_role: EvidenceRole::IndependentMeasurement,
                calibration_ref: "calibration:facility-meter-7:v1".to_owned(),
                lineage_ref: None,
                source_ref: format!("meter:{id}"),
            },
            accepted_output: AcceptedOutputObservation {
                accepted_solid_volume_m3: volume,
                standard_uncertainty_m3: volume * 0.01,
                basis: VolumeBasis::AcceptedSolidVolume,
                evidence_role: EvidenceRole::IndependentMeasurement,
                calibration_ref: "calibration:laser-scan-scale:v1".to_owned(),
                lineage_ref: None,
                source_ref: format!("scan-weigh:{id}"),
            },
            time: TimeObservation {
                elapsed_hours: hours,
                standard_uncertainty_hours: hours * 0.01,
                basis: TimeBasis::FullCycle,
                evidence_role: EvidenceRole::IndependentMeasurement,
                calibration_ref: "calibration:clock-sync:v1".to_owned(),
                lineage_ref: None,
                source_ref: format!("clock-log:{id}"),
            },
        }
    }

    fn four_trials() -> Vec<TrialRecord> {
        vec![
            trial("base-1", TrialArm::MechanicalBaseline, 0.10, 12.0, 2.0),
            trial("base-2", TrialArm::MechanicalBaseline, 0.10, 10.0, 2.0),
            trial("candidate-1", TrialArm::Candidate, 0.12, 10.0, 1.8),
            trial("candidate-2", TrialArm::Candidate, 0.12, 9.0, 1.8),
        ]
    }

    fn has_code(errors: &ValidationErrors, code: ValidationCode) -> bool {
        errors.0.iter().any(|issue| issue.code == code)
    }

    #[test]
    fn valid_matched_comparison_returns_descriptive_pooled_metrics() {
        let result = compare_trials(&profile(), &four_trials()).unwrap();
        assert_eq!(result.baseline.trial_count, 2);
        assert_eq!(result.candidate.trial_count, 2);
        assert!((result.baseline.accepted_output_rate_m3_per_hour - 0.05).abs() < 1e-12);
        assert!((result.candidate.accepted_output_rate_m3_per_hour - (0.24 / 3.6)).abs() < 1e-12);
        assert!(!result.causal_effect_claim_allowed);
    }

    #[test]
    fn model_estimated_output_cannot_be_counted_as_independent_acceptance() {
        let mut trials = four_trials();
        trials[2].accepted_output.evidence_role = EvidenceRole::ModelEstimate;
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::NonIndependentOutputEvidence));
    }

    #[test]
    fn loose_spoil_cannot_replace_accepted_solid_volume() {
        let mut trials = four_trials();
        trials[2].accepted_output.basis = VolumeBasis::LooseSpoilVolume;
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::WrongOutputBasis));
    }

    #[test]
    fn active_cutting_time_cannot_replace_full_cycle_time() {
        let mut trials = four_trials();
        trials[2].time.basis = TimeBasis::ActiveCuttingOnly;
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::WrongTimeBasis));
    }

    #[test]
    fn different_energy_meter_boundaries_cannot_be_compared() {
        let mut trials = four_trials();
        trials[2].energy.boundary_ref = "energy:tool-interface-only:v1".to_owned();
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::EnergyBoundaryMismatch));
    }

    #[test]
    fn incomplete_auxiliary_subsystem_inventory_is_rejected() {
        let mut trials = four_trials();
        trials[2].energy.included_subsystems.pop();
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::SubsystemInventoryMismatch));
    }

    #[test]
    fn process_sensor_energy_is_not_primary_meter_evidence() {
        let mut trials = four_trials();
        trials[2].energy.evidence_role = EvidenceRole::ProcessSensor;
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::NonIndependentEnergyEvidence));
    }

    #[test]
    fn duplicate_trial_ids_are_rejected() {
        let mut trials = four_trials();
        trials[3].trial_id = trials[0].trial_id.clone();
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::DuplicateTrialId));
    }

    #[test]
    fn changed_material_cohort_is_rejected() {
        let mut trials = four_trials();
        trials[2].material_cohort_ref = "cohort:basalt-b:v1".to_owned();
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::ProfileMismatch));
    }

    #[test]
    fn observational_adjacent_interval_comparison_remains_descriptive() {
        let mut comparison_profile = profile();
        comparison_profile.study_design = StudyDesign::ObservationalAdjacentInterval;
        let result = compare_trials(&comparison_profile, &four_trials()).unwrap();
        assert_eq!(result.study_design, StudyDesign::ObservationalAdjacentInterval);
        assert!(!result.causal_effect_claim_allowed);
    }

    #[test]
    fn insufficient_replicates_are_rejected() {
        let trials = vec![
            trial("base-1", TrialArm::MechanicalBaseline, 0.10, 12.0, 2.0),
            trial("candidate-1", TrialArm::Candidate, 0.12, 10.0, 1.8),
        ];
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::InsufficientBaselineReplicates));
        assert!(has_code(&errors, ValidationCode::InsufficientCandidateReplicates));
    }

    #[test]
    fn candidate_must_not_omit_machine_and_tool_identity() {
        let mut trials = four_trials();
        trials[2].machine_profile_ref.clear();
        trials[2].tool_profile_ref.clear();
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::EmptyTrialReference));
    }

    #[test]
    fn non_finite_and_zero_denominators_are_rejected() {
        let mut trials = four_trials();
        trials[2].energy.total_energy_kwh = f64::NAN;
        trials[3].time.elapsed_hours = 0.0;
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::InvalidNumericValue));
    }

    #[test]
    fn declared_system_boundary_basis_must_match_profile_exactly() {
        let mut trials = four_trials();
        trials[2].energy.basis = EnergyBasis::DeclaredSystemEnergy;
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::EnergyBasisMismatch));
    }

    #[test]
    fn single_arm_design_cannot_be_used_as_a_pairwise_comparison() {
        let mut comparison_profile = profile();
        comparison_profile.study_design = StudyDesign::SingleArm;
        let errors = compare_trials(&comparison_profile, &four_trials()).unwrap_err();
        assert!(has_code(&errors, ValidationCode::UnsupportedSingleArmComparison));
    }

    #[test]
    fn trial_order_does_not_change_pooled_summary() {
        let mut trials = four_trials();
        let first = compare_trials(&profile(), &trials).unwrap();
        trials.reverse();
        let second = compare_trials(&profile(), &trials).unwrap();
        assert_eq!(first.baseline.total_accepted_solid_volume_m3, second.baseline.total_accepted_solid_volume_m3);
        assert_eq!(first.candidate.total_full_cycle_energy_kwh, second.candidate.total_full_cycle_energy_kwh);
        assert_eq!(first.candidate_output_rate_change_fraction, second.candidate_output_rate_change_fraction);
    }

    #[test]
    fn missing_energy_accounting_reference_is_rejected() {
        let mut trials = four_trials();
        trials[2].energy.energy_accounting_ref.clear();
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::EmptyTrialReference));
    }

    #[test]
    fn missing_output_calibration_reference_is_rejected() {
        let mut trials = four_trials();
        trials[2].accepted_output.calibration_ref.clear();
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::EmptyTrialReference));
    }

    #[test]
    fn negative_standard_uncertainty_is_rejected() {
        let mut trials = four_trials();
        trials[2].energy.standard_uncertainty_kwh = -0.1;
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::InvalidUncertainty));
    }

    #[test]
    fn duplicate_subsystem_entry_in_a_trial_is_rejected() {
        let mut trials = four_trials();
        trials[2].energy.included_subsystems.push("cooling".to_owned());
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::DuplicateIncludedSubsystem));
    }

    #[test]
    fn changed_acceptance_profile_is_rejected() {
        let mut trials = four_trials();
        trials[2].acceptance_profile_ref = "acceptance:volume-only:v1".to_owned();
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::ProfileMismatch));
    }

    #[test]
    fn operator_reported_elapsed_time_is_not_primary_time_evidence() {
        let mut trials = four_trials();
        trials[2].time.evidence_role = EvidenceRole::OperatorReported;
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::NonIndependentTimeEvidence));
    }

    #[test]
    fn zero_accepted_solid_volume_is_rejected() {
        let mut trials = four_trials();
        trials[2].accepted_output.accepted_solid_volume_m3 = 0.0;
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::InvalidNumericValue));
    }

    #[test]
    fn duplicate_profile_subsystem_inventory_is_rejected() {
        let mut comparison_profile = profile();
        comparison_profile.included_subsystems.push("cooling".to_owned());
        let errors = compare_trials(&comparison_profile, &four_trials()).unwrap_err();
        assert!(has_code(&errors, ValidationCode::DuplicateRequiredSubsystem));
    }

    #[test]
    fn missing_candidate_arm_is_rejected() {
        let trials = vec![
            trial("base-1", TrialArm::MechanicalBaseline, 0.10, 12.0, 2.0),
            trial("base-2", TrialArm::MechanicalBaseline, 0.10, 10.0, 2.0),
        ];
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::MissingCandidateArm));
    }

    #[test]
    fn missing_baseline_arm_is_rejected() {
        let trials = vec![
            trial("candidate-1", TrialArm::Candidate, 0.12, 10.0, 1.8),
            trial("candidate-2", TrialArm::Candidate, 0.12, 9.0, 1.8),
        ];
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::MissingBaselineArm));
    }


    #[test]
    fn aggregate_overflow_is_rejected_even_when_individual_values_are_finite() {
        let mut trials = four_trials();
        trials[0].energy.total_energy_kwh = f64::MAX;
        trials[1].energy.total_energy_kwh = f64::MAX;
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::AggregateMetricOutOfRange));
    }


    #[test]
    fn derived_primary_measurement_requires_source_lineage() {
        let mut trials = four_trials();
        trials[2].accepted_output.evidence_role = EvidenceRole::DerivedFromIndependentMeasurements;
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::MissingEvidenceLineage));
    }

    #[test]
    fn derived_primary_measurement_with_lineage_is_admissible() {
        let mut trials = four_trials();
        trials[2].accepted_output.evidence_role = EvidenceRole::DerivedFromIndependentMeasurements;
        trials[2].accepted_output.lineage_ref = Some("lineage:scan-and-weigh-raw:v1".to_owned());
        let result = compare_trials(&profile(), &trials).unwrap();
        assert_eq!(result.candidate.trial_count, 2);
    }


    #[test]
    fn matched_pair_design_requires_one_baseline_and_one_candidate_per_pair() {
        let mut trials = four_trials();
        trials[3].matched_pair_ref = "matched-pair:01".to_owned();
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::MatchedPairMismatch));
    }

    #[test]
    fn matched_pair_design_requires_pair_reference() {
        let mut trials = four_trials();
        trials[2].matched_pair_ref.clear();
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::MissingMatchedPairReference));
    }

    #[test]
    fn duplicate_physical_specimen_ids_are_rejected() {
        let mut trials = four_trials();
        trials[2].specimen_ref = trials[0].specimen_ref.clone();
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::DuplicateSpecimenReference));
    }

    #[test]
    fn one_arm_cannot_pool_multiple_tool_configurations() {
        let mut trials = four_trials();
        trials[3].tool_profile_ref = "tool:alternate-disc-cutter:v1".to_owned();
        let errors = compare_trials(&profile(), &trials).unwrap_err();
        assert!(has_code(&errors, ValidationCode::MethodConfigurationMismatch));
    }

}
