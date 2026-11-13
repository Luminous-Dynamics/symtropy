#![forbid(unsafe_code)]

//! Read-only value-of-information ranking for engineering diagnosis.
//!
//! This module selects evidence to acquire; it never executes a probe, changes
//! controller state, or authorizes actuation. Passive measurements and explicitly
//! authorized operator inspections may be ranked. Controlled perturbations are
//! excluded and must use a separate safety/authority process.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

const PROBABILITY_TOLERANCE: f64 = 1e-9;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HypothesisPrior {
    pub hypothesis_id: String,
    pub prior_probability: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeClass {
    PassiveMeasurement,
    OperatorInspection,
    ControlledPerturbation,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeAuthorization {
    ApprovedReadOnly,
    HumanAuthorized,
    Missing,
    Denied,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProbeOutcomeLikelihood {
    pub outcome_id: String,
    /// Conditional outcome probabilities aligned with hypothesis IDs in
    /// lexicographic order. Each hypothesis column must sum to one.
    pub likelihood_by_hypothesis: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProbeLikelihoodScenario {
    pub scenario_id: String,
    pub scenario_weight: f64,
    pub outcomes: Vec<ProbeOutcomeLikelihood>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticProbeCandidate {
    pub candidate_id: String,
    pub probe_class: ProbeClass,
    pub authorization: ProbeAuthorization,
    /// Digest reference only; signer identity and authority must be verified upstream.
    pub authorization_receipt_sha256: Option<String>,
    pub likelihood_model_sha256: String,
    pub likelihood_calibration_sha256: String,
    /// Digest reference for an independent likelihood-calibration review receipt.
    pub likelihood_calibration_review_receipt_sha256: String,
    /// Caller-supplied review flag; reviewer identity/signature are verified upstream.
    pub likelihood_calibration_reviewed_by_independent_validator: bool,
    /// Digest of the independently reviewed risk assessment backing risk_score_ppm.
    pub risk_assessment_sha256: String,
    /// Caller-supplied review flag; issuer identity and signature are verified upstream.
    pub risk_assessment_reviewed_by_authority_owner: bool,
    pub hypothesis_manifest_sha256: String,
    pub expected_cost_seconds: f64,
    pub expected_delay_seconds: f64,
    /// Risk estimate in parts per million, 0..=1_000_000.
    pub risk_score_ppm: u32,
    /// Single point-estimate likelihood table, used only when no uncertainty scenarios are supplied.
    pub outcomes: Vec<ProbeOutcomeLikelihood>,
    /// Optional calibrated likelihood scenarios spanning declared model/regime uncertainty.
    /// When non-empty, outcomes must be empty and robust eligibility uses worst-case EIG.
    pub uncertainty_scenarios: Vec<ProbeLikelihoodScenario>,
    /// Digest over every candidate field other than this digest.
    pub candidate_manifest_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProbeSelectionPolicy {
    pub allow_operator_inspection: bool,
    pub maximum_cost_seconds: f64,
    pub maximum_delay_seconds: f64,
    pub maximum_risk_ppm: u32,
    pub minimum_expected_information_gain_nats: f64,
    pub cost_penalty_scale_seconds: f64,
    pub delay_penalty_scale_seconds: f64,
    pub risk_penalty_scale_ppm: f64,
}

impl ProbeSelectionPolicy {
    pub fn conservative_read_only() -> Self {
        Self {
            allow_operator_inspection: true,
            maximum_cost_seconds: 3_600.0,
            maximum_delay_seconds: 86_400.0,
            maximum_risk_ppm: 10_000,
            minimum_expected_information_gain_nats: 1e-6,
            cost_penalty_scale_seconds: 300.0,
            delay_penalty_scale_seconds: 3_600.0,
            risk_penalty_scale_ppm: 10_000.0,
        }
    }

    fn validate(&self) -> Result<(), ProbeError> {
        if !self.maximum_cost_seconds.is_finite()
            || self.maximum_cost_seconds < 0.0
            || !self.maximum_delay_seconds.is_finite()
            || self.maximum_delay_seconds < 0.0
            || self.maximum_risk_ppm > 1_000_000
            || !self.minimum_expected_information_gain_nats.is_finite()
            || self.minimum_expected_information_gain_nats < 0.0
            || !self.cost_penalty_scale_seconds.is_finite()
            || self.cost_penalty_scale_seconds <= 0.0
            || !self.delay_penalty_scale_seconds.is_finite()
            || self.delay_penalty_scale_seconds <= 0.0
            || !self.risk_penalty_scale_ppm.is_finite()
            || self.risk_penalty_scale_ppm <= 0.0
        {
            return Err(ProbeError::InvalidPolicy);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeRejectionReason {
    ControlledPerturbationRequiresSeparateSafetyCase,
    OperatorInspectionDisabled,
    AuthorizationNotApproved,
    AuthorizationReceiptMissingOrInvalid,
    CostBudgetExceeded,
    DelayBudgetExceeded,
    RiskBudgetExceeded,
    RiskEvidenceNotReviewed,
    LikelihoodCalibrationNotReviewed,
    InformationGainBelowMinimum,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ProbeAssessment {
    pub candidate_id: String,
    pub eligible: bool,
    pub rejection_reasons: Vec<ProbeRejectionReason>,
    pub expected_information_gain_nats: f64,
    pub worst_case_information_gain_nats: f64,
    pub expected_posterior_entropy_nats: f64,
    pub scenario_count: usize,
    pub multi_scenario_assessed: bool,
    pub risk_adjusted_utility: Option<f64>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct RankedDiagnosticProbe {
    pub rank: usize,
    pub candidate_id: String,
    pub probe_class: ProbeClass,
    pub expected_information_gain_nats: f64,
    pub worst_case_information_gain_nats: f64,
    pub expected_posterior_entropy_nats: f64,
    pub scenario_count: usize,
    pub multi_scenario_assessed: bool,
    pub risk_adjusted_utility: f64,
    pub expected_cost_seconds: f64,
    pub expected_delay_seconds: f64,
    pub risk_score_ppm: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeBaseline {
    NoAdditionalEvidence,
    CheapestEligible,
    MaximumInformationGain,
    InformationGainPerCost,
    SeededRandomEligible,
    RiskAdjustedWorstCaseInformationGain,
    MaximumWorstCaseChernoffInformation,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ProbeBaselineSelection {
    pub baseline: ProbeBaseline,
    pub selected_candidate_id: Option<String>,
    pub eligible_candidate_count: usize,
    pub candidate_catalogue_sha256: String,
    pub eligible_catalogue_sha256: String,
    pub seed_manifest_sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ProbePosteriorEntry {
    pub hypothesis_id: String,
    pub probability: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProbeObservationLedger {
    pub schema_version: String,
    pub current_posterior_manifest_sha256: String,
    /// Strictly sorted, unique observation receipt digests already applied.
    pub applied_observation_receipt_sha256: Vec<String>,
    pub ledger_sha256: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ProbeObservationUpdate {
    pub candidate_id: String,
    pub candidate_manifest_sha256: String,
    pub observation_receipt_sha256: String,
    pub observed_outcome_id: String,
    pub prior_entropy_nats: f64,
    pub previous_posterior_manifest_sha256: String,
    pub posterior_manifest_sha256: String,
    pub observation_ledger: ProbeObservationLedger,
    pub posterior_entropy_nats: f64,
    /// May be negative for an individual observation even though expected information gain is non-negative.
    pub realized_entropy_reduction_nats: f64,
    /// KL(posterior || prior), non-negative up to floating-point tolerance.
    pub realized_kl_divergence_nats: f64,
    pub posterior: Vec<ProbePosteriorEntry>,
    pub likelihood_scenario_count: usize,
    pub possible_likelihood_scenario_count: usize,
    pub impossible_likelihood_scenario_count: usize,
    pub scenario_posterior_entropy_min_nats: f64,
    pub scenario_posterior_entropy_max_nats: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ProbePortfolio {
    pub hypothesis_manifest_sha256: String,
    /// Exact current posterior/prior distribution, distinct from the stable label-space digest.
    pub prior_distribution_sha256: String,
    pub policy_sha256: String,
    pub candidate_catalogue_sha256: String,
    pub risk_assessment_catalogue_sha256: String,
    pub risk_evidence_reviewed_by_authority_owner: bool,
    pub likelihood_calibration_catalogue_sha256: String,
    pub likelihood_calibration_reviewed_by_independent_validator: bool,
    /// Sorted hypothesis IDs and exact prior probabilities used by this portfolio.
    pub prior: Vec<HypothesisPrior>,
    pub prior_entropy_nats: f64,
    pub ranked_recommendations: Vec<RankedDiagnosticProbe>,
    /// Retains eligible and rejected candidates so exclusion is inspectable.
    pub assessments: Vec<ProbeAssessment>,
    /// Always true: the module is a selector, not an executor or authorizer.
    pub execution_performed: bool,
    pub actuation_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeError {
    InvalidHypothesisSet,
    InvalidPolicy,
    EmptyIdentity,
    DuplicateHypothesis(String),
    DuplicateCandidate(String),
    InvalidCandidateDigest(String),
    HypothesisManifestMismatch(String),
    InvalidLikelihoodTable(String),
    InvalidLikelihoodProbability(String),
    InvalidLikelihoodNormalization(String),
    InvalidScenarioSet(String),
    RiskEvidenceNotReviewed,
    LikelihoodCalibrationNotReviewed,
    CandidateCatalogMismatch,
    MultiScenarioSequentialUpdateRequiresJointPosterior,
    InvalidSeedDigest,
    InvalidChernoffGrid,
    InvalidChernoffInputs,
    ObservationCandidateMismatch,
    CandidateNotEligibleOrSelected,
    SelectionReplayMismatch,
    InvalidObservationReceiptDigest,
    InvalidObservationLedger,
    ObservationLedgerPosteriorMismatch,
    ObservationAlreadyApplied,
    ObservedOutcomeNotFound(String),
    ObservedOutcomeImpossible(String),
    InvalidInformationGain(String),
    Serialization,
}

impl fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHypothesisSet => write!(
                formatter,
                "hypotheses need at least two unique IDs and a finite prior distribution summing to one"
            ),
            Self::InvalidPolicy => write!(
                formatter,
                "probe selection policy contains invalid limits or penalty scales"
            ),
            Self::EmptyIdentity => write!(formatter, "probe or hypothesis identity is empty"),
            Self::DuplicateHypothesis(id) => {
                write!(formatter, "duplicate hypothesis identity: {id}")
            }
            Self::DuplicateCandidate(id) => {
                write!(formatter, "duplicate probe candidate identity: {id}")
            }
            Self::InvalidCandidateDigest(id) => {
                write!(formatter, "candidate manifest digest is invalid: {id}")
            }
            Self::HypothesisManifestMismatch(id) => write!(
                formatter,
                "candidate likelihood table belongs to a different hypothesis manifest: {id}"
            ),
            Self::InvalidLikelihoodTable(id) => write!(
                formatter,
                "probe likelihood table is empty, incomplete, or has duplicate outcomes: {id}"
            ),
            Self::InvalidLikelihoodProbability(id) => write!(
                formatter,
                "probe likelihood is negative, non-finite, or above one: {id}"
            ),
            Self::InvalidLikelihoodNormalization(id) => write!(
                formatter,
                "conditional outcomes do not sum to one for every hypothesis: {id}"
            ),
            Self::InvalidScenarioSet(id) => write!(
                formatter,
                "likelihood uncertainty scenarios are empty, duplicated, unnormalized, or ambiguous: {id}"
            ),
            Self::RiskEvidenceNotReviewed => write!(
                formatter,
                "risk assessment has not been reviewed by the authority owner"
            ),
            Self::LikelihoodCalibrationNotReviewed => write!(
                formatter,
                "likelihood calibration has not been reviewed by an independent validator"
            ),
            Self::CandidateCatalogMismatch => write!(
                formatter,
                "baseline candidates do not match the scored probe catalogue"
            ),
            Self::MultiScenarioSequentialUpdateRequiresJointPosterior => write!(
                formatter,
                "sequential posterior updates for persistent multi-scenario likelihood models require a joint hypothesis-scenario posterior ledger"
            ),
            Self::InvalidSeedDigest => write!(
                formatter,
                "seed manifest must be a lowercase SHA-256 digest"
            ),
            Self::InvalidChernoffGrid => write!(
                formatter,
                "Chernoff grid must contain at least two intervals"
            ),
            Self::InvalidChernoffInputs => write!(
                formatter,
                "Chernoff inputs need normalized likelihoods and at least two positive-prior hypotheses"
            ),
            Self::ObservationCandidateMismatch => write!(
                formatter,
                "observed result belongs to a different probe candidate manifest"
            ),
            Self::CandidateNotEligibleOrSelected => write!(
                formatter,
                "observation candidate was not eligible and selected by the frozen evidence portfolio"
            ),
            Self::SelectionReplayMismatch => write!(
                formatter,
                "selection portfolio or baseline does not replay from its frozen inputs"
            ),
            Self::InvalidObservationLedger => write!(
                formatter,
                "observation ledger identity, ordering or digest is invalid"
            ),
            Self::ObservationLedgerPosteriorMismatch => write!(
                formatter,
                "observation ledger does not bind the supplied current prior posterior"
            ),
            Self::ObservationAlreadyApplied => write!(
                formatter,
                "observation receipt has already been applied to this posterior chain"
            ),
            Self::InvalidObservationReceiptDigest => write!(
                formatter,
                "observation receipt must be a lowercase SHA-256 digest"
            ),
            Self::ObservedOutcomeNotFound(id) => write!(
                formatter,
                "observed outcome is not declared by the candidate likelihood model: {id}"
            ),
            Self::ObservedOutcomeImpossible(id) => write!(
                formatter,
                "observed outcome has zero probability under the frozen prior and likelihood model: {id}"
            ),
            Self::InvalidInformationGain(id) => write!(
                formatter,
                "expected information gain is numerically invalid: {id}"
            ),
            Self::Serialization => write!(
                formatter,
                "failed to serialize probe evidence for canonical hashing"
            ),
        }
    }
}

impl std::error::Error for ProbeError {}

/// Compute the immutable digest of the hypothesis label space.
///
/// The digest deliberately excludes prior probabilities so one calibrated
/// candidate likelihood model remains valid as observations update beliefs.
pub fn hypothesis_manifest_sha256(hypotheses: &[HypothesisPrior]) -> Result<String, ProbeError> {
    let canonical = canonical_hypotheses(hypotheses)?;
    let hypothesis_ids = canonical
        .iter()
        .map(|hypothesis| hypothesis.hypothesis_id.as_str())
        .collect::<Vec<_>>();
    sha256_serializable(
        b"symtropy-diagnostic-hypothesis-space-v1\0",
        &hypothesis_ids,
    )
}

/// Compute the digest of one exact belief distribution over the stable
/// hypothesis space. This changes after each sequential observation.
pub fn hypothesis_prior_sha256(hypotheses: &[HypothesisPrior]) -> Result<String, ProbeError> {
    let canonical = canonical_hypotheses(hypotheses)?;
    sha256_serializable(b"symtropy-diagnostic-prior-distribution-v1\0", &canonical)
}

/// Compute the digest that must be stored in candidate_manifest_sha256.
pub fn candidate_manifest_sha256(
    candidate: &DiagnosticProbeCandidate,
) -> Result<String, ProbeError> {
    let payload = (
        candidate.candidate_id.as_str(),
        candidate.probe_class,
        candidate.authorization,
        &candidate.authorization_receipt_sha256,
        candidate.likelihood_model_sha256.as_str(),
        candidate.likelihood_calibration_sha256.as_str(),
        candidate
            .likelihood_calibration_review_receipt_sha256
            .as_str(),
        candidate.likelihood_calibration_reviewed_by_independent_validator,
        candidate.risk_assessment_sha256.as_str(),
        candidate.risk_assessment_reviewed_by_authority_owner,
        candidate.hypothesis_manifest_sha256.as_str(),
        candidate.expected_cost_seconds,
        candidate.expected_delay_seconds,
        candidate.risk_score_ppm,
        &candidate.outcomes,
        &candidate.uncertainty_scenarios,
    );
    sha256_serializable(b"symtropy-diagnostic-probe-candidate-v1\0", &payload)
}

/// Calculate and rank diagnostic evidence options using expected information
/// gain (prior entropy minus expected posterior entropy), then apply declared
/// cost, delay, risk and authority gates.
///
/// A positive score is decision support only. The function does not read
/// sensors, schedule an inspection, execute a perturbation, or authorize an
/// intervention. Controlled perturbations are never returned as eligible.
pub fn rank_diagnostic_probes(
    hypotheses: &[HypothesisPrior],
    candidates: &[DiagnosticProbeCandidate],
    policy: &ProbeSelectionPolicy,
) -> Result<ProbePortfolio, ProbeError> {
    policy.validate()?;
    let canonical_hypotheses = canonical_hypotheses(hypotheses)?;
    let hypothesis_ids = canonical_hypotheses
        .iter()
        .map(|item| item.hypothesis_id.clone())
        .collect::<Vec<_>>();
    let prior = canonical_hypotheses
        .iter()
        .map(|item| item.prior_probability)
        .collect::<Vec<_>>();
    let hypothesis_digest = hypothesis_manifest_sha256(&canonical_hypotheses)?;
    let prior_distribution_digest = hypothesis_prior_sha256(&canonical_hypotheses)?;
    let policy_digest = sha256_serializable(b"symtropy-diagnostic-probe-policy-v1\0", policy)?;
    let prior_entropy = entropy(&prior);
    let mut candidate_catalogue = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.candidate_id.clone(),
                candidate.candidate_manifest_sha256.clone(),
            )
        })
        .collect::<Vec<_>>();
    candidate_catalogue.sort_by(|left, right| left.0.cmp(&right.0));
    let candidate_catalogue_sha256 = sha256_serializable(
        b"symtropy-diagnostic-candidate-catalogue-v1\0",
        &candidate_catalogue,
    )?;
    let mut risk_assessment_catalogue = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.candidate_id.clone(),
                candidate.risk_assessment_sha256.clone(),
                candidate.risk_assessment_reviewed_by_authority_owner,
                candidate.risk_score_ppm,
            )
        })
        .collect::<Vec<_>>();
    risk_assessment_catalogue.sort_by(|left, right| left.0.cmp(&right.0));
    let risk_assessment_catalogue_sha256 = sha256_serializable(
        b"symtropy-diagnostic-risk-assessment-catalogue-v1\0",
        &risk_assessment_catalogue,
    )?;
    let risk_evidence_reviewed_by_authority_owner = !candidates.is_empty()
        && candidates
            .iter()
            .all(|candidate| candidate.risk_assessment_reviewed_by_authority_owner);
    let mut likelihood_calibration_catalogue = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.candidate_id.clone(),
                candidate.likelihood_calibration_sha256.clone(),
                candidate
                    .likelihood_calibration_review_receipt_sha256
                    .clone(),
                candidate.likelihood_calibration_reviewed_by_independent_validator,
            )
        })
        .collect::<Vec<_>>();
    likelihood_calibration_catalogue.sort_by(|left, right| left.0.cmp(&right.0));
    let likelihood_calibration_catalogue_sha256 = sha256_serializable(
        b"symtropy-diagnostic-likelihood-calibration-catalogue-v1\0",
        &likelihood_calibration_catalogue,
    )?;
    let likelihood_calibration_reviewed_by_independent_validator = !candidates.is_empty()
        && candidates
            .iter()
            .all(|candidate| candidate.likelihood_calibration_reviewed_by_independent_validator);

    let mut candidate_ids = BTreeSet::new();
    let mut assessments = Vec::with_capacity(candidates.len());
    let mut ranked = Vec::new();

    for candidate in candidates {
        if candidate.candidate_id.trim().is_empty() {
            return Err(ProbeError::EmptyIdentity);
        }
        if !candidate_ids.insert(candidate.candidate_id.clone()) {
            return Err(ProbeError::DuplicateCandidate(
                candidate.candidate_id.clone(),
            ));
        }
        if !is_sha256_hex(&candidate.likelihood_model_sha256)
            || !is_sha256_hex(&candidate.likelihood_calibration_sha256)
            || !is_sha256_hex(&candidate.likelihood_calibration_review_receipt_sha256)
            || !is_sha256_hex(&candidate.risk_assessment_sha256)
            || !is_sha256_hex(&candidate.hypothesis_manifest_sha256)
            || !is_sha256_hex(&candidate.candidate_manifest_sha256)
            || candidate_manifest_sha256(candidate)? != candidate.candidate_manifest_sha256
        {
            return Err(ProbeError::InvalidCandidateDigest(
                candidate.candidate_id.clone(),
            ));
        }
        if candidate.hypothesis_manifest_sha256 != hypothesis_digest {
            return Err(ProbeError::HypothesisManifestMismatch(
                candidate.candidate_id.clone(),
            ));
        }

        let (
            information_gain,
            worst_case_information_gain,
            posterior_entropy,
            scenario_count,
            robustness_qualified,
        ) = assess_likelihood_uncertainty(candidate, &prior, hypothesis_ids.len())?;

        let mut rejection_reasons = Vec::new();
        match candidate.probe_class {
            ProbeClass::ControlledPerturbation => rejection_reasons
                .push(ProbeRejectionReason::ControlledPerturbationRequiresSeparateSafetyCase),
            ProbeClass::PassiveMeasurement => {
                if candidate.authorization != ProbeAuthorization::ApprovedReadOnly {
                    rejection_reasons.push(ProbeRejectionReason::AuthorizationNotApproved);
                }
                if candidate
                    .authorization_receipt_sha256
                    .as_deref()
                    .is_none_or(|value| !is_sha256_hex(value))
                {
                    rejection_reasons
                        .push(ProbeRejectionReason::AuthorizationReceiptMissingOrInvalid);
                }
            }
            ProbeClass::OperatorInspection => {
                if !policy.allow_operator_inspection {
                    rejection_reasons.push(ProbeRejectionReason::OperatorInspectionDisabled);
                }
                if candidate.authorization != ProbeAuthorization::HumanAuthorized {
                    rejection_reasons.push(ProbeRejectionReason::AuthorizationNotApproved);
                }
                if candidate
                    .authorization_receipt_sha256
                    .as_deref()
                    .is_none_or(|value| !is_sha256_hex(value))
                {
                    rejection_reasons
                        .push(ProbeRejectionReason::AuthorizationReceiptMissingOrInvalid);
                }
            }
        }
        if !candidate.expected_cost_seconds.is_finite()
            || candidate.expected_cost_seconds < 0.0
            || candidate.expected_cost_seconds > policy.maximum_cost_seconds
        {
            rejection_reasons.push(ProbeRejectionReason::CostBudgetExceeded);
        }
        if !candidate.expected_delay_seconds.is_finite()
            || candidate.expected_delay_seconds < 0.0
            || candidate.expected_delay_seconds > policy.maximum_delay_seconds
        {
            rejection_reasons.push(ProbeRejectionReason::DelayBudgetExceeded);
        }
        if candidate.risk_score_ppm > policy.maximum_risk_ppm
            || candidate.risk_score_ppm > 1_000_000
        {
            rejection_reasons.push(ProbeRejectionReason::RiskBudgetExceeded);
        }
        if !candidate.risk_assessment_reviewed_by_authority_owner {
            rejection_reasons.push(ProbeRejectionReason::RiskEvidenceNotReviewed);
        }
        if !candidate.likelihood_calibration_reviewed_by_independent_validator {
            rejection_reasons.push(ProbeRejectionReason::LikelihoodCalibrationNotReviewed);
        }
        if worst_case_information_gain < policy.minimum_expected_information_gain_nats {
            rejection_reasons.push(ProbeRejectionReason::InformationGainBelowMinimum);
        }

        let eligible = rejection_reasons.is_empty();
        let utility = eligible.then(|| {
            worst_case_information_gain
                / (1.0
                    + candidate.expected_cost_seconds / policy.cost_penalty_scale_seconds
                    + candidate.expected_delay_seconds / policy.delay_penalty_scale_seconds
                    + f64::from(candidate.risk_score_ppm) / policy.risk_penalty_scale_ppm)
        });
        assessments.push(ProbeAssessment {
            candidate_id: candidate.candidate_id.clone(),
            eligible,
            rejection_reasons,
            expected_information_gain_nats: information_gain,
            worst_case_information_gain_nats: worst_case_information_gain,
            expected_posterior_entropy_nats: posterior_entropy,
            scenario_count,
            multi_scenario_assessed: robustness_qualified,
            risk_adjusted_utility: utility,
        });
        if let Some(utility) = utility {
            ranked.push(RankedDiagnosticProbe {
                rank: 0,
                candidate_id: candidate.candidate_id.clone(),
                probe_class: candidate.probe_class,
                expected_information_gain_nats: information_gain,
                worst_case_information_gain_nats: worst_case_information_gain,
                expected_posterior_entropy_nats: posterior_entropy,
                scenario_count,
                multi_scenario_assessed: robustness_qualified,
                risk_adjusted_utility: utility,
                expected_cost_seconds: candidate.expected_cost_seconds,
                expected_delay_seconds: candidate.expected_delay_seconds,
                risk_score_ppm: candidate.risk_score_ppm,
            });
        }
    }

    ranked.sort_by(|left, right| {
        right
            .risk_adjusted_utility
            .total_cmp(&left.risk_adjusted_utility)
            .then_with(|| {
                right
                    .expected_information_gain_nats
                    .total_cmp(&left.expected_information_gain_nats)
            })
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });
    for (index, recommendation) in ranked.iter_mut().enumerate() {
        recommendation.rank = index + 1;
    }
    assessments.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));

    Ok(ProbePortfolio {
        hypothesis_manifest_sha256: hypothesis_digest,
        prior_distribution_sha256: prior_distribution_digest,
        policy_sha256: policy_digest,
        candidate_catalogue_sha256,
        risk_assessment_catalogue_sha256,
        risk_evidence_reviewed_by_authority_owner,
        likelihood_calibration_catalogue_sha256,
        likelihood_calibration_reviewed_by_independent_validator,
        prior: canonical_hypotheses.clone(),
        prior_entropy_nats: prior_entropy,
        ranked_recommendations: ranked,
        assessments,
        execution_performed: false,
        actuation_authorized: false,
    })
}

/// Discretized pairwise Chernoff information for a finite likelihood table.
///
/// For hypothesis pair (i,j), C_ij = -log(min_s sum_o p(o|i)^s p(o|j)^(1-s)).
/// The reported minimum across pairs is the weakest pairwise separation. A
/// finite s-grid yields a conservative lower estimate of the continuous
/// Chernoff information; it is not a finite-sample error guarantee. The
/// classical error-exponent interpretation additionally assumes repeated,
/// conditionally independent observations from the same likelihood regime.
pub fn minimum_pairwise_chernoff_information_nats(
    prior: &[f64],
    outcomes: &[ProbeOutcomeLikelihood],
    grid_intervals: usize,
) -> Result<f64, ProbeError> {
    if grid_intervals < 2 {
        return Err(ProbeError::InvalidChernoffGrid);
    }
    if prior.len() < 2
        || prior
            .iter()
            .any(|probability| !probability.is_finite() || *probability < 0.0 || *probability > 1.0)
        || (prior.iter().sum::<f64>() - 1.0).abs() > PROBABILITY_TOLERANCE
        || prior
            .iter()
            .filter(|probability| **probability > 0.0)
            .count()
            < 2
        || outcomes.len() < 2
    {
        return Err(ProbeError::InvalidChernoffInputs);
    }
    let mut outcome_ids = BTreeSet::new();
    for outcome in outcomes {
        if outcome.outcome_id.trim().is_empty()
            || !outcome_ids.insert(outcome.outcome_id.clone())
            || outcome.likelihood_by_hypothesis.len() != prior.len()
            || outcome.likelihood_by_hypothesis.iter().any(|probability| {
                !probability.is_finite() || *probability < 0.0 || *probability > 1.0
            })
        {
            return Err(ProbeError::InvalidChernoffInputs);
        }
    }
    for hypothesis_index in 0..prior.len() {
        let sum = outcomes
            .iter()
            .map(|outcome| outcome.likelihood_by_hypothesis[hypothesis_index])
            .sum::<f64>();
        if (sum - 1.0).abs() > PROBABILITY_TOLERANCE {
            return Err(ProbeError::InvalidChernoffInputs);
        }
    }

    let supported = prior
        .iter()
        .enumerate()
        .filter_map(|(index, probability)| (*probability > 0.0).then_some(index))
        .collect::<Vec<_>>();
    let mut weakest_pairwise = f64::INFINITY;
    for pair_index in 0..supported.len() {
        for other_index in (pair_index + 1)..supported.len() {
            let left = supported[pair_index];
            let right = supported[other_index];
            let mut minimum_coefficient = 1.0_f64;
            for step in 0..=grid_intervals {
                let s = step as f64 / grid_intervals as f64;
                let complement = 1.0 - s;
                let coefficient = outcomes
                    .iter()
                    .map(|outcome| {
                        let p_left = outcome.likelihood_by_hypothesis[left];
                        let p_right = outcome.likelihood_by_hypothesis[right];
                        let left_term = if s == 0.0 { 1.0 } else { p_left.powf(s) };
                        let right_term = if complement == 0.0 {
                            1.0
                        } else {
                            p_right.powf(complement)
                        };
                        left_term * right_term
                    })
                    .sum::<f64>();
                if !coefficient.is_finite() || coefficient < 0.0 {
                    return Err(ProbeError::InvalidChernoffInputs);
                }
                minimum_coefficient = minimum_coefficient.min(coefficient);
            }
            let distance = if minimum_coefficient <= f64::MIN_POSITIVE {
                -f64::MIN_POSITIVE.ln()
            } else {
                -minimum_coefficient.min(1.0).ln()
            };
            weakest_pairwise = weakest_pairwise.min(distance);
        }
    }
    if !weakest_pairwise.is_finite() {
        return Err(ProbeError::InvalidChernoffInputs);
    }
    Ok(weakest_pairwise)
}

/// Minimum pairwise Chernoff separation across all declared likelihood
/// uncertainty scenarios. A single point-estimate table is evaluated but is
/// not multi-scenario qualified.
pub fn worst_case_pairwise_chernoff_information_nats(
    candidate: &DiagnosticProbeCandidate,
    prior: &[f64],
    grid_intervals: usize,
) -> Result<f64, ProbeError> {
    if candidate.uncertainty_scenarios.is_empty() {
        return minimum_pairwise_chernoff_information_nats(
            prior,
            &candidate.outcomes,
            grid_intervals,
        );
    }
    if !candidate.outcomes.is_empty() || candidate.uncertainty_scenarios.is_empty() {
        return Err(ProbeError::InvalidScenarioSet(
            candidate.candidate_id.clone(),
        ));
    }
    let mut scenario_ids = BTreeSet::new();
    let mut reference_outcome_ids: Option<BTreeSet<String>> = None;
    let mut scenario_weight_sum = 0.0;
    let mut worst_case = f64::INFINITY;
    for scenario in &candidate.uncertainty_scenarios {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.clone())
            || !scenario.scenario_weight.is_finite()
            || scenario.scenario_weight <= 0.0
            || scenario.scenario_weight > 1.0
        {
            return Err(ProbeError::InvalidScenarioSet(
                candidate.candidate_id.clone(),
            ));
        }
        let outcome_ids = scenario
            .outcomes
            .iter()
            .map(|outcome| outcome.outcome_id.clone())
            .collect::<BTreeSet<_>>();
        if reference_outcome_ids
            .as_ref()
            .is_some_and(|reference| reference != &outcome_ids)
        {
            return Err(ProbeError::InvalidScenarioSet(
                candidate.candidate_id.clone(),
            ));
        }
        reference_outcome_ids.get_or_insert(outcome_ids);
        scenario_weight_sum += scenario.scenario_weight;
        let distance =
            minimum_pairwise_chernoff_information_nats(prior, &scenario.outcomes, grid_intervals)?;
        worst_case = worst_case.min(distance);
    }
    if (scenario_weight_sum - 1.0).abs() > PROBABILITY_TOLERANCE {
        return Err(ProbeError::InvalidScenarioSet(
            candidate.candidate_id.clone(),
        ));
    }
    if !worst_case.is_finite() {
        return Err(ProbeError::InvalidScenarioSet(
            candidate.candidate_id.clone(),
        ));
    }
    Ok(worst_case)
}

/// Select a candidate using a frozen baseline over the exact same hard-eligible
/// catalogue. This enables matched comparisons with the risk-adjusted selector
/// without relaxing any authorization, risk or information-gain gate.
pub fn select_probe_baseline(
    portfolio: &ProbePortfolio,
    candidates: &[DiagnosticProbeCandidate],
    baseline: ProbeBaseline,
    seed_manifest_sha256: Option<&str>,
    cost_penalty_scale_seconds: f64,
) -> Result<ProbeBaselineSelection, ProbeError> {
    if !cost_penalty_scale_seconds.is_finite() || cost_penalty_scale_seconds <= 0.0 {
        return Err(ProbeError::InvalidPolicy);
    }
    let candidate_ids = candidates
        .iter()
        .map(|candidate| candidate.candidate_id.as_str())
        .collect::<BTreeSet<_>>();
    let assessment_ids = portfolio
        .assessments
        .iter()
        .map(|assessment| assessment.candidate_id.as_str())
        .collect::<BTreeSet<_>>();
    if candidate_ids.len() != candidates.len()
        || assessment_ids.len() != portfolio.assessments.len()
        || candidate_ids != assessment_ids
    {
        return Err(ProbeError::CandidateCatalogMismatch);
    }
    let mut catalogue_entries = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !is_sha256_hex(&candidate.candidate_manifest_sha256)
            || candidate_manifest_sha256(candidate)? != candidate.candidate_manifest_sha256
        {
            return Err(ProbeError::InvalidCandidateDigest(
                candidate.candidate_id.clone(),
            ));
        }
        catalogue_entries.push((
            candidate.candidate_id.clone(),
            candidate.candidate_manifest_sha256.clone(),
        ));
    }
    catalogue_entries.sort_by(|left, right| left.0.cmp(&right.0));
    let candidate_catalogue_sha256 = sha256_serializable(
        b"symtropy-diagnostic-candidate-catalogue-v1\0",
        &catalogue_entries,
    )?;
    if candidate_catalogue_sha256 != portfolio.candidate_catalogue_sha256 {
        return Err(ProbeError::CandidateCatalogMismatch);
    }

    let eligible_assessments = portfolio
        .assessments
        .iter()
        .filter(|assessment| assessment.eligible)
        .collect::<Vec<_>>();
    let eligible_entries = eligible_assessments
        .iter()
        .filter_map(|assessment| {
            let candidate = candidates
                .iter()
                .find(|candidate| candidate.candidate_id == assessment.candidate_id)?;
            Some((
                candidate.candidate_id.clone(),
                candidate.candidate_manifest_sha256.clone(),
            ))
        })
        .collect::<Vec<_>>();
    let eligible_ids = eligible_entries
        .iter()
        .map(|(candidate_id, _)| candidate_id.clone())
        .collect::<Vec<_>>();
    let eligible_catalogue_sha256 = sha256_serializable(
        b"symtropy-diagnostic-eligible-probe-catalogue-v1\0",
        &eligible_entries,
    )?;
    let eligible_candidates = candidates
        .iter()
        .filter(|candidate| {
            eligible_assessments
                .iter()
                .any(|assessment| assessment.candidate_id == candidate.candidate_id)
        })
        .collect::<Vec<_>>();

    let selected_candidate_id = match baseline {
        ProbeBaseline::NoAdditionalEvidence => None,
        ProbeBaseline::CheapestEligible => eligible_candidates
            .iter()
            .min_by(|left, right| {
                left.expected_cost_seconds
                    .total_cmp(&right.expected_cost_seconds)
                    .then_with(|| left.candidate_id.cmp(&right.candidate_id))
            })
            .map(|candidate| candidate.candidate_id.clone()),
        ProbeBaseline::MaximumInformationGain => eligible_assessments
            .iter()
            .max_by(|left, right| {
                left.expected_information_gain_nats
                    .total_cmp(&right.expected_information_gain_nats)
                    .then_with(|| right.candidate_id.cmp(&left.candidate_id))
            })
            .map(|assessment| assessment.candidate_id.clone()),
        ProbeBaseline::InformationGainPerCost => eligible_assessments
            .iter()
            .filter_map(|assessment| {
                let candidate = eligible_candidates
                    .iter()
                    .find(|candidate| candidate.candidate_id == assessment.candidate_id)?;
                let utility = assessment.expected_information_gain_nats
                    / (1.0 + candidate.expected_cost_seconds / cost_penalty_scale_seconds);
                Some((assessment.candidate_id.as_str(), utility))
            })
            .max_by(|left, right| left.1.total_cmp(&right.1).then_with(|| right.0.cmp(left.0)))
            .map(|(candidate_id, _)| candidate_id.to_owned()),
        ProbeBaseline::SeededRandomEligible => {
            let seed = seed_manifest_sha256.ok_or(ProbeError::InvalidSeedDigest)?;
            if !is_sha256_hex(seed) {
                return Err(ProbeError::InvalidSeedDigest);
            }
            if eligible_ids.is_empty() {
                None
            } else {
                eligible_entries
                    .iter()
                    .map(|(candidate_id, candidate_sha256)| {
                        let draw = sha256_serializable(
                            b"symtropy-diagnostic-random-baseline-v1\0",
                            &(seed, candidate_id, candidate_sha256),
                        )?;
                        Ok((draw, candidate_id.clone()))
                    })
                    .collect::<Result<Vec<_>, ProbeError>>()?
                    .into_iter()
                    .min_by(|left, right| left.0.cmp(&right.0))
                    .map(|(_, candidate_id)| candidate_id)
            }
        }
        ProbeBaseline::RiskAdjustedWorstCaseInformationGain => portfolio
            .ranked_recommendations
            .first()
            .map(|recommendation| recommendation.candidate_id.clone()),
        ProbeBaseline::MaximumWorstCaseChernoffInformation => eligible_candidates
            .iter()
            .map(|candidate| {
                Ok((
                    candidate.candidate_id.as_str(),
                    worst_case_pairwise_chernoff_information_nats(
                        candidate,
                        &portfolio
                            .prior
                            .iter()
                            .map(|item| item.prior_probability)
                            .collect::<Vec<_>>(),
                        100,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>, ProbeError>>()?
            .into_iter()
            .max_by(|left, right| left.1.total_cmp(&right.1).then_with(|| right.0.cmp(left.0)))
            .map(|(candidate_id, _)| candidate_id.to_owned()),
    };
    Ok(ProbeBaselineSelection {
        baseline,
        selected_candidate_id,
        eligible_candidate_count: eligible_ids.len(),
        candidate_catalogue_sha256,
        eligible_catalogue_sha256,
        seed_manifest_sha256: if baseline == ProbeBaseline::SeededRandomEligible {
            seed_manifest_sha256.map(str::to_owned)
        } else {
            None
        },
    })
}

/// Create the genesis ledger for a frozen prior/posterior distribution.
pub fn new_probe_observation_ledger(
    hypotheses: &[HypothesisPrior],
) -> Result<ProbeObservationLedger, ProbeError> {
    let canonical = canonical_hypotheses(hypotheses)?;
    let posterior_manifest_sha256 = posterior_manifest_sha256(&canonical)?;
    let mut ledger = ProbeObservationLedger {
        schema_version: "symtropy-probe-observation-ledger-v1".into(),
        current_posterior_manifest_sha256: posterior_manifest_sha256,
        applied_observation_receipt_sha256: Vec::new(),
        ledger_sha256: String::new(),
    };
    ledger.ledger_sha256 = observation_ledger_digest(&ledger)?;
    Ok(ledger)
}

fn posterior_manifest_sha256(hypotheses: &[HypothesisPrior]) -> Result<String, ProbeError> {
    sha256_serializable(
        b"symtropy-diagnostic-posterior-manifest-v1\0",
        &hypotheses
            .iter()
            .map(|hypothesis| {
                (
                    hypothesis.hypothesis_id.as_str(),
                    hypothesis.prior_probability,
                )
            })
            .collect::<Vec<_>>(),
    )
}

fn observation_ledger_digest(ledger: &ProbeObservationLedger) -> Result<String, ProbeError> {
    sha256_serializable(
        b"symtropy-diagnostic-observation-ledger-v1\0",
        &(
            ledger.schema_version.as_str(),
            ledger.current_posterior_manifest_sha256.as_str(),
            &ledger.applied_observation_receipt_sha256,
        ),
    )
}

fn validate_observation_ledger(ledger: &ProbeObservationLedger) -> Result<(), ProbeError> {
    if ledger.schema_version != "symtropy-probe-observation-ledger-v1"
        || !is_sha256_hex(&ledger.current_posterior_manifest_sha256)
        || !is_sha256_hex(&ledger.ledger_sha256)
        || ledger
            .applied_observation_receipt_sha256
            .iter()
            .any(|receipt| !is_sha256_hex(receipt))
        || ledger
            .applied_observation_receipt_sha256
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || observation_ledger_digest(ledger)? != ledger.ledger_sha256
    {
        return Err(ProbeError::InvalidObservationLedger);
    }
    Ok(())
}

/// Update a hypothesis distribution from an externally acquired observation.
/// This function does not acquire evidence or execute a probe. The observation
/// receipt must be independently created by the acquisition system and is
/// content-bound here; its signer, sensor provenance and authority remain
/// upstream verification duties.
/// Frozen decision context required to replay one evidence-selection decision.
/// Grouping this context prevents callers from accidentally passing a portfolio
/// from one prior or candidate catalogue alongside another ledger.
pub struct ProbeObservationContext<'a> {
    pub hypotheses: &'a [HypothesisPrior],
    pub candidates: &'a [DiagnosticProbeCandidate],
    pub policy: &'a ProbeSelectionPolicy,
    pub portfolio: &'a ProbePortfolio,
    pub selection: &'a ProbeBaselineSelection,
    pub ledger: &'a ProbeObservationLedger,
    pub candidate: &'a DiagnosticProbeCandidate,
    pub expected_candidate_manifest_sha256: &'a str,
}

pub fn realize_probe_observation(
    context: ProbeObservationContext<'_>,
    observed_outcome_id: &str,
    observation_receipt_sha256: &str,
) -> Result<ProbeObservationUpdate, ProbeError> {
    let ProbeObservationContext {
        hypotheses,
        candidates,
        policy,
        portfolio,
        selection,
        ledger,
        candidate,
        expected_candidate_manifest_sha256,
    } = context;

    if observed_outcome_id.trim().is_empty() {
        return Err(ProbeError::EmptyIdentity);
    }
    if !is_sha256_hex(observation_receipt_sha256) {
        return Err(ProbeError::InvalidObservationReceiptDigest);
    }
    if !is_sha256_hex(expected_candidate_manifest_sha256)
        || expected_candidate_manifest_sha256 != candidate.candidate_manifest_sha256
        || candidate_manifest_sha256(candidate)? != candidate.candidate_manifest_sha256
        || !is_sha256_hex(&candidate.likelihood_model_sha256)
        || !is_sha256_hex(&candidate.likelihood_calibration_sha256)
        || !is_sha256_hex(&candidate.likelihood_calibration_review_receipt_sha256)
        || !is_sha256_hex(&candidate.risk_assessment_sha256)
    {
        return Err(ProbeError::ObservationCandidateMismatch);
    }

    let replayed_portfolio = rank_diagnostic_probes(hypotheses, candidates, policy)?;
    if &replayed_portfolio != portfolio {
        return Err(ProbeError::SelectionReplayMismatch);
    }
    let replayed_selection = select_probe_baseline(
        &replayed_portfolio,
        candidates,
        selection.baseline,
        selection.seed_manifest_sha256.as_deref(),
        policy.cost_penalty_scale_seconds,
    )?;
    if &replayed_selection != selection {
        return Err(ProbeError::SelectionReplayMismatch);
    }
    if selection.selected_candidate_id.as_deref() != Some(candidate.candidate_id.as_str())
        || !portfolio.assessments.iter().any(|assessment| {
            assessment.candidate_id == candidate.candidate_id && assessment.eligible
        })
        || !candidates.iter().any(|item| {
            item.candidate_id == candidate.candidate_id
                && item.candidate_manifest_sha256 == expected_candidate_manifest_sha256
        })
    {
        return Err(ProbeError::CandidateNotEligibleOrSelected);
    }

    let canonical_hypotheses = canonical_hypotheses(hypotheses)?;
    let hypothesis_ids = canonical_hypotheses
        .iter()
        .map(|item| item.hypothesis_id.clone())
        .collect::<Vec<_>>();
    let prior = canonical_hypotheses
        .iter()
        .map(|item| item.prior_probability)
        .collect::<Vec<_>>();
    let hypothesis_digest = hypothesis_manifest_sha256(&canonical_hypotheses)?;
    if candidate.hypothesis_manifest_sha256 != hypothesis_digest {
        return Err(ProbeError::HypothesisManifestMismatch(
            candidate.candidate_id.clone(),
        ));
    }
    validate_observation_ledger(ledger)?;
    let previous_posterior_manifest_sha256 = posterior_manifest_sha256(&canonical_hypotheses)?;
    if ledger.current_posterior_manifest_sha256 != previous_posterior_manifest_sha256 {
        return Err(ProbeError::ObservationLedgerPosteriorMismatch);
    }
    if ledger
        .applied_observation_receipt_sha256
        .binary_search(&observation_receipt_sha256.to_owned())
        .is_ok()
    {
        return Err(ProbeError::ObservationAlreadyApplied);
    }
    let (_, _, _, scenario_count, _) =
        assess_likelihood_uncertainty(candidate, &prior, hypothesis_ids.len())?;
    if scenario_count > 1 {
        // Likelihood scenarios represent persistent uncertainty in the frozen
        // model/calibration, not a fresh random draw per sample. Marginalizing
        // them independently on every update would be statistically wrong.
        // Until the ledger carries the joint p(hypothesis, scenario) posterior,
        // only point-model sequential updates are supported.
        return Err(ProbeError::MultiScenarioSequentialUpdateRequiresJointPosterior);
    }

    let likelihood_by_hypothesis = if candidate.uncertainty_scenarios.is_empty() {
        let outcome = candidate
            .outcomes
            .iter()
            .find(|outcome| outcome.outcome_id == observed_outcome_id)
            .ok_or_else(|| ProbeError::ObservedOutcomeNotFound(observed_outcome_id.to_owned()))?;
        outcome.likelihood_by_hypothesis.clone()
    } else {
        let first_scenario = candidate.uncertainty_scenarios.first().unwrap();
        if !first_scenario
            .outcomes
            .iter()
            .any(|outcome| outcome.outcome_id == observed_outcome_id)
        {
            return Err(ProbeError::ObservedOutcomeNotFound(
                observed_outcome_id.to_owned(),
            ));
        }
        (0..hypothesis_ids.len())
            .map(|hypothesis_index| {
                candidate
                    .uncertainty_scenarios
                    .iter()
                    .map(|scenario| {
                        let outcome = scenario
                            .outcomes
                            .iter()
                            .find(|outcome| outcome.outcome_id == observed_outcome_id)
                            .expect("scenario outcome spaces were validated as identical");
                        scenario.scenario_weight
                            * outcome.likelihood_by_hypothesis[hypothesis_index]
                    })
                    .sum::<f64>()
            })
            .collect::<Vec<_>>()
    };

    let observation_probability = prior
        .iter()
        .zip(&likelihood_by_hypothesis)
        .map(|(prior_probability, likelihood)| prior_probability * likelihood)
        .sum::<f64>();
    if !observation_probability.is_finite() || observation_probability <= f64::EPSILON {
        return Err(ProbeError::ObservedOutcomeImpossible(
            observed_outcome_id.to_owned(),
        ));
    }
    let posterior_probabilities = prior
        .iter()
        .zip(&likelihood_by_hypothesis)
        .map(|(prior_probability, likelihood)| {
            prior_probability * likelihood / observation_probability
        })
        .collect::<Vec<_>>();
    let posterior_total = posterior_probabilities.iter().sum::<f64>();
    if !posterior_total.is_finite() || (posterior_total - 1.0).abs() > 1e-8 {
        return Err(ProbeError::InvalidInformationGain(
            candidate.candidate_id.clone(),
        ));
    }
    let prior_entropy = entropy(&prior);
    let posterior_entropy = entropy(&posterior_probabilities);
    let realized_kl = posterior_probabilities
        .iter()
        .zip(&prior)
        .filter(|(posterior_probability, _)| **posterior_probability > 0.0)
        .map(|(posterior_probability, prior_probability)| {
            posterior_probability * (posterior_probability / prior_probability).ln()
        })
        .sum::<f64>();
    if !realized_kl.is_finite() || realized_kl < -1e-8 {
        return Err(ProbeError::InvalidInformationGain(
            candidate.candidate_id.clone(),
        ));
    }

    let mut scenario_entropies = Vec::new();
    if candidate.uncertainty_scenarios.is_empty() {
        scenario_entropies.push(posterior_entropy);
    } else {
        for scenario in &candidate.uncertainty_scenarios {
            let outcome = scenario
                .outcomes
                .iter()
                .find(|outcome| outcome.outcome_id == observed_outcome_id)
                .expect("scenario outcome spaces were validated as identical");
            let scenario_observation_probability = prior
                .iter()
                .zip(&outcome.likelihood_by_hypothesis)
                .map(|(prior_probability, likelihood)| prior_probability * likelihood)
                .sum::<f64>();
            if scenario_observation_probability > f64::EPSILON {
                let scenario_posterior = prior
                    .iter()
                    .zip(&outcome.likelihood_by_hypothesis)
                    .map(|(prior_probability, likelihood)| {
                        prior_probability * likelihood / scenario_observation_probability
                    })
                    .collect::<Vec<_>>();
                scenario_entropies.push(entropy(&scenario_posterior));
            }
        }
    }
    let possible_scenario_count = scenario_entropies.len();
    if possible_scenario_count == 0 {
        return Err(ProbeError::ObservedOutcomeImpossible(
            observed_outcome_id.to_owned(),
        ));
    }
    scenario_entropies.sort_by(f64::total_cmp);

    let posterior = hypothesis_ids
        .into_iter()
        .zip(posterior_probabilities)
        .map(|(hypothesis_id, probability)| ProbePosteriorEntry {
            hypothesis_id,
            probability,
        })
        .collect::<Vec<_>>();
    let posterior_as_hypotheses = posterior
        .iter()
        .map(|entry| HypothesisPrior {
            hypothesis_id: entry.hypothesis_id.clone(),
            prior_probability: entry.probability,
        })
        .collect::<Vec<_>>();
    let posterior_manifest_sha256 = posterior_manifest_sha256(&posterior_as_hypotheses)?;
    let mut updated_ledger = ledger.clone();
    updated_ledger.current_posterior_manifest_sha256 = posterior_manifest_sha256.clone();
    updated_ledger
        .applied_observation_receipt_sha256
        .push(observation_receipt_sha256.to_owned());
    updated_ledger.applied_observation_receipt_sha256.sort();
    updated_ledger.ledger_sha256 = observation_ledger_digest(&updated_ledger)?;

    Ok(ProbeObservationUpdate {
        candidate_id: candidate.candidate_id.clone(),
        candidate_manifest_sha256: candidate.candidate_manifest_sha256.clone(),
        observation_receipt_sha256: observation_receipt_sha256.to_owned(),
        observed_outcome_id: observed_outcome_id.to_owned(),
        previous_posterior_manifest_sha256,
        posterior_manifest_sha256,
        observation_ledger: updated_ledger,
        prior_entropy_nats: prior_entropy,
        posterior_entropy_nats: posterior_entropy,
        realized_entropy_reduction_nats: prior_entropy - posterior_entropy,
        realized_kl_divergence_nats: realized_kl.max(0.0),
        posterior,
        likelihood_scenario_count: scenario_count,
        possible_likelihood_scenario_count: possible_scenario_count,
        impossible_likelihood_scenario_count: scenario_count - possible_scenario_count,
        scenario_posterior_entropy_min_nats: scenario_entropies[0],
        scenario_posterior_entropy_max_nats: *scenario_entropies.last().unwrap(),
    })
}

fn canonical_hypotheses(
    hypotheses: &[HypothesisPrior],
) -> Result<Vec<HypothesisPrior>, ProbeError> {
    if hypotheses.len() < 2 {
        return Err(ProbeError::InvalidHypothesisSet);
    }
    let mut canonical = hypotheses.to_vec();
    canonical.sort_by(|left, right| left.hypothesis_id.cmp(&right.hypothesis_id));
    let mut identifiers = BTreeSet::new();
    let mut total = 0.0;
    for hypothesis in &canonical {
        if hypothesis.hypothesis_id.trim().is_empty() {
            return Err(ProbeError::EmptyIdentity);
        }
        if !identifiers.insert(hypothesis.hypothesis_id.clone()) {
            return Err(ProbeError::DuplicateHypothesis(
                hypothesis.hypothesis_id.clone(),
            ));
        }
        if !hypothesis.prior_probability.is_finite()
            || hypothesis.prior_probability < 0.0
            || hypothesis.prior_probability > 1.0
        {
            return Err(ProbeError::InvalidHypothesisSet);
        }
        total += hypothesis.prior_probability;
    }
    if !total.is_finite() || (total - 1.0).abs() > PROBABILITY_TOLERANCE {
        return Err(ProbeError::InvalidHypothesisSet);
    }
    Ok(canonical)
}

fn validate_likelihood_table(
    candidate_id: &str,
    outcomes: &[ProbeOutcomeLikelihood],
    hypothesis_count: usize,
) -> Result<(), ProbeError> {
    if outcomes.len() < 2 {
        return Err(ProbeError::InvalidLikelihoodTable(candidate_id.to_owned()));
    }
    let mut outcome_ids = BTreeSet::new();
    for outcome in outcomes {
        if outcome.outcome_id.trim().is_empty()
            || !outcome_ids.insert(outcome.outcome_id.clone())
            || outcome.likelihood_by_hypothesis.len() != hypothesis_count
        {
            return Err(ProbeError::InvalidLikelihoodTable(candidate_id.to_owned()));
        }
        if outcome
            .likelihood_by_hypothesis
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0 || *value > 1.0)
        {
            return Err(ProbeError::InvalidLikelihoodProbability(
                candidate_id.to_owned(),
            ));
        }
    }
    for hypothesis_index in 0..hypothesis_count {
        let sum = outcomes
            .iter()
            .map(|outcome| outcome.likelihood_by_hypothesis[hypothesis_index])
            .sum::<f64>();
        if !sum.is_finite() || (sum - 1.0).abs() > PROBABILITY_TOLERANCE {
            return Err(ProbeError::InvalidLikelihoodNormalization(
                candidate_id.to_owned(),
            ));
        }
    }
    Ok(())
}

fn assess_likelihood_uncertainty(
    candidate: &DiagnosticProbeCandidate,
    prior: &[f64],
    hypothesis_count: usize,
) -> Result<(f64, f64, f64, usize, bool), ProbeError> {
    if candidate.uncertainty_scenarios.is_empty() {
        if !candidate.outcomes.is_empty() {
            validate_likelihood_table(
                &candidate.candidate_id,
                &candidate.outcomes,
                hypothesis_count,
            )?;
            let (gain, posterior_entropy) = expected_information_gain(prior, &candidate.outcomes)
                .map_err(|_| {
                ProbeError::InvalidInformationGain(candidate.candidate_id.clone())
            })?;
            return Ok((gain, gain, posterior_entropy, 1, false));
        }
        return Err(ProbeError::InvalidScenarioSet(
            candidate.candidate_id.clone(),
        ));
    }
    if !candidate.outcomes.is_empty() {
        return Err(ProbeError::InvalidScenarioSet(
            candidate.candidate_id.clone(),
        ));
    }
    let mut scenario_ids = BTreeSet::new();
    let mut reference_outcome_ids: Option<BTreeSet<String>> = None;
    let mut weight_sum = 0.0;
    let mut weighted_information_gain = 0.0;
    let mut worst_case_information_gain = f64::INFINITY;
    for scenario in &candidate.uncertainty_scenarios {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.clone())
            || !scenario.scenario_weight.is_finite()
            || scenario.scenario_weight <= 0.0
            || scenario.scenario_weight > 1.0
        {
            return Err(ProbeError::InvalidScenarioSet(
                candidate.candidate_id.clone(),
            ));
        }
        validate_likelihood_table(
            &candidate.candidate_id,
            &scenario.outcomes,
            hypothesis_count,
        )?;
        let outcome_ids = scenario
            .outcomes
            .iter()
            .map(|outcome| outcome.outcome_id.clone())
            .collect::<BTreeSet<_>>();
        if reference_outcome_ids
            .as_ref()
            .is_some_and(|reference| reference != &outcome_ids)
        {
            return Err(ProbeError::InvalidScenarioSet(
                candidate.candidate_id.clone(),
            ));
        }
        reference_outcome_ids.get_or_insert(outcome_ids);
        let (gain, _) = expected_information_gain(prior, &scenario.outcomes)
            .map_err(|_| ProbeError::InvalidInformationGain(candidate.candidate_id.clone()))?;
        weight_sum += scenario.scenario_weight;
        weighted_information_gain += scenario.scenario_weight * gain;
        worst_case_information_gain = worst_case_information_gain.min(gain);
    }
    if (weight_sum - 1.0).abs() > PROBABILITY_TOLERANCE {
        return Err(ProbeError::InvalidScenarioSet(
            candidate.candidate_id.clone(),
        ));
    }
    let expected_information_gain = weighted_information_gain / weight_sum;
    let expected_posterior_entropy = entropy(prior) - expected_information_gain;
    Ok((
        expected_information_gain.max(0.0),
        worst_case_information_gain.max(0.0),
        expected_posterior_entropy.max(0.0),
        candidate.uncertainty_scenarios.len(),
        candidate.uncertainty_scenarios.len() > 1,
    ))
}

fn expected_information_gain(
    prior: &[f64],
    outcomes: &[ProbeOutcomeLikelihood],
) -> Result<(f64, f64), ()> {
    if prior.len() < 2 || outcomes.len() < 2 {
        return Err(());
    }
    let prior_entropy = entropy(prior);
    let mut expected_posterior_entropy = 0.0;
    for outcome in outcomes {
        if outcome.likelihood_by_hypothesis.len() != prior.len() {
            return Err(());
        }
        let outcome_probability = prior
            .iter()
            .zip(&outcome.likelihood_by_hypothesis)
            .map(|(prior_probability, likelihood)| prior_probability * likelihood)
            .sum::<f64>();
        if !outcome_probability.is_finite() || outcome_probability < 0.0 {
            return Err(());
        }
        if outcome_probability <= f64::EPSILON {
            continue;
        }
        let posterior = prior
            .iter()
            .zip(&outcome.likelihood_by_hypothesis)
            .map(|(prior_probability, likelihood)| {
                prior_probability * likelihood / outcome_probability
            })
            .collect::<Vec<_>>();
        let posterior_total = posterior.iter().sum::<f64>();
        if !posterior_total.is_finite() || (posterior_total - 1.0).abs() > 1e-8 {
            return Err(());
        }
        expected_posterior_entropy += outcome_probability * entropy(&posterior);
    }
    let raw_gain = prior_entropy - expected_posterior_entropy;
    if !raw_gain.is_finite() || raw_gain < -1e-8 {
        return Err(());
    }
    Ok((raw_gain.max(0.0), expected_posterior_entropy))
}

fn entropy(probabilities: &[f64]) -> f64 {
    -probabilities
        .iter()
        .filter(|probability| **probability > 0.0)
        .map(|probability| probability * probability.ln())
        .sum::<f64>()
}

fn sha256_serializable<T: Serialize>(domain: &[u8], value: &T) -> Result<String, ProbeError> {
    let encoded = serde_json::to_vec(value).map_err(|_| ProbeError::Serialization)?;
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

    // Keep test fixtures concise; production API groups the frozen decision context.
    #[allow(clippy::too_many_arguments)]
    fn realize_probe_observation(
        hypotheses: &[HypothesisPrior],
        candidates: &[DiagnosticProbeCandidate],
        policy: &ProbeSelectionPolicy,
        portfolio: &ProbePortfolio,
        selection: &ProbeBaselineSelection,
        ledger: &ProbeObservationLedger,
        candidate: &DiagnosticProbeCandidate,
        expected_candidate_manifest_sha256: &str,
        observed_outcome_id: &str,
        observation_receipt_sha256: &str,
    ) -> Result<ProbeObservationUpdate, ProbeError> {
        super::realize_probe_observation(
            ProbeObservationContext {
                hypotheses,
                candidates,
                policy,
                portfolio,
                selection,
                ledger,
                candidate,
                expected_candidate_manifest_sha256,
            },
            observed_outcome_id,
            observation_receipt_sha256,
        )
    }

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn hypotheses() -> Vec<HypothesisPrior> {
        vec![
            HypothesisPrior {
                hypothesis_id: "bearing_degradation".into(),
                prior_probability: 0.5,
            },
            HypothesisPrior {
                hypothesis_id: "fuel_delivery_fault".into(),
                prior_probability: 0.5,
            },
        ]
    }

    fn outcomes(informative: bool) -> Vec<ProbeOutcomeLikelihood> {
        let probability = if informative { 0.95 } else { 0.5 };
        let other = 1.0 - probability;
        vec![
            ProbeOutcomeLikelihood {
                outcome_id: "bearing_signature".into(),
                likelihood_by_hypothesis: vec![probability, other],
            },
            ProbeOutcomeLikelihood {
                outcome_id: "fuel_signature".into(),
                likelihood_by_hypothesis: vec![other, probability],
            },
        ]
    }

    fn candidate(
        candidate_id: &str,
        probe_class: ProbeClass,
        authorization: ProbeAuthorization,
        informative: bool,
    ) -> DiagnosticProbeCandidate {
        let hypothesis_manifest_sha256 = hypothesis_manifest_sha256(&hypotheses()).unwrap();
        let mut candidate = DiagnosticProbeCandidate {
            candidate_id: candidate_id.into(),
            probe_class,
            authorization,
            authorization_receipt_sha256: Some(digest('f')),
            likelihood_model_sha256: digest('a'),
            likelihood_calibration_sha256: digest('b'),
            likelihood_calibration_review_receipt_sha256: digest('d'),
            likelihood_calibration_reviewed_by_independent_validator: true,
            risk_assessment_sha256: digest('c'),
            risk_assessment_reviewed_by_authority_owner: true,
            hypothesis_manifest_sha256,
            expected_cost_seconds: 30.0,
            expected_delay_seconds: 10.0,
            risk_score_ppm: 1_000,
            outcomes: outcomes(informative),
            candidate_manifest_sha256: digest('0'),
            uncertainty_scenarios: Vec::new(),
        };
        candidate.candidate_manifest_sha256 = candidate_manifest_sha256(&candidate).unwrap();
        candidate
    }

    #[test]
    fn unreviewed_risk_and_likelihood_evidence_have_distinct_error_codes() {
        let risk_error = ProbeError::RiskEvidenceNotReviewed;
        let calibration_error = ProbeError::LikelihoodCalibrationNotReviewed;
        assert!(risk_error.to_string().contains("authority owner"));
        assert!(
            calibration_error
                .to_string()
                .contains("independent validator")
        );
        assert_ne!(risk_error, calibration_error);
    }

    #[test]
    fn information_gain_prefers_the_measurement_that_separates_competing_hypotheses() {
        let hypotheses = hypotheses();
        let candidates = vec![
            candidate(
                "weak_probe",
                ProbeClass::PassiveMeasurement,
                ProbeAuthorization::ApprovedReadOnly,
                false,
            ),
            candidate(
                "discriminating_probe",
                ProbeClass::PassiveMeasurement,
                ProbeAuthorization::ApprovedReadOnly,
                true,
            ),
        ];
        let result = rank_diagnostic_probes(
            &hypotheses,
            &candidates,
            &ProbeSelectionPolicy::conservative_read_only(),
        )
        .unwrap();
        assert_eq!(result.ranked_recommendations.len(), 1);
        assert_eq!(
            result.ranked_recommendations[0].candidate_id,
            "discriminating_probe"
        );
        assert!(result.ranked_recommendations[0].expected_information_gain_nats > 0.0);
        let weak = result
            .assessments
            .iter()
            .find(|assessment| assessment.candidate_id == "weak_probe")
            .unwrap();
        assert!(
            weak.rejection_reasons
                .contains(&ProbeRejectionReason::InformationGainBelowMinimum)
        );
        assert!(!result.execution_performed);
        assert!(!result.actuation_authorized);
    }

    #[test]
    fn controlled_perturbation_is_never_recommended_by_the_read_only_selector() {
        let candidate = candidate(
            "throttle_step_test",
            ProbeClass::ControlledPerturbation,
            ProbeAuthorization::HumanAuthorized,
            true,
        );
        let result = rank_diagnostic_probes(
            &hypotheses(),
            &[candidate],
            &ProbeSelectionPolicy::conservative_read_only(),
        )
        .unwrap();
        assert!(result.ranked_recommendations.is_empty());
        assert!(
            result.assessments[0]
                .rejection_reasons
                .contains(&ProbeRejectionReason::ControlledPerturbationRequiresSeparateSafetyCase)
        );
    }

    #[test]
    fn operator_inspection_needs_explicit_human_authority_andpolicy_enablement() {
        let candidate = candidate(
            "inspect_actuator_linkage",
            ProbeClass::OperatorInspection,
            ProbeAuthorization::HumanAuthorized,
            true,
        );
        let mut policy = ProbeSelectionPolicy::conservative_read_only();
        policy.allow_operator_inspection = false;
        let result = rank_diagnostic_probes(&hypotheses(), &[candidate], &policy).unwrap();
        assert!(result.ranked_recommendations.is_empty());
        assert!(
            result.assessments[0]
                .rejection_reasons
                .contains(&ProbeRejectionReason::OperatorInspectionDisabled)
        );
    }

    #[test]
    fn missing_authority_and_excessive_risk_or_cost_are_excluded() {
        let mut unauthorised = candidate(
            "unknown_authority",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::Unknown,
            true,
        );
        unauthorised.authorization_receipt_sha256 = None;
        unauthorised.candidate_manifest_sha256 = candidate_manifest_sha256(&unauthorised).unwrap();

        let mut unsafe_cost = candidate(
            "high_cost_risky_probe",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        unsafe_cost.expected_cost_seconds = 9_999.0;
        unsafe_cost.risk_score_ppm = 50_000;
        unsafe_cost.candidate_manifest_sha256 = candidate_manifest_sha256(&unsafe_cost).unwrap();

        let result = rank_diagnostic_probes(
            &hypotheses(),
            &[unauthorised, unsafe_cost],
            &ProbeSelectionPolicy::conservative_read_only(),
        )
        .unwrap();
        assert!(result.ranked_recommendations.is_empty());
        assert_eq!(result.assessments.len(), 2);
        assert!(
            result
                .assessments
                .iter()
                .all(|assessment| !assessment.eligible)
        );
    }

    #[test]
    fn missing_or_malformed_risk_assessment_digest_fails_before_ranking() {
        let mut candidate = candidate(
            "unreviewed_risk_score",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        candidate.risk_assessment_sha256 = "unreviewed".into();
        candidate.candidate_manifest_sha256 = candidate_manifest_sha256(&candidate).unwrap();
        assert!(matches!(
            rank_diagnostic_probes(
                &hypotheses(),
                &[candidate],
                &ProbeSelectionPolicy::conservative_read_only()
            ),
            Err(ProbeError::InvalidCandidateDigest(_))
        ));
    }

    #[test]
    fn likelihood_calibration_requires_independent_review_and_receipt() {
        let mut unreviewed = candidate(
            "unreviewed_likelihood",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        unreviewed.likelihood_calibration_reviewed_by_independent_validator = false;
        unreviewed.candidate_manifest_sha256 = candidate_manifest_sha256(&unreviewed).unwrap();
        let result = rank_diagnostic_probes(
            &hypotheses(),
            &[unreviewed],
            &ProbeSelectionPolicy::conservative_read_only(),
        )
        .unwrap();
        assert!(result.ranked_recommendations.is_empty());
        assert!(!result.likelihood_calibration_reviewed_by_independent_validator);
        assert!(
            result.assessments[0]
                .rejection_reasons
                .contains(&ProbeRejectionReason::LikelihoodCalibrationNotReviewed)
        );
    }

    #[test]
    fn malformed_likelihood_review_receipt_fails_before_ranking() {
        let mut malformed = candidate(
            "malformed_likelihood_receipt",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        malformed.likelihood_calibration_review_receipt_sha256 = "not-a-digest".into();
        malformed.candidate_manifest_sha256 = candidate_manifest_sha256(&malformed).unwrap();
        assert!(matches!(
            rank_diagnostic_probes(
                &hypotheses(),
                &[malformed],
                &ProbeSelectionPolicy::conservative_read_only()
            ),
            Err(ProbeError::InvalidCandidateDigest(_))
        ));
    }

    #[test]
    fn unreviewed_risk_assessment_is_excluded_even_with_a_valid_digest() {
        let mut candidate = candidate(
            "unreviewed_risk",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        candidate.risk_assessment_reviewed_by_authority_owner = false;
        candidate.candidate_manifest_sha256 = candidate_manifest_sha256(&candidate).unwrap();
        let result = rank_diagnostic_probes(
            &hypotheses(),
            &[candidate],
            &ProbeSelectionPolicy::conservative_read_only(),
        )
        .unwrap();
        assert!(result.ranked_recommendations.is_empty());
        assert!(!result.risk_evidence_reviewed_by_authority_owner);
        assert!(
            result.assessments[0]
                .rejection_reasons
                .contains(&ProbeRejectionReason::RiskEvidenceNotReviewed)
        );
    }

    #[test]
    fn baseline_rejects_same_candidate_ids_with_changed_cost_or_risk_manifest() {
        let candidates = vec![candidate_with_probability("sensor_a", 0.9, 30.0)];
        let policy = ProbeSelectionPolicy::conservative_read_only();
        let portfolio = rank_diagnostic_probes(&hypotheses(), &candidates, &policy).unwrap();
        let mut altered = candidates.clone();
        altered[0].expected_cost_seconds = 1.0;
        altered[0].candidate_manifest_sha256 = candidate_manifest_sha256(&altered[0]).unwrap();
        assert_eq!(
            select_probe_baseline(
                &portfolio,
                &altered,
                ProbeBaseline::CheapestEligible,
                None,
                policy.cost_penalty_scale_seconds
            ),
            Err(ProbeError::CandidateCatalogMismatch)
        );
    }

    #[test]
    fn tampered_candidate_manifest_fails_before_ranking() {
        let mut tampered = candidate(
            "pressure_observation",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        tampered.expected_delay_seconds += 1.0;
        assert!(matches!(
            rank_diagnostic_probes(
                &hypotheses(),
                &[tampered],
                &ProbeSelectionPolicy::conservative_read_only()
            ),
            Err(ProbeError::InvalidCandidateDigest(_))
        ));
    }

    #[test]
    fn likelihood_table_must_be_normalized_for_every_hypothesis() {
        let mut invalid = candidate(
            "bad_likelihood",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        invalid.outcomes[0].likelihood_by_hypothesis[0] = 0.8;
        invalid.candidate_manifest_sha256 = candidate_manifest_sha256(&invalid).unwrap();
        assert!(matches!(
            rank_diagnostic_probes(
                &hypotheses(),
                &[invalid],
                &ProbeSelectionPolicy::conservative_read_only()
            ),
            Err(ProbeError::InvalidLikelihoodNormalization(_))
        ));
    }

    #[test]
    fn no_eligible_probe_returns_an_empty_portfolio_without_weakening_gates() {
        let candidate = candidate(
            "unauthorized_probe",
            ProbeClass::ControlledPerturbation,
            ProbeAuthorization::HumanAuthorized,
            true,
        );
        let result = rank_diagnostic_probes(
            &hypotheses(),
            &[candidate],
            &ProbeSelectionPolicy::conservative_read_only(),
        )
        .unwrap();
        assert!(result.ranked_recommendations.is_empty());
        assert_eq!(
            result.assessments[0].rejection_reasons,
            vec![ProbeRejectionReason::ControlledPerturbationRequiresSeparateSafetyCase]
        );
    }

    fn candidate_with_probability(
        candidate_id: &str,
        probability: f64,
        cost_seconds: f64,
    ) -> DiagnosticProbeCandidate {
        let mut item = candidate(
            candidate_id,
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            false,
        );
        let other = 1.0 - probability;
        item.outcomes = vec![
            ProbeOutcomeLikelihood {
                outcome_id: "bearing_signature".into(),
                likelihood_by_hypothesis: vec![probability, other],
            },
            ProbeOutcomeLikelihood {
                outcome_id: "fuel_signature".into(),
                likelihood_by_hypothesis: vec![other, probability],
            },
        ];
        item.expected_cost_seconds = cost_seconds;
        item.candidate_manifest_sha256 = candidate_manifest_sha256(&item).unwrap();
        item
    }

    fn scenario(
        scenario_id: &str,
        scenario_weight: f64,
        probability: f64,
    ) -> ProbeLikelihoodScenario {
        let other = 1.0 - probability;
        ProbeLikelihoodScenario {
            scenario_id: scenario_id.into(),
            scenario_weight,
            outcomes: vec![
                ProbeOutcomeLikelihood {
                    outcome_id: "bearing_signature".into(),
                    likelihood_by_hypothesis: vec![probability, other],
                },
                ProbeOutcomeLikelihood {
                    outcome_id: "fuel_signature".into(),
                    likelihood_by_hypothesis: vec![other, probability],
                },
            ],
        }
    }

    fn selected_probe_context(
        hypotheses: &[HypothesisPrior],
        candidates: &[DiagnosticProbeCandidate],
    ) -> (
        ProbeSelectionPolicy,
        ProbePortfolio,
        ProbeBaselineSelection,
        ProbeObservationLedger,
    ) {
        let policy = ProbeSelectionPolicy::conservative_read_only();
        let portfolio = rank_diagnostic_probes(hypotheses, candidates, &policy).unwrap();
        let selection = select_probe_baseline(
            &portfolio,
            candidates,
            ProbeBaseline::RiskAdjustedWorstCaseInformationGain,
            None,
            policy.cost_penalty_scale_seconds,
        )
        .unwrap();
        let ledger = new_probe_observation_ledger(hypotheses).unwrap();
        (policy, portfolio, selection, ledger)
    }

    #[test]
    fn robust_selector_rejects_optimistic_measurement_with_uninformative_plausible_scenario() {
        let mut uncertain = candidate(
            "uncertain_sensor",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        uncertain.outcomes.clear();
        uncertain.uncertainty_scenarios = vec![
            scenario("nominal", 0.5, 0.95),
            scenario("degraded_calibration", 0.5, 0.5),
        ];
        uncertain.candidate_manifest_sha256 = candidate_manifest_sha256(&uncertain).unwrap();

        let result = rank_diagnostic_probes(
            &hypotheses(),
            &[uncertain],
            &ProbeSelectionPolicy::conservative_read_only(),
        )
        .unwrap();
        assert!(result.ranked_recommendations.is_empty());
        let assessment = &result.assessments[0];
        assert!(assessment.expected_information_gain_nats > 0.0);
        assert_eq!(assessment.worst_case_information_gain_nats, 0.0);
        assert_eq!(assessment.scenario_count, 2);
        assert!(assessment.multi_scenario_assessed);
        assert!(
            assessment
                .rejection_reasons
                .contains(&ProbeRejectionReason::InformationGainBelowMinimum)
        );
    }

    #[test]
    fn robust_selector_accepts_only_when_all_calibrated_scenarios_are_informative() {
        let mut robust = candidate(
            "stable_pressure_sensor",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        robust.outcomes.clear();
        robust.uncertainty_scenarios = vec![
            scenario("nominal", 0.6, 0.95),
            scenario("high_noise", 0.4, 0.90),
        ];
        robust.candidate_manifest_sha256 = candidate_manifest_sha256(&robust).unwrap();

        let result = rank_diagnostic_probes(
            &hypotheses(),
            &[robust],
            &ProbeSelectionPolicy::conservative_read_only(),
        )
        .unwrap();
        assert_eq!(result.ranked_recommendations.len(), 1);
        let ranked = &result.ranked_recommendations[0];
        assert!(ranked.worst_case_information_gain_nats > 0.0);
        assert!(ranked.expected_information_gain_nats >= ranked.worst_case_information_gain_nats);
        assert_eq!(ranked.scenario_count, 2);
        assert!(ranked.multi_scenario_assessed);
    }

    #[test]
    fn likelihood_uncertainty_scenario_weights_must_sum_to_one() {
        let mut invalid = candidate(
            "bad_scenario_weights",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        invalid.outcomes.clear();
        invalid.uncertainty_scenarios =
            vec![scenario("nominal", 0.4, 0.95), scenario("drift", 0.4, 0.90)];
        invalid.candidate_manifest_sha256 = candidate_manifest_sha256(&invalid).unwrap();
        assert!(matches!(
            rank_diagnostic_probes(
                &hypotheses(),
                &[invalid],
                &ProbeSelectionPolicy::conservative_read_only()
            ),
            Err(ProbeError::InvalidScenarioSet(_))
        ));
    }

    #[test]
    fn chernoff_information_is_zero_for_identical_likelihoods_and_positive_for_separable_modes() {
        let identical = outcomes(false);
        let separated = outcomes(true);
        let prior = vec![0.5, 0.5];
        let identical_distance =
            minimum_pairwise_chernoff_information_nats(&prior, &identical, 100).unwrap();
        let separated_distance =
            minimum_pairwise_chernoff_information_nats(&prior, &separated, 100).unwrap();
        assert!(identical_distance.abs() < 1e-12);
        assert!(separated_distance > identical_distance);
        assert!(matches!(
            minimum_pairwise_chernoff_information_nats(&prior, &separated, 1),
            Err(ProbeError::InvalidChernoffGrid)
        ));
    }

    #[test]
    fn worst_case_chernoff_uses_the_least_separable_calibrated_scenario() {
        let mut candidate = candidate(
            "uncertain_chernoff",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        candidate.outcomes.clear();
        candidate.uncertainty_scenarios = vec![
            scenario("nominal", 0.5, 0.95),
            scenario("degraded", 0.5, 0.5),
        ];
        candidate.candidate_manifest_sha256 = candidate_manifest_sha256(&candidate).unwrap();
        let worst =
            worst_case_pairwise_chernoff_information_nats(&candidate, &[0.5, 0.5], 100).unwrap();
        assert!(worst.abs() < 1e-12);
    }

    #[test]
    fn baselines_compare_the_same_eligible_catalogue_and_are_deterministic() {
        let candidates = vec![
            candidate_with_probability("cheap_low_gain", 0.60, 5.0),
            candidate_with_probability("balanced", 0.80, 50.0),
            candidate_with_probability("expensive_high_gain", 0.95, 300.0),
        ];
        let policy = ProbeSelectionPolicy::conservative_read_only();
        let portfolio = rank_diagnostic_probes(&hypotheses(), &candidates, &policy).unwrap();
        let seed = digest('9');

        let no_evidence = select_probe_baseline(
            &portfolio,
            &candidates,
            ProbeBaseline::NoAdditionalEvidence,
            None,
            policy.cost_penalty_scale_seconds,
        )
        .unwrap();
        let cheapest = select_probe_baseline(
            &portfolio,
            &candidates,
            ProbeBaseline::CheapestEligible,
            None,
            policy.cost_penalty_scale_seconds,
        )
        .unwrap();
        let maximum_gain = select_probe_baseline(
            &portfolio,
            &candidates,
            ProbeBaseline::MaximumInformationGain,
            None,
            policy.cost_penalty_scale_seconds,
        )
        .unwrap();
        let per_cost = select_probe_baseline(
            &portfolio,
            &candidates,
            ProbeBaseline::InformationGainPerCost,
            None,
            policy.cost_penalty_scale_seconds,
        )
        .unwrap();
        let random_first = select_probe_baseline(
            &portfolio,
            &candidates,
            ProbeBaseline::SeededRandomEligible,
            Some(&seed),
            policy.cost_penalty_scale_seconds,
        )
        .unwrap();
        let random_second = select_probe_baseline(
            &portfolio,
            &candidates,
            ProbeBaseline::SeededRandomEligible,
            Some(&seed),
            policy.cost_penalty_scale_seconds,
        )
        .unwrap();
        let robust = select_probe_baseline(
            &portfolio,
            &candidates,
            ProbeBaseline::RiskAdjustedWorstCaseInformationGain,
            None,
            policy.cost_penalty_scale_seconds,
        )
        .unwrap();
        let chernoff = select_probe_baseline(
            &portfolio,
            &candidates,
            ProbeBaseline::MaximumWorstCaseChernoffInformation,
            None,
            policy.cost_penalty_scale_seconds,
        )
        .unwrap();

        assert_eq!(no_evidence.selected_candidate_id, None);
        assert_eq!(
            cheapest.selected_candidate_id.as_deref(),
            Some("cheap_low_gain")
        );
        assert_eq!(
            maximum_gain.selected_candidate_id.as_deref(),
            Some("expensive_high_gain")
        );
        assert!(per_cost.selected_candidate_id.is_some());
        assert_eq!(
            random_first.selected_candidate_id,
            random_second.selected_candidate_id
        );
        assert_eq!(
            robust.selected_candidate_id,
            Some(portfolio.ranked_recommendations[0].candidate_id.clone())
        );
        assert_eq!(
            chernoff.selected_candidate_id.as_deref(),
            Some("expensive_high_gain")
        );
        for selection in [
            &cheapest,
            &maximum_gain,
            &per_cost,
            &random_first,
            &robust,
            &chernoff,
        ] {
            assert_eq!(selection.eligible_candidate_count, 3);
            assert_eq!(
                selection.eligible_catalogue_sha256,
                cheapest.eligible_catalogue_sha256
            );
        }
    }

    #[test]
    fn seeded_random_baseline_requires_a_valid_frozen_seed_digest() {
        let candidates = vec![candidate(
            "informative",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        )];
        let policy = ProbeSelectionPolicy::conservative_read_only();
        let portfolio = rank_diagnostic_probes(&hypotheses(), &candidates, &policy).unwrap();
        assert_eq!(
            select_probe_baseline(
                &portfolio,
                &candidates,
                ProbeBaseline::SeededRandomEligible,
                Some("not-a-digest"),
                policy.cost_penalty_scale_seconds,
            ),
            Err(ProbeError::InvalidSeedDigest)
        );
    }

    #[test]
    fn acquired_observation_updates_posterior_and_binds_candidate_and_receipt() {
        let item = candidate(
            "pressure_observation",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        let hypotheses = hypotheses();
        let candidates = vec![item.clone()];
        let (policy, portfolio, selection, ledger) =
            selected_probe_context(&hypotheses, &candidates);
        let update = realize_probe_observation(
            &hypotheses,
            &candidates,
            &policy,
            &portfolio,
            &selection,
            &ledger,
            &item,
            &item.candidate_manifest_sha256,
            "bearing_signature",
            &digest('e'),
        )
        .unwrap();
        assert_eq!(update.candidate_id, "pressure_observation");
        assert_eq!(update.observation_receipt_sha256, digest('e'));
        assert!((update.posterior[0].probability - 0.95).abs() < 1e-12);
        assert!((update.posterior[1].probability - 0.05).abs() < 1e-12);
        assert!(update.realized_entropy_reduction_nats > 0.0);
        assert!(update.realized_kl_divergence_nats > 0.0);
        assert_eq!(update.likelihood_scenario_count, 1);
        assert_eq!(update.possible_likelihood_scenario_count, 1);
        assert_eq!(update.impossible_likelihood_scenario_count, 0);
    }

    #[test]
    fn sequential_update_rejects_persistent_scenario_models_without_joint_posterior_ledger() {
        let mut item = candidate(
            "noisy_pressure_observation",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        item.outcomes.clear();
        item.uncertainty_scenarios = vec![
            scenario("nominal", 0.5, 0.95),
            scenario("degraded", 0.5, 0.90),
        ];
        item.candidate_manifest_sha256 = candidate_manifest_sha256(&item).unwrap();
        let hypotheses = hypotheses();
        let candidates = vec![item.clone()];
        let (policy, portfolio, selection, ledger) =
            selected_probe_context(&hypotheses, &candidates);
        assert_eq!(
            realize_probe_observation(
                &hypotheses,
                &candidates,
                &policy,
                &portfolio,
                &selection,
                &ledger,
                &item,
                &item.candidate_manifest_sha256,
                "bearing_signature",
                &digest('d'),
            ),
            Err(ProbeError::MultiScenarioSequentialUpdateRequiresJointPosterior)
        );
    }

    #[test]
    fn observed_update_rejects_changed_candidate_and_unknown_or_impossible_outcomes() {
        let item = candidate(
            "pressure_observation",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        let hypotheses = hypotheses();
        let candidates = vec![item.clone()];
        let (policy, portfolio, selection, ledger) =
            selected_probe_context(&hypotheses, &candidates);
        assert_eq!(
            realize_probe_observation(
                &hypotheses,
                &candidates,
                &policy,
                &portfolio,
                &selection,
                &ledger,
                &item,
                &digest('0'),
                "bearing_signature",
                &digest('e')
            ),
            Err(ProbeError::ObservationCandidateMismatch)
        );
        assert_eq!(
            realize_probe_observation(
                &hypotheses,
                &candidates,
                &policy,
                &portfolio,
                &selection,
                &ledger,
                &item,
                &item.candidate_manifest_sha256,
                "unlisted_outcome",
                &digest('e')
            ),
            Err(ProbeError::ObservedOutcomeNotFound(
                "unlisted_outcome".into()
            ))
        );

        let mut impossible = candidate(
            "impossible_measurement",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        impossible.outcomes = vec![
            ProbeOutcomeLikelihood {
                outcome_id: "bearing_signature".into(),
                likelihood_by_hypothesis: vec![1.0, 0.0],
            },
            ProbeOutcomeLikelihood {
                outcome_id: "fuel_signature".into(),
                likelihood_by_hypothesis: vec![0.0, 1.0],
            },
            ProbeOutcomeLikelihood {
                outcome_id: "impossible_signature".into(),
                likelihood_by_hypothesis: vec![0.0, 0.0],
            },
        ];
        impossible.candidate_manifest_sha256 = candidate_manifest_sha256(&impossible).unwrap();
        let impossible_candidates = vec![impossible.clone()];
        let (policy, portfolio, selection, ledger) =
            selected_probe_context(&hypotheses, &impossible_candidates);
        assert_eq!(
            realize_probe_observation(
                &hypotheses,
                &impossible_candidates,
                &policy,
                &portfolio,
                &selection,
                &ledger,
                &impossible,
                &impossible.candidate_manifest_sha256,
                "impossible_signature",
                &digest('e')
            ),
            Err(ProbeError::ObservedOutcomeImpossible(
                "impossible_signature".into()
            ))
        );
    }

    #[test]
    fn hypothesis_space_digest_is_stable_while_prior_distribution_changes() {
        let initial = hypotheses();
        let updated = vec![
            HypothesisPrior {
                hypothesis_id: "bearing_degradation".into(),
                prior_probability: 0.95,
            },
            HypothesisPrior {
                hypothesis_id: "fuel_delivery_fault".into(),
                prior_probability: 0.05,
            },
        ];
        assert_eq!(
            hypothesis_manifest_sha256(&initial).unwrap(),
            hypothesis_manifest_sha256(&updated).unwrap()
        );
        assert_ne!(
            hypothesis_prior_sha256(&initial).unwrap(),
            hypothesis_prior_sha256(&updated).unwrap()
        );
    }

    #[test]
    fn sequential_observations_reuse_stable_candidate_likelihood_model() {
        let item = candidate(
            "pressure_observation",
            ProbeClass::PassiveMeasurement,
            ProbeAuthorization::ApprovedReadOnly,
            true,
        );
        let candidates = vec![item.clone()];
        let initial = hypotheses();
        let (policy, first_portfolio, first_selection, ledger) =
            selected_probe_context(&initial, &candidates);
        let first = realize_probe_observation(
            &initial,
            &candidates,
            &policy,
            &first_portfolio,
            &first_selection,
            &ledger,
            &item,
            &item.candidate_manifest_sha256,
            "bearing_signature",
            &digest('e'),
        )
        .unwrap();

        let updated_prior = first
            .posterior
            .iter()
            .map(|entry| HypothesisPrior {
                hypothesis_id: entry.hypothesis_id.clone(),
                prior_probability: entry.probability,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            hypothesis_manifest_sha256(&initial).unwrap(),
            hypothesis_manifest_sha256(&updated_prior).unwrap()
        );
        assert_ne!(
            hypothesis_prior_sha256(&initial).unwrap(),
            hypothesis_prior_sha256(&updated_prior).unwrap()
        );

        let (second_policy, second_portfolio, second_selection, _) =
            selected_probe_context(&updated_prior, &candidates);
        assert_eq!(
            second_portfolio.hypothesis_manifest_sha256,
            first_portfolio.hypothesis_manifest_sha256
        );
        assert_eq!(
            second_portfolio.prior_distribution_sha256,
            hypothesis_prior_sha256(&updated_prior).unwrap()
        );
        let mut tampered_ledger = first.observation_ledger.clone();
        tampered_ledger.current_posterior_manifest_sha256 = digest('0');
        assert_eq!(
            realize_probe_observation(
                &updated_prior,
                &candidates,
                &second_policy,
                &second_portfolio,
                &second_selection,
                &tampered_ledger,
                &item,
                &item.candidate_manifest_sha256,
                "fuel_signature",
                &digest('f')
            ),
            Err(ProbeError::InvalidObservationLedger)
        );
        assert_eq!(
            realize_probe_observation(
                &updated_prior,
                &candidates,
                &second_policy,
                &second_portfolio,
                &second_selection,
                &first.observation_ledger,
                &item,
                &item.candidate_manifest_sha256,
                "fuel_signature",
                &digest('e')
            ),
            Err(ProbeError::ObservationAlreadyApplied)
        );

        let second = realize_probe_observation(
            &updated_prior,
            &candidates,
            &second_policy,
            &second_portfolio,
            &second_selection,
            &first.observation_ledger,
            &item,
            &item.candidate_manifest_sha256,
            "fuel_signature",
            &digest('f'),
        )
        .unwrap();

        assert_eq!(
            second.previous_posterior_manifest_sha256,
            first.posterior_manifest_sha256
        );
        assert_eq!(
            second
                .observation_ledger
                .applied_observation_receipt_sha256
                .len(),
            2
        );
        assert!(second.realized_entropy_reduction_nats < 0.0);
        assert!(second.realized_kl_divergence_nats > 0.0);
    }

    #[test]
    fn hypothesis_manifest_is_independent_of_input_order() {
        let forward = hypotheses();
        let reverse = forward.iter().cloned().rev().collect::<Vec<_>>();
        assert_eq!(
            hypothesis_manifest_sha256(&forward).unwrap(),
            hypothesis_manifest_sha256(&reverse).unwrap()
        );
    }

    #[test]
    fn active_evidence_protocol_preserves_non_execution_and_authority_boundaries() {
        let protocol: serde_json::Value =
            serde_json::from_str(include_str!("../active-evidence-selection-v0.1.json")).unwrap();
        assert_eq!(
            protocol["schema_version"],
            "liu-ice-active-evidence-selection-v0.6"
        );
        assert_eq!(protocol["status"], "protocol_only_not_executed");
        assert!(
            protocol["method"]["identity_contract"]["hypothesis_space_digest"]
                .as_str()
                .unwrap()
                .contains("sorted unique hypothesis IDs only")
        );
        assert!(
            protocol["method"]["identity_contract"]["prior_distribution_digest"]
                .as_str()
                .unwrap()
                .contains("changes whenever evidence changes the posterior")
        );
        assert_eq!(protocol["comparators"].as_array().unwrap().len(), 7);
        assert_eq!(
            protocol["candidate_classes"]["controlled_perturbation"]["eligible_when"],
            false
        );
        assert!(
            protocol["authority"]["authority_status"]
                .as_str()
                .unwrap()
                .contains("external mandatory checks")
        );
        assert!(protocol["hard_gates"].as_array().unwrap().len() >= 50);
        assert!(
            protocol["method"]["observation_update"]["posterior"]
                .as_str()
                .unwrap()
                .contains("joint_scenario_posterior")
        );
        assert_eq!(
            protocol["method"]["sequential_update_policy"]["status"],
            "joint_hypothesis_scenario_ledger_supported"
        );
        assert_eq!(
            protocol["method"]["sequential_update_policy"]["no_silent_fallback"],
            true
        );
        assert_eq!(
            protocol["method"]["joint_scenario_ledger"]["rust_module"],
            "src/joint_scenario_posterior.rs"
        );
        assert_eq!(
            protocol["method"]["joint_scenario_ledger"]["genesis"],
            "p(H,S)=p(H)*p(S)"
        );
        assert_eq!(
            protocol["method"]["joint_scenario_ledger"]["status"],
            "source_authored_ci_pending"
        );
        assert!(
            protocol["method"]["sequential_update_policy"]["scope"]
                .as_str()
                .unwrap()
                .contains("One ledger is bound to one immutable selected candidate")
        );
        assert!(
            protocol["method"]["sequential_update_policy"]["ledger"]
                .as_str()
                .unwrap()
                .contains("append-only ledger")
        );
        assert_eq!(
            protocol["method"]["sequential_update_policy"]["factorized_prior_assumption"],
            "v0.1 genesis prior assumes p(H,S)=p(H)*p(S). A dependent joint prior requires a separately reviewed extension."
        );
        assert!(
            protocol["method"]["observation_update"]["realized_information"]
                .as_str()
                .unwrap()
                .contains("may be negative")
        );
        assert!(
            protocol["method"]["observation_update"]["receipt_binding"]
                .as_str()
                .unwrap()
                .contains("does not authenticate its signer")
        );
        assert!(
            protocol["required_metrics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|metric| metric == "realized_kl_divergence_nats")
        );
        assert!(
            protocol["method"]["uncertainty_scenarios"]["eligibility"]
                .as_str()
                .unwrap()
                .contains("worst-case")
        );
        assert!(
            protocol["method"]["likelihood_calibration_review"]["requirement"]
                .as_str()
                .unwrap()
                .contains("independent calibration-review receipt")
        );
        assert!(
            protocol["method"]["likelihood_calibration_review"]["qualification"]
                .as_str()
                .unwrap()
                .contains("not self-authenticating")
        );
        assert!(
            protocol["method"]["chernoff_information"]["assumptions"]
                .as_str()
                .unwrap()
                .contains("not a finite-sample error guarantee")
        );
        assert!(
            protocol["comparators"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["id"] == "maximum_worst_case_pairwise_chernoff_information")
        );
        assert!(protocol["hard_gates"].as_array().unwrap().len() >= 50);
        assert!(
            protocol["comparators"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["id"] == "risk_adjusted_worst_case_eig")
        );
        assert!(
            protocol["research_targets_not_achieved"]
                .as_array()
                .unwrap()
                .iter()
                .all(|target| target["status"] == "aspirational")
        );
    }

    #[test]
    fn run_manifest_requires_joint_scenario_ledger_digest_for_joint_qualification() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../benchmark-run-manifest.schema.json")).unwrap();
        let active = &schema["properties"]["protocol"]["properties"]["active_evidence_selection"];
        assert!(
            active["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field == "sequential_update_status")
        );
        let conditions = active["allOf"].as_array().unwrap();
        let rule_for = |status: &str| {
            conditions
                .iter()
                .find(|condition| {
                    condition["if"]["properties"]["sequential_update_status"]["const"] == status
                })
                .expect("run-manifest schema retains the requested status rule")
        };
        let qualified = rule_for("joint_hypothesis_scenario_ledger_qualified");
        assert_eq!(
            qualified["then"]["properties"]["joint_scenario_posterior_ledger_sha256"]["type"],
            "string"
        );
        let blocked = rule_for("multi_scenario_blocked_joint_posterior_missing");
        assert_eq!(
            blocked["then"]["properties"]["joint_scenario_posterior_ledger_sha256"]["type"],
            "null"
        );
        let synthetic = rule_for("joint_scenario_ledger_synthetic_contract_only");
        assert_eq!(
            synthetic["then"]["properties"]["joint_scenario_posterior_ledger_sha256"]["type"],
            "string"
        );
        assert_eq!(
            synthetic["then"]["properties"]["joint_scenario_observation_count"]["type"],
            "integer"
        );
    }

    #[test]
    fn duplicate_hypotheses_are_rejected_before_evidence_selection() {
        let mut hypotheses = hypotheses();
        hypotheses.push(hypotheses[0].clone());
        assert_eq!(
            rank_diagnostic_probes(
                &hypotheses,
                &[],
                &ProbeSelectionPolicy::conservative_read_only()
            ),
            Err(ProbeError::DuplicateHypothesis(
                "bearing_degradation".into()
            ))
        );
    }
}
