#![forbid(unsafe_code)]

//! Hard-gated multiobjective comparison for textile schedule candidates.
//!
//! This module ranks only candidates that have already passed independent
//! schedule, route, utility, receiver-acceptance and (when required) disruption
//! gates. It computes Pareto fronts over a fixed, versioned objective vector;
//! it does not construct schedules or manufacture a weighted scalar score.

use std::collections::{BTreeMap, BTreeSet};

use crate::resilience::ResilienceReport;
use crate::utilities::UtilityReport;
use crate::{Finding, ValidationReport};

pub const OBJECTIVE_SCHEMA_VERSION: &str = "textile-schedule-objectives-v0.1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectiveMetric {
    /// Receiver-accepted output, milligrams.
    AcceptedOutputMassMg,
    /// Conservative fibre-length-qualified output per incoming mass, ppm.
    FibreLengthQualifiedOutputPerInputPpm,
    /// Residual attachment/trim mass, milligrams.
    ResidualTrimMassMg,
    /// Independently inspected damaged textile area, square millimetres.
    DamageAreaMm2,
    /// Electricity consumed per receiver-accepted kilogram, watt-hours.
    ElectricityWhPerAcceptedKg,
    /// Steam consumed per receiver-accepted kilogram, grams.
    SteamGramsPerAcceptedKg,
    /// Water consumed per receiver-accepted kilogram, millilitres.
    WaterMlPerAcceptedKg,
    /// Wastewater generated per receiver-accepted kilogram, millilitres.
    WastewaterMlPerAcceptedKg,
    /// Operator and setup labour per receiver-accepted kilogram, minutes.
    OperatorMinutesPerAcceptedKg,
    /// Schedule makespan, caller-declared integer ticks.
    MakespanTicks,
    /// Total tardiness, caller-declared integer ticks.
    TotalTardinessTicks,
    /// Number of changed/added/dropped/reassigned production operations.
    PlanChurnOperationCount,
    /// Completion horizon after the disruption observation tick, in ticks.
    RecoveryHorizonTicks,
}

impl ObjectiveMetric {
    pub const ALL: [Self; 13] = [
        Self::AcceptedOutputMassMg,
        Self::FibreLengthQualifiedOutputPerInputPpm,
        Self::ResidualTrimMassMg,
        Self::DamageAreaMm2,
        Self::ElectricityWhPerAcceptedKg,
        Self::SteamGramsPerAcceptedKg,
        Self::WaterMlPerAcceptedKg,
        Self::WastewaterMlPerAcceptedKg,
        Self::OperatorMinutesPerAcceptedKg,
        Self::MakespanTicks,
        Self::TotalTardinessTicks,
        Self::PlanChurnOperationCount,
        Self::RecoveryHorizonTicks,
    ];

    pub fn unit(self) -> &'static str {
        match self {
            Self::AcceptedOutputMassMg => "mg",
            Self::FibreLengthQualifiedOutputPerInputPpm => "ppm",
            Self::ResidualTrimMassMg => "mg",
            Self::DamageAreaMm2 => "mm2",
            Self::ElectricityWhPerAcceptedKg => "Wh/kg_accepted",
            Self::SteamGramsPerAcceptedKg => "g/kg_accepted",
            Self::WaterMlPerAcceptedKg => "mL/kg_accepted",
            Self::WastewaterMlPerAcceptedKg => "mL/kg_accepted",
            Self::OperatorMinutesPerAcceptedKg => "min/kg_accepted",
            Self::MakespanTicks => "tick",
            Self::TotalTardinessTicks => "tick",
            Self::PlanChurnOperationCount => "operation",
            Self::RecoveryHorizonTicks => "tick",
        }
    }

    fn direction(self) -> Direction {
        match self {
            Self::AcceptedOutputMassMg
            | Self::FibreLengthQualifiedOutputPerInputPpm => Direction::Maximize,
            Self::ResidualTrimMassMg
            | Self::DamageAreaMm2
            | Self::ElectricityWhPerAcceptedKg
            | Self::SteamGramsPerAcceptedKg
            | Self::WaterMlPerAcceptedKg
            | Self::WastewaterMlPerAcceptedKg
            | Self::OperatorMinutesPerAcceptedKg
            | Self::MakespanTicks
            | Self::TotalTardinessTicks
            | Self::PlanChurnOperationCount
            | Self::RecoveryHorizonTicks => Direction::Minimize,
        }
    }

    fn required_basis(self) -> &'static [MetricEvidenceBasis] {
        use MetricEvidenceBasis as Basis;
        match self {
            Self::AcceptedOutputMassMg => &[Basis::IndependentReceiverAcceptance],
            Self::FibreLengthQualifiedOutputPerInputPpm => &[Basis::PhysicalFibreMetrology],
            Self::ResidualTrimMassMg | Self::DamageAreaMm2 => &[Basis::IndependentPhysicalInspection],
            Self::ElectricityWhPerAcceptedKg
            | Self::SteamGramsPerAcceptedKg
            | Self::WaterMlPerAcceptedKg
            | Self::WastewaterMlPerAcceptedKg => &[
                Basis::CalibratedMeterObservation,
                Basis::CalibratedProcessTrial,
            ],
            Self::OperatorMinutesPerAcceptedKg => &[Basis::WorkerTimeRecord],
            Self::MakespanTicks
            | Self::TotalTardinessTicks
            | Self::PlanChurnOperationCount
            | Self::RecoveryHorizonTicks => &[Basis::IndependentScheduleReplay],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Maximize,
    Minimize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricEvidenceBasis {
    IndependentReceiverAcceptance,
    PhysicalFibreMetrology,
    IndependentPhysicalInspection,
    CalibratedMeterObservation,
    CalibratedProcessTrial,
    WorkerTimeRecord,
    IndependentScheduleReplay,
    EngineeringEstimate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricEvidence {
    pub evidence_id: String,
    pub sha256: String,
    pub method_ref: String,
    pub basis: MetricEvidenceBasis,
    /// Exact physical lot to which this observation refers, when applicable.
    pub lot_id: String,
    /// Exact physical process trial to which this observation refers, when applicable.
    pub process_trial_evidence_id: String,
    /// Exact baseline/disruption scenario for schedule, utility and labour observations.
    pub scenario_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectiveObservation {
    pub metric: ObjectiveMetric,
    pub value: u64,
    pub unit: String,
    pub evidence: MetricEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiverOutcome {
    Accepted,
    Rejected,
    Conditional,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiverAcceptance {
    pub outcome: ReceiverOutcome,
    pub output_lot_id: String,
    pub process_trial_evidence_id: String,
    pub specification_id: String,
    pub specification_version: String,
    pub accepted_mass_mg: u64,
    pub issuer_id: String,
    pub verifier_id: String,
    pub evidence_id: String,
    pub evidence_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub candidate_id: String,
    pub benchmark_suite_id: String,
    pub benchmark_suite_version: String,
    pub objective_schema_version: String,
    pub hard_gate_policy_version: String,
    pub problem_name: String,
    pub scenario_id: String,
    pub requires_disruption_replay: bool,
    pub input_lot_id: String,
    pub input_mass_mg: u64,
    pub output_lot_id: String,
    pub process_trial_evidence_id: String,
    pub receiving_specification_id: String,
    pub receiving_specification_version: String,
    pub utility_tick_unit: String,
    pub route_receipt_ids: Vec<String>,
    pub schedule_validation: ValidationReport,
    /// Findings from validate_route_gates for this exact candidate/problem.
    pub route_findings: Vec<Finding>,
    /// None is missing utility evidence, not zero demand or zero consumption.
    pub utility_validation: Option<UtilityReport>,
    /// Required only for a disruption tournament; the report must match scenario_id.
    pub resilience_validation: Option<ResilienceReport>,
    pub receiver_acceptance: ReceiverAcceptance,
    pub objectives: Vec<ObjectiveObservation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateEligibility {
    Eligible,
    Ineligible,
    InsufficientEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateFinding {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateAssessment {
    pub candidate_id: String,
    pub eligibility: CandidateEligibility,
    pub findings: Vec<CandidateFinding>,
    pub objective_values: BTreeMap<ObjectiveMetric, u64>,
    pub pareto_front_index: Option<usize>,
    pub dominated_by: Vec<String>,
    pub dominates: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DominanceRelation {
    pub dominates: String,
    pub dominated: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TournamentFinding {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TournamentReport {
    pub valid: bool,
    pub benchmark_suite_id: String,
    pub benchmark_suite_version: String,
    pub scenario_id: String,
    pub input_lot_id: String,
    pub receiving_specification_id: String,
    pub objective_schema_version: String,
    pub assessments: Vec<CandidateAssessment>,
    /// Each front is sorted by candidate_id for deterministic output.
    pub pareto_fronts: Vec<Vec<String>>,
    pub dominance_relations: Vec<DominanceRelation>,
    pub findings: Vec<TournamentFinding>,
}

fn add(
    findings: &mut Vec<CandidateFinding>,
    code: &'static str,
    message: impl Into<String>,
) {
    findings.push(CandidateFinding {
        code,
        message: message.into(),
    });
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn candidate_metadata_findings(candidate: &Candidate, reference: &Candidate) -> Vec<CandidateFinding> {
    let mut findings = Vec::new();
    if candidate.benchmark_suite_id != reference.benchmark_suite_id
        || candidate.benchmark_suite_version != reference.benchmark_suite_version
        || candidate.objective_schema_version != reference.objective_schema_version
        || candidate.hard_gate_policy_version != reference.hard_gate_policy_version
        || candidate.problem_name != reference.problem_name
        || candidate.scenario_id != reference.scenario_id
        || candidate.requires_disruption_replay != reference.requires_disruption_replay
        || candidate.input_lot_id != reference.input_lot_id
        || candidate.input_mass_mg != reference.input_mass_mg
        || candidate.receiving_specification_id != reference.receiving_specification_id
        || candidate.receiving_specification_version != reference.receiving_specification_version
        || candidate.utility_tick_unit != reference.utility_tick_unit
    {
        add(
            &mut findings,
            "TOURNAMENT_IDENTITY_MISMATCH",
            "candidate differs in benchmark suite/version, scenario, incoming lot/mass, receiving specification, utility tick unit, objective schema or hard-gate policy",
        );
    }
    findings
}

fn validate_receiver(candidate: &Candidate, findings: &mut Vec<CandidateFinding>) {
    let acceptance = &candidate.receiver_acceptance;
    if acceptance.outcome == ReceiverOutcome::Rejected {
        add(
            findings,
            "RECEIVER_REJECTED",
            "the exact output lot was rejected by its receiving process",
        );
    } else if acceptance.outcome != ReceiverOutcome::Accepted {
        add(
            findings,
            "RECEIVER_ACCEPTANCE_UNRESOLVED",
            "conditional or unknown receiver disposition cannot enter the accepted-output Pareto set",
        );
    }
    if !nonempty(&acceptance.output_lot_id)
        || acceptance.output_lot_id != candidate.output_lot_id
        || !nonempty(&acceptance.process_trial_evidence_id)
        || acceptance.process_trial_evidence_id != candidate.process_trial_evidence_id
        || !nonempty(&acceptance.specification_id)
        || acceptance.specification_id != candidate.receiving_specification_id
        || acceptance.specification_version != candidate.receiving_specification_version
        || acceptance.accepted_mass_mg == 0
        || acceptance.accepted_mass_mg > candidate.input_mass_mg
    {
        add(
            findings,
            "RECEIVER_BINDING_INVALID",
            "receiver acceptance must bind the exact output lot, producing trial, specification/version and positive accepted mass not exceeding input mass",
        );
    }
    if !nonempty(&acceptance.issuer_id)
        || !nonempty(&acceptance.verifier_id)
        || acceptance.issuer_id == acceptance.verifier_id
        || !nonempty(&acceptance.evidence_id)
        || !valid_sha256(&acceptance.evidence_sha256)
    {
        add(
            findings,
            "RECEIVER_PROVENANCE_INVALID",
            "receiver evidence requires a SHA-256 reference and distinct non-empty issuer/verifier identities",
        );
    }
}

fn validate_objectives(
    candidate: &Candidate,
    findings: &mut Vec<CandidateFinding>,
) -> BTreeMap<ObjectiveMetric, u64> {
    let mut observations = BTreeMap::new();
    for observation in &candidate.objectives {
        if observations.insert(observation.metric, observation).is_some() {
            add(
                findings,
                "OBJECTIVE_DUPLICATE",
                format!("objective {:?} is supplied more than once", observation.metric),
            );
        }
    }
    for metric in ObjectiveMetric::ALL {
        let Some(observation) = observations.get(&metric) else {
            add(
                findings,
                "OBJECTIVE_MISSING",
                format!("required objective {:?} is absent", metric),
            );
            continue;
        };
        if observation.unit != metric.unit() {
            add(
                findings,
                "OBJECTIVE_UNIT_MISMATCH",
                format!(
                    "objective {:?} must use unit {}, not {}",
                    metric,
                    metric.unit(),
                    observation.unit
                ),
            );
        }
        if !nonempty(&observation.evidence.evidence_id)
            || !valid_sha256(&observation.evidence.sha256)
            || !nonempty(&observation.evidence.method_ref)
        {
            add(
                findings,
                "OBJECTIVE_EVIDENCE_INVALID",
                format!("objective {:?} lacks a valid evidence reference/method", metric),
            );
        }
        let lot_bound_metric = matches!(
            metric,
            ObjectiveMetric::AcceptedOutputMassMg
                | ObjectiveMetric::FibreLengthQualifiedOutputPerInputPpm
                | ObjectiveMetric::ResidualTrimMassMg
                | ObjectiveMetric::DamageAreaMm2
        );
        if lot_bound_metric
            && (observation.evidence.lot_id != candidate.output_lot_id
                || observation.evidence.process_trial_evidence_id
                    != candidate.process_trial_evidence_id)
        {
            add(
                findings,
                "OBJECTIVE_LOT_TRIAL_BINDING_MISMATCH",
                format!(
                    "objective {:?} must bind the exact candidate output lot and producing trial",
                    metric
                ),
            );
        }
        if observation.evidence.scenario_id != candidate.scenario_id {
            add(
                findings,
                "OBJECTIVE_SCENARIO_BINDING_MISMATCH",
                format!("objective {:?} must bind the exact candidate scenario", metric),
            );
        }
        if !metric.required_basis().contains(&observation.evidence.basis) {
            add(
                findings,
                "OBJECTIVE_EVIDENCE_BASIS_MISMATCH",
                format!(
                    "objective {:?} cannot be promoted from evidence basis {:?}",
                    metric, observation.evidence.basis
                ),
            );
        }
        if metric == ObjectiveMetric::FibreLengthQualifiedOutputPerInputPpm
            && observation.value > 1_000_000
        {
            add(
                findings,
                "OBJECTIVE_VALUE_OUT_OF_RANGE",
                "fibre-length-qualified output fraction must not exceed 1,000,000 ppm",
            );
        }
    }
    observations
        .into_iter()
        .map(|(metric, observation)| (metric, observation.value))
        .collect()
}

fn assess_candidate(candidate: &Candidate, reference: &Candidate) -> CandidateAssessment {
    let mut findings = candidate_metadata_findings(candidate, reference);
    let mut hard_reject = !findings.is_empty();
    let mut insufficient = false;

    if !nonempty(&candidate.candidate_id)
        || !nonempty(&candidate.benchmark_suite_id)
        || !nonempty(&candidate.benchmark_suite_version)
        || !nonempty(&candidate.objective_schema_version)
        || !nonempty(&candidate.hard_gate_policy_version)
        || !nonempty(&candidate.problem_name)
        || !nonempty(&candidate.scenario_id)
        || !nonempty(&candidate.input_lot_id)
        || candidate.input_mass_mg == 0
        || !nonempty(&candidate.output_lot_id)
        || !nonempty(&candidate.process_trial_evidence_id)
        || !nonempty(&candidate.receiving_specification_id)
        || !nonempty(&candidate.receiving_specification_version)
        || !nonempty(&candidate.utility_tick_unit)
    {
        add(
            &mut findings,
            "CANDIDATE_IDENTITY_INVALID",
            "candidate, lot, trial, benchmark and receiver identities plus positive input mass are required",
        );
        hard_reject = true;
    }

    if !candidate.schedule_validation.valid
        || candidate.schedule_validation.problem_name != candidate.problem_name
        || candidate.schedule_validation.makespan_ticks.is_none()
    {
        add(
            &mut findings,
            "SCHEDULE_GATE_FAILED",
            "candidate lacks a valid independent schedule report for the declared problem",
        );
        hard_reject = true;
    }

    if candidate.route_receipt_ids.is_empty()
        || candidate.route_receipt_ids.iter().any(|value| !nonempty(value))
    {
        add(
            &mut findings,
            "ROUTE_RECEIPT_MISSING",
            "candidate must bind one or more exact route receipt IDs",
        );
        insufficient = true;
    }
    if !candidate.route_findings.is_empty() {
        add(
            &mut findings,
            "ROUTE_GATE_FAILED",
            format!("upstream route validator returned {} finding(s)", candidate.route_findings.len()),
        );
        hard_reject = true;
    }

    match &candidate.utility_validation {
        None => {
            add(
                &mut findings,
                "UTILITY_EVIDENCE_MISSING",
                "a candidate without a time-indexed utility report cannot enter the measured Pareto set",
            );
            insufficient = true;
        }
        Some(report) => {
            if !report.valid
                || report.problem_name != candidate.problem_name
                || report.tick_unit != candidate.utility_tick_unit
            {
                add(
                    &mut findings,
                    "UTILITY_GATE_FAILED",
                    "utility report is invalid or does not match the candidate problem/tick unit",
                );
                hard_reject = true;
            }
        }
    }

    if candidate.requires_disruption_replay {
        match &candidate.resilience_validation {
            None => {
                add(
                    &mut findings,
                    "RESILIENCE_EVIDENCE_MISSING",
                    "disruption tournament requires an exact scenario replay report",
                );
                insufficient = true;
            }
            Some(report) => {
                if report.scenario_id != candidate.scenario_id
                    || !report.baseline_feasible
                    || !report.replan_feasible
                    || !report.schedule_validation.valid
                    || report.utility_validation.as_ref().is_none_or(|utility| !utility.valid)
                {
                    add(
                        &mut findings,
                        "RESILIENCE_GATE_FAILED",
                        "resilience report does not show a feasible baseline and feasible schedule/utility replan for this exact scenario",
                    );
                    hard_reject = true;
                }
            }
        }
    }

    validate_receiver(candidate, &mut findings);
    if candidate.receiver_acceptance.outcome == ReceiverOutcome::Rejected {
        hard_reject = true;
    } else if candidate.receiver_acceptance.outcome != ReceiverOutcome::Accepted {
        insufficient = true;
    }
    if findings
        .iter()
        .any(|finding| finding.code == "RECEIVER_BINDING_INVALID" || finding.code == "RECEIVER_PROVENANCE_INVALID")
    {
        hard_reject = true;
    }

    let objective_values = validate_objectives(candidate, &mut findings);
    if findings.iter().any(|finding| matches!(
        finding.code,
        "OBJECTIVE_DUPLICATE"
            | "OBJECTIVE_UNIT_MISMATCH"
            | "OBJECTIVE_EVIDENCE_INVALID"
            | "OBJECTIVE_EVIDENCE_BASIS_MISMATCH"
            | "OBJECTIVE_VALUE_OUT_OF_RANGE"
            | "OBJECTIVE_LOT_TRIAL_BINDING_MISMATCH"
            | "OBJECTIVE_SCENARIO_BINDING_MISMATCH"
    )) {
        hard_reject = true;
    }
    if findings.iter().any(|finding| finding.code == "OBJECTIVE_MISSING") {
        insufficient = true;
    }

    if let Some(value) = objective_values.get(&ObjectiveMetric::AcceptedOutputMassMg) {
        let evidence_matches = candidate
            .objectives
            .iter()
            .find(|item| item.metric == ObjectiveMetric::AcceptedOutputMassMg)
            .is_some_and(|item| {
                item.evidence.evidence_id == candidate.receiver_acceptance.evidence_id
                    && item.evidence.sha256 == candidate.receiver_acceptance.evidence_sha256
            });
        if *value != candidate.receiver_acceptance.accepted_mass_mg || !evidence_matches {
            add(
                &mut findings,
                "ACCEPTED_MASS_METRIC_MISMATCH",
                "accepted-output objective must equal and reference the exact independent receiver-accepted mass evidence",
            );
            hard_reject = true;
        }
    }
    if let Some(value) = objective_values.get(&ObjectiveMetric::MakespanTicks)
        && candidate.schedule_validation.makespan_ticks != Some(*value)
    {
        add(
            &mut findings,
            "MAKESPAN_METRIC_MISMATCH",
            "makespan objective must equal the independent schedule report",
        );
        hard_reject = true;
    }
    if candidate.requires_disruption_replay
        && let Some(report) = &candidate.resilience_validation
    {
        for (metric, expected) in [
            (ObjectiveMetric::MakespanTicks, report.replan_makespan_ticks),
            (ObjectiveMetric::TotalTardinessTicks, Some(report.total_tardiness_ticks)),
            (
                ObjectiveMetric::PlanChurnOperationCount,
                Some(report.churn.changed_operation_count as u64),
            ),
            (
                ObjectiveMetric::RecoveryHorizonTicks,
                report.post_event_completion_horizon_ticks,
            ),
        ] {
            if let Some(value) = objective_values.get(&metric) {
                if expected != Some(*value) {
                    add(
                        &mut findings,
                        "RESILIENCE_METRIC_MISMATCH",
                        format!("objective {:?} does not match the exact resilience report", metric),
                    );
                    hard_reject = true;
                }
            }
        }
    }

    let eligibility = if hard_reject {
        CandidateEligibility::Ineligible
    } else if insufficient {
        CandidateEligibility::InsufficientEvidence
    } else {
        CandidateEligibility::Eligible
    };

    CandidateAssessment {
        candidate_id: candidate.candidate_id.clone(),
        eligibility,
        findings,
        objective_values,
        pareto_front_index: None,
        dominated_by: Vec::new(),
        dominates: Vec::new(),
    }
}

fn dominates(left: &BTreeMap<ObjectiveMetric, u64>, right: &BTreeMap<ObjectiveMetric, u64>) -> bool {
    let mut strictly_better = false;
    for metric in ObjectiveMetric::ALL {
        let Some(left_value) = left.get(&metric) else {
            return false;
        };
        let Some(right_value) = right.get(&metric) else {
            return false;
        };
        match metric.direction() {
            Direction::Maximize => {
                if left_value < right_value {
                    return false;
                }
                strictly_better |= left_value > right_value;
            }
            Direction::Minimize => {
                if left_value > right_value {
                    return false;
                }
                strictly_better |= left_value < right_value;
            }
        }
    }
    strictly_better
}

/// Evaluate candidates with common benchmark identity, hard-gate eligibility
/// and deterministic non-dominated sorting. Ineligible and insufficient
/// candidates are reported but never appear on a Pareto front.
pub fn evaluate_tournament(candidates: &[Candidate]) -> TournamentReport {
    let reference = candidates.first();
    let suite_id = reference.map(|value| value.benchmark_suite_id.clone()).unwrap_or_default();
    let suite_version = reference
        .map(|value| value.benchmark_suite_version.clone())
        .unwrap_or_default();
    let scenario_id = reference.map(|value| value.scenario_id.clone()).unwrap_or_default();
    let input_lot_id = reference.map(|value| value.input_lot_id.clone()).unwrap_or_default();
    let receiving_specification_id = reference
        .map(|value| value.receiving_specification_id.clone())
        .unwrap_or_default();
    let objective_schema_version = reference
        .map(|value| value.objective_schema_version.clone())
        .unwrap_or_default();

    let mut tournament_findings = Vec::new();
    if candidates.is_empty() {
        tournament_findings.push(TournamentFinding {
            code: "TOURNAMENT_EMPTY",
            message: "at least one candidate is required".to_owned(),
        });
    }

    let mut seen_ids = BTreeSet::new();
    for candidate in candidates {
        if !seen_ids.insert(candidate.candidate_id.as_str()) {
            tournament_findings.push(TournamentFinding {
                code: "CANDIDATE_ID_DUPLICATE",
                message: format!("candidate ID {:?} occurs more than once", candidate.candidate_id),
            });
        }
    }

    if let Some(reference) = reference {
        for candidate in candidates.iter().skip(1) {
            let mismatch = candidate_metadata_findings(candidate, reference);
            if !mismatch.is_empty() {
                tournament_findings.push(TournamentFinding {
                    code: "TOURNAMENT_IDENTITY_MISMATCH",
                    message: format!(
                        "candidate {:?} does not share the exact comparator identity: {}",
                        candidate.candidate_id,
                        mismatch.iter().map(|item| item.message.as_str()).collect::<Vec<_>>().join("; ")
                    ),
                });
            }
        }
    }

    let mut assessments: Vec<CandidateAssessment> = candidates
        .iter()
        .map(|candidate| {
            reference.map_or_else(
                || assess_candidate(candidate, candidate),
                |reference| assess_candidate(candidate, reference),
            )
        })
        .collect();
    assessments.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    if !tournament_findings.is_empty() {
        return TournamentReport {
            valid: false,
            benchmark_suite_id: suite_id,
            benchmark_suite_version: suite_version,
            scenario_id,
            input_lot_id,
            receiving_specification_id,
            objective_schema_version,
            assessments,
            pareto_fronts: Vec::new(),
            dominance_relations: Vec::new(),
            findings: tournament_findings,
        };
    }

    let eligible: Vec<usize> = assessments
        .iter()
        .enumerate()
        .filter_map(|(index, item)| (item.eligibility == CandidateEligibility::Eligible).then_some(index))
        .collect();

    let mut dominates_map = BTreeMap::<usize, Vec<usize>>::new();
    let mut dominated_by_count = BTreeMap::<usize, usize>::new();
    let mut dominance_relations = Vec::new();
    for &left in &eligible {
        for &right in &eligible {
            if left == right {
                continue;
            }
            if dominates(
                &assessments[left].objective_values,
                &assessments[right].objective_values,
            ) {
                dominates_map.entry(left).or_default().push(right);
                *dominated_by_count.entry(right).or_default() += 1;
                dominance_relations.push(DominanceRelation {
                    dominates: assessments[left].candidate_id.clone(),
                    dominated: assessments[right].candidate_id.clone(),
                });
            }
        }
    }

    dominance_relations.sort_by(|left, right| {
        (&left.dominates, &left.dominated).cmp(&(&right.dominates, &right.dominated))
    });

    let mut current_front: Vec<usize> = eligible
        .iter()
        .copied()
        .filter(|index| dominated_by_count.get(index).copied().unwrap_or(0) == 0)
        .collect();
    let mut fronts = Vec::<Vec<String>>::new();
    let mut front_index = 0_usize;

    while !current_front.is_empty() {
        current_front.sort_by(|left, right| {
            assessments[*left].candidate_id.cmp(&assessments[*right].candidate_id)
        });
        let current_set: BTreeSet<usize> = current_front.iter().copied().collect();
        let mut next_front = Vec::new();

        for &index in &current_front {
            assessments[index].pareto_front_index = Some(front_index);
            for dominated in dominates_map.get(&index).into_iter().flatten() {
                if let Some(count) = dominated_by_count.get_mut(dominated) {
                    *count = count.saturating_sub(1);
                    if *count == 0 && !current_set.contains(dominated) {
                        next_front.push(*dominated);
                    }
                }
            }
        }

        fronts.push(
            current_front
                .iter()
                .map(|index| assessments[*index].candidate_id.clone())
                .collect(),
        );
        current_front = next_front;
        front_index += 1;
    }

    for relation in &dominance_relations {
        if let Some(item) = assessments.iter_mut().find(|item| item.candidate_id == relation.dominates) {
            item.dominates.push(relation.dominated.clone());
        }
        if let Some(item) = assessments.iter_mut().find(|item| item.candidate_id == relation.dominated) {
            item.dominated_by.push(relation.dominates.clone());
        }
    }
    for assessment in &mut assessments {
        assessment.dominates.sort();
        assessment.dominated_by.sort();
    }

    TournamentReport {
        valid: true,
        benchmark_suite_id: suite_id,
        benchmark_suite_version: suite_version,
        scenario_id,
        input_lot_id,
        receiving_specification_id,
        objective_schema_version,
        assessments,
        pareto_fronts: fronts,
        dominance_relations,
        findings: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resilience::{PlanChurn, ResilienceReport};
    use crate::utilities::{UtilityFinding, UtilityReport};
    use crate::Finding;

    fn evidence(
        id: &str,
        ch: char,
        basis: MetricEvidenceBasis,
        lot_id: &str,
        process_trial_evidence_id: &str,
        scenario_id: &str,
    ) -> MetricEvidence {
        MetricEvidence {
            evidence_id: id.to_owned(),
            sha256: ch.to_string().repeat(64),
            method_ref: "method-v1".to_owned(),
            basis,
            lot_id: lot_id.to_owned(),
            process_trial_evidence_id: process_trial_evidence_id.to_owned(),
            scenario_id: scenario_id.to_owned(),
        }
    }

    fn objective(
        metric: ObjectiveMetric,
        value: u64,
        candidate_id: &str,
        scenario_id: &str,
    ) -> ObjectiveObservation {
        let basis = metric.required_basis()[0];
        let lot_id = format!("output-{candidate_id}");
        let trial_id = format!("trial-{candidate_id}");
        let evidence_id = if metric == ObjectiveMetric::AcceptedOutputMassMg {
            format!("receiver-{candidate_id}")
        } else {
            format!("evidence-{metric:?}-{candidate_id}")
        };
        let evidence_digest = if metric == ObjectiveMetric::AcceptedOutputMassMg {
            'b'
        } else {
            'a'
        };
        ObjectiveObservation {
            metric,
            value,
            unit: metric.unit().to_owned(),
            evidence: evidence(
                &evidence_id,
                evidence_digest,
                basis,
                &lot_id,
                &trial_id,
                scenario_id,
            ),
        }
    }

    fn candidate(id: &str, output_mass: u64, energy: u64, makespan: u64) -> Candidate {
        let mut objectives = vec![
            objective(ObjectiveMetric::AcceptedOutputMassMg, output_mass, id, "baseline"),
            objective(ObjectiveMetric::FibreLengthQualifiedOutputPerInputPpm, 700_000, id, "baseline"),
            objective(ObjectiveMetric::ResidualTrimMassMg, 500, id, "baseline"),
            objective(ObjectiveMetric::DamageAreaMm2, 10, id, "baseline"),
            objective(ObjectiveMetric::ElectricityWhPerAcceptedKg, energy, id, "baseline"),
            objective(ObjectiveMetric::SteamGramsPerAcceptedKg, 12, id, "baseline"),
            objective(ObjectiveMetric::WaterMlPerAcceptedKg, 100, id, "baseline"),
            objective(ObjectiveMetric::WastewaterMlPerAcceptedKg, 80, id, "baseline"),
            objective(ObjectiveMetric::OperatorMinutesPerAcceptedKg, 30, id, "baseline"),
            objective(ObjectiveMetric::MakespanTicks, makespan, id, "baseline"),
            objective(ObjectiveMetric::TotalTardinessTicks, 0, id, "baseline"),
            objective(ObjectiveMetric::PlanChurnOperationCount, 0, id, "baseline"),
            objective(ObjectiveMetric::RecoveryHorizonTicks, makespan, id, "baseline"),
        ];

        Candidate {
            candidate_id: id.to_owned(),
            benchmark_suite_id: "textile-benchmark-v1".to_owned(),
            benchmark_suite_version: "1.0".to_owned(),
            objective_schema_version: OBJECTIVE_SCHEMA_VERSION.to_owned(),
            hard_gate_policy_version: "hard-gates-v1".to_owned(),
            problem_name: "matched-lot-problem".to_owned(),
            scenario_id: "baseline".to_owned(),
            requires_disruption_replay: false,
            input_lot_id: "incoming-lot-01".to_owned(),
            input_mass_mg: 10_000_000,
            output_lot_id: format!("output-{id}"),
            process_trial_evidence_id: format!("trial-{id}"),
            receiving_specification_id: "ring-spinning-spec".to_owned(),
            receiving_specification_version: "2.0".to_owned(),
            utility_tick_unit: "minute".to_owned(),
            route_receipt_ids: vec![format!("route-{id}")],
            schedule_validation: ValidationReport {
                valid: true,
                problem_name: "matched-lot-problem".to_owned(),
                expected_operations: 8,
                supplied_assignments: 8,
                makespan_ticks: Some(makespan),
                findings: Vec::new(),
            },
            route_findings: Vec::new(),
            utility_validation: Some(UtilityReport {
                valid: true,
                problem_name: "matched-lot-problem".to_owned(),
                tick_unit: "minute".to_owned(),
                operation_consumers: 8,
                setup_consumers: 0,
                evidence_basis_counts: BTreeMap::new(),
                findings: Vec::<UtilityFinding>::new(),
            }),
            resilience_validation: None,
            receiver_acceptance: ReceiverAcceptance {
                outcome: ReceiverOutcome::Accepted,
                output_lot_id: format!("output-{id}"),
                process_trial_evidence_id: format!("trial-{id}"),
                specification_id: "ring-spinning-spec".to_owned(),
                specification_version: "2.0".to_owned(),
                accepted_mass_mg: output_mass,
                issuer_id: "receiver-lab".to_owned(),
                verifier_id: "independent-auditor".to_owned(),
                evidence_id: format!("receiver-{id}"),
                evidence_sha256: "b".repeat(64),
            },
            objectives: {
                objectives.sort_by_key(|item| item.metric);
                objectives
            },
        }
    }

    #[test]
    fn incomparable_eligible_candidates_share_first_pareto_front() {
        let a = candidate("high-output", 8_000_000, 150, 15);
        let b = candidate("low-energy", 7_000_000, 90, 17);
        let report = evaluate_tournament(&[a, b]);
        assert!(report.valid, "{:?}", report.findings);
        assert_eq!(report.pareto_fronts, vec![vec!["high-output".to_owned(), "low-energy".to_owned()]]);
        assert!(report.dominance_relations.is_empty());
    }

    #[test]
    fn candidate_dominated_on_every_objective_is_ranked_on_later_front() {
        let mut dominant = candidate("dominant", 9_000_000, 80, 12);
        let dominated = candidate("dominated", 8_000_000, 120, 15);
        for objective in &mut dominant.objectives {
            match objective.metric {
                ObjectiveMetric::FibreLengthQualifiedOutputPerInputPpm => objective.value = 800_000,
                _ => {}
            }
        }
        let report = evaluate_tournament(&[dominated, dominant]);
        assert!(report.valid, "{:?}", report.findings);
        assert_eq!(report.pareto_fronts[0], vec!["dominant".to_owned()]);
        assert_eq!(report.pareto_fronts[1], vec!["dominated".to_owned()]);
        assert_eq!(report.dominance_relations.len(), 1);
        assert_eq!(report.assessments.iter().find(|x| x.candidate_id == "dominated").unwrap().dominated_by, vec!["dominant".to_owned()]);
    }

    #[test]
    fn infeasible_schedule_is_excluded_before_pareto_ranking() {
        let mut invalid_candidate = candidate("invalid-schedule", 9_900_000, 1, 1);
        invalid_candidate.schedule_validation.valid = false;
        let valid = candidate("valid", 8_000_000, 100, 15);
        let report = evaluate_tournament(&[invalid_candidate, valid]);
        assert!(report.valid);
        assert_eq!(report.pareto_fronts, vec![vec!["valid".to_owned()]]);
        assert_eq!(
            report.assessments.iter().find(|x| x.candidate_id == "invalid-schedule").unwrap().eligibility,
            CandidateEligibility::Ineligible
        );
    }

    #[test]
    fn missing_utility_report_is_insufficient_not_zero_consumption() {
        let mut missing = candidate("missing-utility", 9_000_000, 0, 10);
        missing.utility_validation = None;
        let report = evaluate_tournament(&[missing]);
        assert!(report.valid);
        assert!(report.pareto_fronts.is_empty());
        assert_eq!(report.assessments[0].eligibility, CandidateEligibility::InsufficientEvidence);
    }

    #[test]
    fn failed_utility_gate_is_hard_ineligible() {
        let mut invalid = candidate("utility-failure", 9_000_000, 0, 10);
        invalid.utility_validation.as_mut().unwrap().valid = false;
        let report = evaluate_tournament(&[invalid]);
        assert!(report.pareto_fronts.is_empty());
        assert_eq!(report.assessments[0].eligibility, CandidateEligibility::Ineligible);
    }

    #[test]
    fn conditional_receiver_acceptance_is_not_eligible() {
        let mut conditional = candidate("conditional", 9_000_000, 80, 10);
        conditional.receiver_acceptance.outcome = ReceiverOutcome::Conditional;
        let report = evaluate_tournament(&[conditional]);
        assert!(report.pareto_fronts.is_empty());
        assert_eq!(report.assessments[0].eligibility, CandidateEligibility::InsufficientEvidence);
    }

    #[test]
    fn receiver_acceptance_cannot_be_borrowed_from_another_output_lot() {
        let mut borrowed = candidate("borrowed", 9_000_000, 80, 10);
        borrowed.receiver_acceptance.output_lot_id = "other-lot".to_owned();
        let report = evaluate_tournament(&[borrowed]);
        assert!(report.pareto_fronts.is_empty());
        assert_eq!(report.assessments[0].eligibility, CandidateEligibility::Ineligible);
    }

    #[test]
    fn estimated_utility_cannot_be_promoted_as_a_calibrated_meter_objective() {
        let mut estimated = candidate("estimated", 9_000_000, 80, 10);
        estimated
            .objectives
            .iter_mut()
            .find(|item| item.metric == ObjectiveMetric::ElectricityWhPerAcceptedKg)
            .unwrap()
            .evidence
            .basis = MetricEvidenceBasis::EngineeringEstimate;
        let report = evaluate_tournament(&[estimated]);
        assert!(report.pareto_fronts.is_empty());
        assert_eq!(report.assessments[0].eligibility, CandidateEligibility::Ineligible);
    }

    #[test]
    fn mismatched_suite_lot_spec_or_scenario_invalidates_the_comparator() {
        let a = candidate("a", 9_000_000, 100, 10);
        let mut b = candidate("b", 8_000_000, 90, 11);
        b.input_lot_id = "different-lot".to_owned();
        let report = evaluate_tournament(&[a, b]);
        assert!(!report.valid);
        assert!(report.pareto_fronts.is_empty());
        assert_eq!(report.findings[0].code, "TOURNAMENT_IDENTITY_MISMATCH");
    }

    #[test]
    fn candidate_order_does_not_change_front_membership() {
        let a = candidate("a", 9_000_000, 100, 10);
        let b = candidate("b", 8_000_000, 90, 11);
        let c = candidate("c", 7_000_000, 110, 13);
        let forward = evaluate_tournament(&[a.clone(), b.clone(), c.clone()]);
        let reverse = evaluate_tournament(&[c, b, a]);
        assert_eq!(forward.pareto_fronts, reverse.pareto_fronts);
        assert_eq!(forward.dominance_relations, reverse.dominance_relations);
    }

    #[test]
    fn equal_objective_vectors_tie_without_invented_preference() {
        let a = candidate("a", 8_000_000, 100, 12);
        let mut b = candidate("b", 8_000_000, 100, 12);
        b.output_lot_id = "output-b".to_owned();
        b.receiver_acceptance.output_lot_id = "output-b".to_owned();
        b.process_trial_evidence_id = "trial-b".to_owned();
        b.receiver_acceptance.process_trial_evidence_id = "trial-b".to_owned();
        b.receiver_acceptance.evidence_id = "receiver-b".to_owned();
        let report = evaluate_tournament(&[b, a]);
        assert!(report.valid);
        assert_eq!(report.pareto_fronts, vec![vec!["a".to_owned(), "b".to_owned()]]);
        assert!(report.dominance_relations.is_empty());
    }

    #[test]
    fn disruption_tournament_requires_feasible_exact_scenario_replay() {
        let mut candidate = candidate("static-only", 8_000_000, 100, 12);
        candidate.requires_disruption_replay = true;
        candidate.scenario_id = "machine-outage-01".to_owned();
        let report = evaluate_tournament(&[candidate]);
        assert!(report.pareto_fronts.is_empty());
        assert_eq!(report.assessments[0].eligibility, CandidateEligibility::InsufficientEvidence);
    }

    #[test]
    fn resilience_metrics_must_match_the_exact_replay() {
        let mut candidate = candidate("replay", 8_000_000, 100, 12);
        candidate.requires_disruption_replay = true;
        candidate.scenario_id = "machine-outage-01".to_owned();
        candidate.resilience_validation = Some(ResilienceReport {
            scenario_id: candidate.scenario_id.clone(),
            observed_at_tick: 5,
            baseline_feasible: true,
            replan_feasible: true,
            baseline_makespan_ticks: Some(15),
            replan_makespan_ticks: Some(12),
            post_event_completion_horizon_ticks: Some(7),
            total_tardiness_ticks: 0,
            maximum_tardiness_ticks: 0,
            churn: PlanChurn {
                changed_operation_count: 1,
                ..PlanChurn::default()
            },
            schedule_validation: candidate.schedule_validation.clone(),
            utility_validation: candidate.utility_validation.clone(),
        });
        let report = evaluate_tournament(&[candidate]);
        assert!(report.pareto_fronts.is_empty());
        assert!(report.assessments[0].findings.iter().any(|item| item.code == "RESILIENCE_METRIC_MISMATCH"));
    }
}
