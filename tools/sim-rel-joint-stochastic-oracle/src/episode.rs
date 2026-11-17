//! Interactive episode adapter for the exact joint stochastic world.
//!
//! The provider owns the sampled mechanism, physical state, sensor state, and
//! deterministic simulation PRNG. Policy-visible views contain only the public
//! action catalogue, observations already returned by the world, time/step
//! counters, budget counters, and the caller's own attempt dispositions.
//!
//! The SplitMix64 stream is reproducible simulation randomness, not a
//! cryptographic secret. A blind evaluation still requires the independent
//! runner/oracle custody boundary described in the research protocol.

use super::{JointKernel, Rational};

/// Hard cap on one in-memory episode's retained action-attempt records.
pub const MAX_ATTEMPT_RECEIPTS: usize = 65_536;

// SplitMix64 emits 2^53 equiprobable values. Requiring at least two grid
// points per event avoids silently treating a positive sub-resolution event
// as a reliably sampleable trajectory outcome.
const MIN_SAMPLE_MASS: f64 = 2.0 / 9_007_199_254_740_992.0;

/// Resource and horizon limits for one interactive episode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EpisodeConfig {
    /// Maximum number of accepted interventions.
    pub max_attempts: u64,
    /// Maximum accumulated cost of accepted interventions.
    pub max_cost_units: u64,
    /// Maximum number of world transitions.
    pub max_world_steps: u64,
}

impl Default for EpisodeConfig {
    fn default() -> Self {
        Self {
            max_attempts: 8,
            max_cost_units: 64,
            max_world_steps: 8,
        }
    }
}

/// Public, non-secret identities that bind one episode to its exact model inputs.
///
/// All digests are lowercase SHA-256 hex strings supplied by the trusted runner.
/// These values may be shown to the research policy; per-episode seeds and hidden
/// initial-state realizations must not be included here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodeManifest {
    /// Digest of the public episode contract and admissible action/observation surface.
    pub contract_sha256: String,
    /// Digest of the exact owner-model implementation.
    pub world_model_sha256: String,
    /// Digest of the exact selected model/profile and applicability declaration.
    pub model_profile_sha256: String,
    /// Digest of the world-generation recipe, excluding private episode seed/state.
    pub world_recipe_sha256: String,
    /// Digest of the clock semantics used by this episode.
    pub clock_profile_sha256: String,
    /// Digest of the episode's time-origin definition.
    pub time_origin_sha256: String,
}

impl EpisodeManifest {
    fn validate(&self) -> Result<(), EpisodeError> {
        for digest in [
            self.contract_sha256.as_str(),
            self.world_model_sha256.as_str(),
            self.model_profile_sha256.as_str(),
            self.world_recipe_sha256.as_str(),
            self.clock_profile_sha256.as_str(),
            self.time_origin_sha256.as_str(),
        ] {
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(EpisodeError::InvalidManifest);
            }
        }
        Ok(())
    }
}

/// Public description of one action available to the research policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicAction {
    /// Stable action identity from the admitted model profile.
    pub action_id: String,
    /// Declared cost for an accepted intervention.
    pub cost_units: u64,
}

/// An observation that has been emitted by the owner model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodeObservation {
    /// Stable observation identity from the admitted model profile.
    pub observation_id: String,
    /// Discrete world step at which the modeled phenomenon occurred.
    pub event_step: u64,
    /// Discrete world step at which the model's observation channel acquired it.
    pub acquisition_step: u64,
    /// Discrete world step at which the observation became policy-visible.
    pub available_at_step: u64,
}

/// Disposition of one policy action request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttemptDisposition {
    /// The intervention was executed and returned a safe observation.
    Applied {
        /// Observation emitted by the model.
        observation_id: String,
    },
    /// The intervention was executed and entered a terminal hazard state.
    HazardTerminal,
    /// No action with this identity exists in the public action catalogue.
    RejectedUnknownAction,
    /// The episode was already terminal.
    RejectedTerminal,
    /// The request exceeded the configured accepted-intervention count budget.
    RejectedAttemptBudget,
    /// The request would exceed the configured cumulative cost budget.
    RejectedCostBudget,
    /// The request exceeded the configured world-step horizon.
    RejectedStepBudget,
}

/// Append-only record for one policy action request.
///
/// Rejected requests are retained. requested_cost_units is the catalogue
/// cost when the action is known. charged_cost_units is non-zero only when
/// the owner model actually executed the intervention.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodeAttempt {
    /// Stable episode-local attempt identity.
    pub attempt_id: String,
    /// Contiguous zero-based ordinal.
    pub ordinal: u64,
    /// Action identity supplied by the policy, including unknown IDs.
    pub requested_action_id: String,
    /// Catalogue cost, or None for an unknown action.
    pub requested_cost_units: Option<u64>,
    /// Cost charged for an executed intervention.
    pub charged_cost_units: u64,
    /// Decision step at which the request was received.
    pub requested_at_step: u64,
    /// World step at which the model executed the action; absent for rejected requests.
    pub executed_at_step: Option<u64>,
    /// Provider-owned outcome.
    pub disposition: AttemptDisposition,
}

/// Policy-visible snapshot of an interactive episode.
///
/// The hidden hypothesis index, physical state, sensor state, full joint
/// kernel, initial-belief realization, and PRNG seed are intentionally absent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodeView {
    /// Opaque episode identity supplied by the trusted runner.
    pub episode_id: String,
    /// Public exact model/profile/recipe/clock identities.
    pub manifest: EpisodeManifest,
    /// Current discrete world step.
    pub current_step: u64,
    /// Public action catalogue.
    pub actions: Vec<PublicAction>,
    /// Observations already emitted and available by current_step.
    pub observations: Vec<EpisodeObservation>,
    /// Attempt receipts, including rejected and failed requests.
    pub attempts: Vec<EpisodeAttempt>,
    /// Number of accepted interventions.
    pub accepted_attempts: u64,
    /// Accumulated cost of accepted interventions.
    pub spent_cost_units: u64,
    /// Frozen accepted-intervention count budget.
    pub max_attempts: u64,
    /// Frozen cost budget.
    pub max_cost_units: u64,
    /// Frozen world-step horizon.
    pub max_world_steps: u64,
    /// Number of requests not individually retained after the receipt cap was reached.
    pub dropped_attempts: u64,
    /// False once any attempt receipt has been dropped; never promote an incomplete trace.
    pub trace_complete: bool,
    /// Whether a hazard or terminal condition ended the episode.
    pub terminal: bool,
}

/// Fail-closed episode-provider error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EpisodeError {
    /// The joint stochastic model did not pass its owner validator.
    InvalidKernel(String),
    /// The episode ID is malformed.
    InvalidEpisodeId,
    /// One or more required public manifest digests are malformed.
    InvalidManifest,
    /// The configured horizon exceeds the joint oracle's supported horizon.
    InvalidConfig,
    /// A validated kernel unexpectedly lacks a transition row or observation.
    MissingTransitionData,
    /// A kernel row contains no outcomes.
    EmptyOutcomeRow,
    /// The floating-point sampler could not construct a valid distribution.
    InvalidSamplingDistribution,
    /// A positive rational event probability is below the 53-bit sampler resolution.
    SamplingResolutionTooCoarse,
    /// The action-attempt receipt cap was reached.
    AttemptReceiptLimit,
    /// An attempt ordinal overflowed.
    AttemptOrdinalOverflow,
    /// The total accepted cost overflowed.
    CostOverflow,
}

/// One active scientific-world episode over a frozen exact joint kernel.
///
/// The hidden state is evaluator/provider-owned. Consumers receive cloned
/// EpisodeView values and cannot read or mutate the hidden tuple.
pub struct EpisodeProvider {
    episode_id: String,
    manifest: EpisodeManifest,
    kernel: JointKernel,
    hypothesis: usize,
    physical_state: usize,
    sensor_state: usize,
    rng: SplitMix64,
    config: EpisodeConfig,
    current_step: u64,
    accepted_attempts: u64,
    spent_cost_units: u64,
    terminal: bool,
    dropped_attempts: u64,
    trace_complete: bool,
    observations: Vec<EpisodeObservation>,
    attempts: Vec<EpisodeAttempt>,
}

impl EpisodeProvider {
    /// Admit a validated kernel, sample one private initial joint state, and
    /// create an episode. The supplied seed remains private to this provider.
    pub fn new(
        episode_id: impl Into<String>,
        manifest: EpisodeManifest,
        kernel: JointKernel,
        config: EpisodeConfig,
        seed: u64,
    ) -> Result<Self, EpisodeError> {
        let episode_id = episode_id.into();
        if !valid_episode_id(&episode_id) {
            return Err(EpisodeError::InvalidEpisodeId);
        }
        if config.max_world_steps > 12
            || config.max_attempts > MAX_ATTEMPT_RECEIPTS as u64
        {
            return Err(EpisodeError::InvalidConfig);
        }
        manifest.validate()?;
        kernel
            .validate()
            .map_err(EpisodeError::InvalidKernel)?;

        let mut rng = SplitMix64::new(seed);
        let selected = sample_index(
            &kernel.initial_belief,
            |point| point.mass,
            &mut rng,
        )?;
        let initial = &kernel.initial_belief[selected];
        let hypothesis = initial.hypothesis;
        let physical_state = initial.physical_state;
        let sensor_state = initial.sensor_state;

        Ok(Self {
            episode_id,
            manifest,
            kernel,
            hypothesis,
            physical_state,
            sensor_state,
            rng,
            config,
            current_step: 0,
            accepted_attempts: 0,
            spent_cost_units: 0,
            terminal: false,
            dropped_attempts: 0,
            trace_complete: true,
            observations: Vec::new(),
            attempts: Vec::new(),
        })
    }

    /// Return a cloned policy-visible episode snapshot.
    ///
    /// Only observations whose available_at_step is no later than the
    /// current decision step are returned.
    pub fn view(&self) -> EpisodeView {
        let actions = self
            .kernel
            .action_ids
            .iter()
            .zip(self.kernel.action_costs.iter())
            .map(|(action_id, cost_units)| PublicAction {
                action_id: action_id.clone(),
                cost_units: *cost_units,
            })
            .collect();

        let observations = self
            .observations
            .iter()
            .filter(|observation| observation.available_at_step <= self.current_step)
            .cloned()
            .collect();

        EpisodeView {
            episode_id: self.episode_id.clone(),
            manifest: self.manifest.clone(),
            current_step: self.current_step,
            actions,
            observations,
            attempts: self.attempts.clone(),
            accepted_attempts: self.accepted_attempts,
            spent_cost_units: self.spent_cost_units,
            max_attempts: self.config.max_attempts,
            max_cost_units: self.config.max_cost_units,
            max_world_steps: self.config.max_world_steps,
            dropped_attempts: self.dropped_attempts,
            trace_complete: self.trace_complete,
            terminal: self.terminal,
        }
    }

    /// Submit one intervention request and retain its disposition.
    ///
    /// Requests are recorded even when the action is unknown, the episode is
    /// terminal, or a budget/horizon gate rejects the request. Rejected
    /// requests do not mutate the hidden world state and do not incur action
    /// cost. This provider's first profile is synchronous and discrete:
    /// event, acquisition, and availability steps share the resulting world
    /// step. Asynchronous observation timing requires a richer owner model.
    pub fn submit_action(
        &mut self,
        action_id: &str,
    ) -> Result<EpisodeAttempt, EpisodeError> {
        if self.attempts.len() >= MAX_ATTEMPT_RECEIPTS {
            self.dropped_attempts = self.dropped_attempts.saturating_add(1);
            self.trace_complete = false;
            self.terminal = true;
            return Err(EpisodeError::AttemptReceiptLimit);
        }

        let ordinal = u64::try_from(self.attempts.len())
            .map_err(|_| EpisodeError::AttemptOrdinalOverflow)?;
        let attempt_id = format!("attempt-{ordinal:08}");
        let requested_at_step = self.current_step;
        let action_index = self
            .kernel
            .action_ids
            .iter()
            .position(|candidate| candidate.as_str() == action_id);
        let requested_cost_units = action_index.map(|index| self.kernel.action_costs[index]);

        let disposition = match action_index {
            None => AttemptDisposition::RejectedUnknownAction,
            Some(_) if self.terminal => AttemptDisposition::RejectedTerminal,
            Some(_) if self.accepted_attempts >= self.config.max_attempts => {
                AttemptDisposition::RejectedAttemptBudget
            }
            Some(_) if self.current_step >= self.config.max_world_steps => {
                AttemptDisposition::RejectedStepBudget
            }
            Some(action_index) => {
                let action_cost = self.kernel.action_costs[action_index];
                let proposed_cost = self
                    .spent_cost_units
                    .checked_add(action_cost)
                    .ok_or(EpisodeError::CostOverflow)?;
                if proposed_cost > self.config.max_cost_units {
                    AttemptDisposition::RejectedCostBudget
                } else {
                    let outcome = {
                        let row = self
                            .kernel
                            .transition_row(
                                self.hypothesis,
                                self.physical_state,
                                self.sensor_state,
                                action_index,
                            )
                            .ok_or(EpisodeError::MissingTransitionData)?;
                        let outcome_index = sample_index(
                            &row.outcomes,
                            |item| item.probability,
                            &mut self.rng,
                        )?;
                        row.outcomes
                            .get(outcome_index)
                            .ok_or(EpisodeError::MissingTransitionData)?
                            .clone()
                    };

                    let observation_id = match outcome.observation {
                        Some(observation_index) => Some(
                            self.kernel
                                .observation_ids
                                .get(observation_index)
                                .ok_or(EpisodeError::MissingTransitionData)?
                                .clone(),
                        ),
                        None => None,
                    };

                    self.spent_cost_units = proposed_cost;
                    self.accepted_attempts += 1;
                    self.current_step += 1;
                    self.physical_state = outcome.next_physical_state;
                    self.sensor_state = outcome.next_sensor_state;

                    match observation_id {
                        Some(observation_id) => {
                            self.observations.push(EpisodeObservation {
                                observation_id: observation_id.clone(),
                                event_step: self.current_step,
                                acquisition_step: self.current_step,
                                available_at_step: self.current_step,
                            });
                            AttemptDisposition::Applied { observation_id }
                        }
                        None => {
                            self.terminal = true;
                            AttemptDisposition::HazardTerminal
                        }
                    }
                }
            }
        };

        let charged_cost_units = if matches!(
            &disposition,
            AttemptDisposition::Applied { .. } | AttemptDisposition::HazardTerminal
        ) {
            requested_cost_units.unwrap_or(0)
        } else {
            0
        };

        let executed_at_step = (charged_cost_units > 0).then_some(self.current_step);
        let receipt = EpisodeAttempt {
            attempt_id,
            ordinal,
            requested_action_id: action_id.to_owned(),
            requested_cost_units,
            charged_cost_units,
            requested_at_step,
            executed_at_step,
            disposition,
        };
        self.attempts.push(receipt.clone());
        Ok(receipt)
    }
}

fn valid_episode_id(value: &str) -> bool {
    value.len() >= 16
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

/// Deterministic, non-cryptographic SplitMix64 simulation stream.
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    /// Uniform value on [0, 1) with 53 bits of mantissa precision.
    fn next_unit_f64(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        mantissa as f64 / 9_007_199_254_740_992.0
    }
}

fn rational_as_f64(value: Rational) -> f64 {
    value.numerator() as f64 / value.denominator() as f64
}

/// Sample an outcome from an exactly normalized rational row using a
/// reproducible 53-bit floating-point draw.
///
/// The exact joint oracle remains the authority for exact finite-horizon
/// probability/frontier calculations. This trajectory sampler is a bounded
/// simulation adapter, not an exact sampler and not cryptographic randomness.
fn sample_index<T>(
    values: &[T],
    mass: impl Fn(&T) -> Rational,
    rng: &mut SplitMix64,
) -> Result<usize, EpisodeError> {
    if values.is_empty() {
        return Err(EpisodeError::EmptyOutcomeRow);
    }

    let mut weights = Vec::with_capacity(values.len());
    let mut total = 0.0_f64;
    for value in values {
        let weight = rational_as_f64(mass(value));
        if !weight.is_finite() || weight < 0.0 {
            return Err(EpisodeError::InvalidSamplingDistribution);
        }
        if weight < MIN_SAMPLE_MASS {
            return Err(EpisodeError::SamplingResolutionTooCoarse);
        }
        total += weight;
        weights.push(weight);
    }
    if !total.is_finite() || total <= 0.0 {
        return Err(EpisodeError::InvalidSamplingDistribution);
    }

    let threshold = rng.next_unit_f64() * total;
    let mut cumulative = 0.0_f64;
    for (index, weight) in weights.iter().enumerate() {
        cumulative += weight;
        if threshold < cumulative {
            return Ok(index);
        }
    }

    // Floating-point summation can round the final cumulative value down.
    // The exact kernel validator has already established positive total mass.
    Ok(values.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JointMass, JointOutcome, JointKernel, KernelRow};

    fn deterministic_kernel(hazard: bool) -> JointKernel {
        let outcomes = if hazard {
            vec![JointOutcome {
                probability: Rational::one(),
                next_physical_state: 0,
                next_sensor_state: 0,
                observation: None,
            }]
        } else {
            vec![JointOutcome {
                probability: Rational::one(),
                next_physical_state: 0,
                next_sensor_state: 0,
                observation: Some(0),
            }]
        };

        JointKernel {
            hypothesis_ids: vec!["hidden-mechanism-0".to_owned()],
            physical_state_count: 1,
            sensor_state_count: 1,
            action_ids: vec!["measure".to_owned()],
            observation_ids: vec!["signal".to_owned()],
            action_costs: vec![2],
            initial_belief: vec![JointMass {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                mass: Rational::one(),
            }],
            rows: vec![KernelRow {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                action: 0,
                outcomes,
            }],
        }
    }

    fn persistent_sensor_confounding_kernel() -> JointKernel {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            for physical_state in 0..2 {
                for sensor_state in 0..2 {
                    for action in 0..2 {
                        let outcome = if action == 0 {
                            JointOutcome {
                                probability: Rational::one(),
                                next_physical_state: physical_state,
                                next_sensor_state: sensor_state,
                                observation: Some(physical_state ^ sensor_state),
                            }
                        } else {
                            JointOutcome {
                                probability: Rational::one(),
                                next_physical_state: physical_state,
                                next_sensor_state: 0,
                                observation: Some(0),
                            }
                        };
                        rows.push(KernelRow {
                            hypothesis,
                            physical_state,
                            sensor_state,
                            action,
                            outcomes: vec![outcome],
                        });
                    }
                }
            }
        }

        JointKernel {
            hypothesis_ids: vec!["latent-mechanism-a".to_owned(), "latent-mechanism-b".to_owned()],
            physical_state_count: 2,
            sensor_state_count: 2,
            action_ids: vec!["measure-combined-signal".to_owned(), "calibrate-sensor".to_owned()],
            observation_ids: vec!["zero".to_owned(), "one".to_owned()],
            action_costs: vec![1, 1],
            initial_belief: vec![
                JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 1,
                    mass: Rational::new(1, 2).unwrap(),
                },
                JointMass {
                    hypothesis: 1,
                    physical_state: 1,
                    sensor_state: 0,
                    mass: Rational::new(1, 2).unwrap(),
                },
            ],
            rows,
        }
    }

    fn stochastic_kernel() -> JointKernel {
        JointKernel {
            hypothesis_ids: vec!["private-h0".to_owned()],
            physical_state_count: 1,
            sensor_state_count: 1,
            action_ids: vec!["measure".to_owned()],
            observation_ids: vec!["signal-a".to_owned(), "signal-b".to_owned()],
            action_costs: vec![1],
            initial_belief: vec![JointMass {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                mass: Rational::one(),
            }],
            rows: vec![KernelRow {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                action: 0,
                outcomes: vec![
                    JointOutcome {
                        probability: Rational::new(1, 2).unwrap(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(0),
                    },
                    JointOutcome {
                        probability: Rational::new(1, 2).unwrap(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(1),
                    },
                ],
            }],
        }
    }

    fn new_provider(kernel: JointKernel, seed: u64) -> EpisodeProvider {
        EpisodeProvider::new(
            "opaque-episode-0001",
            kernel,
            EpisodeConfig {
                max_attempts: 4,
                max_cost_units: 10,
                max_world_steps: 4,
            },
            seed,
        )
        .expect("valid episode")
    }

    #[test]
    fn public_view_contains_catalogue_and_no_hidden_model_fields() {
        let provider = new_provider(deterministic_kernel(false), 7);
        let view = provider.view();
        assert_eq!(view.episode_id, "opaque-episode-0001");
        assert_eq!(view.actions[0].action_id, "measure");
        assert!(view.observations.is_empty());
        assert_eq!(view.current_step, 0);
        assert!(!view.terminal);
    }

    #[test]
    fn deterministic_kernel_emits_observation_and_charges_exact_action_cost() {
        let mut provider = new_provider(deterministic_kernel(false), 7);
        let receipt = provider.submit_action("measure").unwrap();
        assert_eq!(receipt.charged_cost_units, 2);
        assert_eq!(
            receipt.disposition,
            AttemptDisposition::Applied {
                observation_id: "signal".to_owned()
            }
        );

        let view = provider.view();
        assert_eq!(view.current_step, 1);
        assert_eq!(view.accepted_attempts, 1);
        assert_eq!(view.spent_cost_units, 2);
        assert_eq!(view.observations.len(), 1);
        assert_eq!(view.observations[0].event_step, 1);
        assert_eq!(view.observations[0].acquisition_step, 1);
        assert_eq!(view.observations[0].available_at_step, 1);
    }

    #[test]
    fn hazard_outcome_terminates_episode_without_fabricating_observation() {
        let mut provider = new_provider(deterministic_kernel(true), 7);
        let receipt = provider.submit_action("measure").unwrap();
        assert_eq!(receipt.charged_cost_units, 2);
        assert_eq!(receipt.disposition, AttemptDisposition::HazardTerminal);

        let view = provider.view();
        assert!(view.terminal);
        assert!(view.observations.is_empty());

        let rejected = provider.submit_action("measure").unwrap();
        assert_eq!(rejected.disposition, AttemptDisposition::RejectedTerminal);
        assert_eq!(rejected.charged_cost_units, 0);
        assert_eq!(provider.view().attempts.len(), 2);
    }

    #[test]
    fn unknown_action_is_retained_without_world_mutation_or_cost() {
        let mut provider = new_provider(deterministic_kernel(false), 7);
        let receipt = provider.submit_action("hidden-action").unwrap();
        assert_eq!(
            receipt.disposition,
            AttemptDisposition::RejectedUnknownAction
        );
        assert_eq!(receipt.requested_cost_units, None);
        assert_eq!(receipt.charged_cost_units, 0);
        let view = provider.view();
        assert_eq!(view.attempts.len(), 1);
        assert_eq!(view.current_step, 0);
        assert_eq!(view.spent_cost_units, 0);
    }

    #[test]
    fn cost_budget_rejects_mutation_but_retains_request() {
        let mut provider = EpisodeProvider::new(
            "opaque-episode-0002",
            deterministic_kernel(false),
            EpisodeConfig {
                max_attempts: 4,
                max_cost_units: 1,
                max_world_steps: 4,
            },
            3,
        )
        .unwrap();
        let receipt = provider.submit_action("measure").unwrap();
        assert_eq!(
            receipt.disposition,
            AttemptDisposition::RejectedCostBudget
        );
        assert_eq!(receipt.requested_cost_units, Some(2));
        assert_eq!(receipt.charged_cost_units, 0);
        let view = provider.view();
        assert_eq!(view.attempts.len(), 1);
        assert_eq!(view.current_step, 0);
        assert_eq!(view.spent_cost_units, 0);
    }

    #[test]
    fn accepted_attempt_budget_rejects_additional_interventions_and_records_them() {
        let mut provider = EpisodeProvider::new(
            "opaque-episode-0003",
            deterministic_kernel(false),
            EpisodeConfig {
                max_attempts: 1,
                max_cost_units: 10,
                max_world_steps: 4,
            },
            3,
        )
        .unwrap();

        assert!(matches!(
            provider.submit_action("measure").unwrap().disposition,
            AttemptDisposition::Applied { .. }
        ));
        let rejected = provider.submit_action("measure").unwrap();
        assert_eq!(rejected.disposition, AttemptDisposition::RejectedAttemptBudget);
        assert_eq!(provider.view().attempts.len(), 2);
        assert_eq!(provider.view().current_step, 1);
    }

    #[test]
    fn physical_sensor_confound_requires_calibration_before_discriminating_measurement() {
        let kernel = persistent_sensor_confounding_kernel();
        let config = EpisodeConfig {
            max_attempts: 4,
            max_cost_units: 10,
            max_world_steps: 4,
        };
        // With this fixture and SplitMix64 mapping, seed 3 selects the first
        // initial tuple and seed 0 selects the second. The public histories
        // remain identical until the final post-calibration measurement.
        let mut first = EpisodeProvider::new("opaque-confound-a-0001", kernel.clone(), config, 3)
            .expect("first episode");
        let mut second = EpisodeProvider::new("opaque-confound-b-0001", kernel, config, 0)
            .expect("second episode");

        let first_measure = first.submit_action("measure-combined-signal").unwrap();
        let second_measure = second.submit_action("measure-combined-signal").unwrap();
        assert_eq!(
            first_measure.disposition,
            AttemptDisposition::Applied {
                observation_id: "one".to_owned()
            }
        );
        assert_eq!(first_measure.disposition, second_measure.disposition);
        assert_eq!(first.view().observations, second.view().observations);

        let first_calibration = first.submit_action("calibrate-sensor").unwrap();
        let second_calibration = second.submit_action("calibrate-sensor").unwrap();
        assert_eq!(
            first_calibration.disposition,
            AttemptDisposition::Applied {
                observation_id: "zero".to_owned()
            }
        );
        assert_eq!(first_calibration.disposition, second_calibration.disposition);
        assert_eq!(first.view().observations, second.view().observations);

        let first_discriminator = first.submit_action("measure-combined-signal").unwrap();
        let second_discriminator = second.submit_action("measure-combined-signal").unwrap();
        assert_eq!(
            first_discriminator.disposition,
            AttemptDisposition::Applied {
                observation_id: "zero".to_owned()
            }
        );
        assert_eq!(
            second_discriminator.disposition,
            AttemptDisposition::Applied {
                observation_id: "one".to_owned()
            }
        );
    }

    #[test]
    fn same_seed_and_same_action_trace_produce_same_observation_trace() {
        let kernel_a = stochastic_kernel();
        let kernel_b = stochastic_kernel();
        let mut first = new_provider(kernel_a, 99);
        let mut second = new_provider(kernel_b, 99);

        let first_receipt = first.submit_action("measure").unwrap();
        let second_receipt = second.submit_action("measure").unwrap();
        assert_eq!(first_receipt, second_receipt);
        assert_eq!(first.view(), second.view());
    }

    #[test]
    fn rejected_unknown_request_does_not_advance_the_private_random_stream() {
        let mut with_rejection = new_provider(stochastic_kernel(), 1234);
        let mut without_rejection = new_provider(stochastic_kernel(), 1234);

        assert_eq!(
            with_rejection
                .submit_action("not-an-action")
                .unwrap()
                .disposition,
            AttemptDisposition::RejectedUnknownAction
        );

        let after_rejection = with_rejection.submit_action("measure").unwrap();
        let direct = without_rejection.submit_action("measure").unwrap();
        assert_eq!(after_rejection.disposition, direct.disposition);
        assert_eq!(with_rejection.view().observations, without_rejection.view().observations);
    }

    #[test]
    fn transition_row_accessor_rejects_out_of_range_hidden_tuple_indices() {
        let kernel = deterministic_kernel(false);
        assert!(kernel.transition_row(1, 0, 0, 0).is_none());
        assert!(kernel.transition_row(0, 1, 0, 0).is_none());
        assert!(kernel.transition_row(0, 0, 1, 0).is_none());
        assert!(kernel.transition_row(0, 0, 0, 1).is_none());
    }

    #[test]
    fn a_different_seed_is_not_exposed_in_the_policy_view() {
        let first = new_provider(stochastic_kernel(), 11);
        let second = new_provider(stochastic_kernel(), 12);
        // The initial public contract is identical; the private sampled state
        // and PRNG differ, but neither is exposed before an observation.
        assert_eq!(first.view(), second.view());
    }

    #[test]
    fn configured_world_horizon_is_enforced_and_request_is_retained() {
        let mut provider = EpisodeProvider::new(
            "opaque-episode-0004",
            deterministic_kernel(false),
            EpisodeConfig {
                max_attempts: 4,
                max_cost_units: 10,
                max_world_steps: 1,
            },
            7,
        )
        .unwrap();

        assert!(matches!(
            provider.submit_action("measure").unwrap().disposition,
            AttemptDisposition::Applied { .. }
        ));
        let rejected = provider.submit_action("measure").unwrap();
        assert_eq!(rejected.disposition, AttemptDisposition::RejectedStepBudget);
        assert_eq!(provider.view().attempts.len(), 2);
        assert_eq!(provider.view().current_step, 1);
    }

    #[test]
    fn invalid_kernel_or_episode_id_is_rejected_before_episode_creation() {
        let invalid_id = EpisodeProvider::new(
            "short",
            deterministic_kernel(false),
            EpisodeConfig::default(),
            0,
        );
        assert!(matches!(
            invalid_id,
            Err(EpisodeError::InvalidEpisodeId)
        ));

        let mut invalid = deterministic_kernel(false);
        invalid.rows.clear();
        assert!(matches!(
            EpisodeProvider::new(
                "opaque-episode-0005",
                invalid,
                EpisodeConfig::default(),
                0,
            ),
            Err(EpisodeError::InvalidKernel(_))
        ));
    }
}
