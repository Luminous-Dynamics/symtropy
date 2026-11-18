//! Public predictive models for the declared candidate family.
//!
//! The catalogue contains formal candidate priors, conditional initial
//! physical/sensor-state beliefs, and exact action-conditioned transition,
//! observation and hazard distributions. It never contains the sampled
//! owner generator identity, sampled hidden state or episode RNG seed.

use crate::{JointKernel, JointMass, JointOutcome, KernelRow, Rational};
use crate::episode::{PublicAction, PublicHypothesis};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Stable schema version for candidate predictive model catalogues.
pub const PUBLIC_MODEL_CATALOGUE_VERSION: &str = "sim-rel-public-model-catalogue-v0";

/// Maximum canonical serialized catalogue size accepted by the process protocol.
pub const MAX_PUBLIC_MODEL_CATALOGUE_BYTES: usize = 16 * 1_024 * 1_024;

/// Exact rational represented as decimal strings to avoid JSON/JavaScript
/// floating-point loss for u128 numerators and denominators.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicProbability {
    /// Non-negative integer numerator in canonical decimal form.
    pub numerator: String,
    /// Positive integer denominator in canonical decimal form.
    pub denominator: String,
}

impl PublicProbability {
    /// Construct a canonical exact rational probability.
    pub fn new(numerator: u128, denominator: u128) -> Result<Self, String> {
        let value = Rational::new(numerator, denominator)?;
        Ok(Self::from_rational(value))
    }

    /// Convert a core rational to its public lossless representation.
    pub fn from_rational(value: Rational) -> Self {
        Self {
            numerator: value.numerator().to_string(),
            denominator: value.denominator().to_string(),
        }
    }

    /// Parse and normalize the exact rational representation.
    pub fn to_rational(&self) -> Result<Rational, String> {
        let numerator = self
            .numerator
            .parse::<u128>()
            .map_err(|_| "public probability numerator is not a u128 decimal".to_owned())?;
        let denominator = self
            .denominator
            .parse::<u128>()
            .map_err(|_| "public probability denominator is not a u128 decimal".to_owned())?;
        let value = Rational::new(numerator, denominator)?;
        if value.numerator().to_string() != self.numerator
            || value.denominator().to_string() != self.denominator
        {
            return Err("public probability must use canonical reduced decimal form".to_owned());
        }
        if !value.is_probability() {
            return Err("public probability must be in [0, 1]".to_owned());
        }
        Ok(value)
    }
}

/// Conditional initial joint physical/sensor-state mass for one candidate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicInitialStateMass {
    /// Candidate-local physical state index.
    pub physical_state: usize,
    /// Candidate-local sensor state index.
    pub sensor_state: usize,
    /// Exact P(physical state, sensor state | candidate mechanism).
    pub conditional_mass: PublicProbability,
}

/// One public outcome in a candidate model transition row.
///
/// observation_id = None denotes a terminal hazard. Safe outcomes carry the
/// public categorical observation ID.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicTransitionOutcome {
    /// Exact conditional outcome probability.
    pub probability: PublicProbability,
    /// Next physical state index.
    pub next_physical_state: usize,
    /// Next sensor state index.
    pub next_sensor_state: usize,
    /// Public observation symbol, or None for a terminal hazard.
    pub observation_id: Option<String>,
}

/// Candidate-model transition row for one physical/sensor/action tuple.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicTransitionRow {
    /// Current physical state index.
    pub physical_state: usize,
    /// Current sensor state index.
    pub sensor_state: usize,
    /// Public action ID.
    pub action_id: String,
    /// Complete exact conditional outcome distribution.
    pub outcomes: Vec<PublicTransitionOutcome>,
}

/// Formal predictive model for one publicly declared candidate mechanism.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicCandidateModel {
    /// Candidate ID; this is not the sampled owner generator ID.
    pub hypothesis_id: String,
    /// Initial joint physical/sensor state belief conditional on this candidate.
    pub initial_state_belief: Vec<PublicInitialStateMass>,
    /// Complete rows in physical-state, sensor-state, action catalogue order.
    pub transition_rows: Vec<PublicTransitionRow>,
}

/// Formal public model catalogue, independent of the hidden owner generator.
///
/// Candidate priors live with public hypothesis descriptions in the
/// PublicResearchProfile; this catalogue contains candidate-conditional
/// state and transition models. The SHA-256 commits to canonical serialized
/// contents excluding the digest field itself.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicModelCatalogue {
    /// Public schema version.
    pub version: String,
    /// Number of candidate-local physical states.
    pub physical_state_count: usize,
    /// Number of candidate-local sensor states.
    pub sensor_state_count: usize,
    /// Ordered public action IDs.
    pub action_ids: Vec<String>,
    /// Ordered public categorical observation IDs.
    pub observation_ids: Vec<String>,
    /// Candidate models in the same order as public candidate descriptions.
    pub candidate_models: Vec<PublicCandidateModel>,
    /// SHA-256 of canonical JSON serialization of all preceding catalogue fields.
    pub catalogue_sha256: String,
}

#[derive(Serialize)]
struct CatalogueHashContent<'a> {
    version: &'a str,
    physical_state_count: usize,
    sensor_state_count: usize,
    action_ids: &'a [String],
    observation_ids: &'a [String],
    candidate_models: &'a [PublicCandidateModel],
}

/// SHA-256 of canonical serde JSON for any public profile/catalogue object.
pub fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{digest:x}"))
}

impl PublicModelCatalogue {
    /// Build a public candidate catalogue from the owner kernel for an explicit
    /// candidate ID subset. Candidate priors are task priors supplied separately;
    /// no sampled generator prior/identity is copied into this object.
    pub fn from_kernel(
        kernel: &JointKernel,
        candidate_ids: &[String],
    ) -> Result<Self, String> {
        kernel.validate()?;
        if candidate_ids.is_empty() {
            return Err("public candidate catalogue must not be empty".to_owned());
        }
        let mut seen = BTreeSet::new();
        let mut candidate_models = Vec::with_capacity(candidate_ids.len());
        for candidate_id in candidate_ids {
            if !seen.insert(candidate_id.as_str()) {
                return Err("public candidate catalogue contains a duplicate ID".to_owned());
            }
            let hypothesis = kernel
                .hypothesis_ids
                .iter()
                .position(|id| id == candidate_id)
                .ok_or_else(|| format!("public candidate {candidate_id} is absent from owner kernel"))?;

            let mut candidate_initial_mass = Rational::zero();
            for point in &kernel.initial_belief {
                if point.hypothesis == hypothesis {
                    candidate_initial_mass = candidate_initial_mass.checked_add(point.mass)?;
                }
            }
            if candidate_initial_mass.is_zero() {
                return Err(format!(
                    "public candidate {candidate_id} has no conditional initial-state support"
                ));
            }

            let mut initial_state_belief = Vec::new();
            for point in &kernel.initial_belief {
                if point.hypothesis == hypothesis {
                    initial_state_belief.push(PublicInitialStateMass {
                        physical_state: point.physical_state,
                        sensor_state: point.sensor_state,
                        conditional_mass: PublicProbability::from_rational(
                            point.mass.checked_div(candidate_initial_mass)?,
                        ),
                    });
                }
            }

            let mut transition_rows = Vec::with_capacity(
                kernel.physical_state_count
                    * kernel.sensor_state_count
                    * kernel.action_ids.len(),
            );
            for physical_state in 0..kernel.physical_state_count {
                for sensor_state in 0..kernel.sensor_state_count {
                    for (action, action_id) in kernel.action_ids.iter().enumerate() {
                        let row = kernel
                            .transition_row(hypothesis, physical_state, sensor_state, action)
                            .ok_or_else(|| "owner kernel lacks a candidate transition row".to_owned())?;
                        let mut outcomes = Vec::with_capacity(row.outcomes.len());
                        for outcome in &row.outcomes {
                            let observation_id = outcome
                                .observation
                                .map(|index| {
                                    kernel
                                        .observation_ids
                                        .get(index)
                                        .cloned()
                                        .ok_or_else(|| "owner kernel outcome has an invalid observation".to_owned())
                                })
                                .transpose()?;
                            outcomes.push(PublicTransitionOutcome {
                                probability: PublicProbability::from_rational(outcome.probability),
                                next_physical_state: outcome.next_physical_state,
                                next_sensor_state: outcome.next_sensor_state,
                                observation_id,
                            });
                        }
                        transition_rows.push(PublicTransitionRow {
                            physical_state,
                            sensor_state,
                            action_id: action_id.clone(),
                            outcomes,
                        });
                    }
                }
            }

            candidate_models.push(PublicCandidateModel {
                hypothesis_id: candidate_id.clone(),
                initial_state_belief,
                transition_rows,
            });
        }

        let mut catalogue = Self {
            version: PUBLIC_MODEL_CATALOGUE_VERSION.to_owned(),
            physical_state_count: kernel.physical_state_count,
            sensor_state_count: kernel.sensor_state_count,
            action_ids: kernel.action_ids.clone(),
            observation_ids: kernel.observation_ids.clone(),
            candidate_models,
            catalogue_sha256: String::new(),
        };
        catalogue.catalogue_sha256 = catalogue.compute_sha256()?;
        let encoded = serde_json::to_vec(&catalogue).map_err(|error| error.to_string())?;
        if encoded.len() > MAX_PUBLIC_MODEL_CATALOGUE_BYTES {
            return Err(format!(
                "public model catalogue exceeds {} byte cap",
                MAX_PUBLIC_MODEL_CATALOGUE_BYTES
            ));
        }
        Ok(catalogue)
    }

    /// Reconstruct the declared candidate predictive kernel without access to
    /// the hidden owner generator. Task priors come from the public profile;
    /// costs come from the public action catalogue.
    pub fn to_joint_kernel(
        &self,
        hypotheses: &[PublicHypothesis],
        actions: &[PublicAction],
    ) -> Result<JointKernel, String> {
        self.verify_sha256()?;
        if self.version != PUBLIC_MODEL_CATALOGUE_VERSION {
            return Err("unsupported public model catalogue version".to_owned());
        }
        if hypotheses.is_empty()
            || hypotheses.len() != self.candidate_models.len()
            || actions.len() != self.action_ids.len()
        {
            return Err("public catalogue/profile dimensions do not agree".to_owned());
        }

        let mut prior_total = Rational::zero();
        let mut initial_belief = Vec::new();
        let mut hypothesis_ids = Vec::with_capacity(hypotheses.len());
        let mut hypothesis_seen = BTreeSet::new();
        for (index, (hypothesis, model)) in
            hypotheses.iter().zip(&self.candidate_models).enumerate()
        {
            if hypothesis.hypothesis_id != model.hypothesis_id
                || !hypothesis_seen.insert(hypothesis.hypothesis_id.as_str())
            {
                return Err("public candidate and model catalogue order/IDs disagree".to_owned());
            }
            let prior = hypothesis.prior_probability.to_rational()?;
            if prior.is_zero() {
                return Err("public task prior must be positive for each candidate".to_owned());
            }
            prior_total = prior_total.checked_add(prior)?;
            hypothesis_ids.push(hypothesis.hypothesis_id.clone());

            let mut conditional_total = Rational::zero();
            for state in &model.initial_state_belief {
                if state.physical_state >= self.physical_state_count
                    || state.sensor_state >= self.sensor_state_count
                {
                    return Err("public initial-state belief references an invalid state".to_owned());
                }
                let conditional = state.conditional_mass.to_rational()?;
                if conditional.is_zero() {
                    return Err("public initial-state conditional mass must be positive".to_owned());
                }
                conditional_total = conditional_total.checked_add(conditional)?;
                initial_belief.push(JointMass {
                    hypothesis: index,
                    physical_state: state.physical_state,
                    sensor_state: state.sensor_state,
                    mass: prior.checked_mul(conditional)?,
                });
            }
            if conditional_total != Rational::one() {
                return Err("each candidate's conditional initial-state mass must sum to one".to_owned());
            }
        }
        if prior_total != Rational::one() {
            return Err("public candidate task priors must sum exactly to one".to_owned());
        }

        let action_ids: Vec<String> = actions.iter().map(|action| action.action_id.clone()).collect();
        let action_costs: Vec<u64> = actions.iter().map(|action| action.cost_units).collect();
        if action_ids != self.action_ids || action_costs.iter().any(|cost| *cost == 0) {
            return Err("public action identities/costs do not match model catalogue".to_owned());
        }
        let mut observation_seen = BTreeSet::new();
        for observation_id in &self.observation_ids {
            if observation_id.trim().is_empty() || !observation_seen.insert(observation_id.as_str()) {
                return Err("public observation IDs must be non-empty and unique".to_owned());
            }
        }

        let mut rows = Vec::new();
        let expected_rows = self
            .physical_state_count
            .checked_mul(self.sensor_state_count)
            .and_then(|count| count.checked_mul(self.action_ids.len()))
            .ok_or_else(|| "public candidate row count overflow".to_owned())?;
        for (hypothesis_index, model) in self.candidate_models.iter().enumerate() {
            if model.transition_rows.len() != expected_rows {
                return Err("public candidate model has an incomplete transition table".to_owned());
            }
            let mut seen_rows = BTreeSet::new();
            let mut indexed_rows = Vec::with_capacity(expected_rows);
            for public_row in &model.transition_rows {
                let action_index = self
                    .action_ids
                    .iter()
                    .position(|action_id| action_id == &public_row.action_id)
                    .ok_or_else(|| "public transition row references an unknown action".to_owned())?;
                if public_row.physical_state >= self.physical_state_count
                    || public_row.sensor_state >= self.sensor_state_count
                    || !seen_rows.insert((
                        public_row.physical_state,
                        public_row.sensor_state,
                        action_index,
                    ))
                    || public_row.outcomes.is_empty()
                {
                    return Err("public transition row has invalid/duplicate coordinates".to_owned());
                }
                let mut outcomes = Vec::with_capacity(public_row.outcomes.len());
                let mut probability_total = Rational::zero();
                for public_outcome in &public_row.outcomes {
                    if public_outcome.next_physical_state >= self.physical_state_count
                        || public_outcome.next_sensor_state >= self.sensor_state_count
                    {
                        return Err("public transition outcome points outside state dimensions".to_owned());
                    }
                    let observation = match &public_outcome.observation_id {
                        Some(id) => Some(
                            self.observation_ids
                                .iter()
                                .position(|candidate| candidate == id)
                                .ok_or_else(|| "public transition outcome references unknown observation".to_owned())?,
                        ),
                        None => None,
                    };
                    let probability = public_outcome.probability.to_rational()?;
                    if probability.is_zero() {
                        return Err("public transition outcome probability must be positive".to_owned());
                    }
                    probability_total = probability_total.checked_add(probability)?;
                    outcomes.push(JointOutcome {
                        probability,
                        next_physical_state: public_outcome.next_physical_state,
                        next_sensor_state: public_outcome.next_sensor_state,
                        observation,
                    });
                }
                if probability_total != Rational::one() {
                    return Err("public transition outcome probabilities must sum exactly to one".to_owned());
                }
                indexed_rows.push((
                    public_row.physical_state,
                    public_row.sensor_state,
                    action_index,
                    KernelRow {
                        hypothesis: hypothesis_index,
                        physical_state: public_row.physical_state,
                        sensor_state: public_row.sensor_state,
                        action: action_index,
                        outcomes,
                    },
                ));
            }
            if seen_rows.len() != expected_rows {
                return Err("public candidate transition table is not a complete state/action grid".to_owned());
            }
            indexed_rows.sort_by_key(|(physical, sensor, action, _)| (*physical, *sensor, *action));
            rows.extend(indexed_rows.into_iter().map(|(_, _, _, row)| row));
        }

        let kernel = JointKernel {
            hypothesis_ids,
            physical_state_count: self.physical_state_count,
            sensor_state_count: self.sensor_state_count,
            action_ids: self.action_ids.clone(),
            observation_ids: self.observation_ids.clone(),
            action_costs,
            initial_belief,
            rows,
        };
        kernel.validate()?;
        Ok(kernel)
    }

    /// Recompute the catalogue digest and verify exact owner-derived model rows.
    pub fn validate_against_kernel(
        &self,
        kernel: &JointKernel,
        candidate_ids: &[String],
    ) -> Result<(), String> {
        if self.version != PUBLIC_MODEL_CATALOGUE_VERSION {
            return Err("unsupported public model catalogue version".to_owned());
        }
        let expected = Self::from_kernel(kernel, candidate_ids)?;
        if self != &expected {
            return Err(
                "public predictive catalogue does not match the declared candidate models in owner kernel"
                    .to_owned(),
            );
        }
        Ok(())
    }

    /// Verify only the canonical SHA-256, without access to an owner kernel.
    pub fn verify_sha256(&self) -> Result<(), String> {
        let encoded = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        if encoded.len() > MAX_PUBLIC_MODEL_CATALOGUE_BYTES {
            return Err(format!(
                "public model catalogue exceeds {} byte cap",
                MAX_PUBLIC_MODEL_CATALOGUE_BYTES
            ));
        }
        let expected = self.compute_sha256()?;
        if self.catalogue_sha256 != expected {
            return Err("public model catalogue SHA-256 mismatch".to_owned());
        }
        Ok(())
    }

    fn compute_sha256(&self) -> Result<String, String> {
        let content = CatalogueHashContent {
            version: &self.version,
            physical_state_count: self.physical_state_count,
            sensor_state_count: self.sensor_state_count,
            action_ids: &self.action_ids,
            observation_ids: &self.observation_ids,
            candidate_models: &self.candidate_models,
        };
        canonical_sha256(&content)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::episode::{
        PublicAction, PublicActionDescription, PublicHypothesis, PublicObservationDescription,
        PublicProbability,
    };

    fn tiny_kernel() -> JointKernel {
        JointKernel {
            hypothesis_ids: vec!["candidate-a".to_owned(), "candidate-b".to_owned()],
            physical_state_count: 1,
            sensor_state_count: 1,
            action_ids: vec!["measure".to_owned()],
            observation_ids: vec!["zero".to_owned(), "one".to_owned()],
            action_costs: vec![2],
            initial_belief: vec![
                JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: Rational::new(1, 2).unwrap(),
                },
                JointMass {
                    hypothesis: 1,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: Rational::new(1, 2).unwrap(),
                },
            ],
            rows: vec![
                KernelRow {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    action: 0,
                    outcomes: vec![JointOutcome {
                        probability: Rational::one(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(0),
                    }],
                },
                KernelRow {
                    hypothesis: 1,
                    physical_state: 0,
                    sensor_state: 0,
                    action: 0,
                    outcomes: vec![JointOutcome {
                        probability: Rational::one(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(1),
                    }],
                },
            ],
        }
    }

    fn public_hypotheses() -> Vec<PublicHypothesis> {
        vec![
            PublicHypothesis {
                hypothesis_id: "candidate-a".to_owned(),
                structural_family: "test".to_owned(),
                description: "Predicts zero.".to_owned(),
                prior_probability: PublicProbability::new(1, 2).unwrap(),
            },
            PublicHypothesis {
                hypothesis_id: "candidate-b".to_owned(),
                structural_family: "test".to_owned(),
                description: "Predicts one.".to_owned(),
                prior_probability: PublicProbability::new(1, 2).unwrap(),
            },
        ]
    }

    fn public_actions() -> Vec<PublicAction> {
        vec![PublicAction {
            action_id: "measure".to_owned(),
            description: "Measure the signal.".to_owned(),
            target: "signal".to_owned(),
            cost_units: 2,
        }]
    }

    #[test]
    fn public_probability_round_trips_exact_u128_values_without_float_conversion() {
        let value = PublicProbability::new(u128::MAX - 1, u128::MAX).unwrap();
        assert_eq!(value.to_rational().unwrap().numerator(), u128::MAX - 1);
        assert_eq!(value.to_rational().unwrap().denominator(), u128::MAX);
        assert!(PublicProbability {
            numerator: "02".to_owned(),
            denominator: "04".to_owned(),
        }
        .to_rational()
        .is_err());
    }

    #[test]
    fn public_catalogue_reconstructs_the_exact_candidate_kernel_and_task_prior() {
        let owner = tiny_kernel();
        let ids = owner.hypothesis_ids.clone();
        let catalogue = PublicModelCatalogue::from_kernel(&owner, &ids).unwrap();
        catalogue.verify_sha256().unwrap();
        let reconstructed = catalogue
            .to_joint_kernel(&public_hypotheses(), &public_actions())
            .unwrap();
        assert_eq!(reconstructed.hypothesis_ids, owner.hypothesis_ids);
        assert_eq!(reconstructed.action_costs, vec![2]);
        assert_eq!(reconstructed.rows, owner.rows);
        assert_eq!(reconstructed.initial_belief, owner.initial_belief);
    }

    #[test]
    fn public_catalogue_hash_detects_modified_candidate_transition() {
        let owner = tiny_kernel();
        let ids = owner.hypothesis_ids.clone();
        let mut catalogue = PublicModelCatalogue::from_kernel(&owner, &ids).unwrap();
        catalogue.candidate_models[0].transition_rows[0].outcomes[0].probability =
            PublicProbability::new(1, 2).unwrap();
        assert!(catalogue.verify_sha256().is_err());
    }

    #[test]
    fn public_catalogue_is_subset_scoped_and_never_serializes_hidden_candidate() {
        let owner = tiny_kernel();
        let ids = vec!["candidate-a".to_owned()];
        let catalogue = PublicModelCatalogue::from_kernel(&owner, &ids).unwrap();
        let wire = serde_json::to_string(&catalogue).unwrap();
        assert!(wire.contains("candidate-a"));
        assert!(!wire.contains("candidate-b"));
        assert_eq!(catalogue.candidate_models.len(), 1);
    }

    #[test]
    fn public_task_prior_is_independent_of_hidden_generator_sampling_prior() {
        let owner = tiny_kernel();
        let ids = owner.hypothesis_ids.clone();
        let catalogue = PublicModelCatalogue::from_kernel(&owner, &ids).unwrap();
        let mut hypotheses = public_hypotheses();
        hypotheses[0].prior_probability = PublicProbability::new(3, 4).unwrap();
        hypotheses[1].prior_probability = PublicProbability::new(1, 4).unwrap();
        let reconstructed = catalogue
            .to_joint_kernel(&hypotheses, &public_actions())
            .unwrap();
        let mass_a = reconstructed
            .initial_belief
            .iter()
            .find(|point| point.hypothesis == 0)
            .unwrap()
            .mass;
        let mass_b = reconstructed
            .initial_belief
            .iter()
            .find(|point| point.hypothesis == 1)
            .unwrap()
            .mass;
        assert_eq!(mass_a, Rational::new(3, 4).unwrap());
        assert_eq!(mass_b, Rational::new(1, 4).unwrap());
        assert_ne!(reconstructed.initial_belief, owner.initial_belief);
    }

    #[test]
    fn task_prior_and_candidate_model_order_must_match() {
        let owner = tiny_kernel();
        let ids = owner.hypothesis_ids.clone();
        let catalogue = PublicModelCatalogue::from_kernel(&owner, &ids).unwrap();
        let mut hypotheses = public_hypotheses();
        hypotheses.swap(0, 1);
        assert!(catalogue.to_joint_kernel(&hypotheses, &public_actions()).is_err());
    }

    #[test]
    fn public_catalogue_rejects_incomplete_or_duplicate_transition_rows() {
        let owner = tiny_kernel();
        let ids = owner.hypothesis_ids.clone();
        let mut catalogue = PublicModelCatalogue::from_kernel(&owner, &ids).unwrap();
        catalogue.candidate_models[0].transition_rows.pop();
        catalogue.catalogue_sha256 = catalogue.compute_sha256().unwrap();
        assert!(catalogue.to_joint_kernel(&public_hypotheses(), &public_actions()).is_err());
    }

    #[test]
    fn public_catalogue_rejects_candidate_prior_that_does_not_sum_to_one() {
        let owner = tiny_kernel();
        let ids = owner.hypothesis_ids.clone();
        let catalogue = PublicModelCatalogue::from_kernel(&owner, &ids).unwrap();
        let mut hypotheses = public_hypotheses();
        hypotheses[0].prior_probability = PublicProbability::new(3, 4).unwrap();
        assert!(catalogue.to_joint_kernel(&hypotheses, &public_actions()).is_err());
    }
}
