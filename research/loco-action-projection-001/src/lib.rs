//! Typed contract for robust policy-to-dynamics action projection.
//!
//! This reference validates supplied optimizer/authority results. It does not
//! solve a continuous whole-body QP and does not authenticate physical receipts.

use std::collections::BTreeMap;

pub const MAX_CONSTRAINTS: usize = 4096;
pub const MAX_POSSIBILITIES: usize = 256;
pub const MAX_CONSTRAINT_CHECKS: usize = 1_000_000;

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
    pub physical_possibility_digest: String,
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
    pub physical_possibility_digest: String,
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
    pub physical_possibility_digest: String,
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
    pub physical_possibility_digest: String,
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
    pub physical_possibility_digest: String,
    pub horizon_steps: u32,
    pub viability_status: AuthorityStatus,
    pub assurance_status: AuthorityStatus,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
    pub intended_apply_at_ns: u64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalPossibilityReceipt {
    pub receipt_id: String,
    pub result_digest: String,
    pub possibility_set_digest: String,
    pub possibility_ids: Vec<String>,
    pub input_state_digest: String,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub domain_digest: String,
    pub actuator_digest: String,
    pub horizon_steps: u32,
    pub complete: bool,
    pub status: AuthorityStatus,
    pub witness_or_reason: Option<String>,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionGateProblem {
    pub now_ns: u64,
    pub physical_possibility_receipt: PhysicalPossibilityReceipt,
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
pub enum RealizationStatus {
    Matched,
    Mismatch,
    Unknown,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActuatorRealizationReceipt {
    pub receipt_id: String,
    pub result_digest: String,
    pub executed_command_digest: String,
    pub input_state_digest: String,
    /// Opaque identity for measured command-to-actuator response evidence.
    pub measured_response_digest: Option<String>,
    /// New post-action state; never reuse the pre-action state certificate.
    pub resulting_state_digest: Option<String>,
    pub state_observation_digest: Option<String>,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub domain_digest: String,
    pub actuator_digest: String,
    pub status: RealizationStatus,
    pub witness_or_reason: Option<String>,
    pub applied_at_ns: u64,
    pub observed_at_ns: u64,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealizationDisposition {
    ConfirmedDelivered { result_digest: String },
    DeliveryMismatch { witness_id: String },
    Unknown { reason: String },
    Invalid { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryReconciliation {
    pub disposition: RealizationDisposition,
    pub measured_response_digest: Option<String>,
    pub resulting_state_digest: Option<String>,
    pub state_reconciled: bool,
    pub command_realized_as_expected: bool,
    /// Every next action must be a new proposal with fresh state/model/viability
    /// and application-time assurance evidence for the resulting state.
    pub fresh_proposal_and_viability_required: bool,
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
    PhysicalPossibilityDigestMismatch,
    PhysicalPossibilityReceiptMismatch,
    IncompletePhysicalPossibilitySet,
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
    UnexpectedExecutionReceipt,
    ExecutionReceiptMismatch,
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
        || !nonempty(&fallback.constraint_inventory_digest)
        || !nonempty(&fallback.physical_possibility_digest)
        || fallback.constraint_inventory_digest != problem.proposal.constraint_inventory_digest
        || fallback.physical_possibility_digest != problem.proposal.physical_possibility_digest
        || fallback.horizon_steps != problem.proposal.horizon_steps
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

fn rejected_or_fallback(
    fallback: &Option<AdmittedFallback>,
    problem: &ProjectionGateProblem,
    reason: &str,
    witness_id: Option<String>,
) -> GateDisposition {
    if fallback.is_some() {
        fallback_disposition(fallback, problem, reason)
    } else {
        GateDisposition::Rejected {
            reason: reason.to_owned(),
            witness_id,
        }
    }
}

fn invalid_or_fallback(
    fallback: &Option<AdmittedFallback>,
    problem: &ProjectionGateProblem,
    reason: &str,
) -> GateDisposition {
    if fallback.is_some() {
        fallback_disposition(fallback, problem, reason)
    } else {
        GateDisposition::Invalid {
            reason: reason.to_owned(),
        }
    }
}

fn validate_authority_witness(
    status: AuthorityStatus,
    witness_or_reason: &Option<String>,
) -> Result<(), ProjectionError> {
    if status != AuthorityStatus::Admitted
        && witness_or_reason
            .as_deref()
            .is_none_or(|value| !nonempty(value))
    {
        return Err(ProjectionError::MissingStatusReason);
    }
    Ok(())
}

fn possibility_receipt_disposition(
    problem: &ProjectionGateProblem,
) -> Result<Option<GateDisposition>, ProjectionError> {
    let receipt = &problem.physical_possibility_receipt;
    for id in [
        receipt.receipt_id.as_str(),
        receipt.result_digest.as_str(),
        receipt.possibility_set_digest.as_str(),
        receipt.input_state_digest.as_str(),
        receipt.model_digest.as_str(),
        receipt.abstraction_digest.as_str(),
        receipt.domain_digest.as_str(),
        receipt.actuator_digest.as_str(),
    ] {
        if !nonempty(id) {
            return Err(ProjectionError::EmptyIdentifier);
        }
    }
    validate_authority_witness(receipt.status, &receipt.witness_or_reason)?;
    if receipt.possibility_set_digest != problem.proposal.physical_possibility_digest
        || receipt.input_state_digest != problem.proposal.input_state_digest
        || receipt.model_digest != problem.proposal.model_digest
        || receipt.abstraction_digest != problem.proposal.abstraction_digest
        || receipt.domain_digest != problem.proposal.domain_digest
        || receipt.actuator_digest != problem.proposal.actuator_digest
        || receipt.horizon_steps != problem.proposal.horizon_steps
    {
        return Err(ProjectionError::PhysicalPossibilityReceiptMismatch);
    }
    validate_receipt_interval(
        receipt.valid_from_ns,
        receipt.valid_until_ns,
        problem.proposal.intended_apply_at_ns,
    )?;

    let mut receipt_ids = std::collections::BTreeSet::new();
    for id in &receipt.possibility_ids {
        if !nonempty(id) {
            return Err(ProjectionError::EmptyIdentifier);
        }
        if !receipt_ids.insert(id.as_str()) {
            return Err(ProjectionError::DuplicatePhysicalPossibility);
        }
    }
    let mut problem_ids = std::collections::BTreeSet::new();
    for id in &problem.physical_possibility_ids {
        if !nonempty(id) {
            return Err(ProjectionError::EmptyIdentifier);
        }
        if !problem_ids.insert(id.as_str()) {
            return Err(ProjectionError::DuplicatePhysicalPossibility);
        }
    }
    if receipt_ids != problem_ids {
        return Err(ProjectionError::PhysicalPossibilityReceiptMismatch);
    }

    if !receipt.complete {
        return Ok(Some(fallback_disposition(
            &problem.fallback,
            problem,
            "physical possibility enumeration incomplete",
        )));
    }
    match receipt.status {
        AuthorityStatus::Admitted => Ok(None),
        AuthorityStatus::Rejected => Ok(Some(rejected_or_fallback(
            &problem.fallback,
            problem,
            "physical possibility authority rejected set",
            receipt.witness_or_reason.clone(),
        ))),
        AuthorityStatus::Unknown => Ok(Some(fallback_disposition(
            &problem.fallback,
            problem,
            "physical possibility set is Unknown",
        ))),
        AuthorityStatus::Invalid => Ok(Some(invalid_or_fallback(
            &problem.fallback,
            problem,
            "physical possibility receipt is Invalid",
        ))),
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
        proposal.physical_possibility_digest.as_str(),
        result.solver_id.as_str(),
        result.solver_version.as_str(),
    ] {
        if !nonempty(id) {
            return Err(ProjectionError::EmptyIdentifier);
        }
    }
    if result.disposition == SolverDisposition::Feasible
        && !nonempty(&result.projected_command_digest)
    {
        return Err(ProjectionError::EmptyIdentifier);
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
    if result.physical_possibility_digest != proposal.physical_possibility_digest {
        return Err(ProjectionError::PhysicalPossibilityDigestMismatch);
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
    physical_possibility_ids: &[String],
) -> Result<
    (
        Vec<ConstraintFinding>,
        Vec<String>,
        Vec<String>,
        Vec<String>,
    ),
    ProjectionError,
> {
    if result.declared_constraints.is_empty() || frozen_inventory.is_empty() {
        return Err(ProjectionError::NoConstraintRequirements);
    }
    if result.declared_constraints.len() > MAX_CONSTRAINTS
        || frozen_inventory.len() > MAX_CONSTRAINTS
    {
        return Err(ProjectionError::TooManyConstraints);
    }
    if physical_possibility_ids.is_empty() {
        return Err(ProjectionError::NoPhysicalPossibilities);
    }
    if physical_possibility_ids.len() > MAX_POSSIBILITIES {
        return Err(ProjectionError::TooManyPossibilities);
    }

    let mut possibilities = std::collections::BTreeSet::new();
    for possibility_id in physical_possibility_ids {
        if !nonempty(possibility_id) {
            return Err(ProjectionError::EmptyIdentifier);
        }
        if !possibilities.insert(possibility_id.as_str()) {
            return Err(ProjectionError::DuplicatePhysicalPossibility);
        }
    }

    let mut frozen_by_id = BTreeMap::new();
    for requirement in frozen_inventory {
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
        if frozen_by_id
            .insert(requirement.id.as_str(), requirement)
            .is_some()
        {
            return Err(ProjectionError::DuplicateConstraintRequirement);
        }
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

    let expected_checks = possibilities
        .len()
        .checked_mul(requirements.len())
        .ok_or(ProjectionError::ArithmeticOverflow)?;
    if expected_checks > MAX_CONSTRAINT_CHECKS {
        return Err(ProjectionError::TooManyConstraintChecks);
    }

    let mut evidence_by_key = BTreeMap::new();
    for evidence in &result.constraint_evidence {
        if !nonempty(&evidence.possibility_id)
            || !nonempty(&evidence.id)
            || !nonempty(&evidence.residual_unit_id)
            || !nonempty(&evidence.margin_unit_id)
        {
            return Err(ProjectionError::EmptyIdentifier);
        }
        if !possibilities.contains(evidence.possibility_id.as_str()) {
            return Err(ProjectionError::UnknownPhysicalPossibility);
        }
        if !requirements.contains_key(evidence.id.as_str()) {
            return Err(ProjectionError::UnexpectedConstraintEvidence);
        }
        if evidence_by_key
            .insert(
                (evidence.possibility_id.as_str(), evidence.id.as_str()),
                evidence,
            )
            .is_some()
        {
            return Err(ProjectionError::DuplicateConstraintEvidence);
        }
    }
    if evidence_by_key.len() != expected_checks {
        return Err(ProjectionError::MissingConstraintEvidence);
    }

    let mut findings = Vec::with_capacity(expected_checks);
    let mut hard_failures = Vec::new();
    let mut contact_failures = Vec::new();
    let mut numerical_unqualified = Vec::new();

    for possibility_id in &possibilities {
        for (id, requirement) in &requirements {
            let evidence = evidence_by_key
                .get(&(*possibility_id, *id))
                .ok_or(ProjectionError::MissingConstraintEvidence)?;
            if evidence.residual_unit_id != requirement.residual_unit_id
                || evidence.margin_unit_id != requirement.margin_unit_id
            {
                return Err(ProjectionError::ConstraintUnitMismatch);
            }

            let residual_passed = evidence.residual <= requirement.residual_tolerance;
            let margin_passed = evidence.margin >= requirement.minimum_margin;
            let passed = residual_passed && margin_passed;
            let finding_id = format!("{possibility_id}:{id}");

            if requirement.required {
                match requirement.class {
                    ConstraintClass::HardPhysical => {
                        if !residual_passed {
                            numerical_unqualified.push(finding_id.clone());
                        }
                        if !margin_passed {
                            hard_failures.push(finding_id.clone());
                        }
                    }
                    ConstraintClass::RobustContact => {
                        if !residual_passed {
                            numerical_unqualified.push(finding_id.clone());
                        }
                        if !margin_passed {
                            contact_failures.push(finding_id.clone());
                        }
                    }
                    ConstraintClass::TaskTracking
                    | ConstraintClass::ProposalProximity
                    | ConstraintClass::Efficiency => {}
                }
            }

            findings.push(ConstraintFinding {
                possibility_id: (*possibility_id).to_owned(),
                id: (*id).to_owned(),
                class: requirement.class,
                residual: evidence.residual,
                residual_tolerance: requirement.residual_tolerance,
                residual_passed,
                margin: evidence.margin,
                minimum_margin: requirement.minimum_margin,
                margin_passed,
                passed,
            });
        }
    }

    findings.sort_by(|left, right| {
        left.possibility_id
            .cmp(&right.possibility_id)
            .then_with(|| left.id.cmp(&right.id))
    });
    hard_failures.sort();
    contact_failures.sort();
    numerical_unqualified.sort();
    Ok((
        findings,
        hard_failures,
        contact_failures,
        numerical_unqualified,
    ))
}

fn validate_independent_receipts(problem: &ProjectionGateProblem) -> Result<(), ProjectionError> {
    let proposal = &problem.proposal;
    let result = &problem.result;
    let viability = &problem.viability;
    let assurance = &problem.assurance;

    validate_authority_witness(viability.status, &viability.witness_or_reason)?;
    validate_authority_witness(assurance.status, &assurance.witness_or_reason)?;

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
    if viability.physical_possibility_digest != proposal.physical_possibility_digest
        || assurance.physical_possibility_digest != proposal.physical_possibility_digest
    {
        return Err(ProjectionError::PhysicalPossibilityDigestMismatch);
    }
    if assurance.abstraction_digest != proposal.abstraction_digest {
        return Err(ProjectionError::AbstractionIdentityMismatch);
    }
    if assurance.domain_digest != proposal.domain_digest {
        return Err(ProjectionError::DomainIdentityMismatch);
    }
    if assurance.constraint_inventory_digest != proposal.constraint_inventory_digest {
        return Err(ProjectionError::ConstraintInventoryMismatch);
    }
    if assurance.horizon_steps != proposal.horizon_steps {
        return Err(ProjectionError::InvalidReceipt);
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

/// Reconcile a command that the independent projection gate authorized.
/// A mismatch/unknown result blocks the next state-changing command until the
/// actuator outcome is independently reconciled.
pub fn reconcile_delivery(
    disposition: &GateDisposition,
    proposal: &PolicyProposal,
    receipt: &ActuatorRealizationReceipt,
    now_ns: u64,
) -> Result<DeliveryReconciliation, ProjectionError> {
    let authorized_digest = match disposition {
        GateDisposition::ExecuteProjected { command_digest }
        | GateDisposition::ExecuteFallback { command_digest, .. } => command_digest,
        GateDisposition::Rejected { .. }
        | GateDisposition::Unknown { .. }
        | GateDisposition::Invalid { .. } => {
            return Err(ProjectionError::UnexpectedExecutionReceipt);
        }
    };

    for id in [
        receipt.receipt_id.as_str(),
        receipt.result_digest.as_str(),
        receipt.executed_command_digest.as_str(),
        receipt.input_state_digest.as_str(),
        receipt.model_digest.as_str(),
        receipt.abstraction_digest.as_str(),
        receipt.domain_digest.as_str(),
        receipt.actuator_digest.as_str(),
    ] {
        if !nonempty(id) {
            return Err(ProjectionError::EmptyIdentifier);
        }
    }
    if receipt.executed_command_digest != *authorized_digest {
        return Err(ProjectionError::ExecutionReceiptMismatch);
    }
    if receipt.input_state_digest != proposal.input_state_digest {
        return Err(ProjectionError::StateIdentityMismatch);
    }
    if receipt.model_digest != proposal.model_digest {
        return Err(ProjectionError::ModelIdentityMismatch);
    }
    if receipt.abstraction_digest != proposal.abstraction_digest {
        return Err(ProjectionError::AbstractionIdentityMismatch);
    }
    if receipt.domain_digest != proposal.domain_digest {
        return Err(ProjectionError::DomainIdentityMismatch);
    }
    if receipt.actuator_digest != proposal.actuator_digest {
        return Err(ProjectionError::ActuatorIdentityMismatch);
    }
    if receipt.applied_at_ns != proposal.intended_apply_at_ns
        || receipt.observed_at_ns < receipt.applied_at_ns
        || receipt.observed_at_ns > now_ns
    {
        return Err(ProjectionError::InvalidSolverTiming);
    }
    validate_receipt_interval(
        receipt.valid_from_ns,
        receipt.valid_until_ns,
        receipt.observed_at_ns,
    )?;

    let (state_reconciled, command_realized_as_expected) = match receipt.status {
        RealizationStatus::Matched | RealizationStatus::Mismatch => {
            for digest in [
                receipt.measured_response_digest.as_deref(),
                receipt.resulting_state_digest.as_deref(),
                receipt.state_observation_digest.as_deref(),
            ] {
                if digest.is_none_or(|value| !nonempty(value)) {
                    return Err(ProjectionError::InvalidReceipt);
                }
            }
            (true, receipt.status == RealizationStatus::Matched)
        }
        RealizationStatus::Unknown | RealizationStatus::Invalid => (false, false),
    };

    let disposition = match receipt.status {
        RealizationStatus::Matched => {
            if receipt.witness_or_reason.is_some() {
                return Err(ProjectionError::InvalidReceipt);
            }
            RealizationDisposition::ConfirmedDelivered {
                result_digest: receipt.result_digest.clone(),
            }
        }
        RealizationStatus::Mismatch => {
            let witness = receipt
                .witness_or_reason
                .as_deref()
                .filter(|value| nonempty(value))
                .ok_or(ProjectionError::MissingStatusReason)?;
            RealizationDisposition::DeliveryMismatch {
                witness_id: witness.to_owned(),
            }
        }
        RealizationStatus::Unknown => {
            let reason = receipt
                .witness_or_reason
                .as_deref()
                .filter(|value| nonempty(value))
                .ok_or(ProjectionError::MissingStatusReason)?;
            RealizationDisposition::Unknown {
                reason: reason.to_owned(),
            }
        }
        RealizationStatus::Invalid => {
            let reason = receipt
                .witness_or_reason
                .as_deref()
                .filter(|value| nonempty(value))
                .ok_or(ProjectionError::MissingStatusReason)?;
            RealizationDisposition::Invalid {
                reason: reason.to_owned(),
            }
        }
    };
    Ok(DeliveryReconciliation {
        disposition,
        measured_response_digest: if state_reconciled {
            receipt.measured_response_digest.clone()
        } else {
            None
        },
        resulting_state_digest: if state_reconciled {
            receipt.resulting_state_digest.clone()
        } else {
            None
        },
        state_reconciled,
        command_realized_as_expected,
        fresh_proposal_and_viability_required: true,
    })
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
        numerically_unqualified: Vec::new(),
    };

    if let Some(disposition) = possibility_receipt_disposition(problem)? {
        report.disposition = disposition;
        return Ok(report);
    }

    match problem.result.disposition {
        SolverDisposition::Infeasible => {
            let witness = problem
                .result
                .infeasibility_witness_id
                .as_deref()
                .filter(|value| nonempty(value))
                .ok_or(ProjectionError::MissingInfeasibilityWitness)?;
            report.disposition = rejected_or_fallback(
                &problem.fallback,
                problem,
                "projection infeasible",
                Some(witness.to_owned()),
            );
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
            if problem
                .result
                .status_reason
                .as_deref()
                .is_none_or(|value| !nonempty(value))
            {
                return Err(ProjectionError::MissingStatusReason);
            }
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
                invalid_or_fallback(&problem.fallback, problem, "projection result invalid");
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

    let (findings, hard_failures, contact_failures, numerical_unqualified) = constraint_gate(
        &problem.result,
        &problem.required_constraint_inventory,
        &problem.physical_possibility_ids,
    )?;
    report.constraint_findings = findings;
    report.hard_constraint_failures = hard_failures;
    report.robust_contact_failures = contact_failures;
    report.numerically_unqualified = numerical_unqualified;

    if !report.numerically_unqualified.is_empty() {
        report.disposition = fallback_disposition(
            &problem.fallback,
            problem,
            "per-possibility constraint residual exceeds frozen tolerance",
        );
        return Ok(report);
    }
    if let Some(witness) = report.hard_constraint_failures.first().cloned() {
        report.disposition = rejected_or_fallback(
            &problem.fallback,
            problem,
            "hard physical constraint margin failed",
            Some(witness),
        );
        return Ok(report);
    }
    if let Some(witness) = report.robust_contact_failures.first().cloned() {
        report.disposition = rejected_or_fallback(
            &problem.fallback,
            problem,
            "robust contact constraint margin failed",
            Some(witness),
        );
        return Ok(report);
    }

    validate_independent_receipts(problem)?;
    match problem.viability.status {
        AuthorityStatus::Rejected => {
            let witness = problem
                .viability
                .witness_or_reason
                .as_ref()
                .expect("validated rejection witness")
                .clone();
            report.disposition = rejected_or_fallback(
                &problem.fallback,
                problem,
                "independent viability gate rejected projected command",
                Some(witness),
            );
            return Ok(report);
        }
        AuthorityStatus::Unknown => {
            report.disposition = fallback_disposition(
                &problem.fallback,
                problem,
                "independent viability result is Unknown",
            );
            return Ok(report);
        }
        AuthorityStatus::Invalid => {
            report.disposition =
                invalid_or_fallback(&problem.fallback, problem, "independent viability result is Invalid");
            return Ok(report);
        }
        AuthorityStatus::Admitted => {}
    }
    match problem.assurance.status {
        AuthorityStatus::Admitted => {}
        AuthorityStatus::Rejected => {
            let witness = problem
                .assurance
                .witness_or_reason
                .as_ref()
                .expect("validated assurance rejection witness")
                .clone();
            report.disposition = rejected_or_fallback(
                &problem.fallback,
                problem,
                "application-time assurance rejected projected command",
                Some(witness),
            );
            return Ok(report);
        }
        AuthorityStatus::Unknown => {
            report.disposition = fallback_disposition(
                &problem.fallback,
                problem,
                "application-time assurance is Unknown",
            );
            return Ok(report);
        }
        AuthorityStatus::Invalid => {
            report.disposition =
                invalid_or_fallback(&problem.fallback, problem, "application-time assurance is Invalid");
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
            physical_possibility_digest: "possibilities-v1".to_owned(),
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
            minimum_margin: 0,
            required: true,
        }
    }

    fn evidence(id: &str, residual: u64, margin: i64) -> ConstraintEvidence {
        ConstraintEvidence {
            possibility_id: "contact-mode-a".to_owned(),
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
            physical_possibility_receipt: PhysicalPossibilityReceipt {
                receipt_id: "possibility-receipt".to_owned(),
                result_digest: "possibility-result".to_owned(),
                possibility_set_digest: "possibilities-v1".to_owned(),
                possibility_ids: vec!["contact-mode-a".to_owned()],
                input_state_digest: "state-v1".to_owned(),
                model_digest: "model-v1".to_owned(),
                abstraction_digest: "abstraction-v1".to_owned(),
                domain_digest: "domain-v1".to_owned(),
                actuator_digest: "actuator-v1".to_owned(),
                horizon_steps: 10,
                complete: true,
                status: AuthorityStatus::Admitted,
                witness_or_reason: None,
                valid_from_ns: 150,
                valid_until_ns: 250,
            },
            proposal: proposal(),
            required_constraint_inventory: vec![
                requirement("torque-limit", ConstraintClass::HardPhysical),
                requirement("friction-cone", ConstraintClass::RobustContact),
                requirement("tracking", ConstraintClass::TaskTracking),
            ],
            physical_possibility_ids: vec!["contact-mode-a".to_owned()],
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
                physical_possibility_digest: "possibilities-v1".to_owned(),
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
                physical_possibility_digest: "possibilities-v1".to_owned(),
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
                abstraction_digest: "abstraction-v1".to_owned(),
                domain_digest: "domain-v1".to_owned(),
                actuator_digest: "actuator-v1".to_owned(),
                constraint_inventory_digest: "constraints-v1".to_owned(),
                physical_possibility_digest: "possibilities-v1".to_owned(),
                horizon_steps: 10,
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
            constraint_inventory_digest: "constraints-v1".to_owned(),
            physical_possibility_digest: "possibilities-v1".to_owned(),
            horizon_steps: 10,
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
        assert_eq!(
            report.disposition,
            GateDisposition::Rejected {
                reason: "hard physical constraint margin failed".to_owned(),
                witness_id: Some("contact-mode-a:torque-limit".to_owned()),
            }
        );
        assert_eq!(
            report.hard_constraint_failures,
            vec!["contact-mode-a:torque-limit".to_owned()]
        );
    }

    #[test]
    fn robust_contact_failure_prevents_execution() {
        let mut input = problem();
        input.result.constraint_evidence[1].margin = -1;
        let report = qualify_projected_action(&input).expect("contact gate");
        assert_eq!(
            report.disposition,
            GateDisposition::Rejected {
                reason: "robust contact constraint margin failed".to_owned(),
                witness_id: Some("contact-mode-a:friction-cone".to_owned()),
            }
        );
        assert_eq!(
            report.robust_contact_failures,
            vec!["contact-mode-a:friction-cone".to_owned()]
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
        assert_eq!(
            report.disposition,
            GateDisposition::Rejected {
                reason: "independent viability gate rejected projected command".to_owned(),
                witness_id: Some("no-recovery-state".to_owned()),
            }
        );
    }

    #[test]
    fn deadline_missed_projection_never_executes_stale_command() {
        let mut input = problem();
        input.now_ns = 205;
        input.proposal.intended_apply_at_ns = 205;
        input.proposal.deadline_ns = 205;
        input.result.deadline_ns = 205;
        input.result.disposition = SolverDisposition::DeadlineMiss;
        input.result.status_reason = Some("projection deadline exceeded".to_owned());
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

    fn evidence_for(
        possibility_id: &str,
        id: &str,
        residual: u64,
        margin: i64,
    ) -> ConstraintEvidence {
        ConstraintEvidence {
            possibility_id: possibility_id.to_owned(),
            id: id.to_owned(),
            residual_unit_id: "residual_nano".to_owned(),
            residual,
            margin,
            margin_unit_id: "margin_ppm".to_owned(),
        }
    }

    fn delivery_receipt(
        status: RealizationStatus,
        witness_or_reason: Option<&str>,
    ) -> ActuatorRealizationReceipt {
        let measured = matches!(
            status,
            RealizationStatus::Matched | RealizationStatus::Mismatch
        );
        ActuatorRealizationReceipt {
            receipt_id: "delivery-receipt".to_owned(),
            result_digest: "delivery-result".to_owned(),
            executed_command_digest: "command-v1".to_owned(),
            input_state_digest: "state-v1".to_owned(),
            measured_response_digest: measured.then(|| "measured-response-v1".to_owned()),
            resulting_state_digest: measured.then(|| "post-state-v1".to_owned()),
            state_observation_digest: measured.then(|| "post-state-observation-v1".to_owned()),
            model_digest: "model-v1".to_owned(),
            abstraction_digest: "abstraction-v1".to_owned(),
            domain_digest: "domain-v1".to_owned(),
            actuator_digest: "actuator-v1".to_owned(),
            status,
            witness_or_reason: witness_or_reason.map(str::to_owned),
            applied_at_ns: 200,
            observed_at_ns: 210,
            valid_from_ns: 200,
            valid_until_ns: 300,
        }
    }

    #[test]
    fn all_constraints_are_checked_for_every_physical_possibility() {
        let mut input = problem();
        input.physical_possibility_ids.push("contact-mode-b".to_owned());
        input
            .physical_possibility_receipt
            .possibility_ids
            .push("contact-mode-b".to_owned());
        input
            .physical_possibility_receipt
            .possibility_ids
            .push("contact-mode-b".to_owned());
        input
            .physical_possibility_receipt
            .possibility_ids
            .push("contact-mode-b".to_owned());
        for (id, margin) in [
            ("torque-limit", 90),
            ("friction-cone", 40),
            ("tracking", 0),
        ] {
            input
                .result
                .constraint_evidence
                .push(evidence_for("contact-mode-b", id, 1, margin));
        }
        let report = qualify_projected_action(&input).expect("complete branch matrix");
        assert!(matches!(
            report.disposition,
            GateDisposition::ExecuteProjected { .. }
        ));
        assert_eq!(report.constraint_findings.len(), 6);
    }

    #[test]
    fn one_missing_physical_possibility_constraint_row_is_rejected() {
        let mut input = problem();
        input.physical_possibility_ids.push("contact-mode-b".to_owned());
        input
            .result
            .constraint_evidence
            .push(evidence_for("contact-mode-b", "torque-limit", 1, 90));
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::MissingConstraintEvidence)
        );
    }

    #[test]
    fn robust_failure_in_one_possible_contact_mode_rejects_projection() {
        let mut input = problem();
        input.physical_possibility_ids.push("contact-mode-b".to_owned());
        input
            .result
            .constraint_evidence
            .push(evidence_for("contact-mode-b", "torque-limit", 1, 90));
        input
            .result
            .constraint_evidence
            .push(evidence_for("contact-mode-b", "friction-cone", 1, -1));
        input
            .result
            .constraint_evidence
            .push(evidence_for("contact-mode-b", "tracking", 20, 0));
        let report = qualify_projected_action(&input).expect("branch-specific contact failure");
        assert_eq!(
            report.disposition,
            GateDisposition::Rejected {
                reason: "robust contact constraint margin failed".to_owned(),
                witness_id: Some("contact-mode-b:friction-cone".to_owned()),
            }
        );
    }

    #[test]
    fn hard_constraint_margin_reserve_is_explicit_and_enforced() {
        let mut input = problem();
        input.required_constraint_inventory[0].minimum_margin = 50;
        input.result.declared_constraints[0].minimum_margin = 50;
        input.result.constraint_evidence[0].margin = 20;
        let report = qualify_projected_action(&input).expect("minimum reserve gate");
        assert!(matches!(report.disposition, GateDisposition::Rejected { .. }));
    }

    #[test]
    fn hard_constraint_residual_failure_is_unknown_not_physical_infeasibility() {
        let mut input = problem();
        input.result.constraint_evidence[0].residual = 11;
        let report = qualify_projected_action(&input).expect("unqualified residual");
        assert!(matches!(report.disposition, GateDisposition::Unknown { .. }));
        assert_eq!(
            report.numerically_unqualified,
            vec!["contact-mode-a:torque-limit".to_owned()]
        );
    }

    #[test]
    fn incomplete_physical_possibility_receipt_never_authorizes_projection() {
        let mut input = problem();
        input.physical_possibility_receipt.complete = false;
        let report = qualify_projected_action(&input).expect("incomplete set is unknown");
        assert!(matches!(report.disposition, GateDisposition::Unknown { .. }));
    }

    #[test]
    fn physical_possibility_receipt_must_match_exact_id_inventory() {
        let mut input = problem();
        input.physical_possibility_receipt.possibility_ids[0] = "other-contact-mode".to_owned();
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::PhysicalPossibilityReceiptMismatch)
        );
    }

    #[test]
    fn physical_possibility_receipt_must_be_fresh_at_application_time() {
        let mut input = problem();
        input.physical_possibility_receipt.valid_until_ns = 199;
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::StaleReceipt)
        );
    }

    #[test]
    fn duplicate_physical_possibility_id_is_invalid() {
        let mut input = problem();
        input
            .physical_possibility_ids
            .push("contact-mode-a".to_owned());
        assert_eq!(
            qualify_projected_action(&input),
            Err(ProjectionError::DuplicatePhysicalPossibility)
        );
    }

    #[test]
    fn matched_delivery_closes_cycle_but_requires_fresh_viability_for_next_action() {
        let report = qualify_projected_action(&problem()).expect("projection admission");
        let reconciliation = reconcile_delivery(
            &report.disposition,
            &problem().proposal,
            &delivery_receipt(RealizationStatus::Matched, None),
            220,
        )
        .expect("delivery reconciliation");
        assert_eq!(
            reconciliation.disposition,
            RealizationDisposition::ConfirmedDelivered {
                result_digest: "delivery-result".to_owned()
            }
        );
        assert!(reconciliation.state_reconciled);
        assert!(reconciliation.command_realized_as_expected);
        assert_eq!(
            reconciliation.resulting_state_digest.as_deref(),
            Some("post-state-v1")
        );
        assert!(reconciliation.fresh_proposal_and_viability_required);
    }

    #[test]
    fn actuator_delivery_mismatch_inhibits_subsequent_command() {
        let input = problem();
        let report = qualify_projected_action(&input).expect("projection admission");
        let reconciliation = reconcile_delivery(
            &report.disposition,
            &input.proposal,
            &delivery_receipt(
                RealizationStatus::Mismatch,
                Some("delivered torque outside actuator envelope"),
            ),
            220,
        )
        .expect("delivery mismatch evidence");
        assert!(matches!(
            reconciliation.disposition,
            RealizationDisposition::DeliveryMismatch { .. }
        ));
        assert!(reconciliation.state_reconciled);
        assert!(!reconciliation.command_realized_as_expected);
        assert!(reconciliation.resulting_state_digest.is_some());
        assert!(reconciliation.fresh_proposal_and_viability_required);
    }

    #[test]
    fn unknown_actuator_delivery_inhibits_subsequent_command() {
        let input = problem();
        let report = qualify_projected_action(&input).expect("projection admission");
        let reconciliation = reconcile_delivery(
            &report.disposition,
            &input.proposal,
            &delivery_receipt(
                RealizationStatus::Unknown,
                Some("actuator readback delayed"),
            ),
            220,
        )
        .expect("unknown delivery evidence");
        assert!(matches!(
            reconciliation.disposition,
            RealizationDisposition::Unknown { .. }
        ));
        assert!(!reconciliation.state_reconciled);
        assert!(!reconciliation.command_realized_as_expected);
        assert!(reconciliation.resulting_state_digest.is_none());
        assert!(reconciliation.fresh_proposal_and_viability_required);
    }

    #[test]
    fn execution_receipt_without_authorized_command_is_invalid() {
        let mut input = problem();
        input.viability.status = AuthorityStatus::Rejected;
        input.viability.witness_or_reason = Some("not-viable".to_owned());
        let report = qualify_projected_action(&input).expect("rejected projection");
        assert!(matches!(report.disposition, GateDisposition::Rejected { .. }));
        assert_eq!(
            reconcile_delivery(
                &report.disposition,
                &input.proposal,
                &delivery_receipt(RealizationStatus::Matched, None),
                220,
            ),
            Err(ProjectionError::UnexpectedExecutionReceipt)
        );
    }

    #[test]
    fn execution_receipt_must_bind_exact_command_digest() {
        let input = problem();
        let report = qualify_projected_action(&input).expect("projection admission");
        let mut receipt = delivery_receipt(RealizationStatus::Matched, None);
        receipt.executed_command_digest = "different-command".to_owned();
        assert_eq!(
            reconcile_delivery(&report.disposition, &input.proposal, &receipt, 220),
            Err(ProjectionError::ExecutionReceiptMismatch)
        );
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
