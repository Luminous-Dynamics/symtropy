//! Public predictive models for the declared candidate family.
//!
//! The catalogue contains formal candidate priors, conditional initial
//! physical/sensor-state beliefs, and exact action-conditioned transition,
//! observation and hazard distributions. It never contains the sampled
//! owner generator identity, sampled hidden state or episode RNG seed.

use crate::{JointKernel, Rational};
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
        let bytes = serde_json::to_vec(&content).map_err(|error| error.to_string())?;
        let digest = Sha256::digest(bytes);
        Ok(format!("sha256:{digest:x}"))
    }
}
