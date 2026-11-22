//! Anytime-valid sequential model-discrimination evidence.
//!
//! For a fixed null model and a fixed prior mixture over alternative models,
//! the mixture of conditional likelihood ratios is a nonnegative martingale
//! under the null when every forecast is the correctly specified conditional
//! distribution given the past and the action chosen using only past data.
//! Ville's inequality then controls the probability of ever crossing 1/alpha.
//!
//! This module does not make an invalid predictive model valid. The caller must
//! bind the forecast to the complete pre-outcome history and intervention,
//! freeze the prediction manifest, and supply calibrated conditional
//! probabilities. Plug-in likelihoods that ignore serial dependence or
//! pseudoreplicate animals do not inherit the guarantee.

use std::collections::{BTreeMap, BTreeSet};

use super::{EvidenceTier, Feasibility, InterventionCandidate, OutcomeLikelihood, Posterior};

const PROB_TOL: f64 = 1.0e-10;
const FINGERPRINT_ALGORITHM: &str = "fnv1a64-non-cryptographic-v1";

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalForecast {
    /// Immutable model/estimator/parameter/calibration manifest identifier.
    pub prediction_manifest_digest: String,
    /// Digest of the pre-outcome history used to form this conditional forecast.
    pub conditioning_state_digest: String,
    pub candidate_id: String,
    pub protocol_id: String,
    pub outcomes: Vec<OutcomeLikelihood>,
}

/// One observed outcome event, captured after the action and delivery are audited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EProcessObservation<'a> {
    pub assigned_candidate_id: &'a str,
    pub delivered_protocol_id: &'a str,
    pub step_id: &'a str,
    pub outcome_id: &'a str,
    pub sample_id: &'a str,
    pub replication_unit_id: &'a str,
    pub session_id: &'a str,
    pub evidence_tier: EvidenceTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EProcessError {
    EmptyIdentifier(&'static str),
    InvalidAlpha,
    InvalidAlternativePrior,
    NullModelInAlternativeSet,
    PredictionManifestDrift,
    EmptyConditioningState,
    AssignmentMismatch,
    DeliveryMismatch,
    CandidateNotPermitted,
    DuplicateStep(String),
    DuplicateSample(String),
    DuplicateOutcome(String),
    EmptyOutcomeSet,
    ModelSetMismatch,
    InvalidProbability,
    UnnormalizedConditionalLikelihood,
    UnknownOutcome(String),
    NullModelAssignedZeroProbability,
    NumericalFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus {
    Continue,
    RejectNullAnytimeValid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EProcessFingerprint {
    /// FNV-1a 64-bit: accidental-drift detection only, not cryptographic security.
    pub algorithm: String,
    pub value_hex: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnytimeEvidenceReceipt {
    pub step_id: String,
    pub sample_id: String,
    pub replication_unit_id: String,
    pub session_id: String,
    pub steps_observed: u64,
    pub evidence_tier: EvidenceTier,
    pub log_e_value: f64,
    pub log_rejection_threshold: f64,
    pub alpha: f64,
    pub status: EvidenceStatus,
    pub fingerprint: EProcessFingerprint,
}

/// A pre-registered likelihood-ratio e-process for one null family.
///
/// Alternative model weights are frozen at construction. To test multiple null
/// families, allocate alpha across them before looking at outcomes (for example,
/// fixed Bonferroni allocations whose sum does not exceed the familywise alpha).
#[derive(Debug, Clone, PartialEq)]
pub struct LikelihoodRatioEProcess {
    null_model_id: String,
    alternative_weights: Posterior,
    log_likelihood_ratios: BTreeMap<String, f64>,
    alpha: f64,
    frozen_prediction_manifest_digest: String,
    seen_step_ids: BTreeSet<String>,
    seen_sample_ids: BTreeSet<String>,
    steps_observed: u64,
    log_e_value: f64,
}

impl LikelihoodRatioEProcess {
    pub fn new(
        null_model_id: impl Into<String>,
        alternative_weights: Posterior,
        alpha: f64,
        frozen_prediction_manifest_digest: impl Into<String>,
    ) -> Result<Self, EProcessError> {
        let null_model_id = null_model_id.into();
        let frozen_prediction_manifest_digest = frozen_prediction_manifest_digest.into();
        if null_model_id.trim().is_empty() {
            return Err(EProcessError::EmptyIdentifier("null_model_id"));
        }
        if frozen_prediction_manifest_digest.trim().is_empty() {
            return Err(EProcessError::EmptyIdentifier("prediction_manifest_digest"));
        }
        if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
            return Err(EProcessError::InvalidAlpha);
        }
        if alternative_weights.is_empty()
            || alternative_weights
                .iter()
                .any(|(id, weight)| id.trim().is_empty() || !weight.is_finite() || *weight <= 0.0)
            || (alternative_weights.values().sum::<f64>() - 1.0).abs() > PROB_TOL
        {
            return Err(EProcessError::InvalidAlternativePrior);
        }
        if alternative_weights.contains_key(&null_model_id) {
            return Err(EProcessError::NullModelInAlternativeSet);
        }
        let log_likelihood_ratios = alternative_weights
            .keys()
            .cloned()
            .map(|id| (id, 0.0))
            .collect();
        Ok(Self {
            null_model_id,
            alternative_weights,
            log_likelihood_ratios,
            alpha,
            frozen_prediction_manifest_digest,
            seen_step_ids: BTreeSet::new(),
            seen_sample_ids: BTreeSet::new(),
            steps_observed: 0,
            log_e_value: 0.0,
        })
    }

    pub fn log_e_value(&self) -> f64 {
        self.log_e_value
    }

    pub fn steps_observed(&self) -> u64 {
        self.steps_observed
    }

    /// Add one outcome using conditional model probabilities frozen before
    /// observing that outcome. The action must have been chosen predictably
    /// from prior history. Repeated observations are valid only when the
    /// supplied likelihoods condition on the complete dependence history.
    pub fn observe(
        &mut self,
        candidate: &InterventionCandidate,
        forecast: &ConditionalForecast,
        observation: EProcessObservation<'_>,
    ) -> Result<AnytimeEvidenceReceipt, EProcessError> {
        if observation.step_id.trim().is_empty() {
            return Err(EProcessError::EmptyIdentifier("step_id"));
        }
        if observation.sample_id.trim().is_empty() {
            return Err(EProcessError::EmptyIdentifier("sample_id"));
        }
        if observation.replication_unit_id.trim().is_empty() {
            return Err(EProcessError::EmptyIdentifier("replication_unit_id"));
        }
        if observation.session_id.trim().is_empty() {
            return Err(EProcessError::EmptyIdentifier("session_id"));
        }
        if forecast.prediction_manifest_digest != self.frozen_prediction_manifest_digest {
            return Err(EProcessError::PredictionManifestDrift);
        }
        if forecast.conditioning_state_digest.trim().is_empty() {
            return Err(EProcessError::EmptyConditioningState);
        }
        if observation.assigned_candidate_id != candidate.candidate_id
            || forecast.candidate_id != candidate.candidate_id
        {
            return Err(EProcessError::AssignmentMismatch);
        }
        if observation.delivered_protocol_id != candidate.protocol_id
            || forecast.protocol_id != candidate.protocol_id
        {
            return Err(EProcessError::DeliveryMismatch);
        }
        if self.seen_step_ids.contains(observation.step_id) {
            return Err(EProcessError::DuplicateStep(observation.step_id.to_owned()));
        }
        if self.seen_sample_ids.contains(observation.sample_id) {
            return Err(EProcessError::DuplicateSample(
                observation.sample_id.to_owned(),
            ));
        }
        validate_candidate_for_eprocess(candidate)?;
        validate_conditional_forecast(forecast, &self.null_model_id, &self.alternative_weights)?;
        let outcome = forecast
            .outcomes
            .iter()
            .find(|item| item.outcome_id == observation.outcome_id)
            .ok_or_else(|| EProcessError::UnknownOutcome(observation.outcome_id.to_owned()))?;
        let p_null = outcome.model_probabilities[&self.null_model_id];
        // An exact-zero null likelihood is a model-adequacy failure, not an
        // infinite-evidence shortcut. Do not manufacture a rejection from it.
        if p_null <= 0.0 {
            return Err(EProcessError::NullModelAssignedZeroProbability);
        }

        let mut next_log_likelihood_ratios = self.log_likelihood_ratios.clone();
        for (alternative_id, log_ratio) in &mut next_log_likelihood_ratios {
            let p_alternative = outcome.model_probabilities[alternative_id];
            if p_alternative <= 0.0 {
                *log_ratio = f64::NEG_INFINITY;
            } else if log_ratio.is_finite() {
                *log_ratio += p_alternative.ln() - p_null.ln();
            }
        }
        let log_e_value =
            mixture_log_e_value(&self.alternative_weights, &next_log_likelihood_ratios);
        if log_e_value.is_nan() || log_e_value == f64::INFINITY {
            return Err(EProcessError::NumericalFailure);
        }
        let log_rejection_threshold = -self.alpha.ln();
        let status = if log_e_value >= log_rejection_threshold {
            EvidenceStatus::RejectNullAnytimeValid
        } else {
            EvidenceStatus::Continue
        };
        let next_steps_observed = self
            .steps_observed
            .checked_add(1)
            .ok_or(EProcessError::NumericalFailure)?;

        self.log_likelihood_ratios = next_log_likelihood_ratios;
        self.log_e_value = log_e_value;
        self.steps_observed = next_steps_observed;
        self.seen_step_ids.insert(observation.step_id.to_owned());
        self.seen_sample_ids
            .insert(observation.sample_id.to_owned());

        let fingerprint = receipt_fingerprint(ReceiptFingerprintInput {
            step_id: observation.step_id,
            steps_observed: self.steps_observed,
            prediction_manifest_digest: &forecast.prediction_manifest_digest,
            conditioning_state_digest: &forecast.conditioning_state_digest,
            candidate_id: &candidate.candidate_id,
            protocol_id: &candidate.protocol_id,
            outcome_id: observation.outcome_id,
            sample_id: observation.sample_id,
            replication_unit_id: observation.replication_unit_id,
            session_id: observation.session_id,
            evidence_tier: observation.evidence_tier,
            log_e_value: self.log_e_value,
            log_threshold: log_rejection_threshold,
            alpha: self.alpha,
        });
        Ok(AnytimeEvidenceReceipt {
            step_id: observation.step_id.to_owned(),
            sample_id: observation.sample_id.to_owned(),
            replication_unit_id: observation.replication_unit_id.to_owned(),
            session_id: observation.session_id.to_owned(),
            steps_observed: self.steps_observed,
            evidence_tier: observation.evidence_tier,
            log_e_value: self.log_e_value,
            log_rejection_threshold,
            alpha: self.alpha,
            status,
            fingerprint,
        })
    }
}

fn validate_candidate_for_eprocess(candidate: &InterventionCandidate) -> Result<(), EProcessError> {
    if candidate.candidate_id.trim().is_empty()
        || candidate.protocol_id.trim().is_empty()
        || candidate.cost_unit_id.trim().is_empty()
        || !candidate.cost.is_finite()
        || candidate.cost <= 0.0
        || !matches!(&candidate.feasibility, Feasibility::Permitted)
    {
        return Err(EProcessError::CandidateNotPermitted);
    }
    let validity = &candidate.causal_validity;
    if !validity.assignment_design_valid
        || !validity.delivery_identity_verified
        || !validity.delivery_fidelity.is_finite()
        || !(0.90..=1.0).contains(&validity.delivery_fidelity)
        || !validity.interference_controlled
        || !validity.carryover_cleared
    {
        return Err(EProcessError::CandidateNotPermitted);
    }
    Ok(())
}

fn validate_conditional_forecast(
    forecast: &ConditionalForecast,
    null_model_id: &str,
    alternative_weights: &Posterior,
) -> Result<(), EProcessError> {
    if forecast.outcomes.is_empty() {
        return Err(EProcessError::EmptyOutcomeSet);
    }
    let expected_models: BTreeSet<String> = std::iter::once(null_model_id.to_owned())
        .chain(alternative_weights.keys().cloned())
        .collect();
    let mut observed_models = BTreeSet::new();
    let mut totals: BTreeMap<String, f64> = expected_models
        .iter()
        .cloned()
        .map(|id| (id, 0.0))
        .collect();
    let mut outcome_ids = BTreeSet::new();
    for outcome in &forecast.outcomes {
        if outcome.outcome_id.trim().is_empty() {
            return Err(EProcessError::EmptyIdentifier("outcome_id"));
        }
        if !outcome_ids.insert(outcome.outcome_id.clone()) {
            return Err(EProcessError::DuplicateOutcome(outcome.outcome_id.clone()));
        }
        let model_ids: BTreeSet<String> = outcome.model_probabilities.keys().cloned().collect();
        if model_ids != expected_models {
            return Err(EProcessError::ModelSetMismatch);
        }
        for (model_id, probability) in &outcome.model_probabilities {
            if !probability.is_finite() || !(0.0..=1.0).contains(probability) {
                return Err(EProcessError::InvalidProbability);
            }
            *totals
                .get_mut(model_id)
                .expect("model identity set checked above") += probability;
        }
        observed_models.extend(model_ids);
    }
    if observed_models != expected_models
        || totals.values().any(|sum| (*sum - 1.0).abs() > PROB_TOL)
    {
        return Err(EProcessError::UnnormalizedConditionalLikelihood);
    }
    Ok(())
}

fn mixture_log_e_value(weights: &Posterior, log_likelihood_ratios: &BTreeMap<String, f64>) -> f64 {
    let terms: Vec<f64> = weights
        .iter()
        .map(|(id, weight)| weight.ln() + log_likelihood_ratios[id])
        .collect();
    let maximum = terms.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if maximum == f64::NEG_INFINITY {
        return f64::NEG_INFINITY;
    }
    maximum
        + terms
            .iter()
            .map(|term| (term - maximum).exp())
            .sum::<f64>()
            .ln()
}

#[allow(clippy::too_many_arguments)]
struct ReceiptFingerprintInput<'a> {
    step_id: &'a str,
    steps_observed: u64,
    prediction_manifest_digest: &'a str,
    conditioning_state_digest: &'a str,
    candidate_id: &'a str,
    protocol_id: &'a str,
    outcome_id: &'a str,
    sample_id: &'a str,
    replication_unit_id: &'a str,
    session_id: &'a str,
    evidence_tier: EvidenceTier,
    log_e_value: f64,
    log_threshold: f64,
    alpha: f64,
}

fn receipt_fingerprint(input: ReceiptFingerprintInput<'_>) -> EProcessFingerprint {
    let mut bytes = Vec::new();
    for value in [
        input.step_id,
        input.prediction_manifest_digest,
        input.conditioning_state_digest,
        input.candidate_id,
        input.protocol_id,
        input.outcome_id,
        input.sample_id,
        input.replication_unit_id,
        input.session_id,
    ] {
        bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    bytes.extend_from_slice(&input.steps_observed.to_be_bytes());
    bytes.push(match input.evidence_tier {
        EvidenceTier::SyntheticFixture => 0,
        EvidenceTier::InSilicoGenerated => 1,
        EvidenceTier::PreprintMeasured => 2,
        EvidenceTier::PeerReviewedMeasured => 3,
    });
    bytes.extend_from_slice(&input.log_e_value.to_bits().to_be_bytes());
    bytes.extend_from_slice(&input.log_threshold.to_bits().to_be_bytes());
    bytes.extend_from_slice(&input.alpha.to_bits().to_be_bytes());
    // FNV-1a is intentionally non-cryptographic; it is not a trust anchor.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    EProcessFingerprint {
        algorithm: FINGERPRINT_ALGORITHM.to_owned(),
        value_hex: format!("{hash:016x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> InterventionCandidate {
        InterventionCandidate {
            candidate_id: "stim-a".to_owned(),
            protocol_id: "protocol-a".to_owned(),
            cost_unit_id: "experiment-credit".to_owned(),
            cost: 1.0,
            feasibility: Feasibility::Permitted,
            causal_validity: super::super::CausalValidity {
                assignment_design_valid: true,
                delivery_fidelity: 1.0,
                delivery_identity_verified: true,
                interference_controlled: true,
                carryover_cleared: true,
            },
            outcomes: vec![],
        }
    }

    fn forecast(null_yes: f64, alt_yes: f64, manifest: &str, history: &str) -> ConditionalForecast {
        ConditionalForecast {
            prediction_manifest_digest: manifest.to_owned(),
            conditioning_state_digest: history.to_owned(),
            candidate_id: "stim-a".to_owned(),
            protocol_id: "protocol-a".to_owned(),
            outcomes: vec![
                OutcomeLikelihood {
                    outcome_id: "yes".to_owned(),
                    model_probabilities: BTreeMap::from([
                        ("null".to_owned(), null_yes),
                        ("alternative".to_owned(), alt_yes),
                    ]),
                },
                OutcomeLikelihood {
                    outcome_id: "no".to_owned(),
                    model_probabilities: BTreeMap::from([
                        ("null".to_owned(), 1.0 - null_yes),
                        ("alternative".to_owned(), 1.0 - alt_yes),
                    ]),
                },
            ],
        }
    }

    fn process(alpha: f64) -> LikelihoodRatioEProcess {
        LikelihoodRatioEProcess::new(
            "null",
            BTreeMap::from([("alternative".to_owned(), 1.0)]),
            alpha,
            "frozen-manifest-sha",
        )
        .unwrap()
    }

    fn observe(
        process: &mut LikelihoodRatioEProcess,
        outcome: &str,
        step: &str,
        f: &ConditionalForecast,
    ) -> AnytimeEvidenceReceipt {
        let c = candidate();
        process
            .observe(
                &c,
                f,
                EProcessObservation {
                    assigned_candidate_id: "stim-a",
                    delivered_protocol_id: "protocol-a",
                    step_id: step,
                    outcome_id: outcome,
                    sample_id: step,
                    replication_unit_id: "animal-synthetic-1",
                    session_id: "session-synthetic-1",
                    evidence_tier: EvidenceTier::SyntheticFixture,
                },
            )
            .unwrap()
    }

    #[test]
    fn identical_null_and_alternative_predictions_produce_e_value_one() {
        let mut e = process(0.05);
        let f = forecast(0.4, 0.4, "frozen-manifest-sha", "history-0");
        let receipt = observe(&mut e, "yes", "step-1", &f);
        assert_eq!(receipt.status, EvidenceStatus::Continue);
        assert!(receipt.log_e_value.abs() < 1.0e-12);
    }

    #[test]
    fn repeated_peeks_cross_anytime_threshold_in_log_space() {
        let mut e = process(0.05);
        for index in 0..3 {
            let f = forecast(0.2, 0.8, "frozen-manifest-sha", &format!("history-{index}"));
            let receipt = observe(&mut e, "yes", &format!("step-{index}"), &f);
            if index < 2 {
                assert_eq!(receipt.status, EvidenceStatus::Continue);
            } else {
                assert_eq!(receipt.status, EvidenceStatus::RejectNullAnytimeValid);
            }
        }
        assert!(e.log_e_value() >= 20.0_f64.ln());
    }

    #[test]
    fn prediction_manifest_drift_is_rejected_without_updating() {
        let mut e = process(0.05);
        let f = forecast(0.2, 0.8, "changed-manifest", "history-0");
        let c = candidate();
        let result = e.observe(
            &c,
            &f,
            EProcessObservation {
                assigned_candidate_id: "stim-a",
                delivered_protocol_id: "protocol-a",
                step_id: "step-1",
                outcome_id: "yes",
                sample_id: "sample-1",
                replication_unit_id: "animal-1",
                session_id: "session-1",
                evidence_tier: EvidenceTier::SyntheticFixture,
            },
        );
        assert_eq!(result, Err(EProcessError::PredictionManifestDrift));
        assert_eq!(e.steps_observed(), 0);
    }

    #[test]
    fn null_zero_probability_fails_closed() {
        let mut e = process(0.05);
        let f = forecast(0.0, 0.2, "frozen-manifest-sha", "history-0");
        let c = candidate();
        assert_eq!(
            e.observe(
                &c,
                &f,
                EProcessObservation {
                    assigned_candidate_id: "stim-a",
                    delivered_protocol_id: "protocol-a",
                    step_id: "step-1",
                    outcome_id: "yes",
                    sample_id: "sample-1",
                    replication_unit_id: "animal-1",
                    session_id: "session-1",
                    evidence_tier: EvidenceTier::SyntheticFixture,
                }
            ),
            Err(EProcessError::NullModelAssignedZeroProbability)
        );
        assert_eq!(e.steps_observed(), 0);
    }

    #[test]
    fn duplicate_step_cannot_be_counted_twice() {
        let mut e = process(0.05);
        let f = forecast(0.2, 0.8, "frozen-manifest-sha", "history-0");
        observe(&mut e, "yes", "step-1", &f);
        let c = candidate();
        assert_eq!(
            e.observe(
                &c,
                &f,
                EProcessObservation {
                    assigned_candidate_id: "stim-a",
                    delivered_protocol_id: "protocol-a",
                    step_id: "step-1",
                    outcome_id: "yes",
                    sample_id: "sample-1",
                    replication_unit_id: "animal-1",
                    session_id: "session-1",
                    evidence_tier: EvidenceTier::SyntheticFixture,
                }
            ),
            Err(EProcessError::DuplicateStep("step-1".to_owned()))
        );
        assert_eq!(e.steps_observed(), 1);
    }

    #[test]
    fn repeated_cluster_observations_are_allowed_only_as_sequential_events() {
        let mut e = process(0.05);
        let first = forecast(0.2, 0.8, "frozen-manifest-sha", "history-before-step-1");
        let first_receipt = observe(&mut e, "yes", "step-1", &first);
        let second = forecast(0.2, 0.8, "frozen-manifest-sha", "history-including-step-1");
        let second_receipt = observe(&mut e, "yes", "step-2", &second);
        assert_eq!(first_receipt.replication_unit_id, "animal-synthetic-1");
        assert_eq!(second_receipt.replication_unit_id, "animal-synthetic-1");
        assert_eq!(first_receipt.session_id, second_receipt.session_id);
        assert_eq!(e.steps_observed(), 2);
    }

    #[test]
    fn duplicate_sample_identity_cannot_be_counted_twice() {
        let mut e = process(0.05);
        let c = candidate();
        let f = forecast(0.2, 0.8, "frozen-manifest-sha", "history-0");
        let event = EProcessObservation {
            assigned_candidate_id: "stim-a",
            delivered_protocol_id: "protocol-a",
            step_id: "step-1",
            outcome_id: "yes",
            sample_id: "sample-duplicate",
            replication_unit_id: "animal-1",
            session_id: "session-1",
            evidence_tier: EvidenceTier::SyntheticFixture,
        };
        e.observe(&c, &f, event).unwrap();
        let repeated = EProcessObservation {
            step_id: "step-2",
            ..event
        };
        assert_eq!(
            e.observe(&c, &f, repeated),
            Err(EProcessError::DuplicateSample(
                "sample-duplicate".to_owned()
            ))
        );
        assert_eq!(e.steps_observed(), 1);
    }

    #[test]
    fn unsafe_candidate_cannot_contribute_evidence() {
        let mut e = process(0.05);
        let f = forecast(0.2, 0.8, "frozen-manifest-sha", "history-0");
        let mut c = candidate();
        c.feasibility = Feasibility::SafetyRejected {
            reason: "fixture safety failure".to_owned(),
        };
        assert_eq!(
            e.observe(
                &c,
                &f,
                EProcessObservation {
                    assigned_candidate_id: "stim-a",
                    delivered_protocol_id: "protocol-a",
                    step_id: "step-1",
                    outcome_id: "yes",
                    sample_id: "sample-1",
                    replication_unit_id: "animal-1",
                    session_id: "session-1",
                    evidence_tier: EvidenceTier::SyntheticFixture,
                }
            ),
            Err(EProcessError::CandidateNotPermitted)
        );
        assert_eq!(e.steps_observed(), 0);
    }

    #[test]
    fn exact_null_rejection_probability_is_bounded_at_any_stopping_time() {
        // Exhaustively enumerate every 12-observation path under Bernoulli(0.5).
        // The alternative is Bernoulli(0.8); alpha is 0.05, so the e-boundary is 20.
        let horizon = 12_u32;
        let threshold = 20.0_f64.ln();
        let mut rejection_probability = 0.0;
        for path in 0..(1_u32 << horizon) {
            let mut log_e = 0.0;
            let mut rejected = false;
            for step in 0..horizon {
                let success = (path & (1 << step)) != 0;
                log_e += if success {
                    (0.8_f64 / 0.5).ln()
                } else {
                    (0.2_f64 / 0.5).ln()
                };
                if log_e >= threshold {
                    rejected = true;
                    break;
                }
            }
            if rejected {
                rejection_probability += 0.5_f64.powi(horizon as i32);
            }
        }
        assert!(
            rejection_probability <= 0.05 + 1.0e-12,
            "observed exact null rejection probability {rejection_probability}"
        );
    }
}
