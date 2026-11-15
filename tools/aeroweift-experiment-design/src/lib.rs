//! Decision-relevant aviation validation experiment design.
//!
//! This selector is deliberately not a simulator, aircraft controller, authority system,
//! generic Bayesian optimizer or physics qualification corpus. It filters candidate
//! experiments by owner-supplied admissibility facts, then returns the nondominated
//! candidates across explicit benefit, uncertainty, relevance, ambiguity and cost axes.
//! No opaque weighted score is produced.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;

/// Physical or computational modality of a proposed validation experiment.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExperimentModality {
    /// Analytic or manufactured reference solution.
    AnalyticReference,
    /// Independent implementation comparison under matched semantics.
    CrossImplementation,
    /// High-fidelity or reduced-order simulation.
    Simulation,
    /// Software-in-the-loop.
    SoftwareInLoop,
    /// Hardware-in-the-loop.
    HardwareInLoop,
    /// Calibrated component or instrument bench experiment.
    ComponentBench,
    /// Controlled flight test, requiring separate external authorization.
    ControlledFlight,
    /// Shadow-mode replay against independently sourced observations.
    ShadowReplay,
}

/// The engineering question that the experiment is designed to resolve.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExperimentObjective {
    /// Distinguish competing model forms or independent model families.
    ModelDiscrimination,
    /// Identify parameters within a declared model family.
    ParameterIdentification,
    /// Resolve whether a prediction crosses a named engineering decision boundary.
    DecisionBoundaryResolution,
}

/// Authority state supplied by the responsible owner.
/// The selector does not authenticate this declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorizationState {
    /// No separate external authorization is required for this modality/campaign.
    NotRequired,
    /// The responsible authority has supplied an affirmative authorization receipt.
    Authorized,
    /// The responsible authority has explicitly not authorized the experiment.
    NotAuthorized,
    /// Authorization is missing or indeterminate.
    Unknown,
}

/// Whether model outputs have been shown to represent comparable semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticEquivalence {
    /// Independent owner has established matched semantics for the compared models.
    Established,
    /// The experiment does not compare implementations and does not require this gate.
    NotRequired,
    /// Semantic equivalence has not been established.
    Unknown,
    /// The candidate comparison is known to be semantically mismatched.
    Rejected,
}

/// Dataset role used when selecting the proposed experiment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionEvidenceRole {
    /// Adaptive development evidence; may inform the next experiment.
    AdaptiveDevelopment,
    /// Frozen confirmatory evidence; must not be inspected to select the experiment.
    ConfirmatorySealed,
    /// Untouched challenge evidence; must not be inspected to select the experiment.
    ChallengeSealed,
}

/// Independent candidate model-family/correlation group.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HypothesisGroup {
    /// Stable model-family or genuinely independent hypothesis-group identity.
    pub group_id: String,
}

/// Candidate test/experiment and its expected decision-relevant utility vector.
#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentCandidate {
    /// Stable candidate identity.
    pub candidate_id: String,
    /// Engineering decision/question this experiment is intended to resolve.
    pub decision_question_id: String,
    /// Type of epistemic objective.
    pub objective: ExperimentObjective,
    /// Experiment modality.
    pub modality: ExperimentModality,
    /// Whether separate external authorization is required.
    pub requires_external_authorization: bool,
    /// Owner-supplied authorization state.
    pub authorization: AuthorizationState,
    /// Whether all physical and procedural constraints are clear for this candidate.
    pub safety_constraints_clear: bool,
    /// Whether the candidate lies within the currently approved test envelope.
    pub within_declared_envelope: bool,
    /// Whether a genuinely independent reference measurement/oracle is available.
    pub independent_reference_available: bool,
    /// Semantic-equivalence status when implementations are compared.
    pub semantic_equivalence: SemanticEquivalence,
    /// Which data partition was inspected to select this experiment.
    pub selection_evidence_role: SelectionEvidenceRole,
    /// Independent model-family groups actually distinguished by this experiment.
    pub hypothesis_groups: Vec<HypothesisGroup>,
    /// Expected decision-relevant predictive uncertainty reduction, normalized [0,1].
    pub expected_uncertainty_reduction: f64,
    /// Expected discrimination among the declared model hypotheses, normalized [0,1].
    pub expected_model_discrimination: f64,
    /// Worst-case reduction under the frozen robust uncertainty set, normalized [0,1].
    pub worst_case_robust_reduction: f64,
    /// Relevance to the declared engineering decision, normalized [0,1].
    pub decision_relevance: f64,
    /// Probability the result is expected to be uninformative, normalized [0,1].
    pub probability_uninformative: f64,
    /// Cost in the policy's declared integer cost units.
    pub cost_units: u64,
}

/// Frozen selection policy, including domain-owned thresholds and resource budget.
#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentDesignPolicy {
    /// Stable question identity.
    pub decision_question_id: String,
    /// Objective to be resolved.
    pub objective: ExperimentObjective,
    /// Frozen policy manifest digest; syntax checked only.
    pub policy_sha256: String,
    /// Maximum total cost available for one selected experiment.
    pub maximum_cost_units: u64,
    /// Minimum decision relevance required for admission.
    pub minimum_decision_relevance: f64,
    /// Minimum robust uncertainty reduction required for admission.
    pub minimum_worst_case_robust_reduction: f64,
    /// Minimum model discrimination required for ModelDiscrimination objectives.
    pub minimum_model_discrimination: f64,
}

/// Why a candidate was excluded before Pareto ranking.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RejectionReason {
    /// Candidate identity or question does not match the policy.
    QuestionOrObjectiveMismatch,
    /// Required external authorization was not affirmatively supplied.
    AuthorizationNotGranted,
    /// Owner-supplied physical/procedural safety constraint is not clear.
    SafetyConstraintNotClear,
    /// Candidate is outside its declared approved test envelope.
    OutsideDeclaredEnvelope,
    /// No independently qualified reference measurement/oracle is available.
    MissingIndependentReference,
    /// Cross-implementation semantics are not established or are rejected.
    SemanticEquivalenceNotEstablished,
    /// Sealed confirmatory/challenge evidence was inspected during selection.
    SealedEvidenceUsedForSelection,
    /// Too few independent model-family groups are distinguished.
    InsufficientIndependentHypothesisGroups,
    /// Candidate utility estimate is malformed or non-finite.
    InvalidUtilityVector,
    /// Decision relevance is below the frozen policy threshold.
    InsufficientDecisionRelevance,
    /// Robust uncertainty reduction is below the frozen policy threshold.
    InsufficientRobustReduction,
    /// Model-discrimination objective is below the frozen policy threshold.
    InsufficientModelDiscrimination,
    /// Candidate exceeds the frozen resource budget.
    OverBudget,
}

/// Rejected candidate and all reasons it was excluded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RejectedCandidate {
    /// Candidate identity.
    pub candidate_id: String,
    /// Reasons for rejection, canonically sorted and deduplicated.
    pub reasons: Vec<RejectionReason>,
}

/// Candidate on the Pareto frontier; dimensions remain separately inspectable.
#[derive(Clone, Debug, PartialEq)]
pub struct ParetoCandidate {
    /// Exact candidate identity.
    pub candidate_id: String,
    /// Expected decision-relevant uncertainty reduction.
    pub expected_uncertainty_reduction: f64,
    /// Expected discrimination among independent model hypotheses.
    pub expected_model_discrimination: f64,
    /// Worst-case robust uncertainty reduction.
    pub worst_case_robust_reduction: f64,
    /// Decision relevance.
    pub decision_relevance: f64,
    /// Probability of an uninformative outcome.
    pub probability_uninformative: f64,
    /// Resource cost.
    pub cost_units: u64,
}

/// Deterministic Pareto selection result.
#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentDesignResult {
    /// Nondominated admissible candidates, sorted by stable candidate identity.
    pub pareto_front: Vec<ParetoCandidate>,
    /// Candidates excluded before ranking, sorted by stable candidate identity.
    pub rejected: Vec<RejectedCandidate>,
    /// Whether the candidate set contained any admissible experiment.
    pub disposition: DesignDisposition,
    /// Digest of the frozen policy used to produce this result.
    pub policy_sha256: String,
}

/// Overall selection disposition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesignDisposition {
    /// One or more admissible candidates remain on the Pareto frontier.
    ParetoFrontAvailable,
    /// No admissible experiment exists under the supplied policy and candidate set.
    AbstainNoAdmissibleExperiment,
}

/// Typed policy/structural input failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesignError {
    /// Frozen policy identity, objective or thresholds are malformed.
    InvalidPolicy,
    /// Candidate identities are duplicated.
    DuplicateCandidateId,
}

/// Check digest syntax only; cryptographic verification is external.
fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn bounded_unit_interval(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn validate_policy(policy: &ExperimentDesignPolicy) -> Result<(), DesignError> {
    if policy.decision_question_id.trim().is_empty()
        || !is_sha256(&policy.policy_sha256)
        || policy.maximum_cost_units == 0
        || !bounded_unit_interval(policy.minimum_decision_relevance)
        || !bounded_unit_interval(policy.minimum_worst_case_robust_reduction)
        || !bounded_unit_interval(policy.minimum_model_discrimination)
    {
        return Err(DesignError::InvalidPolicy);
    }
    Ok(())
}

fn independent_group_count(candidate: &ExperimentCandidate) -> usize {
    candidate
        .hypothesis_groups
        .iter()
        .map(|group| group.group_id.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn candidate_rejection_reasons(
    candidate: &ExperimentCandidate,
    policy: &ExperimentDesignPolicy,
) -> Vec<RejectionReason> {
    let mut reasons = BTreeSet::new();

    if candidate.candidate_id.trim().is_empty()
        || candidate.decision_question_id != policy.decision_question_id
        || candidate.objective != policy.objective
    {
        reasons.insert(RejectionReason::QuestionOrObjectiveMismatch);
    }
    if candidate.requires_external_authorization
        && candidate.authorization != AuthorizationState::Authorized
    {
        reasons.insert(RejectionReason::AuthorizationNotGranted);
    }
    if !candidate.safety_constraints_clear {
        reasons.insert(RejectionReason::SafetyConstraintNotClear);
    }
    if !candidate.within_declared_envelope {
        reasons.insert(RejectionReason::OutsideDeclaredEnvelope);
    }
    if !candidate.independent_reference_available {
        reasons.insert(RejectionReason::MissingIndependentReference);
    }
    if candidate.modality == ExperimentModality::CrossImplementation
        && candidate.semantic_equivalence != SemanticEquivalence::Established
    {
        reasons.insert(RejectionReason::SemanticEquivalenceNotEstablished);
    }
    if candidate.selection_evidence_role != SelectionEvidenceRole::AdaptiveDevelopment {
        reasons.insert(RejectionReason::SealedEvidenceUsedForSelection);
    }
    if candidate.objective == ExperimentObjective::ModelDiscrimination
        && independent_group_count(candidate) < 2
    {
        reasons.insert(RejectionReason::InsufficientIndependentHypothesisGroups);
    }

    if !bounded_unit_interval(candidate.expected_uncertainty_reduction)
        || !bounded_unit_interval(candidate.expected_model_discrimination)
        || !bounded_unit_interval(candidate.worst_case_robust_reduction)
        || !bounded_unit_interval(candidate.decision_relevance)
        || !bounded_unit_interval(candidate.probability_uninformative)
        || candidate.cost_units == 0
    {
        reasons.insert(RejectionReason::InvalidUtilityVector);
    }
    if candidate.decision_relevance < policy.minimum_decision_relevance {
        reasons.insert(RejectionReason::InsufficientDecisionRelevance);
    }
    if candidate.worst_case_robust_reduction < policy.minimum_worst_case_robust_reduction {
        reasons.insert(RejectionReason::InsufficientRobustReduction);
    }
    if candidate.objective == ExperimentObjective::ModelDiscrimination
        && candidate.expected_model_discrimination < policy.minimum_model_discrimination
    {
        reasons.insert(RejectionReason::InsufficientModelDiscrimination);
    }
    if candidate.cost_units > policy.maximum_cost_units {
        reasons.insert(RejectionReason::OverBudget);
    }

    reasons.into_iter().collect()
}

/// Return true only when the left candidate Pareto-dominates the right candidate.
///
/// Benefits are maximized, uninformative probability and cost are minimized.
/// No scalar weighting or tie-breaking on different objective dimensions occurs.
fn dominates(left: &ExperimentCandidate, right: &ExperimentCandidate) -> bool {
    let no_worse = left.expected_uncertainty_reduction >= right.expected_uncertainty_reduction
        && left.expected_model_discrimination >= right.expected_model_discrimination
        && left.worst_case_robust_reduction >= right.worst_case_robust_reduction
        && left.decision_relevance >= right.decision_relevance
        && left.probability_uninformative <= right.probability_uninformative
        && left.cost_units <= right.cost_units;

    let strictly_better = left.expected_uncertainty_reduction > right.expected_uncertainty_reduction
        || left.expected_model_discrimination > right.expected_model_discrimination
        || left.worst_case_robust_reduction > right.worst_case_robust_reduction
        || left.decision_relevance > right.decision_relevance
        || left.probability_uninformative < right.probability_uninformative
        || left.cost_units < right.cost_units;

    no_worse && strictly_better
}

fn to_pareto(candidate: &ExperimentCandidate) -> ParetoCandidate {
    ParetoCandidate {
        candidate_id: candidate.candidate_id.clone(),
        expected_uncertainty_reduction: candidate.expected_uncertainty_reduction,
        expected_model_discrimination: candidate.expected_model_discrimination,
        worst_case_robust_reduction: candidate.worst_case_robust_reduction,
        decision_relevance: candidate.decision_relevance,
        probability_uninformative: candidate.probability_uninformative,
        cost_units: candidate.cost_units,
    }
}

/// Select the nondominated admissible candidate set under one frozen aviation policy.
///
/// The selector never runs an experiment and never authorizes an experiment. Safety,
/// envelope, authority, semantic equivalence and reference evidence are supplied facts
/// that must be established by the respective owners before this function is called.
pub fn select_experiments(
    candidates: &[ExperimentCandidate],
    policy: &ExperimentDesignPolicy,
) -> Result<ExperimentDesignResult, DesignError> {
    validate_policy(policy)?;

    let mut seen = BTreeSet::new();
    for candidate in candidates {
        if !seen.insert(candidate.candidate_id.as_str()) {
            return Err(DesignError::DuplicateCandidateId);
        }
    }

    let mut admissible: Vec<&ExperimentCandidate> = Vec::new();
    let mut rejected: Vec<RejectedCandidate> = Vec::new();

    for candidate in candidates {
        let reasons = candidate_rejection_reasons(candidate, policy);
        if reasons.is_empty() {
            admissible.push(candidate);
        } else {
            rejected.push(RejectedCandidate {
                candidate_id: candidate.candidate_id.clone(),
                reasons,
            });
        }
    }

    let front: Vec<&ExperimentCandidate> = admissible
        .iter()
        .copied()
        .filter(|candidate| {
            !admissible
                .iter()
                .copied()
                .any(|other| other.candidate_id != candidate.candidate_id && dominates(other, candidate))
        })
        .collect();

    let mut pareto_front: Vec<ParetoCandidate> =
        front.into_iter().map(to_pareto).collect();
    pareto_front.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    rejected.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    let disposition = if pareto_front.is_empty() {
        DesignDisposition::AbstainNoAdmissibleExperiment
    } else {
        DesignDisposition::ParetoFrontAvailable
    };

    Ok(ExperimentDesignResult {
        pareto_front,
        rejected,
        disposition,
        policy_sha256: policy.policy_sha256.clone(),
    })
}
