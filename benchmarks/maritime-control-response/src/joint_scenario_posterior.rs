#![forbid(unsafe_code)]

//! Persistent joint posterior over a diagnostic hypothesis and a likelihood
//! calibration/model scenario.
//!
//! The scenario variable is epistemic and persistent. It is not re-drawn for
//! every observation. Sequential updates therefore maintain p(H, S), then
//! derive p(H) and p(S) as marginals. This module is decision-support math only:
//! it does not acquire observations, authenticate sensor receipts, or execute
//! any inspection or intervention.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::active_probe::{
    candidate_manifest_sha256, hypothesis_manifest_sha256, rank_diagnostic_probes,
    select_probe_baseline, DiagnosticProbeCandidate, HypothesisPrior, ProbeBaselineSelection,
    ProbePortfolio, ProbeSelectionPolicy,
};

const PROBABILITY_TOLERANCE: f64 = 1e-9;
const LEDGER_SCHEMA: &str = "symtropy-joint-hypothesis-scenario-ledger-v1";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct JointPosteriorCell {
    pub hypothesis_id: String,
    pub scenario_id: String,
    pub probability: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioPosteriorMarginal {
    pub scenario_id: String,
    pub probability: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct JointScenarioPosteriorLedger {
    pub schema_version: String,
    pub candidate_id: String,
    pub candidate_manifest_sha256: String,
    pub hypothesis_space_sha256: String,
    pub scenario_manifest_sha256: String,
    /// Sorted immutable hypothesis IDs.
    pub hypothesis_ids: Vec<String>,
    /// Sorted persistent scenario IDs.
    pub scenario_ids: Vec<String>,
    /// Initial scenario prior weights, aligned with scenario_ids.
    pub initial_scenario_prior_weights: Vec<f64>,
    /// Current joint p(H,S), row-major by hypothesis_id then scenario_id.
    pub joint_posterior: Vec<JointPosteriorCell>,
    /// Observation receipts are retained in append order, not sorted.
    pub applied_observation_receipt_sha256: Vec<String>,
    pub parent_ledger_sha256: Option<String>,
    pub observation_count: u64,
    pub ledger_sha256: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct JointHypothesisMarginal {
    pub hypothesis_id: String,
    pub probability: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct JointScenarioPosteriorUpdate {
    pub candidate_id: String,
    pub candidate_manifest_sha256: String,
    pub observation_receipt_sha256: String,
    pub observed_outcome_id: String,
    pub previous_ledger_sha256: String,
    pub ledger: JointScenarioPosteriorLedger,
    /// P(observation | all evidence before this observation).
    pub observation_probability: f64,
    pub log_evidence_increment: f64,
    pub prior_joint_entropy_nats: f64,
    pub posterior_joint_entropy_nats: f64,
    /// Individual observations may increase entropy, so this may be negative.
    pub realized_joint_entropy_reduction_nats: f64,
    pub realized_joint_kl_divergence_nats: f64,
    pub posterior_hypotheses: Vec<JointHypothesisMarginal>,
    pub posterior_scenarios: Vec<ScenarioPosteriorMarginal>,
    pub posterior_hypothesis_entropy_nats: f64,
    pub posterior_scenario_entropy_nats: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JointPosteriorError {
    InvalidSelectionReplay,
    CandidateNotSelected,
    CandidateManifestMismatch,
    HypothesisSpaceMismatch,
    InvalidScenarioSet,
    InvalidPrior,
    InvalidObservationReceipt,
    EmptyOutcomeIdentity,
    OutcomeNotFound(String),
    OutcomeImpossible(String),
    DuplicateObservationReceipt,
    InvalidLedger,
    LedgerCandidateMismatch,
    Serialization,
    NumericalFailure,
}

impl fmt::Display for JointPosteriorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSelectionReplay => write!(formatter, "portfolio or baseline does not replay from the frozen selection inputs"),
            Self::CandidateNotSelected => write!(formatter, "candidate was not eligible and selected by the frozen baseline"),
            Self::CandidateManifestMismatch => write!(formatter, "candidate content does not match the ledger-bound manifest"),
            Self::HypothesisSpaceMismatch => write!(formatter, "candidate hypothesis space differs from the ledger-bound hypothesis space"),
            Self::InvalidScenarioSet => write!(formatter, "scenario IDs, weights, outcome spaces or likelihood tables are invalid"),
            Self::InvalidPrior => write!(formatter, "hypothesis prior or joint posterior does not form a finite probability distribution"),
            Self::InvalidObservationReceipt => write!(formatter, "observation receipt must be a lowercase SHA-256 digest"),
            Self::EmptyOutcomeIdentity => write!(formatter, "observed outcome identity is empty"),
            Self::OutcomeNotFound(id) => write!(formatter, "observed outcome is absent from a scenario likelihood table: {id}"),
            Self::OutcomeImpossible(id) => write!(formatter, "observed outcome has zero probability under the current joint posterior: {id}"),
            Self::DuplicateObservationReceipt => write!(formatter, "observation receipt has already been applied"),
            Self::InvalidLedger => write!(formatter, "joint posterior ledger identity, ordering or digest is invalid"),
            Self::LedgerCandidateMismatch => write!(formatter, "joint posterior ledger belongs to a different candidate or scenario catalogue"),
            Self::Serialization => write!(formatter, "failed to serialize canonical joint posterior evidence"),
            Self::NumericalFailure => write!(formatter, "joint posterior update is non-finite or fails normalization"),
        }
    }
}

impl std::error::Error for JointPosteriorError {}

/// Frozen selection context required to start a persistent multi-scenario
/// posterior. Only an eligible, selected candidate can create a ledger.
pub struct JointScenarioStartContext<'a> {
    pub hypotheses: &'a [HypothesisPrior],
    pub candidates: &'a [DiagnosticProbeCandidate],
    pub policy: &'a ProbeSelectionPolicy,
    pub portfolio: &'a ProbePortfolio,
    pub selection: &'a ProbeBaselineSelection,
    pub candidate: &'a DiagnosticProbeCandidate,
}

/// Create the joint prior p(H,S) = p(H) * p(S), where scenario weights are the
/// declared initial epistemic prior over a persistent, frozen scenario set.
/// All future observations update this same joint distribution.
pub fn new_joint_scenario_posterior_ledger(
    context: JointScenarioStartContext<'_>,
) -> Result<JointScenarioPosteriorLedger, JointPosteriorError> {
    let JointScenarioStartContext {
        hypotheses,
        candidates,
        policy,
        portfolio,
        selection,
        candidate,
    } = context;

    let replayed_portfolio = rank_diagnostic_probes(hypotheses, candidates, policy)
        .map_err(|_| JointPosteriorError::InvalidSelectionReplay)?;
    if &replayed_portfolio != portfolio {
        return Err(JointPosteriorError::InvalidSelectionReplay);
    }
    let replayed_selection = select_probe_baseline(
        &replayed_portfolio,
        candidates,
        selection.baseline,
        selection.seed_manifest_sha256.as_deref(),
        policy.cost_penalty_scale_seconds,
    )
    .map_err(|_| JointPosteriorError::InvalidSelectionReplay)?;
    if &replayed_selection != selection {
        return Err(JointPosteriorError::InvalidSelectionReplay);
    }
    if selection.selected_candidate_id.as_deref() != Some(candidate.candidate_id.as_str())
        || !portfolio.assessments.iter().any(|assessment| {
            assessment.candidate_id == candidate.candidate_id && assessment.eligible
        })
    {
        return Err(JointPosteriorError::CandidateNotSelected);
    }

    validate_candidate_scenarios(candidate)?;
    let candidate_digest = candidate_manifest_sha256(candidate)
        .map_err(|_| JointPosteriorError::CandidateManifestMismatch)?;
    if candidate_digest != candidate.candidate_manifest_sha256 {
        return Err(JointPosteriorError::CandidateManifestMismatch);
    }
    let canonical_hypotheses = canonical_hypotheses(hypotheses)?;
    let space_digest = hypothesis_manifest_sha256(&canonical_hypotheses)
        .map_err(|_| JointPosteriorError::HypothesisSpaceMismatch)?;
    if candidate.hypothesis_manifest_sha256 != space_digest {
        return Err(JointPosteriorError::HypothesisSpaceMismatch);
    }

    let scenario_weights = scenario_prior(candidate)?;
    let scenario_ids = scenario_weights
        .iter()
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let initial_scenario_prior_weights = scenario_weights
        .iter()
        .map(|(_, weight)| *weight)
        .collect::<Vec<_>>();
    let scenario_digest = scenario_manifest_sha256(
        candidate,
        &scenario_ids,
        &initial_scenario_prior_weights,
    )?;

    let mut joint_posterior = Vec::with_capacity(canonical_hypotheses.len() * scenario_ids.len());
    for hypothesis in &canonical_hypotheses {
        for (scenario_id, scenario_weight) in &scenario_weights {
            joint_posterior.push(JointPosteriorCell {
                hypothesis_id: hypothesis.hypothesis_id.clone(),
                scenario_id: scenario_id.clone(),
                probability: hypothesis.prior_probability * scenario_weight,
            });
        }
    }

    let mut ledger = JointScenarioPosteriorLedger {
        schema_version: LEDGER_SCHEMA.to_owned(),
        candidate_id: candidate.candidate_id.clone(),
        candidate_manifest_sha256: candidate.candidate_manifest_sha256.clone(),
        hypothesis_space_sha256: space_digest,
        scenario_manifest_sha256: scenario_digest,
        hypothesis_ids: canonical_hypotheses
            .iter()
            .map(|item| item.hypothesis_id.clone())
            .collect(),
        scenario_ids,
        initial_scenario_prior_weights,
        joint_posterior,
        applied_observation_receipt_sha256: Vec::new(),
        parent_ledger_sha256: None,
        observation_count: 0,
        ledger_sha256: String::new(),
    };
    ledger.ledger_sha256 = ledger_digest(&ledger)?;
    validate_joint_ledger(&ledger)?;
    Ok(ledger)
}

/// Apply one externally acquired outcome to the persistent joint distribution.
/// The supplied receipt digest is content-bound but its signer, timestamp,
/// sensor source and authority must be verified by the acquisition/trust layer.
pub fn realize_joint_scenario_observation(
    ledger: &JointScenarioPosteriorLedger,
    candidate: &DiagnosticProbeCandidate,
    observed_outcome_id: &str,
    observation_receipt_sha256: &str,
) -> Result<JointScenarioPosteriorUpdate, JointPosteriorError> {
    validate_joint_ledger(ledger)?;
    validate_candidate_scenarios(candidate)?;
    if observed_outcome_id.trim().is_empty() {
        return Err(JointPosteriorError::EmptyOutcomeIdentity);
    }
    if !is_sha256_hex(observation_receipt_sha256) {
        return Err(JointPosteriorError::InvalidObservationReceipt);
    }
    if ledger
        .applied_observation_receipt_sha256
        .iter()
        .any(|receipt| receipt == observation_receipt_sha256)
    {
        return Err(JointPosteriorError::DuplicateObservationReceipt);
    }
    let candidate_digest = candidate_manifest_sha256(candidate)
        .map_err(|_| JointPosteriorError::CandidateManifestMismatch)?;
    if candidate_digest != candidate.candidate_manifest_sha256
        || candidate.candidate_id != ledger.candidate_id
        || candidate.candidate_manifest_sha256 != ledger.candidate_manifest_sha256
    {
        return Err(JointPosteriorError::LedgerCandidateMismatch);
    }
    let space_digest = hypothesis_manifest_sha256_from_ids(&ledger.hypothesis_ids)?;
    if space_digest != ledger.hypothesis_space_sha256
        || candidate.hypothesis_manifest_sha256 != ledger.hypothesis_space_sha256
    {
        return Err(JointPosteriorError::HypothesisSpaceMismatch);
    }
    let scenario_weights = scenario_prior(candidate)?;
    let scenario_ids = scenario_weights
        .iter()
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let weights = scenario_weights
        .iter()
        .map(|(_, weight)| *weight)
        .collect::<Vec<_>>();
    if scenario_ids != ledger.scenario_ids
        || weights != ledger.initial_scenario_prior_weights
        || scenario_manifest_sha256(candidate, &scenario_ids, &weights)?
            != ledger.scenario_manifest_sha256
    {
        return Err(JointPosteriorError::LedgerCandidateMismatch);
    }

    let scenario_outcomes = candidate
        .uncertainty_scenarios
        .iter()
        .map(|scenario| {
            scenario
                .outcomes
                .iter()
                .find(|outcome| outcome.outcome_id == observed_outcome_id)
                .map(|outcome| (scenario.scenario_id.as_str(), outcome))
                .ok_or_else(|| JointPosteriorError::OutcomeNotFound(observed_outcome_id.to_owned()))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    let previous_ledger_sha256 = ledger.ledger_sha256.clone();
    let prior_joint_probabilities = ledger
        .joint_posterior
        .iter()
        .map(|cell| cell.probability)
        .collect::<Vec<_>>();
    let prior_joint_entropy = entropy(&prior_joint_probabilities);
    let mut unnormalized = Vec::with_capacity(ledger.joint_posterior.len());
    for cell in &ledger.joint_posterior {
        let hypothesis_index = ledger
            .hypothesis_ids
            .binary_search(&cell.hypothesis_id)
            .map_err(|_| JointPosteriorError::InvalidLedger)?;
        let outcome = scenario_outcomes
            .get(cell.scenario_id.as_str())
            .ok_or(JointPosteriorError::InvalidLedger)?;
        let likelihood = outcome.likelihood_by_hypothesis[hypothesis_index];
        unnormalized.push(cell.probability * likelihood);
    }
    let observation_probability = unnormalized.iter().sum::<f64>();
    if !observation_probability.is_finite() || observation_probability <= f64::EPSILON {
        return Err(JointPosteriorError::OutcomeImpossible(observed_outcome_id.to_owned()));
    }
    let posterior_probabilities = unnormalized
        .iter()
        .map(|value| value / observation_probability)
        .collect::<Vec<_>>();
    if posterior_probabilities
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0 || *value > 1.0)
        || (posterior_probabilities.iter().sum::<f64>() - 1.0).abs() > 1e-8
    {
        return Err(JointPosteriorError::NumericalFailure);
    }

    let posterior_joint_entropy = entropy(&posterior_probabilities);
    let realized_kl = posterior_probabilities
        .iter()
        .zip(&prior_joint_probabilities)
        .filter(|(posterior, _)| **posterior > 0.0)
        .map(|(posterior, prior)| posterior * (posterior / prior).ln())
        .sum::<f64>();
    if !realized_kl.is_finite() || realized_kl < -1e-8 {
        return Err(JointPosteriorError::NumericalFailure);
    }

    let mut posterior_cells = ledger.joint_posterior.clone();
    for (cell, probability) in posterior_cells.iter_mut().zip(posterior_probabilities) {
        cell.probability = probability;
    }
    let hypothesis_marginals = marginalize_hypotheses(&posterior_cells, &ledger.hypothesis_ids);
    let scenario_marginals = marginalize_scenarios(&posterior_cells, &ledger.scenario_ids);
    let hypothesis_entropy = entropy(
        &hypothesis_marginals
            .iter()
            .map(|item| item.probability)
            .collect::<Vec<_>>(),
    );
    let scenario_entropy = entropy(
        &scenario_marginals
            .iter()
            .map(|item| item.probability)
            .collect::<Vec<_>>(),
    );

    let mut updated_ledger = ledger.clone();
    updated_ledger.joint_posterior = posterior_cells;
    updated_ledger.parent_ledger_sha256 = Some(previous_ledger_sha256.clone());
    updated_ledger
        .applied_observation_receipt_sha256
        .push(observation_receipt_sha256.to_owned());
    updated_ledger.observation_count += 1;
    updated_ledger.ledger_sha256 = ledger_digest(&updated_ledger)?;
    validate_joint_ledger(&updated_ledger)?;

    Ok(JointScenarioPosteriorUpdate {
        candidate_id: candidate.candidate_id.clone(),
        candidate_manifest_sha256: candidate.candidate_manifest_sha256.clone(),
        observation_receipt_sha256: observation_receipt_sha256.to_owned(),
        observed_outcome_id: observed_outcome_id.to_owned(),
        previous_ledger_sha256,
        ledger: updated_ledger,
        observation_probability,
        log_evidence_increment: observation_probability.ln(),
        prior_joint_entropy_nats: prior_joint_entropy,
        posterior_joint_entropy_nats: posterior_joint_entropy,
        realized_joint_entropy_reduction_nats: prior_joint_entropy - posterior_joint_entropy,
        realized_joint_kl_divergence_nats: realized_kl.max(0.0),
        posterior_hypotheses: hypothesis_marginals,
        posterior_scenarios: scenario_marginals,
        posterior_hypothesis_entropy_nats: hypothesis_entropy,
        posterior_scenario_entropy_nats: scenario_entropy,
    })
}

fn validate_candidate_scenarios(
    candidate: &DiagnosticProbeCandidate,
) -> Result<(), JointPosteriorError> {
    if !candidate.outcomes.is_empty() || candidate.uncertainty_scenarios.len() < 2 {
        return Err(JointPosteriorError::InvalidScenarioSet);
    }
    let mut scenario_ids = BTreeSet::new();
    let mut outcome_ids: Option<BTreeSet<String>> = None;
    let hypothesis_count = candidate.uncertainty_scenarios[0]
        .outcomes
        .first()
        .map(|outcome| outcome.likelihood_by_hypothesis.len())
        .unwrap_or(0);
    if hypothesis_count < 2 {
        return Err(JointPosteriorError::InvalidScenarioSet);
    }

    for scenario in &candidate.uncertainty_scenarios {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.clone())
            || !scenario.scenario_weight.is_finite()
            || scenario.scenario_weight <= 0.0
            || scenario.scenario_weight > 1.0
            || scenario.outcomes.len() < 2
        {
            return Err(JointPosteriorError::InvalidScenarioSet);
        }
        let mut this_outcome_ids = BTreeSet::new();
        for outcome in &scenario.outcomes {
            if outcome.outcome_id.trim().is_empty()
                || !this_outcome_ids.insert(outcome.outcome_id.clone())
                || outcome.likelihood_by_hypothesis.len() != hypothesis_count
                || outcome
                    .likelihood_by_hypothesis
                    .iter()
                    .any(|value| !value.is_finite() || *value < 0.0 || *value > 1.0)
            {
                return Err(JointPosteriorError::InvalidScenarioSet);
            }
        }
        if outcome_ids
            .as_ref()
            .is_some_and(|expected| expected != &this_outcome_ids)
        {
            return Err(JointPosteriorError::InvalidScenarioSet);
        }
        outcome_ids.get_or_insert(this_outcome_ids);
        for index in 0..hypothesis_count {
            let sum = scenario
                .outcomes
                .iter()
                .map(|outcome| outcome.likelihood_by_hypothesis[index])
                .sum::<f64>();
            if (sum - 1.0).abs() > PROBABILITY_TOLERANCE {
                return Err(JointPosteriorError::InvalidScenarioSet);
            }
        }
    }
    let weight_sum = candidate
        .uncertainty_scenarios
        .iter()
        .map(|scenario| scenario.scenario_weight)
        .sum::<f64>();
    if (weight_sum - 1.0).abs() > PROBABILITY_TOLERANCE
        || !is_sha256_hex(&candidate.candidate_manifest_sha256)
        || candidate_manifest_sha256(candidate)
            .map_err(|_| JointPosteriorError::CandidateManifestMismatch)?
            != candidate.candidate_manifest_sha256
    {
        return Err(JointPosteriorError::InvalidScenarioSet);
    }
    Ok(())
}

fn scenario_prior(
    candidate: &DiagnosticProbeCandidate,
) -> Result<Vec<(String, f64)>, JointPosteriorError> {
    let mut scenarios = candidate
        .uncertainty_scenarios
        .iter()
        .map(|scenario| (scenario.scenario_id.clone(), scenario.scenario_weight))
        .collect::<Vec<_>>();
    scenarios.sort_by(|left, right| left.0.cmp(&right.0));
    if scenarios
        .windows(2)
        .any(|pair| pair[0].0 == pair[1].0)
        || (scenarios.iter().map(|item| item.1).sum::<f64>() - 1.0).abs()
            > PROBABILITY_TOLERANCE
    {
        return Err(JointPosteriorError::InvalidScenarioSet);
    }
    Ok(scenarios)
}

fn canonical_hypotheses(
    hypotheses: &[HypothesisPrior],
) -> Result<Vec<HypothesisPrior>, JointPosteriorError> {
    if hypotheses.len() < 2 {
        return Err(JointPosteriorError::InvalidPrior);
    }
    let mut values = hypotheses.to_vec();
    values.sort_by(|left, right| left.hypothesis_id.cmp(&right.hypothesis_id));
    let mut ids = BTreeSet::new();
    for item in &values {
        if item.hypothesis_id.trim().is_empty()
            || !ids.insert(item.hypothesis_id.clone())
            || !item.prior_probability.is_finite()
            || item.prior_probability < 0.0
            || item.prior_probability > 1.0
        {
            return Err(JointPosteriorError::InvalidPrior);
        }
    }
    if (values.iter().map(|item| item.prior_probability).sum::<f64>() - 1.0).abs()
        > PROBABILITY_TOLERANCE
    {
        return Err(JointPosteriorError::InvalidPrior);
    }
    Ok(values)
}

fn hypothesis_manifest_sha256_from_ids(ids: &[String]) -> Result<String, JointPosteriorError> {
    if ids.len() < 2
        || ids.iter().any(|id| id.trim().is_empty())
        || ids.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(JointPosteriorError::InvalidLedger);
    }
    sha256_serializable(
        b"symtropy-diagnostic-hypothesis-space-v1\0",
        &ids,
    )
}

fn scenario_manifest_sha256(
    candidate: &DiagnosticProbeCandidate,
    scenario_ids: &[String],
    weights: &[f64],
) -> Result<String, JointPosteriorError> {
    if scenario_ids.len() < 2
        || scenario_ids.len() != weights.len()
        || scenario_ids.iter().any(|id| id.trim().is_empty())
        || scenario_ids.windows(2).any(|pair| pair[0] >= pair[1])
        || weights.iter().any(|weight| !weight.is_finite() || *weight <= 0.0)
        || (weights.iter().sum::<f64>() - 1.0).abs() > PROBABILITY_TOLERANCE
    {
        return Err(JointPosteriorError::InvalidScenarioSet);
    }
    sha256_serializable(
        b"symtropy-diagnostic-persistent-scenario-manifest-v1\0",
        &(
            candidate.likelihood_model_sha256.as_str(),
            candidate.likelihood_calibration_sha256.as_str(),
            candidate.likelihood_calibration_review_receipt_sha256.as_str(),
            candidate.likelihood_calibration_reviewed_by_independent_validator,
            scenario_ids,
            weights,
        ),
    )
}

fn ledger_digest(ledger: &JointScenarioPosteriorLedger) -> Result<String, JointPosteriorError> {
    sha256_serializable(
        b"symtropy-joint-hypothesis-scenario-ledger-v1\0",
        &(
            ledger.schema_version.as_str(),
            ledger.candidate_id.as_str(),
            ledger.candidate_manifest_sha256.as_str(),
            ledger.hypothesis_space_sha256.as_str(),
            ledger.scenario_manifest_sha256.as_str(),
            &ledger.hypothesis_ids,
            &ledger.scenario_ids,
            &ledger.initial_scenario_prior_weights,
            &ledger.joint_posterior,
            &ledger.applied_observation_receipt_sha256,
            &ledger.parent_ledger_sha256,
            ledger.observation_count,
        ),
    )
}

fn validate_joint_ledger(
    ledger: &JointScenarioPosteriorLedger,
) -> Result<(), JointPosteriorError> {
    if ledger.schema_version != LEDGER_SCHEMA
        || ledger.candidate_id.trim().is_empty()
        || !is_sha256_hex(&ledger.candidate_manifest_sha256)
        || !is_sha256_hex(&ledger.hypothesis_space_sha256)
        || !is_sha256_hex(&ledger.scenario_manifest_sha256)
        || !is_sha256_hex(&ledger.ledger_sha256)
        || ledger.hypothesis_ids.len() < 2
        || ledger.scenario_ids.len() < 2
        || ledger.hypothesis_ids.windows(2).any(|pair| pair[0] >= pair[1])
        || ledger.scenario_ids.windows(2).any(|pair| pair[0] >= pair[1])
        || ledger.initial_scenario_prior_weights.len() != ledger.scenario_ids.len()
        || ledger.initial_scenario_prior_weights.iter().any(|weight| !weight.is_finite() || *weight <= 0.0)
        || (ledger.initial_scenario_prior_weights.iter().sum::<f64>() - 1.0).abs() > PROBABILITY_TOLERANCE
        || ledger.applied_observation_receipt_sha256.len() as u64 != ledger.observation_count
        || ledger.applied_observation_receipt_sha256.iter().any(|receipt| !is_sha256_hex(receipt))
        || ledger.applied_observation_receipt_sha256.iter().collect::<BTreeSet<_>>().len()
            != ledger.applied_observation_receipt_sha256.len()
        || (ledger.observation_count == 0) != ledger.parent_ledger_sha256.is_none()
        || ledger.parent_ledger_sha256.as_ref().is_some_and(|digest| !is_sha256_hex(digest))
        || ledger.joint_posterior.len() != ledger.hypothesis_ids.len() * ledger.scenario_ids.len()
    {
        return Err(JointPosteriorError::InvalidLedger);
    }
    let expected_hypothesis_ids = ledger.hypothesis_ids.iter().cloned().collect::<BTreeSet<_>>();
    let expected_scenario_ids = ledger.scenario_ids.iter().cloned().collect::<BTreeSet<_>>();
    let mut total = 0.0;
    let mut prior_cell_index = 0usize;
    for hypothesis_id in &ledger.hypothesis_ids {
        for scenario_id in &ledger.scenario_ids {
            let cell = ledger
                .joint_posterior
                .get(prior_cell_index)
                .ok_or(JointPosteriorError::InvalidLedger)?;
            if &cell.hypothesis_id != hypothesis_id
                || &cell.scenario_id != scenario_id
                || !cell.probability.is_finite()
                || cell.probability < 0.0
                || cell.probability > 1.0
            {
                return Err(JointPosteriorError::InvalidLedger);
            }
            total += cell.probability;
            prior_cell_index += 1;
        }
    }
    if expected_hypothesis_ids.len() != ledger.hypothesis_ids.len()
        || expected_scenario_ids.len() != ledger.scenario_ids.len()
        || !total.is_finite()
        || (total - 1.0).abs() > 1e-8
        || hypothesis_manifest_sha256_from_ids(&ledger.hypothesis_ids)?
            != ledger.hypothesis_space_sha256
        || ledger_digest(ledger)? != ledger.ledger_sha256
    {
        return Err(JointPosteriorError::InvalidLedger);
    }
    Ok(())
}

fn marginalize_hypotheses(
    cells: &[JointPosteriorCell],
    hypothesis_ids: &[String],
) -> Vec<JointHypothesisMarginal> {
    hypothesis_ids
        .iter()
        .map(|id| JointHypothesisMarginal {
            hypothesis_id: id.clone(),
            probability: cells
                .iter()
                .filter(|cell| &cell.hypothesis_id == id)
                .map(|cell| cell.probability)
                .sum(),
        })
        .collect()
}

fn marginalize_scenarios(
    cells: &[JointPosteriorCell],
    scenario_ids: &[String],
) -> Vec<ScenarioPosteriorMarginal> {
    scenario_ids
        .iter()
        .map(|id| ScenarioPosteriorMarginal {
            scenario_id: id.clone(),
            probability: cells
                .iter()
                .filter(|cell| &cell.scenario_id == id)
                .map(|cell| cell.probability)
                .sum(),
        })
        .collect()
}

fn entropy(probabilities: &[f64]) -> f64 {
    -probabilities
        .iter()
        .filter(|probability| **probability > 0.0)
        .map(|probability| probability * probability.ln())
        .sum::<f64>()
}

fn sha256_serializable<T: Serialize>(
    domain: &[u8],
    value: &T,
) -> Result<String, JointPosteriorError> {
    let encoded = serde_json::to_vec(value).map_err(|_| JointPosteriorError::Serialization)?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((encoded.len() as u64).to_le_bytes());
    hasher.update(encoded);
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_probe::{
        candidate_manifest_sha256, hypothesis_manifest_sha256, DiagnosticProbeCandidate,
        ProbeAuthorization, ProbeBaseline, ProbeLikelihoodScenario, ProbeOutcomeLikelihood,
        ProbeSelectionPolicy, rank_diagnostic_probes, select_probe_baseline,
    };

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn hypotheses() -> Vec<HypothesisPrior> {
        vec![
            HypothesisPrior {
                hypothesis_id: "bearing_degradation".into(),
                prior_probability: 0.7,
            },
            HypothesisPrior {
                hypothesis_id: "fuel_delivery_fault".into(),
                prior_probability: 0.3,
            },
        ]
    }

    fn outcome_table(bearing: [f64; 2]) -> Vec<ProbeOutcomeLikelihood> {
        vec![
            ProbeOutcomeLikelihood {
                outcome_id: "bearing_signature".into(),
                likelihood_by_hypothesis: bearing.to_vec(),
            },
            ProbeOutcomeLikelihood {
                outcome_id: "fuel_signature".into(),
                likelihood_by_hypothesis: (0..2).map(|i| 1.0 - bearing[i]).collect(),
            },
        ]
    }

    fn candidate() -> DiagnosticProbeCandidate {
        let prior = hypotheses();
        let mut candidate = DiagnosticProbeCandidate {
            candidate_id: "pressure_observation".into(),
            probe_class: crate::active_probe::ProbeClass::PassiveMeasurement,
            authorization: ProbeAuthorization::ApprovedReadOnly,
            authorization_receipt_sha256: Some(digest('a')),
            likelihood_model_sha256: digest('b'),
            likelihood_calibration_sha256: digest('c'),
            likelihood_calibration_review_receipt_sha256: digest('d'),
            likelihood_calibration_reviewed_by_independent_validator: true,
            risk_assessment_sha256: digest('e'),
            risk_assessment_reviewed_by_authority_owner: true,
            hypothesis_manifest_sha256: hypothesis_manifest_sha256(&prior).unwrap(),
            expected_cost_seconds: 30.0,
            expected_delay_seconds: 10.0,
            risk_score_ppm: 500,
            outcomes: Vec::new(),
            uncertainty_scenarios: vec![
                ProbeLikelihoodScenario {
                    scenario_id: "nominal".into(),
                    scenario_weight: 0.5,
                    outcomes: outcome_table([0.95, 0.05]),
                },
                ProbeLikelihoodScenario {
                    scenario_id: "degraded_calibration".into(),
                    scenario_weight: 0.5,
                    outcomes: outcome_table([0.60, 0.40]),
                },
            ],
            candidate_manifest_sha256: digest('0'),
        };
        candidate.candidate_manifest_sha256 = candidate_manifest_sha256(&candidate).unwrap();
        candidate
    }

    fn initial_ledger(item: &DiagnosticProbeCandidate) -> JointScenarioPosteriorLedger {
        let hypotheses = hypotheses();
        let candidates = vec![item.clone()];
        let policy = ProbeSelectionPolicy::conservative_read_only();
        let portfolio = rank_diagnostic_probes(&hypotheses, &candidates, &policy).unwrap();
        let selection = select_probe_baseline(
            &portfolio,
            &candidates,
            ProbeBaseline::RiskAdjustedWorstCaseInformationGain,
            None,
            policy.cost_penalty_scale_seconds,
        )
        .unwrap();
        new_joint_scenario_posterior_ledger(JointScenarioStartContext {
            hypotheses: &hypotheses,
            candidates: &candidates,
            policy: &policy,
            portfolio: &portfolio,
            selection: &selection,
            candidate: item,
        })
        .unwrap()
    }

    #[test]
    fn genesis_ledger_is_joint_normalized_and_binds_selection_and_scenario_priors() {
        let item = candidate();
        let ledger = initial_ledger(&item);
        assert_eq!(ledger.schema_version, LEDGER_SCHEMA);
        assert_eq!(ledger.hypothesis_ids, vec!["bearing_degradation", "fuel_delivery_fault"]);
        assert_eq!(ledger.scenario_ids, vec!["degraded_calibration", "nominal"]);
        assert_eq!(ledger.observation_count, 0);
        assert_eq!(ledger.applied_observation_receipt_sha256.len(), 0);
        assert!((ledger.joint_posterior.iter().map(|cell| cell.probability).sum::<f64>() - 1.0).abs() < 1e-12);
        assert!(ledger.parent_ledger_sha256.is_none());
        assert!(validate_joint_ledger(&ledger).is_ok());
    }

    #[test]
    fn persistent_scenario_posterior_updates_after_each_observation() {
        let item = candidate();
        let genesis = initial_ledger(&item);
        let first = realize_joint_scenario_observation(
            &genesis,
            &item,
            "bearing_signature",
            &digest('f'),
        )
        .unwrap();
        let initial_weights = &genesis.initial_scenario_prior_weights;
        assert_ne!(
            first.posterior_scenarios[0].probability,
            initial_weights[0]
        );
        assert_eq!(first.ledger.observation_count, 1);
        assert_eq!(first.ledger.parent_ledger_sha256.as_deref(), Some(genesis.ledger_sha256.as_str()));
        assert!((first.posterior_hypotheses.iter().map(|entry| entry.probability).sum::<f64>() - 1.0).abs() < 1e-12);
        assert!((first.posterior_scenarios.iter().map(|entry| entry.probability).sum::<f64>() - 1.0).abs() < 1e-12);

        let second = realize_joint_scenario_observation(
            &first.ledger,
            &item,
            "bearing_signature",
            &digest('9'),
        )
        .unwrap();
        assert_eq!(second.ledger.observation_count, 2);
        assert_eq!(second.ledger.parent_ledger_sha256.as_deref(), Some(first.ledger.ledger_sha256.as_str()));
        assert_ne!(
            second.posterior_scenarios[0].probability,
            first.posterior_scenarios[0].probability
        );
        assert!(second.realized_joint_kl_divergence_nats > 0.0);
    }

    #[test]
    fn same_likelihood_scenarios_are_not_redrawn_at_each_sample() {
        let item = candidate();
        let genesis = initial_ledger(&item);
        let first = realize_joint_scenario_observation(
            &genesis,
            &item,
            "bearing_signature",
            &digest('f'),
        )
        .unwrap();
        let second = realize_joint_scenario_observation(
            &first.ledger,
            &item,
            "fuel_signature",
            &digest('9'),
        )
        .unwrap();

        let nominal = second
            .posterior_scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "nominal")
            .unwrap()
            .probability;
        let degraded = second
            .posterior_scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "degraded_calibration")
            .unwrap()
            .probability;
        assert!((nominal + degraded - 1.0).abs() < 1e-12);
        assert_ne!(nominal, 0.5);
    }

    #[test]
    fn receipt_replay_is_append_only_and_duplicate_receipts_are_rejected() {
        let item = candidate();
        let ledger = initial_ledger(&item);
        let first = realize_joint_scenario_observation(
            &ledger,
            &item,
            "bearing_signature",
            &digest('f'),
        )
        .unwrap();
        assert_eq!(
            realize_joint_scenario_observation(
                &first.ledger,
                &item,
                "fuel_signature",
                &digest('f'),
            ),
            Err(JointPosteriorError::DuplicateObservationReceipt)
        );
        assert_eq!(first.ledger.applied_observation_receipt_sha256, vec![digest('f')]);
    }

    #[test]
    fn tampered_joint_ledger_or_different_candidate_fails_closed() {
        let item = candidate();
        let ledger = initial_ledger(&item);
        let mut tampered = ledger.clone();
        tampered.joint_posterior[0].probability += 0.1;
        assert_eq!(
            realize_joint_scenario_observation(
                &tampered,
                &item,
                "bearing_signature",
                &digest('f'),
            ),
            Err(JointPosteriorError::InvalidLedger)
        );

        let mut changed = item.clone();
        changed.expected_cost_seconds += 5.0;
        changed.candidate_manifest_sha256 = candidate_manifest_sha256(&changed).unwrap();
        assert_eq!(
            realize_joint_scenario_observation(
                &ledger,
                &changed,
                "bearing_signature",
                &digest('f'),
            ),
            Err(JointPosteriorError::LedgerCandidateMismatch)
        );
    }

    #[test]
    fn impossible_outcome_and_malformed_receipt_fail_closed() {
        let item = candidate();
        let ledger = initial_ledger(&item);
        assert_eq!(
            realize_joint_scenario_observation(
                &ledger,
                &item,
                "absent_outcome",
                &digest('f'),
            ),
            Err(JointPosteriorError::OutcomeNotFound("absent_outcome".into()))
        );
        assert_eq!(
            realize_joint_scenario_observation(
                &ledger,
                &item,
                "bearing_signature",
                "not-a-digest",
            ),
            Err(JointPosteriorError::InvalidObservationReceipt)
        );
    }

    #[test]
    fn posterior_is_independent_of_candidate_scenario_input_order() {
        let mut item = candidate();
        item.uncertainty_scenarios.reverse();
        item.candidate_manifest_sha256 = candidate_manifest_sha256(&item).unwrap();
        let ledger = initial_ledger(&item);
        assert_eq!(ledger.scenario_ids, vec!["degraded_calibration", "nominal"]);
        assert!(validate_joint_ledger(&ledger).is_ok());
    }
}
