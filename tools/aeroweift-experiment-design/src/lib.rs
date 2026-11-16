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

/// Owner evidence role used by an experiment candidate's admissibility gates.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GateEvidenceRole {
    /// External authorization for a test modality/campaign.
    Authorization,
    /// Affirmative safety and procedural clearance.
    Safety,
    /// Affirmative membership in the declared test envelope.
    Envelope,
    /// Availability and qualification of the independent reference measurement.
    IndependentReference,
    /// Semantic equivalence for cross-implementation comparison.
    SemanticEquivalence,
}

/// Structurally scoped reference to an owner-provided gate receipt.
///
/// Digest syntax and exact scope binding are checked here. Signature authenticity,
/// issuer authority, revocation and evidence truth remain with Track Q / Mycelix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateEvidenceReceipt {
    /// Stable receipt identifier.
    pub receipt_id: String,
    /// Digest of the exact receipt payload.
    pub payload_sha256: String,
    /// Owner namespace that issued the receipt; not authenticated here.
    pub issuer_id: String,
    /// Admissibility gate this receipt is allowed to satisfy.
    pub role: GateEvidenceRole,
    /// Exact campaign digest.
    pub campaign_sha256: String,
    /// Exact model/sensor/control scope digest.
    pub model_scope_sha256: String,
    /// Exact intended-use/risk profile digest.
    pub intended_use_profile_sha256: String,
    /// Exact decision/question identity.
    pub decision_question_id: String,
    /// Exact experiment design digest.
    pub design_sha256: String,
    /// Exact scenario/initial-condition manifest digest.
    pub scenario_manifest_sha256: String,
    /// Exact measurement/reference protocol digest.
    pub measurement_plan_sha256: String,
    /// Exact selection policy manifest digest.
    pub policy_sha256: String,
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
    /// Stable exact hypothesis identity.
    pub hypothesis_id: String,
    /// Digest of the exact model variant/parameterization being tested.
    pub hypothesis_sha256: String,
    /// Stable model-family or genuinely independent hypothesis-group identity.
    pub group_id: String,
}

/// Explicit pairwise prediction contrast between two hypothesis variants.
#[derive(Clone, Debug, PartialEq)]
pub struct HypothesisContrast {
    /// First exact hypothesis identity.
    pub left_hypothesis_id: String,
    /// Second exact hypothesis identity.
    pub right_hypothesis_id: String,
    /// Conservative lower bound on outcome separation under this experiment.
    pub separation_lower_bound: f64,
    /// Digest of the frozen contrast estimate and its assumptions.
    pub contrast_estimate_sha256: String,
}

/// Independent sampling unit used to estimate the candidate's utility vector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EstimateSampleUnit {
    /// Distinct controlled flights.
    Flight,
    /// Distinct encounter events.
    Encounter,
    /// Distinct trajectory segments.
    TrajectorySegment,
    /// Distinct independently measured experiment windows.
    MeasurementWindow,
    /// Distinct independent model-family contrasts.
    ModelFamilyPair,
}

/// Scope-bound owner receipt for the expected-utility estimates used in selection.
///
/// The payload digest, signer identity and issuer authority are not verified by this
/// crate. Track Q / Mycelix must authenticate the actual receipt before operational use.
#[derive(Clone, Debug, PartialEq)]
pub struct UtilityEstimateReceipt {
    /// Stable receipt identifier.
    pub receipt_id: String,
    /// Digest of the exact receipt payload.
    pub payload_sha256: String,
    /// Owner namespace that issued the estimate receipt; not authenticated here.
    pub issuer_id: String,
    /// Exact candidate identity; prevents receipt replay across candidate alternatives.
    pub candidate_id: String,
    /// Exact campaign digest.
    pub campaign_sha256: String,
    /// Exact model/sensor/control scope digest.
    pub model_scope_sha256: String,
    /// Exact intended-use/risk profile digest.
    pub intended_use_profile_sha256: String,
    /// Exact decision/question identity.
    pub decision_question_id: String,
    /// Exact experiment design digest.
    pub design_sha256: String,
    /// Exact scenario/initial-condition manifest digest.
    pub scenario_manifest_sha256: String,
    /// Exact measurement/reference protocol digest.
    pub measurement_plan_sha256: String,
    /// Exact decision boundary or loss-function profile being optimized.
    pub decision_boundary_sha256: String,
    /// Exact candidate hypothesis-set manifest.
    pub hypothesis_manifest_sha256: String,
    /// Exact robust/model-ambiguity set used for worst-case estimation.
    pub robust_uncertainty_set_sha256: String,
    /// Exact adaptive-development dataset manifest used by the estimator.
    pub adaptive_data_manifest_sha256: String,
    /// Exact estimator build digest.
    pub estimator_build_sha256: String,
    /// Exact estimate-method digest.
    pub estimate_method_sha256: String,
    /// Confidence level for all conservative estimate bounds in this receipt.
    pub confidence_level: f64,
    /// Independent sampling unit represented by the estimate denominator.
    pub sample_unit: EstimateSampleUnit,
    /// Number of independent units, not raw correlated rows.
    pub independent_unit_count: u64,
    /// Exact hypothesis variants included in the estimator's model comparison.
    pub hypothesis_groups: Vec<HypothesisGroup>,
    /// Exact pairwise model contrasts included in the estimator's model comparison.
    pub hypothesis_contrasts: Vec<HypothesisContrast>,
    /// Conservative lower bound on decision-relevant predictive uncertainty reduction.
    pub expected_uncertainty_reduction: f64,
    /// Conservative lower bound on discrimination among the declared model hypotheses.
    pub expected_model_discrimination: f64,
    /// Conservative lower bound under the frozen robust uncertainty set.
    pub worst_case_robust_reduction: f64,
    /// Conservative lower bound on decision relevance.
    pub decision_relevance: f64,
    /// Upper bound on probability the result is uninformative.
    pub probability_uninformative: f64,
}

/// Candidate test/experiment and its expected decision-relevant utility vector.
#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentCandidate {
    /// Stable candidate identity.
    pub candidate_id: String,
    /// Exact campaign digest.
    pub campaign_sha256: String,
    /// Exact model/sensor/control scope digest.
    pub model_scope_sha256: String,
    /// Exact intended-use/risk profile digest.
    pub intended_use_profile_sha256: String,
    /// Exact experiment design digest.
    pub design_sha256: String,
    /// Exact scenario/initial-condition manifest digest.
    pub scenario_manifest_sha256: String,
    /// Exact measurement/reference protocol digest.
    pub measurement_plan_sha256: String,
    /// Exact decision boundary or loss-function profile being optimized.
    pub decision_boundary_sha256: String,
    /// Exact candidate hypothesis-set manifest.
    pub hypothesis_manifest_sha256: String,
    /// Exact robust/model-ambiguity set used for worst-case estimation.
    pub robust_uncertainty_set_sha256: String,
    /// Exact adaptive-development dataset manifest used by the estimator.
    pub adaptive_data_manifest_sha256: String,
    /// Structured estimate receipt binding every utility value to this exact candidate.
    pub utility_estimate_receipt: Option<UtilityEstimateReceipt>,
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
    /// Owner receipt for affirmative authorization, where required.
    pub authorization_receipt_sha256: Option<String>,
    /// Structurally scope-bound owner gate receipts; authentication is external.
    pub gate_evidence_receipts: Vec<GateEvidenceReceipt>,
    /// Whether all physical and procedural constraints are clear for this candidate.
    pub safety_constraints_clear: bool,
    /// Receipt proving the affirmative safety/procedure gate.
    pub safety_gate_receipt_sha256: Option<String>,
    /// Whether the candidate lies within the currently approved test envelope.
    pub within_declared_envelope: bool,
    /// Receipt proving the affirmative envelope gate.
    pub envelope_receipt_sha256: Option<String>,
    /// Whether a genuinely independent reference measurement/oracle is available.
    pub independent_reference_available: bool,
    /// Receipt proving reference availability and qualification.
    pub independent_reference_receipt_sha256: Option<String>,
    /// Semantic-equivalence status when implementations are compared.
    pub semantic_equivalence: SemanticEquivalence,
    /// Receipt proving semantic equivalence when required.
    pub semantic_equivalence_receipt_sha256: Option<String>,
    /// Which data partition was inspected to select this experiment.
    pub selection_evidence_role: SelectionEvidenceRole,
    /// Exact hypothesis variants considered by this experiment.
    pub hypothesis_groups: Vec<HypothesisGroup>,
    /// Explicit contrasts demonstrating which cross-family pairs may be distinguished.
    pub hypothesis_contrasts: Vec<HypothesisContrast>,
    /// Conservative lower bound on decision-relevant predictive uncertainty reduction, normalized [0,1].
    pub expected_uncertainty_reduction: f64,
    /// Conservative lower bound on discrimination among the declared model hypotheses, normalized [0,1].
    pub expected_model_discrimination: f64,
    /// Conservative lower bound under the frozen robust uncertainty set, normalized [0,1].
    pub worst_case_robust_reduction: f64,
    /// Conservative lower bound on decision relevance, normalized [0,1].
    pub decision_relevance: f64,
    /// Upper bound on probability the result is uninformative, normalized [0,1].
    pub probability_uninformative: f64,
    /// Confidence level supporting conservative utility estimates.
    pub estimate_confidence_level: f64,
    /// Frozen estimation-method digest.
    pub estimate_method_sha256: String,
    /// Exact utility-estimator build digest.
    pub estimator_build_sha256: String,
    /// Wall-clock elapsed minutes in the declared planning environment.
    pub cost_units: u64,
    /// Non-aggregated resource costs; units remain separate.
    pub resource_cost: ResourceCost,
}

/// Independent resource dimensions. Values are not added or exchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceCost {
    /// Setup/preparation minutes.
    pub setup_minutes: u64,
    /// Facility/operator minutes.
    pub facility_minutes: u64,
    /// CPU compute seconds.
    pub cpu_seconds: u64,
    /// Accelerator/GPU compute seconds.
    pub accelerator_seconds: u64,
    /// Energy joules.
    pub energy_joules: u64,
    /// Adjudication/review person-minutes.
    pub adjudication_person_minutes: u64,
}

/// Independent resource budgets; dimensions cannot compensate for one another.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceBudget {
    /// Maximum setup/preparation minutes.
    pub setup_minutes: u64,
    /// Maximum facility/operator minutes.
    pub facility_minutes: u64,
    /// Maximum CPU compute seconds.
    pub cpu_seconds: u64,
    /// Maximum accelerator/GPU compute seconds.
    pub accelerator_seconds: u64,
    /// Maximum energy joules.
    pub energy_joules: u64,
    /// Maximum adjudication/review person-minutes.
    pub adjudication_person_minutes: u64,
}

impl ResourceCost {
    fn within(self, budget: ResourceBudget) -> bool {
        self.setup_minutes <= budget.setup_minutes
            && self.facility_minutes <= budget.facility_minutes
            && self.cpu_seconds <= budget.cpu_seconds
            && self.accelerator_seconds <= budget.accelerator_seconds
            && self.energy_joules <= budget.energy_joules
            && self.adjudication_person_minutes <= budget.adjudication_person_minutes
    }
}

/// Frozen selection policy, including domain-owned thresholds and resource budget.
#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentDesignPolicy {
    /// Exact campaign digest.
    pub campaign_sha256: String,
    /// Exact model/sensor/control scope digest.
    pub model_scope_sha256: String,
    /// Exact intended-use/risk profile digest.
    pub intended_use_profile_sha256: String,
    /// Exact decision boundary or loss-function profile required for this campaign.
    pub decision_boundary_sha256: String,
    /// Exact candidate hypothesis-set manifest required for this campaign.
    pub hypothesis_manifest_sha256: String,
    /// Exact robust/model-ambiguity set required for this campaign.
    pub robust_uncertainty_set_sha256: String,
    /// Exact adaptive-development dataset manifest required for this campaign.
    pub adaptive_data_manifest_sha256: String,
    /// Minimum independent sampling units required for utility estimates.
    pub minimum_estimate_independent_units: u64,
    /// Independent sampling unit required by the frozen estimator policy.
    pub required_estimate_sample_unit: EstimateSampleUnit,
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
    /// Minimum per-pair conservative outcome separation.
    pub minimum_pair_separation: f64,
    /// Minimum number of distinct model-family pairs with qualified contrast estimates.
    pub minimum_distinct_hypothesis_group_pairs: usize,
    /// Maximum acceptable upper bound on an uninformative result.
    pub maximum_probability_uninformative: f64,
    /// Minimum confidence level for utility bounds.
    pub minimum_estimate_confidence_level: f64,
    /// Exact estimator build required by the frozen policy.
    pub required_estimator_build_sha256: String,
    /// Exact estimate method required by the frozen policy.
    pub required_estimate_method_sha256: String,
    /// Modalities that require affirmative external authorization under this campaign.
    pub authorization_required_modalities: Vec<ExperimentModality>,
    /// Per-dimension non-substitutable resource budget.
    pub resource_budget: ResourceBudget,
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
    /// Explicit pairwise contrast evidence is missing, duplicated or malformed.
    InvalidHypothesisContrast,
    /// Too few distinct model-family pairs meet the contrast-separation floor.
    InsufficientQualifiedHypothesisContrasts,
    /// Candidate utility estimate is malformed or non-finite.
    InvalidUtilityVector,
    /// A positive gate is missing its owner evidence receipt.
    MissingEvidenceReceipt,
    /// Scope/design/manifest identity is malformed or mismatched.
    InvalidEvidenceBinding,
    /// Estimator build or confidence method differs from the frozen policy.
    ConfidenceContractMismatch,
    /// No structured scope-bound utility-estimate receipt was supplied.
    MissingUtilityEstimateReceipt,
    /// Utility-estimate receipt was replayed, malformed or bound to another experiment.
    InvalidUtilityEstimateReceipt,
    /// Utility estimate was derived from too few independent samples.
    InsufficientEstimateSamples,
    /// Decision relevance is below the frozen policy threshold.
    InsufficientDecisionRelevance,
    /// Robust uncertainty reduction is below the frozen policy threshold.
    InsufficientRobustReduction,
    /// Model-discrimination objective is below the frozen policy threshold.
    InsufficientModelDiscrimination,
    /// Candidate exceeds the frozen resource budget.
    OverBudget,
    /// Candidate is too likely to produce an uninformative result.
    TooLikelyUninformative,
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
    /// Number of distinct model-family pairs with admitted contrast evidence.
    pub independent_hypothesis_group_pair_count: usize,
    /// Worst-case robust uncertainty reduction.
    pub worst_case_robust_reduction: f64,
    /// Decision relevance.
    pub decision_relevance: f64,
    /// Probability of an uninformative outcome.
    pub probability_uninformative: f64,
    /// Wall-clock elapsed minutes.
    pub cost_units: u64,
    /// Non-aggregated resource-cost vector.
    pub resource_cost: ResourceCost,
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
    /// One utility-estimate receipt ID is reused across candidate alternatives.
    DuplicateUtilityEstimateReceiptId,
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
        || !is_sha256(&policy.campaign_sha256)
        || !is_sha256(&policy.model_scope_sha256)
        || !is_sha256(&policy.intended_use_profile_sha256)
        || !is_sha256(&policy.decision_boundary_sha256)
        || !is_sha256(&policy.hypothesis_manifest_sha256)
        || !is_sha256(&policy.robust_uncertainty_set_sha256)
        || !is_sha256(&policy.adaptive_data_manifest_sha256)
        || policy.minimum_estimate_independent_units == 0
        || !is_sha256(&policy.required_estimator_build_sha256)
        || !is_sha256(&policy.required_estimate_method_sha256)
        || !bounded_unit_interval(policy.minimum_estimate_confidence_level)
        || policy.minimum_estimate_confidence_level == 0.0
        || policy.minimum_estimate_confidence_level == 1.0
        || policy
            .authorization_required_modalities
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != policy.authorization_required_modalities.len()
        || !is_sha256(&policy.policy_sha256)
        || policy.maximum_cost_units == 0
        || !bounded_unit_interval(policy.minimum_decision_relevance)
        || !bounded_unit_interval(policy.minimum_worst_case_robust_reduction)
        || !bounded_unit_interval(policy.minimum_model_discrimination)
        || !bounded_unit_interval(policy.minimum_pair_separation)
        || (policy.objective == ExperimentObjective::ModelDiscrimination
            && policy.minimum_distinct_hypothesis_group_pairs == 0)
        || !bounded_unit_interval(policy.maximum_probability_uninformative)
    {
        return Err(DesignError::InvalidPolicy);
    }
    Ok(())
}

fn gate_receipt_matches_scope(
    receipt: &GateEvidenceReceipt,
    candidate: &ExperimentCandidate,
    policy: &ExperimentDesignPolicy,
) -> bool {
    !receipt.receipt_id.trim().is_empty()
        && !receipt.issuer_id.trim().is_empty()
        && is_sha256(&receipt.payload_sha256)
        && receipt.campaign_sha256 == candidate.campaign_sha256
        && receipt.model_scope_sha256 == candidate.model_scope_sha256
        && receipt.intended_use_profile_sha256 == candidate.intended_use_profile_sha256
        && receipt.decision_question_id == candidate.decision_question_id
        && receipt.design_sha256 == candidate.design_sha256
        && receipt.scenario_manifest_sha256 == candidate.scenario_manifest_sha256
        && receipt.measurement_plan_sha256 == candidate.measurement_plan_sha256
        && receipt.policy_sha256 == policy.policy_sha256
}

fn gate_receipt_payload_matches(
    candidate: &ExperimentCandidate,
    role: GateEvidenceRole,
    payload_sha256: &str,
) -> bool {
    let expected = match role {
        GateEvidenceRole::Authorization => candidate.authorization_receipt_sha256.as_deref(),
        GateEvidenceRole::Safety => candidate.safety_gate_receipt_sha256.as_deref(),
        GateEvidenceRole::Envelope => candidate.envelope_receipt_sha256.as_deref(),
        GateEvidenceRole::IndependentReference => {
            candidate.independent_reference_receipt_sha256.as_deref()
        }
        GateEvidenceRole::SemanticEquivalence => {
            candidate.semantic_equivalence_receipt_sha256.as_deref()
        }
    };
    expected.is_none_or(|expected_digest| expected_digest == payload_sha256)
}

fn validate_gate_evidence_receipts(
    candidate: &ExperimentCandidate,
    policy: &ExperimentDesignPolicy,
    reasons: &mut BTreeSet<RejectionReason>,
) {
    let mut receipt_ids = BTreeSet::new();
    let mut roles = BTreeSet::new();

    for receipt in &candidate.gate_evidence_receipts {
        if !receipt_ids.insert(receipt.receipt_id.as_str()) || !roles.insert(receipt.role) {
            reasons.insert(RejectionReason::InvalidEvidenceBinding);
        }
        if !gate_receipt_matches_scope(receipt, candidate, policy)
            || !gate_receipt_payload_matches(candidate, receipt.role, &receipt.payload_sha256)
        {
            reasons.insert(RejectionReason::InvalidEvidenceBinding);
        }
    }

    let required_roles = [
        (
            GateEvidenceRole::Authorization,
            candidate.authorization == AuthorizationState::Authorized,
        ),
        (GateEvidenceRole::Safety, candidate.safety_constraints_clear),
        (
            GateEvidenceRole::Envelope,
            candidate.within_declared_envelope,
        ),
        (
            GateEvidenceRole::IndependentReference,
            candidate.independent_reference_available,
        ),
        (
            GateEvidenceRole::SemanticEquivalence,
            candidate.semantic_equivalence == SemanticEquivalence::Established,
        ),
    ];

    for (role, required) in required_roles {
        if required && !roles.contains(&role) {
            reasons.insert(RejectionReason::MissingEvidenceReceipt);
        }
    }
}

fn same_hypothesis_set(left: &[HypothesisGroup], right: &[HypothesisGroup]) -> bool {
    let mut left = left.to_vec();
    let mut right = right.to_vec();
    left.sort_unstable();
    right.sort_unstable();
    left == right
}

fn same_hypothesis_contrast_set(
    left: &[HypothesisContrast],
    right: &[HypothesisContrast],
) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut left = left.to_vec();
    let mut right = right.to_vec();
    let sort_contrasts = |items: &mut Vec<HypothesisContrast>| {
        items.sort_by(|a, b| {
            a.left_hypothesis_id
                .cmp(&b.left_hypothesis_id)
                .then_with(|| a.right_hypothesis_id.cmp(&b.right_hypothesis_id))
                .then_with(|| a.separation_lower_bound.total_cmp(&b.separation_lower_bound))
                .then_with(|| a.contrast_estimate_sha256.cmp(&b.contrast_estimate_sha256))
        });
    };
    sort_contrasts(&mut left);
    sort_contrasts(&mut right);
    left == right
}

fn validate_utility_estimate_receipt(
    candidate: &ExperimentCandidate,
    policy: &ExperimentDesignPolicy,
    reasons: &mut BTreeSet<RejectionReason>,
) {
    let Some(receipt) = candidate.utility_estimate_receipt.as_ref() else {
        reasons.insert(RejectionReason::MissingUtilityEstimateReceipt);
        return;
    };

    let scope_matches = !receipt.receipt_id.trim().is_empty()
        && !receipt.issuer_id.trim().is_empty()
        && is_sha256(&receipt.payload_sha256)
        && receipt.candidate_id == candidate.candidate_id
        && receipt.campaign_sha256 == candidate.campaign_sha256
        && receipt.model_scope_sha256 == candidate.model_scope_sha256
        && receipt.intended_use_profile_sha256 == candidate.intended_use_profile_sha256
        && receipt.decision_question_id == candidate.decision_question_id
        && receipt.design_sha256 == candidate.design_sha256
        && receipt.scenario_manifest_sha256 == candidate.scenario_manifest_sha256
        && receipt.measurement_plan_sha256 == candidate.measurement_plan_sha256
        && receipt.decision_boundary_sha256 == candidate.decision_boundary_sha256
        && receipt.hypothesis_manifest_sha256 == candidate.hypothesis_manifest_sha256
        && receipt.robust_uncertainty_set_sha256 == candidate.robust_uncertainty_set_sha256
        && receipt.adaptive_data_manifest_sha256 == candidate.adaptive_data_manifest_sha256
        && receipt.estimator_build_sha256 == candidate.estimator_build_sha256
        && receipt.estimate_method_sha256 == candidate.estimate_method_sha256
        && receipt.confidence_level == candidate.estimate_confidence_level
        && same_hypothesis_set(&receipt.hypothesis_groups, &candidate.hypothesis_groups)
        && same_hypothesis_contrast_set(
            &receipt.hypothesis_contrasts,
            &candidate.hypothesis_contrasts,
        )
        && receipt.expected_uncertainty_reduction == candidate.expected_uncertainty_reduction
        && receipt.expected_model_discrimination == candidate.expected_model_discrimination
        && receipt.worst_case_robust_reduction == candidate.worst_case_robust_reduction
        && receipt.decision_relevance == candidate.decision_relevance
        && receipt.probability_uninformative == candidate.probability_uninformative
        && receipt.campaign_sha256 == policy.campaign_sha256
        && receipt.model_scope_sha256 == policy.model_scope_sha256
        && receipt.intended_use_profile_sha256 == policy.intended_use_profile_sha256
        && receipt.decision_question_id == policy.decision_question_id
        && receipt.decision_boundary_sha256 == policy.decision_boundary_sha256
        && receipt.hypothesis_manifest_sha256 == policy.hypothesis_manifest_sha256
        && receipt.robust_uncertainty_set_sha256 == policy.robust_uncertainty_set_sha256
        && receipt.adaptive_data_manifest_sha256 == policy.adaptive_data_manifest_sha256
        && receipt.estimator_build_sha256 == policy.required_estimator_build_sha256
        && receipt.estimate_method_sha256 == policy.required_estimate_method_sha256
        && receipt.sample_unit == policy.required_estimate_sample_unit
        && bounded_unit_interval(receipt.confidence_level)
        && receipt.confidence_level > 0.0
        && receipt.confidence_level < 1.0
        && receipt.confidence_level >= policy.minimum_estimate_confidence_level
        && receipt.independent_unit_count >= policy.minimum_estimate_independent_units;

    if !scope_matches {
        reasons.insert(RejectionReason::InvalidUtilityEstimateReceipt);
    }
    if receipt.independent_unit_count < policy.minimum_estimate_independent_units
        || receipt.sample_unit != policy.required_estimate_sample_unit
    {
        reasons.insert(RejectionReason::InsufficientEstimateSamples);
    }
}

fn independent_group_count(candidate: &ExperimentCandidate) -> usize {
    candidate
        .hypothesis_groups
        .iter()
        .map(|group| group.group_id.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn independent_group_pair_count(candidate: &ExperimentCandidate, minimum_separation: f64) -> usize {
    if candidate.objective != ExperimentObjective::ModelDiscrimination {
        return 0;
    }
    let hypotheses: std::collections::BTreeMap<&str, &str> = candidate
        .hypothesis_groups
        .iter()
        .map(|item| (item.hypothesis_id.as_str(), item.group_id.as_str()))
        .collect();
    let mut pairs = BTreeSet::new();
    for contrast in &candidate.hypothesis_contrasts {
        if contrast.separation_lower_bound < minimum_separation {
            continue;
        }
        let (Some(left_group), Some(right_group)) = (
            hypotheses.get(contrast.left_hypothesis_id.as_str()),
            hypotheses.get(contrast.right_hypothesis_id.as_str()),
        ) else {
            continue;
        };
        if left_group == right_group {
            continue;
        }
        let pair = if left_group < right_group {
            (*left_group, *right_group)
        } else {
            (*right_group, *left_group)
        };
        pairs.insert(pair);
    }
    pairs.len()
}

fn validate_hypothesis_contract(candidate: &ExperimentCandidate) -> bool {
    let mut hypothesis_ids = BTreeSet::new();
    let mut digest_to_group = std::collections::BTreeMap::new();
    for hypothesis in &candidate.hypothesis_groups {
        if hypothesis.hypothesis_id.trim().is_empty()
            || hypothesis.group_id.trim().is_empty()
            || !is_sha256(&hypothesis.hypothesis_sha256)
            || !hypothesis_ids.insert(hypothesis.hypothesis_id.as_str())
        {
            return false;
        }
        if let Some(previous_group) = digest_to_group.insert(
            hypothesis.hypothesis_sha256.as_str(),
            hypothesis.group_id.as_str(),
        ) && previous_group != hypothesis.group_id
        {
            // One exact model artifact cannot be relabelled as two independent families.
            return false;
        }
    }

    let mut pairs = BTreeSet::new();
    for contrast in &candidate.hypothesis_contrasts {
        if contrast.left_hypothesis_id.trim().is_empty()
            || contrast.right_hypothesis_id.trim().is_empty()
            || contrast.left_hypothesis_id == contrast.right_hypothesis_id
            || !bounded_unit_interval(contrast.separation_lower_bound)
            || !is_sha256(&contrast.contrast_estimate_sha256)
        {
            return false;
        }
        let (Some(left), Some(right)) = (
            candidate
                .hypothesis_groups
                .iter()
                .find(|h| h.hypothesis_id == contrast.left_hypothesis_id),
            candidate
                .hypothesis_groups
                .iter()
                .find(|h| h.hypothesis_id == contrast.right_hypothesis_id),
        ) else {
            return false;
        };
        if left.group_id == right.group_id {
            return false;
        }
        let pair = if contrast.left_hypothesis_id < contrast.right_hypothesis_id {
            (
                contrast.left_hypothesis_id.as_str(),
                contrast.right_hypothesis_id.as_str(),
            )
        } else {
            (
                contrast.right_hypothesis_id.as_str(),
                contrast.left_hypothesis_id.as_str(),
            )
        };
        if !pairs.insert(pair) {
            return false;
        }
    }
    true
}

fn candidate_rejection_reasons(
    candidate: &ExperimentCandidate,
    policy: &ExperimentDesignPolicy,
) -> Vec<RejectionReason> {
    let mut reasons = BTreeSet::new();

    if candidate.candidate_id.trim().is_empty()
        || candidate.campaign_sha256 != policy.campaign_sha256
        || candidate.model_scope_sha256 != policy.model_scope_sha256
        || candidate.intended_use_profile_sha256 != policy.intended_use_profile_sha256
        || candidate.decision_question_id != policy.decision_question_id
        || candidate.objective != policy.objective
    {
        reasons.insert(RejectionReason::QuestionOrObjectiveMismatch);
    }
    if (candidate.requires_external_authorization
        || candidate.modality == ExperimentModality::ControlledFlight
        || policy
            .authorization_required_modalities
            .contains(&candidate.modality))
        && candidate.authorization != AuthorizationState::Authorized
    {
        reasons.insert(RejectionReason::AuthorizationNotGranted);
    }
    if candidate.safety_constraints_clear
        && candidate
            .safety_gate_receipt_sha256
            .as_deref()
            .is_none_or(|value| !is_sha256(value))
    {
        reasons.insert(RejectionReason::MissingEvidenceReceipt);
    }
    if !candidate.safety_constraints_clear {
        reasons.insert(RejectionReason::SafetyConstraintNotClear);
    }
    if candidate.within_declared_envelope
        && candidate
            .envelope_receipt_sha256
            .as_deref()
            .is_none_or(|value| !is_sha256(value))
    {
        reasons.insert(RejectionReason::MissingEvidenceReceipt);
    }
    if !candidate.within_declared_envelope {
        reasons.insert(RejectionReason::OutsideDeclaredEnvelope);
    }
    if candidate.independent_reference_available
        && candidate
            .independent_reference_receipt_sha256
            .as_deref()
            .is_none_or(|value| !is_sha256(value))
    {
        reasons.insert(RejectionReason::MissingEvidenceReceipt);
    }
    if !candidate.independent_reference_available {
        reasons.insert(RejectionReason::MissingIndependentReference);
    }
    if candidate.semantic_equivalence == SemanticEquivalence::Established
        && candidate
            .semantic_equivalence_receipt_sha256
            .as_deref()
            .is_none_or(|value| !is_sha256(value))
    {
        reasons.insert(RejectionReason::MissingEvidenceReceipt);
    }
    if candidate.authorization == AuthorizationState::Authorized
        && candidate
            .authorization_receipt_sha256
            .as_deref()
            .is_none_or(|value| !is_sha256(value))
    {
        reasons.insert(RejectionReason::MissingEvidenceReceipt);
    }
    validate_gate_evidence_receipts(candidate, policy, &mut reasons);
    validate_utility_estimate_receipt(candidate, policy, &mut reasons);
    if candidate.modality == ExperimentModality::CrossImplementation
        && candidate.semantic_equivalence != SemanticEquivalence::Established
    {
        reasons.insert(RejectionReason::SemanticEquivalenceNotEstablished);
    }
    if candidate.selection_evidence_role != SelectionEvidenceRole::AdaptiveDevelopment {
        reasons.insert(RejectionReason::SealedEvidenceUsedForSelection);
    }
    if !validate_hypothesis_contract(candidate) {
        reasons.insert(RejectionReason::InvalidHypothesisContrast);
    }
    if candidate.objective == ExperimentObjective::ModelDiscrimination {
        if independent_group_count(candidate) < 2 {
            reasons.insert(RejectionReason::InsufficientIndependentHypothesisGroups);
        }
        if independent_group_pair_count(candidate, policy.minimum_pair_separation)
            < policy.minimum_distinct_hypothesis_group_pairs
        {
            reasons.insert(RejectionReason::InsufficientQualifiedHypothesisContrasts);
        }
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
    if !bounded_unit_interval(candidate.estimate_confidence_level)
        || candidate.estimate_confidence_level == 0.0
        || candidate.estimate_confidence_level == 1.0
        || !is_sha256(&candidate.estimate_method_sha256)
        || !is_sha256(&candidate.estimator_build_sha256)
    {
        reasons.insert(RejectionReason::InvalidUtilityVector);
    }
    if !is_sha256(&candidate.campaign_sha256)
        || !is_sha256(&candidate.model_scope_sha256)
        || !is_sha256(&candidate.intended_use_profile_sha256)
        || !is_sha256(&candidate.decision_boundary_sha256)
        || !is_sha256(&candidate.hypothesis_manifest_sha256)
        || !is_sha256(&candidate.robust_uncertainty_set_sha256)
        || !is_sha256(&candidate.adaptive_data_manifest_sha256)
        || !is_sha256(&candidate.design_sha256)
        || !is_sha256(&candidate.scenario_manifest_sha256)
        || !is_sha256(&candidate.measurement_plan_sha256)
        || candidate
            .authorization_receipt_sha256
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
        || candidate
            .safety_gate_receipt_sha256
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
        || candidate
            .envelope_receipt_sha256
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
        || candidate
            .independent_reference_receipt_sha256
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
        || candidate
            .semantic_equivalence_receipt_sha256
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
    {
        reasons.insert(RejectionReason::InvalidEvidenceBinding);
    }
    if candidate.estimator_build_sha256 != policy.required_estimator_build_sha256
        || candidate.estimate_method_sha256 != policy.required_estimate_method_sha256
        || candidate.estimate_confidence_level < policy.minimum_estimate_confidence_level
    {
        reasons.insert(RejectionReason::ConfidenceContractMismatch);
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
    if candidate.cost_units > policy.maximum_cost_units
        || !candidate.resource_cost.within(policy.resource_budget)
    {
        reasons.insert(RejectionReason::OverBudget);
    }
    if candidate.probability_uninformative > policy.maximum_probability_uninformative {
        reasons.insert(RejectionReason::TooLikelyUninformative);
    }
    if candidate.hypothesis_groups.iter().any(|group| {
        group.group_id.trim().is_empty()
            || group.hypothesis_id.trim().is_empty()
            || !is_sha256(&group.hypothesis_sha256)
    }) {
        reasons.insert(RejectionReason::InvalidUtilityVector);
    }

    reasons.into_iter().collect()
}

/// Return true only when the left candidate Pareto-dominates the right candidate.
///
/// Benefits are maximized, uninformative probability and cost are minimized.
/// No scalar weighting or tie-breaking on different objective dimensions occurs.
fn dominates(
    left: &ExperimentCandidate,
    right: &ExperimentCandidate,
    minimum_pair_separation: f64,
    objective: ExperimentObjective,
) -> bool {
    let left_model_discrimination = if objective == ExperimentObjective::ModelDiscrimination {
        left.expected_model_discrimination
    } else {
        0.0
    };
    let right_model_discrimination = if objective == ExperimentObjective::ModelDiscrimination {
        right.expected_model_discrimination
    } else {
        0.0
    };
    let no_worse = left.expected_uncertainty_reduction >= right.expected_uncertainty_reduction
        && left_model_discrimination >= right_model_discrimination
        && independent_group_pair_count(left, minimum_pair_separation)
            >= independent_group_pair_count(right, minimum_pair_separation)
        && left.worst_case_robust_reduction >= right.worst_case_robust_reduction
        && left.decision_relevance >= right.decision_relevance
        && left.probability_uninformative <= right.probability_uninformative
        && left.cost_units <= right.cost_units
        && left.resource_cost.setup_minutes <= right.resource_cost.setup_minutes
        && left.resource_cost.facility_minutes <= right.resource_cost.facility_minutes
        && left.resource_cost.cpu_seconds <= right.resource_cost.cpu_seconds
        && left.resource_cost.accelerator_seconds <= right.resource_cost.accelerator_seconds
        && left.resource_cost.energy_joules <= right.resource_cost.energy_joules
        && left.resource_cost.adjudication_person_minutes
            <= right.resource_cost.adjudication_person_minutes;

    let strictly_better = left.expected_uncertainty_reduction
        > right.expected_uncertainty_reduction
        || left_model_discrimination > right_model_discrimination
        || independent_group_pair_count(left, minimum_pair_separation)
            > independent_group_pair_count(right, minimum_pair_separation)
        || left.worst_case_robust_reduction > right.worst_case_robust_reduction
        || left.decision_relevance > right.decision_relevance
        || left.probability_uninformative < right.probability_uninformative
        || left.cost_units < right.cost_units
        || left.resource_cost.setup_minutes < right.resource_cost.setup_minutes
        || left.resource_cost.facility_minutes < right.resource_cost.facility_minutes
        || left.resource_cost.cpu_seconds < right.resource_cost.cpu_seconds
        || left.resource_cost.accelerator_seconds < right.resource_cost.accelerator_seconds
        || left.resource_cost.energy_joules < right.resource_cost.energy_joules
        || left.resource_cost.adjudication_person_minutes
            < right.resource_cost.adjudication_person_minutes;

    no_worse && strictly_better
}

fn to_pareto(
    candidate: &ExperimentCandidate,
    minimum_pair_separation: f64,
    objective: ExperimentObjective,
) -> ParetoCandidate {
    ParetoCandidate {
        candidate_id: candidate.candidate_id.clone(),
        expected_uncertainty_reduction: candidate.expected_uncertainty_reduction,
        expected_model_discrimination: if objective == ExperimentObjective::ModelDiscrimination {
            candidate.expected_model_discrimination
        } else {
            0.0
        },
        independent_hypothesis_group_pair_count: independent_group_pair_count(
            candidate,
            minimum_pair_separation,
        ),
        worst_case_robust_reduction: candidate.worst_case_robust_reduction,
        decision_relevance: candidate.decision_relevance,
        probability_uninformative: candidate.probability_uninformative,
        cost_units: candidate.cost_units,
        resource_cost: candidate.resource_cost,
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
    let mut utility_receipt_ids = BTreeSet::new();
    for candidate in candidates {
        if !seen.insert(candidate.candidate_id.as_str()) {
            return Err(DesignError::DuplicateCandidateId);
        }
        if let Some(receipt) = candidate.utility_estimate_receipt.as_ref()
            && !receipt.receipt_id.trim().is_empty()
            && !utility_receipt_ids.insert(receipt.receipt_id.as_str())
        {
            return Err(DesignError::DuplicateUtilityEstimateReceiptId);
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
            !admissible.iter().copied().any(|other| {
                other.candidate_id != candidate.candidate_id
                    && dominates(
                        other,
                        candidate,
                        policy.minimum_pair_separation,
                        policy.objective,
                    )
            })
        })
        .collect();

    let mut pareto_front: Vec<ParetoCandidate> = front
        .into_iter()
        .map(|candidate| {
            to_pareto(
                candidate,
                policy.minimum_pair_separation,
                policy.objective,
            )
        })
        .collect();
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
