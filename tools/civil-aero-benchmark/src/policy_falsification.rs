//! Independent post-admission feasibility falsification for matched-budget design policies.
//!
//! The policy benchmark scorecard records caller-bound admission and a caller-supplied feasibility
//! boolean. This module adds a separate adjudication frame, preserving confirmed, falsified,
//! inconclusive and unassessed candidates without silently dropping unresolved cases.
//!
//! Rates are bounded over the complete admitted-candidate frame:
//!   lower = falsified / admitted
//!   upper = (falsified + inconclusive + unassessed) / admitted
//! A resolved-only rate is reported separately and is not an unbiased population estimate unless
//! the adjudication design supports that interpretation. Verifier identities are caller-declared;
//! this module does not authenticate trust anchors, signatures or independence.

use std::collections::{BTreeMap, BTreeSet};

use crate::budget::{LEDGER_INPUT_HEADER, LedgerOperation};
use crate::policy_benchmark::{
    CandidateAdmissionStatus, CandidateObservation, PolicyBenchmarkRow, PolicyBenchmarkTask,
    evaluate_policy_benchmark,
};
use crate::integrity::sha256_hex;
use crate::policy_benchmark::{PolicyBenchmarkRow, verify_policy_benchmark_source_receipt};

pub const POLICY_FEASIBILITY_AUDIT_SCHEMA: &str = "aero-policy-feasibility-audit/0.1.0";
pub const POLICY_FEASIBILITY_AUDIT_EVIDENCE_LEVEL: &str = "exact_admitted_candidate_frame_with_separate_caller_declared_independent_adjudication_and_worst_case_unresolved_bounds_not_authenticated_verifier_independence_or_physical_certification";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeasibilityAdjudicationMethod {
    IndependentHigherFidelityAnalysis,
    IndependentConstraintReconstruction,
    PhysicalTest,
    IndependentExpertReview,
}

impl FeasibilityAdjudicationMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IndependentHigherFidelityAnalysis => "independent_higher_fidelity_analysis",
            Self::IndependentConstraintReconstruction => "independent_constraint_reconstruction",
            Self::PhysicalTest => "physical_test",
            Self::IndependentExpertReview => "independent_expert_review",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeasibilityAdjudicationOutcome {
    ConfirmedFeasible,
    FalsifiedInfeasible,
    Inconclusive { reason: String },
}

impl FeasibilityAdjudicationOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ConfirmedFeasible => "confirmed_feasible",
            Self::FalsifiedInfeasible => "falsified_infeasible",
            Self::Inconclusive { .. } => "inconclusive",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedCandidateFrameEntry {
    pub task_id: String,
    pub replicate_id: String,
    pub policy_id: String,
    pub candidate_id: String,
    pub operation_id: String,
    pub output_sha256: String,
    pub admission_receipt_sha256: String,
    pub admission_verifier_id: String,
    pub admission_verifier_sha256: String,
    pub scope_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndependentFeasibilityAdjudication {
    pub candidate_id: String,
    pub output_sha256: String,
    pub scope_sha256: String,
    pub adjudicator_id: String,
    pub adjudicator_sha256: String,
    pub adjudication_receipt_sha256: String,
    pub method: FeasibilityAdjudicationMethod,
    pub outcome: FeasibilityAdjudicationOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeasibilityAdjudicationStatus {
    ConfirmedFeasible,
    FalsifiedInfeasible,
    Inconclusive,
    Unassessed,
}

impl FeasibilityAdjudicationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfirmedFeasible => "confirmed_feasible",
            Self::FalsifiedInfeasible => "falsified_infeasible",
            Self::Inconclusive => "inconclusive",
            Self::Unassessed => "unassessed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyFeasibilityCandidateResult {
    pub task_id: String,
    pub replicate_id: String,
    pub operation_id: String,
    pub candidate_id: String,
    pub output_sha256: String,
    pub scope_sha256: String,
    pub admission_receipt_sha256: String,
    pub admission_verifier_id: String,
    pub admission_verifier_sha256: String,
    pub status: FeasibilityAdjudicationStatus,
    pub method: String,
    pub adjudicator_id: String,
    pub adjudicator_sha256: String,
    pub adjudication_receipt_sha256: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdjudicationSelectionDesign {
    FullCensus {
        sampling_frame_sha256: String,
    },
    PreRegisteredProbabilitySample {
        sampling_frame_sha256: String,
        selection_plan_sha256: String,
        selection_receipt_sha256: String,
    },
    TargetedChallengeSet {
        sampling_frame_sha256: String,
        selection_plan_sha256: String,
        selection_receipt_sha256: String,
    },
}

impl AdjudicationSelectionDesign {
    fn sampling_frame_sha256(&self) -> &str {
        match self {
            Self::FullCensus {
                sampling_frame_sha256,
            }
            | Self::PreRegisteredProbabilitySample {
                sampling_frame_sha256,
                ..
            }
            | Self::TargetedChallengeSet {
                sampling_frame_sha256,
                ..
            } => sampling_frame_sha256,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::FullCensus { .. } => "full_census",
            Self::PreRegisteredProbabilitySample { .. } => "pre_registered_probability_sample",
            Self::TargetedChallengeSet { .. } => "targeted_challenge_set",
        }
    }

    fn selection_references(&self) -> (Option<&str>, Option<&str>) {
        match self {
            Self::FullCensus { .. } => (None, None),
            Self::PreRegisteredProbabilitySample {
                selection_plan_sha256,
                selection_receipt_sha256,
                ..
            }
            | Self::TargetedChallengeSet {
                selection_plan_sha256,
                selection_receipt_sha256,
                ..
            } => (Some(selection_plan_sha256), Some(selection_receipt_sha256)),
        }
    }

    fn validate(&self) -> Result<(), String> {
        if !is_sha256(self.sampling_frame_sha256()) {
            return Err(
                "adjudication selection design requires a valid sampling-frame digest".to_owned(),
            );
        }
        match self {
            Self::FullCensus { .. } => {}
            Self::PreRegisteredProbabilitySample {
                selection_plan_sha256,
                selection_receipt_sha256,
                ..
            }
            | Self::TargetedChallengeSet {
                selection_plan_sha256,
                selection_receipt_sha256,
                ..
            } => {
                if !is_sha256(selection_plan_sha256) || !is_sha256(selection_receipt_sha256) {
                    return Err("sampled adjudication requires valid selection-plan and selection-receipt digests".to_owned());
                }
            }
        }
        Ok(())
    }
}

pub struct PolicyFeasibilityAuditRequest<'a> {
    pub policy_id: &'a str,
    pub frame_id: &'a str,
    pub design: &'a AdjudicationSelectionDesign,
    pub admitted_candidates: &'a [AdmittedCandidateFrameEntry],
    pub adjudications: &'a [IndependentFeasibilityAdjudication],
    pub source_scorecard_row: &'a PolicyBenchmarkRow,
    pub source_operation_ledger: &'a [LedgerOperation],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyFeasibilityAuditReport {
    pub schema: String,
    pub frame_id: String,
    pub policy_id: String,
    pub source_task_id: String,
    pub source_replicate_id: String,
    pub source_candidate_frame_sha256: String,
    pub source_operation_ledger_sha256: String,
    pub source_operation_count: usize,
    pub source_admission_frame_receipt_sha256: String,
    pub selection_design: String,
    pub selection_design_sha256: String,
    pub selection_plan_sha256: Option<String>,
    pub selection_receipt_sha256: Option<String>,
    pub sampling_frame_sha256: String,
    pub candidate_frame_sha256: String,
    pub adjudication_manifest_sha256: String,
    pub admitted_candidate_count: usize,
    pub adjudicated_candidate_count: usize,
    pub confirmed_feasible_count: usize,
    pub falsified_infeasible_count: usize,
    pub inconclusive_count: usize,
    pub unassessed_count: usize,
    pub resolved_count: usize,
    /// One auditable result per admitted candidate, including explicit unassessed entries.
    pub candidate_results: Vec<PolicyFeasibilityCandidateResult>,
    /// Exact lower bound over all admitted candidates; unresolved candidates count as not yet falsified.
    pub false_feasible_rate_lower_bound: Option<f64>,
    /// Exact worst-case upper bound over all admitted candidates; every unresolved candidate may be false-feasible.
    pub false_feasible_rate_upper_bound: Option<f64>,
    /// False-feasible fraction among resolved adjudications only; potentially selection-biased.
    pub false_feasible_rate_among_resolved: Option<f64>,
    pub status: String,
    pub evidence_level: &'static str,
    pub report_sha256: String,
}

pub struct PolicyFeasibilityAuditInputs<'a> {
    pub policy_id: &'a str,
    pub frame_id: &'a str,
    pub design: &'a AdjudicationSelectionDesign,
    pub admitted_candidates: &'a [AdmittedCandidateFrameEntry],
    pub adjudications: &'a [IndependentFeasibilityAdjudication],
    /// Exact upstream scorecard row returned by evaluate_policy_benchmark.
    pub source_scorecard_row: &'a PolicyBenchmarkRow,
    /// Complete policy/task/replicate operation slice used to create that scorecard row.
    pub source_operation_ledger: &'a [LedgerOperation],
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value == value.to_ascii_lowercase()
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn append_optional_f64(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        None => bytes.push(0),
    }
}

fn candidate_frame_bytes(
    policy_id: &str,
    frame_id: &str,
    candidates: &BTreeMap<String, &AdmittedCandidateFrameEntry>,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_string(&mut bytes, "aero-admitted-candidate-frame/0.1.0");
    append_string(&mut bytes, policy_id);
    append_string(&mut bytes, frame_id);
    append_u64(&mut bytes, candidates.len() as u64);
    for (candidate_id, candidate) in candidates {
        for value in [
            candidate_id.as_str(),
            candidate.task_id.as_str(),
            candidate.replicate_id.as_str(),
            candidate.policy_id.as_str(),
            candidate.operation_id.as_str(),
            candidate.output_sha256.as_str(),
            candidate.admission_receipt_sha256.as_str(),
            candidate.admission_verifier_id.as_str(),
            candidate.admission_verifier_sha256.as_str(),
            candidate.scope_sha256.as_str(),
        ] {
            append_string(&mut bytes, value);
        }
    }
    bytes
}

fn source_admitted_candidate_frame_bytes(
    task_id: &str,
    replicate_id: &str,
    policy_id: &str,
    candidates: &BTreeMap<String, &AdmittedCandidateFrameEntry>,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_string(&mut bytes, "aero-policy-admitted-candidate-frame/0.1.0");
    append_string(&mut bytes, task_id);
    append_string(&mut bytes, replicate_id);
    append_string(&mut bytes, policy_id);
    append_u64(&mut bytes, candidates.len() as u64);
    for (candidate_id, candidate) in candidates {
        for value in [
            candidate_id.as_str(),
            candidate.operation_id.as_str(),
            candidate.output_sha256.as_str(),
            candidate.admission_receipt_sha256.as_str(),
            candidate.admission_verifier_id.as_str(),
            candidate.admission_verifier_sha256.as_str(),
            candidate.scope_sha256.as_str(),
        ] {
            append_string(&mut bytes, value);
        }
    }
    bytes
}

fn selection_design_bytes(design: &AdjudicationSelectionDesign) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_string(&mut bytes, "aero-adjudication-selection-design/0.1.0");
    append_string(&mut bytes, design.as_str());
    append_string(&mut bytes, design.sampling_frame_sha256());
    match design {
        AdjudicationSelectionDesign::FullCensus { .. } => {}
        AdjudicationSelectionDesign::PreRegisteredProbabilitySample {
            selection_plan_sha256,
            selection_receipt_sha256,
            ..
        }
        | AdjudicationSelectionDesign::TargetedChallengeSet {
            selection_plan_sha256,
            selection_receipt_sha256,
            ..
        } => {
            append_string(&mut bytes, selection_plan_sha256);
            append_string(&mut bytes, selection_receipt_sha256);
        }
    }
    bytes
}

fn adjudication_manifest_bytes(
    adjudications: &BTreeMap<String, &IndependentFeasibilityAdjudication>,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    append_string(
        &mut bytes,
        "aero-independent-feasibility-adjudications/0.1.0",
    );
    append_u64(&mut bytes, adjudications.len() as u64);
    for (candidate_id, adjudication) in adjudications {
        for hash in [
            adjudication.output_sha256.as_str(),
            adjudication.scope_sha256.as_str(),
            adjudication.adjudicator_sha256.as_str(),
            adjudication.adjudication_receipt_sha256.as_str(),
        ] {
            if !is_sha256(hash) {
                return Err(format!(
                    "adjudication for candidate {candidate_id} contains a malformed SHA-256 digest"
                ));
            }
        }
        if adjudication.adjudicator_id.trim().is_empty() {
            return Err(format!(
                "adjudication for candidate {candidate_id} needs an adjudicator identity"
            ));
        }
        append_string(&mut bytes, candidate_id);
        append_string(&mut bytes, &adjudication.output_sha256);
        append_string(&mut bytes, &adjudication.scope_sha256);
        append_string(&mut bytes, &adjudication.adjudicator_id);
        append_string(&mut bytes, &adjudication.adjudicator_sha256);
        append_string(&mut bytes, &adjudication.adjudication_receipt_sha256);
        append_string(&mut bytes, adjudication.method.as_str());
        append_string(&mut bytes, adjudication.outcome.as_str());
        if let FeasibilityAdjudicationOutcome::Inconclusive { reason } = &adjudication.outcome {
            if reason.trim().is_empty() {
                return Err(format!(
                    "inconclusive adjudication for candidate {candidate_id} needs a reason"
                ));
            }
            append_string(&mut bytes, reason);
        } else {
            append_string(&mut bytes, "");
        }
    }
    Ok(bytes)
}

fn report_bytes(report: &PolicyFeasibilityAuditReport) -> Vec<u8> {
    let mut bytes = Vec::new();
    let source_operation_count = report.source_operation_count.to_string();
    for value in [
        report.schema.as_str(),
        report.frame_id.as_str(),
        report.policy_id.as_str(),
        report.source_task_id.as_str(),
        report.source_replicate_id.as_str(),
        report.source_candidate_frame_sha256.as_str(),
        report.source_operation_ledger_sha256.as_str(),
        source_operation_count.as_str(),
        report.source_admission_frame_receipt_sha256.as_str(),
        report.selection_design.as_str(),
        report.selection_design_sha256.as_str(),
        report.selection_plan_sha256.as_deref().unwrap_or(""),
        report.selection_receipt_sha256.as_deref().unwrap_or(""),
        report.sampling_frame_sha256.as_str(),
        report.candidate_frame_sha256.as_str(),
        report.adjudication_manifest_sha256.as_str(),
        report.status.as_str(),
        report.evidence_level,
    ] {
        append_string(&mut bytes, value);
    }
    for count in [
        report.admitted_candidate_count,
        report.adjudicated_candidate_count,
        report.confirmed_feasible_count,
        report.falsified_infeasible_count,
        report.inconclusive_count,
        report.unassessed_count,
        report.resolved_count,
    ] {
        append_u64(&mut bytes, count as u64);
    }
    append_u64(&mut bytes, report.candidate_results.len() as u64);
    for result in &report.candidate_results {
        for value in [
            result.task_id.as_str(),
            result.replicate_id.as_str(),
            result.operation_id.as_str(),
            result.candidate_id.as_str(),
            result.output_sha256.as_str(),
            result.scope_sha256.as_str(),
            result.admission_receipt_sha256.as_str(),
            result.admission_verifier_id.as_str(),
            result.admission_verifier_sha256.as_str(),
            result.status.as_str(),
            result.method.as_str(),
            result.adjudicator_id.as_str(),
            result.adjudicator_sha256.as_str(),
            result.adjudication_receipt_sha256.as_str(),
            result.reason.as_str(),
        ] {
            append_string(&mut bytes, value);
        }
    }
    append_optional_f64(&mut bytes, report.false_feasible_rate_lower_bound);
    append_optional_f64(&mut bytes, report.false_feasible_rate_upper_bound);
    append_optional_f64(&mut bytes, report.false_feasible_rate_among_resolved);
    bytes
}

pub fn evaluate_policy_feasibility_audit(
    inputs: &PolicyFeasibilityAuditInputs<'_>,
) -> Result<PolicyFeasibilityAuditReport, String> {
    if inputs.policy_id.trim().is_empty() || inputs.frame_id.trim().is_empty() {
        return Err("policy_id and frame_id must be non-empty".to_owned());
    }
    inputs.design.validate()?;

    let source_row = inputs.source_scorecard_row;
    if source_row.policy_id != inputs.policy_id
        || source_row.replicate_id.trim().is_empty()
        || source_row.task_id.trim().is_empty()
        || source_row.admitted_candidates != inputs.admitted_candidates.len()
    {
        return Err(
            "source policy benchmark row does not match the complete downstream candidate frame"
                .to_owned(),
        );
    }
    verify_policy_benchmark_source_receipt(source_row, inputs.source_operation_ledger)?;

    let mut candidate_map = BTreeMap::<String, &AdmittedCandidateFrameEntry>::new();
    let mut operations = BTreeSet::<(String, String, String, String)>::new();
    for candidate in inputs.admitted_candidates {
        if candidate.task_id != source_row.task_id
            || candidate.replicate_id != source_row.replicate_id
            || candidate.policy_id != source_row.policy_id
        {
            return Err(format!(
                "candidate {} does not belong to the exact source scorecard row",
                candidate.candidate_id
            ));
        }
        if candidate.policy_id != inputs.policy_id {
            return Err(format!(
                "candidate {} belongs to policy {}, not {}",
                candidate.candidate_id, candidate.policy_id, inputs.policy_id
            ));
        }
        for (name, value) in [
            ("task_id", candidate.task_id.as_str()),
            ("replicate_id", candidate.replicate_id.as_str()),
            ("candidate_id", candidate.candidate_id.as_str()),
            ("operation_id", candidate.operation_id.as_str()),
            (
                "admission_verifier_id",
                candidate.admission_verifier_id.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(format!("candidate {name} must be non-empty"));
            }
        }
        for (name, value) in [
            ("output_sha256", candidate.output_sha256.as_str()),
            (
                "admission_receipt_sha256",
                candidate.admission_receipt_sha256.as_str(),
            ),
            (
                "admission_verifier_sha256",
                candidate.admission_verifier_sha256.as_str(),
            ),
            ("scope_sha256", candidate.scope_sha256.as_str()),
        ] {
            if !is_sha256(value) {
                return Err(format!(
                    "candidate {} has malformed {name}",
                    candidate.candidate_id
                ));
            }
        }
        if candidate_map
            .insert(candidate.candidate_id.clone(), candidate)
            .is_some()
        {
            return Err(format!(
                "duplicate candidate ID in admitted-candidate frame: {}",
                candidate.candidate_id
            ));
        }
        let operation_key = (
            candidate.task_id.clone(),
            candidate.replicate_id.clone(),
            candidate.policy_id.clone(),
            candidate.operation_id.clone(),
        );
        if !operations.insert(operation_key.clone()) {
            return Err(format!(
                "multiple candidates bind to high-fidelity operation {operation_key:?}"
            ));
        }
    }

    let source_candidate_frame_sha256 = sha256_hex(&source_admitted_candidate_frame_bytes(
        &source_row.task_id,
        &source_row.replicate_id,
        &source_row.policy_id,
        &candidate_map,
    ))?;
    if source_candidate_frame_sha256 != source_row.admitted_candidate_frame_sha256 {
        return Err(
            "downstream admitted-candidate frame differs from the upstream policy scorecard"
                .to_owned(),
        );
    }
    let candidate_frame_sha256 = sha256_hex(&candidate_frame_bytes(
        inputs.policy_id,
        inputs.frame_id,
        &candidate_map,
    ))?;
    if inputs.design.sampling_frame_sha256() != candidate_frame_sha256 {
        return Err(
            "selection design sampling-frame digest differs from exact admitted-candidate frame"
                .to_owned(),
        );
    }

    let mut adjudication_map = BTreeMap::<String, &IndependentFeasibilityAdjudication>::new();
    for adjudication in inputs.adjudications {
        let candidate = candidate_map
            .get(&adjudication.candidate_id)
            .ok_or_else(|| {
                format!(
                    "adjudication references candidate outside the admitted-candidate frame: {}",
                    adjudication.candidate_id
                )
            })?;
        if candidate.output_sha256 != adjudication.output_sha256
            || candidate.scope_sha256 != adjudication.scope_sha256
        {
            return Err(format!(
                "adjudication for candidate {} does not match exact output and scope identity",
                adjudication.candidate_id
            ));
        }
        if candidate.admission_verifier_id == adjudication.adjudicator_id
            || candidate.admission_verifier_sha256 == adjudication.adjudicator_sha256
            || candidate.admission_receipt_sha256 == adjudication.adjudication_receipt_sha256
        {
            return Err(format!(
                "adjudicator and receipt for candidate {} are not distinct from its admission verifier/receipt",
                adjudication.candidate_id
            ));
        }
        if adjudication_map
            .insert(adjudication.candidate_id.clone(), adjudication)
            .is_some()
        {
            return Err(format!(
                "duplicate independent adjudication for candidate {}",
                adjudication.candidate_id
            ));
        }
    }

    let adjudication_bytes = adjudication_manifest_bytes(&adjudication_map)?;
    let adjudication_manifest_sha256 = sha256_hex(&adjudication_bytes)?;
    let mut confirmed = 0usize;
    let mut falsified = 0usize;
    let mut inconclusive = 0usize;
    for adjudication in adjudication_map.values() {
        match &adjudication.outcome {
            FeasibilityAdjudicationOutcome::ConfirmedFeasible => confirmed += 1,
            FeasibilityAdjudicationOutcome::FalsifiedInfeasible => falsified += 1,
            FeasibilityAdjudicationOutcome::Inconclusive { .. } => inconclusive += 1,
        }
    }
    let adjudicated = adjudication_map.len();
    let admitted = candidate_map.len();
    let unassessed = admitted - adjudicated;
    let mut candidate_results = Vec::with_capacity(admitted);
    for (candidate_id, candidate) in &candidate_map {
        if let Some(adjudication) = adjudication_map.get(candidate_id) {
            let (status, reason) = match &adjudication.outcome {
                FeasibilityAdjudicationOutcome::ConfirmedFeasible => (
                    FeasibilityAdjudicationStatus::ConfirmedFeasible,
                    String::new(),
                ),
                FeasibilityAdjudicationOutcome::FalsifiedInfeasible => (
                    FeasibilityAdjudicationStatus::FalsifiedInfeasible,
                    String::new(),
                ),
                FeasibilityAdjudicationOutcome::Inconclusive { reason } => {
                    (FeasibilityAdjudicationStatus::Inconclusive, reason.clone())
                }
            };
            candidate_results.push(PolicyFeasibilityCandidateResult {
                task_id: candidate.task_id.clone(),
                replicate_id: candidate.replicate_id.clone(),
                operation_id: candidate.operation_id.clone(),
                candidate_id: candidate_id.clone(),
                output_sha256: candidate.output_sha256.clone(),
                scope_sha256: candidate.scope_sha256.clone(),
                admission_receipt_sha256: candidate.admission_receipt_sha256.clone(),
                admission_verifier_id: candidate.admission_verifier_id.clone(),
                admission_verifier_sha256: candidate.admission_verifier_sha256.clone(),
                status,
                method: adjudication.method.as_str().to_owned(),
                adjudicator_id: adjudication.adjudicator_id.clone(),
                adjudicator_sha256: adjudication.adjudicator_sha256.clone(),
                adjudication_receipt_sha256: adjudication.adjudication_receipt_sha256.clone(),
                reason,
            });
        } else {
            candidate_results.push(PolicyFeasibilityCandidateResult {
                task_id: candidate.task_id.clone(),
                replicate_id: candidate.replicate_id.clone(),
                operation_id: candidate.operation_id.clone(),
                candidate_id: candidate_id.clone(),
                output_sha256: candidate.output_sha256.clone(),
                scope_sha256: candidate.scope_sha256.clone(),
                admission_receipt_sha256: candidate.admission_receipt_sha256.clone(),
                admission_verifier_id: candidate.admission_verifier_id.clone(),
                admission_verifier_sha256: candidate.admission_verifier_sha256.clone(),
                status: FeasibilityAdjudicationStatus::Unassessed,
                method: String::new(),
                adjudicator_id: String::new(),
                adjudicator_sha256: String::new(),
                adjudication_receipt_sha256: String::new(),
                reason: "no independent adjudication supplied".to_owned(),
            });
        }
    }
    if matches!(
        inputs.design,
        AdjudicationSelectionDesign::FullCensus { .. }
    ) && unassessed != 0
    {
        return Err(format!(
            "full-census design has {unassessed} admitted candidates without an adjudication"
        ));
    }
    let resolved = confirmed + falsified;
    let unresolved = inconclusive + unassessed;
    let denominator = admitted as f64;
    let (lower_bound, upper_bound, resolved_rate) = if admitted == 0 {
        (None, None, None)
    } else {
        (
            Some(falsified as f64 / denominator),
            Some((falsified + unresolved) as f64 / denominator),
            (resolved > 0).then_some(falsified as f64 / resolved as f64),
        )
    };
    let status = if admitted == 0 {
        "no_admitted_candidates"
    } else if unresolved == 0
        && matches!(
            inputs.design,
            AdjudicationSelectionDesign::FullCensus { .. }
        )
    {
        "complete_census_all_adjudications_resolved"
    } else if unresolved == 0 {
        "sampled_or_targeted_frame_all_selected_adjudications_resolved"
    } else {
        "unresolved_candidates_included_in_worst_case_bounds"
    };

    let mut report = PolicyFeasibilityAuditReport {
        schema: POLICY_FEASIBILITY_AUDIT_SCHEMA.to_owned(),
        frame_id: inputs.frame_id.to_owned(),
        policy_id: inputs.policy_id.to_owned(),
        source_task_id: source_row.task_id.clone(),
        source_replicate_id: source_row.replicate_id.clone(),
        source_candidate_frame_sha256,
        source_operation_ledger_sha256: source_row.operation_ledger_sha256.clone(),
        source_operation_count: source_row.operation_count,
        source_admission_frame_receipt_sha256: source_row.admission_frame_receipt_sha256.clone(),
        selection_design: inputs.design.as_str().to_owned(),
        selection_design_sha256: sha256_hex(&selection_design_bytes(inputs.design))?,
        selection_plan_sha256: inputs.design.selection_references().0.map(str::to_owned),
        selection_receipt_sha256: inputs.design.selection_references().1.map(str::to_owned),
        sampling_frame_sha256: inputs.design.sampling_frame_sha256().to_owned(),
        candidate_frame_sha256,
        adjudication_manifest_sha256,
        admitted_candidate_count: admitted,
        adjudicated_candidate_count: adjudicated,
        confirmed_feasible_count: confirmed,
        falsified_infeasible_count: falsified,
        inconclusive_count: inconclusive,
        unassessed_count: unassessed,
        resolved_count: resolved,
        candidate_results,
        false_feasible_rate_lower_bound: lower_bound,
        false_feasible_rate_upper_bound: upper_bound,
        false_feasible_rate_among_resolved: resolved_rate,
        status: status.to_owned(),
        evidence_level: POLICY_FEASIBILITY_AUDIT_EVIDENCE_LEVEL,
        report_sha256: String::new(),
    };
    report.report_sha256 = sha256_hex(&report_bytes(&report))?;
    Ok(report)
}

pub fn verify_policy_feasibility_audit(
    inputs: &PolicyFeasibilityAuditInputs<'_>,
    report: &PolicyFeasibilityAuditReport,
) -> Result<(), String> {
    if report.schema != POLICY_FEASIBILITY_AUDIT_SCHEMA
        || report.evidence_level != POLICY_FEASIBILITY_AUDIT_EVIDENCE_LEVEL
    {
        return Err("unsupported policy feasibility audit schema or evidence ceiling".to_owned());
    }
    let recomputed = evaluate_policy_feasibility_audit(inputs)?;
    if recomputed != *report {
        return Err("replayed policy feasibility audit differs from supplied report".to_owned());
    }
    if sha256_hex(&report_bytes(report))? != report.report_sha256 {
        return Err("policy feasibility audit canonical digest mismatch".to_owned());
    }
    Ok(())
}

fn optional_rate(value: Option<f64>) -> String {
    value.map(|rate| format!("{rate:.12}")).unwrap_or_default()
}

fn optional_rate_bits(value: Option<f64>) -> String {
    value
        .map(|rate| format!("{:016x}", rate.to_bits()))
        .unwrap_or_default()
}

fn append_csv_row(output: &mut String, values: &[String]) {
    output.push_str(
        &values
            .iter()
            .map(|value| csv_escape(value))
            .collect::<Vec<_>>()
            .join(","),
    );
    output.push('\n');
}

pub fn report_to_csv(report: &PolicyFeasibilityAuditReport) -> String {
    let header = "schema,frame_id,policy_id,source_task_id,source_replicate_id,source_candidate_frame_sha256,source_operation_ledger_sha256,source_operation_count,source_admission_frame_receipt_sha256,selection_design,selection_design_sha256,selection_plan_sha256,selection_receipt_sha256,sampling_frame_sha256,candidate_frame_sha256,adjudication_manifest_sha256,admitted_candidate_count,adjudicated_candidate_count,confirmed_feasible_count,falsified_infeasible_count,inconclusive_count,unassessed_count,resolved_count,false_feasible_rate_lower_bound,false_feasible_rate_upper_bound,false_feasible_rate_among_resolved,false_feasible_rate_lower_bound_f64_bits,false_feasible_rate_upper_bound_f64_bits,false_feasible_rate_among_resolved_f64_bits,status,report_sha256,evidence_level,task_id,replicate_id,operation_id,candidate_id,output_sha256,scope_sha256,admission_receipt_sha256,admission_verifier_id,admission_verifier_sha256,adjudication_status,method,adjudicator_id,adjudicator_sha256,adjudication_receipt_sha256,reason";
    let mut output = format!("{header}\n");
    let summary = vec![
        report.schema.clone(),
        report.frame_id.clone(),
        report.policy_id.clone(),
        report.source_task_id.clone(),
        report.source_replicate_id.clone(),
        report.source_candidate_frame_sha256.clone(),
        report.source_operation_ledger_sha256.clone(),
        report.source_operation_count.to_string(),
        report.source_admission_frame_receipt_sha256.clone(),
        report.selection_design.clone(),
        report.selection_design_sha256.clone(),
        report.selection_plan_sha256.clone().unwrap_or_default(),
        report.selection_receipt_sha256.clone().unwrap_or_default(),
        report.sampling_frame_sha256.clone(),
        report.candidate_frame_sha256.clone(),
        report.adjudication_manifest_sha256.clone(),
        report.admitted_candidate_count.to_string(),
        report.adjudicated_candidate_count.to_string(),
        report.confirmed_feasible_count.to_string(),
        report.falsified_infeasible_count.to_string(),
        report.inconclusive_count.to_string(),
        report.unassessed_count.to_string(),
        report.resolved_count.to_string(),
        optional_rate(report.false_feasible_rate_lower_bound),
        optional_rate(report.false_feasible_rate_upper_bound),
        optional_rate(report.false_feasible_rate_among_resolved),
        optional_rate_bits(report.false_feasible_rate_lower_bound),
        optional_rate_bits(report.false_feasible_rate_upper_bound),
        optional_rate_bits(report.false_feasible_rate_among_resolved),
        report.status.clone(),
        report.report_sha256.clone(),
        report.evidence_level.to_owned(),
    ];
    if report.candidate_results.is_empty() {
        let mut values = summary;
        values.extend(vec![String::new(); 15]);
        append_csv_row(&mut output, &values);
        return output;
    }
    for result in &report.candidate_results {
        let mut values = summary.clone();
        values.extend([
            result.task_id.clone(),
            result.replicate_id.clone(),
            result.operation_id.clone(),
            result.candidate_id.clone(),
            result.output_sha256.clone(),
            result.scope_sha256.clone(),
            result.admission_receipt_sha256.clone(),
            result.admission_verifier_id.clone(),
            result.admission_verifier_sha256.clone(),
            result.status.as_str().to_owned(),
            result.method.clone(),
            result.adjudicator_id.clone(),
            result.adjudicator_sha256.clone(),
            result.adjudication_receipt_sha256.clone(),
            result.reason.clone(),
        ]);
        append_csv_row(&mut output, &values);
    }
    output
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn source_operation(
        task: &str,
        replicate: &str,
        policy: &str,
        operation: &str,
        kind: &str,
        output: char,
        success: bool,
    ) -> LedgerOperation {
        LedgerOperation {
            task_id: task.to_owned(),
            replicate_id: replicate.to_owned(),
            policy_id: policy.to_owned(),
            operation_id: operation.to_owned(),
            operation_kind: kind.to_owned(),
            fidelity: if kind == "high_fidelity" { "high" } else { "low" }.to_owned(),
            aerodynamic_condition_calls_attempted: u64::from(matches!(kind, "high_fidelity" | "low_fidelity")),
            aerodynamic_condition_calls_succeeded: u64::from(
                matches!(kind, "high_fidelity" | "low_fidelity") && success
            ),
            cost_units: 1.0,
            budget_cap_units: 100.0,
            wall_seconds: 2.0,
            cpu_seconds: 1.0,
            success,
            output_sha256: if success { digest(output) } else { "-".to_owned() },
            reason_code: if success { "-".to_owned() } else { "synthetic_solver_failure".to_owned() },
        }
    }

    fn source_observation(candidate: &AdmittedCandidateFrameEntry) -> CandidateObservation {
        CandidateObservation {
            task_id: candidate.task_id.clone(),
            replicate_id: candidate.replicate_id.clone(),
            policy_id: candidate.policy_id.clone(),
            operation_id: candidate.operation_id.clone(),
            candidate_id: candidate.candidate_id.clone(),
            output_sha256: candidate.output_sha256.clone(),
            admission_status: CandidateAdmissionStatus::Admitted,
            admission_receipt_sha256: candidate.admission_receipt_sha256.clone(),
            scope_sha256: Some(candidate.scope_sha256.clone()),
            admission_verifier_id: Some(candidate.admission_verifier_id.clone()),
            admission_verifier_sha256: Some(candidate.admission_verifier_sha256.clone()),
            feasible: Some(true),
            objective_1: Some(0.02),
            objective_2: Some(500.0),
        }
    }

    fn operation_ledger_csv(operations: &[LedgerOperation]) -> String {
        let mut output = format!("{LEDGER_INPUT_HEADER}\\n");
        for operation in operations {
            output.push_str(&format!(
                "{},{},{},{},{},{},{},{},{:.12},{:.12},{:.12},{:.12},{},{},{}\\n",
                operation.task_id,
                operation.replicate_id,
                operation.policy_id,
                operation.operation_id,
                operation.operation_kind,
                operation.fidelity,
                operation.aerodynamic_condition_calls_attempted,
                operation.aerodynamic_condition_calls_succeeded,
                operation.cost_units,
                operation.budget_cap_units,
                operation.wall_seconds,
                operation.cpu_seconds,
                operation.success,
                operation.output_sha256,
                operation.reason_code
            ));
        }
        output
    }

    fn source_scorecard_for_frame(
        frame_candidates: &[AdmittedCandidateFrameEntry],
    ) -> (PolicyBenchmarkRow, Vec<LedgerOperation>, String, String) {
        let task_id = "airfoil-family-a";
        let target_replicate = "seed-1";
        let target_policy = "symtropy-voi";
        let task = PolicyBenchmarkTask {
            task_id: task_id.to_owned(),
            objective_1_name: "drag_coefficient".to_owned(),
            objective_2_name: "mission_energy_wh".to_owned(),
            ideal_point_1: 0.0,
            ideal_point_2: 0.0,
            reference_point_1: 0.1,
            reference_point_2: 1000.0,
        };
        let mut operations = Vec::new();
        let mut observations = Vec::new();
        for replicate in ["seed-1", "seed-2"] {
            for policy in ["symtropy-voi", "baseline"] {
                let is_source_group = replicate == target_replicate && policy == target_policy;
                if is_source_group && !frame_candidates.is_empty() {
                    for candidate in frame_candidates {
                        let output = candidate.output_sha256.as_bytes()[0] as char;
                        operations.push(source_operation(
                            task_id, replicate, policy, &candidate.operation_id,
                            "high_fidelity", output, true,
                        ));
                        observations.push(source_observation(candidate));
                    }
                } else {
                    let output = if is_source_group { '9' } else if policy == "baseline" { '8' } else { '7' };
                    let operation_id = format!("synthetic-{replicate}-{policy}");
                    operations.push(source_operation(
                        task_id, replicate, policy, &operation_id,
                        "high_fidelity", output, true,
                    ));
                    if is_source_group {
                        observations.push(CandidateObservation {
                            task_id: task_id.to_owned(),
                            replicate_id: replicate.to_owned(),
                            policy_id: policy.to_owned(),
                            operation_id,
                            candidate_id: "synthetic-not-admitted".to_owned(),
                            output_sha256: digest(output),
                            admission_status: CandidateAdmissionStatus::NotAdmitted,
                            admission_receipt_sha256: "-".to_owned(),
                            scope_sha256: None,
                            admission_verifier_id: None,
                            admission_verifier_sha256: None,
                            feasible: None,
                            objective_1: None,
                            objective_2: None,
                        });
                    } else {
                        let synthetic = AdmittedCandidateFrameEntry {
                            task_id: task_id.to_owned(),
                            replicate_id: replicate.to_owned(),
                            policy_id: policy.to_owned(),
                            candidate_id: format!("synthetic-{replicate}-{policy}"),
                            operation_id,
                            output_sha256: digest(output),
                            admission_receipt_sha256: digest('a'),
                            admission_verifier_id: "admission-verifier-v1".to_owned(),
                            admission_verifier_sha256: digest('b'),
                            scope_sha256: digest('c'),
                        };
                        observations.push(source_observation(&synthetic));
                    }
                }
                operations.push(source_operation(
                    task_id, replicate, policy, &format!("low-{replicate}-{policy}"),
                    "low_fidelity", if policy == "baseline" { '5' } else { '4' }, true,
                ));
                operations.push(source_operation(
                    task_id, replicate, policy, &format!("failed-{replicate}-{policy}"),
                    "high_fidelity", '0', false,
                ));
            }
        }
        let report = evaluate_policy_benchmark(&[task], &operations, &observations)
            .expect("synthetic source policy scorecard");
        let row = report.rows.iter().find(|row| {
            row.task_id == task_id
                && row.replicate_id == target_replicate
                && row.policy_id == target_policy
        }).expect("target scorecard row").clone();
        (
            row,
            operations.clone(),
            crate::policy_benchmark::report_to_csv(&report),
            operation_ledger_csv(&operations),
        )
    }

    fn candidate(
        id: &str,
        operation: &str,
        scope: char,
        output: char,
    ) -> AdmittedCandidateFrameEntry {
        AdmittedCandidateFrameEntry {
            task_id: "airfoil-family-a".to_owned(),
            replicate_id: "seed-1".to_owned(),
            policy_id: "symtropy-voi".to_owned(),
            candidate_id: id.to_owned(),
            operation_id: operation.to_owned(),
            output_sha256: digest(output),
            admission_receipt_sha256: digest('a'),
            admission_verifier_id: "admission-verifier-v1".to_owned(),
            admission_verifier_sha256: digest('b'),
            scope_sha256: digest(scope),
        }
    }

    fn adjudication(
        candidate: &AdmittedCandidateFrameEntry,
        outcome: FeasibilityAdjudicationOutcome,
        id: &str,
        verifier_digest: char,
    ) -> IndependentFeasibilityAdjudication {
        IndependentFeasibilityAdjudication {
            candidate_id: candidate.candidate_id.clone(),
            output_sha256: candidate.output_sha256.clone(),
            scope_sha256: candidate.scope_sha256.clone(),
            adjudicator_id: id.to_owned(),
            adjudicator_sha256: digest(verifier_digest),
            adjudication_receipt_sha256: digest('d'),
            method: FeasibilityAdjudicationMethod::IndependentHigherFidelityAnalysis,
            outcome,
        }
    }

    fn frame_digest(
        policy: &str,
        frame: &str,
        candidates: &[AdmittedCandidateFrameEntry],
    ) -> String {
        let map = candidates
            .iter()
            .map(|candidate| (candidate.candidate_id.clone(), candidate))
            .collect::<BTreeMap<_, _>>();
        sha256_hex(&candidate_frame_bytes(policy, frame, &map)).expect("frame digest")
    }

    #[test]
    fn full_census_bounds_include_inconclusive_outcomes() {
        let candidates = vec![
            candidate("c1", "op1", '1', 'a'),
            candidate("c2", "op2", '2', 'b'),
            candidate("c3", "op3", '3', 'c'),
        ];
        let adjudications = vec![
            adjudication(
                &candidates[0],
                FeasibilityAdjudicationOutcome::ConfirmedFeasible,
                "independent-verifier-a",
                'e',
            ),
            adjudication(
                &candidates[1],
                FeasibilityAdjudicationOutcome::FalsifiedInfeasible,
                "independent-verifier-b",
                'f',
            ),
            adjudication(
                &candidates[2],
                FeasibilityAdjudicationOutcome::Inconclusive {
                    reason: "physical test was underpowered".to_owned(),
                },
                "independent-verifier-c",
                '1',
            ),
        ];
        let frame = "campaign-2026-blind-family-a";
        let sampling_frame_sha256 = frame_digest("symtropy-voi", frame, &candidates);
        let design = AdjudicationSelectionDesign::FullCensus {
            sampling_frame_sha256,
        };
        let (
            source_scorecard_row,
            source_operation_ledger,
            source_scorecard_csv,
            source_operation_ledger_csv,
        ) = source_scorecard_for_frame(&candidates);
        let request = PolicyFeasibilityAuditInputs {
            policy_id: "symtropy-voi",
            frame_id: frame,
            design: &design,
            admitted_candidates: &candidates,
            adjudications: &adjudications,
            source_scorecard_row: &source_scorecard_row,
            source_operation_ledger: &source_operation_ledger,
        };
        let report = evaluate_policy_feasibility_audit(&request).expect("full census audit");
        assert_eq!(report.admitted_candidate_count, 3);
        assert_eq!(report.adjudicated_candidate_count, 3);
        assert_eq!(report.confirmed_feasible_count, 1);
        assert_eq!(report.falsified_infeasible_count, 1);
        assert_eq!(report.inconclusive_count, 1);
        assert_eq!(report.unassessed_count, 0);
        assert_eq!(report.false_feasible_rate_lower_bound, Some(1.0 / 3.0));
        assert_eq!(report.false_feasible_rate_upper_bound, Some(2.0 / 3.0));
        assert_eq!(report.false_feasible_rate_among_resolved, Some(0.5));
        assert_eq!(report.candidate_results.len(), 3);
        assert_eq!(
            report.candidate_results[0].status,
            FeasibilityAdjudicationStatus::ConfirmedFeasible
        );
        assert_eq!(
            report.candidate_results[1].status,
            FeasibilityAdjudicationStatus::FalsifiedInfeasible
        );
        assert_eq!(
            report.candidate_results[2].status,
            FeasibilityAdjudicationStatus::Inconclusive
        );
        assert_eq!(
            report.selection_design_sha256,
            sha256_hex(&selection_design_bytes(&design)).expect("selection digest")
        );
        let csv = report_to_csv(&report);
        assert!(csv.contains("falsified_infeasible"));
        assert!(csv.contains("physical test was underpowered"));
        assert!(csv.contains("admission-verifier-v1"));
        assert!(csv.contains("independent-verifier-b"));
        assert!(csv.contains(&digest('a')));
        assert!(csv.contains(&digest('d')));
        assert_eq!(
            csv.lines().count(),
            4,
            "one CSV row per candidate plus header"
        );
        #[cfg(target_os = "linux")]
        {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let csv_path = std::env::temp_dir().join(format!(
                "policy-feasibility-oracle-{}-{nonce}.csv",
                std::process::id()
            ));
            let scorecard_path = std::env::temp_dir().join(format!(
                "policy-scorecard-oracle-{}-{nonce}.csv",
                std::process::id()
            ));
            let ledger_path = std::env::temp_dir().join(format!(
                "policy-ledger-oracle-{}-{nonce}.csv",
                std::process::id()
            ));
            std::fs::write(&csv_path, &csv).expect("write audit CSV for independent oracle");
            std::fs::write(&scorecard_path, &source_scorecard_csv)
                .expect("write upstream policy scorecard");
            std::fs::write(&ledger_path, &source_operation_ledger_csv)
                .expect("write complete source operation ledger");
            let oracle = std::process::Command::new("python3")
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/scripts/verify-policy-feasibility-independent.py"
                ))
                .arg(&csv_path)
                .arg(&scorecard_path)
                .arg(&ledger_path)
                .output()
                .expect("run independent policy-feasibility oracle");
            let _ = std::fs::remove_file(&csv_path);
            let _ = std::fs::remove_file(&scorecard_path);
            let _ = std::fs::remove_file(&ledger_path);
            assert!(
                oracle.status.success(),
                "independent policy oracle failed: {}",
                String::from_utf8_lossy(&oracle.stderr)
            );
            assert!(
                String::from_utf8_lossy(&oracle.stdout)
                    .contains("independent_policy_feasibility_oracle=pass")
            );
        }
        verify_policy_feasibility_audit(&request, &report).expect("audit replay");
    }

    #[test]
    fn sampled_frames_keep_unassessed_candidates_in_upper_bound() {
        let candidates = vec![
            candidate("c1", "op1", '1', 'a'),
            candidate("c2", "op2", '2', 'b'),
            candidate("c3", "op3", '3', 'c'),
            candidate("c4", "op4", '4', 'e'),
        ];
        let adjudications = vec![
            adjudication(
                &candidates[0],
                FeasibilityAdjudicationOutcome::FalsifiedInfeasible,
                "independent-verifier-a",
                'e',
            ),
            adjudication(
                &candidates[1],
                FeasibilityAdjudicationOutcome::ConfirmedFeasible,
                "independent-verifier-b",
                'f',
            ),
        ];
        let frame = "campaign-2026-sampled-policy";
        let sampling_frame_sha256 = frame_digest("symtropy-voi", frame, &candidates);
        let design = AdjudicationSelectionDesign::PreRegisteredProbabilitySample {
            sampling_frame_sha256,
            selection_plan_sha256: digest('7'),
            selection_receipt_sha256: digest('8'),
        };
        let (
            source_scorecard_row,
            source_operation_ledger,
            _source_scorecard_csv,
            _source_operation_ledger_csv,
        ) = source_scorecard_for_frame(&candidates);
        let request = PolicyFeasibilityAuditInputs {
            policy_id: "symtropy-voi",
            frame_id: frame,
            design: &design,
            admitted_candidates: &candidates,
            adjudications: &adjudications,
            source_scorecard_row: &source_scorecard_row,
            source_operation_ledger: &source_operation_ledger,
        };
        let report = evaluate_policy_feasibility_audit(&request).expect("sampled audit");
        assert_eq!(report.adjudicated_candidate_count, 2);
        assert_eq!(report.unassessed_count, 2);
        assert_eq!(report.false_feasible_rate_lower_bound, Some(0.25));
        assert_eq!(report.false_feasible_rate_upper_bound, Some(0.75));
        assert_eq!(report.false_feasible_rate_among_resolved, Some(0.5));
        assert_eq!(
            report
                .candidate_results
                .iter()
                .filter(|result| result.status == FeasibilityAdjudicationStatus::Unassessed)
                .count(),
            2
        );
        let csv = report_to_csv(&report);
        assert!(csv.contains("unassessed"));
        assert!(csv.contains("no independent adjudication supplied"));
        assert!(
            csv.contains(&digest('8')),
            "selection receipt remains visible"
        );
    }

    #[test]
    fn full_census_rejects_missing_adjudication() {
        let candidates = vec![candidate("c1", "op1", '1', 'a')];
        let frame = "campaign-2026-full-census";
        let design = AdjudicationSelectionDesign::FullCensus {
            sampling_frame_sha256: frame_digest("symtropy-voi", frame, &candidates),
        };
        let (
            source_scorecard_row,
            source_operation_ledger,
            _source_scorecard_csv,
            _source_operation_ledger_csv,
        ) = source_scorecard_for_frame(&candidates);
        let request = PolicyFeasibilityAuditInputs {
            policy_id: "symtropy-voi",
            frame_id: frame,
            design: &design,
            admitted_candidates: &candidates,
            adjudications: &[],
            source_scorecard_row: &source_scorecard_row,
            source_operation_ledger: &source_operation_ledger,
        };
        assert!(
            evaluate_policy_feasibility_audit(&request)
                .expect_err("full census cannot omit an admitted candidate")
                .contains("without an adjudication")
        );
    }

    #[test]
    fn adjudication_rejects_mismatched_scope_output_duplicate_and_same_verifier() {
        let candidates = vec![candidate("c1", "op1", '1', 'a')];
        let frame = "campaign-2026-adversarial";
        let design = AdjudicationSelectionDesign::PreRegisteredProbabilitySample {
            sampling_frame_sha256: frame_digest("symtropy-voi", frame, &candidates),
            selection_plan_sha256: digest('7'),
            selection_receipt_sha256: digest('8'),
        };
        let mut wrong_scope = adjudication(
            &candidates[0],
            FeasibilityAdjudicationOutcome::ConfirmedFeasible,
            "independent-verifier",
            'e',
        );
        wrong_scope.scope_sha256 = digest('9');
        let wrong_scope_adjudications = [wrong_scope.clone()];
        let (
            source_scorecard_row,
            source_operation_ledger,
            _source_scorecard_csv,
            _source_operation_ledger_csv,
        ) = source_scorecard_for_frame(&candidates);
        let request = PolicyFeasibilityAuditInputs {
            policy_id: "symtropy-voi",
            frame_id: frame,
            design: &design,
            admitted_candidates: &candidates,
            adjudications: &wrong_scope_adjudications,
            source_scorecard_row: &source_scorecard_row,
            source_operation_ledger: &source_operation_ledger,
        };
        assert!(
            evaluate_policy_feasibility_audit(&request)
                .expect_err("scope mismatch rejected")
                .contains("exact output and scope")
        );

        wrong_scope.scope_sha256 = candidates[0].scope_sha256.clone();
        wrong_scope.output_sha256 = digest('0');
        let wrong_output_adjudications = [wrong_scope.clone()];
        let (
            source_scorecard_row,
            source_operation_ledger,
            _source_scorecard_csv,
            _source_operation_ledger_csv,
        ) = source_scorecard_for_frame(&candidates);
        let request = PolicyFeasibilityAuditInputs {
            policy_id: "symtropy-voi",
            frame_id: frame,
            design: &design,
            admitted_candidates: &candidates,
            adjudications: &wrong_output_adjudications,
            source_scorecard_row: &source_scorecard_row,
            source_operation_ledger: &source_operation_ledger,
        };
        assert!(
            evaluate_policy_feasibility_audit(&request)
                .expect_err("output mismatch rejected")
                .contains("exact output and scope")
        );

        let same_verifier = adjudication(
            &candidates[0],
            FeasibilityAdjudicationOutcome::ConfirmedFeasible,
            "admission-verifier-v1",
            'e',
        );
        let same_verifier_adjudications = [same_verifier];
        let (
            source_scorecard_row,
            source_operation_ledger,
            _source_scorecard_csv,
            _source_operation_ledger_csv,
        ) = source_scorecard_for_frame(&candidates);
        let request = PolicyFeasibilityAuditInputs {
            policy_id: "symtropy-voi",
            frame_id: frame,
            design: &design,
            admitted_candidates: &candidates,
            adjudications: &same_verifier_adjudications,
            source_scorecard_row: &source_scorecard_row,
            source_operation_ledger: &source_operation_ledger,
        };
        assert!(
            evaluate_policy_feasibility_audit(&request)
                .expect_err("same verifier must not self-adjudicate")
                .contains("not distinct")
        );
    }

    #[test]
    fn exact_frame_digest_and_unique_adjudication_receipts_are_required() {
        let candidates = vec![candidate("c1", "op1", '1', 'a')];
        let frame = "campaign-2026-selection-frame";
        let design = AdjudicationSelectionDesign::PreRegisteredProbabilitySample {
            sampling_frame_sha256: digest('9'),
            selection_plan_sha256: digest('7'),
            selection_receipt_sha256: digest('8'),
        };
        let adjudications = vec![adjudication(
            &candidates[0],
            FeasibilityAdjudicationOutcome::ConfirmedFeasible,
            "independent-verifier",
            'e',
        )];
        let (
            source_scorecard_row,
            source_operation_ledger,
            _source_scorecard_csv,
            _source_operation_ledger_csv,
        ) = source_scorecard_for_frame(&candidates);
        let request = PolicyFeasibilityAuditInputs {
            policy_id: "symtropy-voi",
            frame_id: frame,
            design: &design,
            admitted_candidates: &candidates,
            adjudications: &adjudications,
            source_scorecard_row: &source_scorecard_row,
            source_operation_ledger: &source_operation_ledger,
        };
        assert!(
            evaluate_policy_feasibility_audit(&request)
                .expect_err("wrong sampling frame digest rejected")
                .contains("sampling-frame digest")
        );

        let mut shared_receipt = adjudications[0].clone();
        shared_receipt.adjudication_receipt_sha256 = candidates[0].admission_receipt_sha256.clone();
        let correct_design = AdjudicationSelectionDesign::PreRegisteredProbabilitySample {
            sampling_frame_sha256: frame_digest("symtropy-voi", frame, &candidates),
            selection_plan_sha256: digest('7'),
            selection_receipt_sha256: digest('8'),
        };
        let shared_receipt_adjudications = [shared_receipt];
        let (
            source_scorecard_row,
            source_operation_ledger,
            _source_scorecard_csv,
            _source_operation_ledger_csv,
        ) = source_scorecard_for_frame(&candidates);
        let request = PolicyFeasibilityAuditInputs {
            policy_id: "symtropy-voi",
            frame_id: frame,
            design: &correct_design,
            admitted_candidates: &candidates,
            adjudications: &shared_receipt_adjudications,
            source_scorecard_row: &source_scorecard_row,
            source_operation_ledger: &source_operation_ledger,
        };
        assert!(
            evaluate_policy_feasibility_audit(&request)
                .expect_err("same admission receipt cannot be reused as independent adjudication")
                .contains("not distinct")
        );

        let duplicate = adjudications[0].clone();
        let duplicate_adjudications = [adjudications[0].clone(), duplicate];
        let (
            source_scorecard_row,
            source_operation_ledger,
            _source_scorecard_csv,
            _source_operation_ledger_csv,
        ) = source_scorecard_for_frame(&candidates);
        let request = PolicyFeasibilityAuditInputs {
            policy_id: "symtropy-voi",
            frame_id: frame,
            design: &correct_design,
            admitted_candidates: &candidates,
            adjudications: &duplicate_adjudications,
            source_scorecard_row: &source_scorecard_row,
            source_operation_ledger: &source_operation_ledger,
        };
        assert!(
            evaluate_policy_feasibility_audit(&request)
                .expect_err("duplicate independent adjudication rejected")
                .contains("duplicate independent adjudication")
        );
    }

    #[test]
    fn empty_admitted_frame_has_undefined_rates_not_zero() {
        let candidates = Vec::new();
        let frame = "campaign-2026-no-admissions";
        let design = AdjudicationSelectionDesign::FullCensus {
            sampling_frame_sha256: frame_digest("symtropy-voi", frame, &candidates),
        };
        let (
            source_scorecard_row,
            source_operation_ledger,
            _source_scorecard_csv,
            _source_operation_ledger_csv,
        ) = source_scorecard_for_frame(&candidates);
        let request = PolicyFeasibilityAuditInputs {
            policy_id: "symtropy-voi",
            frame_id: frame,
            design: &design,
            admitted_candidates: &candidates,
            adjudications: &[],
            source_scorecard_row: &source_scorecard_row,
            source_operation_ledger: &source_operation_ledger,
        };
        let report = evaluate_policy_feasibility_audit(&request).expect("empty frame is defined");
        assert_eq!(report.false_feasible_rate_lower_bound, None);
        assert_eq!(report.false_feasible_rate_upper_bound, None);
        assert_eq!(report.false_feasible_rate_among_resolved, None);
        assert_eq!(report.candidate_results.len(), 0);
        assert_eq!(report.selection_plan_sha256, None);
        assert_eq!(report.selection_receipt_sha256, None);
        assert_eq!(report_to_csv(&report).lines().count(), 2);
    }
}
