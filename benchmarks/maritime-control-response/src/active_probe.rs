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
pub struct DiagnosticProbeCandidate {
    pub candidate_id: String,
    pub probe_class: ProbeClass,
    pub authorization: ProbeAuthorization,
    pub authorization_receipt_sha256: Option<String>,
    pub likelihood_model_sha256: String,
    pub likelihood_calibration_sha256: String,
    pub hypothesis_manifest_sha256: String,
    pub expected_cost_seconds: f64,
    pub expected_delay_seconds: f64,
    /// Risk estimate in parts per million, 0..=1_000_000.
    pub risk_score_ppm: u32,
    pub outcomes: Vec<ProbeOutcomeLikelihood>,
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
            minimum_expected_information_gain_nats: 0.0,
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
    InformationGainBelowMinimum,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ProbeAssessment {
    pub candidate_id: String,
    pub eligible: bool,
    pub rejection_reasons: Vec<ProbeRejectionReason>,
    pub expected_information_gain_nats: f64,
    pub expected_posterior_entropy_nats: f64,
    pub risk_adjusted_utility: Option<f64>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct RankedDiagnosticProbe {
    pub rank: usize,
    pub candidate_id: String,
    pub probe_class: ProbeClass,
    pub expected_information_gain_nats: f64,
    pub expected_posterior_entropy_nats: f64,
    pub risk_adjusted_utility: f64,
    pub expected_cost_seconds: f64,
    pub expected_delay_seconds: f64,
    pub risk_score_ppm: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ProbePortfolio {
    pub hypothesis_manifest_sha256: String,
    pub policy_sha256: String,
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
    InvalidInformationGain(String),
    Serialization,
}

impl fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHypothesisSet => write!(formatter, "hypotheses need at least two unique IDs and a finite prior distribution summing to one"),
            Self::InvalidPolicy => write!(formatter, "probe selection policy contains invalid limits or penalty scales"),
            Self::EmptyIdentity => write!(formatter, "probe or hypothesis identity is empty"),
            Self::DuplicateHypothesis(id) => write!(formatter, "duplicate hypothesis identity: {id}"),
            Self::DuplicateCandidate(id) => write!(formatter, "duplicate probe candidate identity: {id}"),
            Self::InvalidCandidateDigest(id) => write!(formatter, "candidate manifest digest is invalid: {id}"),
            Self::HypothesisManifestMismatch(id) => write!(formatter, "candidate likelihood table belongs to a different hypothesis manifest: {id}"),
            Self::InvalidLikelihoodTable(id) => write!(formatter, "probe likelihood table is empty, incomplete, or has duplicate outcomes: {id}"),
            Self::InvalidLikelihoodProbability(id) => write!(formatter, "probe likelihood is negative, non-finite, or above one: {id}"),
            Self::InvalidLikelihoodNormalization(id) => write!(formatter, "conditional outcomes do not sum to one for every hypothesis: {id}"),
            Self::InvalidInformationGain(id) => write!(formatter, "expected information gain is numerically invalid: {id}"),
            Self::Serialization => write!(formatter, "failed to serialize probe evidence for canonical hashing"),
        }
    }
}

impl std::error::Error for ProbeError {}

/// Compute the canonical digest of a prior and hypothesis label space.
pub fn hypothesis_manifest_sha256(
    hypotheses: &[HypothesisPrior],
) -> Result<String, ProbeError> {
    let canonical = canonical_hypotheses(hypotheses)?;
    sha256_serializable(
        b"symtropy-diagnostic-hypothesis-manifest-v1\0",
        &canonical,
    )
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
        candidate.hypothesis_manifest_sha256.as_str(),
        candidate.expected_cost_seconds,
        candidate.expected_delay_seconds,
        candidate.risk_score_ppm,
        &candidate.outcomes,
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
    let hypothesis_digest = sha256_serializable(
        b"symtropy-diagnostic-hypothesis-manifest-v1\0",
        &canonical_hypotheses,
    )?;
    let policy_digest =
        sha256_serializable(b"symtropy-diagnostic-probe-policy-v1\0", policy)?;
    let prior_entropy = entropy(&prior);

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

        validate_likelihood_table(candidate, hypothesis_ids.len())?;
        let (information_gain, posterior_entropy) =
            expected_information_gain(&prior, &candidate.outcomes)
                .map_err(|_| ProbeError::InvalidInformationGain(candidate.candidate_id.clone()))?;

        let mut rejection_reasons = Vec::new();
        match candidate.probe_class {
            ProbeClass::ControlledPerturbation => rejection_reasons
                .push(ProbeRejectionReason::ControlledPerturbationRequiresSeparateSafetyCase),
            ProbeClass::PassiveMeasurement => {
                if candidate.authorization != ProbeAuthorization::ApprovedReadOnly {
                    rejection_reasons
                        .push(ProbeRejectionReason::AuthorizationNotApproved);
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
                    rejection_reasons
                        .push(ProbeRejectionReason::OperatorInspectionDisabled);
                }
                if candidate.authorization != ProbeAuthorization::HumanAuthorized {
                    rejection_reasons
                        .push(ProbeRejectionReason::AuthorizationNotApproved);
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
        if information_gain < policy.minimum_expected_information_gain_nats {
            rejection_reasons.push(ProbeRejectionReason::InformationGainBelowMinimum);
        }

        let eligible = rejection_reasons.is_empty();
        let utility = eligible.then(|| {
            information_gain
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
            expected_posterior_entropy_nats: posterior_entropy,
            risk_adjusted_utility: utility,
        });
        if let Some(utility) = utility {
            ranked.push(RankedDiagnosticProbe {
                rank: 0,
                candidate_id: candidate.candidate_id.clone(),
                probe_class: candidate.probe_class,
                expected_information_gain_nats: information_gain,
                expected_posterior_entropy_nats: posterior_entropy,
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
        policy_sha256: policy_digest,
        prior_entropy_nats: prior_entropy,
        ranked_recommendations: ranked,
        assessments,
        execution_performed: false,
        actuation_authorized: false,
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
    candidate: &DiagnosticProbeCandidate,
    hypothesis_count: usize,
) -> Result<(), ProbeError> {
    if candidate.outcomes.len() < 2 {
        return Err(ProbeError::InvalidLikelihoodTable(
            candidate.candidate_id.clone(),
        ));
    }
    let mut outcome_ids = BTreeSet::new();
    for outcome in &candidate.outcomes {
        if outcome.outcome_id.trim().is_empty()
            || !outcome_ids.insert(outcome.outcome_id.clone())
            || outcome.likelihood_by_hypothesis.len() != hypothesis_count
        {
            return Err(ProbeError::InvalidLikelihoodTable(
                candidate.candidate_id.clone(),
            ));
        }
        if outcome
            .likelihood_by_hypothesis
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0 || *value > 1.0)
        {
            return Err(ProbeError::InvalidLikelihoodProbability(
                candidate.candidate_id.clone(),
            ));
        }
    }
    for hypothesis_index in 0..hypothesis_count {
        let sum = candidate
            .outcomes
            .iter()
            .map(|outcome| outcome.likelihood_by_hypothesis[hypothesis_index])
            .sum::<f64>();
        if !sum.is_finite() || (sum - 1.0).abs() > PROBABILITY_TOLERANCE {
            return Err(ProbeError::InvalidLikelihoodNormalization(
                candidate.candidate_id.clone(),
            ));
        }
    }
    Ok(())
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
        if !posterior_total.is_finite()
            || (posterior_total - 1.0).abs() > 1e-8
        {
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
            hypothesis_manifest_sha256,
            expected_cost_seconds: 30.0,
            expected_delay_seconds: 10.0,
            risk_score_ppm: 1_000,
            outcomes: outcomes(informative),
            candidate_manifest_sha256: digest('0'),
        };
        candidate.candidate_manifest_sha256 = candidate_manifest_sha256(&candidate).unwrap();
        candidate
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
        assert_eq!(result.ranked_recommendations[0].candidate_id, "discriminating_probe");
        assert!(
            result.ranked_recommendations[0].expected_information_gain_nats
                > result.ranked_recommendations[1].expected_information_gain_nats
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
        assert!(result.assessments[0]
            .rejection_reasons
            .contains(&ProbeRejectionReason::ControlledPerturbationRequiresSeparateSafetyCase));
    }

    #[test]
    fn operator_inspection_needs_explicit_human_authority_and_policy_enablement() {
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
        assert!(result.assessments[0]
            .rejection_reasons
            .contains(&ProbeRejectionReason::OperatorInspectionDisabled));
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
        assert!(result.assessments.iter().all(|assessment| !assessment.eligible));
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
