//! Sequential, causality-gated intervention design primitives.
//!
//! This is an offline policy oracle, not a biological model, wet-lab controller,
//! or evidence of a biological/SOTA result. It consumes model-conditional
//! likelihood tables from separately versioned model and inference components.
//! All tests use synthetic fixtures.

pub mod anytime;
pub mod benchmark;
pub mod scenario_tree;

use std::collections::{BTreeMap, BTreeSet};

const PROB_TOL: f64 = 1.0e-10;
const FINGERPRINT_ALGORITHM: &str = "fnv1a64-non-cryptographic-v1";

pub type Posterior = BTreeMap<String, f64>;

#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeLikelihood {
    pub outcome_id: String,
    /// P(outcome | model), keyed by stable model-family ID.
    pub model_probabilities: Posterior,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Feasibility {
    Permitted,
    Infeasible { reason: String },
    SafetyRejected { reason: String },
}

/// Protocol preconditions. A declaration is not proof that the protocol is valid.
#[derive(Debug, Clone, PartialEq)]
pub struct CausalValidity {
    pub assignment_design_valid: bool,
    pub delivery_fidelity: f64,
    pub delivery_identity_verified: bool,
    pub interference_controlled: bool,
    pub carryover_cleared: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterventionCandidate {
    pub candidate_id: String,
    pub protocol_id: String,
    /// Shared unit identifier for all candidates in one decision manifest.
    pub cost_unit_id: String,
    pub cost: f64,
    pub feasibility: Feasibility,
    pub causal_validity: CausalValidity,
    pub outcomes: Vec<OutcomeLikelihood>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceTier {
    SyntheticFixture,
    InSilicoGenerated,
    PreprintMeasured,
    PeerReviewedMeasured,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentRecord {
    pub sample_id: String,
    pub evidence_tier: EvidenceTier,
    pub assigned_candidate_id: String,
    pub delivered_protocol_id: String,
    pub outcome_id: String,
    /// Animal, preparation, or other preregistered independent replication unit.
    pub replication_unit_id: String,
    pub session_id: String,
    pub washout_cleared: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelAdequacy {
    Adequate,
    FailedPosteriorPredictiveCheck,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionState {
    pub posterior: Posterior,
    /// Alternative posterior states in the declared robustness/ambiguity set.
    pub ambiguity_posteriors: Vec<Posterior>,
    pub remaining_cost_budget: f64,
    pub completed_experiments: u32,
    pub model_adequacy: ModelAdequacy,
    pub history: Vec<ExperimentRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionPolicy {
    /// Immediate EIG ranking. Greedy does not mean globally optimal.
    GreedyEig,
    /// EIG divided by comparable intervention cost.
    CostNormalizedEig,
    /// Maximize worst-case EIG per cost over the declared ambiguity set.
    RobustMaximin,
    /// Reproducible random feasible comparator.
    DeterministicRandom { seed: u64 },
    /// Fixed alphabetic manifest-order comparator, independent of model scores.
    FixedCanonicalOrder,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionConfig {
    pub policy: SelectionPolicy,
    pub minimum_information_gain_nats: f64,
    pub posterior_stop_probability: f64,
    pub maximum_experiments: u32,
    pub minimum_delivery_fidelity: f64,
}

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            policy: SelectionPolicy::CostNormalizedEig,
            minimum_information_gain_nats: 1.0e-12,
            posterior_stop_probability: 0.95,
            maximum_experiments: 24,
            minimum_delivery_fidelity: 0.90,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    EmptyIdentifier(&'static str),
    InvalidProbability(&'static str),
    UnnormalizedDistribution(&'static str),
    ModelSetMismatch,
    DuplicateCandidate(String),
    DuplicateOutcome(String),
    DuplicateSample(String),
    InvalidCost(String),
    CostUnitMismatch,
    InvalidBudget,
    InvalidThreshold(&'static str),
    EmptyOutcomeSet(String),
    UnknownOutcome(String),
    AssignmentMismatch,
    DeliveryMismatch,
    WashoutNotCleared,
    InvalidReplicationIdentity,
    CandidateNotPermittedForUpdate,
    InvalidUpdateValidityThreshold,
    RepeatedReplicationUnit(String),
    RepeatedSession(String),
    InvalidDeliveryFidelity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectionReason {
    Infeasible(String),
    SafetyRejected(String),
    AssignmentDesignNotValid,
    DeliveryIdentityNotVerified,
    DeliveryFidelityBelowThreshold,
    InterferenceNotControlled,
    CarryoverNotCleared,
    BudgetExceeded,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateRejection {
    pub candidate_id: String,
    pub reason: RejectionReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReproducibilityFingerprint {
    /// FNV-1a is non-cryptographic: drift detection only, not a security boundary.
    pub algorithm: String,
    pub value_hex: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyReceipt {
    pub policy_version: String,
    pub state_fingerprint: ReproducibilityFingerprint,
    pub candidate_manifest_fingerprint: ReproducibilityFingerprint,
    pub selected_candidate_id: Option<String>,
    pub expected_information_gain_nats: Option<f64>,
    pub robust_information_gain_nats: Option<f64>,
    pub rejected_candidates: Vec<CandidateRejection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    PosteriorThresholdReached,
    PreregisteredExperimentLimitReached,
    ModelInadequateRequestRevision,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AbstainReason {
    NoPermittedCandidate,
    NoInformativeCandidate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Selection {
    pub candidate_id: String,
    pub expected_information_gain_nats: f64,
    pub robust_information_gain_nats: f64,
    pub policy_score: f64,
    pub receipt: PolicyReceipt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    Select(Selection),
    Stop {
        reason: StopReason,
        receipt: PolicyReceipt,
    },
    Abstain {
        reason: AbstainReason,
        receipt: PolicyReceipt,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PosteriorUpdate {
    Updated {
        posterior: Posterior,
        marginal_likelihood: f64,
    },
    /// The observation is impossible under every surviving model; request revision.
    ModelInadequate { marginal_likelihood: f64 },
}

pub fn validate_posterior(posterior: &Posterior) -> Result<(), EngineError> {
    if posterior.is_empty() {
        return Err(EngineError::InvalidProbability("empty posterior"));
    }
    for (id, probability) in posterior {
        if id.trim().is_empty() {
            return Err(EngineError::EmptyIdentifier("model_id"));
        }
        if !probability.is_finite() || !(0.0..=1.0).contains(probability) {
            return Err(EngineError::InvalidProbability("posterior probability"));
        }
    }
    if (posterior.values().sum::<f64>() - 1.0).abs() > PROB_TOL {
        return Err(EngineError::UnnormalizedDistribution("posterior"));
    }
    Ok(())
}

fn entropy(distribution: &Posterior) -> f64 {
    distribution
        .values()
        .filter(|p| **p > 0.0)
        .map(|p| -*p * p.ln())
        .sum()
}

fn validate_candidate(
    candidate: &InterventionCandidate,
    model_ids: &BTreeSet<String>,
) -> Result<(), EngineError> {
    if candidate.candidate_id.trim().is_empty() {
        return Err(EngineError::EmptyIdentifier("candidate_id"));
    }
    if candidate.protocol_id.trim().is_empty() {
        return Err(EngineError::EmptyIdentifier("protocol_id"));
    }
    if candidate.cost_unit_id.trim().is_empty() {
        return Err(EngineError::EmptyIdentifier("cost_unit_id"));
    }
    if !candidate.cost.is_finite() || candidate.cost <= 0.0 {
        return Err(EngineError::InvalidCost(candidate.candidate_id.clone()));
    }
    if candidate.outcomes.is_empty() {
        return Err(EngineError::EmptyOutcomeSet(candidate.candidate_id.clone()));
    }
    let fidelity = candidate.causal_validity.delivery_fidelity;
    if !fidelity.is_finite() || !(0.0..=1.0).contains(&fidelity) {
        return Err(EngineError::InvalidDeliveryFidelity);
    }

    let mut outcome_ids = BTreeSet::new();
    let mut totals: BTreeMap<String, f64> = model_ids.iter().cloned().map(|id| (id, 0.0)).collect();
    for outcome in &candidate.outcomes {
        if outcome.outcome_id.trim().is_empty() {
            return Err(EngineError::EmptyIdentifier("outcome_id"));
        }
        if !outcome_ids.insert(outcome.outcome_id.clone()) {
            return Err(EngineError::DuplicateOutcome(outcome.outcome_id.clone()));
        }
        let observed_models: BTreeSet<String> =
            outcome.model_probabilities.keys().cloned().collect();
        if &observed_models != model_ids {
            return Err(EngineError::ModelSetMismatch);
        }
        for (model_id, probability) in &outcome.model_probabilities {
            if !probability.is_finite() || !(0.0..=1.0).contains(probability) {
                return Err(EngineError::InvalidProbability("outcome likelihood"));
            }
            *totals.get_mut(model_id).expect("validated model identity") += probability;
        }
    }
    if totals.values().any(|sum| (*sum - 1.0).abs() > PROB_TOL) {
        return Err(EngineError::UnnormalizedDistribution("P(outcome | model)"));
    }
    Ok(())
}

/// Exact categorical EIG: H(Model) - E_outcome[H(Model | outcome)].
/// This finite calculation has no Monte Carlo integration error.
pub fn expected_information_gain(
    posterior: &Posterior,
    candidate: &InterventionCandidate,
) -> Result<f64, EngineError> {
    validate_posterior(posterior)?;
    let model_ids: BTreeSet<String> = posterior.keys().cloned().collect();
    validate_candidate(candidate, &model_ids)?;

    let prior_entropy = entropy(posterior);
    let mut expected_posterior_entropy = 0.0;
    for outcome in &candidate.outcomes {
        let marginal: f64 = posterior
            .iter()
            .map(|(model, prior)| {
                prior
                    * outcome
                        .model_probabilities
                        .get(model)
                        .copied()
                        .unwrap_or(0.0)
            })
            .sum();
        if marginal == 0.0 {
            continue;
        }
        let conditional: Posterior = posterior
            .iter()
            .map(|(model, prior)| {
                let joint = prior * outcome.model_probabilities[model];
                (model.clone(), joint / marginal)
            })
            .collect();
        expected_posterior_entropy += marginal * entropy(&conditional);
    }
    Ok((prior_entropy - expected_posterior_entropy).max(0.0))
}

/// Condition on a delivered outcome only when assignment, delivery identity,
/// replication-unit identity and washout are verified.
/// The V0 updater assumes one observation per independent replication unit.
/// Repeated observations from the same animal/preparation or session require a
/// joint cluster/trajectory likelihood and are rejected until that adapter is
/// supplied. This avoids treating correlated time points as independent trials.
pub fn condition_on_outcome(
    posterior: &Posterior,
    candidate: &InterventionCandidate,
    record: &ExperimentRecord,
    prior_history: &[ExperimentRecord],
    minimum_delivery_fidelity: f64,
) -> Result<PosteriorUpdate, EngineError> {
    validate_posterior(posterior)?;
    let model_ids: BTreeSet<String> = posterior.keys().cloned().collect();
    validate_candidate(candidate, &model_ids)?;
    if !minimum_delivery_fidelity.is_finite() || !(0.0..=1.0).contains(&minimum_delivery_fidelity) {
        return Err(EngineError::InvalidUpdateValidityThreshold);
    }
    let validity = &candidate.causal_validity;
    if !matches!(&candidate.feasibility, Feasibility::Permitted)
        || !validity.assignment_design_valid
        || !validity.delivery_identity_verified
        || validity.delivery_fidelity < minimum_delivery_fidelity
        || !validity.interference_controlled
        || !validity.carryover_cleared
    {
        return Err(EngineError::CandidateNotPermittedForUpdate);
    }
    if record.assigned_candidate_id != candidate.candidate_id {
        return Err(EngineError::AssignmentMismatch);
    }
    if record.delivered_protocol_id != candidate.protocol_id {
        return Err(EngineError::DeliveryMismatch);
    }
    if !record.washout_cleared {
        return Err(EngineError::WashoutNotCleared);
    }
    if record.sample_id.trim().is_empty()
        || record.replication_unit_id.trim().is_empty()
        || record.session_id.trim().is_empty()
    {
        return Err(EngineError::InvalidReplicationIdentity);
    }
    for previous in prior_history {
        if previous.sample_id == record.sample_id {
            return Err(EngineError::DuplicateSample(record.sample_id.clone()));
        }
        if previous.replication_unit_id == record.replication_unit_id {
            return Err(EngineError::RepeatedReplicationUnit(
                record.replication_unit_id.clone(),
            ));
        }
        if previous.session_id == record.session_id {
            return Err(EngineError::RepeatedSession(record.session_id.clone()));
        }
    }
    let outcome = candidate
        .outcomes
        .iter()
        .find(|item| item.outcome_id == record.outcome_id)
        .ok_or_else(|| EngineError::UnknownOutcome(record.outcome_id.clone()))?;
    let marginal: f64 = posterior
        .iter()
        .map(|(model, prior)| prior * outcome.model_probabilities[model])
        .sum();
    if marginal == 0.0 {
        return Ok(PosteriorUpdate::ModelInadequate {
            marginal_likelihood: marginal,
        });
    }
    let updated: Posterior = posterior
        .iter()
        .map(|(model, prior)| {
            (
                model.clone(),
                prior * outcome.model_probabilities[model] / marginal,
            )
        })
        .collect();
    validate_posterior(&updated)?;
    Ok(PosteriorUpdate::Updated {
        posterior: updated,
        marginal_likelihood: marginal,
    })
}

fn validate_state(state: &DecisionState, config: &DecisionConfig) -> Result<(), EngineError> {
    validate_posterior(&state.posterior)?;
    if !state.remaining_cost_budget.is_finite() || state.remaining_cost_budget < 0.0 {
        return Err(EngineError::InvalidBudget);
    }
    if !config.minimum_information_gain_nats.is_finite()
        || config.minimum_information_gain_nats < 0.0
    {
        return Err(EngineError::InvalidThreshold(
            "minimum_information_gain_nats",
        ));
    }
    if !config.posterior_stop_probability.is_finite()
        || !(0.0..=1.0).contains(&config.posterior_stop_probability)
    {
        return Err(EngineError::InvalidThreshold("posterior_stop_probability"));
    }
    if !config.minimum_delivery_fidelity.is_finite()
        || !(0.0..=1.0).contains(&config.minimum_delivery_fidelity)
    {
        return Err(EngineError::InvalidThreshold("minimum_delivery_fidelity"));
    }
    for alternative in &state.ambiguity_posteriors {
        validate_posterior(alternative)?;
        if alternative.keys().ne(state.posterior.keys()) {
            return Err(EngineError::ModelSetMismatch);
        }
    }
    let mut samples = BTreeSet::new();
    for record in &state.history {
        if record.sample_id.trim().is_empty() {
            return Err(EngineError::EmptyIdentifier("history.sample_id"));
        }
        if !samples.insert(record.sample_id.clone()) {
            return Err(EngineError::DuplicateSample(record.sample_id.clone()));
        }
    }
    Ok(())
}

fn reject_candidate(
    candidate: &InterventionCandidate,
    config: &DecisionConfig,
    state: &DecisionState,
) -> Option<RejectionReason> {
    match &candidate.feasibility {
        Feasibility::Infeasible { reason } => {
            return Some(RejectionReason::Infeasible(reason.clone()));
        }
        Feasibility::SafetyRejected { reason } => {
            return Some(RejectionReason::SafetyRejected(reason.clone()));
        }
        Feasibility::Permitted => {}
    }
    let valid = &candidate.causal_validity;
    if !valid.assignment_design_valid {
        return Some(RejectionReason::AssignmentDesignNotValid);
    }
    if !valid.delivery_identity_verified {
        return Some(RejectionReason::DeliveryIdentityNotVerified);
    }
    if valid.delivery_fidelity < config.minimum_delivery_fidelity {
        return Some(RejectionReason::DeliveryFidelityBelowThreshold);
    }
    if !valid.interference_controlled {
        return Some(RejectionReason::InterferenceNotControlled);
    }
    if !valid.carryover_cleared {
        return Some(RejectionReason::CarryoverNotCleared);
    }
    if candidate.cost > state.remaining_cost_budget {
        return Some(RejectionReason::BudgetExceeded);
    }
    None
}

fn push_len(bytes: &mut Vec<u8>, len: usize) {
    bytes.extend_from_slice(&(len as u64).to_be_bytes());
}

fn push_str(bytes: &mut Vec<u8>, value: &str) {
    push_len(bytes, value.len());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_f64(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(&value.to_bits().to_be_bytes());
}

fn fingerprint(bytes: &[u8]) -> ReproducibilityFingerprint {
    // FNV-1a 64-bit: deterministic accidental-drift detection only.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    ReproducibilityFingerprint {
        algorithm: FINGERPRINT_ALGORITHM.to_owned(),
        value_hex: format!("{hash:016x}"),
    }
}

fn candidate_fingerprint(candidates: &[InterventionCandidate]) -> ReproducibilityFingerprint {
    let mut sorted: Vec<&InterventionCandidate> = candidates.iter().collect();
    sorted.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
    let mut bytes = Vec::new();
    push_len(&mut bytes, sorted.len());
    for candidate in sorted {
        push_str(&mut bytes, &candidate.candidate_id);
        push_str(&mut bytes, &candidate.protocol_id);
        push_str(&mut bytes, &candidate.cost_unit_id);
        push_f64(&mut bytes, candidate.cost);
        match &candidate.feasibility {
            Feasibility::Permitted => bytes.push(0),
            Feasibility::Infeasible { reason } => {
                bytes.push(1);
                push_str(&mut bytes, reason);
            }
            Feasibility::SafetyRejected { reason } => {
                bytes.push(2);
                push_str(&mut bytes, reason);
            }
        }
        let validity = &candidate.causal_validity;
        bytes.extend_from_slice(&[
            validity.assignment_design_valid as u8,
            validity.delivery_identity_verified as u8,
            validity.interference_controlled as u8,
            validity.carryover_cleared as u8,
        ]);
        push_f64(&mut bytes, validity.delivery_fidelity);
        let mut outcomes: Vec<&OutcomeLikelihood> = candidate.outcomes.iter().collect();
        outcomes.sort_by(|a, b| a.outcome_id.cmp(&b.outcome_id));
        push_len(&mut bytes, outcomes.len());
        for outcome in outcomes {
            push_str(&mut bytes, &outcome.outcome_id);
            push_len(&mut bytes, outcome.model_probabilities.len());
            for (model, probability) in &outcome.model_probabilities {
                push_str(&mut bytes, model);
                push_f64(&mut bytes, *probability);
            }
        }
    }
    fingerprint(&bytes)
}

fn state_fingerprint(state: &DecisionState, config: &DecisionConfig) -> ReproducibilityFingerprint {
    let mut bytes = Vec::new();
    push_len(&mut bytes, state.posterior.len());
    for (model, probability) in &state.posterior {
        push_str(&mut bytes, model);
        push_f64(&mut bytes, *probability);
    }
    let mut ambiguity: Vec<ReproducibilityFingerprint> = state
        .ambiguity_posteriors
        .iter()
        .map(|posterior| {
            let mut encoded = Vec::new();
            push_len(&mut encoded, posterior.len());
            for (model, probability) in posterior {
                push_str(&mut encoded, model);
                push_f64(&mut encoded, *probability);
            }
            fingerprint(&encoded)
        })
        .collect();
    ambiguity.sort_by(|a, b| a.value_hex.cmp(&b.value_hex));
    push_len(&mut bytes, ambiguity.len());
    for item in ambiguity {
        push_str(&mut bytes, &item.value_hex);
    }
    push_f64(&mut bytes, state.remaining_cost_budget);
    bytes.extend_from_slice(&state.completed_experiments.to_be_bytes());
    bytes.push(match state.model_adequacy {
        ModelAdequacy::Adequate => 0,
        ModelAdequacy::FailedPosteriorPredictiveCheck => 1,
    });
    push_str(&mut bytes, &format!("{:?}", config.policy));
    push_f64(&mut bytes, config.minimum_information_gain_nats);
    push_f64(&mut bytes, config.posterior_stop_probability);
    bytes.extend_from_slice(&config.maximum_experiments.to_be_bytes());
    push_f64(&mut bytes, config.minimum_delivery_fidelity);
    let mut history: Vec<&ExperimentRecord> = state.history.iter().collect();
    history.sort_by(|a, b| a.sample_id.cmp(&b.sample_id));
    push_len(&mut bytes, history.len());
    for record in history {
        push_str(&mut bytes, &record.sample_id);
        push_str(&mut bytes, &record.assigned_candidate_id);
        push_str(&mut bytes, &record.delivered_protocol_id);
        push_str(&mut bytes, &record.outcome_id);
        push_str(&mut bytes, &record.replication_unit_id);
        push_str(&mut bytes, &record.session_id);
        bytes.push(match record.evidence_tier {
            EvidenceTier::SyntheticFixture => 0,
            EvidenceTier::InSilicoGenerated => 1,
            EvidenceTier::PreprintMeasured => 2,
            EvidenceTier::PeerReviewedMeasured => 3,
        });
        bytes.push(record.washout_cleared as u8);
    }
    fingerprint(&bytes)
}

struct ReceiptSummary {
    selected_candidate_id: Option<String>,
    expected_information_gain_nats: Option<f64>,
    robust_information_gain_nats: Option<f64>,
    rejected_candidates: Vec<CandidateRejection>,
}

fn make_receipt(
    state: &DecisionState,
    candidates: &[InterventionCandidate],
    config: &DecisionConfig,
    summary: ReceiptSummary,
) -> PolicyReceipt {
    PolicyReceipt {
        policy_version: "anima-sequential-policy-v0.1".to_owned(),
        state_fingerprint: state_fingerprint(state, config),
        candidate_manifest_fingerprint: candidate_fingerprint(candidates),
        selected_candidate_id: summary.selected_candidate_id,
        expected_information_gain_nats: summary.expected_information_gain_nats,
        robust_information_gain_nats: summary.robust_information_gain_nats,
        rejected_candidates: summary.rejected_candidates,
    }
}

fn splitmix64(seed: u64) -> u64 {
    let mut value = seed.wrapping_add(0x9e3779b97f4a7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

/// Choose the next action or return an explicit stop/abstention. Every candidate
/// is validated before feasibility filtering, so malformed entries are not
/// silently dropped from the manifest.
pub fn choose_next(
    state: &DecisionState,
    candidates: &[InterventionCandidate],
    config: &DecisionConfig,
) -> Result<Decision, EngineError> {
    validate_state(state, config)?;
    let model_ids: BTreeSet<String> = state.posterior.keys().cloned().collect();
    let mut candidate_ids = BTreeSet::new();
    let mut cost_units = BTreeSet::new();
    for candidate in candidates {
        validate_candidate(candidate, &model_ids)?;
        cost_units.insert(candidate.cost_unit_id.clone());
        if !candidate_ids.insert(candidate.candidate_id.clone()) {
            return Err(EngineError::DuplicateCandidate(
                candidate.candidate_id.clone(),
            ));
        }
    }
    if cost_units.len() > 1 {
        return Err(EngineError::CostUnitMismatch);
    }
    let rejected: Vec<CandidateRejection> = candidates
        .iter()
        .filter_map(|candidate| {
            reject_candidate(candidate, config, state).map(|reason| CandidateRejection {
                candidate_id: candidate.candidate_id.clone(),
                reason,
            })
        })
        .collect();

    let summary = |selected, eig, robust, rejected| ReceiptSummary {
        selected_candidate_id: selected,
        expected_information_gain_nats: eig,
        robust_information_gain_nats: robust,
        rejected_candidates: rejected,
    };
    if state.model_adequacy == ModelAdequacy::FailedPosteriorPredictiveCheck {
        let receipt = make_receipt(
            state,
            candidates,
            config,
            summary(None, None, None, rejected),
        );
        return Ok(Decision::Stop {
            reason: StopReason::ModelInadequateRequestRevision,
            receipt,
        });
    }
    if state.posterior.values().copied().fold(0.0_f64, f64::max)
        >= config.posterior_stop_probability
    {
        let receipt = make_receipt(
            state,
            candidates,
            config,
            summary(None, None, None, rejected),
        );
        return Ok(Decision::Stop {
            reason: StopReason::PosteriorThresholdReached,
            receipt,
        });
    }
    if state.completed_experiments >= config.maximum_experiments {
        let receipt = make_receipt(
            state,
            candidates,
            config,
            summary(None, None, None, rejected),
        );
        return Ok(Decision::Stop {
            reason: StopReason::PreregisteredExperimentLimitReached,
            receipt,
        });
    }

    let mut eligible: Vec<&InterventionCandidate> = candidates
        .iter()
        .filter(|candidate| reject_candidate(candidate, config, state).is_none())
        .collect();
    eligible.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
    if eligible.is_empty() {
        let receipt = make_receipt(
            state,
            candidates,
            config,
            summary(None, None, None, rejected),
        );
        return Ok(Decision::Abstain {
            reason: AbstainReason::NoPermittedCandidate,
            receipt,
        });
    }

    let mut scored = Vec::with_capacity(eligible.len());
    for candidate in eligible {
        let current_gain = expected_information_gain(&state.posterior, candidate)?;
        let robust_gain = if config.policy == SelectionPolicy::RobustMaximin {
            let gains = state
                .ambiguity_posteriors
                .iter()
                .map(|posterior| expected_information_gain(posterior, candidate))
                .collect::<Result<Vec<_>, _>>()?;
            gains.into_iter().fold(current_gain, f64::min)
        } else {
            current_gain
        };
        let score = match config.policy {
            SelectionPolicy::GreedyEig => robust_gain,
            SelectionPolicy::CostNormalizedEig | SelectionPolicy::RobustMaximin => {
                robust_gain / candidate.cost
            }
            SelectionPolicy::DeterministicRandom { .. } | SelectionPolicy::FixedCanonicalOrder => {
                0.0
            }
        };
        scored.push((candidate, current_gain, robust_gain, score));
    }

    let selected_index = match config.policy {
        SelectionPolicy::DeterministicRandom { seed } => (splitmix64(seed) as usize) % scored.len(),
        SelectionPolicy::FixedCanonicalOrder => {
            scored.sort_by(|a, b| a.0.candidate_id.cmp(&b.0.candidate_id));
            0
        }
        _ => {
            scored.sort_by(|a, b| {
                b.3.total_cmp(&a.3)
                    .then_with(|| a.0.candidate_id.cmp(&b.0.candidate_id))
            });
            0
        }
    };
    let (candidate, current_gain, robust_gain, score) = scored[selected_index];
    let adaptive = !matches!(
        config.policy,
        SelectionPolicy::DeterministicRandom { .. } | SelectionPolicy::FixedCanonicalOrder
    );
    if adaptive && robust_gain <= config.minimum_information_gain_nats {
        let receipt = make_receipt(
            state,
            candidates,
            config,
            summary(None, Some(current_gain), Some(robust_gain), rejected),
        );
        return Ok(Decision::Abstain {
            reason: AbstainReason::NoInformativeCandidate,
            receipt,
        });
    }
    let receipt = make_receipt(
        state,
        candidates,
        config,
        summary(
            Some(candidate.candidate_id.clone()),
            Some(current_gain),
            Some(robust_gain),
            rejected,
        ),
    );
    Ok(Decision::Select(Selection {
        candidate_id: candidate.candidate_id.clone(),
        expected_information_gain_nats: current_gain,
        robust_information_gain_nats: robust_gain,
        policy_score: score,
        receipt,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn posterior(p: f64) -> Posterior {
        BTreeMap::from([("model-a".to_owned(), p), ("model-b".to_owned(), 1.0 - p)])
    }

    fn candidate(id: &str, a_yes: f64, b_yes: f64, cost: f64) -> InterventionCandidate {
        InterventionCandidate {
            candidate_id: id.to_owned(),
            protocol_id: format!("protocol-{id}"),
            cost_unit_id: "experiment-credit".to_owned(),
            cost,
            feasibility: Feasibility::Permitted,
            causal_validity: CausalValidity {
                assignment_design_valid: true,
                delivery_fidelity: 1.0,
                delivery_identity_verified: true,
                interference_controlled: true,
                carryover_cleared: true,
            },
            outcomes: vec![
                OutcomeLikelihood {
                    outcome_id: "yes".to_owned(),
                    model_probabilities: BTreeMap::from([
                        ("model-a".to_owned(), a_yes),
                        ("model-b".to_owned(), b_yes),
                    ]),
                },
                OutcomeLikelihood {
                    outcome_id: "no".to_owned(),
                    model_probabilities: BTreeMap::from([
                        ("model-a".to_owned(), 1.0 - a_yes),
                        ("model-b".to_owned(), 1.0 - b_yes),
                    ]),
                },
            ],
        }
    }

    fn state(p: f64) -> DecisionState {
        DecisionState {
            posterior: posterior(p),
            ambiguity_posteriors: vec![],
            remaining_cost_budget: 10.0,
            completed_experiments: 0,
            model_adequacy: ModelAdequacy::Adequate,
            history: vec![],
        }
    }

    fn record(candidate: &InterventionCandidate, outcome: &str) -> ExperimentRecord {
        ExperimentRecord {
            sample_id: "sample-1".to_owned(),
            evidence_tier: EvidenceTier::SyntheticFixture,
            assigned_candidate_id: candidate.candidate_id.clone(),
            delivered_protocol_id: candidate.protocol_id.clone(),
            outcome_id: outcome.to_owned(),
            replication_unit_id: "animal-1".to_owned(),
            session_id: "session-1".to_owned(),
            washout_cleared: true,
        }
    }

    #[test]
    fn identical_predictions_have_zero_eig() {
        assert!(
            expected_information_gain(&posterior(0.5), &candidate("same", 0.8, 0.8, 1.0))
                .unwrap()
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn disagreement_has_positive_eig() {
        assert!(
            expected_information_gain(&posterior(0.5), &candidate("separating", 0.9, 0.1, 1.0))
                .unwrap()
                > 0.3
        );
    }

    #[test]
    fn greedy_selects_highest_eig() {
        let items = vec![
            candidate("weak", 0.6, 0.4, 1.0),
            candidate("strong", 0.9, 0.1, 1.0),
        ];
        let result = choose_next(
            &state(0.5),
            &items,
            &DecisionConfig {
                policy: SelectionPolicy::GreedyEig,
                ..DecisionConfig::default()
            },
        )
        .unwrap();
        assert!(matches!(result, Decision::Select(s) if s.candidate_id == "strong"));
    }

    #[test]
    fn incompatible_cost_units_fail_closed() {
        let mut second = candidate("second", 0.8, 0.2, 1.0);
        second.cost_unit_id = "minutes".to_owned();
        assert_eq!(
            choose_next(
                &state(0.5),
                &[candidate("first", 0.9, 0.1, 1.0), second],
                &DecisionConfig::default()
            ),
            Err(EngineError::CostUnitMismatch)
        );
    }

    #[test]
    fn cost_normalization_can_prefer_the_cheaper_test() {
        let items = vec![
            candidate("expensive", 0.99, 0.01, 10.0),
            candidate("efficient", 0.85, 0.15, 1.0),
        ];
        let result = choose_next(&state(0.5), &items, &DecisionConfig::default()).unwrap();
        assert!(matches!(result, Decision::Select(s) if s.candidate_id == "efficient"));
    }

    #[test]
    fn sequential_update_changes_the_next_best_intervention() {
        let alpha = candidate("alpha", 0.8, 0.1, 1.0);
        let beta = candidate("beta", 0.1, 0.8, 1.0);
        let first = choose_next(
            &state(0.5),
            &[alpha.clone(), beta.clone()],
            &DecisionConfig::default(),
        )
        .unwrap();
        assert!(matches!(first, Decision::Select(ref s) if s.candidate_id == "alpha"));
        let update =
            condition_on_outcome(&posterior(0.5), &alpha, &record(&alpha, "yes"), &[], 0.9)
                .unwrap();
        let new_posterior = match update {
            PosteriorUpdate::Updated { posterior, .. } => posterior,
            other => panic!("unexpected update: {other:?}"),
        };
        let mut next_state = state(0.5);
        next_state.posterior = new_posterior;
        next_state.completed_experiments = 1;
        let second = choose_next(&next_state, &[alpha, beta], &DecisionConfig::default()).unwrap();
        assert!(matches!(second, Decision::Select(ref s) if s.candidate_id == "beta"));
    }

    #[test]
    fn update_rejects_assignment_mismatch() {
        let c = candidate("stim", 0.9, 0.1, 1.0);
        let mut r = record(&c, "yes");
        r.assigned_candidate_id = "wrong".to_owned();
        assert_eq!(
            condition_on_outcome(&posterior(0.5), &c, &r, &[], 0.9),
            Err(EngineError::AssignmentMismatch)
        );
    }

    #[test]
    fn update_rejects_delivery_mismatch() {
        let c = candidate("stim", 0.9, 0.1, 1.0);
        let mut r = record(&c, "yes");
        r.delivered_protocol_id = "wrong".to_owned();
        assert_eq!(
            condition_on_outcome(&posterior(0.5), &c, &r, &[], 0.9),
            Err(EngineError::DeliveryMismatch)
        );
    }

    #[test]
    fn update_rejects_uncleared_washout() {
        let c = candidate("stim", 0.9, 0.1, 1.0);
        let mut r = record(&c, "yes");
        r.washout_cleared = false;
        assert_eq!(
            condition_on_outcome(&posterior(0.5), &c, &r, &[], 0.9),
            Err(EngineError::WashoutNotCleared)
        );
    }

    #[test]
    fn invalid_or_unsafe_candidate_cannot_update_posterior() {
        let mut c = candidate("unsafe", 0.9, 0.1, 1.0);
        c.feasibility = Feasibility::SafetyRejected {
            reason: "dose ceiling".into(),
        };
        assert_eq!(
            condition_on_outcome(&posterior(0.5), &c, &record(&c, "yes"), &[], 0.9),
            Err(EngineError::CandidateNotPermittedForUpdate)
        );
    }

    #[test]
    fn failed_delivery_fidelity_cannot_update_posterior() {
        let mut c = candidate("weak-delivery", 0.9, 0.1, 1.0);
        c.causal_validity.delivery_fidelity = 0.5;
        assert_eq!(
            condition_on_outcome(&posterior(0.5), &c, &record(&c, "yes"), &[], 0.9),
            Err(EngineError::CandidateNotPermittedForUpdate)
        );
    }

    #[test]
    fn repeated_replication_unit_requires_joint_likelihood() {
        let c = candidate("stim", 0.9, 0.1, 1.0);
        let previous = record(&c, "yes");
        let mut next = record(&c, "no");
        next.sample_id = "sample-2".to_owned();
        assert_eq!(
            condition_on_outcome(&posterior(0.5), &c, &next, &[previous], 0.9),
            Err(EngineError::RepeatedReplicationUnit("animal-1".to_owned()))
        );
    }

    #[test]
    fn zero_probability_observation_requests_model_revision() {
        let c = candidate("stim", 1.0, 1.0, 1.0);
        let result =
            condition_on_outcome(&posterior(0.5), &c, &record(&c, "no"), &[], 0.9).unwrap();
        assert_eq!(
            result,
            PosteriorUpdate::ModelInadequate {
                marginal_likelihood: 0.0
            }
        );
    }

    #[test]
    fn safety_gate_overrides_information_gain() {
        let mut unsafe_item = candidate("unsafe", 0.99, 0.01, 1.0);
        unsafe_item.feasibility = Feasibility::SafetyRejected {
            reason: "protocol ceiling".into(),
        };
        let safe = candidate("safe", 0.7, 0.3, 1.0);
        let result = choose_next(
            &state(0.5),
            &[unsafe_item, safe],
            &DecisionConfig::default(),
        )
        .unwrap();
        assert!(matches!(result, Decision::Select(s) if s.candidate_id == "safe"));
    }

    #[test]
    fn invalid_causal_design_is_rejected() {
        let mut c = candidate("invalid", 0.99, 0.01, 1.0);
        c.causal_validity.assignment_design_valid = false;
        assert!(matches!(
            choose_next(&state(0.5), &[c], &DecisionConfig::default()).unwrap(),
            Decision::Abstain {
                reason: AbstainReason::NoPermittedCandidate,
                ..
            }
        ));
    }

    #[test]
    fn low_delivery_fidelity_is_rejected() {
        let mut c = candidate("poor-delivery", 0.99, 0.01, 1.0);
        c.causal_validity.delivery_fidelity = 0.5;
        assert!(matches!(
            choose_next(&state(0.5), &[c], &DecisionConfig::default()).unwrap(),
            Decision::Abstain {
                reason: AbstainReason::NoPermittedCandidate,
                ..
            }
        ));
    }

    #[test]
    fn budget_is_a_hard_constraint() {
        let mut s = state(0.5);
        s.remaining_cost_budget = 0.5;
        assert!(matches!(
            choose_next(
                &s,
                &[candidate("costly", 0.99, 0.01, 1.0)],
                &DecisionConfig::default()
            )
            .unwrap(),
            Decision::Abstain {
                reason: AbstainReason::NoPermittedCandidate,
                ..
            }
        ));
    }

    #[test]
    fn no_information_causes_abstention() {
        assert!(matches!(
            choose_next(
                &state(0.5),
                &[candidate("none", 0.5, 0.5, 1.0)],
                &DecisionConfig::default()
            )
            .unwrap(),
            Decision::Abstain {
                reason: AbstainReason::NoInformativeCandidate,
                ..
            }
        ));
    }

    #[test]
    fn posterior_threshold_causes_preregistered_stop() {
        assert!(matches!(
            choose_next(
                &state(0.96),
                &[candidate("x", 0.9, 0.1, 1.0)],
                &DecisionConfig::default()
            )
            .unwrap(),
            Decision::Stop {
                reason: StopReason::PosteriorThresholdReached,
                ..
            }
        ));
    }

    #[test]
    fn predictive_failure_stops_for_model_revision() {
        let mut s = state(0.5);
        s.model_adequacy = ModelAdequacy::FailedPosteriorPredictiveCheck;
        assert!(matches!(
            choose_next(
                &s,
                &[candidate("x", 0.9, 0.1, 1.0)],
                &DecisionConfig::default()
            )
            .unwrap(),
            Decision::Stop {
                reason: StopReason::ModelInadequateRequestRevision,
                ..
            }
        ));
    }

    #[test]
    fn fixed_and_random_baselines_are_not_filtered_by_eig() {
        let c = candidate("low", 0.5, 0.5, 1.0);
        let fixed = DecisionConfig {
            policy: SelectionPolicy::FixedCanonicalOrder,
            ..DecisionConfig::default()
        };
        let random = DecisionConfig {
            policy: SelectionPolicy::DeterministicRandom { seed: 7 },
            ..DecisionConfig::default()
        };
        assert!(matches!(
            choose_next(&state(0.5), std::slice::from_ref(&c), &fixed).unwrap(),
            Decision::Select(_)
        ));
        assert!(matches!(
            choose_next(&state(0.5), &[c], &random).unwrap(),
            Decision::Select(_)
        ));
    }

    #[test]
    fn robust_policy_reports_worst_case_gain() {
        let mut s = state(0.5);
        s.ambiguity_posteriors = vec![posterior(0.99)];
        let c = candidate("robust", 0.9, 0.1, 1.0);
        let expected = expected_information_gain(&s.posterior, &c)
            .unwrap()
            .min(expected_information_gain(&s.ambiguity_posteriors[0], &c).unwrap());
        let result = choose_next(
            &s,
            &[c],
            &DecisionConfig {
                policy: SelectionPolicy::RobustMaximin,
                ..DecisionConfig::default()
            },
        )
        .unwrap();
        assert!(
            matches!(result, Decision::Select(s) if (s.robust_information_gain_nats - expected).abs() < 1.0e-12)
        );
    }

    #[test]
    fn candidate_order_does_not_change_deterministic_decision() {
        let a = candidate("alpha", 0.9, 0.1, 1.0);
        let b = candidate("beta", 0.7, 0.3, 1.0);
        let first = choose_next(
            &state(0.5),
            &[a.clone(), b.clone()],
            &DecisionConfig::default(),
        )
        .unwrap();
        let second = choose_next(&state(0.5), &[b, a], &DecisionConfig::default()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn random_policy_is_seed_reproducible() {
        let items = vec![candidate("a", 0.7, 0.3, 1.0), candidate("b", 0.8, 0.2, 1.0)];
        let config = DecisionConfig {
            policy: SelectionPolicy::DeterministicRandom { seed: 31 },
            ..DecisionConfig::default()
        };
        assert_eq!(
            choose_next(&state(0.5), &items, &config).unwrap(),
            choose_next(&state(0.5), &items, &config).unwrap()
        );
    }

    #[test]
    fn duplicate_candidate_ids_fail_closed() {
        let c = candidate("dup", 0.9, 0.1, 1.0);
        assert_eq!(
            choose_next(&state(0.5), &[c.clone(), c], &DecisionConfig::default()),
            Err(EngineError::DuplicateCandidate("dup".to_owned()))
        );
    }

    #[test]
    fn evidence_tier_is_bound_into_state_fingerprint() {
        let c = candidate("evidence", 0.9, 0.1, 1.0);
        let mut s = state(0.5);
        s.history.push(record(&c, "yes"));
        let before = state_fingerprint(&s, &DecisionConfig::default());
        s.history[0].evidence_tier = EvidenceTier::PeerReviewedMeasured;
        let after = state_fingerprint(&s, &DecisionConfig::default());
        assert_ne!(before, after);
    }

    #[test]
    fn outcome_order_does_not_change_eig() {
        let mut c = candidate("x", 0.8, 0.2, 1.0);
        let before = expected_information_gain(&posterior(0.5), &c).unwrap();
        c.outcomes.reverse();
        let after = expected_information_gain(&posterior(0.5), &c).unwrap();
        assert!((before - after).abs() < 1.0e-12);
    }

    #[test]
    fn forty_parameterized_synthetic_known_answer_cases() {
        // Eight operating/failure regimes × five parameter settings.
        // These are deterministic software fixtures, not biological observations.
        for regime in 0..8_u32 {
            for variant in 0..5_u32 {
                let p = 0.60 + f64::from(variant) * 0.075;
                let prior_a = 0.25 + f64::from(variant) * 0.10;
                let mut s = state(prior_a);
                let mut good = candidate(&format!("good-{regime}-{variant}"), p, 1.0 - p, 0.5);
                let mut poor = candidate(&format!("poor-{regime}-{variant}"), 0.52, 0.48, 0.8);
                let mut items = vec![poor.clone(), good.clone()];
                let mut config = DecisionConfig::default();
                match regime {
                    0 => {}
                    1 => {
                        items.reverse();
                    }
                    2 => {
                        good.outcomes.reverse();
                        items[1] = good;
                    }
                    3 => {
                        poor.feasibility = Feasibility::SafetyRejected {
                            reason: "fixture".into(),
                        };
                        items[0] = poor;
                    }
                    4 => {
                        poor.causal_validity.assignment_design_valid = false;
                        items[0] = poor;
                    }
                    5 => {
                        s.remaining_cost_budget = 0.5;
                    }
                    6 => {
                        s.ambiguity_posteriors = vec![posterior(0.5)];
                        config.policy = SelectionPolicy::RobustMaximin;
                    }
                    7 => {
                        items[1].causal_validity.delivery_fidelity = 0.2;
                    }
                    _ => unreachable!(),
                }
                let result = choose_next(&s, &items, &config).unwrap();
                if regime == 7 {
                    assert!(
                        matches!(result, Decision::Select(ref selection) if selection.candidate_id != format!("good-{regime}-{variant}"))
                    );
                } else {
                    assert!(
                        matches!(result, Decision::Select(_)),
                        "regime={regime}, variant={variant}: {result:?}"
                    );
                }
            }
        }
    }
}
