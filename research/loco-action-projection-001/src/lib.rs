//! Typed contract for robust policy-to-dynamics action projection.
//!
//! This reference validates supplied optimizer/authority results. It does not
//! solve a continuous whole-body QP and does not authenticate physical receipts.

use std::collections::BTreeMap;

pub const MAX_CONSTRAINTS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyProposal {
    pub policy_id: String,
    pub proposal_digest: String,
    pub input_state_digest: String,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub domain_digest: String,
    pub actuator_digest: String,
    pub constraint_inventory_digest: String,
    pub submitted_at_ns: u64,
    pub intended_apply_at_ns: u64,
    pub deadline_ns: u64,
    pub horizon_steps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverDisposition {
    Feasible,
    Infeasible,
    Unknown,
    DeadlineMiss,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintClass {
    HardPhysical,
    RobustContact,
    TaskTracking,
    ProposalProximity,
    Efficiency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintRequirement {
    pub id: String,
    pub class: ConstraintClass,
    pub residual_unit_id: String,
    pub residual_tolerance: u64,
    pub margin_unit_id: String,
    pub minimum_margin: i64,
    /// All declared hard/contact constraints are mandatory.
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintEvidence {
    pub possibility_id: String,
    pub id: String,
    pub residual_unit_id: String,
    pub residual: u64,
    /// Non-negative is feasible; negative is a hard/contact violation.
    pub margin: i64,
    pub margin_unit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionResult {
    pub solver_id: String,
    pub solver_version: String,
    pub disposition: SolverDisposition,
    pub infeasibility_witness_id: Option<String>,
    pub status_reason: Option<String>,
    pub proposal_digest: String,
    pub projected_command_digest: String,
    pub input_state_digest: String,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub domain_digest: String,
    pub actuator_digest: String,
    pub constraint_inventory_digest: String,
    pub started_at_ns: u64,
    pub completed_at_ns: u64,
    pub deadline_ns: u64,
    pub primal_residual: u64,
    pub primal_tolerance: u64,
    pub dual_residual: u64,
    pub dual_tolerance: u64,
    pub kkt_residual: u64,
    pub kkt_tolerance: u64,
    pub declared_constraints: Vec<ConstraintRequirement>,
    pub constraint_evidence: Vec<ConstraintEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityStatus {
    Admitted,
    Rejected,
    Unknown,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViabilityReceipt {
    pub receipt_id: String,
    pub result_digest: String,
    pub projected_command_digest: String,
    pub input_state_digest: String,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub domain_digest: String,
    pub actuator_digest: String,
    pub constraint_inventory_digest: String,
    pub status: AuthorityStatus,
    pub witness_or_reason: Option<String>,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
    pub horizon_steps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssuranceReceipt {
    pub receipt_id: String,
    pub result_digest: String,
    pub projected_command_digest: String,
    pub input_state_digest: String,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub domain_digest: String,
    pub actuator_digest: String,
    pub constraint_inventory_digest: String,
    pub horizon_steps: u32,
    pub status: AuthorityStatus,
    pub witness_or_reason: Option<String>,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
    pub intended_apply_at_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedFallback {
    pub action_digest: String,
    pub state_digest: String,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub domain_digest: String,
    pub actuator_digest: String,
    pub viability_receipt_id: String,
    pub viability_result_digest: String,
    pub assurance_receipt_id: String,
    pub assurance_result_digest: String,
    pub constraint_inventory_digest: String,
    pub horizon_steps: u32,
    pub viability_status: AuthorityStatus,
    pub assurance_status: AuthorityStatus,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
    pub intended_apply_at_ns: u64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionGateProblem {
    pub now_ns: u64,
    pub proposal: PolicyProposal,
    /// Independent frozen inventory bound by constraint_inventory_digest.
    pub required_constraint_inventory: Vec<ConstraintRequirement>,
    /// Exact complete contact/dynamics possibility inventory from #4083.
    pub physical_possibility_ids: Vec<String>,
    pub result: ProjectionResult,
    pub viability: ViabilityReceipt,
    pub assurance: AssuranceReceipt,
    pub fallback: Option<AdmittedFallback>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDisposition {
    ExecuteProjected {
        command_digest: String,
    },
    ExecuteFallback {
        command_digest: String,
        reason: String,
    },
    Rejected {
        reason: String,
        witness_id: Option<String>,
    },
    Unknown {
        reason: String,
    },
    Invalid {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintFinding {
    pub possibility_id: String,
    pub id: String,
    pub class: ConstraintClass,
    pub residual: u64,
    pub residual_tolerance: u64,
    pub residual_passed: bool,
    pub margin: i64,
    pub minimum_margin: i64,
    pub margin_passed: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionGateReport {
    pub disposition: GateDisposition,
    pub projected_command_digest: String,
    pub primal_residual: u64,
    pub dual_residual: u64,
    pub kkt_residual: u64,
    pub constraint_findings: Vec<ConstraintFinding>,
    pub hard_constraint_failures: Vec<String>,
    pub robust_contact_failures: Vec<String>,
    pub numerically_unqualified: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionError {
    EmptyIdentifier,
    InvalidProposalTiming,
    InvalidSolverTiming,
    ProposalDigestMismatch,
    StateIdentityMismatch,
    ModelIdentityMismatch,
    AbstractionIdentityMismatch,
    DomainIdentityMismatch,
    ActuatorIdentityMismatch,
    ResultDeadlineMismatch,
    InvalidResidualTolerance,
    ConstraintInventoryMismatch,
    PhysicalConstraintCannotBeOptional,
    TooManyConstraints,
    TooManyPossibilities,
    TooManyConstraintChecks,
    NoPhysicalPossibilities,
    DuplicatePhysicalPossibility,
    UnknownPhysicalPossibility,
    NoConstraintRequirements,
    DuplicateConstraintRequirement,
    DuplicateConstraintEvidence,
    MissingConstraintEvidence,
    UnexpectedConstraintEvidence,
    ConstraintUnitMismatch,
    MissingInfeasibilityWitness,
    MissingStatusReason,
    InvalidReceipt,
    StaleReceipt,
    ReceiptContextMismatch,
    ProjectedCommandMismatch,
    FallbackNotAdmitted,
    ArithmeticOverflow,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_receipt_interval(
    valid_from_ns: u64,
    valid_until_ns: u64,
    at_ns: u64,
) -> Result<(), ProjectionError> {
    if valid_from_ns > valid_until_ns {
        return Err(ProjectionError::InvalidReceipt);
    }
    if at_ns < valid_from_ns || at_ns > valid_until_ns {
        return Err(ProjectionError::StaleReceipt);
    }
    Ok(())
}

fn fallback_disposition(
    fallback: &Option<AdmittedFallback>,
    problem: &ProjectionGateProblem,
    reason: &str,
) -> GateDisposition {
    let Some(fallback) = fallback else {
        return GateDisposition::Unknown {
            reason: format!("{reason}; no independently admitted fallback"),
        };
    };
    if fallback.action_digest.trim().is_empty()
        || fallback.action_digest == problem.result.projected_command_digest
        || !nonempty(&fallback.reason)
        || !nonempty(&fallback.viability_receipt_id)
        || !nonempty(&fallback.viability_result_digest)
        || !nonempty(&fallback.assurance_receipt_id)
        || !nonempty(&fallback.assurance_result_digest)
        || fallback.state_digest != problem.proposal.input_state_digest
        || fallback.model_digest != problem.proposal.model_digest
        || fallback.abstraction_digest != problem.proposal.abstraction_digest
        || fallback.domain_digest != problem.proposal.domain_digest
        || fallback.actuator_digest != problem.proposal.actuator_digest
        || fallback.intended_apply_at_ns != problem.proposal.intended_apply_at_ns
        || fallback.valid_from_ns > fallback.valid_until_ns
        || problem.now_ns < fallback.valid_from_ns
        || problem.now_ns > fallback.valid_until_ns
        || fallback.intended_apply_at_ns < fallback.valid_from_ns
        || fallback.intended_apply_at_ns > fallback.valid_until_ns
        || fallback.viability_status != AuthorityStatus::Admitted
        || fallback.assurance_status != AuthorityStatus::Admitted
    {
        return GateDisposition::Unknown {
            reason: format!("{reason}; fallback is not currently admitted"),
        };
    }
    GateDisposition::ExecuteFallback {
        command_digest: fallback.action_digest.clone(),
        reason: reason.to_owned(),
    }
}

fn identity_gate(problem: &ProjectionGateProblem) -> Result<(), ProjectionError> {
    let proposal = &problem.proposal;
    let result = &problem.result;
    for id in [
        proposal.policy_id.as_str(),
        proposal.proposal_digest.as_str(),
        proposal.input_state_digest.as_str(),
        proposal.model_digest.as_str(),
        proposal.abstraction_digest.as_str(),
        proposal.domain_digest.as_str(),
        proposal.actuator_digest.as_str(),
        proposal.constraint_inventory_digest.as_str(),
        result.solver_id.as_str(),
        result.solver_version.as_str(),
        result.projected_command_digest.as_str(),
    ] {
        if !nonempty(id) {
            return Err(ProjectionError::EmptyIdentifier);
        }
    }
    if proposal.horizon_steps == 0
        || proposal.submitted_at_ns > problem.now_ns
        || proposal.submitted_at_ns > proposal.intended_apply_at_ns
        || proposal.intended_apply_at_ns > proposal.deadline_ns
        || problem.now_ns > proposal.intended_apply_at_ns
    {
        return Err(ProjectionError::InvalidProposalTiming);
    }
    if result.started_at_ns > result.completed_at_ns
        || result.started_at_ns < proposal.submitted_at_ns
        || result.completed_at_ns > problem.now_ns
    {
        return Err(ProjectionError::InvalidSolverTiming);
    }
    if result.proposal_digest != proposal.proposal_digest {
        return Err(ProjectionError::ProposalDigestMismatch);
    }
    if result.input_state_digest != proposal.input_state_digest {
        return Err(ProjectionError::StateIdentityMismatch);
    }
    if result.model_digest != proposal.model_digest {
        return Err(ProjectionError::ModelIdentityMismatch);
    }
    if result.abstraction_digest != proposal.abstraction_digest {
        return Err(ProjectionError::AbstractionIdentityMismatch);
    }
    if result.domain_digest != proposal.domain_digest {
        return Err(ProjectionError::DomainIdentityMismatch);
    }
    if result.actuator_digest != proposal.actuator_digest {
        return Err(ProjectionError::ActuatorIdentityMismatch);
    }
    if result.constraint_inventory_digest != proposal.constraint_inventory_digest {
        return Err(ProjectionError::ConstraintInventoryMismatch);
    }
    if result.deadline_ns != proposal.deadline_ns
        || result.completed_at_ns > proposal.deadline_ns
        || result.completed_at_ns > proposal.intended_apply_at_ns
    {
        return Err(ProjectionError::ResultDeadlineMismatch);
    }
    if result.primal_residual > u64::MAX / 2
        || result.dual_residual > u64::MAX / 2
        || result.kkt_residual > u64::MAX / 2
    {
        return Err(ProjectionError::InvalidResidualTolerance);
    }
    Ok(())
}

fn constraint_gate(
    result: &ProjectionResult,
    frozen_inventory: &[ConstraintRequirement],
) -> Result<(Vec<ConstraintFinding>, Vec<String>, Vec<String>), ProjectionError> {
    if result.declared_constraints.is_empty() {
        return Err(ProjectionError::NoConstraintRequirements);
    }
    if result.declared_constraints.len() > MAX_CONSTRAINTS
        || frozen_inventory.len() > MAX_CONSTRAINTS
    {
        return Err(ProjectionError::TooManyConstraints);
    }
    let mut frozen_by_id = BTreeMap::new();
    for requirement in frozen_inventory {
        if !nonempty(&requirement.id) {
            return Err(ProjectionError::EmptyIdentifier);
        }
        if frozen_by_id
            .insert(requirement.id.as_str(), requirement)
            .is_some()
        {
            return Err(ProjectionError::DuplicateConstraintRequirement);
        }
    }
    if frozen_by_id.is_empty() {
        return Err(ProjectionError::NoConstraintRequirements);
    }
    let mut requirements = BTreeMap::new();
    for requirement in &result.declared_constraints {
        if !nonempty(&requirement.id)
            || !nonempty(&requirement.residual_unit_id)
            || !nonempty(&requirement.margin_unit_id)
        {
            return Err(ProjectionError::EmptyIdentifier);
        }
        if matches!(
            requirement.class,
            ConstraintClass::HardPhysical | ConstraintClass::RobustContact
        ) && !requirement.required
        {
            return Err(ProjectionError::PhysicalConstraintCannotBeOptional);
        }
        if requirements
            .insert(requirement.id.as_str(), requirement)
            .is_some()
        {
            return Err(ProjectionError::DuplicateConstraintRequirement);
        }
    }
    if requirements.len() != frozen_by_id.len()
        || requirements.iter().any(|(id, requirement)| {
            frozen_by_id
                .get(id)
                .is_none_or(|frozen| *frozen != *requirement)
        })
    {
        return Err(ProjectionError::ConstraintInventoryMismatch);
    }

    let mut evidence_by_id = BTreeMap::new();
    for evidence in &result.constraint_evidence {
        if !nonempty(&evidence.id)
            || !nonempty(&evidence.residual_unit_id)
            || !nonempty(&evidence.margin_unit_id)
        {
            return Err(ProjectionError::EmptyIdentifier);
        }
        if evidence_by_id
            .insert(evidence.id.as_str(), evidence)
            .is_some()
        {
            return Err(ProjectionError::DuplicateConstraintEvidence);
        }
        if !requirements.contains_key(evidence.id.as_str()) {
            return Err(ProjectionError::UnexpectedConstraintEvidence);
        }
    }

    let mut findings = Vec::with_capacity(requirements.len());
    let mut hard_failures = Vec::new();
    let mut contact_failures = Vec::new();
    for (id, requirement) in &requirements {
        let evidence = evidence_by_id
            .get(id)
            .ok_or(ProjectionError::MissingConstraintEvidence)?;
        if evidence.residual_unit_id != requirement.residual_unit_id
            || evidence.margin_unit_id != requirement.margin_unit_id
        {
            return Err(ProjectionError::ConstraintUnitMismatch);
        }
        let passed = evidence.residual <= requirement.residual_tolerance && evidence.margin >= 0;
        if requirement.required && !passed {
            match requirement.class {
                ConstraintClass::HardPhysical => hard_failures.push((*id).to_owned()),
                ConstraintClass::RobustContact => contact_failures.push((*id).to_owned()),
                ConstraintClass::TaskTracking
                | ConstraintClass::ProposalProximity
                | ConstraintClass::Efficiency => {}
            }
        }
        findings.push(ConstraintFinding {
            id: (*id).to_owned(),
            class: requirement.class,
            residual: evidence.residual,
            residual_tolerance: requirement.residual_tolerance,
            margin: evidence.margin,
            passed,
        });
    }
    findings.sort_by(|left, right| left.id.cmp(&right.id));
    hard_failures.sort();
    contact_failures.sort();
    Ok((findings, hard_failures, contact_failures))
}

fn validate_independent_receipts(problem: &ProjectionGateProblem) -> Result<(), ProjectionError> {
    let proposal = &problem.proposal;
    let result = &problem.result;
    let viability = &problem.viability;
    let assurance = &problem.assurance;

    for id in [
        viability.receipt_id.as_str(),
        viability.result_digest.as_str(),
        assurance.receipt_id.as_str(),
        assurance.result_digest.as_str(),
    ] {
        if !nonempty(id) {
            return Err(ProjectionError::EmptyIdentifier);
        }
    }
    if viability.projected_command_digest != result.projected_command_digest
        || assurance.projected_command_digest != result.projected_command_digest
    {
        return Err(ProjectionError::ProjectedCommandMismatch);
    }
    if viability.input_state_digest != proposal.input_state_digest
        || assurance.input_state_digest != proposal.input_state_digest
    {
        return Err(ProjectionError::StateIdentityMismatch);
    }
    if viability.model_digest != proposal.model_digest
        || assurance.model_digest != proposal.model_digest
    {
        return Err(ProjectionError::ModelIdentityMismatch);
    }
    if viability.abstraction_digest != proposal.abstraction_digest {
        return Err(ProjectionError::AbstractionIdentityMismatch);
    }
    if viability.domain_digest != proposal.domain_digest {
        return Err(ProjectionError::DomainIdentityMismatch);
    }
    if viability.actuator_digest != proposal.actuator_digest
        || assurance.actuator_digest != proposal.actuator_digest
    {
        return Err(ProjectionError::ActuatorIdentityMismatch);
    }
    if viability.constraint_inventory_digest != proposal.constraint_inventory_digest {
        return Err(ProjectionError::ConstraintInventoryMismatch);
    }
    if viability.horizon_steps == 0 || viability.horizon_steps != proposal.horizon_steps {
        return Err(ProjectionError::InvalidReceipt);
    }
    validate_receipt_interval(
        viability.valid_from_ns,
        viability.valid_until_ns,
        proposal.intended_apply_at_ns,
    )?;
    validate_receipt_interval(
        assurance.valid_from_ns,
        assurance.valid_until_ns,
        proposal.intended_apply_at_ns,
    )?;
    if assurance.intended_apply_at_ns != proposal.intended_apply_at_ns {
        return Err(ProjectionError::ReceiptContextMismatch);
    }
    Ok(())
}

pub fn qualify_projected_action(
    problem: &ProjectionGateProblem,
) -> Result<ProjectionGateReport, ProjectionError> {
    identity_gate(problem)?;

    let mut report = ProjectionGateReport {
        disposition: GateDisposition::Unknown {
            reason: "projection gate not evaluated".to_owned(),
        },
        projected_command_digest: problem.result.projected_command_digest.clone(),
        primal_residual: problem.result.primal_residual,
        dual_residual: problem.result.dual_residual,
        kkt_residual: problem.result.kkt_residual,
        constraint_findings: Vec::new(),
        hard_constraint_failures: Vec::new(),
        robust_contact_failures: Vec::new(),
    };

    match problem.result.disposition {
        SolverDisposition::Infeasible => {
            if problem
                .result
                .infeasibility_witness_id
                .as_deref()
                .is_none_or(|value| !nonempty(value))
            {
                return Err(ProjectionError::MissingInfeasibilityWitness);
            }
            report.disposition =
                fallback_disposition(&problem.fallback, problem, "projection infeasible");
            return Ok(report);
        }
        SolverDisposition::Unknown => {
            if problem
                .result
                .status_reason
                .as_deref()
                .is_none_or(|value| !nonempty(value))
            {
                return Err(ProjectionError::MissingStatusReason);
            }
            report.disposition =
                fallback_disposition(&problem.fallback, problem, "projection unknown");
            return Ok(report);
        }
        SolverDisposition::DeadlineMiss => {
            report.disposition =
                fallback_disposition(&problem.fallback, problem, "projection deadline missed");
            return Ok(report);
        }
        SolverDisposition::Invalid => {
            if problem
                .result
                .status_reason
                .as_deref()
                .is_none_or(|value| !nonempty(value))
            {
                return Err(ProjectionError::MissingStatusReason);
            }
            report.disposition =
                fallback_disposition(&problem.fallback, problem, "projection invalid");
            return Ok(report);
        }
        SolverDisposition::Feasible => {}
    }

    if problem.result.primal_residual > problem.result.primal_tolerance
        || problem.result.dual_residual > problem.result.dual_tolerance
        || problem.result.kkt_residual > problem.result.kkt_tolerance
    {
        report.disposition = fallback_disposition(
            &problem.fallback,
            problem,
            "solver residual exceeds frozen tolerance",
        );
        return Ok(report);
    }

    let (findings, hard_failures, contact_failures) =
        constraint_gate(&problem.result, &problem.required_constraint_inventory)?;
    report.constraint_findings = findings;
    report.hard_constraint_failures = hard_failures;
    report.robust_contact_failures = contact_failures;

    if !report.hard_constraint_failures.is_empty() {
        report.disposition = fallback_disposition(
            &problem.fallback,
            problem,
            "hard physical constraint failure",
        );
        return Ok(report);
    }
    if !report.robust_contact_failures.is_empty() {
        report.disposition = fallback_disposition(
            &problem.fallback,
            problem,
            "robust contact constraint failure",
        );
        return Ok(report);
    }

    validate_independent_receipts(problem)?;
    match problem.viability.status {
        AuthorityStatus::Rejected => {
            report.disposition = fallback_disposition(
                &problem.fallback,
                problem,
                "independent viability gate rejected projected command",
            );
            return Ok(report);
        }
        AuthorityStatus::Unknown | AuthorityStatus::Invalid => {
            report.disposition = fallback_disposition(
                &problem.fallback,
                problem,
                "independent viability result is not admitted",
            );
            return Ok(report);
        }
        AuthorityStatus::Admitted => {}
    }
    match problem.assurance.status {
        AuthorityStatus::Admitted => {}
        AuthorityStatus::Rejected | AuthorityStatus::Unknown | AuthorityStatus::Invalid => {
            report.disposition = fallback_disposition(
                &problem.fallback,
                problem,
                "application-time assurance is not valid",
            );
            return Ok(report);
        }
    }

    report.disposition = GateDisposition::ExecuteProjected {
        command_digest: problem.result.projected_command_digest.clone(),
    };
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal() -> PolicyProposal {
        PolicyProposal {
            policy_id: "policy-v1".to_owned(),
            proposal_digest: "proposal-v1".to_owned(),
            input_state_digest: "state-v1".to_owned(),
            model_digest: "model-v1".to_owned(),
            abstraction_digest: "abstraction-v1".to_owned(),
            domain_digest: "domain-v1".to_owned(),
            actuator_digest: "actuator-v1".to_owned(),
            constraint_inventory_digest: "constraints-v1".to_owned(),
            submitted_at_ns: 100,
            intended_apply_at_ns: 200,
            deadline_ns: 200,
            horizon_steps: 10,
        }
    }

    fn requirement(id: &str, class: ConstraintClass) -> ConstraintRequirement {
        ConstraintRequirement {
            id: id.to_owned(),
            class,
            residual_unit_id: "residual_nano".to_owned(),
            residual_tolerance: 10,
            margin_unit_id: "margin_ppm".to_owned(),
            required: true,
        }
    }

    fn evidence(id: &str, residual: u64, margin: i64) -> ConstraintEvidence {
        ConstraintEvidence {
            id: id.to_owned(),
            residual_unit_id: "residual_nano".to_owned(),
            residual,
            margin,
            margin_unit_id: "margin_ppm".to_owned(),
        }
    }

    fn problem() -> ProjectionGateProblem {
        ProjectionGateProblem {
            now_ns: 190,
            proposal: proposal(),
            required_constraint_inventory: vec![
                requirement("torque-limit", ConstraintClass::HardPhysical),
                requirement("friction-cone", ConstraintClass::RobustContact),
                requirement("tracking", ConstraintClass::TaskTracking),
            ],
            result: ProjectionResult {
                solver_id: "qp-solver".to_owned(),
                solver_version: "1.0".to_owned(),
                disposition: SolverDisposition::Feasible,
                infeasibility_witness_id: None,
                status_reason: None,
                proposal_digest: "proposal-v1".to_owned(),
                projected_command_digest: "command-v1".to_owned(),
                input_state_digest: "state-v1".to_owned(),
                model_digest: "model-v1".to_owned(),
                abstraction_digest: "abstraction-v1".to_owned(),
                domain_digest: "domain-v1".to_owned(),
                actuator_digest: "actuator-v1".to_owned(),
                constraint_inventory_digest: "constraints-v1".to_owned(),
                started_at_ns: 110,
                completed_at_ns: 180,
                deadline_ns: 200,
                primal_residual: 1,
                primal_tolerance: 10,
                dual_residual: 1,
                dual_tolerance: 10,
                kkt_residual: 1,
                kkt_tolerance: 10,
                declared_constraints: vec![
                    requirement("torque-limit", ConstraintClass::HardPhysical),
                    requirement("friction-cone", ConstraintClass::RobustContact),
                    requirement("tracking", ConstraintClass::TaskTracking),
                ],
                constraint_evidence: vec![
                    evidence("torque-limit", 1, 100),
                    evidence("friction-cone", 1, 50),
                    evidence("tracking", 20, 0),
                ],
            },
            viability: ViabilityReceipt {
                receipt_id: "viability-receipt".to_owned(),
                result_digest: "viability-result".to_owned(),
                projected_command_digest: "command-v1".to_owned(),
                input_state_digest: "state-v1".to_owned(),
                model_digest: "model-v1".to_owned(),
                abstraction_digest: "abstraction-v1".to_owned(),
                domain_digest: "domain-v1".to_owned(),
                actuator_digest: "actuator-v1".to_owned(),
                constraint_inventory_digest: "constraints-v1".to_owned(),
                status: AuthorityStatus::Admitted,
                witness_or_reason: None,
                valid_from_ns: 150,
                valid_until_ns: 250,
                horizon_steps: 10,
            },
            assurance: AssuranceReceipt {
                receipt_id: "assurance-receipt".to_owned(),
                result_digest: "assurance-result".to_owned(),
                projected_command_digest: "command-v1".to_owned(),
                input_state_digest: "state-v1".to_owned(),
                model_digest: "model-v1".to_owned(),
                actuator_digest: "actuator-v1".to_owned(),
                status: AuthorityStatus::Admitted,
                witness_or_reason: None,
                valid_from_ns: 150,
                valid_until_ns: 250,
                intended_apply_at_ns: 200,
            },
            fallback: None,
        }
    }

    fn admitted_fallback() -> AdmittedFallback {
        AdmittedFallback {
            action_digest: "fallback-v1".to_owned(),
            state_digest: "state-v1".to_owned(),
            model_digest: "model-v1".to_owned(),
            abstraction_digest: "abstraction-v1".to_owned(),
            domain_digest: "domain-v1".to_owned(),
            actuator_digest: "actuator-v1".to_owned(),
            viability_receipt_id: "fallback-viability-receipt".to_owned(),
            viability_result_digest: "fallback-viability-result".to_owned(),
            assurance_receipt_id: "fallback-assurance-receipt".to_owned(),
            assurance_result_digest: "fallback-assurance-result".to_owned(),
            viability_status: AuthorityStatus::Admitted,
            assurance_status: AuthorityStatus::Admitted,
            valid_from_ns: 150,
            valid_until_ns: 250,
            intended_apply_at_ns: 200,
            reason: "prequalified hold".to_owned(),
        }
    }

    #[test]
    fn exact_projected_command_requires_independent_viability_and_assurance() {
        let report = qualify_projected_action(&problem()).expect("valid projection");
        assert_eq!(
            report.disposition,
            GateDisposition::ExecuteProjected {
                command_digest: "command-v1".to_owned()
            }
        );
    }

    #[test]
    fn infeasible_projection_executes_only_separately_admitted_fallback() {
        let mut input = problem();
        input.result.disposition = SolverDisposition::Infeasible;
        input.result.infeasibility_witness_id = Some("empty-friction-intersection".to_owned());
        input.fallback = Some(admitted_fallback());
        let report = qualify_projected_action(&input).expect("fallback disposition");
        assert_eq!(
            report.disposition,
            GateDisposition::ExecuteFallback {
                command_digest: "fallback-v1".to_owned(),
                reason: "projection infeasible".to_owned()
            }
        );
    }

    #[test]
    fn missing_fallback_does_not_execute_partial_solver_command() {
        let mut input = problem();
        input.result.disposition = SolverDisposition::Unknown;
        input.result.status_reason = Some("mode enumeration incomplete".to_owned());
        let report = qualify_projected_action(&input).expect("unknown disposition");
        assert!(matches!(
            report.disposition,
            GateDisposition::Unknown { .. }
        ));
    }

    #[test]
    fn residual_over_tolerance_prevents_execution() {
        let mut input = problem();
        input.result.kkt_residual = 11;
        let report = qualify_projected_action(&input).expect("residual gate");
        assert!(matches!(report.disposition, GateDisposition::Unknown { .. }));
    }

    #[test]
    fn hard_actuator_constraint_failure_prevents_execution() {
        let mut input = problem();
        input.result.constraint_evidence[0].margin = -1;
        let report = qualify_projected_action(&input).expect("hard constraint gate");
        assert!(matches!(report.disposition, GateDisposition::Unknown { .. }));
        assert_eq!(
            report.hard_constraint_failures,
            vec!["torque-limit".to_owned()]
        );
    }

    #[test]
    fn robust_contact_failure_prevents_execution() {
        let mut input = problem();
        input.result.constraint_evidence[1].margin = -1;
        let report = qualify_projected_action(&input).expect("contact gate");
        assert!(matches!(report.disposition, GateDisposition::Unknown { .. }));
        assert_eq!(
            report.robust_contact_failures,
            vec!["friction-cone".to_owned()]
        );
    }

    #[test]
    fn missing_required_constraint_evidence_is_invalid() {
        let mut input = problem();
        input.result.constraint_evidence.pop();
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::MissingConstraintEvidence)
        );
    }

    #[test]
    fn command_digest_mismatch_with_viability_receipt_is_invalid() {
        let mut input = problem();
        input.viability.projected_command_digest = "different-command".to_owned();
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::ProjectedCommandMismatch)
        );
    }

    #[test]
    fn viability_horizon_must_match_the_proposal_horizon() {
        let mut input = problem();
        input.viability.horizon_steps = 9;
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::InvalidReceipt)
        );
    }

    #[test]
    fn stale_application_assurance_prevents_execution() {
        let mut input = problem();
        input.assurance.valid_until_ns = 199;
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::StaleReceipt)
        );
    }

    #[test]
    fn independent_viability_rejection_prevents_execution() {
        let mut input = problem();
        input.viability.status = AuthorityStatus::Rejected;
        input.viability.witness_or_reason = Some("no-recovery-state".to_owned());
        let report = qualify_projected_action(&input).expect("viability rejection");
        assert!(matches!(report.disposition, GateDisposition::Unknown { .. }));
    }

    #[test]
    fn deadline_missed_projection_never_executes_stale_command() {
        let mut input = problem();
        input.now_ns = 205;
        input.proposal.intended_apply_at_ns = 205;
        input.proposal.deadline_ns = 205;
        input.result.deadline_ns = 205;
        input.result.disposition = SolverDisposition::DeadlineMiss;
        input.result.completed_at_ns = 201;
        let mut fallback = admitted_fallback();
        fallback.intended_apply_at_ns = 205;
        input.fallback = Some(fallback);
        let report = qualify_projected_action(&input).expect("deadline fallback");
        assert!(matches!(report.disposition, GateDisposition::ExecuteFallback { .. }));
    }

    #[test]
    fn solver_cannot_omit_independently_required_constraint_family() {
        let mut input = problem();
        input.result.declared_constraints.pop();
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::ConstraintInventoryMismatch)
        );
    }

    #[test]
    fn residual_unit_mismatch_is_invalid() {
        let mut input = problem();
        input.result.constraint_evidence[0].residual_unit_id = "wrong_unit".to_owned();
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::ConstraintUnitMismatch)
        );
    }

    #[test]
    fn constraint_inventory_digest_mismatch_is_invalid() {
        let mut input = problem();
        input.result.constraint_inventory_digest = "different-constraints".to_owned();
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::ConstraintInventoryMismatch)
        );
    }

    #[test]
    fn hard_constraint_cannot_be_declared_optional() {
        let mut input = problem();
        input.result.declared_constraints[0].required = false;
        input.required_constraint_inventory[0].required = false;
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::PhysicalConstraintCannotBeOptional)
        );
    }

    #[test]
    fn future_projection_result_is_not_accepted_before_completion() {
        let mut input = problem();
        input.now_ns = 170;
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::InvalidSolverTiming)
        );
    }

    #[test]
    fn duplicate_constraint_evidence_is_invalid() {
        let mut input = problem();
        input
            .result
            .constraint_evidence
            .push(evidence("friction-cone", 1, 50));
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::DuplicateConstraintEvidence)
        );
    }

    #[test]
    fn fallback_with_wrong_model_or_state_is_never_executed() {
        let mut input = problem();
        input.result.disposition = SolverDisposition::Unknown;
        input.result.status_reason = Some("unknown-solver-state".to_owned());
        let mut fallback = admitted_fallback();
        fallback.model_digest = "different-model".to_owned();
        input.fallback = Some(fallback);
        let report = qualify_projected_action(&input).expect("mismatched fallback withheld");
        assert!(matches!(report.disposition, GateDisposition::Unknown { .. }));
    }

    #[test]
    fn fallback_with_unknown_assurance_is_never_executed() {
        let mut input = problem();
        input.result.disposition = SolverDisposition::Unknown;
        input.result.status_reason = Some("incomplete-contact-set".to_owned());
        let mut fallback = admitted_fallback();
        fallback.assurance_status = AuthorityStatus::Unknown;
        input.fallback = Some(fallback);
        let report = qualify_projected_action(&input).expect("fallback withheld");
        assert!(matches!(report.disposition, GateDisposition::Unknown { .. }));
    }

    #[test]
    fn constraint_order_does_not_change_canonical_report() {
        let original = qualify_projected_action(&problem()).expect("original");
        let mut permuted = problem();
        permuted.result.declared_constraints.reverse();
        permuted.result.constraint_evidence.reverse();
        let reordered = qualify_projected_action(&permuted).expect("permuted");
        assert_eq!(original, reordered);
    }
}
